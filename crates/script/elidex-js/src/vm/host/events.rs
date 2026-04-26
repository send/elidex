//! Event object construction — the JS-side view of a `DispatchEvent`
//! that gets passed to every listener.
//!
//! The event object is rebuilt **per listener invocation** (mirrors
//! boa's behaviour and sidesteps `currentTarget` mutation between
//! capture / target / bubble phases).  The flag fields are threaded through
//! `ObjectKind::Event`'s internal slots; `DispatchFlags` is synced
//! **in** (at construction) and **out** (in PR3 C5 `call_listener`) so
//! accumulated state (e.g. a prior listener's `preventDefault`)
//! propagates correctly.
//!
//! ## Per-instance vs prototype
//!
//! Methods (`preventDefault` / `stopPropagation` /
//! `stopImmediatePropagation` / `composedPath`) and the
//! `defaultPrevented` accessor live on the shared `Event.prototype`
//! (`VmInner::event_prototype`, populated once at `register_globals`
//! time alongside the `Event` global constructor).  Per-event
//! allocation is therefore just the data-property writes — no fresh
//! `NativeFunction` objects per dispatch, no fresh shape transitions
//! for the method properties.
//!
//! `Event.prototype` is JS-visible via `globalThis.Event.prototype`.
//! Both UA-initiated dispatch (via `create_event_object`) and script
//! construction (via `native_event_constructor`) chain through this
//! same object.
//!
//! ## Properties installed on each event
//!
//! | Property | Source | Shape |
//! |----------|--------|-------|
//! | `type` | `event.event_type` | data, RO |
//! | `bubbles` | `event.bubbles` | data, RO |
//! | `cancelable` | `event.cancelable` | data, RO |
//! | `eventPhase` | `event.phase as u8` | data, RO |
//! | `target` | `target_wrapper_id` | data, RO |
//! | `currentTarget` | `current_target_id` | data, RO |
//! | `timeStamp` | `start_instant.elapsed()` ms (shared with `performance.now`) | data, RO |
//! | `composed` | `event.composed` | data, RO |
//! | `isTrusted` | `event.is_trusted` | data, RO |
//! | `<payload-specific>` | `event.payload` | data, RO |
//!
//! ## Deferred to later PRs
//!
//! - `returnValue` legacy accessor → revisit when WPT
//!   `events/Event-*.html` runs.
//! - `initEvent` / `initXXXEvent` legacy initializers → rare, skipped.

#![cfg(feature = "engine")]

use elidex_script_session::event_dispatch::DispatchEvent;

use super::super::natives_event::{
    native_event_composed_path, native_event_get_default_prevented, native_event_prevent_default,
    native_event_stop_immediate_propagation, native_event_stop_propagation,
};
use super::super::shape::{self, PropertyAttrs, ShapeId};
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, StringId, VmError,
};
use super::super::{NativeFn, VmInner};
use super::event_shapes::CORE_KEY_COUNT;

