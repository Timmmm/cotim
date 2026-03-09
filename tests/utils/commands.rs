
use std::{io, fmt, path::Path, process::{self, Command}};
// use colored::*;
// use log::{debug};

// Define our error types. These may be customized for our error handling cases.
// Now we will be able to write our own errors, defer to an underlying error
// implementation, or do something in between.
#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Process(ProcessError),
}

pub struct ProcessError {
    output: process::Output,
    command: Vec<String>,
}

impl fmt::Debug for ProcessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {}\n{} {:?}\n{} {}\n{} {}\n",
            "process exited with exit code",//.red(),
            self.output.status.to_string(),//.red().bold(),
            "Command:",//.bold(),
            self.command,
            "Stdout:",//.bold(),
            String::from_utf8_lossy(&self.output.stdout),
            "Stderr:",//.bold(),
            String::from_utf8_lossy(&self.output.stderr),
        )
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            Self::Io(e) => e.fmt(f),
            Self::Process(e) => {
                write!(
                    f,
                    "{} {}\n{} {:?}\n{} {}\n{} {}\n",
                    "process exited with exit code",//.red(),
                    e.output.status.to_string(),//.red().bold(),
                    "Command:",//.bold(),
                    e.command,
                    "Stdout:",//.bold(),
                    String::from_utf8_lossy(&e.output.stdout),
                    "Stderr:",//.bold(),
                    String::from_utf8_lossy(&e.output.stderr),
                )
            }
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl std::error::Error for Error {
}

/// Run a git command with the given arguments in the given directory.
pub fn command(cmd: &str, args: &[&str], working_dir: Option<&Path>) -> Result<process::Output, Error> {
    // debug!("{} $ {} {}", working_dir.unwrap_or(Path::new("")).to_string_lossy(), cmd.bold(), args.join(" ").bold());

    let mut command = Command::new(cmd);
    if let Some(working_dir) = working_dir {
        command.current_dir(working_dir);
    }

    let output = command
        .args(args)
        .output()?;

    // debug!("{:?}", output);

    if !output.status.success() {
        return Err(Error::Process(ProcessError {
            output,
            command: std::iter::once(&cmd).chain(args.iter()).map(|&s| s.to_owned()).collect(),
        }));
    }

    Ok(output)
}

pub fn verilator(args: &[&str], working_dir: Option<&Path>) -> Result<process::Output, Error> {
    // `verilator` is just a Perl script that calls `verilator_bin`. It's easier
    // to call it directly rather than deal with Perl on Windows.
    command("verilator_bin", args, working_dir)
}

pub fn cargo(args: &[&str], working_dir: Option<&Path>) -> Result<process::Output, Error> {
    command("cargo", args, working_dir)
}

/// Run a git command with the given arguments in the given directory.
pub fn cotim(args: &[&str], working_dir: Option<&Path>) -> Result<process::Output, Error> {
    // debug!("{} $ {} {}", working_dir.unwrap_or(Path::new("")).to_string_lossy(), "cotim".bold(), args.join(" ").bold());

    let mut command = Command::new(assert_cmd::cargo::cargo_bin!());

    if let Some(working_dir) = working_dir {
        command.current_dir(working_dir);
    }

    let output = command
        .args(args)
        .output()?;

    // debug!("{:?}", output);

    if !output.status.success() {
        return Err(Error::Process(ProcessError {
            output,
            command: std::iter::once(&"cotim_generator").chain(args.iter()).map(|&s| s.to_owned()).collect(),
        }));
    }

    Ok(output)
}
