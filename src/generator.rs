use anyhow::{anyhow, Result};

use crate::parser::{ParseResult, Port, Direction};

pub struct Output {
    pub sv: String,
    pub rs: String,
}

/// Generate SystemVerilog and Rust code.
pub fn generate(parse_result: &ParseResult) -> Result<Output> {
    Ok(Output {
        sv: generate_sv(parse_result)?,
        rs: pretty_print_rust(&generate_rs(parse_result)?)?,
    })
}

/// Pretty print some Rust code. Only works on valid Rust code so this
/// also acts as a syntax check.
fn pretty_print_rust(input: &str) -> Result<String> {
    let syntax_tree = syn::parse_str(input)?;
    Ok(prettyplease::unparse(&syntax_tree))
}

trait ParseResultUtils {
    /// Get the SV trigger expression based on the (* trigger *) annotations.
    /// E.g. "posedge i_clk".
    fn sv_trigger_expression(&self) -> Result<String>;
}

impl ParseResultUtils for ParseResult {
    fn sv_trigger_expression(&self) -> Result<String> {
        Ok(match &self.trigger {
            Some(trigger) => {
                trigger.clone()
            }
            None => {
                let trigger_port = self.ports.iter().find(|p| p.trigger).ok_or(anyhow!("No trigger set"))?;
                format!("posedge {}", trigger_port.name)
            },
        })
    }
}

trait PortGenerationUtils {
    fn sv_arg_decl_direction(&self) -> &'static str;
    fn rs_field_type(&self) -> String;
    fn sv_tick_args(&self) -> (DeclArgs, CallArgs);
    fn rs_tick_decl_args(&self) -> DeclArgs;
    fn rs_input_output_initialisation(&self, direction: Direction) -> String;
}

struct DeclArgs(Vec<String>);
struct CallArgs(Vec<String>);

impl PortGenerationUtils for Port {
    // Return the string to use in the SV tick function declaration that
    // depends on the parameter direction.
    fn sv_arg_decl_direction(&self) -> &'static str {
        match self.direction {
            Direction::Input => "input",
            Direction::Output => "output",
        }
    }

    // Return the Rust type that should be used for the `Inputs`/`Outputs`
    // struct field.
    fn rs_field_type(&self) -> String {
        let ref_type = match self.direction {
            Direction::Input => "&'a ",
            Direction::Output => "&'a mut ",
        };
        match self.size_outer {
            Some(size_outer) => format!("[{ref_type}BitSlice<u32>; {}]", size_outer),
            None => format!("{ref_type}BitSlice<u32>"),
        }
    }

    fn sv_tick_args(&self) -> (DeclArgs, CallArgs) {
        let mut decl_args = Vec::new();
        let mut call_args = Vec::new();

        let name = &self.name;

        let mut add_args = |outer_index: Option<u64>| {
            let outer_slice_fragment = match outer_index {
                Some(i) => format!("[{i}]"),
                None => "".to_string(),
            };
            let decl_name = match outer_index {
                Some(i) => format!("{name}___{i}"),
                None => name.clone(),
            };

            decl_args.push(format!("{} bit[{}:0] {decl_name}", self.sv_arg_decl_direction(), self.size_inner - 1));
            call_args.push(format!("{name}{outer_slice_fragment}"));
        };

        match self.size_outer {
            Some(size_outer) => {
                // Array.
                for i in 0..size_outer {
                    add_args(Some(i));
                }
            },
            None => {
                // Single value.
                add_args(None);
            },
        }

        (DeclArgs(decl_args), CallArgs(call_args))
    }

    fn rs_tick_decl_args(&self) -> DeclArgs {
        let mut decl_args = Vec::new();

        let mut add_args = |outer_index: Option<u64>| {
            let decl_name = match outer_index {
                Some(i) => format!("{}___{}", self.name, i),
                None => self.name.clone(),
            };

            let ptr_type = match self.direction {
                Direction::Input => "*const",
                Direction::Output => "*mut",
            };
            decl_args.push(format!("{decl_name}: {ptr_type} u32"));
        };

        match self.size_outer {
            Some(size_outer) => {
                // Array.
                for i in 0..size_outer {
                    add_args(Some(i));
                }
            },
            None => {
                // Single value. (But 128 bits are sent as 2x 64-bit.)
                add_args(None);
            },
        }

        DeclArgs(decl_args)
    }

    // Rust code to initialise the inputs to the Rust tick function for this port.
    // Only output ports whose direction matches `direction`.
    fn rs_input_output_initialisation(&self, direction: Direction) -> String {
        if self.direction == direction {
            let name = &self.name;
            let size_inner = &self.size_inner;

            let func = match self.direction {
                Direction::Input => "bitslice_from_raw_parts",
                Direction::Output => "bitslice_from_raw_parts_mut",
            };
            let ref_type = match self.direction {
                Direction::Input => "&",
                Direction::Output => "&mut ",
            };
            match self.size_outer {
                Some(size_outer) => {
                    let elems = (0..size_outer).map(|i| format!("{ref_type}*{func}({name}___{i}.try_into().unwrap(), {size_inner})")).collect::<Vec<String>>().join(",");
                    format!("{name}: [{elems}],")
                }
                None => {
                    format!("{name}: {ref_type}*{func}({name}.try_into().unwrap(), {size_inner}),")
                }
            }
        } else {
            String::new()
        }
    }
}

