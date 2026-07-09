//! `deliver_history_step_events` — the VM history-step UA event fire path
//! (WHATWG HTML §7.4.6.2 "update document for history step application").
//!
//! popstate fires **synchronously** (step 6.4.3 "fire an event"); hashchange is
//! **enqueued** as a task (step 6.4.5 "queue a global task on the DOM
//! manipulation task source"), so popstate is observed strictly before
//! hashchange.
//!
//! **Flip-inert (S5-5b)**: the VM fires these, but the live shell engine is
//! still boa (which does not implement `HostDriver`), so no shell path calls
//! this yet — these tests are the pre-flip oracle. It goes live at the S5-6
//! boa→VM cutover. Drives `Vm::inner::deliver_history_step_events` directly,
//! mirroring how `tests_post_message` drives the same-window task queue.

#![cfg(feature = "engine")]

use elidex_ecs::EcsDom;
use elidex_script_session::{HistoryAction, SessionCore};

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;

/// A bound VM/session/dom/doc quad kept on the test's own stack — the `bind_vm`
/// raw pointers must stay valid until `unbind`, so the bindings cannot move into
/// a struct (mirrors `tests_post_message`). The Window entity is allocated at
/// bind, which `deliver_history_step_events` dispatches at.
macro_rules! setup_bound_vm {
    ($vm:ident, $session:ident, $dom:ident, $doc:ident) => {
        let mut $vm = Vm::new();
        let mut $session = SessionCore::new();
        let mut $dom = EcsDom::new();
        let $doc = $dom.create_document_root();
        #[allow(unsafe_code)]
        unsafe {
            bind_vm(&mut $vm, &mut $session, &mut $dom, $doc);
        }
    };
}

fn eval_string(vm: &mut Vm, source: &str) -> String {
    match vm.eval(source).unwrap() {
        JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string, got {other:?}"),
    }
}

/// Install popstate/hashchange Window listeners that record the fire ORDER —
/// plus a microtask queued from the popstate handler, so the assertion pins that
/// popstate's synchronous microtask checkpoint completes BEFORE the enqueued
/// hashchange task — and capture popstate `state` + hashchange `oldURL`/`newURL`.
fn install_recorders(vm: &mut Vm) {
    vm.eval(
        "globalThis.order = [];
         globalThis.popStateType = 'UNSET';
         globalThis.hashOld = 'UNSET';
         globalThis.hashNew = 'UNSET';
         window.addEventListener('popstate', function (e) {
             globalThis.order.push('popstate');
             // `typeof null` is 'object', so record a discriminating tag.
             globalThis.popStateType = (e.state === null) ? 'null' : String(e.state);
             Promise.resolve().then(function () {
                 globalThis.order.push('popstate-microtask');
             });
         });
         window.addEventListener('hashchange', function (e) {
             globalThis.order.push('hashchange');
             globalThis.hashOld = e.oldURL;
             globalThis.hashNew = e.newURL;
         });",
    )
    .unwrap();
}

#[test]
fn popstate_sync_then_hashchange_enqueued_in_order() {
    setup_bound_vm!(vm, session, dom, doc);
    install_recorders(&mut vm);

    // Fragment nav: popstate with state=null, hashchange with differing frags.
    vm.inner.deliver_history_step_events(
        Some(None),
        Some(("http://x/a#old".to_string(), "http://x/a#new".to_string())),
    );

    // popstate fires SYNC and its microtask checkpoint completes strictly BEFORE
    // the ENQUEUED hashchange task runs (§7.4.6.2 step 6.4.3 fire vs 6.4.5
    // queue-a-task).
    assert_eq!(
        eval_string(&mut vm, "globalThis.order.join(',')"),
        "popstate,popstate-microtask,hashchange"
    );
    // 5b fragment nav: popstate `state` is null.
    assert_eq!(eval_string(&mut vm, "globalThis.popStateType"), "null");
    // hashchange carries the serialized old/new URLs.
    assert_eq!(eval_string(&mut vm, "globalThis.hashOld"), "http://x/a#old");
    assert_eq!(eval_string(&mut vm, "globalThis.hashNew"), "http://x/a#new");

    vm.unbind();
}

