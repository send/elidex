//! Tree navigation DOM API handlers: parentNode, firstChild, lastChild,
//! nextSibling, previousSibling, and their element-only counterparts,
//! plus node info accessors (tagName, nodeName, nodeType, nodeValue,
//! childElementCount, hasChildNodes).

use elidex_ecs::{CommentData, DocTypeData, EcsDom, Entity, NodeKind, TagType, TextContent};
use elidex_plugin::JsValue;
use elidex_script_session::{ComponentKind, DomApiError, DomApiHandler, SessionCore};

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

/// Convert an optional entity to a `JsValue::ObjectRef` (wrapped via the
/// identity map) or `JsValue::Null`.
fn entity_or_null(entity: Option<Entity>, session: &mut SessionCore, dom: &EcsDom) -> JsValue {
    match entity {
        Some(e) => {
            let kind = dom
                .node_kind(e)
                .map_or(ComponentKind::Element, ComponentKind::from_node_kind);
            let obj_ref = session.get_or_create_wrapper(e, kind);
            JsValue::ObjectRef(obj_ref.to_raw())
        }
        None => JsValue::Null,
    }
}

/// Walk a chain of nodes using `step_fn`, returning the first element found.
fn find_element_in_chain(
    start: Option<Entity>,
    dom: &EcsDom,
    step_fn: fn(&EcsDom, Entity) -> Option<Entity>,
) -> Option<Entity> {
    let mut cursor = start;
    while let Some(c) = cursor {
        if dom.is_element(c) {
            return Some(c);
        }
        cursor = step_fn(dom, c);
    }
    None
}

// ---------------------------------------------------------------------------
// parentNode
// ---------------------------------------------------------------------------

/// `node.parentNode` getter — returns the parent node or null.
pub struct GetParentNode;

impl DomApiHandler for GetParentNode {
    fn method_name(&self) -> &str {
        "parentNode.get"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        Ok(entity_or_null(dom.get_parent(this), session, dom))
    }
}

// ---------------------------------------------------------------------------
// parentElement
// ---------------------------------------------------------------------------

/// `node.parentElement` getter — returns the parent only if it is an element.
pub struct GetParentElement;

impl DomApiHandler for GetParentElement {
    fn method_name(&self) -> &str {
        "parentElement.get"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let parent = dom.get_parent(this).filter(|&p| dom.is_element(p));
        Ok(entity_or_null(parent, session, dom))
    }
}

// ---------------------------------------------------------------------------
// firstChild
// ---------------------------------------------------------------------------

/// `node.firstChild` getter.
pub struct GetFirstChild;

impl DomApiHandler for GetFirstChild {
    fn method_name(&self) -> &str {
        "firstChild.get"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        Ok(entity_or_null(dom.get_first_child(this), session, dom))
    }
}

// ---------------------------------------------------------------------------
// lastChild
// ---------------------------------------------------------------------------

/// `node.lastChild` getter.
pub struct GetLastChild;

impl DomApiHandler for GetLastChild {
    fn method_name(&self) -> &str {
        "lastChild.get"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        Ok(entity_or_null(dom.get_last_child(this), session, dom))
    }
}

// ---------------------------------------------------------------------------
// nextSibling
// ---------------------------------------------------------------------------

/// `node.nextSibling` getter.
pub struct GetNextSibling;

impl DomApiHandler for GetNextSibling {
    fn method_name(&self) -> &str {
        "nextSibling.get"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        Ok(entity_or_null(dom.get_next_sibling(this), session, dom))
    }
}

// ---------------------------------------------------------------------------
// previousSibling
// ---------------------------------------------------------------------------

/// `node.previousSibling` getter.
pub struct GetPrevSibling;

impl DomApiHandler for GetPrevSibling {
    fn method_name(&self) -> &str {
        "previousSibling.get"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        Ok(entity_or_null(dom.get_prev_sibling(this), session, dom))
    }
}

// ---------------------------------------------------------------------------
// firstElementChild
// ---------------------------------------------------------------------------

