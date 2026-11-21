use std::num::ParseIntError;
use tdrive_telegram::stable_random_id;

#[test]
fn matches_go_stable_random_id_goldens() -> Result<(), ParseIntError> {
    let fixtures = include_str!("../../../tests/fixtures/telegram/random-ids.tsv");
    let mut seen = 0;
    for row in fixtures
        .lines()
        .filter(|r| !r.is_empty() && !r.starts_with('#'))
    {
        let mut it = row.split('\t');
        let operation_id = it.next().unwrap();
        let step = it.next().unwrap();
        let expected: i64 = it.next().unwrap().parse()?;
        assert!(it.next().is_none(), "unexpected column");
        assert_eq!(
            stable_random_id(operation_id, step).unwrap().get(),
            expected,
            "{operation_id} / {step}"
        );
        seen += 1;
    }
    assert!(seen >= 5);
    Ok(())
}
