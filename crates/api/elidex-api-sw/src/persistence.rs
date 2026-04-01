//! Service Worker registration persistence (WHATWG SW §3.1).
//!
//! "A user agent MUST persistently store service worker registrations."
//! Uses StorageBackend to persist to SQLite.

use elidex_storage_core::{
    Migration, SqliteBackend, SqliteConnection, StorageBackend, StorageConnection, StorageError,
};

use crate::registration::{SwRegistration, SwState, UpdateViaCache};
use crate::update;

/// Schema migrations for SW registration persistence.
const SW_MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    sql: "\
CREATE TABLE IF NOT EXISTS _sw_registrations (
    scope TEXT PRIMARY KEY,
    script_url TEXT NOT NULL,
    script_hash INTEGER,
    state TEXT NOT NULL DEFAULT 'parsed',
    update_via_cache TEXT NOT NULL DEFAULT 'imports'
);",
}];

/// Persistent storage for SW registrations.
pub struct SwPersistence {
    conn: SqliteConnection,
}

impl SwPersistence {
    /// Create from a SqliteConnection (typically from OriginStorageManager).
    pub fn new(conn: SqliteConnection) -> Result<Self, StorageError> {
        let backend = SqliteBackend::new();
        backend.migrate(&conn, SW_MIGRATIONS)?;
        Ok(Self { conn })
    }

    /// Create an in-memory instance (for testing).
    pub fn open_in_memory() -> Result<Self, StorageError> {
        let conn = SqliteConnection::open_in_memory()?;
        Self::new(conn)
    }

    /// Save a registration to persistent storage.
    pub fn save(&self, reg: &SwRegistration) -> Result<(), StorageError> {
        let hash = reg.script_hash.map(|h| h as i64);
        let state = state_to_str(reg.state);
        let cache = reg.update_via_cache.as_str();

        self.conn.raw_connection().execute(
            "INSERT OR REPLACE INTO _sw_registrations \
             (scope, script_url, script_hash, state, update_via_cache) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                reg.scope.as_str(),
                reg.script_url.as_str(),
                hash,
                state,
                cache
            ],
        )?;
        Ok(())
    }

    /// Load all persisted registrations.
    pub fn load_all(&self) -> Result<Vec<SwRegistration>, StorageError> {
        let mut stmt = self.conn.raw_connection().prepare(
            "SELECT scope, script_url, script_hash, state, update_via_cache \
             FROM _sw_registrations",
        )?;

        let rows = stmt
            .query_map([], |row| {
                let scope: String = row.get(0)?;
                let script_url: String = row.get(1)?;
                let hash: Option<i64> = row.get(2)?;
                let state: String = row.get(3)?;
                let cache: String = row.get(4)?;
                Ok((scope, script_url, hash, state, cache))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut registrations = Vec::with_capacity(rows.len());
        for (scope, script_url, hash, state, cache) in rows {
            let Some(scope_url) = url::Url::parse(&scope).ok() else {
                continue;
            };
            let Some(script_url) = url::Url::parse(&script_url).ok() else {
                continue;
            };
            registrations.push(SwRegistration {
                scope: scope_url,
                script_url,
                state: str_to_state(&state),
                #[allow(clippy::cast_sign_loss)]
                script_hash: hash.map(|h| h as u64),
                last_update_check: None, // Not persisted; reset on load
                update_via_cache: UpdateViaCache::parse(&cache).unwrap_or_default(),
            });
        }
        Ok(registrations)
    }

    /// Delete a registration by scope.
    pub fn delete(&self, scope: &url::Url) -> Result<bool, StorageError> {
        let count = self.conn.raw_connection().execute(
            "DELETE FROM _sw_registrations WHERE scope = ?1",
            [scope.as_str()],
        )?;
        Ok(count > 0)
    }

    /// Update script hash after an update check.
    pub fn update_hash(&self, scope: &url::Url, new_body: &[u8]) -> Result<(), StorageError> {
        let hash = update::hash_script(new_body) as i64;
        self.conn.raw_connection().execute(
            "UPDATE _sw_registrations SET script_hash = ?2 WHERE scope = ?1",
            rusqlite::params![scope.as_str(), hash],
        )?;
        Ok(())
    }
}

fn state_to_str(state: SwState) -> &'static str {
    match state {
        SwState::Parsed => "parsed",
        SwState::Installing => "installing",
        SwState::Installed => "installed",
        SwState::Activating => "activating",
        SwState::Activated => "activated",
        SwState::Redundant => "redundant",
    }
}

fn str_to_state(s: &str) -> SwState {
    match s {
        "installing" => SwState::Installing,
        "installed" => SwState::Installed,
        "activating" => SwState::Activating,
        "activated" => SwState::Activated,
        "redundant" => SwState::Redundant,
        _ => SwState::Parsed,
    }
}

impl std::fmt::Debug for SwPersistence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SwPersistence").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> url::Url {
        url::Url::parse(s).unwrap()
    }

    fn sample_reg(scope: &str) -> SwRegistration {
        SwRegistration {
            scope: url(scope),
            script_url: url(&format!("{scope}sw.js")),
            state: SwState::Activated,
            script_hash: Some(12345),
            last_update_check: None,
            update_via_cache: UpdateViaCache::Imports,
        }
    }

    #[test]
    fn save_and_load() {
        let p = SwPersistence::open_in_memory().unwrap();
        let reg = sample_reg("https://example.com/");
        p.save(&reg).unwrap();

        let loaded = p.load_all().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].scope.as_str(), "https://example.com/");
        assert_eq!(loaded[0].state, SwState::Activated);
        assert_eq!(loaded[0].script_hash, Some(12345));
        assert_eq!(loaded[0].update_via_cache, UpdateViaCache::Imports);
    }

    #[test]
    fn save_overwrites() {
        let p = SwPersistence::open_in_memory().unwrap();
        let mut reg = sample_reg("https://example.com/");
        p.save(&reg).unwrap();

        reg.state = SwState::Installed;
        reg.script_hash = Some(99999);
        p.save(&reg).unwrap();

        let loaded = p.load_all().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].state, SwState::Installed);
        assert_eq!(loaded[0].script_hash, Some(99999));
    }

    #[test]
    fn delete_registration() {
        let p = SwPersistence::open_in_memory().unwrap();
        p.save(&sample_reg("https://example.com/")).unwrap();

        assert!(p.delete(&url("https://example.com/")).unwrap());
        assert!(p.load_all().unwrap().is_empty());

        assert!(!p.delete(&url("https://example.com/")).unwrap()); // already gone
    }

    #[test]
    fn multiple_registrations() {
        let p = SwPersistence::open_in_memory().unwrap();
        p.save(&sample_reg("https://a.com/")).unwrap();
        p.save(&sample_reg("https://b.com/")).unwrap();
        p.save(&sample_reg("https://c.com/")).unwrap();

        let loaded = p.load_all().unwrap();
        assert_eq!(loaded.len(), 3);
    }

    #[test]
    fn update_hash() {
        let p = SwPersistence::open_in_memory().unwrap();
        p.save(&sample_reg("https://example.com/")).unwrap();

        let new_body = b"console.log('v2')";
        p.update_hash(&url("https://example.com/"), new_body)
            .unwrap();

        let loaded = p.load_all().unwrap();
        assert_eq!(loaded[0].script_hash, Some(update::hash_script(new_body)));
    }

    #[test]
    fn load_empty() {
        let p = SwPersistence::open_in_memory().unwrap();
        assert!(p.load_all().unwrap().is_empty());
    }
}
