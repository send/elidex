//! Element-level DOM API handlers: appendChild, insertBefore, removeChild,
//! getAttribute/setAttribute/removeAttribute, textContent, innerHTML.

use elidex_ecs::{Attributes, EcsDom, Entity, TagType, TextContent};
use elidex_plugin::JsValue;
use elidex_script_session::{
    DomApiError, DomApiErrorKind, DomApiHandler, JsObjectRef, SessionCore,
};

use crate::util::{escape_attr, escape_html, require_object_ref_arg, require_string_arg};

// ---------------------------------------------------------------------------
// appendChild
// ---------------------------------------------------------------------------

/// `element.appendChild(child)` — appends a child node.
pub struct AppendChild;

impl DomApiHandler for AppendChild {
    fn method_name(&self) -> &str {
        "appendChild"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let child_ref = require_object_ref_arg(args, 0)?;
        let (child_entity, _) = session
            .identity_map()
            .get(JsObjectRef::from_raw(child_ref))
            .ok_or_else(|| DomApiError {
                kind: DomApiErrorKind::NotFoundError,
                message: "child not found".into(),
            })?;
        if !dom.append_child(this, child_entity) {
            return Err(DomApiError {
                kind: DomApiErrorKind::HierarchyRequestError,
                message: "appendChild failed".into(),
            });
        }
        Ok(JsValue::ObjectRef(child_ref))
    }
}

// ---------------------------------------------------------------------------
// insertBefore
// ---------------------------------------------------------------------------

/// `element.insertBefore(newChild, refChild)` — inserts a child before a reference child.
pub struct InsertBefore;

impl DomApiHandler for InsertBefore {
    fn method_name(&self) -> &str {
        "insertBefore"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let new_ref = require_object_ref_arg(args, 0)?;
        let (new_entity, _) = session
            .identity_map()
            .get(JsObjectRef::from_raw(new_ref))
            .ok_or_else(|| DomApiError {
                kind: DomApiErrorKind::NotFoundError,
                message: "newChild not found".into(),
            })?;

        // Per DOM spec, insertBefore(node, null) is equivalent to appendChild(node).
        let ref_child_is_null = matches!(args.get(1), None | Some(JsValue::Null));
        if ref_child_is_null {
            if !dom.append_child(this, new_entity) {
                return Err(DomApiError {
                    kind: DomApiErrorKind::HierarchyRequestError,
                    message: "insertBefore(node, null) failed (appendChild equivalent)".into(),
                });
            }
            return Ok(JsValue::ObjectRef(new_ref));
        }

        let ref_ref = require_object_ref_arg(args, 1)?;
        let (ref_entity, _) = session
            .identity_map()
            .get(JsObjectRef::from_raw(ref_ref))
            .ok_or_else(|| DomApiError {
                kind: DomApiErrorKind::NotFoundError,
                message: "refChild not found".into(),
            })?;
        if !dom.insert_before(this, new_entity, ref_entity) {
            return Err(DomApiError {
                kind: DomApiErrorKind::HierarchyRequestError,
                message: "insertBefore failed".into(),
            });
        }
        Ok(JsValue::ObjectRef(new_ref))
    }
}

// ---------------------------------------------------------------------------
// removeChild
// ---------------------------------------------------------------------------

/// `element.removeChild(child)` — removes a child node.
pub struct RemoveChild;

impl DomApiHandler for RemoveChild {
    fn method_name(&self) -> &str {
        "removeChild"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let child_ref = require_object_ref_arg(args, 0)?;
        let (child_entity, _) = session
            .identity_map()
            .get(JsObjectRef::from_raw(child_ref))
            .ok_or_else(|| DomApiError {
                kind: DomApiErrorKind::NotFoundError,
                message: "child not found".into(),
            })?;
        if !dom.remove_child(this, child_entity) {
            return Err(DomApiError {
                kind: DomApiErrorKind::NotFoundError,
                message: "child is not a child of this element".into(),
            });
        }
        Ok(JsValue::ObjectRef(child_ref))
    }
}

