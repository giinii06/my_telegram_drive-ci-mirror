//! Strict one-request HTTP/1.1 read-only routing and bearer authentication.
//! No socket, database or file side effects in this module.
const MAX_HEADER_BYTES: usize = 8192;

#[derive(Debug, PartialEq, Eq)]
pub enum ApiRoute {
    Health,
    Parts { channel_id: i64, message_id: i64 },
    Folder { channel_id: i64, parent: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpError {
    BadRequest,
    Unauthorized,
    NotFound,
    MethodNotAllowed,
    TooLarge,
}

pub fn valid_token(secret: &[u8]) -> bool {
    (32..=128).contains(&secret.len())
        && secret
            .iter()
            .copied()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

fn constant_time_equal(a: &[u8], b: &[u8]) -> bool {
    let mut diff = a.len() ^ b.len();
    for i in 0..a.len().max(b.len()) {
        let left = a.get(i).copied().unwrap_or_default();
        let right = b.get(i).copied().unwrap_or_default();
        diff |= usize::from(left ^ right);
    }
    diff == 0
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn decode_component(raw: &str) -> Option<String> {
    let bytes = raw.as_bytes();
    if bytes.len() > 512 {
        return None;
    }
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' => {
                if i + 2 >= bytes.len() {
                    return None;
                }
                let hi = hex_digit(bytes[i + 1])?;
                let lo = hex_digit(bytes[i + 2])?;
                decoded.push(hi << 4 | lo);
                i += 3;
            }
            b'+' => {
                decoded.push(b' ');
                i += 1;
            }
            byte => {
                decoded.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8(decoded).ok()
}

fn parse_folder(target: &str) -> Result<ApiRoute, HttpError> {
    let (_, query) = target.split_once('?').ok_or(HttpError::BadRequest)?;
    let mut channel: Option<i64> = None;
    let mut parent: Option<String> = None;
    for pair in query.split('&') {
        let (name, value) = pair.split_once('=').ok_or(HttpError::BadRequest)?;
        match name {
            "channel" if channel.is_none() => {
                if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
                    return Err(HttpError::BadRequest);
                }
                let id = value.parse::<i64>().map_err(|_| HttpError::BadRequest)?;
                if id <= 0 {
                    return Err(HttpError::BadRequest);
                }
                channel = Some(id);
            }
            "parent" if parent.is_none() => {
                let value = decode_component(value).ok_or(HttpError::BadRequest)?;
                if !value.is_empty()
                    && (!value.starts_with("d:")
                        || value.len() <= 2
                        || value.len() > 256
                        || !value.as_bytes()[2..].iter().all(|byte| {
                            byte.is_ascii_alphanumeric() || *byte == b'-' || *byte == b'_'
                        }))
                {
                    return Err(HttpError::BadRequest);
                }
                parent = Some(value);
            }
            _ => return Err(HttpError::BadRequest),
        }
    }
    Ok(ApiRoute::Folder {
        channel_id: channel.ok_or(HttpError::BadRequest)?,
        parent: parent.unwrap_or_default(),
    })
}

fn parse_parts(target: &str) -> Result<ApiRoute, HttpError> {
    let (_, query) = target.split_once('?').ok_or(HttpError::BadRequest)?;
    let mut channel = None;
    let mut message = None;
    for pair in query.split('&') {
        let (name, value) = pair.split_once('=').ok_or(HttpError::BadRequest)?;
        if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(HttpError::BadRequest);
        }
        let id = value.parse::<i64>().map_err(|_| HttpError::BadRequest)?;
        if id <= 0 {
            return Err(HttpError::BadRequest);
        }
        match name {
            "channel" if channel.is_none() => channel = Some(id),
            "message" if message.is_none() => message = Some(id),
            _ => return Err(HttpError::BadRequest),
        }
    }
    Ok(ApiRoute::Parts {
        channel_id: channel.ok_or(HttpError::BadRequest)?,
        message_id: message.ok_or(HttpError::BadRequest)?,
    })
}

/// Parses a complete, non-pipelined GET request. Always authenticate before
/// looking up a route, so anonymous callers cannot enumerate API endpoints.
pub fn parse_request(bytes: &[u8], expected_token: &[u8]) -> Result<ApiRoute, HttpError> {
    if bytes.len() > MAX_HEADER_BYTES {
        return Err(HttpError::TooLarge);
    }
    if !bytes.ends_with(b"\r\n\r\n") {
        return Err(HttpError::BadRequest);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| HttpError::BadRequest)?;
    let mut lines = text[..text.len() - 4].split("\r\n");
    let request_line = lines.next().ok_or(HttpError::BadRequest)?;
    let mut parts = request_line.split(' ');
    let method = parts.next().ok_or(HttpError::BadRequest)?;
    let target = parts.next().ok_or(HttpError::BadRequest)?;
    let version = parts.next().ok_or(HttpError::BadRequest)?;
    if parts.next().is_some() || version != "HTTP/1.1" || !target.starts_with('/') {
        return Err(HttpError::BadRequest);
    }
    let mut bearer = None;
    let mut host_seen = false;
    for line in lines {
        if line.is_empty() || line.starts_with(' ') || line.starts_with('\t') {
            return Err(HttpError::BadRequest);
        }
        let (name, value) = line.split_once(':').ok_or(HttpError::BadRequest)?;
        if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
            return Err(HttpError::BadRequest);
        }
        if value.bytes().any(|byte| (byte < 32 && byte != b'\t') || byte == 127) {
            return Err(HttpError::BadRequest);
        }
        if name.eq_ignore_ascii_case("host") {
            let authority = value.trim();
            if host_seen || authority.is_empty() || authority.bytes().any(|byte| byte.is_ascii_whitespace()) {
                return Err(HttpError::BadRequest);
            }
            host_seen = true;
        } else if name.eq_ignore_ascii_case("authorization") {
            if bearer.is_some() {
                return Err(HttpError::BadRequest);
            }
            bearer = Some(value.trim());
        } else if name.eq_ignore_ascii_case("content-length")
            || name.eq_ignore_ascii_case("transfer-encoding")
        {
            return Err(HttpError::BadRequest);
        }
    }

    if !host_seen {
        return Err(HttpError::BadRequest);
    }
    if !valid_token(expected_token) {
        return Err(HttpError::Unauthorized);
    }
    let supplied = bearer
        .ok_or(HttpError::Unauthorized)?
        .strip_prefix("Bearer ")
        .ok_or(HttpError::Unauthorized)?;
    if !constant_time_equal(supplied.as_bytes(), expected_token) {
        return Err(HttpError::Unauthorized);
    }
    if method != "GET" {
        return Err(HttpError::MethodNotAllowed);
    }
    if target == "/health" {
        return Ok(ApiRoute::Health);
    }
    if target.starts_with("/v1/parts?") {
        return parse_parts(target);
    }
    if target.starts_with("/v1/folders?") {
        return parse_folder(target);
    }
    Err(HttpError::NotFound)
}
