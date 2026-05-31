//! IndexedDB JS-surface tests (W3C Indexed Database API 3.0, slot
//! `#11-indexed-db-vm` / D-20a).
//!
//! The async model is the focus: a request's `success` / `error` event fires
//! from a **database task** drained at the `drain_tasks` tail (§5.6 step 5.6),
//! *not* inline (the boa bridge fired inline = bug, not copied).  `Vm::eval`
//! drains tasks after the top-level script returns, so the pattern here is:
//! run a setup script that wires `onupgradeneeded` / `onsuccess` callbacks
//! writing into a persistent `globalThis.__log`, then read `__log` in a second
//! `eval` (by which point the post-eval drain has run every queued task).

#![cfg(feature = "engine")]

use elidex_ecs::{Attributes, EcsDom};
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;

fn build_min_fixture(dom: &mut EcsDom) -> elidex_ecs::Entity {
    let doc = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    let body = dom.create_element("body", Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, body));
    doc
}

struct UnbindOnDrop<'a>(&'a mut Vm);

impl Drop for UnbindOnDrop<'_> {
    fn drop(&mut self) {
        self.0.unbind();
    }
}

fn with_vm<F, R>(f: F) -> R
where
    F: FnOnce(&mut Vm) -> R,
{
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_min_fixture(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let guard = UnbindOnDrop(&mut vm);
    f(guard.0)
}

fn eval_bool(vm: &mut Vm, source: &str) -> bool {
    match vm.eval(source).unwrap() {
        JsValue::Boolean(b) => b,
        other => panic!("expected bool, got {other:?} for `{source}`"),
    }
}

fn eval_string(vm: &mut Vm, source: &str) -> String {
    match vm.eval(source).unwrap() {
        JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string, got {other:?} for `{source}`"),
    }
}

fn eval_number(vm: &mut Vm, source: &str) -> f64 {
    match vm.eval(source).unwrap() {
        JsValue::Number(n) => n,
        other => panic!("expected number, got {other:?} for `{source}`"),
    }
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

#[test]
fn globals_are_registered() {
    with_vm(|vm| {
        assert_eq!(eval_string(vm, "typeof indexedDB"), "object");
        assert!(eval_bool(vm, "typeof indexedDB.open === 'function'"));
        assert!(eval_bool(
            vm,
            "typeof indexedDB.deleteDatabase === 'function'"
        ));
        assert!(eval_bool(vm, "typeof indexedDB.cmp === 'function'"));
        assert!(eval_bool(vm, "typeof IDBKeyRange === 'function'"));
        assert!(eval_bool(vm, "typeof IDBKeyRange.bound === 'function'"));
        assert!(eval_bool(vm, "typeof IDBRequest === 'function'"));
        assert!(eval_bool(vm, "typeof IDBDatabase === 'function'"));
        assert!(eval_bool(vm, "typeof IDBObjectStore === 'function'"));
        assert!(eval_bool(vm, "typeof IDBTransaction === 'function'"));
    });
}

#[test]
fn interface_objects_are_illegal_constructors() {
    with_vm(|vm| {
        // WebIDL §3.7.1: no constructor operation → `new` throws.
        assert!(eval_bool(
            vm,
            "(() => { try { new IDBRequest(); return false; } \
             catch (e) { return e instanceof TypeError; } })()"
        ));
        assert!(eval_bool(
            vm,
            "(() => { try { new IDBKeyRange(); return false; } \
             catch (e) { return e instanceof TypeError; } })()"
        ));
    });
}

// ---------------------------------------------------------------------------
// Async correctness — the bug class the boa bridge had
// ---------------------------------------------------------------------------

#[test]
fn open_result_is_pending_synchronously_then_done_after_drain() {
    with_vm(|vm| {
        // Synchronously after `open()` the request is still pending — the
        // success event has NOT fired inline (§4.1 / §5.6).
        vm.eval(
            "globalThis.__o = indexedDB.open('db_sync', 1); \
             globalThis.__rs = globalThis.__o.readyState;",
        )
        .unwrap();
        assert_eq!(eval_string(vm, "globalThis.__rs"), "pending");
        // After the post-eval task drain, the open has resolved.
        assert_eq!(eval_string(vm, "globalThis.__o.readyState"), "done");
    });
}

#[test]
fn open_upgrade_add_get_roundtrip_and_autocommit() {
    with_vm(|vm| {
        // Full happy path: open → upgradeneeded creates a store → success
        // opens a readwrite txn → add + get → the value round-trips and the
        // transaction auto-commits (oncomplete fires without an explicit
        // `tx.commit()`).  Every callback runs from the task drain.
        vm.eval(
            "globalThis.__log = [];
             const open = indexedDB.open('db_round', 1);
             open.onupgradeneeded = (e) => {
                 const db = e.target.result;
                 db.createObjectStore('books', { keyPath: 'id' });
                 globalThis.__log.push('upgrade');
             };
             open.onsuccess = (e) => {
                 const db = e.target.result;
                 const tx = db.transaction(['books'], 'readwrite');
                 const store = tx.objectStore('books');
                 store.add({ id: 1, title: 'Dune' });
                 const g = store.get(1);
                 g.onsuccess = () => { globalThis.__log.push('got:' + g.result.title); };
                 tx.oncomplete = () => { globalThis.__log.push('complete'); };
             };",
        )
        .unwrap();
        // upgrade ran, the value round-tripped, and the txn auto-committed.
        assert_eq!(
            eval_string(vm, "globalThis.__log.join(',')"),
            "upgrade,got:Dune,complete"
        );
    });
}

#[test]
fn get_inside_onsuccess_reactivates_the_transaction() {
    with_vm(|vm| {
        // §5.9 step 6: a request issued from within a `success` handler
        // reactivates the (otherwise inactive) transaction, so a chained
        // read on the same txn succeeds rather than throwing
        // TransactionInactiveError.
        vm.eval(
            "globalThis.__log = [];
             const open = indexedDB.open('db_chain', 1);
             open.onupgradeneeded = (e) => {
                 e.target.result.createObjectStore('s');
             };
             open.onsuccess = (e) => {
                 const tx = e.target.result.transaction(['s'], 'readwrite');
                 const store = tx.objectStore('s');
                 store.put('a', 1);
                 const g1 = store.get(1);
                 g1.onsuccess = () => {
                     globalThis.__log.push('g1:' + g1.result);
                     // chained request from inside the success handler
                     const g2 = store.get(1);
                     g2.onsuccess = () => { globalThis.__log.push('g2:' + g2.result); };
                 };
             };",
        )
        .unwrap();
        assert_eq!(eval_string(vm, "globalThis.__log.join(',')"), "g1:a,g2:a");
    });
}

#[test]
fn add_in_readonly_transaction_throws_read_only_error() {
    with_vm(|vm| {
        vm.eval(
            "globalThis.__err = 'none';
             const open = indexedDB.open('db_ro', 1);
             open.onupgradeneeded = (e) => { e.target.result.createObjectStore('s'); };
             open.onsuccess = (e) => {
                 const tx = e.target.result.transaction(['s'], 'readonly');
                 try { tx.objectStore('s').add('v', 1); }
                 catch (err) { globalThis.__err = err.name; }
             };",
        )
        .unwrap();
        assert_eq!(eval_string(vm, "globalThis.__err"), "ReadOnlyError");
    });
}

#[test]
fn add_failure_fires_error_event_not_inline() {
    with_vm(|vm| {
        // A duplicate `add` produces a ConstraintError delivered via the
        // request's `error` event (from the task drain), not an inline throw.
        vm.eval(
            "globalThis.__log = [];
             const open = indexedDB.open('db_dup', 1);
             open.onupgradeneeded = (e) => { e.target.result.createObjectStore('s'); };
             open.onsuccess = (e) => {
                 const tx = e.target.result.transaction(['s'], 'readwrite');
                 const store = tx.objectStore('s');
                 store.add('first', 1);
                 const dup = store.add('second', 1);
                 dup.onerror = (ev) => {
                     ev.preventDefault();
                     globalThis.__log.push('err:' + dup.error.name);
                 };
             };",
        )
        .unwrap();
        assert_eq!(
            eval_string(vm, "globalThis.__log.join(',')"),
            "err:ConstraintError"
        );
    });
}

#[test]
fn aborted_transaction_records_its_error_and_fires_abort() {
    with_vm(|vm| {
        // An uncanceled error event aborts the transaction (§5.10 step 8.3);
        // the abort cause is exposed via `transaction.error` (§4.10) and the
        // `abort` event fires.
        vm.eval(
            "globalThis.__log = [];
             const open = indexedDB.open('db_abort', 1);
             open.onupgradeneeded = (e) => { e.target.result.createObjectStore('s'); };
             open.onsuccess = (e) => {
                 const tx = e.target.result.transaction(['s'], 'readwrite');
                 const store = tx.objectStore('s');
                 store.add('first', 1);
                 store.add('dup', 1); // ConstraintError, not prevented → abort
                 tx.onabort = () => {
                     globalThis.__log.push('abort:' + (tx.error && tx.error.name));
                 };
             };",
        )
        .unwrap();
        assert_eq!(
            eval_string(vm, "globalThis.__log.join(',')"),
            "abort:ConstraintError"
        );
    });
}

// ---------------------------------------------------------------------------
// IDBKeyRange + cmp (synchronous surface)
// ---------------------------------------------------------------------------

#[test]
fn key_range_constructors_and_accessors() {
    with_vm(|vm| {
        assert!(eval_bool(vm, "IDBKeyRange.lowerBound(5).lower === 5"));
        assert!(eval_bool(
            vm,
            "IDBKeyRange.lowerBound(5, true).lowerOpen === true"
        ));
        assert!(eval_bool(vm, "IDBKeyRange.upperBound(9).upper === 9"));
        assert!(eval_bool(
            vm,
            "IDBKeyRange.bound(1, 3).includes(2) === true"
        ));
        assert!(eval_bool(
            vm,
            "IDBKeyRange.bound(1, 3).includes(5) === false"
        ));
        assert!(eval_bool(vm, "IDBKeyRange.only('x').lower === 'x'"));
        // bound with lower > upper → DataError.
        assert!(eval_bool(
            vm,
            "(() => { try { IDBKeyRange.bound(3, 1); return false; } \
             catch (e) { return e.name === 'DataError'; } })()"
        ));
    });
}

#[test]
fn cmp_orders_keys() {
    with_vm(|vm| {
        assert_eq!(eval_number(vm, "indexedDB.cmp(1, 2)"), -1.0);
        assert_eq!(eval_number(vm, "indexedDB.cmp(2, 2)"), 0.0);
        assert_eq!(eval_number(vm, "indexedDB.cmp(3, 2)"), 1.0);
        assert_eq!(eval_number(vm, "indexedDB.cmp('b', 'a')"), 1.0);
        // Invalid key → DataError.
        assert!(eval_bool(
            vm,
            "(() => { try { indexedDB.cmp(undefined, 1); return false; } \
             catch (e) { return e.name === 'DataError'; } })()"
        ));
    });
}

#[test]
fn database_metadata_accessors() {
    with_vm(|vm| {
        vm.eval(
            "globalThis.__log = [];
             const open = indexedDB.open('db_meta', 3);
             open.onupgradeneeded = (e) => {
                 const db = e.target.result;
                 db.createObjectStore('alpha', { keyPath: 'k', autoIncrement: true });
                 db.createObjectStore('beta');
             };
             open.onsuccess = (e) => {
                 const db = e.target.result;
                 globalThis.__log.push('name:' + db.name);
                 globalThis.__log.push('version:' + db.version);
                 globalThis.__log.push('stores:' + db.objectStoreNames.join('|'));
                 const tx = db.transaction(['alpha'], 'readonly');
                 const store = tx.objectStore('alpha');
                 globalThis.__log.push('kp:' + store.keyPath);
                 globalThis.__log.push('ai:' + store.autoIncrement);
             };",
        )
        .unwrap();
        assert_eq!(
            eval_string(vm, "globalThis.__log.join(',')"),
            "name:db_meta,version:3,stores:alpha|beta,kp:k,ai:true"
        );
    });
}
