//! Native user-account MTProto diagnostic. No Bot API, no Go bridge.
//!
//! This is intentionally independent of the production Go projection engine;
//! never use this CLI to mutate a live Go v16 database or a production vault.
#![cfg(unix)]

use grammers_client::message::InputMessage;
use grammers_client::peer::Peer;
use grammers_client::{Client, SignInError};
use grammers_mtsender::SenderPool;
use grammers_session::storages::SqliteSession;
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tdrive_mtproto::inbox::{CatchupDecision, CatchupWindow, Inbox, InboxEvent, InboxOutcome};
use tdrive_mtproto::policy::{
    CliAction, parse_action, validate_api_credentials, validate_download_path,
    validate_session_path, validate_source,
};

use tdrive_mtproto::verified_download::{ExpectedContent, publish_verified, validate_private_target};

type AnyError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, AnyError>;
const MAX_DIALOGS: usize = 4096;

fn prompt(label: &str) -> Result<String> {
    print!("{label}");
    io::stdout().flush()?;
    let mut line = String::new();
    if io::stdin().read_line(&mut line)? == 0 {
        return Err("interactive input ended".into());
    }
    let text = line.trim().to_owned();
    if text.is_empty() || text.len() > 128 {
        return Err("invalid interactive input".into());
    }
    Ok(text)
}

fn safe_line(value: &str) -> String {
    value.chars().map(|c| if c.is_control() { ' ' } else { c }).collect()
}

fn private_session(path: &Path) -> Result<()> {
    validate_session_path(path)?;
    if fs::symlink_metadata(path).is_err() {
        // The containing directory must be 0700, and new files must be 0600.
        let _file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)?;
    }
    validate_session_path(path)?;
    Ok(())
}

async fn resolve_peer(client: &Client, requested: &str) -> Result<Peer> {
    let wanted = requested.trim_start_matches('@');
    let mut dialogs = client.iter_dialogs();
    for _ in 0..MAX_DIALOGS {
        let Some(dialog) = dialogs.next().await? else { break };
        let peer = dialog.peer();
        let matching_id = peer.id().to_string() == requested;
        let matching_username = peer.username()
            .is_some_and(|value| value.eq_ignore_ascii_case(wanted));
        if matching_id || matching_username {
            return Ok(peer.clone());
        }
    }

    if requested.starts_with('@') {
        return client.resolve_username(wanted).await?
            .ok_or_else(|| "Telegram peer is not reachable".into());
    }
    Err("peer not found in accessible dialogs (use the ID printed by dialogs)".into())
}

/// Bounded Telegram-to-Rust ingestion into an ISOLATED inbox. This does not
/// mutate Go v16 projection tables and is not a complete history subscription.
async fn ingest(client: &Client, name: &str, limit: usize) -> Result<()> {
    let path = env::var_os("TDRIVE_INBOX_PATH")
        .map(PathBuf::from)
        .ok_or("TDRIVE_INBOX_PATH is required for ingest")?;
    let peer = resolve_peer(client, name).await?;
    let peer_kind = match &peer {
        Peer::User(_) => "user",
        Peer::Group(_) => "group",
        Peer::Channel(_) => "channel",
        Peer::Community(_) => "community",
    };
    let peer_key = format!("{peer_kind}:{}", peer.id().bare_id_unchecked());
    let reference = peer.to_ref().await?
        .ok_or("Telegram peer reference not available")?;
    let mut inbox = Inbox::open(&path)?;
    let mut messages = client.iter_messages(reference).limit(limit);

    let (mut inserted, mut seen, mut tampered, mut skipped) = (0_u32, 0_u32, 0_u32, 0_u32);
    while let Some(message) = messages.next().await? {
        // Telegram captions may include user notes after the TDX1 header.
        let header = message.text().split('\n').next().unwrap_or("");
        if !header.starts_with("TDX1|") {
            skipped += 1;
            continue;
        }
        let event = InboxEvent {
            peer_key: &peer_key,
            msg_id: i64::from(message.id()),
            raw_header: header,
        };
        match inbox.append(&event)? {
            InboxOutcome::Inserted => inserted += 1,
            InboxOutcome::AlreadySeen => seen += 1,
            InboxOutcome::TamperRecorded => tampered += 1,
        }
    }
    println!(
        "Native Rust inbox ingest: inserted={inserted} seen={seen} tamper={tampered} skipped={skipped}"
    );
    println!("Bounded page only; this is not complete history/difference sync");
    Ok(())
}

