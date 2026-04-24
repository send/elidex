//! Tests for `window.postMessage(message, targetOrigin, transfer?)`
//! (WHATWG HTML §9.4.3) + the shared same-window task queue
//! infrastructure (`vm/host/pending_tasks.rs`).
//!
//! Covers argument-count validation, legacy / dict-form signature
//! dispatch, targetOrigin matching (`*` / `/` / URL / malformed),
//! structuredClone integration (clone failures surface synchronously),
//! transfer list validation, task FIFO ordering, and the actual
//! MessageEvent delivery to `addEventListener('message', ...)`
//! listeners.
//!
//! ## Eval vs. drain timing
//!
//! Listener delivery happens in `drain_tasks`, which runs at the end
//! of each `vm.eval()` call (after the microtask flush).  A test that
//! posts a message and reads the listener-mutated state in the same
//! `eval` would observe the pre-drain value — the `postMessage` call
//! only queues a task, it does not synchronously invoke listeners.
//! So every delivery test runs in two `eval` calls: the first posts
//! (drain fires on its return), the second reads the resulting
//! global state.

#![cfg(feature = "engine")]

use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;

/// Inline-construct the bound VM / session / dom trio.  The
/// `bind_vm` call stores raw pointers into `session` / `dom`, so all
/// three must live on the caller's stack for the whole test — moving
/// them into a `struct` after `bind_vm` invalidates the stored
/// pointers (the struct's storage moves during the return).  The
/// macro keeps every binding on the test function's own stack.
///
/// Expands to four named bindings (`$vm` / `$session` / `$dom` /
/// `$doc`); callers access `$vm` for eval and must drop via
/// `$vm.unbind()` before the bindings go out of scope.
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
// Binding-level validation
// ---------------------------------------------------------------------------

#[test]
fn post_message_without_args_throws_type_error() {
    setup_bound_vm!(vm, session, dom, doc);
    let err = vm.eval("window.postMessage();");
    assert!(err.is_err(), "expected TypeError for missing arg");
    vm.unbind();
}

#[test]
fn post_message_primitive_target_origin_coerces_to_string() {
    setup_bound_vm!(vm, session, dom, doc);
    // Numeric targetOrigin coerces via ToString → "42", which fails
    // to parse as a URL → SyntaxError (DOMException).
    let src = "
        var caught = null;
        try { window.postMessage('x', 42); }
        catch (e) { caught = e.name; }
        caught;";
    assert_eq!(eval_string(&mut vm, src), "SyntaxError");
    vm.unbind();
}

// ---------------------------------------------------------------------------
// targetOrigin matching
// ---------------------------------------------------------------------------

#[test]
fn post_message_star_origin_delivers() {
    setup_bound_vm!(vm, session, dom, doc);
    vm.eval(
        "globalThis.hit = 0;
         window.addEventListener('message', function(e){ globalThis.hit = e.data; });
         window.postMessage(42, '*');",
    )
    .unwrap();
    // Drain fires on `eval` return; the next `eval` observes the
    // mutated global.
    assert_eq!(eval_number(&mut vm, "globalThis.hit;"), 42.0);
    vm.unbind();
}

#[test]
fn post_message_slash_origin_delivers_to_same_window() {
    setup_bound_vm!(vm, session, dom, doc);
    vm.eval(
        "globalThis.hit = '';
         window.addEventListener('message', function(e){ globalThis.hit = e.data; });
         window.postMessage('ok', '/');",
    )
    .unwrap();
    assert_eq!(eval_string(&mut vm, "globalThis.hit;"), "ok");
    vm.unbind();
}

#[test]
fn post_message_mismatched_url_origin_drops_silently() {
    setup_bound_vm!(vm, session, dom, doc);
    // Own origin is opaque ('about:blank' → WHATWG "null origin"),
    // so `https://example.com` cannot match; listener must not fire.
    vm.eval(
        "globalThis.hit = 0;
         window.addEventListener('message', function(e){ globalThis.hit = 1; });
         window.postMessage('x', 'https://example.com');",
    )
    .unwrap();
    assert_eq!(eval_number(&mut vm, "globalThis.hit;"), 0.0);
    vm.unbind();
}

