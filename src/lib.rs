use std::{fs, path::Path};
use anyhow::Result;

mod generator;
mod parser;

pub fn build(input: &Path, sv: &Path, rs: &Path) -> Result<()> {
    let parse_result = parser::parse(&input)?;

    parser::validate(&parse_result)?;

    let output = generator::generate(&parse_result)?;

    fs::write(sv, output.sv)?;
    fs::write(rs, output.rs)?;

    Ok(())
}
