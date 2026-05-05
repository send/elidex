//! Document-level DOM API handlers: querySelector, getElementById, createElement, createTextNode.

use elidex_css::{parse_selector_from_str, Selector};
use elidex_ecs::{Attributes, EcsDom, Entity, TagType};
use elidex_plugin::JsValue;
use elidex_script_session::{
    ComponentKind, DomApiError, DomApiErrorKind, DomApiHandler, SessionCore,
};

use crate::util::require_string_arg;

// ---------------------------------------------------------------------------
// querySelector
// ---------------------------------------------------------------------------

/// `document.querySelector(selector)` — returns the first matching element or null.
pub struct QuerySelector;

impl DomApiHandler for QuerySelector {
    fn method_name(&self) -> &str {
        "querySelector"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let selector_str = require_string_arg(args, 0)?;
        let selectors = parse_selector_from_str(&selector_str).map_err(|()| DomApiError {
            kind: DomApiErrorKind::SyntaxError,
            message: format!("Invalid selector: {selector_str}"),
        })?;
        reject_shadow_pseudos(&selectors)?;
        let mut matched = None;
        dom.traverse_descendants(this, |entity| {
            if dom.world().get::<&TagType>(entity).is_ok()
                && selectors.iter().any(|sel| sel.matches(entity, dom))
            {
                matched = Some(entity);
                false
            } else {
                true
            }
        });
        match matched {
            Some(entity) => {
                let obj_ref = session.get_or_create_wrapper(entity, ComponentKind::Element);
                Ok(JsValue::ObjectRef(obj_ref.to_raw()))
            }
            None => Ok(JsValue::Null),
        }
    }
}

// ---------------------------------------------------------------------------
// querySelectorAll (standalone helper — returns Vec<Entity>)
// ---------------------------------------------------------------------------

/// Find all elements matching any of the given selectors under `root`.
///
/// Returns entities in document order (pre-order DFS).
/// This is a standalone function (not a `DomApiHandler`) because it returns
/// multiple entities; the boa bridge converts the result to a JS array.
pub fn query_selector_all(
    root: Entity,
    selector_str: &str,
    dom: &EcsDom,
) -> Result<Vec<Entity>, DomApiError> {
    let selectors = parse_selector_from_str(selector_str).map_err(|()| DomApiError {
        kind: DomApiErrorKind::SyntaxError,
        message: format!("Invalid selector: {selector_str}"),
    })?;
    reject_shadow_pseudos(&selectors)?;
    let mut result = Vec::new();
    dom.traverse_descendants(root, |entity| {
        if dom.world().get::<&TagType>(entity).is_ok()
            && selectors.iter().any(|sel| sel.matches(entity, dom))
        {
            result.push(entity);
        }
        true
    });
    Ok(result)
}

// ---------------------------------------------------------------------------
// getElementById
// ---------------------------------------------------------------------------

/// `document.getElementById(id)` — returns the element with matching id or null.
pub struct GetElementById;

impl DomApiHandler for GetElementById {
    fn method_name(&self) -> &str {
        "getElementById"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let id = require_string_arg(args, 0)?;
        match dom.find_by_id(this, &id) {
            Some(entity) => {
                let obj_ref = session.get_or_create_wrapper(entity, ComponentKind::Element);
                Ok(JsValue::ObjectRef(obj_ref.to_raw()))
            }
            None => Ok(JsValue::Null),
        }
    }
}

// ---------------------------------------------------------------------------
// createElement
// ---------------------------------------------------------------------------

/// `document.createElement(tagName)` — creates a new element.
pub struct CreateElement;

/// Validates that a tag name contains only ASCII alphanumeric, `-`, `_`, and `.`.
fn is_valid_tag_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.')
}

