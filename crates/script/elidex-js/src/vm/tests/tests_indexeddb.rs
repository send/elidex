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

#[test]
fn zero_request_transaction_auto_commits_complete_in_same_turn() {
    with_vm(|vm| {
        // A transaction with no requests issued must still auto-commit when
        // control returns to the event loop (§2.7.1) — and its `complete`
        // event must fire in THIS drain (the sweep feeds its deferred
        // IdbCommitDone back into the drain loop), not be stranded.
        vm.eval(
            "globalThis.__log = [];
             const open = indexedDB.open('db_zero', 1);
             open.onupgradeneeded = (e) => { e.target.result.createObjectStore('s'); };
             open.onsuccess = (e) => {
                 const tx = e.target.result.transaction(['s'], 'readwrite');
                 tx.objectStore('s'); // no add/get/put, no explicit commit
                 tx.oncomplete = () => { globalThis.__log.push('complete'); };
             };",
        )
        .unwrap();
        assert_eq!(eval_string(vm, "globalThis.__log.join(',')"), "complete");
    });
}

#[test]
fn aborted_upgrade_fires_error_at_open_request() {
    with_vm(|vm| {
        // An upgrade handler that throws aborts the version-change txn; the
        // open request must fire `error` (§5.1 step 10.8), not only the
        // transaction's `abort`.
        vm.eval(
            "globalThis.__log = [];
             const open = indexedDB.open('db_upfail', 1);
             open.onupgradeneeded = (e) => {
                 e.target.result.createObjectStore('s');
                 throw new Error('boom');
             };
             open.onerror = (e) => {
                 e.preventDefault();
                 globalThis.__log.push('err:' + (open.error && open.error.name));
             };",
        )
        .unwrap();
        assert_eq!(
            eval_string(vm, "globalThis.__log.join(',')"),
            "err:AbortError"
        );
    });
}

#[test]
fn get_all_count_zero_and_negative_return_all_records() {
    with_vm(|vm| {
        // §6.2 step 1: count 0 (or absent) means "all records"; a negative
        // count is ToUint32-wrapped (not silently 0 → empty).
        vm.eval(
            "globalThis.__log = [];
             const open = indexedDB.open('db_count', 1);
             open.onupgradeneeded = (e) => { e.target.result.createObjectStore('s'); };
             open.onsuccess = (e) => {
                 const tx = e.target.result.transaction(['s'], 'readwrite');
                 const store = tx.objectStore('s');
                 store.put('a', 1); store.put('b', 2); store.put('c', 3);
                 const all0 = store.getAll(undefined, 0);
                 all0.onsuccess = () => { globalThis.__log.push('zero:' + all0.result.length); };
                 const allNeg = store.getAll(undefined, -1);
                 allNeg.onsuccess = () => { globalThis.__log.push('neg:' + allNeg.result.length); };
                 const two = store.getAll(undefined, 2);
                 two.onsuccess = () => { globalThis.__log.push('two:' + two.result.length); };
             };",
        )
        .unwrap();
        assert_eq!(
            eval_string(vm, "globalThis.__log.join(',')"),
            "zero:3,neg:3,two:2"
        );
    });
}

#[test]
fn error_event_bubbles_to_transaction_and_preventdefault_cancels_abort() {
    with_vm(|vm| {
        // §5.10 + bubbling: a request error bubbles to the transaction; a
        // `tx.onerror` that calls preventDefault() cancels the auto-abort, so
        // the transaction commits (fires `complete`) instead of aborting.
        vm.eval(
            "globalThis.__log = [];
             const open = indexedDB.open('db_bubble', 1);
             open.onupgradeneeded = (e) => { e.target.result.createObjectStore('s'); };
             open.onsuccess = (e) => {
                 const tx = e.target.result.transaction(['s'], 'readwrite');
                 const store = tx.objectStore('s');
                 store.add('first', 1);
                 store.add('dup', 1); // ConstraintError, bubbles to tx
                 tx.onerror = (ev) => {
                     globalThis.__log.push('tx-onerror');
                     ev.preventDefault(); // cancel the auto-abort
                 };
                 tx.oncomplete = () => { globalThis.__log.push('complete'); };
                 tx.onabort = () => { globalThis.__log.push('abort'); };
             };",
        )
        .unwrap();
        // tx.onerror fired (bubbled) and preventDefault kept the txn alive →
        // complete, not abort.
        assert_eq!(
            eval_string(vm, "globalThis.__log.join(',')"),
            "tx-onerror,complete"
        );
    });
}

