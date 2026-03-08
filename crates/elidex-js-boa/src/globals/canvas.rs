//! Canvas 2D context JS bindings.
//!
//! Provides the `CanvasRenderingContext2D` object returned by
//! `canvas.getContext("2d")`. Drawing methods delegate to the
//! `Canvas2dContext` stored in the `HostBridge`.

use std::sync::Arc;

use boa_engine::object::builtins::JsArray;
use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsNativeError, JsResult, JsValue, NativeFunction};
use elidex_api_canvas::{serialize_canvas_color, Canvas2dContext};
use elidex_ecs::ImageData;
use elidex_plugin::CssColor;

use crate::bridge::HostBridge;
use crate::globals::element::ENTITY_KEY;

/// Key for storing the canvas element reference on the context2d object.
const CANVAS_ELEMENT_KEY: &str = "__elidex_canvas_element__";

/// Register a canvas drawing method on an `ObjectInitializer`.
///
/// Handles the common boilerplate: bridge clone, entity extraction,
/// f32 arg conversion, `Canvas2dContext` dispatch, and optional `ImageData` sync.
macro_rules! canvas_method {
    // Zero-arg method (function pointer), no sync.
    ($init:expr, $bridge:expr, $name:literal, $method:expr) => {
        let b = $bridge.clone();
        $init.function(
            NativeFunction::from_copy_closure_with_captures(
                |this, _args, bridge, ctx| {
                    let bits = extract_entity_bits(this, ctx)?;
                    bridge.with_canvas(bits, $method);
                    Ok(JsValue::undefined())
                },
                b,
            ),
            js_string!($name),
            0,
        );
    };
    // Zero-arg method (function pointer), with ImageData sync.
    ($init:expr, $bridge:expr, $name:literal, $method:expr, sync) => {
        let b = $bridge.clone();
        $init.function(
            NativeFunction::from_copy_closure_with_captures(
                |this, _args, bridge, ctx| {
                    let bits = extract_entity_bits(this, ctx)?;
                    bridge.with_canvas(bits, $method);
                    sync_canvas_to_image_data(bridge, bits);
                    Ok(JsValue::undefined())
                },
                b,
            ),
            js_string!($name),
            0,
        );
    };
    // N f32-arg method, no sync.
    ($init:expr, $bridge:expr, $name:literal, $n:literal,
     |$c:ident, $($p:ident),+| $body:expr) => {
        let b = $bridge.clone();
        $init.function(
            NativeFunction::from_copy_closure_with_captures(
                |this, args, bridge, ctx| {
                    let bits = extract_entity_bits(this, ctx)?;
                    let mut _i = 0usize;
                    $(
                        let $p = arg_f32(args, _i, ctx)?;
                        _i += 1;
                    )+
                    let _ = _i;
                    bridge.with_canvas(bits, |$c| $body);
                    Ok(JsValue::undefined())
                },
                b,
            ),
            js_string!($name),
            $n,
        );
    };
    // N f32-arg method, with ImageData sync.
    ($init:expr, $bridge:expr, $name:literal, $n:literal,
     |$c:ident, $($p:ident),+| $body:expr, sync) => {
        let b = $bridge.clone();
        $init.function(
            NativeFunction::from_copy_closure_with_captures(
                |this, args, bridge, ctx| {
                    let bits = extract_entity_bits(this, ctx)?;
                    let mut _i = 0usize;
                    $(
                        let $p = arg_f32(args, _i, ctx)?;
                        _i += 1;
                    )+
                    let _ = _i;
                    bridge.with_canvas(bits, |$c| $body);
                    sync_canvas_to_image_data(bridge, bits);
                    Ok(JsValue::undefined())
                },
                b,
            ),
            js_string!($name),
            $n,
        );
    };
}

