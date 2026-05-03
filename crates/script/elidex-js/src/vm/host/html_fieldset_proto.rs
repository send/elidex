//! `HTMLFieldSetElement.prototype` intrinsic — per-tag prototype
//! layer for `<fieldset>` wrappers (HTML §4.10.15).
//!
//! Chain (slot #11-tags-T1 Phase 3):
//!
//! ```text
//! fieldset wrapper
//!   → HTMLFieldSetElement.prototype  (this intrinsic)
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
//!   attribute.
//! - **`name`** — DOMString reflect of the `name` content attribute.
//! - **`type`** getter — always returns `"fieldset"` per HTML
//!   §4.10.15.5.  No setter (read-only IDL accessor).
//! - **`form`** getter — derived through HTML §4.10.18.3 form
//!   association (`form="<id>"` IDREF takes precedence, otherwise
//!   nearest `<form>` ancestor).
//! - **`elements`** getter — returns a live HTMLFormControlsCollection
//!   over the fieldset's descendant listed elements (HTML
//!   §4.10.15.5 step 2 — scoped to fieldset's tree).  Each access
//!   re-walks the descendants (cache opt-out per
//!   [`super::dom_collection::LiveCollectionKind::is_cacheable`]).
//!
//! ConstraintValidation methods (`checkValidity` / `reportValidity`
//! / `setCustomValidity`) install on this prototype during Phase 9
//! through the shared mixin — see plan §B "ValidityState +
//! ConstraintValidation".

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{JsValue, NativeContext, Object, ObjectKind, PropertyStorage, VmError};
use super::super::VmInner;

use elidex_ecs::{Entity, NodeKind};

const INTERFACE: &str = "HTMLFieldSetElement";

impl VmInner {
    /// Allocate `HTMLFieldSetElement.prototype`.  Must run after
    /// `register_html_element_prototype`.
    pub(in crate::vm) fn register_html_fieldset_prototype(&mut self) {
        let parent = self.html_element_prototype.expect(
            "register_html_fieldset_prototype called before register_html_element_prototype",
        );
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(parent),
            extensible: true,
        });
        self.html_fieldset_prototype = Some(proto_id);

        // disabled (boolean reflect)
        self.install_accessor_pair(
            proto_id,
            self.well_known.disabled,
            native_fieldset_get_disabled,
            Some(native_fieldset_set_disabled),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        // name (DOMString reflect)
        self.install_accessor_pair(
            proto_id,
            self.well_known.name,
            native_fieldset_get_name,
            Some(native_fieldset_set_name),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        // type (constant "fieldset", no setter)
        self.install_accessor_pair(
            proto_id,
            self.well_known.type_attr,
            native_fieldset_get_type,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        // form (derived getter, no setter)
        self.install_accessor_pair(
            proto_id,
            self.well_known.form_attr,
            native_fieldset_get_form,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        // elements (HTMLFormControlsCollection, no setter)
        self.install_accessor_pair(
            proto_id,
            self.well_known.elements_attr,
            native_fieldset_get_elements,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
    }
}

fn require_fieldset_receiver(
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
    if !ctx.host().tag_matches_ascii_case(entity, "fieldset") {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on '{INTERFACE}': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

// --- disabled / name reflected attrs ------------------------------

fn native_fieldset_get_disabled(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_fieldset_receiver(ctx, this, "disabled")? else {
        return Ok(JsValue::Boolean(false));
    };
    Ok(JsValue::Boolean(
        ctx.host().dom().has_attribute(entity, "disabled"),
    ))
}

fn native_fieldset_set_disabled(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_fieldset_receiver(ctx, this, "disabled")? else {
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

fn native_fieldset_get_name(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let empty = ctx.vm.well_known.empty;
    let Some(entity) = require_fieldset_receiver(ctx, this, "name")? else {
        return Ok(JsValue::String(empty));
    };
    let sid = match ctx.dom_and_strings_if_bound() {
        Some((dom, strings)) => {
            dom.with_attribute(entity, "name", |v| v.map_or(empty, |s| strings.intern(s)))
        }
        None => empty,
    };
    Ok(JsValue::String(sid))
}

fn native_fieldset_set_name(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_fieldset_receiver(ctx, this, "name")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = super::super::coerce::to_string(ctx.vm, val)?;
    let s = ctx.vm.strings.get_utf8(sid);
    ctx.host().dom().set_attribute(entity, "name", s);
    Ok(JsValue::Undefined)
}

// --- type ---------------------------------------------------------

/// `type` always returns `"fieldset"` per HTML §4.10.15.5 — the
/// fieldset is treated as a single fixed-type form-associated
/// element regardless of any contained controls.
fn native_fieldset_get_type(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(_entity) = require_fieldset_receiver(ctx, this, "type")? else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    Ok(JsValue::String(ctx.vm.well_known.fieldset_str))
}

// --- form ---------------------------------------------------------

fn native_fieldset_get_form(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_fieldset_receiver(ctx, this, "form")? else {
        return Ok(JsValue::Null);
    };
    match super::form_assoc::resolve_form_association(ctx, entity) {
        Some(f) => Ok(JsValue::Object(ctx.vm.create_element_wrapper(f))),
        None => Ok(JsValue::Null),
    }
}

// --- elements -----------------------------------------------------

/// `fieldset.elements` returns a live HTMLFormControlsCollection
/// over the fieldset's listed-element descendants.  Re-walks per
/// access (cache opt-out — see
/// [`super::dom_collection::LiveCollectionKind::is_cacheable`]).
fn native_fieldset_get_elements(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_fieldset_receiver(ctx, this, "elements")? else {
        return Ok(JsValue::Null);
    };
    let kind = super::dom_collection::LiveCollectionKind::FormControls { scope: entity };
    let id = ctx.vm.alloc_collection(kind);
    Ok(JsValue::Object(id))
}
