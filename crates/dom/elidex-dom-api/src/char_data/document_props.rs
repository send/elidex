//! Additional `Document` property handlers.

use elidex_ecs::{Attributes, EcsDom, Entity, TagType, TextContent};
use elidex_plugin::JsValue;
use elidex_script_session::{ComponentKind, DomApiError, DomApiHandler, SessionCore};

use crate::util::require_string_arg;

// ===========================================================================
// Document property handlers
// ===========================================================================

/// `document.URL` getter — returns `"about:blank"` for now.
pub struct GetDocumentUrl;

impl DomApiHandler for GetDocumentUrl {
    fn method_name(&self) -> &str {
        "document.URL.get"
    }

    fn invoke(
        &self,
        _this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        _dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        Ok(JsValue::String("about:blank".into()))
    }
}

/// `document.readyState` getter.
pub struct GetReadyState;

impl DomApiHandler for GetReadyState {
    fn method_name(&self) -> &str {
        "document.readyState.get"
    }

    fn invoke(
        &self,
        _this: Entity,
        _args: &[JsValue],
        session: &mut SessionCore,
        _dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        Ok(JsValue::String(
            session.document_ready_state.as_str().into(),
        ))
    }
}

/// `document.compatMode` getter — returns `"CSS1Compat"` (standards mode).
pub struct GetCompatMode;

impl DomApiHandler for GetCompatMode {
    fn method_name(&self) -> &str {
        "document.compatMode.get"
    }

    fn invoke(
        &self,
        _this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        _dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        Ok(JsValue::String("CSS1Compat".into()))
    }
}

/// `document.characterSet` getter — returns `"UTF-8"`.
pub struct GetCharacterSet;

impl DomApiHandler for GetCharacterSet {
    fn method_name(&self) -> &str {
        "document.characterSet.get"
    }

    fn invoke(
        &self,
        _this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        _dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        Ok(JsValue::String("UTF-8".into()))
    }
}

/// Find the first child element of `parent` whose tag matches
/// `tag_name` ASCII case-insensitively.  Mirrors
/// [`EcsDom::first_child_with_tag`] — HTML elements are identified
/// by their localName per WHATWG, but `TagType` stores the raw
/// (parser- or API-supplied) tag string which may not yet be
/// normalised, so accessor lookups must compare without regard to
/// case.
fn find_child_element(dom: &EcsDom, parent: Entity, tag_name: &str) -> Option<Entity> {
    dom.first_child_with_tag(parent, tag_name)
}

/// Find the first child of `html` that is a `<body>` or `<frameset>`
/// element, in document order, ASCII case-insensitively.
///
/// Shared between `GetBody::invoke` (this module) and
/// `vm/host/document.rs::native_document_get_active_element` (the
/// VM-side focus-fallback walker, currently deferred to slot
/// `#11-focus-management-hoist`).  Centralising the body-or-frameset
/// match here keeps the two accessors locked together — PR #156 R6
/// already drifted them once when activeElement carried a private
/// `first_child_with_tag("body").or_else(... "frameset")` two-pass
/// fallback that lost document order vs `GetBody`'s single-walk
/// scan.
#[must_use]
pub fn first_body_or_frameset_child(dom: &EcsDom, html: Entity) -> Option<Entity> {
    dom.children_iter(html).find(|child| {
        dom.world().get::<&TagType>(*child).ok().is_some_and(|t| {
            t.0.eq_ignore_ascii_case("body") || t.0.eq_ignore_ascii_case("frameset")
        })
    })
}

/// `document.documentElement` getter — first Element child of the document.
pub struct GetDocumentElement;

impl DomApiHandler for GetDocumentElement {
    fn method_name(&self) -> &str {
        "document.documentElement.get"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        for child in dom.children_iter(this) {
            if dom.world().get::<&TagType>(child).is_ok() {
                let obj_ref = session.get_or_create_wrapper(child, ComponentKind::Element);
                return Ok(JsValue::ObjectRef(obj_ref.to_raw()));
            }
        }
        Ok(JsValue::Null)
    }
}

/// `document.head` getter — finds `<html>` child, then `<head>` child.
pub struct GetHead;

impl DomApiHandler for GetHead {
    fn method_name(&self) -> &str {
        "document.head.get"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let Some(html) = find_child_element(dom, this, "html") else {
            return Ok(JsValue::Null);
        };
        let Some(head) = find_child_element(dom, html, "head") else {
            return Ok(JsValue::Null);
        };
        let obj_ref = session.get_or_create_wrapper(head, ComponentKind::Element);
        Ok(JsValue::ObjectRef(obj_ref.to_raw()))
    }
}