/// Sync the canvas pixel buffer to the ECS `ImageData` component.
///
/// Called after each drawing operation to ensure the next render frame
/// picks up the updated pixels via the existing `DisplayItem::Image` path.
///
/// # Implementation note
///
/// Pixel extraction (`with_canvas`) and ECS insertion (`with`) must be
/// separate calls because both borrow the `HostBridge` inner `RefCell`.
/// Nesting them would cause a double-borrow panic.
// TODO(Phase 4): defer ImageData sync to once-per-frame instead of per-draw-call.
fn sync_canvas_to_image_data(bridge: &HostBridge, entity_bits: u64) {
    let Some((width, height, pixels)) = bridge.with_canvas(entity_bits, |ctx| {
        (ctx.width(), ctx.height(), ctx.to_rgba8_straight())
    }) else {
        return;
    };

    bridge.with(|_session, dom| {
        let image_data = ImageData {
            pixels: Arc::new(pixels),
            width,
            height,
        };
        let Some(entity) = elidex_ecs::Entity::from_bits(entity_bits) else {
            return;
        };
        let _ = dom.world_mut().insert_one(entity, image_data);
    });
}

/// Create a `CanvasRenderingContext2D` JS object for the given canvas entity.
///
/// The object has drawing methods that delegate to the `Canvas2dContext`
/// stored in the `HostBridge`.
#[allow(clippy::too_many_lines)]
pub(crate) fn create_context2d_object(
    entity_bits: u64,
    canvas_element: &JsValue,
    bridge: &HostBridge,
    ctx: &mut Context,
) -> JsValue {
    let mut init = ObjectInitializer::new(ctx);

    // Store entity reference for identity.
    // TODO(Phase 4): entity bits stored as f64 loses precision for values > 2^53.
    // hecs entity IDs are typically small, so this is safe for now.
    init.property(
        js_string!(ENTITY_KEY),
        JsValue::from(entity_bits as f64),
        Attribute::empty(),
    );

    // Store canvas element reference for the `canvas` property (WHATWG spec).
    init.property(
        js_string!(CANVAS_ELEMENT_KEY),
        canvas_element.clone(),
        Attribute::empty(),
    );

    let realm = init.context().realm().clone();

    // --- Rectangle methods ---
    canvas_method!(
        init,
        bridge,
        "fillRect",
        4,
        |c, x, y, w, h| c.fill_rect(x, y, w, h),
        sync
    );
    canvas_method!(
        init,
        bridge,
        "strokeRect",
        4,
        |c, x, y, w, h| c.stroke_rect(x, y, w, h),
        sync
    );
    canvas_method!(
        init,
        bridge,
        "clearRect",
        4,
        |c, x, y, w, h| c.clear_rect(x, y, w, h),
        sync
    );

    // --- Path methods ---
    canvas_method!(init, bridge, "beginPath", Canvas2dContext::begin_path);
    canvas_method!(init, bridge, "moveTo", 2, |c, x, y| c.move_to(x, y));
    canvas_method!(init, bridge, "lineTo", 2, |c, x, y| c.line_to(x, y));
    canvas_method!(init, bridge, "closePath", Canvas2dContext::close_path);
    canvas_method!(init, bridge, "rect", 4, |c, x, y, w, h| c.rect(x, y, w, h));
    register_arc_method(&mut init, bridge);
    canvas_method!(init, bridge, "fill", Canvas2dContext::fill, sync);
    canvas_method!(init, bridge, "stroke", Canvas2dContext::stroke, sync);

    // --- State methods ---
    canvas_method!(init, bridge, "save", Canvas2dContext::save);
    canvas_method!(init, bridge, "restore", Canvas2dContext::restore);

    // --- Transform methods ---
    canvas_method!(init, bridge, "translate", 2, |c, tx, ty| c
        .translate(tx, ty));
    canvas_method!(init, bridge, "rotate", 1, |c, angle| c.rotate(angle));
    canvas_method!(init, bridge, "scale", 2, |c, sx, sy| c.scale(sx, sy));

    // --- Text ---
    register_measure_text(&mut init, bridge);

    // --- Image data ---
    register_image_data_methods(&mut init, bridge);

    // --- Style accessors ---
    register_color_accessor(
        &mut init,
        &realm,
        bridge,
        "fillStyle",
        Canvas2dContext::fill_style,
        Canvas2dContext::set_fill_style,
    );
    register_color_accessor(
        &mut init,
        &realm,
        bridge,
        "strokeStyle",
        Canvas2dContext::stroke_style,
        Canvas2dContext::set_stroke_style,
    );
    register_f32_accessor(
        &mut init,
        &realm,
        bridge,
        "lineWidth",
        Canvas2dContext::line_width,
        1.0,
        Canvas2dContext::set_line_width,
    );
    register_f32_accessor(
        &mut init,
        &realm,
        bridge,
        "globalAlpha",
        Canvas2dContext::global_alpha,
        1.0,
        Canvas2dContext::set_global_alpha,
    );

    // canvas (read-only getter) — returns the canvas element (WHATWG spec).
    let getter = NativeFunction::from_copy_closure(|this, _args, ctx| {
        let obj = this
            .as_object()
            .ok_or_else(|| JsNativeError::typ().with_message("expected a context object"))?;
        let canvas = obj.get(js_string!(CANVAS_ELEMENT_KEY), ctx)?;
        Ok(canvas)
    })
    .to_js_function(&realm);

    init.accessor(
        js_string!("canvas"),
        Some(getter),
        None,
        Attribute::CONFIGURABLE,
    );

    init.build().into()
}

