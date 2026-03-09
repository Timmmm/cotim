use anyhow::Result;
use std::path::PathBuf;

use clap::Parser;

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

    cotim::build(&args.input, &args.sv, &args.rs)
}
