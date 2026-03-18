//! Element-level DOM API handlers: appendChild, insertBefore, removeChild,
//! getAttribute/setAttribute/removeAttribute, textContent, innerHTML.

use elidex_ecs::{Attributes, EcsDom, Entity, TagType, TextContent};
use elidex_plugin::JsValue;
use elidex_script_session::{
    DomApiError, DomApiErrorKind, DomApiHandler, JsObjectRef, SessionCore,
};

use crate::util::{
    escape_attr, escape_html, not_found_error, require_attrs, require_attrs_mut,
    require_object_ref_arg, require_string_arg,
};

/// HTML raw text elements whose text children must NOT be escaped during
/// serialization (the content is literal, not entity-decoded by parsers).
const RAW_TEXT_ELEMENTS: &[&str] = &[
    "script", "style", "xmp", "iframe", "noembed", "noframes", "noscript",
];

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

        // Per DOM spec, insertBefore(node, null) is equivalent to appendChild(node).
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
        let name = require_string_arg(args, 0)?.to_ascii_lowercase();
        let attrs = require_attrs(this, dom)?;
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
        let raw_name = require_string_arg(args, 0)?;
        validate_attribute_name(&raw_name)?;
        // Per DOM spec, attribute names are lowercased for HTML elements.
        let name = raw_name.to_ascii_lowercase();
        let value = require_string_arg(args, 1)?;
        let mut attrs = require_attrs_mut(this, dom)?;
        attrs.set(name, value);
        drop(attrs);
        dom.rev_version(this);
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
        let raw_name = require_string_arg(args, 0)?;
        validate_attribute_name(&raw_name)?;
        let name = raw_name.to_ascii_lowercase();
        let mut attrs = require_attrs_mut(this, dom)?;
        attrs.remove(&name);
        drop(attrs);
        dom.rev_version(this);
        Ok(JsValue::Undefined)
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
/// HTML serialization (`"`, `>`, `<`, `=`, or ASCII whitespace).
fn is_safe_attr_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b != b'"' && b != b'>' && b != b'<' && b != b'=' && !b.is_ascii_whitespace())
}

