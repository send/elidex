//! Element selector methods: matches, closest.

use elidex_css::parse_selector_from_str;
use elidex_ecs::{EcsDom, Entity, TagType};
use elidex_plugin::JsValue;
use elidex_script_session::{
    ComponentKind, DomApiError, DomApiErrorKind, DomApiHandler, SessionCore,
};

use crate::document::reject_shadow_pseudos;
use crate::util::require_string_arg;

/// `element.matches(selector)` — returns true if this element matches the selector.
pub struct Matches;

impl DomApiHandler for Matches {
    fn method_name(&self) -> &str {
        "matches"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let selector_str = require_string_arg(args, 0)?;
        let selectors = parse_selector_from_str(&selector_str).map_err(|()| DomApiError {
            kind: DomApiErrorKind::SyntaxError,
            message: format!("Invalid selector: {selector_str}"),
        })?;
        // CSS Scoping §3: `:host` / `::slotted()` are only valid inside
        // a shadow tree.  Browsers throw `DOMException("SyntaxError")`
        // from `matches` / `closest` when the selector uses these
        // pseudos against a non-shadow root.
        reject_shadow_pseudos(&selectors)?;
        let matched = selectors.iter().any(|sel| sel.matches(this, dom));
        Ok(JsValue::Bool(matched))
    }
}

// ---------------------------------------------------------------------------
// Element selector methods: closest
// ---------------------------------------------------------------------------

/// `element.closest(selector)` — returns the closest ancestor (including self)
/// that matches the selector, or null.
pub struct Closest;

impl DomApiHandler for Closest {
    fn method_name(&self) -> &str {
        "closest"
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
        // Same shadow-pseudo rejection as `matches` / `querySelector`
        // (CSS Scoping §3).
        reject_shadow_pseudos(&selectors)?;

        // Walk self → parent ancestors, returning the first matching
        // Element.  WHATWG §4.9 closest() is inclusive and stops at
        // the first non-Element parent — this is also how the walk
        // honours the shadow boundary, since `ShadowRoot` carries no
        // `TagType` (only `Element`s have it) so a walk from inside a
        // shadow tree does not climb to the host.  The Document root
        // also has no `TagType`, so the walk stops there in the
        // normal case too.
        let mut current = Some(this);
        while let Some(entity) = current {
            let is_element = dom.world().get::<&TagType>(entity).is_ok();
            if is_element && selectors.iter().any(|sel| sel.matches(entity, dom)) {
                let obj_ref = session.get_or_create_wrapper(entity, ComponentKind::Element);
                return Ok(JsValue::ObjectRef(obj_ref.to_raw()));
            }
            // Stop the walk at the first non-Element parent so the
            // shadow boundary (ShadowRoot has no TagType) is not
            // crossed.  Without this filter, closest() inside a shadow
            // tree would silently match elements in the light tree.
            current = dom
                .get_parent(entity)
                .filter(|p| dom.world().get::<&TagType>(*p).is_ok());
        }

        Ok(JsValue::Null)
    }
}
