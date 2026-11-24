use std::time::{Duration, Instant};
use tdrive_mount::{LockDepth, LockRegistry, LockError};

#[test]
fn move_checks_both_paths_before_running_mutation() {
    let now = Instant::now();
    let mut registry = LockRegistry::new(8);
    let ttl = Duration::from_secs(60);
    let source = registry.create(now, "/src/child", LockDepth::Zero, ttl).unwrap();
    let target = registry.create(now, "/dst", LockDepth::Infinity, ttl).unwrap();
    let mut calls = 0;
    for tokens in [vec![], vec![source.as_str()], vec![target.as_str()]] {
        let result = registry.with_mutation(now, &[("/src", true), ("/dst", true)], &tokens, || { calls += 1; });
        assert_eq!(result, Err(LockError::Conflict));
    }
    assert_eq!(calls, 0);
    let result = registry.with_mutation(now, &[("/src", true), ("/dst", true)], &[&source, &target], || { calls += 1; 42 });
    assert_eq!(result, Ok(42));
    assert_eq!(calls, 1);
}

#[test]
fn malformed_or_empty_targets_do_not_run_callback() {
    let now = Instant::now();
    let mut registry = LockRegistry::new(8);
    assert!(registry.with_mutation(now, &[], &[], || panic!("empty mutation")).is_err());
    assert!(registry.with_mutation(now, &[("/a", false), ("/../b", false)], &[], || panic!("invalid mutation")).is_err());
}