impl VmInner {
    /// Populate `event_prototype` with the four event methods +
    /// `defaultPrevented` accessor.  This is the spec `Event.prototype`
    /// (WebIDL §2.2) — JS-visible via the `Event` global constructor
    /// installed by [`Self::register_event_global`].
    ///
    /// Called once from `register_globals` after `Object.prototype`
    /// exists; the resulting object is the prototype every event
    /// instance inherits from.
    pub(in crate::vm) fn register_event_prototype(&mut self) {
        let proto_id = self.create_object_with_methods(&[
            ("preventDefault", native_event_prevent_default as NativeFn),
            ("stopPropagation", native_event_stop_propagation),
            (
                "stopImmediatePropagation",
                native_event_stop_immediate_propagation,
            ),
            ("composedPath", native_event_composed_path),
        ]);
        // `defaultPrevented` is an accessor (live getter), not a data
        // property — WHATWG DOM §2.9 requires it to reflect the current
        // canceled flag including writes from `preventDefault()` made
        // inside the same listener body.
        self.install_accessor_pair(
            proto_id,
            self.well_known.default_prevented,
            native_event_get_default_prevented,
            None,
            PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        self.event_prototype = Some(proto_id);
    }

    /// Build the JS event object for a single listener invocation.
    ///
    /// `target_wrapper_id` and `current_target_wrapper_id` are the
    /// pre-resolved `HostObject` wrappers for the event's target and
    /// currentTarget entities — built by the caller via
    /// `create_element_wrapper`.  Keeping wrapper resolution out of
    /// this function lets the caller share target wrappers across
    /// phases (target wrapper is constant across capture / at-target /
    /// bubble; only `currentTarget` changes per phase).
    ///
    /// `passive` threads through from the listener's registration; the
    /// `Event` variant stores it so `preventDefault()` can no-op
    /// without looking it up from `HostData`.
    ///
    /// Property installation goes through the precomputed-shape fast
    /// path — see `host/event_shapes.rs` module doc for the layout
    /// and [`VmInner::define_with_precomputed_shape`] for the
    /// single-operation slot publish.
    ///
    /// # GC safety
    ///
    /// The just-allocated event id is rooted internally via
    /// [`VmInner::push_temp_root`] across all subsequent allocations
    /// (Focus payloads' `relatedTarget` allocates a wrapper; the
    /// `composedPath` array allocation does too).  Without rooting,
    /// the event obj would be the only thing tying its
    /// prototype/payload to a root and would be reclaimed
    /// mid-construction.  The guard drops before return — so the
    /// returned `ObjectId` becomes vulnerable to collection from the
    /// next allocation by the caller.  Root it immediately (push to
    /// stack via [`VmInner::push_temp_root`], store in a frame slot,
    /// etc.) before any further VM operations that may allocate or
    /// run user JS.
    pub(crate) fn create_event_object(
        &mut self,
        event: &DispatchEvent,
        target_wrapper_id: ObjectId,
        current_target_wrapper_id: ObjectId,
        passive: bool,
    ) -> ObjectId {
        // Intern the event type up front so it can seed both the
        // internal `type_sid` slot (authoritative for dispatch) and
        // the same-value JS `type` data property below.
        let type_sid = self.strings.intern(&event.event_type);
        let event_id = self.alloc_object(Object {
            kind: ObjectKind::Event {
                default_prevented: event.flags.default_prevented,
                propagation_stopped: event.flags.propagation_stopped,
                immediate_propagation_stopped: event.flags.immediate_propagation_stopped,
                cancelable: event.cancelable,
                passive,
                type_sid,
                bubbles: event.bubbles,
                composed: event.composed,
                composed_path: None,
            },
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            // Methods + `defaultPrevented` accessor inherited from
            // `Event.prototype` (shared across all events — UA-
            // initiated and script-constructed alike).
            prototype: self.event_prototype,
            extensible: true,
        });

        // Root the just-allocated event_id across composed-path /
        // relatedTarget wrapper allocations below.
        let mut g = self.push_temp_root(JsValue::Object(event_id));

        // ---- composedPath internal slot ----
        // If the dispatch path populated `event.composed_path` (the
        // ECS-side propagation list), translate each Entity into its
        // HostObject wrapper and seed the Event's `composed_path`
        // slot with the resulting Array.  `composedPath()` returns
        // this Array directly (identity-preserving).  Empty
        // `composed_path` leaves the slot None — `composedPath()`'s
        // lazy-allocate path then provides an empty Array on first
        // call and caches it (per WHATWG DOM §2.9 identity rule).
        if !event.composed_path.is_empty() {
            let elements: Vec<JsValue> = event
                .composed_path
                .iter()
                .map(|&entity| JsValue::Object(g.create_element_wrapper(entity)))
                .collect();
            let arr_id = g.create_array_object(elements);
            if let ObjectKind::Event { composed_path, .. } = &mut g.get_object_mut(event_id).kind {
                *composed_path = Some(arr_id);
            }
        }

        // ---- Assemble slot Vec in shape order ----
        // Core 9 first, then payload — matching
        // `build_precomputed_event_shapes`'s transition chain.  Any
        // reordering here must be mirrored there or slot values land
        // under the wrong JS-visible keys.
        //
        // Built as `Vec<PropertyValue>` directly (not `Vec<JsValue>`
        // with a later `.map(Data).collect()`) so
        // `define_with_precomputed_shape` can *move* the vector into
        // the object's slot storage — saves one heap allocation per
        // dispatch.  `type_sid` was interned before the alloc (see
        // top of function) so the JS-visible `type` property and the
        // internal `type_sid` slot share the same StringId.
        // 9 core + up to 8 payload (Mouse is the largest).  All 16 payload
        // variants fit in this capacity with no reallocation.
        let mut slots: Vec<PropertyValue> = Vec::with_capacity(17);
        slots.push(PropertyValue::Data(JsValue::String(type_sid)));
        slots.push(PropertyValue::Data(JsValue::Boolean(event.bubbles)));
        slots.push(PropertyValue::Data(JsValue::Boolean(event.cancelable)));
        slots.push(PropertyValue::Data(JsValue::Number(f64::from(
            event.phase as u8,
        ))));
        slots.push(PropertyValue::Data(JsValue::Object(target_wrapper_id)));
        slots.push(PropertyValue::Data(JsValue::Object(
            current_target_wrapper_id,
        )));
        // `timeStamp` is the monotonic ms elapsed since `Vm::new` —
        // shares `start_instant` with `performance.now()` so values
        // inside the same listener body are directly comparable
        // (HR-Time §5: identical time origin).
        let timestamp_ms = g.start_instant.elapsed().as_secs_f64() * 1000.0;
        slots.push(PropertyValue::Data(JsValue::Number(timestamp_ms)));
        slots.push(PropertyValue::Data(JsValue::Boolean(event.composed)));
        slots.push(PropertyValue::Data(JsValue::Boolean(event.is_trusted)));

        // Payload-specific slot values + matching terminal shape.
        // Shape selection and payload-slot writes live in a single
        // match so adding a variant touches exactly one function
        // (see `event_shapes::dispatch_payload`).
        //
        // May allocate (Focus's relatedTarget via
        // `create_element_wrapper`); the returned wrapper ObjectId
        // is immediately rooted in `HostData::wrapper_cache` inside
        // `create_element_wrapper` before we push it here.  The
        // existing `slots` Vec holds only primitives and already-
        // rooted wrappers (target/currentTarget, composed-path
        // wrappers, Focus relatedTarget) — no JsValue in the Vec
        // would be reclaimed if a GC ran during the Focus allocation.
        let shape_id = super::event_shapes::dispatch_payload(&mut g, &mut slots, &event.payload);
        g.define_with_precomputed_shape(event_id, shape_id, slots);

        drop(g);
        event_id
    }

    /// Build a freshly-constructed Event object for `new Event(type,
    /// init)` and subsequent specialized constructors (UIEvent family,
    /// PromiseRejectionEvent, ErrorEvent, etc.).  The pre-allocated
    /// `this` receiver from `do_new` is promoted in place to
    /// `ObjectKind::Event` so
    /// the subclass prototype chain (`class Sub extends Event {}`)
    /// is preserved — overwriting `this` with a fresh allocation
    /// would drop the `Sub.prototype` link.
    ///
    /// Core-9 slot values are:
    /// `type` / `bubbles` / `cancelable` / `eventPhase = 0` /
    /// `target = null` / `currentTarget = null` / `timeStamp = <now>` /
    /// `composed` / `isTrusted`.  `payload_slots` extends this in the
    /// order implied by `shape_id`.  `shape_id` must refer to a shape
    /// built by `build_precomputed_event_shapes` (or an augmented
    /// variant) — length of the combined slot vec must equal
    /// `shape.property_count()`, otherwise
    /// `define_with_precomputed_shape` debug-asserts.
    pub(crate) fn create_fresh_event_object(
        &mut self,
        this: JsValue,
        type_sid: StringId,
        init: EventInit,
        shape_id: ShapeId,
        payload_slots: Vec<PropertyValue>,
        is_trusted: bool,
    ) -> ObjectId {
        // `ensure_instance_or_alloc` in construct-mode returns `this`
        // as-is (already allocated by `do_new` with the subclass
        // prototype); in call-mode it allocates a fresh Ordinary
        // whose prototype is `Event.prototype`.  Constructors gate
        // call-mode out via `is_construct()` before reaching here,
        // so call-mode only runs through tests / assertions.
        let receiver = self.ensure_instance_or_alloc(this, self.event_prototype);
        let JsValue::Object(id) = receiver else {
            unreachable!("ensure_instance_or_alloc always yields an Object");
        };
        // Promote the pre-allocated Ordinary to `ObjectKind::Event`.
        // `cancelable` stored in the internal slot because
        // `preventDefault()` consults it without a property read
        // (hot path).  `passive` is always false for script-
        // constructed events — passive is a listener-registration
        // flag, not an event-construction one.  `type_sid` /
        // `bubbles` / `composed` are seeded here too so dispatch
        // reads the authoritative ctor-supplied values rather than
        // the JS-exposed (user-mutable) data properties.
        self.get_object_mut(id).kind = ObjectKind::Event {
            default_prevented: false,
            propagation_stopped: false,
            immediate_propagation_stopped: false,
            cancelable: init.cancelable,
            passive: false,
            type_sid,
            bubbles: init.bubbles,
            composed: init.composed,
            composed_path: None,
        };
        let timestamp_ms = self.start_instant.elapsed().as_secs_f64() * 1000.0;
        let mut slots: Vec<PropertyValue> =
            Vec::with_capacity(CORE_KEY_COUNT + payload_slots.len());
        slots.push(PropertyValue::Data(JsValue::String(type_sid)));
        slots.push(PropertyValue::Data(JsValue::Boolean(init.bubbles)));
        slots.push(PropertyValue::Data(JsValue::Boolean(init.cancelable)));
        // eventPhase = NONE (WHATWG DOM §2.2).  Mutated to
        // CAPTURING_PHASE / AT_TARGET / BUBBLING_PHASE by
        // `dispatchEvent`.
        slots.push(PropertyValue::Data(JsValue::Number(0.0)));
        slots.push(PropertyValue::Data(JsValue::Null));
        slots.push(PropertyValue::Data(JsValue::Null));
        slots.push(PropertyValue::Data(JsValue::Number(timestamp_ms)));
        slots.push(PropertyValue::Data(JsValue::Boolean(init.composed)));
        slots.push(PropertyValue::Data(JsValue::Boolean(is_trusted)));
        slots.extend(payload_slots);
        self.define_with_precomputed_shape(id, shape_id, slots);
        id
    }

    /// Install the `Event` global constructor + populate
    /// `Event.prototype.constructor`.  Must run **after**
    /// [`Self::register_event_prototype`] (that creates
    /// `self.event_prototype`).
    pub(in crate::vm) fn register_event_global(&mut self) {
        let proto_id = self
            .event_prototype
            .expect("register_event_global called before register_event_prototype");
        let ctor = self.create_constructable_function("Event", native_event_constructor);
        let proto_key = PropertyKey::String(self.well_known.prototype);
        self.define_shaped_property(
            ctor,
            proto_key,
            PropertyValue::Data(JsValue::Object(proto_id)),
            PropertyAttrs::BUILTIN,
        );
        let ctor_key = PropertyKey::String(self.well_known.constructor);
        self.define_shaped_property(
            proto_id,
            ctor_key,
            PropertyValue::Data(JsValue::Object(ctor)),
            PropertyAttrs::METHOD,
        );
        let name = self.well_known.event_global;
        self.globals.insert(name, JsValue::Object(ctor));
    }

    /// Install `CustomEvent.prototype` (chained to `Event.prototype`),
    /// `CustomEvent.prototype.detail` accessor, `.constructor`
    /// back-pointer, and the `CustomEvent` global.  Must run after
    /// [`Self::register_event_global`] (which sets
    /// `self.event_prototype.constructor`).
    pub(in crate::vm) fn register_custom_event_global(&mut self) {
        let event_proto = self
            .event_prototype
            .expect("register_custom_event_global called before register_event_prototype");
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(event_proto),
            extensible: true,
        });
        // `detail` accessor — reads the `detail` own-data slot on the
        // CustomEvent instance.  Installed as a prototype accessor so
        // `Object.keys(new CustomEvent('x', {detail: 1}))` still
        // contains `detail` via the slot (own property) while
        // prototype-side lookups route through the getter for
        // wrong-brand / subclass-without-slot cases.
        //
        // NOTE: Because CustomEvent stores `detail` as an own data
        // property (slot 9 of the `custom_event` shape), the accessor
        // is shadowed by the own property for normal instances — it
        // only fires for e.g. `CustomEvent.prototype.detail` reads
        // (`undefined`, matching browsers).
        self.install_accessor_pair(
            proto_id,
            self.well_known.detail,
            native_custom_event_get_detail,
            None,
            PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        self.custom_event_prototype = Some(proto_id);

        let ctor =
            self.create_constructable_function("CustomEvent", native_custom_event_constructor);
        let proto_key = PropertyKey::String(self.well_known.prototype);
        self.define_shaped_property(
            ctor,
            proto_key,
            PropertyValue::Data(JsValue::Object(proto_id)),
            PropertyAttrs::BUILTIN,
        );
        let ctor_key = PropertyKey::String(self.well_known.constructor);
        self.define_shaped_property(
            proto_id,
            ctor_key,
            PropertyValue::Data(JsValue::Object(ctor)),
            PropertyAttrs::METHOD,
        );
        let name = self.well_known.custom_event_global;
        self.globals.insert(name, JsValue::Object(ctor));
    }
}

