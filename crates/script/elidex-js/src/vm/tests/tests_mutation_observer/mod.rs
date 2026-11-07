//! M4-12 #11-mutation-observer — `MutationObserver` thin VM binding tests.
//!
//! Phase C2 surface: constructor + brand check + 3 method stubs.
//! Phase C3 surface: `observe` / `disconnect` / `takeRecords` semantics.
//! Phase C4 surface: `mutation_record_to_js` + `Vm::deliver_mutation_records`.
//! Phase C5 surface: post-unbind tolerance + `Vm::unbind` cleanup.
//!
//! The original 1319-line single file was split phase-aligned to keep
//! each child under the 1000-line convention:
//!
//! - [`setup`] — C2 (constructor + brand) + C3 (observe / init parsing) +
//!   argument-validation edge cases.
//! - [`delivery`] — C4 (record delivery) + §G / §H + multi-observer +
//!   callback-exception isolation + takeRecords semantics.
//! - [`lifecycle`] — C5 (post-unbind tolerance + cleanup) + rebind.

#![cfg(feature = "engine")]

mod attr_node;
mod attributes;
mod char_data;
mod delivery;
mod direct_tree_ops;
mod integration;
mod lifecycle;
mod range_ops;
mod reflected;
mod select_options;
mod setup;
mod text_content;
mod transient;

use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;

pub(super) fn build_doc(dom: &mut EcsDom) -> elidex_ecs::Entity {
    let doc = dom.create_document_root();
    let html = dom.create_element("html", elidex_ecs::Attributes::default());
    let body = dom.create_element("body", elidex_ecs::Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, body));
    doc
}

pub(super) fn run(script: &str) -> String {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let result = vm.eval(script).unwrap();
    let JsValue::String(sid) = result else {
        panic!("expected string, got {result:?}")
    };
    let out = vm.inner.strings.get_utf8(sid);
    vm.unbind();
    out
}

pub(super) fn run_throws(script: &str) -> String {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let err = vm.eval(script).expect_err("expected an error");
    vm.unbind();
    format!("{err:?}")
}

/// Shared **pre-bind** prelude for [`setup_with_root`] /
/// [`setup_with_root_and_svg`]: build the doc, resolve `<body>`, create the
/// `<div>` root and append it, returning the still-unbound `(doc, body, root)`
/// entities. The bind + global wiring (and the SVG variant's extra sibling) are
/// the callers' job — the prelude can't bind because `setup_with_root_and_svg`
/// needs to mutate `dom` further (append the SVG) before the bind borrows it.
fn build_root_tree(
    dom: &mut EcsDom,
) -> (elidex_ecs::Entity, elidex_ecs::Entity, elidex_ecs::Entity) {
    let doc = build_doc(dom);
    let body = dom
        .first_child_with_tag(dom.first_child_with_tag(doc, "html").unwrap(), "body")
        .unwrap();
    // Created in `doc`: the Selection same-document check reads the node
    // document (C0b), and production cannot create an ownerless node.
    let root = dom.create_element_with_owner(
        "div",
        elidex_ecs::Attributes::default(),
        dom.node_document_for_new_nodes(doc),
    );
    assert!(dom.append_child(body, root));
    (doc, body, root)
}

/// Build a typical document tree with a `<div>` returned for
/// targeted mutations, and bind the VM.  Exposes the root `<div>`
/// element's JS wrapper as `globalThis.root` (the variable name
/// "root" is the JS-side identifier; the element itself has no
/// `id` attribute).
pub(super) fn setup_with_root(
    vm: &mut Vm,
    session: &mut SessionCore,
    dom: &mut EcsDom,
) -> (elidex_ecs::Entity, elidex_ecs::Entity) {
    let (doc, _body, root) = build_root_tree(dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(vm, session, dom, doc);
    }
    let wrapper = vm.inner.create_element_wrapper(root);
    vm.set_global("root", JsValue::Object(wrapper));
    (doc, root)
}

/// Like [`setup_with_root`], but ALSO appends an **SVG-namespaced** element
/// (carrying a case-preserved `viewBox` attribute) and exposes it as
/// `globalThis.svg`. SVG / MathML elements are normally parser-constructed
/// (JS `createElementNS` is not VM-wired — plan §7); building it here via
/// [`elidex_ecs::EcsDom::create_element_ns`] is the test-construction
/// equivalent (the resolver reads the `Namespace` component regardless of
/// whether the parser or `create_element_ns` attached it), so the
/// HTML-namespace-gated attribute-name resolver case-preserves `viewBox` on it.
pub(super) fn setup_with_root_and_svg(
    vm: &mut Vm,
    session: &mut SessionCore,
    dom: &mut EcsDom,
) -> (elidex_ecs::Entity, elidex_ecs::Entity, elidex_ecs::Entity) {
    let (doc, body, root) = build_root_tree(dom);

    let mut svg_attrs = elidex_ecs::Attributes::default();
    svg_attrs.set("viewBox", "0 0 10 10");
    let svg = dom.create_element_ns(
        "svg",
        elidex_ecs::Namespace::Svg,
        svg_attrs,
        elidex_ecs::NewNodeOwner::ownerless_fixture(),
    );
    assert!(dom.append_child(body, svg));

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(vm, session, dom, doc);
    }
    let root_wrapper = vm.inner.create_element_wrapper(root);
    vm.set_global("root", JsValue::Object(root_wrapper));
    let svg_wrapper = vm.inner.create_element_wrapper(svg);
    vm.set_global("svg", JsValue::Object(svg_wrapper));
    (doc, root, svg)
}

/// Shared body of the C0a T17 / T17b same-parent replace move tests (D-12):
/// builds `P = [A, B]`, observes `P`, runs `mutating_call` (a JS statement
/// that replaces `A` with `B` within `P`), and asserts the two records: r[0] the
/// adopt step 2 source removal of `B` with `A` still in place, r[1] the coalesced
/// replace record (step 10).
pub(super) fn assert_same_parent_replace_move_records(mutating_call: &str) {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(&format!(
        "globalThis.P = document.createElement('div'); \
         globalThis.A = document.createElement('a'); \
         globalThis.B = document.createElement('b'); \
         P.append(A, B); \
         var mo = new MutationObserver(function(){{}}); \
         mo.observe(P, {{childList:true}}); \
         {mutating_call}; \
         globalThis.r = mo.takeRecords();"
    ))
    .unwrap();

    assert_eq!(vm.eval("r.length").unwrap(), JsValue::Number(2.0));
    // r[0]: adopt step 2's removal of B, with A (old child) still in place.
    assert_eq!(
        vm.eval("r[0].removedNodes.length === 1 && r[0].removedNodes[0] === B && r[0].addedNodes.length === 0")
            .unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("r[0].previousSibling === A").unwrap(),
        JsValue::Boolean(true),
        "source-removal previousSibling is A (replace step 6 adopts before step 7 removes A)"
    );
    assert_eq!(
        vm.eval("r[0].nextSibling === null").unwrap(),
        JsValue::Boolean(true)
    );
    // r[1]: the coalesced replace record (step 10).
    assert_eq!(
        vm.eval("r[1].addedNodes.length === 1 && r[1].addedNodes[0] === B && r[1].removedNodes.length === 1 && r[1].removedNodes[0] === A")
            .unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("r[1].previousSibling === null && r[1].nextSibling === null")
            .unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
}
