//! Read-only TDX1 decoder based on backend/projection/wire.go.
//! It deliberately rejects rendition descriptors until their JSON validation
//! is ported. It does not write Telegram messages or databases.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

const MAX_UNIX_TIMESTAMP: i64 = 253_402_300_799;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpType {
    File,
    Meta,
    Mkdir,
    Rename,
    Move,
    Rmdir,
    Tomb,
    Part,
    Manifest,
    EncryptionConfig,
    FileCommit,
    FolderCommit,
    FileReplace,
    Relocate,
    Trash,
    HardDelete,
    Untrash,
}

impl OpType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::File => "f",
            Self::Meta => "meta",
            Self::Mkdir => "mkdir",
            Self::Rename => "rename",
            Self::Move => "move",
            Self::Rmdir => "rmdir",
            Self::Tomb => "tomb",
            Self::Part => "part",
            Self::Manifest => "manifest",
            Self::EncryptionConfig => "encfg",
            Self::FileCommit => "fcommit",
            Self::FolderCommit => "dcommit",
            Self::FileReplace => "freplace",
            Self::Relocate => "relocate",
            Self::Trash => "trash",
            Self::HardDelete => "harddel",
            Self::Untrash => "untrash",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "f" => Self::File,
            "meta" => Self::Meta,
            "mkdir" => Self::Mkdir,
            "rename" => Self::Rename,
            "move" => Self::Move,
            "rmdir" => Self::Rmdir,
            "tomb" => Self::Tomb,
            "part" => Self::Part,
            "manifest" => Self::Manifest,
            "encfg" => Self::EncryptionConfig,
            "fcommit" => Self::FileCommit,
            "dcommit" => Self::FolderCommit,
            "freplace" => Self::FileReplace,
            "relocate" => Self::Relocate,
            "trash" => Self::Trash,
            "harddel" => Self::HardDelete,
            "untrash" => Self::Untrash,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireError {
    MissingHeader,
    BadVersion,
    BadOpType,
    Malformed,
    BadObject,
    BadParent,
    UnsupportedRendition,
    UnsupportedFormat,
}

impl WireError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MissingHeader => "missing_header",
            Self::BadVersion => "bad_version",
            Self::BadOpType => "bad_op_type",
            Self::Malformed => "malformed",
            Self::BadObject => "bad_object",
            Self::BadParent => "bad_parent",
            Self::UnsupportedRendition => "unsupported_rendition",
            Self::UnsupportedFormat => "unsupported_format",
        }
    }
}

impl fmt::Display for WireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TDX1: {}", self.as_str())
    }
}

impl Error for WireError {}

/// Parsed metadata, not a production replay operation. Sensitive vault fields
/// are validated but never exposed or printed by this API.
#[derive(Clone, PartialEq, Eq)]
pub struct Operation {
    pub op_type: OpType,
    pub object: Option<String>,
    pub parent: String,
    pub name: Option<String>,
    pub encrypted_name: Option<String>,
    pub file_size: i64,
    pub upload_time: i64,
    pub encrypted: bool,
    pub part_count: i64,
    pub content_message_id: i64,
    pub operation_id: Option<String>,
    pub protocol_version: i64,
    pub upload_uuid: Option<String>,
    pub part_index: i64,
    pub content_hash: Option<String>,
    pub expected_revision: i64,
    pub expected_destination_revision: i64,
    pub destination_object: Option<String>,
    pub overwrite: bool,
    pub deleted_at: i64,
    pub purge_after: i64,
    pub retained_until: i64,
    pub name_key_version: i64,
    pub encryption_version: i64,
    pub plaintext_size: i64,
}

impl Operation {
    fn new(op_type: OpType) -> Self {
        Self {
            op_type,
            object: None,
            parent: String::new(),
            name: None,
            encrypted_name: None,
            file_size: 0,
            upload_time: 0,
            encrypted: false,
            part_count: 0,
            content_message_id: 0,
            operation_id: None,
            protocol_version: 0,
            upload_uuid: None,
            part_index: 0,
            content_hash: None,
            expected_revision: 0,
            expected_destination_revision: 0,
            destination_object: None,
            overwrite: false,
            deleted_at: 0,
            purge_after: 0,
            retained_until: 0,
            name_key_version: 0,
            encryption_version: 0,
            plaintext_size: 0,
        }
    }
}

