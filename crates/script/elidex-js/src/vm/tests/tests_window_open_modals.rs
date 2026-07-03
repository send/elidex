//! S5-4c VM sandbox method gates — simple dialogs (WHATWG HTML §8.9.1) +
//! `window.open` (§7.2.2.1 window open steps).
//!
//! The modal oracle is the chokepoint's **return shape** (alert → undefined /
//! confirm → false / prompt → null on BOTH gate branches — the permanent
//! §8.9.1 step-4 opt-in makes the gate behaviorally invisible, memo E12) plus
//! WebIDL argument-conversion observability.  The `window.open` oracle is the
//! back-channel **queue structure**: which of `pending_navigation` /
//! `pending_open_tabs` / `pending_frame_navigations` a call reaches (or does
//! not reach — a blocked request never enters a queue) per
//! target × sandbox-flag row.

#![cfg(feature = "engine")]

use elidex_plugin::IframeSandboxFlags;
use elidex_script_session::{NamedFrameNavigation, NavigationRequest, OpenTabRequest};

use super::super::host_data::HostData;
use super::super::value::JsValue;
use super::super::Vm;

/// A VM with `HostData` installed (so sandbox flags can be configured — the
/// natives read them via `host_opt`, no DOM bind required) and a committed
/// tuple base URL so relative `window.open` inputs resolve like
/// `location.assign` inputs.
fn vm_with_flags(flags: Option<IframeSandboxFlags>) -> Vm {
    let mut vm = Vm::new();
    vm.install_host_data(HostData::new());
    vm.host_data()
        .expect("HostData was just installed")
        .set_sandbox_flags(flags);
    vm.inner
        .navigation
        .set_current_url(Some(url::Url::parse("https://example.com/").unwrap()));
    vm
}

fn eval_bool(vm: &mut Vm, src: &str) -> bool {
    match vm.eval(src).unwrap() {
        JsValue::Boolean(b) => b,
        other => panic!("expected boolean, got {other:?} (src: {src})"),
    }
}

fn eval_string(vm: &mut Vm, src: &str) -> String {
    match vm.eval(src).unwrap() {
        JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string, got {other:?} (src: {src})"),
    }
}

/// Drain the popup / `_blank` back-channel queue.
fn open_tabs(vm: &mut Vm) -> Vec<OpenTabRequest> {
    vm.inner.navigation.pending_open_tabs.drain(..).collect()
}

/// Drain the named-target back-channel queue.
fn frame_navs(vm: &mut Vm) -> Vec<NamedFrameNavigation> {
    vm.inner
        .navigation
        .pending_frame_navigations
        .drain(..)
        .collect()
}

/// Take the (single-slot) own-context navigation intent, if any.
fn take_nav(vm: &mut Vm) -> Option<NavigationRequest> {
    vm.inner.navigation.pending_navigation.take()
}

/// Assert every `window.open` back-channel is empty (the blocked-path
/// oracle: a gated-off request never enters ANY queue).
fn assert_no_intents(vm: &mut Vm) {
    assert!(take_nav(vm).is_none());
    assert!(open_tabs(vm).is_empty());
    assert!(frame_navs(vm).is_empty());
}

// ---------------------------------------------------------------------------
// Simple dialogs — the §8.9.1 return-shape triple on both gate branches
// ---------------------------------------------------------------------------

/// The spec's step-1 return triple: alert → undefined / confirm → false /
/// prompt → null (§8.9.1 method steps).  Asserted as one combined check so
/// every flag row pins all three shapes.
fn assert_modal_return_triple(vm: &mut Vm) {
    assert!(eval_bool(
        vm,
        "window.alert('m') === undefined \
         && window.confirm('m') === false \
         && window.prompt('m', 'd') === null;",
    ));
}

#[test]
fn modals_return_triple_unsandboxed() {
    let mut vm = vm_with_flags(None);
    assert_modal_return_triple(&mut vm);
    // Also reachable as bare globals (Window methods on the prototype).
    assert!(eval_bool(
        &mut vm,
        "alert() === undefined && confirm() === false && prompt() === null;",
    ));
}

#[test]
fn modals_return_triple_sandboxed_no_allow_modals() {
    // Some(empty) = maximum restriction → the sandboxed modals flag is set
    // (§8.9.1 cannot-show step 1 fires).  The observable shape is IDENTICAL
    // to the unsandboxed row — no throw, same returns (E12: the oracle is
    // the return shape, not a UI diff).
    let mut vm = vm_with_flags(Some(IframeSandboxFlags::empty()));
    assert_modal_return_triple(&mut vm);
}

#[test]
fn modals_return_triple_with_allow_modals() {
    // `allow-modals` clears step 1; the permanent step-4 opt-in still keeps
    // the triple identical (presentation never happens).
    let mut vm = vm_with_flags(Some(IframeSandboxFlags::ALLOW_MODALS));
    assert_modal_return_triple(&mut vm);
}