#[test]
fn popstate_and_hashchange_target_is_the_real_window() {
    // Regression pin (S5-5b VM finding): a Window-targeted UA dispatch must seed
    // `event.target` / `event.currentTarget` with the *real* Window — the global
    // object (`globalThis` / `window`, i.e. `VmInner::global_object`) — NOT a
    // parallel Node-prototype `HostObject` allocated for the window entity. Both
    // slots route through `create_element_wrapper(window_entity)`, which must
    // resolve the window entity to `global_object` (never a synthetic node
    // wrapper), so `popstate.target === window` / `hashchange.target === window`.
    // `currentTarget` is read INSIDE the handler because it is cleared at dispatch
    // finalize (as is `target`), so both are only observable mid-dispatch.
    setup_bound_vm!(vm, session, dom, doc);

    vm.eval(
        "globalThis.popTarget = 'UNSET';
         globalThis.popCurrent = 'UNSET';
         globalThis.hashTarget = 'UNSET';
         globalThis.hashCurrent = 'UNSET';
         window.addEventListener('popstate', function (e) {
             globalThis.popTarget = (e.target === globalThis) ? 'window' : 'other';
             globalThis.popCurrent = (e.currentTarget === globalThis) ? 'window' : 'other';
         });
         window.addEventListener('hashchange', function (e) {
             globalThis.hashTarget = (e.target === globalThis) ? 'window' : 'other';
             globalThis.hashCurrent = (e.currentTarget === globalThis) ? 'window' : 'other';
         });",
    )
    .unwrap();

    // Fragment nav: fires popstate (sync) then hashchange (enqueued task).
    vm.inner.deliver_history_step_events(
        Some(None),
        Some(("http://x/a#old".to_string(), "http://x/a#new".to_string())),
    );

    // popstate: both target and currentTarget are the real Window (=== globalThis).
    assert_eq!(
        eval_string(&mut vm, "globalThis.popTarget"),
        "window",
        "popstate.target must be the real window (globalThis), not a node wrapper"
    );
    assert_eq!(
        eval_string(&mut vm, "globalThis.popCurrent"),
        "window",
        "popstate.currentTarget must be the real window (globalThis)"
    );
    // hashchange (enqueued task): same invariant.
    assert_eq!(
        eval_string(&mut vm, "globalThis.hashTarget"),
        "window",
        "hashchange.target must be the real window (globalThis), not a node wrapper"
    );
    assert_eq!(
        eval_string(&mut vm, "globalThis.hashCurrent"),
        "window",
        "hashchange.currentTarget must be the real window (globalThis)"
    );

    vm.unbind();
}

#[test]
fn popstate_fires_without_hashchange_when_fragment_unchanged() {
    setup_bound_vm!(vm, session, dom, doc);
    install_recorders(&mut vm);

    // Identical-via-href (`location.href = currentURL`): oldFrag == newFrag, so
    // the shell passes `hashchange: None` — popstate STILL fires (state=null),
    // hashchange does NOT (§7.4.6.2 step 6.4.5 gates on the fragment differing).
    vm.inner.deliver_history_step_events(Some(None), None);

    assert_eq!(
        eval_string(&mut vm, "globalThis.order.join(',')"),
        "popstate,popstate-microtask"
    );
    assert_eq!(eval_string(&mut vm, "globalThis.popStateType"), "null");
    // hashchange never fired — the recorders' sentinel is untouched.
    assert_eq!(eval_string(&mut vm, "globalThis.hashOld"), "UNSET");

    vm.unbind();
}

#[test]
fn neither_fires_when_popstate_state_is_none() {
    setup_bound_vm!(vm, session, dom, doc);
    install_recorders(&mut vm);

    // `popstate_state: None` ⇒ do not fire popstate; `hashchange: None` ⇒ do not
    // fire hashchange. A no-op deliver (e.g. a same-document application the
    // shell decided fires nothing).
    vm.inner.deliver_history_step_events(None, None);

    assert_eq!(eval_string(&mut vm, "globalThis.order.join(',')"), "");
    assert_eq!(eval_string(&mut vm, "globalThis.popStateType"), "UNSET");
    assert_eq!(eval_string(&mut vm, "globalThis.hashOld"), "UNSET");

    vm.unbind();
}

#[test]
fn deliver_is_a_noop_when_unbound() {
    // No bind ⇒ no Window entity to dispatch at ⇒ the `is_bound` gate makes the
    // whole deliver a silent no-op (never panics resolving a missing window).
    let mut vm = Vm::new();
    vm.inner.deliver_history_step_events(
        Some(None),
        Some(("http://x/a".to_string(), "http://x/a#f".to_string())),
    );
    // Reached here without panic — the unbound gate held.
}