fn field<'a>(fields: &HashMap<&'a str, &'a str>, key: &str) -> &'a str {
    fields.get(key).copied().unwrap_or("")
}

fn positive(raw: &str) -> Result<i64, WireError> {
    let n: i64 = raw.parse().map_err(|_| WireError::Malformed)?;
    if n <= 0 {
        return Err(WireError::Malformed);
    }
    Ok(n)
}

fn non_negative(raw: &str) -> Result<i64, WireError> {
    let n: i64 = raw.parse().map_err(|_| WireError::Malformed)?;
    if n < 0 {
        return Err(WireError::Malformed);
    }
    Ok(n)
}

fn valid_object(raw: &str, expected: Option<char>) -> Result<String, WireError> {
    let valid = match expected {
        Some('f') => raw.starts_with("f:"),
        Some('d') => raw.starts_with("d:"),
        _ => raw.starts_with("f:") || raw.starts_with("d:"),
    };
    if valid {
        Ok(raw.to_owned())
    } else {
        Err(WireError::BadObject)
    }
}

fn valid_parent(raw: &str) -> Result<String, WireError> {
    if raw.is_empty() || raw.starts_with("d:") {
        Ok(raw.to_owned())
    } else {
        Err(WireError::BadParent)
    }
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// Form-style percent decoding matches Go QueryUnescape for valid UTF-8.
fn query_unescape(raw: &str) -> Result<String, WireError> {
    let src = raw.as_bytes();
    let mut bytes = Vec::with_capacity(src.len());
    let mut i = 0;
    while i < src.len() {
        match src[i] {
            b'+' => bytes.push(b' '),
            b'%' => {
                if i + 2 >= src.len() {
                    return Err(WireError::Malformed);
                }
                let a = hex_digit(src[i + 1]).ok_or(WireError::Malformed)?;
                let b = hex_digit(src[i + 2]).ok_or(WireError::Malformed)?;
                bytes.push((a << 4) | b);
                i += 2;
            }
            byte => bytes.push(byte),
        }
        i += 1;
    }
    String::from_utf8(bytes).map_err(|_| WireError::Malformed)
}

/// Unpadded URL-safe base64 validation for legacy vault metadata.
fn valid_base64url(raw: &str) -> bool {
    if raw.is_empty() {
        return false;
    }
    let (mut bits, mut tail) = (0_u8, 0_u32);
    let mut decoded = 0_usize;
    for byte in raw.bytes() {
        let digit = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'-' => 62,
            b'_' => 63,
            _ => return false,
        };
        tail = (tail << 6) | u32::from(digit);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            decoded += 1;
            tail &= (1 << bits) - 1;
        }
    }
    decoded > 0 && tail == 0
}

fn name(op: &mut Operation, fields: &HashMap<&str, &str>) -> Result<(), WireError> {
    let raw = field(fields, "n");
    if raw.is_empty() {
        return Err(WireError::Malformed);
    }
    op.name = Some(query_unescape(raw)?);
    if let Some(envelope) = fields.get("nenc") {
        if !envelope.starts_with("v1:") {
            return Err(WireError::Malformed);
        }
        op.name_key_version = positive(field(fields, "kid"))?;
        op.encrypted_name = Some((*envelope).to_owned());
    }
    Ok(())
}

fn writable(op: &mut Operation, fields: &HashMap<&str, &str>) -> Result<(), WireError> {
    if field(fields, "v") != "1" {
        return Err(WireError::Malformed);
    }
    let raw_id = field(fields, "oid");
    if raw_id.is_empty() {
        return Err(WireError::Malformed);
    }
    let id = query_unescape(raw_id)?;
    if id.is_empty() {
        return Err(WireError::Malformed);
    }
    op.operation_id = Some(id);
    op.protocol_version = 1;
    Ok(())
}

fn content_reference(op: &mut Operation, fields: &HashMap<&str, &str>) -> Result<(), WireError> {
    let mid = field(fields, "cmid");
    if !mid.is_empty() {
        op.content_message_id = positive(mid)?;
    }
    let uuid = field(fields, "u");
    if !uuid.is_empty() {
        op.upload_uuid = Some(uuid.to_owned());
    }
    let count = field(fields, "pc");
    if !count.is_empty() {
        op.part_count = positive(count)?;
    }
    if !field(fields, "hash").is_empty() {
        op.content_hash = Some(query_unescape(field(fields, "hash"))?);
    }
    let multipart = !uuid.is_empty() && op.part_count > 0;
    if (op.content_message_id == 0 && !multipart)
        || (op.content_message_id > 0 && (!uuid.is_empty() || op.part_count != 0))
    {
        return Err(WireError::Malformed);
    }
    Ok(())
}

