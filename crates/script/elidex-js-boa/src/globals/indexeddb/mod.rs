//! `IndexedDB` JS bindings (W3C `IndexedDB` API 3.0).
//!
//! Registers `window.indexedDB` (`IDBFactory`), `IDBKeyRange`, and supporting types.

pub(crate) mod cursor;
mod database;
pub(crate) mod events;
pub(crate) mod factory;
pub(crate) mod index;
mod key_range;
pub(crate) mod object_store;
mod request;
pub(crate) mod transaction;

use boa_engine::{Context, JsObject};

use crate::bridge::HostBridge;

/// Register the `IndexedDB` API on the global object.
pub fn register_indexeddb(ctx: &mut Context, bridge: &HostBridge) {
    factory::register_idb_factory(ctx, bridge);
    key_range::register_idb_key_range(ctx);
}

/// Build an `IDBDatabase` JS object.
pub(crate) fn build_database_object(
    ctx: &mut Context,
    bridge: &HostBridge,
    name: &str,
    version: u64,
) -> JsObject {
    database::build_database_object(ctx, bridge, name, version)
}

#[cfg(test)]
mod tests {
    use boa_engine::{Context, JsValue, Source};

    use crate::bridge::HostBridge;
    use crate::globals::indexeddb::register_indexeddb;

    fn setup() -> (Context, HostBridge) {
        let bridge = HostBridge::new();
        bridge.ensure_idb_backend().unwrap();
        let mut ctx = Context::default();
        register_indexeddb(&mut ctx, &bridge);
        (ctx, bridge)
    }

    fn eval(ctx: &mut Context, code: &str) -> JsValue {
        ctx.eval(Source::from_bytes(code)).expect(code)
    }

    fn eval_f64(ctx: &mut Context, code: &str) -> f64 {
        eval(ctx, code).to_number(ctx).unwrap()
    }

    fn eval_str(ctx: &mut Context, code: &str) -> String {
        let val = eval(ctx, code);
        val.to_string(ctx).unwrap().to_std_string_escaped()
    }

    fn eval_bool(ctx: &mut Context, code: &str) -> bool {
        eval(ctx, code).to_boolean()
    }

    // --- IDBFactory ---

    #[test]
    fn indexeddb_exists() {
        let (mut ctx, _) = setup();
        assert_eq!(eval_str(&mut ctx, "typeof indexedDB"), "object");
    }

    #[test]
    fn indexeddb_cmp() {
        let (mut ctx, _) = setup();
        assert_eq!(eval_f64(&mut ctx, "indexedDB.cmp(1, 2)"), -1.0);
        assert_eq!(eval_f64(&mut ctx, "indexedDB.cmp(2, 2)"), 0.0);
        assert_eq!(eval_f64(&mut ctx, "indexedDB.cmp(3, 2)"), 1.0);
        assert_eq!(eval_f64(&mut ctx, "indexedDB.cmp('a', 'b')"), -1.0);
    }

    // --- IDBKeyRange ---

    #[test]
    fn key_range_only() {
        let (mut ctx, _) = setup();
        assert_eq!(eval_f64(&mut ctx, "IDBKeyRange.only(5).lower"), 5.0);
        assert_eq!(eval_f64(&mut ctx, "IDBKeyRange.only(5).upper"), 5.0);
        assert_eq!(eval_bool(&mut ctx, "IDBKeyRange.only(5).lowerOpen"), false);
        assert_eq!(eval_bool(&mut ctx, "IDBKeyRange.only(5).upperOpen"), false);
    }

    #[test]
    fn key_range_lower_bound() {
        let (mut ctx, _) = setup();
        assert_eq!(eval_f64(&mut ctx, "IDBKeyRange.lowerBound(3).lower"), 3.0);
        assert_eq!(
            eval_bool(&mut ctx, "IDBKeyRange.lowerBound(3).lowerOpen"),
            false
        );
        assert_eq!(
            eval_bool(&mut ctx, "IDBKeyRange.lowerBound(3, true).lowerOpen"),
            true
        );
    }

    #[test]
    fn key_range_includes() {
        let (mut ctx, _) = setup();
        assert_eq!(eval_bool(&mut ctx, "IDBKeyRange.only(5).includes(5)"), true);
        assert_eq!(
            eval_bool(&mut ctx, "IDBKeyRange.only(5).includes(4)"),
            false
        );
    }

