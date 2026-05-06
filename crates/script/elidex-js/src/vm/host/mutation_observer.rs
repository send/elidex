//! `MutationObserver` interface (WHATWG DOM §4.3) — VM thin
//! binding to the engine-independent
//! [`elidex_api_observers::mutation::MutationObserverRegistry`].
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", this file contains only the
//! engine-bound responsibilities: prototype install, brand check,
//! `MutationObserverInit` dictionary parsing (JS → Rust marshalling),
//! and one-line dispatch into the registry helpers.  The actual
//! observation algorithm — target list management, subtree
//! ancestry checks, attribute-filter matching, record building —
//! lives in [`elidex_api_observers::mutation`].
//!
//! ## State storage
//!
//! Observer state is split between two side tables:
//!
//! - [`super::super::host_data::HostData::mutation_observers`] — the
//!   [`elidex_api_observers::mutation::MutationObserverRegistry`]
//!   that owns target lists, init options, and pending records.
//! - [`super::super::host_data::HostData::mutation_observer_callbacks`]
//!   — `HashMap<u64, ObjectId>` from observer ID to JS callback
//!   `ObjectId`.  Rooted via
//!   [`super::super::host_data::HostData::gc_root_object_ids`] so
//!   the callback survives GC for the observer's lifetime.
//!
//! [`super::super::value::ObjectKind::MutationObserver`] carries the
//! observer ID inline (`observer_id: u64`); the JS object itself
//! has no other own state.
//!
//! ## Post-unbind tolerance
//!
//! User code can retain a `MutationObserver` reference across
//! [`super::super::Vm::unbind`].  Each native checks
//! `ctx.host_if_bound()` first and returns a safe no-op
//! (`takeRecords` → empty array, `observe` / `disconnect` →
//! undefined).  Constructor calls are top-level and unreachable
//! while unbound (no JS executes).

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, VmError,
};
use super::super::{NativeFn, VmInner};

use elidex_api_observers::mutation::MutationObserverId;

impl VmInner {
    /// Allocate `MutationObserver.prototype` chained to
    /// `Object.prototype`, install its three method natives, and
    /// expose the `MutationObserver` constructor on `globalThis`.
    ///
    /// Called from `register_globals()` after `register_prototypes`
    /// (which populates `object_prototype`).
    ///
    /// # Panics
    ///
    /// Panics if `object_prototype` is `None` — would mean
    /// `register_prototypes` was skipped or the wrong order.
    pub(in crate::vm) fn register_mutation_observer_global(&mut self) {
        let object_proto = self
            .object_prototype
            .expect("register_mutation_observer_global called before register_prototypes");

        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });

        let wk = &self.well_known;
        let entries = [
            (wk.observe, native_mutation_observer_observe as NativeFn),
            (
                wk.disconnect,
                native_mutation_observer_disconnect as NativeFn,
            ),
            (
                wk.take_records,
                native_mutation_observer_take_records as NativeFn,
            ),
        ];
        for (name_sid, func) in entries {
            self.install_native_method(proto_id, name_sid, func, shape::PropertyAttrs::METHOD);
        }
        self.mutation_observer_prototype = Some(proto_id);

        let ctor = self.create_constructable_function(
            "MutationObserver",
            native_mutation_observer_constructor,
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
        let name_sid = self.well_known.mutation_observer_global;
        self.globals.insert(name_sid, JsValue::Object(ctor));
    }
}

// ---------------------------------------------------------------------------
// Brand check
// ---------------------------------------------------------------------------

/// Recover the `MutationObserverId` from `this`, returning a TypeError
/// if `this` is not a `MutationObserver` instance.
fn require_mutation_observer_receiver(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &'static str,
) -> Result<MutationObserverId, VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'MutationObserver': Illegal invocation"
        )));
    };
    let ObjectKind::MutationObserver { observer_id } = ctx.vm.get_object(id).kind else {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'MutationObserver': Illegal invocation"
        )));
    };
    Ok(MutationObserverId::from_raw(observer_id))
}

// ---------------------------------------------------------------------------
// Constructor
// ---------------------------------------------------------------------------

