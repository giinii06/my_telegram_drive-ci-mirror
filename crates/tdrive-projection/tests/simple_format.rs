use tdrive_projection::{WireError, format_simple_wire, parse_wire};

#[test]
fn canonical_headers_match_the_go_formatter() {
    let fixtures = include_str!("../../../tests/fixtures/tdx1/formatted.tsv");
    let mut cases = 0;
    for line in fixtures.lines().filter(|line| !line.is_empty() && !line.starts_with('#')) {
        let (name, raw) = line.split_once('\t').expect("two fixture columns");
        let decoded = parse_wire(raw).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(
            format_simple_wire(&decoded).unwrap(),
            raw,
            "mismatch in {name}"
        );
        cases += 1;
    }
    assert!(cases >= 14);
}

#[test]
fn writer_only_and_encrypted_headers_fail_closed() {
    for header in [
        "TDX1|t=fcommit|v=1|oid=commit|p=|n=a|cmid=91",
        "TDX1|t=manifest|u=up|p=|n=x|pc=2",
        "TDX1|t=f|p=|n=_|nenc=v1:AA|enc=1|ev=1|kid=1",
    ] {
        let parsed = parse_wire(header).unwrap();
        assert_eq!(
            format_simple_wire(&parsed),
            Err(WireError::UnsupportedFormat),
            "{header}"
        );
    }
}