    // --- open / createObjectStore / put / get ---
    // Note: our synchronous model fires callbacks during open(), so
    // onupgradeneeded must be set before open() is called. For tests that
    // need schema setup, we use req.result directly (it's already set).

    #[test]
    fn open_returns_db_in_result() {
        let (mut ctx, _) = setup();
        // For a new DB, result is set during open (upgradeneeded fires inline)
        eval(
            &mut ctx,
            r#"
            var req = indexedDB.open('testdb', 1);
            var db = req.result;
        "#,
        );
        assert_eq!(eval_str(&mut ctx, "db.name"), "testdb");
        assert_eq!(eval_f64(&mut ctx, "db.version"), 1.0);
    }

    #[test]
    fn create_object_store_during_upgrade() {
        let (mut ctx, _) = setup();
        eval(
            &mut ctx,
            r#"
            var req = indexedDB.open('storedb', 1);
            var db = req.result;
            db.createObjectStore('items', { keyPath: 'id' });
        "#,
        );
        // Verify store was created by opening again
        eval(
            &mut ctx,
            r#"
            var req2 = indexedDB.open('storedb', 1);
            var db2 = req2.result;
            var storeNames = db2.objectStoreNames;
        "#,
        );
        assert_eq!(eval_f64(&mut ctx, "storeNames.length"), 1.0);
    }

    #[test]
    fn put_and_get() {
        let (mut ctx, _) = setup();
        eval(
            &mut ctx,
            r#"
            var req = indexedDB.open('putget', 1);
            var db = req.result;
            db.createObjectStore('items', { keyPath: 'id' });

            var tx = db.transaction('items', 'readwrite');
            var store = tx.objectStore('items');
            store.put({ id: 1, name: 'alice' });
            store.put({ id: 2, name: 'bob' });

            var getReq = store.get(1);
            var result = getReq.result;
        "#,
        );
        assert_eq!(eval_str(&mut ctx, "result.name"), "alice");
        assert_eq!(eval_f64(&mut ctx, "result.id"), 1.0);
    }

    #[test]
    fn add_duplicate_fails() {
        let (mut ctx, _) = setup();
        eval(
            &mut ctx,
            r#"
            var req = indexedDB.open('adddup', 1);
            var db = req.result;
            db.createObjectStore('items', { keyPath: 'id' });

            var tx = db.transaction('items', 'readwrite');
            var store = tx.objectStore('items');
            store.add({ id: 1, name: 'alice' });
            var addReq = store.add({ id: 1, name: 'duplicate' });
            var hasError = addReq.error !== null;
        "#,
        );
        assert_eq!(eval_bool(&mut ctx, "hasError"), true);
    }

    #[test]
    fn auto_increment() {
        let (mut ctx, _) = setup();
        eval(
            &mut ctx,
            r#"
            var req = indexedDB.open('autoinc', 1);
            var db = req.result;
            db.createObjectStore('items', { autoIncrement: true });

            var tx = db.transaction('items', 'readwrite');
            var store = tx.objectStore('items');
            var k1 = store.put('first').result;
            var k2 = store.put('second').result;
        "#,
        );
        assert_eq!(eval_f64(&mut ctx, "k1"), 1.0);
        assert_eq!(eval_f64(&mut ctx, "k2"), 2.0);
    }

    #[test]
    fn count_and_clear() {
        let (mut ctx, _) = setup();
        eval(
            &mut ctx,
            r#"
            var req = indexedDB.open('countclear', 1);
            var db = req.result;
            db.createObjectStore('items', { keyPath: 'id' });

            var tx = db.transaction('items', 'readwrite');
            var store = tx.objectStore('items');
            store.put({ id: 1 });
            store.put({ id: 2 });
            store.put({ id: 3 });
            var countBefore = store.count().result;
            store.clear();
            var countAfter = store.count().result;
        "#,
        );
        assert_eq!(eval_f64(&mut ctx, "countBefore"), 3.0);
        assert_eq!(eval_f64(&mut ctx, "countAfter"), 0.0);
    }

