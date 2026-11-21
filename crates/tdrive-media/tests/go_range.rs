use tdrive_media::parse_range_header;

#[test]
fn match_go_media_range_parser_at_protocol_boundaries() {
    let cases = include_str!("../../../tests/fixtures/media/range.tsv");
    let mut count = 0;
    for line in cases
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
    {
        let fields: Vec<_> = line.split('\t').collect();
        assert_eq!(fields.len(), 7, "invalid fixture: {line}");
        let raw = if fields[1] == "<none>" { "" } else { fields[1] };
        let size = fields[2].parse::<i64>().unwrap();
        let got = parse_range_header(raw, size);
        assert_eq!(
            got.start,
            fields[3].parse::<i64>().unwrap(),
            "{}",
            fields[0]
        );
        assert_eq!(got.end, fields[4].parse::<i64>().unwrap(), "{}", fields[0]);
        assert_eq!(got.partial.to_string(), fields[5], "{} partial", fields[0]);
        assert_eq!(got.ok.to_string(), fields[6], "{} validity", fields[0]);
        count += 1;
    }
    assert!(count >= 18);
}
