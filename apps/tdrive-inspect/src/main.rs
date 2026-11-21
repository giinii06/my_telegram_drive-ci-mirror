//! Offline contract inspector. Never opens Telegram or production SQLite.
use std::env;
use std::io::{self, Read};
use std::process::ExitCode;

fn main() -> ExitCode {
    match env::args().skip(1).collect::<Vec<_>>().as_slice() {
        [option] if option == "--version" => {
            println!("tdrive-inspect {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        [option] if option == "--wire-version" => {
            println!("{}", tdrive_projection::WIRE_VERSION);
            ExitCode::SUCCESS
        }
        [option] if option == "--check-stdin" => {
            let mut input = String::new();
            if io::stdin().read_to_string(&mut input).is_err() {
                eprintln!("stdin read failed");
                return ExitCode::FAILURE;
            }
            match tdrive_projection::parse_wire(&input) {
                Ok(op) => {
                    println!("valid TDX1 operation: {}", op.op_type.as_str());
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("invalid TDX1: {}", error.as_str());
                    ExitCode::FAILURE
                }
            }
        }
        _ => {
            eprintln!("usage: tdrive-inspect --version | --wire-version | --check-stdin");
            ExitCode::FAILURE
        }
    }
}
