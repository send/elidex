//! Cross-feature integration tests for PR5b (`HTMLCollection` +
//! `NodeList` + `NamedNodeMap` + `Attr` + `postMessage` +
//! `structuredClone`).
//!
//! Exercises combinations that individual sub-tranche tests do not
//! cover: live-collection observation through
//! `element.children.length` mutation windows, `structuredClone`
//! interaction with the message-dispatch path, task ordering
//! alongside microtask reactions, and identity guarantees across
//! multi-eval boundaries.

#![cfg(feature = "engine")]

use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;

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

fn eval_number(vm: &mut Vm, source: &str) -> f64 {
    match vm.eval(source).unwrap() {
        JsValue::Number(n) => n,
        other => panic!("expected number, got {other:?}"),
    }
}

fn eval_bool(vm: &mut Vm, source: &str) -> bool {
    match vm.eval(source).unwrap() {
        JsValue::Boolean(b) => b,
        other => panic!("expected bool, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// postMessage + structuredClone — cycle payload survives round-trip
// with Element references stripped (HostObject is unclonable, so a
// `{el}` payload must throw DataCloneError; this is the cross-feature
// contract the individual tests do not verify together).
// ---------------------------------------------------------------------------

#[test]
fn post_message_with_element_payload_throws_data_clone_error() {
    setup_bound_vm!(vm, session, dom, doc);
    // Structured clone rejects Element wrappers (HostObject).
    // The sync DataCloneError surfaces from postMessage, confirming
    // the §9.4.3 step 5 clone-before-origin-match ordering also
    // rejects DOM nodes captured in user payloads.
    let src = "
        var caught = null;
        try {
            var el = document.createElement('div');
            window.postMessage({node: el}, '*');
        } catch (e) { caught = e.name; }
        caught;";
    assert_eq!(eval_string(&mut vm, src), "DataCloneError");
    vm.unbind();
}

// ---------------------------------------------------------------------------
// postMessage + microtask checkpoint ordering — the spec's
// "microtask checkpoint after each task" means reactions attached
// inside a listener body run BEFORE the next postMessage task, not
// after the full drain completes.
// ---------------------------------------------------------------------------

#[test]
fn microtask_checkpoint_runs_between_tasks() {
    setup_bound_vm!(vm, session, dom, doc);
    // Listener for task #1 enqueues a microtask; task #2's listener
    // observes the microtask's side effect.  If microtasks only
    // drained at the END of drain_tasks, task #2 would see the
    // initial `0`.
    vm.eval(
        "globalThis.log = '';
         window.addEventListener('message', function(e){
             globalThis.log += e.data;
             if (e.data === 'a') {
                 Promise.resolve().then(function(){ globalThis.log += '-mt-'; });
             }
         });
         window.postMessage('a', '*');
         window.postMessage('b', '*');",
    )
    .unwrap();
    assert_eq!(eval_string(&mut vm, "globalThis.log;"), "a-mt-b");
    vm.unbind();
}

// ---------------------------------------------------------------------------
// Live HTMLCollection observation after dispatch-time mutation
// — the collection is re-read on every property access, so adding a
// child from inside a 'message' listener reflects on subsequent
// reads through the same wrapper.
// ---------------------------------------------------------------------------

#[test]
fn live_collection_reflects_mutation_from_message_listener() {
    setup_bound_vm!(vm, session, dom, doc);
    vm.eval(
        "globalThis.root = document.createElement('div');
         globalThis.before = globalThis.root.children.length;
         window.addEventListener('message', function(){
             globalThis.root.appendChild(document.createElement('span'));
         });
         window.postMessage(0, '*');",
    )
    .unwrap();
    assert_eq!(eval_number(&mut vm, "globalThis.before;"), 0.0);
    assert_eq!(
        eval_number(&mut vm, "globalThis.root.children.length;"),
        1.0
    );
    vm.unbind();
}

// ---------------------------------------------------------------------------
// structuredClone of a fresh NodeList is a DataCloneError — the
// wrapper is not a cloneable platform object.
// ---------------------------------------------------------------------------

#[test]
fn structured_clone_rejects_node_list_wrapper() {
    setup_bound_vm!(vm, session, dom, doc);
    let src = "
        var caught = null;
        var root = document.createElement('div');
        root.appendChild(document.createElement('span'));
        try { structuredClone(root.childNodes); } catch (e) { caught = e.name; }
        caught;";
    assert_eq!(eval_string(&mut vm, src), "DataCloneError");
    vm.unbind();
}

// ---------------------------------------------------------------------------
// Listener body that issues another postMessage — the re-entrant
// post is deferred (task_drain_depth guard) until after the current
// drain iteration finishes its microtask checkpoint.  Because we
// DON'T reentrantly drain, the nested `postMessage` is picked up by
// the outer drain's next iteration.
// ---------------------------------------------------------------------------

#[test]
fn nested_post_message_is_picked_up_by_outer_drain() {
    setup_bound_vm!(vm, session, dom, doc);
    vm.eval(
        "globalThis.order = '';
         window.addEventListener('message', function(e){
             globalThis.order += e.data;
             if (e.data === 'a') window.postMessage('b', '*');
         });
         window.postMessage('a', '*');",
    )
    .unwrap();
    // Nested 'b' is queued during 'a' dispatch, then drained in the
    // same outer drain loop — both fire within the single eval.
    assert_eq!(eval_string(&mut vm, "globalThis.order;"), "ab");
    vm.unbind();
}
