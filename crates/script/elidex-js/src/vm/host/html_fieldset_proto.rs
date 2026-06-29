//! `HTMLFieldSetElement.prototype` intrinsic — per-tag prototype
//! layer for `<fieldset>` wrappers (HTML §4.10.15).
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", this module is engine-bound
//! responsibilities only.  Fieldset disabled propagation lives in
//! [`elidex_form::is_fieldset_disabled`] /
//! [`elidex_form::propagate_fieldset_disabled`]; this module just
//! reflects the `disabled` content attribute.
//!
//! ## Members installed
//!
//! - `disabled` — boolean reflect.
//! - `name` — DOMString reflect.
//! - `type` — read-only constant `"fieldset"` per HTML §4.10.15.
//! - `form` — read-only via `elidex_form::find_form_ancestor`.
//! - `elements` — read-only.  `[SameObject]`-cached
//!   `HTMLFormControlsCollection` over listed-element descendants,
//!   backed by `CollectionFilter::FormControls` (Phase 7 + the cache
//!   wiring added in this PR).
//! - ConstraintValidation mixin (`validity` / `validationMessage` /
//!   `willValidate` / `checkValidity()` / `reportValidity()` /
//!   `setCustomValidity()`) is installed by
//!   `VmInner::install_constraint_validation_mixin` from Phase 9.
//!   `<fieldset>` is a candidate for the mixin even though it is
//!   not itself "submittable" — `willValidate` returns `false` and
//!   the methods short-circuit per HTML §4.10.20.3.

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{JsValue, NativeContext, Object, ObjectKind, PropertyStorage, VmError};
use super::super::VmInner;

use elidex_ecs::{Entity, NodeKind};

impl VmInner {
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

        let attrs = shape::PropertyAttrs::WEBIDL_RO_ACCESSOR;

        self.install_accessor_pair(
            proto_id,
            self.well_known.disabled,
            native_fieldset_get_disabled,
            Some(native_fieldset_set_disabled),
            attrs,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.name,
            native_fieldset_get_name,
            Some(native_fieldset_set_name),
            attrs,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.type_attr,
            native_fieldset_get_type,
            None,
            attrs,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.form_attr,
            native_fieldset_get_form,
            None,
            attrs,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.elements_attr,
            native_fieldset_get_elements,
            None,
            attrs,
        );
    }
}

fn require_fieldset_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "HTMLFieldSetElement", method, |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(None);
    };
    if !ctx.host().tag_matches_ascii_case(entity, "fieldset") {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLFieldSetElement': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

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
        super::element_attrs::attr_set(ctx, entity, "disabled", "");
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
    super::element_attrs::attr_set(ctx, entity, "name", &s);
    Ok(JsValue::Undefined)
}

fn native_fieldset_get_type(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let _ = require_fieldset_receiver(ctx, this, "type")?;
    let sid = ctx.vm.strings.intern("fieldset");
    Ok(JsValue::String(sid))
}

fn native_fieldset_get_form(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_fieldset_receiver(ctx, this, "form")? else {
        return Ok(JsValue::Null);
    };
    let form = elidex_form::find_form_ancestor(ctx.host().dom(), entity);
    Ok(super::dom_bridge::wrap_entity_or_null(ctx.vm, form))
}

/// `fieldset.elements` — HTML §4.10.15.  Returns an
/// `HTMLFormControlsCollection` over the listed-element descendants
/// of the fieldset.  Backed by
/// [`elidex_dom_api::CollectionFilter::FormControls`].
fn native_fieldset_get_elements(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_fieldset_receiver(ctx, this, "elements")?;
    // `[SameObject]` per WebIDL.  Shares
    // `form_controls_collection_wrappers` with `form.elements` because
    // both surfaces produce one HTMLFormControlsCollection per owner
    // entity, keyed by that entity.
    let id = super::dom_collection::cached_form_collection(
        ctx.vm,
        entity,
        elidex_dom_api::CollectionFilter::FormControls,
        super::dom_collection::FormCollectionCache::FormControls,
    );
    Ok(JsValue::Object(id))
}