// ---------------------------------------------------------------------
// Constructors + init-dict parsers
// ---------------------------------------------------------------------

/// Shared WebIDL `[Constructor]` gate — every Event family ctor
/// must reject call-mode invocation (`Event('click')` without `new`)
/// before reaching any argument coercion.  Returns `Err(TypeError)`
/// in call mode, `Ok(())` in construct mode.  Error message format
/// matches the `Event` / `CustomEvent` ctors originally in this file.
pub(super) fn check_construct(ctx: &NativeContext<'_>, interface: &str) -> Result<(), VmError> {
    if ctx.is_construct() {
        Ok(())
    } else {
        Err(VmError::type_error(format!(
            "Failed to construct '{interface}': Please use the 'new' operator",
        )))
    }
}

/// Extract the required `type` first-argument from a ctor call.
/// Absent arg → TypeError; Symbol or other non-string values pass
/// through `coerce::to_string` (Symbol throws per ES2020 §7.1.12).
pub(super) fn type_arg(
    ctx: &mut NativeContext<'_>,
    args: &[JsValue],
    interface: &str,
) -> Result<StringId, VmError> {
    let v = args.first().copied().ok_or_else(|| {
        VmError::type_error(format!(
            "Failed to construct '{interface}': 1 argument required, but only 0 present.",
        ))
    })?;
    super::super::coerce::to_string(ctx.vm, v)
}

