//! Ordered multipart send orchestration. The journal and transport adapters
//! must enforce durable identity, byte verification, and exclusive ownership.
use crate::{DurableSendKey, PortFuture, TelegramError};
use std::collections::{HashMap, HashSet};
use sha2::{Digest, Sha256};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use tdrive_types::{ChannelId, MessageId};

#[derive(Clone, PartialEq, Eq)]
pub struct UploadStep {
    channel: ChannelId,
    key: DurableSendKey,
    caption: String,
    part: Option<(i64, [u8; 32])>,
}

impl UploadStep {
    pub fn channel(&self) -> ChannelId {
        self.channel
    }

    pub fn key(&self) -> &DurableSendKey {
        &self.key
    }

    pub fn caption(&self) -> &str {
        &self.caption
    }

    pub fn part(&self) -> Option<(i64, [u8; 32])> {
        self.part
    }

    fn fingerprint(&self) -> [u8; 32] {
        let mut hash = Sha256::new();
        hash.update(b"tdrive.upload.step.v1\0");
        hash.update(self.channel.get().to_be_bytes());
        for field in [self.key.operation_id(), self.key.step(), &self.caption] {
            hash.update((field.len() as u64).to_be_bytes());
            hash.update(field.as_bytes());
        }
        match self.part {
            Some((size, digest)) => {
                hash.update([1]);
                hash.update(size.to_be_bytes());
                hash.update(digest);
            }
            None => hash.update([0]),
        }
        hash.finalize().into()
    }

    pub fn is_manifest(&self) -> bool {
        self.part.is_none()
    }
}

pub struct UploadPlan {
    steps: Vec<UploadStep>,
}

impl UploadPlan {
    pub fn fingerprint(&self) -> [u8; 32] {
        let mut hash = Sha256::new();
        hash.update(b"tdrive.upload.plan.v1\0");
        hash.update((self.steps.len() as u64).to_be_bytes());
        for step in &self.steps {
            hash.update(step.fingerprint());
        }
        hash.finalize().into()
    }