fn generate_sv(parse_result: &ParseResult) -> Result<String> {
    // Arguments to the tick() function in the declaration.
    let mut tick_decl_args = vec!["input chandle ___instance".to_string()];
    // Arguments to the tick() function at the call site.
    let mut tick_call_args = vec!["___instance___".to_string()];

    for port in parse_result.ports.iter() {
        let (port_tick_decl_args, port_tick_call_args) = port.sv_tick_args();
        tick_decl_args.extend(port_tick_decl_args.0.into_iter());
        tick_call_args.extend(port_tick_call_args.0.into_iter());
    }

    Ok(format!(r#"
    import "DPI-C" function chandle {prefix}_new({new_decl_args});
    import "DPI-C" function void {prefix}_free(input chandle ___instance);
    import "DPI-C" function byte unsigned {prefix}_tick({tick_decl_args});

    chandle ___instance___ = null;

    initial begin
        ___instance___ = {prefix}_new({new_call_args});
        if (___instance___ == null) begin
            $fatal(0, "Failed to create {prefix} instance.");
        end
    end

    final begin
        if (___instance___ !== null) begin
            {prefix}_free(___instance___);
            ___instance___ = null;
        end
    end

    always @({trigger}) begin
        if ({prefix}_tick({tick_call_args}) != 0) begin
            $fatal(0, "Failed to tick {prefix} instance.");
        end
    end
    "#,
        prefix = parse_result.module_name,
        trigger = parse_result.sv_trigger_expression()?,
        new_decl_args = "", // TODO: Add new args.
        tick_decl_args = tick_decl_args.join(", "),
        new_call_args = "",
        tick_call_args = tick_call_args.join(", "),
    ))
}

fn generate_rs(parse_result: &ParseResult) -> Result<String> {

    // User should implement this code:
    //
    // struct Instance {
    //     x: String,
    // }

    // impl Instance {
    //     fn new(module_path: &str) -> Result<Arc<Mutex<Self>>> {
    //         Arc::new(Mutex::new(Self {
    //             x: "hello".to_string(),
    //         }))
    //         // Can optionally save in registry.
    //     }

    //     fn tick(&mut self, inputs: Inputs) -> Result<Outputs> {
    //         // Your code here.
    //     }
    // }

    let mut input_members = String::new();
    let mut output_members = String::new();
    // Map from member name to either an expression or an array of expressions (for arrays).
    let mut input_initialisation = String::new();
    let mut output_initialisation = String::new();
    let mut tick_decl_args = vec!["instance: *const Mutex<Instance>".to_string()];

    for port in parse_result.ports.iter() {
        // Add a member to either the `Inputs` or `Outputs` struct for the port.
        let members = match port.direction {
            Direction::Input => &mut input_members,
            Direction::Output => &mut output_members,
        };

        members.push_str(&format!("pub {}: {},", port.name, port.rs_field_type()));

        tick_decl_args.extend(port.rs_tick_decl_args().0.into_iter());

        input_initialisation.push_str(&port.rs_input_output_initialisation(Direction::Input));
        output_initialisation.push_str(&port.rs_input_output_initialisation(Direction::Output));

    }
    Ok(format!(r#"
use std::sync::{{Arc, Mutex}};
use bitvec::{{slice::BitSlice, ptr::{{bitslice_from_raw_parts, bitslice_from_raw_parts_mut}}}};
use super::Instance;

pub struct Inputs<'a> {{
{input_members}
}}

pub struct Outputs<'a> {{
{output_members}
}}

#[no_mangle]
extern "C" fn {prefix}_new(module_path: &str) -> *const Mutex<Instance> {{
    match Instance::new(module_path) {{
        Ok(instance) => {{
            Arc::into_raw(instance)
        }}
        Err(e) => {{
            eprintln!("Error in {prefix}_new: {{e:?}}");
            std::ptr::null()
        }}
    }}
}}

#[repr(u8)]
enum ReturnCode {{
    Success = 0,
    Failure = 1,
}}

#[no_mangle]
extern "C" fn {prefix}_tick({tick_decl_args}) -> ReturnCode {{
    unsafe {{
        let inputs = Inputs {{
            {input_initialisation}
        }};
        let mut outputs = Outputs {{
            {output_initialisation}
        }};
        let mut guard = instance.as_ref().unwrap().lock().unwrap();
        match guard.tick(&inputs, &mut outputs) {{
            Ok(()) => {{
                ReturnCode::Success
            }}
            Err(e) => {{
                eprintln!("Error in {prefix}_tick: {{e:?}}");
                ReturnCode::Failure
            }}
        }}
    }}
}}

#[no_mangle]
extern "C" fn {prefix}_free(instance: *const Mutex<Instance>) {{
    // Make new Arc and drop it to free it.
    unsafe {{
        Arc::from_raw(instance);
    }}
}}
"#,
        prefix=parse_result.module_name,
        input_members=input_members,
        output_members=output_members,
        input_initialisation=input_initialisation,
        output_initialisation=output_initialisation,
        tick_decl_args=tick_decl_args.join(", "),
    ))
}
