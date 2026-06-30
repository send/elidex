//! Attribute-related handlers: hasAttribute, toggleAttribute, getAttributeNames,
//! className, id, dataset.

use elidex_ecs::{Attributes, EcsDom, Entity};
use elidex_plugin::JsValue;
use elidex_script_session::{
    apply_remove_attribute, apply_set_attribute, DomApiError, DomApiHandler, SessionCore,
};

use super::tree::validate_attribute_name;
use crate::util::{require_attrs, require_live_element, require_string_arg};

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
        // DOM §4.9 has-an-attribute-by-name step 1: HTML-namespace-gated
        // lowercase via the single canonical resolver (B2-Slice-3).
        let raw = require_string_arg(args, 0)?;
        let name = dom.resolve_attribute_qname(this, &raw);
        let attrs = require_attrs(this, dom)?;
        Ok(JsValue::Bool(attrs.contains(name.as_ref())))
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
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let raw_name = require_string_arg(args, 0)?;
        // §4.9 step 1 validates the qualified name (unlike removeAttribute);
        // keep that. The name is then resolved through the single canonical
        // `EcsDom::resolve_attribute_qname` (HTML-namespace-gated lowercase,
        // SVG / MathML case-preserved) — B2-Slice-3 folds the whole surface.
        validate_attribute_name(&raw_name)?;
        let name = dom.resolve_attribute_qname(this, &raw_name).into_owned();

        let force = match args.get(1) {
            Some(JsValue::Bool(b)) => Some(*b),
            _ => None,
        };

        // Preserve the "stale / non-Element receiver → NotFoundError"
        // contract uniformly across every branch. The `has` probe below
        // collapses both "live element, attribute absent" and "dead
        // receiver" to `false`, and the forced-removal branch reaches no
        // chokepoint that could report the dead receiver — so guard up
        // front (the prior `require_attrs_mut` borrow surfaced this error).
        require_live_element(dom, this)?;

        let has = dom
            .world()
            .get::<&Attributes>(this)
            .is_ok_and(|a| a.contains(&name));

        // Route the set/remove through the record-producing
        // `apply_set_attribute` / `apply_remove_attribute` seams — the
        // canonical `EcsDom` chokepoints (so `toggleAttribute('style')`
        // invalidates the lazily-hydrated `InlineStyle` cache and every toggle
        // bumps `rev_version` + dispatches `MutationEvent::AttributeChange`)
        // PLUS the §4.9 "attributes" MutationObserver record (B2-Slice-1).
        // Boolean attributes are stored with an empty value per the HTML
        // serialization of a present boolean attribute. The receiver is a
        // confirmed live Element and each arm only mutates when it flips
        // presence, so each `apply_*` returns `Some` in the arm that runs.
        let result = match force {
            Some(true) => {
                if !has {
                    if let Some(record) = apply_set_attribute(dom, this, &name, "") {
                        session.push_notify_record(record);
                    }
                }
                true
            }
            Some(false) => {
                if has {
                    if let Some(record) = apply_remove_attribute(dom, this, &name) {
                        session.push_notify_record(record);
                    }
                }
                false
            }
            None => {
                if has {
                    if let Some(record) = apply_remove_attribute(dom, this, &name) {
                        session.push_notify_record(record);
                    }
                    false
                } else {
                    if let Some(record) = apply_set_attribute(dom, this, &name, "") {
                        session.push_notify_record(record);
                    }
                    true
                }
            }
        };
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
        // Route through the canonical `EcsDom::set_attribute` chokepoint (not a
        // direct `Attributes` write) so the write bumps `rev_version` AND
        // dispatches `MutationEvent::AttributeChange` (DOM §4.9 "handle
        // attribute changes" → §4.3.2 queue a mutation record) — the prior
        // direct path skipped the mutation event. `require_live_element`
        // preserves the "stale / non-Element receiver → NotFoundError" contract
        // the chokepoint's silent-`false` return would otherwise drop (mirrors
        // `ToggleAttribute`).
        require_live_element(dom, this)?;
        dom.set_attribute(this, "class", &value);
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
        // Chokepoint-route (see `SetClassName`): fires `MutationEvent` + bumps
        // `rev_version`; `require_live_element` keeps the dead-receiver error.
        require_live_element(dom, this)?;
        dom.set_attribute(this, "id", &value);
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
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let key = require_string_arg(args, 0)?;
        let value = require_string_arg(args, 1)?;
        let attr_name = camel_to_data_attr(&key);
        // Record-producing chokepoint-route (B2-Slice-2, HTML §3.2.6.6
        // data-* attributes). `camel_to_data_attr` emits the converted
        // lowercase `data-*` local name, so the §4.9 "attributes" record's
        // `attributeName` is the content-attr name (`data-foo-bar`), NOT the
        // JS camelCase key (invariant I4). `apply_set_attribute` always
        // records on a landed write; push only — `invoke_dom_api` drains
        // (Phase 2.5).
        require_live_element(dom, this)?;
        if let Some(record) = apply_set_attribute(dom, this, &attr_name, &value) {
            session.push_notify_record(record);
        }
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
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let key = require_string_arg(args, 0)?;
        let attr_name = camel_to_data_attr(&key);
        // Record-producing chokepoint-route (B2-Slice-2, HTML §3.2.6.6
        // data-* attributes): `apply_remove_attribute` routes through
        // `EcsDom::remove_attribute` (fires `MutationEvent` only when the
        // attribute was actually present per DOM §4.9, bumps `rev_version`,
        // runs the reconcile seam) and builds the §4.9 step-1 record from the
        // removed value — recording ONLY when something was removed
        // (delete-of-absent → `None` → no record, invariant I11). Push only —
        // `invoke_dom_api` drains (Phase 2.5).
        require_live_element(dom, this)?;
        if let Some(record) = apply_remove_attribute(dom, this, &attr_name) {
            session.push_notify_record(record);
        }
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
