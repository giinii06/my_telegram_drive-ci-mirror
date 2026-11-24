//! Ordered multipart send orchestration. The journal and transport adapters
//! must enforce durable identity, byte verification, and exclusive ownership.
use crate::{DurableSendKey, PortFuture, TelegramError};
use std::collections::HashSet;
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

    pub fn is_manifest(&self) -> bool {
        self.part.is_none()
    }
}

pub struct UploadPlan {
    steps: Vec<UploadStep>,
}

impl UploadPlan {
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
