use std::time::Duration;
use tdrive_mount::{LockDepth, parse_lock_depth, parse_lock_timeout};

#[test]
fn depth_defaults_to_infinity_and_rejects_one() {
    assert_eq!(parse_lock_depth(""), Some(LockDepth::Infinity));
    assert_eq!(parse_lock_depth("0"), Some(LockDepth::Zero));
    assert_eq!(parse_lock_depth("1"), None);
    assert_eq!(parse_lock_depth("Infinity"), None);
}

#[test]
fn timeout_matches_go_defaults_and_bounds() {
    for (header, seconds) in [("", 600), ("Infinite", 3600), ("Second-9000", 3600), ("bad, Second-15", 15)] {
        assert_eq!(parse_lock_timeout(header), Some(Duration::from_secs(seconds)));
    }
    for header in ["Second-0", "Second--1", "Second-4294967296", "Second-+1", "infinite"] {
        assert_eq!(parse_lock_timeout(header), None);
    }
}
