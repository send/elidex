//! Shared helpers for form-control attribute setters that route through
//! the [`EcsDom::set_attribute`] / [`EcsDom::remove_attribute`]
//! chokepoint (via the VM-layer [`element_attrs::attr_remove`] wrapper
//! for the removal branch).
//!
//! ## Why this exists
//!
//! `<input>` and `<textarea>` both expose constraint-bearing attributes
//! (`disabled` / `required` / `readOnly` / `maxLength` / `minLength`)
//! whose IDL setters share two non-trivial branches: the boolean-
//! presence reflect rule (`true` ⇒ set empty-string attr, `false` ⇒
//! remove attr) and the long-reflect negative-clears rule (HTML
//! §6.13.1) for `maxlength` / `minlength`.  Consolidating the wiring
//! here gives any future form-control proto (datalist / output /
//! progress / meter per the T2 carve-out — see
//! `m4-12-platform-gap-roadmap.md` §D-9) the same path out of the box.
//!
//! These helpers perform content-attribute reflection only.
//! `FormControlState` mirroring lives in
//! [`FormControlReconciler`](elidex_form::FormControlReconciler) (a
//! `MutationEvent::AttributeChange` consumer composed into the VM's
//! `ConsumerDispatcher`), which observes the mutations fired by the
//! `EcsDom::set_attribute` / `EcsDom::remove_attribute` chokepoint and
//! re-derives
//! FCS fields uniformly across IDL setter / `setAttribute` / parser /
//! `innerHTML` paths.  Module / function names below therefore reflect
//! attribute-reflection responsibility, NOT FCS sync (which was the
//! pre-D-31 placement that these helpers carried in their original
//! `_with_state_sync` suffix).
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", these helpers are pure
//! marshalling glue: the brand check (interface-specific
//! `require_input_receiver` / `require_textarea_receiver`) is
//! injected by the call site.  The reflection rule (negative-value-
//! clears-attr per HTML §6.13.1) lives in this module since it's
//! identical across protos.

#![cfg(feature = "engine")]
// Cast-sign-loss: every `as usize` conversion here is gated by an
// explicit `n < 0` branch so the cast is value-preserving.
#![allow(clippy::cast_sign_loss)]

use elidex_ecs::Entity;

use super::super::value::{JsValue, NativeContext, VmError};

/// `fn`-pointer alias for a per-interface brand check that returns
/// the bound entity on success or `None` when the receiver is bound
/// to a non-element prototype path.  Both
/// `html_input_proto::require_input_receiver` and
/// `html_textarea_proto::require_textarea_receiver` satisfy this
/// signature.
pub(super) type RequireReceiver =
    fn(&mut NativeContext<'_>, JsValue, &str) -> Result<Option<Entity>, VmError>;

/// Boolean-presence content-attribute setter (HTML §2.5.2 boolean
/// attribute rule — `true` ⇒ presence ⇒ empty string; `false` ⇒
/// absence).
///
/// `attr` is the lowercase content-attribute name written through to
/// `EcsDom::set_attribute` (e.g. `"readonly"` even for the IDL alias
/// `readOnly`).  `FormControlState` mirroring is done downstream by
/// the [`FormControlReconciler`](elidex_form::FormControlReconciler)
/// consumer of the `MutationEvent::AttributeChange` fired by the
/// chokepoint.
pub(super) fn bool_attr_reflect(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
    method: &str,
    attr: &str,
    require: RequireReceiver,
) -> Result<JsValue, VmError> {
    let Some(entity) = require(ctx, this, method)? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let flag = super::super::coerce::to_boolean(ctx.vm, val);
    if flag {
        ctx.host().dom().set_attribute(entity, attr, "");
    } else {
        super::element_attrs::attr_remove(ctx, entity, attr);
    }
    Ok(JsValue::Undefined)
}

/// Long-reflect content-attribute setter (HTML §6.13.1) — negative
/// values remove the attribute rather than persisting an illegal
/// `maxlength="-1"`.  `FormControlState` mirroring is done
/// downstream by the [`FormControlReconciler`](elidex_form::FormControlReconciler)
/// consumer of the `MutationEvent::AttributeChange` fired by the
/// chokepoint.
pub(super) fn length_attr_reflect(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
    method: &str,
    attr: &str,
    require: RequireReceiver,
) -> Result<JsValue, VmError> {
    let Some(entity) = require(ctx, this, method)? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = super::super::coerce::to_int32(ctx.vm, val)?;
    if n < 0 {
        super::element_attrs::attr_remove(ctx, entity, attr);
    } else {
        ctx.host().dom().set_attribute(entity, attr, &n.to_string());
    }
    Ok(JsValue::Undefined)
}
