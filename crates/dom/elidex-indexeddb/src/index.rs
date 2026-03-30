//! `IndexedDB` index operations.
//!
//! Indexes provide secondary access paths into object stores.
//! Each index has its own `SQLite` table mapping index-key → primary-key.

use rusqlite::{params, OptionalExtension};

use crate::backend::{BackendError, IdbBackend};
use crate::key::IdbKey;
use crate::key_range::IdbKeyRange;
use crate::util;

/// Metadata for an index.
#[derive(Debug, Clone)]
pub struct IndexMeta {
    pub name: String,
    pub key_path: String,
    pub unique: bool,
    pub multi_entry: bool,
}

/// Create an index on an object store.
///
/// Creates the index metadata row and a backing `SQLite` table.
/// If the store already has data, the index is populated from existing records.
pub fn create_index(
    backend: &IdbBackend,
    db_name: &str,
    store_name: &str,
    index_name: &str,
    key_path: &str,
    unique: bool,
    multi_entry: bool,
) -> Result<(), BackendError> {
    /// Maximum indexes per store (prevent resource exhaustion).
    const MAX_INDEXES: i64 = 100;

    // Check count limit
    let idx_count: i64 = backend.conn().query_row(
        "SELECT COUNT(*) FROM _idb_indexes WHERE db_name = ?1 AND store_name = ?2",
        params![db_name, store_name],
        |row| row.get(0),
    )?;
    if idx_count >= MAX_INDEXES {
        return Err(BackendError::ConstraintError(format!(
            "maximum number of indexes ({MAX_INDEXES}) per store reached"
        )));
    }

    // Check for duplicate
    let exists: bool = backend.conn().query_row(
        "SELECT COUNT(*) > 0 FROM _idb_indexes WHERE db_name = ?1 AND store_name = ?2 AND index_name = ?3",
        params![db_name, store_name, index_name],
        |row| row.get(0),
    )?;
    if exists {
        return Err(BackendError::ConstraintError(format!(
            "Index '{index_name}' already exists on store '{store_name}'"
        )));
    }

    backend
        .conn()
        .execute_batch("SAVEPOINT create_idx")
        .map_err(BackendError::from)?;

    let result = (|| -> Result<(), BackendError> {
        backend.conn().execute(
            "INSERT INTO _idb_indexes (db_name, store_name, index_name, key_path, is_unique, multi_entry) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![db_name, store_name, index_name, key_path, i32::from(unique), i32::from(multi_entry)],
        )?;

        let idx_table = util::index_table_name(db_name, store_name, index_name);
        let create_sql = if unique {
            format!("CREATE TABLE [{idx_table}] (index_key BLOB NOT NULL UNIQUE, primary_key BLOB NOT NULL)")
        } else {
            format!(
                "CREATE TABLE [{idx_table}] (index_key BLOB NOT NULL, primary_key BLOB NOT NULL)"
            )
        };
        backend.conn().execute_batch(&create_sql)?;

        // Populate from existing records
        populate_index(
            backend,
            db_name,
            store_name,
            index_name,
            key_path,
            multi_entry,
        )?;

        Ok(())
    })();

    match result {
        Ok(()) => {
            backend
                .conn()
                .execute_batch("RELEASE create_idx")
                .map_err(BackendError::from)?;
            Ok(())
        }
        Err(e) => {
            let _ = backend.conn().execute_batch("ROLLBACK TO create_idx");
            let _ = backend.conn().execute_batch("RELEASE create_idx");
            Err(e)
        }
    }
}

/// Delete an index.
pub fn delete_index(
    backend: &IdbBackend,
    db_name: &str,
    store_name: &str,
    index_name: &str,
) -> Result<(), BackendError> {
    let deleted = backend.conn().execute(
        "DELETE FROM _idb_indexes WHERE db_name = ?1 AND store_name = ?2 AND index_name = ?3",
        params![db_name, store_name, index_name],
    )?;
    if deleted == 0 {
        return Err(BackendError::NotFoundError(format!(
            "Index '{index_name}' not found on store '{store_name}'"
        )));
    }

    let idx_table = util::index_table_name(db_name, store_name, index_name);
    backend
        .conn()
        .execute(&format!("DROP TABLE IF EXISTS [{idx_table}]"), [])?;
    Ok(())
}

