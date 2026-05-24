//! `OffscreenCanvasRenderingContext2D.prototype` method bodies (HTML §4.12.5.3.1,
//! same surface as §4.12.5.1). Split out of [`super`] to keep each file under
//! the ~1000-line convention.
//!
//! Each `native_oc_*` here is a 5-line dispatch wrapper that brand-checks via
//! [`super::require_offscreen_canvas_2d_context`] and forwards to the shared
//! [`crate::vm::host::canvas::dispatch_2d_method`] generic (one-issue-one-way
//! with D-21's `<canvas>` 2D-context methods). The `oc_*_getter` / `_setter`
//! accessors follow the same pattern.
//!
//! Marshalling-only per the Layering mandate: brand-check, coercion, and
//! dispatch into the engine-independent raster backend.

#![cfg(feature = "engine")]

use elidex_web_canvas::Canvas2dContext;

use super::super::super::value::{
    JsValue, NativeContext, Object, ObjectKind, PropertyKey, PropertyStorage, PropertyValue,
    VmError,
};
use super::super::super::{coerce, shape};
use super::super::canvas::dispatch_2d_method;
use super::require_offscreen_canvas_2d_context;

// ---------------------------------------------------------------------------
// arg coercion helpers (mirror canvas/mod.rs to keep brands isolated)
// ---------------------------------------------------------------------------

fn arg_f32(ctx: &mut NativeContext<'_>, args: &[JsValue], i: usize) -> Result<f32, VmError> {
    match args.get(i).copied() {
        #[allow(clippy::cast_possible_truncation)]
        Some(v) => Ok(coerce::to_number(ctx.vm, v)? as f32),
        None => Ok(0.0),
    }
}

fn rect_args(
    ctx: &mut NativeContext<'_>,
    args: &[JsValue],
) -> Result<(f32, f32, f32, f32), VmError> {
    Ok((
        arg_f32(ctx, args, 0)?,
        arg_f32(ctx, args, 1)?,
        arg_f32(ctx, args, 2)?,
        arg_f32(ctx, args, 3)?,
    ))
}

// ---------------------------------------------------------------------------
// 2D context methods (parallel D-21's CONTEXT_METHODS, brand-checked into OC)
//
// Each `native_oc_*` is a 5-line dispatch wrapper that brand-checks via
// `require_offscreen_canvas_2d_context` and forwards to the shared
// `dispatch_2d_method` generic. WHATWG HTML §4.12.5.1 sub-sections by group:
//   §4.12.5.1.2 — canvas state stack: save / restore
//   §4.12.5.1.5 — building paths: beginPath / closePath / moveTo / lineTo /
//                                  rect / arc
//   §4.12.5.1.6 — transforms: translate / rotate / scale
//   §4.12.5.1.8 — fill/stroke styles: fillStyle / strokeStyle (accessors)
//   §4.12.5.1.9 — drawing rectangles: fillRect / strokeRect / clearRect
//   §4.12.5.1.10 — text: measureText
//   §4.12.5.1.11 — drawing paths: fill / stroke
//   §4.12.5.1.13 — compositing: globalAlpha (accessor)
// ---------------------------------------------------------------------------

/// `save()` (HTML §4.12.5.1.2 — canvas state stack push).
pub(super) fn native_oc_save(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _: &[JsValue],
) -> Result<JsValue, VmError> {
    dispatch_2d_method(
        ctx,
        this,
        "save",
        false,
        require_offscreen_canvas_2d_context,
        Canvas2dContext::save,
    )?;
    Ok(JsValue::Undefined)
}

/// `restore()` (HTML §4.12.5.1.2 — canvas state stack pop).
pub(super) fn native_oc_restore(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _: &[JsValue],
) -> Result<JsValue, VmError> {
    dispatch_2d_method(
        ctx,
        this,
        "restore",
        false,
        require_offscreen_canvas_2d_context,
        Canvas2dContext::restore,
    )?;
    Ok(JsValue::Undefined)
}

/// `beginPath()` (HTML §4.12.5.1.5 — clear the current default path).
pub(super) fn native_oc_begin_path(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _: &[JsValue],
) -> Result<JsValue, VmError> {
    dispatch_2d_method(
        ctx,
        this,
        "beginPath",
        false,
        require_offscreen_canvas_2d_context,
        Canvas2dContext::begin_path,
    )?;
    Ok(JsValue::Undefined)
}

