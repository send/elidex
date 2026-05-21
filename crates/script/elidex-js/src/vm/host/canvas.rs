//! `HTMLCanvasElement.getContext('2d')` + `CanvasRenderingContext2D` +
//! `ImageData` host binding (WHATWG HTML §4.12.5 "The 2D rendering context").
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", this file holds only the engine-bound
//! marshalling: brand-check, `JsValue`↔`f32`/`String` coercion, method dispatch
//! into the engine-independent raster backend, and wrapper creation. The raster
//! algorithm lives in [`elidex_web_canvas::Canvas2dContext`]; the per-canvas-
//! entity component plumbing (insert/query/dirty/sync + the width/height bitmap-
//! reset reconciler) lives in [`elidex_api_canvas`].
//!
//! ## ECS-native storage + wrapper identity
//!
//! The raster state is a [`Canvas2dContext`] **component on the canvas
//! `Element` entity** (`Send + Sync`, so per the side-store audit it belongs on
//! the entity — SameObject = component get, despawn = automatic drop). The JS
//! context wrapper's *identity* (`canvas.getContext('2d') === …`) is interned
//! through the wrapper-identity seam under
//! [`WrapperKind::Canvas2dContext`](super::super::wrapper_intern::WrapperKind::Canvas2dContext);
//! that seam entry doubles as the **brand**: a `HostObject` is a 2D context iff
//! it is the interned `Canvas2dContext` wrapper for its entity. The wrapper
//! shares the canvas entity in its `entity_bits`, so the entity alone cannot
//! distinguish the context wrapper from the canvas-element wrapper — hence the
//! ObjectId-keyed seam brand here + the reverse exclusion in
//! [`require_node_arg`](super::node_proto::require_node_arg) that rejects a
//! context wrapper as a `Node`.

#![cfg(feature = "engine")]

use elidex_api_canvas::{ensure_context, mark_dirty, with_context};
use elidex_ecs::Entity;
use elidex_web_canvas::{serialize_canvas_color, Canvas2dContext};

use super::super::value::{
    ElementKind, JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey,
    PropertyStorage, PropertyValue, VmError,
};
use super::super::wrapper_intern::{WrapperKey, WrapperKind};
use super::super::{coerce, shape, NativeFn, VmInner};
use super::event_target::entity_from_this;

/// `CanvasRenderingContext2D.prototype` methods (HTML §4.12.5.1).
const CONTEXT_METHODS: &[(&str, NativeFn)] = &[
    ("save", native_save),
    ("restore", native_restore),
    ("beginPath", native_begin_path),
    ("closePath", native_close_path),
    ("moveTo", native_move_to),
    ("lineTo", native_line_to),
    ("rect", native_rect),
    ("arc", native_arc),
    ("fill", native_fill),
    ("stroke", native_stroke),
    ("fillRect", native_fill_rect),
    ("strokeRect", native_stroke_rect),
    ("clearRect", native_clear_rect),
    ("translate", native_translate),
    ("rotate", native_rotate),
    ("scale", native_scale),
    ("measureText", native_measure_text),
    ("getImageData", native_get_image_data),
    ("putImageData", native_put_image_data),
    ("createImageData", native_create_image_data),
];

