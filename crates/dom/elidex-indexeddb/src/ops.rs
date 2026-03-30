//! CRUD operations on `IndexedDB` object stores.
//!
//! All operations work on serialized `IdbKey` BLOBs in `SQLite`,
//! preserving W3C key ordering via lexicographic byte comparison.

use rusqlite::params;

use crate::backend::{BackendError, IdbBackend};
use crate::key::IdbKey;
use crate::key_range::IdbKeyRange;

use crate::util;

/// Extract a key from a JSON value using a dot-separated key path.
///
/// Returns `None` if the path doesn't resolve to a valid IDB key type.
pub fn extract_key_from_value(value: &str, key_path: &str) -> Option<IdbKey> {
    let parsed: serde_json::Value = serde_json::from_str(value).ok()?;
    let resolved = util::resolve_path(&parsed, key_path)?;
    util::json_to_idb_key(resolved)
}

/// Inject a key into a JSON value at the given key path.
///
/// Creates intermediate objects as needed. Returns the modified JSON string.
pub fn inject_key_into_value(value: &str, key_path: &str, key: &IdbKey) -> Option<String> {
    let mut parsed: serde_json::Value = serde_json::from_str(value).ok()?;
    let key_json = util::idb_key_to_json(key);
    set_path(&mut parsed, key_path, key_json)?;
    serde_json::to_string(&parsed).ok()
}

fn set_path(val: &mut serde_json::Value, path: &str, new_val: serde_json::Value) -> Option<()> {
    let segments: Vec<&str> = path.split('.').collect();
    let mut current = val;
    for &seg in &segments[..segments.len() - 1] {
        if !current.is_object() {
            return None;
        }
        let obj = current.as_object_mut()?;
        if !obj.contains_key(seg) {
            obj.insert(
                seg.to_owned(),
                serde_json::Value::Object(serde_json::Map::default()),
            );
        }
        current = obj.get_mut(seg)?;
    }
    let obj = current.as_object_mut()?;
    obj.insert(segments.last()?.to_string(), new_val);
    Some(())
}

/// Resolve the effective key for a put/add operation.
///
/// Priority: explicit key > key extracted from value via `key_path` > auto-increment.
pub fn resolve_key(
    backend: &IdbBackend,
    db_name: &str,
    store_name: &str,
    explicit_key: Option<IdbKey>,
    value: &str,
) -> Result<(IdbKey, Option<String>), BackendError> {
    let (key_path, auto_increment) = backend.get_store_meta(db_name, store_name)?;

    // 1. Explicit key provided
    if let Some(key) = explicit_key {
        // W3C §4.5: If store uses in-line keys and key was given, throw DataError.
        if key_path.is_some() {
            return Err(BackendError::DataError(
                "explicit key not allowed for stores with keyPath".into(),
            ));
        }
        // If auto-increment, bump the counter
        if auto_increment {
            backend.maybe_bump_auto_key(db_name, store_name, &key)?;
        }
        return Ok((key, None));
    }

    // 2. Extract key from value via key_path
    if let Some(kp) = &key_path {
        if let Some(key) = extract_key_from_value(value, kp) {
            if auto_increment {
                backend.maybe_bump_auto_key(db_name, store_name, &key)?;
            }
            return Ok((key, None));
        }
        // key_path set but key not found in value — fall through to auto-increment
    }

    // 3. Auto-increment
    if auto_increment {
        let key = backend.next_auto_key(db_name, store_name)?;
        // If key_path exists, inject auto key into value
        if let Some(kp) = &key_path {
            let new_value = inject_key_into_value(value, kp, &key).ok_or_else(|| {
                BackendError::Internal("Failed to inject auto key into value".into())
            })?;
            return Ok((key, Some(new_value)));
        }
        return Ok((key, None));
    }

    Err(BackendError::DataError(
        "no key provided and store has no keyPath or autoIncrement".into(),
    ))
}

