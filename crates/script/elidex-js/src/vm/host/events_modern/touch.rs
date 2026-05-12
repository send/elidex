//! `Touch` / `TouchList` / `TouchEvent` constructors + prototypes
//! (Touch Events §5).
//!
//! ## Inheritance
//!
//! ```text
//! TouchEvent : UIEvent
//! Touch      : (Object)
//! TouchList  : (Object — indexed exotic, no constructor)
//! ```
//!
//! ## State storage
//!
//! - `Touch` instances are wrapper-only; the 12 IDL members live in
//!   [`super::TouchState`] keyed by the wrapper's `ObjectId`.
//! - `TouchList` instances likewise carry their ordered Vec of
//!   member [`super::super::super::value::ObjectKind::Touch`] wrapper
//!   IDs in [`super::TouchListState`].
//! - `TouchEvent` instances are conventional shape-resident: the 3
//!   `TouchList` references + 4 modifier flags occupy the 7 own-data
//!   slots of the `touch_event_constructed` shape.

#![cfg(feature = "engine")]

use super::super::super::shape;
use super::super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, VmError,
};
use super::super::super::VmInner;
use super::super::events::{check_construct, type_arg};
use super::super::events_ui::{opts_object_id, parse_ui_event_init, register_descendant};
use super::{TouchListState, TouchState};

// ---------------------------------------------------------------------------
// Registration glue
// ---------------------------------------------------------------------------

pub(in crate::vm) fn register_touch_global(vm: &mut VmInner) {
    // `Touch` is NOT an Event — its prototype chains to
    // `Object.prototype`, NOT UIEvent.prototype.  We allocate the
    // prototype with `Object.prototype` as parent and install a
    // freestanding constructor via `install_ctor`.
    let parent = vm
        .object_prototype
        .expect("register_touch_global requires object_prototype");
    let proto_id = vm.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: Some(parent),
        extensible: true,
    });
    vm.touch_prototype = Some(proto_id);
    super::super::events::install_ctor(
        vm,
        proto_id,
        "Touch",
        native_touch_constructor,
        vm.well_known.touch_global,
    );
    install_touch_accessors(vm, proto_id);
}

pub(in crate::vm) fn register_touch_list_global(vm: &mut VmInner) {
    // `TouchList` has no public ctor — but we still need the
    // prototype + global identifier (instanceof + brand checks).
    // Use `install_ctor` with a throw-on-construct native.
    let parent = vm
        .object_prototype
        .expect("register_touch_list_global requires object_prototype");
    let proto_id = vm.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: Some(parent),
        extensible: true,
    });
    vm.touch_list_prototype = Some(proto_id);
    super::super::events::install_ctor(
        vm,
        proto_id,
        "TouchList",
        native_touch_list_illegal_constructor,
        vm.well_known.touch_list_global,
    );
    // `length` accessor + `item(index)` method.
    let length_sid = vm.well_known.length;
    let item_sid = vm.well_known.item;
    vm.install_accessor_pair(
        proto_id,
        length_sid,
        native_touch_list_get_length,
        None,
        shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
    );
    vm.install_native_method(
        proto_id,
        item_sid,
        native_touch_list_item,
        shape::PropertyAttrs::METHOD,
    );
}

pub(in crate::vm) fn register_touch_event_global(vm: &mut VmInner) {
    register_descendant(
        vm,
        vm.ui_event_prototype,
        "TouchEvent",
        native_touch_event_constructor,
        vm.well_known.touch_event_global,
        |vm, id| vm.touch_event_prototype = Some(id),
    );
}

// ---------------------------------------------------------------------------
// Touch prototype accessors (12 readonly)
// ---------------------------------------------------------------------------

