// boa skip: VM-only surface; the legacy boa runtime
// (`crates/script/elidex-js-boa/`) is on the deletion path per
// `m4-12-platform-gap-roadmap.md` §E-2 Round 20 PR7.  See
// `memory/project_boa_runtime_deletion.md`.

//! `TreeWalker` interface (WHATWG DOM §6.4) — VM thin binding to the
//! engine-independent
//! [`elidex_dom_api::traversal::step_with_filter_*`] direction-specific
//! filter walks.
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", this file performs only
//! prototype install, brand check, marshalling, and filter-callback
//! dispatch.  The actual §6.4 traversal algorithms (children /
//! siblings / parentNode / next-node / previous-node) live in
//! [`elidex_dom_api::traversal`].
//!
//! ## State storage
//!
//! - [`super::super::host_data::HostData::tree_walker_states`] —
//!   `HashMap<u64, TreeWalkerState>` keyed by walker_id.  TreeWalker
//!   is **not** registered with `MutationBridge` (WHATWG §6.4 has
//!   no pre-removing-steps).
//! - [`super::super::host_data::HostData::tree_walker_instances`] —
//!   reverse lookup table for GC sweep.
//!
//! [`super::super::value::ObjectKind::TreeWalker`] carries the
//! `walker_id: u64` inline; the wrapper has no other data.

#![cfg(feature = "engine")]
// Step closures dereference a stable `&EcsDom` snapshot via raw
// pointer so the `ctx.host().dom()` mutable borrow does not conflict
// with the simultaneous `JsFilter::ctx: &mut NativeContext`
// borrow.  Same SAFETY contract as `HostData::split_dom_and_observers`.
#![allow(unsafe_code)]

use elidex_dom_api::traversal::{
    step_with_filter, step_with_filter_first_child, step_with_filter_last_child,
    step_with_filter_next_sibling, step_with_filter_parent_node, step_with_filter_previous_node,
    step_with_filter_previous_sibling, FilterError, TreeWalker,
};
use elidex_ecs::Entity;

use super::super::shape;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, VmError,
};
use super::super::{NativeFn, VmInner};
use super::node_filter_dispatch::JsFilter;

impl VmInner {
    /// Allocate `TreeWalker.prototype` chained to `Object.prototype`
    /// and expose the (constructor-less, spec-mandated `[Exposed]`)
    /// `TreeWalker` constructor on `globalThis`.  Per spec the ctor
    /// is `[NoInterfaceObject]` legacy — modern WHATWG drops the
    /// legacy attribute and exposes the interface; browsers expose
    /// `globalThis.TreeWalker` though `new TreeWalker()` throws
    /// (illegal-invocation).  Implemented here as a `Function` whose
    /// `[[Construct]]` invariably throws.
    pub(in crate::vm) fn register_tree_walker_global(&mut self) {
        let object_proto = self
            .object_prototype
            .expect("register_tree_walker_global called before register_prototypes");

        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });

        let methods: [(_, NativeFn); 7] = [
            (
                self.well_known.parent_node_method,
                native_tree_walker_parent_node as NativeFn,
            ),
            (
                self.well_known.first_child_method,
                native_tree_walker_first_child,
            ),
            (
                self.well_known.last_child_method,
                native_tree_walker_last_child,
            ),
            (
                self.well_known.next_sibling_method,
                native_tree_walker_next_sibling,
            ),
            (
                self.well_known.previous_sibling_method,
                native_tree_walker_previous_sibling,
            ),
            (
                self.well_known.next_node_method,
                native_tree_walker_next_node,
            ),
            (
                self.well_known.previous_node_method,
                native_tree_walker_previous_node,
            ),
        ];
        for (name_sid, func) in methods {
            self.install_native_method(proto_id, name_sid, func, shape::PropertyAttrs::METHOD);
        }

        // Accessors (WHATWG §6.4 has root / whatToShow / filter as
        // readonly attrs; currentNode is writable).  `root` has no
        // dedicated well-known SID — install via local intern.
        let attrs_ro = shape::PropertyAttrs::WEBIDL_RO_ACCESSOR;
        let root_sid = self.strings.intern("root");
        self.install_accessor_pair(
            proto_id,
            root_sid,
            native_tree_walker_get_root,
            None,
            attrs_ro,
        );
        let accessors_ro: [(_, NativeFn); 2] = [
            (
                self.well_known.what_to_show,
                native_tree_walker_get_what_to_show,
            ),
            (self.well_known.filter_attr, native_tree_walker_get_filter),
        ];
        for (name_sid, getter) in accessors_ro {
            self.install_accessor_pair(proto_id, name_sid, getter, None, attrs_ro);
        }
        // currentNode (read-write).
        self.install_accessor_pair(
            proto_id,
            self.well_known.current_node,
            native_tree_walker_get_current_node,
            Some(native_tree_walker_set_current_node),
            attrs_ro,
        );

        self.tree_walker_prototype = Some(proto_id);

        let ctor = self.create_constructable_function("TreeWalker", native_tree_walker_constructor);
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
        let name_sid = self.well_known.tree_walker_global;
        self.globals.insert(name_sid, JsValue::Object(ctor));
    }
}

