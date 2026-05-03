//! `HTMLOptGroupElement.prototype` intrinsic — per-tag prototype layer
//! for `<optgroup>` wrappers (HTML §4.10.9).
//!
//! Chain (slot #11-tags-T1):
//!
//! ```text
//! optgroup wrapper
//!   → HTMLOptGroupElement.prototype  (this intrinsic)
//!     → HTMLElement.prototype
//!       → Element.prototype
//!         → Node.prototype
//!           → EventTarget.prototype
//!             → Object.prototype
//! ```
//!
//! Members installed here:
//!
//! - **`disabled`** — boolean reflect of the `disabled` content
//!   attribute (HTML §4.10.9).  Attribute presence is the IDL
//!   boolean; setter true → `setAttribute("disabled", "")`,
//!   setter false → `removeAttribute("disabled")`.
//! - **`label`** — DOMString reflect of the `label` content
//!   attribute.  Read-as-`""`-when-absent; write coerces to string.
//!
//! Slot #11-tags-T1 small triplet warm-up alongside HTMLLabelElement +
//! HTMLLegendElement.

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{JsValue, NativeContext, Object, ObjectKind, PropertyStorage, VmError};
use super::super::VmInner;

use elidex_ecs::{Entity, NodeKind};

const INTERFACE: &str = "HTMLOptGroupElement";

impl VmInner {
    /// Allocate `HTMLOptGroupElement.prototype` with
    /// `HTMLElement.prototype` as its parent so
    /// `og instanceof HTMLElement === true`.
    /// Must run after `register_html_element_prototype`.
    pub(in crate::vm) fn register_html_optgroup_prototype(&mut self) {
        let parent = self.html_element_prototype.expect(
            "register_html_optgroup_prototype called before register_html_element_prototype",
        );
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(parent),
            extensible: true,
        });
        self.html_optgroup_prototype = Some(proto_id);

        // `disabled` boolean reflect.
        self.install_accessor_pair(
            proto_id,
            self.well_known.disabled,
            native_optgroup_get_disabled,
            Some(native_optgroup_set_disabled),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        // `label` string reflect.
        self.install_accessor_pair(
            proto_id,
            self.well_known.label_attr,
            native_optgroup_get_label,
            Some(native_optgroup_set_label),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
    }
}

/// Brand check for `<optgroup>` receivers.
fn require_optgroup_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    let Some(entity) = super::event_target::require_receiver(ctx, this, INTERFACE, method, |k| {
        k == NodeKind::Element
    })?
    else {
        return Ok(None);
    };
    if !ctx.host().tag_matches_ascii_case(entity, "optgroup") {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on '{INTERFACE}': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

/// `disabled` getter — boolean reflect.
fn native_optgroup_get_disabled(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_optgroup_receiver(ctx, this, "disabled")? else {
        return Ok(JsValue::Boolean(false));
    };
    Ok(JsValue::Boolean(
        ctx.host().dom().has_attribute(entity, "disabled"),
    ))
}

/// `disabled` setter — `ToBoolean(v)` toggles attribute presence.
fn native_optgroup_set_disabled(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_optgroup_receiver(ctx, this, "disabled")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let flag = super::super::coerce::to_boolean(ctx.vm, val);
    if flag {
        ctx.host()
            .dom()
            .set_attribute(entity, "disabled", String::new());
    } else {
        super::element_attrs::attr_remove(ctx, entity, "disabled");
    }
    Ok(JsValue::Undefined)
}

/// `label` getter — DOMString reflect of `label` content attribute.
fn native_optgroup_get_label(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let empty = ctx.vm.well_known.empty;
    let Some(entity) = require_optgroup_receiver(ctx, this, "label")? else {
        return Ok(JsValue::String(empty));
    };
    let sid = match ctx.dom_and_strings_if_bound() {
        Some((dom, strings)) => {
            dom.with_attribute(entity, "label", |v| v.map_or(empty, |s| strings.intern(s)))
        }
        None => empty,
    };
    Ok(JsValue::String(sid))
}

/// `label` setter — coerce-to-string write.
fn native_optgroup_set_label(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_optgroup_receiver(ctx, this, "label")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = super::super::coerce::to_string(ctx.vm, val)?;
    let s = ctx.vm.strings.get_utf8(sid);
    ctx.host().dom().set_attribute(entity, "label", s);
    Ok(JsValue::Undefined)
}
