//! `IndexedDB` cursor implementation.
//!
//! Cursors iterate over object store or index records in key order.
//! State is tracked externally (in the JS bridge) via `IdbCursorState`.

use crate::backend::{BackendError, IdbBackend};
use crate::key::IdbKey;
use crate::key_range::IdbKeyRange;

/// Cursor iteration direction (W3C `IndexedDB` 3.0 §2.9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorDirection {
    /// Iterate in ascending key order.
    Next,
    /// Ascending, skip duplicate index keys.
    NextUnique,
    /// Iterate in descending key order.
    Prev,
    /// Descending, skip duplicate index keys.
    PrevUnique,
}

impl CursorDirection {
    /// Parse from JS string value.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "next" => Some(Self::Next),
            "nextunique" => Some(Self::NextUnique),
            "prev" => Some(Self::Prev),
            "prevunique" => Some(Self::PrevUnique),
            _ => None,
        }
    }

    fn is_forward(self) -> bool {
        matches!(self, Self::Next | Self::NextUnique)
    }

    fn is_unique(self) -> bool {
        matches!(self, Self::NextUnique | Self::PrevUnique)
    }

    fn order_clause(self) -> &'static str {
        if self.is_forward() {
            "ASC"
        } else {
            "DESC"
        }
    }
}

/// Source of a cursor — either an object store or an index.
#[derive(Debug, Clone)]
pub enum CursorSource {
    ObjectStore {
        db_name: String,
        store_name: String,
    },
    Index {
        db_name: String,
        store_name: String,
        index_name: String,
    },
}

impl CursorSource {
    fn db_and_store(&self) -> (&str, &str) {
        match self {
            Self::ObjectStore {
                db_name,
                store_name,
            }
            | Self::Index {
                db_name,
                store_name,
                ..
            } => (db_name.as_str(), store_name.as_str()),
        }
    }
}

/// A single cursor position entry.
#[derive(Debug, Clone)]
pub struct CursorEntry {
    /// The key at the current position (index key for index cursors).
    pub key: IdbKey,
    /// The primary key (same as `key` for object store cursors).
    pub primary_key: IdbKey,
    /// The record value (only for value cursors, `None` for key-only cursors).
    pub value: Option<String>,
}

/// Cursor state — tracks position, direction, and whether the cursor is exhausted.
pub struct IdbCursorState {
    source: CursorSource,
    direction: CursorDirection,
    range: Option<IdbKeyRange>,
    key_only: bool,
    current: Option<CursorEntry>,
    /// Set to `true` after `delete_current()` — value is gone until next `continue`.
    got_deleted: bool,
}

impl IdbCursorState {
    /// Returns the current entry, or `None` if exhausted or deleted.
    pub fn current(&self) -> Option<&CursorEntry> {
        if self.got_deleted {
            return None;
        }
        self.current.as_ref()
    }

    /// Returns the direction.
    pub fn direction(&self) -> CursorDirection {
        self.direction
    }

    /// Returns the source.
    pub fn source(&self) -> &CursorSource {
        &self.source
    }

    /// Returns whether this is a key-only cursor.
    pub fn is_key_only(&self) -> bool {
        self.key_only
    }
}

/// Open a cursor on an object store.
///
/// If `query` is an `IdbKey`, it is converted to `IdbKeyRange::only(key)`.
pub fn open_store_cursor(
    backend: &IdbBackend,
    db_name: &str,
    store_name: &str,
    range: Option<IdbKeyRange>,
    direction: CursorDirection,
    key_only: bool,
) -> Result<IdbCursorState, BackendError> {
    let source = CursorSource::ObjectStore {
        db_name: db_name.to_owned(),
        store_name: store_name.to_owned(),
    };

    let first = fetch_store_rows(
        backend,
        db_name,
        store_name,
        range.as_ref(),
        direction,
        1,
        None,
    )?;
    let current = first.into_iter().next();

    Ok(IdbCursorState {
        source,
        direction,
        range,
        key_only,
        current,
        got_deleted: false,
    })
}

/// Open a cursor on an index.
pub fn open_index_cursor(
    backend: &IdbBackend,
    db_name: &str,
    store_name: &str,
    index_name: &str,
    range: Option<IdbKeyRange>,
    direction: CursorDirection,
    key_only: bool,
) -> Result<IdbCursorState, BackendError> {
    let source = CursorSource::Index {
        db_name: db_name.to_owned(),
        store_name: store_name.to_owned(),
        index_name: index_name.to_owned(),
    };

    let first = fetch_index_rows(
        backend,
        db_name,
        store_name,
        index_name,
        range.as_ref(),
        direction,
        1,
        None,
    )?;
    let current = first.into_iter().next();

    Ok(IdbCursorState {
        source,
        direction,
        range,
        key_only,
        current,
        got_deleted: false,
    })
}