// --- Sub-functions for complex method registration ---

/// Register the `arc(x, y, radius, startAngle, endAngle, anticlockwise?)` method.
fn register_arc_method(init: &mut ObjectInitializer<'_>, bridge: &HostBridge) {
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let bits = extract_entity_bits(this, ctx)?;
                let x = arg_f32(args, 0, ctx)?;
                let y = arg_f32(args, 1, ctx)?;
                let r = arg_f32(args, 2, ctx)?;
                let start = arg_f32(args, 3, ctx)?;
                let end = arg_f32(args, 4, ctx)?;
                let ccw = args.get(5).is_some_and(JsValue::to_boolean);
                bridge.with_canvas(bits, |c| c.arc(x, y, r, start, end, ccw));
                Ok(JsValue::undefined())
            },
            b,
        ),
        js_string!("arc"),
        5,
    );
}

/// Register the `measureText(text) → { width }` method.
fn register_measure_text(init: &mut ObjectInitializer<'_>, bridge: &HostBridge) {
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let bits = extract_entity_bits(this, ctx)?;
                let text = args
                    .first()
                    .map(|v| v.to_string(ctx))
                    .transpose()?
                    .map(|s| s.to_std_string_escaped())
                    .unwrap_or_default();
                let width = bridge
                    .with_canvas(bits, |c| c.measure_text(&text))
                    .unwrap_or(0.0);
                let result = ObjectInitializer::new(ctx)
                    .property(
                        js_string!("width"),
                        JsValue::from(f64::from(width)),
                        Attribute::all(),
                    )
                    .build();
                Ok(result.into())
            },
            b,
        ),
        js_string!("measureText"),
        1,
    );
}

