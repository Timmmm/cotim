# Cotim

Cotim is a simple tool to help SystemVerilog modules in Rust using DPI-C.

First you write a SystemVerilog module that you want to implement in Rust. Use a `trigger` annotation to indicate the clock.

```
// Either a full expression (useful for multiple triggers)
(* trigger="posedge i_clk" *)
module mux(
    // (* trigger *) // Or you can specify a port and it will use @(posedge <the port>)
    input var logic i_clk,
    input var logic i_rst,
    input var logic i_sel,
    input var logic i_a,
    input var logic i_b,
    input var logic[120:0] i_double,
    input var logic[7:0][15:0] i_u16_array,
    output var logic o_aorb,
    output var logic[3:0] o_slice,
    output var logic[64:0] o_double_slice,
    // If you need more than 128 bits you can do an array like this!
    output var logic[1:0][127:0] o_wide
);

// Include this generated file.
`include "mux.dpi.sv"

endmodule;
```

Next generate the `mux.dpi.sv` file using this tool.

```
cotim --input mux.sv --sv mux.dpi.sv --rs mux.rs
```

Finally write the Rust code to implement it.

```
mod mux;

struct Instance {
    x: String,
}

impl Instance {
    fn new(module_path: &str, plusarg: &str) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            x: "hello".to_string(),
        }))
    }

    fn tick(&mut self, inputs: Inputs) -> Outputs {
        // Your code here.
    }
}
```

`plusarg` will be based on the plusarg given to the simulator, e.g. `+mux=foo=1;baz=2` will set `plusarg` to `foo=1;baz=2`. It will be an empty string if not given.

Any single-`bit` or `logic` argument gets passed as `bool` or `&mut bool` in `Inputs` and `Outputs`. Bit vectors get passed using the `bitvec` crate. Packed arrays are not supported, but 2D packed arrays are (each outer array is passed as a separate argument). All sizes must be integer constants (not parameters).