/// Reconcile all messages newer than the persisted watermark. Updates to
/// older Telegram messages (edits/deletes) need a separate difference API.
/// The watermark is deliberately not moved if the history scan is partial.
async fn catchup(client: &Client, name: &str, budget: usize) -> Result<()> {
    let path = env::var_os("TDRIVE_INBOX_PATH")
        .map(PathBuf::from)
        .ok_or("TDRIVE_INBOX_PATH is required for catchup")?;
    let peer = resolve_peer(client, name).await?;
    let peer_kind = match &peer {
        Peer::User(_) => "user",
        Peer::Group(_) => "group",
        Peer::Channel(_) => "channel",
        Peer::Community(_) => "community",
    };
    let peer_key = format!("{peer_kind}:{}", peer.id().bare_id_unchecked());
    let reference = peer.to_ref().await?
        .ok_or("Telegram peer reference not available")?;

    let mut inbox = Inbox::open(&path)?;
    let previous = inbox.high_watermark(&peer_key)?;
    let mut window = CatchupWindow::new(previous, budget)?;
    let mut messages = client.iter_messages(reference).limit(budget + 1);
    let (mut inserted, mut already_seen, mut tampered, mut skipped) =
        (0_usize, 0_usize, 0_usize, 0_usize);

    let mut reached = false;
    while let Some(message) = messages.next().await? {
        match window.observe(i64::from(message.id()))? {
            CatchupDecision::Complete => {
                reached = true;
                break;
            }
            CatchupDecision::Append => {}
        }
        let header = message.text().split('\n').next().unwrap_or("");
        if !header.starts_with("TDX1|") {
            skipped += 1;
            continue;
        }
        match inbox.append(&InboxEvent {
            peer_key: &peer_key,
            msg_id: i64::from(message.id()),
            raw_header: header,
        })? {
            InboxOutcome::Inserted => inserted += 1,
            InboxOutcome::AlreadySeen => already_seen += 1,
            InboxOutcome::TamperRecorded => tampered += 1,
        }
    }
    if !reached {
        // Only a successfully exhausted async Telegram iterator reaches here.
        // Any connection error or exceeded budget exits via ? before this line.
        window.exhaust();
    }
    let newest = window
        .completed_newest()
        .ok_or("catchup did not reach previous position or Telegram history end")?;
    inbox.mark_scanned(&peer_key, previous, newest)?;
    println!(
        "Native Rust catchup complete: previous={previous} newest={newest} \
         scanned={} inserted={inserted} seen={already_seen} tamper={tampered} skipped={skipped}",
        window.processed(),
    );
    Ok(())
}

/// Supervised headless worker: an initial catch-up followed by periodic
/// bounded scans. No Go v16 database writes or public HTTP listener.
///
/// Cancellation is safe: catches can leave already-appended idempotent inbox
/// records, but mark_scanned is only called after the complete interval.
async fn watch(client: &Client, peer: &str, budget: usize, poll_seconds: u64) -> Result<()> {
    use tokio::signal::unix::{SignalKind, signal};
    use tokio::time::{Duration, sleep};

    let mut terminate = signal(SignalKind::terminate())?;
    let mut interrupt = signal(SignalKind::interrupt())?;
    println!("Native MTProto inbox watch active (no public HTTP port)");

    loop {
        tokio::select! {
            result = catchup(client, peer, budget) => result?,
            _ = terminate.recv() => {
                println!("Watch stopped on SIGTERM");
                return Ok(());
            }
            _ = interrupt.recv() => {
                println!("Watch stopped on SIGINT");
                return Ok(());
            }
        }
        tokio::select! {
            _ = sleep(Duration::from_secs(poll_seconds)) => {}
            _ = terminate.recv() => {
                println!("Watch stopped on SIGTERM");
                return Ok(());
            }
            _ = interrupt.recv() => {
                println!("Watch stopped on SIGINT");
                return Ok(());
            }
        }
    }
}