/// Wire a ctor function object to its prototype + the `name` global.
///
/// Shared between `Event` / `CustomEvent` (this module), the UIEvent
/// family ([`super::events_ui`]) and the direct-Event descendants
/// ([`super::events_extras`]) — all four register sites installed the
/// same three-step pattern: create the native ctor function, set
/// `ctor.prototype = proto_id` with BUILTIN attrs, set
/// `proto_id.constructor = ctor` with METHOD attrs, expose on
/// `globals[name]`.
pub(super) fn install_ctor(
    vm: &mut VmInner,
    proto_id: ObjectId,
    name: &str,
    func: NativeFn,
    global_sid: StringId,
) {
    let ctor = vm.create_constructable_function(name, func);
    let proto_key = PropertyKey::String(vm.well_known.prototype);
    vm.define_shaped_property(
        ctor,
        proto_key,
        PropertyValue::Data(JsValue::Object(proto_id)),
        PropertyAttrs::BUILTIN,
    );
    let ctor_key = PropertyKey::String(vm.well_known.constructor);
    vm.define_shaped_property(
        proto_id,
        ctor_key,
        PropertyValue::Data(JsValue::Object(ctor)),
        PropertyAttrs::METHOD,
    );
    vm.globals.insert(global_sid, JsValue::Object(ctor));
}

