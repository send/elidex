//! PR4f C5: `Element.prototype.getElementsByTagName` /
//! `getElementsByClassName` — WHATWG §4.2.6 subtree form.

#![cfg(feature = "engine")]

use elidex_ecs::{Attributes, EcsDom};
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;

/// Layout: doc > html > body > div#root > (span/span/p/div#inner > span/span)
/// + sibling section > div > span (ignored by Element subtree queries)
fn build_fixture(dom: &mut EcsDom) -> elidex_ecs::Entity {
    let doc = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    let body = dom.create_element("body", Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, body));

    let root = dom.create_element("div", {
        let mut a = Attributes::default();
        a.set("id", "root");
        a
    });
    assert!(dom.append_child(body, root));
    let s1 = dom.create_element("span", Attributes::default());
    let s2 = dom.create_element("span", {
        let mut a = Attributes::default();
        a.set("class", "x y");
        a
    });
    let p = dom.create_element("p", {
        let mut a = Attributes::default();
        a.set("class", "x");
        a
    });
    let inner = dom.create_element("div", {
        let mut a = Attributes::default();
        a.set("id", "inner");
        a.set("class", "y x z");
        a
    });
    let inner_s1 = dom.create_element("span", {
        let mut a = Attributes::default();
        a.set("class", "x");
        a
    });
    let inner_s2 = dom.create_element("span", Attributes::default());
    assert!(dom.append_child(root, s1));
    assert!(dom.append_child(root, s2));
    assert!(dom.append_child(root, p));
    assert!(dom.append_child(root, inner));
    assert!(dom.append_child(inner, inner_s1));
    assert!(dom.append_child(inner, inner_s2));

    // Sibling subtree that subtree-scoped queries on #root must ignore.
    let section = dom.create_element("section", Attributes::default());
    let stray_div = dom.create_element("div", Attributes::default());
    let stray_span = dom.create_element("span", Attributes::default());
    assert!(dom.append_child(body, section));
    assert!(dom.append_child(section, stray_div));
    assert!(dom.append_child(stray_div, stray_span));

    doc
}

fn run(script: &str) -> String {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let result = vm.eval(script).unwrap();
    let JsValue::String(sid) = result else {
        panic!("expected string, got {result:?}");
    };
    let out = vm.inner.strings.get_utf8(sid);
    vm.unbind();
    out
}

fn run_number(script: &str) -> f64 {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let result = vm.eval(script).unwrap();
    let JsValue::Number(n) = result else {
        panic!("expected number, got {result:?}");
    };
    vm.unbind();
    n
}

#[test]
fn element_get_elements_by_tag_name_star_excludes_receiver() {
    // #root has 4 direct + 2 nested = 6 descendant Elements; the
    // receiver itself is excluded.
    let n = run_number(
        "var root = document.getElementById('root');\
         root.getElementsByTagName('*').length;",
    );
    assert_eq!(n, 6.0);
}

#[test]
fn element_get_elements_by_tag_name_specific() {
    let n = run_number("document.getElementById('root').getElementsByTagName('span').length;");
    // span × 2 direct + 2 nested = 4
    assert_eq!(n, 4.0);
}

#[test]
fn element_get_elements_by_tag_name_is_ascii_case_insensitive() {
    let n = run_number("document.getElementById('root').getElementsByTagName('SPAN').length;");
    assert_eq!(n, 4.0);
}

#[test]
fn element_get_elements_by_tag_name_excludes_sibling_subtree() {
    // `stray_span` under <section> must NOT be in root's result set.
    let out = run("var root = document.getElementById('root');\
         var body = document.body;\
         var rootSpans = root.getElementsByTagName('span').length;\
         var bodySpans = body.getElementsByTagName('span').length;\
         rootSpans + ':' + bodySpans;");
    // body.getElementsByTagName('span') picks up all 5 spans
    // (4 inside #root + 1 inside <section>), root picks up only 4.
    assert_eq!(out, "4:5");
}

#[test]
fn element_get_elements_by_class_name_all_tokens_required() {
    let n = run_number("document.getElementById('root').getElementsByClassName('x y').length;");
    // span#2 (class="x y") + div#inner (class="y x z") → 2 elements
    assert_eq!(n, 2.0);
}

#[test]
fn element_get_elements_by_class_name_empty_token_set_is_empty() {
    let n = run_number("document.getElementById('root').getElementsByClassName('   ').length;");
    assert_eq!(n, 0.0);
}

#[test]
fn element_get_elements_by_class_name_token_order_independent() {
    let a = run_number("document.getElementById('root').getElementsByClassName('x y').length;");
    let b = run_number("document.getElementById('root').getElementsByClassName('y x').length;");
    assert_eq!(a, b);
    assert_eq!(a, 2.0);
}

#[test]
fn element_get_elements_by_class_name_single_token() {
    let n = run_number("document.getElementById('root').getElementsByClassName('x').length;");
    // Elements with an 'x' class token under #root (receiver excluded):
    //   span#2 ("x y"), p ("x"), div#inner ("y x z"), inner_s1 ("x") = 4
    assert_eq!(n, 4.0);
}

#[test]
fn element_get_elements_by_tag_name_brand_check_precedes_tostring() {
    // Copilot R3 F8 lock-in: WebIDL brand-check runs BEFORE argument
    // ToString.  Calling the native on a plain object via
    // `Function.prototype.call` must NOT trigger the user-supplied
    // toString on the argument — the silent-no-op-for-invalid-receiver
    // policy means no user code can observe that we even got here.
    //
    // elidex does not expose a global `Element` constructor yet
    // (PR5b), so we reach the method via `Object.getPrototypeOf` on
    // an actual element wrapper.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let result = vm
        .eval(
            "var called = false; \
             var spy = { toString: function() { called = true; return '*'; } }; \
             var proto = Object.getPrototypeOf(document.createElement('div')); \
             var getByTag = proto.getElementsByTagName; \
             var r = getByTag.call({}, spy); \
             (r.length === 0) + '|' + called;",
        )
        .unwrap();
    let JsValue::String(sid) = result else {
        panic!("expected string, got {result:?}")
    };
    assert_eq!(vm.inner.strings.get_utf8(sid), "true|false");
    vm.unbind();
}

#[test]
fn element_get_elements_by_class_name_brand_check_precedes_tostring() {
    // Copilot R3 F9 lock-in: same as above for getElementsByClassName.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let result = vm
        .eval(
            "var called = false; \
             var spy = { toString: function() { called = true; return 'x'; } }; \
             var proto = Object.getPrototypeOf(document.createElement('div')); \
             var getByClass = proto.getElementsByClassName; \
             var r = getByClass.call({}, spy); \
             (r.length === 0) + '|' + called;",
        )
        .unwrap();
    let JsValue::String(sid) = result else {
        panic!("expected string, got {result:?}")
    };
    assert_eq!(vm.inner.strings.get_utf8(sid), "true|false");
    vm.unbind();
}

#[test]
fn element_subtree_query_does_not_include_receiver() {
    // `#inner` has class "y x z"; `#inner.getElementsByClassName('y')`
    // must NOT include `#inner` itself — per spec "every descendant".
    let n = run_number("document.getElementById('inner').getElementsByClassName('y').length;");
    assert_eq!(n, 0.0);
}
