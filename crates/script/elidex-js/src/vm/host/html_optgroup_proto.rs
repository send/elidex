//! `HTMLOptGroupElement.prototype` intrinsic — per-tag prototype
//! layer for `<optgroup>` wrappers (HTML §4.10.9).
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", this file contains only the
//! engine-bound responsibilities: prototype install, brand check,
//! and reflected-attribute getter/setter shaping.  No algorithm
//! lives here — `<optgroup>` has no behaviour beyond reflecting
//! `disabled` (boolean) and `label` (DOMString) per HTML §4.10.9.
//!
//! ## Chain
//!
//! ```text
//! optgroup wrapper
//!   → HTMLOptGroupElement.prototype     (this module)
//!     → HTMLElement.prototype → … → Object.prototype
//! ```

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{JsValue, NativeContext, Object, ObjectKind, PropertyStorage, VmError};
use super::super::VmInner;

use elidex_ecs::{Entity, NodeKind};

impl VmInner {
    /// Allocate `HTMLOptGroupElement.prototype` chained to
    /// `HTMLElement.prototype`.
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
        // `label` DOMString reflect.
        self.install_accessor_pair(
            proto_id,
            self.well_known.label_attr,
            native_optgroup_get_label,
            Some(native_optgroup_set_label),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
    }
}

fn require_optgroup_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "HTMLOptGroupElement", method, |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(None);
    };
    if !ctx.host().tag_matches_ascii_case(entity, "optgroup") {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLOptGroupElement': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

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