/// `EventInit` dictionary (WHATWG DOM §2.4).  Defaults: all `false`.
///
/// `pub(crate)` because [`VmInner::create_fresh_event_object`]
/// exposes it — both the plain `Event` ctor and the specialized
/// constructors in sibling `host/*` modules build an `EventInit` via
/// [`parse_event_init`] and hand it off.
#[derive(Default, Clone, Copy)]
pub(crate) struct EventInit {
    pub(crate) bubbles: bool,
    pub(crate) cancelable: bool,
    pub(crate) composed: bool,
}

/// WHATWG DOM §2.4: parse an `EventInit` dictionary from `val`.
///
/// - `undefined` / `null` / missing → all flags `false`.
/// - Object → read `bubbles`, `cancelable`, `composed` (boolean coercion;
///   missing keys default to `false`).  Getter side-effects on the
///   init object are observable.
/// - Other (string / number / etc.) → `TypeError` matching WebIDL
///   dictionary coercion.
///
/// `interface` names the constructor for the error message
/// (`Event` / `CustomEvent`).
pub(super) fn parse_event_init(
    ctx: &mut NativeContext<'_>,
    val: JsValue,
    interface: &str,
) -> Result<EventInit, VmError> {
    match val {
        JsValue::Undefined | JsValue::Null => Ok(EventInit::default()),
        JsValue::Object(opts_id) => {
            // Read order matches Chrome's invocation order: bubbles,
            // cancelable, composed (verified via userland getter probe).
            // Each `get_property_value` may fire user getters; side
            // effects on the init object are observable.
            let mut out = EventInit::default();
            for (key_sid, slot) in [
                (ctx.vm.well_known.bubbles, &mut out.bubbles),
                (ctx.vm.well_known.cancelable, &mut out.cancelable),
                (ctx.vm.well_known.composed, &mut out.composed),
            ] {
                let v = ctx
                    .vm
                    .get_property_value(opts_id, PropertyKey::String(key_sid))?;
                *slot = super::super::coerce::to_boolean(ctx.vm, v);
            }
            Ok(out)
        }
        _ => Err(VmError::type_error(format!(
            "Failed to construct '{interface}': \
             The provided value is not of type '{interface}Init'.",
        ))),
    }
}

