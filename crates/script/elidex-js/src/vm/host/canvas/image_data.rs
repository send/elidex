//! `ImageData` interface + canvas pixel manipulation (HTML §4.12.5.1.16 "Pixel
//! manipulation"): the `getImageData`, `putImageData`, `createImageData`
//! methods plus the constructable `ImageData` global. Split out of the canvas
//! host module (the [`super`] sibling) to keep each file under the ~1000-line
//! convention.
//!
//! Marshalling-only, per the Layering mandate: brand-check, coercion, and
//! `Uint8ClampedArray` construction, reaching the raster backend through the
//! parent module's [`super::dispatch_context`].

#![cfg(feature = "engine")]

use elidex_web_canvas::Canvas2dContext;

use super::super::super::value::{
    ElementKind, JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey,
    PropertyStorage, PropertyValue, VmError,
};
use super::super::super::{coerce, shape, VmInner};
use super::super::array_buffer;
use super::{arg_i32, dispatch_context, require_canvas_2d_context};

/// `getImageData(sx, sy, sw, sh)` — returns a fresh `ImageData` whose `data`
/// `Uint8ClampedArray` holds the requested region (straight-alpha RGBA8).
pub(super) fn native_get_image_data(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let sx = arg_i32(ctx, args, 0)?;
    let sy = arg_i32(ctx, args, 1)?;
    let sw = require_image_data_dim(ctx, args, 2, "getImageData", "width")?;
    let sh = require_image_data_dim(ctx, args, 3, "getImageData", "height")?;
    // Cap before the backend's `vec![0; sw*sh*4]` so untrusted JS can't OOM-abort.
    checked_image_bytes(sw, sh)?;
    let pixels = dispatch_context(ctx, this, "getImageData", false, |c| {
        c.get_image_data(sx, sy, sw, sh)
    })?;
    build_image_data(ctx, sw, sh, pixels)
}

/// `putImageData(imageData, dx, dy)` — writes the `ImageData`'s pixels into the
/// bitmap at `(dx, dy)`.
pub(super) fn native_put_image_data(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let img = args.first().copied().unwrap_or(JsValue::Undefined);
    let (width, height, data) = read_image_data_object(ctx, img)?;
    let dx = arg_i32(ctx, args, 1)?;
    let dy = arg_i32(ctx, args, 2)?;
    dispatch_context(ctx, this, "putImageData", true, |c| {
        c.put_image_data(&data, dx, dy, width, height);
    })?;
    Ok(JsValue::Undefined)
}

/// `createImageData(sw, sh)` / `createImageData(imagedata)` — returns a fresh
/// transparent-black `ImageData` of the given (or copied) dimensions.
pub(super) fn native_create_image_data(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_canvas_2d_context(ctx, this, "createImageData")?;
    let (w, h) = match args.first().copied() {
        Some(JsValue::Object(_)) => {
            let (width, height, _) = read_image_data_object(ctx, args[0])?;
            (width, height)
        }
        _ => (
            require_image_data_dim(ctx, args, 0, "createImageData", "width")?,
            require_image_data_dim(ctx, args, 1, "createImageData", "height")?,
        ),
    };
    checked_image_bytes(w, h)?;
    let pixels = Canvas2dContext::create_image_data(w, h);
    build_image_data(ctx, w, h, pixels)
}

