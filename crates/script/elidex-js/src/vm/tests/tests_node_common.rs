//! Node common-member tests — `ownerDocument`, `isSameNode`,
//! `getRootNode`, `isEqualNode`, `compareDocumentPosition`.

#![cfg(feature = "engine")]

use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;

fn setup() -> (Vm, SessionCore, EcsDom, elidex_ecs::Entity) {
    let vm = Vm::new();
    let session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    (vm, session, dom, doc)
}

// ---------------------------------------------------------------------------
// ownerDocument
// ---------------------------------------------------------------------------

#[test]
fn owner_document_of_element_is_document() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let JsValue::Boolean(b) = vm
        .eval("document.createElement('p').ownerDocument === document;")
        .unwrap()
    else {
        panic!()
    };
    assert!(b);
    vm.unbind();
}

#[test]
fn owner_document_of_text_is_document() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let JsValue::Boolean(b) = vm
        .eval("document.createTextNode('x').ownerDocument === document;")
        .unwrap()
    else {
        panic!()
    };
    assert!(b);
    vm.unbind();
}

#[test]
fn owner_document_of_document_is_null() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let JsValue::Null = vm.eval("document.ownerDocument;").unwrap() else {
        panic!("expected null");
    };
    vm.unbind();
}

#[test]
fn owner_document_of_node_in_cloned_document_is_that_clone() {
    // `ownerDocument` must report the tree root it actually lives
    // under — for nodes inside a cloned Document, that's the clone,
    // not the bound global document.
    let (mut vm, mut session, mut dom, doc) = setup();
    let html = dom.create_element("html", elidex_ecs::Attributes::default());
    let body = dom.create_element("body", elidex_ecs::Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, body));

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let JsValue::Boolean(b) = vm
        .eval(
            "var cloned = document.cloneNode(true);\n\
             var clonedBody = cloned.body;\n\
             clonedBody.ownerDocument === cloned;",
        )
        .unwrap()
    else {
        panic!()
    };
    assert!(
        b,
        "node inside a cloned Document must report the clone as its ownerDocument"
    );
    vm.unbind();
}

#[test]
fn owner_document_of_cloned_node_is_document() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let JsValue::Boolean(b) = vm
        .eval(
            "var p = document.createElement('p');\n\
             p.cloneNode(false).ownerDocument === document;",
        )
        .unwrap()
    else {
        panic!()
    };
    assert!(b);
    vm.unbind();
}

// ---------------------------------------------------------------------------
// isSameNode
// ---------------------------------------------------------------------------

#[test]
fn is_same_node_same_wrapper_true() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let JsValue::Boolean(b) = vm
        .eval(
            "var p = document.createElement('p');\n\
             p.isSameNode(p);",
        )
        .unwrap()
    else {
        panic!()
    };
    assert!(b);
    vm.unbind();
}

#[test]
fn is_same_node_distinct_nodes_false() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    // Two separately created elements → different wrappers.
    let JsValue::Boolean(b) = vm
        .eval(
            "var a = document.createElement('p');\n\
             var b = document.createElement('p');\n\
             a.isSameNode(b);",
        )
        .unwrap()
    else {
        panic!()
    };
    assert!(!b);
    vm.unbind();
}

#[test]
fn is_same_node_null_arg_false() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let JsValue::Boolean(b) = vm
        .eval(
            "var p = document.createElement('p');\n\
             p.isSameNode(null);",
        )
        .unwrap()
    else {
        panic!()
    };
    assert!(!b);
    vm.unbind();
}

#[test]
fn is_same_node_non_node_arg_throws() {
    // WebIDL `boolean isSameNode(Node? otherNode)`: a non-Node
    // object (e.g. plain `{}`) is a conversion failure and must
    // throw TypeError, matching `contains` / `isEqualNode` /
    // `compareDocumentPosition`.
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let threw = vm
        .eval(
            "var p = document.createElement('p');\n\
             var err = null;\n\
             try { p.isSameNode({}); } catch (e) { err = e; }\n\
             err !== null;",
        )
        .unwrap();
    assert!(matches!(threw, JsValue::Boolean(true)));
    vm.unbind();
}

// ---------------------------------------------------------------------------
// getRootNode
// ---------------------------------------------------------------------------