impl VmInner {
    /// Install `HTMLCanvasElement.prototype` (HTML §4.12.5) chaining
    /// `HTMLElement.prototype`: the `getContext` method + `width` / `height`
    /// numeric-reflect accessors (the bitmap reset they trigger is driven from
    /// the `AttributeChange` SoT by `elidex_api_canvas::CanvasReconciler`, not
    /// these setters).
    pub(in crate::vm) fn register_html_canvas_prototype(&mut self) {
        let parent = self
            .html_element_prototype
            .expect("register_html_canvas_prototype called before register_html_element_prototype");
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(parent),
            extensible: true,
        });
        self.install_methods(proto_id, &[("getContext", native_get_context)]);
        let width_sid = self.well_known.width;
        let height_sid = self.well_known.height;
        self.install_accessor_pair(
            proto_id,
            width_sid,
            canvas_width_getter,
            Some(canvas_width_setter),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        self.install_accessor_pair(
            proto_id,
            height_sid,
            canvas_height_getter,
            Some(canvas_height_setter),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        self.html_canvas_prototype = Some(proto_id);
    }

    /// Install `CanvasRenderingContext2D.prototype` (HTML §4.12.5.1) chaining
    /// `Object.prototype`, plus the (non-constructable) `CanvasRenderingContext2D`
    /// interface object on `globalThis` so `ctx instanceof CanvasRenderingContext2D`
    /// holds.
    pub(in crate::vm) fn register_canvas_rendering_context_2d_prototype(&mut self) {
        let object_proto = self.object_prototype.expect(
            "register_canvas_rendering_context_2d_prototype called before register_prototypes",
        );
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });
        self.install_methods(proto_id, CONTEXT_METHODS);
        self.install_context_accessors(proto_id);
        self.canvas_rendering_context_2d_prototype = Some(proto_id);

        let global_sid = self.strings.intern("CanvasRenderingContext2D");
        super::events::install_ctor(
            self,
            proto_id,
            "CanvasRenderingContext2D",
            native_illegal_constructor,
            global_sid,
        );
    }

    /// Install `ImageData.prototype` + the constructable `ImageData` interface
    /// object on `globalThis` (HTML §4.12.5.1.16 "Pixel manipulation" — `new
    /// ImageData(w, h)` / `new ImageData(Uint8ClampedArray, w[, h])`).
    pub(in crate::vm) fn register_image_data_global(&mut self) {
        let object_proto = self
            .object_prototype
            .expect("register_image_data_global called before register_prototypes");
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });
        self.image_data_prototype = Some(proto_id);

        let global_sid = self.strings.intern("ImageData");
        super::events::install_ctor(
            self,
            proto_id,
            "ImageData",
            native_image_data_constructor,
            global_sid,
        );
    }

    /// `fillStyle` / `strokeStyle` (CSS color string) + `lineWidth` /
    /// `globalAlpha` (number) read/write accessors + read-only `canvas`
    /// back-reference (HTML §4.12.5.1 "Fill and stroke styles" / "Line styles" /
    /// "Compositing").
    fn install_context_accessors(&mut self, proto_id: ObjectId) {
        let pairs: &[(&str, NativeFn, Option<NativeFn>)] = &[
            ("fillStyle", fill_style_getter, Some(fill_style_setter)),
            (
                "strokeStyle",
                stroke_style_getter,
                Some(stroke_style_setter),
            ),
            ("lineWidth", line_width_getter, Some(line_width_setter)),
            (
                "globalAlpha",
                global_alpha_getter,
                Some(global_alpha_setter),
            ),
            ("canvas", canvas_back_ref_getter, None),
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
// Brand check
// ---------------------------------------------------------------------------

/// Resolve `this` to the backing canvas `Entity` if it brands as a
/// `CanvasRenderingContext2D` — i.e. it is the seam-interned
/// [`WrapperKind::Canvas2dContext`] wrapper for its entity (the brand). A
/// `TypeError` ("Illegal invocation") otherwise (including a plain canvas-
/// element wrapper, which shares the entity but is not the interned context
/// wrapper).
fn require_canvas_2d_context(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Entity, VmError> {
    let illegal = || {
        VmError::type_error(format!(
            "Failed to execute '{method}' on 'CanvasRenderingContext2D': Illegal invocation"
        ))
    };
    let JsValue::Object(id) = this else {
        return Err(illegal());
    };
    let ObjectKind::HostObject { entity_bits } = ctx.vm.get_object(id).kind else {
        return Err(illegal());
    };
    let entity = Entity::from_bits(entity_bits).ok_or_else(illegal)?;
    if ctx
        .vm
        .get_wrapper(WrapperKey::entity(entity, WrapperKind::Canvas2dContext))
        == Some(id)
    {
        Ok(entity)
    } else {
        Err(illegal())
    }
}

/// Is `id` the interned 2D-context wrapper for the entity it wraps? Used by
/// [`super::node_proto::require_node_arg`] to reject a context wrapper as a
/// `Node` argument (the reverse half of the bidirectional brand — a context
/// wrapper shares its canvas entity, so `is_node()` alone would wrongly accept
/// it, e.g. `document.appendChild(ctx)`).
pub(super) fn is_canvas_2d_context_wrapper(vm: &VmInner, id: ObjectId, entity: Entity) -> bool {
    vm.get_wrapper(WrapperKey::entity(entity, WrapperKind::Canvas2dContext)) == Some(id)
}

// ---------------------------------------------------------------------------
// getContext + dispatch helpers
// ---------------------------------------------------------------------------

/// `HTMLCanvasElement.getContext(contextId)` (HTML §4.12.5 "2D context creation
/// algorithm"). Returns the SameObject `CanvasRenderingContext2D` wrapper for
/// `'2d'`, `null` for any other (unsupported) context type, and `null` if the
/// canvas bitmap cannot be allocated (e.g. a zero dimension).
fn native_get_context(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Err(VmError::type_error(
            "Failed to execute 'getContext' on 'HTMLCanvasElement': Illegal invocation",
        ));
    };
    let context_type = match args.first().copied() {
        Some(v) => {
            let sid = coerce::to_string(ctx.vm, v)?;
            ctx.vm.strings.get_utf8(sid)
        }
        None => {
            return Err(VmError::type_error(
                "Failed to execute 'getContext' on 'HTMLCanvasElement': 1 argument required, but only 0 present.",
            ))
        }
    };
    if context_type != "2d" {
        return Ok(JsValue::Null);
    }
    if !ensure_context(ctx.host().dom(), entity) {
        return Ok(JsValue::Null);
    }
    let proto = ctx.vm.canvas_rendering_context_2d_prototype;
    let wrapper = ctx.vm.intern_wrapper(
        WrapperKey::entity(entity, WrapperKind::Canvas2dContext),
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

/// Brand-check `this`, then run `f` against the canvas entity's
/// [`Canvas2dContext`] component, marking the canvas dirty when `dirty` (a draw
/// that mutates the bitmap). The component is guaranteed present — the wrapper
/// only exists post-`ensure_context`.
fn dispatch_context<R>(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
    dirty: bool,
    f: impl FnOnce(&mut Canvas2dContext) -> R,
) -> Result<R, VmError> {
    let entity = require_canvas_2d_context(ctx, this, method)?;
    let result = with_context(ctx.host().dom(), entity, f)
        .expect("context wrapper exists ⇒ Canvas2dContext component present");
    if dirty {
        mark_dirty(ctx.host().dom(), entity);
    }
    Ok(result)
}

/// Coerce `args[i]` to `f32` via ToNumber (a missing arg → `0.0`, matching the
/// reference binding). Non-finite results are silently ignored by the backend
/// per the WHATWG spec.
fn arg_f32(ctx: &mut NativeContext<'_>, args: &[JsValue], i: usize) -> Result<f32, VmError> {
    match args.get(i).copied() {
        #[allow(clippy::cast_possible_truncation)]
        Some(v) => Ok(coerce::to_number(ctx.vm, v)? as f32),
        None => Ok(0.0),
    }
}

// ---------------------------------------------------------------------------
// State / path / transform / draw methods
// ---------------------------------------------------------------------------

fn native_save(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _: &[JsValue],
) -> Result<JsValue, VmError> {
    dispatch_context(ctx, this, "save", false, Canvas2dContext::save)?;
    Ok(JsValue::Undefined)
}

fn native_restore(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _: &[JsValue],
) -> Result<JsValue, VmError> {
    dispatch_context(ctx, this, "restore", false, Canvas2dContext::restore)?;
    Ok(JsValue::Undefined)
}

fn native_begin_path(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _: &[JsValue],
) -> Result<JsValue, VmError> {
    dispatch_context(ctx, this, "beginPath", false, Canvas2dContext::begin_path)?;
    Ok(JsValue::Undefined)
}

fn native_close_path(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _: &[JsValue],
) -> Result<JsValue, VmError> {
    dispatch_context(ctx, this, "closePath", false, Canvas2dContext::close_path)?;
    Ok(JsValue::Undefined)
}

fn native_move_to(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (x, y) = (arg_f32(ctx, args, 0)?, arg_f32(ctx, args, 1)?);
    dispatch_context(ctx, this, "moveTo", false, |c| c.move_to(x, y))?;
    Ok(JsValue::Undefined)
}

fn native_line_to(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (x, y) = (arg_f32(ctx, args, 0)?, arg_f32(ctx, args, 1)?);
    dispatch_context(ctx, this, "lineTo", false, |c| c.line_to(x, y))?;
    Ok(JsValue::Undefined)
}

fn native_rect(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (x, y, w, h) = rect_args(ctx, args)?;
    dispatch_context(ctx, this, "rect", false, |c| c.rect(x, y, w, h))?;
    Ok(JsValue::Undefined)
}

fn native_arc(
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
    dispatch_context(ctx, this, "arc", false, |c| {
        c.arc(x, y, radius, start, end, anticlockwise);
    })?;
    Ok(JsValue::Undefined)
}

fn native_fill(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _: &[JsValue],
) -> Result<JsValue, VmError> {
    dispatch_context(ctx, this, "fill", true, Canvas2dContext::fill)?;
    Ok(JsValue::Undefined)
}

fn native_stroke(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _: &[JsValue],
) -> Result<JsValue, VmError> {
    dispatch_context(ctx, this, "stroke", true, Canvas2dContext::stroke)?;
    Ok(JsValue::Undefined)
}

fn native_fill_rect(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (x, y, w, h) = rect_args(ctx, args)?;
    dispatch_context(ctx, this, "fillRect", true, |c| c.fill_rect(x, y, w, h))?;
    Ok(JsValue::Undefined)
}

fn native_stroke_rect(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (x, y, w, h) = rect_args(ctx, args)?;
    dispatch_context(ctx, this, "strokeRect", true, |c| c.stroke_rect(x, y, w, h))?;
    Ok(JsValue::Undefined)
}

fn native_clear_rect(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (x, y, w, h) = rect_args(ctx, args)?;
    dispatch_context(ctx, this, "clearRect", true, |c| c.clear_rect(x, y, w, h))?;
    Ok(JsValue::Undefined)
}

fn native_translate(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (tx, ty) = (arg_f32(ctx, args, 0)?, arg_f32(ctx, args, 1)?);
    dispatch_context(ctx, this, "translate", false, |c| c.translate(tx, ty))?;
    Ok(JsValue::Undefined)
}

fn native_rotate(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let angle = arg_f32(ctx, args, 0)?;
    dispatch_context(ctx, this, "rotate", false, |c| c.rotate(angle))?;
    Ok(JsValue::Undefined)
}

fn native_scale(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (sx, sy) = (arg_f32(ctx, args, 0)?, arg_f32(ctx, args, 1)?);
    dispatch_context(ctx, this, "scale", false, |c| c.scale(sx, sy))?;
    Ok(JsValue::Undefined)
}

/// Coerce the standard `(x, y, w, h)` rectangle argument quadruple.
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

/// `measureText(text)` (HTML §4.12.5.1.12) — returns a `TextMetrics`-shaped
/// object carrying `width` only (full glyph metrics are deferred with text
/// rendering, slot `#11-canvas-2d-extended-ops`).
fn native_measure_text(
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
    let width = dispatch_context(ctx, this, "measureText", false, |c| c.measure_text(&text))?;
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
// Style / dimension accessors
// ---------------------------------------------------------------------------

fn fill_style_getter(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _: &[JsValue],
) -> Result<JsValue, VmError> {
    let color = dispatch_context(ctx, this, "fillStyle", false, |c| c.fill_style())?;
    let sid = ctx.vm.strings.intern(&serialize_canvas_color(color));
    Ok(JsValue::String(sid))
}

fn fill_style_setter(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let value = color_arg(ctx, args)?;
    dispatch_context(ctx, this, "fillStyle", false, |c| c.set_fill_style(&value))?;
    Ok(JsValue::Undefined)
}

fn stroke_style_getter(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _: &[JsValue],
) -> Result<JsValue, VmError> {
    let color = dispatch_context(ctx, this, "strokeStyle", false, |c| c.stroke_style())?;
    let sid = ctx.vm.strings.intern(&serialize_canvas_color(color));
    Ok(JsValue::String(sid))
}

fn stroke_style_setter(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let value = color_arg(ctx, args)?;
    dispatch_context(ctx, this, "strokeStyle", false, |c| {
        c.set_stroke_style(&value);
    })?;
    Ok(JsValue::Undefined)
}

fn line_width_getter(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _: &[JsValue],
) -> Result<JsValue, VmError> {
    let w = dispatch_context(ctx, this, "lineWidth", false, |c| c.line_width())?;
    Ok(JsValue::Number(f64::from(w)))
}

fn line_width_setter(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let w = arg_f32(ctx, args, 0)?;
    dispatch_context(ctx, this, "lineWidth", false, |c| c.set_line_width(w))?;
    Ok(JsValue::Undefined)
}

fn global_alpha_getter(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _: &[JsValue],
) -> Result<JsValue, VmError> {
    let a = dispatch_context(ctx, this, "globalAlpha", false, |c| c.global_alpha())?;
    Ok(JsValue::Number(f64::from(a)))
}

fn global_alpha_setter(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let a = arg_f32(ctx, args, 0)?;
    dispatch_context(ctx, this, "globalAlpha", false, |c| c.set_global_alpha(a))?;
    Ok(JsValue::Undefined)
}

/// Read-only `canvas` back-reference — returns the owning `<canvas>` element
/// wrapper (the context wrapper shares its entity, so the element wrapper is
/// just the seam-interned `Node` wrapper for that entity).
fn canvas_back_ref_getter(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_canvas_2d_context(ctx, this, "canvas")?;
    Ok(JsValue::Object(ctx.vm.create_element_wrapper(entity)))
}

/// `ToString`-coerce a `fillStyle` / `strokeStyle` assignment (only CSS color
/// strings are supported in v1; gradients/patterns deferred to slot
/// `#11-canvas-gradient-pattern`). An invalid color string leaves the current
/// style unchanged (backend behavior).
fn color_arg(ctx: &mut NativeContext<'_>, args: &[JsValue]) -> Result<String, VmError> {
    let arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = coerce::to_string(ctx.vm, arg)?;
    Ok(ctx.vm.strings.get_utf8(sid))
}

// ---------------------------------------------------------------------------
// canvas.width / canvas.height numeric-reflect accessors
// ---------------------------------------------------------------------------

fn canvas_width_getter(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Number(f64::from(canvas_dim_attr(
        ctx, this, "width",
    ))))
}

fn canvas_height_getter(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Number(f64::from(canvas_dim_attr(
        ctx, this, "height",
    ))))
}

fn canvas_width_setter(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    set_canvas_dim_attr(ctx, this, "width", args)
}

fn canvas_height_setter(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    set_canvas_dim_attr(ctx, this, "height", args)
}

/// Read a `<canvas>` `width`/`height` IDL attribute (reflected non-negative
/// integer, default 300 / 150 — HTML §4.12.5).
fn canvas_dim_attr(ctx: &mut NativeContext<'_>, this: JsValue, name: &str) -> u32 {
    let default = if name == "width" {
        elidex_web_canvas::DEFAULT_WIDTH
    } else {
        elidex_web_canvas::DEFAULT_HEIGHT
    };
    let Some(entity) = entity_from_this(ctx, this) else {
        return default;
    };
    let (w, h) = elidex_api_canvas::canvas_dimensions(ctx.host().dom(), entity);
    if name == "width" {
        w
    } else {
        h
    }
}

/// Write a `<canvas>` `width`/`height` IDL attribute. Routes through the
/// `set_attribute` chokepoint so the bitmap reset fires uniformly via
/// `CanvasReconciler` (the `AttributeChange` SoT), not here.
fn set_canvas_dim_attr(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    name: &str,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Err(VmError::type_error(format!(
            "Failed to set the '{name}' property on 'HTMLCanvasElement': Illegal invocation"
        )));
    };
    let value = arg_f32(ctx, args, 0)?;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let n = if value.is_finite() && value >= 0.0 {
        value as u32
    } else {
        0
    };
    ctx.host().dom().set_attribute(entity, name, &n.to_string());
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// ImageData interface + pixel manipulation (HTML §4.12.5.1.16)
// ---------------------------------------------------------------------------

/// `getImageData(sx, sy, sw, sh)` — returns a fresh `ImageData` whose `data`
/// `Uint8ClampedArray` holds the requested region (straight-alpha RGBA8).
fn native_get_image_data(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    #[allow(clippy::cast_possible_truncation)]
    let sx = arg_f32(ctx, args, 0)? as i32;
    #[allow(clippy::cast_possible_truncation)]
    let sy = arg_f32(ctx, args, 1)? as i32;
    let sw = require_image_data_dim(ctx, args, 2, "getImageData", "width")?;
    let sh = require_image_data_dim(ctx, args, 3, "getImageData", "height")?;
    let pixels = dispatch_context(ctx, this, "getImageData", false, |c| {
        c.get_image_data(sx, sy, sw, sh)
    })?;
    build_image_data(ctx, sw, sh, &pixels)
}

/// `putImageData(imageData, dx, dy)` — writes the `ImageData`'s pixels into the
/// bitmap at `(dx, dy)`.
fn native_put_image_data(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let img = args.first().copied().unwrap_or(JsValue::Undefined);
    let (width, height, data) = read_image_data_object(ctx, img)?;
    #[allow(clippy::cast_possible_truncation)]
    let dx = arg_f32(ctx, args, 1)? as i32;
    #[allow(clippy::cast_possible_truncation)]
    let dy = arg_f32(ctx, args, 2)? as i32;
    dispatch_context(ctx, this, "putImageData", true, |c| {
        c.put_image_data(&data, dx, dy, width, height);
    })?;
    Ok(JsValue::Undefined)
}

/// `createImageData(sw, sh)` / `createImageData(imagedata)` — returns a fresh
/// transparent-black `ImageData` of the given (or copied) dimensions.
fn native_create_image_data(
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
    let pixels = Canvas2dContext::create_image_data(w, h);
    build_image_data(ctx, w, h, &pixels)
}

/// `new ImageData(width, height)` / `new ImageData(Uint8ClampedArray, width[,
/// height])` (HTML §4.12.5.1.16). The single-arg-object forms of the canvas
/// factories do not pass through here.
fn native_image_data_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if !ctx.is_construct() {
        return Err(VmError::type_error(
            "Failed to construct 'ImageData': Please use the 'new' operator",
        ));
    }
    let first = args.first().copied().unwrap_or(JsValue::Undefined);
    let (width, height, pixels) = match first {
        // new ImageData(Uint8ClampedArray, width[, height]) — HTML §4.12.5.1.16.
        // The data overload requires a `Uint8ClampedArray` specifically; any
        // other TypedArray fails WebIDL overload resolution → TypeError.
        JsValue::Object(id)
            if matches!(
                ctx.vm.get_object(id).kind,
                ObjectKind::TypedArray {
                    element_kind: ElementKind::Uint8Clamped,
                    ..
                }
            ) =>
        {
            let data = read_typed_array_bytes(ctx.vm, id).ok_or_else(|| {
                VmError::type_error("Failed to construct 'ImageData': invalid data array")
            })?;
            // Data length must be a nonzero integral multiple of 4 (RGBA).
            if data.is_empty() || data.len() % 4 != 0 {
                return Err(VmError::dom_exception(
                    ctx.vm.well_known.dom_exc_invalid_state_error,
                    "Failed to construct 'ImageData': The input data length is not a non-zero multiple of 4.",
                ));
            }
            let len_pixels = data.len() / 4;
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
                if len_pixels % width_px != 0 {
                    return Err(VmError::dom_exception(
                        ctx.vm.well_known.dom_exc_index_size_error,
                        "Failed to construct 'ImageData': The input data length is not a multiple of (4 * width).",
                    ));
                }
                #[allow(clippy::cast_possible_truncation)]
                let h = (len_pixels / width_px) as u32;
                h
            };
            (width, height, data)
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
        // new ImageData(width, height)
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
            let pixels = Canvas2dContext::create_image_data(width, height);
            (width, height, pixels)
        }
    };
    // Promote the `do_new`-allocated receiver in place (preserves
    // `new.target.prototype` for subclassing).
    let JsValue::Object(inst_id) = this else {
        unreachable!("constructor `this` is always an Object after `do_new`")
    };
    populate_image_data(ctx, inst_id, width, height, &pixels)?;
    Ok(JsValue::Object(inst_id))
}

/// Coerce a non-negative integer dimension argument (`sw` / `sh` / `width` /
/// `height`), defaulting a missing arg to 0.
fn dim_arg(ctx: &mut NativeContext<'_>, args: &[JsValue], i: usize) -> Result<u32, VmError> {
    let v = args.get(i).copied().unwrap_or(JsValue::Undefined);
    let n = coerce::to_number(ctx.vm, v)?;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let out = if n.is_finite() && n >= 0.0 {
        n as u32
    } else {
        0
    };
    Ok(out)
}

/// Coerce a `getImageData` / `createImageData` `sw`/`sh` argument (WebIDL
/// `long`, HTML §4.12.5.1.16): a zero or non-finite magnitude throws
/// `IndexSizeError`; otherwise the absolute magnitude is used (a negative
/// dimension flips the rectangle, per spec). `dim` names the member for the
/// error message.
fn require_image_data_dim(
    ctx: &mut NativeContext<'_>,
    args: &[JsValue],
    i: usize,
    method: &str,
    dim: &str,
) -> Result<u32, VmError> {
    let v = args.get(i).copied().unwrap_or(JsValue::Undefined);
    let n = coerce::to_number(ctx.vm, v)?;
    if !n.is_finite() || n == 0.0 {
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_index_size_error,
            format!(
                "Failed to execute '{method}' on 'CanvasRenderingContext2D': The source {dim} is zero."
            ),
        ));
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    Ok(n.abs() as u32)
}

