//! Specialized Event constructors for the UIEvent family.
//!
//! WebIDL IDL tree (UI Events §3 / §5 / §6 / §7 / §8):
//!
//! ```text
//! UIEvent : Event
//! MouseEvent : UIEvent
//! KeyboardEvent : UIEvent
//! FocusEvent : UIEvent
//! InputEvent : UIEvent
//! ```
//!
//! Each subclass's prototype chains through `UIEvent.prototype → Event.prototype`.
//! `view` and `detail` live as own-data slots on every instance (at
//! slot 9 / 10 of the `ui_event_constructed` shape and its descendants);
//! this makes `event.view` / `event.detail` resolve via the own-property
//! fast path instead of a prototype-accessor + side-table lookup.
//!
//! Init-dict coercion follows Chrome's invocation order verified via a
//! userland getter probe (matches the pattern established in
//! [`super::events::parse_event_init`]).  Numeric fields use
//! [`coerce::to_number`]; boolean modifier flags use
//! [`coerce::to_boolean`]; relatedTarget / view accept only nullish
//! values + Window (for `view`) or HostObject with a bound DOM entity
//! (for `relatedTarget`) — otherwise TypeError per WebIDL
//! `EventTarget?` / `Window?` interface coercion.
//!
//! ## Constructor gate
//!
//! Every ctor starts with `ctx.is_construct()` (WebIDL `[Constructor]`
//! §2.2) → call-mode `UIEvent('x')` throws TypeError matching all major
//! browsers.

#![cfg(feature = "engine")]

use super::super::shape::ShapeId;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, StringId, VmError,
};
use super::super::{NativeFn, VmInner};
use super::events::{check_construct, install_ctor, parse_event_init, type_arg, EventInit};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Resolve a UIEventInit `view` member (WebIDL `Window? view = null`).
///
/// Accepts:
/// - `undefined` / `null` / missing → JS `null`
/// - `globalThis` (same `ObjectId` as `vm.global_object`) → the global
///   Window HostObject
/// - Any other value → TypeError ("member view is not of type 'Window'")
///
/// PR4b landed `globalThis` as a Window HostObject wrapper (entity_bits
/// = window_entity once bound).  An unbound VM still has `global_object`
/// with entity_bits = 0; in that state only `null` / `undefined`
/// / `globalThis` are accepted — passing another HostObject would fail
/// the entity-bit match.
fn resolve_view(
    ctx: &NativeContext<'_>,
    val: JsValue,
    interface: &str,
) -> Result<JsValue, VmError> {
    match val {
        JsValue::Undefined | JsValue::Null => Ok(JsValue::Null),
        JsValue::Object(id) if id == ctx.vm.global_object => Ok(JsValue::Object(id)),
        _ => Err(VmError::type_error(format!(
            "Failed to construct '{interface}': \
             member view is not of type 'Window'."
        ))),
    }
}

/// Resolve a MouseEventInit / FocusEventInit `relatedTarget` member
/// (WebIDL `EventTarget? relatedTarget = null`).
///
/// Accepts:
/// - `undefined` / `null` / missing → JS `null`
/// - DOM wrapper (`ObjectKind::HostObject` with a bound ECS entity) →
///   pass through as-is
/// - [`ObjectKind::AbortSignal`] → pass through as-is (AbortSignal is
///   an EventTarget per WHATWG DOM §3.1 without being a Node)
/// - Any other Object / primitive → TypeError
///
/// The brand check rejects plain `{}`, arrays and primitives to
/// match real-browser `EventTarget?` coercion.  If future EventTarget
/// `ObjectKind` variants are introduced (e.g. `Worker`,
/// `BroadcastChannel`), they must be added to the accept list to
/// stay spec-compliant — add an exhaustive match or a `match_event_target`
/// helper at that point.
fn resolve_related_target(vm: &VmInner, val: JsValue, interface: &str) -> Result<JsValue, VmError> {
    match val {
        JsValue::Undefined | JsValue::Null => Ok(JsValue::Null),
        JsValue::Object(id) => {
            // WebIDL `EventTarget?` brand check — plain objects and
            // non-EventTarget `ObjectKind`s are rejected.
            match vm.get_object(id).kind {
                ObjectKind::HostObject { entity_bits }
                    if elidex_ecs::Entity::from_bits(entity_bits).is_some() =>
                {
                    Ok(val)
                }
                ObjectKind::AbortSignal => Ok(val),
                _ => Err(VmError::type_error(format!(
                    "Failed to construct '{interface}': \
                     member relatedTarget is not of type 'EventTarget'."
                ))),
            }
        }
        _ => Err(VmError::type_error(format!(
            "Failed to construct '{interface}': \
             member relatedTarget is not of type 'EventTarget'."
        ))),
    }
}

