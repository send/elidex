//! B2-Slice-3 — end-to-end `MutationObserver` integration for the
//! **node-identity attribute mutators** (`Attr.value` setter,
//! `Element.setAttributeNode` / `removeAttributeNode`,
//! `NamedNodeMap.setNamedItem` / `removeNamedItem`), driven by REAL JS
//! mutations, PLUS the whole-surface attribute-name casing fold.
//!
//! These five APIs now route their attribute write through the
//! record-producing `apply_set_attribute` / `apply_remove_attribute`
//! primitives (the same seam as Slice-1's generic `setAttribute` / Slice-2's
//! reflected setters), so every node-identity content-attribute write emits one
//! DOM §4.9 "attributes" record. The load-bearing negative controls are
//! (a) a *detached* `attr.value =` (set-an-existing-attribute-value step 1 —
//! record-less by construction, I1) and (b) `setAttributeNode(oldAttr)` where
//! `oldAttr === attr` (set-an-attribute step 4 — return-before-write, NO
//! record).
//!
//! The casing fold routes every name-based attribute lookup through the single
//! canonical `EcsDom::resolve_attribute_qname` (HTML-namespace-gated lowercase,
//! SVG / MathML case-preserved), so an HTML `getAttributeNode('ID')` finds
//! `id`, the VM `hasAttribute('ID')` matches the dom-api path, and an SVG
//! `viewBox` survives verbatim — while WebIDL `nnm['ID']` bracket access stays
//! case-sensitive (supported-property-names, no lookup-name lowercase).

use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::super::value::JsValue;
use super::super::super::Vm;
use super::{setup_with_root, setup_with_root_and_svg};

// ===========================================================================
// Record production (positive)
// ===========================================================================

/// `attr = el.getAttributeNode('id'); attr.value = 'x'` (ATTACHED) fires one
/// `attributes` record carrying the prior value as `oldValue`.
#[test]
fn attr_value_setter_attached_fires_record() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "root.setAttribute('data-x', 'old'); \
         var attr = root.getAttributeNode('data-x'); \
         globalThis.records = null; \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {attributes:true, attributeOldValue:true}); \
         attr.value = 'new';",
    )
    .unwrap();

    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval("records[0].type === 'attributes' && records[0].target === root")
            .unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("records[0].attributeName === 'data-x'").unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("records[0].oldValue === 'old'").unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("root.getAttribute('data-x') === 'new'").unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
}

/// I4 — an attached `attr.value = sameValue` still fires a record (DOM §4.9
/// "change an attribute" queues unconditionally).
#[test]
fn attr_value_setter_same_value_still_fires_record() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "root.setAttribute('data-x', 'same'); \
         var attr = root.getAttributeNode('data-x'); \
         globalThis.records = null; \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {attributes:true, attributeOldValue:true}); \
         attr.value = 'same';",
    )
    .unwrap();

    assert_eq!(
        vm.eval("records.length").unwrap(),
        JsValue::Number(1.0),
        "same-value attr.value write must still queue a record (I4)"
    );
    assert_eq!(
        vm.eval("records[0].oldValue === 'same'").unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
}

