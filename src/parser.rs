use anyhow::{anyhow, bail, Result};
use std::collections::HashSet;
use std::path::PathBuf;
use std::{collections::HashMap, path::Path};
use sv_parser::{
    parse_sv, AnsiPortDeclaration, ConstantExpression, ConstantPrimary, DecimalNumber,
    IntegralNumber, ModuleDeclarationAnsi, NodeEvent, Number, PrimaryLiteral, RefNode, Signing,
    SyntaxTree, PackedDimension, AttributeInstance,
};

/// Port direction.
#[derive(Debug, PartialEq, Eq)]
pub enum Direction {
    // Input to the SV module and the tick function.
    Input,
    // Output from the SV module and the tick function.
    Output,
}

#[derive(Debug)]
pub struct Port {
    /// Port name. This is an identifier.
    pub name: String,
    /// Port direction (input or output).
    pub direction: Direction,
    /// Width of the port's inner dimension (this can be up to 128).
    pub size_inner: u64,
    /// Width of the port's outer dimension (if specified).
    pub size_outer: Option<u64>,
    /// True if there is a (* trigger *) attribute for this port.
    pub trigger: bool,
}


/// Result of parsing the .sv file. It should contain a single module
/// with some ports.
#[derive(Debug)]
pub struct ParseResult {
    pub module_name: String,
    // (* trigger="foo" *) attribute if present.
    pub trigger: Option<String>,
    pub ports: Vec<Port>,
}

/// Parse a SystemVerilog file containing a single module and return the
/// module name and the list of its ports.
pub fn parse(path: &Path) -> Result<ParseResult> {
    let defines = HashMap::new();
    let includes: Vec<PathBuf> = Vec::new();

    let (syntax_tree, _new_defines) = parse_sv(&path, &defines, &includes, true, false)?;
    // print_full_tree(&syntax_tree, true);
    let ports = analyze_defs(&syntax_tree)?;
    Ok(ports)
}

fn get_direction(port_decl: &AnsiPortDeclaration) -> Result<Direction> {
    Ok(match port_decl {
        AnsiPortDeclaration::Net(_) => todo!(),
        AnsiPortDeclaration::Variable(v) => {
            let header = v
                .nodes
                .0
                .as_ref()
                .ok_or(anyhow!("Missing variable port header"))?;
            let direction = header
                .nodes
                .0
                .as_ref()
                .ok_or(anyhow!("Missing port direction"))?;
            match direction {
                sv_parser::PortDirection::Input(_) => Direction::Input,
                sv_parser::PortDirection::Output(_) => Direction::Output,
                sv_parser::PortDirection::Inout(_) => bail!("inout not allowed"),
                sv_parser::PortDirection::Ref(_) => bail!("Ref not allowed"),
            }
        }
        AnsiPortDeclaration::Paren(_) => todo!(),
    })
}



enum SimpleConstant {
    UnsignedDecimal(u64),
    String(String),
}

/// Return the unsigned number or string from a constant expression. It must be a plain
/// decimal number (or string) for this to succeed.
fn get_simple_constant(node: &ConstantExpression, syntax_tree: &SyntaxTree) -> Result<SimpleConstant> {
    let ConstantExpression::ConstantPrimary(constant_primary) = node else { bail!("expected unsigned integer literal"); };
    let ConstantPrimary::PrimaryLiteral(primary_literal) = constant_primary.as_ref() else { bail!("expected unsigned integer literal"); };
    Ok(match primary_literal.as_ref() {
        PrimaryLiteral::Number(number) => {
            let Number::IntegralNumber(integral_number) = number.as_ref() else { bail!("expected unsigned integer literal"); };
            let IntegralNumber::DecimalNumber(decimal_number) = integral_number.as_ref() else { bail!("expected unsigned integer literal"); };
            let DecimalNumber::UnsignedNumber(unsigned_number) = decimal_number .as_ref() else { bail!("expected unsigned integer literal"); };
            let text = syntax_tree.get_str_trim(unsigned_number).ok_or(anyhow!("couldn't find integer span in source"))?;
            SimpleConstant::UnsignedDecimal(text.parse()?)
        },
        PrimaryLiteral::TimeLiteral(_) => bail!("unexpected time literal"),
        PrimaryLiteral::UnbasedUnsizedLiteral(_) => bail!("unexpected unbased unsized literal"),
        PrimaryLiteral::StringLiteral(string_literal) => {
            let text = syntax_tree.get_str_trim(&string_literal.nodes.0).ok_or(anyhow!("couldn't find text span in source"))?;
            // This is the raw quoted string, e.g. `"hello"`. I assume including escape sequences.
            // But for now we'll just strip the leading/trailing "".
            SimpleConstant::String(decode_string_literal(text)?)
        },
    })
}

