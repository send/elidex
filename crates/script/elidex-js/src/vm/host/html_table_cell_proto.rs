//! `HTMLTableCellElement.prototype` intrinsic — per-tag prototype
//! layer for `<td>` / `<th>` wrappers (HTML §4.9.9-10, slot
//! `#11-tags-T2c-table`).
//!
//! IDL surface:
//! - `cellIndex` — long, position in parent `<tr>`'s cells list
//!   (`-1` if not in a `<tr>`).
//! - `colSpan` — long, default 1, clamped `≥1` (zero or negative →
//!   1).
//! - `rowSpan` — long, default 1, clamped `0..=65534` (`0` means
//!   "span all remaining rows in section"; HTML §4.9.10).
//! - `headers` — DOMString reflect.
//! - `abbr` — DOMString reflect.
//! - `scope` — DOMString enumerated reflect (HTML §4.9.10):
//!   `"row"` / `"col"` / `"rowgroup"` / `"colgroup"`, ASCII-CI
//!   canonicalisation; missing default `""`, invalid default `""`.
//!   Per spec the IDL is on the shared HTMLTableCellElement
//!   interface so both `<th>.scope` and `<td>.scope` work; only
//!   `<th>.scope` carries semantic weight.
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", marshalling-only.  All algorithm
//! lives in `elidex-dom-api` (`numeric_reflect` / `enumerated_reflect`
//! / `table_mutation::cell_index`).

#![cfg(feature = "engine")]

use elidex_dom_api::element::enumerated_reflect::{
    canonicalize_enumerated_attr, SCOPE_INVALID_DEFAULT, SCOPE_MISSING_DEFAULT, SCOPE_VALUES,
};
use elidex_dom_api::element::numeric_reflect::parse_long_or_default;
use elidex_dom_api::element::table_mutation;
use elidex_ecs::{Entity, NodeKind};

use super::super::shape;
use super::super::value::{JsValue, NativeContext, VmError};
use super::super::VmInner;
use super::dom_bridge::{coerce_first_arg_to_string_id, invoke_dom_api};
use super::idl_coerce::serialise_long_idl_arg;

const COL_ROW_SPAN_DEFAULT: i32 = 1;
const COLSPAN_MIN: i32 = 1;
const ROWSPAN_MAX: i32 = 65534;

impl VmInner {
    pub(in crate::vm) fn register_html_table_cell_prototype(&mut self) {
        let parent = self.html_element_prototype.expect(
            "register_html_table_cell_prototype called before register_html_element_prototype",
        );
        let proto_id = self.alloc_html_subclass_prototype(parent);
        self.html_table_cell_prototype = Some(proto_id);

        let attrs = shape::PropertyAttrs::WEBIDL_RO_ACCESSOR;
        for (name, getter, setter) in [
            (
                "cellIndex",
                cell_get_cell_index as super::super::NativeFn,
                None,
            ),
            (
                "colSpan",
                cell_get_col_span,
                Some(cell_set_col_span as super::super::NativeFn),
            ),
            ("rowSpan", cell_get_row_span, Some(cell_set_row_span)),
            ("headers", cell_get_headers, Some(cell_set_headers)),
            ("abbr", cell_get_abbr, Some(cell_set_abbr)),
            ("scope", cell_get_scope, Some(cell_set_scope)),
        ] {
            let sid = self.strings.intern(name);
            self.install_accessor_pair(proto_id, sid, getter, setter, attrs);
        }
    }
}

fn require_cell_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "HTMLTableCellElement", method, |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(None);
    };
    let host = ctx.host();
    if !host.tag_matches_ascii_case(entity, "td") && !host.tag_matches_ascii_case(entity, "th") {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLTableCellElement': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

// -- cellIndex (read-only) -------------------------------------------------

fn cell_get_cell_index(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_cell_receiver(ctx, this, "cellIndex")? else {
        return Ok(JsValue::Number(-1.0));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Number(-1.0));
    }
    let dom = ctx.host().dom_shared();
    Ok(JsValue::Number(f64::from(table_mutation::cell_index(
        entity, dom,
    ))))
}

// -- colSpan / rowSpan (long IDL with clamping) ----------------------------

fn clamp_col_span(parsed: i32) -> i32 {
    parsed.max(COLSPAN_MIN)
}

fn clamp_row_span(parsed: i32) -> i32 {
    parsed.clamp(0, ROWSPAN_MAX)
}

