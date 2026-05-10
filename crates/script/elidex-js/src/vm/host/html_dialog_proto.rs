//! `HTMLDialogElement.prototype` intrinsic — per-tag prototype layer
//! for `<dialog>` wrappers (HTML §4.11.4, slot
//! `#11-tags-T2d-interactive`).
//!
//! IDL surface (HTML §4.11.4):
//! - `open` — boolean reflect of the `open` content attribute.
//! - `returnValue` — DOMString state, stored in
//!   [`elidex_ecs::DialogReturnValue`].  Defaulted to the empty string
//!   until first set/closed-with-arg.
//! - `show()` — open as non-modal.  M4-12 stub: throw
//!   `InvalidStateError` if already open as modal (per
//!   [`elidex_ecs::IsModalDialog`] marker).
//! - `showModal()` — open as modal: set `open` content attribute and
//!   insert `IsModalDialog` ECS marker.  Throws `InvalidStateError`
//!   if already open as modal.  Render-side top-layer / focus
//!   management is deferred to slot `#11-dialog-top-layer` (Phase 4).
//!   The "in document tree" check is deferred to
//!   `#11-dialog-tree-check`.
//! - `close(optional DOMString returnValue)` — clear `open`, clear
//!   `IsModalDialog` marker, set `returnValue` if arg provided,
//!   dispatch a `close` event (bubbles=false, cancelable=false) via
//!   [`super::event_target_dispatch::dispatch_simple_event`].
//!
//! Event-handler IDL attrs (`oncancel` / `onclose`) are deferred along
//! with the rest of the per-tag event-handler reflects (D-10
//! `#11-events-misc`); `<form method="dialog">` integration is
//! deferred to `#11-dialog-form-method`.
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", marshalling-only.  ECS marker /
//! state writes are direct world inserts (engine-indep API) — there is
//! no spec algorithm to delegate.  Event firing routes through
//! engine-bound `dispatch_simple_event` (it lives in
//! `vm/host/event_target_dispatch.rs`).

#![cfg(feature = "engine")]

use elidex_ecs::{DialogReturnValue, Entity, IsModalDialog, NodeKind};

use super::super::shape;
use super::super::value::{JsValue, NativeContext, VmError};
use super::super::VmInner;
use super::dom_bridge::invoke_dom_api;

impl VmInner {
    pub(in crate::vm) fn register_html_dialog_prototype(&mut self) {
        let parent = self
            .html_element_prototype
            .expect("register_html_dialog_prototype called before register_html_element_prototype");
        let proto_id = self.alloc_html_subclass_prototype(parent);
        self.html_dialog_prototype = Some(proto_id);

        let attrs = shape::PropertyAttrs::WEBIDL_RO_ACCESSOR;
        let open_sid = self.strings.intern("open");
        self.install_accessor_pair(
            proto_id,
            open_sid,
            dialog_get_open,
            Some(dialog_set_open),
            attrs,
        );
        let return_value_sid = self.strings.intern("returnValue");
        self.install_accessor_pair(
            proto_id,
            return_value_sid,
            dialog_get_return_value,
            Some(dialog_set_return_value),
            attrs,
        );

        let method_attrs = shape::PropertyAttrs::METHOD;
        let show_sid = self.strings.intern("show");
        self.install_native_method(proto_id, show_sid, dialog_show, method_attrs);
        let show_modal_sid = self.strings.intern("showModal");
        self.install_native_method(proto_id, show_modal_sid, dialog_show_modal, method_attrs);
        let close_sid = self.strings.intern("close");
        self.install_native_method(proto_id, close_sid, dialog_close, method_attrs);
    }
}

fn require_dialog_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "HTMLDialogElement", method, |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(None);
    };
    if !ctx.host().tag_matches_ascii_case(entity, "dialog") {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLDialogElement': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

// ---------------------------------------------------------------------------
// open / returnValue accessors
// ---------------------------------------------------------------------------

fn dialog_get_open(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_dialog_receiver(ctx, this, "open")? else {
        return Ok(JsValue::Boolean(false));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Boolean(false));
    }
    let attr_sid = ctx.vm.strings.intern("open");
    invoke_dom_api(ctx, "hasAttribute", entity, &[JsValue::String(attr_sid)])
}

fn dialog_set_open(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_dialog_receiver(ctx, this, "open")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let truthy = super::super::coerce::to_boolean(ctx.vm, val);
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    let attr_sid = ctx.vm.strings.intern("open");
    if truthy {
        let empty_sid = ctx.vm.well_known.empty;
        invoke_dom_api(
            ctx,
            "setAttribute",
            entity,
            &[JsValue::String(attr_sid), JsValue::String(empty_sid)],
        )
    } else {
        // Clearing `open` content attribute via the IDL setter does
        // NOT itself fire the `close` event (per HTML §4.11.4 — only
        // the `close()` method dispatches the event).  It does clear
        // the modal marker, since the dialog is no longer open.
        let _ = ctx
            .host()
            .dom()
            .world_mut()
            .remove_one::<IsModalDialog>(entity);
        invoke_dom_api(ctx, "removeAttribute", entity, &[JsValue::String(attr_sid)])
    }
}

