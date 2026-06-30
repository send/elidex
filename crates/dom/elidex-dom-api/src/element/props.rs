//! Attribute get/set/remove handlers.

use elidex_ecs::{AttrEntityCache, EcsDom, Entity};
use elidex_plugin::JsValue;
use elidex_script_session::{
    apply_remove_attribute, apply_set_attribute, DomApiError, DomApiHandler, SessionCore,
};

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
        // DOM §4.9 get-an-attribute-by-name step 1: HTML-namespace-gated
        // lowercase (SVG / MathML local names stay case-preserved). The single
        // canonical `EcsDom::resolve_attribute_qname` resolver (B2-Slice-3).
        let raw = require_string_arg(args, 0)?;
        let name = dom.resolve_attribute_qname(this, &raw);
        let attrs = require_attrs(this, dom)?;
        match attrs.get(name.as_ref()) {
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
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let raw_name = require_string_arg(args, 0)?;
        validate_attribute_name(&raw_name)?;
        // DOM §4.9 setAttribute step 2: HTML-namespace-gated lowercase (the
        // single canonical resolver — SVG / MathML names case-preserved).
        let name = dom.resolve_attribute_qname(this, &raw_name);
        let value = require_string_arg(args, 1)?;
        // Lesson #181 / #341: the record-producing `apply_set_attribute` seam
        // routes through the canonical `EcsDom::set_attribute` chokepoint so
        // `MutationEvent::AttributeChange` fires + the attribute-mutation
        // revision bump runs (essential for D-31 `BaseUrlMaintainer` to react
        // to `<base>.href` writes) + any materialized `Attr` node's value
        // syncs (so `getAttributeNode("x").value` reflects the write while
        // preserving identity — see `EcsDom::sync_cached_attr_value`) — AND it
        // surfaces the §4.9 "attributes" MutationObserver record (B2-Slice-1).
        // `None` = the receiver was destroyed / not an Element (the chokepoint
        // short-circuits) → NotFoundError, mirroring the pre-record contract.
        match apply_set_attribute(dom, this, name.as_ref(), &value) {
            Some(record) => session.push_notify_record(record),
            None => return Err(crate::util::not_found_error("element not found")),
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
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        // WHATWG DOM §4.9 `removeAttribute(qualifiedName)` = "remove an
        // attribute given qualifiedName and this" — it does NOT validate the
        // qualified name (unlike `setAttribute` / `toggleAttribute`, whose
        // step 1 throws `InvalidCharacterError` for an invalid local name). An
        // invalid or absent name is a no-op here, never a throw. B2-Slice-1
        // converged the VM `removeAttribute` native onto this handler, which
        // surfaced + fixed the prior spec-wrong validate-on-remove. The name is
        // resolved through the single canonical
        // `EcsDom::resolve_attribute_qname` (HTML-namespace-gated lowercase,
        // SVG / MathML case-preserved) — B2-Slice-3 folds the whole surface.
        // The ONE resolved binding feeds BOTH the chokepoint remove key AND the
        // `AttrEntityCache` evict key (§8 I-CACHE-KEY: resolved-vs-raw mismatch
        // would leak a stale cached Attr).
        let raw = require_string_arg(args, 0)?;
        let name = dom.resolve_attribute_qname(this, &raw).into_owned();
        // Uniform with the rest of the Element attribute surface
        // (setAttribute / toggleAttribute / *AttributeNode): a stale /
        // non-Element receiver errors rather than silently no-op'ing —
        // `EcsDom::remove_attribute` short-circuits on such a receiver, so
        // guard before the chokepoint. A live Element with the attribute
        // merely absent stays a correct no-op (removeAttribute never throws
        // for a missing attribute per DOM).
        require_live_element(dom, this)?;
        // Route through the record-producing `apply_remove_attribute` seam →
        // the canonical `EcsDom::remove_attribute` chokepoint (mirrors
        // `SetAttribute` → `apply_set_attribute`, lesson #181): it invalidates
        // the lazily-hydrated `InlineStyle` cache — otherwise a prior
        // `el.style.*` read materializes a cache that survives
        // `removeAttribute('style')` and resurrects the removed declaration —
        // and also bumps `rev_version` + dispatches
        // `MutationEvent::AttributeChange`, both of which the prior direct
        // `Attributes::remove` skipped — AND surfaces the §4.9 "attributes"
        // record (B2-Slice-1). `None` = nothing removed: the attribute was
        // absent (a spec no-op; `require_live_element` already rejected a
        // dead / non-Element receiver above), so push a record only when one
        // was produced.
        if let Some(record) = apply_remove_attribute(dom, this, &name) {
            session.push_notify_record(record);
        }
        // Invalidate Attr identity cache for this attribute (Attr-node
        // identity is an `elidex-dom-api` concern, not owned by the
        // chokepoint).
        if let Ok(mut cache) = dom.world_mut().get::<&mut AttrEntityCache>(this) {
            cache.entries.remove(&name);
        }
        Ok(JsValue::Undefined)
    }
}