/// `el.setAttributeNode(freshDetachedAttr)` on a NEW name fires one record with
/// `oldValue === null`; on an EXISTING name fires one change record (set-an-
/// attribute step 6 "replace" → handle-changes ONCE, A1×A2) and returns the
/// prior Attr.
#[test]
fn set_attribute_node_fresh_then_replace_fires_records() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    // A detached source Attr comes from a prior removeAttributeNode (no JS
    // createAttribute) — §7 input-construction constraint.
    vm.eval(
        "var donor = document.createElement('span'); \
         donor.setAttribute('data-k', 'fresh'); \
         globalThis.detached = donor.removeAttributeNode(donor.getAttributeNode('data-k')); \
         globalThis.count = 0; globalThis.last = null; \
         var mo = new MutationObserver(function(r){ globalThis.count += r.length; globalThis.last = r; }); \
         mo.observe(root, {attributes:true, attributeOldValue:true}); \
         root.setAttributeNode(detached);",
    )
    .unwrap();
    assert_eq!(
        vm.eval("count").unwrap(),
        JsValue::Number(1.0),
        "fresh-name setAttributeNode fires one record"
    );
    assert_eq!(
        vm.eval("last[0].attributeName === 'data-k' && last[0].oldValue === null")
            .unwrap(),
        JsValue::Boolean(true),
        "fresh attribute → oldValue null"
    );
    assert_eq!(
        vm.eval("root.getAttribute('data-k') === 'fresh'").unwrap(),
        JsValue::Boolean(true)
    );

    // Replace the same name with another detached Attr → 1 change record,
    // returns the prior Attr (detached snapshot of the replaced value).
    vm.eval(
        "var donor2 = document.createElement('span'); \
         donor2.setAttribute('data-k', 'replaced'); \
         var det2 = donor2.removeAttributeNode(donor2.getAttributeNode('data-k')); \
         globalThis.prev = root.setAttributeNode(det2);",
    )
    .unwrap();
    assert_eq!(
        vm.eval("count").unwrap(),
        JsValue::Number(2.0),
        "replace fires a SECOND record (ONE change record, not remove+append)"
    );
    assert_eq!(
        vm.eval("last[0].attributeName === 'data-k' && last[0].oldValue === 'fresh'")
            .unwrap(),
        JsValue::Boolean(true),
        "replace record carries the replaced value as oldValue"
    );
    assert_eq!(
        vm.eval("prev.value === 'fresh' && prev.ownerElement === null")
            .unwrap(),
        JsValue::Boolean(true),
        "setAttributeNode returns the prior Attr, detached at the replaced value"
    );
    assert_eq!(
        vm.eval("root.getAttribute('data-k') === 'replaced'")
            .unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
}

/// `el.removeAttributeNode(attr)` fires one record (oldValue = removed value)
/// and returns the SAME (now-detached) Attr.
#[test]
fn remove_attribute_node_fires_record() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "root.setAttribute('data-x', 'bye'); \
         var attr = root.getAttributeNode('data-x'); \
         globalThis.records = null; \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {attributes:true, attributeOldValue:true}); \
         globalThis.returned = root.removeAttributeNode(attr); \
         globalThis.sameObject = (returned === attr);",
    )
    .unwrap();

    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval("records[0].type === 'attributes' && records[0].attributeName === 'data-x'")
            .unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("records[0].oldValue === 'bye'").unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(vm.eval("sameObject").unwrap(), JsValue::Boolean(true));
    assert_eq!(
        vm.eval("returned.value === 'bye' && returned.ownerElement === null")
            .unwrap(),
        JsValue::Boolean(true),
        "removeAttributeNode returns the passed Attr, frozen at the removed value"
    );
    assert_eq!(
        vm.eval("root.hasAttribute('data-x')").unwrap(),
        JsValue::Boolean(false)
    );
    vm.unbind();
}

/// `el.attributes.setNamedItem(attr)` fires a record mirroring setAttributeNode.
#[test]
fn set_named_item_fires_record() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "var donor = document.createElement('span'); \
         donor.setAttribute('data-n', 'via-nnm'); \
         var detached = donor.removeAttributeNode(donor.getAttributeNode('data-n')); \
         globalThis.records = null; \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {attributes:true, attributeOldValue:true}); \
         root.attributes.setNamedItem(detached);",
    )
    .unwrap();

    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval("records[0].attributeName === 'data-n' && records[0].oldValue === null")
            .unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("root.getAttribute('data-n') === 'via-nnm'")
            .unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
}

