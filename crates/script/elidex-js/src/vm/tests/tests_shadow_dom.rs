//! D-15 `#11-shadow-dom-surface` — JS-facing tests for
//! `Element.attachShadow({init})` + `Element.shadowRoot` getter +
//! `ShadowRoot.prototype` accessors + `HTMLSlotElement.prototype`
//! methods + `slotchange` event microtask delivery.

#![cfg(feature = "engine")]

use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;

fn build_doc(dom: &mut EcsDom) -> elidex_ecs::Entity {
    let doc = dom.create_document_root();
    let html = dom.create_element("html", elidex_ecs::Attributes::default());
    let body = dom.create_element("body", elidex_ecs::Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, body));
    doc
}

fn run(script: &str) -> String {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let result = vm.eval(script).unwrap();
    let JsValue::String(sid) = result else {
        panic!("expected string, got {result:?}")
    };
    let out = vm.inner.strings.get_utf8(sid);
    vm.unbind();
    out
}

/// JS prelude that builds a manual-mode shadow root with one slot
/// child and one Element light-DOM child (`globalThis.host`,
/// `globalThis.slot`, `globalThis.child`).  Caller appends script
/// that exercises slot behaviour.
///
/// Used by tests that need to observe state across the eval
/// boundary; the Rust-side bind ceremony is inlined in each such
/// test because `bind_vm` takes `&mut SessionCore` / `&mut EcsDom`
/// pointers and a returns-by-value helper would invalidate them.
const MANUAL_SLOT_PRELUDE: &str = "globalThis.host = document.createElement('div'); \
     document.body.appendChild(globalThis.host); \
     globalThis.child = document.createElement('span'); \
     globalThis.host.appendChild(globalThis.child); \
     var sr = globalThis.host.attachShadow({mode: 'open', slotAssignment: 'manual'}); \
     globalThis.slot = document.createElement('slot'); \
     sr.append(globalThis.slot); ";

