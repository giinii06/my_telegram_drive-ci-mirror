use std::task::{Context, Poll, Waker};
use tdrive_telegram::*;
use tdrive_types::{ChannelId, MessageId};

#[derive(Default)]
struct Journal {
    states: Vec<(UploadStep, UploadState)>,
}
impl UploadJournal for Journal {
    fn state(&mut self, step: &UploadStep) -> Result<UploadState, TelegramError> {
        Ok(self.states.iter().find(|(key, _)| key == step)
            .map_or(UploadState::New, |(_, state)| *state))
    }
    fn begin(&mut self, step: &UploadStep) -> Result<(), TelegramError> {
        assert_eq!(self.state(step)?, UploadState::New);
        self.states.push((step.clone(), UploadState::Attempted));
        Ok(())
    }
    fn confirm(&mut self, step: &UploadStep, id: MessageId) -> Result<(), TelegramError> {
        let entry = self.states.iter_mut().find(|(key, _)| key == step).unwrap();
        assert_eq!(entry.1, UploadState::Attempted);
        entry.1 = UploadState::Confirmed(id);
        Ok(())
    }
}

#[derive(Default)]
struct Port {
    sent: std::sync::Mutex<Vec<UploadStep>>,
    ambiguous: std::sync::atomic::AtomicBool,
    found: std::sync::atomic::AtomicBool,
}
impl MultipartSendPort for Port {
    fn send_step<'a>(&'a self, step: &'a UploadStep) -> PortFuture<'a, MessageId> {
        Box::pin(async move {
            let mut sent = self.sent.lock().unwrap();
            sent.push(step.clone());
            if self.ambiguous.load(std::sync::atomic::Ordering::SeqCst) {
                return Err(TelegramError::SendOutcomeUnknown);
            }
            Ok(MessageId::new(sent.len() as i64).unwrap())
        })
    }
    fn reconcile_step<'a>(&'a self, _step: &'a UploadStep) -> PortFuture<'a, Option<MessageId>> {
        Box::pin(async move {
            Ok(self.found.load(std::sync::atomic::Ordering::SeqCst)
                .then(|| MessageId::new(1).unwrap()))
        })
    }
}

fn run<T>(future: impl std::future::Future<Output = T>) -> T {
    let mut future = std::pin::pin!(future);
    match future.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("fake port must not block"),
    }
}

fn plan() -> UploadPlan {
    UploadPlan::new(
        ChannelId::new(7).unwrap(), "upload-1", "", "file",
        &[(2, [1; 32]), (3, [2; 32])], 5, 100,
    ).unwrap()
}

#[test]
fn manifest_is_last_and_confirmed_steps_are_not_resent() {
    let port = Port::default();
    let mut journal = Journal::default();
    let plan = plan();
    let first = run(send_multipart(&port, &mut journal, &plan)).unwrap();
    assert_eq!(first.get(), 3);
    let sent = port.sent.lock().unwrap();
    assert_eq!(sent.len(), 3);
    assert_eq!(sent[0].key().step(), "part:0");
    assert_eq!(sent[1].key().step(), "part:1");
    assert!(sent[2].is_manifest());
    drop(sent);
    assert_eq!(run(send_multipart(&port, &mut journal, &plan)).unwrap(), first);
    assert_eq!(port.sent.lock().unwrap().len(), 3);
}

#[test]
fn unresolved_send_blocks_manifest_and_restart_does_not_resend() {
    use std::sync::atomic::Ordering::SeqCst;
    let port = Port::default();
    let mut journal = Journal::default();
    let plan = plan();
    port.ambiguous.store(true, SeqCst);
    assert_eq!(run(send_multipart(&port, &mut journal, &plan)), Err(TelegramError::SendOutcomeUnknown));
    assert_eq!(port.sent.lock().unwrap().len(), 1);
    port.ambiguous.store(false, SeqCst);
    assert_eq!(run(send_multipart(&port, &mut journal, &plan)), Err(TelegramError::SendOutcomeUnknown));
    assert_eq!(port.sent.lock().unwrap().len(), 1);
    port.found.store(true, SeqCst);
    assert!(run(send_multipart(&port, &mut journal, &plan)).is_ok());
    assert_eq!(port.sent.lock().unwrap().len(), 3);
}
