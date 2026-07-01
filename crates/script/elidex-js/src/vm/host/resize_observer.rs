//! `ResizeObserver` interface (W3C Resize Observer §2.1) — VM thin
//! binding to the engine-independent
//! [`elidex_api_observers::resize::ResizeObserverRegistry`].
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", this file contains only the
//! engine-bound responsibilities: prototype install, brand check,
//! `ResizeObserverOptions` dictionary parsing (JS → Rust marshalling),
//! one-line dispatch into the registry helpers, and per-entry
//! marshalling at delivery time.  The actual observation algorithm —
//! target tracking, size change detection, frame-broadcast gathering —
//! lives in [`elidex_api_observers::resize`].
//!
//! ## State storage
//!
//! Observer state is split between two side tables:
//!
//! - [`super::super::host_data::HostData::resize_observers`] — the
//!   [`elidex_api_observers::resize::ResizeObserverRegistry`] that
//!   owns the monotonic observer ID counter (target tracking lives as
//!   per-entity `ResizeObservedBy` components, not in the registry).
//! - [`super::super::host_data::HostData::resize_observer_bindings`]
//!   — `HashMap<u64, ObserverBinding>` from observer ID to the
//!   `(callback, instance)` JS-identity pair.  Both `ObjectId`s in
//!   each binding are rooted by the keepalive seam's active-observation
//!   predicate ([`super::super::gc::keepalive::keepalive_survivors`], S5-3c)
//!   so the callback + instance survive GC while the observer observes ≥1
//!   target, and the binding row is sweep-pruned once collectible.
//!
//! [`super::super::value::ObjectKind::Observer`] with
//! [`super::super::value::ObserverKind::Resize`] carries the
//! observer ID inline (`observer_id: u64`); the JS object itself has no
//! other own state.
//!
//! ## `native_*` docstring convention
//!
//! Per-method native `fn`s in this module (and the sibling
//! `mutation_observer.rs` / `intersection_observer.rs`) deliberately
//! rely on the constructor's docstring + the module-level spec
//! citation as their primary documentation.  Brand-checked function
//! names (`native_resize_observer_observe` etc.) are unique enough
//! to disambiguate without a per-fn docstring; the spec section is
//! cited inline at the call site where it actually matters.
//!
//! ## Lifecycle preconditions
//!
//! - **Constructor** (`new ResizeObserver(cb)`) requires
//!   [`super::super::Vm::install_host_data`] to have been called.  It
//!   does *not* require a bound `EcsDom` because callback / instance
//!   bookkeeping lives entirely on `HostData`-owned side tables.
//! - **Method natives** (`observe` / `unobserve` / `disconnect`) check
//!   `ctx.host_if_bound()` first and return a safe no-op (`undefined`)
//!   so a retained `ro` reference survives a
//!   [`super::super::Vm::unbind`] boundary.

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectKind, PropertyKey, PropertyStorage, PropertyValue,
    VmError,
};
use super::super::{NativeFn, VmInner};
use super::observer_common::{build_marshalled_array, deliver_to_observer_callbacks};

use elidex_api_observers::resize::{
    ResizeObserverBoxOptions, ResizeObserverId, ResizeObserverOptions,
};

impl VmInner {
    /// Allocate `ResizeObserver.prototype` chained to `Object.prototype`,
    /// install its three method natives, and expose the `ResizeObserver`
    /// constructor on `globalThis`.
    ///
    /// Called from `register_globals()` after `register_prototypes`.
    ///
    /// # Panics
    ///
    /// Panics if `object_prototype` is `None`.
    pub(in crate::vm) fn register_resize_observer_global(&mut self) {
        let object_proto = self
            .object_prototype
            .expect("register_resize_observer_global called before register_prototypes");

        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });

        let wk = &self.well_known;
        let entries = [
            (wk.observe, native_resize_observer_observe as NativeFn),
            (wk.unobserve, native_resize_observer_unobserve as NativeFn),
            (wk.disconnect, native_resize_observer_disconnect as NativeFn),
        ];
        for (name_sid, func) in entries {
            self.install_native_method(proto_id, name_sid, func, shape::PropertyAttrs::METHOD);
        }
        self.resize_observer_prototype = Some(proto_id);

        let ctor = self
            .create_constructor_only_function("ResizeObserver", native_resize_observer_constructor);
        let proto_key = PropertyKey::String(self.well_known.prototype);
        self.define_shaped_property(
            ctor,
            proto_key,
            PropertyValue::Data(JsValue::Object(proto_id)),
            shape::PropertyAttrs::BUILTIN,
        );
        let ctor_key = PropertyKey::String(self.well_known.constructor);
        self.define_shaped_property(
            proto_id,
            ctor_key,
            PropertyValue::Data(JsValue::Object(ctor)),
            shape::PropertyAttrs::METHOD,
        );
        let name_sid = self.well_known.resize_observer_global;
        self.globals.insert(name_sid, JsValue::Object(ctor));
    }
}

