//! `PointerEvent` constructor + prototype (UI Events Pointer §6).
//!
//! `PointerEvent.prototype` chains to `MouseEvent.prototype`.  The
//! constructor accepts a [`PointerEventInit`] dictionary; instances
//! carry the 13 mouse-event own slots PLUS 12 pointer-specific
//! own slots (pointerId / width / height / pressure / tangential
//! Pressure / tiltX / tiltY / twist / altitudeAngle / azimuthAngle /
//! pointerType / isPrimary).
//!
//! `getCoalescedEvents()` / `getPredictedEvents()` are prototype
//! methods returning fresh empty Arrays — real UA-fired aggregation
//! infra is deferred to slot `#11-pointer-event-coalesced-
//! predicted`.

#![cfg(feature = "engine")]

use super::super::super::shape;
use super::super::super::value::{JsValue, NativeContext, ObjectId, PropertyValue, VmError};
use super::super::super::VmInner;
use super::super::events::{check_construct, type_arg};
use super::super::events_extras::{read_bool, read_number};
use super::super::events_ui::{
    opts_object_id, parse_mouse_event_members, parse_ui_event_init, register_descendant,
    MouseEventMembers,
};

// ---------------------------------------------------------------------------
// VmInner: registration glue
// ---------------------------------------------------------------------------

pub(in crate::vm) fn register_pointer_event_global(vm: &mut VmInner) {
    register_descendant(
        vm,
        vm.mouse_event_prototype,
        "PointerEvent",
        native_pointer_event_constructor,
        vm.well_known.pointer_event_global,
        |vm, id| vm.pointer_event_prototype = Some(id),
    );
    let proto_id = vm
        .pointer_event_prototype
        .expect("register_pointer_event_global just stored pointer_event_prototype");
    let get_coalesced_sid = vm.well_known.get_coalesced_events;
    let get_predicted_sid = vm.well_known.get_predicted_events;
    vm.install_native_method(
        proto_id,
        get_coalesced_sid,
        native_pointer_event_get_coalesced_events,
        shape::PropertyAttrs::METHOD,
    );
    vm.install_native_method(
        proto_id,
        get_predicted_sid,
        native_pointer_event_get_predicted_events,
        shape::PropertyAttrs::METHOD,
    );
}

// ---------------------------------------------------------------------------
// Constructor
// ---------------------------------------------------------------------------

/// Result of parsing the 12 PointerEvent-specific init-dict members.
/// `getCoalescedEvents` / `getPredictedEvents` sequence inputs are
/// validated (each entry must be PointerEvent-brand) but not stored
/// — D-9 returns fresh empty Arrays.
struct PointerEventMembers {
    slots: Vec<PropertyValue>,
}