/// Advance the cursor by `count` positions.
///
/// `count` must be > 0, otherwise returns `TypeError`.
pub fn advance(
    backend: &IdbBackend,
    state: &mut IdbCursorState,
    count: u32,
) -> Result<(), BackendError> {
    if count == 0 {
        return Err(BackendError::DataError(
            "advance count must be positive".into(),
        ));
    }

    for _ in 0..count {
        advance_one(backend, state, None)?;
        if state.current.is_none() {
            break;
        }
    }
    Ok(())
}

/// Continue the cursor to the next position, optionally to a specific key.
///
/// If `target_key` is provided, validates direction consistency.
pub fn continue_cursor(
    backend: &IdbBackend,
    state: &mut IdbCursorState,
    target_key: Option<&IdbKey>,
) -> Result<(), BackendError> {
    // Validate target key direction
    if let (Some(target), Some(current)) = (target_key, &state.current) {
        let cmp = target.cmp(&current.key);
        if state.direction.is_forward() && cmp != std::cmp::Ordering::Greater {
            return Err(BackendError::DataError(
                "continue key must be greater than current key for forward cursor".into(),
            ));
        }
        if !state.direction.is_forward() && cmp != std::cmp::Ordering::Less {
            return Err(BackendError::DataError(
                "continue key must be less than current key for backward cursor".into(),
            ));
        }
    }

    advance_one(backend, state, target_key)?;
    Ok(())
}

/// Continue the cursor to a specific (index key, primary key) position.
///
/// Only valid for index cursors with `Next` or `Prev` direction (not `*Unique`).
/// W3C `IndexedDB` §4.9 `continuePrimaryKey(key, primaryKey)`.
pub fn continue_primary_key(
    backend: &IdbBackend,
    state: &mut IdbCursorState,
    key: &IdbKey,
    primary_key: &IdbKey,
) -> Result<(), BackendError> {
    // Must be an index cursor
    if matches!(state.source, CursorSource::ObjectStore { .. }) {
        return Err(BackendError::InvalidAccessError(
            "continuePrimaryKey only valid on index cursors".into(),
        ));
    }
    // Must not be *Unique direction
    if state.direction.is_unique() {
        return Err(BackendError::InvalidAccessError(
            "continuePrimaryKey not valid with nextunique/prevunique direction".into(),
        ));
    }
    // Key must be in the correct direction relative to current
    if let Some(current) = &state.current {
        let cmp = key.cmp(&current.key);
        if state.direction.is_forward() && cmp == std::cmp::Ordering::Less {
            return Err(BackendError::DataError(
                "key must be >= current key for forward cursor".into(),
            ));
        }
        if !state.direction.is_forward() && cmp == std::cmp::Ordering::Greater {
            return Err(BackendError::DataError(
                "key must be <= current key for backward cursor".into(),
            ));
        }
        // If same key, primary key must advance
        if cmp == std::cmp::Ordering::Equal {
            let pk_cmp = primary_key.cmp(&current.primary_key);
            if state.direction.is_forward() && pk_cmp != std::cmp::Ordering::Greater {
                return Err(BackendError::DataError(
                    "primaryKey must be > current primaryKey when key is equal".into(),
                ));
            }
            if !state.direction.is_forward() && pk_cmp != std::cmp::Ordering::Less {
                return Err(BackendError::DataError(
                    "primaryKey must be < current primaryKey when key is equal".into(),
                ));
            }
        }
    }

    // Advance to the target key, then filter by primary key
    // For simplicity, advance to key then scan for matching primary key
    advance_one(backend, state, Some(key))?;

    // Skip entries until we find one with the right primary key
    let forward = state.direction.is_forward();
    while let Some(entry) = &state.current {
        let dominated = if forward {
            entry.key > *key || (entry.key == *key && entry.primary_key >= *primary_key)
        } else {
            entry.key < *key || (entry.key == *key && entry.primary_key <= *primary_key)
        };
        if dominated {
            break;
        }
        advance_one(backend, state, None)?;
    }

    Ok(())
}