#[test]
fn get_root_node_detached_returns_self() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    // Detached element: root is itself.
    let JsValue::Boolean(b) = vm
        .eval(
            "var p = document.createElement('p');\n\
             p.getRootNode() === p;",
        )
        .unwrap()
    else {
        panic!()
    };
    assert!(b);
    vm.unbind();
}

// ---------------------------------------------------------------------------
// isEqualNode
// ---------------------------------------------------------------------------

#[test]
fn is_equal_node_same_tag_same_attrs_true() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let JsValue::Boolean(b) = vm
        .eval(
            "var a = document.createElement('p');\n\
             var b = document.createElement('p');\n\
             a.setAttribute('id', 'x');\n\
             b.setAttribute('id', 'x');\n\
             a.isEqualNode(b);",
        )
        .unwrap()
    else {
        panic!()
    };
    assert!(b);
    vm.unbind();
}

#[test]
fn is_equal_node_different_attrs_false() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let JsValue::Boolean(b) = vm
        .eval(
            "var a = document.createElement('p');\n\
             var b = document.createElement('p');\n\
             a.setAttribute('id', 'x');\n\
             b.setAttribute('id', 'y');\n\
             a.isEqualNode(b);",
        )
        .unwrap()
    else {
        panic!()
    };
    assert!(!b);
    vm.unbind();
}

#[test]
fn is_equal_node_different_children_order_false() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let JsValue::Boolean(b) = vm
        .eval(
            "var a = document.createElement('p');\n\
             a.appendChild(document.createElement('span'));\n\
             a.appendChild(document.createElement('em'));\n\
             var b = document.createElement('p');\n\
             b.appendChild(document.createElement('em'));\n\
             b.appendChild(document.createElement('span'));\n\
             a.isEqualNode(b);",
        )
        .unwrap()
    else {
        panic!()
    };
    assert!(!b);
    vm.unbind();
}

#[test]
fn is_equal_node_different_kind_false() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let JsValue::Boolean(b) = vm
        .eval(
            "var a = document.createElement('p');\n\
             var t = document.createTextNode('hi');\n\
             a.isEqualNode(t);",
        )
        .unwrap()
    else {
        panic!()
    };
    assert!(!b);
    vm.unbind();
}

#[test]
fn is_equal_node_non_node_arg_throws() {
    // WebIDL `Node? other` non-null non-Node (number / string /
    // plain object) is a conversion failure → TypeError.
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let threw = vm
        .eval(
            "var a = document.createElement('p');\n\
             var err = null;\n\
             try { a.isEqualNode({}); } catch (e) { err = e; }\n\
             err !== null;",
        )
        .unwrap();
    assert!(matches!(threw, JsValue::Boolean(true)));
    vm.unbind();
}

#[test]
fn is_equal_node_null_arg_false() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let JsValue::Boolean(b) = vm
        .eval(
            "var a = document.createElement('p');\n\
             a.isEqualNode(null);",
        )
        .unwrap()
    else {
        panic!()
    };
    assert!(!b);
    vm.unbind();
}

#[test]
fn is_equal_node_different_doctype_fields_false() {
    // Two DocumentType nodes with different name / publicId /
    // systemId must not compare equal.  elidex currently has no JS
    // surface to create doctypes, so the entities are built via the
    // ECS API then wrapped.
    use super::super::value::JsValue as V;

    let vm = Vm::new();
    let session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let dt_a = dom.create_document_type("html", "", "");
    let dt_b = dom.create_document_type("html", "-//W3C//DTD HTML 4.01//EN", "");
    let (mut vm, mut session, mut dom) = (vm, session, dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let wa = vm.inner.create_element_wrapper(dt_a);
    let wb = vm.inner.create_element_wrapper(dt_b);
    vm.set_global("dtA", V::Object(wa));
    vm.set_global("dtB", V::Object(wb));
    let JsValue::Boolean(b) = vm.eval("dtA.isEqualNode(dtB);").unwrap() else {
        panic!()
    };
    assert!(!b, "different DocumentType payloads must not compare equal");
    vm.unbind();
}

#[test]
fn is_equal_node_matching_doctype_true() {
    use super::super::value::JsValue as V;

    let vm = Vm::new();
    let session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let dt_a = dom.create_document_type("html", "pub", "sys");
    let dt_b = dom.create_document_type("html", "pub", "sys");
    let (mut vm, mut session, mut dom) = (vm, session, dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let wa = vm.inner.create_element_wrapper(dt_a);
    let wb = vm.inner.create_element_wrapper(dt_b);
    vm.set_global("dtA", V::Object(wa));
    vm.set_global("dtB", V::Object(wb));
    let JsValue::Boolean(b) = vm.eval("dtA.isEqualNode(dtB);").unwrap() else {
        panic!()
    };
    assert!(b);
    vm.unbind();
}

#[test]
fn is_equal_node_self_true() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let JsValue::Boolean(b) = vm
        .eval(
            "var a = document.createElement('p');\n\
             a.isEqualNode(a);",
        )
        .unwrap()
    else {
        panic!()
    };
    assert!(b);
    vm.unbind();
}

// ---------------------------------------------------------------------------
// compareDocumentPosition
// ---------------------------------------------------------------------------

fn setup_fixture() -> (Vm, SessionCore, EcsDom, elidex_ecs::Entity) {
    let vm = Vm::new();
    let session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let html = dom.create_element("html", elidex_ecs::Attributes::default());
    let body = dom.create_element("body", elidex_ecs::Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, body));
    (vm, session, dom, doc)
}