/// Take the literal span of a `StringLiteral`, e.g. `"foo"` and decode it to
/// a string by stripping the quotes and replacing escape sequences.
fn decode_string_literal(s: &str) -> Result<String> {
    // It should start and end with "
    if s.len() < 2 {
        bail!("Invalid string literal (too short): {s:?}");
    }
    if let Some(s) = s.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
        // TODO: Decode backslashes. It's pretty simple - see section 5.9 in the LRM.
        Ok(s.to_owned())
    } else {
        bail!("Invalid string literal (doesn't start or end with '\"'): {s:?}");
    }
}

fn get_packed_dimension_width(node: &PackedDimension, syntax_tree: &SyntaxTree) -> Result<u64> {
    match node {
        sv_parser::PackedDimension::Range(r) => {
            let left = get_simple_constant(&r.nodes.0.nodes.1.nodes.0, syntax_tree)?;
            let right = get_simple_constant(&r.nodes.0.nodes.1.nodes.2, syntax_tree)?;

            if let (SimpleConstant::UnsignedDecimal(left), SimpleConstant::UnsignedDecimal(right)) = (left, right) {
                if right != 0 {
                    bail!("right part of range must be 0 (got {right})");
                }
                Ok(left + 1)
            } else {
                bail!("expected integer constants");
            }
        }
        sv_parser::PackedDimension::UnsizedDimension(_) => {
            bail!("port packed dimension cannot be unsized");
        }
    }
}

fn port_list(mod_def: &ModuleDeclarationAnsi, syntax_tree: &SyntaxTree) -> Result<Vec<Port>> {
    let list_of_port_declarations = match &mod_def.nodes.0.nodes.6 {
        Some(p) => p,
        None => {
            // This is hit if they just have `module foo;` with no brackets.
            return Ok(Vec::new());
        }
    };

    let list = match &list_of_port_declarations.nodes.0.nodes.1 {
        Some(l) => l.contents(),
        None => {
            // This is hit if they just have `module foo();`.
            return Ok(Vec::new());
        }
    };

    let mut ports = Vec::new();

    for item in list {

        let trigger = get_attribute(&item.0, syntax_tree, "trigger")?;

        let trigger = match trigger {
            // (* trigger *)
            Some(None) => true,
            // Attribute not present.
            None => false,
            // (* trigger=something *)
            _ => {
                bail!("port trigger attribute cannot have a value");
            }
        };

        let port_decl = &item.1;
        match port_decl {
            sv_parser::AnsiPortDeclaration::Variable(v) => {
                let name = v.nodes.1.nodes.0.clone();
                let name = syntax_tree
                    .get_str_trim(&name)
                    .ok_or(anyhow!(
                        "parse error: couldn't find port name span in source"
                    ))?
                    .to_owned();
                let direction = get_direction(port_decl)?;

                // Check there is no unpacked/unsized/etc dimensions.
                if !v.nodes.2.is_empty() {
                    bail!("port cannot have unpacked dimensions");
                }

                // Get the type (must be bit or logic) and slice dimension.
                let type_ = &v
                    .nodes
                    .0
                    .as_ref()
                    .ok_or(anyhow!(
                        "port must have direction and type specified explicitly"
                    ))?
                    .nodes
                    .1
                    .nodes
                    .0;

                let mut size_inner = 1;
                let mut size_outer = None;

                match type_ {
                    sv_parser::VarDataType::DataType(_) => {
                        // Just a data type with no `var` keyword.
                        bail!("'var' must be explicitly used in port");
                    }
                    sv_parser::VarDataType::Var(v) => {
                        // `var <datatype>`
                        match &v.nodes.1 {
                            sv_parser::DataTypeOrImplicit::DataType(dt) => match **dt {
                                sv_parser::DataType::Vector(ref v) => {
                                    // bit/logic/reg [signed/unsigned] {dimension}*
                                    match &v.nodes.0 {
                                        sv_parser::IntegerVectorType::Bit(_) => {}
                                        sv_parser::IntegerVectorType::Logic(_) => {}
                                        sv_parser::IntegerVectorType::Reg(_) => {
                                            bail!("Type must be bit or logic not reg")
                                        }
                                    }

                                    match &v.nodes.1 {
                                        Some(Signing::Signed(_)) => bail!("ports can't be signed"),
                                        _ => {}
                                    }

                                    match v.nodes.2.as_slice() {
                                        [] => {}
                                        [inner_dim] => {
                                            size_inner = get_packed_dimension_width(inner_dim, syntax_tree)?;
                                        }
                                        [outer_dim, inner_dim] => {
                                            size_inner = get_packed_dimension_width(inner_dim, syntax_tree)?;
                                            size_outer = Some(get_packed_dimension_width(outer_dim, syntax_tree)?);
                                        }
                                        _ => {
                                            bail!("ports must have a maximum of two packed dimensions");
                                        }
                                    }
                                }
                                _ => {
                                    bail!("port data type must be vector (e.g. `input var logic[3:0] i_data`)");
                                }
                            },
                            sv_parser::DataTypeOrImplicit::ImplicitDataType(_) => {
                                bail!("port data type cannot be implicit");
                            }
                        }
                    }
                }

                ports.push(Port {
                    name,
                    direction,
                    size_inner,
                    size_outer,
                    trigger,
                });
            }
            _ => {
                // Param - i.e. `.foo(bar)` or net declaration. For simplicity
                // we only support var declarations.
                bail!("all ports must be explicitly declared as `var` ports");
            }
        }
    }

    Ok(ports)
}