// ---------------------------------------------------------------------------
// Brand check
// ---------------------------------------------------------------------------

fn require_resize_observer_receiver(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &'static str,
) -> Result<ResizeObserverId, VmError> {
    let raw = super::observer_common::require_observer_receiver(
        ctx,
        this,
        super::super::value::ObserverKind::Resize,
        method,
    )?;
    Ok(ResizeObserverId::from_raw(raw))
}

// ---------------------------------------------------------------------------
// Constructor
// ---------------------------------------------------------------------------

fn native_resize_observer_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let callback_id = match args.first().copied() {
        Some(JsValue::Object(id)) if ctx.vm.get_object(id).kind.is_callable() => id,
        _ => {
            return Err(VmError::type_error(
                "Failed to construct 'ResizeObserver': parameter 1 is not of type 'Function'.",
            ));
        }
    };
    let JsValue::Object(this_id) = this else {
        unreachable!("constructor `this` is always an Object after `do_new`");
    };

    if ctx.host_opt().is_none() {
        return Err(VmError::type_error(
            "Failed to construct 'ResizeObserver': host environment is not initialised",
        ));
    }
    let observer_id = ctx.host().resize_observers.register().raw();
    ctx.vm.get_object_mut(this_id).kind = ObjectKind::Observer {
        kind: super::super::value::ObserverKind::Resize,
        observer_id,
    };
    ctx.host().resize_observer_bindings.insert(
        observer_id,
        super::observer_common::ObserverBinding {
            callback: callback_id,
            instance: this_id,
        },
    );

    Ok(JsValue::Object(this_id))
}

// ---------------------------------------------------------------------------
// Natives
// ---------------------------------------------------------------------------

fn native_resize_observer_observe(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_resize_observer_receiver(ctx, this, "observe")?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    let target = require_target_node(ctx, args.first().copied(), "observe")?;
    let options = parse_resize_observer_options(ctx, args.get(1).copied())?;
    let (dom, observers) = ctx.host().split_dom_mut_and_resize_observers();
    observers.observe(dom, id, target, options);
    Ok(JsValue::Undefined)
}

fn native_resize_observer_unobserve(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_resize_observer_receiver(ctx, this, "unobserve")?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    let target = require_target_node(ctx, args.first().copied(), "unobserve")?;
    let (dom, observers) = ctx.host().split_dom_mut_and_resize_observers();
    observers.unobserve(dom, id, target);
    Ok(JsValue::Undefined)
}

