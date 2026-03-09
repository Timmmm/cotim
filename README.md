# Cotim

Cotim is a simple tool to help SystemVerilog modules in Rust using DPI-C.

First you write a SystemVerilog module that you want to implement in Rust:

```
module mux(
    input var logic i_clk,
    input var logic i_rst,
    input var logic i_sel,
    input var logic i_a,
    input var logic i_b,
    output var logic o_aorb
);

endmodule;
```

Next, include a generated file in it.

```
module mux(
    input var logic i_clk,
    input var logic i_rst,
    input var logic i_sel,
    input var logic i_a,
    input var logic i_b,
    output var logic o_aorb
);

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
    fn new(module_path: &str) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            x: "hello".to_string(),
        }))
        // Can optionally save in registry.
    }

    fn tick(&mut self, inputs: Inputs) -> Outputs {
        // Your code here.
    }
}
```
