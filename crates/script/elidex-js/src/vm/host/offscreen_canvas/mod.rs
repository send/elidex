//! `OffscreenCanvas` + `OffscreenCanvasRenderingContext2D` host binding
//! (WHATWG HTML §4.12.5.1.7 "The OffscreenCanvas interface"). Main-thread
//! side only; worker-side transferable receipt is deferred to
//! `#11-offscreen-canvas-worker-transfer`.
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", this file holds only engine-bound
//! marshalling: brand-check, `JsValue`↔`u32`/`f32`/`String` coercion, method
//! dispatch into the shared 2D raster backend, wrapper creation. The OC
//! component plumbing (`OffscreenCanvasDims`, `PlaceholderCanvas`,
//! `spawn_offscreen_canvas_entity`, `ensure_offscreen_context`,
//! `set_offscreen_canvas_*`, `transfer_canvas_to_offscreen`) lives in the
//! engine-independent [`elidex_api_canvas`] crate; the raster algorithm
//! itself is `Canvas2dContext` from [`elidex_web_canvas`] — shared verbatim
//! with the `<canvas>` D-21 binding (DR-2: one issue, one way).
//!
//! ## ECS-native storage + wrapper identity
//!
//! The OC entity is a [`elidex_ecs::NodeKind::OffscreenCanvas`] in the main
//! `EcsDom` (one entity per `new OffscreenCanvas(w, h)` OR per
//! `transferControlToOffscreen()` invocation). It carries
//! `OffscreenCanvasDims` (always) and `Canvas2dContext` (lazy, after first
//! `getContext('2d')`). The primary JS wrapper goes through the existing
//! `cache_wrapper` / `WrapperKind::Node` path (TRUE Worker precedent,
//! mirror of `worker.rs:400` cache_wrapper call). The context wrapper's
//! *identity* (`oc.getContext('2d') === …`) is interned via the
//! wrapper-identity seam under
//! [`WrapperKind::OffscreenCanvas2dContext`](super::super::wrapper_intern::WrapperKind::OffscreenCanvas2dContext)
//! — a 1-variant seam extension parallel to D-21's `Canvas2dContext`. Brand
//! checks read `NodeKind::OffscreenCanvas` from the entity (mirror of
//! `worker.rs::require_worker`).

#![cfg(feature = "engine")]

use elidex_api_canvas::{
    ensure_offscreen_context, offscreen_canvas_dimensions, set_offscreen_canvas_height,
    set_offscreen_canvas_width, spawn_offscreen_canvas_entity,
};
use elidex_ecs::{Entity, NodeKind};
use elidex_web_canvas::Canvas2dContext;

use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, VmError,
};
use super::super::wrapper_intern::{WrapperKey, WrapperKind};
use super::super::{coerce, shape, NativeFn, VmInner};
use super::canvas::dispatch_2d_method;

mod encode;

/// `OffscreenCanvasRenderingContext2D.prototype` methods (HTML §4.12.5.1.7,
/// shares the §4.12.5.1 surface). Same names as D-21's `CONTEXT_METHODS`;
/// brand-checked into the OC-context entity instead of a `<canvas>` entity.
const CONTEXT_METHODS: &[(&str, NativeFn)] = &[
    ("save", native_oc_save),
    ("restore", native_oc_restore),
    ("beginPath", native_oc_begin_path),
    ("closePath", native_oc_close_path),
    ("moveTo", native_oc_move_to),
    ("lineTo", native_oc_line_to),
    ("rect", native_oc_rect),
    ("arc", native_oc_arc),
    ("fill", native_oc_fill),
    ("stroke", native_oc_stroke),
    ("fillRect", native_oc_fill_rect),
    ("strokeRect", native_oc_stroke_rect),
    ("clearRect", native_oc_clear_rect),
    ("translate", native_oc_translate),
    ("rotate", native_oc_rotate),
    ("scale", native_oc_scale),
    ("measureText", native_oc_measure_text),
];