fn native_resize_observer_disconnect(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_resize_observer_receiver(ctx, this, "disconnect")?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    let (dom, observers) = ctx.host().split_dom_mut_and_resize_observers();
    observers.disconnect(dom, id);
    // `disconnect()` per W3C Resize Observer §3.5 only clears
    // observation targets; the observer stays usable so a subsequent
    // `ro.observe(other)` works.  The callback / instance maps must
    // therefore NOT be removed here — doing so would let a later
    // delivery's `(callback, instance)` lookup return `None` and
    // silently drop the records.  Matches `MutationObserver::disconnect`.
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// Argument parsing
// ---------------------------------------------------------------------------

fn require_target_node(
    ctx: &mut NativeContext<'_>,
    arg: Option<JsValue>,
    method: &'static str,
) -> Result<elidex_ecs::Entity, VmError> {
    super::node_proto::require_node_arg_required(ctx, arg, "ResizeObserver", method)
}

/// Parse the `ResizeObserverOptions` dictionary (WebIDL §3.10.7).
/// `undefined` / `null` / missing → default (`{ box: "content-box" }`).
/// Object → read the `box` member as a `ResizeObserverBoxOptions` enum.
/// Unrecognised enum values throw `TypeError` per WebIDL enum semantics.
fn parse_resize_observer_options(
    ctx: &mut NativeContext<'_>,
    arg: Option<JsValue>,
) -> Result<ResizeObserverOptions, VmError> {
    let mut options = ResizeObserverOptions::default();
    let value = match arg {
        None | Some(JsValue::Undefined | JsValue::Null) => return Ok(options),
        Some(v) => v,
    };
    let JsValue::Object(opts_id) = value else {
        // Primitives would be ToObject-coerced per WebIDL §3.10.7.
        // Inherits the simplification scope already deferred by
        // `MutationObserver`'s `parse_mutation_observer_init` under
        // `#11-mutation-observer-extras` — the observer family
        // shares the simplification.
        return Err(VmError::type_error(
            "Failed to execute 'observe' on 'ResizeObserver': options is not an object",
        ));
    };
    let wk_box = ctx.vm.well_known.box_option_key;
    let raw = ctx.get_property_value(opts_id, PropertyKey::String(wk_box))?;
    if matches!(raw, JsValue::Undefined) {
        return Ok(options);
    }
    let sid = ctx.to_string_val(raw)?;
    let s = ctx.vm.strings.get_utf8(sid);
    options.box_model = ResizeObserverBoxOptions::from_webidl(&s).ok_or_else(|| {
        VmError::type_error(format!(
            "Failed to execute 'observe' on 'ResizeObserver': The provided value '{s}' \
             is not a valid enum value of type ResizeObserverBoxOptions."
        ))
    })?;
    Ok(options)
}

// ---------------------------------------------------------------------------
// Entry marshalling (used by `VmInner::deliver_resize_observations`)
// ---------------------------------------------------------------------------

/// Marshal one `ResizeObserverEntry` (W3C Resize Observer §2.3) to a
/// JS Object with `target`, `contentRect` (DOMRectReadOnly), and the
/// `contentBoxSize` / `borderBoxSize` `FrozenArray<ResizeObserverSize>`
/// pair.  `devicePixelContentBoxSize` is deferred to
/// `#11-resize-observer-device-pixel-box`.
fn resize_entry_to_js(
    vm: &mut VmInner,
    entry: &elidex_api_observers::resize::ResizeObserverEntry,
) -> JsValue {
    use super::super::shape::PropertyAttrs;

    let target_val = JsValue::Object(vm.create_element_wrapper(entry.target));
    let mut target_guard = vm.push_temp_root(target_val);
    let content_rect = target_guard.build_dom_rect_readonly(
        f64::from(entry.content_rect.origin.x),
        f64::from(entry.content_rect.origin.y),
        f64::from(entry.content_rect.size.width),
        f64::from(entry.content_rect.size.height),
    );
    let mut rect_guard = target_guard.push_temp_root(content_rect);
    let content_box_arr = build_resize_size_sequence(
        &mut rect_guard,
        entry.content_rect.size.width,
        entry.content_rect.size.height,
    );
    let mut content_box_guard = rect_guard.push_temp_root(content_box_arr);
    let border_box_arr = build_resize_size_sequence(
        &mut content_box_guard,
        entry.border_box_size.width,
        entry.border_box_size.height,
    );
    let mut border_box_guard = content_box_guard.push_temp_root(border_box_arr);

    let object_proto = border_box_guard.object_prototype;
    let entry_obj = border_box_guard.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        storage: PropertyStorage::shaped(super::super::shape::ROOT_SHAPE),
        prototype: object_proto,
        extensible: true,
    });

    let wk_target = border_box_guard.well_known.target;
    let wk_content_rect = border_box_guard.well_known.content_rect;
    let wk_content_box_size = border_box_guard.well_known.content_box_size;
    let wk_border_box_size = border_box_guard.well_known.border_box_size;
    for (key_sid, value) in [
        (wk_target, target_val),
        (wk_content_rect, content_rect),
        (wk_content_box_size, content_box_arr),
        (wk_border_box_size, border_box_arr),
    ] {
        border_box_guard.define_shaped_property(
            entry_obj,
            PropertyKey::String(key_sid),
            PropertyValue::Data(value),
            PropertyAttrs::WEBIDL_RO,
        );
    }

    drop(border_box_guard);
    drop(content_box_guard);
    drop(rect_guard);
    drop(target_guard);
    JsValue::Object(entry_obj)
}

// ---------------------------------------------------------------------------
// Embedder API — `Vm::deliver_resize_observations` core logic
// ---------------------------------------------------------------------------