// ---------------------------------------------------------------------------
// getAttribute
// ---------------------------------------------------------------------------

/// `element.getAttribute(name)` — returns attribute value or null.
pub struct GetAttribute;

impl DomApiHandler for GetAttribute {
    fn method_name(&self) -> &str {
        "getAttribute"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let name = require_string_arg(args, 0)?;
        let attrs = dom
            .world()
            .get::<&Attributes>(this)
            .map_err(|_| DomApiError {
                kind: DomApiErrorKind::NotFoundError,
                message: "element not found".into(),
            })?;
        match attrs.get(&name) {
            Some(val) => Ok(JsValue::String(val.to_string())),
            None => Ok(JsValue::Null),
        }
    }
}

// ---------------------------------------------------------------------------
// setAttribute
// ---------------------------------------------------------------------------

/// `element.setAttribute(name, value)` — sets an attribute.
pub struct SetAttribute;

impl DomApiHandler for SetAttribute {
    fn method_name(&self) -> &str {
        "setAttribute"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let name = require_string_arg(args, 0)?;
        let value = require_string_arg(args, 1)?;
        let mut attrs = dom
            .world_mut()
            .get::<&mut Attributes>(this)
            .map_err(|_| DomApiError {
                kind: DomApiErrorKind::NotFoundError,
                message: "element not found".into(),
            })?;
        attrs.set(name, value);
        Ok(JsValue::Undefined)
    }
}

// ---------------------------------------------------------------------------
// removeAttribute
// ---------------------------------------------------------------------------

/// `element.removeAttribute(name)` — removes an attribute.
pub struct RemoveAttribute;

impl DomApiHandler for RemoveAttribute {
    fn method_name(&self) -> &str {
        "removeAttribute"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let name = require_string_arg(args, 0)?;
        let mut attrs = dom
            .world_mut()
            .get::<&mut Attributes>(this)
            .map_err(|_| DomApiError {
                kind: DomApiErrorKind::NotFoundError,
                message: "element not found".into(),
            })?;
        attrs.remove(&name);
        Ok(JsValue::Undefined)
    }
}

// ---------------------------------------------------------------------------
// textContent getter
// ---------------------------------------------------------------------------

/// `element.textContent` getter — concatenates all descendant text content.
pub struct GetTextContent;

impl DomApiHandler for GetTextContent {
    fn method_name(&self) -> &str {
        "textContent.get"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let text = collect_text_content(this, dom);
        Ok(JsValue::String(text))
    }
}

/// Collect all text content from an entity and its descendants.
pub fn collect_text_content(entity: Entity, dom: &EcsDom) -> String {
    let mut result = String::new();
    collect_text_recursive(entity, dom, &mut result);
    result
}

fn collect_text_recursive(entity: Entity, dom: &EcsDom, result: &mut String) {
    // Per DOM spec, textContent on a text node returns its own data only.
    if let Ok(tc) = dom.world().get::<&TextContent>(entity) {
        result.push_str(&tc.0);
        return;
    }
    for child in dom.children_iter(entity) {
        collect_text_recursive(child, dom, result);
    }
}

// ---------------------------------------------------------------------------
// textContent setter
// ---------------------------------------------------------------------------

/// `element.textContent` setter — removes all children and sets text.
pub struct SetTextContent;

impl DomApiHandler for SetTextContent {
    fn method_name(&self) -> &str {
        "textContent.set"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let text = require_string_arg(args, 0)?;

        // If entity is a text node, just update its content.
        if let Ok(mut tc) = dom.world_mut().get::<&mut TextContent>(this) {
            text.clone_into(&mut tc.0);
            return Ok(JsValue::Undefined);
        }

        // Remove all children.
        let children = dom.children(this);
        for child in children {
            if !dom.remove_child(this, child) {
                return Err(DomApiError {
                    kind: DomApiErrorKind::HierarchyRequestError,
                    message: "SetTextContent: failed to remove existing child".into(),
                });
            }
        }

        // Create and append a text node if text is non-empty.
        if !text.is_empty() {
            let text_node = dom.create_text(text);
            let _ = dom.append_child(this, text_node);
        }

        Ok(JsValue::Undefined)
    }
}