/// `new MutationObserver(callback)` (WHATWG DOM §4.3.2).
fn native_mutation_observer_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if !ctx.is_construct() {
        return Err(VmError::type_error(
            "Failed to construct 'MutationObserver': Please use the 'new' operator",
        ));
    }
    // Validate the callback up front — must be callable per WebIDL
    // §3.10.2 (MutationCallback).
    let callback_id = match args.first().copied() {
        Some(JsValue::Object(id)) if ctx.vm.get_object(id).kind.is_callable() => id,
        _ => {
            return Err(VmError::type_error(
                "Failed to construct 'MutationObserver': parameter 1 is not of type 'Function'.",
            ));
        }
    };
    let JsValue::Object(this_id) = this else {
        unreachable!("constructor `this` is always an Object after `do_new`");
    };

    // Allocate an observer ID in the registry, then promote the
    // pre-allocated Ordinary instance to `MutationObserver` so the
    // `new.target.prototype` chain installed by `do_new` is preserved
    // (URL/URLSearchParams precedent — PR5a2 R7.2/R7.3 lesson).
    let observer_id = ctx.host().mutation_observers.register().raw();
    ctx.vm.get_object_mut(this_id).kind = ObjectKind::MutationObserver { observer_id };
    let host = ctx.host();
    host.mutation_observer_callbacks
        .insert(observer_id, callback_id);
    host.mutation_observer_instances
        .insert(observer_id, this_id);

    Ok(JsValue::Object(this_id))
}

// ---------------------------------------------------------------------------
// Natives
// ---------------------------------------------------------------------------

fn native_mutation_observer_observe(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_mutation_observer_receiver(ctx, this, "observe")?;
    let target = require_target_node(ctx, args.first().copied(), "observe")?;
    let init = parse_mutation_observer_init(ctx, args.get(1).copied())?;
    if !init.child_list && !init.attributes && !init.character_data {
        return Err(VmError::type_error(
            "Failed to execute 'observe' on 'MutationObserver': The options object must \
             set at least one of 'attributes', 'characterData', or 'childList' to true.",
        ));
    }
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    ctx.host().mutation_observers.observe(id, target, init);
    Ok(JsValue::Undefined)
}

fn native_mutation_observer_disconnect(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_mutation_observer_receiver(ctx, this, "disconnect")?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    ctx.host().mutation_observers.disconnect(id);
    Ok(JsValue::Undefined)
}

fn native_mutation_observer_take_records(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_mutation_observer_receiver(ctx, this, "takeRecords")?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Object(ctx.vm.create_array_object(Vec::new())));
    }
    let records = ctx.host().mutation_observers.take_records(id);
    Ok(build_mutation_records_array(ctx.vm, &records))
}

/// Marshal a slice of [`elidex_api_observers::mutation::MutationRecord`]s
/// into a JS Array of MutationRecord objects (WHATWG DOM §4.3.5).
///
/// Each per-record allocation roots its outgoing addedNodes /
/// removedNodes / target wrapper sub-objects via `push_temp_root`
/// before allocating the wrapper Object so a GC triggered between
/// the array allocation and the property writes does not collect
/// the embedded element wrappers (Lesson R8).
pub(super) fn build_mutation_records_array(
    vm: &mut VmInner,
    records: &[elidex_api_observers::mutation::MutationRecord],
) -> JsValue {
    // Allocate the outer array up front so per-record allocations
    // can be appended directly.  Empty initial elements means the
    // outer array survives any per-record GC because it has no
    // intermediate "list-of-pending-objects" buffer that GC could
    // tear apart.
    let outer = vm.create_array_object(Vec::with_capacity(records.len()));
    let mut outer_guard = vm.push_temp_root(JsValue::Object(outer));
    for record in records {
        let record_value = mutation_record_to_js(&mut outer_guard, record);
        // Append into the outer array via direct slot push so the
        // ObjectId stays consistent across allocations (no `arr.push`
        // user-visible side effects, no recompute of length).
        if let ObjectKind::Array { ref mut elements } = outer_guard.get_object_mut(outer).kind {
            elements.push(record_value);
        }
    }
    drop(outer_guard);
    JsValue::Object(outer)
}