/// `closePath()` (HTML §4.12.5.1.5 — close the current subpath).
pub(super) fn native_oc_close_path(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _: &[JsValue],
) -> Result<JsValue, VmError> {
    dispatch_2d_method(
        ctx,
        this,
        "closePath",
        false,
        require_offscreen_canvas_2d_context,
        Canvas2dContext::close_path,
    )?;
    Ok(JsValue::Undefined)
}

/// `moveTo(x, y)` (HTML §4.12.5.1.5 — start a new subpath).
pub(super) fn native_oc_move_to(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (x, y) = (arg_f32(ctx, args, 0)?, arg_f32(ctx, args, 1)?);
    dispatch_2d_method(
        ctx,
        this,
        "moveTo",
        false,
        require_offscreen_canvas_2d_context,
        |c| c.move_to(x, y),
    )?;
    Ok(JsValue::Undefined)
}

/// `lineTo(x, y)` (HTML §4.12.5.1.5 — add a straight line segment).
pub(super) fn native_oc_line_to(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (x, y) = (arg_f32(ctx, args, 0)?, arg_f32(ctx, args, 1)?);
    dispatch_2d_method(
        ctx,
        this,
        "lineTo",
        false,
        require_offscreen_canvas_2d_context,
        |c| c.line_to(x, y),
    )?;
    Ok(JsValue::Undefined)
}

/// `rect(x, y, w, h)` (HTML §4.12.5.1.5 — add a closed rectangle subpath).
pub(super) fn native_oc_rect(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (x, y, w, h) = rect_args(ctx, args)?;
    dispatch_2d_method(
        ctx,
        this,
        "rect",
        false,
        require_offscreen_canvas_2d_context,
        |c| c.rect(x, y, w, h),
    )?;
    Ok(JsValue::Undefined)
}

/// `arc(x, y, radius, startAngle, endAngle, anticlockwise?)` (HTML §4.12.5.1.5
/// — add a circular arc subpath).
pub(super) fn native_oc_arc(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let x = arg_f32(ctx, args, 0)?;
    let y = arg_f32(ctx, args, 1)?;
    let radius = arg_f32(ctx, args, 2)?;
    let start = arg_f32(ctx, args, 3)?;
    let end = arg_f32(ctx, args, 4)?;
    let anticlockwise =
        coerce::to_boolean(ctx.vm, args.get(5).copied().unwrap_or(JsValue::Undefined));
    dispatch_2d_method(
        ctx,
        this,
        "arc",
        false,
        require_offscreen_canvas_2d_context,
        |c| {
            c.arc(x, y, radius, start, end, anticlockwise);
        },
    )?;
    Ok(JsValue::Undefined)
}

/// `fill()` (HTML §4.12.5.1.11 — fill the current default path with fillStyle).
pub(super) fn native_oc_fill(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _: &[JsValue],
) -> Result<JsValue, VmError> {
    dispatch_2d_method(
        ctx,
        this,
        "fill",
        true,
        require_offscreen_canvas_2d_context,
        Canvas2dContext::fill,
    )?;
    Ok(JsValue::Undefined)
}

/// `stroke()` (HTML §4.12.5.1.11 — stroke the current default path with strokeStyle).
pub(super) fn native_oc_stroke(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _: &[JsValue],
) -> Result<JsValue, VmError> {
    dispatch_2d_method(
        ctx,
        this,
        "stroke",
        true,
        require_offscreen_canvas_2d_context,
        Canvas2dContext::stroke,
    )?;
    Ok(JsValue::Undefined)
}

/// `fillRect(x, y, w, h)` (HTML §4.12.5.1.9 — fill a rectangle with fillStyle).
pub(super) fn native_oc_fill_rect(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (x, y, w, h) = rect_args(ctx, args)?;
    dispatch_2d_method(
        ctx,
        this,
        "fillRect",
        true,
        require_offscreen_canvas_2d_context,
        |c| c.fill_rect(x, y, w, h),
    )?;
    Ok(JsValue::Undefined)
}

/// `strokeRect(x, y, w, h)` (HTML §4.12.5.1.9 — stroke a rectangle's outline).
pub(super) fn native_oc_stroke_rect(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (x, y, w, h) = rect_args(ctx, args)?;
    dispatch_2d_method(
        ctx,
        this,
        "strokeRect",
        true,
        require_offscreen_canvas_2d_context,
        |c| c.stroke_rect(x, y, w, h),
    )?;
    Ok(JsValue::Undefined)
}

