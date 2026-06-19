//! `window.matchMedia` + `MediaQueryList` interface tests (CSSOM-View §4 /
//! §4.2) — Slice 2b-i.
//!
//! Covers the static-snapshot MQL: `matchMedia` returns a live
//! `MediaQueryList`, `.matches` / `.media` reads, the EventTarget
//! integration (`addEventListener('change')` / `onchange` / legacy
//! `addListener` with `this === mql`), interface identity, and the
//! ObjectId-keyed side-table lifecycle (survives unbind, GC-pruned). The
//! host-driven report-changes fire is Slice 2b-ii — delivery is exercised
//! here via `dispatchEvent`.

#![cfg(feature = "engine")]

use super::super::value::JsValue;
use super::super::Vm;

/// A `Vm` with an (unbound) `HostData` installed — `MediaQueryList`'s
/// `change` listeners live in the unified `vm_event_listeners` home (no DOM
/// needed), exactly like `AbortSignal`. `matchMedia` itself reads only the
/// `VmInner::viewport` default (1024×768), so it works regardless.
fn new_vm() -> Vm {
    let mut v = Vm::new();
    v.install_host_data(super::super::host_data::HostData::new());
    v
}

fn eval_bool(vm: &mut Vm, source: &str) -> bool {
    match vm.eval(source).unwrap() {
        JsValue::Boolean(b) => b,
        other => panic!("expected bool, got {other:?}"),
    }
}

fn eval_string(vm: &mut Vm, source: &str) -> String {
    match vm.eval(source).unwrap() {
        JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string, got {other:?}"),
    }
}

// --- matchMedia + .matches / .media ----------------------------------------

#[test]
fn match_media_returns_object() {
    let mut vm = new_vm();
    assert!(eval_bool(
        &mut vm,
        "typeof matchMedia('(min-width: 1px)') === 'object';"
    ));
}

#[test]
fn matches_true_at_default_viewport() {
    // Default viewport = 1024×768; 1024 >= 500.
    let mut vm = new_vm();
    assert!(eval_bool(
        &mut vm,
        "matchMedia('(min-width: 500px)').matches;"
    ));
}

#[test]
fn matches_false_when_query_exceeds_viewport() {
    let mut vm = new_vm();
    assert!(!eval_bool(
        &mut vm,
        "matchMedia('(min-width: 2000px)').matches;"
    ));
}

#[test]
fn empty_query_matches_true() {
    // mediaqueries §2.1: an empty media query list evaluates to `true`.
    let mut vm = new_vm();
    assert!(eval_bool(&mut vm, "matchMedia('').matches;"));
}

#[test]
fn media_serializes_canonically() {
    // `.media` returns the serialized (canonical) query (#364).
    let mut vm = new_vm();
    assert_eq!(
        eval_string(&mut vm, "matchMedia('(min-width: 500px)').media;"),
        "(min-width: 500px)"
    );
}

#[test]
fn media_normalizes_case_and_whitespace() {
    let mut vm = new_vm();
    assert_eq!(
        eval_string(&mut vm, "matchMedia('(MIN-WIDTH:500PX)').media;"),
        "(min-width: 500px)"
    );
}

#[test]
fn boa_parity_min_max_width_height() {
    // Every query boa's string-splitter supported returns the same verdict
    // at 1024×768 (superset, no regression).
    let mut vm = new_vm();
    assert!(eval_bool(
        &mut vm,
        "matchMedia('(min-width: 1024px)').matches \
         && matchMedia('(max-width: 1024px)').matches \
         && matchMedia('(min-height: 768px)').matches \
         && matchMedia('(max-height: 768px)').matches \
         && !matchMedia('(min-width: 1025px)').matches \
         && !matchMedia('(max-width: 1023px)').matches;"
    ));
}

// --- interface identity ----------------------------------------------------

#[test]
fn instanceof_media_query_list() {
    let mut vm = new_vm();
    assert!(eval_bool(
        &mut vm,
        "matchMedia('(color)') instanceof MediaQueryList;"
    ));
}

#[test]
fn distinct_objects_per_call() {
    // CSSOM does not mandate identity across calls; boa parity = per-call.
    let mut vm = new_vm();
    assert!(eval_bool(
        &mut vm,
        "matchMedia('(color)') !== matchMedia('(color)');"
    ));
}

#[test]
fn new_media_query_list_throws() {
    // WebIDL: MediaQueryList has no constructor.
    let mut vm = new_vm();
    assert!(eval_bool(
        &mut vm,
        "var threw = false; try { new MediaQueryList(); } \
         catch (e) { threw = e instanceof TypeError; } threw;"
    ));
}

#[test]
fn matches_is_readonly() {
    // RO accessor (no setter) → strict-mode assignment throws (elidex is
    // strict-only).
    let mut vm = new_vm();
    assert!(eval_bool(
        &mut vm,
        "var m = matchMedia('(min-width: 1px)'); var threw = false; \
         try { m.matches = false; } catch (e) { threw = e instanceof TypeError; } \
         threw;"
    ));
}