fn optional_positive(fields: &HashMap<&str, &str>, name: &str) -> Result<i64, WireError> {
    let value = field(fields, name);
    if value.is_empty() {
        Ok(0)
    } else {
        positive(value)
    }
}

/// Only decodes and validates; no projection writer calls this yet.
pub fn parse_wire(raw: &str) -> Result<Operation, WireError> {
    let header = raw.split('\n').next().unwrap_or("").trim();
    if header.is_empty() {
        return Err(WireError::MissingHeader);
    }
    if !header.contains('|') {
        return Err(WireError::Malformed);
    }
    let mut components = header.split('|');
    if components.next() != Some("TDX1") {
        return Err(WireError::BadVersion);
    }
    let mut fields = HashMap::new();
    for part in components {
        let (key, val) = part.split_once('=').ok_or(WireError::Malformed)?;
        if key.is_empty() {
            return Err(WireError::Malformed);
        }
        // Same last-key-wins behavior as the Go map parser.
        fields.insert(key, val);
    }
    let kind = OpType::parse(field(&fields, "t")).ok_or_else(|| {
        if fields.contains_key("t") {
            WireError::BadOpType
        } else {
            WireError::Malformed
        }
    })?;
    let mut op = Operation::new(kind);
    match kind {
        OpType::File => {
            op.parent = valid_parent(field(&fields, "p"))?;
            name(&mut op, &fields)?;
        }
        OpType::Meta => {
            op.object = Some(valid_object(field(&fields, "obj"), Some('f'))?);
            op.parent = valid_parent(field(&fields, "p"))?;
            name(&mut op, &fields)?;
        }
        OpType::Mkdir => {
            op.object = Some(valid_object(field(&fields, "obj"), Some('d'))?);
            op.parent = valid_parent(field(&fields, "p"))?;
            name(&mut op, &fields)?;
        }
        OpType::Rename => {
            op.object = Some(valid_object(field(&fields, "obj"), None)?);
            name(&mut op, &fields)?;
        }
        OpType::Move => {
            op.object = Some(valid_object(field(&fields, "obj"), None)?);
            op.parent = valid_parent(field(&fields, "p"))?;
        }
        OpType::Rmdir => op.object = Some(valid_object(field(&fields, "obj"), Some('d'))?),
        OpType::Tomb => op.object = Some(valid_object(field(&fields, "obj"), Some('f'))?),
        OpType::Part => {
            if field(&fields, "u").is_empty() {
                return Err(WireError::Malformed);
            }
            op.upload_uuid = Some(field(&fields, "u").to_owned());
            op.part_index = non_negative(field(&fields, "pix"))?;
        }
        OpType::Manifest => {
            if field(&fields, "u").is_empty() {
                return Err(WireError::Malformed);
            }
            op.upload_uuid = Some(field(&fields, "u").to_owned());
            op.parent = valid_parent(field(&fields, "p"))?;
            name(&mut op, &fields)?;
            op.part_count = positive(field(&fields, "pc"))?;
        }
        OpType::EncryptionConfig => {
            for key in ["salt", "wrap", "check"] {
                if !valid_base64url(field(&fields, key)) {
                    return Err(WireError::Malformed);
                }
            }
            if field(&fields, "kdf").is_empty() {
                return Err(WireError::Malformed);
            }
            query_unescape(field(&fields, "kdf"))?;
            if let Some(hint) = fields.get("hint") {
                query_unescape(hint)?;
            }
            if let Some(version) = fields.get("cv") {
                version.parse::<i64>().map_err(|_| WireError::Malformed)?;
            }
        }
        OpType::FileCommit => {
            writable(&mut op, &fields)?;
            op.parent = valid_parent(field(&fields, "p"))?;
            name(&mut op, &fields)?;
            content_reference(&mut op, &fields)?;
        }
        OpType::FolderCommit => {
            writable(&mut op, &fields)?;
            op.object = Some(valid_object(field(&fields, "obj"), Some('d'))?);
            op.parent = valid_parent(field(&fields, "p"))?;
            name(&mut op, &fields)?;
        }
        OpType::FileReplace => {
            writable(&mut op, &fields)?;
            op.object = Some(valid_object(field(&fields, "obj"), Some('f'))?);
            op.expected_revision = positive(field(&fields, "rev"))?;
            content_reference(&mut op, &fields)?;
            op.retained_until = positive(field(&fields, "retain"))?;
        }
        OpType::Relocate => {
            writable(&mut op, &fields)?;
            op.object = Some(valid_object(field(&fields, "obj"), None)?);
            op.parent = valid_parent(field(&fields, "p"))?;
            name(&mut op, &fields)?;
            op.expected_revision = positive(field(&fields, "rev"))?;
            op.overwrite = field(&fields, "ow") == "1";
            if !field(&fields, "dst").is_empty() {
                op.destination_object = Some(valid_object(field(&fields, "dst"), None)?);
            }
            op.expected_destination_revision = optional_positive(&fields, "drev")?;
            op.deleted_at = optional_positive(&fields, "del")?;
            op.purge_after = optional_positive(&fields, "purge")?;
        }
        OpType::Trash => {
            writable(&mut op, &fields)?;
            op.object = Some(valid_object(field(&fields, "obj"), None)?);
            op.expected_revision = positive(field(&fields, "rev"))?;
            op.deleted_at = positive(field(&fields, "del"))?;
            op.purge_after = positive(field(&fields, "purge"))?;
        }
        OpType::HardDelete => {
            writable(&mut op, &fields)?;
            op.object = Some(valid_object(field(&fields, "obj"), None)?);
            op.expected_revision = positive(field(&fields, "rev"))?;
        }
        OpType::Untrash => {
            writable(&mut op, &fields)?;
            op.object = Some(valid_object(field(&fields, "obj"), None)?);
            op.parent = valid_parent(field(&fields, "p"))?;
            name(&mut op, &fields)?;
            op.expected_revision = positive(field(&fields, "rev"))?;
        }
    }
    if let Some(raw_size) = fields.get("sz") {
        op.file_size = non_negative(raw_size)?;
    }
    if let Some(raw_time) = fields.get("ts") {
        op.upload_time = non_negative(raw_time)?;
        if op.upload_time > MAX_UNIX_TIMESTAMP {
            return Err(WireError::Malformed);
        }
    }
    op.encrypted = field(&fields, "enc") == "1";
    if op.encrypted {
        op.encryption_version = 1;
    }
    if let Some(raw_ev) = fields.get("ev") {
        op.encryption_version = raw_ev.parse::<i64>().map_err(|_| WireError::Malformed)?;
    }
    if let Some(raw_plaintext) = fields.get("psz") {
        op.plaintext_size = non_negative(raw_plaintext)?;
    }
    if fields.contains_key("rend") {
        return if kind == OpType::Part {
            Err(WireError::UnsupportedRendition)
        } else {
            Err(WireError::Malformed)
        };
    }
    Ok(op)
}

