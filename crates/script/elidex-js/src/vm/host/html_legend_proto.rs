//! `HTMLLegendElement.prototype` intrinsic — per-tag prototype layer
//! for `<legend>` wrappers (HTML §4.10.16).
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", this file contains only the
//! engine-bound responsibilities: prototype install, brand check,
//! and JsValue↔Entity marshalling.  No algorithm lives here —
//! `<legend>` exposes `align` (legacy DOMString reflect) plus the
//! read-only `form` accessor that resolves through the parent
//! `<fieldset>`'s form ancestor.
//!
//! ## Chain
//!
//! ```text
//! legend wrapper
//!   → HTMLLegendElement.prototype       (this module)
//!     → HTMLElement.prototype → … → Object.prototype
//! ```

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{JsValue, NativeContext, Object, ObjectKind, PropertyStorage, VmError};
use super::super::VmInner;

use elidex_ecs::{Entity, NodeKind};

impl VmInner {
    /// Allocate `HTMLLegendElement.prototype` chained to
    /// `HTMLElement.prototype`.
    pub(in crate::vm) fn register_html_legend_prototype(&mut self) {
        let parent = self
            .html_element_prototype
            .expect("register_html_legend_prototype called before register_html_element_prototype");
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(parent),
            extensible: true,
        });
        self.html_legend_prototype = Some(proto_id);

        // `form` read-only accessor — HTML §4.10.16: returns the
        // form owner of the parent `<fieldset>` if any, else null.
        self.install_accessor_pair(
            proto_id,
            self.well_known.form_attr,
            native_legend_get_form,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
    }
}

fn require_legend_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "HTMLLegendElement", method, |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(None);
    };
    if !ctx.host().tag_matches_ascii_case(entity, "legend") {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLLegendElement': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

fn native_legend_get_form(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_legend_receiver(ctx, this, "form")? else {
        return Ok(JsValue::Null);
    };
    // HTML §4.10.16: legend.form returns the form owner of the
    // nearest ancestor fieldset (if any).
    let dom = ctx.host().dom();
    let parent = dom.get_parent(entity);
    let Some(parent_entity) = parent else {
        return Ok(JsValue::Null);
    };
    // Only walk through if parent is a fieldset.
    let parent_is_fieldset = ctx.host().tag_matches_ascii_case(parent_entity, "fieldset");
    if !parent_is_fieldset {
        return Ok(JsValue::Null);
    }
    let form = elidex_form::find_form_ancestor(ctx.host().dom(), parent_entity);
    Ok(super::dom_bridge::wrap_entity_or_null(ctx.vm, form))
}
