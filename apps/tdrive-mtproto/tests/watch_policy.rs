#![cfg(unix)]
//! Offline contract for unattended user-MTProto polling.
use tdrive_mtproto::policy::{parse_action, CliAction, PolicyError};

fn argv(parts: &[&str]) -> Vec<String> { parts.iter().map(|p| (*p).to_owned()).collect() }

#[test]
fn watch_uses_explicit_peer_bounded_history_and_sane_poll_interval() {
    assert_eq!(
        parse_action(&argv(&["watch", "@telegram_channel", "10000", "3600"])),
        Ok(CliAction::Watch {
            peer: "@telegram_channel".to_owned(),
            budget: 10000,
            poll_seconds: 3600,
        })
    );
    assert_eq!(
        parse_action(&argv(&["watch", "channel:12345", "1", "15"])),
        Ok(CliAction::Watch {
            peer: "channel:12345".to_owned(),
            budget: 1,
            poll_seconds: 15,
        })
    );
}

#[test]
fn watch_rejects_unbounded_or_busy_polling_and_invalid_peers() {
    for invalid in [
        ["watch", "@telegram_channel", "0", "60"],
        ["watch", "@telegram_channel", "10001", "60"],
        ["watch", "@telegram_channel", "100", "0"],
        ["watch", "@telegram_channel", "100", "14"],
        ["watch", "@telegram_channel", "100", "3601"],
        ["watch", "@telegram_channel", "100", "-60"],
        ["watch", "a/b", "100", "60"],
    ] {
        assert_eq!(parse_action(&argv(&invalid)), Err(PolicyError::InvalidArguments));
    }
    for invalid in [
        vec!["watch", "@telegram_channel", "100"],
        vec!["watch", "@telegram_channel", "100", "60", "password"],
    ] {
        assert_eq!(parse_action(&argv(&invalid)), Err(PolicyError::InvalidArguments));
    }
}