fn install_touch_accessors(vm: &mut VmInner, proto_id: ObjectId) {
    let entries: &[(
        super::super::super::value::StringId,
        super::super::super::NativeFn,
    )] = &[
        (vm.well_known.identifier, native_touch_get_identifier),
        (vm.well_known.target, native_touch_get_target),
        (vm.well_known.client_x, native_touch_get_client_x),
        (vm.well_known.client_y, native_touch_get_client_y),
        (vm.well_known.screen_x, native_touch_get_screen_x),
        (vm.well_known.screen_y, native_touch_get_screen_y),
        (vm.well_known.page_x, native_touch_get_page_x),
        (vm.well_known.page_y, native_touch_get_page_y),
        (vm.well_known.radius_x, native_touch_get_radius_x),
        (vm.well_known.radius_y, native_touch_get_radius_y),
        (
            vm.well_known.rotation_angle,
            native_touch_get_rotation_angle,
        ),
        (vm.well_known.force, native_touch_get_force),
    ];
    for (sid, getter) in entries {
        vm.install_accessor_pair(
            proto_id,
            *sid,
            *getter,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
    }
}

fn require_touch_receiver(
    ctx: &NativeContext<'_>,
    this: JsValue,
    member: &str,
) -> Result<ObjectId, VmError> {
    match this {
        JsValue::Object(id) if matches!(ctx.vm.get_object(id).kind, ObjectKind::Touch) => Ok(id),
        _ => Err(VmError::type_error(format!(
            "Failed to read the '{member}' property from 'Touch': \
             Illegal invocation."
        ))),
    }
}

fn touch_state(vm: &VmInner, id: ObjectId) -> &TouchState {
    vm.touch_states
        .get(&id)
        .expect("Touch state must exist for branded Touch instance")
}

fn native_touch_get_identifier(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_touch_receiver(ctx, this, "identifier")?;
    Ok(JsValue::Number(f64::from(
        touch_state(ctx.vm, id).identifier,
    )))
}

fn native_touch_get_target(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_touch_receiver(ctx, this, "target")?;
    Ok(match touch_state(ctx.vm, id).target {
        Some(tid) => JsValue::Object(tid),
        None => JsValue::Null,
    })
}

macro_rules! impl_touch_number_getter {
    ($name:ident, $field:ident, $member:literal) => {
        fn $name(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            _args: &[JsValue],
        ) -> Result<JsValue, VmError> {
            let id = require_touch_receiver(ctx, this, $member)?;
            Ok(JsValue::Number(touch_state(ctx.vm, id).$field))
        }
    };
}

impl_touch_number_getter!(native_touch_get_client_x, client_x, "clientX");
impl_touch_number_getter!(native_touch_get_client_y, client_y, "clientY");
impl_touch_number_getter!(native_touch_get_screen_x, screen_x, "screenX");
impl_touch_number_getter!(native_touch_get_screen_y, screen_y, "screenY");
impl_touch_number_getter!(native_touch_get_page_x, page_x, "pageX");
impl_touch_number_getter!(native_touch_get_page_y, page_y, "pageY");
impl_touch_number_getter!(native_touch_get_radius_x, radius_x, "radiusX");
impl_touch_number_getter!(native_touch_get_radius_y, radius_y, "radiusY");
impl_touch_number_getter!(
    native_touch_get_rotation_angle,
    rotation_angle,
    "rotationAngle"
);
impl_touch_number_getter!(native_touch_get_force, force, "force");

// ---------------------------------------------------------------------------
// Touch constructor
// ---------------------------------------------------------------------------

fn native_touch_constructor(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    check_construct(ctx, "Touch")?;
    // `TouchInit` is a `required` dictionary — missing or null/
    // undefined triggers a TypeError per WebIDL §3.10.18.
    let init_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let JsValue::Object(opts_id) = init_arg else {
        return Err(VmError::type_error(
            "Failed to construct 'Touch': \
             1 argument required, but only 0 present.",
        ));
    };

    let k_identifier = ctx.vm.well_known.identifier;
    let k_target = ctx.vm.well_known.target;
    let k_client_x = ctx.vm.well_known.client_x;
    let k_client_y = ctx.vm.well_known.client_y;
    let k_screen_x = ctx.vm.well_known.screen_x;
    let k_screen_y = ctx.vm.well_known.screen_y;
    let k_page_x = ctx.vm.well_known.page_x;
    let k_page_y = ctx.vm.well_known.page_y;
    let k_radius_x = ctx.vm.well_known.radius_x;
    let k_radius_y = ctx.vm.well_known.radius_y;
    let k_rotation_angle = ctx.vm.well_known.rotation_angle;
    let k_force = ctx.vm.well_known.force;

    // `required long identifier` — missing throws.  Match
    // FormDataEvent.formData precedent for the required-check
    // wording.
    let identifier_raw = ctx
        .vm
        .get_property_value(opts_id, PropertyKey::String(k_identifier))?;
    if matches!(identifier_raw, JsValue::Undefined) {
        return Err(VmError::type_error(
            "Failed to construct 'Touch': \
             required member identifier is undefined.",
        ));
    }
    let identifier_num = super::super::super::coerce::to_number(ctx.vm, identifier_raw)?;
    let identifier = super::super::super::coerce::f64_to_int32(identifier_num);

    // `required EventTarget target` — accept any EventTarget brand
    // (HostObject with a bound entity / AbortSignal / etc.).
    let target_raw = ctx
        .vm
        .get_property_value(opts_id, PropertyKey::String(k_target))?;
    if matches!(target_raw, JsValue::Undefined) {
        return Err(VmError::type_error(
            "Failed to construct 'Touch': \
             required member target is undefined.",
        ));
    }
    let target = coerce_event_target_required(ctx.vm, target_raw, "Touch", "target")?;

    let client_x = read_optional_number(ctx, opts_id, k_client_x, 0.0)?;
    let client_y = read_optional_number(ctx, opts_id, k_client_y, 0.0)?;
    let screen_x = read_optional_number(ctx, opts_id, k_screen_x, 0.0)?;
    let screen_y = read_optional_number(ctx, opts_id, k_screen_y, 0.0)?;
    let page_x = read_optional_number(ctx, opts_id, k_page_x, 0.0)?;
    let page_y = read_optional_number(ctx, opts_id, k_page_y, 0.0)?;
    let radius_x = read_optional_number(ctx, opts_id, k_radius_x, 0.0)?;
    let radius_y = read_optional_number(ctx, opts_id, k_radius_y, 0.0)?;
    let rotation_angle = read_optional_number(ctx, opts_id, k_rotation_angle, 0.0)?;
    let force = read_optional_number(ctx, opts_id, k_force, 0.0)?;

    let target_id = match target {
        JsValue::Object(id) => Some(id),
        _ => None,
    };
    let proto = ctx
        .vm
        .touch_prototype
        .expect("Touch.prototype must be registered before ctor");
    let touch_id = ctx.vm.alloc_object(Object {
        kind: ObjectKind::Touch,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: Some(proto),
        extensible: true,
    });
    ctx.vm.touch_states.insert(
        touch_id,
        TouchState {
            identifier,
            target: target_id,
            client_x,
            client_y,
            screen_x,
            screen_y,
            page_x,
            page_y,
            radius_x,
            radius_y,
            rotation_angle,
            force,
        },
    );
    Ok(JsValue::Object(touch_id))
}

fn read_optional_number(
    ctx: &mut NativeContext<'_>,
    opts_id: ObjectId,
    key: super::super::super::value::StringId,
    default: f64,
) -> Result<f64, VmError> {
    let v = ctx
        .vm
        .get_property_value(opts_id, PropertyKey::String(key))?;
    match v {
        JsValue::Undefined => Ok(default),
        _ => super::super::super::coerce::to_number(ctx.vm, v),
    }
}

/// Brand check for `required EventTarget target` — accepts any
/// EventTarget shape (HostObject with bound entity / AbortSignal /
/// future EventTarget variants).  Mirrors `events_ui::resolve_
/// related_target` but for the required (non-nullable) case.
fn coerce_event_target_required(
    vm: &VmInner,
    val: JsValue,
    interface: &str,
    member: &str,
) -> Result<JsValue, VmError> {
    if let JsValue::Object(id) = val {
        match vm.get_object(id).kind {
            ObjectKind::HostObject { entity_bits }
                if elidex_ecs::Entity::from_bits(entity_bits).is_some() =>
            {
                return Ok(val);
            }
            ObjectKind::AbortSignal => return Ok(val),
            _ => {}
        }
    }
    Err(VmError::type_error(format!(
        "Failed to construct '{interface}': \
         member {member} is not of type 'EventTarget'."
    )))
}

// ---------------------------------------------------------------------------
// TouchList constructor (illegal) + accessors
// ---------------------------------------------------------------------------

fn native_touch_list_illegal_constructor(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Err(VmError::type_error("Illegal constructor"))
}

fn require_touch_list_receiver(
    ctx: &NativeContext<'_>,
    this: JsValue,
    member: &str,
) -> Result<ObjectId, VmError> {
    match this {
        JsValue::Object(id) if matches!(ctx.vm.get_object(id).kind, ObjectKind::TouchList) => {
            Ok(id)
        }
        _ => Err(VmError::type_error(format!(
            "Failed to read the '{member}' property from 'TouchList': \
             Illegal invocation."
        ))),
    }
}

fn native_touch_list_get_length(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_touch_list_receiver(ctx, this, "length")?;
    let len = ctx
        .vm
        .touch_list_states
        .get(&id)
        .map_or(0, |s| u32::try_from(s.items.len()).unwrap_or(u32::MAX));
    Ok(JsValue::Number(f64::from(len)))
}

fn native_touch_list_item(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = match this {
        JsValue::Object(id) if matches!(ctx.vm.get_object(id).kind, ObjectKind::TouchList) => id,
        _ => {
            return Err(VmError::type_error(
                "Failed to execute 'item' on 'TouchList': Illegal invocation.",
            ));
        }
    };
    let idx_raw = args.first().copied().unwrap_or(JsValue::Undefined);
    let idx_num = super::super::super::coerce::to_number(ctx.vm, idx_raw)?;
    let idx = super::super::super::coerce::f64_to_uint32(idx_num);
    Ok(ctx
        .vm
        .touch_list_states
        .get(&id)
        .and_then(|s| s.items.get(idx as usize).copied())
        .map_or(JsValue::Null, JsValue::Object))
}

/// Allocate a TouchList wrapper carrying the given items.  Used by
/// the TouchEvent ctor to populate `touches` / `targetTouches` /
/// `changedTouches`.
pub(in crate::vm) fn alloc_touch_list(vm: &mut VmInner, items: Vec<ObjectId>) -> ObjectId {
    let proto = vm
        .touch_list_prototype
        .expect("TouchList.prototype must be registered before alloc_touch_list");
    let id = vm.alloc_object(Object {
        kind: ObjectKind::TouchList,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: Some(proto),
        extensible: true,
    });
    vm.touch_list_states.insert(id, TouchListState { items });
    id
}

// ---------------------------------------------------------------------------
// TouchEvent constructor
// ---------------------------------------------------------------------------

fn native_touch_event_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    check_construct(ctx, "TouchEvent")?;
    let type_sid = type_arg(ctx, args, "TouchEvent")?;
    let init_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let ui = parse_ui_event_init(ctx, init_arg, "TouchEvent")?;
    let opts_id = opts_object_id(init_arg);

    let (touches_id, target_touches_id, changed_touches_id, ctrl, shift, alt, meta) =
        if let Some(opts_id) = opts_id {
            let k_touches = ctx.vm.well_known.touches;
            let k_target_touches = ctx.vm.well_known.target_touches;
            let k_changed_touches = ctx.vm.well_known.changed_touches;
            let k_ctrl = ctx.vm.well_known.ctrl_key;
            let k_shift = ctx.vm.well_known.shift_key;
            let k_alt = ctx.vm.well_known.alt_key;
            let k_meta = ctx.vm.well_known.meta_key;
            let touches = parse_touch_sequence(ctx, opts_id, k_touches)?;
            let target_touches = parse_touch_sequence(ctx, opts_id, k_target_touches)?;
            let changed_touches = parse_touch_sequence(ctx, opts_id, k_changed_touches)?;
            let ctrl = read_optional_bool(ctx, opts_id, k_ctrl, false)?;
            let shift = read_optional_bool(ctx, opts_id, k_shift, false)?;
            let alt = read_optional_bool(ctx, opts_id, k_alt, false)?;
            let meta = read_optional_bool(ctx, opts_id, k_meta, false)?;
            (
                touches,
                target_touches,
                changed_touches,
                ctrl,
                shift,
                alt,
                meta,
            )
        } else {
            (
                Vec::new(),
                Vec::new(),
                Vec::new(),
                false,
                false,
                false,
                false,
            )
        };

    let touches_list = alloc_touch_list(ctx.vm, touches_id);
    let target_touches_list = alloc_touch_list(ctx.vm, target_touches_id);
    let changed_touches_list = alloc_touch_list(ctx.vm, changed_touches_id);

    let shape_id = ctx
        .vm
        .precomputed_event_shapes
        .as_ref()
        .expect("precomputed_event_shapes missing")
        .touch_event_constructed;
    let slots = vec![
        PropertyValue::Data(JsValue::Object(touches_list)),
        PropertyValue::Data(JsValue::Object(target_touches_list)),
        PropertyValue::Data(JsValue::Object(changed_touches_list)),
        PropertyValue::Data(JsValue::Boolean(ctrl)),
        PropertyValue::Data(JsValue::Boolean(shift)),
        PropertyValue::Data(JsValue::Boolean(alt)),
        PropertyValue::Data(JsValue::Boolean(meta)),
    ];
    let mut g0 = ctx.vm.push_temp_root(JsValue::Object(touches_list));
    let mut g1 = g0.push_temp_root(JsValue::Object(target_touches_list));
    let mut g = g1.push_temp_root(JsValue::Object(changed_touches_list));
    let touch_event_proto = g
        .touch_event_prototype
        .expect("TouchEvent.prototype must be registered before ctor");
    let id = g.build_ui_event_instance(this, type_sid, ui, shape_id, touch_event_proto, slots);
    drop(g);
    drop(g1);
    drop(g0);
    Ok(JsValue::Object(id))
}

