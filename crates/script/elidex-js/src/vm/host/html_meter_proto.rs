//! `HTMLMeterElement.prototype` intrinsic — per-tag prototype layer
//! for `<meter>` wrappers (HTML §4.10.15, slot
//! `#11-tags-T2d-interactive`).
//!
//! IDL surface (HTML §4.10.15):
//! - `value` / `min` / `max` / `low` / `high` / `optimum` — all
//!   `double` IDL.  Defaults: min=0, max=1, value=0.  low / high /
//!   optimum default to the appropriate clamp (low ≥ min, high ≤ max,
//!   optimum in [min,max]).
//! - `labels` — empty `NodeList` stub.
//!
//! Setters route the JS Number through
//! [`super::idl_coerce::coerce_double_idl_arg`] so `valueOf` /
//! `toString` on objects fires per WebIDL §3.10.5; serialisation is
//! `f64::to_string()` per the spec's "best representation" rule.
//!
//! Note: `<meter>` does NOT expose a `.position` IDL accessor (only
//! `<progress>` has one — HTML §4.10.14 vs §4.10.15).
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", marshalling-only.  Floating-point
//! parse algorithm lives engine-indep in
//! `elidex_dom_api::element::numeric_reflect::parse_double_or_default`.

#![cfg(feature = "engine")]

use elidex_dom_api::element::numeric_reflect::parse_double_or_default;
use elidex_ecs::{Attributes, Entity, NodeKind};

use super::super::shape;
use super::super::value::{JsValue, NativeContext, VmError};
use super::super::VmInner;
use super::dom_bridge::invoke_dom_api;
use super::idl_coerce::coerce_double_idl_arg;

impl VmInner {
    pub(in crate::vm) fn register_html_meter_prototype(&mut self) {
        let parent = self
            .html_element_prototype
            .expect("register_html_meter_prototype called before register_html_element_prototype");
        let proto_id = self.alloc_html_subclass_prototype(parent);
        self.html_meter_prototype = Some(proto_id);

        let attrs = shape::PropertyAttrs::WEBIDL_RO_ACCESSOR;
        for (idl_name, getter, setter) in [
            (
                "value",
                meter_get_value as super::super::NativeFn,
                meter_set_value as super::super::NativeFn,
            ),
            ("min", meter_get_min, meter_set_min),
            ("max", meter_get_max, meter_set_max),
            ("low", meter_get_low, meter_set_low),
            ("high", meter_get_high, meter_set_high),
            ("optimum", meter_get_optimum, meter_set_optimum),
        ] {
            let sid = self.strings.intern(idl_name);
            self.install_accessor_pair(proto_id, sid, getter, Some(setter), attrs);
        }
        let labels_sid = self.strings.intern("labels");
        self.install_accessor_pair(proto_id, labels_sid, meter_get_labels, None, attrs);
    }
}

fn require_meter_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "HTMLMeterElement", method, |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(None);
    };
    if !ctx.host().tag_matches_ascii_case(entity, "meter") {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLMeterElement': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

fn read_attr_raw(ctx: &mut NativeContext<'_>, entity: Entity, name: &str) -> Option<String> {
    ctx.host()
        .dom()
        .world()
        .get::<&Attributes>(entity)
        .ok()
        .and_then(|a| a.get(name).map(str::to_owned))
}

/// Compute the actual min / max pair per HTML §4.10.15:
/// - `min` defaults to 0.
/// - `max` defaults to 1; if `max < min`, set `max = min` (the
///   spec's "actual maximum" guarantees `max >= min`).
fn parsed_min_max(ctx: &mut NativeContext<'_>, entity: Entity) -> (f64, f64) {
    let min = parse_double_or_default(read_attr_raw(ctx, entity, "min").as_deref(), 0.0);
    let mut max = parse_double_or_default(read_attr_raw(ctx, entity, "max").as_deref(), 1.0);
    if max < min {
        max = min;
    }
    (min, max)
}

/// Read `attr_name` as a clamped value within `[min, max]`, defaulting
/// to `default_within_range` when the attribute is missing / invalid.
/// Used for `value` (default min) / `low` (default min) / `high`
/// (default max) / `optimum` ((min+max)/2).
fn read_clamped(
    ctx: &mut NativeContext<'_>,
    entity: Entity,
    attr_name: &str,
    default_within_range: f64,
    min: f64,
    max: f64,
) -> f64 {
    parse_double_or_default(
        read_attr_raw(ctx, entity, attr_name).as_deref(),
        default_within_range,
    )
    .clamp(min, max)
}

fn meter_get_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_meter_receiver(ctx, this, "value")? else {
        return Ok(JsValue::Number(0.0));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Number(0.0));
    }
    let (min, max) = parsed_min_max(ctx, entity);
    // HTML §4.10.15: default actual value is `0` then clamped to
    // `[min, max]` — distinct from `min` when `min > 0` or `min < 0`
    // bracket `0`.
    Ok(JsValue::Number(read_clamped(
        ctx, entity, "value", 0.0, min, max,
    )))
}

