//! `HTMLLegendElement.prototype` intrinsic — per-tag prototype layer
//! for `<legend>` wrappers (HTML §4.10.16).
//!
//! Chain (slot #11-tags-T1):
//!
//! ```text
//! legend wrapper
//!   → HTMLLegendElement.prototype  (this intrinsic)
//!     → HTMLElement.prototype
//!       → Element.prototype
//!         → Node.prototype
//!           → EventTarget.prototype
//!             → Object.prototype
//! ```
//!
//! Members installed here:
//!
//! - **`form`** getter — returns the form owner of the legend's
//!   parent `<fieldset>` per HTML §4.10.16.  Returns `null` when the
//!   parent is not a fieldset, when there is no enclosing form, or
//!   when the legend has no parent.  No setter (read-only IDL
//!   accessor — the form association is purely derived).
//!
//! Slot #11-tags-T1 small triplet warm-up alongside HTMLLabelElement +
//! HTMLOptGroupElement.

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{JsValue, NativeContext, Object, ObjectKind, PropertyStorage, VmError};
use super::super::VmInner;

use elidex_ecs::{Entity, NodeKind};

const INTERFACE: &str = "HTMLLegendElement";

impl VmInner {
    /// Allocate `HTMLLegendElement.prototype` with
    /// `HTMLElement.prototype` as its parent so
    /// `lg instanceof HTMLElement === true`.
    /// Must run after `register_html_element_prototype`.
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

        // `form` getter — read-only.
        self.install_accessor_pair(
            proto_id,
            self.well_known.form_attr,
            native_legend_get_form,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
    }
}

/// Brand check for `<legend>` receivers.
fn require_legend_receiver(
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
    if !ctx.host().tag_matches_ascii_case(entity, "legend") {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on '{INTERFACE}': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

/// `form` getter — derived through the parent `<fieldset>`'s form
/// association (HTML §4.10.16).
fn native_legend_get_form(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_legend_receiver(ctx, this, "form")? else {
        return Ok(JsValue::Null);
    };
    let dom = ctx.host().dom();
    let parent = match dom.get_parent(entity) {
        Some(p) => p,
        None => return Ok(JsValue::Null),
    };
    if !dom.has_tag(parent, "fieldset") {
        return Ok(JsValue::Null);
    }
    // Resolve the fieldset's form association.  The form attribute
    // (`<fieldset form="<id>">`) takes precedence per HTML §4.10.18.3;
    // otherwise climb to the nearest `<form>` ancestor.
    let form = resolve_form_association(ctx, parent);
    match form {
        Some(f) => Ok(JsValue::Object(ctx.vm.create_element_wrapper(f))),
        None => Ok(JsValue::Null),
    }
}

/// HTML §4.10.18.3 form association walk: the `form="<id>"` content
/// attribute (when present) names the form by id within the same
/// tree; otherwise the form is the nearest `<form>` ancestor.
fn resolve_form_association(ctx: &mut NativeContext<'_>, fieldset: Entity) -> Option<Entity> {
    let dom = ctx.host().dom();
    let form_id = dom.with_attribute(fieldset, "form", |v| {
        v.filter(|s| !s.is_empty()).map(String::from)
    });
    if let Some(id) = form_id {
        // IDREF lookup scoped to the fieldset's tree.  Fall back to
        // ancestor walk if the id does not name an element OR names
        // a non-form element (per spec, only a `<form>` matches).
        let root = dom.owner_document(fieldset).unwrap_or_else(|| {
            let mut cur = fieldset;
            let mut depth: u32 = 0;
            while let Some(p) = dom.get_parent(cur) {
                if depth > 1024 {
                    break;
                }
                cur = p;
                depth += 1;
            }
            cur
        });
        if let Some(target) = dom.find_by_id(root, &id) {
            if dom.has_tag(target, "form") {
                return Some(target);
            }
        }
        // form= attribute set but resolves to nothing (or to a
        // non-form) → no association per spec; do NOT fall back to
        // ancestor walk.
        return None;
    }

    // Climb the ancestor chain for the nearest `<form>`.
    let mut cur = dom.get_parent(fieldset);
    let mut depth: u32 = 0;
    while let Some(p) = cur {
        if depth > 1024 {
            return None;
        }
        if dom.has_tag(p, "form") {
            return Some(p);
        }
        cur = dom.get_parent(p);
        depth += 1;
    }
    None
}
