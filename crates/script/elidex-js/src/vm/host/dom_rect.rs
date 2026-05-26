//! `DOMRectReadOnly` + `DOMRect` constructors / prototypes / side-table
//! state (W3C Geometry Interfaces Module Level 1 §3 "The DOMRect
//! interfaces").
//!
//! ## Shape
//!
//! ```text
//! DOMRect instance (ObjectKind::Ordinary, no own data props)
//!   → DOMRect.prototype             (x/y/width/height read-write accessors)
//!     → DOMRectReadOnly.prototype   (x/y/width/height + top/right/bottom/left
//!                                      read-only accessors, toJSON)
//!       → Object.prototype
//! ```
//!
//! Both interfaces are pure value-type objects with **no associated DOM
//! entity** — their `{x, y, width, height}` (plus a `mutable` brand bit)
//! live in the per-`ObjectId` side table [`VmInner::dom_rect_states`],
//! exactly like [`super::dom_exception::DomExceptionState`].  Because the
//! state is not per-entity, the side-store→component rule does not apply
//! (there is no entity on which to place a component).
//!
//! ## Consumers
//!
//! Script-side construction goes through the `DOMRectReadOnly` / `DOMRect`
//! constructors and `fromRect`.  The Rust-facing marshalling seam
//! [`VmInner::build_dom_rect_readonly`] mints a fresh `DOMRectReadOnly`
//! from four coordinates over [`VmInner::alloc_dom_rect`]; it is the
//! single chokepoint used by the D-22 observer pair (`contentRect` /
//! `boundingClientRect` / `intersectionRect` / `rootBounds`) and the
//! later `getBoundingClientRect` / `getClientRects` consumers, so every
//! Rust-side rect allocation goes through the same `alloc_dom_rect` →
//! side-table-insert path.

#![cfg(feature = "engine")]

use super::super::shape::{self, PropertyAttrs};
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, StringId, VmError,
};
use super::super::{NativeFn, VmInner};

// ---------------------------------------------------------------------------
// Side-table state
// ---------------------------------------------------------------------------

/// Per-`DOMRectReadOnly` / `DOMRect` out-of-band state, keyed by the
/// instance's own `ObjectId` on [`VmInner::dom_rect_states`].
///
/// `mutable` is the DOMRect-vs-DOMRectReadOnly brand: the `x`/`y`/
/// `width`/`height` setters (installed only on `DOMRect.prototype`)
/// require `mutable == true`, so cross-calling a DOMRect setter against a
/// DOMRectReadOnly receiver throws.  All fields are `Copy`, so GC needs
/// no trace pass — the sweep tail prunes collected keys.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DomRectState {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    mutable: bool,
}

/// Computed edges (W3C Geometry §3): NaN-safe min/max of a coordinate and
/// coordinate-plus-dimension, so a negative width/height swaps the edge
/// order.  `f64::min`/`max` already yield the spec's NaN-safe result.
impl DomRectState {
    fn left(&self) -> f64 {
        self.x.min(self.x + self.width)
    }
    fn right(&self) -> f64 {
        self.x.max(self.x + self.width)
    }
    fn top(&self) -> f64 {
        self.y.min(self.y + self.height)
    }
    fn bottom(&self) -> f64 {
        self.y.max(self.y + self.height)
    }
}

/// The four read-write coordinates a `DOMRect` setter can target.
#[derive(Clone, Copy)]
enum RectCoord {
    X,
    Y,
    Width,
    Height,
}

impl RectCoord {
    fn name(self) -> &'static str {
        match self {
            RectCoord::X => "x",
            RectCoord::Y => "y",
            RectCoord::Width => "width",
            RectCoord::Height => "height",
        }
    }
}

// ---------------------------------------------------------------------------
// Brand checks
// ---------------------------------------------------------------------------

/// Resolve `this` to its `DomRectState` (WebIDL §3.2 brand check).
/// Any non-rect receiver throws `TypeError` ("Illegal invocation").
fn require_dom_rect(
    ctx: &NativeContext<'_>,
    this: JsValue,
    attr: &str,
) -> Result<DomRectState, VmError> {
    match this {
        JsValue::Object(id) => ctx
            .vm
            .dom_rect_states
            .get(&id)
            .copied()
            .ok_or_else(|| read_wrong_brand(attr)),
        _ => Err(read_wrong_brand(attr)),
    }
}

