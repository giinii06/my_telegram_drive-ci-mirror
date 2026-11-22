//! Offline contract inspector. Never opens Telegram or production services.
mod folder_json;
#[cfg(unix)]
mod http_contract;
#[cfg(unix)]
mod offline_crypto;
#[cfg(unix)]
mod offline_http;

use std::env;
use std::io::{self, Read};
use std::path::Path;
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
        [option, path] if option == "--sqlite-check" => {
            // Explicitly supplied detached snapshot only. No DB is created.
            match tdrive_storage::inspect_snapshot(Path::new(path)) {
                Ok(report) => {
                    println!("Go SQLite v{} snapshot verified", report.schema_version);
                    for (table, count) in report.table_counts {
                        println!("{table}: {count}");
                    }
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("snapshot invalid or unsafe: {error}");
                    ExitCode::FAILURE
                }
            }
        }
        #[cfg(unix)]
        [option, source, destination, key] if option == "--encrypt-file" => {
            offline_file(offline_crypto::Mode::Encrypt, source, destination, key)
        }
        #[cfg(unix)]
        [option, source, destination, key] if option == "--decrypt-file" => {
            offline_file(offline_crypto::Mode::Decrypt, source, destination, key)
        }
        [option, path, channel, parent] if option == "--list-folder" => {
            list_folder(path, channel, parent)
        }
        #[cfg(unix)]
        [option, snapshot, token, port] if option == "--serve-readonly" => {
            match port.parse::<u16>() {
                Ok(port) if port != 0 => {
                    match offline_http::serve(Path::new(snapshot), Path::new(token), port) {
                        Ok(()) => ExitCode::SUCCESS,
                        Err(message) => {
                            eprintln!("read-only API failed: {message}");
                            ExitCode::FAILURE
                        }
                    }
                }
                _ => {
                    eprintln!("invalid loopback port");
                    ExitCode::FAILURE
                }
            }
        }
        _ => {
            eprintln!(
                "usage: tdrive-inspect --version | --wire-version | --check-stdin | --sqlite-check <detached-snapshot.sqlite> | --encrypt-file <src> <dst> <private-key-file> | --decrypt-file <src> <dst> <private-key-file> | --list-folder <detached-snapshot.sqlite> <channel-id> <parent-id> | --serve-readonly <detached-snapshot.sqlite> <private-token-file> <loopback-port>"
            );
            ExitCode::FAILURE
        }
    }
}

#[cfg(unix)]
fn offline_file(mode: offline_crypto::Mode, source: &str, destination: &str, key: &str) -> ExitCode {
    match offline_crypto::run(mode, Path::new(source), Path::new(destination), Path::new(key)) {
        Ok(()) => {
            println!("TDE1 offline file created");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("TDE1 offline file failed: {message}");
            ExitCode::FAILURE
        }
    }
}

fn list_folder(path: &str, channel: &str, parent: &str) -> ExitCode {
    let valid_channel = !channel.is_empty() && channel.bytes().all(|byte| byte.is_ascii_digit());
    let parsed = valid_channel
        .then(|| channel.parse::<i64>().ok())
        .flatten()
        .filter(|value| *value > 0);
    let Some(channel_id) = parsed else {
        eprintln!("invalid positive decimal channel ID");
        return ExitCode::FAILURE;
    };
    let path = Path::new(path);
    if !path.is_absolute() || !(parent.is_empty() || parent.starts_with("d:")) {
        eprintln!("invalid detached SQLite snapshot path or folder ID");
        return ExitCode::FAILURE;
    }
    match tdrive_storage::list_folder_contents_snapshot(path, channel_id, parent) {
        Ok(listing) => {
            println!("{}", folder_json::encode_json(&listing));
            ExitCode::SUCCESS
        }
        Err(_) => {
            eprintln!("detached SQLite snapshot is invalid or unsafe");
            ExitCode::FAILURE
        }
    }
}