/// `el.attributes.removeNamedItem('x')` fires a record (oldValue = removed
/// value) and returns a FRESH detached Attr over the removed value.
#[test]
fn remove_named_item_fires_record() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "root.setAttribute('data-x', 'gone'); \
         globalThis.records = null; \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {attributes:true, attributeOldValue:true}); \
         globalThis.returned = root.attributes.removeNamedItem('data-x');",
    )
    .unwrap();

    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval("records[0].attributeName === 'data-x' && records[0].oldValue === 'gone'")
            .unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("returned.value === 'gone' && returned.ownerElement === null")
            .unwrap(),
        JsValue::Boolean(true),
        "removeNamedItem returns a fresh detached Attr over the removed value"
    );
    assert_eq!(
        vm.eval("root.hasAttribute('data-x')").unwrap(),
        JsValue::Boolean(false)
    );
    vm.unbind();
}

// ===========================================================================
// Negative controls (record-less by construction)
// ===========================================================================

/// I1 / A2×A2 — a DETACHED `attr.value =` (set-an-existing-attribute-value
/// step 1: element == null) mutates only the snapshot, reaching NO chokepoint,
/// so it fires NO record. The detached Attr comes from a prior
/// removeAttributeNode (no JS createAttribute).
#[test]
fn detached_attr_value_setter_fires_no_record() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "root.setAttribute('data-x', 'v0'); \
         globalThis.detached = root.removeAttributeNode(root.getAttributeNode('data-x')); \
         globalThis.records = null; \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {attributes:true, attributeOldValue:true}); \
         detached.value = 'mutated';",
    )
    .unwrap();

    assert_eq!(
        vm.eval("records").unwrap(),
        JsValue::Null,
        "a detached attr.value write reaches no chokepoint → no record (I1)"
    );
    assert_eq!(
        vm.eval("detached.value === 'mutated'").unwrap(),
        JsValue::Boolean(true),
        "the detached snapshot is still updated in place"
    );
    assert_eq!(
        vm.eval("root.hasAttribute('data-x')").unwrap(),
        JsValue::Boolean(false),
        "the former owner is unaffected"
    );
    vm.unbind();
}

/// A1×A5 corner — `el.setAttributeNode(el.getAttributeNode('id'))` where
/// oldAttr IS attr: set-an-attribute step 4 returns attr BEFORE any change, so
/// NO record (and identity preserved).
#[test]
fn set_attribute_node_old_attr_is_attr_fires_no_record() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "root.setAttribute('id', 'self'); \
         globalThis.a = root.getAttributeNode('id'); \
         globalThis.records = null; \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {attributes:true}); \
         globalThis.ret = root.setAttributeNode(a);",
    )
    .unwrap();

    assert_eq!(
        vm.eval("records").unwrap(),
        JsValue::Null,
        "setAttributeNode(oldAttr===attr) returns before any write → no record"
    );
    assert_eq!(
        vm.eval("ret === a").unwrap(),
        JsValue::Boolean(true),
        "step 4 returns the same attr"
    );
    assert_eq!(
        vm.eval("root.getAttributeNode('id') === a").unwrap(),
        JsValue::Boolean(true),
        "identity preserved (no cache churn)"
    );
    vm.unbind();
}

/// The setNamedItem facet of the oldAttr===attr short-circuit: NO record.
#[test]
fn set_named_item_old_attr_is_attr_fires_no_record() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "root.setAttribute('id', 'self'); \
         globalThis.a = root.getAttributeNode('id'); \
         globalThis.records = null; \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {attributes:true}); \
         globalThis.ret = root.attributes.setNamedItem(a);",
    )
    .unwrap();

    assert_eq!(vm.eval("records").unwrap(), JsValue::Null);
    assert_eq!(vm.eval("ret === a").unwrap(), JsValue::Boolean(true));
    vm.unbind();
}

/// `removeNamedItem('missing')` throws NotFoundError BEFORE any mutation, so it
/// fires NO record.
#[test]
fn remove_named_item_missing_throws_and_fires_no_record() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {attributes:true}); \
         globalThis.caught = null; \
         try { root.attributes.removeNamedItem('missing'); } \
         catch (e) { globalThis.caught = e.name; }",
    )
    .unwrap();

    assert_eq!(
        vm.eval("caught === 'NotFoundError'").unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("records").unwrap(),
        JsValue::Null,
        "an absent removeNamedItem throws before any mutation → no record"
    );
    vm.unbind();
}

