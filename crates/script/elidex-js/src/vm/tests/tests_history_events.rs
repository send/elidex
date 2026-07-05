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
use elidex_script_session::SessionCore;

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
