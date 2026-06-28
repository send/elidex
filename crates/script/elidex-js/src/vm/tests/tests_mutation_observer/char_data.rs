//! B1.3-i — end-to-end `MutationObserver` integration for the
//! **CharacterData data-mutation methods** (`setData` / `appendData` /
//! `insertData` / `deleteData` / `replaceData`).
//!
//! These methods now route through the record-producing
//! `apply_replace_data` primitive (`elidex-script-session`) → the B1
//! chokepoint (`session.push_notify_record` → `drain_notify_records` at
//! the `invoke_dom_api` boundary → §4.3 microtask → callback), producing
//! a single `MutationKind::CharacterData` record per call. `oldValue` is
//! gated by `characterDataOldValue:true`; the records themselves by
//! `characterData:true` (DOM §4.10 / §4.3.2).

use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::super::value::JsValue;
use super::super::super::Vm;
use super::setup_with_root;

/// Scenario 1 — `text.data = "new"` (setData) fires a characterData record.
#[test]
fn text_node_set_data_fires_characterdata_record() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         globalThis.t = document.createTextNode('old'); root.appendChild(t); \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(t, {characterData:true}); \
         t.data = 'new';",
    )
    .unwrap();

    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval("records[0].type === 'characterData'").unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("records[0].target === t").unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(vm.eval("t.data === 'new'").unwrap(), JsValue::Boolean(true));
    vm.unbind();
}

/// Scenario 2 — `replaceData` with `characterDataOldValue:true` exposes the
/// pre-mutation data string.
#[test]
fn text_node_replace_data_fires_record_with_old_value() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         globalThis.t = document.createTextNode('hello'); root.appendChild(t); \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(t, {characterData:true, characterDataOldValue:true}); \
         t.replaceData(0, 2, 'X');",
    )
    .unwrap();

    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval("records[0].oldValue === 'hello'").unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("t.data === 'Xllo'").unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
}

/// Scenario 3 — without `characterDataOldValue`, `oldValue` is null.
#[test]
fn text_node_replace_data_no_old_value_when_not_requested() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         globalThis.t = document.createTextNode('hello'); root.appendChild(t); \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(t, {characterData:true}); \
         t.replaceData(0, 2, 'X');",
    )
    .unwrap();

    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval("records[0].oldValue === null").unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
}

/// Scenario 4 — `appendData` fires a characterData record.
#[test]
fn text_node_append_data_fires_record() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         globalThis.t = document.createTextNode('ab'); root.appendChild(t); \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(t, {characterData:true}); \
         t.appendData('cd');",
    )
    .unwrap();

    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval("records[0].type === 'characterData'").unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("t.data === 'abcd'").unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
}

/// Scenario 5 — `insertData` + `deleteData` each fire a record (2 callbacks).
#[test]
fn text_node_insert_delete_data_fires_records() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.count = 0; globalThis.last = null; \
         globalThis.t = document.createTextNode('hello'); root.appendChild(t); \
         var mo = new MutationObserver(function(r){ globalThis.count += r.length; globalThis.last = r; }); \
         mo.observe(t, {characterData:true}); \
         t.insertData(2, 'XY');",
    )
    .unwrap();
    assert_eq!(vm.eval("count").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval("t.data === 'heXYllo'").unwrap(),
        JsValue::Boolean(true)
    );

    vm.eval("t.deleteData(0, 2);").unwrap();
    assert_eq!(vm.eval("count").unwrap(), JsValue::Number(2.0));
    assert_eq!(
        vm.eval("last[0].type === 'characterData'").unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("t.data === 'XYllo'").unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
}

/// Scenario 6 — a Comment node fires the same-shape characterData record
/// (A3), observed through its parent's subtree.
#[test]
fn comment_node_set_data_fires_characterdata_record() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         globalThis.c = document.createComment('old'); root.appendChild(c); \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {characterData:true, subtree:true}); \
         c.data = 'x';",
    )
    .unwrap();

    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval("records[0].type === 'characterData'").unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("records[0].target === c").unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(vm.eval("c.data === 'x'").unwrap(), JsValue::Boolean(true));
    vm.unbind();
}

/// Scenario 7 — a `{childList:true}`-only observer sees NO record for a data
/// mutation (characterData gating).
#[test]
fn characterdata_record_not_fired_without_characterdata_option() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         globalThis.t = document.createTextNode('old'); root.appendChild(t); \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(t, {childList:true}); \
         t.data = 'new';",
    )
    .unwrap();

    assert_eq!(vm.eval("records").unwrap(), JsValue::Null);
    assert_eq!(vm.eval("t.data === 'new'").unwrap(), JsValue::Boolean(true));
    vm.unbind();
}