#[test]
fn explicit_commit_with_outstanding_request_error_rolls_back() {
    with_vm(|vm| {
        // §5.4 step 2.1: commit() must wait for outstanding request deliveries;
        // an uncanceled error among them still aborts (the durable write is
        // deferred until the list drains, so it can roll back). Here a put
        // succeeds and a dup add fails (uncanceled) in the same txn that calls
        // commit() synchronously — the txn must abort, not commit.
        vm.eval(
            "globalThis.__log = [];
             const open = indexedDB.open('db_commit_err', 1);
             open.onupgradeneeded = (e) => { e.target.result.createObjectStore('s'); };
             open.onsuccess = (e) => {
                 const db = e.target.result;
                 const tx = db.transaction(['s'], 'readwrite');
                 const store = tx.objectStore('s');
                 store.add('first', 1);
                 store.add('dup', 1); // ConstraintError, not prevented
                 tx.oncomplete = () => { globalThis.__log.push('complete'); };
                 tx.onabort = () => { globalThis.__log.push('abort'); };
                 tx.commit(); // requested while the two adds are still queued
                 globalThis.__db_commit_err = db;
             };",
        )
        .unwrap();
        // The uncanceled ConstraintError aborts despite the explicit commit().
        assert_eq!(eval_string(vm, "globalThis.__log.join(',')"), "abort");
        // And the write rolled back: store '1' must not exist.
        vm.eval(
            "globalThis.__log2 = [];
             const tx = globalThis.__db_commit_err.transaction(['s'], 'readonly');
             const g = tx.objectStore('s').get(1);
             g.onsuccess = () => { globalThis.__log2.push('v:' + g.result); };",
        )
        .unwrap();
        assert_eq!(
            eval_string(vm, "globalThis.__log2.join(',')"),
            "v:undefined"
        );
    });
}

#[test]
fn abort_after_commit_throws_invalid_state() {
    with_vm(|vm| {
        // §4.10: abort() once the txn is committing throws InvalidStateError
        // (prevents an impossible complete+abort sequence).
        vm.eval(
            "globalThis.__err = 'none';
             const open = indexedDB.open('db_commit_abort', 1);
             open.onupgradeneeded = (e) => { e.target.result.createObjectStore('s'); };
             open.onsuccess = (e) => {
                 const tx = e.target.result.transaction(['s'], 'readwrite');
                 tx.objectStore('s').put('v', 1);
                 tx.commit();
                 try { tx.abort(); }
                 catch (err) { globalThis.__err = err.name; }
             };",
        )
        .unwrap();
        assert_eq!(eval_string(vm, "globalThis.__err"), "InvalidStateError");
    });
}

#[test]
fn numeric_keypath_coerces_to_string() {
    with_vm(|vm| {
        // WebIDL DOMString coercion: { keyPath: 1 } is the in-line key path
        // "1", so add({1: ...}) extracts the key without an explicit key arg.
        vm.eval(
            "globalThis.__log = [];
             const open = indexedDB.open('db_kp', 1);
             open.onupgradeneeded = (e) => {
                 const store = e.target.result.createObjectStore('s', { keyPath: 1 });
                 globalThis.__log.push('kp:' + store.keyPath);
             };
             open.onsuccess = (e) => {
                 const tx = e.target.result.transaction(['s'], 'readwrite');
                 const store = tx.objectStore('s');
                 const add = store.add({ 1: 42, v: 'x' }); // in-line key 42
                 add.onsuccess = () => { globalThis.__log.push('key:' + add.result); };
             };",
        )
        .unwrap();
        assert_eq!(eval_string(vm, "globalThis.__log.join(',')"), "kp:1,key:42");
    });
}

#[test]
fn request_error_throws_invalid_state_while_pending() {
    with_vm(|vm| {
        // §4.1: the `error` getter throws InvalidStateError before the request
        // completes (symmetric with `result`), not `null`.  Captured
        // synchronously (before the post-eval drain resolves the request).
        vm.eval(
            "globalThis.__o = indexedDB.open('db_err_pending', 1);
             globalThis.__pend = (() => { try { globalThis.__o.error; return 'no-throw'; } \
                 catch (e) { return e.name; } })();",
        )
        .unwrap();
        assert_eq!(eval_string(vm, "globalThis.__pend"), "InvalidStateError");
        // After completion (drain) it reads as null (success, no error).
        assert!(eval_bool(vm, "globalThis.__o.error === null"));
    });
}

#[test]
fn dispatch_event_rejects_non_event_argument() {
    with_vm(|vm| {
        // WebIDL `Event event`: a non-Event argument throws TypeError, matching
        // the shared EventTarget.dispatchEvent — it must NOT run listeners.
        vm.eval("globalThis.__o = indexedDB.open('db_dispatch', 1);")
            .unwrap();
        assert!(eval_bool(
            vm,
            "(() => { try { globalThis.__o.dispatchEvent({ type: 'success' }); return false; } \
             catch (e) { return e instanceof TypeError; } })()"
        ));
    });
}