/// Marshal a single [`elidex_api_observers::mutation::MutationRecord`]
/// to a JS Object with WHATWG DOM §4.3.5 shape.
fn mutation_record_to_js(
    vm: &mut VmInner,
    record: &elidex_api_observers::mutation::MutationRecord,
) -> JsValue {
    use super::super::shape::PropertyAttrs;

    // Build the inner arrays first, rooting each so the subsequent
    // wrapper allocation (which itself can trigger GC) cannot
    // collect them.
    let added = build_node_array(vm, &record.added_nodes);
    let mut added_guard = vm.push_temp_root(added);
    let removed = build_node_array(&mut added_guard, &record.removed_nodes);
    let mut removed_guard = added_guard.push_temp_root(removed);

    // Marshal the optional sibling / attribute / oldValue fields —
    // none allocate, so no rooting needed.
    let target_id = removed_guard.create_element_wrapper(record.target);
    let target_val = JsValue::Object(target_id);
    let prev_sibling = match record.previous_sibling {
        Some(e) => JsValue::Object(removed_guard.create_element_wrapper(e)),
        None => JsValue::Null,
    };
    let next_sibling = match record.next_sibling {
        Some(e) => JsValue::Object(removed_guard.create_element_wrapper(e)),
        None => JsValue::Null,
    };
    let attribute_name = match &record.attribute_name {
        Some(name) => JsValue::String(removed_guard.strings.intern(name)),
        None => JsValue::Null,
    };
    let old_value = match &record.old_value {
        Some(v) => JsValue::String(removed_guard.strings.intern(v)),
        None => JsValue::Null,
    };
    let type_sid = removed_guard.strings.intern(record.mutation_type.as_str());
    let type_val = JsValue::String(type_sid);

    // Allocate the record wrapper now that every field value is in hand.
    let object_proto = removed_guard.object_prototype;
    let record_obj = removed_guard.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        storage: PropertyStorage::shaped(super::super::shape::ROOT_SHAPE),
        prototype: object_proto,
        extensible: true,
    });

    let wk_type = removed_guard.well_known.event_type;
    let wk_target = removed_guard.well_known.target;
    let wk_added = removed_guard.well_known.added_nodes;
    let wk_removed = removed_guard.well_known.removed_nodes;
    let wk_prev = removed_guard.well_known.previous_sibling;
    let wk_next = removed_guard.well_known.next_sibling;
    let wk_attr_name = removed_guard.well_known.attribute_name;
    let wk_old_value = removed_guard.well_known.old_value;
    for (key_sid, value) in [
        (wk_type, type_val),
        (wk_target, target_val),
        (wk_added, added),
        (wk_removed, removed),
        (wk_prev, prev_sibling),
        (wk_next, next_sibling),
        (wk_attr_name, attribute_name),
        (wk_old_value, old_value),
    ] {
        removed_guard.define_shaped_property(
            record_obj,
            PropertyKey::String(key_sid),
            PropertyValue::Data(value),
            PropertyAttrs::DATA,
        );
    }

    drop(removed_guard);
    drop(added_guard);
    JsValue::Object(record_obj)
}

/// Build a JS Array of element wrappers for the given entity slice.
/// Element wrappers go through [`super::elements::create_element_wrapper`]
/// so identity caching applies (`addedNodes[0] === document.body` for
/// matched targets).
fn build_node_array(vm: &mut VmInner, entities: &[elidex_ecs::Entity]) -> JsValue {
    let mut elements = Vec::with_capacity(entities.len());
    for &e in entities {
        let id = vm.create_element_wrapper(e);
        elements.push(JsValue::Object(id));
    }
    JsValue::Object(vm.create_array_object(elements))
}

// ---------------------------------------------------------------------------
// Embedder API — `Vm::deliver_mutation_records` core logic
// ---------------------------------------------------------------------------

impl VmInner {
    /// See [`super::super::Vm::deliver_mutation_records`] for the
    /// documented semantics.  Implementation lives here next to
    /// `mutation_record_to_js`.
    pub(crate) fn deliver_mutation_records(
        &mut self,
        records: &[elidex_script_session::MutationRecord],
    ) {
        // Step 1 — fan out session records to the registry.  Each
        // call resolves subtree matches via `EcsDom::get_parent`.
        // The `Some(host)` guard makes the embedder API silent
        // post-unbind (a stray late delivery from the shell does
        // not panic).
        if !self
            .host_data
            .as_deref()
            .is_some_and(super::super::host_data::HostData::is_bound)
        {
            return;
        }
        for record in records {
            self.notify_one(record);
        }

        // Step 2 — collect observer IDs that have pending records.
        // Done up front so re-entrant `mo.observe` / `mo.disconnect`
        // calls from callbacks see the post-callback registry state
        // rather than a snapshot taken mid-loop.
        let host = self
            .host_data
            .as_deref_mut()
            .expect("deliver_mutation_records: HostData required when bound");
        let observer_ids: Vec<u64> = host
            .mutation_observers
            .observers_with_records()
            .map(elidex_api_observers::mutation::MutationObserverId::raw)
            .collect();

        // Step 3 — for each observer with pending records, take +
        // marshal + invoke.  Re-borrows `host` per iteration so a
        // callback that mutates the registry is observed by the
        // next iteration's `take_records`.
        for observer_id in observer_ids {
            let mo_id = elidex_api_observers::mutation::MutationObserverId::from_raw(observer_id);
            let records = {
                let host = self
                    .host_data
                    .as_deref_mut()
                    .expect("deliver_mutation_records: HostData required when bound");
                host.mutation_observers.take_records(mo_id)
            };
            if records.is_empty() {
                continue;
            }
            let (callback_id, observer_obj_id) = {
                let host = self.host_data.as_deref().expect("bound earlier");
                (
                    host.mutation_observer_callbacks.get(&observer_id).copied(),
                    host.mutation_observer_instances.get(&observer_id).copied(),
                )
            };
            let Some(callback_id) = callback_id else {
                continue;
            };
            let Some(observer_obj_id) = observer_obj_id else {
                continue;
            };
            let observer_val = JsValue::Object(observer_obj_id);
            let mut observer_guard = self.push_temp_root(observer_val);
            let records_arr = build_mutation_records_array(&mut observer_guard, &records);
            let mut records_guard = observer_guard.push_temp_root(records_arr);
            let _ = records_guard
                .call(callback_id, observer_val, &[records_arr, observer_val])
                .map_err(|err| {
                    eprintln!("[JS MutationObserver Error] {err:?}");
                });
            drop(records_guard);
            drop(observer_guard);
        }

        // Step 4 — microtask checkpoint so any
        // `Promise.resolve().then(...)` chained inside a callback
        // fires before the embedder API returns (WHATWG §8.1.4.3).
        self.drain_microtasks();
    }

