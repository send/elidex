//! `HostBridge` `IndexedDB` accessors.

use boa_engine::JsObject;
use elidex_indexeddb::cursor::IdbCursorState;
use elidex_indexeddb::IdbBackend;

use super::HostBridge;

impl HostBridge {
    /// Register an open `IDBDatabase` JS object for versionchange event dispatch.
    pub fn register_idb_connection(&self, db_name: &str, db_obj: JsObject) {
        let mut inner = self.inner.borrow_mut();
        inner
            .idb_open_connections
            .entry(db_name.to_owned())
            .or_default()
            .push(db_obj);
    }

    /// Fire `versionchange` event on all open connections for a database.
    ///
    /// Returns the list of connections (caller can check if any remain open).
    pub fn fire_idb_versionchange(
        &self,
        db_name: &str,
        old_version: u64,
        new_version: Option<u64>,
        ctx: &mut boa_engine::Context,
    ) {
        let inner = self.inner.borrow();
        let Some(connections) = inner.idb_open_connections.get(db_name) else {
            return;
        };
        let connections: Vec<JsObject> = connections.clone();
        drop(inner); // release borrow before calling JS

        for conn in &connections {
            let handler = conn
                .get(boa_engine::js_string!("onversionchange"), ctx)
                .unwrap_or(boa_engine::JsValue::null());
            if let Some(func) = handler.as_callable() {
                let event = crate::globals::indexeddb::events::build_version_change_event(
                    "versionchange",
                    old_version,
                    new_version,
                    conn,
                    ctx,
                );
                let _ = func.call(
                    &boa_engine::JsValue::from(conn.clone()),
                    &[boa_engine::JsValue::from(event)],
                    ctx,
                );
            }
        }
    }

    /// Remove a database connection (called by `IDBDatabase.close()`).
    pub fn unregister_idb_connection(&self, db_name: &str, db_obj: &JsObject) {
        let mut inner = self.inner.borrow_mut();
        if let Some(conns) = inner.idb_open_connections.get_mut(db_name) {
            conns.retain(|c| !JsObject::equals(c, db_obj));
            if conns.is_empty() {
                inner.idb_open_connections.remove(db_name);
            }
        }
    }

    /// Check if a database is currently in upgrade mode.
    pub fn is_idb_upgrading(&self, db_name: &str) -> bool {
        self.inner.borrow().idb_upgrading_db.as_deref() == Some(db_name)
    }

    /// Set or clear the upgrading database name.
    pub fn set_idb_upgrading(&self, db_name: Option<&str>) {
        self.inner.borrow_mut().idb_upgrading_db = db_name.map(str::to_owned);
    }

    /// Get or lazily initialize the `IndexedDB` backend for this origin.
    ///
    /// Uses an in-memory `SQLite` database. Persistent file-backed storage
    /// will be integrated via `OriginIdbManager` when the shell provides a data directory.
    pub fn ensure_idb_backend(&self) -> Result<(), elidex_indexeddb::BackendError> {
        let mut inner = self.inner.borrow_mut();
        if inner.idb_backend.is_none() {
            inner.idb_backend = Some(IdbBackend::open_in_memory()?);
        }
        Ok(())
    }

    /// Access the `IndexedDB` backend through a closure.
    ///
    /// Returns `None` if the backend hasn't been initialized.
    pub fn with_idb<R>(&self, f: impl FnOnce(&IdbBackend) -> R) -> Option<R> {
        let inner = self.inner.borrow();
        inner.idb_backend.as_ref().map(f)
    }

    /// Store a cursor state and return its unique ID.
    pub fn store_idb_cursor(&self, cursor: IdbCursorState) -> u64 {
        let mut inner = self.inner.borrow_mut();
        let id = inner.idb_cursor_next_id;
        inner.idb_cursor_next_id += 1;
        inner.idb_cursors.insert(id, cursor);
        id
    }

    /// Access a cursor state by ID through a closure.
    ///
    /// Temporarily removes the cursor from the map to avoid holding a mutable
    /// borrow on `inner` while also accessing the backend immutably.
    pub fn with_idb_cursor<R>(
        &self,
        cursor_id: u64,
        f: impl FnOnce(&IdbBackend, &mut IdbCursorState) -> R,
    ) -> Option<R> {
        // Remove cursor temporarily to avoid holding mutable borrow
        let mut cursor_state = self.inner.borrow_mut().idb_cursors.remove(&cursor_id)?;
        let result = self.with_idb(|backend| f(backend, &mut cursor_state));
        // Always reinsert cursor even if backend is None to avoid state loss
        self.inner
            .borrow_mut()
            .idb_cursors
            .insert(cursor_id, cursor_state);
        result
    }

    /// Remove a cursor state.
    pub fn remove_idb_cursor(&self, cursor_id: u64) {
        self.inner.borrow_mut().idb_cursors.remove(&cursor_id);
    }
}