async fn login(client: &Client, api_hash: &str) -> Result<()> {
    if client.is_authorized().await? {
        println!("User account already authorized");
        return Ok(());
    }
    let phone = prompt("Phone (international format): ")?;
    if !phone.starts_with('+')
        || !phone[1..].bytes().all(|c| c.is_ascii_digit())
        || !(8..=18).contains(&phone.len())
    {
        return Err("phone must be an international E.164 number".into());
    }
    let token = client.request_login_code(&phone, api_hash)
        .await.map_err(|_| "Telegram login code request failed")?;
    let code = prompt("Telegram login code: ")?;
    match client.sign_in(&token, &code).await {
        Ok(_) => {}
        Err(SignInError::PasswordRequired(challenge)) => {
            let password = rpassword::prompt_password("Telegram 2FA password: ")?;
            if password.is_empty() {
                return Err("Telegram 2FA password required".into());
            }
            client.check_password(challenge, password.as_bytes())
                .await.map_err(|_| "Telegram 2FA sign-in failed")?;
        }
        Err(_) => return Err("Telegram login verification failed".into()),
    }
    if !client.is_authorized().await? {
        return Err("Telegram authorization not confirmed".into());
    }
    println!("User account authorized; session is stored in the private directory");
    Ok(())
}

async fn list_dialogs(client: &Client) -> Result<()> {
    let mut dialogs = client.iter_dialogs();
    for _ in 0..MAX_DIALOGS {
        let Some(dialog) = dialogs.next().await? else { break };
        let peer = dialog.peer();
        println!("{}\t{}", peer.id(), safe_line(peer.name().unwrap_or("unnamed")));
    }
    Ok(())
}

async fn history(client: &Client, name: &str, limit: usize) -> Result<()> {
    let peer = resolve_peer(client, name).await?;
    let reference = peer.to_ref().await?
        .ok_or("Telegram peer reference not available")?;
    let mut messages = client.iter_messages(reference).limit(limit);
    while let Some(message) = messages.next().await? {
        println!("{}\t{}", message.id(), safe_line(message.text()));
    }
    Ok(())
}

async fn upload(client: &Client, name: &str, path: &Path) -> Result<()> {
    validate_source(path)?;
    let metadata = fs::metadata(path)?;
    if metadata.len() == 0 {
        return Err("refusing zero-byte Telegram upload".into());
    }
    let peer = resolve_peer(client, name).await?;
    let reference = peer.to_ref().await?
        .ok_or("Telegram peer reference not available")?;

    println!("About to transmit a file to Telegram. Type UPLOAD to confirm.");
    let confirm = prompt("Confirmation: ")?;
    if confirm != "UPLOAD" {
        return Err("upload canceled before contacting Telegram file servers".into());
    }
    let file = client.upload_file(path).await
        .map_err(|_| "MTProto upload to Telegram servers failed")?;
    // The final send is non-idempotent here. Never auto-retry ambiguous sends.
    let message = client.send_message(
        reference,
        InputMessage::new().text("").file(file),
    ).await.map_err(|_| "Telegram message send outcome unknown; inspect history before retry")?;
    println!("Telegram message created: {}", message.id());
    Ok(())
}

struct StagedDownload(PathBuf);

