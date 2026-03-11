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
        match find_first_match(this, &selectors, dom) {
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
    Ok(find_all_matches(root, &selectors, dom))
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
        match find_by_id(this, &id, dom) {
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
        _this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let tag = require_string_arg(args, 0)?;
        if !is_valid_tag_name(&tag) {
            return Err(DomApiError {
                kind: DomApiErrorKind::SyntaxError,
                message: format!("Invalid tag name: {tag}"),
            });
        }
        let entity = dom.create_element(tag.to_ascii_lowercase(), Attributes::default());
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
        _this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let text = require_string_arg(args, 0)?;
        let entity = dom.create_text(text);
        // Note: ComponentKind::Element is used for text nodes because
        // ComponentKind has no TextNode variant. The JS bridge treats both
        // element and text node wrappers uniformly (same property layout).
        let obj_ref = session.get_or_create_wrapper(entity, ComponentKind::Element);
        Ok(JsValue::ObjectRef(obj_ref.to_raw()))
    }
}

// ---------------------------------------------------------------------------
// DOM tree walk helpers
// ---------------------------------------------------------------------------

/// Pre-order DFS traversal starting from `root` (excluding root itself).
/// Calls `visitor` for each entity. The visitor returns `true` to continue, `false` to stop early.
fn traverse_pre_order(dom: &EcsDom, root: Entity, mut visitor: impl FnMut(Entity) -> bool) {
    let mut stack: Vec<Entity> = Vec::new();
    // Push children of root in reverse order for correct traversal order.
    let children: Vec<_> = dom.children_iter(root).collect();
    for child in children.into_iter().rev() {
        stack.push(child);
    }
    while let Some(entity) = stack.pop() {
        if !visitor(entity) {
            return;
        }
        let children: Vec<_> = dom.children_iter(entity).collect();
        for child in children.into_iter().rev() {
            stack.push(child);
        }
    }
}

/// Check that no selector uses shadow-scoped pseudos (`:host`, `::slotted()`).
///
/// These are invalid in `querySelector`/`querySelectorAll` per CSS Scoping §3.
fn reject_shadow_pseudos(selectors: &[Selector]) -> Result<(), DomApiError> {
    if selectors.iter().any(Selector::has_shadow_pseudo) {
        return Err(DomApiError {
            kind: DomApiErrorKind::SyntaxError,
            message: ":host and ::slotted() are not valid in querySelector".to_string(),
        });
    }
    Ok(())
}

/// Find the first element matching any selector under `root` (pre-order DFS).
fn find_first_match(root: Entity, selectors: &[Selector], dom: &EcsDom) -> Option<Entity> {
    let mut result = None;
    traverse_pre_order(dom, root, |entity| {
        if dom.world().get::<&TagType>(entity).is_ok()
            && selectors.iter().any(|sel| sel.matches(entity, dom))
        {
            result = Some(entity);
            false // stop
        } else {
            true // continue
        }
    });
    result
}

/// Find all elements matching any selector under `root` (pre-order DFS).
fn find_all_matches(root: Entity, selectors: &[Selector], dom: &EcsDom) -> Vec<Entity> {
    let mut result = Vec::new();
    traverse_pre_order(dom, root, |entity| {
        if dom.world().get::<&TagType>(entity).is_ok()
            && selectors.iter().any(|sel| sel.matches(entity, dom))
        {
            result.push(entity);
        }
        true // always continue
    });
    result
}

/// Find the first element with a matching `id` attribute under `root`.
fn find_by_id(root: Entity, id: &str, dom: &EcsDom) -> Option<Entity> {
    let mut result = None;
    traverse_pre_order(dom, root, |entity| {
        if let Ok(attrs) = dom.world().get::<&Attributes>(entity) {
            if attrs.get("id") == Some(id) {
                result = Some(entity);
                return false; // stop
            }
        }
        true // continue
    });
    result
}

#[cfg(test)]
#[allow(unused_must_use)]
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
}