#[test]
fn modal_arg_coercion_runs_even_when_sandboxed() {
    // WebIDL argument conversion precedes the method steps, so a passed
    // object's `toString` MUST run even when the modals gate would return
    // at step 1 — and `prompt` converts BOTH of its arguments.
    let mut vm = vm_with_flags(Some(IframeSandboxFlags::empty()));
    vm.eval(
        "globalThis.calls = ''; \
         function probe(tag) { \
             return { toString: function () { globalThis.calls += tag; return tag; } }; \
         } \
         window.alert(probe('a')); \
         window.confirm(probe('c')); \
         window.prompt(probe('p'), probe('q'));",
    )
    .unwrap();
    assert_eq!(eval_string(&mut vm, "globalThis.calls;"), "acpq");
}

// ---------------------------------------------------------------------------
// window.open — target × flag dispatch onto the back-channels
// ---------------------------------------------------------------------------

#[test]
fn open_blank_unsandboxed_queues_open_tab_and_returns_null() {
    let mut vm = vm_with_flags(None);
    // Returns null on the gate-passed path too (WindowProxy = S5-8).
    assert!(eval_bool(
        &mut vm,
        "window.open('https://other.example/p', '_blank') === null;",
    ));
    assert_eq!(
        open_tabs(&mut vm),
        vec![OpenTabRequest {
            url: "https://other.example/p".to_string()
        }]
    );
    // Only the open-tab channel was touched.
    assert!(take_nav(&mut vm).is_none());
    assert!(frame_navs(&mut vm).is_empty());
}

#[test]
fn open_blank_sandboxed_is_a_silent_null_and_never_enqueues() {
    // §7.3.1.7 step 8 sandboxed-auxiliary-navigation case: blocked popup =
    // silent null; the request never enters the queue (enqueue-time gating).
    let mut vm = vm_with_flags(Some(IframeSandboxFlags::empty()));
    assert!(eval_bool(
        &mut vm,
        "window.open('https://other.example/', '_blank') === null;",
    ));
    assert_no_intents(&mut vm);
}

#[test]
fn open_blank_allow_popups_queues() {
    let mut vm = vm_with_flags(Some(IframeSandboxFlags::ALLOW_POPUPS));
    vm.eval("window.open('https://other.example/', '_blank');")
        .unwrap();
    assert_eq!(open_tabs(&mut vm).len(), 1);
}

#[test]
fn open_top_sandboxed_blocked() {
    let mut vm = vm_with_flags(Some(IframeSandboxFlags::empty()));
    assert!(eval_bool(&mut vm, "window.open('/x', '_top') === null;"));
    assert_no_intents(&mut vm);
}

#[test]
fn open_top_allow_top_navigation_enqueues_navigation() {
    // Gate passed → the own-context NavigationRequest channel (single-
    // navigable model routing), with the RESOLVED absolute URL — the same
    // shape `location.assign` enqueues.
    let mut vm = vm_with_flags(Some(IframeSandboxFlags::ALLOW_TOP_NAVIGATION));
    assert!(eval_bool(&mut vm, "window.open('/x', '_top') === null;"));
    let nav = take_nav(&mut vm).expect("a navigation request was enqueued");
    assert_eq!(nav.url, "https://example.com/x");
    assert!(!nav.replace);
    assert!(open_tabs(&mut vm).is_empty());
}

#[test]
fn open_top_by_user_activation_only_is_blocked_for_script() {
    // `allow-top-navigation-by-user-activation` grants the WITH-activation
    // arm only (§7.4.2.4 step 3.2); a script-initiated `window.open` passes
    // the conservative no-activation constant (no user-activation tracking
    // yet — carve `#11-transient-activation-tracking`), so step 3.3 blocks.
    let mut vm = vm_with_flags(Some(
        IframeSandboxFlags::ALLOW_TOP_NAVIGATION_BY_USER_ACTIVATION,
    ));
    assert!(eval_bool(&mut vm, "window.open('/x', '_top') === null;"));
    assert_no_intents(&mut vm);
}

#[test]
fn open_self_is_never_popup_gated() {
    // §7.3.1.7 resolves `_self` to the current navigable before any flag
    // check — maximum restriction still navigates the own context.
    let mut vm = vm_with_flags(Some(IframeSandboxFlags::empty()));
    assert!(eval_bool(&mut vm, "window.open('/y', '_self') === null;"));
    let nav = take_nav(&mut vm).expect("a navigation request was enqueued");
    assert_eq!(nav.url, "https://example.com/y");
    assert!(!nav.replace);
}

