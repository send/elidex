//! B1.2c — end-to-end `MutationObserver` integration for the
//! `Node.textContent` setter on an Element / DocumentFragment (WHATWG DOM §4.4
//! "string replace all" → §4.2.3 replace-all).
//!
//! Before this slice the Element branch did a raw `EcsDom` remove-loop + append
//! (MO-silent); the convergence routes it through the record-producing
//! `apply_replace_all` (the same primitive `replaceChildren` uses) → ONE coalesced
//! childList record. Here we drive a **real JS mutation** and assert the delivered
//! records (handler-direct / boa-parity tests live in `elidex-dom-api`
//! `node_methods::tests::text_content`).

use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::super::value::JsValue;
use super::super::super::Vm;
use super::setup_with_root;

fn string_global(vm: &mut Vm, expr: &str) -> String {
    let v = vm.eval(expr).unwrap();
    let JsValue::String(sid) = v else {
        panic!("expected string from `{expr}`, got {v:?}")
    };
    vm.inner.strings.get_utf8(sid).clone()
}

#[test]
fn textcontent_set_on_element_with_children_one_coalesced_record() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         globalThis.a = document.createElement('span'); \
         globalThis.b = document.createElement('span'); \
         root.appendChild(a); root.appendChild(b); \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {childList:true}); \
         root.textContent = 'hi';",
    )
    .unwrap();

    // §4.2.3 replace-all step 7: ONE coalesced record (removed = [a,b], added = [Text]).
    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(string_global(&mut vm, "records[0].type"), "childList");
    assert_eq!(
        vm.eval(
            "records[0].removedNodes.length === 2 && records[0].removedNodes[0] === a \
                 && records[0].removedNodes[1] === b"
        )
        .unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("records[0].addedNodes.length").unwrap(),
        JsValue::Number(1.0)
    );
    // The single added node is a Text (nodeType 3) carrying the value.
    assert_eq!(
        vm.eval("records[0].addedNodes[0].nodeType").unwrap(),
        JsValue::Number(3.0)
    );
    assert_eq!(
        vm.eval("records[0].target === root").unwrap(),
        JsValue::Boolean(true)
    );
    // read-your-writes: textContent now reads back the value.
    assert_eq!(string_global(&mut vm, "root.textContent"), "hi");
    vm.unbind();
}

#[test]
fn textcontent_empty_string_on_element_with_children_removes_all_one_record() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         globalThis.a = document.createElement('span'); \
         root.appendChild(a); \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {childList:true}); \
         root.textContent = '';",
    )
    .unwrap();

    // Empty string → node is null → replace-all removes children, adds nothing:
    // ONE record (removed = [a], added = «»).
    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval("records[0].removedNodes.length === 1 && records[0].removedNodes[0] === a")
            .unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("records[0].addedNodes.length").unwrap(),
        JsValue::Number(0.0)
    );
    assert_eq!(
        vm.eval("root.childNodes.length").unwrap(),
        JsValue::Number(0.0)
    );
    vm.unbind();
}

#[test]
fn textcontent_empty_string_on_empty_element_no_record() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         globalThis.box = document.createElement('div'); root.appendChild(box); \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(box, {childList:true}); \
         box.textContent = '';",
    )
    .unwrap();

    // §4.2.3 replace-all step 7: added ∪ removed both empty → NO record queued.
    assert_eq!(vm.eval("records").unwrap(), JsValue::Null);
    vm.unbind();
}

#[test]
fn textcontent_added_text_inherits_owner_document() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    // WHATWG §4.4: the new Text's node document is this's node document.
    vm.eval("globalThis.box = document.createElement('div'); root.appendChild(box); box.textContent = 'x';")
        .unwrap();
    assert_eq!(
        vm.eval("box.firstChild.nodeType === 3 && box.firstChild.ownerDocument === document")
            .unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
}

#[test]
fn textcontent_on_text_node_emits_no_childlist_record() {
    // A Text receiver's textContent set is a characterData mutation (B1.3), NOT a
    // childList op — a childList observer on the parent must see nothing.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         globalThis.t = document.createTextNode('old'); root.appendChild(t); \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {childList:true, subtree:true}); \
         t.textContent = 'new';",
    )
    .unwrap();

    assert_eq!(vm.eval("records").unwrap(), JsValue::Null);
    // The data did update (read-your-writes), it's just not a childList record.
    assert_eq!(string_global(&mut vm, "t.textContent"), "new");
    vm.unbind();
}