/// INSERT OR REPLACE a record. Returns the inserted key.
pub fn put(
    backend: &IdbBackend,
    db_name: &str,
    store_name: &str,
    key: Option<IdbKey>,
    value: &str,
) -> Result<IdbKey, BackendError> {
    let (resolved_key, new_value) = resolve_key(backend, db_name, store_name, key, value)?;
    let final_value = new_value.as_deref().unwrap_or(value);
    let key_bytes = resolved_key.serialize();
    let table = backend.data_table(db_name, store_name);

    backend.conn().execute(
        &format!("INSERT OR REPLACE INTO [{table}] (key_data, value) VALUES (?1, ?2)"),
        params![key_bytes, final_value],
    )?;

    Ok(resolved_key)
}

/// INSERT a record (fails on duplicate key with `ConstraintError`). Returns the inserted key.
pub fn add(
    backend: &IdbBackend,
    db_name: &str,
    store_name: &str,
    key: Option<IdbKey>,
    value: &str,
) -> Result<IdbKey, BackendError> {
    let (resolved_key, new_value) = resolve_key(backend, db_name, store_name, key, value)?;
    let final_value = new_value.as_deref().unwrap_or(value);
    let key_bytes = resolved_key.serialize();
    let table = backend.data_table(db_name, store_name);

    backend
        .conn()
        .execute(
            &format!("INSERT INTO [{table}] (key_data, value) VALUES (?1, ?2)"),
            params![key_bytes, final_value],
        )
        .map_err(|e| match e {
            rusqlite::Error::SqliteFailure(err, _)
                if err.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                BackendError::ConstraintError("Key already exists".into())
            }
            other => BackendError::from(other),
        })?;

    Ok(resolved_key)
}

/// Get a value by exact key.
pub fn get(
    backend: &IdbBackend,
    db_name: &str,
    store_name: &str,
    key: &IdbKey,
) -> Result<Option<String>, BackendError> {
    let key_bytes = key.serialize();
    let table = backend.data_table(db_name, store_name);

    let result: Option<String> = backend
        .conn()
        .query_row(
            &format!("SELECT value FROM [{table}] WHERE key_data = ?1"),
            params![key_bytes],
            |row| row.get(0),
        )
        .optional()?;
    Ok(result)
}

/// Check if a key exists (returns the key itself if found).
pub fn get_key(
    backend: &IdbBackend,
    db_name: &str,
    store_name: &str,
    key: &IdbKey,
) -> Result<Option<IdbKey>, BackendError> {
    let key_bytes = key.serialize();
    let table = backend.data_table(db_name, store_name);

    let result: Option<Vec<u8>> = backend
        .conn()
        .query_row(
            &format!("SELECT key_data FROM [{table}] WHERE key_data = ?1"),
            params![key_bytes],
            |row| row.get(0),
        )
        .optional()?;
    Ok(result.and_then(|b| IdbKey::deserialize(&b)))
}