// ---------------------------------------------------------------------------
// Constructor (illegal-invocation throw)
// ---------------------------------------------------------------------------

fn native_tree_walker_constructor(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // WHATWG §6.4 — `TreeWalker` ctor throws; instances are created
    // via `document.createTreeWalker(...)`.
    Err(VmError::type_error(
        "Failed to construct 'TreeWalker': Illegal constructor",
    ))
}

// ---------------------------------------------------------------------------
// Brand check
// ---------------------------------------------------------------------------

fn require_tree_walker_receiver(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &'static str,
) -> Result<u64, VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'TreeWalker': Illegal invocation"
        )));
    };
    match ctx.vm.get_object(id).kind {
        ObjectKind::TreeWalker { walker_id } => Ok(walker_id),
        _ => Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'TreeWalker': Illegal invocation"
        ))),
    }
}

// ---------------------------------------------------------------------------
// Accessors
// ---------------------------------------------------------------------------

fn native_tree_walker_get_root(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let walker_id = require_tree_walker_receiver(ctx, this, "root")?;
    let root = ctx
        .host()
        .tree_walker_states
        .get(&walker_id)
        .map(|s| s.root);
    match root {
        Some(e) => Ok(JsValue::Object(ctx.vm.create_element_wrapper(e))),
        None => Err(walker_detached_error("root")),
    }
}

fn native_tree_walker_get_what_to_show(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let walker_id = require_tree_walker_receiver(ctx, this, "whatToShow")?;
    let mask = ctx
        .host()
        .tree_walker_states
        .get(&walker_id)
        .map(|s| s.what_to_show);
    Ok(mask.map_or(JsValue::Number(0.0), |m| JsValue::Number(f64::from(m))))
}

fn native_tree_walker_get_filter(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let walker_id = require_tree_walker_receiver(ctx, this, "filter")?;
    let filter = ctx
        .host()
        .tree_walker_states
        .get(&walker_id)
        .and_then(|s| s.filter_object_id);
    Ok(match filter {
        Some(bits) => JsValue::Object(ObjectId(bits as u32)),
        None => JsValue::Null,
    })
}

fn native_tree_walker_get_current_node(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let walker_id = require_tree_walker_receiver(ctx, this, "currentNode")?;
    let entity = ctx
        .host()
        .tree_walker_states
        .get(&walker_id)
        .map(|s| s.current);
    match entity {
        Some(e) => Ok(JsValue::Object(ctx.vm.create_element_wrapper(e))),
        None => Err(walker_detached_error("currentNode")),
    }
}