/// Encode only the plain seven-operation subset of Go Format().
///
/// All writable CAS, multipart and sealed-filename operations are rejected
/// until full-field Go↔Rust differential tests cover their entire envelope.
/// Never use this helper to publish Telegram messages.
pub fn format_simple_wire(op: &Operation) -> Result<String, WireError> {
    if !matches!(
        op.op_type,
        OpType::File
            | OpType::Meta
            | OpType::Mkdir
            | OpType::Rename
            | OpType::Move
            | OpType::Rmdir
            | OpType::Tomb
    ) {
        return Err(WireError::UnsupportedFormat);
    }
    if op.encrypted
        || op.encrypted_name.is_some()
        || op.part_count != 0
        || op.content_message_id != 0
        || op.operation_id.is_some()
        || op.protocol_version != 0
        || op.upload_uuid.is_some()
        || op.part_index != 0
        || op.content_hash.is_some()
        || op.expected_revision != 0
        || op.expected_destination_revision != 0
        || op.destination_object.is_some()
        || op.overwrite
        || op.deleted_at != 0
        || op.purge_after != 0
        || op.retained_until != 0
        || op.name_key_version != 0
        || op.encryption_version != 0
        || op.plaintext_size != 0
        || op.file_size < 0
        || op.upload_time < 0
    {
        return Err(WireError::UnsupportedFormat);
    }
    if !matches!(op.op_type, OpType::File | OpType::Meta)
        && (op.file_size != 0 || op.upload_time != 0)
    {
        return Err(WireError::UnsupportedFormat);
    }

    let mut out = format!("TDX1|t={}", op.op_type.as_str());
    match op.op_type {
        OpType::File => {
            let parent = valid_parent(&op.parent)?;
            out.push_str("|p=");
            out.push_str(&parent);
            append_simple_name(&mut out, op)?;
            append_simple_file_attrs(&mut out, op);
        }
        OpType::Meta => {
            append_simple_object(&mut out, op, Some('f'))?;
            let parent = valid_parent(&op.parent)?;
            out.push_str("|p=");
            out.push_str(&parent);
            append_simple_name(&mut out, op)?;
            append_simple_file_attrs(&mut out, op);
        }
        OpType::Mkdir => {
            append_simple_object(&mut out, op, Some('d'))?;
            let parent = valid_parent(&op.parent)?;
            out.push_str("|p=");
            out.push_str(&parent);
            append_simple_name(&mut out, op)?;
        }
        OpType::Rename => {
            append_simple_object(&mut out, op, None)?;
            append_simple_name(&mut out, op)?;
        }
        OpType::Move => {
            append_simple_object(&mut out, op, None)?;
            let parent = valid_parent(&op.parent)?;
            out.push_str("|p=");
            out.push_str(&parent);
        }
        OpType::Rmdir => append_simple_object(&mut out, op, Some('d'))?,
        OpType::Tomb => append_simple_object(&mut out, op, Some('f'))?,
        _ => return Err(WireError::UnsupportedFormat),
    }
    Ok(out)
}