    pub fn new(
        channel: ChannelId,
        upload_uuid: &str,
        parent: &str,
        name: &str,
        parts: &[(i64, [u8; 32])],
        total: i64,
        timestamp: i64,
    ) -> Result<Self, TelegramError> {
        if parts.is_empty() || parts.len() > 100_000 {
            return Err(TelegramError::InvalidRequest);
        }
        let sizes: Vec<_> = parts.iter().map(|part| part.0).collect();
        let wire = tdrive_projection::format_plain_multipart(
            upload_uuid, parent, name, &sizes, total, timestamp,
        )
        .map_err(|_| TelegramError::InvalidRequest)?;
        let mut steps = Vec::with_capacity(parts.len() + 1);
        for (index, (caption, part)) in wire.parts.into_iter().zip(parts).enumerate() {
            let key = DurableSendKey::new(upload_uuid.to_owned(), format!("part:{index}"))
                .map_err(|_| TelegramError::InvalidRequest)?;
            steps.push(UploadStep {
                channel,
                key,
                caption,
                part: Some(*part),
            });
        }
        let key = DurableSendKey::new(upload_uuid.to_owned(), "manifest".to_owned())
            .map_err(|_| TelegramError::InvalidRequest)?;
        steps.push(UploadStep {
            channel,
            key,
            caption: wire.manifest,
            part: None,
        });
        Ok(Self { steps })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UploadState {
    New,
    Attempted,
    Confirmed(MessageId),
}

/// Implementations must durably bind the complete plan, including manifest
/// and all expected digests, before any remote send; hold exclusive ownership
/// throughout this call. Lookup is by channel, operation and step key; changed
/// caption, digest, or size must be an error, never a fresh step. begin and
/// confirm must commit atomically before returning success.
pub trait UploadJournal {
    fn bind_plan(&mut self, plan: &UploadPlan) -> Result<(), TelegramError>;
    fn state(&mut self, step: &UploadStep) -> Result<UploadState, TelegramError>;
    fn begin(&mut self, step: &UploadStep) -> Result<(), TelegramError>;
    fn confirm(&mut self, step: &UploadStep, id: MessageId) -> Result<(), TelegramError>;
}

/// Implementations resolve the exact channel, verify staged part bytes against
/// size and SHA-256 before send, and use the supplied stable random_id. A
/// reconciliation receipt must prove the exact operation, channel and bytes.
pub trait MultipartSendPort: Send + Sync {
    fn send_step<'a>(&'a self, step: &'a UploadStep) -> PortFuture<'a, MessageId>;
    fn reconcile_step<'a>(&'a self, step: &'a UploadStep) -> PortFuture<'a, Option<MessageId>>;
}

async fn reconcile<P: MultipartSendPort>(
    port: &P,
    step: &UploadStep,
) -> Result<MessageId, TelegramError> {
    match port.reconcile_step(step).await {
        Ok(Some(id)) => Ok(id),
        Ok(None) | Err(_) => Err(TelegramError::SendOutcomeUnknown),
    }
}

/// A manifest is sent only after every part receipt is durable. An attempted
/// step is never blindly resent, even after process restart. No cleanup of
/// uncertain remote messages is performed. A failed begin must not send.
pub async fn send_multipart<P: MultipartSendPort, J: UploadJournal>(
    port: &P,
    journal: &mut J,
    plan: &UploadPlan,
) -> Result<MessageId, TelegramError> {
    journal.bind_plan(plan)?;
    let mut seen = HashSet::with_capacity(plan.steps.len());
    let mut final_id = None;
    for step in &plan.steps {
        let state = journal.state(step)?;
        let id = match state {
            UploadState::Confirmed(id) => id,
            UploadState::Attempted => reconcile(port, step).await?,
            UploadState::New => {
                journal.begin(step)?;
                match port.send_step(step).await {
                    Ok(id) => id,
                    Err(TelegramError::SendOutcomeUnknown) => reconcile(port, step).await?,
                    Err(error) => return Err(error),
                }
            }
        };
        if !seen.insert(id) {
            return Err(TelegramError::InvalidRequest);
        }
        if !matches!(state, UploadState::Confirmed(_)) {
            journal.confirm(step, id)?;
        }
        final_id = Some(id);
    }
    final_id.ok_or(TelegramError::InvalidRequest)
}


const JOURNAL_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS multipart_upload_runs(
    upload_uuid TEXT PRIMARY KEY NOT NULL,
    channel_id INTEGER NOT NULL CHECK(channel_id > 0),
    fingerprint BLOB NOT NULL CHECK(length(fingerprint) = 32)
);
CREATE TABLE IF NOT EXISTS multipart_upload_steps(
    upload_uuid TEXT NOT NULL REFERENCES multipart_upload_runs(upload_uuid),
    step TEXT NOT NULL,
    message_id INTEGER CHECK(message_id IS NULL OR message_id > 0),
    PRIMARY KEY(upload_uuid, step),
    UNIQUE(upload_uuid, message_id)
);";

struct BoundPlan {
    operation: String,
    channel: ChannelId,
    steps: HashMap<String, [u8; 32]>,
}

/// Separate operator-owned, private on-disk SQLite journal. Do not pass the
/// production projection DB. The caller must serialize execution per upload,
/// keep this DB private and retain it across process restarts.
pub struct SqliteUploadJournal {
    connection: Connection,
    bound: Option<BoundPlan>,
}

fn database_error(_: rusqlite::Error) -> TelegramError {
    TelegramError::TransientTransport
}

impl SqliteUploadJournal {
    pub fn new(connection: Connection) -> Result<Self, TelegramError> {
        if !connection.is_autocommit() {
            return Err(TelegramError::InvalidRequest);
        }
        let mode: String = connection
            .query_row("PRAGMA journal_mode=DELETE", [], |row| row.get(0))
            .map_err(database_error)?;
        if !mode.eq_ignore_ascii_case("delete") {
            return Err(TelegramError::InvalidRequest);
        }
        connection.execute_batch(
            "PRAGMA synchronous=FULL; PRAGMA foreign_keys=ON;"
        ).map_err(database_error)?;
        connection.execute_batch(JOURNAL_SCHEMA).map_err(database_error)?;
        Ok(Self { connection, bound: None })
    }

    fn check_step(&self, step: &UploadStep) -> Result<(), TelegramError> {
        let bound = self.bound.as_ref().ok_or(TelegramError::InvalidRequest)?;
        if bound.channel != step.channel
            || bound.operation != step.key.operation_id()
            || bound.steps.get(step.key.step()) != Some(&step.fingerprint())
        {
            return Err(TelegramError::InvalidRequest);
        }
        Ok(())
    }
}

impl UploadJournal for SqliteUploadJournal {
    fn bind_plan(&mut self, plan: &UploadPlan) -> Result<(), TelegramError> {
        self.bound = None;
        let first = plan.steps.first().ok_or(TelegramError::InvalidRequest)?;
        let uuid = first.key.operation_id();
        let fingerprint = plan.fingerprint();
        let transaction = self.connection.transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        transaction.execute(
            "INSERT INTO multipart_upload_runs(upload_uuid,channel_id,fingerprint)
             VALUES(?1,?2,?3) ON CONFLICT(upload_uuid) DO NOTHING",
            params![uuid, first.channel.get(), &fingerprint[..]],
        ).map_err(database_error)?;
        let (channel, stored): (i64, Vec<u8>) = transaction.query_row(
            "SELECT channel_id,fingerprint FROM multipart_upload_runs WHERE upload_uuid=?1",
            [uuid],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).map_err(database_error)?;
        if channel != first.channel.get() || stored.as_slice() != &fingerprint[..] {
            return Err(TelegramError::InvalidRequest);
        }
        transaction.commit().map_err(database_error)?;
        self.bound = Some(BoundPlan {
            operation: uuid.to_owned(),
            channel: first.channel,
            steps: plan.steps.iter().map(|step| {
                (step.key.step().to_owned(), step.fingerprint())
            }).collect(),
        });
        Ok(())
    }

    fn state(&mut self, step: &UploadStep) -> Result<UploadState, TelegramError> {
        self.check_step(step)?;
        let state: Option<Option<i64>> = self.connection.query_row(
            "SELECT message_id FROM multipart_upload_steps WHERE upload_uuid=?1 AND step=?2",
            params![step.key.operation_id(), step.key.step()],
            |row| row.get(0),
        ).optional().map_err(database_error)?;
        match state {
            None => Ok(UploadState::New),
            Some(None) => Ok(UploadState::Attempted),
            Some(Some(id)) => MessageId::new(id)
                .map(UploadState::Confirmed)
                .map_err(|_| TelegramError::InvalidRequest),
        }
    }

    fn begin(&mut self, step: &UploadStep) -> Result<(), TelegramError> {
        self.check_step(step)?;
        self.connection.execute(
            "INSERT INTO multipart_upload_steps(upload_uuid,step,message_id) VALUES(?1,?2,NULL)",
            params![step.key.operation_id(), step.key.step()],
        ).map_err(database_error)?;
        Ok(())
    }

    fn confirm(&mut self, step: &UploadStep, id: MessageId) -> Result<(), TelegramError> {
        self.check_step(step)?;
        let count = self.connection.execute(
            "UPDATE multipart_upload_steps SET message_id=?3
             WHERE upload_uuid=?1 AND step=?2 AND message_id IS NULL",
            params![step.key.operation_id(), step.key.step(), id.get()],
        ).map_err(database_error)?;
        if count != 1 {
            return Err(TelegramError::InvalidRequest);
        }
        Ok(())
    }
}