/// Register `getImageData`, `putImageData`, and `createImageData` methods.
fn register_image_data_methods(init: &mut ObjectInitializer<'_>, bridge: &HostBridge) {
    // getImageData(sx, sy, sw, sh)
    // TODO(Phase 4): return Uint8ClampedArray instead of plain Array for spec compliance.
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let bits = extract_entity_bits(this, ctx)?;
                let sx = arg_i32(args, 0, ctx)?;
                let sy = arg_i32(args, 1, ctx)?;
                let sw = arg_u32(args, 2, ctx)?;
                let sh = arg_u32(args, 3, ctx)?;
                if sw == 0 || sh == 0 {
                    return Err(JsNativeError::range()
                        .with_message("getImageData: width and height must be non-zero")
                        .into());
                }
                let data = bridge
                    .with_canvas(bits, |c| c.get_image_data(sx, sy, sw, sh))
                    .unwrap_or_default();
                build_image_data_object(&data, sw, sh, ctx)
            },
            b,
        ),
        js_string!("getImageData"),
        4,
    );

    // putImageData(imageData, dx, dy)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let bits = extract_entity_bits(this, ctx)?;
                let image_data_val = args.first().ok_or_else(|| {
                    JsNativeError::typ().with_message("putImageData requires an ImageData argument")
                })?;
                let image_data_obj = image_data_val.as_object().ok_or_else(|| {
                    JsNativeError::typ().with_message("putImageData requires an ImageData object")
                })?;
                let dx = arg_i32(args, 1, ctx)?;
                let dy = arg_i32(args, 2, ctx)?;
                let sw = image_data_obj.get(js_string!("width"), ctx)?.to_u32(ctx)?;
                let sh = image_data_obj.get(js_string!("height"), ctx)?.to_u32(ctx)?;
                let data_obj = image_data_obj
                    .get(js_string!("data"), ctx)?
                    .as_object()
                    .ok_or_else(|| {
                        JsNativeError::typ().with_message("ImageData.data is not an object")
                    })?
                    .clone();
                let Some(len) = (sw as usize)
                    .checked_mul(sh as usize)
                    .and_then(|n| n.checked_mul(4))
                else {
                    return Err(JsNativeError::range()
                        .with_message("ImageData dimensions too large")
                        .into());
                };
                let mut pixels = vec![0u8; len];
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                for (i, pixel) in pixels.iter_mut().enumerate() {
                    let val = data_obj.get(i as u32, ctx)?;
                    let n = val.to_number(ctx)?;
                    *pixel = n.clamp(0.0, 255.0) as u8;
                }
                bridge.with_canvas(bits, |c| c.put_image_data(&pixels, dx, dy, sw, sh));
                sync_canvas_to_image_data(bridge, bits);
                Ok(JsValue::undefined())
            },
            b,
        ),
        js_string!("putImageData"),
        3,
    );

    // createImageData(width, height)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, _bridge, ctx| {
                let w = arg_u32(args, 0, ctx)?;
                let h = arg_u32(args, 1, ctx)?;
                if w == 0 || h == 0 {
                    return Err(JsNativeError::range()
                        .with_message("createImageData: width and height must be non-zero")
                        .into());
                }
                let Some(len) = (w as usize)
                    .checked_mul(h as usize)
                    .and_then(|n| n.checked_mul(4))
                else {
                    return Err(JsNativeError::range()
                        .with_message("ImageData dimensions too large")
                        .into());
                };
                let data = vec![0u8; len];
                build_image_data_object(&data, w, h, ctx)
            },
            b,
        ),
        js_string!("createImageData"),
        2,
    );
}

// --- Accessor registration helpers ---

/// Register a CSS color property accessor (getter returns serialized color string,
/// setter parses CSS color string).
fn register_color_accessor(
    init: &mut ObjectInitializer<'_>,
    realm: &boa_engine::realm::Realm,
    bridge: &HostBridge,
    name: &str,
    get: fn(&Canvas2dContext) -> CssColor,
    set: fn(&mut Canvas2dContext, &str),
) {
    let b = bridge.clone();
    let getter = NativeFunction::from_copy_closure_with_captures(
        move |this, _args, bridge, ctx| {
            let bits = extract_entity_bits(this, ctx)?;
            let color = bridge
                .with_canvas(bits, |c| get(c))
                .unwrap_or(CssColor::BLACK);
            Ok(JsValue::from(js_string!(serialize_canvas_color(color))))
        },
        b,
    )
    .to_js_function(realm);

    let b = bridge.clone();
    let setter = NativeFunction::from_copy_closure_with_captures(
        move |this, args, bridge, ctx| {
            let bits = extract_entity_bits(this, ctx)?;
            let color_str = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            bridge.with_canvas(bits, |c| set(c, &color_str));
            Ok(JsValue::undefined())
        },
        b,
    )
    .to_js_function(realm);

    init.accessor(
        js_string!(name),
        Some(getter),
        Some(setter),
        Attribute::CONFIGURABLE,
    );
}