#[test]
fn aborted_upgrade_open_request_result_is_undefined() {
    with_vm(|vm| {
        // §4.1: a done-with-error request's `result` is undefined, not the
        // stale connection it briefly held before the upgrade aborted.
        vm.eval(
            "globalThis.__log = [];
             const open = indexedDB.open('db_up_result', 1);
             open.onupgradeneeded = (e) => {
                 e.target.result.createObjectStore('s');
                 throw new Error('boom'); // aborts the upgrade
             };
             open.onerror = (e) => {
                 e.preventDefault();
                 globalThis.__log.push('err:' + open.error.name);
                 globalThis.__log.push('result:' + String(open.result));
             };",
        )
        .unwrap();
        assert_eq!(
            eval_string(vm, "globalThis.__log.join(',')"),
            "err:AbortError,result:undefined"
        );
    });
}

#[test]
fn open_version_out_of_range_throws_type_error() {
    with_vm(|vm| {
        // WebIDL [EnforceRange] unsigned long long: out-of-range → TypeError
        // (not silent saturation); a fractional version truncates (no throw).
        assert!(eval_bool(
            vm,
            "(() => { try { indexedDB.open('db_oor', 1e30); return false; } \
             catch (e) { return e instanceof TypeError; } })()"
        ));
        assert!(eval_bool(
            vm,
            "(() => { try { indexedDB.open('db_oor2', -1); return false; } \
             catch (e) { return e instanceof TypeError; } })()"
        ));
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

// ---------------------------------------------------------------------------
// Lifecycle, GC, and remaining surface coverage
// ---------------------------------------------------------------------------

#[test]
fn gc_preserves_reachable_idb_objects() {
    with_vm(|vm| {
        // The IDB GC trace must root every ObjectId reachable from a live
        // wrapper (request result / db / store / txn state).  Open a db,
        // stash the connection in a global, force a collection, then use the
        // stashed db — if the trace missed the db / store state it would be
        // swept and the second turn would fault.
        vm.eval(
            "const o = indexedDB.open('db_gc', 1);
             o.onupgradeneeded = (e) => {
                 e.target.result.createObjectStore('s', { keyPath: 'id' });
             };
             o.onsuccess = (e) => {
                 globalThis.__db = e.target.result;
                 const tx = __db.transaction(['s'], 'readwrite');
                 tx.objectStore('s').add({ id: 1, v: 'kept' });
             };",
        )
        .unwrap();
        // `__db` is reachable only through the global; everything else
        // (its state, the backend) must survive collection.
        vm.inner.collect_garbage();
        vm.eval(
            "globalThis.__log = [];
             const tx = globalThis.__db.transaction(['s'], 'readonly');
             const g = tx.objectStore('s').get(1);
             g.onsuccess = () => { globalThis.__log.push(g.result.v); };",
        )
        .unwrap();
        assert_eq!(eval_string(vm, "globalThis.__log.join(',')"), "kept");
    });
}

#[test]
fn open_with_lower_version_fires_version_error() {
    with_vm(|vm| {
        // §5.1 step 7: opening at a version below the stored one delivers a
        // VersionError via the open request's error event.
        vm.eval(
            "globalThis.__log = [];
             const up = indexedDB.open('db_ver', 3);
             up.onupgradeneeded = (e) => { e.target.result.createObjectStore('s'); };
             up.onsuccess = (e) => {
                 e.target.result.close();
                 const down = indexedDB.open('db_ver', 1);
                 down.onerror = (ev) => {
                     ev.preventDefault();
                     globalThis.__log.push(down.error.name);
                 };
             };",
        )
        .unwrap();
        assert_eq!(
            eval_string(vm, "globalThis.__log.join(',')"),
            "VersionError"
        );
    });
}

#[test]
fn add_event_listener_delivers_success() {
    with_vm(|vm| {
        // The addEventListener delivery path (in-VM listener vec) is distinct
        // from the on* handler attribute — exercise it directly.
        vm.eval(
            "globalThis.__log = [];
             const open = indexedDB.open('db_ael', 1);
             open.onupgradeneeded = (e) => { e.target.result.createObjectStore('s'); };
             open.addEventListener('success', (e) => {
                 const tx = e.target.result.transaction(['s'], 'readwrite');
                 const g = tx.objectStore('s').put('v', 1);
                 g.addEventListener('success', () => { globalThis.__log.push('put-ok'); });
                 tx.addEventListener('complete', () => { globalThis.__log.push('done'); });
             });",
        )
        .unwrap();
        assert_eq!(eval_string(vm, "globalThis.__log.join(',')"), "put-ok,done");
    });
}

#[test]
fn explicit_commit_persists_and_abort_rolls_back() {
    with_vm(|vm| {
        // Explicit tx.commit() commits ('keep' persists); a separate later
        // tx.abort() rolls its write back ('drop' does not).  The second
        // transaction is opened in the first's `oncomplete` — the backend is
        // single-connection, so transactions are serialized (overlapping
        // connections are out of v1 scope, #11-idb-connection-queue).
        vm.eval(
            "globalThis.__log = [];
             const open = indexedDB.open('db_xc', 1);
             open.onupgradeneeded = (e) => { e.target.result.createObjectStore('s'); };
             open.onsuccess = (e) => {
                 const db = e.target.result;
                 globalThis.__xc_db = db;
                 const t1 = db.transaction(['s'], 'readwrite');
                 t1.objectStore('s').put('keep', 1);
                 t1.oncomplete = () => {
                     globalThis.__log.push('t1-complete');
                     const t2 = db.transaction(['s'], 'readwrite');
                     t2.objectStore('s').put('drop', 2);
                     t2.onabort = () => { globalThis.__log.push('t2-abort'); };
                     t2.abort();
                 };
                 t1.commit();
             };",
        )
        .unwrap();
        assert_eq!(
            eval_string(vm, "globalThis.__log.join(',')"),
            "t1-complete,t2-abort"
        );
        // 'keep' (committed) present, 'drop' (aborted) absent.
        vm.eval(
            "globalThis.__log2 = [];
             const tx = globalThis.__xc_db.transaction(['s'], 'readonly');
             const store = tx.objectStore('s');
             const a = store.get(1);
             a.onsuccess = () => { globalThis.__log2.push('k1:' + a.result); };
             const b = store.get(2);
             b.onsuccess = () => { globalThis.__log2.push('k2:' + b.result); };",
        )
        .unwrap();
        assert_eq!(
            eval_string(vm, "globalThis.__log2.join(',')"),
            "k1:keep,k2:undefined"
        );
    });
}

#[test]
fn transaction_objectstore_unknown_store_throws_not_found() {
    with_vm(|vm| {
        vm.eval(
            "globalThis.__err = 'none';
             const open = indexedDB.open('db_nf', 1);
             open.onupgradeneeded = (e) => { e.target.result.createObjectStore('s'); };
             open.onsuccess = (e) => {
                 const tx = e.target.result.transaction(['s'], 'readonly');
                 try { tx.objectStore('nope'); }
                 catch (err) { globalThis.__err = err.name; }
             };",
        )
        .unwrap();
        assert_eq!(eval_string(vm, "globalThis.__err"), "NotFoundError");
    });
}

#[test]
fn get_all_with_key_range() {
    with_vm(|vm| {
        // getAll over an explicit IDBKeyRange returns only the in-range
        // records, in key order.
        vm.eval(
            "globalThis.__log = [];
             const open = indexedDB.open('db_range', 1);
             open.onupgradeneeded = (e) => { e.target.result.createObjectStore('s'); };
             open.onsuccess = (e) => {
                 const tx = e.target.result.transaction(['s'], 'readwrite');
                 const store = tx.objectStore('s');
                 for (let i = 1; i <= 5; i++) store.put('v' + i, i);
                 const g = store.getAll(IDBKeyRange.bound(2, 4));
                 g.onsuccess = () => { globalThis.__log.push(g.result.join(',')); };
             };",
        )
        .unwrap();
        assert_eq!(eval_string(vm, "globalThis.__log.join(',')"), "v2,v3,v4");
    });
}

#[test]
fn databases_lists_and_delete_database_removes() {
    with_vm(|vm| {
        // databases() resolves with the open databases; deleteDatabase
        // removes one and fires success.
        vm.eval(
            "globalThis.__log = [];
             const open = indexedDB.open('db_del', 1);
             open.onupgradeneeded = (e) => { e.target.result.createObjectStore('s'); };
             open.onsuccess = (e) => {
                 e.target.result.close();
                 indexedDB.databases().then((list) => {
                     globalThis.__log.push('count:' + list.length);
                     globalThis.__log.push('has:' + list.some((d) => d.name === 'db_del'));
                     const del = indexedDB.deleteDatabase('db_del');
                     del.onsuccess = () => { globalThis.__log.push('deleted'); };
                 });
             };",
        )
        .unwrap();
        assert_eq!(
            eval_string(vm, "globalThis.__log.join(',')"),
            "count:1,has:true,deleted"
        );
    });
}