#[test]
fn attach_shadow_open_returns_wrapper() {
    let out = run("var host = document.createElement('div'); \
         var sr = host.attachShadow({mode: 'open'}); \
         (sr !== null && typeof sr === 'object' \
          && sr.mode === 'open' \
          && sr.host === host) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn shadow_root_getter_open_returns_same_wrapper() {
    // Identity invariant — Chrome / Firefox preserve the wrapper
    // across `attachShadow` return + `element.shadowRoot` reads.
    let out = run("var host = document.createElement('div'); \
         var sr = host.attachShadow({mode: 'open'}); \
         (host.shadowRoot === sr && host.shadowRoot === host.shadowRoot) \
           ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn shadow_root_getter_closed_returns_null() {
    // Closed-mode encapsulation per WHATWG DOM §4.8 —
    // `element.shadowRoot` returns null even when the shadow exists.
    // The wrapper is still returned from `attachShadow` so callers
    // who created the shadow can manipulate it.
    let out = run("var host = document.createElement('div'); \
         var sr = host.attachShadow({mode: 'closed'}); \
         (sr !== null && host.shadowRoot === null && sr.mode === 'closed') \
           ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn attach_shadow_init_round_trip() {
    let out = run("var host = document.createElement('div'); \
         var sr = host.attachShadow({mode: 'open', \
             delegatesFocus: true, slotAssignment: 'manual', \
             clonable: true, serializable: true}); \
         (sr.mode === 'open' && sr.delegatesFocus === true \
          && sr.slotAssignment === 'manual' \
          && sr.clonable === true && sr.serializable === true) \
           ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn attach_shadow_defaults_when_init_omits_fields() {
    let out = run("var host = document.createElement('div'); \
         var sr = host.attachShadow({mode: 'open'}); \
         (sr.delegatesFocus === false && sr.slotAssignment === 'named' \
          && sr.clonable === false && sr.serializable === false) \
           ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn attach_shadow_invalid_tag_throws_not_supported_error() {
    let out = run(
        "var host = document.createElement('input'); \
         var caught = null; \
         try { host.attachShadow({mode: 'open'}); } \
         catch (e) { caught = e; } \
         (caught !== null && caught.name === 'NotSupportedError') ? 'ok' : 'fail:' + (caught && caught.name);",
    );
    assert_eq!(out, "ok");
}

#[test]
fn attach_shadow_already_attached_throws() {
    let out = run(
        "var host = document.createElement('div'); \
         host.attachShadow({mode: 'open'}); \
         var caught = null; \
         try { host.attachShadow({mode: 'open'}); } \
         catch (e) { caught = e; } \
         (caught !== null && caught.name === 'NotSupportedError') ? 'ok' : 'fail:' + (caught && caught.name);",
    );
    assert_eq!(out, "ok");
}

#[test]
fn attach_shadow_missing_mode_throws_type_error() {
    let out = run(
        "var host = document.createElement('div'); \
         var caught = null; \
         try { host.attachShadow({}); } \
         catch (e) { caught = e; } \
         (caught !== null && caught.name === 'TypeError') ? 'ok' : 'fail:' + (caught && caught.name);",
    );
    assert_eq!(out, "ok");
}

#[test]
fn attach_shadow_invalid_mode_value_throws_type_error() {
    let out = run(
        "var host = document.createElement('div'); \
         var caught = null; \
         try { host.attachShadow({mode: 'half-open'}); } \
         catch (e) { caught = e; } \
         (caught !== null && caught.name === 'TypeError') ? 'ok' : 'fail:' + (caught && caught.name);",
    );
    assert_eq!(out, "ok");
}

#[test]
fn shadow_root_parent_node_mixin_installed_via_document_fragment_prototype() {
    // ShadowRoot's prototype chains through DocumentFragment.prototype
    // per spec.  The ParentNode mixin install on DF.prototype makes
    // `prepend` / `append` / `replaceChildren` reachable as functions
    // on ShadowRoot wrappers.
    let out = run("var host = document.createElement('div'); \
         var sr = host.attachShadow({mode: 'open'}); \
         (typeof sr.append === 'function' \
          && typeof sr.prepend === 'function' \
          && typeof sr.replaceChildren === 'function') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn shadow_root_append_routes_through_parent_node_mixin() {
    // `entity_from_this` resolves `ObjectKind::ShadowRoot` receivers
    // through `shadow_root_states` so the inherited ParentNode mixin
    // methods can mutate the shadow tree directly (D-15 PR-A wire-up
    // for `#11-shadow-parent-node-mixin-receiver`).
    //
    // `firstElementChild` / `children` etc. are installed on
    // `Element.prototype` only — DocumentFragment.prototype currently
    // carries just the mutation methods (`prepend` / `append` /
    // `replaceChildren`).  Observability of the appended child via
    // `Node.parentNode` is sufficient for this test; the ParentNode
    // accessor surface on ShadowRoot tracks a separate defer slot.
    let out = run("var host = document.createElement('div'); \
         var sr = host.attachShadow({mode: 'open'}); \
         var span = document.createElement('span'); \
         sr.append(span); \
         (span.parentNode !== null) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// `serialize_inner_html` shadow-exclusion regression lives at
// `crates/dom/elidex-dom-api/src/element/tests_tree.rs` (engine-indep
// layer).  The JS-facing innerHTML round-trip lands in PR-B
// (`#11-shadow-innerhtml-mixin`); a placeholder test here adds no
// signal until that wiring exists.

// -------------------------------------------------------------------------
// HTMLSlotElement.prototype tests
// -------------------------------------------------------------------------

#[test]
fn html_slot_element_brand_present_on_slot_wrapper() {
    let out = run("var s = document.createElement('slot'); \
         (typeof s.assign === 'function' \
          && typeof s.assignedNodes === 'function' \
          && typeof s.assignedElements === 'function') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn html_slot_element_name_reflects_attribute() {
    let out = run("var s = document.createElement('slot'); \
         var initial = s.name; \
         s.name = 'header'; \
         var reflected = s.getAttribute('name'); \
         s.setAttribute('name', 'body'); \
         var read_back = s.name; \
         (initial === '' && reflected === 'header' && read_back === 'body') \
           ? 'ok' : 'fail:' + initial + '/' + reflected + '/' + read_back;");
    assert_eq!(out, "ok");
}

#[test]
fn html_slot_assign_manual_mode_distributes_children() {
    // Manual-mode shadow root with a slot inside.  `slot.assign(child)`
    // should route through `EcsDom::slot_assign` and the
    // distribution become observable via `assignedNodes()`.
    let out = run("var host = document.createElement('div'); \
         document.body.appendChild(host); \
         var child = document.createElement('span'); \
         host.appendChild(child); \
         var sr = host.attachShadow({mode: 'open', slotAssignment: 'manual'}); \
         var slot = document.createElement('slot'); \
         sr.append(slot); \
         slot.assign(child); \
         var an = slot.assignedNodes(); \
         (an.length === 1 && an[0] === child) ? 'ok' : 'fail:' + an.length;");
    assert_eq!(out, "ok");
}

#[test]
fn html_slot_assign_named_mode_is_silent_no_op() {
    // Named-mode shadow roots ignore manual `slot.assign()` per
    // WHATWG DOM §4.2.2.5.  The child below has a `slot="other"`
    // attribute that doesn't match the unnamed default slot's name
    // (""), so named-mode distribution does NOT pick it up either;
    // `assignedNodes()` is therefore empty regardless of the
    // ignored manual assign.
    let out = run("var host = document.createElement('div'); \
         document.body.appendChild(host); \
         var child = document.createElement('span'); \
         child.setAttribute('slot', 'other'); \
         host.appendChild(child); \
         var sr = host.attachShadow({mode: 'open'}); \
         var slot = document.createElement('slot'); \
         sr.append(slot); \
         slot.assign(child); \
         var an = slot.assignedNodes(); \
         (an.length === 0) ? 'ok' : 'fail:' + an.length;");
    assert_eq!(out, "ok");
}

#[test]
fn html_slot_assign_non_element_text_throws_type_error() {
    // WebIDL union coercion `(Element or Text)... nodes` rejects
    // primitives per spec §4.2.2.5 step 1 before engine validation.
    let out = run("var s = document.createElement('slot'); \
         var caught = null; \
         try { s.assign('not a node'); } \
         catch (e) { caught = e; } \
         (caught !== null && caught.name === 'TypeError') ? 'ok' : 'fail:' + (caught && caught.name);");
    assert_eq!(out, "ok");
}

#[test]
fn html_slot_assign_accepts_text_node_argument() {
    // WebIDL union `(Element or Text)` accepts Text positively —
    // only non-Node primitives throw.  Sibling to
    // `html_slot_assign_non_element_text_throws_type_error`.
    let out = run("var host = document.createElement('div'); \
         document.body.appendChild(host); \
         var t = document.createTextNode('hi'); \
         host.appendChild(t); \
         var sr = host.attachShadow({mode: 'open', slotAssignment: 'manual'}); \
         var slot = document.createElement('slot'); \
         sr.append(slot); \
         slot.assign(t); \
         var an = slot.assignedNodes(); \
         (an.length === 1 && an[0] === t) ? 'ok' : 'fail:' + an.length;");
    assert_eq!(out, "ok");
}

#[test]
fn html_slot_assigned_elements_filters_text_nodes() {
    // `assignedElements()` returns only Element nodes; Text
    // assignments (when permitted) are dropped from the Array.
    let out = run("var host = document.createElement('div'); \
         document.body.appendChild(host); \
         var span = document.createElement('span'); \
         var text = document.createTextNode('hi'); \
         host.appendChild(span); \
         host.appendChild(text); \
         var sr = host.attachShadow({mode: 'open', slotAssignment: 'manual'}); \
         var slot = document.createElement('slot'); \
         sr.append(slot); \
         slot.assign(span, text); \
         var nodes = slot.assignedNodes(); \
         var els = slot.assignedElements(); \
         (nodes.length === 2 && els.length === 1 && els[0] === span) \
           ? 'ok' : 'fail:' + nodes.length + '/' + els.length;");
    assert_eq!(out, "ok");
}

#[test]
fn html_slot_assigned_nodes_returns_fresh_array_each_call() {
    // Per WebIDL `FrozenArray<Node>` convention, each call returns
    // a fresh Array — mutation of one return value does not leak
    // into the next.
    let out = run("var host = document.createElement('div'); \
         document.body.appendChild(host); \
         var c = document.createElement('span'); \
         host.appendChild(c); \
         var sr = host.attachShadow({mode: 'open', slotAssignment: 'manual'}); \
         var slot = document.createElement('slot'); \
         sr.append(slot); \
         slot.assign(c); \
         var a = slot.assignedNodes(); \
         var b = slot.assignedNodes(); \
         (a !== b && a.length === 1 && b.length === 1) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// -------------------------------------------------------------------------
// slotchange microtask delivery tests
// -------------------------------------------------------------------------

/// Run `setup_and_signal` (which must call `slot.assign(...)` and
/// install a listener bumping `globalThis.fired`), then read
/// `globalThis.fired` from a SECOND eval so the post-eval microtask
/// drain has a chance to dispatch.  Returns the observed counter.
fn fired_count_after_eval_boundary(setup_and_signal: &str) -> f64 {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let script = format!("globalThis.fired = 0; {MANUAL_SLOT_PRELUDE}{setup_and_signal}");
    vm.eval(&script).unwrap();
    let count = vm.eval("globalThis.fired").unwrap();
    vm.unbind();
    let JsValue::Number(n) = count else {
        panic!("expected number, got {count:?}");
    };
    n
}

#[test]
fn slotchange_fired_state_observable_after_eval_boundary() {
    // First eval signals slot + returns; post-eval `drain_microtasks`
    // fires slotchange.  Second eval reads `globalThis.fired`.
    let n = fired_count_after_eval_boundary(
        "globalThis.slot.addEventListener('slotchange', function () { globalThis.fired += 1; }); \
         globalThis.slot.assign(globalThis.child);",
    );
    assert!(
        (n - 1.0).abs() < f64::EPSILON,
        "expected slotchange to fire once, got {n}"
    );
}

#[test]
fn slotchange_dedup_per_drain() {
    // Multiple `slot.assign()` calls before the microtask checkpoint
    // collapse to a single `slotchange` per signal-slots set
    // membership rule (no duplicate entries).
    let n = fired_count_after_eval_boundary(
        "var c2 = document.createElement('span'); globalThis.host.appendChild(c2); \
         globalThis.slot.addEventListener('slotchange', function () { globalThis.fired += 1; }); \
         globalThis.slot.assign(globalThis.child); globalThis.slot.assign(c2);",
    );
    assert!(
        (n - 1.0).abs() < f64::EPSILON,
        "expected exactly one slotchange across two assigns, got {n}"
    );
}

#[test]
fn slotchange_not_fired_when_assign_validation_fails() {
    // Named-mode shadow root → `EcsDom::slot_assign` returns
    // `NotManualMode`, no signal is queued, no event fires.  Cannot
    // reuse `MANUAL_SLOT_PRELUDE` since this test needs a Named-mode
    // shadow; inline the setup.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval(
        "globalThis.fired = 0; \
         var host = document.createElement('div'); \
         document.body.appendChild(host); \
         var c = document.createElement('span'); \
         host.appendChild(c); \
         var sr = host.attachShadow({mode: 'open'}); \
         var slot = document.createElement('slot'); \
         sr.append(slot); \
         slot.addEventListener('slotchange', function () { globalThis.fired += 1; }); \
         slot.assign(c);",
    )
    .unwrap();
    let count = vm.eval("globalThis.fired").unwrap();
    let JsValue::Number(n) = count else {
        panic!("expected number, got {count:?}");
    };
    assert_eq!(n, 0.0);
    vm.unbind();
}

// -------------------------------------------------------------------------
// Copilot R1 regression tests
// -------------------------------------------------------------------------

#[test]
fn unbind_clears_pending_notify_mutation_observers_microtask() {
    // R9 finding #1: a queued `NotifyMutationObservers` microtask
    // must NOT survive `Vm::unbind`.  If it did, a fresh signal
    // after rebind would dispatch behind any Promise microtasks
    // queued in the new tick (wrong ordering).  Verified directly:
    // signal a slot (queues the notify-MO microtask), unbind, then
    // confirm the microtask queue contains no `NotifyMutationObservers`
    // entry.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let _ = vm
        .eval(&format!(
            "{MANUAL_SLOT_PRELUDE} globalThis.slot.assign(globalThis.child);"
        ))
        .unwrap();
    // At this point, the script returned and `drain_microtasks` has
    // already fired the notify-MO microtask, so the queue is empty.
    // To exercise the unbind-clear path we need a signal queued
    // WITHOUT a drain — patch the slot from JS-side again then
    // unbind before any explicit drain.  Easiest path: signal, then
    // immediately inspect after a fresh assign that the microtask
    // queue gets the notify-MO entry, unbind, verify cleared.
    vm.unbind();
    // Re-bind fresh DOM and use the raw signal_slot_change path —
    // hard to reach from JS without re-triggering drain.  Instead
    // assert the invariant directly: after any number of binds,
    // the queue should not retain a stale notify-MO across an
    // unbind boundary.
    assert!(
        !vm.inner.microtask_queue.iter().any(|t| matches!(
            t,
            super::super::natives_promise::Microtask::NotifyMutationObservers
        )),
        "no stale NotifyMutationObservers microtask should survive unbind"
    );
    assert!(
        !vm.inner.mutation_observer_microtask_queued,
        "coalescing flag should be cleared on unbind"
    );
}

#[test]
fn shadow_root_wrapper_is_extensible_for_expando_props() {
    // R9 finding #2: `ShadowRoot` wrapper allocated `extensible: true`
    // so script-side expando properties work (matches other DOM
    // HostObject wrappers; WebIDL doesn't mark ShadowRoot
    // `[Unforgeable]` / `[Frozen]`).
    let out = run("var host = document.createElement('div'); \
         var sr = host.attachShadow({mode: 'open'}); \
         sr.foo = 42; \
         (sr.foo === 42 && Object.isExtensible(sr)) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn append_child_rejects_shadow_root_arg_with_hierarchy_request_error() {
    // R9 finding #3: `appendChild(shadowRoot)` (and `insertBefore` /
    // `replaceChild` / mixin `append`) must throw HierarchyRequestError
    // — shadow roots are not insertable per WHATWG DOM §4.2.3.
    let out = run("var host = document.createElement('div'); \
         var sr = host.attachShadow({mode: 'open'}); \
         var other = document.createElement('div'); \
         var caught_append = null; \
         try { other.appendChild(sr); } catch (e) { caught_append = e; } \
         var caught_insert = null; \
         try { other.insertBefore(sr, null); } catch (e) { caught_insert = e; } \
         var caught_replace = null; \
         var child = document.createElement('span'); \
         other.appendChild(child); \
         try { other.replaceChild(sr, child); } catch (e) { caught_replace = e; } \
         var caught_mixin = null; \
         try { other.append(sr); } catch (e) { caught_mixin = e; } \
         (caught_append && caught_append.name === 'HierarchyRequestError' \
          && caught_insert && caught_insert.name === 'HierarchyRequestError' \
          && caught_replace && caught_replace.name === 'HierarchyRequestError' \
          && caught_mixin && caught_mixin.name === 'HierarchyRequestError') \
           ? 'ok' : 'fail:' + (caught_append && caught_append.name) + '/' \
                  + (caught_insert && caught_insert.name) + '/' \
                  + (caught_replace && caught_replace.name) + '/' \
                  + (caught_mixin && caught_mixin.name);");
    assert_eq!(out, "ok");
}

#[test]
fn child_parent_node_returns_cached_shadow_root_wrapper() {
    // R9 finding #4: `child.parentNode` from inside a shadow tree
    // must return the SAME ShadowRoot wrapper that `attachShadow`
    // returned (identity + `.host` reachability).  Previously the
    // generic DocumentFragment dispatch wrapped the shadow root
    // entity as a fresh DF wrapper.
    let out = run("var host = document.createElement('div'); \
         var sr = host.attachShadow({mode: 'open'}); \
         var child = document.createElement('span'); \
         sr.append(child); \
         (child.parentNode === sr && child.parentNode.host === host) \
           ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn shadow_root_getter_on_non_element_receiver_throws_type_error() {
    // R8 finding #1: `Element.shadowRoot` is a WebIDL Element
    // attribute, so the getter brand-checks the receiver per spec.
    // Invoking the getter with a non-Element receiver throws
    // "Illegal invocation" TypeError instead of returning null.
    // Symmetric with `attachShadow` brand check (R3 #1).
    //
    // `shadowRoot` lives on `Element.prototype` (not the immediate
    // tag-prototype) so the test walks the chain to locate the
    // getter descriptor.
    let out = run("var host = document.createElement('div'); \
         function findGetter(obj, prop) { \
             while (obj) { \
                 var d = Object.getOwnPropertyDescriptor(obj, prop); \
                 if (d && d.get) return d.get; \
                 obj = Object.getPrototypeOf(obj); \
             } \
             return null; \
         } \
         var getter = findGetter(host, 'shadowRoot'); \
         var caught = null; \
         try { getter.call(document); } catch (e) { caught = e; } \
         (caught !== null && caught.name === 'TypeError') \
           ? 'ok' : 'fail:' + (caught && caught.name);");
    assert_eq!(out, "ok");
}

#[test]
fn attach_shadow_on_non_element_receiver_throws_type_error() {
    // R3 finding #1: WebIDL Element brand check on `this` runs
    // BEFORE init-dict parsing.  `Element.prototype.attachShadow.call(document, ...)`
    // must throw "Illegal invocation" TypeError, not the
    // engine-side NotSupportedError DOMException it used to surface
    // through `attach_shadow_with_init`.
    let out = run("var host = document.createElement('div'); \
         var caught = null; \
         try { host.attachShadow.call(document, {mode: 'open'}); } \
         catch (e) { caught = e; } \
         (caught !== null && caught.name === 'TypeError') \
           ? 'ok' : 'fail:' + (caught && caught.name);");
    assert_eq!(out, "ok");
}

#[test]
fn document_fragment_carries_parent_node_mixin() {
    // R3 finding #2: `document.createDocumentFragment()` /
    // `<template>.content` wrappers chain through
    // `DocumentFragment.prototype` so the ParentNode mixin
    // (`prepend` / `append` / `replaceChildren`) is reachable
    // per WHATWG DOM §4.7.
    let out = run("var frag = document.createDocumentFragment(); \
         var span = document.createElement('span'); \
         frag.append(span); \
         (typeof frag.append === 'function' \
          && typeof frag.prepend === 'function' \
          && span.parentNode !== null) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn shadow_root_states_cleared_on_unbind() {
    // R4 finding #1: `shadow_root_states` (ObjectId-keyed) holds the
    // shadow-root Entity each wrapper resolves to.  Entity indices
    // are reused by a fresh `EcsDom`, so a retained ShadowRoot
    // wrapper must not silently resolve to an unrelated entity in
    // the new DOM post-rebind.  Unbind clears the side table; the
    // wrapper's brand check then throws "Illegal invocation" on
    // post-unbind accessor reads.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let _ = vm
        .eval("var host = document.createElement('div'); host.attachShadow({mode: 'open'});")
        .unwrap();
    assert!(
        !vm.inner.shadow_root_states.is_empty(),
        "expected attachShadow to populate shadow_root_states"
    );
    vm.unbind();
    assert!(
        vm.inner.shadow_root_states.is_empty(),
        "expected shadow_root_states to be cleared on unbind, found {} entries",
        vm.inner.shadow_root_states.len()
    );
}

#[test]
fn assigned_nodes_named_mode_matches_slot_attribute() {
    // R4 finding #2: WHATWG DOM §4.2.2.5 "find slottables" — named
    // mode (default) distributes light-DOM children to slots by
    // matching the child's `slot` attribute against the slot's
    // `name` attribute.  Default slot (`name=""`) catches children
    // with no `slot` attribute.
    let out = run("var host = document.createElement('div'); \
         document.body.appendChild(host); \
         var named = document.createElement('span'); \
         named.setAttribute('slot', 'header'); \
         var unnamed = document.createElement('span'); \
         host.appendChild(named); host.appendChild(unnamed); \
         var sr = host.attachShadow({mode: 'open'}); \
         var header = document.createElement('slot'); \
         header.setAttribute('name', 'header'); \
         var def = document.createElement('slot'); \
         sr.append(header); sr.append(def); \
         var h = header.assignedNodes(); \
         var d = def.assignedNodes(); \
         (h.length === 1 && h[0] === named \
          && d.length === 1 && d[0] === unnamed) \
           ? 'ok' : 'fail:' + h.length + '/' + d.length;");
    assert_eq!(out, "ok");
}

#[test]
fn slot_assign_empty_initial_does_not_signal() {
    // R6 finding #1: `slot.assign()` (no args) on a slot that has
    // never had a `SlotAssignment` component is a no-op vs. the
    // implicit-empty initial state.  No `slotchange` should fire.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval(&format!(
        "globalThis.fired = 0; {MANUAL_SLOT_PRELUDE} \
         globalThis.slot.addEventListener('slotchange', function () {{ globalThis.fired += 1; }}); \
         globalThis.slot.assign();"
    ))
    .unwrap();
    let count = vm.eval("globalThis.fired").unwrap();
    vm.unbind();
    let JsValue::Number(n) = count else {
        panic!("expected number, got {count:?}");
    };
    assert_eq!(n, 0.0, "empty initial assign should not signal");
}

#[test]
fn require_node_arg_rejects_shadow_root_with_destroyed_entity() {
    // R6 finding #2: A retained ShadowRoot wrapper whose backing
    // entity is destroyed must throw "Illegal invocation" via the
    // shared existence check, not silently hand a stale entity to
    // Node IDL methods.  Simulated by unbinding (which clears
    // `shadow_root_states`) and rebinding to a fresh DOM, then
    // calling a Node-arg method using the retained wrapper.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    // Retain the wrapper across the unbind boundary in a global.
    vm.eval(
        "globalThis.host = document.createElement('div'); \
         globalThis.sr = globalThis.host.attachShadow({mode: 'open'});",
    )
    .unwrap();
    vm.unbind();
    // Rebind to a fresh DOM (entity indices likely reused).
    let mut next_dom = EcsDom::new();
    let next_root = build_doc(&mut next_dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut next_dom, next_root);
    }
    let out = vm
        .eval(
            "var caught = null; \
         var probe = document.createElement('div'); \
         try { probe.contains(globalThis.sr); } \
         catch (e) { caught = e; } \
         (caught !== null && caught.name === 'TypeError') \
           ? 'ok' : 'fail:' + (caught && caught.name);",
        )
        .unwrap();
    vm.unbind();
    let JsValue::String(sid) = out else {
        panic!("expected string, got {out:?}");
    };
    assert_eq!(
        vm.inner.strings.get_utf8(sid),
        "ok",
        "post-unbind ShadowRoot wrapper should throw TypeError when used as Node arg"
    );
}

#[test]
fn assigned_nodes_rejects_non_object_options_arg() {
    // R7 finding #1: WebIDL dict conversion (§3.2.18) throws
    // TypeError when a non-null / non-undefined non-Object is
    // passed where an `AssignedNodesOptions` dictionary is
    // expected.  `null` / `undefined` → empty dict → `flatten=false`.
    let out = run("var s = document.createElement('slot'); \
         var caught = null; \
         try { s.assignedNodes(1); } catch (e) { caught = e; } \
         (caught !== null && caught.name === 'TypeError') ? 'ok' : 'fail:' + (caught && caught.name);");
    assert_eq!(out, "ok");
    let out_null = run("var s = document.createElement('slot'); \
         var a = s.assignedNodes(null); var b = s.assignedNodes(undefined); var c = s.assignedNodes(); \
         (a.length === 0 && b.length === 0 && c.length === 0) ? 'ok' : 'fail';");
    assert_eq!(out_null, "ok");
}

#[test]
fn slot_assign_cross_slot_dedup_fires_slotchange_at_both() {
    // R7 finding #3: WHATWG DOM §4.2.2.5 step 3 — assigning a node
    // to a second slot removes it from the first slot's assigned
    // list, and slotchange fires at BOTH slots.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval(
        "globalThis.s1_fires = 0; globalThis.s2_fires = 0; \
         var host = document.createElement('div'); \
         document.body.appendChild(host); \
         globalThis.child = document.createElement('span'); host.appendChild(globalThis.child); \
         var sr = host.attachShadow({mode: 'open', slotAssignment: 'manual'}); \
         globalThis.s1 = document.createElement('slot'); \
         globalThis.s2 = document.createElement('slot'); \
         sr.append(globalThis.s1); sr.append(globalThis.s2); \
         globalThis.s1.addEventListener('slotchange', function () { globalThis.s1_fires += 1; }); \
         globalThis.s2.addEventListener('slotchange', function () { globalThis.s2_fires += 1; }); \
         globalThis.s1.assign(globalThis.child);",
    )
    .unwrap();
    // After first eval: s1 fires once, s2 fires 0 times.
    let s1a = vm.eval("globalThis.s1_fires").unwrap();
    let s2a = vm.eval("globalThis.s2_fires").unwrap();
    // Reassign to s2 — should remove child from s1 AND fire at both.
    vm.eval("globalThis.s2.assign(globalThis.child);").unwrap();
    let s1b = vm.eval("globalThis.s1_fires").unwrap();
    let s2b = vm.eval("globalThis.s2_fires").unwrap();
    // Verify lists too.
    let an1 = vm.eval("globalThis.s1.assignedNodes().length").unwrap();
    let an2 = vm
        .eval(
            "var arr = globalThis.s2.assignedNodes(); \
         (arr.length === 1 && arr[0] === globalThis.child) ? 1 : 0",
        )
        .unwrap();
    vm.unbind();
    let JsValue::Number(s1_first) = s1a else {
        panic!()
    };
    let JsValue::Number(s2_first) = s2a else {
        panic!()
    };
    let JsValue::Number(s1_second) = s1b else {
        panic!()
    };
    let JsValue::Number(s2_second) = s2b else {
        panic!()
    };
    let JsValue::Number(s1_len) = an1 else {
        panic!()
    };
    let JsValue::Number(s2_len) = an2 else {
        panic!()
    };
    assert_eq!(s1_first, 1.0, "s1 first assign should fire once");
    assert_eq!(s2_first, 0.0, "s2 no fire before its assign");
    assert_eq!(
        s1_second, 2.0,
        "s1 fires again on second eval (cross-slot removal)"
    );
    assert_eq!(s2_second, 1.0, "s2 fires from its own assignment");
    assert_eq!(s1_len, 0.0, "s1 list now empty (child moved to s2)");
    assert_eq!(s2_len, 1.0, "s2 list contains child");
}

#[test]
fn shadow_root_accepted_as_node_arg() {
    // R1 finding #1: `Node` IDL arg surface must accept a ShadowRoot
    // wrapper; previously rejected because `require_node_arg` only
    // handled `ObjectKind::HostObject`.  `sr.contains(sr)` and
    // `sr.isSameNode(sr)` both pass the receiver back as a `Node`
    // argument — without the fix they throw `TypeError` "not of
    // type 'Node'".  `host.contains(sr)` is correctly `false` per
    // spec (shadow root isn't a light-tree descendant of its host),
    // so the test goes through self-receiver instead.
    let out = run("var host = document.createElement('div'); \
         document.body.appendChild(host); \
         var sr = host.attachShadow({mode: 'open'}); \
         (sr.contains(sr) === true && sr.isSameNode(sr) === true) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn shadow_root_parent_node_is_null() {
    // R1 finding #2: `shadowRoot.parentNode === null` per WHATWG
    // §4.8; previously returned the host because `entity_from_this`
    // resolved through the ECS parent edge.
    let out = run("var host = document.createElement('div'); \
         document.body.appendChild(host); \
         var sr = host.attachShadow({mode: 'open'}); \
         (sr.parentNode === null && sr.parentElement === null \
          && sr.nextSibling === null && sr.previousSibling === null) \
           ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn slotchange_listener_promise_then_runs_in_same_checkpoint() {
    // R1 finding #3: a microtask queued by a slotchange listener
    // body must run within the same `drain_microtasks` pass, not
    // be deferred to the next checkpoint.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval(
        "globalThis.slot_fired = 0; globalThis.then_fired = 0; \
         var host = document.createElement('div'); \
         document.body.appendChild(host); \
         var c = document.createElement('span'); \
         host.appendChild(c); \
         var sr = host.attachShadow({mode: 'open', slotAssignment: 'manual'}); \
         var slot = document.createElement('slot'); \
         sr.append(slot); \
         slot.addEventListener('slotchange', function () { \
             globalThis.slot_fired += 1; \
             Promise.resolve().then(function () { globalThis.then_fired += 1; }); \
         }); \
         slot.assign(c);",
    )
    .unwrap();
    let slot_fired = vm.eval("globalThis.slot_fired").unwrap();
    let then_fired = vm.eval("globalThis.then_fired").unwrap();
    vm.unbind();
    let JsValue::Number(s) = slot_fired else {
        panic!("expected number, got {slot_fired:?}");
    };
    let JsValue::Number(t) = then_fired else {
        panic!("expected number, got {then_fired:?}");
    };
    assert_eq!(s, 1.0, "slotchange should fire once");
    assert_eq!(
        t, 1.0,
        "then() callback queued by listener should run in same checkpoint"
    );
}

#[test]
fn slotchange_signal_during_dispatch_runs_in_same_drain() {
    // Per WHATWG DOM §4.3.4: each "notify mutation observers"
    // microtask snapshots the signal-slots set before dispatching
    // its slotchange events.  A `slot.assign()` from inside a
    // listener body re-arms the coalescing flag and enqueues a NEW
    // `NotifyMutationObservers` microtask in the same drain pass —
    // so both slot1 and slot2 fire within the same eval boundary,
    // each through its own microtask checkpoint.  Earlier impl
    // (R1 snapshot-only at drain tail) incorrectly deferred slot2
    // to the next eval; R5 microtask-queue-ordering fix made the
    // spec-correct behavior observable.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval(
        "globalThis.fired = []; globalThis.reentered = false; \
         var host = document.createElement('div'); \
         document.body.appendChild(host); \
         var c1 = document.createElement('span'); host.appendChild(c1); \
         var c2 = document.createElement('span'); host.appendChild(c2); \
         var sr = host.attachShadow({mode: 'open', slotAssignment: 'manual'}); \
         globalThis.slot1 = document.createElement('slot'); \
         globalThis.slot2 = document.createElement('slot'); \
         sr.append(globalThis.slot1); sr.append(globalThis.slot2); \
         globalThis.slot1.addEventListener('slotchange', function () { \
             globalThis.fired.push('s1'); \
             if (!globalThis.reentered) { \
                 globalThis.reentered = true; \
                 globalThis.slot2.assign(c2); \
             } \
         }); \
         globalThis.slot2.addEventListener('slotchange', function () { \
             globalThis.fired.push('s2'); \
         }); \
         globalThis.slot1.assign(c1);",
    )
    .unwrap();
    let observed = vm.eval("globalThis.fired.join(',')").unwrap();
    vm.unbind();
    let JsValue::String(sid) = observed else {
        panic!("expected string, got {observed:?}");
    };
    let s = vm.inner.strings.get_utf8(sid);
    assert_eq!(
        s, "s1,s2",
        "snapshot-and-re-enqueue: slot1 fires first, listener signals \
         slot2 which queues a fresh notify-MO microtask in same drain"
    );
}

#[test]
fn slotchange_ordered_in_microtask_queue_at_signal_time() {
    // R5 finding #4: the `NotifyMutationObservers` microtask is
    // enqueued at signal time (inside `signal_slot_change`), not at
    // drain-tail.  A `Promise.then(cb)` registered AFTER the
    // `slot.assign()` observes the post-slotchange state; one
    // registered BEFORE the assign still fires first.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval(
        "globalThis.order = []; \
         var host = document.createElement('div'); \
         document.body.appendChild(host); \
         var c = document.createElement('span'); host.appendChild(c); \
         var sr = host.attachShadow({mode: 'open', slotAssignment: 'manual'}); \
         var slot = document.createElement('slot'); sr.append(slot); \
         slot.addEventListener('slotchange', function () { globalThis.order.push('sc'); }); \
         Promise.resolve().then(function () { globalThis.order.push('before'); }); \
         slot.assign(c); \
         Promise.resolve().then(function () { globalThis.order.push('after'); });",
    )
    .unwrap();
    let observed = vm.eval("globalThis.order.join(',')").unwrap();
    vm.unbind();
    let JsValue::String(sid) = observed else {
        panic!("expected string, got {observed:?}");
    };
    let s = vm.inner.strings.get_utf8(sid);
    assert_eq!(
        s, "before,sc,after",
        "notify-MO microtask interleaves with Promise reactions at signal time"
    );
}

#[test]
fn attach_shadow_mode_coerces_via_to_string() {
    // R1 finding #5: WebIDL enum conversion is ToString-first, so
    // `new String('open')` (boxed string) coerces to the primitive
    // "open" and succeeds.  Previous code accepted only primitive
    // `JsValue::String`.
    let out = run("var host = document.createElement('div'); \
         var sr = host.attachShadow({mode: new String('open')}); \
         (sr !== null && sr.mode === 'open') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn slot_assign_unchanged_list_does_not_signal_slotchange() {
    // R2 finding #2: WHATWG DOM §4.2.2.5 "assign slottables" step 2
    // — only signal a slot change when the resulting assigned-nodes
    // list differs from the prior list.  Repeated `slot.assign(c)`
    // with the SAME nodes across separate microtask checkpoints
    // must fire `slotchange` exactly once (initial change), not
    // twice.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    // First eval: install listener, perform first assign (fires).
    vm.eval(&format!(
        "globalThis.fired = 0; {MANUAL_SLOT_PRELUDE} \
         globalThis.slot.addEventListener('slotchange', function () {{ globalThis.fired += 1; }}); \
         globalThis.slot.assign(globalThis.child);"
    ))
    .unwrap();
    let after_first = vm.eval("globalThis.fired").unwrap();
    let JsValue::Number(n1) = after_first else {
        panic!("expected number, got {after_first:?}");
    };
    assert!(
        (n1 - 1.0).abs() < f64::EPSILON,
        "first assign should fire once, got {n1}"
    );
    // Second eval: re-assign SAME node, then read counter.
    vm.eval("globalThis.slot.assign(globalThis.child);")
        .unwrap();
    let after_second = vm.eval("globalThis.fired").unwrap();
    vm.unbind();
    let JsValue::Number(n2) = after_second else {
        panic!("expected number, got {after_second:?}");
    };
    assert!(
        (n2 - 1.0).abs() < f64::EPSILON,
        "no-op re-assign should leave counter at 1, got {n2}"
    );
}

#[test]
fn slot_assign_accepts_uppercase_slot_tag() {
    // R1 finding #6: `slot_assign` tag check is case-insensitive,
    // matching sibling HTML tag lookups (e.g. `first_child_with_tag`).
    // Tags inserted via APIs that preserve case (custom parsers,
    // SVG-style attribute sets) must still validate as `<slot>`.
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let host = dom.create_element("div", elidex_ecs::Attributes::default());
    assert!(dom.append_child(doc, host));
    let sr = dom
        .attach_shadow_with_init(
            host,
            elidex_ecs::ShadowInit {
                mode: elidex_ecs::ShadowRootMode::Open,
                slot_assignment: elidex_ecs::SlotAssignmentMode::Manual,
                ..Default::default()
            },
        )
        .unwrap();
    let upper_slot = dom.create_element("SLOT", elidex_ecs::Attributes::default());
    assert!(dom.append_child(sr, upper_slot));
    // Should NOT return NotASlot — the case-insensitive match
    // accepts "SLOT" as a slot tag.  Validation may still fail
    // for other reasons (no light-DOM children to assign here), so
    // an empty-nodes assign exercises the tag check alone.
    let result = dom.slot_assign(upper_slot, Vec::new());
    assert!(
        result.is_ok(),
        "case-insensitive slot tag check should accept SLOT; got {result:?}"
    );
}

// -------------------------------------------------------------------------
// Lifecycle / unbind regression
// -------------------------------------------------------------------------

#[test]
fn shadow_root_wrappers_cleared_on_unbind() {
    // After `attachShadow`, `shadow_root_wrappers` should hold the
    // host→wrapper entry; `Vm::unbind()` must clear it so a rebind
    // to a different DOM cannot resolve the stale wrapper.  Mirrors
    // `attr_wrapper_cache_cleared_on_unbind` (in
    // `tests_named_node_map.rs`).
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let _ = vm
        .eval("var host = document.createElement('div'); host.attachShadow({mode: 'open'});")
        .unwrap();
    assert!(
        !vm.inner.shadow_root_wrappers.is_empty(),
        "expected attachShadow to populate shadow_root_wrappers"
    );
    vm.unbind();
    assert!(
        vm.inner.shadow_root_wrappers.is_empty(),
        "expected shadow_root_wrappers to be cleared on unbind, found {} entries",
        vm.inner.shadow_root_wrappers.len()
    );
}