fn get_attribute(attribute_instances: &[AttributeInstance], syntax_tree: &SyntaxTree, attribute_name: &str) -> Result<Option<Option<SimpleConstant>>> {
    let mut result = None;
    for attribute_instance in attribute_instances {
        for attribute_spec in attribute_instance.nodes.1.contents() {
            let identifier = syntax_tree.get_str_trim(&attribute_spec.nodes.0).ok_or(anyhow!("error getting string"))?;
            if identifier == attribute_name {
                if result.is_some() {
                    bail!("duplicate attributes: {}", attribute_name);
                }
                // Get value.
                let val = attribute_spec.nodes.1.as_ref().map(|(_, constant_expression)| get_simple_constant(constant_expression, syntax_tree)).transpose()?;
                result = Some(val);
            }
        }
    }
    Ok(result)
}

fn analyze_defs(syntax_tree: &SyntaxTree) -> Result<ParseResult> {
    let mut result = None;
    for node in syntax_tree {
        match node {
            RefNode::ModuleDeclarationNonansi(_) => {
                bail!("Non-ansi module declarations not supported");
            }
            RefNode::ModuleDeclarationAnsi(mod_def) => {
                if result.is_some() {
                    bail!("more than one module declaration found");
                }

                let module_name = &mod_def.nodes.0.nodes.3.nodes.0;
                let module_name = syntax_tree.get_str_trim(module_name).ok_or(anyhow!("error getting string"))?.to_owned();

                let trigger = get_attribute(&mod_def.nodes.0.nodes.0, syntax_tree, "trigger")?;

                let trigger = match trigger {
                    // (* trigger="posedge i_clk" *)
                    Some(Some(SimpleConstant::String(s))) => Some(s),
                    // No trigger.
                    None => None,
                    // (* trigger *) or (* trigger=3 *)
                    _ => {
                        bail!("trigger attribute on module must specify a string value");
                    }

                };

                let ports = port_list(mod_def, syntax_tree)?;

                result = Some(
                    ParseResult {
                        trigger,
                        module_name,
                        ports,
                    }
                );
            }
            _ => {}
        }
    }
    result.ok_or(anyhow!("no module declarations found"))
}

pub fn validate(parse_result: &ParseResult) -> Result<()> {
    // Check that all inner dimensions are in [1, 128] and, that port names
    // are unique and that there is exactly one trigger specified.
    for port in parse_result.ports.iter() {
        if port.size_inner < 1 {
            bail!("Port '{}' inner size must be at least 1", port.name);
        }
        if port.size_inner > 128 {
            bail!("Port '{}' inner size must be at most 128. To pass more data use a 2D packed array.", port.name);
        }
    }

    let unique_names = parse_result.ports.iter().map(|port| port.name.as_str()).collect::<HashSet<&str>>();
    if unique_names.len() != parse_result.ports.len() {
        bail!("Port names must be unique");
        // TODO: Say which names are duplicate.
    }

    if unique_names.iter().any(|name| name.contains("___")) {
        bail!("Port names cannot contain '___'");
    }

    let trigger_count = parse_result.trigger.is_some() as u64 + parse_result.ports.iter().map(|port| port.trigger as u64).sum::<u64>();

    if trigger_count != 1 {
        bail!("exactly one trigger attribute must be present (either on a port or on the module)");
        // TODO: More detail.
    }

    Ok(())
}

#[allow(unused)]
fn print_full_tree(syntax_tree: &SyntaxTree, include_whitespace: bool) {
    let mut skip = false;
    let mut depth = 3;
    for node in syntax_tree.into_iter().event() {
        match node {
            NodeEvent::Enter(RefNode::Locate(locate)) => {
                if !skip {
                    println!(
                        "{}- Token: {}",
                        "  ".repeat(depth),
                        syntax_tree.get_str(locate).unwrap()
                    );
                    println!("{}  Line: {}", "  ".repeat(depth), locate.line);
                }
                depth += 1;
            }
            NodeEvent::Enter(RefNode::WhiteSpace(_)) => {
                if !include_whitespace {
                    skip = true;
                }
            }
            NodeEvent::Leave(RefNode::WhiteSpace(_)) => {
                skip = false;
            }
            NodeEvent::Enter(x) => {
                if !skip {
                    println!("{}- {}:", "  ".repeat(depth), x);
                }
                depth += 1;
            }
            NodeEvent::Leave(_) => {
                depth -= 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::path::Path;

    #[test]
    fn parse_ok() {
        dbg!(parse(Path::new("../mux.sv")).unwrap());
    }
}