// ===========================================================================
// Casing (whole-surface)
// ===========================================================================

/// HTML element: a mixed-case `getAttributeNode('ID')` finds the `id` attr (was
/// a latent miss before the casing fold), and `hasAttribute('ID')` (VM) matches.
#[test]
fn html_get_attribute_node_mixed_case_finds_lowercase() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    let out = vm
        .eval(
            "root.setAttribute('id', 'x'); \
             var byUpper = root.getAttributeNode('ID'); \
             var byLower = root.getAttributeNode('id'); \
             (byUpper !== null && byUpper === byLower \
              && root.hasAttribute('ID') && root.hasAttribute('id')) ? 'ok' : 'fail';",
        )
        .unwrap();
    assert_eq!(out, JsValue::String(vm.inner.strings.intern("ok")));
    vm.unbind();
}

/// VM `hasAttribute('ID')` path consistency with the dom-api `HasAttribute`
/// (the bug fix — both now route through the canonical resolver, so the VM
/// path no longer bypasses lowercasing).
#[test]
fn vm_has_attribute_matches_dom_api_path_on_mixed_case() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    // `setAttribute` (dom-api path) lowercases 'ID' → 'id'. The VM
    // `hasAttribute('ID')` must agree (lowercase) rather than do a raw
    // case-sensitive read that would miss.
    let out = vm
        .eval(
            "root.setAttribute('ID', 'x'); \
             (root.hasAttribute('ID') === true && root.hasAttribute('id') === true \
              && root.getAttribute('id') === 'x') ? 'ok' : 'fail';",
        )
        .unwrap();
    assert_eq!(out, JsValue::String(vm.inner.strings.intern("ok")));
    vm.unbind();
}

/// SVG element (parser-built, `Namespace::Svg`): `viewBox` is case-preserved —
/// `getAttribute('viewBox')` hits, `getAttribute('viewbox')` misses; the
/// `removeNamedItem('viewBox')` record's attributeName is the verbatim
/// `viewBox`.
#[test]
fn svg_view_box_case_preserved_and_record_verbatim() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root, _svg) = setup_with_root_and_svg(&mut vm, &mut session, &mut dom);

    // Case-preservation on read.
    assert_eq!(
        vm.eval("svg.getAttribute('viewBox') === '0 0 10 10'")
            .unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("svg.getAttribute('viewbox') === null").unwrap(),
        JsValue::Boolean(true),
        "SVG attribute name must NOT collapse to lowercase"
    );
    assert_eq!(
        vm.eval("svg.getAttributeNode('viewBox') !== null").unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("svg.getAttributeNode('viewbox') === null").unwrap(),
        JsValue::Boolean(true)
    );

    // removeNamedItem('viewBox') removes it and the record's attributeName is
    // the verbatim case-preserved name.
    vm.eval(
        "globalThis.records = null; \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(svg, {attributes:true, attributeOldValue:true}); \
         svg.attributes.removeNamedItem('viewBox');",
    )
    .unwrap();
    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval("records[0].attributeName === 'viewBox'").unwrap(),
        JsValue::Boolean(true),
        "the record's attributeName is the case-preserved SVG name"
    );
    assert_eq!(
        vm.eval("records[0].oldValue === '0 0 10 10'").unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("svg.hasAttribute('viewBox')").unwrap(),
        JsValue::Boolean(false)
    );
    vm.unbind();
}

