use anyhow::Result;
use std::{fs, path::PathBuf};

use clap::Parser;

mod generator;
mod parser;

/// Cotim generator.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input module file (e.g. fifo.sv)
    #[arg(long)]
    input: PathBuf,

    /// Output SystemVerilog file (e.g. fifo.dpi.sv)
    #[arg(long)]
    sv: PathBuf,

    /// Output Rust file (e.g. fifo.rs)
    #[arg(long)]
    rs: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let parse_result = parser::parse(&args.input)?;

    parser::validate(&parse_result)?;

    let output = generator::generate(&parse_result)?;

    fs::write(args.sv, output.sv)?;
    fs::write(args.rs, output.rs)?;

    Ok(())
}