fn read_wrong_brand(attr: &str) -> VmError {
    VmError::type_error(format!(
        "Failed to read '{attr}' on 'DOMRectReadOnly': Illegal invocation"
    ))
}

fn set_wrong_brand(attr: &str) -> VmError {
    VmError::type_error(format!(
        "Failed to set '{attr}' on 'DOMRect': Illegal invocation"
    ))
}

// ---------------------------------------------------------------------------
// Getters — stored coordinates
// ---------------------------------------------------------------------------

fn native_dom_rect_get_x(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Number(require_dom_rect(ctx, this, "x")?.x))
}
fn native_dom_rect_get_y(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Number(require_dom_rect(ctx, this, "y")?.y))
}
fn native_dom_rect_get_width(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Number(require_dom_rect(ctx, this, "width")?.width))
}
fn native_dom_rect_get_height(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Number(
        require_dom_rect(ctx, this, "height")?.height,
    ))
}

// ---------------------------------------------------------------------------
// Getters — computed edges (NaN-safe min/max, Geometry §3)
// ---------------------------------------------------------------------------

fn native_dom_rect_get_top(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Number(require_dom_rect(ctx, this, "top")?.top()))
}
fn native_dom_rect_get_right(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Number(
        require_dom_rect(ctx, this, "right")?.right(),
    ))
}
fn native_dom_rect_get_bottom(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Number(
        require_dom_rect(ctx, this, "bottom")?.bottom(),
    ))
}
fn native_dom_rect_get_left(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Number(require_dom_rect(ctx, this, "left")?.left()))
}

// ---------------------------------------------------------------------------
// Setters (DOMRect only)
// ---------------------------------------------------------------------------

fn set_rect_coord(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
    coord: RectCoord,
) -> Result<JsValue, VmError> {
    // Brand check before value conversion (WebIDL attribute-setter order):
    // receiver must be a *mutable* rect, else throw.
    let JsValue::Object(id) = this else {
        return Err(set_wrong_brand(coord.name()));
    };
    match ctx.vm.dom_rect_states.get(&id) {
        Some(state) if state.mutable => {}
        _ => return Err(set_wrong_brand(coord.name())),
    }
    // Setter parameter is a (non-optional) `unrestricted double`, so it
    // goes through ES `ToNumber` — `r.x = undefined` yields NaN, not 0
    // (unlike the constructor / `DOMRectInit` members, which are
    // optional-with-default-0).  A missing arg (`.set.call(obj)`)
    // behaves as `undefined`.
    let value = super::super::coerce::to_number(
        ctx.vm,
        args.first().copied().unwrap_or(JsValue::Undefined),
    )?;
    // `to_number` may run user `valueOf`; re-fetch in case the entry was
    // dropped mid-conversion (graceful no-op rather than panic).
    if let Some(state) = ctx.vm.dom_rect_states.get_mut(&id) {
        match coord {
            RectCoord::X => state.x = value,
            RectCoord::Y => state.y = value,
            RectCoord::Width => state.width = value,
            RectCoord::Height => state.height = value,
        }
    }
    Ok(JsValue::Undefined)
}

fn native_dom_rect_set_x(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    set_rect_coord(ctx, this, args, RectCoord::X)
}
fn native_dom_rect_set_y(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    set_rect_coord(ctx, this, args, RectCoord::Y)
}
fn native_dom_rect_set_width(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    set_rect_coord(ctx, this, args, RectCoord::Width)
}
fn native_dom_rect_set_height(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    set_rect_coord(ctx, this, args, RectCoord::Height)
}

// ---------------------------------------------------------------------------
// toJSON (Geometry §3)
// ---------------------------------------------------------------------------

fn native_dom_rect_to_json(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let state = require_dom_rect(ctx, this, "toJSON")?;
    let proto = ctx.vm.object_prototype;
    let obj = ctx.vm.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });
    for (name, value) in [
        ("x", state.x),
        ("y", state.y),
        ("width", state.width),
        ("height", state.height),
        ("top", state.top()),
        ("right", state.right()),
        ("bottom", state.bottom()),
        ("left", state.left()),
    ] {
        let key = PropertyKey::String(ctx.vm.strings.intern(name));
        ctx.vm.define_shaped_property(
            obj,
            key,
            PropertyValue::Data(JsValue::Number(value)),
            PropertyAttrs::DATA,
        );
    }
    Ok(JsValue::Object(obj))
}