#[test]
fn open_named_sandboxed_snapshots_negative_aux_verdict() {
    // A named target is never blocked at enqueue — the §7.3.1.7 step-3
    // flag-set snapshot rides the payload for the shell's MISS branch.
    let mut vm = vm_with_flags(Some(IframeSandboxFlags::empty()));
    assert!(eval_bool(&mut vm, "window.open('/f', 'frameA') === null;"));
    assert_eq!(
        frame_navs(&mut vm),
        vec![NamedFrameNavigation {
            name: "frameA".to_string(),
            url: "https://example.com/f".to_string(),
            aux_nav_allowed: false,
        }]
    );
    assert!(open_tabs(&mut vm).is_empty());
    assert!(take_nav(&mut vm).is_none());
}

#[test]
fn open_named_allow_popups_snapshots_positive_aux_verdict() {
    let mut vm = vm_with_flags(Some(IframeSandboxFlags::ALLOW_POPUPS));
    vm.eval("window.open('/f', 'frameA');").unwrap();
    let navs = frame_navs(&mut vm);
    assert_eq!(navs.len(), 1);
    assert!(navs[0].aux_nav_allowed);
}

#[test]
fn open_keyword_detection_is_ascii_case_insensitive() {
    // "_BLANK" is the `_blank` keyword (§7.3.1.7), not a frame name.
    let mut vm = vm_with_flags(None);
    vm.eval("window.open('https://other.example/', '_BLANK');")
        .unwrap();
    assert_eq!(open_tabs(&mut vm).len(), 1);
    assert!(frame_navs(&mut vm).is_empty());
}

#[test]
fn open_invalid_url_throws_syntax_error_dom_exception() {
    // §7.2.2.1 step 4.2: "If urlRecord is failure, then throw a
    // \"SyntaxError\" DOMException" — thrown at the boundary, BEFORE any
    // dispatch/enqueue (nothing reaches a queue).
    let mut vm = vm_with_flags(None);
    let check = vm
        .eval(
            "var thrown = null;\
             try { window.open('http://[invalid', '_blank'); } \
             catch (e) { thrown = e; }\
             thrown && thrown.name === 'SyntaxError' \
             && thrown instanceof DOMException;",
        )
        .unwrap();
    assert!(matches!(check, JsValue::Boolean(true)));
    assert_no_intents(&mut vm);
}

#[test]
fn open_no_args_opens_about_blank_tab() {
    // WebIDL defaults: url = "" (→ about:blank, §7.2.2.1 step 15.3),
    // target = "_blank".
    let mut vm = vm_with_flags(None);
    assert!(eval_bool(&mut vm, "window.open() === null;"));
    assert_eq!(
        open_tabs(&mut vm),
        vec![OpenTabRequest {
            url: "about:blank".to_string()
        }]
    );
}

#[test]
fn open_empty_target_string_is_blank() {
    // §7.2.2.1 step 5: an empty target is `_blank` — popup-gated.
    let mut vm = vm_with_flags(Some(IframeSandboxFlags::empty()));
    assert!(eval_bool(&mut vm, "window.open('/z', '') === null;"));
    assert_no_intents(&mut vm);
    let mut vm = vm_with_flags(Some(IframeSandboxFlags::ALLOW_POPUPS));
    vm.eval("window.open('/z', '');").unwrap();
    assert_eq!(open_tabs(&mut vm).len(), 1);
}

#[test]
fn open_multiple_calls_preserve_fifo_order() {
    // Several `window.open` calls in one turn must ALL surface, in call
    // order (the FIFO queue contract — a last-wins slot would drop work).
    let mut vm = vm_with_flags(None);
    vm.eval(
        "window.open('https://a.example/', '_blank'); \
         window.open('https://b.example/', '_blank'); \
         window.open('/c', 'frameA'); \
         window.open('/d', 'frameB');",
    )
    .unwrap();
    let tabs = open_tabs(&mut vm);
    assert_eq!(tabs[0].url, "https://a.example/");
    assert_eq!(tabs[1].url, "https://b.example/");
    let navs = frame_navs(&mut vm);
    assert_eq!(navs[0].name, "frameA");
    assert_eq!(navs[1].name, "frameB");
}

#[test]
fn open_features_string_is_converted_then_ignored() {
    // `features` is WebIDL-converted (side effects observable) then ignored
    // (tokenization = S5-8); junk and `null` (`[LegacyNullToEmptyString]`)
    // both leave the call functional.
    let mut vm = vm_with_flags(None);
    vm.eval(
        "globalThis.fRan = false; \
         window.open('https://a.example/', '_blank', \
                     { toString: function () { globalThis.fRan = true; return 'x=1,junk'; } }); \
         window.open('https://b.example/', '_blank', null);",
    )
    .unwrap();
    assert!(eval_bool(&mut vm, "globalThis.fRan;"));
    assert_eq!(open_tabs(&mut vm).len(), 2);
}