impl StagedDownload {
    fn create(target: &Path) -> Result<Self> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let file = target.file_name().ok_or("invalid destination file name")?;
        let sibling = target.with_file_name(
            format!(".tdrive-{}-{now}-{}", std::process::id(), file.to_string_lossy())
        );
        let _file = OpenOptions::new()
            .write(true).create_new(true).mode(0o600).open(&sibling)?;
        Ok(Self(sibling))
    }
}
impl Drop for StagedDownload {
    fn drop(&mut self) { let _ = fs::remove_file(&self.0); }
}

async fn download(client: &Client, name: &str, message_id: i32, target: &Path, expected: Option<ExpectedContent>) -> Result<()> {
    validate_download_path(target)?;
    if expected.is_some() {
        validate_private_target(target)?;
    }
    let peer = resolve_peer(client, name).await?;
    let reference = peer.to_ref().await?
        .ok_or("Telegram peer reference not available")?;
    let found = client.get_messages_by_id(reference, &[message_id]).await
        .map_err(|_| "Telegram message fetch failed")?;
    let message = found.into_iter().next().flatten()
        .ok_or("Telegram message not found")?;
    if message.media().is_none() {
        return Err("message has no downloadable file".into());
    }
    let temporary = StagedDownload::create(target)?;
    let downloaded = message.download_media(&temporary.0).await
        .map_err(|_| "Telegram media download failed")?;
    if !downloaded {
        return Err("message media was unavailable".into());
    }
    // A hard link publishes the already-downloaded file with O_EXCL-like
    // no-clobber semantics. Do not use rename() which replaces existing files.
    if let Some(expected) = expected {
        publish_verified(&temporary.0, target, expected)?;
        println!("Verified download saved");
    } else {
        fs::hard_link(&temporary.0, target)?;
        println!("Downloaded file saved (content digest not checked)");
    }
    Ok(())
}

/// Read only from the explicitly selected Telegram channel. The detached
/// snapshot and trusted digest must describe the same stored object.
/// No projected state is written, and temporary bytes are removed on error.
async fn download_multipart(
    client: &Client,
    name: &str,
    snapshot: &Path,
    channel_id: i64,
    file_message_id: i64,
    target: &Path,
    expected: ExpectedContent,
) -> Result<()> {
    use tdrive_mtproto::multipart_download::{copy_exact_part, validate_manifest};

    validate_private_target(target)?;
    let parts = tdrive_storage::resolve_multipart_snapshot(snapshot, channel_id, file_message_id)?
        .ok_or("no complete active multipart manifest")?;
    validate_manifest(&parts, expected.size())?;

    let peer = resolve_peer(client, name).await?;
    if !matches!(&peer, Peer::Channel(_) | Peer::Community(_))
        || peer.id().bare_id_unchecked().to_string() != channel_id.to_string()
    {
        return Err("Telegram peer does not match the snapshot channel".into());
    }

    let reference = peer.to_ref().await?
        .ok_or("Telegram peer reference not available")?;
    let complete = StagedDownload::create(target)?;
    let mut output = OpenOptions::new().write(true).open(&complete.0)?;
    for part in &parts {
        let message_id = i32::try_from(part.msg_id)?;
        let found = client.get_messages_by_id(reference.clone(), &[message_id]).await
            .map_err(|_| "Telegram part fetch failed")?;
        let message = found.into_iter().next().flatten()
            .ok_or("Telegram part not found")?;
        if message.id() != message_id || message.media().is_none() {
            return Err("Telegram part identity or media mismatch".into());
        }
        let staged = StagedDownload::create(target)?;
        if !message.download_media(&staged.0).await
            .map_err(|_| "Telegram part download failed")?
        {
            return Err("Telegram part unavailable".into());
        }
        let mut input = File::open(&staged.0)?;
        copy_exact_part(&mut input, &mut output, part.size as u64)?;
    }
    output.sync_all()?;
    drop(output);
    publish_verified(&complete.0, target, expected)?;
    println!("Verified multipart download saved");
    Ok(())
}