/// Rename an index (W3C §4.6 `IDBIndex.name` setter).
pub fn rename_index(
    backend: &IdbBackend,
    db_name: &str,
    store_name: &str,
    old_name: &str,
    new_name: &str,
) -> Result<(), BackendError> {
    let old_table = util::index_table_name(db_name, store_name, old_name);
    let new_table = util::index_table_name(db_name, store_name, new_name);
    backend.conn().execute(
        "UPDATE _idb_indexes SET index_name = ?4 WHERE db_name = ?1 AND store_name = ?2 AND index_name = ?3",
        params![db_name, store_name, old_name, new_name],
    )?;
    backend.conn().execute_batch(&format!(
        "ALTER TABLE [{old_table}] RENAME TO [{new_table}]"
    ))?;
    Ok(())
}

/// Get the first value matching an index key.
pub fn index_get(
    backend: &IdbBackend,
    db_name: &str,
    store_name: &str,
    index_name: &str,
    key: &IdbKey,
) -> Result<Option<String>, BackendError> {
    let idx_table = util::index_table_name(db_name, store_name, index_name);
    let data_table = backend.data_table(db_name, store_name);
    let index_key_bytes = key.serialize();

    let result: Option<String> = backend
        .conn()
        .query_row(
            &format!(
                "SELECT d.value FROM [{idx_table}] i JOIN [{data_table}] d ON i.primary_key = d.key_data WHERE i.index_key = ?1 ORDER BY i.primary_key ASC LIMIT 1"
            ),
            params![index_key_bytes],
            |row| row.get(0),
        )
        .optional()?;
    Ok(result)
}

/// Get the primary key for an index key lookup.
pub fn index_get_key(
    backend: &IdbBackend,
    db_name: &str,
    store_name: &str,
    index_name: &str,
    key: &IdbKey,
) -> Result<Option<IdbKey>, BackendError> {
    let idx_table = util::index_table_name(db_name, store_name, index_name);
    let index_key_bytes = key.serialize();

    let result: Option<Vec<u8>> = backend
        .conn()
        .query_row(
            &format!("SELECT primary_key FROM [{idx_table}] WHERE index_key = ?1 ORDER BY primary_key ASC LIMIT 1"),
            params![index_key_bytes],
            |row| row.get(0),
        )
        .optional()?;
    Ok(result.and_then(|b| IdbKey::deserialize(&b)))
}