#[test]
fn compare_document_position_same_node_zero() {
    let (mut vm, mut session, mut dom, doc) = setup_fixture();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let JsValue::Number(n) = vm
        .eval("document.body.compareDocumentPosition(document.body);")
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(n, 0.0);
    vm.unbind();
}

#[test]
fn compare_document_position_sibling_following() {
    let (mut vm, mut session, mut dom, doc) = setup_fixture();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    // Add two siblings a, b under body; a.compareDocumentPosition(b)
    // should set FOLLOWING (0x04).
    let JsValue::Number(n) = vm
        .eval(
            "var a = document.createElement('a');\n\
             var b = document.createElement('b');\n\
             document.body.appendChild(a);\n\
             document.body.appendChild(b);\n\
             a.compareDocumentPosition(b);",
        )
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(n, 4.0);
    vm.unbind();
}

#[test]
fn compare_document_position_sibling_preceding() {
    let (mut vm, mut session, mut dom, doc) = setup_fixture();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    // b.compareDocumentPosition(a) → PRECEDING (0x02).
    let JsValue::Number(n) = vm
        .eval(
            "var a = document.createElement('a');\n\
             var b = document.createElement('b');\n\
             document.body.appendChild(a);\n\
             document.body.appendChild(b);\n\
             b.compareDocumentPosition(a);",
        )
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(n, 2.0);
    vm.unbind();
}

#[test]
fn compare_document_position_ancestor_contains() {
    let (mut vm, mut session, mut dom, doc) = setup_fixture();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    // body contains child → child.compareDocumentPosition(body) =
    // CONTAINS | PRECEDING = 0x08 | 0x02 = 0x0a = 10.
    let JsValue::Number(n) = vm
        .eval(
            "var child = document.createElement('child');\n\
             document.body.appendChild(child);\n\
             child.compareDocumentPosition(document.body);",
        )
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(n, 10.0);
    vm.unbind();
}

#[test]
fn compare_document_position_descendant_contained_by() {
    let (mut vm, mut session, mut dom, doc) = setup_fixture();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    // body.compareDocumentPosition(child) = CONTAINED_BY | FOLLOWING
    // = 0x10 | 0x04 = 0x14 = 20.
    let JsValue::Number(n) = vm
        .eval(
            "var child = document.createElement('child');\n\
             document.body.appendChild(child);\n\
             document.body.compareDocumentPosition(child);",
        )
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(n, 20.0);
    vm.unbind();
}

#[test]
fn compare_document_position_disconnected() {
    let (mut vm, mut session, mut dom, doc) = setup_fixture();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    // WHATWG §4.4: for disconnected nodes the result must include
    // DISCONNECTED (0x01) | IMPLEMENTATION_SPECIFIC (0x20) | one of
    // PRECEDING (0x02) / FOLLOWING (0x04), with a *consistent* order.
    // elidex uses `Entity::to_bits()` as the tiebreaker — `b` is
    // allocated after `a`, so `a.compareDocumentPosition(b)` returns
    // FOLLOWING (0x20 | 0x01 | 0x04 = 0x25 = 37).
    let JsValue::Number(n) = vm
        .eval(
            "var a = document.createElement('a');\n\
             var b = document.createElement('b');\n\
             a.compareDocumentPosition(b);",
        )
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(n, 37.0);
    vm.unbind();
}