fn dialog_get_return_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_dialog_receiver(ctx, this, "returnValue")? else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    let value = ctx
        .host()
        .dom()
        .world()
        .get::<&DialogReturnValue>(entity)
        .map(|drv| drv.0.clone())
        .unwrap_or_default();
    let sid = ctx.vm.strings.intern(value.as_str());
    Ok(JsValue::String(sid))
}

fn dialog_set_return_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_dialog_receiver(ctx, this, "returnValue")? else {
        return Ok(JsValue::Undefined);
    };
    let raw = args.first().copied().unwrap_or(JsValue::Undefined);
    let new_value_sid = super::super::coerce::to_string(ctx.vm, raw)?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    let owned = ctx.vm.strings.get_utf8(new_value_sid);
    write_return_value(ctx, entity, owned);
    Ok(JsValue::Undefined)
}

fn write_return_value(ctx: &mut NativeContext<'_>, entity: Entity, value: String) {
    let world = ctx.host().dom().world_mut();
    if let Ok(mut existing) = world.get::<&mut DialogReturnValue>(entity) {
        existing.0 = value;
        return;
    }
    let _ = world.insert_one(entity, DialogReturnValue(value));
}

// ---------------------------------------------------------------------------
// show / showModal / close
// ---------------------------------------------------------------------------

fn dialog_show(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_dialog_receiver(ctx, this, "show")? else {
        return Ok(JsValue::Undefined);
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    if has_modal_marker(ctx, entity) {
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_invalid_state_error,
            "Failed to execute 'show' on 'HTMLDialogElement': \
             The element already has an 'open' attribute, and is in a modal state.",
        ));
    }
    set_open_attribute(ctx, entity)
}

fn dialog_show_modal(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_dialog_receiver(ctx, this, "showModal")? else {
        return Ok(JsValue::Undefined);
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    let already_open = has_open_attribute(ctx, entity)?;
    if already_open {
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_invalid_state_error,
            "Failed to execute 'showModal' on 'HTMLDialogElement': \
             The element already has an 'open' attribute.",
        ));
    }
    let _ = ctx
        .host()
        .dom()
        .world_mut()
        .insert_one(entity, IsModalDialog);
    set_open_attribute(ctx, entity)
}

fn dialog_close(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_dialog_receiver(ctx, this, "close")? else {
        return Ok(JsValue::Undefined);
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    // Per HTML §4.11.4 step 1: if the element does not have an `open`
    // content attribute, return early with no event fire and no state
    // mutation.
    if !has_open_attribute(ctx, entity)? {
        return Ok(JsValue::Undefined);
    }
    if let Some(arg) = args.first().copied() {
        if !matches!(arg, JsValue::Undefined) {
            let new_value_sid = super::super::coerce::to_string(ctx.vm, arg)?;
            let owned = ctx.vm.strings.get_utf8(new_value_sid);
            write_return_value(ctx, entity, owned);
        }
    }
    let _ = ctx
        .host()
        .dom()
        .world_mut()
        .remove_one::<IsModalDialog>(entity);
    let attr_sid = ctx.vm.strings.intern("open");
    invoke_dom_api(ctx, "removeAttribute", entity, &[JsValue::String(attr_sid)])?;
    let close_sid = ctx.vm.well_known.close;
    let _ = super::event_target_dispatch::dispatch_simple_event(
        ctx, entity, close_sid, /*bubbles=*/ false, /*cancelable=*/ false,
    )?;
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn has_modal_marker(ctx: &mut NativeContext<'_>, entity: Entity) -> bool {
    ctx.host()
        .dom()
        .world()
        .get::<&IsModalDialog>(entity)
        .is_ok()
}

fn has_open_attribute(ctx: &mut NativeContext<'_>, entity: Entity) -> Result<bool, VmError> {
    let attr_sid = ctx.vm.strings.intern("open");
    let result = invoke_dom_api(ctx, "hasAttribute", entity, &[JsValue::String(attr_sid)])?;
    Ok(matches!(result, JsValue::Boolean(true)))
}

fn set_open_attribute(ctx: &mut NativeContext<'_>, entity: Entity) -> Result<JsValue, VmError> {
    let attr_sid = ctx.vm.strings.intern("open");
    let empty_sid = ctx.vm.well_known.empty;
    invoke_dom_api(
        ctx,
        "setAttribute",
        entity,
        &[JsValue::String(attr_sid), JsValue::String(empty_sid)],
    )?;
    Ok(JsValue::Undefined)
}