/// `UIEventInit` — base of the MouseEvent / KeyboardEvent / FocusEvent /
/// InputEvent init dictionaries.  Fields default to the WebIDL
/// "no value supplied" form: `view` null, `detail` zero, plus the
/// inherited [`EventInit`] booleans.
#[derive(Clone, Copy)]
struct UIEventInit {
    base: EventInit,
    view: JsValue,
    detail: f64,
}

/// Parse the `view` + `detail` members of a UIEventInit (WebIDL §3.2).
/// Caller is responsible for already having a resolved [`EventInit`]
/// (the bubbles / cancelable / composed triple).  Missing / nullish
/// values yield the spec defaults; getter side effects on the init
/// object are observable.
fn parse_ui_members(
    ctx: &mut NativeContext<'_>,
    opts_id: Option<ObjectId>,
    interface: &str,
) -> Result<(JsValue, f64), VmError> {
    let Some(opts_id) = opts_id else {
        return Ok((JsValue::Null, 0.0));
    };
    let view_val = ctx
        .vm
        .get_property_value(opts_id, PropertyKey::String(ctx.vm.well_known.view))?;
    let view = resolve_view(ctx, view_val, interface)?;
    let detail_val = ctx
        .vm
        .get_property_value(opts_id, PropertyKey::String(ctx.vm.well_known.detail))?;
    // `detail` is WebIDL `long` (UI Events §3.2) — ToInt32 per WebIDL
    // §3.10.9 (NaN / ±Infinity / ±0 → 0; otherwise truncate + mod 2^32
    // with two's-complement reinterpret).  Browsers surface 0 for
    // `new UIEvent('x', {detail: NaN}).detail`; preserving NaN was the
    // pre-R3.4 behaviour.
    let detail = if let JsValue::Undefined = detail_val {
        0.0
    } else {
        let n = super::super::coerce::to_number(ctx.vm, detail_val)?;
        f64::from(super::super::coerce::f64_to_int32(n))
    };
    Ok((view, detail))
}

fn opts_object_id(val: JsValue) -> Option<ObjectId> {
    match val {
        JsValue::Object(id) => Some(id),
        _ => None,
    }
}

fn parse_ui_event_init(
    ctx: &mut NativeContext<'_>,
    val: JsValue,
    interface: &str,
) -> Result<UIEventInit, VmError> {
    let base = parse_event_init(ctx, val, interface)?;
    let opts_id = opts_object_id(val);
    let (view, detail) = parse_ui_members(ctx, opts_id, interface)?;
    Ok(UIEventInit { base, view, detail })
}

// Modifier flag cluster (altKey / ctrlKey / metaKey / shiftKey) used by
// MouseEventInit and KeyboardEventInit — same four booleans in both.
fn parse_modifier_flags(
    ctx: &mut NativeContext<'_>,
    opts_id: Option<ObjectId>,
) -> Result<[bool; 4], VmError> {
    let Some(opts_id) = opts_id else {
        return Ok([false, false, false, false]);
    };
    let mut out = [false; 4];
    let keys = [
        ctx.vm.well_known.ctrl_key,
        ctx.vm.well_known.shift_key,
        ctx.vm.well_known.alt_key,
        ctx.vm.well_known.meta_key,
    ];
    for (slot, key) in out.iter_mut().zip(keys.iter()) {
        let v = ctx
            .vm
            .get_property_value(opts_id, PropertyKey::String(*key))?;
        *slot = super::super::coerce::to_boolean(ctx.vm, v);
    }
    Ok(out)
}