/// Build a fresh `ImageData` object (own `data` / `width` / `height`) backed by
/// a `Uint8ClampedArray` holding `pixels`.
fn build_image_data(
    ctx: &mut NativeContext<'_>,
    width: u32,
    height: u32,
    pixels: &[u8],
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
    populate_image_data(&mut ctx2, inst, width, height, pixels)?;
    Ok(JsValue::Object(inst))
}

/// Set the `data` (`Uint8ClampedArray` over `pixels`) / `width` / `height` own
/// properties on an `ImageData` instance.
#[allow(clippy::similar_names)]
fn populate_image_data(
    ctx: &mut NativeContext<'_>,
    inst: ObjectId,
    width: u32,
    height: u32,
    pixels: &[u8],
) -> Result<(), VmError> {
    let data_id = make_uint8_clamped_array(ctx, pixels)?;
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

/// Allocate a `Uint8ClampedArray` whose backing buffer owns a copy of `bytes`.
fn make_uint8_clamped_array(
    ctx: &mut NativeContext<'_>,
    bytes: &[u8],
) -> Result<ObjectId, VmError> {
    super::array_buffer::create_typed_array_from_bytes(
        ctx.vm,
        bytes.to_vec(),
        ElementKind::Uint8Clamped,
    )
}

/// Read + validate an `ImageData` argument (`putImageData` / `createImageData`):
/// its `width` / `height` and the bytes of its `data` `Uint8ClampedArray`.
///
/// A real `ImageData` is internally consistent (`data.length == width*height*4`),
/// so anything failing that invariant — a spoofed plain object, a wrong-typed
/// `data`, or a length mismatch — is rejected as not-an-`ImageData` (TypeError),
/// matching the WebIDL `[EnforceRange]`/branding the spec assumes (no new
/// `ObjectKind` brand, per lesson #276 — the `Uint8ClampedArray`-of-exact-length
/// invariant is the brand).
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
    let width_sid = ctx.vm.well_known.width;
    let height_sid = ctx.vm.well_known.height;
    let data_sid = ctx.vm.well_known.data;
    let width = ctx.get_property_value(id, PropertyKey::String(width_sid))?;
    let height = ctx.get_property_value(id, PropertyKey::String(height_sid))?;
    let data = ctx.get_property_value(id, PropertyKey::String(data_sid))?;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let width = coerce::to_number(ctx.vm, width)? as u32;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let height = coerce::to_number(ctx.vm, height)? as u32;
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
    let bytes = read_typed_array_bytes(ctx.vm, data_id).ok_or_else(not_image_data)?;
    // Internal-consistency invariant: data.length == width * height * 4.
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|wh| wh.checked_mul(4));
    if expected != Some(bytes.len()) {
        return Err(not_image_data());
    }
    Ok((width, height, bytes))
}

/// Snapshot the bytes a `Uint8ClampedArray` (or any TypedArray) view exposes,
/// or `None` if `id` is not a TypedArray. Delegates the buffer slicing to the
/// shared [`super::array_buffer::array_buffer_view_bytes`].
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
    Some(super::array_buffer::array_buffer_view_bytes(
        vm,
        buffer_id,
        byte_offset,
        byte_length,
    ))
}

/// The `CanvasRenderingContext2D` / interface object is exposed for `instanceof`
/// but is not constructable (WebIDL — no `[[Construct]]`).
fn native_illegal_constructor(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Err(VmError::type_error("Illegal constructor"))
}