/// `element.firstElementChild` getter — first child that is an element.
pub struct GetFirstElementChild;

impl DomApiHandler for GetFirstElementChild {
    fn method_name(&self) -> &str {
        "firstElementChild.get"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let found = find_element_in_chain(dom.get_first_child(this), dom, EcsDom::get_next_sibling);
        Ok(entity_or_null(found, session, dom))
    }
}

// ---------------------------------------------------------------------------
// lastElementChild
// ---------------------------------------------------------------------------

/// `element.lastElementChild` getter — last child that is an element.
pub struct GetLastElementChild;

impl DomApiHandler for GetLastElementChild {
    fn method_name(&self) -> &str {
        "lastElementChild.get"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let found = find_element_in_chain(dom.get_last_child(this), dom, EcsDom::get_prev_sibling);
        Ok(entity_or_null(found, session, dom))
    }
}

// ---------------------------------------------------------------------------
// nextElementSibling
// ---------------------------------------------------------------------------

/// `element.nextElementSibling` getter — next sibling that is an element.
pub struct GetNextElementSibling;

impl DomApiHandler for GetNextElementSibling {
    fn method_name(&self) -> &str {
        "nextElementSibling.get"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let found =
            find_element_in_chain(dom.get_next_sibling(this), dom, EcsDom::get_next_sibling);
        Ok(entity_or_null(found, session, dom))
    }
}

// ---------------------------------------------------------------------------
// previousElementSibling
// ---------------------------------------------------------------------------

/// `element.previousElementSibling` getter — previous sibling that is an element.
pub struct GetPrevElementSibling;

impl DomApiHandler for GetPrevElementSibling {
    fn method_name(&self) -> &str {
        "previousElementSibling.get"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let found =
            find_element_in_chain(dom.get_prev_sibling(this), dom, EcsDom::get_prev_sibling);
        Ok(entity_or_null(found, session, dom))
    }
}

// ---------------------------------------------------------------------------
// tagName
// ---------------------------------------------------------------------------

/// `element.tagName` getter — returns the uppercase tag name.
pub struct GetTagName;

impl DomApiHandler for GetTagName {
    fn method_name(&self) -> &str {
        "tagName.get"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        if let Ok(tag) = dom.world().get::<&TagType>(this) {
            Ok(JsValue::String(tag.0.to_ascii_uppercase()))
        } else {
            Ok(JsValue::Null)
        }
    }
}

// ---------------------------------------------------------------------------
// nodeName
// ---------------------------------------------------------------------------

/// `node.nodeName` getter — returns the appropriate name per WHATWG DOM §4.4.
pub struct GetNodeName;

impl DomApiHandler for GetNodeName {
    fn method_name(&self) -> &str {
        "nodeName.get"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let kind = dom.node_kind(this);
        let name = match kind {
            Some(NodeKind::Element) => {
                if let Ok(tag) = dom.world().get::<&TagType>(this) {
                    tag.0.to_ascii_uppercase()
                } else {
                    String::new()
                }
            }
            Some(NodeKind::Text) => "#text".to_string(),
            Some(NodeKind::Document) => "#document".to_string(),
            Some(NodeKind::Comment) => "#comment".to_string(),
            Some(NodeKind::DocumentType) => {
                if let Ok(dt) = dom.world().get::<&DocTypeData>(this) {
                    dt.name.clone()
                } else {
                    String::new()
                }
            }
            Some(NodeKind::DocumentFragment) => "#document-fragment".to_string(),
            Some(NodeKind::CdataSection) => "#cdata-section".to_string(),
            Some(NodeKind::ProcessingInstruction) => {
                // PI target would be the nodeName, but we don't store it separately.
                String::new()
            }
            Some(NodeKind::Attribute) => {
                // Attr.nodeName is the attribute name, but we don't have it here.
                String::new()
            }
            // Window is not a Node per WHATWG and does not have a
            // nodeName.  `None` also collapses to empty per the spec
            // fallthrough.
            Some(NodeKind::Window) | None => String::new(),
        };
        Ok(JsValue::String(name))
    }
}