#[test]
fn compare_document_position_disconnected_is_antisymmetric() {
    // WHATWG §4.4: swapping the operands must flip PRECEDING ↔
    // FOLLOWING.  DISCONNECTED and IMPLEMENTATION_SPECIFIC stay set
    // on both sides; XOR of the two results must equal
    // (PRECEDING | FOLLOWING) = 0x06.
    let (mut vm, mut session, mut dom, doc) = setup_fixture();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let JsValue::Number(xored) = vm
        .eval(
            "var a = document.createElement('a');\n\
             var b = document.createElement('b');\n\
             (a.compareDocumentPosition(b) ^ b.compareDocumentPosition(a));",
        )
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(xored, 6.0);
    vm.unbind();
}

// ---------------------------------------------------------------------------
// arch-hoist-d pre-emptive regression tests (skill lesson #149).
//
// Pinning behaviour the elidex-ecs algorithm move (PR #11-arch-hoist-d
// C1) introduces or preserves through the bridge, so any future
// regression surfaces at JS-level rather than only at the unit-test
// layer.
// ---------------------------------------------------------------------------

#[test]
fn is_equal_node_deep_tree_no_overflow_js() {
    // JS-level mirror of the ECS deep-tree contract: two trees of
    // depth ~3000 must compare equal without exhausting the call
    // stack.  Pre-PR the VM-side iterative walker handled this; the
    // post-PR delegation must keep the property end-to-end.
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let JsValue::Boolean(b) = vm
        .eval(
            "var build = function() {\n\
               var root = document.createElement('div');\n\
               var cur = root;\n\
               for (var i = 0; i < 3000; i++) {\n\
                 var child = document.createElement('span');\n\
                 cur.appendChild(child);\n\
                 cur = child;\n\
               }\n\
               return root;\n\
             };\n\
             build().isEqualNode(build());",
        )
        .unwrap()
    else {
        panic!()
    };
    assert!(b);
    vm.unbind();
}

#[test]
fn compare_document_position_disconnected_antisymmetric_js() {
    // Pin antisymmetry through the bridge: cmp(a,b) ^ cmp(b,a)
    // must equal PRECEDING|FOLLOWING (= 0x06) for two disconnected
    // operands.  Regression here means the ECS impl lost the
    // entity-bits ordering tiebreak.
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let JsValue::Number(n) = vm
        .eval(
            "var a = document.createElement('div');\n\
             var b = document.createElement('div');\n\
             a.compareDocumentPosition(b) ^ b.compareDocumentPosition(a);",
        )
        .unwrap()
    else {
        panic!()
    };
    // 0x02 (PRECEDING) | 0x04 (FOLLOWING) == 0x06.
    assert_eq!(n, 6.0);
    vm.unbind();
}

#[test]
fn compare_document_position_same_tree_following_js() {
    // Pin the same-tree FOLLOWING bit (= 0x04) through handler
    // dispatch.  Attr-vs-Attr is exercised at the ECS unit-test
    // layer (no Attr JS surface yet — Attr lifecycle WIP).
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let JsValue::Number(n) = vm
        .eval(
            "var p = document.createElement('div');\n\
             var a = document.createElement('span');\n\
             var b = document.createElement('span');\n\
             p.appendChild(a);\n\
             p.appendChild(b);\n\
             a.compareDocumentPosition(b);",
        )
        .unwrap()
    else {
        panic!()
    };
    // FOLLOWING == 0x04.
    assert_eq!(n, 4.0);
    vm.unbind();
}

#[test]
fn get_root_node_attached_returns_document() {
    let (mut vm, mut session, mut dom, doc) = setup();
    // Build: doc > html > p
    let html = dom.create_element("html", elidex_ecs::Attributes::default());
    let p = dom.create_element("p", elidex_ecs::Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, p));

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let JsValue::Boolean(b) = vm
        .eval("document.documentElement.firstChild.getRootNode() === document;")
        .unwrap()
    else {
        panic!()
    };
    assert!(b);
    vm.unbind();
}