/// `new Event(type, eventInitDict?)` (WHATWG DOM §2.4).
///
/// - `type` required; absent → `TypeError`.  Non-string values
///   coerce via `ToString` (Symbol throws).
/// - `eventInitDict` optional; see [`parse_event_init`].
/// - `new` required; call-mode (`Event('click')`) → `TypeError`
///   (WebIDL `[Constructor]` gate — matches all major browsers).
fn native_event_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if !ctx.is_construct() {
        return Err(VmError::type_error(
            "Failed to construct 'Event': Please use the 'new' operator",
        ));
    }
    let type_arg = args.first().copied().ok_or_else(|| {
        VmError::type_error("Failed to construct 'Event': 1 argument required, but only 0 present.")
    })?;
    let type_sid = super::super::coerce::to_string(ctx.vm, type_arg)?;
    let init = parse_event_init(
        ctx,
        args.get(1).copied().unwrap_or(JsValue::Undefined),
        "Event",
    )?;
    let shape_id = ctx
        .vm
        .precomputed_event_shapes
        .as_ref()
        .expect("precomputed_event_shapes missing — register_globals did not run")
        .core;
    let id = ctx
        .vm
        .create_fresh_event_object(this, type_sid, init, shape_id, Vec::new(), false);
    Ok(JsValue::Object(id))
}