#[test]
fn post_message_malformed_origin_throws_syntax_error() {
    setup_bound_vm!(vm, session, dom, doc);
    let src = "
        var caught = null;
        try { window.postMessage('x', 'not a url'); }
        catch (e) { caught = e.name; }
        caught;";
    assert_eq!(eval_string(&mut vm, src), "SyntaxError");
    vm.unbind();
}

// ---------------------------------------------------------------------------
// MessageEvent shape
// ---------------------------------------------------------------------------

#[test]
fn message_event_carries_cloned_structured_data() {
    setup_bound_vm!(vm, session, dom, doc);
    vm.eval(
        "globalThis.sawInner = 0;
         globalThis.sameIdentity = true;
         globalThis.original = {inner: {n: 7}};
         window.addEventListener('message', function(e){
             globalThis.sawInner = e.data.inner.n;
             globalThis.sameIdentity = (e.data === globalThis.original);
         });
         window.postMessage(globalThis.original, '*');",
    )
    .unwrap();
    assert_eq!(eval_number(&mut vm, "globalThis.sawInner;"), 7.0);
    assert!(!eval_bool(&mut vm, "globalThis.sameIdentity;"));
    vm.unbind();
}

#[test]
fn message_event_type_is_message() {
    setup_bound_vm!(vm, session, dom, doc);
    vm.eval(
        "globalThis.eventType = '';
         window.addEventListener('message', function(e){ globalThis.eventType = e.type; });
         window.postMessage(0, '*');",
    )
    .unwrap();
    assert_eq!(eval_string(&mut vm, "globalThis.eventType;"), "message");
    vm.unbind();
}

#[test]
fn message_event_source_is_window() {
    setup_bound_vm!(vm, session, dom, doc);
    vm.eval(
        "globalThis.matches = false;
         window.addEventListener('message', function(e){
             globalThis.matches = (e.source === window);
         });
         window.postMessage(0, '*');",
    )
    .unwrap();
    assert!(eval_bool(&mut vm, "globalThis.matches;"));
    vm.unbind();
}

#[test]
fn message_event_ports_is_empty_array() {
    setup_bound_vm!(vm, session, dom, doc);
    vm.eval(
        "globalThis.portsLen = -1;
         globalThis.portsIsArray = false;
         window.addEventListener('message', function(e){
             globalThis.portsLen = e.ports.length;
             globalThis.portsIsArray = Array.isArray(e.ports);
         });
         window.postMessage(0, '*');",
    )
    .unwrap();
    assert_eq!(eval_number(&mut vm, "globalThis.portsLen;"), 0.0);
    assert!(eval_bool(&mut vm, "globalThis.portsIsArray;"));
    vm.unbind();
}

#[test]
fn message_event_last_event_id_is_empty_string() {
    setup_bound_vm!(vm, session, dom, doc);
    vm.eval(
        "globalThis.lid = 'unset';
         window.addEventListener('message', function(e){ globalThis.lid = e.lastEventId; });
         window.postMessage(0, '*');",
    )
    .unwrap();
    assert_eq!(eval_string(&mut vm, "globalThis.lid;"), "");
    vm.unbind();
}

// ---------------------------------------------------------------------------
// Task ordering + queue behaviour
// ---------------------------------------------------------------------------

#[test]
fn multiple_post_messages_fire_in_fifo_order() {
    setup_bound_vm!(vm, session, dom, doc);
    vm.eval(
        "globalThis.order = '';
         window.addEventListener('message', function(e){ globalThis.order += e.data; });
         window.postMessage('a', '*');
         window.postMessage('b', '*');
         window.postMessage('c', '*');",
    )
    .unwrap();
    assert_eq!(eval_string(&mut vm, "globalThis.order;"), "abc");
    vm.unbind();
}

