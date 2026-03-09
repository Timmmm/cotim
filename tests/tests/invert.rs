use crate::utils::*;
use tempfile::tempdir;

// Simple test that uses every combinary of input/output 1D/2D ports.
// The device simply inverts the inputs and puts them on the output.

#[test]
fn test_verilator() {
    // Make temporary directory.
    let temp_dir = tempdir().expect("error making temporary directory");
    // TODO: Revert when it's working
    // let temp_dir_path = temp_dir.path();
    let temp_dir_path = temp_dir.keep();

    dbg!(&temp_dir_path);

    let cargo_toml_path = temp_dir_path.join("Cargo.toml");
    let cpp_path = temp_dir_path.join("main.cpp");
    let sv_path = temp_dir_path.join("top.sv");
    let sv_dpi_path = temp_dir_path.join("top.dpi.sv");
    let src_path = temp_dir_path.join("src");
    std::fs::create_dir(&src_path).expect("error creating src directory");
    let rs_path = src_path.join("lib.rs"); // TODO: Rename to `top` and add a lib.rs.
    let rs_dpi_path = src_path.join("top_dpi.rs");
    let rs_dylib_path = temp_dir_path.join("target").join("debug").join("libtop.so");

    // TODO: Figure out how restarting simulations works.

    let sv = r#"
module top(
    (* trigger *)
    input var logic clk,
    input var logic [3:0] in_1d,
    input var logic [3:0][1:0] in_2d,
    output var logic [3:0] out_1d,
    output var logic [3:0][1:0] out_2d
);
    `include "top.dpi.sv"
endmodule;
"#;

    let rs = r#"
mod top_dpi;

use std::sync::{Arc, Mutex};
use top_dpi::{Inputs, Outputs};
use bitvec::field::BitField;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

struct Instance {
    x: String,
}

impl Instance {
    fn new(module_path: &str) -> Result<Arc<Mutex<Self>>> {
        Ok(Arc::new(Mutex::new(Self {
            x: "hello".to_string(),
        })))
        // Can optionally save in registry.
    }

    fn tick(&mut self, inputs: &Inputs, outputs: &mut Outputs) -> Result<()> {
        // outputs.out_1d.set_bool(!inputs.in_1d.bool());
        // outputs.out_1d[0].set_bool(!inputs.in_1d[0].bool());
        // outputs.out_1d[1].set_bool(!inputs.in_1d[1].bool());
        // outputs.out_1d[2].set_bool(!inputs.in_1d[2].bool());
        // outputs.out_1d[3].set_bool(!inputs.in_1d[3].bool());
        outputs.out_1d.store_le(!inputs.in_1d.load_le::<u32>());
        Ok(())

        // Ok(Outputs {
        //     out_1d: !inputs.in_1d,
        //     out_2d: [
        //         !inputs.in_2d[0],
        //         !inputs.in_2d[1],
        //         !inputs.in_2d[2],
        //         !inputs.in_2d[3],
        //     ],
        // })
    }
}
"#;

    let cargo_toml = r#"
[package]
name = "top"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
bitvec = "1.0.1"
"#;

    let cpp = r#"
#include <iostream>

#include <verilated.h>
#include "Vtop.h"

int main(int argc, char* argv[]) {
    Verilated::commandArgs(argc, argv);
    Vtop top;
    for (unsigned i = 0; i < 10; ++i) {
        top.clk = 0;
        top.in_1d = i;
        top.eval();
        top.clk = 1;
        top.eval();
        std::cout << "out_1d: " << (unsigned)top.out_1d << std::endl;
    }
    return 0;
}
"#;

    std::fs::write(&sv_path, sv).expect("error writing top.sv");
    std::fs::write(&rs_path, rs).expect("error writing top.rs");
    std::fs::write(&cargo_toml_path, cargo_toml).expect("error writing Cargo.toml");
    std::fs::write(&cpp_path, cpp).expect("error writing main.cpp");

    // 1. Run the binary to generate the Rust code etc.
    cotim(&["--input", sv_path.to_str().unwrap(), "--rs", rs_dpi_path.to_str().unwrap(), "--sv", sv_dpi_path.to_str().unwrap()], None).expect("error running cotim");

    // 2. Compile the Rust crate to cdylib.
    cargo(&["build"], Some(&temp_dir_path)).expect("error running cargo build");

    // 3. Run Verilator/VCS/Questa and link with it.
    verilator(&["--cc", "-sv", &format!("-I{}", temp_dir_path.to_str().unwrap()), sv_path.to_str().unwrap(), "--exe", cpp_path.to_str().unwrap(), "--build", "--top-module", "top", rs_dylib_path.to_str().unwrap()], Some(&temp_dir_path)).expect("error running verilator");

    // 4. Run the testbench.
    command("obj_dir/Vtop", &[], Some(&temp_dir_path)).expect("error running testbench");


}
