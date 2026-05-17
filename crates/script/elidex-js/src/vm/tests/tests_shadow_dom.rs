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
    // WHATWG DOM §4.2.2.5 — assignment fails engine validation and
    // is observable as an empty `assignedNodes()`.
    let out = run("var host = document.createElement('div'); \
         document.body.appendChild(host); \
         var child = document.createElement('span'); \
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
fn slotchange_signal_during_dispatch_defers_to_next_checkpoint() {
    // R1 finding #4: `signal_slots` is snapshotted before dispatch;
    // a `slot.assign()` from inside a slotchange listener body
    // queues for the next checkpoint, not re-entrantly in the same
    // dispatch.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    // First eval: install listener that signals slot2 the FIRST
    // time slot1's slotchange fires; both slots assigned initially.
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
    // After first eval: slot1 fires (recorded 's1'), then re-entered
    // dispatch attempts slot2 — but spec says snapshot-then-drain,
    // so slot2 signal lands on next checkpoint.  Force the next
    // checkpoint via a second eval boundary.
    let after_first = vm.eval("globalThis.fired.join(',')").unwrap();
    let JsValue::String(sid) = after_first else {
        panic!("expected string after first eval, got {after_first:?}");
    };
    let first_observed = vm.inner.strings.get_utf8(sid);
    let after_second = vm.eval("globalThis.fired.join(',')").unwrap();
    let JsValue::String(sid2) = after_second else {
        panic!("expected string after second eval, got {after_second:?}");
    };
    let second_observed = vm.inner.strings.get_utf8(sid2);
    vm.unbind();
    assert_eq!(
        first_observed, "s1",
        "first checkpoint should fire only the originally-signaled slot1"
    );
    assert_eq!(
        second_observed, "s1,s2",
        "next checkpoint should pick up slot2 signaled by listener"
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
