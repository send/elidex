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
//!
//! B1.3-ii adds `Text.splitText` coverage here: the §4.11 "split a Text
//! node" AO queues a childList insert (step 7, parented only) + a
//! characterData head-truncate (step 8 via the §4.10 replace-data AO),
//! both delivered through the shared chokepoint.

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

// ---------------------------------------------------------------------------
// R1 (Codex P2) — textContent / nodeValue setters on a CharacterData node are
// also characterData data-mutations (DOM §4.4 = `replace data(0, length, value)`)
// and must produce characterData records, matching the data-mutation methods.
// ---------------------------------------------------------------------------

/// `text.textContent = "new"` fires a characterData record with oldValue.
#[test]
fn text_node_text_content_set_fires_characterdata_record() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         globalThis.t = document.createTextNode('old'); root.appendChild(t); \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(t, {characterData:true, characterDataOldValue:true}); \
         t.textContent = 'new';",
    )
    .unwrap();

    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval("records[0].type === 'characterData' && records[0].target === t")
            .unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("records[0].oldValue === 'old'").unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(vm.eval("t.data === 'new'").unwrap(), JsValue::Boolean(true));
    vm.unbind();
}

/// `text.nodeValue = "new"` fires a characterData record.
#[test]
fn text_node_node_value_set_fires_characterdata_record() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         globalThis.t = document.createTextNode('old'); root.appendChild(t); \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(t, {characterData:true}); \
         t.nodeValue = 'new';",
    )
    .unwrap();

    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval("records[0].type === 'characterData'").unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(vm.eval("t.data === 'new'").unwrap(), JsValue::Boolean(true));
    vm.unbind();
}

/// `comment.textContent = "x"` (Comment node) fires a characterData record —
/// the Comment branch routes through `EcsDom::replace_comment_data`.
#[test]
fn comment_node_text_content_set_fires_characterdata_record() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         globalThis.c = document.createComment('old'); root.appendChild(c); \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {characterData:true, subtree:true}); \
         c.textContent = 'x';",
    )
    .unwrap();

    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval("records[0].type === 'characterData' && records[0].target === c")
            .unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(vm.eval("c.data === 'x'").unwrap(), JsValue::Boolean(true));
    vm.unbind();
}

/// B1.3-ii — `Text.splitText` on a **parented** node fires TWO records in spec
/// order: the §4.11 step-7 childList insert (new tail node on the parent) THEN
/// the step-8 characterData head-truncate (on the original node). `oldValue` =
/// the full pre-split data.
#[test]
fn split_text_parented_fires_childlist_then_characterdata() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         globalThis.t = document.createTextNode('hello world'); root.appendChild(t); \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {childList:true, characterData:true, \
                           characterDataOldValue:true, subtree:true}); \
         globalThis.tail = t.splitText(5);",
    )
    .unwrap();

    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(2.0));
    // Record 0: childList insert of the new tail node on root.
    assert_eq!(
        vm.eval("records[0].type === 'childList' && records[0].target === root")
            .unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("records[0].addedNodes.length === 1 && records[0].addedNodes[0] === tail")
            .unwrap(),
        JsValue::Boolean(true)
    );
    // Record 1: characterData head-truncate on the original node.
    assert_eq!(
        vm.eval("records[1].type === 'characterData' && records[1].target === t")
            .unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("records[1].oldValue === 'hello world'").unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("t.data === 'hello'").unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("tail.data === ' world'").unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
}

/// B1.3-ii — `Text.splitText` on an **orphan** node (no parent) fires only the
/// §4.11 step-8 characterData record; step 7 (insert) is skipped, so there is no
/// childList record.
#[test]
fn split_text_orphan_fires_only_characterdata() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    // `t` is created but NOT appended — it is an orphan. Observe `t` itself
    // (no parent to observe via subtree).
    vm.eval(
        "globalThis.records = null; \
         globalThis.t = document.createTextNode('hello world'); \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(t, {characterData:true, characterDataOldValue:true}); \
         globalThis.tail = t.splitText(5);",
    )
    .unwrap();

    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval("records[0].type === 'characterData' && records[0].target === t")
            .unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("records[0].oldValue === 'hello world'").unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("t.data === 'hello'").unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
}

/// B1.3-ii — `characterDataOldValue` gate: a characterData record produced by
/// `splitText` exposes `oldValue` ONLY to a registration with
/// `characterDataOldValue:true`; with `false` (the default) it is `null`
/// (DOM §4.3.2 delivery filter, reused from #424).
#[test]
fn split_text_oldvalue_gated_by_character_data_old_value() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    // characterDataOldValue NOT set (defaults false) → oldValue is null.
    vm.eval(
        "globalThis.records = null; \
         globalThis.t = document.createTextNode('hello world'); root.appendChild(t); \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(t, {characterData:true}); \
         t.splitText(5);",
    )
    .unwrap();

    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval("records[0].type === 'characterData'").unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("records[0].oldValue === null").unwrap(),
        JsValue::Boolean(true),
        "characterDataOldValue:false (default) → oldValue null"
    );
    vm.unbind();
}