#[test]
fn traversal_popstate_fires_with_restored_state() {
    // 5c same-document TRAVERSAL to a pushState'd entry: the shell delivers the
    // entry's `StructuredSerializeForStorage` bytes (JSON-shortcut interim) as
    // `Some(Some(bytes))`. popstate fires with `StructuredDeserialize(bytes)`, and
    // `history.state` is restored to it BEFORE the fire (§7.4.6.2 step 6.3 → 6.4.3)
    // — so a synchronous handler reads `history.state === popstate.state`.
    setup_bound_vm!(vm, session, dom, doc);
    vm.eval(
        "globalThis.popN = 'UNSET';
         globalThis.histN = 'UNSET';
         window.addEventListener('popstate', function (e) {
             globalThis.popN = (e.state === null) ? 'null' : String(e.state.n);
             globalThis.histN = (history.state === null) ? 'null' : String(history.state.n);
         });",
    )
    .unwrap();

    vm.inner
        .deliver_history_step_events(Some(Some(b"{\"n\":1}".to_vec())), None);

    assert_eq!(eval_string(&mut vm, "globalThis.popN"), "1");
    assert_eq!(eval_string(&mut vm, "globalThis.histN"), "1");
    // history.state persists as the restored value after the traversal.
    assert_eq!(
        eval_string(&mut vm, "JSON.stringify(history.state)"),
        "{\"n\":1}"
    );

    vm.unbind();
}

#[test]
fn traversal_across_fragment_fires_popstate_restored_and_hashchange() {
    // The §4.5 matrix row "traversal → same-document entry WITH state, fragment
    // differs": popstate (restored state) fires SYNC, then hashchange as a later
    // task — popstate strictly before hashchange, and popstate carries the restored
    // value (not null).
    setup_bound_vm!(vm, session, dom, doc);
    vm.eval(
        "globalThis.order = [];
         globalThis.popN = 'UNSET';
         window.addEventListener('popstate', function (e) {
             globalThis.order.push('popstate');
             globalThis.popN = (e.state === null) ? 'null' : String(e.state.n);
         });
         window.addEventListener('hashchange', function () {
             globalThis.order.push('hashchange');
         });",
    )
    .unwrap();

    vm.inner.deliver_history_step_events(
        Some(Some(b"{\"n\":5}".to_vec())),
        Some(("http://x/a#old".to_string(), "http://x/a#new".to_string())),
    );

    assert_eq!(
        eval_string(&mut vm, "globalThis.order.join(',')"),
        "popstate,hashchange"
    );
    assert_eq!(eval_string(&mut vm, "globalThis.popN"), "5");

    vm.unbind();
}

#[test]
fn traversal_to_no_state_entry_fires_popstate_null() {
    // The §4.5 matrix row "traversal → same-document entry, NO state" (a plain-nav
    // or boa-`None` entry): the outer `Some` still fires popstate (the entry
    // changed — state-agnostic), with `state = null` (`Some(None)`). Distinct from
    // `None` (which would SKIP popstate — spec-wrong for a changed entry).
    setup_bound_vm!(vm, session, dom, doc);
    install_recorders(&mut vm);

    vm.inner.deliver_history_step_events(Some(None), None);

    assert_eq!(
        eval_string(&mut vm, "globalThis.order.join(',')"),
        "popstate,popstate-microtask"
    );
    assert_eq!(eval_string(&mut vm, "globalThis.popStateType"), "null");

    vm.unbind();
}

