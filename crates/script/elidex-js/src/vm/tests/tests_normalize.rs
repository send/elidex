//! PR4f C3: `Node.prototype.normalize()` — WHATWG DOM §4.4.

#![cfg(feature = "engine")]

use elidex_ecs::{Attributes, EcsDom};
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::{JsValue, ObjectKind};
use super::super::Vm;

fn empty_doc(dom: &mut EcsDom) -> elidex_ecs::Entity {
    dom.create_document_root()
}

fn entity_of(vm: &Vm, handle: JsValue) -> elidex_ecs::Entity {
    let JsValue::Object(id) = handle else {
        panic!("expected HostObject wrapper")
    };
    let ObjectKind::HostObject { entity_bits } = vm.inner.get_object(id).kind else {
        panic!("expected HostObject")
    };
    elidex_ecs::Entity::from_bits(entity_bits).unwrap()
}

#[test]
fn normalize_merges_adjacent_text_siblings() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = empty_doc(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let joined = vm
        .eval(
            "var p = document.createElement('p');\
             p.appendChild(document.createTextNode('ab'));\
             p.appendChild(document.createTextNode('cd'));\
             p.appendChild(document.createTextNode('ef'));\
             p.normalize();\
             p.childNodes.length + ':' + p.firstChild.data;",
        )
        .unwrap();
    let JsValue::String(sid) = joined else {
        panic!()
    };
    assert_eq!(vm.inner.strings.get_utf8(sid), "1:abcdef");
    vm.unbind();
}

#[test]
fn normalize_removes_empty_text_nodes() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = empty_doc(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let count = vm
        .eval(
            "var p = document.createElement('p');\
             p.appendChild(document.createTextNode(''));\
             p.appendChild(document.createElement('span'));\
             p.appendChild(document.createTextNode(''));\
             p.normalize();\
             p.childNodes.length;",
        )
        .unwrap();
    assert!(matches!(count, JsValue::Number(n) if (n - 1.0).abs() < 1e-9));
    vm.unbind();
}

#[test]
fn normalize_recurses_into_descendants() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = empty_doc(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let check = vm
        .eval(
            "var root = document.createElement('section');\
             var inner = document.createElement('div');\
             root.appendChild(inner);\
             inner.appendChild(document.createTextNode('hi '));\
             inner.appendChild(document.createTextNode('there'));\
             root.normalize();\
             inner.childNodes.length === 1 && inner.firstChild.data === 'hi there';",
        )
        .unwrap();
    assert!(matches!(check, JsValue::Boolean(true)));
    vm.unbind();
}

#[test]
fn normalize_on_text_receiver_is_noop() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = empty_doc(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let ok = vm
        .eval(
            "var t = document.createTextNode('unchanged');\
             t.normalize();\
             t.data === 'unchanged';",
        )
        .unwrap();
    assert!(matches!(ok, JsValue::Boolean(true)));
    vm.unbind();
}

#[test]
fn normalize_on_document_normalises_all_subtrees() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    // Pre-populate with <html><body>hello world</body></html> split
    // across two adjacent Text siblings so normalize has work to do.
    let html = dom.create_element("html", Attributes::default());
    let body = dom.create_element("body", Attributes::default());
    let t1 = dom.create_text("hello ");
    let t2 = dom.create_text("world");
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, body));
    assert!(dom.append_child(body, t1));
    assert!(dom.append_child(body, t2));

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let check = vm
        .eval(
            "document.normalize();\
             document.body.childNodes.length === 1 && \
             document.body.firstChild.data === 'hello world';",
        )
        .unwrap();
    assert!(matches!(check, JsValue::Boolean(true)));
    vm.unbind();
}

#[test]
fn normalize_is_no_op_when_no_adjacent_text() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = empty_doc(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    // Each Text node is separated by an Element — no merging.
    let result = vm
        .eval(
            "var p = document.createElement('p');\
             p.appendChild(document.createTextNode('a'));\
             p.appendChild(document.createElement('br'));\
             p.appendChild(document.createTextNode('b'));\
             p.normalize();\
             p.childNodes.length + '|' + p.firstChild.data + '|' + p.lastChild.data;",
        )
        .unwrap();
    let JsValue::String(sid) = result else {
        panic!()
    };
    assert_eq!(vm.inner.strings.get_utf8(sid), "3|a|b");
    vm.unbind();
}

#[test]
fn normalize_interleaved_5_text_3_element_preserves_ordering() {
    // S3 lock-in: mixed Text / Element iteration safety.
    //
    // Layout (children of `p`, in order):
    //   T("A") T("B") <span/> T("") T("C") <i>X</i> T("") T("")
    // After normalize:
    //   - T("A")+T("B") merge → "AB"
    //   - T("") removed
    //   - T("C") alone → unchanged
    //   - <span/> and <i>X</i> untouched, <i>X</i> stays intact
    //   - trailing two empties removed
    // Expected child sequence:
    //   T("AB") <span/> T("C") <i>X</i>
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = empty_doc(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    // Use JS to build the fixture so we exercise the live-tree helpers.
    let handle = vm
        .eval(
            "var p = document.createElement('p');\
             p.appendChild(document.createTextNode('A'));\
             p.appendChild(document.createTextNode('B'));\
             p.appendChild(document.createElement('span'));\
             p.appendChild(document.createTextNode(''));\
             p.appendChild(document.createTextNode('C'));\
             var i = document.createElement('i');\
             i.appendChild(document.createTextNode('X'));\
             p.appendChild(i);\
             p.appendChild(document.createTextNode(''));\
             p.appendChild(document.createTextNode(''));\
             p.normalize();\
             p;",
        )
        .unwrap();
    let p = entity_of(&vm, handle);
    vm.unbind();

    let children: Vec<_> = dom.children(p);
    assert_eq!(children.len(), 4, "expected 4 children after normalise");
    // child 0: merged Text "AB"
    let child0_text = dom
        .world()
        .get::<&elidex_ecs::TextContent>(children[0])
        .unwrap()
        .0
        .clone();
    assert_eq!(child0_text, "AB");
    // child 1: <span>
    assert!(dom.is_element(children[1]));
    // child 2: Text "C"
    let child2_text = dom
        .world()
        .get::<&elidex_ecs::TextContent>(children[2])
        .unwrap()
        .0
        .clone();
    assert_eq!(child2_text, "C");
    // child 3: <i>X</i> intact (normalize doesn't remove it)
    assert!(dom.is_element(children[3]));
    assert_eq!(dom.children(children[3]).len(), 1);
}