// ---------------------------------------------------------------------------
// innerHTML getter
// ---------------------------------------------------------------------------

/// `element.innerHTML` getter — serializes children to HTML.
pub struct GetInnerHtml;

impl DomApiHandler for GetInnerHtml {
    fn method_name(&self) -> &str {
        "innerHTML.get"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let html = serialize_inner_html(this, dom);
        Ok(JsValue::String(html))
    }
}

/// Serialize children of an entity to HTML.
pub fn serialize_inner_html(entity: Entity, dom: &EcsDom) -> String {
    let mut html = String::new();
    for child in dom.children_iter(entity) {
        serialize_node(child, dom, &mut html);
    }
    html
}

/// HTML void elements that must not have a closing tag.
const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

fn serialize_node(entity: Entity, dom: &EcsDom, html: &mut String) {
    // Text node.
    if let Ok(tc) = dom.world().get::<&TextContent>(entity) {
        html.push_str(&escape_html(&tc.0));
        return;
    }
    // Element node.
    if let Ok(tag) = dom.world().get::<&TagType>(entity) {
        html.push('<');
        html.push_str(&tag.0);
        if let Ok(attrs) = dom.world().get::<&Attributes>(entity) {
            for (name, value) in attrs.iter() {
                html.push(' ');
                html.push_str(name);
                html.push_str("=\"");
                html.push_str(&escape_attr(value));
                html.push('"');
            }
        }
        html.push('>');
        // Void elements must not have closing tags or children content.
        if VOID_ELEMENTS.contains(&tag.0.as_str()) {
            return;
        }
        for child in dom.children_iter(entity) {
            serialize_node(child, dom, html);
        }
        html.push_str("</");
        html.push_str(&tag.0);
        html.push('>');
        return;
    }
    // Non-element, non-text nodes (e.g., document roots): recurse into children.
    for child in dom.children_iter(entity) {
        serialize_node(child, dom, html);
    }
}

#[cfg(test)]
#[allow(unused_must_use)]
mod tests {
    use super::*;
    use elidex_script_session::ComponentKind;

    fn setup() -> (EcsDom, Entity, Entity, SessionCore) {
        let mut dom = EcsDom::new();
        let parent = dom.create_element("div", Attributes::default());
        let child = dom.create_element("span", Attributes::default());
        let mut session = SessionCore::new();
        // Pre-register entities so we can pass ObjectRef args.
        session.get_or_create_wrapper(parent, ComponentKind::Element);
        session.get_or_create_wrapper(child, ComponentKind::Element);
        (dom, parent, child, session)
    }

    #[test]
    fn append_child_success() {
        let (mut dom, parent, child, mut session) = setup();
        let child_ref = session
            .get_or_create_wrapper(child, ComponentKind::Element)
            .to_raw();
        let handler = AppendChild;
        let result = handler
            .invoke(
                parent,
                &[JsValue::ObjectRef(child_ref)],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::ObjectRef(child_ref));
        assert_eq!(dom.children(parent), vec![child]);
    }

    #[test]
    fn remove_child_success() {
        let (mut dom, parent, child, mut session) = setup();
        dom.append_child(parent, child);
        let child_ref = session
            .get_or_create_wrapper(child, ComponentKind::Element)
            .to_raw();
        let handler = RemoveChild;
        let result = handler
            .invoke(
                parent,
                &[JsValue::ObjectRef(child_ref)],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::ObjectRef(child_ref));
        assert!(dom.children(parent).is_empty());
    }

