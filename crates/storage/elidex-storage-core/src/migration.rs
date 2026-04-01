// Migration types are defined in backend.rs (Migration struct).
// This module provides helpers for building migration sets.

use crate::backend::Migration;

/// Build a migration set from a slice of (sql, version) pairs.
///
/// Sorts by version to ensure correct application order regardless of input order.
pub fn migrations_from_pairs(pairs: &[(&'static str, u32)]) -> Vec<Migration> {
    let mut migrations: Vec<Migration> = pairs
        .iter()
        .map(|(sql, version)| Migration {
            version: *version,
            sql,
        })
        .collect();
    migrations.sort_by_key(|m| m.version);
    migrations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_pairs() {
        let migrations = migrations_from_pairs(&[
            ("CREATE TABLE t1 (id INTEGER PRIMARY KEY)", 1),
            ("CREATE TABLE t2 (id INTEGER PRIMARY KEY)", 2),
        ]);
        assert_eq!(migrations.len(), 2);
        assert_eq!(migrations[0].version, 1);
        assert_eq!(migrations[1].version, 2);
    }
}
