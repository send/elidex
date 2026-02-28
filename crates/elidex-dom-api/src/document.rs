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

/// Find the first element matching any selector under `root` (pre-order DFS).
fn find_first_match(root: Entity, selectors: &[Selector], dom: &EcsDom) -> Option<Entity> {
    let mut stack = Vec::new();
    // Push children in reverse for correct DFS order.
    let children = dom.children(root);
    for child in children.into_iter().rev() {
        stack.push(child);
    }
    while let Some(entity) = stack.pop() {
        if dom.world().get::<&TagType>(entity).is_ok()
            && selectors.iter().any(|sel| sel.matches(entity, dom))
        {
            return Some(entity);
        }
        let children = dom.children(entity);
        for child in children.into_iter().rev() {
            stack.push(child);
        }
    }
    None
}

/// Find all elements matching any selector under `root` (pre-order DFS).
fn find_all_matches(root: Entity, selectors: &[Selector], dom: &EcsDom) -> Vec<Entity> {
    let mut result = Vec::new();
    let mut stack = Vec::new();
    let children = dom.children(root);
    for child in children.into_iter().rev() {
        stack.push(child);
    }
    while let Some(entity) = stack.pop() {
        if dom.world().get::<&TagType>(entity).is_ok()
            && selectors.iter().any(|sel| sel.matches(entity, dom))
        {
            result.push(entity);
        }
        let children = dom.children(entity);
        for child in children.into_iter().rev() {
            stack.push(child);
        }
    }
    result
}

/// Find the first element with a matching `id` attribute under `root`.
fn find_by_id(root: Entity, id: &str, dom: &EcsDom) -> Option<Entity> {
    let mut stack = Vec::new();
    let children = dom.children(root);
    for child in children.into_iter().rev() {
        stack.push(child);
    }
    while let Some(entity) = stack.pop() {
        if let Ok(attrs) = dom.world().get::<&Attributes>(entity) {
            if attrs.get("id") == Some(id) {
                return Some(entity);
            }
        }
        let children = dom.children(entity);
        for child in children.into_iter().rev() {
            stack.push(child);
        }
    }
    None
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
            &[JsValue::String("[invalid".into())],
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
}
