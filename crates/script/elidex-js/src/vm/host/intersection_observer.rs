//! `IntersectionObserver` interface (W3C Intersection Observer §2.2) —
//! VM thin binding to the engine-independent
//! [`elidex_api_observers::intersection::IntersectionObserverRegistry`].
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", this file contains only the
//! engine-bound responsibilities: prototype install, brand check,
//! `IntersectionObserverInit` dictionary parsing (JS → Rust
//! marshalling), one-line dispatch into the registry helpers, and
//! per-entry marshalling at delivery time.  The actual observation
//! algorithm — target tracking, root rect lookup, `rootMargin` parsing
//! and application, ratio computation, threshold-crossing detection,
//! frame-broadcast gathering — lives in
//! [`elidex_api_observers::intersection`].
//!
//! ## State storage
//!
//! Observer state is split between two side tables:
//!
//! - [`super::super::host_data::HostData::intersection_observers`] —
//!   the
//!   [`elidex_api_observers::intersection::IntersectionObserverRegistry`]
//!   that owns the monotonic observer ID counter plus the per-observer
//!   `IntersectionObserverInit` (root / rootMargin / thresholds).
//!   Target tracking lives as per-entity `IntersectionObservedBy`
//!   components, not in the registry.
//! - [`super::super::host_data::HostData::intersection_observer_bindings`]
//!   — `HashMap<u64, ObserverBinding>` from observer ID to the
//!   `(callback, instance)` JS-identity pair.  Both `ObjectId`s in
//!   each binding are rooted via
//!   [`super::super::host_data::HostData::gc_root_object_ids`] so the
//!   callback + instance survive GC for the observer's lifetime.
//!
//! [`super::super::value::ObjectKind::Observer`] with
//! [`super::super::value::ObserverKind::Intersection`] carries the
//! observer ID inline (`observer_id: u64`); the JS object itself
//! has no other own state.
//!
//! ## Per-frame `dom_rect_states` growth
//!
//! Each delivered entry mints up to **three** `DOMRectReadOnly`
//! instances via [`super::super::VmInner::build_dom_rect_readonly`]:
//! `boundingClientRect`, `intersectionRect`, and (for same-origin
//! roots) `rootBounds`.  Each mint inserts a row into
//! [`super::super::VmInner::dom_rect_states`].  Steady-state delivery
//! without an intervening GC cycle therefore grows that map linearly
//! in the count of delivered entries — bounded in practice by the GC
//! sweep tail at `gc/collect.rs::collect_garbage` which prunes rows
//! whose key `ObjectId` was collected.  Frame-tick allocation pressure
//! normally triggers periodic GC, so a long-running page with many
//! intersection callbacks does not accumulate unboundedly.  A pooled
//! / per-callback-arena reuse strategy is tracked as a possible
//! follow-up if real-world profiles surface this.
//!
//! ## `native_*` docstring convention
//!
//! Per-method native `fn`s in this module (and the sibling
//! `mutation_observer.rs` / `resize_observer.rs`) deliberately rely
//! on the constructor's docstring + the module-level spec citation
//! as their primary documentation.  Their brand-checked name
//! (`native_intersection_observer_observe` etc.) is unique enough to
//! disambiguate without a per-fn docstring; the spec section is
//! cited inline at the call site where it actually matters.  This
//! mirrors the convention already in place across the M4-12 boa→VM
//! port surfaces (e.g. `vm/host/mutation_observer.rs`).

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectKind, PropertyKey, PropertyStorage, PropertyValue,
    VmError,
};
use super::super::{NativeFn, VmInner};

use elidex_api_observers::intersection::{IntersectionObserverId, IntersectionObserverInit};

use super::super::webidl_sequence::webidl_iter_to_vec;
use super::observer_common::{build_marshalled_array, deliver_to_observer_callbacks};

