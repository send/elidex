//! `HTMLLabelElement.prototype` intrinsic — per-tag prototype layer
//! for `<label>` wrappers (HTML §4.10.4).
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", this file contains only the
//! engine-bound responsibilities: prototype install, brand check,
//! reflected-attribute getter/setter shaping, and JsValue↔Entity
//! marshalling.  Label association (the algorithm that finds the
//! labelled control) lives in [`elidex_form::find_label_target`] /
//! [`elidex_form::resolve_label_for`]; this module is a thin binding
//! that hands its receiver entity to those functions.
//!
//! ## Chain
//!
//! ```text
//! label wrapper
//!   → HTMLLabelElement.prototype        (this module)
//!     → HTMLElement.prototype
//!       → Element.prototype → Node → EventTarget → Object.prototype
//! ```
//!
//! ## Members installed
//!
//! - `htmlFor` — DOMString reflect of `for` content attribute.
//! - `control` — read-only accessor; walks via
//!   [`elidex_form::find_label_target`] and returns the labelled
//!   element wrapper or `null`.
//! - `form` — read-only accessor; resolves the labelled control's
//!   form ancestor via [`elidex_form::find_form_ancestor`] (HTML
//!   §4.10.4 step 3).

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{JsValue, NativeContext, Object, ObjectKind, PropertyStorage, VmError};
use super::super::VmInner;

use elidex_ecs::{Entity, NodeKind};

impl VmInner {
    /// Allocate `HTMLLabelElement.prototype` chained to
    /// `HTMLElement.prototype`.  Must run after
    /// `register_html_element_prototype`.
    pub(in crate::vm) fn register_html_label_prototype(&mut self) {
        let parent = self
            .html_element_prototype
            .expect("register_html_label_prototype called before register_html_element_prototype");
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(parent),
            extensible: true,
        });
        self.html_label_prototype = Some(proto_id);

        // `htmlFor` accessor — DOMString reflect of `for`.
        self.install_accessor_pair(
            proto_id,
            self.well_known.html_for,
            native_label_get_html_for,
            Some(native_label_set_html_for),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        // `control` read-only accessor.
        self.install_accessor_pair(
            proto_id,
            self.well_known.control,
            native_label_get_control,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        // `form` read-only accessor.
        self.install_accessor_pair(
            proto_id,
            self.well_known.form_attr,
            native_label_get_form,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
    }
}

// ---------------------------------------------------------------------------
// Brand check
// ---------------------------------------------------------------------------

/// Recover the `<label>` entity from `this`, returning a TypeError
/// when `this` is not a `<label>` element wrapper.
fn require_label_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "HTMLLabelElement", method, |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(None);
    };
    if !ctx.host().tag_matches_ascii_case(entity, "label") {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLLabelElement': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

// ---------------------------------------------------------------------------
// Natives
// ---------------------------------------------------------------------------

fn native_label_get_html_for(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let empty = ctx.vm.well_known.empty;
    let Some(entity) = require_label_receiver(ctx, this, "htmlFor")? else {
        return Ok(JsValue::String(empty));
    };
    let sid = match ctx.dom_and_strings_if_bound() {
        Some((dom, strings)) => {
            dom.with_attribute(entity, "for", |v| v.map_or(empty, |s| strings.intern(s)))
        }
        None => empty,
    };
    Ok(JsValue::String(sid))
}

fn native_label_set_html_for(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_label_receiver(ctx, this, "htmlFor")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = super::super::coerce::to_string(ctx.vm, val)?;
    let s = ctx.vm.strings.get_utf8(sid);
    super::element_attrs::attr_set(ctx, entity, "for", &s);
    Ok(JsValue::Undefined)
}

fn native_label_get_control(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_label_receiver(ctx, this, "control")? else {
        return Ok(JsValue::Null);
    };
    // Engine-independent algorithm — `find_label_target` walks the
    // `for` attribute first, then the descendant subtree (HTML
    // §4.10.4).
    let target = elidex_form::find_label_target(ctx.host().dom(), entity);
    Ok(super::dom_bridge::wrap_entity_or_null(ctx.vm, target))
}

fn native_label_get_form(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_label_receiver(ctx, this, "form")? else {
        return Ok(JsValue::Null);
    };
    // HTML §4.10.4 step 3: `label.form` returns the form owner of
    // the labelled control.  Resolve `control` first; if no labelled
    // control or it has no form ancestor, return null.
    let dom = ctx.host().dom();
    let Some(control) = elidex_form::find_label_target(dom, entity) else {
        return Ok(JsValue::Null);
    };
    let form = elidex_form::find_form_ancestor(dom, control);
    Ok(super::dom_bridge::wrap_entity_or_null(ctx.vm, form))
}
