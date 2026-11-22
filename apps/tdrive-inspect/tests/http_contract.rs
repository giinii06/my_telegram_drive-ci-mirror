// Test-only contract for loopback HTTP, no server or Telegram account.
#[path = "../src/http_contract.rs"]
mod http_contract;

use http_contract::{ApiRoute, HttpError, parse_request, valid_token};

const SECRET: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEF";

fn request(method: &str, path: &str, credentials: &str) -> Vec<u8> {
    format!("{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Bearer {credentials}\r\n\r\n").into_bytes()
}

#[test]
fn valid_token_is_private_header_safe_ascii() {
    assert!(valid_token(SECRET));
    assert!(!valid_token(b"short"));
    assert!(!valid_token(&[b'a'; 129]));
    assert!(!valid_token(b"abcdefghijklmnopqrstuvwxyzABCD\r\n"));
}

#[test]
fn authorized_health_and_precise_channel_folder_ids() {
    let secret = std::str::from_utf8(SECRET).unwrap();
    assert_eq!(
        parse_request(&request("GET", "/health", secret), SECRET).unwrap(),
        ApiRoute::Health
    );
    assert_eq!(
        parse_request(
            &request("GET", "/v1/folders?channel=9007199254740993&parent=d%3Adocs", secret),
            SECRET
        ).unwrap(),
        ApiRoute::Folder { channel_id: 9_007_199_254_740_993, parent: "d:docs".into() }
    );
}

#[test]
fn missing_wrong_or_duplicate_authorization_is_rejected() {
    let secret = std::str::from_utf8(SECRET).unwrap();
    assert_eq!(
        parse_request(b"GET /health HTTP/1.1\r\nHost: localhost\r\n\r\n", SECRET),
        Err(HttpError::Unauthorized)
    );
    assert_eq!(
        parse_request(&request("GET", "/health", "wrong"), SECRET),
        Err(HttpError::Unauthorized)
    );
    let mut bytes = request("GET", "/health", secret);
    bytes.truncate(bytes.len() - 2);
    bytes.extend_from_slice(format!("Authorization: Bearer {secret}\r\n\r\n").as_bytes());
    assert_eq!(parse_request(&bytes, SECRET), Err(HttpError::BadRequest));
}

#[test]
fn no_write_methods_pipelining_or_unbounded_headers() {
    let secret = std::str::from_utf8(SECRET).unwrap();
    assert_eq!(
        parse_request(&request("POST", "/v1/folders?channel=1", secret), SECRET),
        Err(HttpError::MethodNotAllowed)
    );
    let mut bytes = request("GET", "/health", secret);
    bytes.extend_from_slice(b"GET /health HTTP/1.1\r\n\r\n");
    assert_eq!(parse_request(&bytes, SECRET), Err(HttpError::BadRequest));
    let oversized = vec![b'A'; 8193];
    assert_eq!(parse_request(&oversized, SECRET), Err(HttpError::TooLarge));
}

#[test]
fn malformed_folder_ids_channels_and_unknown_routes_never_resolve() {
    let secret = std::str::from_utf8(SECRET).unwrap();
    for url in [
        "/v1/folders?channel=0",
        "/v1/folders?channel=-1",
        "/v1/folders?channel=9223372036854775808",
        "/v1/folders?channel=1&parent=%2e%2e%2f",
        "/v1/folders?channel=1&channel=2",
        "/v1/folders?channel=1&parent=d%3A%0A",
        "/v1/folders?channel=1&parent=d%3Afoo%25GG",
    ] {
        assert_eq!(
            parse_request(&request("GET", url, secret), SECRET),
            Err(HttpError::BadRequest),
            "unexpected URL accepted: {url}"
        );
    }
    assert_eq!(
        parse_request(&request("GET", "/admin", secret), SECRET),
        Err(HttpError::NotFound)
    );
}