#[test]
fn accessor_on_non_mql_this_throws() {
    let mut vm = new_vm();
    assert!(eval_bool(
        &mut vm,
        "var g = Object.getOwnPropertyDescriptor(MediaQueryList.prototype, 'matches').get; \
         var threw = false; try { g.call({}); } catch (e) { threw = e instanceof TypeError; } \
         threw;"
    ));
}

// --- EventTarget integration (this === mql; the boa fresh-`this` bug) ------

#[test]
fn add_event_listener_change_fires_with_mql_target_and_this() {
    let mut vm = new_vm();
    assert!(eval_bool(
        &mut vm,
        "var m = matchMedia('(min-width: 1px)'); var okThis = false, okTarget = false; \
         m.addEventListener('change', function (e) { okThis = (this === m); okTarget = (e.target === m); }); \
         m.dispatchEvent(new Event('change')); \
         okThis && okTarget;"
    ));
}

#[test]
fn onchange_fires_with_mql_this() {
    let mut vm = new_vm();
    assert!(eval_bool(
        &mut vm,
        "var m = matchMedia('(min-width: 1px)'); var okThis = false; \
         m.onchange = function () { okThis = (this === m); }; \
         m.dispatchEvent(new Event('change')); \
         okThis;"
    ));
}

#[test]
fn remove_event_listener_stops_delivery() {
    let mut vm = new_vm();
    assert!(eval_bool(
        &mut vm,
        "var m = matchMedia('(min-width: 1px)'); var n = 0; \
         function cb() { n++; } \
         m.addEventListener('change', cb); m.removeEventListener('change', cb); \
         m.dispatchEvent(new Event('change')); \
         n === 0;"
    ));
}

// --- legacy addListener / removeListener (CSSOM-View §4.2) -----------------

#[test]
fn legacy_add_listener_fires() {
    let mut vm = new_vm();
    assert!(eval_bool(
        &mut vm,
        "var m = matchMedia('(min-width: 1px)'); var n = 0; \
         m.addListener(function () { n++; }); \
         m.dispatchEvent(new Event('change')); \
         n === 1;"
    ));
}

#[test]
fn legacy_remove_listener_stops() {
    let mut vm = new_vm();
    assert!(eval_bool(
        &mut vm,
        "var m = matchMedia('(min-width: 1px)'); var n = 0; \
         function cb() { n++; } \
         m.addListener(cb); m.removeListener(cb); \
         m.dispatchEvent(new Event('change')); \
         n === 0;"
    ));
}

#[test]
fn legacy_add_listener_dedupes_like_add_event_listener() {
    // addListener(cb) twice = one registration (DOM §2.7 "add an event
    // listener" step 5).
    let mut vm = new_vm();
    assert!(eval_bool(
        &mut vm,
        "var m = matchMedia('(min-width: 1px)'); var n = 0; \
         function cb() { n++; } \
         m.addListener(cb); m.addListener(cb); \
         m.dispatchEvent(new Event('change')); \
         n === 1;"
    ));
}

#[test]
fn mql_accepted_as_event_related_target() {
    // MediaQueryList is a non-Node EventTarget, so it is a valid WebIDL
    // `EventTarget?` relatedTarget — exercises the unified
    // `ObjectKind::is_non_node_event_target` accept-list (the new brand must
    // be recognized by the relatedTarget coercion, not just listener routing).
    let mut vm = new_vm();
    assert!(eval_bool(
        &mut vm,
        "var m = matchMedia('(min-width: 1px)'); \
         var e = new MouseEvent('click', { relatedTarget: m }); \
         e.relatedTarget === m;"
    ));
}

// --- ObjectId-keyed side-table lifecycle (F2 survive-unbind / F3 GC) -------

#[test]
fn registry_survives_unbind() {
    // F2: the registry value is DOM-free, so a retained MQL survives unbind
    // (AbortSignal parity) — it is NOT in the unbind clear-set.
    let mut vm = new_vm();
    vm.eval("globalThis.m = matchMedia('(min-width: 1px)');")
        .unwrap();
    assert_eq!(vm.inner.media_query_list_registry.len(), 1);
    vm.unbind();
    assert_eq!(
        vm.inner.media_query_list_registry.len(),
        1,
        "MQL registry must survive unbind (DOM-free, AbortSignal parity)"
    );
}

#[test]
fn gc_prunes_dropped_mql() {
    // F3: dropping the only JS reference + a GC prunes the registry entry
    // (the sweep-prune is the sole delete-path; no trace root).
    let mut vm = new_vm();
    vm.eval("globalThis.m = matchMedia('(min-width: 1px)');")
        .unwrap();
    assert_eq!(vm.inner.media_query_list_registry.len(), 1);
    vm.eval("globalThis.m = null;").unwrap();
    vm.inner.collect_garbage();
    assert_eq!(
        vm.inner.media_query_list_registry.len(),
        0,
        "collected MQL must leave no stale registry entry"
    );
}