fn native_tree_walker_set_current_node(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let walker_id = require_tree_walker_receiver(ctx, this, "currentNode")?;
    let value = args.first().copied().unwrap_or(JsValue::Undefined);
    let entity = super::node_proto::require_node_arg(ctx, value, "currentNode")?;
    let host = ctx.host();
    if let Some(state) = host.tree_walker_states.get_mut(&walker_id) {
        state.current = entity;
    }
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// Method natives
// ---------------------------------------------------------------------------

fn walker_detached_error(method: &str) -> VmError {
    VmError::type_error(format!(
        "Failed to read '{method}' on 'TreeWalker': the walker has been detached."
    ))
}

/// Common scaffold: brand-check + active-flag set + run `step` with
/// `JsFilter`, then unpack outcome.
///
/// The `step` closure receives a mutable `TreeWalker` snapshot (so
/// the engine-indep `step_with_filter_*` mutates it), the dom, and
/// the filter.  After `step` returns, we write back the snapshot's
/// `current_node` to the persisted `TreeWalkerState`.
fn run_walker_step<S>(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &'static str,
    step: S,
) -> Result<JsValue, VmError>
where
    S: FnOnce(
        &mut TreeWalker,
        &elidex_ecs::EcsDom,
        &mut JsFilter<'_, '_>,
    ) -> Result<Option<Entity>, FilterError>,
{
    let walker_id = require_tree_walker_receiver(ctx, this, method)?;
    // Active-flag check (§6.3 step 2).
    let (root, what_to_show, filter_id, current) = {
        let host = ctx.host();
        let state = host
            .tree_walker_states
            .get_mut(&walker_id)
            .ok_or_else(|| walker_detached_error(method))?;
        if state.active {
            return Err(VmError::dom_exception(
                ctx.vm.well_known.dom_exc_invalid_state_error,
                format!("Failed to execute '{method}' on 'TreeWalker': filter is already active."),
            ));
        }
        state.active = true;
        (
            state.root,
            state.what_to_show,
            state.filter_object_id,
            state.current,
        )
    };

    let mut walker = TreeWalker {
        root,
        current_node: current,
        what_to_show,
    };

    let result = {
        let mut filter = JsFilter::new(ctx, filter_id);
        // Snapshot the dom pointer separately so the filter borrow
        // and step call don't conflict.
        let dom_ptr: *const elidex_ecs::EcsDom = std::ptr::from_ref(filter.ctx.host().dom());
        // SAFETY: dom is a stable `&EcsDom` for the duration of this
        // call (no other path can deallocate the bound EcsDom while
        // a VM call is in flight — same contract as `host().dom()`).
        let dom = unsafe { &*dom_ptr };
        let step_result = step(&mut walker, dom, &mut filter);
        if let Some(err) = filter.take_pending_error() {
            // Reset active-flag before propagating user throw.
            let host = filter.ctx.host();
            if let Some(state) = host.tree_walker_states.get_mut(&walker_id) {
                state.active = false;
            }
            return Err(err);
        }
        step_result
    };

    // Reset active + write back current_node.
    {
        let host = ctx.host();
        if let Some(state) = host.tree_walker_states.get_mut(&walker_id) {
            state.active = false;
            state.current = walker.current_node;
        }
    }

    match result {
        Ok(Some(e)) => Ok(JsValue::Object(ctx.vm.create_element_wrapper(e))),
        // `FilterError::Throw` arm shares this body — the filter's
        // `pending_error` was already drained on the throw path
        // (we surfaced it earlier via `return Err(...)`), so the
        // method appears null-returning to JS here.
        Ok(None) | Err(FilterError::Throw) => Ok(JsValue::Null),
        Err(FilterError::AlreadyActive) => Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_invalid_state_error,
            format!("Failed to execute '{method}' on 'TreeWalker': filter is already active."),
        )),
    }
}

fn native_tree_walker_parent_node(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    run_walker_step(ctx, this, "parentNode", |w, d, f| {
        step_with_filter_parent_node(w, d, f)
    })
}

fn native_tree_walker_first_child(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    run_walker_step(ctx, this, "firstChild", |w, d, f| {
        step_with_filter_first_child(w, d, f)
    })
}

fn native_tree_walker_last_child(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    run_walker_step(ctx, this, "lastChild", |w, d, f| {
        step_with_filter_last_child(w, d, f)
    })
}

fn native_tree_walker_next_sibling(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    run_walker_step(ctx, this, "nextSibling", |w, d, f| {
        step_with_filter_next_sibling(w, d, f)
    })
}

fn native_tree_walker_previous_sibling(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    run_walker_step(ctx, this, "previousSibling", |w, d, f| {
        step_with_filter_previous_sibling(w, d, f)
    })
}

fn native_tree_walker_next_node(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    run_walker_step(ctx, this, "nextNode", |w, d, f| step_with_filter(w, d, f))
}

fn native_tree_walker_previous_node(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    run_walker_step(ctx, this, "previousNode", |w, d, f| {
        step_with_filter_previous_node(w, d, f)
    })
}