// Read a numeric init-dict member — `undefined` (missing) → default.
fn read_number(
    ctx: &mut NativeContext<'_>,
    opts_id: ObjectId,
    key: StringId,
    default: f64,
) -> Result<f64, VmError> {
    let v = ctx
        .vm
        .get_property_value(opts_id, PropertyKey::String(key))?;
    match v {
        JsValue::Undefined => Ok(default),
        _ => super::super::coerce::to_number(ctx.vm, v),
    }
}

// Read a boolean init-dict member — missing / undefined → `false`.
fn read_bool(
    ctx: &mut NativeContext<'_>,
    opts_id: ObjectId,
    key: StringId,
) -> Result<bool, VmError> {
    let v = ctx
        .vm
        .get_property_value(opts_id, PropertyKey::String(key))?;
    Ok(super::super::coerce::to_boolean(ctx.vm, v))
}

// Read a string init-dict member — missing / undefined → empty string
// (WebIDL DOMString default).  Non-strings coerce via ToString
// (Symbol throws, as with Event ctor).
fn read_string(
    ctx: &mut NativeContext<'_>,
    opts_id: ObjectId,
    key: StringId,
) -> Result<StringId, VmError> {
    let v = ctx
        .vm
        .get_property_value(opts_id, PropertyKey::String(key))?;
    match v {
        JsValue::Undefined => Ok(ctx.vm.strings.intern("")),
        _ => super::super::coerce::to_string(ctx.vm, v),
    }
}

// ToInt16 (WebIDL §3.10.4).  Modular conversion for out-of-range
// values; NaN/±Infinity/0 → 0.  Used by MouseEventInit.button.
fn to_int16(n: f64) -> i16 {
    if n.is_nan() || n.is_infinite() || n == 0.0 {
        return 0;
    }
    let int = n.trunc();
    let m = int.rem_euclid(65_536.0);
    if m >= 32_768.0 {
        (m - 65_536.0) as i16
    } else {
        m as i16
    }
}

// ---------------------------------------------------------------------------
// UIEvent ctor (UI Events §3.2)
// ---------------------------------------------------------------------------

