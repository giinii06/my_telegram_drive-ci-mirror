use std::time::{Duration, Instant};
use tdrive_mount::{LockDepth, LockRegistry, DavRequestError, parse_lock_token_header};

#[test]
fn positive_single_resource_if_is_checked_before_mutation() {
    let now = Instant::now();
    let mut locks = LockRegistry::new(8);
    let token = locks.create(now, "/folder", LockDepth::Infinity, Duration::from_secs(60)).unwrap();
    let condition = format!("(<{token}>)");
    let mut changes = 0;
    assert!(locks.with_dav_if(now, "/folder/file", false, Some(&condition), || changes += 1).is_ok());
    assert_eq!(changes, 1);
    for invalid in [
        format!("(Not <{token}>)"),
        format!("(<{token}>) (<{token}>)"),
        format!("<http://localhost/folder> (<{token}>)"),
        format!("(<{token}> [\"etag\"])"),
    ] {
        assert_eq!(locks.with_dav_if(now, "/folder/file", false, Some(&invalid), || changes += 1),
                   Err(DavRequestError::UnsupportedCondition));
    }
    assert_eq!(locks.with_dav_if(now, "/other", false, Some(&condition), || changes += 1),
               Err(DavRequestError::PreconditionFailed));
    assert!(locks.with_dav_if(now, "/folder/file", false, None, || changes += 1).is_err());
    assert_eq!(changes, 1);
}

#[test]
fn lock_token_header_and_refresh_unlock_are_path_scoped() {
    let now = Instant::now();
    let mut locks = LockRegistry::new(8);
    let ttl = Duration::from_secs(60);
    let token = locks.create(now, "/a", LockDepth::Zero, ttl).unwrap();
    let header = format!("<{token}>");
    assert_eq!(parse_lock_token_header(&header).unwrap(), token.as_str());
    assert!(parse_lock_token_header(&format!("{header} {header}")).is_err());
    assert!(locks.unlock_dav(now, "/b", &header).is_err());
    assert!(locks.refresh_dav(now, "/a", &format!("({header})"), ttl).is_ok());
    assert!(locks.unlock_dav(now, "/a", &header).is_ok());
    assert!(locks.unlock_dav(now, "/a", &header).is_err());
}