#[test]
fn traversal_restores_pushed_state_round_trip() {
    // End-to-end round-trip through `StructuredSerializeForStorage` (§7.2.5 step 3,
    // the pushState serialize) + `StructuredDeserialize` (§7.4.6.2 restore step 2,
    // the traversal deliver): pushState serializes `{n:7}` onto the enqueued
    // action; a same-document traversal to that entry delivers those very bytes,
    // and popstate fires with the deserialized value — restoring `history.state`
    // over a since-changed value.
    setup_bound_vm!(vm, session, dom, doc);
    vm.inner
        .navigation
        .set_current_url(Some(url::Url::parse("http://x/a").unwrap()));
    vm.eval(
        "globalThis.popN = 'UNSET';
         globalThis.histN = 'UNSET';
         window.addEventListener('popstate', function (e) {
             globalThis.popN = (e.state === null) ? 'null' : String(e.state.n);
             globalThis.histN = (history.state === null) ? 'null' : String(history.state.n);
         });",
    )
    .unwrap();

    // pushState serializes {n: 7} onto the enqueued action.
    vm.eval("history.pushState({n: 7}, '', '/b');").unwrap();
    let bytes = match std::mem::take(&mut vm.inner.navigation.pending_history).pop_front() {
        Some(HistoryAction::PushState {
            serialized_state, ..
        }) => serialized_state.expect("VM serializes state"),
        other => panic!("expected PushState with serialized_state, got {other:?}"),
    };
    // Change `history.state` so the restore is observable (not already {n:7}).
    vm.eval("history.replaceState({n: 99}, '');").unwrap();

    // A same-document traversal back to the {n:7} entry: deliver its bytes.
    vm.inner
        .deliver_history_step_events(Some(Some(bytes)), None);

    assert_eq!(eval_string(&mut vm, "globalThis.popN"), "7");
    assert_eq!(eval_string(&mut vm, "globalThis.histN"), "7");
    assert_eq!(eval_string(&mut vm, "String(history.state.n)"), "7");

    vm.unbind();
}

#[test]
fn fragment_nav_popstate_resets_history_state_to_null() {
    // A fragment nav fires popstate with state=null AND must reset the persistent
    // `history.state` getter to null (§7.4.6.2 step 6.3 "restore the history
    // object's state" before step 6.4.3 fire; §7.4.2.3.3 step 11.1 "Set history's
    // state to null"). Regression pin: without the restore, popstate.state is null
    // but `history.state` keeps the stale pre-nav `pushState` value.
    setup_bound_vm!(vm, session, dom, doc);

    // pushState sets the classic history state; `history.state` reflects it.
    vm.eval("history.pushState({a: 1}, '')").unwrap();
    assert_eq!(
        eval_string(&mut vm, "JSON.stringify(history.state)"),
        "{\"a\":1}"
    );

    // Inside the popstate handler, `history.state` must already be null and agree
    // with `event.state` (both null) — the restore happens BEFORE the fire.
    vm.eval(
        "globalThis.stateInHandler = 'UNSET';
         window.addEventListener('popstate', function (e) {
             globalThis.stateInHandler = (history.state === null && e.state === null)
                 ? 'both-null'
                 : String(history.state) + '/' + String(e.state);
         });",
    )
    .unwrap();

    // Fragment nav: popstate with state=null.
    vm.inner.deliver_history_step_events(Some(None), None);

    // The handler observed both null, and `history.state` stays null afterward
    // (not the stale `{a:1}`).
    assert_eq!(
        eval_string(&mut vm, "globalThis.stateInHandler"),
        "both-null"
    );
    assert_eq!(eval_string(&mut vm, "String(history.state)"), "null");

    vm.unbind();
}

#[test]
fn seed_history_state_restores_without_firing_popstate() {
    // The CROSS-document-traversal seed (J5/J6): `seed_history_state` restores
    // `history.state` from the target entry's bytes WITHOUT firing popstate (the
    // rebuilt document is `documentIsNew=true`, so §7.4.6.2 step 6.4 is skipped) —
    // distinct from `deliver_history_step_events`, which fires. A fresh document's
    // initial script reads the restored state; no popstate handler runs.
    setup_bound_vm!(vm, session, dom, doc);
    install_recorders(&mut vm);

    vm.inner.seed_history_state(Some(b"{\"n\":3}".to_vec()));

    // history.state restored...
    assert_eq!(
        eval_string(&mut vm, "JSON.stringify(history.state)"),
        "{\"n\":3}"
    );
    // ...but NOTHING fired (the seed is restore-without-fire — no popstate).
    assert_eq!(eval_string(&mut vm, "globalThis.order.join(',')"), "");
    assert_eq!(eval_string(&mut vm, "globalThis.popStateType"), "UNSET");

    vm.unbind();
}

#[test]
fn seed_history_state_none_is_null() {
    // A fresh navigation / reload (not a traversal) seeds `None` → `history.state`
    // is `null` in the rebuilt document.
    setup_bound_vm!(vm, session, dom, doc);
    vm.inner.seed_history_state(None);
    assert_eq!(eval_string(&mut vm, "String(history.state)"), "null");
    vm.unbind();
}
