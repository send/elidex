// boa skip: VM-only surface; the legacy boa runtime
// (`crates/script/elidex-js-boa/`) is on the deletion path per
// `m4-12-platform-gap-roadmap.md` §E-2 Round 20 PR7.  See
// `memory/project_boa_runtime_deletion.md`.

//! `NodeIterator` interface (WHATWG DOM §6.1) — VM thin binding to
//! [`elidex_dom_api::traversal::step_with_filter_node_iterator_*`].
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", this file performs only
//! prototype install, brand check, marshalling, and filter callback
//! dispatch.  The actual §6.1 forward / backward walks +
//! pre-removing-steps adjustment live in
//! [`elidex_dom_api::traversal`].
//!
//! ## State storage
//!
//! - [`super::super::host_data::HostData::node_iterator_states_shared`]
//!   — `Arc<Mutex<HashMap<u64, NodeIteratorState>>>` shared with
//!   [`elidex_dom_api::MutationBridge`] so the hook fire-path can
//!   apply §6.1 pre-removing-steps synchronously.
//! - [`super::super::host_data::HostData::node_iterator_instances`]
//!   — reverse lookup table for GC sweep.

#![cfg(feature = "engine")]
// See sibling `tree_walker_proto.rs` for the same raw-pointer SAFETY
// rationale for snapshotting `&EcsDom` past a borrow conflict.
#![allow(unsafe_code)]

use elidex_dom_api::traversal::{
    step_with_filter_node_iterator_next, step_with_filter_node_iterator_previous, FilterError,
};
use elidex_dom_api::NodeIteratorState;
use elidex_ecs::Entity;

use super::super::shape;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, VmError,
};
use super::super::{NativeFn, VmInner};
use super::node_filter_dispatch::JsFilter;

impl VmInner {
    /// Allocate `NodeIterator.prototype` + expose constructor on
    /// `globalThis`.  Per spec the constructor throws — instances
    /// are created via `document.createNodeIterator(...)`.
    pub(in crate::vm) fn register_node_iterator_global(&mut self) {
        let object_proto = self
            .object_prototype
            .expect("register_node_iterator_global called before register_prototypes");

        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });

        let methods: [(_, NativeFn); 3] = [
            (
                self.well_known.next_node_method,
                native_node_iterator_next_node as NativeFn,
            ),
            (
                self.well_known.previous_node_method,
                native_node_iterator_previous_node,
            ),
            (self.well_known.detach_method, native_node_iterator_detach),
        ];
        for (name_sid, func) in methods {
            self.install_native_method(proto_id, name_sid, func, shape::PropertyAttrs::METHOD);
        }

        let attrs = shape::PropertyAttrs::WEBIDL_RO_ACCESSOR;
        let root_sid = self.strings.intern("root");
        self.install_accessor_pair(
            proto_id,
            root_sid,
            native_node_iterator_get_root,
            None,
            attrs,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.what_to_show,
            native_node_iterator_get_what_to_show,
            None,
            attrs,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.filter_attr,
            native_node_iterator_get_filter,
            None,
            attrs,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.reference_node,
            native_node_iterator_get_reference_node,
            None,
            attrs,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.pointer_before_reference_node,
            native_node_iterator_get_pointer_before,
            None,
            attrs,
        );

        self.node_iterator_prototype = Some(proto_id);

        let ctor = self
            .create_illegal_constructor_function("NodeIterator", native_node_iterator_constructor);
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
        let name_sid = self.well_known.node_iterator_global;
        self.globals.insert(name_sid, JsValue::Object(ctor));
    }
}

fn native_node_iterator_constructor(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // Unreachable: `CallShape::IllegalConstructor` gate throws before
    // this body runs (dispatch / `do_new`).
    unreachable!("NodeIterator IllegalConstructor gate throws before body runs")
}

// ---------------------------------------------------------------------------
// Brand check
// ---------------------------------------------------------------------------

fn require_node_iterator_receiver(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &'static str,
) -> Result<u64, VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'NodeIterator': Illegal invocation"
        )));
    };
    match ctx.vm.get_object(id).kind {
        ObjectKind::NodeIterator { iterator_id } => Ok(iterator_id),
        _ => Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'NodeIterator': Illegal invocation"
        ))),
    }
}

// ---------------------------------------------------------------------------
// Accessors
// ---------------------------------------------------------------------------

fn read_state<R>(
    ctx: &mut NativeContext<'_>,
    iterator_id: u64,
    f: impl FnOnce(&NodeIteratorState) -> R,
) -> Option<R> {
    let host = ctx.host();
    let guard = host.node_iterator_states_shared.lock().ok()?;
    guard.get(&iterator_id).map(f)
}

fn native_node_iterator_get_root(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_node_iterator_receiver(ctx, this, "root")?;
    let entity = read_state(ctx, id, |s| s.root).ok_or_else(|| detached("root"))?;
    Ok(JsValue::Object(ctx.vm.create_element_wrapper(entity)))
}

fn native_node_iterator_get_what_to_show(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_node_iterator_receiver(ctx, this, "whatToShow")?;
    // Copilot R10: surface detached state instead of defaulting
    // to 0 (which a JS caller can mistake for "filter disabled").
    let mask = read_state(ctx, id, |s| s.what_to_show).ok_or_else(|| detached("whatToShow"))?;
    Ok(JsValue::Number(f64::from(mask)))
}