impl DomApiHandler for CreateElement {
    fn method_name(&self) -> &str {
        "createElement"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let tag = require_string_arg(args, 0)?;
        if !is_valid_tag_name(&tag) {
            // WHATWG DOM §4.5.2 (`Document.createElement`): an invalid
            // local name throws `InvalidCharacterError`, NOT
            // `SyntaxError`.  `SyntaxError` is reserved for selector /
            // URL / CSS-OM parse failures (DOM §4.7 legacy code 12)
            // and would surface the wrong `DOMException.name` to JS
            // (`e.name === "InvalidCharacterError"` vs `"SyntaxError"`).
            return Err(DomApiError {
                kind: DomApiErrorKind::InvalidCharacterError,
                message: format!("Invalid tag name: {tag}"),
            });
        }
        // Anchor the new node's "node document" (WHATWG DOM §4.4) to
        // the receiver Document so `newEl.ownerDocument` reports the
        // creating document even before insertion — critical for
        // clones and iframes where the bound global and the receiver
        // differ.  Pre-VM dispatch this lived in
        // `vm/host/document.rs::native_document_create_element` as an
        // explicit `create_element_with_owner` call; the handler now
        // owns the spec-precise behaviour so both boa and VM paths
        // observe the same ownerDocument semantics.
        let entity = dom.create_element_with_owner(
            tag.to_ascii_lowercase(),
            Attributes::default(),
            Some(this),
        );
        let obj_ref = session.get_or_create_wrapper(entity, ComponentKind::Element);
        Ok(JsValue::ObjectRef(obj_ref.to_raw()))
    }
}

// ---------------------------------------------------------------------------
// createTextNode
// ---------------------------------------------------------------------------

/// `document.createTextNode(data)` — creates a new text node.
pub struct CreateTextNode;

impl DomApiHandler for CreateTextNode {
    fn method_name(&self) -> &str {
        "createTextNode"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let text = require_string_arg(args, 0)?;
        let entity = dom.create_text_with_owner(text, Some(this));
        let obj_ref = session.get_or_create_wrapper(entity, ComponentKind::TextNode);
        Ok(JsValue::ObjectRef(obj_ref.to_raw()))
    }
}

// ---------------------------------------------------------------------------
// DOM tree walk helpers
// ---------------------------------------------------------------------------

