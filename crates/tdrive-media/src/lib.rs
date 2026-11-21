//! HTTP Range decision logic compatible with the existing Go media server.
//! No HTTP listener, Telegram read, cache or HLS decoder is created here.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RangeDecision {
    pub start: i64,
    pub end: i64,
    pub partial: bool,
    pub ok: bool,
}

impl RangeDecision {
    const fn absent() -> Self {
        Self {
            start: 0,
            end: 0,
            partial: false,
            ok: true,
        }
    }

    const fn invalid() -> Self {
        Self {
            start: 0,
            end: 0,
            partial: true,
            ok: false,
        }
    }

    const fn selected(start: i64, end: i64) -> Self {
        Self {
            start,
            end,
            partial: true,
            ok: true,
        }
    }

    /// The caller sends HTTP 206 and Content-Range for partial valid ranges,
    /// HTTP 416 with bytes */size for invalid ranges, or HTTP 200 otherwise.
    pub fn response_code(self) -> u16 {
        if !self.ok {
            416
        } else if self.partial {
            206
        } else {
            200
        }
    }

    pub fn length(self, file_size: i64) -> Option<i64> {
        if !self.ok {
            return None;
        }
        if !self.partial {
            return Some(file_size.max(0));
        }
        Some(self.end - self.start + 1)
    }

    pub fn content_range(self, file_size: i64) -> Option<String> {
        if !self.ok {
            return Some(format!("bytes */{file_size}"));
        }
        if self.partial {
            return Some(format!("bytes {}-{}/{file_size}", self.start, self.end));
        }
        None
    }
}

/// Direct translation of backend/media/server.go parseRangeHeader.
/// This intentionally rejects multiple ranges rather than concatenating them.
pub fn parse_range_header(raw: &str, size: i64) -> RangeDecision {
    if raw.is_empty() {
        return RangeDecision::absent();
    }
    if size <= 0 || !raw.starts_with("bytes=") || raw.contains(',') {
        return RangeDecision::invalid();
    }
    let spec = raw["bytes=".len()..].trim();
    let Some((left, right)) = spec.split_once('-') else {
        return RangeDecision::invalid();
    };
    if left.is_empty() {
        let Ok(mut suffix_len) = right.parse::<i64>() else {
            return RangeDecision::invalid();
        };
        if suffix_len <= 0 {
            return RangeDecision::invalid();
        }
        if suffix_len > size {
            suffix_len = size;
        }
        return RangeDecision::selected(size - suffix_len, size - 1);
    }
    if right.is_empty() {
        let Ok(start) = left.parse::<i64>() else {
            return RangeDecision::invalid();
        };
        if start < 0 || start >= size {
            return RangeDecision::invalid();
        }
        return RangeDecision::selected(start, size - 1);
    }
    let (Ok(start), Ok(mut end)) = (left.parse::<i64>(), right.parse::<i64>()) else {
        return RangeDecision::invalid();
    };
    if start < 0 || end < start || start >= size {
        return RangeDecision::invalid();
    }
    if end >= size {
        end = size - 1;
    }
    RangeDecision::selected(start, end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statuses_and_lengths_remain_consistent() {
        let whole = parse_range_header("", 2048);
        assert_eq!(whole.response_code(), 200);
        assert_eq!(whole.length(2048), Some(2048));
        assert_eq!(whole.content_range(2048), None);

        let first = parse_range_header("bytes=0-1", 2048);
        assert_eq!(first.response_code(), 206);
        assert_eq!(first.length(2048), Some(2));
        assert_eq!(first.content_range(2048).as_deref(), Some("bytes 0-1/2048"));

        let bad = parse_range_header("bytes=2048-", 2048);
        assert_eq!(bad.response_code(), 416);
        assert_eq!(bad.length(2048), None);
        assert_eq!(bad.content_range(2048).as_deref(), Some("bytes */2048"));
    }

    #[test]
    fn overflow_and_multiple_ranges_are_rejected() {
        for raw in [
            "bytes=9223372036854775808-",
            "bytes=0-18446744073709551616",
            "bytes=0-1,2-3",
        ] {
            assert!(!parse_range_header(raw, 4096).ok);
        }
    }
}