fn native_node_iterator_get_filter(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_node_iterator_receiver(ctx, this, "filter")?;
    // Copilot R10: detached state surfaces an error rather than
    // silently returning null.
    let filter = read_state(ctx, id, |s| s.filter_object_id).ok_or_else(|| detached("filter"))?;
    Ok(match filter {
        Some(bits) => JsValue::Object(ObjectId(bits as u32)),
        None => JsValue::Null,
    })
}

fn native_node_iterator_get_reference_node(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_node_iterator_receiver(ctx, this, "referenceNode")?;
    let entity = read_state(ctx, id, |s| s.reference).ok_or_else(|| detached("referenceNode"))?;
    Ok(JsValue::Object(ctx.vm.create_element_wrapper(entity)))
}

fn native_node_iterator_get_pointer_before(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_node_iterator_receiver(ctx, this, "pointerBeforeReferenceNode")?;
    // Copilot R10 — same detached-state surface as `whatToShow`.
    let before = read_state(ctx, id, |s| s.pointer_before)
        .ok_or_else(|| detached("pointerBeforeReferenceNode"))?;
    Ok(JsValue::Boolean(before))
}

// ---------------------------------------------------------------------------
// Methods
// ---------------------------------------------------------------------------

fn detached(method: &str) -> VmError {
    VmError::type_error(format!(
        "Failed to read '{method}' on 'NodeIterator': the iterator has been detached."
    ))
}

fn run_iterator_step<S>(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &'static str,
    step: S,
) -> Result<JsValue, VmError>
where
    S: FnOnce(
        &mut NodeIteratorState,
        &elidex_ecs::EcsDom,
        &mut JsFilter<'_, '_>,
    ) -> Result<Option<Entity>, FilterError>,
{
    let iterator_id = require_node_iterator_receiver(ctx, this, method)?;
    // Pre-read SID before borrowing host (avoid mutex-guard / vm
    // borrow conflict for the InvalidStateError DOMException).
    let invalid_state_sid = ctx.vm.well_known.dom_exc_invalid_state_error;

    // Snapshot + active-flag set inside a tight lock scope.
    let (mut state_snapshot, filter_id) = {
        let host = ctx.host();
        let mut guard = host
            .node_iterator_states_shared
            .lock()
            .map_err(|_| VmError::type_error("NodeIterator state mutex poisoned"))?;
        let state = guard
            .get_mut(&iterator_id)
            .ok_or_else(|| detached(method))?;
        if state.active {
            return Err(VmError::dom_exception(
                invalid_state_sid,
                format!(
                    "Failed to execute '{method}' on 'NodeIterator': filter is already active."
                ),
            ));
        }
        state.active = true;
        (state.clone(), state.filter_object_id)
    };

    let result = {
        let mut filter = JsFilter::new(ctx, filter_id);
        let dom_ptr: *const elidex_ecs::EcsDom = std::ptr::from_ref(filter.ctx.host().dom());
        // SAFETY: stable `&EcsDom` for the duration of this call.
        let dom = unsafe { &*dom_ptr };
        let step_result = step(&mut state_snapshot, dom, &mut filter);
        if let Some(err) = filter.take_pending_error() {
            // Write back active-flag clear before propagating throw.
            commit_state(filter.ctx, iterator_id, &state_snapshot, false);
            return Err(err);
        }
        step_result
    };

    commit_state(ctx, iterator_id, &state_snapshot, false);

    match result {
        Ok(Some(e)) => Ok(JsValue::Object(ctx.vm.create_element_wrapper(e))),
        // `FilterError::Throw` arm shares this body — the filter's
        // pending_error was already surfaced before reaching here.
        Ok(None) | Err(FilterError::Throw) => Ok(JsValue::Null),
        Err(FilterError::AlreadyActive) => Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_invalid_state_error,
            format!("Failed to execute '{method}' on 'NodeIterator': filter is already active."),
        )),
    }
}

fn commit_state(
    ctx: &mut NativeContext<'_>,
    iterator_id: u64,
    snapshot: &NodeIteratorState,
    active: bool,
) {
    let host = ctx.host();
    if let Ok(mut guard) = host.node_iterator_states_shared.lock() {
        if let Some(state) = guard.get_mut(&iterator_id) {
            state.reference = snapshot.reference;
            state.pointer_before = snapshot.pointer_before;
            state.active = active;
        }
    }
}

fn native_node_iterator_next_node(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    run_iterator_step(ctx, this, "nextNode", |s, d, f| {
        step_with_filter_node_iterator_next(s, d, f)
    })
}

fn native_node_iterator_previous_node(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    run_iterator_step(ctx, this, "previousNode", |s, d, f| {
        step_with_filter_node_iterator_previous(s, d, f)
    })
}

fn native_node_iterator_detach(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // WHATWG §6.1 — `detach()` is a legacy no-op since 2014, but as a
    // WebIDL operation it still must reject non-NodeIterator
    // receivers per WebIDL §3.10 brand-check (Copilot R1).
    let _ = require_node_iterator_receiver(ctx, this, "detach")?;
    Ok(JsValue::Undefined)
}
