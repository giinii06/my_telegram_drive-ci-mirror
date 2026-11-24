//! Offline contract inspector. Never opens Telegram or production services.
mod folder_json;
mod offline_media;
mod parts_json;
#[cfg(unix)]
mod http_contract;
#[cfg(unix)]
mod http_deadline;
#[cfg(unix)]
mod offline_crypto;
#[cfg(unix)]
mod offline_upload;
#[cfg(unix)]
mod offline_assembly;
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
        [option, source, range] if option == "--read-media-range" => {
            match offline_media::read_range(Path::new(source), range, &mut io::stdout().lock()) {
                Ok(()) => ExitCode::SUCCESS,
                Err(_) => {
                    eprintln!("media range read rejected or incomplete");
                    ExitCode::FAILURE
                }
            }
        }
        #[cfg(unix)]
        [option, source, destination, size] if option == "--prepare-upload" => {
            match offline_upload::run(Path::new(source), Path::new(destination), size) {
                Ok(()) => {
                    println!("local upload parts prepared");
                    ExitCode::SUCCESS
                }
                Err(_) => {
                    eprintln!("upload preparation failed");
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
        #[cfg(unix)]
        [option, source, destination] if option == "--create-backup" => {
            backup_command(BackupAction::Create, source, Some(destination))
        }
        #[cfg(unix)]
        [option, source] if option == "--verify-backup" => {
            backup_command(BackupAction::Verify, source, None)
        }
        #[cfg(unix)]
        [option, source, destination] if option == "--restore-clone" => {
            backup_command(BackupAction::Restore, source, Some(destination))
        }
        [option, baseline, candidate] if option == "--compare-snapshots" => {
            compare_snapshots(Path::new(baseline), Path::new(candidate))
        }
        #[cfg(unix)]
        [option, snapshot, channel, message, parts, target, digest] if option == "--assemble-parts" => {
            match offline_assembly::run(
                Path::new(snapshot), channel, message, Path::new(parts), Path::new(target), digest,
            ) {
                Ok(()) => {
                    println!("verified multipart file created");
                    ExitCode::SUCCESS
                }
                Err(_) => {
                    eprintln!("multipart assembly failed");
                    ExitCode::FAILURE
                }
            }
        }
        [option, path, channel, message] if option == "--list-parts" => {
            list_parts(path, channel, message)
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
                "usage: tdrive-inspect --version | --wire-version | --check-stdin | --read-media-range <absolute-local-file> <range> | --prepare-upload <absolute-source> <new-private-parts-directory> <part-size> | --sqlite-check <detached-snapshot.sqlite> | --compare-snapshots <baseline.sqlite> <candidate.sqlite> | --create-backup <source.sqlite> <new-backup.sqlite> | --verify-backup <backup.sqlite> | --restore-clone <backup.sqlite> <new-clone.sqlite> | --encrypt-file <src> <dst> <private-key-file> | --decrypt-file <src> <dst> <private-key-file> | --list-folder <detached-snapshot.sqlite> <channel-id> <parent-id> | --list-parts <detached-snapshot.sqlite> <channel-id> <file-message-id> | --assemble-parts <snapshot> <channel-id> <file-message-id> <private-parts-dir> <private-target> <sha256> | --serve-readonly <detached-snapshot.sqlite> <private-token-file> <loopback-port>"
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

fn compare_snapshots(baseline: &Path, candidate: &Path) -> ExitCode {
    if !baseline.is_absolute() || !candidate.is_absolute() {
        eprintln!("snapshot comparison failed: absolute detached paths required");
        return ExitCode::FAILURE;
    }
    let (Ok(left), Ok(right)) = (
        tdrive_storage::snapshot_table_digests(baseline),
        tdrive_storage::snapshot_table_digests(candidate),
    ) else {
        eprintln!("snapshot comparison failed: invalid or unsafe detached snapshot");
        return ExitCode::FAILURE;
    };
    if left == right {
        println!("core tables match: {}", left.len());
        return ExitCode::SUCCESS;
    }
    for table in &left {
        if !right.iter().any(|other| other == table) {
            eprintln!("core table differs: {}", table.table);
        }
    }
    eprintln!("snapshot comparison failed: core table mismatch");
    ExitCode::FAILURE
}

fn list_parts(path: &str, channel: &str, message: &str) -> ExitCode {
    fn positive_decimal(value: &str) -> Option<i64> {
        if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        value.parse::<i64>().ok().filter(|id| *id > 0)
    }

    let (Some(channel_id), Some(message_id)) =
        (positive_decimal(channel), positive_decimal(message))
    else {
        eprintln!("invalid positive decimal channel or message ID");
        return ExitCode::FAILURE;
    };
    let snapshot = Path::new(path);
    if !snapshot.is_absolute() {
        eprintln!("absolute detached SQLite snapshot path required");
        return ExitCode::FAILURE;
    }
    match tdrive_storage::resolve_multipart_snapshot(snapshot, channel_id, message_id) {
        Ok(Some(parts)) => {
            println!("{}", parts_json::encode_json(&parts));
            ExitCode::SUCCESS
        }
        Ok(None) => {
            eprintln!("no multipart manifest");
            ExitCode::FAILURE
        }
        Err(_) => {
            eprintln!("multipart snapshot is incomplete or unsafe");
            ExitCode::FAILURE
        }
    }
}

#[cfg(unix)]
enum BackupAction {
    Create,
    Verify,
    Restore,
}

#[cfg(unix)]
fn backup_command(action: BackupAction, source: &str, destination: Option<&str>) -> ExitCode {
    use std::path::Component;
    fn absolute_simple(path: &Path) -> bool {
        path.is_absolute()
            && !path.components().any(|component| {
                matches!(component, Component::ParentDir | Component::CurDir)
            })
    }

    let source = Path::new(source);
    let destination = destination.map(Path::new);
    if !absolute_simple(source) || destination.is_some_and(|path| !absolute_simple(path)) {
        eprintln!("backup and clone paths must be absolute");
        return ExitCode::FAILURE;
    }

    let result = match (action, destination) {
        (BackupAction::Create, Some(target)) => {
            // This operator diagnostic accepts detached snapshots only.
            // A production online backup requires the daemon's ownership policy.
            tdrive_storage::inspect_snapshot(source).and_then(|_| {
                tdrive_storage::create_consistent_snapshot(source, target).map(|_| ())
            })
        }
        (BackupAction::Verify, None) => tdrive_storage::verify_snapshot(source).map(|_| ()),
        (BackupAction::Restore, Some(target)) => {
            tdrive_storage::restore_verified_clone(source, target).map(|_| ())
        }
        _ => {
            eprintln!("invalid backup command");
            return ExitCode::FAILURE;
        }
    };

    match result {
        Ok(()) => {
            println!("backup operation verified");
            ExitCode::SUCCESS
        }
        Err(_) => {
            eprintln!("backup operation rejected");
            ExitCode::FAILURE
        }
    }
}
