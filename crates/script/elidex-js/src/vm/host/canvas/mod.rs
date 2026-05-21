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
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, VmError,
};
use super::super::wrapper_intern::{WrapperKey, WrapperKind};
use super::super::{coerce, shape, NativeFn, VmInner};
use super::event_target::entity_from_this;

mod image_data;

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
    ("getImageData", image_data::native_get_image_data),
    ("putImageData", image_data::native_put_image_data),
    ("createImageData", image_data::native_create_image_data),
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
            image_data::native_image_data_constructor,
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

/// Resolve `this` to the `<canvas>` `Entity` it wraps, or `TypeError` ("Illegal
/// invocation") if `this` is not a `HostObject` over a `canvas` element. Used by
/// the `HTMLCanvasElement.prototype` receivers (`getContext`, `width`/`height`)
/// so an extracted method called on a non-canvas (`getContext.call(div, '2d')`)
/// throws rather than attaching canvas state to an unrelated element — the
/// receiver brand-check every element-specific prototype binding performs.
fn require_canvas_element(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Entity, VmError> {
    let illegal = || {
        VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLCanvasElement': Illegal invocation"
        ))
    };
    let JsValue::Object(id) = this else {
        return Err(illegal());
    };
    let entity = entity_from_this(ctx, this).ok_or_else(illegal)?;
    // The `CanvasRenderingContext2D` wrapper shares its canvas entity (so the
    // tag check below would pass); reject it explicitly — only the canonical
    // canvas *element* wrapper is a valid HTMLCanvasElement receiver. Mirror of
    // the `require_node_arg` reverse-exclusion (the bidirectional brand).
    if is_canvas_2d_context_wrapper(ctx.vm, id, entity) {
        return Err(illegal());
    }
    if ctx.host().tag_matches_ascii_case(entity, "canvas") {
        Ok(entity)
    } else {
        Err(illegal())
    }
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
    let entity = require_canvas_element(ctx, this, "getContext")?;
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

/// Coerce `args[i]` to `i32` via ToNumber, going `f64`→`i32` **directly** (NOT
/// through `f32`, which loses integer precision above 2²⁴). For the WebIDL
/// `long` pixel-offset params (`getImageData` sx/sy, `putImageData` dx/dy); a
/// missing arg → `0`, non-finite → `0`.
fn arg_i32(ctx: &mut NativeContext<'_>, args: &[JsValue], i: usize) -> Result<i32, VmError> {
    match args.get(i).copied() {
        Some(v) => {
            let n = coerce::to_number(ctx.vm, v)?;
            #[allow(clippy::cast_possible_truncation)]
            Ok(if n.is_finite() { n as i32 } else { 0 })
        }
        None => Ok(0),
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
    let entity = require_canvas_element(ctx, this, "width")?;
    let (w, _) = elidex_api_canvas::canvas_dimensions(ctx.host().dom(), entity);
    Ok(JsValue::Number(f64::from(w)))
}

fn canvas_height_getter(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_canvas_element(ctx, this, "height")?;
    let (_, h) = elidex_api_canvas::canvas_dimensions(ctx.host().dom(), entity);
    Ok(JsValue::Number(f64::from(h)))
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

/// Write a `<canvas>` `width`/`height` IDL attribute (reflected `unsigned long`,
/// HTML §4.12.5). Coerces via `f64`→`u32` directly (NOT through `f32`, which
/// loses integer precision above 2²⁴ and would set a wrong bitmap dimension)
/// and routes through the `set_attribute` chokepoint so the bitmap reset fires
/// uniformly via `CanvasReconciler` (the `AttributeChange` SoT), not here.
fn set_canvas_dim_attr(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    name: &str,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_canvas_element(ctx, this, name)?;
    let v = args.first().copied().unwrap_or(JsValue::Undefined);
    let value = coerce::to_number(ctx.vm, v)?;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let n = if value.is_finite() && value >= 0.0 {
        value.min(f64::from(u32::MAX)) as u32
    } else {
        0
    };
    ctx.host().dom().set_attribute(entity, name, &n.to_string());
    Ok(JsValue::Undefined)
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