impl VmInner {
    /// Install `UIEvent.prototype` chained to `Event.prototype` and
    /// publish the `UIEvent` global constructor.  `UIEvent.prototype`
    /// itself carries no own methods — every accessor for the instance
    /// members (`view`, `detail`) resolves against the shape's own-data
    /// slots (slot 9 / 10 on `ui_event_constructed`).  The prototype
    /// exists mainly as the chain anchor for MouseEvent / KeyboardEvent
    /// / FocusEvent / InputEvent.
    pub(in crate::vm) fn register_ui_event_global(&mut self) {
        let parent = self
            .event_prototype
            .expect("register_ui_event_global called before register_event_prototype");
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(super::super::shape::ROOT_SHAPE),
            prototype: Some(parent),
            extensible: true,
        });
        self.ui_event_prototype = Some(proto_id);

        install_ctor(
            self,
            proto_id,
            "UIEvent",
            native_ui_event_constructor,
            self.well_known.ui_event_global,
        );
    }

    pub(in crate::vm) fn register_mouse_event_global(&mut self) {
        register_descendant(
            self,
            self.ui_event_prototype,
            "MouseEvent",
            native_mouse_event_constructor,
            self.well_known.mouse_event_global,
            |vm, id| vm.mouse_event_prototype = Some(id),
        );
    }

    pub(in crate::vm) fn register_keyboard_event_global(&mut self) {
        register_descendant(
            self,
            self.ui_event_prototype,
            "KeyboardEvent",
            native_keyboard_event_constructor,
            self.well_known.keyboard_event_global,
            |vm, id| vm.keyboard_event_prototype = Some(id),
        );
    }

    pub(in crate::vm) fn register_focus_event_global(&mut self) {
        register_descendant(
            self,
            self.ui_event_prototype,
            "FocusEvent",
            native_focus_event_constructor,
            self.well_known.focus_event_global,
            |vm, id| vm.focus_event_prototype = Some(id),
        );
    }

    pub(in crate::vm) fn register_input_event_global(&mut self) {
        register_descendant(
            self,
            self.ui_event_prototype,
            "InputEvent",
            native_input_event_constructor,
            self.well_known.input_event_global,
            |vm, id| vm.input_event_prototype = Some(id),
        );
    }

    /// Shared post-construction path for every UIEvent-family ctor.
    /// Assembles the UIEvent-prefix slots (`view`, `detail`) on top of
    /// the core-9 values [`Self::create_fresh_event_object`] writes,
    /// then appends variant-specific slots supplied by the caller.
    ///
    /// `proto` is the target prototype (MouseEvent.prototype / …).  The
    /// base [`Self::create_fresh_event_object`] installs
    /// `Event.prototype` on the allocated receiver; we swap it to the
    /// descendant prototype afterwards so the chain ends
    /// `instance → <Descendant>.prototype → UIEvent.prototype →
    /// Event.prototype`.
    fn build_ui_event_instance(
        &mut self,
        this: JsValue,
        type_sid: StringId,
        init: UIEventInit,
        shape_id: ShapeId,
        _descendant_proto: ObjectId,
        variant_slots: Vec<PropertyValue>,
    ) -> ObjectId {
        // `view` / `detail` precede the descendant's own slots (matches
        // the shape chain `core → +view → +detail → +<variant keys>`).
        let mut payload = Vec::with_capacity(2 + variant_slots.len());
        payload.push(PropertyValue::Data(init.view));
        payload.push(PropertyValue::Data(JsValue::Number(init.detail)));
        payload.extend(variant_slots);
        // `create_fresh_event_object`'s `ensure_instance_or_alloc`
        // already preserves the receiver's prototype in construct-mode
        // (i.e. whatever `do_new` set based on `new.target.prototype`).
        // For `new MouseEvent()` that's `MouseEvent.prototype`; for
        // `class Sub extends MouseEvent; new Sub()` it's
        // `Sub.prototype`.  A blanket overwrite to `descendant_proto`
        // here would correctly handle the former (no-op) but silently
        // break the subclass chain in the latter, so we do NOT touch
        // the prototype — `_descendant_proto` is retained in the
        // signature as a load-bearing registration-check
        // (`.expect()` at the call site catches a missed prototype
        // install) but the value itself is no longer applied.
        self.create_fresh_event_object(this, type_sid, init.base, shape_id, payload, false)
    }
}

fn register_descendant(
    vm: &mut VmInner,
    parent: Option<ObjectId>,
    name: &str,
    func: NativeFn,
    global_sid: StringId,
    store: impl FnOnce(&mut VmInner, ObjectId),
) {
    let parent = parent.expect("UIEvent.prototype must be registered first");
    let proto_id = vm.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        storage: PropertyStorage::shaped(super::super::shape::ROOT_SHAPE),
        prototype: Some(parent),
        extensible: true,
    });
    store(vm, proto_id);
    install_ctor(vm, proto_id, name, func, global_sid);
}

// ---------------------------------------------------------------------------
// Native constructor fns
// ---------------------------------------------------------------------------

fn native_ui_event_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    check_construct(ctx, "UIEvent")?;
    let type_sid = type_arg(ctx, args, "UIEvent")?;
    let init_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let init = parse_ui_event_init(ctx, init_arg, "UIEvent")?;
    let shape_id = ctx
        .vm
        .precomputed_event_shapes
        .as_ref()
        .expect("precomputed_event_shapes missing")
        .ui_event_constructed;
    // `UIEvent` instance targets `UIEvent.prototype` (no descendant).
    let ui_proto = ctx
        .vm
        .ui_event_prototype
        .expect("UIEvent.prototype must be registered before native_ui_event_constructor");
    let id = ctx
        .vm
        .build_ui_event_instance(this, type_sid, init, shape_id, ui_proto, Vec::new());
    Ok(JsValue::Object(id))
}

