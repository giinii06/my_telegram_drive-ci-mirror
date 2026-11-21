//! Offline inspection command, intentionally NOT a replacement for tdrive CLI.
use std::env;
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
        _ => {
            eprintln!("usage: tdrive-inspect --version | --wire-version");
            ExitCode::FAILURE
        }
    }
}
