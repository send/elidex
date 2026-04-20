//! Element / Node mutation, selector, and prototype-separation tests.
//!
//! Covers:
//! - DOM mutation (`appendChild` / `removeChild` / `insertBefore` /
//!   `replaceChild` / `remove`).
//! - Selector helpers (`matches` / `closest`).
//! - Node / Window prototype separation.
//! - Element-only members must NOT surface on Text nodes.
//!
//! Split out of [`super::tests_element_methods`] to keep that file
//! under the project's 1000-line convention (PR5a C9).

#![cfg(feature = "engine")]

use elidex_ecs::{Attributes, EcsDom};
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;
use super::tests_element_methods::build_element_fixture;

// ---------------------------------------------------------------------------
// DOM mutation
// ---------------------------------------------------------------------------

#[test]
fn element_append_child_adds_new_element_and_returns_it() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let v = vm
        .eval(
            "var el = document.createElement('section'); \
             document.getElementById('root').appendChild(el) === el;",
        )
        .unwrap();
    assert!(matches!(v, JsValue::Boolean(true)));

    // Count elements on body (originally 2 — p, div; now 3 with new section).
    assert!(matches!(
        vm.eval("document.getElementById('root').childElementCount;")
            .unwrap(),
        JsValue::Number(n) if (n - 3.0).abs() < f64::EPSILON
    ));

    vm.unbind();
}

#[test]
fn element_remove_child_detaches_and_returns_node() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let p_wrapper = vm.inner.create_element_wrapper(p);
    vm.set_global("_p", JsValue::Object(p_wrapper));
    let v = vm
        .eval("document.getElementById('root').removeChild(_p) === _p;")
        .unwrap();
    assert!(matches!(v, JsValue::Boolean(true)));

    // `_p.parentNode` is now null.
    assert!(matches!(vm.eval("_p.parentNode;").unwrap(), JsValue::Null));

    vm.unbind();
}

#[test]
fn element_remove_child_of_non_child_throws() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    // span is a grandchild, not a direct child of body.  PR5a C4
    // upgrades the throw from TypeError to
    // `DOMException("NotFoundError")` (legacy code 8) per WHATWG
    // DOM §4.4.
    let span_wrapper = vm.inner.create_element_wrapper(span);
    vm.set_global("_span", JsValue::Object(span_wrapper));
    let check = vm
        .eval(
            "var root = document.getElementById('root');\
             var thrown = null;\
             try { root.removeChild(_span); } catch (e) { thrown = e; }\
             thrown && thrown.name === 'NotFoundError' \
             && thrown instanceof DOMException && thrown.code === 8;",
        )
        .unwrap();
    assert!(matches!(check, JsValue::Boolean(true)));

    vm.unbind();
}

#[test]
fn element_append_child_rejects_non_node_argument() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let r = vm.eval("document.getElementById('root').appendChild({});");
    assert!(r.is_err());
    let r = vm.eval("document.getElementById('root').appendChild(null);");
    assert!(r.is_err());

    vm.unbind();
}

#[test]
fn element_insert_before_places_new_child_ahead_of_ref() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, p, div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    // Insert a new section before div.  Ordering: p, text, section, div, comment.
    let div_wrapper = vm.inner.create_element_wrapper(div);
    vm.set_global("_div", JsValue::Object(div_wrapper));
    let _ = p;
    let v = vm
        .eval(
            "var s = document.createElement('section'); \
             document.getElementById('root').insertBefore(s, _div); \
             _div.previousElementSibling === s;",
        )
        .unwrap();
    assert!(matches!(v, JsValue::Boolean(true)));

    vm.unbind();
}

#[test]
fn element_insert_before_with_null_ref_appends() {
    // insertBefore(new, null) behaves like appendChild.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let v = vm
        .eval(
            "var s = document.createElement('section'); \
             document.getElementById('root').insertBefore(s, null); \
             document.getElementById('root').lastElementChild === s;",
        )
        .unwrap();
    assert!(matches!(v, JsValue::Boolean(true)));

    vm.unbind();
}

#[test]
fn element_replace_child_returns_old_node() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let p_wrapper = vm.inner.create_element_wrapper(p);
    vm.set_global("_p", JsValue::Object(p_wrapper));
    let v = vm
        .eval(
            "var h = document.createElement('h1'); \
             document.getElementById('root').replaceChild(h, _p) === _p;",
        )
        .unwrap();
    assert!(matches!(v, JsValue::Boolean(true)));

    // _p is now detached from body; h is in its place.
    assert!(matches!(vm.eval("_p.parentNode;").unwrap(), JsValue::Null));

    vm.unbind();
}