// ---------------------------------------------------------------------------
// Argument / dictionary coercion
// ---------------------------------------------------------------------------

/// `unrestricted double` positional argument with default `0` (WebIDL
/// §3.10.5 ToNumber — NaN / ±Infinity pass through, no TypeError).
fn coerce_coord_arg(
    ctx: &mut NativeContext<'_>,
    args: &[JsValue],
    index: usize,
) -> Result<f64, VmError> {
    match args.get(index).copied() {
        Some(JsValue::Undefined) | None => Ok(0.0),
        Some(v) => super::super::coerce::to_number(ctx.vm, v),
    }
}

/// Read a `DOMRectInit` member (default `0` when absent / `undefined`).
fn read_init_field(
    ctx: &mut NativeContext<'_>,
    obj_id: super::super::value::ObjectId,
    name: &str,
) -> Result<f64, VmError> {
    let key = PropertyKey::String(ctx.vm.strings.intern(name));
    match ctx.get_property_value(obj_id, key)? {
        JsValue::Undefined => Ok(0.0),
        v => super::super::coerce::to_number(ctx.vm, v),
    }
}

/// Convert a `DOMRectInit` argument (WebIDL §3.10.7 dictionary
/// conversion; dictionary defined in Geometry §3): `undefined` / `null`
/// → all-zero; object → read members; anything else → TypeError.
fn read_rect_init(
    ctx: &mut NativeContext<'_>,
    init: JsValue,
    interface: &str,
) -> Result<(f64, f64, f64, f64), VmError> {
    match init {
        JsValue::Undefined | JsValue::Null => Ok((0.0, 0.0, 0.0, 0.0)),
        JsValue::Object(id) => Ok((
            read_init_field(ctx, id, "x")?,
            read_init_field(ctx, id, "y")?,
            read_init_field(ctx, id, "width")?,
            read_init_field(ctx, id, "height")?,
        )),
        _ => Err(VmError::type_error(format!(
            "Failed to execute 'fromRect' on '{interface}': The provided value is not of type 'DOMRectInit'."
        ))),
    }
}

// ---------------------------------------------------------------------------
// Constructors + fromRect
// ---------------------------------------------------------------------------

fn construct_rect(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
    mutable: bool,
) -> Result<JsValue, VmError> {
    let interface = if mutable {
        "DOMRect"
    } else {
        "DOMRectReadOnly"
    };
    if !ctx.is_construct() {
        return Err(VmError::type_error(format!(
            "Failed to construct '{interface}': Please use the 'new' operator"
        )));
    }
    let x = coerce_coord_arg(ctx, args, 0)?;
    let y = coerce_coord_arg(ctx, args, 1)?;
    let width = coerce_coord_arg(ctx, args, 2)?;
    let height = coerce_coord_arg(ctx, args, 3)?;
    let proto = if mutable {
        ctx.vm.dom_rect_prototype
    } else {
        ctx.vm.dom_rect_readonly_prototype
    };
    // Reuse the `new`-allocated receiver so a subclass `this` keeps its
    // prototype (mirrors the DOMException constructor).
    let receiver = ctx.vm.ensure_instance_or_alloc(this, proto, ctx.mode);
    let JsValue::Object(id) = receiver else {
        return Err(VmError::internal(
            "DOMRect constructor: receiver allocation did not yield an object",
        ));
    };
    ctx.vm.dom_rect_states.insert(
        id,
        DomRectState {
            x,
            y,
            width,
            height,
            mutable,
        },
    );
    Ok(JsValue::Object(id))
}

fn native_dom_rect_readonly_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    construct_rect(ctx, this, args, false)
}
fn native_dom_rect_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    construct_rect(ctx, this, args, true)
}