/// `new CustomEvent(type, customEventInitDict?)` (WHATWG DOM §2.3).
///
/// Extends `EventInit` with `detail: any = null`.  Missing `detail`
/// defaults to `null` (WHATWG default); an explicit `{detail:
/// undefined}` is also normalised to `null` by this implementation
/// (see the in-body comment — the WebIDL `any` type technically
/// preserves `undefined`, and a future change may relax this).
fn native_custom_event_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if !ctx.is_construct() {
        return Err(VmError::type_error(
            "Failed to construct 'CustomEvent': Please use the 'new' operator",
        ));
    }
    let type_arg = args.first().copied().ok_or_else(|| {
        VmError::type_error(
            "Failed to construct 'CustomEvent': 1 argument required, but only 0 present.",
        )
    })?;
    let type_sid = super::super::coerce::to_string(ctx.vm, type_arg)?;
    let init_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let base = parse_event_init(ctx, init_arg, "CustomEvent")?;
    // Read `detail` separately — WebIDL `any` preserves the
    // supplied value including `undefined`.  Missing key also yields
    // `Undefined` from `get_property_value`; the WHATWG default is
    // `null`.  We collapse both to `null` for parity with Chrome's
    // common-case behaviour; a strict "own-key-present" distinction
    // (which would preserve explicit `{detail: undefined}`) can be
    // added later if tests require it.
    let detail = match init_arg {
        JsValue::Object(opts_id) => {
            let v = ctx
                .vm
                .get_property_value(opts_id, PropertyKey::String(ctx.vm.well_known.detail))?;
            if matches!(v, JsValue::Undefined) {
                JsValue::Null
            } else {
                v
            }
        }
        _ => JsValue::Null,
    };
    let shape_id = ctx
        .vm
        .precomputed_event_shapes
        .as_ref()
        .expect("precomputed_event_shapes missing — register_globals did not run")
        .custom_event;
    // Root `detail` across the in-place promotion inside
    // `create_fresh_event_object` — if `detail` is an Object, GC
    // could collect it between here and the slot write without a
    // root.  The guard also borrows the VM mutably, so subsequent
    // ops go through the guard's `Deref<Target = VmInner>`.
    let mut g = ctx.vm.push_temp_root(detail);
    let payload_slots = vec![PropertyValue::Data(detail)];
    let id = g.create_fresh_event_object(this, type_sid, base, shape_id, payload_slots, false);
    drop(g);
    Ok(JsValue::Object(id))
}