#[test]
fn element_replace_child_rejects_non_child_with_not_found_error() {
    // WHATWG DOM §4.4: `replaceChild` throws
    // `DOMException("NotFoundError")` when `old` is not a child of
    // the receiver.  PR5a C4 upgrade from the pre-C4 TypeError
    // surface.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let span_wrapper = vm.inner.create_element_wrapper(span);
    vm.set_global("_span", JsValue::Object(span_wrapper));
    let check = vm
        .eval(
            "var root = document.getElementById('root');\
             var h = document.createElement('h1');\
             var thrown = null;\
             try { root.replaceChild(h, _span); } catch (e) { thrown = e; }\
             thrown && thrown.name === 'NotFoundError' \
             && thrown instanceof DOMException && thrown.code === 8;",
        )
        .unwrap();
    assert!(matches!(check, JsValue::Boolean(true)));

    vm.unbind();
}

#[test]
fn element_remove_detaches_from_parent() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let p_wrapper = vm.inner.create_element_wrapper(p);
    vm.set_global("_p", JsValue::Object(p_wrapper));
    vm.eval("_p.remove();").unwrap();
    assert!(matches!(vm.eval("_p.parentNode;").unwrap(), JsValue::Null));

    vm.unbind();
}

#[test]
fn element_remove_on_detached_node_is_no_op() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    // `createElement` produces a detached element.  `.remove()` is silent.
    vm.eval("document.createElement('aside').remove();")
        .unwrap();

    vm.unbind();
}

// ---------------------------------------------------------------------------
// matches / closest
// ---------------------------------------------------------------------------

#[test]
fn element_matches_tag_class_id() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let div_wrapper = vm.inner.create_element_wrapper(div);
    vm.set_global("_div", JsValue::Object(div_wrapper));
    assert!(matches!(
        vm.eval("_div.matches('div');").unwrap(),
        JsValue::Boolean(true)
    ));
    assert!(matches!(
        vm.eval("_div.matches('.box');").unwrap(),
        JsValue::Boolean(true)
    ));
    assert!(matches!(
        vm.eval("_div.matches('.nonexistent');").unwrap(),
        JsValue::Boolean(false)
    ));
    assert!(matches!(
        vm.eval("document.getElementById('root').matches('#root');")
            .unwrap(),
        JsValue::Boolean(true)
    ));

    vm.unbind();
}

#[test]
fn element_matches_throws_on_invalid_selector() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let r = vm.eval("document.getElementById('root').matches('!!!');");
    assert!(r.is_err(), "invalid selector must throw SyntaxError");

    vm.unbind();
}

#[test]
fn element_matches_rejects_shadow_pseudos() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let r = vm.eval("document.getElementById('root').matches(':host');");
    assert!(r.is_err());

    vm.unbind();
}

#[test]
fn element_closest_returns_self_when_self_matches() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let div_wrapper = vm.inner.create_element_wrapper(div);
    vm.set_global("_div", JsValue::Object(div_wrapper));
    assert!(matches!(
        vm.eval("_div.closest('.box') === _div;").unwrap(),
        JsValue::Boolean(true)
    ));

    vm.unbind();
}

#[test]
fn element_closest_walks_up_to_matching_ancestor() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, body, _p, _div, span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let span_wrapper = vm.inner.create_element_wrapper(span);
    let body_wrapper = vm.inner.create_element_wrapper(body);
    vm.set_global("_span", JsValue::Object(span_wrapper));
    vm.set_global("_body", JsValue::Object(body_wrapper));
    assert!(matches!(
        vm.eval("_span.closest('#root') === _body;").unwrap(),
        JsValue::Boolean(true)
    ));

    vm.unbind();
}

#[test]
fn element_closest_returns_null_when_no_match() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    assert!(matches!(
        vm.eval("document.getElementById('root').closest('.nonexistent');")
            .unwrap(),
        JsValue::Null
    ));

    vm.unbind();
}

// ---------------------------------------------------------------------------
// Node / Window prototype separation (addresses Copilot #1 / #2)
// ---------------------------------------------------------------------------

#[test]
fn window_does_not_expose_node_members() {
    // WHATWG: Window is an EventTarget but NOT a Node.
    // `window.nodeType` / `window.parentNode` / `window.textContent`
    // must all be `undefined` — they live on `Node.prototype` which
    // is NOT in Window's prototype chain.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    for expr in [
        "typeof window.nodeType",
        "typeof window.parentNode",
        "typeof window.parentElement",
        "typeof window.textContent",
        "typeof window.firstChild",
        "typeof window.appendChild",
    ] {
        let v = vm.eval(&format!("{expr};")).unwrap();
        let JsValue::String(sid) = v else {
            panic!("{expr}: unexpected {v:?}");
        };
        assert_eq!(
            vm.get_string(sid),
            "undefined",
            "{expr} must be undefined on Window"
        );
    }
    // But window.addEventListener — an EventTarget method — is still present.
    let v = vm.eval("typeof window.addEventListener;").unwrap();
    let JsValue::String(sid) = v else { panic!() };
    assert_eq!(vm.get_string(sid), "function");

    vm.unbind();
}