/// Check that no selector uses shadow-scoped pseudos (`:host`, `::slotted()`).
///
/// These are invalid in `querySelector` / `querySelectorAll` /
/// `Element.matches` / `Element.closest` per CSS Scoping §3 — the
/// pseudos are only valid inside a shadow tree's scoped style or
/// when the selector is being matched from a `ShadowRoot`.  Shared
/// across selector-accepting handlers in this crate (intentional
/// `pub(crate)` visibility — no `elidex-dom-api` external caller
/// has a reason to validate selector shape independently of a handler).
pub(crate) fn reject_shadow_pseudos(selectors: &[Selector]) -> Result<(), DomApiError> {
    if selectors.iter().any(Selector::has_shadow_pseudo) {
        return Err(DomApiError {
            kind: DomApiErrorKind::SyntaxError,
            // API-neutral wording — the helper is shared across
            // `querySelector` / `querySelectorAll` / `matches` /
            // `closest`, so naming a single API in the message
            // misleads callers of the others.
            message: ":host and ::slotted() are only valid inside a shadow tree".to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
#[allow(unused_must_use)] // Test setup calls dom.append_child() etc. without checking return values
mod tests {
    use super::*;

    fn setup_dom() -> (EcsDom, Entity) {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let html = dom.create_element("html", Attributes::default());
        let body = dom.create_element("body", Attributes::default());
        dom.append_child(doc, html);
        dom.append_child(html, body);

        let mut div_attrs = Attributes::default();
        div_attrs.set("id", "target");
        div_attrs.set("class", "box");
        let div = dom.create_element("div", div_attrs);
        dom.append_child(body, div);

        let span = dom.create_element("span", Attributes::default());
        dom.append_child(div, span);

        let p = dom.create_element("p", Attributes::default());
        dom.append_child(body, p);

        (dom, doc)
    }

    #[test]
    fn query_selector_by_tag() {
        let (mut dom, doc) = setup_dom();
        let mut session = SessionCore::new();
        let handler = QuerySelector;
        let result = handler
            .invoke(
                doc,
                &[JsValue::String("span".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert!(matches!(result, JsValue::ObjectRef(_)));
    }

    #[test]
    fn query_selector_by_id() {
        let (mut dom, doc) = setup_dom();
        let mut session = SessionCore::new();
        let handler = QuerySelector;
        let result = handler
            .invoke(
                doc,
                &[JsValue::String("#target".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert!(matches!(result, JsValue::ObjectRef(_)));
    }

    #[test]
    fn query_selector_no_match() {
        let (mut dom, doc) = setup_dom();
        let mut session = SessionCore::new();
        let handler = QuerySelector;
        let result = handler
            .invoke(
                doc,
                &[JsValue::String("article".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Null);
    }

    #[test]
    fn query_selector_invalid_selector() {
        let (mut dom, doc) = setup_dom();
        let mut session = SessionCore::new();
        let handler = QuerySelector;
        let result = handler.invoke(
            doc,
            &[JsValue::String(">>>".into())],
            &mut session,
            &mut dom,
        );
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind, DomApiErrorKind::SyntaxError);
    }

    #[test]
    fn query_selector_all_multiple() {
        let (dom, doc) = setup_dom();
        let matches = query_selector_all(doc, "div, p", &dom).unwrap();
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn get_element_by_id_found() {
        let (mut dom, doc) = setup_dom();
        let mut session = SessionCore::new();
        let handler = GetElementById;
        let result = handler
            .invoke(
                doc,
                &[JsValue::String("target".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert!(matches!(result, JsValue::ObjectRef(_)));
    }

    #[test]
    fn get_element_by_id_not_found() {
        let (mut dom, doc) = setup_dom();
        let mut session = SessionCore::new();
        let handler = GetElementById;
        let result = handler
            .invoke(
                doc,
                &[JsValue::String("nonexistent".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Null);
    }

    #[test]
    fn create_element_returns_ref() {
        let (mut dom, doc) = setup_dom();
        let mut session = SessionCore::new();
        let handler = CreateElement;
        let result = handler
            .invoke(
                doc,
                &[JsValue::String("section".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert!(matches!(result, JsValue::ObjectRef(_)));
    }

    #[test]
    fn create_text_node_returns_ref() {
        let (mut dom, doc) = setup_dom();
        let mut session = SessionCore::new();
        let handler = CreateTextNode;
        let result = handler
            .invoke(
                doc,
                &[JsValue::String("hello world".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert!(matches!(result, JsValue::ObjectRef(_)));
    }

    #[test]
    fn query_selector_missing_arg() {
        let (mut dom, doc) = setup_dom();
        let mut session = SessionCore::new();
        let handler = QuerySelector;
        let result = handler.invoke(doc, &[], &mut session, &mut dom);
        assert!(result.is_err());
    }

    // --- M2: Shadow pseudo rejection in querySelector ---

    #[test]
    fn query_selector_host_rejected() {
        let (mut dom, doc) = setup_dom();
        let mut session = SessionCore::new();
        let handler = QuerySelector;
        let result = handler.invoke(
            doc,
            &[JsValue::String(":host".into())],
            &mut session,
            &mut dom,
        );
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind, DomApiErrorKind::SyntaxError);
    }

    #[test]
    fn query_selector_slotted_rejected() {
        let (dom, doc) = setup_dom();
        let result = query_selector_all(doc, "::slotted(div)", &dom);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind, DomApiErrorKind::SyntaxError);
    }

    #[test]
    fn create_element_invalid_tag_name_throws_invalid_character_error() {
        // WHATWG DOM §4.5.2 (`Document.createElement`): an invalid
        // local name throws `InvalidCharacterError` — *not*
        // `SyntaxError`, which is reserved for selector / URL /
        // CSS-OM parse failures (DOM §4.7 legacy code 12).  A space
        // in the tag name is the simplest invalid-name input that
        // trips `is_valid_tag_name`.
        let (mut dom, doc) = setup_dom();
        let mut session = SessionCore::new();
        let handler = CreateElement;
        let result = handler.invoke(
            doc,
            &[JsValue::String("bad tag".into())],
            &mut session,
            &mut dom,
        );
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().kind,
            DomApiErrorKind::InvalidCharacterError
        );
    }
}
