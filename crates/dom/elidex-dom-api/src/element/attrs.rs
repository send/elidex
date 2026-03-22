//! Attribute-related handlers: hasAttribute, toggleAttribute, getAttributeNames,
//! className, id, dataset.

use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::JsValue;
use elidex_script_session::{DomApiError, DomApiHandler, SessionCore};

use crate::util::{require_attrs, require_attrs_mut, require_string_arg};
use super::tree::validate_attribute_name;

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