/// `clearRect(x, y, w, h)` (HTML §4.12.5.1.9 — clear a rectangle to transparent black).
pub(super) fn native_oc_clear_rect(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (x, y, w, h) = rect_args(ctx, args)?;
    dispatch_2d_method(
        ctx,
        this,
        "clearRect",
        true,
        require_offscreen_canvas_2d_context,
        |c| c.clear_rect(x, y, w, h),
    )?;
    Ok(JsValue::Undefined)
}

/// `translate(tx, ty)` (HTML §4.12.5.1.6 — translate the current transform).
pub(super) fn native_oc_translate(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (tx, ty) = (arg_f32(ctx, args, 0)?, arg_f32(ctx, args, 1)?);
    dispatch_2d_method(
        ctx,
        this,
        "translate",
        false,
        require_offscreen_canvas_2d_context,
        |c| c.translate(tx, ty),
    )?;
    Ok(JsValue::Undefined)
}

/// `rotate(angle)` (HTML §4.12.5.1.6 — rotate the current transform, radians).
pub(super) fn native_oc_rotate(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let angle = arg_f32(ctx, args, 0)?;
    dispatch_2d_method(
        ctx,
        this,
        "rotate",
        false,
        require_offscreen_canvas_2d_context,
        |c| c.rotate(angle),
    )?;
    Ok(JsValue::Undefined)
}

/// `scale(sx, sy)` (HTML §4.12.5.1.6 — scale the current transform).
pub(super) fn native_oc_scale(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (sx, sy) = (arg_f32(ctx, args, 0)?, arg_f32(ctx, args, 1)?);
    dispatch_2d_method(
        ctx,
        this,
        "scale",
        false,
        require_offscreen_canvas_2d_context,
        |c| c.scale(sx, sy),
    )?;
    Ok(JsValue::Undefined)
}

/// `measureText(text)` (HTML §4.12.5.1.10 — measure text width).
/// Returns a `TextMetrics`-shaped object carrying `width` only (full glyph
/// metrics deferred with text rendering, `#11-canvas-2d-extended-ops`).
pub(super) fn native_oc_measure_text(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let text = match args.first().copied() {
        Some(v) => {
            let sid = coerce::to_string(ctx.vm, v)?;
            ctx.vm.strings.get_utf8(sid)
        }
        None => String::new(),
    };
    let width = dispatch_2d_method(
        ctx,
        this,
        "measureText",
        false,
        require_offscreen_canvas_2d_context,
        |c| c.measure_text(&text),
    )?;
    let object_proto = ctx.vm.object_prototype;
    let metrics = ctx.vm.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: object_proto,
        extensible: true,
    });
    let width_sid = ctx.vm.well_known.width;
    ctx.vm.define_shaped_property(
        metrics,
        PropertyKey::String(width_sid),
        PropertyValue::Data(JsValue::Number(f64::from(width))),
        shape::PropertyAttrs::WEBIDL_RO,
    );
    Ok(JsValue::Object(metrics))
}

// ---------------------------------------------------------------------------
// Style + back-ref accessors (HTML §4.12.5.1.8 / §4.12.5.1.13 + §4.12.5.1 IDL)
// ---------------------------------------------------------------------------

/// `fillStyle` getter (HTML §4.12.5.1.8 — fill style serialized as CSS color string).
pub(super) fn oc_fill_style_getter(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _: &[JsValue],
) -> Result<JsValue, VmError> {
    let color = dispatch_2d_method(
        ctx,
        this,
        "fillStyle",
        false,
        require_offscreen_canvas_2d_context,
        |c| c.fill_style(),
    )?;
    let sid = ctx
        .vm
        .strings
        .intern(&elidex_web_canvas::serialize_canvas_color(color));
    Ok(JsValue::String(sid))
}

/// `fillStyle` setter (HTML §4.12.5.1.8 — accepts CSS color string; gradients /
/// patterns deferred to `#11-canvas-gradient-pattern`).
pub(super) fn oc_fill_style_setter(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let value = color_arg(ctx, args)?;
    dispatch_2d_method(
        ctx,
        this,
        "fillStyle",
        false,
        require_offscreen_canvas_2d_context,
        |c| c.set_fill_style(&value),
    )?;
    Ok(JsValue::Undefined)
}