fn append_simple_object(
    out: &mut String,
    op: &Operation,
    kind: Option<char>,
) -> Result<(), WireError> {
    let id = op.object.as_deref().ok_or(WireError::BadObject)?;
    valid_object(id, kind)?;
    out.push_str("|obj=");
    out.push_str(id);
    Ok(())
}

fn append_simple_name(out: &mut String, op: &Operation) -> Result<(), WireError> {
    let name = op.name.as_deref().ok_or(WireError::Malformed)?;
    if name.is_empty() {
        return Err(WireError::Malformed);
    }
    out.push_str("|n=");
    out.push_str(&query_escape(name));
    Ok(())
}

fn append_simple_file_attrs(out: &mut String, op: &Operation) {
    if op.file_size > 0 {
        out.push_str("|sz=");
        out.push_str(&op.file_size.to_string());
    }
    if op.upload_time > 0 {
        out.push_str("|ts=");
        out.push_str(&op.upload_time.to_string());
    }
}

/// Valid-UTF-8 subset of Go net/url.QueryEscape: form encoding uses '+' for
/// spaces, upper-case hexadecimal escapes, and unescaped ASCII -_.~.
fn query_escape(input: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(char::from(byte));
            }
            b' ' => out.push('+'),
            _ => {
                out.push('%');
                out.push(char::from(HEX[usize::from(byte >> 4)]));
                out.push(char::from(HEX[usize::from(byte & 15)]));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_decoding_supports_spaces_plus_and_korean() {
        assert_eq!(
            query_unescape("hello+world%2B2026").unwrap(),
            "hello world+2026"
        );
        assert_eq!(query_unescape("%ED%95%9C%EA%B8%80").unwrap(), "한글");
        assert!(query_unescape("%GG").is_err());
        assert!(query_unescape("%ED").is_err());
    }

    #[test]
    fn base64url_rejects_padding_and_invalid_tail_bits() {
        assert!(valid_base64url("c2FsdA"));
        assert!(valid_base64url("Y2hlY2s"));
        assert!(valid_base64url("d3JhcHBlZA"));
        assert!(!valid_base64url("c2FsdA="));
        assert!(!valid_base64url("AR"));
        assert!(!valid_base64url("A"));
    }

    #[test]
    fn caption_after_newline_does_not_change_header() {
        let operation =
            parse_wire("TDX1|t=f|p=d:abc|n=sunset.jpg|sz=4321|ts=99\nprivate caption").unwrap();
        assert_eq!(operation.parent, "d:abc");
        assert_eq!(operation.file_size, 4321);
        assert_eq!(operation.upload_time, 99);
    }

    #[test]
    fn sealed_name_kept_opaque() {
        let operation = parse_wire("TDX1|t=f|p=|n=_|nenc=v1:AA|kid=1|enc=1").unwrap();
        assert_eq!(operation.encrypted_name.as_deref(), Some("v1:AA"));
        assert!(operation.encrypted);
    }
}