#[test]
fn text_parent_element_returns_parent_element() {
    // `parentElement` is a Node member (WHATWG §4.4), not Element-
    // specific — so a Text wrapper must expose it and return the
    // parent element when its parent is one.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, body, _p, _div, _span, raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let raw_wrapper = vm.inner.create_element_wrapper(raw);
    let body_wrapper = vm.inner.create_element_wrapper(body);
    vm.set_global("_raw", JsValue::Object(raw_wrapper));
    vm.set_global("_body", JsValue::Object(body_wrapper));
    assert!(matches!(
        vm.eval("_raw.parentElement === _body;").unwrap(),
        JsValue::Boolean(true)
    ));

    vm.unbind();
}

#[test]
fn text_wrapper_sees_node_members() {
    // Text nodes chain through `Node.prototype`, so Node-level
    // accessors and methods must all resolve — `firstChild` returns
    // null, `appendChild` exists, `textContent` returns own data.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let raw_wrapper = vm.inner.create_element_wrapper(raw);
    vm.set_global("_raw", JsValue::Object(raw_wrapper));

    // firstChild / lastChild / childNodes on Text — exist, return
    // null / empty.
    assert!(matches!(
        vm.eval("_raw.firstChild;").unwrap(),
        JsValue::Null
    ));
    assert!(matches!(vm.eval("_raw.lastChild;").unwrap(), JsValue::Null));
    assert!(matches!(
        vm.eval("_raw.childNodes.length;").unwrap(),
        JsValue::Number(n) if n.abs() < f64::EPSILON
    ));
    // hasChildNodes → false on a text node.
    assert!(matches!(
        vm.eval("_raw.hasChildNodes();").unwrap(),
        JsValue::Boolean(false)
    ));
    // appendChild exists as a function.
    let v = vm.eval("typeof _raw.appendChild;").unwrap();
    let JsValue::String(sid) = v else { panic!() };
    assert_eq!(vm.get_string(sid), "function");

    vm.unbind();
}

#[test]
fn append_child_rejects_window_argument() {
    // Window is an EventTarget but not a Node in WHATWG — passing
    // it as the child argument to a mutation method must throw the
    // same "parameter is not of type 'Node'" TypeError as any other
    // non-Node.  Covers appendChild / removeChild / insertBefore /
    // replaceChild (they share `require_node_arg`).
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    for expr in [
        "document.getElementById('root').appendChild(window);",
        "document.getElementById('root').removeChild(window);",
        "document.getElementById('root').insertBefore(window, null);",
        "document.getElementById('root').replaceChild(window, \
             document.getElementById('root').firstChild);",
    ] {
        assert!(
            vm.eval(expr).is_err(),
            "{expr} must throw TypeError when passed Window"
        );
    }

    vm.unbind();
}

#[test]
fn shadow_host_hides_shadow_root_from_light_tree_traversal() {
    // A host with an attached shadow root and one light-DOM child
    // must expose ONLY the light child via `firstChild` /
    // `lastChild` / `childNodes` / `hasChildNodes()`.  The shadow
    // root is internal and must not leak through any of these
    // accessors.
    use elidex_ecs::ShadowRootMode;
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, div, _span, _raw, _com) = build_element_fixture(&mut dom);
    // div currently has one child (span).  Attach a shadow root
    // and also place a node inside it.
    let shadow_root = dom
        .attach_shadow(div, ShadowRootMode::Open)
        .expect("attach_shadow");
    let shadow_el = dom.create_element("article", Attributes::default());
    assert!(dom.append_child(shadow_root, shadow_el));

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let div_wrapper = vm.inner.create_element_wrapper(div);
    vm.set_global("_div", JsValue::Object(div_wrapper));

    // firstChild / lastChild resolve to the span, not the shadow root.
    let v = vm.eval("_div.firstChild.tagName;").unwrap();
    let JsValue::String(sid) = v else { panic!() };
    assert_eq!(vm.get_string(sid), "SPAN");
    let v = vm.eval("_div.lastChild.tagName;").unwrap();
    let JsValue::String(sid) = v else { panic!() };
    assert_eq!(vm.get_string(sid), "SPAN");

    // childNodes / childElementCount count the span only.
    assert!(matches!(
        vm.eval("_div.childNodes.length;").unwrap(),
        JsValue::Number(n) if (n - 1.0).abs() < f64::EPSILON
    ));
    assert!(matches!(
        vm.eval("_div.hasChildNodes();").unwrap(),
        JsValue::Boolean(true)
    ));

    vm.unbind();
}