/// Update the value at the current cursor position.
///
/// Only valid for readwrite cursors on object stores (not key-only).
pub fn update_current(
    backend: &IdbBackend,
    state: &IdbCursorState,
    new_value: &str,
) -> Result<(), BackendError> {
    if state.got_deleted {
        return Err(BackendError::InvalidStateError(
            "cursor record has been deleted".into(),
        ));
    }
    let entry = state
        .current
        .as_ref()
        .ok_or_else(|| BackendError::InvalidStateError("cursor has no current value".into()))?;

    let (db_name, store_name) = state.source.db_and_store();

    // B15: If store has keyPath, validate the key in new_value matches the cursor key
    if let Ok((Some(kp), _)) = backend.get_store_meta(db_name, store_name) {
        if let Some(new_key) = crate::ops::extract_key_from_value(new_value, &kp) {
            if new_key != entry.primary_key {
                return Err(BackendError::DataError(
                    "key extracted from value does not match cursor key".into(),
                ));
            }
        }
    }

    let table = backend.data_table(db_name, store_name);
    let pk_bytes = entry.primary_key.serialize();
    backend.conn().execute(
        &format!("UPDATE [{table}] SET value = ?1 WHERE key_data = ?2"),
        rusqlite::params![new_value, pk_bytes],
    )?;

    // Update indexes
    crate::index::update_indexes_for_put(
        backend,
        db_name,
        store_name,
        &entry.primary_key,
        new_value,
    )?;

    Ok(())
}

/// Delete the record at the current cursor position.
pub fn delete_current(
    backend: &IdbBackend,
    state: &mut IdbCursorState,
) -> Result<(), BackendError> {
    if state.got_deleted {
        return Err(BackendError::InvalidStateError(
            "cursor record has already been deleted".into(),
        ));
    }
    let entry = state
        .current
        .as_ref()
        .ok_or_else(|| BackendError::InvalidStateError("cursor has no current value".into()))?;

    let (db_name, store_name) = state.source.db_and_store();

    let table = backend.data_table(db_name, store_name);
    let pk_bytes = entry.primary_key.serialize();
    backend.conn().execute(
        &format!("DELETE FROM [{table}] WHERE key_data = ?1"),
        rusqlite::params![pk_bytes],
    )?;

    crate::index::remove_indexes_for_delete(backend, db_name, store_name, &entry.primary_key)?;

    state.got_deleted = true;
    Ok(())
}

// -- Internal helpers --

fn advance_one(
    backend: &IdbBackend,
    state: &mut IdbCursorState,
    target_key: Option<&IdbKey>,
) -> Result<(), BackendError> {
    state.got_deleted = false;

    let after = if let Some(target) = target_key {
        // Jump to target — use a boundary just before target for forward, just after for reverse
        Some(target.clone())
    } else {
        state.current.as_ref().map(|e| e.key.clone())
    };

    let rows = match &state.source {
        CursorSource::ObjectStore {
            db_name,
            store_name,
        } => {
            if target_key.is_some() {
                // For targeted continue, we want >= target (forward) or <= target (reverse)
                fetch_store_rows_from(
                    backend,
                    db_name,
                    store_name,
                    state.range.as_ref(),
                    state.direction,
                    1,
                    after.as_ref(),
                    true,
                )?
            } else {
                fetch_store_rows(
                    backend,
                    db_name,
                    store_name,
                    state.range.as_ref(),
                    state.direction,
                    1,
                    after.as_ref(),
                )?
            }
        }
        CursorSource::Index {
            db_name,
            store_name,
            index_name,
        } => {
            if target_key.is_some() {
                fetch_index_rows_from(
                    backend,
                    db_name,
                    store_name,
                    index_name,
                    state.range.as_ref(),
                    state.direction,
                    1,
                    after.as_ref(),
                    true,
                )?
            } else {
                fetch_index_rows(
                    backend,
                    db_name,
                    store_name,
                    index_name,
                    state.range.as_ref(),
                    state.direction,
                    1,
                    after.as_ref(),
                )?
            }
        }
    };

    state.current = rows.into_iter().next();
    Ok(())
}

/// Fetch rows from an object store with optional "after" boundary (exclusive).
fn fetch_store_rows(
    backend: &IdbBackend,
    db_name: &str,
    store_name: &str,
    range: Option<&IdbKeyRange>,
    direction: CursorDirection,
    limit: u32,
    after: Option<&IdbKey>,
) -> Result<Vec<CursorEntry>, BackendError> {
    fetch_store_rows_from(
        backend, db_name, store_name, range, direction, limit, after, false,
    )
}