/// SVG `setAttribute('viewBox', v)` (the value-mode write through the resolver)
/// fires a record with the verbatim attributeName, and `removeAttribute` of the
/// case-preserved name removes it.
#[test]
fn svg_set_then_remove_attribute_case_preserved_records() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root, _svg) = setup_with_root_and_svg(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.count = 0; globalThis.last = null; \
         var mo = new MutationObserver(function(r){ globalThis.count += r.length; globalThis.last = r; }); \
         mo.observe(svg, {attributes:true}); \
         svg.setAttribute('preserveAspectRatio', 'xMidYMid');",
    )
    .unwrap();
    assert_eq!(vm.eval("count").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval("last[0].attributeName === 'preserveAspectRatio'")
            .unwrap(),
        JsValue::Boolean(true),
        "SVG setAttribute keeps the case-preserved name in the record"
    );
    assert_eq!(
        vm.eval("svg.getAttribute('preserveAspectRatio') === 'xMidYMid'")
            .unwrap(),
        JsValue::Boolean(true)
    );

    vm.eval("svg.removeAttribute('preserveAspectRatio');")
        .unwrap();
    assert_eq!(
        vm.eval("count").unwrap(),
        JsValue::Number(2.0),
        "case-preserved removeAttribute fires the second record"
    );
    assert_eq!(
        vm.eval("svg.hasAttribute('preserveAspectRatio')").unwrap(),
        JsValue::Boolean(false)
    );
    vm.unbind();
}

/// Bracket access stays case-sensitive (NOT-applied lock): WebIDL
/// supported-property-names do NOT lowercase the lookup name. `nnm['id']` hits,
/// `nnm['ID']` is undefined (HTML); `svg.attributes['viewBox']` hits,
/// `['viewbox']` is undefined (SVG).
#[test]
fn bracket_access_stays_case_sensitive() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root, _svg) = setup_with_root_and_svg(&mut vm, &mut session, &mut dom);

    let out = vm
        .eval(
            "root.setAttribute('id', 'x'); \
             var htmlHit = root.attributes['id']; \
             var htmlMiss = root.attributes['ID']; \
             var svgHit = svg.attributes['viewBox']; \
             var svgMiss = svg.attributes['viewbox']; \
             (htmlHit !== undefined && htmlHit.value === 'x' \
              && htmlMiss === undefined \
              && svgHit !== undefined && svgHit.value === '0 0 10 10' \
              && svgMiss === undefined) ? 'ok' \
              : 'fail:' + (htmlHit && htmlHit.value) + '/' + htmlMiss + '/' \
                        + (svgHit && svgHit.value) + '/' + svgMiss;",
        )
        .unwrap();
    assert_eq!(out, JsValue::String(vm.inner.strings.intern("ok")));
    vm.unbind();
}

// ===========================================================================
// Wrapper identity / detach
// ===========================================================================

/// remove → same-name re-add allocates a FRESH canonical wrapper (the prior
/// held Attr stays detached at its snapshot), and the re-add is record-producing.
#[test]
fn remove_then_readd_allocates_fresh_wrapper_and_records() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "root.setAttribute('data-x', 'first'); \
         var held = root.getAttributeNode('data-x'); \
         globalThis.count = 0; \
         var mo = new MutationObserver(function(r){ globalThis.count += r.length; }); \
         mo.observe(root, {attributes:true}); \
         root.removeAttributeNode(held); \
         root.setAttribute('data-x', 'second'); \
         globalThis.fresh = root.getAttributeNode('data-x'); \
         globalThis.heldStaysSnapshot = (held.value === 'first' && held.ownerElement === null); \
         globalThis.freshIsCanonical = (fresh !== held && fresh.value === 'second');",
    )
    .unwrap();

    assert_eq!(
        vm.eval("count").unwrap(),
        JsValue::Number(2.0),
        "removeAttributeNode + same-name setAttribute each fire a record"
    );
    assert_eq!(
        vm.eval("heldStaysSnapshot").unwrap(),
        JsValue::Boolean(true),
        "the held detached wrapper keeps its removal-time snapshot"
    );
    assert_eq!(
        vm.eval("freshIsCanonical").unwrap(),
        JsValue::Boolean(true),
        "a same-name re-add allocates a fresh canonical wrapper distinct from the held one"
    );
    vm.unbind();
}