#[test]
fn multiple_listeners_all_fire() {
    setup_bound_vm!(vm, session, dom, doc);
    vm.eval(
        "globalThis.count = 0;
         window.addEventListener('message', function(){ globalThis.count++; });
         window.addEventListener('message', function(){ globalThis.count++; });
         window.postMessage(0, '*');",
    )
    .unwrap();
    assert_eq!(eval_number(&mut vm, "globalThis.count;"), 2.0);
    vm.unbind();
}

#[test]
fn post_message_without_listener_drops_silently() {
    setup_bound_vm!(vm, session, dom, doc);
    // No listener registered — must not throw / panic.
    let _ = vm.eval("window.postMessage('vanish', '*');").unwrap();
    vm.unbind();
}

// ---------------------------------------------------------------------------
// structuredClone / transfer integration
// ---------------------------------------------------------------------------

#[test]
fn post_message_unclonable_throws_data_clone_error_sync() {
    setup_bound_vm!(vm, session, dom, doc);
    // Function is non-cloneable; the DataCloneError must surface
    // synchronously at the postMessage call site (spec §9.4.3 step
    // 5: clone runs *before* origin match).
    let src = "
        var caught = null;
        try { window.postMessage(function(){}, '*'); }
        catch (e) { caught = e.name; }
        caught;";
    assert_eq!(eval_string(&mut vm, src), "DataCloneError");
    vm.unbind();
}

#[test]
fn post_message_dict_form_accepts_target_origin() {
    setup_bound_vm!(vm, session, dom, doc);
    vm.eval(
        "globalThis.hit = 0;
         window.addEventListener('message', function(e){ globalThis.hit = e.data; });
         window.postMessage(5, {targetOrigin: '*'});",
    )
    .unwrap();
    assert_eq!(eval_number(&mut vm, "globalThis.hit;"), 5.0);
    vm.unbind();
}

#[test]
fn post_message_dict_form_default_target_origin_is_slash() {
    setup_bound_vm!(vm, session, dom, doc);
    // Omitted `targetOrigin` in options → default `"/"`, which
    // matches own origin (same window) so listener fires.
    vm.eval(
        "globalThis.hit = 0;
         window.addEventListener('message', function(e){ globalThis.hit = e.data; });
         window.postMessage(9, {});",
    )
    .unwrap();
    assert_eq!(eval_number(&mut vm, "globalThis.hit;"), 9.0);
    vm.unbind();
}

#[test]
fn post_message_empty_transfer_array_accepted() {
    setup_bound_vm!(vm, session, dom, doc);
    vm.eval(
        "globalThis.hit = 0;
         window.addEventListener('message', function(e){ globalThis.hit = e.data; });
         window.postMessage(11, '*', []);",
    )
    .unwrap();
    assert_eq!(eval_number(&mut vm, "globalThis.hit;"), 11.0);
    vm.unbind();
}

#[test]
fn post_message_nonempty_transfer_throws_data_clone_error() {
    setup_bound_vm!(vm, session, dom, doc);
    let src = "
        var caught = null;
        var buf = new ArrayBuffer(4);
        try { window.postMessage(buf, '*', [buf]); }
        catch (e) { caught = e.name; }
        caught;";
    assert_eq!(eval_string(&mut vm, src), "DataCloneError");
    vm.unbind();
}

// ---------------------------------------------------------------------------
// Cycle round-trip through postMessage
// ---------------------------------------------------------------------------

#[test]
fn post_message_with_cycle_round_trips() {
    setup_bound_vm!(vm, session, dom, doc);
    vm.eval(
        "globalThis.selfMatches = false;
         var a = {};
         a.self = a;
         window.addEventListener('message', function(e){
             globalThis.selfMatches = (e.data.self === e.data);
         });
         window.postMessage(a, '*');",
    )
    .unwrap();
    assert!(eval_bool(&mut vm, "globalThis.selfMatches;"));
    vm.unbind();
}

// ---------------------------------------------------------------------------
// stopImmediatePropagation short-circuits remaining listeners
// ---------------------------------------------------------------------------

