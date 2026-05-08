//! Shared helpers for form-control attribute setters that mirror the
//! parsed value into [`elidex_form::FormControlState`] alongside the
//! content-attribute write.
//!
//! ## Why this exists
//!
//! `<input>` and `<textarea>` both expose constraint-bearing attributes
//! (`disabled` / `required` / `readOnly` / `maxLength` / `minLength`)
//! whose IDL setters must do two things:
//!
//! 1. write through to the content attribute (so DOM-side serialization
//!    and CSS attribute selectors see the new value), and
//! 2. mirror the parsed value into the matching `FormControlState`
//!    field so subsequent
//!    [`elidex_form::validate_control`](elidex_form::validate_control)
//!    calls observe the constraint without requiring an attach pass.
//!
//! The two protos previously carried near-identical
//! `bool_attr_with_state_sync` / `length_set_with_state_sync` helpers;
//! consolidating them here removes the duplication and gives any
//! future form-control proto (datalist / output / progress / meter
//! per the T2 carve-out — see `m4-12-platform-gap-roadmap.md` §D-9)
//! the same wiring out of the box.
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", these helpers are pure
//! marshalling glue: the brand check (interface-specific
//! `require_input_receiver` / `require_textarea_receiver`) is
//! injected by the call site, and `apply` is the field-mutation
//! callback that writes a `FormControlState` field.  The reflection
//! rule itself (negative-value-clears-attr per HTML §6.13.1) lives
//! in this module since it's identical across protos.

#![cfg(feature = "engine")]
// Cast-sign-loss: every `as usize` conversion here is gated by an
// explicit `n < 0` branch so the cast is value-preserving.
#![allow(clippy::cast_sign_loss)]

use elidex_ecs::Entity;
use elidex_form::FormControlState;

use super::super::value::{JsValue, NativeContext, VmError};

/// `fn`-pointer alias for a per-interface brand check that returns
/// the bound entity on success or `None` when the receiver is bound
/// to a non-element prototype path.  Both
/// `html_input_proto::require_input_receiver` and
/// `html_textarea_proto::require_textarea_receiver` satisfy this
/// signature.
pub(super) type RequireReceiver =
    fn(&mut NativeContext<'_>, JsValue, &str) -> Result<Option<Entity>, VmError>;

/// Boolean reflect setter that ALSO mirrors into the matching
/// `FormControlState` field via `apply` after writing the content
/// attribute.
///
/// `attr` is the lowercase content-attribute name written through to
/// `EcsDom::set_attribute` (e.g. `"readonly"` even for the IDL alias
/// `readOnly`).
pub(super) fn bool_attr_with_state_sync<F>(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
    method: &str,
    attr: &str,
    require: RequireReceiver,
    apply: F,
) -> Result<JsValue, VmError>
where
    F: FnOnce(&mut FormControlState, bool),
{
    let Some(entity) = require(ctx, this, method)? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let flag = super::super::coerce::to_boolean(ctx.vm, val);
    if flag {
        ctx.host().dom().set_attribute(entity, attr, String::new());
    } else {
        super::element_attrs::attr_remove(ctx, entity, attr);
    }
    let dom = ctx.host().dom();
    if let Ok(mut state) = dom.world_mut().get::<&mut FormControlState>(entity) {
        apply(&mut state, flag);
    }
    Ok(JsValue::Undefined)
}

/// Length-attribute setter that mirrors the parsed value into the
/// matching `FormControlState` field so subsequent
/// `validate_control()` observes the constraint without a re-attach
/// (HTML §4.10.20.3).  Negative values map to `None` (no constraint)
/// per HTML §6.13.1 reflection rules — the content attribute is
/// removed rather than persisting an illegal `maxlength="-1"`.
pub(super) fn length_set_with_state_sync<F>(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
    method: &str,
    attr: &str,
    require: RequireReceiver,
    apply: F,
) -> Result<JsValue, VmError>
where
    F: FnOnce(&mut FormControlState, Option<usize>),
{
    let Some(entity) = require(ctx, this, method)? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = super::super::coerce::to_int32(ctx.vm, val)?;
    if n < 0 {
        super::element_attrs::attr_remove(ctx, entity, attr);
    } else {
        ctx.host().dom().set_attribute(entity, attr, n.to_string());
    }
    let dom = ctx.host().dom();
    if let Ok(mut state) = dom.world_mut().get::<&mut FormControlState>(entity) {
        apply(&mut state, if n < 0 { None } else { Some(n as usize) });
    }
    Ok(JsValue::Undefined)
}