fn read_long_attr_clamped<F: Fn(i32) -> i32>(
    ctx: &mut NativeContext<'_>,
    entity: Entity,
    attr_name: &str,
    default: i32,
    clamp: F,
) -> Result<JsValue, VmError> {
    let attr_sid = ctx.vm.strings.intern(attr_name);
    let raw_value = invoke_dom_api(ctx, "getAttribute", entity, &[JsValue::String(attr_sid)])?;
    let raw_str = match raw_value {
        JsValue::String(sid) => Some(ctx.vm.strings.get_utf8(sid)),
        _ => None,
    };
    let parsed = parse_long_or_default(raw_str.as_deref(), default);
    Ok(JsValue::Number(f64::from(clamp(parsed))))
}

fn cell_get_col_span(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_cell_receiver(ctx, this, "colSpan")? else {
        return Ok(JsValue::Number(f64::from(COL_ROW_SPAN_DEFAULT)));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Number(f64::from(COL_ROW_SPAN_DEFAULT)));
    }
    read_long_attr_clamped(ctx, entity, "colspan", COL_ROW_SPAN_DEFAULT, clamp_col_span)
}

fn cell_set_col_span(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_cell_receiver(ctx, this, "colSpan")? else {
        return Ok(JsValue::Undefined);
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    let serialised = serialise_long_idl_arg(ctx, args)?;
    let attr_sid = ctx.vm.strings.intern("colspan");
    let value_sid = ctx.vm.strings.intern(&serialised);
    invoke_dom_api(
        ctx,
        "setAttribute",
        entity,
        &[JsValue::String(attr_sid), JsValue::String(value_sid)],
    )
}

fn cell_get_row_span(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_cell_receiver(ctx, this, "rowSpan")? else {
        return Ok(JsValue::Number(f64::from(COL_ROW_SPAN_DEFAULT)));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Number(f64::from(COL_ROW_SPAN_DEFAULT)));
    }
    read_long_attr_clamped(ctx, entity, "rowspan", COL_ROW_SPAN_DEFAULT, clamp_row_span)
}

fn cell_set_row_span(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_cell_receiver(ctx, this, "rowSpan")? else {
        return Ok(JsValue::Undefined);
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    let serialised = serialise_long_idl_arg(ctx, args)?;
    let attr_sid = ctx.vm.strings.intern("rowspan");
    let value_sid = ctx.vm.strings.intern(&serialised);
    invoke_dom_api(
        ctx,
        "setAttribute",
        entity,
        &[JsValue::String(attr_sid), JsValue::String(value_sid)],
    )
}

// -- headers / abbr (string reflect) ---------------------------------------

fn cell_get_headers(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    string_reflect_get(ctx, this, "headers", "headers")
}

fn cell_set_headers(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    string_reflect_set(ctx, this, args, "headers", "headers")
}

fn cell_get_abbr(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    string_reflect_get(ctx, this, "abbr", "abbr")
}

fn cell_set_abbr(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    string_reflect_set(ctx, this, args, "abbr", "abbr")
}

fn string_reflect_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
    attr: &str,
) -> Result<JsValue, VmError> {
    let Some(entity) = require_cell_receiver(ctx, this, method)? else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    let attr_sid = ctx.vm.strings.intern(attr);
    invoke_dom_api(ctx, "getAttribute", entity, &[JsValue::String(attr_sid)]).map(|v| match v {
        JsValue::Null => JsValue::String(ctx.vm.well_known.empty),
        other => other,
    })
}

fn string_reflect_set(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
    method: &str,
    attr: &str,
) -> Result<JsValue, VmError> {
    let Some(entity) = require_cell_receiver(ctx, this, method)? else {
        return Ok(JsValue::Undefined);
    };
    let value_sid = coerce_first_arg_to_string_id(ctx, args)?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    let attr_sid = ctx.vm.strings.intern(attr);
    invoke_dom_api(
        ctx,
        "setAttribute",
        entity,
        &[JsValue::String(attr_sid), JsValue::String(value_sid)],
    )
}

// -- scope (enumerated reflect, ASCII-CI) ----------------------------------

fn cell_get_scope(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_cell_receiver(ctx, this, "scope")? else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    let attr_sid = ctx.vm.strings.intern("scope");
    let raw_value = invoke_dom_api(ctx, "getAttribute", entity, &[JsValue::String(attr_sid)])?;
    let raw = match raw_value {
        JsValue::String(sid) => Some(ctx.vm.strings.get_utf8(sid)),
        _ => None,
    };
    let canonical = canonicalize_enumerated_attr(
        raw.as_deref(),
        SCOPE_VALUES,
        SCOPE_MISSING_DEFAULT,
        SCOPE_INVALID_DEFAULT,
    );
    let out_sid = ctx.vm.strings.intern(canonical);
    Ok(JsValue::String(out_sid))
}

fn cell_set_scope(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    string_reflect_set(ctx, this, args, "scope", "scope")
}