fn meter_get_min(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_meter_receiver(ctx, this, "min")? else {
        return Ok(JsValue::Number(0.0));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Number(0.0));
    }
    let (min, _) = parsed_min_max(ctx, entity);
    Ok(JsValue::Number(min))
}

fn meter_get_max(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_meter_receiver(ctx, this, "max")? else {
        return Ok(JsValue::Number(1.0));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Number(1.0));
    }
    let (_, max) = parsed_min_max(ctx, entity);
    Ok(JsValue::Number(max))
}

fn meter_get_low(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_meter_receiver(ctx, this, "low")? else {
        return Ok(JsValue::Number(0.0));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Number(0.0));
    }
    let (min, max) = parsed_min_max(ctx, entity);
    // HTML §4.10.15: `low` defaults to `min` when missing; clamp to
    // `[min, max]`.
    Ok(JsValue::Number(read_clamped(
        ctx, entity, "low", min, min, max,
    )))
}

fn meter_get_high(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_meter_receiver(ctx, this, "high")? else {
        return Ok(JsValue::Number(1.0));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Number(1.0));
    }
    let (min, max) = parsed_min_max(ctx, entity);
    // HTML §4.10.15: `high` defaults to `max` when missing; clamp to
    // `[low, max]` (NOT `[min, max]`) so an explicit `high < low`
    // promotes to the actual-low value per spec step 3.  The lower
    // bound is therefore `low`, computed via `read_clamped` on the
    // `low` attribute (which itself clamps to `[min, max]`).
    let low = read_clamped(ctx, entity, "low", min, min, max);
    Ok(JsValue::Number(read_clamped(
        ctx, entity, "high", max, low, max,
    )))
}

fn meter_get_optimum(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_meter_receiver(ctx, this, "optimum")? else {
        return Ok(JsValue::Number(0.5));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Number(0.5));
    }
    let (min, max) = parsed_min_max(ctx, entity);
    let mid = f64::midpoint(min, max);
    Ok(JsValue::Number(read_clamped(
        ctx, entity, "optimum", mid, min, max,
    )))
}

fn write_double_attr(
    ctx: &mut NativeContext<'_>,
    entity: Entity,
    attr_name: &str,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let n = coerce_double_idl_arg(ctx, args)?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    let serialised = format!("{n}");
    let value_sid = ctx.vm.strings.intern(&serialised);
    let attr_sid = ctx.vm.strings.intern(attr_name);
    invoke_dom_api(
        ctx,
        "setAttribute",
        entity,
        &[JsValue::String(attr_sid), JsValue::String(value_sid)],
    )
}

fn meter_set_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_meter_receiver(ctx, this, "value")? else {
        return Ok(JsValue::Undefined);
    };
    write_double_attr(ctx, entity, "value", args)
}

fn meter_set_min(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_meter_receiver(ctx, this, "min")? else {
        return Ok(JsValue::Undefined);
    };
    write_double_attr(ctx, entity, "min", args)
}

fn meter_set_max(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_meter_receiver(ctx, this, "max")? else {
        return Ok(JsValue::Undefined);
    };
    write_double_attr(ctx, entity, "max", args)
}

fn meter_set_low(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_meter_receiver(ctx, this, "low")? else {
        return Ok(JsValue::Undefined);
    };
    write_double_attr(ctx, entity, "low", args)
}

fn meter_set_high(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_meter_receiver(ctx, this, "high")? else {
        return Ok(JsValue::Undefined);
    };
    write_double_attr(ctx, entity, "high", args)
}

fn meter_set_optimum(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_meter_receiver(ctx, this, "optimum")? else {
        return Ok(JsValue::Undefined);
    };
    write_double_attr(ctx, entity, "optimum", args)
}

fn meter_get_labels(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let _ = require_meter_receiver(ctx, this, "labels")?;
    Ok(JsValue::Object(
        super::dom_collection::empty_labels_collection(ctx.vm),
    ))
}
