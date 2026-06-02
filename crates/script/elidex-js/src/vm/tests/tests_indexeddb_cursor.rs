//! IndexedDB cursor + index JS-surface tests (W3C Indexed Database API 3.0,
//! slot `#11-indexed-db-vm` / D-20b).
//!
//! Same drain model as `tests_indexeddb` (a request's event fires from the
//! post-eval database-task drain, §5.6).  The focus here is the cursor
//! iteration state machine (the D-20b novelty, plan §3): the got-value flag +
//! per-iteration attribute snapshots committed at delivery, and the
//! `continue` / `advance` re-fire of a cursor's EXISTING request.

#![cfg(feature = "engine")]

use super::tests_indexeddb_common::{eval_bool, eval_string, with_vm};

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

#[test]
fn cursor_index_globals_are_registered() {
    with_vm(|vm| {
        assert!(eval_bool(vm, "typeof IDBIndex === 'function'"));
        assert!(eval_bool(vm, "typeof IDBCursor === 'function'"));
        assert!(eval_bool(vm, "typeof IDBCursorWithValue === 'function'"));
        // IDBCursorWithValue.prototype chains to IDBCursor.prototype.
        assert!(eval_bool(
            vm,
            "Object.getPrototypeOf(IDBCursorWithValue.prototype) === IDBCursor.prototype"
        ));
        // WebIDL §3.7.1: no constructor operation → `new` throws.
        assert!(eval_bool(
            vm,
            "(() => { try { new IDBCursor(); return false; } \
             catch (e) { return e instanceof TypeError; } })()"
        ));
        assert!(eval_bool(
            vm,
            "(() => { try { new IDBIndex(); return false; } \
             catch (e) { return e instanceof TypeError; } })()"
        ));
    });
}

// ---------------------------------------------------------------------------
// Cursor iteration state machine (plan §3 — the D-20b novelty)
// ---------------------------------------------------------------------------

/// A store seeded with `{id:1..3}` + the open boilerplate; `body` runs in the
/// `onsuccess` handler with `db` in scope and `__log` collecting results.
fn cursor_fixture(body: &str) -> String {
    format!(
        "globalThis.__log = [];
         const open = indexedDB.open('curdb_{tag}', 1);
         open.onupgradeneeded = (e) => {{
             const store = e.target.result.createObjectStore('items', {{ keyPath: 'id' }});
             store.add({{ id: 1, n: 'a' }});
             store.add({{ id: 2, n: 'b' }});
             store.add({{ id: 3, n: 'c' }});
         }};
         open.onsuccess = (e) => {{ const db = e.target.result; {body} }};",
        tag = body.len(),
        body = body
    )
}

#[test]
fn cursor_forward_iteration_collects_keys_in_order() {
    with_vm(|vm| {
        vm.eval(&cursor_fixture(
            "const tx = db.transaction(['items'], 'readonly');
             const req = tx.objectStore('items').openCursor();
             req.onsuccess = () => {
                 const c = req.result;
                 if (c) { __log.push(c.key); c.continue(); } else { __log.push('done'); }
             };",
        ))
        .unwrap();
        assert_eq!(eval_string(vm, "globalThis.__log.join(',')"), "1,2,3,done");
    });
}

#[test]
fn cursor_keeps_transaction_alive_until_exhausted() {
    with_vm(|vm| {
        // A pending continue() re-adds the request to the list, so auto-commit
        // (oncomplete) must wait until iteration finishes (plan §3 DR-1).
        vm.eval(&cursor_fixture(
            "const tx = db.transaction(['items'], 'readonly');
             const req = tx.objectStore('items').openCursor();
             req.onsuccess = () => {
                 const c = req.result;
                 if (c) { __log.push(c.key); c.continue(); } else { __log.push('done'); }
             };
             tx.oncomplete = () => __log.push('complete');",
        ))
        .unwrap();
        assert_eq!(
            eval_string(vm, "globalThis.__log.join(',')"),
            "1,2,3,done,complete"
        );
    });
}