#[test]
fn stop_immediate_propagation_halts_further_listeners() {
    setup_bound_vm!(vm, session, dom, doc);
    vm.eval(
        "globalThis.first = 0;
         globalThis.second = 0;
         window.addEventListener('message', function(e){
             globalThis.first = 1;
             e.stopImmediatePropagation();
         });
         window.addEventListener('message', function(){ globalThis.second = 1; });
         window.postMessage(0, '*');",
    )
    .unwrap();
    assert_eq!(eval_number(&mut vm, "globalThis.first;"), 1.0);
    assert_eq!(eval_number(&mut vm, "globalThis.second;"), 0.0);
    vm.unbind();
}

// ---------------------------------------------------------------------------
// Listener option semantics — {once} / {signal} / {passive}
//
// These exercise the listener-option state that was silently ignored
// by PR5b's initial manual-walk dispatch.  Routing MessageEvents
// through `dispatch_script_event` makes them observable.
// ---------------------------------------------------------------------------

#[test]
fn once_listener_fires_only_once() {
    // `{once: true}` listeners MUST auto-remove after their first
    // invocation (WHATWG DOM §2.10 step 15).
    setup_bound_vm!(vm, session, dom, doc);
    vm.eval(
        "globalThis.count = 0;
         window.addEventListener(
             'message',
             function(){ globalThis.count += 1; },
             { once: true });
         window.postMessage('a', '*');
         window.postMessage('b', '*');",
    )
    .unwrap();
    assert_eq!(eval_number(&mut vm, "globalThis.count;"), 1.0);
    vm.unbind();
}

#[test]
fn listener_with_aborted_signal_does_not_fire() {
    // Passing an already-aborted `AbortSignal` via
    // `addEventListener(..., {signal})` MUST skip registration
    // entirely (WHATWG DOM §2.10 step 4) — no invocation on
    // subsequent dispatch.
    setup_bound_vm!(vm, session, dom, doc);
    vm.eval(
        "globalThis.count = 0;
         var ctl = new AbortController();
         ctl.abort();
         window.addEventListener(
             'message',
             function(){ globalThis.count += 1; },
             { signal: ctl.signal });
         window.postMessage('x', '*');",
    )
    .unwrap();
    assert_eq!(eval_number(&mut vm, "globalThis.count;"), 0.0);
    vm.unbind();
}

// ---------------------------------------------------------------------------
// MessageEvent.target identity — must be canonical `window`, not a
// fresh HostObject wrapper allocated off the Window ECS entity.
// Regression guard for Copilot R1 #2 (Window wrapper identity bug
// introduced when C8 routed dispatch through `dispatch_script_event`
// without caching `(window_entity → global_object)` in
// `wrapper_cache`).
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Copilot R4 #2 regression — `validate_transfer` must follow WebIDL
// `sequence<object>` semantics.  A non-iterable object (e.g. `{}`)
// raises TypeError, NOT DataCloneError.  An empty iterable (e.g. a
// fresh `Set()`) passes.  A non-empty iterable raises
// DataCloneError (Phase 2 transferables unsupported).
// ---------------------------------------------------------------------------

#[test]
fn post_message_transfer_non_iterable_object_throws_type_error() {
    setup_bound_vm!(vm, session, dom, doc);
    // `{}` has no `@@iterator`, so WebIDL §3.2.27 step 3 raises
    // TypeError (not DataCloneError — the pre-R4 behavior).
    let src = "
        var caught = null;
        try { window.postMessage('x', '*', {}); }
        catch (e) { caught = (e instanceof TypeError) ? 'TypeError' : (e.name || 'other'); }
        caught;";
    assert_eq!(eval_string(&mut vm, src), "TypeError");
    vm.unbind();
}

