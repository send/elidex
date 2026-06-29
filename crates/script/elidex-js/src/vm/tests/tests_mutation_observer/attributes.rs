//! B2-Slice-1 — end-to-end `MutationObserver` integration for the
//! **attribute-mutation methods** (`setAttribute` / `removeAttribute` /
//! `toggleAttribute`), driven by REAL JS mutations.
//!
//! These now route through the record-producing `apply_set_attribute` /
//! `apply_remove_attribute` primitives (`elidex-script-session`), which call
//! the real `EcsDom::set_attribute` / `remove_attribute` chokepoint (full
//! `ConsumerDispatcher` fan-out + derived-state reconcile preserved) and
//! surface the `oldValue` so the seam builds the §4.9 "handle attribute
//! changes" step-1 record. Delivery is the shared B1 path
//! (`push_notify_record` → `drain_notify_records` at the `invoke_dom_api`
//! boundary → §4.3 microtask → callback), producing one
//! `MutationKind::Attribute` record per attribute change.
//!
//! `oldValue` is gated by `attributeOldValue:true`; the records themselves by
//! `attributes:true` / `attributeFilter` (DOM §4.9 / §4.3.2). Unlike the
//! `delivery` module (which hand-builds a `SessionRecord` and calls
//! `deliver_mutation_records` directly), these tests close the B0
//! "test-invisible" gap by driving production `setAttribute` etc. from JS.

use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::super::value::JsValue;
use super::super::super::Vm;
use super::setup_with_root;

/// `setAttribute` of a NEW attribute fires one `attributes` record with
/// `oldValue === null` (newly added).
#[test]
fn set_attribute_new_fires_attributes_record() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {attributes:true, attributeOldValue:true}); \
         root.setAttribute('data-x', 'v1');",
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
        vm.eval("records[0].oldValue === null").unwrap(),
        JsValue::Boolean(true),
        "a newly-added attribute has null oldValue"
    );
    assert_eq!(
        vm.eval("root.getAttribute('data-x') === 'v1'").unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
}

/// `setAttribute` that CHANGES an existing attribute, with
/// `attributeOldValue:true`, exposes the pre-write value.
#[test]
fn set_attribute_change_fires_record_with_old_value() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "root.setAttribute('class', 'old'); \
         globalThis.records = null; \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {attributes:true, attributeOldValue:true}); \
         root.setAttribute('class', 'new');",
    )
    .unwrap();

    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval("records[0].oldValue === 'old'").unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("root.getAttribute('class') === 'new'").unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
}

/// Without `attributeOldValue`, the record's `oldValue` is null even on a
/// change (DOM §4.3.2 delivery filter).
#[test]
fn set_attribute_change_no_old_value_when_not_requested() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "root.setAttribute('class', 'old'); \
         globalThis.records = null; \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {attributes:true}); \
         root.setAttribute('class', 'new');",
    )
    .unwrap();

    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval("records[0].oldValue === null").unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
}

/// I4 — a SAME-VALUE `setAttribute` still fires a record (DOM §4.9 "change an
/// attribute" queues a record unconditionally; the attribute mutation is
/// performed even when newValue == oldValue).
#[test]
fn set_attribute_same_value_still_fires_record() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "root.setAttribute('class', 'same'); \
         globalThis.records = null; \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {attributes:true, attributeOldValue:true}); \
         root.setAttribute('class', 'same');",
    )
    .unwrap();

    assert_eq!(
        vm.eval("records.length").unwrap(),
        JsValue::Number(1.0),
        "same-value setAttribute must still queue a record (I4)"
    );
    assert_eq!(
        vm.eval("records[0].oldValue === 'same'").unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
}

/// `removeAttribute` of a PRESENT attribute fires a record carrying the
/// removed value as `oldValue`.
#[test]
fn remove_attribute_present_fires_record_with_old_value() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "root.setAttribute('data-x', 'bye'); \
         globalThis.records = null; \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {attributes:true, attributeOldValue:true}); \
         root.removeAttribute('data-x');",
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
    assert_eq!(
        vm.eval("root.hasAttribute('data-x')").unwrap(),
        JsValue::Boolean(false)
    );
    vm.unbind();
}

/// I4 — `removeAttribute` of an ABSENT attribute performs no mutation, so it
/// fires NO record (DOM §4.9 "remove an attribute by name" only queues when
/// the attribute existed).
#[test]
fn remove_attribute_missing_fires_no_record() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {attributes:true}); \
         root.removeAttribute('never-existed');",
    )
    .unwrap();

    assert_eq!(
        vm.eval("records").unwrap(),
        JsValue::Null,
        "removeAttribute on an absent attribute must queue no record (I4)"
    );
    vm.unbind();
}