#[test]
fn cursor_double_continue_in_one_handler_throws_invalid_state() {
    with_vm(|vm| {
        // The got-value flag flips false on the first continue() and only back
        // to true at the NEXT delivery, so the second continue() in the same
        // handler throws InvalidStateError (plan §3 DR-2 double-continue guard).
        vm.eval(&cursor_fixture(
            "const tx = db.transaction(['items'], 'readonly');
             const req = tx.objectStore('items').openCursor();
             let first = true;
             req.onsuccess = () => {
                 const c = req.result;
                 if (!c) return;
                 if (first) {
                     first = false;
                     c.continue();
                     try { c.continue(); __log.push('no-throw'); }
                     catch (e) { __log.push(e.name); }
                 }
             };",
        ))
        .unwrap();
        assert_eq!(
            eval_string(vm, "globalThis.__log.join(',')"),
            "InvalidStateError"
        );
    });
}

#[test]
fn cursor_advance_zero_is_type_error() {
    with_vm(|vm| {
        vm.eval(&cursor_fixture(
            "const tx = db.transaction(['items'], 'readonly');
             const req = tx.objectStore('items').openCursor();
             let done = false;
             req.onsuccess = () => {
                 const c = req.result;
                 if (c && !done) {
                     done = true;
                     try { c.advance(0); __log.push('no-throw'); }
                     catch (e) { __log.push(e.name); }
                 }
             };",
        ))
        .unwrap();
        assert_eq!(eval_string(vm, "globalThis.__log.join(',')"), "TypeError");
    });
}

#[test]
fn cursor_advance_skips_records() {
    with_vm(|vm| {
        vm.eval(&cursor_fixture(
            "const tx = db.transaction(['items'], 'readonly');
             const req = tx.objectStore('items').openCursor();
             let step = 0;
             req.onsuccess = () => {
                 const c = req.result;
                 if (!c) { __log.push('done'); return; }
                 __log.push(c.key);
                 if (step === 0) { step = 1; c.advance(2); } else { c.continue(); }
             };",
        ))
        .unwrap();
        // Start at 1, advance(2) → 3, then continue → exhausted.
        assert_eq!(eval_string(vm, "globalThis.__log.join(',')"), "1,3,done");
    });
}

#[test]
fn cursor_exhaustion_yields_null_result() {
    with_vm(|vm| {
        vm.eval(&cursor_fixture(
            "const tx = db.transaction(['items'], 'readonly');
             const req = tx.objectStore('items').openCursor();
             req.onsuccess = () => {
                 const c = req.result;
                 if (c) c.continue();
                 else __log.push(req.result === null ? 'null' : 'not-null');
             };",
        ))
        .unwrap();
        assert_eq!(eval_string(vm, "globalThis.__log.join(',')"), "null");
    });
}

#[test]
fn cursor_value_snapshot_has_stable_identity() {
    with_vm(|vm| {
        // Two reads of `cursor.value` within one iteration return the SAME
        // object (the committed snapshot), not a fresh backend re-read.
        vm.eval(&cursor_fixture(
            "const tx = db.transaction(['items'], 'readonly');
             const req = tx.objectStore('items').openCursor();
             let done = false;
             req.onsuccess = () => {
                 const c = req.result;
                 if (c && !done) { done = true; __log.push(c.value === c.value); }
             };",
        ))
        .unwrap();
        assert_eq!(eval_string(vm, "globalThis.__log.join(',')"), "true");
    });
}

#[test]
fn cursor_delete_keeps_key_readable_then_continues() {
    with_vm(|vm| {
        // After delete() the cursor's key/primaryKey snapshot is still readable
        // (held until the next iteration), and continue() proceeds to the next
        // record; the deleted record is gone.
        vm.eval(&cursor_fixture(
            "const tx = db.transaction(['items'], 'readwrite');
             const store = tx.objectStore('items');
             const req = store.openCursor();
             let deleted = false;
             req.onsuccess = () => {
                 const c = req.result;
                 if (!c) {
                     const chk = store.get(1);
                     chk.onsuccess = () => __log.push(chk.result === undefined ? 'gone' : 'present');
                     return;
                 }
                 __log.push(c.key);
                 if (!deleted) { deleted = true; c.delete(); __log.push('after-del:' + c.key); }
                 c.continue();
             };",
        ))
        .unwrap();
        assert_eq!(
            eval_string(vm, "globalThis.__log.join(',')"),
            "1,after-del:1,2,3,gone"
        );
    });
}

