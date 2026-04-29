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

fn run() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let source = match args.next().as_deref() {
        Some("--expr") => args.next().ok_or("missing source after --expr")?,
        Some(path) => fs::read_to_string(path)?,
        None => {
            return Err("usage: rust-lean-compiler <file.lean> | --expr <source>".into());
        }
    };

    let mut session = Session::new();
    Ok(session.run_source(&source)?)
}
