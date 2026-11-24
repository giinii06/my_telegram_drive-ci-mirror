#![cfg(unix)]
#[path = "../src/http_deadline.rs"]
mod http_deadline;

use std::time::{Duration, Instant};
use http_deadline::remaining;

#[test]
fn deadline_never_resets_after_progress() {
    let start = Instant::now();
    let total = Duration::from_secs(3);
    assert_eq!(remaining(start, start, total), Some(total));
    assert_eq!(remaining(start, start + Duration::from_secs(2), total), Some(Duration::from_secs(1)));
    assert_eq!(remaining(start, start + total, total), None);
    assert_eq!(remaining(start, start + Duration::from_secs(4), total), None);
}
