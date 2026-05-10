//! `HTMLTableColElement.prototype` intrinsic — per-tag prototype
//! layer for `<col>` and `<colgroup>` wrappers (HTML §4.9.4, slot
//! `#11-tags-T2c-table`).
//!
//! Single IDL attribute:
//! - `span` — long IDL with default 1, clamped to `1..=1000` per
//!   HTML §4.9.4 (zero or out-of-range values become 1).  The
//!   getter parses via the engine-indep
//!   [`elidex_dom_api::element::numeric_reflect::parse_long_or_default`]
//!   helper, then clamps; the setter coerces the JS value through
//!   [`super::idl_coerce::serialise_long_idl_arg`] (T2b helper).
//!
//! Deprecated `align` / `vAlign` / `width` / `ch` / `chOff` are
//! intentionally not surfaced (defer slot
//! `#11-tags-deprecated-attr-sweep`).
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", marshalling-only.

#![cfg(feature = "engine")]

use elidex_dom_api::element::numeric_reflect::parse_long_or_default;
use elidex_ecs::{Entity, NodeKind};

use super::super::shape;
use super::super::value::{JsValue, NativeContext, VmError};
use super::super::VmInner;
use super::dom_bridge::invoke_dom_api;
use super::idl_coerce::serialise_long_idl_arg;

const COL_SPAN_DEFAULT: i32 = 1;
const COL_SPAN_MAX: i32 = 1000;

impl VmInner {
    pub(in crate::vm) fn register_html_table_col_prototype(&mut self) {
        let parent = self.html_element_prototype.expect(
            "register_html_table_col_prototype called before register_html_element_prototype",
        );
        let proto_id = self.alloc_html_subclass_prototype(parent);
        self.html_table_col_prototype = Some(proto_id);

        let span_sid = self.strings.intern("span");
        self.install_accessor_pair(
            proto_id,
            span_sid,
            col_get_span,
            Some(col_set_span),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
    }
}

fn require_col_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "HTMLTableColElement", method, |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(None);
    };
    let host = ctx.host();
    if !host.tag_matches_ascii_case(entity, "col")
        && !host.tag_matches_ascii_case(entity, "colgroup")
    {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLTableColElement': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

/// Clamp parsed `<col>.span` / `<colgroup>.span` value to the spec
/// range `1..=1000` (HTML §4.9.4).  Zero and negative inputs collapse
/// to default 1; values above the cap saturate at 1000.  Distinct
/// from `<td>.colSpan` (`≥1`, no upper cap) and `<td>.rowSpan`
/// (`0..=65534`) clamps in `html_table_cell_proto.rs` — kept in
/// separate files so each lives next to its accessor.
fn clamp_col_element_span(parsed: i32) -> i32 {
    if parsed < 1 {
        COL_SPAN_DEFAULT
    } else if parsed > COL_SPAN_MAX {
        COL_SPAN_MAX
    } else {
        parsed
    }
}

fn col_get_span(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_col_receiver(ctx, this, "span")? else {
        return Ok(JsValue::Number(f64::from(COL_SPAN_DEFAULT)));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Number(f64::from(COL_SPAN_DEFAULT)));
    }
    let attr_sid = ctx.vm.strings.intern("span");
    let raw_value = invoke_dom_api(ctx, "getAttribute", entity, &[JsValue::String(attr_sid)])?;
    let raw_str = match raw_value {
        JsValue::String(sid) => Some(ctx.vm.strings.get_utf8(sid)),
        _ => None,
    };
    let parsed = parse_long_or_default(raw_str.as_deref(), COL_SPAN_DEFAULT);
    Ok(JsValue::Number(f64::from(clamp_col_element_span(parsed))))
}

fn col_set_span(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_col_receiver(ctx, this, "span")? else {
        return Ok(JsValue::Undefined);
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    // Serialise via the shared T2b coercion (ToNumber → i32 saturate)
    // so `valueOf` / fractional truncation match the rest of the long
    // IDL setters.  The serialised string then becomes the content
    // attribute value; the getter clamping above re-applies the spec
    // range on read.
    let serialised = serialise_long_idl_arg(ctx, args)?;
    let attr_sid = ctx.vm.strings.intern("span");
    let value_sid = ctx.vm.strings.intern(&serialised);
    invoke_dom_api(
        ctx,
        "setAttribute",
        entity,
        &[JsValue::String(attr_sid), JsValue::String(value_sid)],
    )
}