#[test]
fn shadow_host_has_child_nodes_false_when_only_shadow_root() {
    // A host whose ONLY child is a shadow root reports
    // `hasChildNodes() === false` (light-tree empty), matching the
    // browser where `childNodes.length` is also 0.
    use elidex_ecs::ShadowRootMode;
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    let body = dom.create_element("body", Attributes::default());
    let host = dom.create_element("section", {
        let mut a = Attributes::default();
        a.set("id", "shadow-host");
        a
    });
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, body));
    assert!(dom.append_child(body, host));
    let _ = dom
        .attach_shadow(host, ShadowRootMode::Open)
        .expect("attach_shadow");

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    assert!(matches!(
        vm.eval("document.getElementById('shadow-host').hasChildNodes();")
            .unwrap(),
        JsValue::Boolean(false)
    ));
    assert!(matches!(
        vm.eval("document.getElementById('shadow-host').firstChild;")
            .unwrap(),
        JsValue::Null
    ));
    assert!(matches!(
        vm.eval("document.getElementById('shadow-host').childNodes.length;")
            .unwrap(),
        JsValue::Number(n) if n.abs() < f64::EPSILON
    ));

    vm.unbind();
}

#[test]
fn contains_stops_at_shadow_boundary() {
    // `host.contains(nodeInsideShadow)` must be false — the shadow
    // root is NOT a light-tree descendant of its host, even though
    // elidex stores it as a child for convenience.
    use elidex_ecs::ShadowRootMode;
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, div, _span, _raw, _com) = build_element_fixture(&mut dom);
    let shadow_root = dom
        .attach_shadow(div, ShadowRootMode::Open)
        .expect("attach_shadow");
    let shadow_el = dom.create_element("article", Attributes::default());
    assert!(dom.append_child(shadow_root, shadow_el));

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let div_wrapper = vm.inner.create_element_wrapper(div);
    let shadow_el_wrapper = vm.inner.create_element_wrapper(shadow_el);
    vm.set_global("_div", JsValue::Object(div_wrapper));
    vm.set_global("_shadow_el", JsValue::Object(shadow_el_wrapper));

    assert!(matches!(
        vm.eval("_div.contains(_shadow_el);").unwrap(),
        JsValue::Boolean(false)
    ));

    vm.unbind();
}

#[test]
fn closest_stops_at_shadow_boundary() {
    // When walking ancestors from inside a shadow tree, `closest`
    // must stop at the shadow root (approximated by "non-Element
    // parent") and not return a match on the shadow host.
    use elidex_ecs::ShadowRootMode;
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, div, _span, _raw, _com) = build_element_fixture(&mut dom);

    // Give div a shadow root, put a child inside the shadow tree.
    let shadow_root = dom
        .attach_shadow(div, ShadowRootMode::Open)
        .expect("attach_shadow");
    let inner = dom.create_element("article", Attributes::default());
    assert!(dom.append_child(shadow_root, inner));

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    // Set div.id = "host" so it would be matched by `#host` — if the
    // walk crossed the shadow boundary, `inner.closest('#host')`
    // would return `div`.  Correct behaviour: return null.
    let div_wrapper = vm.inner.create_element_wrapper(div);
    vm.set_global("_div", JsValue::Object(div_wrapper));
    vm.eval("_div.setAttribute('id', 'host');").unwrap();
    let inner_wrapper = vm.inner.create_element_wrapper(inner);
    vm.set_global("_inner", JsValue::Object(inner_wrapper));

    // Matching its own tag succeeds (self-match).
    let v = vm.eval("_inner.closest('article') === _inner;").unwrap();
    assert!(matches!(v, JsValue::Boolean(true)));
    // But walking to the host is blocked.
    let v = vm.eval("_inner.closest('#host');").unwrap();
    assert!(
        matches!(v, JsValue::Null),
        "closest() must not cross the shadow boundary; got {v:?}"
    );

    vm.unbind();
}

// ---------------------------------------------------------------------------
// Element-only members should NOT surface on Text nodes
// ---------------------------------------------------------------------------

#[test]
fn text_wrapper_does_not_expose_element_placeholder_marker() {
    // Invariant: members installed on `Element.prototype` must be
    // `undefined` on Text wrappers — the Text branch skips
    // `Element.prototype` and inherits from `Node.prototype` (and
    // then `EventTarget.prototype`).  `firstElementChild` is an
    // Element-only accessor, so `typeof` must be `undefined` on a
    // Text node.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let raw_wrapper = vm.inner.create_element_wrapper(raw);
    vm.set_global("_raw", JsValue::Object(raw_wrapper));
    let t = vm.eval("typeof _raw.firstElementChild;").unwrap();
    let JsValue::String(sid) = t else { panic!() };
    assert_eq!(
        vm.get_string(sid),
        "undefined",
        "firstElementChild must not resolve on Text wrappers"
    );

    vm.unbind();
}
