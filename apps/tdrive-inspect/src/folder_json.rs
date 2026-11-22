//! Emit read-only detached SQLite folder listings without requiring Go or a
//! serializer dependency. All wide numbers are decimal strings for JS safety.
use tdrive_storage::FolderListing;
use std::fmt::Write;

fn string_json(into: &mut String, text: &str) {
    into.push('"');
    for ch in text.chars() {
        match ch {
            '"' => into.push_str("\\\""),
            '\\' => into.push_str("\\\\"),
            '\n' => into.push_str("\\n"),
            '\r' => into.push_str("\\r"),
            '\t' => into.push_str("\\t"),
            '\u{2028}' | '\u{2029}' => {
                write!(into, "\\u{:04x}", ch as u32).expect("String writes cannot fail");
            }
            c if c <= '\u{001f}' => {
                write!(into, "\\u{:04x}", c as u32).expect("String writes cannot fail");
            }
            _ => into.push(ch),
        }
    }
    into.push('"');
}

fn string_field(into: &mut String, name: &str, value: &str) {
    into.push(',');
    string_json(into, name);
    into.push(':');
    string_json(into, value);
}

/// JSON lines meant for offline browsing, not an authenticated HTTP API.
/// No database or key writes are performed by this function.
pub fn encode_json(listing: &FolderListing) -> String {
    let mut json = String::from("{\"folders\":[");
    for (index, folder) in listing.folders.iter().enumerate() {
        if index != 0 {
            json.push(',');
        }
        json.push_str("{\"id\":");
        string_json(&mut json, &folder.id);
        string_field(&mut json, "name", &folder.name);
        string_field(&mut json, "parentId", &folder.parent_id);
        string_field(&mut json, "revision", &folder.revision.to_string());
        json.push('}');
    }
    json.push_str("],\"files\":[");
    for (index, file) in listing.files.iter().enumerate() {
        if index != 0 {
            json.push(',');
        }
        json.push_str("{\"msgId\":");
        string_json(&mut json, &file.msg_id.to_string());
        string_field(&mut json, "contentMsgId", &file.content_msg_id.to_string());
        string_field(&mut json, "name", &file.name);
        string_field(&mut json, "contentHash", &file.content_hash);
        string_field(&mut json, "revision", &file.revision.to_string());
        string_field(&mut json, "uploadUuid", &file.upload_uuid);
        string_field(&mut json, "partCount", &file.part_count.to_string());
        string_field(&mut json, "size", &file.size.to_string());
        string_field(&mut json, "parentId", &file.parent_id);
        string_field(&mut json, "uploadTime", &file.upload_time.to_string());
        string_field(&mut json, "uploaderId", &file.uploader_id.to_string());
        json.push_str(",\"encrypted\":");
        json.push_str(if file.encrypted { "true" } else { "false" });
        string_field(&mut json, "plaintextSize", &file.plaintext_size.to_string());
        json.push('}');
    }
    json.push_str("]}");
    json
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_string_escapes_controls_and_preserves_unicode() {
        let mut output = String::new();
        string_json(&mut output, "한\"\\\n\r\t\u{0000}\u{001f}\u{2028}");
        assert_eq!(output, r#""한\"\\\n\r\t\u0000\u001f\u2028""#);
    }
}