/// `new ImageData(width, height)` / `new ImageData(Uint8ClampedArray, width[,
/// height])` (HTML §4.12.5.1.16). The single-arg-object forms of the canvas
/// factories do not pass through here.
pub(super) fn native_image_data_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if !ctx.is_construct() {
        return Err(VmError::type_error(
            "Failed to construct 'ImageData': Please use the 'new' operator",
        ));
    }
    // The `do_new`-allocated receiver, promoted in place (its prototype is
    // already `new.target.prototype`, so subclassing is preserved).
    let JsValue::Object(inst_id) = this else {
        unreachable!("constructor `this` is always an Object after `do_new`")
    };
    let first = args.first().copied().unwrap_or(JsValue::Undefined);
    match first {
        // new ImageData(Uint8ClampedArray, width[, height]) — HTML §4.12.5.1.16.
        // The data overload requires a `Uint8ClampedArray` specifically; any
        // other TypedArray fails WebIDL overload resolution → TypeError. The
        // passed array becomes the `data` property *by reference* (spec
        // "initialize this's data to source") — no copy, identity preserved.
        JsValue::Object(id)
            if matches!(
                ctx.vm.get_object(id).kind,
                ObjectKind::TypedArray {
                    element_kind: ElementKind::Uint8Clamped,
                    ..
                }
            ) =>
        {
            let byte_len = typed_array_byte_len(ctx.vm, id).unwrap_or(0);
            // Data length must be a nonzero integral multiple of 4 (RGBA).
            if byte_len == 0 || !byte_len.is_multiple_of(4) {
                return Err(VmError::dom_exception(
                    ctx.vm.well_known.dom_exc_invalid_state_error,
                    "Failed to construct 'ImageData': The input data length is not a non-zero multiple of 4.",
                ));
            }
            let len_pixels = byte_len / 4;
            let width = dim_arg(ctx, args, 1)?;
            if width == 0 {
                return Err(VmError::dom_exception(
                    ctx.vm.well_known.dom_exc_index_size_error,
                    "Failed to construct 'ImageData': The source width is zero or not a number.",
                ));
            }
            let width_px = width as usize;
            let height = if args.get(2).is_some() {
                // Explicit height: width × height must exactly cover the data.
                let h = dim_arg(ctx, args, 2)?;
                if width_px.checked_mul(h as usize) != Some(len_pixels) {
                    return Err(VmError::dom_exception(
                        ctx.vm.well_known.dom_exc_index_size_error,
                        "Failed to construct 'ImageData': The input data length is not equal to (4 * width * height).",
                    ));
                }
                h
            } else {
                // Derived height: the data must divide evenly by the width.
                if !len_pixels.is_multiple_of(width_px) {
                    return Err(VmError::dom_exception(
                        ctx.vm.well_known.dom_exc_index_size_error,
                        "Failed to construct 'ImageData': The input data length is not a multiple of (4 * width).",
                    ));
                }
                #[allow(clippy::cast_possible_truncation)]
                let h = (len_pixels / width_px) as u32;
                h
            };
            set_image_data_props(ctx, inst_id, width, height, id)?;
        }
        // A non-`Uint8ClampedArray` TypedArray fails the data-overload's WebIDL
        // type check (and must not silently fall through to the (sw, sh) form).
        JsValue::Object(id)
            if matches!(ctx.vm.get_object(id).kind, ObjectKind::TypedArray { .. }) =>
        {
            return Err(VmError::type_error(
                "Failed to construct 'ImageData': The provided value is not of type 'Uint8ClampedArray'.",
            ));
        }
        // new ImageData(width, height) — fresh transparent-black buffer.
        _ => {
            let width = dim_arg(ctx, args, 0)?;
            let height = dim_arg(ctx, args, 1)?;
            if width == 0 {
                return Err(VmError::dom_exception(
                    ctx.vm.well_known.dom_exc_index_size_error,
                    "Failed to construct 'ImageData': The source width is zero or not a number.",
                ));
            }
            if height == 0 {
                return Err(VmError::dom_exception(
                    ctx.vm.well_known.dom_exc_index_size_error,
                    "Failed to construct 'ImageData': The source height is zero or not a number.",
                ));
            }
            checked_image_bytes(width, height)?;
            let bytes = Canvas2dContext::create_image_data(width, height);
            let data_id = make_uint8_clamped_array(ctx, bytes)?;
            set_image_data_props(ctx, inst_id, width, height, data_id)?;
        }
    }
    Ok(JsValue::Object(inst_id))
}

/// Coerce a `new ImageData(sw, sh)` dimension argument — WebIDL `unsigned long`,
/// so the canonical ToUint32 (`coerce::to_uint32`, mod-2³²). A missing arg → 0;
/// the caller rejects 0 / validates the data length.
fn dim_arg(ctx: &mut NativeContext<'_>, args: &[JsValue], i: usize) -> Result<u32, VmError> {
    let v = args.get(i).copied().unwrap_or(JsValue::Undefined);
    coerce::to_uint32(ctx.vm, v)
}

/// Coerce a `getImageData` / `createImageData` `sw`/`sh` argument — WebIDL
/// `long` (HTML §4.12.5.1.16), so the canonical ToInt32 (`coerce::to_int32`).
/// A zero magnitude throws `IndexSizeError`; otherwise the absolute magnitude
/// is used (a negative dimension flips the rectangle, per spec). `unsigned_abs`
/// handles `i32::MIN` without overflow. `dim` names the member for the error.
fn require_image_data_dim(
    ctx: &mut NativeContext<'_>,
    args: &[JsValue],
    i: usize,
    method: &str,
    dim: &str,
) -> Result<u32, VmError> {
    let v = args.get(i).copied().unwrap_or(JsValue::Undefined);
    let n = coerce::to_int32(ctx.vm, v)?;
    if n == 0 {
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_index_size_error,
            format!(
                "Failed to execute '{method}' on 'CanvasRenderingContext2D': The source {dim} is zero."
            ),
        ));
    }
    Ok(n.unsigned_abs())
}

