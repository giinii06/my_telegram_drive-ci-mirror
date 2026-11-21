use tdrive_projection::{OpType, WireError, parse_wire};

#[test]
fn go_tdx1_curated_cases() {
    let raw = include_str!("../../../tests/fixtures/tdx1/cases.tsv");
    let mut cases = 0;
    for line in raw.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let mut fields = line.splitn(3, '\t');
        let name = fields.next().unwrap();
        let expected = fields.next().expect("expected column");
        let header = fields.next().expect("raw header column");
        let result = parse_wire(header);
        if let Some(kind) = expected.strip_prefix("ok:") {
            let parsed = result.unwrap_or_else(|error| panic!("{name}: unexpected {error}"));
            assert_eq!(parsed.op_type, OpType::parse(kind).unwrap(), "{name}");
        } else if let Some(error) = expected.strip_prefix("err:") {
            let actual = result.err().unwrap_or_else(|| panic!("{name}: unexpectedly accepted"));
            assert_eq!(actual.as_str(), error, "{name}");
        } else {
            panic!("{name}: invalid fixture expectation");
        }
        cases += 1;
    }
    assert!(cases >= 40, "fixture coverage reduced");
}

#[test]
fn incomplete_or_destructive_write_operations_fail_closed() {
    for raw in [
        "TDX1|t=fcommit|v=1|oid=foo|p=|n=file.txt",
        "TDX1|t=trash|v=1|oid=trash|obj=f:1|rev=1",
        "TDX1|t=mkdir|obj=f:1|p=|n=wrong-kind",
        "TDX1|t=part|u=uuid|pix=-1",
    ] {
        assert!(parse_wire(raw).is_err(), "{raw}");
    }
}

#[test]
fn unported_rendition_is_explicitly_rejected() {
    let result = parse_wire("TDX1|t=part|u=uuid|pix=0|rend=YWJj");
    assert_eq!(result.err(), Some(WireError::UnsupportedRendition));
}