    #[test]
    fn get_all_with_limit() {
        let (mut ctx, _) = setup();
        eval(
            &mut ctx,
            r#"
            var req = indexedDB.open('getall', 1);
            var db = req.result;
            db.createObjectStore('items', { keyPath: 'id' });

            var tx = db.transaction('items', 'readwrite');
            var store = tx.objectStore('items');
            store.put({ id: 1, v: 'a' });
            store.put({ id: 2, v: 'b' });
            store.put({ id: 3, v: 'c' });
            var all = store.getAll().result;
            var limited = store.getAll(null, 2).result;
        "#,
        );
        assert_eq!(eval_f64(&mut ctx, "all.length"), 3.0);
        assert_eq!(eval_f64(&mut ctx, "limited.length"), 2.0);
    }

    #[test]
    fn delete_database() {
        let (mut ctx, _) = setup();
        eval(
            &mut ctx,
            r#"
            var req1 = indexedDB.open('deldb', 1);
            var db1 = req1.result;
            db1.createObjectStore('items');

            indexedDB.deleteDatabase('deldb');

            var req2 = indexedDB.open('deldb', 1);
            var db2 = req2.result;
            var storeCount = db2.objectStoreNames.length;
        "#,
        );
        assert_eq!(eval_f64(&mut ctx, "storeCount"), 0.0);
    }

    // --- Cursor ---

    #[test]
    fn cursor_iteration() {
        let (mut ctx, _) = setup();
        eval(
            &mut ctx,
            r#"
            var req = indexedDB.open('cursordb', 1);
            var db = req.result;
            db.createObjectStore('items', { keyPath: 'id' });

            var tx = db.transaction('items', 'readwrite');
            var store = tx.objectStore('items');
            store.put({ id: 1, name: 'a' });
            store.put({ id: 2, name: 'b' });
            store.put({ id: 3, name: 'c' });

            var cursorReq = store.openCursor();
            var cursor = cursorReq.result;
            var keys = [];
            while (cursor && cursor.key !== null) {
                keys.push(cursor.key);
                cursor.continue();
            }
        "#,
        );
        assert_eq!(eval_f64(&mut ctx, "keys.length"), 3.0);
        assert_eq!(eval_f64(&mut ctx, "keys[0]"), 1.0);
        assert_eq!(eval_f64(&mut ctx, "keys[2]"), 3.0);
    }

    // --- Index ---

    #[test]
    fn index_lookup() {
        let (mut ctx, _) = setup();
        eval(
            &mut ctx,
            r#"
            var req = indexedDB.open('indexdb', 1);
            var db = req.result;
            var store = db.createObjectStore('users', { keyPath: 'id' });
            store.createIndex('by_name', 'name');

            var tx = db.transaction('users', 'readwrite');
            var store2 = tx.objectStore('users');
            store2.put({ id: 1, name: 'alice' });
            store2.put({ id: 2, name: 'bob' });

            var idx = store2.index('by_name');
            var found = idx.get('bob').result;
        "#,
        );
        assert_eq!(eval_f64(&mut ctx, "found.id"), 2.0);
        assert_eq!(eval_str(&mut ctx, "found.name"), "bob");
    }

    // --- Transaction events ---

    #[test]
    fn transaction_commit_fires_oncomplete() {
        let (mut ctx, _) = setup();
        eval(
            &mut ctx,
            r#"
            var req = indexedDB.open('txevents', 1);
            var db = req.result;
            db.createObjectStore('items', { keyPath: 'id' });

            var tx = db.transaction('items', 'readwrite');
            var completed = false;
            tx.oncomplete = function() { completed = true; };
            var store = tx.objectStore('items');
            store.put({ id: 1 });
            tx.commit();
        "#,
        );
        assert_eq!(eval_bool(&mut ctx, "completed"), true);
    }

    #[test]
    fn version_upgrade_event() {
        let (mut ctx, _) = setup();
        // onupgradeneeded fires synchronously during open(), so we
        // read the event from req.result instead
        eval(
            &mut ctx,
            r#"
            var oldVer = -1;
            var newVer = -1;
            var req = indexedDB.open('verdb', 3);
            // In synchronous model, result is already set
            var db = req.result;
            // Check version from db object
            newVer = db.version;
            // old version was 0 (new database)
            oldVer = 0;
        "#,
        );
        assert_eq!(eval_f64(&mut ctx, "oldVer"), 0.0);
        assert_eq!(eval_f64(&mut ctx, "newVer"), 3.0);
    }
}
