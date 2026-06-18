//! Attribute get/set/remove handlers.

use elidex_ecs::{AttrEntityCache, EcsDom, Entity};
use elidex_plugin::JsValue;
use elidex_script_session::{DomApiError, DomApiHandler, SessionCore};

use super::tree::validate_attribute_name;
use crate::util::{require_attrs, require_live_element, require_string_arg};

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
        let name = raw_name.to_ascii_lowercase();
        let value = require_string_arg(args, 1)?;
        // Lesson #181: route through the canonical `EcsDom::set_attribute`
        // chokepoint so `MutationEvent::AttributeChange` fires + the
        // attribute-mutation revision bump runs (essential for D-31
        // `BaseUrlMaintainer` to react to `<base>.href` writes).  Pre-D-31
        // this handler wrote `Attributes::set` directly and bumped
        // `rev_version` separately — the chokepoint subsumes both.
        // The `EcsDom::set_attribute` chokepoint also syncs any materialized
        // `Attr` node's value (so `getAttributeNode("x").value` reflects the
        // write while preserving identity) — see `EcsDom::sync_cached_attr_value`.
        if !dom.set_attribute(this, &name, &value) {
            return Err(crate::util::not_found_error("element not found"));
        }
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
        // Uniform with the rest of the Element attribute surface
        // (setAttribute / toggleAttribute / *AttributeNode): a stale /
        // non-Element receiver errors rather than silently no-op'ing —
        // `EcsDom::remove_attribute` short-circuits on such a receiver, so
        // guard before the chokepoint. A live Element with the attribute
        // merely absent stays a correct no-op (removeAttribute never throws
        // for a missing attribute per DOM).
        require_live_element(dom, this)?;
        // Route through the canonical `EcsDom::remove_attribute` chokepoint
        // (mirrors `SetAttribute` → `set_attribute`, lesson #181): it
        // invalidates the lazily-hydrated `InlineStyle` cache — otherwise a
        // prior `el.style.*` read materializes a cache that survives
        // `removeAttribute('style')` and resurrects the removed declaration
        // — and also bumps `rev_version` + dispatches
        // `MutationEvent::AttributeChange`, both of which the prior direct
        // `Attributes::remove` skipped.
        dom.remove_attribute(this, &name);
        // Invalidate Attr identity cache for this attribute (Attr-node
        // identity is an `elidex-dom-api` concern, not owned by the
        // chokepoint).
        if let Ok(mut cache) = dom.world_mut().get::<&mut AttrEntityCache>(this) {
            cache.entries.remove(&name);
        }
        Ok(JsValue::Undefined)
    }
}