/// Maximum `ImageData` byte length the VM will allocate. Guards untrusted JS
/// from triggering an OOM-abort with a huge-but-representable getImageData /
/// createImageData / `new ImageData` request (e.g. 50000×50000×4 ≈ 10 GiB) that
/// `checked_mul` alone wouldn't catch.
const MAX_IMAGE_DATA_BYTES: usize = 1 << 31; // 2 GiB

/// `width*height*4` if representable AND within [`MAX_IMAGE_DATA_BYTES`], else a
/// `RangeError`. A preflight before any backend pixel-buffer allocation.
fn checked_image_bytes(width: u32, height: u32) -> Result<usize, VmError> {
    (width as usize)
        .checked_mul(height as usize)
        .and_then(|wh| wh.checked_mul(4))
        .filter(|&n| n <= MAX_IMAGE_DATA_BYTES)
        .ok_or_else(|| {
            VmError::range_error(
                "Failed to allocate ImageData: the requested size exceeds the maximum.",
            )
        })
}

/// Build a fresh `ImageData` object (own `data` / `width` / `height`) backed by
/// a freshly-allocated `Uint8ClampedArray` holding `pixels` (getImageData /
/// createImageData). The constructor's data overload instead reuses the
/// caller's array via [`set_image_data_props`].
fn build_image_data(
    ctx: &mut NativeContext<'_>,
    width: u32,
    height: u32,
    pixels: Vec<u8>,
) -> Result<JsValue, VmError> {
    let proto = ctx.vm.image_data_prototype;
    let inst = ctx.vm.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });
    let mut g = ctx.vm.push_temp_root(JsValue::Object(inst));
    let mut ctx2 = NativeContext { vm: &mut g };
    let data_id = make_uint8_clamped_array(&mut ctx2, pixels)?;
    set_image_data_props(&mut ctx2, inst, width, height, data_id)?;
    Ok(JsValue::Object(inst))
}

/// Set the `data` / `width` / `height` own properties on an `ImageData`
/// instance from an already-allocated `data` `Uint8ClampedArray` (`data_id`).
///
/// Single sink for every ImageData construction. `width*height*4` must be
/// representable AND equal `data`'s byte length — this rejects the
/// `Canvas2dContext::create_image_data` / `get_image_data` overflow case (empty
/// buffer for nonzero dims) that would otherwise yield an `ImageData` with
/// nonzero dims but zero-length data.
#[allow(clippy::similar_names)]
fn set_image_data_props(
    ctx: &mut NativeContext<'_>,
    inst: ObjectId,
    width: u32,
    height: u32,
    data_id: ObjectId,
) -> Result<(), VmError> {
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|wh| wh.checked_mul(4));
    if expected.is_none() || expected != typed_array_byte_len(ctx.vm, data_id) {
        return Err(VmError::range_error(
            "Failed to construct 'ImageData': the requested dimensions are too large.",
        ));
    }
    let data_sid = ctx.vm.well_known.data;
    let width_sid = ctx.vm.well_known.width;
    let height_sid = ctx.vm.well_known.height;
    ctx.vm.define_shaped_property(
        inst,
        PropertyKey::String(data_sid),
        PropertyValue::Data(JsValue::Object(data_id)),
        shape::PropertyAttrs::WEBIDL_RO,
    );
    ctx.vm.define_shaped_property(
        inst,
        PropertyKey::String(width_sid),
        PropertyValue::Data(JsValue::Number(f64::from(width))),
        shape::PropertyAttrs::WEBIDL_RO,
    );
    ctx.vm.define_shaped_property(
        inst,
        PropertyKey::String(height_sid),
        PropertyValue::Data(JsValue::Number(f64::from(height))),
        shape::PropertyAttrs::WEBIDL_RO,
    );
    Ok(())
}

/// Allocate a `Uint8ClampedArray` whose backing buffer takes ownership of
/// `bytes` (no copy — the caller's owned buffer is moved straight into the
/// `ArrayBuffer`).
fn make_uint8_clamped_array(
    ctx: &mut NativeContext<'_>,
    bytes: Vec<u8>,
) -> Result<ObjectId, VmError> {
    array_buffer::create_typed_array_from_bytes(ctx.vm, bytes, ElementKind::Uint8Clamped)
}

/// The `[[ByteLength]]` of a TypedArray view (`None` if `id` is not one).
fn typed_array_byte_len(vm: &VmInner, id: ObjectId) -> Option<usize> {
    match vm.get_object(id).kind {
        ObjectKind::TypedArray { byte_length, .. } => Some(byte_length as usize),
        _ => None,
    }
}

/// Walk `id`'s prototype chain looking for `ImageData.prototype` — the brand
/// for `ImageData` instances and their subclasses. The hop cap guards against a
/// pathological cycle (prototype chains are acyclic in normal operation).
fn prototype_chain_has_image_data(vm: &VmInner, id: ObjectId) -> bool {
    let Some(target) = vm.image_data_prototype else {
        return false;
    };
    let mut cur = vm.get_object(id).prototype;
    for _ in 0..64 {
        match cur {
            Some(p) if p == target => return true,
            Some(p) => cur = vm.get_object(p).prototype,
            None => return false,
        }
    }
    false
}