/// `get CustomEvent.prototype.detail` — fallback accessor for
/// subclass instances (or direct prototype reads) that don't carry
/// the `detail` own-data slot.  Most instances hit the own property
/// (slot 9) first and never reach this getter.
fn native_custom_event_get_detail(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------
// Core-9 slot indices — see `host::event_shapes` CORE_KEY_COUNT and
// `build_precomputed_event_shapes` for the authoritative ordering.
// `dispatchEvent` mutates `target`, `currentTarget`, and `eventPhase`
// in place across phases; the named constants keep call sites
// self-documenting instead of reading as "slot 3".
// ---------------------------------------------------------------------

/// Core-9 slot index for `eventPhase` (`0` / `1=CAPTURING` / `2=AT_TARGET` /
/// `3=BUBBLING`).  WHATWG DOM §2.2.
pub(crate) const EVENT_SLOT_EVENT_PHASE: usize = 3;
/// Core-9 slot index for `target`.  Retargeted per-listener during dispatch
/// (WHATWG DOM §2.5) — restored on dispatch completion.
pub(crate) const EVENT_SLOT_TARGET: usize = 4;
/// Core-9 slot index for `currentTarget`.  Advances through the
/// propagation path; `null` outside a dispatch.
pub(crate) const EVENT_SLOT_CURRENT_TARGET: usize = 5;

/// Overwrite one core-9 slot on an `ObjectKind::Event` in place.
///
/// Script-initiated dispatch needs to advance `currentTarget` /
/// `eventPhase` / `target` on a user-constructed event object
/// without changing its shape — `define_shaped_property` would
/// treat each write as a shape transition (allocating a fresh
/// shape to record attr changes) and defeat the precomputed-shape
/// fast path.
///
/// Storage modes:
/// - **Shaped** (common case): direct slot index write, zero
///   shape-transition overhead.
/// - **Dictionary** (after user-side `delete evt.target` etc.):
///   WebIDL core attributes are `WEBIDL_RO` (configurable=true),
///   so deletion is legal and triggers the Shaped → Dictionary
///   transition.  Spec-wise dispatch must still set the
///   attribute; fall back to a keyed set with two sub-cases:
///   - the key still exists in the dictionary (e.g. the user
///     re-assigned `evt.target = 'x'` after deleting) → update
///     the slot value in place, **preserving the existing
///     attrs** so a user's `Object.defineProperty(evt,
///     'target', …)` custom descriptor isn't clobbered by
///     dispatch.
///   - the key is absent (pure delete, no subsequent write) →
///     append a fresh entry with the `WEBIDL_RO` attrs the ctor
///     originally installed.
///   Matches Chrome semantics (IDL attribute accessor reads the
///   internal slot even post-delete; dispatch re-installs the
///   property but respects any custom descriptor already in
///   place).
pub(crate) fn set_event_slot_raw(
    vm: &mut VmInner,
    event_id: ObjectId,
    slot_idx: usize,
    new_val: JsValue,
) {
    debug_assert!(
        slot_idx < CORE_KEY_COUNT,
        "set_event_slot_raw: slot index {slot_idx} ≥ CORE_KEY_COUNT={CORE_KEY_COUNT} — \
         payload slots are variant-specific and must not be touched by dispatch"
    );
    // Pre-capture the StringId for Dictionary fallback so the
    // subsequent `&mut obj.storage` borrow doesn't race with
    // `vm.well_known`.  Only target / currentTarget / eventPhase
    // are mutated during dispatch; other slot indices are
    // unreachable from C5 + C5/walk_phase call sites.
    let key_sid_opt = match slot_idx {
        EVENT_SLOT_EVENT_PHASE => Some(vm.well_known.event_phase),
        EVENT_SLOT_TARGET => Some(vm.well_known.target),
        EVENT_SLOT_CURRENT_TARGET => Some(vm.well_known.current_target),
        _ => None,
    };
    let obj = vm.get_object_mut(event_id);
    debug_assert!(
        matches!(obj.kind, ObjectKind::Event { .. }),
        "set_event_slot_raw: object is not ObjectKind::Event"
    );
    match &mut obj.storage {
        PropertyStorage::Shaped { slots, .. } => {
            slots[slot_idx] = PropertyValue::Data(new_val);
        }
        PropertyStorage::Dictionary(vec) => {
            // User-invoked `delete e.target` (or similar) flipped
            // the object to Dictionary storage; restore the slot
            // semantically.  Caller must provide a slot_idx that
            // maps to a known well-known key (target /
            // currentTarget / eventPhase) — other indices would
            // indicate a new call site added to dispatch that
            // doesn't have a Dictionary fallback mapped yet.
            let Some(key_sid) = key_sid_opt else {
                debug_assert!(
                    false,
                    "set_event_slot_raw: Dictionary fallback missing for slot index {slot_idx}"
                );
                return;
            };
            let key = PropertyKey::String(key_sid);
            if let Some(entry) = vec.iter_mut().find(|(k, _)| *k == key) {
                entry.1.slot = PropertyValue::Data(new_val);
            } else {
                vec.push((
                    key,
                    super::super::value::Property::from_attrs(
                        PropertyValue::Data(new_val),
                        PropertyAttrs::WEBIDL_RO,
                    ),
                ));
            }
        }
    }
}

// ---------------------------------------------------------------------
// Payload slot assembly lives in `host::event_shapes::dispatch_payload`
// — shape selection and slot writes share a single SSOT match.
// ---------------------------------------------------------------------
