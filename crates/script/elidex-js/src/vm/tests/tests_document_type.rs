//! PR4f C7: `DocumentType.prototype` (name / publicId / systemId) +
//! Document collections (forms / images / links) + cookie / referrer
//! stubs.

#![cfg(feature = "engine")]

use elidex_ecs::{Attributes, EcsDom};
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;

fn build_doctype_fixture(dom: &mut EcsDom, public_id: &str, system_id: &str) -> elidex_ecs::Entity {
    let doc = dom.create_document_root();
    let doctype = dom.create_document_type("html", public_id, system_id);
    let html = dom.create_element("html", Attributes::default());
    assert!(dom.append_child(doc, doctype));
    assert!(dom.append_child(doc, html));
    doc
}

fn build_collections_fixture(dom: &mut EcsDom) -> elidex_ecs::Entity {
    let doc = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    let body = dom.create_element("body", Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, body));
    // 2 forms
    for _ in 0..2 {
        let form = dom.create_element("form", Attributes::default());
        assert!(dom.append_child(body, form));
    }
    // 3 images
    for _ in 0..3 {
        let img = dom.create_element("img", Attributes::default());
        assert!(dom.append_child(body, img));
    }
    // 2 <a href> + 1 <a> without href (excluded) + 1 <area href>
    let a_href = dom.create_element("a", {
        let mut a = Attributes::default();
        a.set("href", "/one");
        a
    });
    let a_nohref = dom.create_element("a", Attributes::default());
    let a_href2 = dom.create_element("a", {
        let mut a = Attributes::default();
        a.set("href", "/two");
        a
    });
    let area_href = dom.create_element("area", {
        let mut a = Attributes::default();
        a.set("href", "/three");
        a
    });
    for e in [a_href, a_nohref, a_href2, area_href] {
        assert!(dom.append_child(body, e));
    }
    doc
}

fn run_with<F: FnOnce(&mut EcsDom) -> elidex_ecs::Entity>(script: &str, fixture: F) -> String {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = fixture(&mut dom);

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

// --- DocumentType.prototype -----------------------------------------

#[test]
fn doctype_name_is_exposed() {
    let out = run_with("document.doctype.name;", |dom| {
        build_doctype_fixture(dom, "", "")
    });
    assert_eq!(out, "html");
}

#[test]
fn doctype_public_id_empty_when_absent() {
    // Per WHATWG §4.7 `publicId` is a non-nullable DOMString — the
    // empty string is the well-defined absence marker.
    let out = run_with(
        "typeof document.doctype.publicId + ':' + document.doctype.publicId;",
        |dom| build_doctype_fixture(dom, "", ""),
    );
    assert_eq!(out, "string:");
}

#[test]
fn doctype_public_and_system_id_exposed_when_set() {
    let out = run_with(
        "document.doctype.publicId + '|' + document.doctype.systemId;",
        |dom| {
            build_doctype_fixture(
                dom,
                "-//W3C//DTD HTML 4.01//EN",
                "http://www.w3.org/TR/html4/strict.dtd",
            )
        },
    );
    assert_eq!(
        out,
        "-//W3C//DTD HTML 4.01//EN|http://www.w3.org/TR/html4/strict.dtd"
    );
}

#[test]
fn doctype_name_on_wrong_host_object_throws() {
    // Brand check: calling the getter on a HostObject that is NOT a
    // DocumentType — e.g. the <html> element — throws "Illegal
    // invocation" per the WebIDL brand-check contract.  Plain non-
    // HostObject receivers (`{}`) fall under the elidex silent no-op
    // policy shared by the other Node accessors (`entity_from_this`
    // returns None).
    let out = run_with(
        "var getName = Object.getOwnPropertyDescriptor( \
             Object.getPrototypeOf(document.doctype), 'name').get; \
         try { getName.call(document.documentElement); 'no-throw'; } \
         catch (e) { (e && e.message && e.message.indexOf('Illegal') >= 0) ? 'ok' : 'bad'; }",
        |dom| build_doctype_fixture(dom, "", ""),
    );
    assert_eq!(out, "ok");
}

// --- Document forms / images / links --------------------------------

#[test]
fn document_forms_snapshot_count() {
    let out = run_with("document.forms.length + '';", build_collections_fixture);
    assert_eq!(out, "2");
}

#[test]
fn document_images_snapshot_count() {
    let out = run_with("document.images.length + '';", build_collections_fixture);
    assert_eq!(out, "3");
}

#[test]
fn document_links_include_a_and_area_with_href_only() {
    // Fixture has 2 <a href> + 1 <a> without href + 1 <area href> = 3.
    let out = run_with("document.links.length + '';", build_collections_fixture);
    assert_eq!(out, "3");
}

#[test]
fn document_forms_returns_new_collection_each_call() {
    // Per-access allocation — `document.forms` is live, but the
    // wrapper identity is still fresh on each read (the live
    // filter state is shared via `live_collection_states` on the
    // same `doc` Entity, but `alloc_collection` allocates a new
    // `ObjectKind::HtmlCollection` wrapper every time).  Matches
    // the no-cache design called out in PR5b §C3.
    let out = run_with(
        "document.forms === document.forms ? 'same' : 'new';",
        build_collections_fixture,
    );
    assert_eq!(out, "new");
}

// --- cookie / referrer stubs ----------------------------------------

#[test]
fn cookie_getter_returns_empty_string() {
    let out = run_with("typeof document.cookie + ':' + document.cookie;", |dom| {
        dom.create_document_root()
    });
    assert_eq!(out, "string:");
}

#[test]
fn cookie_setter_is_noop() {
    let out = run_with("document.cookie = 'k=v'; document.cookie;", |dom| {
        dom.create_document_root()
    });
    // Stub behaviour: writes are silently dropped.
    assert_eq!(out, "");
}

#[test]
fn referrer_getter_returns_empty_string() {
    let out = run_with("document.referrer;", |dom| dom.create_document_root());
    assert_eq!(out, "");
}