async fn run() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    let action = parse_action(&args)?;
    if action == CliAction::Help {
        println!("tdrive-mtproto: native user-account MTProto diagnostic");
        println!("Commands: login | status | dialogs | history <@username|dialog-id> <1..100>");
        println!("          upload <@username|dialog-id> <absolute-source>");
        println!("          download <@username|dialog-id> <message-id> <absolute-target>");
        println!("          download-verified <peer> <message-id> <private-absolute-target> <size> <sha256>");
        println!("          download-multipart-verified <peer> <snapshot> <channel-id> <file-message-id> <private-absolute-target> <size> <sha256>");
        println!("          ingest <@username|dialog-id> <1..100> | inbox-stats");
        println!("          catchup <@username|dialog-id> <1..10000>");
        println!("          watch <@username|dialog-id> <1..10000> <15..3600-seconds>");
        println!("Ingest/stats require TDRIVE_INBOX_PATH (private separate SQLite)");
        println!("Environment: TG_API_ID, TG_API_HASH, TDRIVE_SESSION_PATH (private 0700 parent)");
        return Ok(());
    }
    if action == CliAction::InboxStats {
        let path = env::var_os("TDRIVE_INBOX_PATH")
            .map(PathBuf::from)
            .ok_or("TDRIVE_INBOX_PATH is required")?;
        let inbox = Inbox::open_existing(&path)?;
        let stats = inbox.stats()?;
        println!(
            "Native Rust inbox: messages={} tampered={}",
            stats.messages, stats.tampered
        );
        return Ok(());
    }
    let api_id_raw = env::var("TG_API_ID").map_err(|_| "TG_API_ID is required")?;
    let api_hash = env::var("TG_API_HASH").map_err(|_| "TG_API_HASH is required")?;
    let api_id = validate_api_credentials(&api_id_raw, &api_hash)?;
    let session_path = env::var_os("TDRIVE_SESSION_PATH")
        .map(PathBuf::from).ok_or("TDRIVE_SESSION_PATH is required")?;
    private_session(&session_path)?;

    let session = Arc::new(SqliteSession::open(&session_path).await
        .map_err(|_| "Telegram session DB could not be opened")?);
    let SenderPool { runner, handle, .. } = SenderPool::new(Arc::clone(&session), api_id);
    let client = Client::new(handle);
    let runner_task = tokio::spawn(runner.run());

    let result = async {
        if action == CliAction::Login {
            return login(&client, &api_hash).await;
        }
        if !client.is_authorized().await
            .map_err(|_| "Telegram authorization check failed")?
        {
            return Err("user session not authorized; run login".into());
        }
        match action {
            CliAction::Status => {
                println!("User MTProto session authorized");
                Ok(())
            }
            CliAction::Dialogs => list_dialogs(&client).await,
            CliAction::Ingest { peer, limit } => ingest(&client, &peer, limit).await,
            CliAction::Catchup { peer, budget } => catchup(&client, &peer, budget).await,
            CliAction::Watch { peer, budget, poll_seconds } => watch(&client, &peer, budget, poll_seconds).await,
            CliAction::History { peer, limit } => history(&client, &peer, limit).await,
            CliAction::Upload { peer, source } => upload(&client, &peer, &source).await,
            CliAction::Download { peer, message_id, destination } =>
                download(&client, &peer, message_id, &destination, None).await,
            CliAction::DownloadVerified { peer, message_id, destination, expected } =>
                download(&client, &peer, message_id, &destination, Some(expected)).await,
            CliAction::DownloadMultipartVerified {
                peer, snapshot, channel_id, file_message_id, destination, expected,
            } => download_multipart(
                &client, &peer, &snapshot, channel_id, file_message_id, &destination, expected,
            ).await,
            CliAction::Help | CliAction::InboxStats | CliAction::Login => unreachable!(),
        }
    }.await;

    drop(client);
    runner_task.abort();
    result
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("tdrive-mtproto: {error}");
        std::process::exit(1);
    }
}