impl VmInner {
    /// Allocate `IntersectionObserver.prototype` chained to
    /// `Object.prototype`, install its four method natives, and expose
    /// the `IntersectionObserver` constructor on `globalThis`.
    ///
    /// # Panics
    ///
    /// Panics if `object_prototype` is `None`.
    pub(in crate::vm) fn register_intersection_observer_global(&mut self) {
        let object_proto = self
            .object_prototype
            .expect("register_intersection_observer_global called before register_prototypes");

        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });

        let wk = &self.well_known;
        let entries = [
            (wk.observe, native_intersection_observer_observe as NativeFn),
            (
                wk.unobserve,
                native_intersection_observer_unobserve as NativeFn,
            ),
            (
                wk.disconnect,
                native_intersection_observer_disconnect as NativeFn,
            ),
            (
                wk.take_records,
                native_intersection_observer_take_records as NativeFn,
            ),
        ];
        for (name_sid, func) in entries {
            self.install_native_method(proto_id, name_sid, func, shape::PropertyAttrs::METHOD);
        }
        self.intersection_observer_prototype = Some(proto_id);

        let ctor = self.create_constructable_function(
            "IntersectionObserver",
            native_intersection_observer_constructor,
        );
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
        let name_sid = self.well_known.intersection_observer_global;
        self.globals.insert(name_sid, JsValue::Object(ctor));
    }
}

// ---------------------------------------------------------------------------
// Brand check
// ---------------------------------------------------------------------------

fn require_intersection_observer_receiver(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &'static str,
) -> Result<IntersectionObserverId, VmError> {
    let raw = super::observer_common::require_observer_receiver(
        ctx,
        this,
        super::super::value::ObserverKind::Intersection,
        method,
    )?;
    Ok(IntersectionObserverId::from_raw(raw))
}

// ---------------------------------------------------------------------------
// Constructor
// ---------------------------------------------------------------------------

