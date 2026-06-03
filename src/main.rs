use std::env;
use std::fs;
use std::process::ExitCode;

use rust_lean_compiler::Session;

fn main() -> ExitCode {
    match run() {
        Ok(lines) => {
            for line in lines {
                println!("{line}");
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

const USAGE: &str = "\
rust-lean-compiler — evaluate Lean-like source files

Usage:
  rust-lean-compiler <file.lean>
  rust-lean-compiler --expr <source>
  rust-lean-compiler --help
  rust-lean-compiler --version

Examples:
  rust-lean-compiler examples/basic.lean
  rust-lean-compiler --expr \"#eval 1 + 1\"";

fn run() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let source = match args.next().as_deref() {
        Some("--help" | "-h") => {
            println!("{USAGE}");
            return Ok(Vec::new());
        }
        Some("--version" | "-V") => {
            println!("rust-lean-compiler {}", env!("CARGO_PKG_VERSION"));
            return Ok(Vec::new());
        }
        Some("--expr") => args.next().ok_or("missing source after --expr")?,
        Some(path) => fs::read_to_string(path)?,
        None => return Err(USAGE.into()),
    };

    let mut session = Session::new();
    Ok(session.run_source(&source)?)
}