/// Register an f32 property accessor (getter returns f64, setter takes f32).
fn register_f32_accessor(
    init: &mut ObjectInitializer<'_>,
    realm: &boa_engine::realm::Realm,
    bridge: &HostBridge,
    name: &str,
    get: fn(&Canvas2dContext) -> f32,
    default: f32,
    set: fn(&mut Canvas2dContext, f32),
) {
    let b = bridge.clone();
    let getter = NativeFunction::from_copy_closure_with_captures(
        move |this, _args, bridge, ctx| {
            let bits = extract_entity_bits(this, ctx)?;
            let v = bridge.with_canvas(bits, |c| get(c)).unwrap_or(default);
            Ok(JsValue::from(f64::from(v)))
        },
        b,
    )
    .to_js_function(realm);

    let b = bridge.clone();
    let setter = NativeFunction::from_copy_closure_with_captures(
        move |this, args, bridge, ctx| {
            let bits = extract_entity_bits(this, ctx)?;
            let v = arg_f32(args, 0, ctx)?;
            bridge.with_canvas(bits, |c| set(c, v));
            Ok(JsValue::undefined())
        },
        b,
    )
    .to_js_function(realm);

    init.accessor(
        js_string!(name),
        Some(getter),
        Some(setter),
        Attribute::CONFIGURABLE,
    );
}

// --- Arg extraction helpers ---

/// Extract entity bits from a context2d object's `__elidex_entity__` property.
#[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
fn extract_entity_bits(this: &JsValue, ctx: &mut Context) -> JsResult<u64> {
    let obj = this
        .as_object()
        .ok_or_else(|| JsNativeError::typ().with_message("expected a context object"))?;
    let val = obj.get(js_string!(ENTITY_KEY), ctx)?;
    let n = val.to_number(ctx)?;
    if !n.is_finite() || n < 0.0 {
        return Err(JsNativeError::typ()
            .with_message("invalid entity reference")
            .into());
    }
    Ok(n as u64)
}

/// Extract an `f32` argument from JS args, defaulting to `0.0`.
#[allow(clippy::cast_possible_truncation)]
fn arg_f32(args: &[JsValue], index: usize, ctx: &mut Context) -> JsResult<f32> {
    match args.get(index) {
        Some(v) => Ok(v.to_number(ctx)? as f32),
        None => Ok(0.0),
    }
}

/// Extract an `i32` argument from JS args, defaulting to `0`.
#[allow(clippy::cast_possible_truncation)]
fn arg_i32(args: &[JsValue], index: usize, ctx: &mut Context) -> JsResult<i32> {
    match args.get(index) {
        Some(v) => Ok(v.to_number(ctx)? as i32),
        None => Ok(0),
    }
}

/// Extract a `u32` argument from JS args, defaulting to `0`.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn arg_u32(args: &[JsValue], index: usize, ctx: &mut Context) -> JsResult<u32> {
    match args.get(index) {
        Some(v) => Ok(v.to_number(ctx)? as u32),
        None => Ok(0),
    }
}

/// Build an `ImageData` JS object with `data`, `width`, and `height` properties.
fn build_image_data_object(
    data: &[u8],
    width: u32,
    height: u32,
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let js_array = JsArray::new(ctx);
    for &byte in data {
        js_array.push(JsValue::from(f64::from(byte)), ctx)?;
    }
    let result = ObjectInitializer::new(ctx)
        .property(
            js_string!("data"),
            JsValue::from(js_array),
            Attribute::all(),
        )
        .property(
            js_string!("width"),
            JsValue::from(f64::from(width)),
            Attribute::all(),
        )
        .property(
            js_string!("height"),
            JsValue::from(f64::from(height)),
            Attribute::all(),
        )
        .build();
    Ok(result.into())
}