#[allow(clippy::similar_names)] // spec field names: tilt_x/tilt_y, etc.
fn parse_pointer_event_members(
    ctx: &mut NativeContext<'_>,
    opts_id: Option<ObjectId>,
) -> Result<PointerEventMembers, VmError> {
    use super::super::events_ui::read_string;

    // Default values per Pointer Events §6 IDL: width=1, height=1,
    // altitudeAngle=π/2, others=0 / "" / false.
    let (
        pointer_id,
        width,
        height,
        pressure,
        tangential_pressure,
        tilt_x,
        tilt_y,
        twist,
        altitude_angle,
        azimuth_angle,
        pointer_type_sid,
        is_primary,
    );

    if let Some(opts_id) = opts_id {
        // Snapshot StringId fields by value first (StringId is Copy)
        // to avoid simultaneous immutable borrow of `well_known` +
        // mutable borrow of `ctx.vm` via `read_number_with_default`.
        let k_pointer_id = ctx.vm.well_known.pointer_id;
        let k_width = ctx.vm.well_known.width;
        let k_height = ctx.vm.well_known.height;
        let k_pressure = ctx.vm.well_known.pressure;
        let k_tangential = ctx.vm.well_known.tangential_pressure;
        let k_tilt_x = ctx.vm.well_known.tilt_x;
        let k_tilt_y = ctx.vm.well_known.tilt_y;
        let k_twist = ctx.vm.well_known.twist;
        let k_altitude = ctx.vm.well_known.altitude_angle;
        let k_azimuth = ctx.vm.well_known.azimuth_angle;
        let k_pointer_type = ctx.vm.well_known.pointer_type;
        let k_is_primary = ctx.vm.well_known.is_primary;
        // `pointerId` / `tiltX` / `tiltY` / `twist` are WebIDL `long`
        // — signed-32 truncate (matches MouseEventInit.button precedent
        // but for the wider 32-bit range).
        let id_raw = read_number(ctx, opts_id, k_pointer_id, 0.0)?;
        let tilt_x_raw = read_number(ctx, opts_id, k_tilt_x, 0.0)?;
        let tilt_y_raw = read_number(ctx, opts_id, k_tilt_y, 0.0)?;
        let twist_raw = read_number(ctx, opts_id, k_twist, 0.0)?;
        pointer_id = f64::from(super::super::super::coerce::f64_to_int32(id_raw));
        tilt_x = f64::from(super::super::super::coerce::f64_to_int32(tilt_x_raw));
        tilt_y = f64::from(super::super::super::coerce::f64_to_int32(tilt_y_raw));
        twist = f64::from(super::super::super::coerce::f64_to_int32(twist_raw));
        // `width` / `height` / `pressure` / `tangentialPressure` /
        // `altitudeAngle` / `azimuthAngle` are WebIDL `double` /
        // `float`.  All `double` is a finite-required type — but
        // Pointer Events §6 doesn't apply `[EnforceRange]`, so NaN /
        // ±∞ is allowed at IDL level (matches Chrome / Firefox).
        // ToNumber-only, no clamp.
        width = read_number(ctx, opts_id, k_width, 1.0)?;
        height = read_number(ctx, opts_id, k_height, 1.0)?;
        pressure = read_number(ctx, opts_id, k_pressure, 0.0)?;
        tangential_pressure = read_number(ctx, opts_id, k_tangential, 0.0)?;
        // Default altitudeAngle = π/2 per spec (the "device upright"
        // canonical pose).
        altitude_angle = read_number(ctx, opts_id, k_altitude, std::f64::consts::FRAC_PI_2)?;
        azimuth_angle = read_number(ctx, opts_id, k_azimuth, 0.0)?;
        // `pointerType` is `DOMString` not enumerated (Chrome accepts
        // arbitrary values — spec mentions `"" / "mouse" / "pen" /
        // "touch"` as common values but doesn't enforce).
        pointer_type_sid = read_string(ctx, opts_id, k_pointer_type)?;
        // `isPrimary` is `boolean` — `ToBoolean`.
        is_primary = read_bool(ctx, opts_id, k_is_primary, false)?;
    } else {
        let empty = ctx.vm.strings.intern("");
        pointer_id = 0.0;
        width = 1.0;
        height = 1.0;
        pressure = 0.0;
        tangential_pressure = 0.0;
        tilt_x = 0.0;
        tilt_y = 0.0;
        twist = 0.0;
        altitude_angle = std::f64::consts::FRAC_PI_2;
        azimuth_angle = 0.0;
        pointer_type_sid = empty;
        is_primary = false;
    }

    // Slot order matches the `pointer_event_constructed` shape (see
    // event_shapes.rs).  12 slots in this order.
    let slots = vec![
        PropertyValue::Data(JsValue::Number(pointer_id)),
        PropertyValue::Data(JsValue::Number(width)),
        PropertyValue::Data(JsValue::Number(height)),
        PropertyValue::Data(JsValue::Number(pressure)),
        PropertyValue::Data(JsValue::Number(tangential_pressure)),
        PropertyValue::Data(JsValue::Number(tilt_x)),
        PropertyValue::Data(JsValue::Number(tilt_y)),
        PropertyValue::Data(JsValue::Number(twist)),
        PropertyValue::Data(JsValue::Number(altitude_angle)),
        PropertyValue::Data(JsValue::Number(azimuth_angle)),
        PropertyValue::Data(JsValue::String(pointer_type_sid)),
        PropertyValue::Data(JsValue::Boolean(is_primary)),
    ];

    Ok(PointerEventMembers { slots })
}

fn native_pointer_event_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    check_construct(ctx, "PointerEvent")?;
    let type_sid = type_arg(ctx, args, "PointerEvent")?;
    let init_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let ui = parse_ui_event_init(ctx, init_arg, "PointerEvent")?;
    let opts_id = opts_object_id(init_arg);
    let MouseEventMembers {
        related_target,
        mut slots,
    } = parse_mouse_event_members(ctx, opts_id)?;
    let pointer_members = parse_pointer_event_members(ctx, opts_id)?;
    slots.extend(pointer_members.slots);

    let shape_id = ctx
        .vm
        .precomputed_event_shapes
        .as_ref()
        .expect("precomputed_event_shapes missing")
        .pointer_event_constructed;
    let mut g = ctx.vm.push_temp_root(related_target);
    let pointer_proto = g
        .pointer_event_prototype
        .expect("PointerEvent.prototype must be registered before ctor");
    let id = g.build_ui_event_instance(this, type_sid, ui, shape_id, pointer_proto, slots);
    drop(g);
    Ok(JsValue::Object(id))
}

// ---------------------------------------------------------------------------
// Prototype methods — `getCoalescedEvents()` / `getPredictedEvents()`
// ---------------------------------------------------------------------------

/// `PointerEvent.prototype.getCoalescedEvents()` — Pointer Events
/// §6.5.  D-9 returns a fresh empty Array per call; real
/// UA-aggregated coalesced events are deferred to slot
/// `#11-pointer-event-coalesced-predicted` (paired with a UA
/// pointer-event fire path that produces them).
fn native_pointer_event_get_coalesced_events(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    super::super::events::require_event_subclass_receiver(
        ctx,
        this,
        ctx.vm.pointer_event_prototype,
        "PointerEvent",
        "getCoalescedEvents",
        super::super::events::BrandCheckKind::Operation,
    )?;
    let id = ctx.vm.create_array_object(Vec::new());
    Ok(JsValue::Object(id))
}

/// `PointerEvent.prototype.getPredictedEvents()` — Pointer Events
/// §6.5.  Same stub pattern as `getCoalescedEvents()`.
fn native_pointer_event_get_predicted_events(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    super::super::events::require_event_subclass_receiver(
        ctx,
        this,
        ctx.vm.pointer_event_prototype,
        "PointerEvent",
        "getPredictedEvents",
        super::super::events::BrandCheckKind::Operation,
    )?;
    let id = ctx.vm.create_array_object(Vec::new());
    Ok(JsValue::Object(id))
}