/// `strokeStyle` getter (HTML §4.12.5.1.8 — stroke style serialized as CSS color string).
pub(super) fn oc_stroke_style_getter(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _: &[JsValue],
) -> Result<JsValue, VmError> {
    let color = dispatch_2d_method(
        ctx,
        this,
        "strokeStyle",
        false,
        require_offscreen_canvas_2d_context,
        |c| c.stroke_style(),
    )?;
    let sid = ctx
        .vm
        .strings
        .intern(&elidex_web_canvas::serialize_canvas_color(color));
    Ok(JsValue::String(sid))
}

/// `strokeStyle` setter (HTML §4.12.5.1.8 — accepts CSS color string; gradients /
/// patterns deferred to `#11-canvas-gradient-pattern`).
pub(super) fn oc_stroke_style_setter(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let value = color_arg(ctx, args)?;
    dispatch_2d_method(
        ctx,
        this,
        "strokeStyle",
        false,
        require_offscreen_canvas_2d_context,
        |c| {
            c.set_stroke_style(&value);
        },
    )?;
    Ok(JsValue::Undefined)
}

/// `lineWidth` getter (HTML §4.12.5.1.3 — line-styles state, current stroke width).
pub(super) fn oc_line_width_getter(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _: &[JsValue],
) -> Result<JsValue, VmError> {
    let w = dispatch_2d_method(
        ctx,
        this,
        "lineWidth",
        false,
        require_offscreen_canvas_2d_context,
        |c| c.line_width(),
    )?;
    Ok(JsValue::Number(f64::from(w)))
}

/// `lineWidth` setter (HTML §4.12.5.1.3 — non-positive / non-finite silently ignored per spec).
pub(super) fn oc_line_width_setter(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let w = arg_f32(ctx, args, 0)?;
    dispatch_2d_method(
        ctx,
        this,
        "lineWidth",
        false,
        require_offscreen_canvas_2d_context,
        |c| c.set_line_width(w),
    )?;
    Ok(JsValue::Undefined)
}

/// `globalAlpha` getter (HTML §4.12.5.1.13 — compositing alpha multiplier).
pub(super) fn oc_global_alpha_getter(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _: &[JsValue],
) -> Result<JsValue, VmError> {
    let a = dispatch_2d_method(
        ctx,
        this,
        "globalAlpha",
        false,
        require_offscreen_canvas_2d_context,
        |c| c.global_alpha(),
    )?;
    Ok(JsValue::Number(f64::from(a)))
}

/// `globalAlpha` setter (HTML §4.12.5.1.13 — out-of-`[0.0, 1.0]` silently ignored per spec).
pub(super) fn oc_global_alpha_setter(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let a = arg_f32(ctx, args, 0)?;
    dispatch_2d_method(
        ctx,
        this,
        "globalAlpha",
        false,
        require_offscreen_canvas_2d_context,
        |c| c.set_global_alpha(a),
    )?;
    Ok(JsValue::Undefined)
}

/// Read-only `canvas` back-reference (HTML §4.12.5.3.1 IDL `canvas` attribute
/// on `OffscreenCanvasRenderingContext2D`) — returns the owning
/// `OffscreenCanvas` wrapper (the context wrapper shares its OC entity, so the
/// OC wrapper is just the `cache_wrapper`-cached wrapper for that entity).
pub(super) fn oc_canvas_back_ref_getter(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_offscreen_canvas_2d_context(ctx, this, "canvas")?;
    // Cached at ctor; if unbinding has cleared it, return null (defensive).
    let wrapper = ctx
        .host()
        .get_cached_wrapper(entity)
        .map_or(JsValue::Null, JsValue::Object);
    Ok(wrapper)
}

/// `ToString`-coerce a `fillStyle` / `strokeStyle` assignment (only CSS color
/// strings are supported in v1; gradients/patterns deferred to slot
/// `#11-canvas-gradient-pattern` — shared with D-21 `<canvas>` side).
fn color_arg(ctx: &mut NativeContext<'_>, args: &[JsValue]) -> Result<String, VmError> {
    let arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = coerce::to_string(ctx.vm, arg)?;
    Ok(ctx.vm.strings.get_utf8(sid))
}