/// Get all records matching a key range, with optional count limit.
pub fn get_all(
    backend: &IdbBackend,
    db_name: &str,
    store_name: &str,
    range: Option<&IdbKeyRange>,
    count: Option<u32>,
) -> Result<Vec<(IdbKey, String)>, BackendError> {
    let table = backend.data_table(db_name, store_name);
    let (where_clause, where_params) = range_to_where(range);
    let limit_clause = count.map_or_else(String::new, |c| format!(" LIMIT {c}"));

    let sql = format!(
        "SELECT key_data, value FROM [{table}] WHERE {where_clause} ORDER BY key_data ASC{limit_clause}"
    );

    let mut stmt = backend.conn().prepare(&sql)?;
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = where_params
        .iter()
        .map(|p| p as &dyn rusqlite::types::ToSql)
        .collect();

    let rows = stmt
        .query_map(param_refs.as_slice(), |row| {
            let key_bytes: Vec<u8> = row.get(0)?;
            let value: String = row.get(1)?;
            Ok((key_bytes, value))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut results = Vec::with_capacity(rows.len());
    for (kb, val) in rows {
        if let Some(key) = IdbKey::deserialize(&kb) {
            results.push((key, val));
        }
    }
    Ok(results)
}

/// Get all keys matching a key range, with optional count limit.
pub fn get_all_keys(
    backend: &IdbBackend,
    db_name: &str,
    store_name: &str,
    range: Option<&IdbKeyRange>,
    count: Option<u32>,
) -> Result<Vec<IdbKey>, BackendError> {
    let table = backend.data_table(db_name, store_name);
    let (where_clause, where_params) = range_to_where(range);
    let limit_clause = count.map_or_else(String::new, |c| format!(" LIMIT {c}"));

    let sql = format!(
        "SELECT key_data FROM [{table}] WHERE {where_clause} ORDER BY key_data ASC{limit_clause}"
    );

    let mut stmt = backend.conn().prepare(&sql)?;
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = where_params
        .iter()
        .map(|p| p as &dyn rusqlite::types::ToSql)
        .collect();

    let rows = stmt
        .query_map(param_refs.as_slice(), |row| {
            let key_bytes: Vec<u8> = row.get(0)?;
            Ok(key_bytes)
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(rows
        .into_iter()
        .filter_map(|kb| IdbKey::deserialize(&kb))
        .collect())
}

/// Delete a single key or all keys in a range.
pub fn delete(
    backend: &IdbBackend,
    db_name: &str,
    store_name: &str,
    target: &DeleteTarget,
) -> Result<(), BackendError> {
    let table = backend.data_table(db_name, store_name);
    match target {
        DeleteTarget::Key(key) => {
            let key_bytes = key.serialize();
            backend.conn().execute(
                &format!("DELETE FROM [{table}] WHERE key_data = ?1"),
                params![key_bytes],
            )?;
        }
        DeleteTarget::Range(range) => {
            let (where_clause, where_params) = range.to_sql_clause("key_data");
            let param_refs: Vec<&dyn rusqlite::types::ToSql> = where_params
                .iter()
                .map(|p| p as &dyn rusqlite::types::ToSql)
                .collect();
            backend.conn().execute(
                &format!("DELETE FROM [{table}] WHERE {where_clause}"),
                param_refs.as_slice(),
            )?;
        }
    }
    Ok(())
}

/// Clear all records from an object store.
pub fn clear(backend: &IdbBackend, db_name: &str, store_name: &str) -> Result<(), BackendError> {
    let table = backend.data_table(db_name, store_name);
    backend
        .conn()
        .execute(&format!("DELETE FROM [{table}]"), [])?;
    Ok(())
}

/// Count records matching a key range (or all if `None`).
pub fn count(
    backend: &IdbBackend,
    db_name: &str,
    store_name: &str,
    range: Option<&IdbKeyRange>,
) -> Result<u64, BackendError> {
    let table = backend.data_table(db_name, store_name);
    let (where_clause, where_params) = range_to_where(range);

    let sql = format!("SELECT COUNT(*) FROM [{table}] WHERE {where_clause}");
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

/// Target for a delete operation: a single key or a key range.
pub enum DeleteTarget {
    Key(IdbKey),
    Range(IdbKeyRange),
}

/// Convert an optional key range to a SQL WHERE clause + params.
fn range_to_where(range: Option<&IdbKeyRange>) -> (String, Vec<Vec<u8>>) {
    range.map_or_else(
        || ("1=1".to_owned(), Vec::new()),
        |r| r.to_sql_clause("key_data"),
    )
}

use rusqlite::OptionalExtension;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::IdbBackend;

    fn setup() -> IdbBackend {
        let b = IdbBackend::open_in_memory().unwrap();
        b.set_version("db", 1).unwrap();
        b
    }

    #[test]
    fn put_and_get_explicit_key() {
        let b = setup();
        b.create_object_store("db", "s", None, false).unwrap();

        let key = IdbKey::Number(1.0);
        put(&b, "db", "s", Some(key.clone()), r#"{"name":"alice"}"#).unwrap();

        let val = get(&b, "db", "s", &key).unwrap();
        assert_eq!(val.as_deref(), Some(r#"{"name":"alice"}"#));
    }

    #[test]
    fn put_overwrites_existing() {
        let b = setup();
        b.create_object_store("db", "s", None, false).unwrap();

        let key = IdbKey::Number(1.0);
        put(&b, "db", "s", Some(key.clone()), r#""v1""#).unwrap();
        put(&b, "db", "s", Some(key.clone()), r#""v2""#).unwrap();

        let val = get(&b, "db", "s", &key).unwrap();
        assert_eq!(val.as_deref(), Some(r#""v2""#));
    }

    #[test]
    fn add_duplicate_fails() {
        let b = setup();
        b.create_object_store("db", "s", None, false).unwrap();

        let key = IdbKey::String("k".into());
        add(&b, "db", "s", Some(key.clone()), r#""v1""#).unwrap();
        let err = add(&b, "db", "s", Some(key), r#""v2""#);
        assert!(matches!(err, Err(BackendError::ConstraintError(_))));
    }

    #[test]
    fn auto_increment_put() {
        let b = setup();
        b.create_object_store("db", "s", None, true).unwrap();

        let k1 = put(&b, "db", "s", None, r#""a""#).unwrap();
        let k2 = put(&b, "db", "s", None, r#""b""#).unwrap();
        let k3 = put(&b, "db", "s", None, r#""c""#).unwrap();

        assert_eq!(k1, IdbKey::Number(1.0));
        assert_eq!(k2, IdbKey::Number(2.0));
        assert_eq!(k3, IdbKey::Number(3.0));
    }

    #[test]
    fn key_path_extraction() {
        let b = setup();
        b.create_object_store("db", "s", Some("id"), false).unwrap();

        let k = put(&b, "db", "s", None, r#"{"id":42,"name":"bob"}"#).unwrap();
        assert_eq!(k, IdbKey::Number(42.0));

        let val = get(&b, "db", "s", &IdbKey::Number(42.0)).unwrap();
        assert!(val.is_some());
    }

    #[test]
    fn key_path_auto_increment_injection() {
        let b = setup();
        b.create_object_store("db", "s", Some("id"), true).unwrap();

        // No "id" in value — auto-increment should inject it
        let k = put(&b, "db", "s", None, r#"{"name":"eve"}"#).unwrap();
        assert_eq!(k, IdbKey::Number(1.0));

        let val = get(&b, "db", "s", &IdbKey::Number(1.0)).unwrap().unwrap();
        assert!(val.contains(r#""id":1"#) || val.contains(r#""id":1.0"#));
    }

    #[test]
    fn get_key_exists_and_missing() {
        let b = setup();
        b.create_object_store("db", "s", None, false).unwrap();

        let key = IdbKey::Number(1.0);
        put(&b, "db", "s", Some(key.clone()), r#""x""#).unwrap();

        assert!(get_key(&b, "db", "s", &key).unwrap().is_some());
        assert!(get_key(&b, "db", "s", &IdbKey::Number(999.0))
            .unwrap()
            .is_none());
    }

    #[test]
    fn get_all_with_range_and_count() {
        let b = setup();
        b.create_object_store("db", "s", None, false).unwrap();

        for i in 1..=10 {
            #[allow(clippy::cast_precision_loss)]
            put(
                &b,
                "db",
                "s",
                Some(IdbKey::Number(f64::from(i))),
                &format!(r#""{i}""#),
            )
            .unwrap();
        }

        // All records
        let all = get_all(&b, "db", "s", None, None).unwrap();
        assert_eq!(all.len(), 10);

        // Range [3, 7]
        let range =
            IdbKeyRange::bound(IdbKey::Number(3.0), IdbKey::Number(7.0), false, false).unwrap();
        let subset = get_all(&b, "db", "s", Some(&range), None).unwrap();
        assert_eq!(subset.len(), 5);

        // With count limit
        let limited = get_all(&b, "db", "s", Some(&range), Some(2)).unwrap();
        assert_eq!(limited.len(), 2);
    }

    #[test]
    fn get_all_keys_ordered() {
        let b = setup();
        b.create_object_store("db", "s", None, false).unwrap();

        put(&b, "db", "s", Some(IdbKey::Number(3.0)), r#""c""#).unwrap();
        put(&b, "db", "s", Some(IdbKey::Number(1.0)), r#""a""#).unwrap();
        put(&b, "db", "s", Some(IdbKey::Number(2.0)), r#""b""#).unwrap();

        let keys = get_all_keys(&b, "db", "s", None, None).unwrap();
        assert_eq!(
            keys,
            vec![
                IdbKey::Number(1.0),
                IdbKey::Number(2.0),
                IdbKey::Number(3.0),
            ]
        );
    }

    #[test]
    fn delete_single_key() {
        let b = setup();
        b.create_object_store("db", "s", None, false).unwrap();

        let key = IdbKey::Number(1.0);
        put(&b, "db", "s", Some(key.clone()), r#""v""#).unwrap();
        assert!(get(&b, "db", "s", &key).unwrap().is_some());

        delete(&b, "db", "s", &DeleteTarget::Key(key.clone())).unwrap();
        assert!(get(&b, "db", "s", &key).unwrap().is_none());
    }

    #[test]
    fn delete_range() {
        let b = setup();
        b.create_object_store("db", "s", None, false).unwrap();

        for i in 1..=5 {
            #[allow(clippy::cast_precision_loss)]
            put(
                &b,
                "db",
                "s",
                Some(IdbKey::Number(f64::from(i))),
                &format!(r#""{i}""#),
            )
            .unwrap();
        }

        let range =
            IdbKeyRange::bound(IdbKey::Number(2.0), IdbKey::Number(4.0), false, false).unwrap();
        delete(&b, "db", "s", &DeleteTarget::Range(range)).unwrap();

        let remaining = get_all_keys(&b, "db", "s", None, None).unwrap();
        assert_eq!(remaining, vec![IdbKey::Number(1.0), IdbKey::Number(5.0)]);
    }

    #[test]
    fn clear_store() {
        let b = setup();
        b.create_object_store("db", "s", None, false).unwrap();

        put(&b, "db", "s", Some(IdbKey::Number(1.0)), r#""a""#).unwrap();
        put(&b, "db", "s", Some(IdbKey::Number(2.0)), r#""b""#).unwrap();

        clear(&b, "db", "s").unwrap();
        assert_eq!(count(&b, "db", "s", None).unwrap(), 0);
    }

    #[test]
    fn count_with_range() {
        let b = setup();
        b.create_object_store("db", "s", None, false).unwrap();

        for i in 1..=5 {
            #[allow(clippy::cast_precision_loss)]
            put(
                &b,
                "db",
                "s",
                Some(IdbKey::Number(f64::from(i))),
                &format!(r#""{i}""#),
            )
            .unwrap();
        }

        assert_eq!(count(&b, "db", "s", None).unwrap(), 5);

        let range = IdbKeyRange::lower_bound(IdbKey::Number(3.0), false);
        assert_eq!(count(&b, "db", "s", Some(&range)).unwrap(), 3);
    }

    #[test]
    fn extract_key_nested_path() {
        let key = extract_key_from_value(r#"{"user":{"id":7}}"#, "user.id");
        assert_eq!(key, Some(IdbKey::Number(7.0)));
    }

    #[test]
    fn extract_key_missing_path() {
        let key = extract_key_from_value(r#"{"name":"bob"}"#, "id");
        assert!(key.is_none());
    }
}