impl VmInner {
    /// See [`super::super::Vm::deliver_resize_observations`] for the
    /// documented semantics.  Implementation lives here next to
    /// `resize_entry_to_js`.
    pub(crate) fn deliver_resize_observations(&mut self) {
        // Silent no-op post-unbind so a stray late delivery from the
        // shell does not panic via `host_data.dom()`.  No
        // bindings-empty fast path: per
        // `observer_common::deliver_to_observer_callbacks`'s
        // contract, the trailing microtask drain runs even when the
        // observer_ids slice is empty (HTML §8.1.7.3 — each broadcast
        // is its own microtask checkpoint).  Matches
        // `deliver_mutation_records`.
        if !self
            .host_data
            .as_deref()
            .is_some_and(super::super::host_data::HostData::is_bound)
        {
            return;
        }

        // Phase A: gather observations under a disjoint
        // `(&mut EcsDom, &mut Registry)` split borrow.  The closure
        // reads `LayoutBox` via a shared `&EcsDom` re-projected from
        // the `&mut EcsDom` inside `gather_observations`; the crate
        // manages the borrow life cycle internally.  Layering: the
        // closure only reads layout; box-less targets return `None`
        // and the crate runs the initial-observation step.  Use a
        // `BTreeMap` so iteration follows the crate-side id-sorted
        // order — `gather_observations_is_id_sorted` (crate test)
        // pins the contract; a `HashMap` here would silently break
        // it via non-deterministic `keys()` iteration.
        let observations: std::collections::BTreeMap<u64, _> = {
            let host = self
                .host_data
                .as_deref_mut()
                .expect("deliver_resize_observations: HostData required when bound");
            let (dom, observers) = host.split_dom_mut_and_resize_observers();
            observers
                .gather_observations(dom, &|d, entity| {
                    let lb = d.world().get::<&elidex_plugin::LayoutBox>(entity).ok()?;
                    Some((lb.content_rect_local(), lb.border_box().size))
                })
                .into_iter()
                .map(|(id, entries)| (id.raw(), entries))
                .collect()
        };
        let observer_ids: Vec<u64> = observations.keys().copied().collect();

        deliver_to_observer_callbacks(self, &observer_ids, |vm, id| {
            let entries = observations.get(&id)?;
            let binding = vm
                .host_data
                .as_deref()?
                .resize_observer_bindings
                .get(&id)
                .copied()?;
            let entries_arr = build_marshalled_array(vm, entries, resize_entry_to_js);
            Some((binding, entries_arr))
        });
    }
}

/// Build a single-element JS Array containing one `ResizeObserverSize`
/// object (W3C Resize Observer §4.2).  The spec defines `contentBoxSize`
/// / `borderBoxSize` as `FrozenArray<ResizeObserverSize>`; a fragmented
/// layout box could produce multiple entries, but the current engine
/// only tracks a single content box per element so the sequence has
/// exactly one member.
///
/// inlineSize / blockSize follow the writing-mode mapping (W3C Resize
/// Observer §4.2): for the default horizontal writing mode,
/// `inlineSize` = width, `blockSize` = height.  Writing-mode-aware
/// inversion is tracked along with `devicePixelContentBoxSize` under
/// `#11-resize-observer-device-pixel-box`.
fn build_resize_size_sequence(vm: &mut VmInner, width: f32, height: f32) -> JsValue {
    use super::super::shape::PropertyAttrs;

    let object_proto = vm.object_prototype;
    let size_obj = vm.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        storage: PropertyStorage::shaped(super::super::shape::ROOT_SHAPE),
        prototype: object_proto,
        extensible: true,
    });
    // Root the size object across the trailing `create_array_object`
    // allocation; the per-property writes themselves cannot trigger GC.
    let mut size_guard = vm.push_temp_root(JsValue::Object(size_obj));
    let wk_inline = size_guard.well_known.inline_size;
    let wk_block = size_guard.well_known.block_size;
    for (key_sid, value) in [(wk_inline, f64::from(width)), (wk_block, f64::from(height))] {
        size_guard.define_shaped_property(
            size_obj,
            PropertyKey::String(key_sid),
            PropertyValue::Data(JsValue::Number(value)),
            PropertyAttrs::WEBIDL_RO,
        );
    }
    let arr = size_guard.create_array_object(vec![JsValue::Object(size_obj)]);
    drop(size_guard);
    JsValue::Object(arr)
}
