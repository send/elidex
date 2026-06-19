//! `HTMLDialogElement.prototype` intrinsic — per-tag prototype layer
//! for `<dialog>` wrappers (HTML §4.11.4, slot
//! `#11-tags-T2d-interactive`).
//!
//! IDL surface (HTML §4.11.4):
//! - `open` — boolean reflect of the `open` content attribute.
//! - `returnValue` — DOMString state, stored in
//!   [`elidex_ecs::DialogReturnValue`].  Defaulted to the empty string
//!   until first set/closed-with-arg.
//! - `show()` — open as non-modal (HTML §4.11.4 `show()`): step 2
//!   already-open-modal → throw `InvalidStateError` (per
//!   [`elidex_ecs::IsModalDialog`] marker); step 6 add `open`.  No
//!   connectedness requirement (a non-modal dialog may be shown while
//!   disconnected).
//! - `showModal()` — open as modal (HTML §4.11.4 "show a modal dialog"):
//!   step 2 already-open → throw `InvalidStateError`; step 4 **not
//!   connected → throw `InvalidStateError`** (delegated to the
//!   engine-independent `isConnected.get` DOM API, DOM §4.2.2
//!   "connected") — the core of slot `#11-dialog-tree-check`; then
//!   insert the `IsModalDialog` ECS marker + add `open`.  Step 3 ("not
//!   fully active") is unconditionally satisfied in the single-document
//!   VM and is folded into `#11-browsing-context-state-ecs-components`.
//! - **Deferred (`#11-dialog-top-layer`, depends on a reliable
//!   `is modal` flag → the dialog *removing* steps that reset it on tree
//!   removal):** the step-1 already-open **idempotent return** for both
//!   methods (without removing-steps, the marker can go stale after
//!   `removeChild`, so a step-1 no-op would wrongly succeed — we keep the
//!   conservative throw); the popover-showing check (step 5),
//!   `beforetoggle` (step 6), and render-side top-layer / focus
//!   management (steps 12+).
//! - `close(optional DOMString returnValue)` — delegates the state
//!   mutation (HTML §4.11.4 "close the dialog": set `returnValue` if the
//!   arg is non-undefined, clear `IsModalDialog`, remove `open`) to the
//!   engine-independent [`elidex_dom_api::close_the_dialog`], then
//!   dispatches a `close` event (bubbles=false, cancelable=false) via
//!   [`super::event_target_dispatch::dispatch_simple_event`] iff the
//!   dialog was open. The same `close_the_dialog` algorithm is shared by
//!   `<form method="dialog">` submission in the shell (one-issue-one-way).
//!
//! Event-handler IDL attrs (`oncancel` / `onclose`) are deferred along
//! with the rest of the per-tag event-handler reflects (D-10
//! `#11-events-misc`). `<form method="dialog">` submission shares the
//! engine-independent [`elidex_dom_api::close_the_dialog`] algorithm
//! (HTML §4.10.22.3 step 11; the shell fires the `close` event there,
//! mirroring this method).
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
    // HTML §4.11.4 `show()` step 2: already open as modal → throw.  The
    // step-1 already-open-non-modal idempotent return is deferred: it
    // depends on a reliable `is modal` flag, which requires modelling the
    // dialog removing steps (tree removal resets `is modal`) — deferred to
    // `#11-dialog-top-layer`.  Until then the marker can go stale, so we
    // keep the conservative throw rather than a wrong no-op.
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
    // HTML §4.11.4 "show a modal dialog" step 2: already open → throw.
    // The step-1 already-open-modal idempotent return is deferred (it
    // depends on a reliable `is modal` flag — see `dialog_show` — so the
    // marker can go stale after tree removal; `#11-dialog-top-layer`).
    // Step 2 precedes the connectedness check, so an open-but-disconnected
    // dialog reports the already-open error.
    if has_open_attribute(ctx, entity)? {
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_invalid_state_error,
            "Failed to execute 'showModal' on 'HTMLDialogElement': \
             The element already has an 'open' attribute.",
        ));
    }
    // Step 3 ("not fully active") is unconditionally satisfied in the
    // single-document VM (folded into
    // `#11-browsing-context-state-ecs-components`).
    // Step 4: not connected → throw.  Delegated to the engine-independent
    // `isConnected.get` DOM API (DOM §4.2.2 "connected").  This is the
    // core of slot `#11-dialog-tree-check`.
    if !is_connected(ctx, entity)? {
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_invalid_state_error,
            "Failed to execute 'showModal' on 'HTMLDialogElement': \
             The element is not connected to a document.",
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
    // Coerce the optional `returnValue` argument at the IDL boundary
    // (`close(optional DOMString)`): `undefined`/absent → null (leave
    // returnValue unchanged), else `ToString`. This precedes the
    // open-check inside `close_the_dialog`, so a `valueOf`/`toString`
    // side effect runs even when the dialog is already closed — matching
    // the IDL coercion-before-algorithm ordering.
    let result: Option<String> = match args.first().copied() {
        Some(arg) if !matches!(arg, JsValue::Undefined) => {
            let new_value_sid = super::super::coerce::to_string(ctx.vm, arg)?;
            Some(ctx.vm.strings.get_utf8(new_value_sid))
        }
        _ => None,
    };
    // HTML §4.11.4 "close the dialog" — state mutation (engine-indep:
    // step 1 open-check, step 9 returnValue, step 8 is-modal, step 5
    // remove `open` via the chokepoint). Returns whether the dialog was
    // open; we fire `close` (step 13) here iff it was, mirroring the
    // `reset_form` precedent (caller fires the DOM event).
    let closed = elidex_dom_api::close_the_dialog(ctx.host().dom(), entity, result.as_deref());
    if closed {
        let close_sid = ctx.vm.well_known.close;
        let _ = super::event_target_dispatch::dispatch_simple_event(
            ctx, entity, close_sid, /*bubbles=*/ false, /*cancelable=*/ false,
        )?;
    }
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

/// DOM §4.2.2 "connected" via the engine-independent `isConnected.get`
/// handler (its shadow-including root is a document).
fn is_connected(ctx: &mut NativeContext<'_>, entity: Entity) -> Result<bool, VmError> {
    let result = invoke_dom_api(ctx, "isConnected.get", entity, &[])?;
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