    #[test]
    fn get_set_attribute() {
        let (mut dom, parent, _, mut session) = setup();

        let set_handler = SetAttribute;
        set_handler
            .invoke(
                parent,
                &[
                    JsValue::String("data-x".into()),
                    JsValue::String("42".into()),
                ],
                &mut session,
                &mut dom,
            )
            .unwrap();

        let get_handler = GetAttribute;
        let result = get_handler
            .invoke(
                parent,
                &[JsValue::String("data-x".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::String("42".into()));
    }

    #[test]
    fn get_attribute_missing() {
        let (mut dom, parent, _, mut session) = setup();
        let handler = GetAttribute;
        let result = handler
            .invoke(
                parent,
                &[JsValue::String("nonexistent".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Null);
    }

    #[test]
    fn remove_attribute() {
        let (mut dom, parent, _, mut session) = setup();
        // Set first.
        {
            let mut attrs = dom.world_mut().get::<&mut Attributes>(parent).unwrap();
            attrs.set("class", "active");
        }
        let handler = RemoveAttribute;
        handler
            .invoke(
                parent,
                &[JsValue::String("class".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let attrs = dom.world().get::<&Attributes>(parent).unwrap();
        assert!(!attrs.contains("class"));
    }

    #[test]
    fn text_content_get_set() {
        let (mut dom, parent, _, mut session) = setup();
        let text_node = dom.create_text("original");
        dom.append_child(parent, text_node);

        // Get.
        let get = GetTextContent;
        let result = get.invoke(parent, &[], &mut session, &mut dom).unwrap();
        assert_eq!(result, JsValue::String("original".into()));

        // Set.
        let set = SetTextContent;
        set.invoke(
            parent,
            &[JsValue::String("replaced".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();

        let result = get.invoke(parent, &[], &mut session, &mut dom).unwrap();
        assert_eq!(result, JsValue::String("replaced".into()));
    }

    #[test]
    fn inner_html_serialization() {
        let (mut dom, parent, _, mut session) = setup();
        let text = dom.create_text("hello <world>");
        dom.append_child(parent, text);

        let handler = GetInnerHtml;
        let result = handler.invoke(parent, &[], &mut session, &mut dom).unwrap();
        assert_eq!(result, JsValue::String("hello &lt;world&gt;".into()));
    }

    #[test]
    fn inner_html_void_elements() {
        let mut dom = EcsDom::new();
        let div = dom.create_element("div", Attributes::default());
        let br = dom.create_element("br", Attributes::default());
        let mut img_attrs = Attributes::default();
        img_attrs.set("src", "test.png");
        let img = dom.create_element("img", img_attrs);
        dom.append_child(div, br);
        dom.append_child(div, img);

        let mut session = SessionCore::new();
        let handler = GetInnerHtml;
        let result = handler.invoke(div, &[], &mut session, &mut dom).unwrap();
        assert_eq!(result, JsValue::String("<br><img src=\"test.png\">".into()));
    }

    #[test]
    fn insert_before_null_ref_appends() {
        let (mut dom, parent, child, mut session) = setup();
        let child_ref = session
            .get_or_create_wrapper(child, ComponentKind::Element)
            .to_raw();
        let handler = InsertBefore;
        let result = handler
            .invoke(
                parent,
                &[JsValue::ObjectRef(child_ref), JsValue::Null],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::ObjectRef(child_ref));
        assert_eq!(dom.children(parent), vec![child]);
    }

    #[test]
    fn inner_html_nested() {
        let mut dom = EcsDom::new();
        let div = dom.create_element("div", Attributes::default());
        let mut p_attrs = Attributes::default();
        p_attrs.set("class", "intro");
        let p = dom.create_element("p", p_attrs);
        let text = dom.create_text("hi");
        dom.append_child(div, p);
        dom.append_child(p, text);

        let mut session = SessionCore::new();
        let handler = GetInnerHtml;
        let result = handler.invoke(div, &[], &mut session, &mut dom).unwrap();
        assert_eq!(result, JsValue::String("<p class=\"intro\">hi</p>".into()));
    }
}
