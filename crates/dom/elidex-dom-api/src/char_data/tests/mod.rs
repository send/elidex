mod attr;
mod char_data_methods;
mod doctype;
mod document_props;

use super::*;
use elidex_ecs::{Attributes, EcsDom, Entity, NodeKind};
use elidex_plugin::JsValue;
use elidex_script_session::{DomApiHandler, SessionCore};

// -----------------------------------------------------------------------
// Setup helpers
// -----------------------------------------------------------------------

fn setup_text() -> (EcsDom, Entity, SessionCore) {
    let mut dom = EcsDom::new();
    let text = dom.create_text("Hello, world!");
    let session = SessionCore::new();
    (dom, text, session)
}

fn setup_comment() -> (EcsDom, Entity, SessionCore) {
    let mut dom = EcsDom::new();
    let comment = dom.create_comment("a comment");
    let session = SessionCore::new();
    (dom, comment, session)
}

fn setup_document() -> (EcsDom, Entity, SessionCore) {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let doctype = dom.create_document_type(
        "html",
        "-//W3C//DTD HTML 4.01//EN",
        "http://www.w3.org/TR/html4/strict.dtd",
    );
    let html = dom.create_element("html", Attributes::default());
    let head = dom.create_element("head", Attributes::default());
    let body = dom.create_element("body", Attributes::default());
    dom.append_child(doc, doctype);
    dom.append_child(doc, html);
    dom.append_child(html, head);
    dom.append_child(html, body);
    let session = SessionCore::new();
    (dom, doc, session)
}

/// Find the first child element of `parent` with tag matching `tag_name`.
fn find_child_element(dom: &EcsDom, parent: Entity, tag_name: &str) -> Option<Entity> {
    use elidex_ecs::TagType;
    for child in dom.children_iter(parent) {
        if let Ok(tag) = dom.world().get::<&TagType>(child) {
            if tag.0 == tag_name {
                return Some(child);
            }
        }
    }
    None
}

/// Walk document children to find the first entity with `NodeKind::DocumentType`.
fn find_doctype(dom: &EcsDom, doc: Entity) -> Option<Entity> {
    for child in dom.children_iter(doc) {
        if let Ok(nk) = dom.world().get::<&NodeKind>(child) {
            if *nk == NodeKind::DocumentType {
                return Some(child);
            }
        }
    }
    None
}
