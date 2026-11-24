#[path = "../src/http_contract.rs"]
mod http_contract;

use http_contract::{HttpError, parse_request, valid_token};

const KEY: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEF";

#[test]
fn missing_duplicate_and_invalid_host_are_rejected() {
    assert!(valid_token(KEY));
    let secret = std::str::from_utf8(KEY).unwrap();
    for host in ["", "Host: localhost\r\nHost: other\r\n", "Host: \r\n"] {
        let raw = format!("GET /health HTTP/1.1\r\n{host}Authorization: Bearer {secret}\r\n\r\n");
        assert_eq!(parse_request(raw.as_bytes(), KEY), Err(HttpError::BadRequest));
    }
}

#[test]
fn control_bytes_and_invalid_expected_token_are_rejected() {
    let secret = std::str::from_utf8(KEY).unwrap();
    for header in ["X-Test: x\ny", "X-Test: x\0y", "X-Test: x\ry"] {
        let raw = format!("GET /health HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {secret}\r\n{header}\r\n\r\n");
        assert_eq!(parse_request(raw.as_bytes(), KEY), Err(HttpError::BadRequest));
    }
    let raw = b"GET /health HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer short\r\n\r\n";
    assert_eq!(parse_request(raw, b"short"), Err(HttpError::Unauthorized));
}
