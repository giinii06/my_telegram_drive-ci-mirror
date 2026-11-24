use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Waker};
use tdrive_telegram::{
    DurableSendKey, InputPeer, PortFuture, RandomId, SendReconciliationPort, TelegramError,
    TelegramWritePort, send_control_reconciled,
};
use tdrive_types::{ChannelId, MessageId};

struct Fake {
    result: Result<MessageId, TelegramError>,
    found: Option<MessageId>,
    sends: AtomicUsize,
    lookups: AtomicUsize,
}

impl TelegramWritePort for Fake {
    fn send_control<'a>(
        &'a self,
        _: InputPeer,
        _: &'a str,
        _: bool,
        _: RandomId,
    ) -> PortFuture<'a, MessageId> {
        self.sends.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move { self.result })
    }
}

impl SendReconciliationPort for Fake {
    fn find_sent_control<'a>(
        &'a self,
        _: InputPeer,
        _: &'a DurableSendKey,
    ) -> PortFuture<'a, Option<MessageId>> {
        self.lookups.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move { Ok(self.found) })
    }
}

fn ready<T>(future: impl Future<Output = T>) -> T {
    let mut future = std::pin::pin!(future);
    match future.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("fake unexpectedly pending"),
    }
}

fn peer() -> InputPeer {
    InputPeer {
        channel_id: ChannelId::new(42).unwrap(),
        access_hash: 1,
    }
}

#[test]
fn ambiguous_send_reconciles_without_resending() {
    let key = DurableSendKey::new("upload-1".into(), "manifest".into()).unwrap();
    let message = MessageId::new(10).unwrap();
    for found in [Some(message), None] {
        let fake = Fake {
            result: Err(TelegramError::SendOutcomeUnknown),
            found,
            sends: AtomicUsize::new(0),
            lookups: AtomicUsize::new(0),
        };
        let result = ready(send_control_reconciled(&fake, peer(), "TDX1", true, &key));
        assert_eq!(result, found.ok_or(TelegramError::SendOutcomeUnknown));
        assert_eq!(fake.sends.load(Ordering::SeqCst), 1);
        assert_eq!(fake.lookups.load(Ordering::SeqCst), 1);
    }
}

#[test]
fn ordinary_success_or_denial_does_not_reconcile() {
    let key = DurableSendKey::new("upload-1".into(), "manifest".into()).unwrap();
    for result in [
        Ok(MessageId::new(10).unwrap()),
        Err(TelegramError::Unauthorized),
    ] {
        let fake = Fake {
            result,
            found: None,
            sends: AtomicUsize::new(0),
            lookups: AtomicUsize::new(0),
        };
        assert_eq!(
            ready(send_control_reconciled(&fake, peer(), "TDX1", true, &key)),
            result
        );
        assert_eq!(fake.sends.load(Ordering::SeqCst), 1);
        assert_eq!(fake.lookups.load(Ordering::SeqCst), 0);
    }
}
