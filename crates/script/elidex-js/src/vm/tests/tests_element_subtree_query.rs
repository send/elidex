//! `Element.prototype.querySelector` / `querySelectorAll` tests —
//! subtree-scoped (WHATWG §4.2.6).  `this` is not a match candidate,
//! only its descendants.

#![cfg(feature = "engine")]

use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::test_helpers::{bind_vm, eval_num, eval_str};
use super::super::value::JsValue;
use super::super::Vm;

fn setup() -> (Vm, SessionCore, EcsDom, elidex_ecs::Entity) {
    let vm = Vm::new();
    let session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    (vm, session, dom, doc)
}

/// Build: `<section><div class="target"><span/></div><p class="target"/></section>`
fn build_fixture(vm: &mut Vm) {
    vm.eval(
        "globalThis.section = document.createElement('section');\n\
         globalThis.div = document.createElement('div');\n\
         div.setAttribute('class', 'target');\n\
         globalThis.span = document.createElement('span');\n\
         div.appendChild(span);\n\
         section.appendChild(div);\n\
         globalThis.p = document.createElement('p');\n\
         p.setAttribute('class', 'target');\n\
         section.appendChild(p);",
    )
    .unwrap();
}

#[test]
fn element_query_selector_matches_descendant() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    build_fixture(&mut vm);
    assert_eq!(
        eval_str(&mut vm, "section.querySelector('span').tagName;"),
        "SPAN"
    );
    vm.unbind();
}

#[test]
fn element_query_selector_excludes_self_even_if_it_matches() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    // The receiver itself has class 'target'; querySelector must still
    // skip it and look only at descendants.  Shallow subtree has no
    // match → returns null.
    vm.eval(
        "globalThis.div = document.createElement('div');\n\
         div.setAttribute('class', 'target');",
    )
    .unwrap();
    let v = vm.eval("div.querySelector('.target');").unwrap();
    assert!(matches!(v, JsValue::Null), "expected null, got {v:?}");
    vm.unbind();
}

#[test]
fn element_query_selector_null_when_no_match() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    build_fixture(&mut vm);
    let v = vm.eval("section.querySelector('article');").unwrap();
    assert!(matches!(v, JsValue::Null), "expected null, got {v:?}");
    vm.unbind();
}

#[test]
fn element_query_selector_all_empty_returns_zero_array() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    build_fixture(&mut vm);
    assert_eq!(
        eval_num(&mut vm, "section.querySelectorAll('article').length;"),
        0.0
    );
    vm.unbind();
}

#[test]
fn element_query_selector_all_collects_all() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    build_fixture(&mut vm);
    // .target matches div + p → 2 descendants of section.
    assert_eq!(
        eval_num(&mut vm, "section.querySelectorAll('.target').length;"),
        2.0
    );
    vm.unbind();
}

#[test]
fn element_query_selector_invalid_selector_throws() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval("globalThis.d = document.createElement('div');")
        .unwrap();
    let threw = vm
        .eval(
            "var err = null;\n\
             try { d.querySelector('!!!invalid'); } catch (e) { err = e; }\n\
             err !== null;",
        )
        .unwrap();
    assert!(matches!(threw, JsValue::Boolean(true)));
    vm.unbind();
}

#[test]
fn element_query_selector_relative_combinator_syntax() {
    // Regression: the relative `>` combinator at the start of a
    // selector has no `:scope` to attach to (we don't support
    // `:scope` rooting yet, PR5a).  `parse_dom_selector`
    // (engine-independent layer) must reject this at parse time
    // with `DOMException("SyntaxError")` per WHATWG DOM §4.7 — the
    // assertion below pins that exact outcome.  Will need to be
    // revisited alongside `:scope` (PR5a) — at which point the
    // expected outcome may shift to a successful parse with zero
    // matches.
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    build_fixture(&mut vm);
    // The relative `>` combinator is invalid without a `:scope`
    // prefix.  `parse_dom_selector` (engine-independent layer)
    // rejects it at parse time and throws `DOMException("SyntaxError")`
    // per WHATWG DOM §4.7.  Pinning the throw — `Ok(JsValue::Null)`
    // and any other error kind are regressions of either the
    // selector parser (silently parsing what it should reject) or
    // the error taxonomy (R8 of #11-arch-hoist-a migrated this path
    // off `VmError::syntax_error`).
    let result = vm.eval("section.querySelector('> .target');");
    let syntax_err_name = vm.inner.well_known.dom_exc_syntax_error;
    match result {
        Err(e)
            if matches!(
                e.kind,
                super::super::value::VmErrorKind::DomException { name } if name == syntax_err_name
            ) => {}
        Ok(other) => {
            panic!("expected DOMException(\"SyntaxError\"), got Ok({other:?})");
        }
        Err(e) => {
            panic!("expected DOMException(\"SyntaxError\"), got {e:?}");
        }
    }
    vm.unbind();
}