/// Parse a `sequence<Touch>` member from a TouchEventInit.  Each
/// sequence entry must be Touch-brand; anything else throws
/// TypeError per WebIDL §3.10.21.  Missing key → empty Vec
/// (matches WebIDL default).
fn parse_touch_sequence(
    ctx: &mut NativeContext<'_>,
    opts_id: ObjectId,
    key: super::super::super::value::StringId,
) -> Result<Vec<ObjectId>, VmError> {
    let raw = ctx
        .vm
        .get_property_value(opts_id, PropertyKey::String(key))?;
    let arr_id = match raw {
        JsValue::Undefined => return Ok(Vec::new()),
        JsValue::Null => {
            return Err(VmError::type_error(
                "Failed to construct 'TouchEvent': \
                 sequence<Touch> member is null.",
            ));
        }
        JsValue::Object(id) => id,
        _ => {
            return Err(VmError::type_error(
                "Failed to construct 'TouchEvent': \
                 sequence<Touch> member is not iterable.",
            ));
        }
    };
    // For ObjectKind::Array, read the dense `elements` Vec
    // directly — `get_property_value` with a stringified index
    // does NOT hit the Array's dense storage in this VM (only the
    // bytecode LoadElement opcode does).  For non-Array Array-likes
    // (rare in init dicts) we'd need the indexed-property walker;
    // D-9 init dicts pass plain Arrays so the dense path suffices.
    let entries: Vec<JsValue> = match &ctx.vm.get_object(arr_id).kind {
        ObjectKind::Array { elements } => elements.clone(),
        _ => {
            return Err(VmError::type_error(
                "Failed to construct 'TouchEvent': \
                 sequence<Touch> member is not iterable.",
            ));
        }
    };
    let mut out = Vec::with_capacity(entries.len());
    for entry in entries {
        match entry {
            JsValue::Object(id) if matches!(ctx.vm.get_object(id).kind, ObjectKind::Touch) => {
                out.push(id);
            }
            _ => {
                return Err(VmError::type_error(
                    "Failed to construct 'TouchEvent': \
                     sequence<Touch> entry is not of type 'Touch'.",
                ));
            }
        }
    }
    Ok(out)
}

fn read_optional_bool(
    ctx: &mut NativeContext<'_>,
    opts_id: ObjectId,
    key: super::super::super::value::StringId,
    default: bool,
) -> Result<bool, VmError> {
    let v = ctx
        .vm
        .get_property_value(opts_id, PropertyKey::String(key))?;
    match v {
        JsValue::Undefined => Ok(default),
        _ => Ok(super::super::super::coerce::to_boolean(ctx.vm, v)),
    }
}
