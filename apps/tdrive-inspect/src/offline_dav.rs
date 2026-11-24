//! Bounded, read-only DAV metadata rendering for explicitly detached snapshots.
//! Supports empty-body PROPFIND Depth 0/1 only; no filesystem/remote reads.
use std::fmt::Write as _;
use tdrive_storage::{DavEntry, DavListing};

pub fn valid_name(name: &str) -> bool {
    !name.is_empty() && name.len() <= 240 && name != "." && name != ".."
        && name.chars().all(|c| {
            !c.is_control() && !matches!(c, '/' | '\\' | '\u{fffe}' | '\u{ffff}')
        })
}

fn uri_segment(name: &str) -> String {
    let mut result = String::new();
    for b in name.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
            result.push(char::from(b));
        } else {
            write!(result, "%{b:02X}").expect("string write");
        }
    }
    result
}

fn xml_escape(value: &str, output: &mut String) {
    for c in value.chars() {
        match c {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&apos;"),
            _ => output.push(c),
        }
    }
}

fn entry(output: &mut String, href: &str, resource: &DavEntry) -> Option<()> {
    if !valid_name(&resource.name) {
        return None;
    }
    output.push_str("<D:response><D:href>");
    xml_escape(href, output);
    output.push_str("</D:href><D:propstat><D:prop><D:displayname>");
    xml_escape(&resource.name, output);
    output.push_str("</D:displayname><D:resourcetype>");
    if resource.collection {
        output.push_str("<D:collection/>");
    }
    output.push_str("</D:resourcetype>");
    if let Some(size) = resource.size {
        write!(output, "<D:getcontentlength>{size}</D:getcontentlength>").ok()?;
    }
    output.push_str("</D:prop><D:status>HTTP/1.1 200 OK</D:status></D:propstat></D:response>");
    Some(())
}

pub fn render(channel: i64, segments: &[String], listing: &DavListing) -> Option<String> {
    let mut href = format!("/dav/{channel}/");
    for (i, segment) in segments.iter().enumerate() {
        if !valid_name(segment) { return None; }
        href.push_str(&uri_segment(segment));
        if i + 1 < segments.len() || listing.resource.collection { href.push('/'); }
    }
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?><D:multistatus xmlns:D=\"DAV:\">");
    entry(&mut xml, &href, &listing.resource)?;
    for child in &listing.children {
        if !valid_name(&child.name) { return None; }
        let mut child_href = format!("{href}{}", uri_segment(&child.name));
        if child.collection { child_href.push('/'); }
        entry(&mut xml, &child_href, child)?;
        if xml.len() > 16 * 1024 * 1024 { return None; }
    }
    xml.push_str("</D:multistatus>");
    Some(xml)
}
