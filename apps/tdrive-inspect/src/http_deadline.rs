use std::time::{Duration, Instant};

pub fn remaining(start: Instant, now: Instant, total: Duration) -> Option<Duration> {
    let elapsed = now.checked_duration_since(start)?;
    let left = total.checked_sub(elapsed)?;
    if left.is_zero() { None } else { Some(left) }
}