/// `document.body` getter — finds `<html>` child, then first `<body>` or `<frameset>` child.
pub struct GetBody;

impl DomApiHandler for GetBody {
    fn method_name(&self) -> &str {
        "document.body.get"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let Some(html) = find_child_element(dom, this, "html") else {
            return Ok(JsValue::Null);
        };
        let Some(body) = first_body_or_frameset_child(dom, html) else {
            return Ok(JsValue::Null);
        };
        let obj_ref = session.get_or_create_wrapper(body, ComponentKind::Element);
        Ok(JsValue::ObjectRef(obj_ref.to_raw()))
    }
}

/// Collect text content from direct Text node children only (not descendants).
fn child_text_content(entity: Entity, dom: &EcsDom) -> String {
    let mut result = String::new();
    for child in dom.children_iter(entity) {
        if let Ok(tc) = dom.world().get::<&TextContent>(child) {
            result.push_str(&tc.0);
        }
    }
    result
}

/// `document.title` getter — finds `<title>` in `<head>`, strips and collapses whitespace.
pub struct GetTitle;

impl DomApiHandler for GetTitle {
    fn method_name(&self) -> &str {
        "document.title.get"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let Some(html) = find_child_element(dom, this, "html") else {
            return Ok(JsValue::String(String::new()));
        };
        let Some(head) = find_child_element(dom, html, "head") else {
            return Ok(JsValue::String(String::new()));
        };
        let Some(title_elem) = find_child_element(dom, head, "title") else {
            return Ok(JsValue::String(String::new()));
        };

        let raw = child_text_content(title_elem, dom);
        // WHATWG HTML §dom-document-title: "strip and collapse ASCII
        // whitespace" — only U+0009 / U+000A / U+000C / U+000D /
        // U+0020 collapse to a single SPACE.  Rust's `split_whitespace`
        // would also collapse NBSP / ideographic space / other Unicode
        // whitespace, which mangles localized titles (`is_ascii_whitespace`
        // matches the spec set exactly).
        let collapsed: String = raw
            .split(|c: char| c.is_ascii_whitespace())
            .filter(|seg| !seg.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        Ok(JsValue::String(collapsed))
    }
}

/// `document.title` setter — finds or creates `<title>` in `<head>`, sets text content.
pub struct SetTitle;

impl DomApiHandler for SetTitle {
    fn method_name(&self) -> &str {
        "document.title.set"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let title_text = require_string_arg(args, 0)?;

        let Some(html) = find_child_element(dom, this, "html") else {
            return Ok(JsValue::Undefined);
        };
        let Some(head) = find_child_element(dom, html, "head") else {
            return Ok(JsValue::Undefined);
        };

        let title_elem = if let Some(e) = find_child_element(dom, head, "title") {
            e
        } else {
            // Anchor the new <title> to the receiver Document (WHATWG
            // DOM §4.4 "node document") so a setter call on a cloned
            // doc puts the synthesised element under the clone, not
            // the bound document.
            let t = dom.create_element_with_owner("title", Attributes::default(), Some(this));
            let ok = dom.append_child(head, t);
            debug_assert!(ok, "append_child: head verified");
            t
        };

        // Remove existing children of <title>.
        let children: Vec<Entity> = dom.children_iter(title_elem).collect();
        for child in children {
            let ok = dom.remove_child(title_elem, child);
            debug_assert!(ok, "remove_child: child from children_iter");
        }

        // Add text node — same owner-anchoring contract as the
        // synthesised <title> above.
        if !title_text.is_empty() {
            let text_node = dom.create_text_with_owner(&title_text, Some(this));
            let ok = dom.append_child(title_elem, text_node);
            debug_assert!(ok, "append_child: title_elem verified");
        }

        Ok(JsValue::Undefined)
    }
}

/// `document.createDocumentFragment()`.
pub struct CreateDocumentFragment;

impl DomApiHandler for CreateDocumentFragment {
    fn method_name(&self) -> &str {
        "createDocumentFragment"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let entity = dom.create_document_fragment_with_owner(Some(this));
        let obj_ref = session.get_or_create_wrapper(entity, ComponentKind::DocumentFragment);
        Ok(JsValue::ObjectRef(obj_ref.to_raw()))
    }
}

/// `document.createComment(data)`.
pub struct CreateComment;

impl DomApiHandler for CreateComment {
    fn method_name(&self) -> &str {
        "createComment"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let data = require_string_arg(args, 0)?;
        let entity = dom.create_comment_with_owner(&data, Some(this));
        let obj_ref = session.get_or_create_wrapper(entity, ComponentKind::Comment);
        Ok(JsValue::ObjectRef(obj_ref.to_raw()))
    }
}
