//! Exact content-level digest of all Go v16 core table rows.
//!
//! Each SQLite value is encoded by typeof() + hex(CAST(value AS BLOB)).
//! This preserves embedded NUL bytes and BLOBs, unlike quote() rendering.
//! Every string in the hash is prefixed with an 8-byte BE length. Rows are
//! sorted by all selected fields rather than relying on rowid/insertion order.

use crate::{REQUIRED_TABLES, SnapshotError, inspect_connection, open_verified_snapshot};
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::path::Path;

const MAX_ROWS_PER_TABLE: usize = 1_000_000;
const MAX_DIGEST_INPUT_PER_TABLE: usize = 32 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableDigest {
    pub table: &'static str,
    pub row_count: usize,
    pub sha256: String,
}

/// Safe for a detached Go SQLite snapshot only; never opens a live database.
/// Returns counts and digests only, not stored captions or vault key material.
pub fn snapshot_table_digests(path: &Path) -> Result<Vec<TableDigest>, SnapshotError> {
    let connection = open_verified_snapshot(path)?;
    inspect_connection(&connection)?;
    let mut out = Vec::with_capacity(REQUIRED_TABLES.len());
    for &table in REQUIRED_TABLES {
        out.push(digest_table(&connection, table)?);
    }
    Ok(out)
}

fn quote_identifier(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

fn feed(
    digest: &mut Sha256,
    used: &mut usize,
    value: &str,
) -> Result<(), SnapshotError> {
    let bytes = value.as_bytes();
    *used = used
        .checked_add(bytes.len())
        .and_then(|size| size.checked_add(8))
        .ok_or(SnapshotError::SnapshotTooLarge)?;
    if *used > MAX_DIGEST_INPUT_PER_TABLE {
        return Err(SnapshotError::SnapshotTooLarge);
    }
    digest.update((bytes.len() as u64).to_be_bytes());
    digest.update(bytes);
    Ok(())
}

fn digest_table(connection: &Connection, table: &'static str) -> Result<TableDigest, SnapshotError> {
    let table_name = quote_identifier(table);
    let mut columns_stmt = connection.prepare(&format!("PRAGMA table_info({table_name})"))?;
    let columns_rows = columns_stmt.query_map([], |row| row.get::<_, String>(1))?;
    let mut columns = Vec::new();
    for item in columns_rows {
        columns.push(item?);
    }
    if columns.is_empty() {
        return Err(SnapshotError::MissingTable(table));
    }

    let mut digest = Sha256::new();
    let mut used = 0_usize;
    feed(&mut digest, &mut used, table)?;
    for column in &columns {
        feed(&mut digest, &mut used, column)?;
    }

    let expressions: Vec<String> = columns
        .iter()
        .flat_map(|column| {
            let id = quote_identifier(column);
            [format!("typeof({id})"), format!("hex(CAST({id} AS BLOB))")]
        })
        .collect();
    let order = (1..=expressions.len())
        .map(|index| index.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "SELECT {} FROM {table_name} ORDER BY {order}",
        expressions.join(",")
    );
    let mut stmt = connection.prepare(&sql)?;
    let mut rows = stmt.query([])?;
    let mut count = 0_usize;
    while let Some(row) = rows.next()? {
        count += 1;
        if count > MAX_ROWS_PER_TABLE {
            return Err(SnapshotError::SnapshotTooLarge);
        }
        for index in 0..expressions.len() {
            let value: String = row.get(index)?;
            feed(&mut digest, &mut used, &value)?;
        }
    }
    Ok(TableDigest {
        table,
        row_count: count,
        sha256: format!("{:x}", digest.finalize()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(rows: &[(&str, i64)]) -> TableDigest {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE samples (name TEXT NOT NULL, seq INTEGER NOT NULL, payload BLOB)",
        )
        .unwrap();
        for &(name, seq) in rows {
            conn.execute(
                "INSERT INTO samples (name, seq, payload) VALUES (?1, ?2, x'00FF')",
                rusqlite::params![name, seq],
            )
            .unwrap();
        }
        digest_table(&conn, "samples").unwrap()
    }

    #[test]
    fn deterministic_despite_insert_order_and_binary_data() {
        let first = sample(&[("a\0b", 3), ("한글", 2)]);
        let reverse = sample(&[("한글", 2), ("a\0b", 3)]);
        assert_eq!(first, reverse);
        assert_eq!(first.row_count, 2);
        assert_ne!(first.sha256, sample(&[("a\0b", 4), ("한글", 2)]).sha256);
    }

    #[test]
    fn empty_table_also_has_a_schema_bound_digest() {
        assert_eq!(sample(&[]).row_count, 0);
        assert_ne!(sample(&[]).sha256, sample(&[("a", 1)]).sha256);
    }
}