#[test]
fn cursor_update_round_trips() {
    with_vm(|vm| {
        vm.eval(&cursor_fixture(
            "const tx = db.transaction(['items'], 'readwrite');
             const store = tx.objectStore('items');
             const req = store.openCursor();
             let updated = false;
             req.onsuccess = () => {
                 const c = req.result;
                 if (!c) {
                     const chk = store.get(1);
                     chk.onsuccess = () => __log.push('n=' + chk.result.n);
                     return;
                 }
                 if (!updated) { updated = true; c.update({ id: 1, n: 'Z' }); }
                 c.continue();
             };",
        ))
        .unwrap();
        assert_eq!(eval_string(vm, "globalThis.__log.join(',')"), "n=Z");
    });
}

#[test]
fn open_key_cursor_is_key_only() {
    with_vm(|vm| {
        vm.eval(&cursor_fixture(
            "const tx = db.transaction(['items'], 'readonly');
             const req = tx.objectStore('items').openKeyCursor();
             let done = false;
             req.onsuccess = () => {
                 const c = req.result;
                 if (c && !done) {
                     done = true;
                     __log.push(c instanceof IDBCursor);
                     __log.push(c instanceof IDBCursorWithValue);
                     __log.push(c.value === undefined);
                 }
             };",
        ))
        .unwrap();
        assert_eq!(
            eval_string(vm, "globalThis.__log.join(',')"),
            "true,false,true"
        );
    });
}

#[test]
fn open_cursor_is_with_value() {
    with_vm(|vm| {
        vm.eval(&cursor_fixture(
            "const tx = db.transaction(['items'], 'readonly');
             const req = tx.objectStore('items').openCursor();
             let done = false;
             req.onsuccess = () => {
                 const c = req.result;
                 if (c && !done) {
                     done = true;
                     __log.push(c instanceof IDBCursorWithValue);
                     __log.push(c.request === req);
                     __log.push(c.value.n);
                 }
             };",
        ))
        .unwrap();
        assert_eq!(eval_string(vm, "globalThis.__log.join(',')"), "true,true,a");
    });
}

// ---------------------------------------------------------------------------
// Index (W3C IDB §4.6)
// ---------------------------------------------------------------------------

/// A `users` store keyed by `id` with a non-unique `by_age` index, seeded with
/// three rows; `body` runs in `onsuccess` with `db` + `__log` in scope.
fn index_fixture(body: &str) -> String {
    format!(
        "globalThis.__log = [];
         const open = indexedDB.open('idxdb_{tag}', 1);
         open.onupgradeneeded = (e) => {{
             const store = e.target.result.createObjectStore('users', {{ keyPath: 'id' }});
             store.createIndex('by_age', 'age');
             store.add({{ id: 1, name: 'alice', age: 30 }});
             store.add({{ id: 2, name: 'bob', age: 20 }});
             store.add({{ id: 3, name: 'carol', age: 25 }});
         }};
         open.onsuccess = (e) => {{ const db = e.target.result; {body} }};",
        tag = body.len(),
        body = body
    )
}

#[test]
fn index_handle_is_same_instance_per_store() {
    with_vm(|vm| {
        // §4.5 NOTE: store.index("x") === store.index("x").
        vm.eval(&index_fixture(
            "const tx = db.transaction(['users'], 'readonly');
             const store = tx.objectStore('users');
             __log.push(store.index('by_age') === store.index('by_age'));
             __log.push(store.index('by_age').name);
             __log.push(store.index('by_age').keyPath);",
        ))
        .unwrap();
        assert_eq!(
            eval_string(vm, "globalThis.__log.join(',')"),
            "true,by_age,age"
        );
    });
}

#[test]
fn index_get_and_count() {
    with_vm(|vm| {
        vm.eval(&index_fixture(
            "const tx = db.transaction(['users'], 'readonly');
             const idx = tx.objectStore('users').index('by_age');
             const g = idx.get(25);
             g.onsuccess = () => __log.push('name:' + g.result.name);
             const c = idx.count();
             c.onsuccess = () => __log.push('count:' + c.result);
             const k = idx.getKey(20);
             k.onsuccess = () => __log.push('pk:' + k.result);",
        ))
        .unwrap();
        assert_eq!(
            eval_string(vm, "globalThis.__log.join(',')"),
            "name:carol,count:3,pk:2"
        );
    });
}

