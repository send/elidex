//! `Element.attachShadow` + `Element.shadowRoot` getter + ShadowRoot
//! accessor / brand-check / dict-parsing tests, plus
//! `HTMLSlotElement.prototype` accessors + manual / named-mode
//! `slot.assign()` distribution.  See [`super`] for shared helpers.

#![cfg(feature = "engine")]

use super::{run, FIND_GETTER_PRELUDE};

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
fn attach_shadow_document_registry_accepted() {
    // attachShadow steps 2-3: the document's global registry passes
    // (it IS this's node document's custom element registry).
    let out = run("var host = document.createElement('div'); \
         var sr = host.attachShadow({mode: 'open', \
             customElementRegistry: customElements}); \
         sr.mode === 'open' ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn attach_shadow_non_registry_value_throws_type_error() {
    // WebIDL conversion: `ShadowRootInit.customElementRegistry` is
    // `CustomElementRegistry?` — a plain object is neither null nor a
    // registry platform object (Codex PR331 R8 audit: previously
    // accepted-and-ignored).
    let out = run(
        "var host = document.createElement('div'); var caught = ''; \
         try { host.attachShadow({mode: 'open', customElementRegistry: {}}); } \
         catch (e) { caught = '' + e; } \
         (caught.indexOf('TypeError') !== -1 \
          && caught.indexOf('CustomElementRegistry') !== -1 \
          && host.shadowRoot === null) ? 'ok' : ('fail:' + caught);",
    );
    assert_eq!(out, "ok");
}

#[test]
fn attach_shadow_null_registry_throws_not_supported() {
    // A null-registry shadow tree (spec-legal, never upgraded) needs
    // per-element registry association — rejected loudly until slot
    // `#11-shadow-scoped-custom-element-registry` lands. The throw
    // happens at init parsing, before any mutation.
    let out = run(
        "var host = document.createElement('div'); var caught = ''; \
         try { host.attachShadow({mode: 'open', customElementRegistry: null}); } \
         catch (e) { caught = '' + e; } \
         (caught.indexOf('NotSupportedError') !== -1 \
          && host.shadowRoot === null) ? 'ok' : ('fail:' + caught);",
    );
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
    // ShadowRoot wrappers are `ObjectKind::HostObject { entity_bits }`
    // carrying the shadow root entity, so `entity_from_this` returns
    // it unchanged and the inherited ParentNode mixin methods mutate
    // the shadow tree directly.  Sibling test
    // `shadow_root_reader_mixin_reaches_via_document_fragment_chain`
    // (in `tests_parent_node_mixin.rs`) locks the reader half of the
    // mixin reaching ShadowRoot via the same DocumentFragment.prototype
    // chain.
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

#[test]
fn nested_shadow_slots_dont_interfere_with_outer_distribution() {
    // R14 finding #4 + #5: named-mode `first_named_slot_in_shadow`
    // and manual-mode `all_slots_in_shadow` walkers must NOT
    // descend into nested ShadowRoot subtrees (WHATWG DOM §4.8
    // shadow encapsulation).  Without the boundary skip, an inner
    // shadow's slot would be reported as the outer tree's first
    // matching slot, breaking distribution.
    //
    // Test: outer host with two children (`x` slot=outer,
    // `y` slot=outer), shadow has [inner_host (with its OWN shadow
    // containing a `<slot name="outer">`), outer_slot_named_outer].
    // The outer_slot_named_outer is the LATER slot in tree order,
    // but the inner slot belongs to a different shadow tree so the
    // outer_slot_named_outer should still claim x+y.
    let out = run("var host = document.createElement('div'); \
         document.body.appendChild(host); \
         var x = document.createElement('span'); x.setAttribute('slot', 'outer'); \
         host.appendChild(x); \
         var sr = host.attachShadow({mode: 'open'}); \
         var inner_host = document.createElement('div'); sr.append(inner_host); \
         var inner_sr = inner_host.attachShadow({mode: 'open'}); \
         var inner_slot = document.createElement('slot'); \
         inner_slot.setAttribute('name', 'outer'); \
         inner_sr.append(inner_slot); \
         var outer_slot = document.createElement('slot'); \
         outer_slot.setAttribute('name', 'outer'); \
         sr.append(outer_slot); \
         var an = outer_slot.assignedNodes(); \
         (an.length === 1 && an[0] === x) \
           ? 'ok' : 'fail:' + an.length;");
    assert_eq!(out, "ok");
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
fn slot_assign_accepts_uppercase_slot_tag() {
    // R1 finding #6: `slot_assign` tag check is case-insensitive,
    // matching sibling HTML tag lookups (e.g. `first_child_with_tag`).
    // Tags inserted via APIs that preserve case (custom parsers,
    // SVG-style attribute sets) must still validate as `<slot>`.
    let mut dom = elidex_ecs::EcsDom::new();
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
fn shadow_root_getter_on_non_wrapper_receiver_throws_type_error() {
    // R11 finding #1: `Element.prototype.shadowRoot.get.call({})`
    // (plain Object — not a DOM HostObject) must throw "Illegal
    // invocation" TypeError per WebIDL brand-check semantics.
    // Previously returned `null` because `require_receiver` lumps
    // non-wrapper and unbound-wrapper into the same `Ok(None)`
    // branch; new `this_is_node_wrapper` pre-check distinguishes
    // them.
    let script = format!(
        "{FIND_GETTER_PRELUDE}\
         var host = document.createElement('div'); \
         var getter = findGetter(host, 'shadowRoot'); \
         var caught_plain = null; \
         try {{ getter.call({{}}); }} catch (e) {{ caught_plain = e; }} \
         var caught_prim = null; \
         try {{ getter.call(42); }} catch (e) {{ caught_prim = e; }} \
         (caught_plain && caught_plain.name === 'TypeError' \
          && caught_prim && caught_prim.name === 'TypeError') \
           ? 'ok' : 'fail';"
    );
    let out = run(&script);
    assert_eq!(out, "ok");
}

#[test]
fn html_slot_methods_on_non_wrapper_receiver_throw_type_error() {
    // R11 finding #3: `HTMLSlotElement.prototype.assign.call({})`
    // (and `assignedNodes` / `assignedElements`) must throw
    // "Illegal invocation" TypeError per WebIDL brand-check
    // semantics — `require_slot_receiver` now pre-throws for
    // non-wrapper receivers instead of silently returning
    // default (empty array / no-op).
    let out = run("var s = document.createElement('slot'); \
         var caught_assign = null; \
         try { s.assign.call({}, s); } catch (e) { caught_assign = e; } \
         var caught_nodes = null; \
         try { s.assignedNodes.call({}); } catch (e) { caught_nodes = e; } \
         var caught_elements = null; \
         try { s.assignedElements.call(42); } catch (e) { caught_elements = e; } \
         (caught_assign && caught_assign.name === 'TypeError' \
          && caught_nodes && caught_nodes.name === 'TypeError' \
          && caught_elements && caught_elements.name === 'TypeError') \
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
    let script = format!(
        "{FIND_GETTER_PRELUDE}\
         var host = document.createElement('div'); \
         var getter = findGetter(host, 'shadowRoot'); \
         var caught = null; \
         try {{ getter.call(document); }} catch (e) {{ caught = e; }} \
         (caught !== null && caught.name === 'TypeError') \
           ? 'ok' : 'fail:' + (caught && caught.name);"
    );
    let out = run(&script);
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
fn shadow_root_accessors_reject_element_wrapper_receiver() {
    // H-migration discriminator test (Lesson #276 / [feedback_objectkind-resolution-uniformity]).
    //
    // Before H-migration, `ShadowRoot` wrappers were a distinct
    // `ObjectKind::ShadowRoot` variant and brand checks used
    // `matches!(kind, ObjectKind::ShadowRoot)` — passing an Element
    // wrapper (`ObjectKind::HostObject { .. }`) tripped the variant
    // mismatch and threw immediately.  After H-migration both
    // Element and ShadowRoot wrappers are `HostObject { entity_bits }`;
    // the ONLY thing that distinguishes them is the engine ECS
    // `ShadowRoot` component on the entity.  A regression that
    // dropped the component check would let an Element wrapper
    // silently pass the brand check.
    //
    // This test calls each of the 6 ShadowRoot accessors with an
    // Element wrapper as the receiver and asserts each throws
    // TypeError.  Locks the Phase 2 brand-check rewrite end-to-end.
    let script = format!(
        "{FIND_GETTER_PRELUDE}\
         var host = document.createElement('div'); \
         var sr = host.attachShadow({{mode: 'open'}}); \
         var srProto = Object.getPrototypeOf(sr); \
         var accessors = ['host','mode','delegatesFocus','slotAssignment','clonable','serializable']; \
         var allThrew = true; \
         var which = ''; \
         for (var i = 0; i < accessors.length; i++) {{ \
             var name = accessors[i]; \
             var g = findGetter(srProto, name); \
             if (!g) {{ allThrew = false; which = 'no-getter:' + name; break; }} \
             var threw = false; \
             try {{ g.call(host); }} catch (e) {{ threw = (e && e.name === 'TypeError'); }} \
             if (!threw) {{ allThrew = false; which = name; break; }} \
         }} \
         allThrew ? 'ok' : 'fail:' + which;"
    );
    let out = run(&script);
    assert_eq!(out, "ok");
}

#[test]
fn html_slot_methods_reject_element_wrapper_receiver() {
    // Sibling H-migration discriminator test for HTMLSlotElement
    // brand checks (`assign` / `assignedNodes` / `assignedElements`).
    // Same rationale as `shadow_root_accessors_reject_element_wrapper_receiver`:
    // post-H both `<slot>` and any other Element are `HostObject {
    // entity_bits }`; the slot-specific brand check must verify the
    // engine-side `<slot>` tag (or equivalent ECS marker), not just
    // a wrapper-kind variant.  This test calls each slot method with
    // a non-slot Element wrapper as receiver and asserts TypeError.
    let out = run(
        "var s = document.createElement('slot'); \
         var div = document.createElement('div'); \
         var assignThrew = false; \
         try { s.assign.call(div, s); } catch (e) { assignThrew = (e && e.name === 'TypeError'); } \
         var nodesThrew = false; \
         try { s.assignedNodes.call(div); } catch (e) { nodesThrew = (e && e.name === 'TypeError'); } \
         var elementsThrew = false; \
         try { s.assignedElements.call(div); } catch (e) { elementsThrew = (e && e.name === 'TypeError'); } \
         (assignThrew && nodesThrew && elementsThrew) ? 'ok' \
           : 'fail: assign=' + assignThrew + ' nodes=' + nodesThrew + ' elements=' + elementsThrew;",
    );
    assert_eq!(out, "ok");
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