    /// Wrapper around `MutationObserverRegistry::notify` that
    /// supplies the session-level subtree-ancestry walker.
    fn notify_one(&mut self, record: &elidex_script_session::MutationRecord) {
        // Borrow split: `mutation_observers` (mutable) lives on
        // `host_data`, `dom_ptr` is read through `host_data.dom()`
        // — `dom_shared()` would alias with the `&mut` we obtain
        // through `dom()` because the closure body needs an
        // exclusive `&mut HostData` to reach the registry.
        // Workaround: clone-out the parent map for the closure
        // would explode for deep trees.  Instead: take the
        // `dom_ptr` raw pointer via an unsafe scope that the
        // existing `HostData` already exposes through
        // `with_session_and_dom`.
        //
        // Simplest sound path: capture `dom_shared()` first into a
        // raw pointer, then drop the borrow before re-borrowing
        // `host_data` mutably.  Both pointers were established by
        // the same `bind` call, and the closure does not mutate
        // the DOM — it only walks `get_parent`.
        let host = self
            .host_data
            .as_deref()
            .expect("notify_one: HostData required when bound");
        let dom_ref: *const elidex_ecs::EcsDom = host.dom_shared();
        let host_mut = self
            .host_data
            .as_deref_mut()
            .expect("notify_one: HostData required when bound");
        // SAFETY: `dom_ref` was obtained from the bound
        // `dom_ptr` via `dom_shared()`, which has the same lifetime
        // as the bind window.  We hold no other borrow on the DOM
        // during the closure (the callback is `Fn`), so creating a
        // shared ref through the raw pointer here aliases nothing
        // mutable.  `host_mut` mutates only the registry, which
        // lives in `HostData` and is disjoint from the `EcsDom`
        // allocation per `bind`'s "disjoint allocations"
        // contract.
        #[allow(unsafe_code)]
        let dom_ref: &elidex_ecs::EcsDom = unsafe { &*dom_ref };
        host_mut
            .mutation_observers
            .notify(record, &|target, ancestor| {
                let mut current = dom_ref.get_parent(target);
                while let Some(node) = current {
                    if node == ancestor {
                        return true;
                    }
                    current = dom_ref.get_parent(node);
                }
                false
            });
    }
}

// ---------------------------------------------------------------------------
// MutationObserverInit dictionary parsing + target extraction
// ---------------------------------------------------------------------------

fn require_target_node(
    ctx: &mut NativeContext<'_>,
    arg: Option<JsValue>,
    method: &'static str,
) -> Result<elidex_ecs::Entity, VmError> {
    let value = arg.ok_or_else(|| {
        VmError::type_error(format!(
            "Failed to execute '{method}' on 'MutationObserver': 1 argument required"
        ))
    })?;
    super::node_proto::require_node_arg(ctx, value, method)
}