/// Read + validate an `ImageData` argument (`putImageData` / `createImageData`):
/// its `width` / `height` and the bytes of its `data` `Uint8ClampedArray`.
///
/// Branding (no new `ObjectKind` — `ImageData` is an entity-less value object,
/// so lesson #276's ECS-component brand does not apply): `ImageData.prototype`
/// must be on the receiver's prototype *chain* (rejects plain spoofed objects,
/// accepts `class X extends ImageData {}` subclasses), AND `data` must be a
/// `Uint8ClampedArray` whose length equals `width*height*4` with positive
/// integral dims (the internal-consistency invariant a genuine `ImageData`
/// always holds). Anything else is not-an-`ImageData` (TypeError).
#[allow(clippy::similar_names)]
fn read_image_data_object(
    ctx: &mut NativeContext<'_>,
    value: JsValue,
) -> Result<(u32, u32, Vec<u8>), VmError> {
    let not_image_data =
        || VmError::type_error("parameter is not of type 'ImageData'.".to_string());
    let JsValue::Object(id) = value else {
        return Err(not_image_data());
    };
    // Prototype-chain brand: a genuine ImageData (or a subclass instance) has
    // `ImageData.prototype` on its chain; a plain object literal (chain ends at
    // `Object.prototype`) is rejected here.
    if !prototype_chain_has_image_data(ctx.vm, id) {
        return Err(not_image_data());
    }
    let width_sid = ctx.vm.well_known.width;
    let height_sid = ctx.vm.well_known.height;
    let data_sid = ctx.vm.well_known.data;
    let width = ctx.get_property_value(id, PropertyKey::String(width_sid))?;
    let height = ctx.get_property_value(id, PropertyKey::String(height_sid))?;
    let data = ctx.get_property_value(id, PropertyKey::String(data_sid))?;
    // A real `ImageData` has positive integral u32 dimensions; reject anything
    // else (zero / fractional / non-finite / out-of-range) as not-an-ImageData
    // so a spoofed `{width: 0, height: 0, data: <empty>}` cannot satisfy the
    // `data.length == width*height*4` invariant below by trivial `0 == 0`.
    let width =
        image_data_dim_value(coerce::to_number(ctx.vm, width)?).ok_or_else(not_image_data)?;
    let height =
        image_data_dim_value(coerce::to_number(ctx.vm, height)?).ok_or_else(not_image_data)?;
    let JsValue::Object(data_id) = data else {
        return Err(not_image_data());
    };
    // `data` must be a `Uint8ClampedArray` (not just any TypedArray).
    if !matches!(
        ctx.vm.get_object(data_id).kind,
        ObjectKind::TypedArray {
            element_kind: ElementKind::Uint8Clamped,
            ..
        }
    ) {
        return Err(not_image_data());
    }
    // Cap width*height*4 BEFORE the full-buffer copy below so a branded-but-
    // oversized ImageData (huge `data` array) can't OOM the process — mirrors
    // the getImageData / createImageData preflight (RangeError if too large).
    let expected = checked_image_bytes(width, height)?;
    let bytes = read_typed_array_bytes(ctx.vm, data_id).ok_or_else(not_image_data)?;
    // Internal-consistency invariant: data.length == width * height * 4.
    if expected != bytes.len() {
        return Err(not_image_data());
    }
    Ok((width, height, bytes))
}

/// Validate an `ImageData` `width`/`height` own property: a positive integral
/// value within `u32` range (the invariant a genuine `ImageData` always holds).
/// `None` rejects zero / fractional / non-finite / out-of-range spoofs.
fn image_data_dim_value(n: f64) -> Option<u32> {
    if n.is_finite() && n.fract() == 0.0 && n >= 1.0 && n <= f64::from(u32::MAX) {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        Some(n as u32)
    } else {
        None
    }
}

/// Snapshot the bytes a `Uint8ClampedArray` (or any TypedArray) view exposes,
/// or `None` if `id` is not a TypedArray. Delegates the buffer slicing to the
/// shared [`array_buffer::array_buffer_view_bytes`].
fn read_typed_array_bytes(vm: &VmInner, id: ObjectId) -> Option<Vec<u8>> {
    let ObjectKind::TypedArray {
        buffer_id,
        byte_offset,
        byte_length,
        ..
    } = vm.get_object(id).kind
    else {
        return None;
    };
    Some(array_buffer::array_buffer_view_bytes(
        vm,
        buffer_id,
        byte_offset,
        byte_length,
    ))
}
