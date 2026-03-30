//! Shared utilities for `IndexedDB` backend modules.

use crate::key::IdbKey;

/// Resolve a dot-separated JSON path to a nested value.
pub(crate) fn resolve_path<'a>(
    val: &'a serde_json::Value,
    path: &str,
) -> Option<&'a serde_json::Value> {
    let mut current = val;
    for segment in path.split('.') {
        current = current.get(segment)?;
    }
    Some(current)
}

/// Convert a `serde_json::Value` to an `IdbKey`.
///
/// Maximum nesting depth for JSON-to-key conversion.
const MAX_JSON_KEY_DEPTH: usize = 64;

/// Returns `None` for null, bool, object, or other non-key types.
pub(crate) fn json_to_idb_key(val: &serde_json::Value) -> Option<IdbKey> {
    json_to_idb_key_depth(val, 0)
}

fn json_to_idb_key_depth(val: &serde_json::Value, depth: usize) -> Option<IdbKey> {
    match val {
        serde_json::Value::Number(n) => {
            let v = n.as_f64()?;
            if !v.is_finite() {
                return None;
            }
            Some(IdbKey::Number(v))
        }
        serde_json::Value::String(s) => Some(IdbKey::String(s.clone())),
        serde_json::Value::Array(arr) if depth < MAX_JSON_KEY_DEPTH => {
            let keys: Option<Vec<IdbKey>> = arr
                .iter()
                .map(|v| json_to_idb_key_depth(v, depth + 1))
                .collect();
            Some(IdbKey::Array(keys?))
        }
        _ => None,
    }
}

/// Convert an `IdbKey` to a `serde_json::Value`.
pub(crate) fn idb_key_to_json(key: &IdbKey) -> serde_json::Value {
    match key {
        IdbKey::Number(v) | IdbKey::Date(v) => serde_json::Value::Number(
            serde_json::Number::from_f64(*v).unwrap_or_else(|| serde_json::Number::from(0)),
        ),
        IdbKey::String(s) => serde_json::Value::String(s.clone()),
        IdbKey::Array(items) => {
            serde_json::Value::Array(items.iter().map(idb_key_to_json).collect())
        }
    }
}

/// Encode a string as hex for use in `SQLite` table names.
///
/// Hex encoding is collision-free: distinct inputs always produce distinct outputs.
pub(crate) fn sanitize_sql_name(s: &str) -> String {
    // Short ASCII-only names use a fast path for readability
    if s.len() <= 32 && s.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return s.to_owned();
    }
    // Hex-encode to avoid collisions (e.g., "a-b" vs "a_b")
    s.bytes()
        .fold(String::with_capacity(s.len() * 2), |mut acc, b| {
            use std::fmt::Write;
            let _ = write!(acc, "{b:02x}");
            acc
        })
}

/// Build a data table name for an object store: `store_{db}_{name}`.
pub(crate) fn data_table_name(db_name: &str, store_name: &str) -> String {
    format!(
        "store_{}_{}",
        sanitize_sql_name(db_name),
        sanitize_sql_name(store_name)
    )
}

/// Build an index table name: `idx_{db}_{store}_{index}`.
pub(crate) fn index_table_name(db_name: &str, store_name: &str, index_name: &str) -> String {
    format!(
        "idx_{}_{}_{}",
        sanitize_sql_name(db_name),
        sanitize_sql_name(store_name),
        sanitize_sql_name(index_name)
    )
}