/// `toggleAttribute` add (absent → present) then remove (present → absent)
/// each fire an `attributes` record.
#[test]
fn toggle_attribute_add_then_remove_fires_records() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.count = 0; globalThis.last = null; \
         var mo = new MutationObserver(function(r){ globalThis.count += r.length; globalThis.last = r; }); \
         mo.observe(root, {attributes:true}); \
         globalThis.added = root.toggleAttribute('hidden');",
    )
    .unwrap();
    assert_eq!(vm.eval("added").unwrap(), JsValue::Boolean(true));
    assert_eq!(vm.eval("count").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval("last[0].type === 'attributes' && last[0].attributeName === 'hidden'")
            .unwrap(),
        JsValue::Boolean(true)
    );

    vm.eval("globalThis.removed = root.toggleAttribute('hidden');")
        .unwrap();
    assert_eq!(vm.eval("removed").unwrap(), JsValue::Boolean(false));
    assert_eq!(
        vm.eval("count").unwrap(),
        JsValue::Number(2.0),
        "toggle-off fires a second record"
    );
    assert_eq!(
        vm.eval("root.hasAttribute('hidden')").unwrap(),
        JsValue::Boolean(false)
    );
    vm.unbind();
}

/// `attributeFilter` gates records to the listed local names only.
#[test]
fn attribute_filter_gates_records() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.count = 0; globalThis.last = null; \
         var mo = new MutationObserver(function(r){ globalThis.count += r.length; globalThis.last = r; }); \
         mo.observe(root, {attributes:true, attributeFilter:['class']}); \
         root.setAttribute('data-x', '1'); \
         root.setAttribute('class', 'c');",
    )
    .unwrap();

    assert_eq!(
        vm.eval("count").unwrap(),
        JsValue::Number(1.0),
        "only the filtered 'class' write is delivered, not 'data-x'"
    );
    assert_eq!(
        vm.eval("last[0].attributeName === 'class'").unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
}

/// `subtree:true` observes an attribute change on a DESCENDANT element; the
/// record's `target` is the descendant, not the observed root.
#[test]
fn subtree_observes_descendant_attribute() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.child = document.createElement('span'); root.appendChild(child); \
         globalThis.records = null; \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {attributes:true, subtree:true}); \
         child.setAttribute('data-y', '2');",
    )
    .unwrap();

    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval("records[0].type === 'attributes' && records[0].target === child")
            .unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("records[0].attributeName === 'data-y'").unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
}

/// A `{childList:true}`-only observer sees NO record for an attribute mutation
/// (attributes gating).
#[test]
fn attributes_record_not_fired_without_attributes_option() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {childList:true}); \
         root.setAttribute('data-x', '1');",
    )
    .unwrap();

    assert_eq!(vm.eval("records").unwrap(), JsValue::Null);
    assert_eq!(
        vm.eval("root.getAttribute('data-x') === '1'").unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
}

// ---------------------------------------------------------------------------
// I2 fan-out lock — routing the record-producing seam through the real
// `EcsDom::set_attribute` chokepoint (NOT the prior buffered bypass that
// dropped the dispatch) must STILL drive the attribute-derived-state
// reconcile. These lock that producing the record did not cost the fan-out.
// ---------------------------------------------------------------------------

/// `setAttribute('style', …)` through the record path still re-derives the
/// `InlineStyle` (the bypass-vs-chokepoint regression the plan calls out).
#[test]
fn set_attribute_record_path_preserves_inline_style_reconcile() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {attributes:true}); \
         root.setAttribute('style', 'margin-top: 5px'); \
         globalThis.before = root.style.getPropertyValue('margin-top'); \
         root.setAttribute('style', 'margin-top: 9px');",
    )
    .unwrap();

    // The record still fires for each style write (2 records).
    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(2.0));
    // And the InlineStyle was re-derived from the chokepoint reconcile: the
    // first read materialized '5px', the second write must invalidate it.
    assert_eq!(vm.eval("before === '5px'").unwrap(), JsValue::Boolean(true));
    assert_eq!(
        vm.eval("root.style.getPropertyValue('margin-top') === '9px'")
            .unwrap(),
        JsValue::Boolean(true),
        "InlineStyle reconcile must fire through the record-producing path \
         (else the stale cache survives — the dropped-fan-out bug)"
    );
    vm.unbind();
}

/// `setAttribute('id', …)` through the record path still bumps the revision so
/// the `getElementById` index resolves the new id.
#[test]
fn set_attribute_record_path_preserves_id_index() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {attributes:true}); \
         root.setAttribute('id', 'lock-id');",
    )
    .unwrap();

    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval("document.getElementById('lock-id') === root")
            .unwrap(),
        JsValue::Boolean(true),
        "id-index must see the chokepoint rev_version bump through the record path"
    );
    vm.unbind();
}