fn from_rect_impl(
    ctx: &mut NativeContext<'_>,
    args: &[JsValue],
    mutable: bool,
) -> Result<JsValue, VmError> {
    let interface = if mutable {
        "DOMRect"
    } else {
        "DOMRectReadOnly"
    };
    let (x, y, width, height) = read_rect_init(
        ctx,
        args.first().copied().unwrap_or(JsValue::Undefined),
        interface,
    )?;
    Ok(ctx.vm.alloc_dom_rect(DomRectState {
        x,
        y,
        width,
        height,
        mutable,
    }))
}

fn native_dom_rect_readonly_from_rect(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    from_rect_impl(ctx, args, false)
}
fn native_dom_rect_from_rect(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    from_rect_impl(ctx, args, true)
}

// ---------------------------------------------------------------------------
// Registration + marshalling seam
// ---------------------------------------------------------------------------

impl VmInner {
    /// Allocate a fresh `DOMRectReadOnly` / `DOMRect` instance and record
    /// its side-table state.  Chokepoint for non-constructor allocation
    /// (`fromRect` + [`Self::build_dom_rect_readonly`]).  The constructor
    /// path instead reuses its `new` receiver.
    fn alloc_dom_rect(&mut self, state: DomRectState) -> JsValue {
        let proto = if state.mutable {
            self.dom_rect_prototype
        } else {
            self.dom_rect_readonly_prototype
        };
        let id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: proto,
            extensible: true,
        });
        self.dom_rect_states.insert(id, state);
        JsValue::Object(id)
    }

    /// Mint a fresh read-only `DOMRectReadOnly` from four `f64` coordinates
    /// (W3C Geometry §3).  Single chokepoint for Rust-side rect marshalling
    /// — D-22 `ResizeObserverEntry.contentRect` /
    /// `IntersectionObserverEntry.{boundingClientRect, intersectionRect,
    /// rootBounds}`, later reused by `getBoundingClientRect` /
    /// `getClientRects` — so every host-built rect runs through the same
    /// [`Self::alloc_dom_rect`] → side-table insert path.  The constructor
    /// path (`new DOMRectReadOnly(...)`) reuses its own `new` receiver and
    /// does **not** route through this builder.
    pub(in crate::vm) fn build_dom_rect_readonly(
        &mut self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> JsValue {
        self.alloc_dom_rect(DomRectState {
            x,
            y,
            width,
            height,
            mutable: false,
        })
    }

    /// Install `DOMRectReadOnly` + `DOMRect` (prototypes + constructors +
    /// `fromRect` statics), W3C Geometry Interfaces Module Level 1 §3.
    ///
    /// Ordering: must run **after** `object_prototype` is populated
    /// (both prototypes chain to it).
    ///
    /// # Panics
    ///
    /// Panics if `object_prototype` is `None`.
    pub(in crate::vm) fn register_dom_rect_globals(&mut self) {
        let object_proto = self
            .object_prototype
            .expect("register_dom_rect_globals called before object_prototype");
        let sids = RectMemberSids {
            x: self.strings.intern("x"),
            y: self.strings.intern("y"),
            width: self.strings.intern("width"),
            height: self.strings.intern("height"),
            top: self.strings.intern("top"),
            right: self.strings.intern("right"),
            bottom: self.strings.intern("bottom"),
            left: self.strings.intern("left"),
            from_rect: self.strings.intern("fromRect"),
            to_json: self.well_known.to_json,
        };

        let readonly_proto = self.build_dom_rect_readonly_prototype(object_proto, &sids);
        self.dom_rect_readonly_prototype = Some(readonly_proto);
        let rect_proto = self.build_dom_rect_prototype(readonly_proto, &sids);
        self.dom_rect_prototype = Some(rect_proto);

        let readonly_ctor = self.wire_rect_constructor(
            "DOMRectReadOnly",
            native_dom_rect_readonly_constructor,
            readonly_proto,
            native_dom_rect_readonly_from_rect,
            sids.from_rect,
        );
        let rect_ctor = self.wire_rect_constructor(
            "DOMRect",
            native_dom_rect_constructor,
            rect_proto,
            native_dom_rect_from_rect,
            sids.from_rect,
        );
        // DOMRect : DOMRectReadOnly — chain the constructor functions so
        // `Object.getPrototypeOf(DOMRect) === DOMRectReadOnly` (WebIDL
        // interface inheritance).
        self.get_object_mut(rect_ctor).prototype = Some(readonly_ctor);
    }

    /// `DOMRectReadOnly.prototype`: getter-only `x`/`y`/`width`/`height` +
    /// computed `top`/`right`/`bottom`/`left` accessors + `toJSON`.
    fn build_dom_rect_readonly_prototype(
        &mut self,
        object_proto: ObjectId,
        sids: &RectMemberSids,
    ) -> ObjectId {
        let proto = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });
        let attrs = PropertyAttrs::WEBIDL_RO_ACCESSOR;
        for (sid, getter) in [
            (sids.x, native_dom_rect_get_x as NativeFn),
            (sids.y, native_dom_rect_get_y),
            (sids.width, native_dom_rect_get_width),
            (sids.height, native_dom_rect_get_height),
            (sids.top, native_dom_rect_get_top),
            (sids.right, native_dom_rect_get_right),
            (sids.bottom, native_dom_rect_get_bottom),
            (sids.left, native_dom_rect_get_left),
        ] {
            self.install_accessor_pair(proto, sid, getter, None, attrs);
        }
        let to_json_fn = self.create_native_function("toJSON", native_dom_rect_to_json);
        self.define_shaped_property(
            proto,
            PropertyKey::String(sids.to_json),
            PropertyValue::Data(JsValue::Object(to_json_fn)),
            PropertyAttrs::METHOD,
        );
        proto
    }

    /// `DOMRect.prototype` (chains to `DOMRectReadOnly.prototype`):
    /// re-declares `x`/`y`/`width`/`height` as read-write accessor pairs.
    fn build_dom_rect_prototype(
        &mut self,
        readonly_proto: ObjectId,
        sids: &RectMemberSids,
    ) -> ObjectId {
        let proto = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(readonly_proto),
            extensible: true,
        });
        let attrs = PropertyAttrs::WEBIDL_RO_ACCESSOR;
        for (sid, getter, setter) in [
            (
                sids.x,
                native_dom_rect_get_x as NativeFn,
                native_dom_rect_set_x as NativeFn,
            ),
            (sids.y, native_dom_rect_get_y, native_dom_rect_set_y),
            (
                sids.width,
                native_dom_rect_get_width,
                native_dom_rect_set_width,
            ),
            (
                sids.height,
                native_dom_rect_get_height,
                native_dom_rect_set_height,
            ),
        ] {
            self.install_accessor_pair(proto, sid, getter, Some(setter), attrs);
        }
        proto
    }

    /// Create a Geometry constructor: link its `.prototype` / the
    /// prototype's `.constructor`, install the `fromRect` static, and
    /// expose it as a global.  Returns the constructor `ObjectId`.
    fn wire_rect_constructor(
        &mut self,
        name: &str,
        ctor_fn: NativeFn,
        proto_id: ObjectId,
        from_rect_fn: NativeFn,
        from_rect_sid: StringId,
    ) -> ObjectId {
        let ctor = self.create_constructable_function(name, ctor_fn);
        self.define_shaped_property(
            ctor,
            PropertyKey::String(self.well_known.prototype),
            PropertyValue::Data(JsValue::Object(proto_id)),
            PropertyAttrs::BUILTIN,
        );
        self.define_shaped_property(
            proto_id,
            PropertyKey::String(self.well_known.constructor),
            PropertyValue::Data(JsValue::Object(ctor)),
            PropertyAttrs::METHOD,
        );
        let from_rect = self.create_native_function("fromRect", from_rect_fn);
        self.define_shaped_property(
            ctor,
            PropertyKey::String(from_rect_sid),
            PropertyValue::Data(JsValue::Object(from_rect)),
            PropertyAttrs::METHOD,
        );
        let name_sid = self.strings.intern(name);
        self.globals.insert(name_sid, JsValue::Object(ctor));
        ctor
    }
}

/// Pre-interned `StringId`s for the DOMRect member names, threaded through
/// the prototype builders.
struct RectMemberSids {
    x: StringId,
    y: StringId,
    width: StringId,
    height: StringId,
    top: StringId,
    right: StringId,
    bottom: StringId,
    left: StringId,
    from_rect: StringId,
    to_json: StringId,
}