// ---------------------------------------------------------------------------
// MouseEvent (UI Events §5.2)
// ---------------------------------------------------------------------------

#[allow(clippy::similar_names)] // button/buttons + movement_x/y_raw are spec field names
fn native_mouse_event_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    check_construct(ctx, "MouseEvent")?;
    let type_sid = type_arg(ctx, args, "MouseEvent")?;
    let init_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let ui = parse_ui_event_init(ctx, init_arg, "MouseEvent")?;
    let opts_id = opts_object_id(init_arg);
    let modifiers = parse_modifier_flags(ctx, opts_id)?;
    let (
        screen_x,
        screen_y,
        client_x,
        client_y,
        button,
        buttons,
        related_target,
        movement_x,
        movement_y,
    ) = if let Some(opts_id) = opts_id {
        // StringId is Copy (u32) — extract up front so subsequent
        // `read_number(ctx, ...)` calls don't conflict with a live
        // borrow of `ctx.vm.well_known`.
        let k_screen_x = ctx.vm.well_known.screen_x;
        let k_screen_y = ctx.vm.well_known.screen_y;
        let k_client_x = ctx.vm.well_known.client_x;
        let k_client_y = ctx.vm.well_known.client_y;
        let k_button = ctx.vm.well_known.button;
        let k_buttons = ctx.vm.well_known.buttons;
        let k_related = ctx.vm.well_known.related_target;
        let k_movement_x = ctx.vm.well_known.movement_x;
        let k_movement_y = ctx.vm.well_known.movement_y;
        let screen_x = read_number(ctx, opts_id, k_screen_x, 0.0)?;
        let screen_y = read_number(ctx, opts_id, k_screen_y, 0.0)?;
        let client_x = read_number(ctx, opts_id, k_client_x, 0.0)?;
        let client_y = read_number(ctx, opts_id, k_client_y, 0.0)?;
        // `button` is `short` (WebIDL §3.10.4 ToInt16); default 0.
        let button_num = read_number(ctx, opts_id, k_button, 0.0)?;
        let button = f64::from(to_int16(button_num));
        // `buttons` is `unsigned short` (WebIDL §3.10.5 ToUint16); default 0.
        let buttons_num = read_number(ctx, opts_id, k_buttons, 0.0)?;
        let buttons = f64::from(super::super::coerce::f64_to_uint16(buttons_num));
        let related_raw = ctx
            .vm
            .get_property_value(opts_id, PropertyKey::String(k_related))?;
        let related_target = resolve_related_target(ctx.vm, related_raw, "MouseEvent")?;
        // `movementX` / `movementY` are WebIDL `long` (UI Events §5.1)
        // — same ToInt32 treatment as `UIEvent.detail`.  `screenX`,
        // `screenY`, `clientX`, `clientY` remain `double` per the
        // MouseEvent IDL so ToNumber is correct for them.
        let movement_x_raw = read_number(ctx, opts_id, k_movement_x, 0.0)?;
        let movement_y_raw = read_number(ctx, opts_id, k_movement_y, 0.0)?;
        let movement_x = f64::from(super::super::coerce::f64_to_int32(movement_x_raw));
        let movement_y = f64::from(super::super::coerce::f64_to_int32(movement_y_raw));
        (
            screen_x,
            screen_y,
            client_x,
            client_y,
            button,
            buttons,
            related_target,
            movement_x,
            movement_y,
        )
    } else {
        (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, JsValue::Null, 0.0, 0.0)
    };
    let shape_id = ctx
        .vm
        .precomputed_event_shapes
        .as_ref()
        .expect("precomputed_event_shapes missing")
        .mouse_event_constructed;
    // Slot order must match the transition chain built by
    // `build_precomputed_event_shapes::mouse_event_constructed`:
    // screenX, screenY, clientX, clientY, ctrlKey, shiftKey, altKey,
    // metaKey, button, buttons, relatedTarget, movementX, movementY.
    let [ctrl, shift, alt, meta] = modifiers;
    let slots = vec![
        PropertyValue::Data(JsValue::Number(screen_x)),
        PropertyValue::Data(JsValue::Number(screen_y)),
        PropertyValue::Data(JsValue::Number(client_x)),
        PropertyValue::Data(JsValue::Number(client_y)),
        PropertyValue::Data(JsValue::Boolean(ctrl)),
        PropertyValue::Data(JsValue::Boolean(shift)),
        PropertyValue::Data(JsValue::Boolean(alt)),
        PropertyValue::Data(JsValue::Boolean(meta)),
        PropertyValue::Data(JsValue::Number(button)),
        PropertyValue::Data(JsValue::Number(buttons)),
        PropertyValue::Data(related_target),
        PropertyValue::Data(JsValue::Number(movement_x)),
        PropertyValue::Data(JsValue::Number(movement_y)),
    ];
    // Root the related_target across allocation inside
    // `build_ui_event_instance` (it may allocate upon shape transition).
    // For an ObjectId relatedTarget, GC-rooting avoids the wrapper
    // being swept before it lands in the slot.
    let mut g = ctx.vm.push_temp_root(related_target);
    let mouse_proto = g
        .mouse_event_prototype
        .expect("MouseEvent.prototype must be registered before native_mouse_event_constructor");
    let id = g.build_ui_event_instance(this, type_sid, ui, shape_id, mouse_proto, slots);
    drop(g);
    Ok(JsValue::Object(id))
}