/// Get all records matching an index key range.
pub fn index_get_all(
    backend: &IdbBackend,
    db_name: &str,
    store_name: &str,
    index_name: &str,
    range: Option<&IdbKeyRange>,
    count: Option<u32>,
) -> Result<Vec<(IdbKey, String)>, BackendError> {
    let idx_table = util::index_table_name(db_name, store_name, index_name);
    let data_table = backend.data_table(db_name, store_name);
    let (where_clause, where_params) = range_to_index_where(range);
    let limit_clause = count.map_or_else(String::new, |c| format!(" LIMIT {c}"));

    let sql = format!(
        "SELECT i.primary_key, d.value FROM [{idx_table}] i JOIN [{data_table}] d ON i.primary_key = d.key_data WHERE {where_clause} ORDER BY i.index_key ASC, i.primary_key ASC{limit_clause}"
    );

    let mut stmt = backend.conn().prepare(&sql)?;
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = where_params
        .iter()
        .map(|p| p as &dyn rusqlite::types::ToSql)
        .collect();

    let rows = stmt
        .query_map(param_refs.as_slice(), |row| {
            let pk_bytes: Vec<u8> = row.get(0)?;
            let value: String = row.get(1)?;
            Ok((pk_bytes, value))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut results = Vec::with_capacity(rows.len());
    for (pk, val) in rows {
        if let Some(key) = IdbKey::deserialize(&pk) {
            results.push((key, val));
        }
    }
    Ok(results)
}

/// Get all primary keys matching an index key range.
pub fn index_get_all_keys(
    backend: &IdbBackend,
    db_name: &str,
    store_name: &str,
    index_name: &str,
    range: Option<&IdbKeyRange>,
    count: Option<u32>,
) -> Result<Vec<IdbKey>, BackendError> {
    let idx_table = util::index_table_name(db_name, store_name, index_name);
    let (where_clause, where_params) = range_to_plain_where(range);
    let limit_clause = count.map_or_else(String::new, |c| format!(" LIMIT {c}"));

    let sql = format!(
        "SELECT primary_key FROM [{idx_table}] WHERE {where_clause} ORDER BY index_key ASC{limit_clause}"
    );

    let mut stmt = backend.conn().prepare(&sql)?;
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = where_params
        .iter()
        .map(|p| p as &dyn rusqlite::types::ToSql)
        .collect();

    let rows = stmt
        .query_map(param_refs.as_slice(), |row| {
            let pk_bytes: Vec<u8> = row.get(0)?;
            Ok(pk_bytes)
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(rows
        .into_iter()
        .filter_map(|b| IdbKey::deserialize(&b))
        .collect())
}

/// Count index entries matching a key range.
pub fn index_count(
    backend: &IdbBackend,
    db_name: &str,
    store_name: &str,
    index_name: &str,
    range: Option<&IdbKeyRange>,
) -> Result<u64, BackendError> {
    let idx_table = util::index_table_name(db_name, store_name, index_name);
    let (where_clause, where_params) = range_to_plain_where(range);

    let sql = format!("SELECT COUNT(*) FROM [{idx_table}] WHERE {where_clause}");
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = where_params
        .iter()
        .map(|p| p as &dyn rusqlite::types::ToSql)
        .collect();

    let n: i64 = backend
        .conn()
        .query_row(&sql, param_refs.as_slice(), |row| row.get(0))?;
    #[allow(clippy::cast_sign_loss)]
    Ok(n as u64)
}

// -- Index maintenance (called by ops on put/add/delete) --

/// Update all indexes for a store after a record is inserted or updated.
///
/// Returns early if the store has no indexes (avoids unnecessary metadata query).
pub fn update_indexes_for_put(
    backend: &IdbBackend,
    db_name: &str,
    store_name: &str,
    primary_key: &IdbKey,
    value: &str,
) -> Result<(), BackendError> {
    let indexes = list_index_metas(backend, db_name, store_name)?;
    if indexes.is_empty() {
        return Ok(());
    }
    let pk_bytes = primary_key.serialize();

    for idx in &indexes {
        let idx_table = util::index_table_name(db_name, store_name, &idx.name);
        // Remove old entries for this primary key
        backend.conn().execute(
            &format!("DELETE FROM [{idx_table}] WHERE primary_key = ?1"),
            params![pk_bytes],
        )?;

        // Extract index key(s) from value
        let index_keys = extract_index_keys(value, &idx.key_path, idx.multi_entry);
        for ik in &index_keys {
            let ik_bytes = ik.serialize();
            backend
                .conn()
                .execute(
                    &format!("INSERT INTO [{idx_table}] (index_key, primary_key) VALUES (?1, ?2)"),
                    params![ik_bytes, pk_bytes],
                )
                .map_err(|e| match e {
                    rusqlite::Error::SqliteFailure(err, _)
                        if err.code == rusqlite::ErrorCode::ConstraintViolation && idx.unique =>
                    {
                        BackendError::ConstraintError(format!(
                            "Unique index '{}' constraint violation",
                            idx.name
                        ))
                    }
                    other => BackendError::from(other),
                })?;
        }
    }
    Ok(())
}

/// Remove all index entries for a primary key.
pub fn remove_indexes_for_delete(
    backend: &IdbBackend,
    db_name: &str,
    store_name: &str,
    primary_key: &IdbKey,
) -> Result<(), BackendError> {
    let indexes = list_index_metas(backend, db_name, store_name)?;
    if indexes.is_empty() {
        return Ok(());
    }
    let pk_bytes = primary_key.serialize();

    for idx in &indexes {
        let idx_table = util::index_table_name(db_name, store_name, &idx.name);
        backend.conn().execute(
            &format!("DELETE FROM [{idx_table}] WHERE primary_key = ?1"),
            params![pk_bytes],
        )?;
    }
    Ok(())
}

/// Remove index entries for all records matching a key range (batch operation).
///
/// Uses a single SQL `DELETE ... WHERE primary_key IN (SELECT ...)` per index
/// instead of per-key deletion, avoiding N+1 queries.
pub fn remove_indexes_for_range(
    backend: &IdbBackend,
    db_name: &str,
    store_name: &str,
    range: &crate::key_range::IdbKeyRange,
) -> Result<(), BackendError> {
    let indexes = list_index_metas(backend, db_name, store_name)?;
    if indexes.is_empty() {
        return Ok(());
    }

    let data_table = backend.data_table(db_name, store_name);
    let (where_clause, where_params) = range.to_sql_clause("key_data");

    for idx in &indexes {
        let idx_table = util::index_table_name(db_name, store_name, &idx.name);
        let sql = format!(
            "DELETE FROM [{idx_table}] WHERE primary_key IN (SELECT key_data FROM [{data_table}] WHERE {where_clause})"
        );
        let param_refs: Vec<&dyn rusqlite::types::ToSql> = where_params
            .iter()
            .map(|p| p as &dyn rusqlite::types::ToSql)
            .collect();
        backend.conn().execute(&sql, param_refs.as_slice())?;
    }
    Ok(())
}

/// Clear all index tables for a store.
pub fn clear_indexes(
    backend: &IdbBackend,
    db_name: &str,
    store_name: &str,
) -> Result<(), BackendError> {
    let index_names = backend.list_index_names(db_name, store_name)?;
    for idx_name in &index_names {
        let idx_table = util::index_table_name(db_name, store_name, idx_name);
        backend
            .conn()
            .execute(&format!("DELETE FROM [{idx_table}]"), [])?;
    }
    Ok(())
}

// -- Helpers --

fn list_index_metas(
    backend: &IdbBackend,
    db_name: &str,
    store_name: &str,
) -> Result<Vec<IndexMeta>, BackendError> {
    let mut stmt = backend.conn().prepare(
        "SELECT index_name, key_path, is_unique, multi_entry FROM _idb_indexes WHERE db_name = ?1 AND store_name = ?2",
    )?;
    let metas = stmt
        .query_map(params![db_name, store_name], |row| {
            let name: String = row.get(0)?;
            let key_path: String = row.get(1)?;
            let unique: i32 = row.get(2)?;
            let multi: i32 = row.get(3)?;
            Ok(IndexMeta {
                name,
                key_path,
                unique: unique != 0,
                multi_entry: multi != 0,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(metas)
}

/// Get metadata for a specific index.
pub fn get_index_meta(
    backend: &IdbBackend,
    db_name: &str,
    store_name: &str,
    index_name: &str,
) -> Result<IndexMeta, BackendError> {
    backend
        .conn()
        .query_row(
            "SELECT key_path, is_unique, multi_entry FROM _idb_indexes WHERE db_name = ?1 AND store_name = ?2 AND index_name = ?3",
            params![db_name, store_name, index_name],
            |row| {
                let key_path: String = row.get(0)?;
                let unique: i32 = row.get(1)?;
                let multi: i32 = row.get(2)?;
                Ok(IndexMeta {
                    name: index_name.to_owned(),
                    key_path,
                    unique: unique != 0,
                    multi_entry: multi != 0,
                })
            },
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => BackendError::NotFoundError(format!(
                "Index '{index_name}' not found on store '{store_name}'"
            )),
            other => BackendError::from(other),
        })
}

/// Extract index keys from a JSON value.
///
/// For `multi_entry`: if the value at `key_path` is an array, each element
/// becomes a separate index entry.
fn extract_index_keys(value: &str, key_path: &str, multi_entry: bool) -> Vec<IdbKey> {
    let Some(parsed) = serde_json::from_str::<serde_json::Value>(value).ok() else {
        return Vec::new();
    };
    let Some(resolved) = util::resolve_path(&parsed, key_path) else {
        return Vec::new();
    };

    if multi_entry {
        if let serde_json::Value::Array(arr) = resolved {
            let mut keys: Vec<IdbKey> = arr.iter().filter_map(util::json_to_idb_key).collect();
            keys.sort();
            keys.dedup();
            return keys;
        }
    }

    util::json_to_idb_key(resolved).into_iter().collect()
}

/// Populate an index from all existing records in the store.
fn populate_index(
    backend: &IdbBackend,
    db_name: &str,
    store_name: &str,
    index_name: &str,
    key_path: &str,
    multi_entry: bool,
) -> Result<(), BackendError> {
    let data_table = backend.data_table(db_name, store_name);
    let idx_table = util::index_table_name(db_name, store_name, index_name);

    let mut stmt = backend
        .conn()
        .prepare(&format!("SELECT key_data, value FROM [{data_table}]"))?;
    let rows = stmt
        .query_map([], |row| {
            let pk: Vec<u8> = row.get(0)?;
            let val: String = row.get(1)?;
            Ok((pk, val))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    // Look up whether this index is unique (needed for error mapping).
    let is_unique = get_index_meta(backend, db_name, store_name, index_name)
        .map(|m| m.unique)
        .unwrap_or(false);

    for (pk_bytes, value) in &rows {
        let index_keys = extract_index_keys(value, key_path, multi_entry);
        for ik in &index_keys {
            let ik_bytes = ik.serialize();
            backend
                .conn()
                .execute(
                    &format!("INSERT INTO [{idx_table}] (index_key, primary_key) VALUES (?1, ?2)"),
                    params![ik_bytes, pk_bytes],
                )
                .map_err(|e| match e {
                    rusqlite::Error::SqliteFailure(err, _)
                        if err.code == rusqlite::ErrorCode::ConstraintViolation && is_unique =>
                    {
                        BackendError::ConstraintError(format!(
                            "Unique index '{index_name}' constraint violation during population"
                        ))
                    }
                    other => BackendError::from(other),
                })?;
        }
    }
    Ok(())
}

/// Public accessor for the index table name (used by cursor module).
pub fn index_table_name_for(db_name: &str, store_name: &str, index_name: &str) -> String {
    util::index_table_name(db_name, store_name, index_name)
}

fn range_to_index_where(range: Option<&IdbKeyRange>) -> (String, Vec<Vec<u8>>) {
    range.map_or_else(
        || ("1=1".to_owned(), Vec::new()),
        |r| r.to_sql_clause("i.index_key"),
    )
}

/// For queries on the index table without a table alias.
fn range_to_plain_where(range: Option<&IdbKeyRange>) -> (String, Vec<Vec<u8>>) {
    range.map_or_else(
        || ("1=1".to_owned(), Vec::new()),
        |r| r.to_sql_clause("index_key"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops;

    fn setup() -> IdbBackend {
        let b = IdbBackend::open_in_memory().unwrap();
        b.set_version("db", 1).unwrap();
        b.create_object_store("db", "users", Some("id"), false)
            .unwrap();
        b
    }

    fn insert_user(b: &IdbBackend, id: f64, name: &str, age: f64) {
        let value = format!(r#"{{"id":{id},"name":"{name}","age":{age}}}"#);
        let key = ops::put(b, "db", "users", None, &value).unwrap();
        update_indexes_for_put(b, "db", "users", &key, &value).unwrap();
    }

    #[test]
    fn create_and_delete_index() {
        let b = setup();
        create_index(&b, "db", "users", "by_name", "name", false, false).unwrap();
        let names = b.list_index_names("db", "users").unwrap();
        assert_eq!(names, vec!["by_name"]);

        delete_index(&b, "db", "users", "by_name").unwrap();
        assert!(b.list_index_names("db", "users").unwrap().is_empty());
    }

    #[test]
    fn create_duplicate_index_fails() {
        let b = setup();
        create_index(&b, "db", "users", "by_name", "name", false, false).unwrap();
        let err = create_index(&b, "db", "users", "by_name", "name", false, false);
        assert!(matches!(err, Err(BackendError::ConstraintError(_))));
    }

    #[test]
    fn delete_nonexistent_index_fails() {
        let b = setup();
        let err = delete_index(&b, "db", "users", "nope");
        assert!(matches!(err, Err(BackendError::NotFoundError(_))));
    }

    #[test]
    fn index_get_by_name() {
        let b = setup();
        create_index(&b, "db", "users", "by_name", "name", false, false).unwrap();

        insert_user(&b, 1.0, "alice", 30.0);
        insert_user(&b, 2.0, "bob", 25.0);

        let val = index_get(
            &b,
            "db",
            "users",
            "by_name",
            &IdbKey::String("alice".into()),
        )
        .unwrap();
        assert!(val.is_some());
        assert!(val.unwrap().contains("alice"));

        let val = index_get(
            &b,
            "db",
            "users",
            "by_name",
            &IdbKey::String("charlie".into()),
        )
        .unwrap();
        assert!(val.is_none());
    }

    #[test]
    fn index_get_key_returns_primary_key() {
        let b = setup();
        create_index(&b, "db", "users", "by_name", "name", false, false).unwrap();
        insert_user(&b, 42.0, "eve", 28.0);

        let pk =
            index_get_key(&b, "db", "users", "by_name", &IdbKey::String("eve".into())).unwrap();
        assert_eq!(pk, Some(IdbKey::Number(42.0)));
    }

    #[test]
    fn index_get_all_with_range() {
        let b = setup();
        create_index(&b, "db", "users", "by_age", "age", false, false).unwrap();

        insert_user(&b, 1.0, "alice", 30.0);
        insert_user(&b, 2.0, "bob", 25.0);
        insert_user(&b, 3.0, "charlie", 35.0);

        let range =
            IdbKeyRange::bound(IdbKey::Number(25.0), IdbKey::Number(30.0), false, false).unwrap();
        let results = index_get_all(&b, "db", "users", "by_age", Some(&range), None).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn index_get_all_keys_ordered() {
        let b = setup();
        create_index(&b, "db", "users", "by_age", "age", false, false).unwrap();

        insert_user(&b, 1.0, "alice", 30.0);
        insert_user(&b, 2.0, "bob", 25.0);

        let keys = index_get_all_keys(&b, "db", "users", "by_age", None, None).unwrap();
        // Ordered by index key (age): 25 (id=2) then 30 (id=1)
        assert_eq!(keys, vec![IdbKey::Number(2.0), IdbKey::Number(1.0)]);
    }

    #[test]
    fn index_count_with_range() {
        let b = setup();
        create_index(&b, "db", "users", "by_age", "age", false, false).unwrap();

        insert_user(&b, 1.0, "alice", 30.0);
        insert_user(&b, 2.0, "bob", 25.0);
        insert_user(&b, 3.0, "charlie", 35.0);

        assert_eq!(index_count(&b, "db", "users", "by_age", None).unwrap(), 3);

        let range = IdbKeyRange::upper_bound(IdbKey::Number(30.0), true);
        assert_eq!(
            index_count(&b, "db", "users", "by_age", Some(&range)).unwrap(),
            1
        );
    }

    #[test]
    fn unique_index_rejects_duplicates() {
        let b = setup();
        create_index(&b, "db", "users", "by_name", "name", true, false).unwrap();

        insert_user(&b, 1.0, "alice", 30.0);

        let value = r#"{"id":2,"name":"alice","age":25}"#;
        let key = ops::put(&b, "db", "users", None, value).unwrap();
        let err = update_indexes_for_put(&b, "db", "users", &key, value);
        assert!(matches!(err, Err(BackendError::ConstraintError(_))));
    }

    #[test]
    fn remove_indexes_on_delete() {
        let b = setup();
        create_index(&b, "db", "users", "by_name", "name", false, false).unwrap();

        insert_user(&b, 1.0, "alice", 30.0);
        assert_eq!(index_count(&b, "db", "users", "by_name", None).unwrap(), 1);

        remove_indexes_for_delete(&b, "db", "users", &IdbKey::Number(1.0)).unwrap();
        assert_eq!(index_count(&b, "db", "users", "by_name", None).unwrap(), 0);
    }

    #[test]
    fn multi_entry_index() {
        let b = IdbBackend::open_in_memory().unwrap();
        b.set_version("db", 1).unwrap();
        b.create_object_store("db", "articles", Some("id"), false)
            .unwrap();
        create_index(&b, "db", "articles", "by_tag", "tags", false, true).unwrap();

        let value = r#"{"id":1,"tags":["rust","wasm","browser"]}"#;
        let key = ops::put(&b, "db", "articles", None, value).unwrap();
        update_indexes_for_put(&b, "db", "articles", &key, value).unwrap();

        // Each tag should be an index entry
        assert_eq!(
            index_count(&b, "db", "articles", "by_tag", None).unwrap(),
            3
        );

        // Lookup by individual tag
        let val = index_get(
            &b,
            "db",
            "articles",
            "by_tag",
            &IdbKey::String("wasm".into()),
        )
        .unwrap();
        assert!(val.is_some());
    }

    #[test]
    fn multi_entry_non_array_treated_as_single() {
        let b = IdbBackend::open_in_memory().unwrap();
        b.set_version("db", 1).unwrap();
        b.create_object_store("db", "items", Some("id"), false)
            .unwrap();
        create_index(&b, "db", "items", "by_cat", "category", false, true).unwrap();

        let value = r#"{"id":1,"category":"tools"}"#;
        let key = ops::put(&b, "db", "items", None, value).unwrap();
        update_indexes_for_put(&b, "db", "items", &key, value).unwrap();

        assert_eq!(index_count(&b, "db", "items", "by_cat", None).unwrap(), 1);
    }

    #[test]
    fn index_populated_from_existing_data() {
        let b = setup();
        // Insert data first, then create index
        insert_user(&b, 1.0, "alice", 30.0);
        insert_user(&b, 2.0, "bob", 25.0);

        // No indexes exist yet — insert_user's update_indexes_for_put is a no-op
        // Now create index — it should populate from existing data
        create_index(&b, "db", "users", "by_name", "name", false, false).unwrap();

        assert_eq!(index_count(&b, "db", "users", "by_name", None).unwrap(), 2);
        let val = index_get(&b, "db", "users", "by_name", &IdbKey::String("bob".into())).unwrap();
        assert!(val.is_some());
    }

    #[test]
    fn get_index_meta_returns_metadata() {
        let b = setup();
        create_index(&b, "db", "users", "by_name", "name", true, false).unwrap();

        let meta = get_index_meta(&b, "db", "users", "by_name").unwrap();
        assert_eq!(meta.name, "by_name");
        assert_eq!(meta.key_path, "name");
        assert!(meta.unique);
        assert!(!meta.multi_entry);
    }

    #[test]
    fn multi_entry_deduplicates_sub_keys() {
        let b = IdbBackend::open_in_memory().unwrap();
        b.set_version("db", 1).unwrap();
        b.create_object_store("db", "articles", Some("id"), false)
            .unwrap();
        create_index(&b, "db", "articles", "by_tag", "tags", false, true).unwrap();

        // Array with duplicate "rust" entries — should produce only 3 index entries
        let value = r#"{"id":1,"tags":["rust","wasm","rust","browser"]}"#;
        let key = ops::put(&b, "db", "articles", None, value).unwrap();
        update_indexes_for_put(&b, "db", "articles", &key, value).unwrap();

        assert_eq!(
            index_count(&b, "db", "articles", "by_tag", None).unwrap(),
            3
        );
    }

    #[test]
    fn clear_indexes_empties_all() {
        let b = setup();
        create_index(&b, "db", "users", "by_name", "name", false, false).unwrap();
        create_index(&b, "db", "users", "by_age", "age", false, false).unwrap();

        insert_user(&b, 1.0, "alice", 30.0);
        assert_eq!(index_count(&b, "db", "users", "by_name", None).unwrap(), 1);
        assert_eq!(index_count(&b, "db", "users", "by_age", None).unwrap(), 1);

        clear_indexes(&b, "db", "users").unwrap();
        assert_eq!(index_count(&b, "db", "users", "by_name", None).unwrap(), 0);
        assert_eq!(index_count(&b, "db", "users", "by_age", None).unwrap(), 0);
    }
}