fn serialize_node(entity: Entity, dom: &EcsDom, html: &mut String, in_raw_text: bool) {
    // Text node.
    if let Ok(tc) = dom.world().get::<&TextContent>(entity) {
        if in_raw_text {
            html.push_str(&tc.0);
        } else {
            html.push_str(&escape_html(&tc.0));
        }
        return;
    }
    // Element node.
    if let Ok(tag) = dom.world().get::<&TagType>(entity) {
        html.push('<');
        html.push_str(&tag.0);
        if let Ok(attrs) = dom.world().get::<&Attributes>(entity) {
            // Sort attributes by name for deterministic output.
            let mut sorted: Vec<(&str, &str)> = attrs.iter().collect();
            sorted.sort_by_key(|(name, _)| *name);
            for (name, value) in sorted {
                // Skip attributes with unsafe names that would break serialization.
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
        // Void elements must not have closing tags or children content.
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
    // Non-element, non-text nodes (e.g., document roots): recurse into children.
    for child in dom.children_iter(entity) {
        serialize_node(child, dom, html, false);
    }
}

// ---------------------------------------------------------------------------
// Attribute name validation (WHATWG DOM §5.1)
// ---------------------------------------------------------------------------

/// Validate an attribute name per the WHATWG DOM spec.
///
/// Rejects empty names and names containing whitespace, null (`\0`), `/`, `=`,
/// or `>` characters, returning `InvalidCharacterError`.
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
///
/// Position is ASCII case-insensitive per the WHATWG spec.
/// Throws `InvalidStateError` if `this` is not an Element (WHATWG DOM §6.4.9.5).
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
// hasAttribute (§7i)
// ---------------------------------------------------------------------------

/// `element.hasAttribute(name)` — returns true if the attribute exists.
pub struct HasAttribute;

impl DomApiHandler for HasAttribute {
    fn method_name(&self) -> &str {
        "hasAttribute"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let name = require_string_arg(args, 0)?.to_ascii_lowercase();
        let attrs = require_attrs(this, dom)?;
        Ok(JsValue::Bool(attrs.contains(&name)))
    }
}

// ---------------------------------------------------------------------------
// toggleAttribute (§7i)
// ---------------------------------------------------------------------------

/// `element.toggleAttribute(name, force?)` — toggles a boolean attribute.
pub struct ToggleAttribute;

impl DomApiHandler for ToggleAttribute {
    fn method_name(&self) -> &str {
        "toggleAttribute"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let raw_name = require_string_arg(args, 0)?;
        validate_attribute_name(&raw_name)?;
        let name = raw_name.to_ascii_lowercase();

        let force = match args.get(1) {
            Some(JsValue::Bool(b)) => Some(*b),
            _ => None,
        };

        let mut attrs = require_attrs_mut(this, dom)?;
        let has = attrs.contains(&name);

        let (changed, result) = match force {
            Some(true) => {
                if has {
                    (false, true)
                } else {
                    attrs.set(&name, "");
                    (true, true)
                }
            }
            Some(false) => {
                if has {
                    attrs.remove(&name);
                    (true, false)
                } else {
                    (false, false)
                }
            }
            None => {
                if has {
                    attrs.remove(&name);
                    (true, false)
                } else {
                    attrs.set(&name, "");
                    (true, true)
                }
            }
        };
        drop(attrs);
        if changed {
            dom.rev_version(this);
        }
        Ok(JsValue::Bool(result))
    }
}

// ---------------------------------------------------------------------------
// getAttributeNames (§7i)
// ---------------------------------------------------------------------------

/// `element.getAttributeNames()` — returns attribute names in insertion order,
/// joined by `\0` (the JS bridge will split).
pub struct GetAttributeNames;

impl DomApiHandler for GetAttributeNames {
    fn method_name(&self) -> &str {
        "getAttributeNames"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let attrs = require_attrs(this, dom)?;
        let names: Vec<&str> = attrs.keys();
        Ok(JsValue::String(names.join("\0")))
    }
}

// ---------------------------------------------------------------------------
// className getter/setter (§7i)
// ---------------------------------------------------------------------------

/// `element.className` getter — returns the `class` attribute value.
pub struct GetClassName;

impl DomApiHandler for GetClassName {
    fn method_name(&self) -> &str {
        "className.get"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let attrs = require_attrs(this, dom)?;
        Ok(JsValue::String(
            attrs.get("class").unwrap_or("").to_string(),
        ))
    }
}

/// `element.className` setter — sets the `class` attribute.
pub struct SetClassName;

impl DomApiHandler for SetClassName {
    fn method_name(&self) -> &str {
        "className.set"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let value = require_string_arg(args, 0)?;
        let mut attrs = require_attrs_mut(this, dom)?;
        attrs.set("class", value);
        drop(attrs);
        dom.rev_version(this);
        Ok(JsValue::Undefined)
    }
}

// ---------------------------------------------------------------------------
// id getter/setter (§7i)
// ---------------------------------------------------------------------------

/// `element.id` getter — returns the `id` attribute value.
pub struct GetId;

impl DomApiHandler for GetId {
    fn method_name(&self) -> &str {
        "id.get"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let attrs = require_attrs(this, dom)?;
        Ok(JsValue::String(attrs.get("id").unwrap_or("").to_string()))
    }
}

/// `element.id` setter — sets the `id` attribute.
pub struct SetId;

impl DomApiHandler for SetId {
    fn method_name(&self) -> &str {
        "id.set"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let value = require_string_arg(args, 0)?;
        let mut attrs = require_attrs_mut(this, dom)?;
        attrs.set("id", value);
        drop(attrs);
        dom.rev_version(this);
        Ok(JsValue::Undefined)
    }
}

// ---------------------------------------------------------------------------
// Dataset helpers (§7i)
// ---------------------------------------------------------------------------

/// Convert a `data-*` attribute name to camelCase.
///
/// `"data-foo-bar"` → `"fooBar"`. The `"data-"` prefix is stripped, then
/// each `-x` sequence is converted to uppercase `X`.
#[must_use]
pub fn data_attr_to_camel(name: &str) -> String {
    let stripped = name.strip_prefix("data-").unwrap_or(name);
    let mut result = String::with_capacity(stripped.len());
    let mut capitalize_next = false;
    for ch in stripped.chars() {
        if ch == '-' {
            capitalize_next = true;
        } else if capitalize_next {
            if ch.is_ascii_lowercase() {
                result.extend(ch.to_uppercase());
            } else {
                result.push('-');
                result.push(ch);
            }
            capitalize_next = false;
        } else {
            result.push(ch);
        }
    }
    // Trailing dash.
    if capitalize_next {
        result.push('-');
    }
    result
}

/// Convert a camelCase name to a `data-*` attribute name.
///
/// `"fooBar"` → `"data-foo-bar"`. Uppercase letters are converted to
/// `"-"` + lowercase.
#[must_use]
pub fn camel_to_data_attr(name: &str) -> String {
    let mut result = String::with_capacity(name.len() + 5);
    result.push_str("data-");
    for ch in name.chars() {
        if ch.is_ascii_uppercase() {
            result.push('-');
            result.extend(ch.to_lowercase());
        } else {
            result.push(ch);
        }
    }
    result
}

// ---------------------------------------------------------------------------
// dataset.get / dataset.set / dataset.delete / dataset.keys
// ---------------------------------------------------------------------------

/// `element.dataset.get(key)` — read a data-* attribute by camelCase key.
pub struct DatasetGet;

impl DomApiHandler for DatasetGet {
    fn method_name(&self) -> &str {
        "dataset.get"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let key = require_string_arg(args, 0)?;
        let attr_name = camel_to_data_attr(&key);
        let attrs = require_attrs(this, dom)?;
        match attrs.get(&attr_name) {
            Some(val) => Ok(JsValue::String(val.to_string())),
            None => Ok(JsValue::Undefined),
        }
    }
}

/// `element.dataset.set(key, value)` — set a data-* attribute by camelCase key.
pub struct DatasetSet;

impl DomApiHandler for DatasetSet {
    fn method_name(&self) -> &str {
        "dataset.set"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let key = require_string_arg(args, 0)?;
        let value = require_string_arg(args, 1)?;
        let attr_name = camel_to_data_attr(&key);
        let mut attrs = require_attrs_mut(this, dom)?;
        attrs.set(attr_name, value);
        drop(attrs);
        dom.rev_version(this);
        Ok(JsValue::Undefined)
    }
}

/// `element.dataset.delete(key)` — remove a data-* attribute by camelCase key.
pub struct DatasetDelete;

impl DomApiHandler for DatasetDelete {
    fn method_name(&self) -> &str {
        "dataset.delete"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let key = require_string_arg(args, 0)?;
        let attr_name = camel_to_data_attr(&key);
        let mut attrs = require_attrs_mut(this, dom)?;
        attrs.remove(&attr_name);
        drop(attrs);
        dom.rev_version(this);
        Ok(JsValue::Undefined)
    }
}

/// `element.dataset.keys()` — return all data-* attribute keys as camelCase,
/// joined by `\0`.
pub struct DatasetKeys;

impl DomApiHandler for DatasetKeys {
    fn method_name(&self) -> &str {
        "dataset.keys"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let attrs = require_attrs(this, dom)?;
        let keys: Vec<String> = attrs
            .keys()
            .iter()
            .filter(|k| k.starts_with("data-"))
            .map(|k| data_attr_to_camel(k))
            .collect();
        Ok(JsValue::String(keys.join("\0")))
    }
}

#[cfg(test)]
#[allow(unused_must_use)] // Test setup calls dom.append_child() etc. without checking return values
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
        let get = crate::node_methods::GetTextContentNodeKind;
        let result = get.invoke(parent, &[], &mut session, &mut dom).unwrap();
        assert_eq!(result, JsValue::String("original".into()));

        // Set.
        let set = crate::node_methods::SetTextContentNodeKind;
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
    fn get_attribute_case_insensitive() {
        let (mut dom, parent, _, mut session) = setup();
        let set_handler = SetAttribute;
        set_handler
            .invoke(
                parent,
                &[
                    JsValue::String("Data-X".into()),
                    JsValue::String("42".into()),
                ],
                &mut session,
                &mut dom,
            )
            .unwrap();

        let get_handler = GetAttribute;
        // Both "data-x" and "Data-X" should find the attribute (stored as "data-x")
        let result = get_handler
            .invoke(
                parent,
                &[JsValue::String("Data-X".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::String("42".into()));
    }

    #[test]
    fn remove_attribute_case_insensitive() {
        let (mut dom, parent, _, mut session) = setup();
        {
            let mut attrs = dom.world_mut().get::<&mut Attributes>(parent).unwrap();
            attrs.set("class", "active");
        }
        let handler = RemoveAttribute;
        handler
            .invoke(
                parent,
                &[JsValue::String("CLASS".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let attrs = dom.world().get::<&Attributes>(parent).unwrap();
        assert!(!attrs.contains("class"));
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

    // -----------------------------------------------------------------------
    // validate_attribute_name tests
    // -----------------------------------------------------------------------

    #[test]
    fn validate_attr_name_rejects_empty() {
        let err = validate_attribute_name("").unwrap_err();
        assert_eq!(err.kind, DomApiErrorKind::InvalidCharacterError);
    }

    #[test]
    fn validate_attr_name_rejects_whitespace() {
        let err = validate_attribute_name("a b").unwrap_err();
        assert_eq!(err.kind, DomApiErrorKind::InvalidCharacterError);
    }

    #[test]
    fn validate_attr_name_rejects_null() {
        let err = validate_attribute_name("a\0b").unwrap_err();
        assert_eq!(err.kind, DomApiErrorKind::InvalidCharacterError);
    }

    #[test]
    fn validate_attr_name_rejects_slash() {
        let err = validate_attribute_name("a/b").unwrap_err();
        assert_eq!(err.kind, DomApiErrorKind::InvalidCharacterError);
    }

    #[test]
    fn validate_attr_name_rejects_equals() {
        let err = validate_attribute_name("a=b").unwrap_err();
        assert_eq!(err.kind, DomApiErrorKind::InvalidCharacterError);
    }

    #[test]
    fn validate_attr_name_rejects_gt() {
        let err = validate_attribute_name("a>b").unwrap_err();
        assert_eq!(err.kind, DomApiErrorKind::InvalidCharacterError);
    }

    #[test]
    fn validate_attr_name_accepts_valid() {
        assert!(validate_attribute_name("data-foo").is_ok());
        assert!(validate_attribute_name("class").is_ok());
    }

    #[test]
    fn set_attribute_rejects_invalid_name() {
        let (mut dom, parent, _, mut session) = setup();
        let err = SetAttribute
            .invoke(
                parent,
                &[JsValue::String(String::new()), JsValue::String("v".into())],
                &mut session,
                &mut dom,
            )
            .unwrap_err();
        assert_eq!(err.kind, DomApiErrorKind::InvalidCharacterError);
    }

    #[test]
    fn remove_attribute_rejects_invalid_name() {
        let (mut dom, parent, _, mut session) = setup();
        let err = RemoveAttribute
            .invoke(
                parent,
                &[JsValue::String("a b".into())],
                &mut session,
                &mut dom,
            )
            .unwrap_err();
        assert_eq!(err.kind, DomApiErrorKind::InvalidCharacterError);
    }

    // -----------------------------------------------------------------------
    // insertAdjacentElement / insertAdjacentText tests
    // -----------------------------------------------------------------------

    #[test]
    fn insert_adjacent_element_beforebegin() {
        let (mut dom, parent, _child, mut session) = setup();
        let root = dom.create_element("body", Attributes::default());
        dom.append_child(root, parent);

        let new_elem = dom.create_element("p", Attributes::default());
        session.get_or_create_wrapper(new_elem, ComponentKind::Element);
        let new_ref = session
            .get_or_create_wrapper(new_elem, ComponentKind::Element)
            .to_raw();

        let result = InsertAdjacentElement
            .invoke(
                parent,
                &[
                    JsValue::String("beforebegin".into()),
                    JsValue::ObjectRef(new_ref),
                ],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::ObjectRef(new_ref));
        let children = dom.children(root);
        assert_eq!(children, vec![new_elem, parent]);
    }

    #[test]
    fn insert_adjacent_element_afterbegin() {
        let (mut dom, parent, child, mut session) = setup();
        dom.append_child(parent, child);

        let new_elem = dom.create_element("p", Attributes::default());
        session.get_or_create_wrapper(new_elem, ComponentKind::Element);
        let new_ref = session
            .get_or_create_wrapper(new_elem, ComponentKind::Element)
            .to_raw();

        InsertAdjacentElement
            .invoke(
                parent,
                &[
                    JsValue::String("afterbegin".into()),
                    JsValue::ObjectRef(new_ref),
                ],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let children = dom.children(parent);
        assert_eq!(children[0], new_elem);
        assert_eq!(children[1], child);
    }

    #[test]
    fn insert_adjacent_element_beforeend() {
        let (mut dom, parent, child, mut session) = setup();
        dom.append_child(parent, child);

        let new_elem = dom.create_element("p", Attributes::default());
        session.get_or_create_wrapper(new_elem, ComponentKind::Element);
        let new_ref = session
            .get_or_create_wrapper(new_elem, ComponentKind::Element)
            .to_raw();

        InsertAdjacentElement
            .invoke(
                parent,
                &[
                    JsValue::String("beforeend".into()),
                    JsValue::ObjectRef(new_ref),
                ],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let children = dom.children(parent);
        assert_eq!(children[0], child);
        assert_eq!(children[1], new_elem);
    }

    #[test]
    fn insert_adjacent_element_afterend() {
        let (mut dom, parent, _, mut session) = setup();
        let root = dom.create_element("body", Attributes::default());
        dom.append_child(root, parent);

        let new_elem = dom.create_element("p", Attributes::default());
        session.get_or_create_wrapper(new_elem, ComponentKind::Element);
        let new_ref = session
            .get_or_create_wrapper(new_elem, ComponentKind::Element)
            .to_raw();

        InsertAdjacentElement
            .invoke(
                parent,
                &[
                    JsValue::String("afterend".into()),
                    JsValue::ObjectRef(new_ref),
                ],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let children = dom.children(root);
        assert_eq!(children, vec![parent, new_elem]);
    }

    #[test]
    fn insert_adjacent_element_invalid_position() {
        let (mut dom, parent, child, mut session) = setup();
        let child_ref = session
            .get_or_create_wrapper(child, ComponentKind::Element)
            .to_raw();
        let err = InsertAdjacentElement
            .invoke(
                parent,
                &[
                    JsValue::String("invalid".into()),
                    JsValue::ObjectRef(child_ref),
                ],
                &mut session,
                &mut dom,
            )
            .unwrap_err();
        assert_eq!(err.kind, DomApiErrorKind::SyntaxError);
    }

    #[test]
    fn insert_adjacent_element_case_insensitive() {
        let (mut dom, parent, child, mut session) = setup();
        dom.append_child(parent, child);

        let new_elem = dom.create_element("p", Attributes::default());
        session.get_or_create_wrapper(new_elem, ComponentKind::Element);
        let new_ref = session
            .get_or_create_wrapper(new_elem, ComponentKind::Element)
            .to_raw();

        InsertAdjacentElement
            .invoke(
                parent,
                &[
                    JsValue::String("BeforeEnd".into()),
                    JsValue::ObjectRef(new_ref),
                ],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let children = dom.children(parent);
        assert_eq!(children[1], new_elem);
    }

    #[test]
    fn insert_adjacent_text_beforeend() {
        let (mut dom, parent, _, mut session) = setup();
        InsertAdjacentText
            .invoke(
                parent,
                &[
                    JsValue::String("beforeend".into()),
                    JsValue::String("hello".into()),
                ],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let text = collect_text_content(parent, &dom);
        assert_eq!(text, "hello");
    }

    // -----------------------------------------------------------------------
    // hasAttribute tests
    // -----------------------------------------------------------------------

    #[test]
    fn has_attribute_true() {
        let (mut dom, parent, _, mut session) = setup();
        {
            let mut attrs = dom.world_mut().get::<&mut Attributes>(parent).unwrap();
            attrs.set("id", "test");
        }
        let result = HasAttribute
            .invoke(
                parent,
                &[JsValue::String("id".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Bool(true));
    }

    #[test]
    fn has_attribute_false() {
        let (mut dom, parent, _, mut session) = setup();
        let result = HasAttribute
            .invoke(
                parent,
                &[JsValue::String("id".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Bool(false));
    }

    // -----------------------------------------------------------------------
    // toggleAttribute tests
    // -----------------------------------------------------------------------

    #[test]
    fn toggle_attribute_adds_when_absent() {
        let (mut dom, parent, _, mut session) = setup();
        let result = ToggleAttribute
            .invoke(
                parent,
                &[JsValue::String("hidden".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Bool(true));
        let attrs = dom.world().get::<&Attributes>(parent).unwrap();
        assert_eq!(attrs.get("hidden"), Some(""));
    }

    #[test]
    fn toggle_attribute_removes_when_present() {
        let (mut dom, parent, _, mut session) = setup();
        {
            let mut attrs = dom.world_mut().get::<&mut Attributes>(parent).unwrap();
            attrs.set("hidden", "");
        }
        let result = ToggleAttribute
            .invoke(
                parent,
                &[JsValue::String("hidden".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Bool(false));
        let attrs = dom.world().get::<&Attributes>(parent).unwrap();
        assert!(!attrs.contains("hidden"));
    }

    #[test]
    fn toggle_attribute_force_true() {
        let (mut dom, parent, _, mut session) = setup();
        let result = ToggleAttribute
            .invoke(
                parent,
                &[JsValue::String("hidden".into()), JsValue::Bool(true)],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Bool(true));
        let attrs = dom.world().get::<&Attributes>(parent).unwrap();
        assert!(attrs.contains("hidden"));
    }

    #[test]
    fn toggle_attribute_force_false() {
        let (mut dom, parent, _, mut session) = setup();
        {
            let mut attrs = dom.world_mut().get::<&mut Attributes>(parent).unwrap();
            attrs.set("hidden", "");
        }
        let result = ToggleAttribute
            .invoke(
                parent,
                &[JsValue::String("hidden".into()), JsValue::Bool(false)],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Bool(false));
        let attrs = dom.world().get::<&Attributes>(parent).unwrap();
        assert!(!attrs.contains("hidden"));
    }

    #[test]
    fn toggle_attribute_rejects_invalid_name() {
        let (mut dom, parent, _, mut session) = setup();
        let err = ToggleAttribute
            .invoke(
                parent,
                &[JsValue::String(String::new())],
                &mut session,
                &mut dom,
            )
            .unwrap_err();
        assert_eq!(err.kind, DomApiErrorKind::InvalidCharacterError);
    }

    // -----------------------------------------------------------------------
    // getAttributeNames tests
    // -----------------------------------------------------------------------

    #[test]
    fn get_attribute_names() {
        let (mut dom, parent, _, mut session) = setup();
        {
            let mut attrs = dom.world_mut().get::<&mut Attributes>(parent).unwrap();
            attrs.set("id", "x");
            attrs.set("class", "y");
        }
        let result = GetAttributeNames
            .invoke(parent, &[], &mut session, &mut dom)
            .unwrap();
        if let JsValue::String(s) = result {
            let names: Vec<&str> = s.split('\0').collect();
            assert!(names.contains(&"id"));
            assert!(names.contains(&"class"));
        } else {
            panic!("expected string");
        }
    }

    #[test]
    fn get_attribute_names_empty() {
        let (mut dom, parent, _, mut session) = setup();
        let result = GetAttributeNames
            .invoke(parent, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::String(String::new()));
    }

    // -----------------------------------------------------------------------
    // className getter/setter tests
    // -----------------------------------------------------------------------

    #[test]
    fn classname_get_set() {
        let (mut dom, parent, _, mut session) = setup();
        // Initially empty.
        let result = GetClassName
            .invoke(parent, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::String(String::new()));

        // Set.
        SetClassName
            .invoke(
                parent,
                &[JsValue::String("foo bar".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let result = GetClassName
            .invoke(parent, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::String("foo bar".into()));
    }

    // -----------------------------------------------------------------------
    // id getter/setter tests
    // -----------------------------------------------------------------------

    #[test]
    fn id_get_set() {
        let (mut dom, parent, _, mut session) = setup();
        let result = GetId.invoke(parent, &[], &mut session, &mut dom).unwrap();
        assert_eq!(result, JsValue::String(String::new()));

        SetId
            .invoke(
                parent,
                &[JsValue::String("main".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let result = GetId.invoke(parent, &[], &mut session, &mut dom).unwrap();
        assert_eq!(result, JsValue::String("main".into()));
    }

    // -----------------------------------------------------------------------
    // data_attr_to_camel / camel_to_data_attr tests
    // -----------------------------------------------------------------------

    #[test]
    fn data_attr_to_camel_basic() {
        assert_eq!(data_attr_to_camel("data-foo-bar"), "fooBar");
        assert_eq!(data_attr_to_camel("data-x"), "x");
        assert_eq!(data_attr_to_camel("data-foo-bar-baz"), "fooBarBaz");
    }

    #[test]
    fn camel_to_data_attr_basic() {
        assert_eq!(camel_to_data_attr("fooBar"), "data-foo-bar");
        assert_eq!(camel_to_data_attr("x"), "data-x");
        assert_eq!(camel_to_data_attr("fooBarBaz"), "data-foo-bar-baz");
    }

    #[test]
    fn data_attr_roundtrip() {
        let camel = data_attr_to_camel("data-my-value");
        let attr = camel_to_data_attr(&camel);
        assert_eq!(attr, "data-my-value");
    }

    // -----------------------------------------------------------------------
    // dataset tests
    // -----------------------------------------------------------------------

    #[test]
    fn dataset_set_and_get() {
        let (mut dom, parent, _, mut session) = setup();
        DatasetSet
            .invoke(
                parent,
                &[
                    JsValue::String("fooBar".into()),
                    JsValue::String("42".into()),
                ],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let result = DatasetGet
            .invoke(
                parent,
                &[JsValue::String("fooBar".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::String("42".into()));

        // Verify it's stored as data-foo-bar.
        let attrs = dom.world().get::<&Attributes>(parent).unwrap();
        assert_eq!(attrs.get("data-foo-bar"), Some("42"));
    }

    #[test]
    fn dataset_get_missing() {
        let (mut dom, parent, _, mut session) = setup();
        let result = DatasetGet
            .invoke(
                parent,
                &[JsValue::String("missing".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Undefined);
    }

    #[test]
    fn dataset_delete() {
        let (mut dom, parent, _, mut session) = setup();
        {
            let mut attrs = dom.world_mut().get::<&mut Attributes>(parent).unwrap();
            attrs.set("data-foo-bar", "val");
        }
        DatasetDelete
            .invoke(
                parent,
                &[JsValue::String("fooBar".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let attrs = dom.world().get::<&Attributes>(parent).unwrap();
        assert!(!attrs.contains("data-foo-bar"));
    }

    #[test]
    fn dataset_keys() {
        let (mut dom, parent, _, mut session) = setup();
        {
            let mut attrs = dom.world_mut().get::<&mut Attributes>(parent).unwrap();
            attrs.set("data-x", "1");
            attrs.set("data-foo-bar", "2");
            attrs.set("class", "ignore");
        }
        let result = DatasetKeys
            .invoke(parent, &[], &mut session, &mut dom)
            .unwrap();
        if let JsValue::String(s) = result {
            let keys: Vec<&str> = s.split('\0').collect();
            assert_eq!(keys.len(), 2);
            assert!(keys.contains(&"x"));
            assert!(keys.contains(&"fooBar"));
        } else {
            panic!("expected string");
        }
    }

    #[test]
    fn toggle_attribute_rev_version() {
        let (mut dom, parent, _child, mut session) = setup();
        let v1 = dom.inclusive_descendants_version(parent);
        ToggleAttribute
            .invoke(
                parent,
                &[JsValue::String("hidden".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let v2 = dom.inclusive_descendants_version(parent);
        assert_ne!(v1, v2);
    }

    #[test]
    fn set_class_name_rev_version() {
        let (mut dom, parent, _child, mut session) = setup();
        let v1 = dom.inclusive_descendants_version(parent);
        SetClassName
            .invoke(
                parent,
                &[JsValue::String("foo".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let v2 = dom.inclusive_descendants_version(parent);
        assert_ne!(v1, v2);
    }

    #[test]
    fn set_id_rev_version() {
        let (mut dom, parent, _child, mut session) = setup();
        let v1 = dom.inclusive_descendants_version(parent);
        SetId
            .invoke(
                parent,
                &[JsValue::String("myid".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let v2 = dom.inclusive_descendants_version(parent);
        assert_ne!(v1, v2);
    }

    #[test]
    fn dataset_set_rev_version() {
        let (mut dom, parent, _child, mut session) = setup();
        let v1 = dom.inclusive_descendants_version(parent);
        DatasetSet
            .invoke(
                parent,
                &[JsValue::String("foo".into()), JsValue::String("bar".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let v2 = dom.inclusive_descendants_version(parent);
        assert_ne!(v1, v2);
    }

    #[test]
    fn dataset_delete_rev_version() {
        let (mut dom, parent, _child, mut session) = setup();
        {
            let mut attrs = dom.world_mut().get::<&mut Attributes>(parent).unwrap();
            attrs.set("data-foo", "bar");
        }
        let v1 = dom.inclusive_descendants_version(parent);
        DatasetDelete
            .invoke(
                parent,
                &[JsValue::String("foo".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let v2 = dom.inclusive_descendants_version(parent);
        assert_ne!(v1, v2);
    }

    #[test]
    fn data_attr_to_camel_non_lowercase() {
        // Dash followed by non-lowercase should preserve dash + char.
        assert_eq!(data_attr_to_camel("data-foo-Bar"), "foo-Bar");
        assert_eq!(data_attr_to_camel("data-foo-1"), "foo-1");
        assert_eq!(data_attr_to_camel("data-foo-bar"), "fooBar");
        // Trailing dash should be preserved.
        assert_eq!(data_attr_to_camel("data-foo-"), "foo-");
    }
}
