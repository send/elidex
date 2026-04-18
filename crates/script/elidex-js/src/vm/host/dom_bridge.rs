//! Helpers shared across host-side DOM natives — wrapper lifting
//! and selector parsing.
//!
//! These existed as file-local `fn`s in `document.rs` and
//! `element_proto.rs` before they grew a second consumer.  Keeping
//! them in one place avoids the near-identical copies drifting over
//! time (each had seven call sites between the two files).

#![cfg(feature = "engine")]

use super::super::value::{JsValue, NativeContext, VmError};
use super::super::VmInner;

use elidex_css::{parse_selector_from_str, Selector};
use elidex_ecs::Entity;

/// Return `Option<Entity>` as a JS wrapper or `null` — no intermediate
/// `ObjectId`, so callers can chain it straight into a `Result::Ok`.
pub(super) fn wrap_entity_or_null(vm: &mut VmInner, entity: Option<Entity>) -> JsValue {
    match entity {
        Some(e) => JsValue::Object(vm.create_element_wrapper(e)),
        None => JsValue::Null,
    }
}

/// Wrap a list of entities as a JS Array of element wrappers.  One
/// allocation for the intermediate `Vec<JsValue>`, one for the
/// Array object.
pub(super) fn wrap_entities_as_array(vm: &mut VmInner, entities: &[Entity]) -> JsValue {
    let elements: Vec<JsValue> = entities
        .iter()
        .map(|&e| JsValue::Object(vm.create_element_wrapper(e)))
        .collect();
    JsValue::Object(vm.create_array_object(elements))
}

/// Parse a selector string and reject shadow-scoped pseudos.  Shared
/// by `document.querySelector*` and `Element.prototype.matches` /
/// `closest` — all four throw `SyntaxError` on invalid input and on
/// `:host` / `::slotted()`, which are only valid inside shadow-tree
/// context.
///
/// The `method` name appears in the shadow-pseudo error message so
/// callers get a call-site-accurate complaint (`… are not valid in
/// querySelector` vs `… in matches/closest`).
pub(super) fn parse_dom_selector(
    selector_str: &str,
    shadow_method_label: &str,
) -> Result<Vec<Selector>, VmError> {
    let selectors = parse_selector_from_str(selector_str)
        .map_err(|()| VmError::syntax_error(format!("Invalid selector: {selector_str}")))?;
    if selectors.iter().any(|s| s.has_shadow_pseudo()) {
        return Err(VmError::syntax_error(format!(
            ":host and ::slotted() are not valid in {shadow_method_label}"
        )));
    }
    Ok(selectors)
}

/// Coerce the first argument to a string and hand back its UTF-8
/// materialisation — the shape every selector-accepting native
/// (querySelector, matches, closest, …) starts with.
pub(super) fn coerce_first_arg_to_string(
    ctx: &mut NativeContext<'_>,
    args: &[JsValue],
) -> Result<String, VmError> {
    let arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = super::super::coerce::to_string(ctx.vm, arg)?;
    Ok(ctx.vm.strings.get_utf8(sid))
}
