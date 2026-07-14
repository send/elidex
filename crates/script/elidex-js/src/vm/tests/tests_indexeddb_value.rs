//! IndexedDB **value / key marshalling** constraints — the `host/indexeddb/value.rs`
//! surface (W3C IDB §5.11 clone, §7.3 "convert a key to a value", §7.4 "convert a
//! value to a key").
//!
//! Split out of `tests_indexeddb.rs` at touch time (CLAUDE.md's 1000-line rule;
//! Codex R8), joining the existing `_common` / `_cursor` / `_events` scenario
//! split. Every case here pins the same thing: which JS values may cross into the
//! backend — as a **value** (structured-cloneable *and* faithfully JSON-storable)
//! or as a **key** (§7.4) — and which are **rejected** rather than silently
//! corrupted by the interim JSON storage.

#![cfg(feature = "engine")]

use super::tests_indexeddb_common::{eval_bool, eval_string, with_vm};

#[test]
fn add_cyclic_value_throws_data_clone_error() {
    with_vm(|vm| {
        // R8 / §5.11: a value that cannot be serialized for storage (a cyclic
        // object — JSON.stringify rejects it with TypeError) surfaces as
        // DataCloneError, not the raw `JSON.stringify` TypeError.
        vm.eval(
            "globalThis.__err = 'none';
             const open = indexedDB.open('db_clone', 1);
             open.onupgradeneeded = (e) => { e.target.result.createObjectStore('s'); };
             open.onsuccess = (e) => {
                 const tx = e.target.result.transaction(['s'], 'readwrite');
                 const cyclic = {}; cyclic.self = cyclic;
                 try { tx.objectStore('s').add(cyclic, 1); }
                 catch (err) { globalThis.__err = err.name; }
             };",
        )
        .unwrap();
        assert_eq!(eval_string(vm, "globalThis.__err"), "DataCloneError");
    });
}

#[test]
fn date_is_not_a_valid_key() {
    with_vm(|vm| {
        // §7.4 admits Date keys, but the backend's key routes all pass keys
        // through `json_to_idb_key` / `idb_key_to_json`, which cannot carry an
        // `IdbKey::Date` (see `host/indexeddb/value.rs`) — deferred to
        // `#11-idb-binary-key`.  Until the backend lands, the VM now has a
        // `Date` builtin, so a Date key is reachable and must be a DataError
        // rather than an explicit-key-only half-support.  Exercised
        // synchronously via `indexedDB.cmp` → `js_to_idb_key`.
        assert!(eval_bool(
            vm,
            "(() => { try { indexedDB.cmp(new Date(1000), new Date(2000)); return false; } \
             catch (e) { return e.name === 'DataError'; } })()"
        ));
    });
}

#[test]
fn add_date_value_throws_data_clone_error() {
    with_vm(|vm| {
        // A Date is structured-cloneable but not JSON-representable (`toJSON` →
        // ISO string), so the JSON backend would silently persist a String.
        // Reject with DataCloneError — same stance as Error / RegExp / Blob
        // values — until `#11-idb-structured-clone-storage` lands.
        vm.eval(
            "globalThis.__err = 'none';
             const open = indexedDB.open('db_date_clone', 1);
             open.onupgradeneeded = (e) => { e.target.result.createObjectStore('s'); };
             open.onsuccess = (e) => {
                 const tx = e.target.result.transaction(['s'], 'readwrite');
                 try { tx.objectStore('s').add({ d: new Date(0) }, 1); }
                 catch (err) { globalThis.__err = err.name; }
             };",
        )
        .unwrap();
        assert_eq!(eval_string(vm, "globalThis.__err"), "DataCloneError");
    });
}

#[test]
fn add_with_explicit_key_on_inline_store_throws_sync_data_error() {
    with_vm(|vm| {
        // R9 #1 / §10.2.4: providing an explicit key to an inline-key store is
        // a deterministic DataError thrown SYNCHRONOUSLY from add()/put(), not
        // delivered as an async request error (only ConstraintError is async).
        vm.eval(
            "globalThis.__err = 'none';
             const open = indexedDB.open('db_sync_de', 1);
             open.onupgradeneeded = (e) => {
                 e.target.result.createObjectStore('s', { keyPath: 'id' });
             };
             open.onsuccess = (e) => {
                 const tx = e.target.result.transaction(['s'], 'readwrite');
                 const store = tx.objectStore('s');
                 try { store.add({ id: 1 }, 1); } // explicit key on inline store
                 catch (err) { globalThis.__err = err.name; }
             };",
        )
        .unwrap();
        assert_eq!(eval_string(vm, "globalThis.__err"), "DataError");
    });
}

#[test]
fn lone_surrogate_string_key_is_rejected() {
    with_vm(|vm| {
        // R9 #4: an unpaired-surrogate string key has no UTF-8 representation
        // and would alias under the backend's UTF-8 key storage — reject with
        // DataError rather than lossy-convert.
        assert!(eval_bool(
            vm,
            "(() => { try { indexedDB.cmp('\\uD800', 'a'); return false; } \
             catch (e) { return e.name === 'DataError'; } })()"
        ));
        // A well-formed surrogate pair (😀) is a valid key — no false reject.
        assert!(eval_bool(
            vm,
            "(() => { try { indexedDB.cmp('\\uD83D\\uDE00', 'a'); return true; } \
             catch (e) { return false; } })()"
        ));
    });
}
