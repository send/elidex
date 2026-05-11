//! `HTMLDetailsElement.prototype` intrinsic — per-tag prototype layer
//! for `<details>` wrappers (HTML §4.11.1, slot
//! `#11-tags-T2d-interactive`).
//!
//! IDL surface (HTML §4.11.1):
//! - `open` — boolean reflect of the `open` content attribute.  D-10
//!   wires the spec-mandated `ToggleEvent` fire on open-state change
//!   plus the `<details>.name` multi-disclosure exclusion algorithm
//!   (HTML §4.11.1 — opening a named `<details>` auto-closes other
//!   open `<details>` in the same tree with the byte-equal `name`).
//! - `name` — DOMString reflect of the `name` content attribute
//!   (current spec, accordion-style multi-disclosure groups).  Not
//!   deprecated, so in scope per the core/compat/deprecated tiering
//!   (`docs/design/ja/14-script-engines-webapi.md` §14.1.1 + §14.4.2).
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", VM-side marshalling +
//! dispatch glue.  The pure tree-walk for multi-disclosure exclusion
//! (`collect_open_details_by_name`) lives in `elidex-dom-api` per the
//! engine-indep mandate.

#![cfg(feature = "engine")]

use elidex_dom_api::element::details_exclusion::collect_open_details_by_name;
use elidex_ecs::{Attributes, Entity, NodeKind};

use super::super::shape;
use super::super::value::{JsValue, NativeContext, VmError};
use super::super::VmInner;
use super::dom_bridge::{coerce_first_arg_to_string_id, invoke_dom_api};
use super::event_target_dispatch::dispatch_toggle_event;

impl VmInner {
    pub(in crate::vm) fn register_html_details_prototype(&mut self) {
        let parent = self.html_element_prototype.expect(
            "register_html_details_prototype called before register_html_element_prototype",
        );
        let proto_id = self.alloc_html_subclass_prototype(parent);
        self.html_details_prototype = Some(proto_id);

        let attrs = shape::PropertyAttrs::WEBIDL_RO_ACCESSOR;
        let open_sid = self.strings.intern("open");
        self.install_accessor_pair(
            proto_id,
            open_sid,
            details_get_open,
            Some(details_set_open),
            attrs,
        );
        let name_sid = self.strings.intern("name");
        self.install_accessor_pair(
            proto_id,
            name_sid,
            details_get_name,
            Some(details_set_name),
            attrs,
        );
    }
}

fn require_details_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "HTMLDetailsElement", method, |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(None);
    };
    if !ctx.host().tag_matches_ascii_case(entity, "details") {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLDetailsElement': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

fn details_get_open(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_details_receiver(ctx, this, "open")? else {
        return Ok(JsValue::Boolean(false));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Boolean(false));
    }
    let attr_sid = ctx.vm.strings.intern("open");
    invoke_dom_api(ctx, "hasAttribute", entity, &[JsValue::String(attr_sid)])
}

fn details_set_open(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_details_receiver(ctx, this, "open")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let new_open = super::super::coerce::to_boolean(ctx.vm, val);
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    // 1. Read prior open state via attribute presence.
    let attr_sid = ctx.vm.strings.intern("open");
    let prior_open = matches!(
        invoke_dom_api(ctx, "hasAttribute", entity, &[JsValue::String(attr_sid)])?,
        JsValue::Boolean(true)
    );
    // 2. State unchanged → no-op (spec §4.11.1 — both attribute write
    //    and ToggleEvent fire skipped).  Idempotency is observable:
    //    `d.open = true; d.open = true` fires ToggleEvent exactly once.
    if prior_open == new_open {
        return Ok(JsValue::Undefined);
    }
    // 3. Multi-disclosure exclusion — opening a named `<details>` closes
    //    every other open `<details>` in the same tree with the same
    //    byte-equal `name`.  Pre-collect the snapshot BEFORE the close
    //    loop so listener mutations during one sibling's ToggleEvent
    //    dispatch don't re-enter the outer loop.
    let siblings_to_close: Vec<Entity> = if new_open {
        let owned_name: Option<String> = ctx
            .host()
            .dom()
            .world()
            .get::<&Attributes>(entity)
            .ok()
            .and_then(|attrs| attrs.get("name").map(str::to_owned))
            .filter(|n| !n.is_empty());
        if let Some(name) = owned_name {
            let dom = ctx.host().dom();
            let root = dom.find_tree_root(entity);
            collect_open_details_by_name(dom, root, &name, entity)
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };
    // 4. Close each sibling + fire ToggleEvent on it.  Use the raw
    //    `removeAttribute` DOM call (NOT the JS setter) so we don't
    //    re-enter `details_set_open` recursively — exclusion is a
    //    direct attribute mutation per HTML §4.11.1.
    //
    //    The snapshot reflects the state at exclusion start, but a
    //    prior sibling's `toggle` listener can mutate other siblings
    //    in the snapshot directly (e.g. `b.removeAttribute('open')`
    //    or `b.open = false`).  Per HTML §4.11.1 each step is gated
    //    on "if it is open" — re-check the live attribute presence
    //    before each close so an already-closed sibling does NOT get
    //    a spurious second ToggleEvent dispatched.  Without this
    //    re-check, closing one sibling via a listener side effect
    //    causes that sibling to receive `toggle(open→closed)` twice
    //    (once via the listener-driven setter, once via this loop).
    for sibling in siblings_to_close {
        let still_open = ctx
            .host()
            .dom()
            .world()
            .get::<&Attributes>(sibling)
            .ok()
            .and_then(|attrs| attrs.get("open").map(|_| true))
            .unwrap_or(false);
        if !still_open {
            continue;
        }
        invoke_dom_api(
            ctx,
            "removeAttribute",
            sibling,
            &[JsValue::String(attr_sid)],
        )?;
        let _cancelled = dispatch_toggle_event(ctx, sibling, "open", "closed")?;
    }
    // 5. Apply own attribute mutation.
    if new_open {
        let empty_sid = ctx.vm.well_known.empty;
        invoke_dom_api(
            ctx,
            "setAttribute",
            entity,
            &[JsValue::String(attr_sid), JsValue::String(empty_sid)],
        )?;
    } else {
        invoke_dom_api(ctx, "removeAttribute", entity, &[JsValue::String(attr_sid)])?;
    }
    // 6. Fire ToggleEvent on self with the appropriate state pair.
    let old_state = if prior_open { "open" } else { "closed" };
    let new_state = if new_open { "open" } else { "closed" };
    let _cancelled = dispatch_toggle_event(ctx, entity, old_state, new_state)?;
    Ok(JsValue::Undefined)
}

fn details_get_name(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_details_receiver(ctx, this, "name")? else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    let attr_sid = ctx.vm.strings.intern("name");
    invoke_dom_api(ctx, "getAttribute", entity, &[JsValue::String(attr_sid)]).map(|v| match v {
        JsValue::Null => JsValue::String(ctx.vm.well_known.empty),
        other => other,
    })
}

fn details_set_name(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_details_receiver(ctx, this, "name")? else {
        return Ok(JsValue::Undefined);
    };
    let value_sid = coerce_first_arg_to_string_id(ctx, args)?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    let attr_sid = ctx.vm.strings.intern("name");
    invoke_dom_api(
        ctx,
        "setAttribute",
        entity,
        &[JsValue::String(attr_sid), JsValue::String(value_sid)],
    )
}
