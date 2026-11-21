use tdrive_backup_core::{UploadStatus, summarize_upload};

#[test]
fn go_and_rust_upload_summary_cases_match() {
    let fixture = include_str!("../../../tests/fixtures/backup/upload-status.tsv");
    let mut count = 0;
    for line in fixture
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
    {
        let columns: Vec<_> = line.split('\t').collect();
        assert_eq!(columns.len(), 5, "bad fixture row");
        let statuses: Vec<_> = if columns[1] == "-" {
            Vec::new()
        } else {
            columns[1]
                .split(',')
                .map(|stage| UploadStatus {
                    stage,
                    error: if stage == "error" && columns[2] != "-" {
                        columns[2]
                    } else {
                        ""
                    },
                    size: 0,
                })
                .collect()
        };
        let (done, failure) = summarize_upload(&statuses);
        assert_eq!(done.to_string(), columns[3], "case {}", columns[0]);
        assert_eq!(
            failure,
            if columns[4] == "-" { "" } else { columns[4] },
            "case {}",
            columns[0]
        );
        count += 1;
    }
    assert!(count >= 11);
}