// ---------------------------------------------------------------------------
// KeyboardEvent (UI Events §7.2)
// ---------------------------------------------------------------------------

fn native_keyboard_event_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    check_construct(ctx, "KeyboardEvent")?;
    let type_sid = type_arg(ctx, args, "KeyboardEvent")?;
    let init_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let ui = parse_ui_event_init(ctx, init_arg, "KeyboardEvent")?;
    let opts_id = opts_object_id(init_arg);
    let modifiers = parse_modifier_flags(ctx, opts_id)?;
    let (key_sid, code_sid, location_num, repeat, is_composing) = if let Some(opts_id) = opts_id {
        let k_key = ctx.vm.well_known.key;
        let k_code = ctx.vm.well_known.code;
        let k_location = ctx.vm.well_known.location;
        let k_repeat = ctx.vm.well_known.repeat;
        let k_is_composing = ctx.vm.well_known.is_composing;
        let key_sid = read_string(ctx, opts_id, k_key)?;
        let code_sid = read_string(ctx, opts_id, k_code)?;
        // `location` is `unsigned long` (WebIDL ToUint32); default 0.
        // Slot stores as f64 (all Number values are f64 in VM).
        let loc_num = read_number(ctx, opts_id, k_location, 0.0)?;
        let location_num = f64::from(super::super::coerce::f64_to_uint32(loc_num));
        let repeat = read_bool(ctx, opts_id, k_repeat)?;
        let is_composing = read_bool(ctx, opts_id, k_is_composing)?;
        (key_sid, code_sid, location_num, repeat, is_composing)
    } else {
        let empty = ctx.vm.strings.intern("");
        (empty, empty, 0.0, false, false)
    };
    let shape_id = ctx
        .vm
        .precomputed_event_shapes
        .as_ref()
        .expect("precomputed_event_shapes missing")
        .keyboard_event_constructed;
    // Slot order: key, code, location, ctrlKey, shiftKey, altKey,
    // metaKey, repeat, isComposing.
    let [ctrl, shift, alt, meta] = modifiers;
    let slots = vec![
        PropertyValue::Data(JsValue::String(key_sid)),
        PropertyValue::Data(JsValue::String(code_sid)),
        PropertyValue::Data(JsValue::Number(location_num)),
        PropertyValue::Data(JsValue::Boolean(ctrl)),
        PropertyValue::Data(JsValue::Boolean(shift)),
        PropertyValue::Data(JsValue::Boolean(alt)),
        PropertyValue::Data(JsValue::Boolean(meta)),
        PropertyValue::Data(JsValue::Boolean(repeat)),
        PropertyValue::Data(JsValue::Boolean(is_composing)),
    ];
    let kb_proto = ctx.vm.keyboard_event_prototype.expect(
        "KeyboardEvent.prototype must be registered before native_keyboard_event_constructor",
    );
    let id = ctx
        .vm
        .build_ui_event_instance(this, type_sid, ui, shape_id, kb_proto, slots);
    Ok(JsValue::Object(id))
}