#[test]
fn index_open_cursor_is_ordered_by_index_key() {
    with_vm(|vm| {
        // Iterating the by_age index yields ages ascending (20,25,30) with the
        // matching primary keys (2,3,1).
        vm.eval(&index_fixture(
            "const tx = db.transaction(['users'], 'readonly');
             const req = tx.objectStore('users').index('by_age').openCursor();
             req.onsuccess = () => {
                 const c = req.result;
                 if (c) { __log.push(c.key + ':' + c.primaryKey); c.continue(); }
                 else __log.push('done');
             };",
        ))
        .unwrap();
        assert_eq!(
            eval_string(vm, "globalThis.__log.join(',')"),
            "20:2,25:3,30:1,done"
        );
    });
}

#[test]
fn index_get_all_returns_all_values() {
    with_vm(|vm| {
        vm.eval(&index_fixture(
            "const tx = db.transaction(['users'], 'readonly');
             const req = tx.objectStore('users').index('by_age').getAll();
             req.onsuccess = () => __log.push(req.result.map(u => u.name).join('|'));",
        ))
        .unwrap();
        // Ordered by index key (age): bob(20), carol(25), alice(30).
        assert_eq!(
            eval_string(vm, "globalThis.__log.join(',')"),
            "bob|carol|alice"
        );
    });
}

#[test]
fn continue_primary_key_on_store_cursor_is_invalid_access() {
    with_vm(|vm| {
        vm.eval(&index_fixture(
            "const tx = db.transaction(['users'], 'readonly');
             const req = tx.objectStore('users').openCursor();
             let done = false;
             req.onsuccess = () => {
                 const c = req.result;
                 if (c && !done) {
                     done = true;
                     try { c.continuePrimaryKey(1, 1); __log.push('no-throw'); }
                     catch (e) { __log.push(e.name); }
                 }
             };",
        ))
        .unwrap();
        assert_eq!(
            eval_string(vm, "globalThis.__log.join(',')"),
            "InvalidAccessError"
        );
    });
}

// ---------------------------------------------------------------------------
// createIndex / deleteIndex (W3C IDB §4.5 / DR-4)
// ---------------------------------------------------------------------------

#[test]
fn create_index_outside_upgrade_is_invalid_state() {
    with_vm(|vm| {
        vm.eval(
            "globalThis.__log = [];
             const open = indexedDB.open('ci_nonupg', 1);
             open.onupgradeneeded = (e) => {
                 e.target.result.createObjectStore('s', { keyPath: 'id' });
             };
             open.onsuccess = (e) => {
                 const tx = e.target.result.transaction(['s'], 'readwrite');
                 const store = tx.objectStore('s');
                 try { store.createIndex('by_x', 'x'); __log.push('no-throw'); }
                 catch (err) { __log.push(err.name); }
             };",
        )
        .unwrap();
        assert_eq!(
            eval_string(vm, "globalThis.__log.join(',')"),
            "InvalidStateError"
        );
    });
}

#[test]
fn create_duplicate_index_is_constraint_error() {
    with_vm(|vm| {
        vm.eval(
            "globalThis.__log = [];
             const open = indexedDB.open('ci_dup', 1);
             open.onupgradeneeded = (e) => {
                 const store = e.target.result.createObjectStore('s', { keyPath: 'id' });
                 store.createIndex('by_x', 'x');
                 try { store.createIndex('by_x', 'x'); __log.push('no-throw'); }
                 catch (err) { __log.push(err.name); }
             };",
        )
        .unwrap();
        assert_eq!(
            eval_string(vm, "globalThis.__log.join(',')"),
            "ConstraintError"
        );
    });
}

