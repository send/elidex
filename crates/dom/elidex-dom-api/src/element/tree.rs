//! Tree mutation handlers: appendChild, insertBefore, removeChild, insertAdjacent*.

use elidex_ecs::{EcsDom, Entity, TextContent};
use elidex_plugin::JsValue;
use elidex_script_session::{
    DomApiError, DomApiErrorKind, DomApiHandler, JsObjectRef, Mutation, SessionCore,
};

use crate::util::{not_found_error, require_object_ref_arg, require_string_arg};

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
            .ok_or_else(|| not_found_error("child not found"))?;
        if !dom.append_child(this, child_entity) {
            return Err(DomApiError {
                kind: DomApiErrorKind::HierarchyRequestError,
                message: "appendChild: hierarchy request error (cycle or invalid parent)".into(),
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
            .ok_or_else(|| not_found_error("newChild not found"))?;

        let ref_child_is_null = matches!(args.get(1), None | Some(JsValue::Null));
        if ref_child_is_null {
            if !dom.append_child(this, new_entity) {
                return Err(DomApiError {
                    kind: DomApiErrorKind::HierarchyRequestError,
                    message: "insertBefore: hierarchy request error (cycle or invalid parent)"
                        .into(),
                });
            }
            return Ok(JsValue::ObjectRef(new_ref));
        }

        let ref_ref = require_object_ref_arg(args, 1)?;
        let (ref_entity, _) = session
            .identity_map()
            .get(JsObjectRef::from_raw(ref_ref))
            .ok_or_else(|| not_found_error("refChild not found"))?;
        if !dom.insert_before(this, new_entity, ref_entity) {
            return Err(DomApiError {
                kind: DomApiErrorKind::HierarchyRequestError,
                message: "insertBefore: hierarchy request error (invalid reference child or cycle)"
                    .into(),
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
            .ok_or_else(|| not_found_error("child not found"))?;
        if !dom.remove_child(this, child_entity) {
            return Err(not_found_error("child is not a child of this element"));
        }
        Ok(JsValue::ObjectRef(child_ref))
    }
}

// ---------------------------------------------------------------------------
// textContent helpers
// ---------------------------------------------------------------------------

/// Collect all text content from an entity and its descendants.
pub fn collect_text_content(entity: Entity, dom: &EcsDom) -> String {
    let mut result = String::new();
    collect_text_recursive(entity, dom, &mut result);
    result
}

fn collect_text_recursive(entity: Entity, dom: &EcsDom, result: &mut String) {
    if let Ok(tc) = dom.world().get::<&TextContent>(entity) {
        result.push_str(&tc.0);
        return;
    }
    for child in dom.children_iter(entity) {
        collect_text_recursive(child, dom, result);
    }
}

// ---------------------------------------------------------------------------
// insertAdjacentElement (§7e)
// ---------------------------------------------------------------------------

/// `element.insertAdjacentElement(position, element)` — inserts an element
/// at the specified position relative to `this`.
pub struct InsertAdjacentElement;

impl DomApiHandler for InsertAdjacentElement {
    fn method_name(&self) -> &str {
        "insertAdjacentElement"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let position = require_string_arg(args, 0)?;
        let elem_ref = require_object_ref_arg(args, 1)?;
        let (elem_entity, _) = session
            .identity_map()
            .get(JsObjectRef::from_raw(elem_ref))
            .ok_or_else(|| not_found_error("element not found"))?;

        insert_adjacent(this, &position, elem_entity, dom)?;
        Ok(JsValue::ObjectRef(elem_ref))
    }
}

// ---------------------------------------------------------------------------
// insertAdjacentText (§7e)
// ---------------------------------------------------------------------------

/// `element.insertAdjacentText(position, text)` — creates a text node and
/// inserts it at the specified position relative to `this`.
pub struct InsertAdjacentText;

impl DomApiHandler for InsertAdjacentText {
    fn method_name(&self) -> &str {
        "insertAdjacentText"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let position = require_string_arg(args, 0)?;
        let text = require_string_arg(args, 1)?;
        let text_node = dom.create_text(text);

        insert_adjacent(this, &position, text_node, dom)?;
        Ok(JsValue::Undefined)
    }
}

/// Shared implementation for `insertAdjacentElement` / `insertAdjacentText`.
fn insert_adjacent(
    this: Entity,
    position: &str,
    node: Entity,
    dom: &mut EcsDom,
) -> Result<(), DomApiError> {
    if !dom.is_element(this) {
        return Err(DomApiError {
            kind: DomApiErrorKind::InvalidStateError,
            message: "insertAdjacent: context node must be an Element".into(),
        });
    }
    match position.to_ascii_lowercase().as_str() {
        "beforebegin" => {
            let parent = dom.get_parent(this).ok_or_else(|| DomApiError {
                kind: DomApiErrorKind::HierarchyRequestError,
                message: "insertAdjacent: element has no parent for beforebegin".into(),
            })?;
            if !dom.insert_before(parent, node, this) {
                return Err(DomApiError {
                    kind: DomApiErrorKind::HierarchyRequestError,
                    message: "insertAdjacent: insert_before failed".into(),
                });
            }
        }
        "afterbegin" => {
            let first_child = dom.get_first_child(this);
            if let Some(ref_child) = first_child {
                if !dom.insert_before(this, node, ref_child) {
                    return Err(DomApiError {
                        kind: DomApiErrorKind::HierarchyRequestError,
                        message: "insertAdjacent: insert_before failed".into(),
                    });
                }
            } else if !dom.append_child(this, node) {
                return Err(DomApiError {
                    kind: DomApiErrorKind::HierarchyRequestError,
                    message: "insertAdjacent: append_child failed".into(),
                });
            }
        }
        "beforeend" => {
            if !dom.append_child(this, node) {
                return Err(DomApiError {
                    kind: DomApiErrorKind::HierarchyRequestError,
                    message: "insertAdjacent: append_child failed".into(),
                });
            }
        }
        "afterend" => {
            let parent = dom.get_parent(this).ok_or_else(|| DomApiError {
                kind: DomApiErrorKind::HierarchyRequestError,
                message: "insertAdjacent: element has no parent for afterend".into(),
            })?;
            let next = dom.get_next_sibling(this);
            if let Some(ref_child) = next {
                if !dom.insert_before(parent, node, ref_child) {
                    return Err(DomApiError {
                        kind: DomApiErrorKind::HierarchyRequestError,
                        message: "insertAdjacent: insert_before failed".into(),
                    });
                }
            } else if !dom.append_child(parent, node) {
                return Err(DomApiError {
                    kind: DomApiErrorKind::HierarchyRequestError,
                    message: "insertAdjacent: append_child failed".into(),
                });
            }
        }
        _ => {
            return Err(DomApiError {
                kind: DomApiErrorKind::SyntaxError,
                message: format!(
                    "insertAdjacent: invalid position '{position}' (expected beforebegin, afterbegin, beforeend, afterend)"
                ),
            });
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// innerHTML getter
// ---------------------------------------------------------------------------

use crate::util::{escape_attr, escape_html};
use elidex_ecs::{Attributes, TagType};

/// HTML raw text elements whose text children must NOT be escaped during
/// serialization (the content is literal, not entity-decoded by parsers).
const RAW_TEXT_ELEMENTS: &[&str] = &[
    "script", "style", "xmp", "iframe", "noembed", "noframes", "noscript",
];

/// `element.innerHTML` setter — replaces children with parsed HTML.
///
/// Records a `Mutation::SetInnerHtml` which is applied during `session.flush()`.
/// The mutation handles fragment parsing, child removal, and new child insertion.
pub struct SetInnerHtml;

impl DomApiHandler for SetInnerHtml {
    fn method_name(&self) -> &str {
        "innerHTML.set"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        _dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let html = match args.first() {
            Some(JsValue::String(s)) => s.clone(),
            _ => String::new(),
        };
        session.record_mutation(Mutation::SetInnerHtml { entity: this, html });
        Ok(JsValue::Undefined)
    }
}

/// `element.insertAdjacentHTML(position, text)` — parses HTML and inserts at position.
///
/// Position values: "beforebegin", "afterbegin", "beforeend", "afterend".
/// Uses the same fragment parser as innerHTML setter. Parsed nodes are inserted
/// directly via DOM operations (not via mutation recording, since the parser
/// needs mutable DOM access).
pub struct InsertAdjacentHtml;

impl DomApiHandler for InsertAdjacentHtml {
    fn method_name(&self) -> &str {
        "insertAdjacentHTML"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        _dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let position = match args.first() {
            Some(JsValue::String(s)) => s.to_ascii_lowercase(),
            _ => {
                return Err(DomApiError::syntax_error(
                    "insertAdjacentHTML requires a position string",
                ));
            }
        };
        let html = match args.get(1) {
            Some(JsValue::String(s)) => s.clone(),
            _ => String::new(),
        };

        // Validate position before recording.
        match position.as_str() {
            "beforebegin" | "afterbegin" | "beforeend" | "afterend" => {}
            _ => {
                return Err(DomApiError::syntax_error(
                    "Invalid position for insertAdjacentHTML",
                ));
            }
        }

        // Record mutation — applied during session.flush() with proper
        // MutationRecord generation for MutationObserver support.
        session.record_mutation(Mutation::InsertAdjacentHtml {
            entity: this,
            position,
            html,
        });

        Ok(JsValue::Undefined)
    }
}

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
    let raw_text = dom
        .world()
        .get::<&TagType>(entity)
        .ok()
        .is_some_and(|tag| RAW_TEXT_ELEMENTS.contains(&tag.0.as_str()));
    for child in dom.children_iter(entity) {
        serialize_node(child, dom, &mut html, raw_text);
    }
    html
}

/// HTML void elements that must not have a closing tag.
const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

/// Returns `true` if the attribute name contains characters that would break
/// HTML serialization.
fn is_safe_attr_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b != b'"' && b != b'>' && b != b'<' && b != b'=' && !b.is_ascii_whitespace())
}

fn serialize_node(entity: Entity, dom: &EcsDom, html: &mut String, in_raw_text: bool) {
    if let Ok(tc) = dom.world().get::<&TextContent>(entity) {
        if in_raw_text {
            html.push_str(&tc.0);
        } else {
            html.push_str(&escape_html(&tc.0));
        }
        return;
    }
    if let Ok(tag) = dom.world().get::<&TagType>(entity) {
        html.push('<');
        html.push_str(&tag.0);
        if let Ok(attrs) = dom.world().get::<&Attributes>(entity) {
            let mut sorted: Vec<(&str, &str)> = attrs.iter().collect();
            sorted.sort_by_key(|(name, _)| *name);
            for (name, value) in sorted {
                if !is_safe_attr_name(name) {
                    continue;
                }
                html.push(' ');
                html.push_str(name);
                html.push_str("=\"");
                html.push_str(&escape_attr(value));
                html.push('"');
            }
        }
        html.push('>');
        if VOID_ELEMENTS.contains(&tag.0.as_str()) {
            return;
        }
        let child_raw_text = RAW_TEXT_ELEMENTS.contains(&tag.0.as_str());
        for child in dom.children_iter(entity) {
            serialize_node(child, dom, html, child_raw_text);
        }
        html.push_str("</");
        html.push_str(&tag.0);
        html.push('>');
        return;
    }
    for child in dom.children_iter(entity) {
        serialize_node(child, dom, html, false);
    }
}

// ---------------------------------------------------------------------------
// Attribute name validation (WHATWG DOM §5.1)
// ---------------------------------------------------------------------------

/// Validate an attribute name per the WHATWG DOM spec.
pub fn validate_attribute_name(name: &str) -> Result<(), DomApiError> {
    if name.is_empty() {
        return Err(DomApiError {
            kind: DomApiErrorKind::InvalidCharacterError,
            message: "attribute name must not be empty".into(),
        });
    }
    for ch in name.chars() {
        if ch.is_whitespace() || ch == '\0' || ch == '/' || ch == '=' || ch == '>' {
            return Err(DomApiError {
                kind: DomApiErrorKind::InvalidCharacterError,
                message: format!("attribute name contains invalid character: {ch:?}"),
            });
        }
    }
    Ok(())
}