fn native_intersection_observer_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if !ctx.is_construct() {
        return Err(VmError::type_error(
            "Failed to construct 'IntersectionObserver': Please use the 'new' operator",
        ));
    }
    let callback_id = match args.first().copied() {
        Some(JsValue::Object(id)) if ctx.vm.get_object(id).kind.is_callable() => id,
        _ => {
            return Err(VmError::type_error(
                "Failed to construct 'IntersectionObserver': parameter 1 is not of type 'Function'.",
            ));
        }
    };
    let JsValue::Object(this_id) = this else {
        unreachable!("constructor `this` is always an Object after `do_new`");
    };
    // Match sibling `MutationObserver` / `ResizeObserver` ctors — only
    // `HostData` installation is required at construct time, not a bound
    // DOM.  A non-null `root` option further requires bound state to
    // validate the entity, but that is handled by `require_node_arg`
    // (returns TypeError when unbound, no panic).
    if ctx.host_opt().is_none() {
        return Err(VmError::type_error(
            "Failed to construct 'IntersectionObserver': host environment is not initialised",
        ));
    }

    let init = parse_intersection_observer_init(ctx, args.get(1).copied())?;
    // W3C Intersection Observer §2.2 ctor step — `SyntaxError` if
    // `rootMargin` is not a valid `<length-percentage>{1,4}`.  The
    // crate-side `register` returns `RootMarginParseError` (engine-
    // independent); the host wraps it in an interface-scoped
    // SyntaxError matching the Chrome / Firefox shape.
    let io_id = match ctx.host().intersection_observers.register(init) {
        Ok(id) => id.raw(),
        Err(err) => {
            return Err(VmError::syntax_error(format!(
                "Failed to construct 'IntersectionObserver': {err}"
            )));
        }
    };
    ctx.vm.get_object_mut(this_id).kind = ObjectKind::Observer {
        kind: super::super::value::ObserverKind::Intersection,
        observer_id: io_id,
    };
    ctx.host().intersection_observer_bindings.insert(
        io_id,
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

fn native_intersection_observer_observe(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_intersection_observer_receiver(ctx, this, "observe")?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    let target = require_target_node(ctx, args.first().copied(), "observe")?;
    let (dom, observers) = ctx.host().split_dom_mut_and_intersection_observers();
    observers.observe(dom, id, target);
    Ok(JsValue::Undefined)
}

fn native_intersection_observer_unobserve(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_intersection_observer_receiver(ctx, this, "unobserve")?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    let target = require_target_node(ctx, args.first().copied(), "unobserve")?;
    let (dom, observers) = ctx.host().split_dom_mut_and_intersection_observers();
    observers.unobserve(dom, id, target);
    Ok(JsValue::Undefined)
}

fn native_intersection_observer_disconnect(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_intersection_observer_receiver(ctx, this, "disconnect")?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    let (dom, observers) = ctx.host().split_dom_mut_and_intersection_observers();
    observers.disconnect(dom, id);
    // `disconnect()` per W3C Intersection Observer §3.3 only stops
    // observing all targets; the observer stays usable.  Callback /
    // instance maps are NOT removed — same rationale as
    // `ResizeObserver::disconnect`.
    Ok(JsValue::Undefined)
}

/// `IntersectionObserver.takeRecords()` (W3C Intersection Observer
/// §3.3) — returns the queued entries and clears the queue.  This VM
/// does not buffer entries between frames (per-frame delivery in
/// [`super::super::Vm::deliver_intersection_observations`] consumes
/// every observation directly), so the queue is always empty and this
/// returns a fresh empty array.  Spec-compliant: the algorithm is
/// "let queue be a copy of this's queued entries", "clear this's
/// queued entries", "return queue" — both reads of an empty queue are
/// well-defined.
fn native_intersection_observer_take_records(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let _id = require_intersection_observer_receiver(ctx, this, "takeRecords")?;
    Ok(JsValue::Object(ctx.vm.create_array_object(Vec::new())))
}

// ---------------------------------------------------------------------------
// Argument parsing
// ---------------------------------------------------------------------------

fn require_target_node(
    ctx: &mut NativeContext<'_>,
    arg: Option<JsValue>,
    method: &'static str,
) -> Result<elidex_ecs::Entity, VmError> {
    super::node_proto::require_node_arg_required(ctx, arg, "IntersectionObserver", method)
}

/// Parse the `IntersectionObserverInit` dictionary (W3C Intersection
/// Observer §3.1).  `undefined` / `null` / missing → default init
/// (viewport root, "0px" rootMargin, threshold `[0]`).  The
/// `rootMargin` CSS shorthand is passed through as a raw string —
/// parsing into `MarginComponent`s and applying it to the root rect
/// happens crate-side inside `gather_observations` (Layering mandate:
/// the algorithm is the crate's, the host only marshals the string).
fn parse_intersection_observer_init(
    ctx: &mut NativeContext<'_>,
    arg: Option<JsValue>,
) -> Result<IntersectionObserverInit, VmError> {
    let mut init = IntersectionObserverInit::default();
    let value = match arg {
        // No init → spec default applies.  `threshold = [0]`
        // (§3.1: "If options.threshold is not present, set it to [0]")
        // is canonicalised crate-side in
        // `IntersectionObserverRegistry::register`, so leaving the
        // default empty `Vec` here is intentional — registration is
        // the single canonicalisation point.
        None | Some(JsValue::Undefined | JsValue::Null) => return Ok(init),
        Some(v) => v,
    };
    let JsValue::Object(opts_id) = value else {
        return Err(VmError::type_error(
            "Failed to construct 'IntersectionObserver': options is not an object",
        ));
    };

    let wk_root = ctx.vm.well_known.root_option_key;
    let wk_root_margin = ctx.vm.well_known.root_margin;
    let wk_threshold = ctx.vm.well_known.threshold;

    // `root` — Element | Document | null | undefined.  null/undefined
    // → viewport; an Element/Document arg is marshalled to its Entity.
    // The wrong-type error is re-scoped to the constructor's "Failed
    // to construct 'IntersectionObserver'" shape (Chrome parity);
    // tightening the type to `Element or Document` (vs any Node) is
    // tracked under the IO spec-correctness defer slots.
    let raw_root = ctx.get_property_value(opts_id, PropertyKey::String(wk_root))?;
    init.root = match raw_root {
        JsValue::Undefined | JsValue::Null => None,
        v => Some(
            super::node_proto::require_node_arg(ctx, v, "root").map_err(|_| {
                VmError::type_error(
                    "Failed to construct 'IntersectionObserver': member 'root' is not of type 'Node'.",
                )
            })?,
        ),
    };

    let raw_root_margin = ctx.get_property_value(opts_id, PropertyKey::String(wk_root_margin))?;
    if !matches!(raw_root_margin, JsValue::Undefined) {
        let sid = ctx.to_string_val(raw_root_margin)?;
        init.root_margin = ctx.vm.strings.get_utf8(sid);
    }

    let raw_threshold = ctx.get_property_value(opts_id, PropertyKey::String(wk_threshold))?;
    init.threshold = parse_threshold(ctx, raw_threshold)?;
    // Spec §3.1: thresholds must be sorted ascending + deduplicated +
    // each in [0,1].  Sort+dedup here; range validation per-value
    // happens in `parse_threshold`.  Empty-list canonicalisation to
    // `[0]` lives in `IntersectionObserverRegistry::register`.
    init.threshold
        .sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    init.threshold.dedup();

    Ok(init)
}

/// Parse the `threshold` init member (W3C Intersection Observer §2.4).
/// WebIDL `(double or sequence<double>)`: probe `@@iterator` to pick
/// the sequence branch (Array literal, NodeList, custom iterable);
/// a non-iterable value coerces to a single double via `ToNumber`.
/// Each value must be finite and in `[0, 1]` (RangeError otherwise).
fn parse_threshold(ctx: &mut NativeContext<'_>, raw: JsValue) -> Result<Vec<f64>, VmError> {
    if matches!(raw, JsValue::Undefined) {
        return Ok(Vec::new());
    }
    // WebIDL §3.10.25 union resolution for `(double or sequence<double>)`:
    // probe `@@iterator` only on **Objects**.  Primitives — including
    // `String`, which has an `@@iterator` of its own (code-point
    // iteration) — must coerce directly to the `double` branch.
    // Without the `JsValue::Object` gate, `threshold: '0.5'` would
    // iterate `'.'` / `'5'` / `'0'` as code points and ToNumber-NaN
    // the first one into a RangeError.  `null` falls through to the
    // primitive coercion path as `0` per ES `ToNumber`.
    if matches!(raw, JsValue::Object(_)) {
        if let Some(iter @ JsValue::Object(_)) = ctx.vm.resolve_iterator(raw)? {
            // Sequence branch — drain via the shared WebIDL §3.10.16
            // helper so `Array.prototype[@@iterator]` overrides and
            // other iterables are honoured uniformly.  Cap is a
            // safety bound on pathological iterators; real consumers
            // rarely exceed a few thresholds.
            return webidl_iter_to_vec(
                ctx,
                iter,
                65_536,
                "Failed to construct 'IntersectionObserver': \
                 'threshold' length exceeds the supported maximum",
                |ctx, _idx, v| {
                    let n = ctx.to_number(v)?;
                    validate_threshold(n)?;
                    Ok(n)
                },
            );
        }
    }
    // Primitive value (or Object without `@@iterator`) → single
    // double coercion.
    let n = ctx.to_number(raw)?;
    validate_threshold(n)?;
    Ok(vec![n])
}

fn validate_threshold(n: f64) -> Result<(), VmError> {
    if !n.is_finite() || !(0.0..=1.0).contains(&n) {
        return Err(VmError::range_error(
            "Failed to construct 'IntersectionObserver': \
             'threshold' values must be finite numbers in [0, 1]",
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Entry marshalling (used by `VmInner::deliver_intersection_observations`)
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Embedder API — `Vm::deliver_intersection_observations` core logic
// ---------------------------------------------------------------------------

impl VmInner {
    /// See [`super::super::Vm::deliver_intersection_observations`] for
    /// the documented semantics.  Implementation lives here next to
    /// `intersection_entry_to_js`.
    pub(crate) fn deliver_intersection_observations(&mut self) {
        // Silent no-op post-unbind so a stray late delivery from the
        // shell does not panic via `host_data.dom()`.  No
        // bindings-empty fast path: see `deliver_resize_observations`
        // for the rationale (microtask-drain parity with MO under
        // `observer_common::deliver_to_observer_callbacks`'s
        // unconditional-drain contract).
        if !self
            .host_data
            .as_deref()
            .is_some_and(super::super::host_data::HostData::is_bound)
        {
            return;
        }
        // `time` follows the `Performance.now()` getter shape — see
        // `vm/host/performance.rs`.  Sourced here (rather than passed
        // in) because there is one canonical broadcast timestamp per
        // call and the embedder cannot intercept it meaningfully.
        let time = self.start_instant.elapsed().as_secs_f64() * 1000.0;
        // Viewport composes from the canonical window-state slots
        // (`window.innerWidth` / `innerHeight` / `scrollX` / `scrollY`)
        // the shell already maintains — there is one true viewport
        // per VM at delivery time, so passing it as an arg would be
        // redundant state.
        let v = &self.viewport;
        #[allow(clippy::cast_possible_truncation)] // viewport metrics fit in f32
        let viewport = elidex_plugin::Rect::new(
            v.scroll_x as f32,
            v.scroll_y as f32,
            v.inner_width as f32,
            v.inner_height as f32,
        );

        // `BTreeMap` (not `HashMap`) — preserves the crate-side
        // id-sorted iteration order pinned by
        // `gather_observations_is_id_sorted`.
        let observations: std::collections::BTreeMap<u64, _> = {
            let host = self
                .host_data
                .as_deref_mut()
                .expect("deliver_intersection_observations: HostData required when bound");
            let (dom, observers) = host.split_dom_mut_and_intersection_observers();
            observers
                .gather_observations(
                    dom,
                    &|d, entity| {
                        let lb = d.world().get::<&elidex_plugin::LayoutBox>(entity).ok()?;
                        Some(lb.border_box())
                    },
                    viewport,
                )
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
                .intersection_observer_bindings
                .get(&id)
                .copied()?;
            let entries_arr =
                build_marshalled_array(vm, entries, |vm, e| intersection_entry_to_js(vm, e, time));
            Some((binding, entries_arr))
        });
    }
}

/// Marshal one [`elidex_api_observers::intersection::IntersectionObserverEntry`]
/// (W3C Intersection Observer §3.4 `IntersectionObserverEntry`) to a JS Object
/// with `target`, `time`, `boundingClientRect`, `intersectionRect`,
/// `rootBounds`, `intersectionRatio`, and `isIntersecting` members.  Mirrors
/// the per-record temp-root discipline of `mutation_record_to_js`: the
/// element wrapper + DOMRectReadOnly trio + the partially-filled entry are
/// each rooted across the next allocation so a GC triggered mid-build cannot
/// collect them.
fn intersection_entry_to_js(
    vm: &mut VmInner,
    entry: &elidex_api_observers::intersection::IntersectionObserverEntry,
    time: f64,
) -> JsValue {
    use super::super::shape::PropertyAttrs;

    let target_val = JsValue::Object(vm.create_element_wrapper(entry.target));
    let mut target_guard = vm.push_temp_root(target_val);

    let bcr = target_guard.build_dom_rect_readonly(
        f64::from(entry.bounding_client_rect.origin.x),
        f64::from(entry.bounding_client_rect.origin.y),
        f64::from(entry.bounding_client_rect.size.width),
        f64::from(entry.bounding_client_rect.size.height),
    );
    let mut bcr_guard = target_guard.push_temp_root(bcr);

    let irect = bcr_guard.build_dom_rect_readonly(
        f64::from(entry.intersection_rect.origin.x),
        f64::from(entry.intersection_rect.origin.y),
        f64::from(entry.intersection_rect.size.width),
        f64::from(entry.intersection_rect.size.height),
    );
    let mut irect_guard = bcr_guard.push_temp_root(irect);

    let root_bounds = match entry.root_bounds {
        Some(rb) => irect_guard.build_dom_rect_readonly(
            f64::from(rb.origin.x),
            f64::from(rb.origin.y),
            f64::from(rb.size.width),
            f64::from(rb.size.height),
        ),
        // Cross-origin implicit root reports null per §3.4 — that path
        // is deferred (`#11-intersection-observer-cross-origin-rootbounds`),
        // so same-origin always produces `Some` from the crate side.
        None => JsValue::Null,
    };
    let mut rb_guard = irect_guard.push_temp_root(root_bounds);

    let object_proto = rb_guard.object_prototype;
    let entry_obj = rb_guard.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        storage: PropertyStorage::shaped(super::super::shape::ROOT_SHAPE),
        prototype: object_proto,
        extensible: true,
    });

    let wk_target = rb_guard.well_known.target;
    let wk_time = rb_guard.well_known.time;
    let wk_bcr = rb_guard.well_known.bounding_client_rect;
    let wk_irect = rb_guard.well_known.intersection_rect;
    let wk_rb = rb_guard.well_known.root_bounds;
    let wk_ratio = rb_guard.well_known.intersection_ratio;
    let wk_is_intersecting = rb_guard.well_known.is_intersecting;
    for (key_sid, value) in [
        (wk_target, target_val),
        (wk_time, JsValue::Number(time)),
        (wk_bcr, bcr),
        (wk_irect, irect),
        (wk_rb, root_bounds),
        (wk_ratio, JsValue::Number(entry.intersection_ratio)),
        (wk_is_intersecting, JsValue::Boolean(entry.is_intersecting)),
    ] {
        rb_guard.define_shaped_property(
            entry_obj,
            PropertyKey::String(key_sid),
            PropertyValue::Data(value),
            PropertyAttrs::WEBIDL_RO,
        );
    }

    drop(rb_guard);
    drop(irect_guard);
    drop(bcr_guard);
    drop(target_guard);
    JsValue::Object(entry_obj)
}