#[test]
fn create_index_invalid_key_path_is_syntax_error() {
    with_vm(|vm| {
        vm.eval(
            "globalThis.__log = [];
             const open = indexedDB.open('ci_badkp', 1);
             open.onupgradeneeded = (e) => {
                 const store = e.target.result.createObjectStore('s', { keyPath: 'id' });
                 try { store.createIndex('bad', '1nope..x'); __log.push('no-throw'); }
                 catch (err) { __log.push(err.name); }
             };",
        )
        .unwrap();
        assert_eq!(eval_string(vm, "globalThis.__log.join(',')"), "SyntaxError");
    });
}

#[test]
fn create_index_compound_key_path_is_rejected() {
    with_vm(|vm| {
        // v1: compound (sequence) key paths unsupported — non-multiEntry rejects
        // with NotSupportedError (matching createObjectStore); multiEntry rejects
        // with the spec InvalidAccessError.  Tracked: #11-idb-compound-index-keypath.
        vm.eval(
            "globalThis.__log = [];
             const open = indexedDB.open('ci_compound', 1);
             open.onupgradeneeded = (e) => {
                 const store = e.target.result.createObjectStore('s', { keyPath: 'id' });
                 try { store.createIndex('c', ['a', 'b']); __log.push('no-throw'); }
                 catch (err) { __log.push(err.name); }
                 try { store.createIndex('m', ['a'], { multiEntry: true }); __log.push('no-throw'); }
                 catch (err) { __log.push(err.name); }
             };",
        )
        .unwrap();
        assert_eq!(
            eval_string(vm, "globalThis.__log.join(',')"),
            "NotSupportedError,InvalidAccessError"
        );
    });
}

#[test]
fn create_unique_index_over_duplicate_data_aborts_transaction() {
    with_vm(|vm| {
        // DR-4: createIndex over data violating the unique constraint does NOT
        // throw synchronously — it returns the handle, then aborts the upgrade
        // transaction (deferred `abort` event + version rollback).
        vm.eval(
            "globalThis.__log = [];
             const open = indexedDB.open('ci_unique', 1);
             open.onupgradeneeded = (e) => {
                 const store = e.target.result.createObjectStore('s', { keyPath: 'id' });
                 store.add({ id: 1, tag: 'x' });
                 store.add({ id: 2, tag: 'x' });
                 // Register onabort BEFORE createIndex: the async-abort clears
                 // the open request's `.transaction` link, so grab it first.
                 e.target.transaction.onabort = () => __log.push('abort');
                 const idx = store.createIndex('by_tag', 'tag', { unique: true });
                 __log.push('made:' + (idx instanceof IDBIndex));
             };
             open.onerror = () => __log.push('open-error');",
        )
        .unwrap();
        // createIndex returned a handle (no synchronous throw), then the txn
        // aborted and the open request failed.
        assert_eq!(
            eval_string(vm, "globalThis.__log.join(',')"),
            "made:true,abort,open-error"
        );
    });
}

#[test]
fn delete_index_removes_it() {
    with_vm(|vm| {
        vm.eval(
            "globalThis.__log = [];
             const open = indexedDB.open('di_db', 1);
             open.onupgradeneeded = (e) => {
                 const store = e.target.result.createObjectStore('s', { keyPath: 'id' });
                 store.createIndex('by_x', 'x');
                 __log.push(Array.from(store.indexNames).join('|'));
                 store.deleteIndex('by_x');
                 __log.push(Array.from(store.indexNames).join('|') || 'empty');
             };",
        )
        .unwrap();
        assert_eq!(eval_string(vm, "globalThis.__log.join(',')"), "by_x,empty");
    });
}

#[test]
fn operation_on_deleted_index_is_invalid_state() {
    with_vm(|vm| {
        // §4.6: an operation on an index handle whose index was deleted throws
        // InvalidStateError (not a raw backend error).
        vm.eval(
            "globalThis.__log = [];
             const open = indexedDB.open('idx_del', 1);
             open.onupgradeneeded = (e) => {
                 const store = e.target.result.createObjectStore('s', { keyPath: 'id' });
                 const idx = store.createIndex('by_x', 'x');
                 store.deleteIndex('by_x');
                 try { idx.get(1); __log.push('no-throw'); }
                 catch (err) { __log.push(err.name); }
             };",
        )
        .unwrap();
        assert_eq!(
            eval_string(vm, "globalThis.__log.join(',')"),
            "InvalidStateError"
        );
    });
}
