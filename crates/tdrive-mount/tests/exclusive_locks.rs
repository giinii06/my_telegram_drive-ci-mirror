use std::time::{Duration, Instant};
use tdrive_mount::{LockDepth, LockError, LockRegistry};

#[test]
fn ancestor_locks_block_children_but_not_similarly_named_siblings() {
    let now = Instant::now();
    let ttl = Duration::from_secs(60);
    let mut locks = LockRegistry::new(4);
    let token = locks.create(now, "/docs", LockDepth::Infinity, ttl).unwrap();
    assert!(token.starts_with("opaquelocktoken:"));
    assert_eq!(token.len(), 48);
    assert!(matches!(
        locks.create(now, "/docs/a", LockDepth::Zero, ttl),
        Err(LockError::Conflict)
    ));
    assert!(locks.blocks_mutation(now, "/docs/a", false, &[]).unwrap());
    assert!(!locks.blocks_mutation(now, "/docs/a", false, &[&token]).unwrap());
    assert!(!locks.blocks_mutation(now, "/docs2", false, &[]).unwrap());
    assert!(locks.blocks_mutation(now, "/", true, &[]).unwrap());
}

#[test]
fn expiry_capacity_and_path_bound_refresh_unlock() {
    let now = Instant::now();
    let ttl = Duration::from_secs(10);
    let mut locks = LockRegistry::new(1);
    let token = locks.create(now, "/a", LockDepth::Zero, ttl).unwrap();
    assert!(matches!(
        locks.create(now, "/b", LockDepth::Zero, ttl),
        Err(LockError::Capacity)
    ));
    assert!(locks.refresh(now, "/b", &token, ttl).is_err());
    assert!(locks.unlock(now, "/b", &token).is_err());
    locks.refresh(now + Duration::from_secs(5), "/a", &token, ttl).unwrap();
    assert!(locks.blocks_mutation(now + ttl, "/a", false, &[]).unwrap());
    assert!(!locks.blocks_mutation(now + Duration::from_secs(15), "/a", false, &[]).unwrap());
    let second = locks.create(now + Duration::from_secs(15), "/b", LockDepth::Zero, ttl).unwrap();
    assert_ne!(second, token);
    locks.unlock(now + Duration::from_secs(15), "/b", &second).unwrap();
    assert!(!locks.blocks_mutation(now + Duration::from_secs(15), "/b", false, &[]).unwrap());
}

#[test]
fn zero_depth_does_not_cover_child_and_invalid_paths_fail_closed() {
    let now = Instant::now();
    let mut locks = LockRegistry::new(8);
    let ttl = Duration::from_secs(30);
    locks.create(now, "/parent", LockDepth::Zero, ttl).unwrap();
    assert!(locks.create(now, "/parent/child", LockDepth::Zero, ttl).is_ok());
    assert!(locks.blocks_mutation(now, "/parent", true, &[]).unwrap());
    for path in ["relative", "/a/../b", "/a//b", "/a/", "/a\\b"] {
        assert!(matches!(locks.blocks_mutation(now, path, false, &[]), Err(LockError::InvalidPath)));
    }
    assert!(matches!(locks.create(now, "/b", LockDepth::Zero, Duration::ZERO), Err(LockError::InvalidDuration)));
}