/// Parse the `MutationObserverInit` dictionary (WHATWG DOM §4.3 +
/// §4.3.2 spec corrections) — JS object → Rust struct marshalling.
/// Returns the default-constructed init when `arg` is `undefined` /
/// missing (which subsequently fails the
/// "must set at least one" check at the call site).
fn parse_mutation_observer_init(
    ctx: &mut NativeContext<'_>,
    arg: Option<JsValue>,
) -> Result<elidex_api_observers::mutation::MutationObserverInit, VmError> {
    use elidex_api_observers::mutation::MutationObserverInit;

    let mut init = MutationObserverInit::default();
    let value = match arg {
        None | Some(JsValue::Undefined) => return Ok(init),
        Some(v) => v,
    };
    let JsValue::Object(opts_id) = value else {
        // WebIDL §3.10.7 dictionary conversion — for a non-object,
        // non-undefined input, ToObject would throw.  Match the
        // browser behaviour for null (TypeError) and primitives
        // (silent default-init) by treating only `undefined` as
        // "not provided" and everything else (`null`, primitives)
        // as a TypeError.
        return Err(VmError::type_error(
            "Failed to execute 'observe' on 'MutationObserver': options is not an object",
        ));
    };

    let wk_child_list = ctx.vm.well_known.child_list;
    let wk_attributes = ctx.vm.well_known.attributes;
    let wk_character_data = ctx.vm.well_known.character_data;
    let wk_subtree = ctx.vm.well_known.subtree;
    let wk_attribute_old_value = ctx.vm.well_known.attribute_old_value;
    let wk_character_data_old_value = ctx.vm.well_known.character_data_old_value;
    let wk_attribute_filter = ctx.vm.well_known.attribute_filter;
    let wk_length = ctx.vm.well_known.length;

    let mut attributes_explicit = false;
    let mut character_data_explicit = false;

    if let Some(v) = read_dict_field(ctx, opts_id, wk_child_list)? {
        init.child_list = ctx.to_boolean(v);
    }
    if let Some(v) = read_dict_field(ctx, opts_id, wk_attributes)? {
        init.attributes = ctx.to_boolean(v);
        attributes_explicit = true;
    }
    if let Some(v) = read_dict_field(ctx, opts_id, wk_character_data)? {
        init.character_data = ctx.to_boolean(v);
        character_data_explicit = true;
    }
    if let Some(v) = read_dict_field(ctx, opts_id, wk_subtree)? {
        init.subtree = ctx.to_boolean(v);
    }
    if let Some(v) = read_dict_field(ctx, opts_id, wk_attribute_old_value)? {
        init.attribute_old_value = ctx.to_boolean(v);
    }
    if let Some(v) = read_dict_field(ctx, opts_id, wk_character_data_old_value)? {
        init.character_data_old_value = ctx.to_boolean(v);
    }
    if let Some(JsValue::Object(arr_id)) = read_dict_field(ctx, opts_id, wk_attribute_filter)? {
        let len_val = ctx.get_property_value(arr_id, PropertyKey::String(wk_length))?;
        let len = ctx.to_number(len_val)?;
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let len_u32 = if len.is_finite() && len >= 0.0 {
            (len as u64).min(u64::from(u32::MAX)) as u32
        } else {
            0
        };
        let mut filter = Vec::with_capacity(len_u32 as usize);
        for i in 0..len_u32 {
            let item_key = PropertyKey::String(ctx.vm.strings.intern(&i.to_string()));
            let item = ctx.get_property_value(arr_id, item_key)?;
            let sid = ctx.to_string_val(item)?;
            filter.push(ctx.vm.strings.get_utf8(sid));
        }
        init.attribute_filter = Some(filter);
    }

    // WHATWG DOM §4.3.2 step 3: if `attributeOldValue` or
    // `attributeFilter` is given but `attributes` is not, set
    // `attributes` to true.
    if !attributes_explicit && (init.attribute_old_value || init.attribute_filter.is_some()) {
        init.attributes = true;
    }
    // §4.3.2 step 4: if `characterDataOldValue` is given but
    // `characterData` is not, set `characterData` to true.
    if !character_data_explicit && init.character_data_old_value {
        init.character_data = true;
    }

    Ok(init)
}

/// Look up an own / prototype-chain property by `StringId`,
/// returning `None` for `undefined` (per WebIDL dictionary semantics
/// — an `undefined` value means "not present", and the default
/// applies).  Other values pass through.
fn read_dict_field(
    ctx: &mut NativeContext<'_>,
    obj_id: ObjectId,
    name: super::super::value::StringId,
) -> Result<Option<JsValue>, VmError> {
    let v = ctx.get_property_value(obj_id, PropertyKey::String(name))?;
    if matches!(v, JsValue::Undefined) {
        Ok(None)
    } else {
        Ok(Some(v))
    }
}