// ---------------------------------------------------------------------------
// nodeType
// ---------------------------------------------------------------------------

/// `node.nodeType` getter — returns the numeric WHATWG nodeType value.
pub struct GetNodeType;

impl DomApiHandler for GetNodeType {
    fn method_name(&self) -> &str {
        "nodeType.get"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        match dom.node_kind(this) {
            Some(kind) => Ok(JsValue::Number(f64::from(kind.node_type()))),
            None => Ok(JsValue::Null),
        }
    }
}

// ---------------------------------------------------------------------------
// nodeValue
// ---------------------------------------------------------------------------

/// `node.nodeValue` getter — returns data for Text/Comment nodes, null otherwise.
pub struct GetNodeValue;

impl DomApiHandler for GetNodeValue {
    fn method_name(&self) -> &str {
        "nodeValue.get"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        if let Ok(tc) = dom.world().get::<&TextContent>(this) {
            return Ok(JsValue::String(tc.0.clone()));
        }
        if let Ok(cd) = dom.world().get::<&CommentData>(this) {
            return Ok(JsValue::String(cd.0.clone()));
        }
        Ok(JsValue::Null)
    }
}

// ---------------------------------------------------------------------------
// childElementCount
// ---------------------------------------------------------------------------

/// `element.childElementCount` getter — returns the number of element children.
pub struct GetChildElementCount;

impl DomApiHandler for GetChildElementCount {
    fn method_name(&self) -> &str {
        "childElementCount.get"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let count = dom
            .children_iter(this)
            .filter(|&c| dom.is_element(c))
            .count();
        #[allow(clippy::cast_precision_loss)] // DOM IDL uses f64 for all numeric values
        Ok(JsValue::Number(count as f64))
    }
}

// ---------------------------------------------------------------------------
// hasChildNodes
// ---------------------------------------------------------------------------

/// `node.hasChildNodes()` — returns true if the node has any children.
pub struct HasChildNodes;

