//! IndexedDB event-dispatch tests (slot `#11-indexed-db-vm` / D-20a) — the
//! in-VM `EventTarget` model shared by `IDBRequest` / `IDBTransaction` /
//! `IDBDatabase`: bubbling along the IDB ancestor chain, `on*` handler attrs
//! vs `addEventListener`, `dispatchEvent`, and the WHATWG DOM §2.9
//! bookkeeping (dispatch flag, `currentTarget` / propagation-flag finalize,
//! `once`-prune-at-invocation).  Split out of `tests_indexeddb.rs` to keep
//! each IDB test module under the repo's ~1000-line convention.

#![cfg(feature = "engine")]

use super::tests_indexeddb_common::{eval_bool, eval_string, with_vm};

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
fn dispatch_event_typed_like_a_handler_attr_does_not_invoke_handler() {
    with_vm(|vm| {
        // The no-handler sentinel must not collide with handler-attr names:
        // dispatching an Event whose type is literally "onsuccess" must NOT
        // invoke the onsuccess handler (which is for "success" events).
        vm.eval(
            "globalThis.__log = [];
             const o = indexedDB.open('db_sentinel', 1);
             o.onsuccess = () => { globalThis.__log.push('onsuccess-ran'); };
             o.dispatchEvent(new Event('onsuccess'));",
        )
        .unwrap();
        // Only the real success delivery (post-drain) ran onsuccess; the
        // synthetic 'onsuccess'-typed dispatch did not.
        assert_eq!(
            eval_string(vm, "globalThis.__log.join(',')"),
            "onsuccess-ran"
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
fn commit_in_last_request_success_fires_complete_exactly_once() {
    with_vm(|vm| {
        // R6.1: calling `tx.commit()` from the last request's `success` handler
        // drives `finalize_commit` from BOTH `commit_transaction` (the explicit
        // commit — the request was already removed from the list before its
        // event fired, §5.6) AND `run_post_dispatch` (the committing-branch
        // finalize once the now-empty list is observed), queuing two
        // `IdbCommitDone` tasks.  `dispatch_commit_done` must be idempotent, so
        // `complete` fires exactly once.
        vm.eval(
            "globalThis.__log = [];
             const open = indexedDB.open('db_dblcommit', 1);
             open.onupgradeneeded = (e) => { e.target.result.createObjectStore('s'); };
             open.onsuccess = (e) => {
                 const tx = e.target.result.transaction(['s'], 'readwrite');
                 const req = tx.objectStore('s').add('v', 1);
                 req.onsuccess = () => { tx.commit(); };
                 tx.oncomplete = () => { globalThis.__log.push('complete'); };
             };",
        )
        .unwrap();
        // Exactly one `complete`, not "complete,complete".
        assert_eq!(eval_string(vm, "globalThis.__log.join(',')"), "complete");
    });
}

#[test]
fn dispatch_event_honors_stop_immediate_propagation() {
    with_vm(|vm| {
        // R6: `dispatchEvent` routes through the one shared dispatcher, so a
        // listener calling `stopImmediatePropagation()` suppresses the remaining
        // listeners on the same node (previously the dispatchEvent loop ignored
        // the propagation-stop flags entirely).
        vm.eval(
            "globalThis.__log = [];
             const o = indexedDB.open('db_sip', 1);
             o.addEventListener('foo', (e) => { globalThis.__log.push('a'); e.stopImmediatePropagation(); });
             o.addEventListener('foo', () => { globalThis.__log.push('b'); });
             o.dispatchEvent(new Event('foo'));",
        )
        .unwrap();
        // Second listener suppressed.
        assert_eq!(eval_string(vm, "globalThis.__log.join(',')"), "a");
    });
}

#[test]
fn stop_propagation_on_request_error_halts_bubbling_to_transaction() {
    with_vm(|vm| {
        // R6: a request `error` that calls `stopPropagation()` no longer reaches
        // the transaction's `onerror`, but — not being `preventDefault()`'d —
        // the uncanceled error still aborts the transaction (§5.10 step 8.3).
        vm.eval(
            "globalThis.__log = [];
             const open = indexedDB.open('db_stopprop', 1);
             open.onupgradeneeded = (e) => { e.target.result.createObjectStore('s'); };
             open.onsuccess = (e) => {
                 const tx = e.target.result.transaction(['s'], 'readwrite');
                 const store = tx.objectStore('s');
                 store.add('first', 1);
                 const dup = store.add('dup', 1); // ConstraintError
                 dup.onerror = (ev) => {
                     globalThis.__log.push('req-error');
                     ev.stopPropagation(); // halt bubbling, but do NOT preventDefault
                 };
                 tx.onerror = () => { globalThis.__log.push('tx-onerror'); };
                 tx.onabort = () => { globalThis.__log.push('abort'); };
             };",
        )
        .unwrap();
        // tx.onerror suppressed by stopPropagation; uncanceled error still aborts.
        assert_eq!(
            eval_string(vm, "globalThis.__log.join(',')"),
            "req-error,abort"
        );
    });
}

#[test]
fn dispatch_event_sets_target_and_brackets_dispatch_state() {
    with_vm(|vm| {
        // R7 / WHATWG DOM §2.9 bookkeeping on the IDB dispatchEvent path.
        vm.eval("globalThis.__o = indexedDB.open('db_disp_bk', 1);")
            .unwrap();
        // §2.9: `target` is set even with zero listeners; `currentTarget` is
        // cleared to null after dispatch.
        assert!(eval_bool(
            vm,
            "(() => { const e = new Event('foo'); globalThis.__o.dispatchEvent(e); \
             return e.target === globalThis.__o && e.currentTarget === null; })()"
        ));
        // §2.9 step 1: a re-entrant re-dispatch of an in-flight event throws
        // InvalidStateError (caught inside the listener here).
        assert!(eval_bool(
            vm,
            "(() => { const e = new Event('bar'); let caught = null; \
             globalThis.__o.addEventListener('bar', () => { \
               try { globalThis.__o.dispatchEvent(e); } catch (err) { caught = err; } }); \
             globalThis.__o.dispatchEvent(e); \
             return caught instanceof DOMException && caught.name === 'InvalidStateError'; })()"
        ));
        // Sequential dispatch of the same event object succeeds twice (the
        // dispatch flag is bracketed, not left set).
        assert!(eval_bool(
            vm,
            "(() => { const e = new Event('baz'); \
             return globalThis.__o.dispatchEvent(e) && globalThis.__o.dispatchEvent(e); })()"
        ));
    });
}

#[test]
fn internal_fire_clears_current_target_after_dispatch() {
    with_vm(|vm| {
        // R8 (mod.rs:661): an internal fire (here a request `success`) must
        // clear `currentTarget` after the walk — a handler that captures the
        // event observes the §2.9 "no longer dispatching" state (currentTarget
        // null), not the last node it visited.  `target` stays set.
        vm.eval(
            "globalThis.__cap = null;
             const open = indexedDB.open('db_internal_ct', 1);
             open.onsuccess = (e) => { globalThis.__cap = e; };",
        )
        .unwrap();
        assert!(eval_bool(
            vm,
            "globalThis.__cap !== null \
             && globalThis.__cap.currentTarget === null \
             && globalThis.__cap.target !== null"
        ));
    });
}

#[test]
fn internal_fire_brackets_dispatch_flag_against_reentrant_dispatch() {
    with_vm(|vm| {
        // R8 (mod.rs:596): an internally-fired event is bracketed in the
        // dispatch-flag set for its walk, so a handler that captures it and
        // re-dispatches it re-entrantly throws InvalidStateError (previously
        // only the script-facing `dispatchEvent` wrapper added the bracket).
        vm.eval(
            "globalThis.__err = 'none';
             const open = indexedDB.open('db_internal_reentry', 1);
             open.onsuccess = (e) => {
                 try { open.dispatchEvent(e); }
                 catch (err) { globalThis.__err = err.name; }
             };",
        )
        .unwrap();
        assert_eq!(eval_string(vm, "globalThis.__err"), "InvalidStateError");
    });
}

#[test]
fn once_listener_unreached_by_stop_immediate_survives() {
    with_vm(|vm| {
        // R8 (mod.rs:348): `once` listeners are pruned at the point each is
        // invoked, NOT all up front.  A `once` listener that the walk never
        // reaches (an earlier `stopImmediatePropagation()` halted it) must
        // survive for a later event.
        vm.eval(
            "globalThis.__o = indexedDB.open('db_once_survive', 1);
             globalThis.__log = [];
             globalThis.__o.addEventListener('foo', (e) => {
                 globalThis.__log.push('S'); e.stopImmediatePropagation();
             }, { once: true });
             globalThis.__o.addEventListener('foo', () => {
                 globalThis.__log.push('L');
             }, { once: true });
             globalThis.__o.dispatchEvent(new Event('foo')); // S runs+removed, L unreached
             globalThis.__o.dispatchEvent(new Event('foo')); // L now runs (it survived)",
        )
        .unwrap();
        // With up-front pruning L would have been wrongly removed in dispatch 1
        // and the log would be just "S".
        assert_eq!(eval_string(vm, "globalThis.__log.join(',')"), "S,L");
    });
}

#[test]
fn add_event_listener_signal_removes_on_abort() {
    with_vm(|vm| {
        // R9 #2 / WHATWG DOM §2.7.3: a listener added with `{signal}` is
        // removed when the signal aborts, and an already-aborted signal never
        // adds it at all.  A plain listener added afterward still fires.
        vm.eval(
            "globalThis.__log = [];
             const o = indexedDB.open('db_sig', 1);
             const ctrl = new AbortController();
             o.addEventListener('foo', () => { globalThis.__log.push('live'); }, { signal: ctrl.signal });
             const aborted = AbortSignal.abort();
             o.addEventListener('foo', () => { globalThis.__log.push('pre-aborted'); }, { signal: aborted });
             ctrl.abort();                        // removes the live listener
             o.dispatchEvent(new Event('foo'));   // neither bound listener fires
             o.addEventListener('foo', () => { globalThis.__log.push('plain'); });
             o.dispatchEvent(new Event('foo'));   // only the plain listener fires",
        )
        .unwrap();
        assert_eq!(eval_string(vm, "globalThis.__log.join(',')"), "plain");
    });
}

#[test]
fn dispatch_event_returns_false_for_precanceled_event_with_no_listeners() {
    with_vm(|vm| {
        // R10 #1 / §2.9: `dispatchEvent` returns `false` for an event whose
        // default was already prevented, even with NO matching listeners (the
        // shared dispatcher's no-observer early-return must not mask it).
        vm.eval("globalThis.__o = indexedDB.open('db_precancel', 1);")
            .unwrap();
        assert!(eval_bool(
            vm,
            "(() => { const e = new Event('x', { cancelable: true }); e.preventDefault(); \
             return globalThis.__o.dispatchEvent(e) === false; })()"
        ));
        // A fresh cancelable event with no listeners is not canceled → true.
        assert!(eval_bool(
            vm,
            "(() => { const e = new Event('y', { cancelable: true }); \
             return globalThis.__o.dispatchEvent(e) === true; })()"
        ));
    });
}

// Placed here (not in `tests_indexeddb.rs`) only to keep that module under the
// ~1000-line convention — this exercises add()'s value-clone *ordering*.
#[test]
fn inline_store_explicit_key_rejects_before_running_value_side_effects() {
    with_vm(|vm| {
        // R10 #2 / §10.2.4: the inline-key DataError is thrown BEFORE the value
        // is cloned, so the value's `toJSON` / getter side effects never run.
        vm.eval(
            "globalThis.__log = [];
             const open = indexedDB.open('db_pre_clone', 1);
             open.onupgradeneeded = (e) => {
                 e.target.result.createObjectStore('s', { keyPath: 'id' });
             };
             open.onsuccess = (e) => {
                 const tx = e.target.result.transaction(['s'], 'readwrite');
                 const val = { id: 1, toJSON() { globalThis.__log.push('cloned'); return { id: 1 }; } };
                 try { tx.objectStore('s').add(val, 1); globalThis.__log.push('no-throw'); }
                 catch (err) { globalThis.__log.push('threw:' + err.name); }
             };",
        )
        .unwrap();
        // DataError thrown, and `toJSON` ('cloned') never ran.
        assert_eq!(
            eval_string(vm, "globalThis.__log.join(',')"),
            "threw:DataError"
        );
    });
}

#[test]
fn explicit_abort_fires_error_on_pending_requests() {
    with_vm(|vm| {
        // R11 #1 / §5.5: aborting a transaction fires `error` (AbortError) at
        // each still-pending request, so `req.onerror` IS notified — then the
        // transaction's `abort` event fires (request errors drain first).
        vm.eval(
            "globalThis.__log = [];
             const open = indexedDB.open('db_abort_req', 1);
             open.onupgradeneeded = (e) => { e.target.result.createObjectStore('s'); };
             open.onsuccess = (e) => {
                 const tx = e.target.result.transaction(['s'], 'readwrite');
                 const req = tx.objectStore('s').add('v', 1);
                 req.onerror = () => { globalThis.__log.push('req-error:' + req.error.name); };
                 tx.onabort = () => { globalThis.__log.push('tx-abort'); };
                 tx.abort();
             };",
        )
        .unwrap();
        assert_eq!(
            eval_string(vm, "globalThis.__log.join(',')"),
            "req-error:AbortError,tx-abort"
        );
    });
}

// Placed here (not in `tests_indexeddb.rs`) only to keep that module under the
// ~1000-line convention — this is the upgrade-abort connection-close lifecycle.
#[test]
fn aborted_upgrade_closes_the_database_connection() {
    with_vm(|vm| {
        // R11 #2 / §5.1: a failed upgrade closes the connection — a `db`
        // stashed during `upgradeneeded` is no longer usable after the version
        // rollback, so `db.transaction()` throws `InvalidStateError`.
        vm.eval(
            "globalThis.__err = 'none';
             const open = indexedDB.open('db_up_close', 1);
             open.onupgradeneeded = (e) => {
                 globalThis.__db = e.target.result;
                 e.target.result.createObjectStore('s');
                 throw new Error('boom'); // aborts the upgrade
             };
             open.onerror = (e) => {
                 e.preventDefault();
                 try { globalThis.__db.transaction(['s'], 'readonly'); globalThis.__err = 'no-throw'; }
                 catch (err) { globalThis.__err = err.name; }
             };",
        )
        .unwrap();
        assert_eq!(eval_string(vm, "globalThis.__err"), "InvalidStateError");
    });
}

// Placed here (not in `tests_indexeddb.rs`) only to keep that module under the
// ~1000-line convention — this is the transaction-scope dedup rule.
#[test]
fn transaction_store_names_are_deduplicated() {
    with_vm(|vm| {
        // R12 / §4.4: `transaction(storeNames)` scope is the SET of unique
        // names — a duplicate in the sequence is removed, so
        // `tx.objectStoreNames` exposes each store once.
        vm.eval(
            "globalThis.__log = [];
             const open = indexedDB.open('db_dedup', 1);
             open.onupgradeneeded = (e) => { e.target.result.createObjectStore('s'); };
             open.onsuccess = (e) => {
                 const tx = e.target.result.transaction(['s', 's'], 'readonly');
                 globalThis.__log.push('len:' + tx.objectStoreNames.length);
                 globalThis.__log.push('first:' + tx.objectStoreNames[0]);
             };",
        )
        .unwrap();
        assert_eq!(
            eval_string(vm, "globalThis.__log.join(',')"),
            "len:1,first:s"
        );
    });
}

#[test]
fn remove_event_listener_rejects_non_callable_argument() {
    with_vm(|vm| {
        // R14 #2 / WebIDL `EventListener? callback`: a non-callable, non-null
        // argument throws TypeError (matching the shared
        // `EventTarget.removeEventListener`), not a silent no-op; `null` is the
        // documented no-op.
        vm.eval("globalThis.__o = indexedDB.open('db_rel', 1);")
            .unwrap();
        assert!(eval_bool(
            vm,
            "(() => { try { globalThis.__o.removeEventListener('x', 42); return false; } \
             catch (e) { return e instanceof TypeError; } })()"
        ));
        assert!(eval_bool(
            vm,
            "(() => { try { globalThis.__o.removeEventListener('x', {}); return false; } \
             catch (e) { return e instanceof TypeError; } })()"
        ));
        // null callback is a silent no-op (no throw).
        assert!(eval_bool(
            vm,
            "(() => { try { globalThis.__o.removeEventListener('x', null); return true; } \
             catch (e) { return false; } })()"
        ));
    });
}

#[test]
fn idb_prototype_survives_gc_after_global_constructor_deleted() {
    with_vm(|vm| {
        // R14 #4: the 8 cached IDB interface prototypes are GC roots, so
        // severing a global constructor (its `.prototype` back-reference) does
        // NOT let the prototype be swept while `VmInner` still hands its
        // `ObjectId` to later host-created IDB objects.  Deterministic: the
        // prototype's only remaining references after `delete` are the now-dead
        // constructor cycle and the `VmInner` slot, so without the proto-root
        // it would be collected here.
        let proto = vm
            .inner
            .idb_request_prototype
            .expect("idb_request_prototype is registered");
        vm.eval("delete globalThis.IDBRequest;").unwrap();
        vm.inner.collect_garbage();
        assert!(
            vm.inner.objects[proto.0 as usize].is_some(),
            "IDBRequest.prototype was collected after deleting the global constructor",
        );
    });
}

// Placed here (not in `tests_indexeddb.rs`) only to keep that module under the
// ~1000-line convention — these are clone / query-arg validation rules.
#[test]
fn add_value_with_nested_function_throws_data_clone_error() {
    with_vm(|vm| {
        // R15 #3 / §5.11: a function (or symbol) nested in an otherwise
        // serializable value is a `DataCloneError` — NOT silently dropped by
        // `JSON.stringify` (which would corrupt the stored value).
        vm.eval(
            "globalThis.__err = 'none';
             const open = indexedDB.open('db_nested_fn', 1);
             open.onupgradeneeded = (e) => { e.target.result.createObjectStore('s'); };
             open.onsuccess = (e) => {
                 const tx = e.target.result.transaction(['s'], 'readwrite');
                 try { tx.objectStore('s').add({ a: 1, f() {} }, 1); }
                 catch (err) { globalThis.__err = err.name; }
             };",
        )
        .unwrap();
        assert_eq!(eval_string(vm, "globalThis.__err"), "DataCloneError");
    });
}

#[test]
fn add_stores_structured_clone_not_tojson_output() {
    with_vm(|vm| {
        // R16 / §5.11: IDB stores the STRUCTURED-CLONED value, which must not
        // invoke `toJSON` — persisting a class instance stores its own data
        // properties ({a:1}), not the (inherited) `toJSON`'s return value.
        vm.eval(
            "globalThis.__log = [];
             const open = indexedDB.open('db_tojson', 1);
             open.onupgradeneeded = (e) => { e.target.result.createObjectStore('s'); };
             open.onsuccess = (e) => {
                 const store = e.target.result.transaction(['s'], 'readwrite').objectStore('s');
                 class Foo { constructor() { this.a = 1; } toJSON() { return 'HOOKED'; } }
                 store.add(new Foo(), 1);
                 const g = store.get(1);
                 g.onsuccess = () => { globalThis.__log.push(JSON.stringify(g.result)); };
             };",
        )
        .unwrap();
        assert_eq!(eval_string(vm, "globalThis.__log.join(',')"), "{\"a\":1}");
    });
}

#[test]
fn get_all_with_null_query_throws_data_error() {
    with_vm(|vm| {
        // R15 #4 / §4.5: a SUPPLIED `null` query is not an omitted optional
        // argument — it goes through key conversion and fails with `DataError`
        // (whereas an omitted query returns all records).
        vm.eval(
            "globalThis.__log = [];
             const open = indexedDB.open('db_null_q', 1);
             open.onupgradeneeded = (e) => { e.target.result.createObjectStore('s'); };
             open.onsuccess = (e) => {
                 const store = e.target.result.transaction(['s'], 'readonly').objectStore('s');
                 try { store.getAll(null); globalThis.__log.push('null:no-throw'); }
                 catch (err) { globalThis.__log.push('null:' + err.name); }
                 store.getAll().onsuccess = () => { globalThis.__log.push('omitted:ok'); };
             };",
        )
        .unwrap();
        assert_eq!(
            eval_string(vm, "globalThis.__log.join(',')"),
            "null:DataError,omitted:ok"
        );
    });
}

// Note: two GC-rooting fixes here are verified by construction, not a
// deterministic test (a use-after-free is only observable if the freed slot is
// reused before the stale reference is read, which the heap does not
// guarantee — same caveat as the wrapper/listener rooting tests).
//   * R6: the once-listener `push_stack_scope` rooting in
//     `dispatch_idb_event` (the established idiom for values held only in Rust
//     locals across `call_function`, cf. `natives_array_hof`).
//   * R14 #1: rooting non-`Finished` transactions in `gc::mark_roots` — a
//     zero-request transaction is only `Active` *inside* `drain_tasks` (before
//     the auto-commit sweep), a window not reachable between `Vm::eval` calls,
//     so it cannot be set up deterministically from the test harness.