// ---------------------------------------------------------------------------
// FocusEvent (UI Events §6.2)
// ---------------------------------------------------------------------------

fn native_focus_event_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    check_construct(ctx, "FocusEvent")?;
    let type_sid = type_arg(ctx, args, "FocusEvent")?;
    let init_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let ui = parse_ui_event_init(ctx, init_arg, "FocusEvent")?;
    let opts_id = opts_object_id(init_arg);
    let related_target = if let Some(opts_id) = opts_id {
        let raw = ctx.vm.get_property_value(
            opts_id,
            PropertyKey::String(ctx.vm.well_known.related_target),
        )?;
        resolve_related_target(ctx.vm, raw, "FocusEvent")?
    } else {
        JsValue::Null
    };
    let shape_id = ctx
        .vm
        .precomputed_event_shapes
        .as_ref()
        .expect("precomputed_event_shapes missing")
        .focus_event_constructed;
    let slots = vec![PropertyValue::Data(related_target)];
    let mut g = ctx.vm.push_temp_root(related_target);
    let focus_proto = g
        .focus_event_prototype
        .expect("FocusEvent.prototype must be registered before native_focus_event_constructor");
    let id = g.build_ui_event_instance(this, type_sid, ui, shape_id, focus_proto, slots);
    drop(g);
    Ok(JsValue::Object(id))
}

// ---------------------------------------------------------------------------
// InputEvent (UI Events §8.2)
// ---------------------------------------------------------------------------

fn native_input_event_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    check_construct(ctx, "InputEvent")?;
    let type_sid = type_arg(ctx, args, "InputEvent")?;
    let init_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let ui = parse_ui_event_init(ctx, init_arg, "InputEvent")?;
    let opts_id = opts_object_id(init_arg);
    // InputEventInit: data is `DOMString? = null` (nullable string).
    // Missing / undefined → null; otherwise ToString (Symbol throws).
    let (data_val, is_composing, input_type_sid) = if let Some(opts_id) = opts_id {
        let k_data = ctx.vm.well_known.data;
        let k_is_composing = ctx.vm.well_known.is_composing;
        let k_input_type = ctx.vm.well_known.input_type;
        let data_raw = ctx
            .vm
            .get_property_value(opts_id, PropertyKey::String(k_data))?;
        let data_val = match data_raw {
            JsValue::Undefined | JsValue::Null => JsValue::Null,
            _ => {
                let sid = super::super::coerce::to_string(ctx.vm, data_raw)?;
                JsValue::String(sid)
            }
        };
        let is_composing = read_bool(ctx, opts_id, k_is_composing)?;
        let input_type_sid = read_string(ctx, opts_id, k_input_type)?;
        (data_val, is_composing, input_type_sid)
    } else {
        let empty = ctx.vm.strings.intern("");
        (JsValue::Null, false, empty)
    };
    let shape_id = ctx
        .vm
        .precomputed_event_shapes
        .as_ref()
        .expect("precomputed_event_shapes missing")
        .input_event_constructed;
    // Slot order: data, isComposing, inputType.
    let slots = vec![
        PropertyValue::Data(data_val),
        PropertyValue::Data(JsValue::Boolean(is_composing)),
        PropertyValue::Data(JsValue::String(input_type_sid)),
    ];
    let in_proto = ctx
        .vm
        .input_event_prototype
        .expect("InputEvent.prototype must be registered before native_input_event_constructor");
    let id = ctx
        .vm
        .build_ui_event_instance(this, type_sid, ui, shape_id, in_proto, slots);
    Ok(JsValue::Object(id))
}