impl VmInner {
    /// Install `OffscreenCanvas.prototype` (chaining `EventTarget.prototype` —
    /// OC is an EventTarget but not a Node, HTML §4.12.5.1.7 IDL) and the
    /// constructable `OffscreenCanvas` interface object on `globalThis`.
    pub(in crate::vm) fn register_offscreen_canvas_global(&mut self) {
        let event_target_proto = self.event_target_prototype.expect(
            "register_offscreen_canvas_global called before register_event_target_prototype",
        );
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(event_target_proto),
            extensible: true,
        });
        // `transferToImageBitmap` is intentionally NOT installed — the spec
        // mandates it return an `ImageBitmap`, which is itself a separate
        // interface deferred to `#11-offscreen-canvas-non-2d-contexts` (the
        // ImageBitmap surface lands with `bitmaprenderer`). Per CLAUDE.md
        // "No stub surface", leaving the method undefined lets feature-detect
        // (`typeof oc.transferToImageBitmap === 'function'`) correctly report
        // "not supported" rather than installing a throw-stub that fakes
        // existence.
        self.install_methods(
            proto_id,
            &[
                ("getContext", native_oc_get_context),
                ("convertToBlob", encode::native_oc_convert_to_blob),
            ],
        );
        self.install_offscreen_canvas_accessors(proto_id);
        self.offscreen_canvas_prototype = Some(proto_id);

        let global_sid = self.strings.intern("OffscreenCanvas");
        super::events::install_ctor(
            self,
            proto_id,
            "OffscreenCanvas",
            native_offscreen_canvas_constructor,
            global_sid,
        );
    }

    /// Install `OffscreenCanvasRenderingContext2D.prototype` (HTML §4.12.5.1.7
    /// — shares the §4.12.5.1 surface) chaining `Object.prototype`, plus the
    /// (non-constructable) `OffscreenCanvasRenderingContext2D` interface object
    /// on `globalThis` so `ctx instanceof OffscreenCanvasRenderingContext2D`
    /// holds.
    pub(in crate::vm) fn register_offscreen_canvas_rendering_context_2d_prototype(&mut self) {
        let object_proto = self.object_prototype.expect(
            "register_offscreen_canvas_rendering_context_2d_prototype called before register_prototypes",
        );
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });
        self.install_methods(proto_id, CONTEXT_METHODS);
        self.install_oc_context_accessors(proto_id);
        self.offscreen_canvas_rendering_context_2d_prototype = Some(proto_id);

        let global_sid = self.strings.intern("OffscreenCanvasRenderingContext2D");
        super::events::install_ctor(
            self,
            proto_id,
            "OffscreenCanvasRenderingContext2D",
            native_oc_illegal_constructor,
            global_sid,
        );
    }

    /// IDL `width` / `height` accessor pair on `OffscreenCanvas.prototype`
    /// (HTML §4.12.5.1.7 IDL: `[EnforceRange] unsigned long long`).
    fn install_offscreen_canvas_accessors(&mut self, proto_id: ObjectId) {
        let width_sid = self.well_known.width;
        let height_sid = self.well_known.height;
        self.install_accessor_pair(
            proto_id,
            width_sid,
            oc_width_getter,
            Some(oc_width_setter),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        self.install_accessor_pair(
            proto_id,
            height_sid,
            oc_height_getter,
            Some(oc_height_setter),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
    }

    /// `fillStyle` / `strokeStyle` / `lineWidth` / `globalAlpha` accessor pairs
    /// plus the read-only `canvas` back-reference on
    /// `OffscreenCanvasRenderingContext2D.prototype` (HTML §4.12.5.1.7 / §4.12.5.1).
    fn install_oc_context_accessors(&mut self, proto_id: ObjectId) {
        let pairs: &[(&str, NativeFn, Option<NativeFn>)] = &[
            (
                "fillStyle",
                oc_fill_style_getter,
                Some(oc_fill_style_setter),
            ),
            (
                "strokeStyle",
                oc_stroke_style_getter,
                Some(oc_stroke_style_setter),
            ),
            (
                "lineWidth",
                oc_line_width_getter,
                Some(oc_line_width_setter),
            ),
            (
                "globalAlpha",
                oc_global_alpha_getter,
                Some(oc_global_alpha_setter),
            ),
            ("canvas", oc_canvas_back_ref_getter, None),
        ];
        for &(name, getter, setter) in pairs {
            let sid = self.strings.intern(name);
            self.install_accessor_pair(
                proto_id,
                sid,
                getter,
                setter,
                shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Brand checks
// ---------------------------------------------------------------------------

/// Resolve `this` to the backing `OffscreenCanvas` entity if it brands as one
/// (a `HostObject` over a [`NodeKind::OffscreenCanvas`] entity); a `TypeError`
/// ("Illegal invocation") otherwise. Mirror of the
/// [`super::worker::require_worker`] shape (D-18 precedent — brand-check via
/// `NodeKind` on the entity, no dedicated component needed because `OC` has no
/// per-entity transport handle like Worker does).
fn require_offscreen_canvas(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Entity, VmError> {
    let illegal = || {
        VmError::type_error(format!(
            "Failed to execute '{method}' on 'OffscreenCanvas': Illegal invocation"
        ))
    };
    let JsValue::Object(id) = this else {
        return Err(illegal());
    };
    let ObjectKind::HostObject { entity_bits } = ctx.vm.get_object(id).kind else {
        return Err(illegal());
    };
    let entity = Entity::from_bits(entity_bits).ok_or_else(illegal)?;
    // Reject the OC2D context wrapper, which shares the OC entity in its
    // entity_bits but is not the OC element wrapper (mirror of the
    // require_canvas_element reverse-exclusion in canvas/mod.rs).
    if ctx.vm.get_wrapper(WrapperKey::entity(
        entity,
        WrapperKind::OffscreenCanvas2dContext,
    )) == Some(id)
    {
        return Err(illegal());
    }
    let kind = ctx
        .host()
        .dom()
        .world()
        .get::<&NodeKind>(entity)
        .ok()
        .map(|k| *k);
    if kind == Some(NodeKind::OffscreenCanvas) {
        Ok(entity)
    } else {
        Err(illegal())
    }
}

/// Resolve `this` to the backing OC entity if it brands as an
/// `OffscreenCanvasRenderingContext2D` (i.e. it is the seam-interned
/// [`WrapperKind::OffscreenCanvas2dContext`] wrapper for its entity).
pub(super) fn require_offscreen_canvas_2d_context(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Entity, VmError> {
    let illegal = || {
        VmError::type_error(format!(
            "Failed to execute '{method}' on 'OffscreenCanvasRenderingContext2D': Illegal invocation"
        ))
    };
    let JsValue::Object(id) = this else {
        return Err(illegal());
    };
    let ObjectKind::HostObject { entity_bits } = ctx.vm.get_object(id).kind else {
        return Err(illegal());
    };
    let entity = Entity::from_bits(entity_bits).ok_or_else(illegal)?;
    if ctx.vm.get_wrapper(WrapperKey::entity(
        entity,
        WrapperKind::OffscreenCanvas2dContext,
    )) == Some(id)
    {
        Ok(entity)
    } else {
        Err(illegal())
    }
}

// ---------------------------------------------------------------------------
// Constructor + getContext
// ---------------------------------------------------------------------------

/// Coerce a JsValue per WebIDL §3.10.10 `unsigned long long` (the IDL type
/// declared for the `OffscreenCanvas` ctor args + `width` / `height` setters,
/// HTML §4.12.5.1.7). Uses [`coerce::f64_to_uint64_loose`] (the canonical
/// WebIDL `unsigned long long` coercion shared with `ProgressEvent.loaded` /
/// `.total`, WHATWG XHR §10) then saturates to `u32::MAX` for the backend (the
/// bitmap dimension range — `Canvas2dContext` allocates `u32` pixmaps and
/// clamps 0/unrepresentable to 1×1). Differs from `coerce::to_uint32` (used by
/// `<canvas>.width`/`height` which are WebIDL §3.10.9 `unsigned long`, 32-bit)
/// in that values in `(2^32, 2^53]` saturate rather than wrap-mod-2³².
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn coerce_oc_dim(vm: &mut VmInner, value: JsValue) -> Result<u32, VmError> {
    let n = coerce::to_number(vm, value)?;
    let u64_loose = coerce::f64_to_uint64_loose(n);
    Ok(if u64_loose >= f64::from(u32::MAX) {
        u32::MAX
    } else {
        u64_loose as u32
    })
}

/// `new OffscreenCanvas(width, height)` (HTML §4.12.5.1.7 constructor steps
/// 1-4). Both args are `[EnforceRange] unsigned long long` per IDL; coerced
/// via [`coerce_oc_dim`] (the IDL `unsigned long long` path, saturating to
/// `u32::MAX` for the backend). Throws `TypeError` if fewer than 2 args.
fn native_offscreen_canvas_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if !ctx.is_construct() {
        return Err(VmError::type_error(
            "Failed to construct 'OffscreenCanvas': Please use the 'new' operator",
        ));
    }
    if args.len() < 2 {
        return Err(VmError::type_error(format!(
            "Failed to construct 'OffscreenCanvas': 2 arguments required, but only {} present.",
            args.len()
        )));
    }
    let width = coerce_oc_dim(ctx.vm, args[0])?;
    let height = coerce_oc_dim(ctx.vm, args[1])?;

    let entity = spawn_offscreen_canvas_entity(ctx.host().dom(), width, height);

    let JsValue::Object(inst_id) = this else {
        unreachable!("constructor `this` is always an Object after `do_new`")
    };
    ctx.vm.get_object_mut(inst_id).kind = ObjectKind::HostObject {
        entity_bits: entity.to_bits().get(),
    };
    ctx.host().cache_wrapper(entity, inst_id);
    Ok(JsValue::Object(inst_id))
}

/// `OffscreenCanvas.prototype.getContext(contextId)` (HTML §4.12.5.1.7 — "get
/// a context for a canvas" algorithm). Returns the SameObject
/// `OffscreenCanvasRenderingContext2D` wrapper for `'2d'`, or `null` for any
/// other (unsupported) context type. `'webgl'` / `'webgl2'` /
/// `'bitmaprenderer'` are deferred to `#11-offscreen-canvas-non-2d-contexts`.
fn native_oc_get_context(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_offscreen_canvas(ctx, this, "getContext")?;
    let context_type = match args.first().copied() {
        Some(v) => {
            let sid = coerce::to_string(ctx.vm, v)?;
            ctx.vm.strings.get_utf8(sid)
        }
        None => {
            return Err(VmError::type_error(
                "Failed to execute 'getContext' on 'OffscreenCanvas': 1 argument required, but only 0 present.",
            ))
        }
    };
    if context_type != "2d" {
        return Ok(JsValue::Null);
    }
    if !ensure_offscreen_context(ctx.host().dom(), entity) {
        return Ok(JsValue::Null);
    }
    let proto = ctx.vm.offscreen_canvas_rendering_context_2d_prototype;
    let wrapper = ctx.vm.intern_wrapper(
        WrapperKey::entity(entity, WrapperKind::OffscreenCanvas2dContext),
        |vm| {
            vm.alloc_object(Object {
                kind: ObjectKind::HostObject {
                    entity_bits: entity.to_bits().get(),
                },
                storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
                prototype: proto,
                extensible: true,
            })
        },
    );
    Ok(JsValue::Object(wrapper))
}

/// `OffscreenCanvasRenderingContext2D` interface object is exposed for
/// `instanceof` but is not constructable (WebIDL — no `[[Construct]]`).
fn native_oc_illegal_constructor(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Err(VmError::type_error("Illegal constructor"))
}

// ---------------------------------------------------------------------------
// IDL width / height accessors
// ---------------------------------------------------------------------------

fn oc_width_getter(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_offscreen_canvas(ctx, this, "width")?;
    let (w, _) = offscreen_canvas_dimensions(ctx.host().dom(), entity);
    Ok(JsValue::Number(f64::from(w)))
}

fn oc_height_getter(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_offscreen_canvas(ctx, this, "height")?;
    let (_, h) = offscreen_canvas_dimensions(ctx.host().dom(), entity);
    Ok(JsValue::Number(f64::from(h)))
}

fn oc_width_setter(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_offscreen_canvas(ctx, this, "width")?;
    let v = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = coerce_oc_dim(ctx.vm, v)?;
    set_offscreen_canvas_width(ctx.host().dom(), entity, n);
    Ok(JsValue::Undefined)
}

fn oc_height_setter(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_offscreen_canvas(ctx, this, "height")?;
    let v = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = coerce_oc_dim(ctx.vm, v)?;
    set_offscreen_canvas_height(ctx.host().dom(), entity, n);
    Ok(JsValue::Undefined)
}

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
// ---------------------------------------------------------------------------

fn native_oc_save(
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

fn native_oc_restore(
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

fn native_oc_begin_path(
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

fn native_oc_close_path(
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

fn native_oc_move_to(
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

fn native_oc_line_to(
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

fn native_oc_rect(
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

fn native_oc_arc(
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

fn native_oc_fill(
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

fn native_oc_stroke(
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

fn native_oc_fill_rect(
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

fn native_oc_stroke_rect(
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

fn native_oc_clear_rect(
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

fn native_oc_translate(
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

fn native_oc_rotate(
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

fn native_oc_scale(
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

/// `measureText(text)` (HTML §4.12.5.1 — `OffscreenCanvasRenderingContext2D`
/// shares the §4.12.5.1.12 surface). Returns a `TextMetrics`-shaped object
/// carrying `width` only (full glyph metrics deferred with text rendering,
/// `#11-canvas-2d-extended-ops`).
fn native_oc_measure_text(
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
// Style + back-ref accessors
// ---------------------------------------------------------------------------

fn oc_fill_style_getter(
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

fn oc_fill_style_setter(
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

fn oc_stroke_style_getter(
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

fn oc_stroke_style_setter(
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

fn oc_line_width_getter(
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

fn oc_line_width_setter(
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

fn oc_global_alpha_getter(
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

fn oc_global_alpha_setter(
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

/// Read-only `canvas` back-reference — returns the owning `OffscreenCanvas`
/// wrapper (the context wrapper shares its OC entity, so the OC wrapper is
/// just the `cache_wrapper`-cached wrapper for that entity).
fn oc_canvas_back_ref_getter(
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