#[allow(clippy::too_many_arguments)]
fn fetch_store_rows_from(
    backend: &IdbBackend,
    db_name: &str,
    store_name: &str,
    range: Option<&IdbKeyRange>,
    direction: CursorDirection,
    limit: u32,
    after: Option<&IdbKey>,
    inclusive: bool,
) -> Result<Vec<CursorEntry>, BackendError> {
    let table = backend.data_table(db_name, store_name);
    let order = direction.order_clause();

    let mut conditions = Vec::new();
    let mut params: Vec<Vec<u8>> = Vec::new();

    // Range conditions
    if let Some(r) = range {
        let (clause, rp) = r.to_sql_clause("key_data");
        if clause != "1=1" {
            conditions.push(clause);
            params.extend(rp);
        }
    }

    // After-cursor condition
    if let Some(after_key) = after {
        let op = if inclusive {
            if direction.is_forward() {
                ">="
            } else {
                "<="
            }
        } else if direction.is_forward() {
            ">"
        } else {
            "<"
        };
        conditions.push(format!("key_data {op} ?"));
        params.push(after_key.serialize());
    }

    let where_clause = if conditions.is_empty() {
        "1=1".to_owned()
    } else {
        conditions.join(" AND ")
    };

    let sql = format!(
        "SELECT key_data, value FROM [{table}] WHERE {where_clause} ORDER BY key_data {order} LIMIT {limit}"
    );

    let mut stmt = backend.conn().prepare(&sql)?;
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params
        .iter()
        .map(|p| p as &dyn rusqlite::types::ToSql)
        .collect();

    let rows = stmt
        .query_map(param_refs.as_slice(), |row| {
            let kb: Vec<u8> = row.get(0)?;
            let val: String = row.get(1)?;
            Ok((kb, val))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut entries = Vec::new();
    for (kb, val) in rows {
        if let Some(key) = IdbKey::deserialize(&kb) {
            entries.push(CursorEntry {
                key: key.clone(),
                primary_key: key,
                value: Some(val),
            });
        }
    }
    Ok(entries)
}

/// Fetch rows from an index with optional "after" boundary (exclusive).
#[allow(clippy::too_many_arguments)]
fn fetch_index_rows(
    backend: &IdbBackend,
    db_name: &str,
    store_name: &str,
    index_name: &str,
    range: Option<&IdbKeyRange>,
    direction: CursorDirection,
    limit: u32,
    after: Option<&IdbKey>,
) -> Result<Vec<CursorEntry>, BackendError> {
    fetch_index_rows_from(
        backend, db_name, store_name, index_name, range, direction, limit, after, false,
    )
}

#[allow(clippy::too_many_arguments)]
fn fetch_index_rows_from(
    backend: &IdbBackend,
    db_name: &str,
    store_name: &str,
    index_name: &str,
    range: Option<&IdbKeyRange>,
    direction: CursorDirection,
    limit: u32,
    after: Option<&IdbKey>,
    inclusive: bool,
) -> Result<Vec<CursorEntry>, BackendError> {
    let idx_table = crate::index::index_table_name_for(db_name, store_name, index_name);
    let data_table = backend.data_table(db_name, store_name);
    let order = direction.order_clause();

    let mut conditions = Vec::new();
    let mut params: Vec<Vec<u8>> = Vec::new();

    if let Some(r) = range {
        let (clause, rp) = r.to_sql_clause("i.index_key");
        if clause != "1=1" {
            conditions.push(clause);
            params.extend(rp);
        }
    }

    if let Some(after_key) = after {
        let op = if inclusive {
            if direction.is_forward() {
                ">="
            } else {
                "<="
            }
        } else if direction.is_forward() {
            ">"
        } else {
            "<"
        };
        conditions.push(format!("i.index_key {op} ?"));
        params.push(after_key.serialize());
    }

    let where_clause = if conditions.is_empty() {
        "1=1".to_owned()
    } else {
        conditions.join(" AND ")
    };

    let distinct = if direction.is_unique() {
        "DISTINCT"
    } else {
        ""
    };

    let sql = format!(
        "SELECT {distinct} i.index_key, i.primary_key, d.value FROM [{idx_table}] i JOIN [{data_table}] d ON i.primary_key = d.key_data WHERE {where_clause} ORDER BY i.index_key {order} LIMIT {limit}"
    );

    let mut stmt = backend.conn().prepare(&sql)?;
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params
        .iter()
        .map(|p| p as &dyn rusqlite::types::ToSql)
        .collect();

    let rows = stmt
        .query_map(param_refs.as_slice(), |row| {
            let ik: Vec<u8> = row.get(0)?;
            let pk: Vec<u8> = row.get(1)?;
            let val: String = row.get(2)?;
            Ok((ik, pk, val))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut entries = Vec::new();
    for (ik, pk, val) in rows {
        if let (Some(index_key), Some(primary_key)) =
            (IdbKey::deserialize(&ik), IdbKey::deserialize(&pk))
        {
            entries.push(CursorEntry {
                key: index_key,
                primary_key,
                value: Some(val),
            });
        }
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index;
    use crate::ops;

    fn setup() -> IdbBackend {
        let b = IdbBackend::open_in_memory().unwrap();
        b.set_version("db", 1).unwrap();
        b.create_object_store("db", "items", None, false).unwrap();
        for i in 1..=5 {
            #[allow(clippy::cast_precision_loss)]
            ops::put(
                &b,
                "db",
                "items",
                Some(IdbKey::Number(i as f64)),
                &format!(r#"{{"val":{i}}}"#),
            )
            .unwrap();
        }
        b
    }

    #[test]
    fn cursor_forward_iteration() {
        let b = setup();
        let mut cur =
            open_store_cursor(&b, "db", "items", None, CursorDirection::Next, false).unwrap();

        let mut keys = Vec::new();
        while let Some(entry) = cur.current() {
            keys.push(entry.key.clone());
            continue_cursor(&b, &mut cur, None).unwrap();
        }
        assert_eq!(
            keys,
            vec![
                IdbKey::Number(1.0),
                IdbKey::Number(2.0),
                IdbKey::Number(3.0),
                IdbKey::Number(4.0),
                IdbKey::Number(5.0),
            ]
        );
    }

    #[test]
    fn cursor_reverse_iteration() {
        let b = setup();
        let mut cur =
            open_store_cursor(&b, "db", "items", None, CursorDirection::Prev, false).unwrap();

        let mut keys = Vec::new();
        while let Some(entry) = cur.current() {
            keys.push(entry.key.clone());
            continue_cursor(&b, &mut cur, None).unwrap();
        }
        assert_eq!(
            keys,
            vec![
                IdbKey::Number(5.0),
                IdbKey::Number(4.0),
                IdbKey::Number(3.0),
                IdbKey::Number(2.0),
                IdbKey::Number(1.0),
            ]
        );
    }

    #[test]
    fn cursor_with_range() {
        let b = setup();
        let range =
            IdbKeyRange::bound(IdbKey::Number(2.0), IdbKey::Number(4.0), false, false).unwrap();
        let mut cur =
            open_store_cursor(&b, "db", "items", Some(range), CursorDirection::Next, false)
                .unwrap();

        let mut keys = Vec::new();
        while let Some(entry) = cur.current() {
            keys.push(entry.key.clone());
            continue_cursor(&b, &mut cur, None).unwrap();
        }
        assert_eq!(
            keys,
            vec![
                IdbKey::Number(2.0),
                IdbKey::Number(3.0),
                IdbKey::Number(4.0),
            ]
        );
    }

    #[test]
    fn cursor_advance() {
        let b = setup();
        let mut cur =
            open_store_cursor(&b, "db", "items", None, CursorDirection::Next, false).unwrap();

        advance(&b, &mut cur, 3).unwrap();
        assert_eq!(cur.current().unwrap().key, IdbKey::Number(4.0));
    }

    #[test]
    fn advance_zero_is_error() {
        let b = setup();
        let mut cur =
            open_store_cursor(&b, "db", "items", None, CursorDirection::Next, false).unwrap();
        let err = advance(&b, &mut cur, 0);
        assert!(matches!(err, Err(BackendError::DataError(_))));
    }

    #[test]
    fn continue_with_target_key() {
        let b = setup();
        let mut cur =
            open_store_cursor(&b, "db", "items", None, CursorDirection::Next, false).unwrap();

        assert_eq!(cur.current().unwrap().key, IdbKey::Number(1.0));
        continue_cursor(&b, &mut cur, Some(&IdbKey::Number(4.0))).unwrap();
        assert_eq!(cur.current().unwrap().key, IdbKey::Number(4.0));
    }

    #[test]
    fn continue_wrong_direction_is_error() {
        let b = setup();
        let mut cur =
            open_store_cursor(&b, "db", "items", None, CursorDirection::Next, false).unwrap();

        // Current is 1.0, trying to go to 0.5 in forward direction → error
        let err = continue_cursor(&b, &mut cur, Some(&IdbKey::Number(0.5)));
        assert!(matches!(err, Err(BackendError::DataError(_))));
    }

    #[test]
    fn cursor_delete_current() {
        let b = setup();
        let mut cur =
            open_store_cursor(&b, "db", "items", None, CursorDirection::Next, false).unwrap();

        // Position at key 1.0
        assert_eq!(cur.current().unwrap().key, IdbKey::Number(1.0));

        delete_current(&b, &mut cur).unwrap();
        // After delete, current() returns None until continue
        assert!(cur.current().is_none());

        // Continue should go to 2.0
        continue_cursor(&b, &mut cur, None).unwrap();
        assert_eq!(cur.current().unwrap().key, IdbKey::Number(2.0));

        // Verify 1.0 is actually deleted
        assert!(ops::get(&b, "db", "items", &IdbKey::Number(1.0))
            .unwrap()
            .is_none());
    }

    #[test]
    fn cursor_double_delete_is_error() {
        let b = setup();
        let mut cur =
            open_store_cursor(&b, "db", "items", None, CursorDirection::Next, false).unwrap();
        delete_current(&b, &mut cur).unwrap();
        let err = delete_current(&b, &mut cur);
        assert!(matches!(err, Err(BackendError::InvalidStateError(_))));
    }

    #[test]
    fn cursor_update_current() {
        let b = setup();
        let mut cur =
            open_store_cursor(&b, "db", "items", None, CursorDirection::Next, false).unwrap();

        assert_eq!(cur.current().unwrap().key, IdbKey::Number(1.0));
        update_current(&b, &cur, r#"{"val":999}"#).unwrap();

        let val = ops::get(&b, "db", "items", &IdbKey::Number(1.0))
            .unwrap()
            .unwrap();
        assert_eq!(val, r#"{"val":999}"#);
    }

    #[test]
    fn cursor_update_after_delete_is_error() {
        let b = setup();
        let mut cur =
            open_store_cursor(&b, "db", "items", None, CursorDirection::Next, false).unwrap();
        delete_current(&b, &mut cur).unwrap();
        let err = update_current(&b, &cur, r#""new""#);
        assert!(matches!(err, Err(BackendError::InvalidStateError(_))));
    }

    #[test]
    fn cursor_exhaustion() {
        let b = setup();
        let mut cur =
            open_store_cursor(&b, "db", "items", None, CursorDirection::Next, false).unwrap();

        // Advance past all items
        advance(&b, &mut cur, 10).unwrap();
        assert!(cur.current().is_none());
    }

    #[test]
    fn index_cursor_forward() {
        let b = IdbBackend::open_in_memory().unwrap();
        b.set_version("db", 1).unwrap();
        b.create_object_store("db", "users", Some("id"), false)
            .unwrap();
        index::create_index(&b, "db", "users", "by_age", "age", false, false).unwrap();

        for (id, age) in [(1.0, 30.0), (2.0, 20.0), (3.0, 25.0)] {
            let val = format!(r#"{{"id":{id},"age":{age}}}"#);
            let key = ops::put(&b, "db", "users", None, &val).unwrap();
            index::update_indexes_for_put(&b, "db", "users", &key, &val).unwrap();
        }

        let mut cur = open_index_cursor(
            &b,
            "db",
            "users",
            "by_age",
            None,
            CursorDirection::Next,
            false,
        )
        .unwrap();

        let mut ages = Vec::new();
        while let Some(entry) = cur.current() {
            ages.push(entry.key.clone());
            continue_cursor(&b, &mut cur, None).unwrap();
        }
        // Should be ordered by age (index key)
        assert_eq!(
            ages,
            vec![
                IdbKey::Number(20.0),
                IdbKey::Number(25.0),
                IdbKey::Number(30.0),
            ]
        );
    }

    #[test]
    fn empty_store_cursor() {
        let b = IdbBackend::open_in_memory().unwrap();
        b.set_version("db", 1).unwrap();
        b.create_object_store("db", "empty", None, false).unwrap();

        let cur = open_store_cursor(&b, "db", "empty", None, CursorDirection::Next, false).unwrap();
        assert!(cur.current().is_none());
    }
}