impl DomApiHandler for HasChildNodes {
    fn method_name(&self) -> &str {
        "hasChildNodes"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        Ok(JsValue::Bool(dom.get_first_child(this).is_some()))
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
#[allow(unused_must_use)] // Test setup calls dom.append_child() etc. without checking return values
mod tests {
    use super::*;
    use elidex_ecs::Attributes;

    /// Build: doc → html → body → div → [span, text("hello"), p]
    fn setup() -> (
        EcsDom,
        Entity,
        Entity,
        Entity,
        Entity,
        Entity,
        Entity,
        Entity,
        SessionCore,
    ) {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let html = dom.create_element("html", Attributes::default());
        let body = dom.create_element("body", Attributes::default());
        let div = dom.create_element("div", Attributes::default());
        let span = dom.create_element("span", Attributes::default());
        let text = dom.create_text("hello");
        let p = dom.create_element("p", Attributes::default());

        dom.append_child(doc, html);
        dom.append_child(html, body);
        dom.append_child(body, div);
        dom.append_child(div, span);
        dom.append_child(div, text);
        dom.append_child(div, p);

        let session = SessionCore::new();
        (dom, doc, html, body, div, span, text, p, session)
    }

    // --- parentNode ---

    #[test]
    fn parent_node_element() {
        let (mut dom, _doc, _html, _body, div, span, _text, _p, mut session) = setup();
        let result = GetParentNode
            .invoke(span, &[], &mut session, &mut dom)
            .unwrap();
        if let JsValue::ObjectRef(r) = result {
            let (entity, _) = session
                .identity_map()
                .get(elidex_script_session::JsObjectRef::from_raw(r))
                .unwrap();
            assert_eq!(entity, div);
        } else {
            panic!("expected ObjectRef, got {result:?}");
        }
    }

    #[test]
    fn parent_node_root_is_null() {
        let (mut dom, doc, _html, _body, _div, _span, _text, _p, mut session) = setup();
        let result = GetParentNode
            .invoke(doc, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::Null);
    }

    #[test]
    fn parent_node_orphan() {
        let mut dom = EcsDom::new();
        let orphan = dom.create_element("div", Attributes::default());
        let mut session = SessionCore::new();
        let result = GetParentNode
            .invoke(orphan, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::Null);
    }

    // --- parentElement ---

    #[test]
    fn parent_element_element() {
        let (mut dom, _doc, _html, _body, div, span, _text, _p, mut session) = setup();
        let result = GetParentElement
            .invoke(span, &[], &mut session, &mut dom)
            .unwrap();
        if let JsValue::ObjectRef(r) = result {
            let (entity, _) = session
                .identity_map()
                .get(elidex_script_session::JsObjectRef::from_raw(r))
                .unwrap();
            assert_eq!(entity, div);
        } else {
            panic!("expected ObjectRef, got {result:?}");
        }
    }

    #[test]
    fn parent_element_html_returns_null_for_doc() {
        let (mut dom, _doc, html, _body, _div, _span, _text, _p, mut session) = setup();
        // html's parent is doc (a Document node, not an element).
        let result = GetParentElement
            .invoke(html, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::Null);
    }

    #[test]
    fn parent_element_text() {
        let (mut dom, _doc, _html, _body, div, _span, text, _p, mut session) = setup();
        // text's parent is div, which is an element.
        let result = GetParentElement
            .invoke(text, &[], &mut session, &mut dom)
            .unwrap();
        if let JsValue::ObjectRef(r) = result {
            let (entity, _) = session
                .identity_map()
                .get(elidex_script_session::JsObjectRef::from_raw(r))
                .unwrap();
            assert_eq!(entity, div);
        } else {
            panic!("expected ObjectRef, got {result:?}");
        }
    }

    // --- firstChild ---

    #[test]
    fn first_child_exists() {
        let (mut dom, _doc, _html, _body, div, span, _text, _p, mut session) = setup();
        let result = GetFirstChild
            .invoke(div, &[], &mut session, &mut dom)
            .unwrap();
        if let JsValue::ObjectRef(r) = result {
            let (entity, _) = session
                .identity_map()
                .get(elidex_script_session::JsObjectRef::from_raw(r))
                .unwrap();
            assert_eq!(entity, span);
        } else {
            panic!("expected ObjectRef, got {result:?}");
        }
    }

    #[test]
    fn first_child_empty() {
        let (mut dom, _doc, _html, _body, _div, span, _text, _p, mut session) = setup();
        let result = GetFirstChild
            .invoke(span, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::Null);
    }

    // --- lastChild ---

    #[test]
    fn last_child_exists() {
        let (mut dom, _doc, _html, _body, div, _span, _text, p, mut session) = setup();
        let result = GetLastChild
            .invoke(div, &[], &mut session, &mut dom)
            .unwrap();
        if let JsValue::ObjectRef(r) = result {
            let (entity, _) = session
                .identity_map()
                .get(elidex_script_session::JsObjectRef::from_raw(r))
                .unwrap();
            assert_eq!(entity, p);
        } else {
            panic!("expected ObjectRef, got {result:?}");
        }
    }

    #[test]
    fn last_child_empty() {
        let (mut dom, _doc, _html, _body, _div, span, _text, _p, mut session) = setup();
        let result = GetLastChild
            .invoke(span, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::Null);
    }

    // --- nextSibling ---

    #[test]
    fn next_sibling_exists() {
        let (mut dom, _doc, _html, _body, _div, span, text, _p, mut session) = setup();
        let result = GetNextSibling
            .invoke(span, &[], &mut session, &mut dom)
            .unwrap();
        if let JsValue::ObjectRef(r) = result {
            let (entity, _) = session
                .identity_map()
                .get(elidex_script_session::JsObjectRef::from_raw(r))
                .unwrap();
            assert_eq!(entity, text);
        } else {
            panic!("expected ObjectRef, got {result:?}");
        }
    }

    #[test]
    fn next_sibling_last() {
        let (mut dom, _doc, _html, _body, _div, _span, _text, p, mut session) = setup();
        let result = GetNextSibling
            .invoke(p, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::Null);
    }

    // --- previousSibling ---

    #[test]
    fn prev_sibling_exists() {
        let (mut dom, _doc, _html, _body, _div, span, text, _p, mut session) = setup();
        let result = GetPrevSibling
            .invoke(text, &[], &mut session, &mut dom)
            .unwrap();
        if let JsValue::ObjectRef(r) = result {
            let (entity, _) = session
                .identity_map()
                .get(elidex_script_session::JsObjectRef::from_raw(r))
                .unwrap();
            assert_eq!(entity, span);
        } else {
            panic!("expected ObjectRef, got {result:?}");
        }
    }

    #[test]
    fn prev_sibling_first() {
        let (mut dom, _doc, _html, _body, _div, span, _text, _p, mut session) = setup();
        let result = GetPrevSibling
            .invoke(span, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::Null);
    }

    // --- firstElementChild ---

    #[test]
    fn first_element_child_skip_text() {
        // Rearrange: div → [text, span, p]
        let mut dom = EcsDom::new();
        let div = dom.create_element("div", Attributes::default());
        let text = dom.create_text("hello");
        let span = dom.create_element("span", Attributes::default());
        let p = dom.create_element("p", Attributes::default());
        dom.append_child(div, text);
        dom.append_child(div, span);
        dom.append_child(div, p);

        let mut session = SessionCore::new();
        let result = GetFirstElementChild
            .invoke(div, &[], &mut session, &mut dom)
            .unwrap();
        if let JsValue::ObjectRef(r) = result {
            let (entity, _) = session
                .identity_map()
                .get(elidex_script_session::JsObjectRef::from_raw(r))
                .unwrap();
            assert_eq!(entity, span);
        } else {
            panic!("expected ObjectRef, got {result:?}");
        }
    }

    #[test]
    fn first_element_child_none() {
        let mut dom = EcsDom::new();
        let div = dom.create_element("div", Attributes::default());
        let text = dom.create_text("only text");
        dom.append_child(div, text);

        let mut session = SessionCore::new();
        let result = GetFirstElementChild
            .invoke(div, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::Null);
    }

    // --- lastElementChild ---

    #[test]
    fn last_element_child_skip_text() {
        // div → [span, p, text]
        let mut dom = EcsDom::new();
        let div = dom.create_element("div", Attributes::default());
        let span = dom.create_element("span", Attributes::default());
        let p = dom.create_element("p", Attributes::default());
        let text = dom.create_text("trailing");
        dom.append_child(div, span);
        dom.append_child(div, p);
        dom.append_child(div, text);

        let mut session = SessionCore::new();
        let result = GetLastElementChild
            .invoke(div, &[], &mut session, &mut dom)
            .unwrap();
        if let JsValue::ObjectRef(r) = result {
            let (entity, _) = session
                .identity_map()
                .get(elidex_script_session::JsObjectRef::from_raw(r))
                .unwrap();
            assert_eq!(entity, p);
        } else {
            panic!("expected ObjectRef, got {result:?}");
        }
    }

    #[test]
    fn last_element_child_none() {
        let mut dom = EcsDom::new();
        let div = dom.create_element("div", Attributes::default());
        let text = dom.create_text("only text");
        dom.append_child(div, text);

        let mut session = SessionCore::new();
        let result = GetLastElementChild
            .invoke(div, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::Null);
    }

    // --- nextElementSibling ---

    #[test]
    fn next_element_sibling_skip() {
        // div → [span, text, p]
        let (mut dom, _doc, _html, _body, _div, span, _text, p, mut session) = setup();
        let result = GetNextElementSibling
            .invoke(span, &[], &mut session, &mut dom)
            .unwrap();
        if let JsValue::ObjectRef(r) = result {
            let (entity, _) = session
                .identity_map()
                .get(elidex_script_session::JsObjectRef::from_raw(r))
                .unwrap();
            assert_eq!(entity, p);
        } else {
            panic!("expected ObjectRef, got {result:?}");
        }
    }

    // --- previousElementSibling ---

    #[test]
    fn prev_element_sibling_skip() {
        // div → [span, text, p]
        let (mut dom, _doc, _html, _body, _div, span, _text, p, mut session) = setup();
        let result = GetPrevElementSibling
            .invoke(p, &[], &mut session, &mut dom)
            .unwrap();
        if let JsValue::ObjectRef(r) = result {
            let (entity, _) = session
                .identity_map()
                .get(elidex_script_session::JsObjectRef::from_raw(r))
                .unwrap();
            assert_eq!(entity, span);
        } else {
            panic!("expected ObjectRef, got {result:?}");
        }
    }

    // --- tagName ---

    #[test]
    fn tag_name_uppercase() {
        let (mut dom, _doc, _html, _body, div, _span, _text, _p, mut session) = setup();
        let result = GetTagName.invoke(div, &[], &mut session, &mut dom).unwrap();
        assert_eq!(result, JsValue::String("DIV".into()));
    }

    // --- nodeName ---

    #[test]
    fn node_name_element() {
        let (mut dom, _doc, _html, _body, div, _span, _text, _p, mut session) = setup();
        let result = GetNodeName
            .invoke(div, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::String("DIV".into()));
    }

    #[test]
    fn node_name_text() {
        let (mut dom, _doc, _html, _body, _div, _span, text, _p, mut session) = setup();
        let result = GetNodeName
            .invoke(text, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::String("#text".into()));
    }

    #[test]
    fn node_name_document() {
        let (mut dom, doc, _html, _body, _div, _span, _text, _p, mut session) = setup();
        let result = GetNodeName
            .invoke(doc, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::String("#document".into()));
    }

    // --- nodeType ---

    #[test]
    fn node_type_element() {
        let (mut dom, _doc, _html, _body, div, _span, _text, _p, mut session) = setup();
        let result = GetNodeType
            .invoke(div, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::Number(1.0));
    }

    #[test]
    fn node_type_text() {
        let (mut dom, _doc, _html, _body, _div, _span, text, _p, mut session) = setup();
        let result = GetNodeType
            .invoke(text, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::Number(3.0));
    }

    #[test]
    fn node_type_document() {
        let (mut dom, doc, _html, _body, _div, _span, _text, _p, mut session) = setup();
        let result = GetNodeType
            .invoke(doc, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::Number(9.0));
    }

    // --- nodeValue ---

    #[test]
    fn node_value_text() {
        let (mut dom, _doc, _html, _body, _div, _span, text, _p, mut session) = setup();
        let result = GetNodeValue
            .invoke(text, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::String("hello".into()));
    }

    #[test]
    fn node_value_element() {
        let (mut dom, _doc, _html, _body, div, _span, _text, _p, mut session) = setup();
        let result = GetNodeValue
            .invoke(div, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::Null);
    }

    // --- childElementCount ---

    #[test]
    fn child_element_count() {
        let (mut dom, _doc, _html, _body, div, _span, _text, _p, mut session) = setup();
        // div has [span, text("hello"), p] — 2 elements.
        let result = GetChildElementCount
            .invoke(div, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::Number(2.0));
    }

    // --- hasChildNodes ---

    #[test]
    fn has_child_nodes_true() {
        let (mut dom, _doc, _html, _body, div, _span, _text, _p, mut session) = setup();
        let result = HasChildNodes
            .invoke(div, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::Bool(true));
    }

    #[test]
    fn has_child_nodes_false() {
        let (mut dom, _doc, _html, _body, _div, span, _text, _p, mut session) = setup();
        let result = HasChildNodes
            .invoke(span, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::Bool(false));
    }
}