#[test]
fn post_message_transfer_empty_iterable_non_array_accepted() {
    setup_bound_vm!(vm, session, dom, doc);
    // A user-defined empty iterable (not an Array) must pass the
    // WebIDL `sequence<object>` check: the `@@iterator` probe
    // succeeds and the first `next()` returns `done: true`.  VM
    // does not ship `Set` yet, so roll our own iterable.
    vm.eval(
        "globalThis.hit = 0;
         window.addEventListener('message', function(e){ globalThis.hit = e.data; });
         var emptyIterable = {
             [Symbol.iterator]: function(){
                 return { next: function(){ return {value: undefined, done: true}; } };
             }
         };
         window.postMessage(11, '*', emptyIterable);",
    )
    .unwrap();
    assert_eq!(eval_number(&mut vm, "globalThis.hit;"), 11.0);
    vm.unbind();
}

#[test]
fn message_event_target_is_canonical_window() {
    setup_bound_vm!(vm, session, dom, doc);
    vm.eval(
        "globalThis.targetIsWindow = null;
         globalThis.currentTargetIsWindow = null;
         window.addEventListener('message', function(e){
             globalThis.targetIsWindow = (e.target === window);
             globalThis.currentTargetIsWindow = (e.currentTarget === window);
         });
         window.postMessage('x', '*');",
    )
    .unwrap();
    assert!(eval_bool(&mut vm, "globalThis.targetIsWindow;"));
    assert!(eval_bool(&mut vm, "globalThis.currentTargetIsWindow;"));
    vm.unbind();
}

// ---------------------------------------------------------------------------
// Dict-form options read through WebIDL `Get` (§7.3.1), not the
// inner `storage.get` fast path — inherited / accessor-defined
// `targetOrigin` / `transfer` entries must be observable.
// Regression guard for Copilot R1 #5.
// ---------------------------------------------------------------------------

#[test]
fn post_message_dict_target_origin_inherited_from_prototype() {
    setup_bound_vm!(vm, session, dom, doc);
    // `targetOrigin` lives on the prototype, not the own object.
    // WebIDL conversion walks the proto chain → delivery should
    // succeed (star match).
    vm.eval(
        "globalThis.hit = 0;
         window.addEventListener('message', function(e){ globalThis.hit = e.data; });
         var proto = { targetOrigin: '*' };
         var opts = Object.create(proto);
         window.postMessage(7, opts);",
    )
    .unwrap();
    assert_eq!(eval_number(&mut vm, "globalThis.hit;"), 7.0);
    vm.unbind();
}

#[test]
fn post_message_dict_target_origin_via_getter_is_observed() {
    setup_bound_vm!(vm, session, dom, doc);
    // `targetOrigin` is an accessor — the getter must fire, and its
    // side effect (globalThis.gets incremented) must be observable.
    vm.eval(
        "globalThis.gets = 0;
         globalThis.hit = 0;
         window.addEventListener('message', function(e){ globalThis.hit = e.data; });
         var opts = Object.defineProperty({}, 'targetOrigin', {
             get: function(){ globalThis.gets += 1; return '*'; }
         });
         window.postMessage(9, opts);",
    )
    .unwrap();
    assert_eq!(eval_number(&mut vm, "globalThis.gets;"), 1.0);
    assert_eq!(eval_number(&mut vm, "globalThis.hit;"), 9.0);
    vm.unbind();
}

#[test]
fn signal_abort_during_listener_body_removes_remaining() {
    // Aborting the signal mid-dispatch MUST remove the still-pending
    // listener paired with that signal (WHATWG DOM §2.10 step 15
    // + remove-listener side of §3 AbortSignal "removed listeners").
    // Here A and B are paired with the same signal; A calls
    // `ctl.abort()`, which must unregister B so it does not fire.
    setup_bound_vm!(vm, session, dom, doc);
    vm.eval(
        "globalThis.first = 0;
         globalThis.second = 0;
         var ctl = new AbortController();
         window.addEventListener('message', function(){
             globalThis.first = 1;
             ctl.abort();
         }, { signal: ctl.signal });
         window.addEventListener('message', function(){
             globalThis.second = 1;
         }, { signal: ctl.signal });
         window.postMessage(0, '*');",
    )
    .unwrap();
    assert_eq!(eval_number(&mut vm, "globalThis.first;"), 1.0);
    assert_eq!(eval_number(&mut vm, "globalThis.second;"), 0.0);
    vm.unbind();
}
