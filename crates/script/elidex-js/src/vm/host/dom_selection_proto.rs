// boa skip: VM-only surface; the legacy boa runtime
// (`crates/script/elidex-js-boa/`) is on the deletion path per
// `m4-12-platform-gap-roadmap.md` §E-2 Round 20 PR7.  See
// `memory/project_boa_runtime_deletion.md`.

//! `Selection` interface (Selection API Living Standard §3) — VM thin
//! binding to the engine-independent [`elidex_dom_api::SelectionState`].
//!
//! Distinct from sibling [`super::selection_api`], which owns the
//! HTMLInputElement / HTMLTextAreaElement form-field selection per
//! HTML §4.10.5 — this file owns the WHATWG Selection API per
//! `window.getSelection()` / `document.getSelection()`.
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate": no spec algorithms live here.
//! Every method dispatches into [`elidex_dom_api::SelectionState`] for
//! the actual state mutation / direction derivation / validity gates.
//! `vm/host/` is restricted to: prototype install, brand check,
//! WebIDL arg coercion (JS → Rust), and DOM-exception mapping for
//! Selection-side error variants.
//!
//! ## Singleton storage
//!
//! Selection state lives in [`super::super::host_data::HostData`]:
//!
//! - `selection_state: Option<SelectionState>` — engine-indep state
//!   machine (current `RangeId`, direction bias).  Lazily created on
//!   the first dispatcher entry that needs it.
//! - `selection_instance: Option<ObjectId>` — canonical `[SameObject]`
//!   wrapper returned by `window.getSelection()` /
//!   `document.getSelection()`.  Cleared by GC sweep when the wrapper
//!   becomes unreachable; next `getSelection()` call rebuilds it.
//!
//! Both are `Option<...>` (not maps) because the M4-12 VM is
//! single-Window single-Document.  Multi-document promotion is
//! tracked at `#11-mutation-hook-multiplexer` (D-15 ShadowRoot /
//! iframe entry).
//!
//! ## GC interaction
//!
//! `ObjectKind::Selection` is payload-free.  Trace fan-out lives in
//! `vm/gc/trace.rs`: when this wrapper is marked, the trace marks the
//! cached `Range` wrapper at
//! `range_instances[active_range_id.bits()]` so the LiveRangeRegistry
//! entry survives across sweeps even when the user has dropped their
//! JS Range reference.  If no Range wrapper has been materialised
//! yet (Selection set internally via `collapse(node, 0)` and
//! `getRangeAt(0)` never called), the trace fan-out is a no-op —
//! `getRangeAt(0)` builds a wrapper from the still-registered
//! `RangeId` on demand.

#![cfg(feature = "engine")]

use elidex_dom_api::{RangeId, SelectionDirection, SelectionError, SelectionState, SelectionType};
use elidex_ecs::Entity;

use super::super::shape;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, VmError,
};
use super::super::{NativeFn, VmInner};

impl VmInner {
    /// Allocate `Selection.prototype` chained to `Object.prototype`,
    /// install its 8 accessors + 15 methods, and expose the `Selection`
    /// constructor placeholder on `globalThis` (no user-callable
    /// constructor — `new Selection()` throws per spec, but the
    /// global slot lets `sel instanceof Selection` work).
    ///
    /// Called from `register_globals()` after `register_range_global`
    /// (no hard ordering — `Selection.prototype` chains to
    /// `Object.prototype` only).
    #[allow(clippy::too_many_lines)]
    pub(in crate::vm) fn register_selection_global(&mut self) {
        let object_proto = self
            .object_prototype
            .expect("register_selection_global called before register_prototypes");

        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });

        let methods: [(_, NativeFn); 15] = [
            (
                self.well_known.get_range_at_method,
                native_selection_get_range_at as NativeFn,
            ),
            (self.well_known.add_range_method, native_selection_add_range),
            (
                self.well_known.remove_range_method,
                native_selection_remove_range,
            ),
            (
                self.well_known.remove_all_ranges_method,
                native_selection_remove_all_ranges,
            ),
            (self.well_known.empty_method, native_selection_empty),
            (self.well_known.collapse_method, native_selection_collapse),
            // setPosition is a spec alias of collapse (Selection API §3.2).
            (
                self.well_known.set_position_method,
                native_selection_collapse,
            ),
            (
                self.well_known.collapse_to_start_method,
                native_selection_collapse_to_start,
            ),
            (
                self.well_known.collapse_to_end_method,
                native_selection_collapse_to_end,
            ),
            (self.well_known.extend_method, native_selection_extend),
            (
                self.well_known.set_base_and_extent_method,
                native_selection_set_base_and_extent,
            ),
            (
                self.well_known.select_all_children_method,
                native_selection_select_all_children,
            ),
            (
                self.well_known.delete_from_document_method,
                native_selection_delete_from_document,
            ),
            (
                self.well_known.contains_node_method,
                native_selection_contains_node,
            ),
            (self.well_known.to_string_method, native_selection_to_string),
        ];
        for (name_sid, func) in methods {
            self.install_native_method(proto_id, name_sid, func, shape::PropertyAttrs::METHOD);
        }

        let accessors: [(_, NativeFn); 8] = [
            (
                self.well_known.anchor_node_attr,
                native_selection_get_anchor_node as NativeFn,
            ),
            (
                self.well_known.anchor_offset_attr,
                native_selection_get_anchor_offset,
            ),
            (
                self.well_known.focus_node_attr,
                native_selection_get_focus_node,
            ),
            (
                self.well_known.focus_offset_attr,
                native_selection_get_focus_offset,
            ),
            (
                self.well_known.is_collapsed_attr,
                native_selection_get_is_collapsed,
            ),
            (
                self.well_known.range_count_attr,
                native_selection_get_range_count,
            ),
            (self.well_known.r#type, native_selection_get_type),
            (
                self.well_known.direction_attr,
                native_selection_get_direction,
            ),
        ];
        let attrs = shape::PropertyAttrs::WEBIDL_RO_ACCESSOR;
        for (name_sid, getter) in accessors {
            self.install_accessor_pair(proto_id, name_sid, getter, None, attrs);
        }

        self.selection_prototype = Some(proto_id);

        // Selection has NO user-callable constructor (Selection API
        // §3.2 — instances are obtained via `window.getSelection()` /
        // `document.getSelection()`).  We still expose the global
        // `Selection` constructor function so `sel instanceof
        // Selection` works; invoking it throws `TypeError` per spec
        // "Illegal constructor".
        let ctor = self.create_illegal_constructor_function(
            "Selection",
            super::super::value::native_illegal_constructor_unreachable,
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
        let name_sid = self.well_known.selection_global;
        self.globals.insert(name_sid, JsValue::Object(ctor));
    }

    /// Return the per-document `Selection` `[SameObject]` wrapper,
    /// allocating it on the first `window.getSelection()` /
    /// `document.getSelection()` call.  The wrapper itself is
    /// payload-free (`ObjectKind::Selection`); the actual state is
    /// reached via the `HostData::selection_state` singleton.
    ///
    /// Re-allocates after `Vm::unbind` (which clears
    /// `selection_instance` along with the rest of the per-document
    /// state); a JS reference retained across rebind brand-checks
    /// successfully but its method calls will surface
    /// `InvalidStateError` until the next `getSelection()` call
    /// materialises a fresh wrapper.
    pub(in crate::vm) fn alloc_or_cached_selection(&mut self) -> ObjectId {
        if let Some(host) = self.host_data.as_deref() {
            if let Some(id) = host.selection_instance {
                return id;
            }
        }
        let proto = self
            .selection_prototype
            .expect("alloc_or_cached_selection before register_selection_global");
        let id = self.alloc_object(Object {
            kind: ObjectKind::Selection,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(proto),
            extensible: true,
        });
        if let Some(host) = self.host_data.as_deref_mut() {
            host.selection_instance = Some(id);
            // Eagerly initialise the SelectionState slot too so
            // subsequent accessor reads don't need to (cheap;
            // `SelectionState::default` is just two `None`s).
            if host.selection_state.is_none() {
                host.selection_state = Some(SelectionState::new());
            }
        }
        id
    }
}

// ---------------------------------------------------------------------------
// Brand check
// ---------------------------------------------------------------------------

/// Confirm `this` is the per-document Selection singleton wrapper.
/// Returns `()` because `ObjectKind::Selection` is payload-free — the
/// state lives in `HostData::selection_state` and is reached via the
/// `read_selection_state` / `mutate_selection_state` helpers below.
fn require_selection_receiver(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &'static str,
) -> Result<(), VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'Selection': Illegal invocation"
        )));
    };
    match ctx.vm.get_object(id).kind {
        ObjectKind::Selection => Ok(()),
        _ => Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'Selection': Illegal invocation"
        ))),
    }
}

// ---------------------------------------------------------------------------
// Error mapping (SelectionError → DOMException)
// ---------------------------------------------------------------------------

/// Map an engine-indep `SelectionError` into the matching JS
/// `DOMException` per Selection API §3.2 / §3.3.
fn map_selection_error(
    ctx: &NativeContext<'_>,
    err: SelectionError,
    method: &'static str,
) -> VmError {
    let (sid, label) = match err {
        SelectionError::InvalidNodeType => (
            ctx.vm.well_known.dom_exc_invalid_node_type_error,
            "The node provided is a DocumentType",
        ),
        SelectionError::WrongDocument => (
            ctx.vm.well_known.dom_exc_wrong_document_error,
            "The node provided is from a different document",
        ),
        SelectionError::IndexSize => (
            ctx.vm.well_known.dom_exc_index_size_error,
            "The offset is larger than the node's length",
        ),
        SelectionError::InvalidState => (
            ctx.vm.well_known.dom_exc_invalid_state_error,
            "Selection has no current Range",
        ),
        SelectionError::OutOfRange => (
            ctx.vm.well_known.dom_exc_index_size_error,
            "Index is out of range",
        ),
    };
    VmError::dom_exception(
        sid,
        format!("Failed to execute '{method}' on 'Selection': {label}"),
    )
}

// ---------------------------------------------------------------------------
// State access helpers
// ---------------------------------------------------------------------------

/// Read-only access to the Selection state + LiveRangeRegistry +
/// `&EcsDom` + the active document entity.  Lazily initialises
/// `host_data.selection_state` to a fresh empty state on first access
/// (matches Chrome semantics — `getSelection()` always returns a
/// usable Selection, never null in our single-doc VM).
///
/// Throws `InvalidStateError` if the VM is unbound (retained Selection
/// wrapper across `Vm::unbind()` — same shape as
/// `range_proto::detached_range_error`).
fn read_selection<F, R>(
    ctx: &mut NativeContext<'_>,
    method: &'static str,
    f: F,
) -> Result<R, VmError>
where
    F: FnOnce(
        &SelectionState,
        &mut elidex_dom_api::LiveRangeRegistry,
        &elidex_ecs::EcsDom,
        Entity,
    ) -> R,
{
    if ctx.host_if_bound().is_none() {
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_invalid_state_error,
            format!(
                "Failed to execute '{method}' on 'Selection': host environment is not initialised"
            ),
        ));
    }
    let host = ctx.host();
    let doc = host.document();
    if host.selection_state.is_none() {
        host.selection_state = Some(SelectionState::new());
    }
    let (dom, registry, sel_slot) = host.split_dom_live_ranges_and_selection();
    let state = sel_slot.as_ref().expect("just initialised");
    Ok(f(state, registry, dom, doc))
}

/// Mutable access pattern for Selection methods that mutate state +
/// registry + (optionally) read DOM.  Sets `selectionchange_pending`
/// to `true` on success.  Engine-indep errors returned by the closure
/// (as [`SelectionError`]) are mapped to the corresponding
/// `DOMException` here so the engine-indep crate stays free of
/// VM-side intern dependencies.
///
/// Copilot R1 IMP-3 (registry-leak cleanup): when the closure
/// replaces `active_range_id` with a different `RangeId`, the
/// displaced `RangeId` is unregistered from `LiveRangeRegistry`
/// **only if** no JS `Range` wrapper exists for it in
/// `range_instances`.  This preserves the previous-range-survival
/// contract — user-held `r = sel.getRangeAt(0)` followed by
/// `sel.collapse(...)` keeps `r` live-tracked because its cached
/// wrapper id is in `range_instances` — while preventing unbounded
/// growth of registry entries in tight `sel.collapse(n,0);
/// sel.collapse(n,1); ...` loops that never materialise a wrapper.
fn mutate_selection<F, R>(
    ctx: &mut NativeContext<'_>,
    method: &'static str,
    f: F,
) -> Result<R, VmError>
where
    F: FnOnce(
        &mut SelectionState,
        &mut elidex_dom_api::LiveRangeRegistry,
        &elidex_ecs::EcsDom,
        Entity,
    ) -> Result<R, SelectionError>,
{
    if ctx.host_if_bound().is_none() {
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_invalid_state_error,
            format!(
                "Failed to execute '{method}' on 'Selection': host environment is not initialised"
            ),
        ));
    }
    // Snapshot the pre-call RangeId so we can detect replacement
    // and free the displaced registry entry if no wrapper exists.
    let prev_id = ctx
        .host()
        .selection_state
        .as_ref()
        .and_then(SelectionState::current_range_id);
    // Confine all HostData borrows to this inner block so we can
    // re-borrow `ctx` afterwards for `map_selection_error`,
    // `selectionchange_pending` mutation, and the displaced-id
    // unregister check.
    let outcome = {
        let host = ctx.host();
        let doc = host.document();
        if host.selection_state.is_none() {
            host.selection_state = Some(SelectionState::new());
        }
        let (dom, registry, sel_slot) = host.split_dom_live_ranges_and_selection();
        let state = sel_slot.as_mut().expect("just initialised");
        f(state, registry, dom, doc)
    };
    let value = outcome.map_err(|e| map_selection_error(ctx, e, method))?;
    let new_id = ctx
        .host()
        .selection_state
        .as_ref()
        .and_then(SelectionState::current_range_id);
    if let Some(old) = prev_id {
        if Some(old) != new_id {
            let host = ctx.host();
            if !host.range_instances.contains_key(&old.0) {
                host.live_range_registry.unregister(old);
            }
        }
    }
    ctx.host().selectionchange_pending = true;
    Ok(value)
}

/// `deleteFromDocument` needs `&mut EcsDom` (the spec algorithm
/// mutates the tree) **plus** `&mut LiveRangeRegistry` + `&mut
/// SelectionState`.  Per CLAUDE.md layering mandate (Copilot R1
/// IMP-2), the spec algorithm lives in the engine-indep
/// [`SelectionState::delete_from_document`] — this VM-side function
/// owns only the borrow split + dirty-bit flip.
fn delete_selection_contents(
    ctx: &mut NativeContext<'_>,
    method: &'static str,
) -> Result<(), VmError> {
    if ctx.host_if_bound().is_none() {
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_invalid_state_error,
            format!(
                "Failed to execute '{method}' on 'Selection': host environment is not initialised"
            ),
        ));
    }
    let host = ctx.host();
    if host.selection_state.is_none() {
        host.selection_state = Some(SelectionState::new());
    }
    let (dom_mut, registry, sel_slot) = host.split_dom_mut_live_ranges_and_selection();
    let state = sel_slot.as_mut().expect("just initialised");
    state.delete_from_document(registry, dom_mut);
    ctx.host().selectionchange_pending = true;
    Ok(())
}

// ---------------------------------------------------------------------------
// Arg coercion
// ---------------------------------------------------------------------------

fn arg_node_required(
    ctx: &mut NativeContext<'_>,
    arg: Option<JsValue>,
    method: &'static str,
) -> Result<Entity, VmError> {
    let value = arg.ok_or_else(|| {
        VmError::type_error(format!(
            "Failed to execute '{method}' on 'Selection': 1 argument required"
        ))
    })?;
    if ctx.host_if_bound().is_none() {
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_invalid_state_error,
            format!(
                "Failed to execute '{method}' on 'Selection': host environment is not initialised"
            ),
        ));
    }
    super::node_proto::require_node_arg(ctx, value, method)
}

fn arg_offset_or_default(
    ctx: &mut NativeContext<'_>,
    arg: Option<JsValue>,
) -> Result<usize, VmError> {
    // Selection `collapse(node, offset)`: offset defaults to 0 per
    // Selection API §3.2.  Cast through `to_uint32` for the
    // ToUint32 coercion (per WebIDL §3.10).
    let val = arg.unwrap_or(JsValue::Number(0.0));
    let n = super::super::coerce::to_uint32(ctx.vm, val)?;
    Ok(n as usize)
}

fn arg_offset_required(
    ctx: &mut NativeContext<'_>,
    arg: Option<JsValue>,
    method: &'static str,
) -> Result<usize, VmError> {
    let Some(val) = arg else {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'Selection': required argument missing"
        )));
    };
    let n = super::super::coerce::to_uint32(ctx.vm, val)?;
    Ok(n as usize)
}

fn arg_range(
    ctx: &NativeContext<'_>,
    arg: Option<JsValue>,
    method: &'static str,
) -> Result<RangeId, VmError> {
    let value = arg.ok_or_else(|| {
        VmError::type_error(format!(
            "Failed to execute '{method}' on 'Selection': 1 argument required"
        ))
    })?;
    let JsValue::Object(id) = value else {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'Selection': parameter is not of type 'Range'."
        )));
    };
    match ctx.vm.get_object(id).kind {
        ObjectKind::Range { range_id } => Ok(RangeId(range_id)),
        _ => Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'Selection': parameter is not of type 'Range'."
        ))),
    }
}

fn arg_bool_optional(ctx: &mut NativeContext<'_>, arg: Option<JsValue>) -> Result<bool, VmError> {
    let val = arg.unwrap_or(JsValue::Boolean(false));
    Ok(super::super::coerce::to_boolean(ctx.vm, val))
}

// ---------------------------------------------------------------------------
// Range wrapper materialisation (getRangeAt)
// ---------------------------------------------------------------------------

/// Build or look up the `[SameObject]` Range wrapper for `range_id`.
/// Reuses the existing `HostData::range_instances` cache so
/// `sel.getRangeAt(0) === sel.getRangeAt(0)` holds.
fn wrap_range_id(ctx: &mut NativeContext<'_>, range_id: RangeId) -> ObjectId {
    let host = ctx.host();
    if let Some(&existing) = host.range_instances.get(&range_id.0) {
        return existing;
    }
    let proto = ctx.vm.range_prototype;
    let new_id = ctx.vm.alloc_object(Object {
        kind: ObjectKind::Range {
            range_id: range_id.0,
        },
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });
    ctx.host().range_instances.insert(range_id.0, new_id);
    new_id
}

// ---------------------------------------------------------------------------
// Accessors (8)
// ---------------------------------------------------------------------------

fn native_selection_get_anchor_node(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_selection_receiver(ctx, this, "anchorNode")?;
    let anchor = read_selection(ctx, "anchorNode", |s, reg, dom, _doc| s.anchor(reg, dom))?;
    Ok(match anchor {
        Some((entity, _)) => JsValue::Object(ctx.vm.create_element_wrapper(entity)),
        None => JsValue::Null,
    })
}

fn native_selection_get_anchor_offset(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_selection_receiver(ctx, this, "anchorOffset")?;
    let off = read_selection(ctx, "anchorOffset", |s, reg, dom, _doc| s.anchor(reg, dom))?;
    let n = off.map_or(0, |(_, o)| o);
    #[allow(clippy::cast_precision_loss)]
    Ok(JsValue::Number(n as f64))
}

fn native_selection_get_focus_node(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_selection_receiver(ctx, this, "focusNode")?;
    let focus = read_selection(ctx, "focusNode", |s, reg, dom, _doc| s.focus(reg, dom))?;
    Ok(match focus {
        Some((entity, _)) => JsValue::Object(ctx.vm.create_element_wrapper(entity)),
        None => JsValue::Null,
    })
}

fn native_selection_get_focus_offset(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_selection_receiver(ctx, this, "focusOffset")?;
    let off = read_selection(ctx, "focusOffset", |s, reg, dom, _doc| s.focus(reg, dom))?;
    let n = off.map_or(0, |(_, o)| o);
    #[allow(clippy::cast_precision_loss)]
    Ok(JsValue::Number(n as f64))
}

fn native_selection_get_is_collapsed(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_selection_receiver(ctx, this, "isCollapsed")?;
    let collapsed = read_selection(ctx, "isCollapsed", |s, reg, dom, _doc| {
        s.is_collapsed(reg, dom)
    })?;
    Ok(JsValue::Boolean(collapsed))
}

fn native_selection_get_range_count(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_selection_receiver(ctx, this, "rangeCount")?;
    let n = read_selection(ctx, "rangeCount", |s, _reg, _dom, _doc| s.range_count())?;
    Ok(JsValue::Number(f64::from(n)))
}

fn native_selection_get_type(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_selection_receiver(ctx, this, "type")?;
    let t = read_selection(ctx, "type", |s, reg, dom, _doc| s.selection_type(reg, dom))?;
    // Map directly to pre-interned SIDs to avoid per-call intern.
    let sid = match t {
        SelectionType::None => ctx.vm.well_known.selection_type_none,
        SelectionType::Caret => ctx.vm.well_known.selection_type_caret,
        SelectionType::Range => ctx.vm.well_known.selection_type_range,
    };
    Ok(JsValue::String(sid))
}

fn native_selection_get_direction(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_selection_receiver(ctx, this, "direction")?;
    let d = read_selection(ctx, "direction", |s, reg, dom, _doc| {
        s.current_direction(reg, dom)
    })?;
    let sid = match d {
        SelectionDirection::Forward => ctx.vm.well_known.selection_dir_forward,
        SelectionDirection::Backward => ctx.vm.well_known.selection_dir_backward,
        SelectionDirection::Directionless => ctx.vm.well_known.selection_dir_directionless,
    };
    Ok(JsValue::String(sid))
}

// ---------------------------------------------------------------------------
// Methods (15)
// ---------------------------------------------------------------------------

fn native_selection_get_range_at(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_selection_receiver(ctx, this, "getRangeAt")?;
    let index = arg_offset_required(ctx, args.first().copied(), "getRangeAt")?;
    // Selection API §3.2 step 1: throw IndexSizeError if index >= rangeCount.
    let range_id = read_selection(ctx, "getRangeAt", |s, _reg, _dom, _doc| {
        s.current_range_id()
    })?;
    let Some(rid) = range_id else {
        return Err(map_selection_error(
            ctx,
            SelectionError::OutOfRange,
            "getRangeAt",
        ));
    };
    if index > 0 {
        return Err(map_selection_error(
            ctx,
            SelectionError::OutOfRange,
            "getRangeAt",
        ));
    }
    let wrapper = wrap_range_id(ctx, rid);
    Ok(JsValue::Object(wrapper))
}

fn native_selection_add_range(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_selection_receiver(ctx, this, "addRange")?;
    let range_id = arg_range(ctx, args.first().copied(), "addRange")?;
    // We need the range's owner document AND the selection's owner
    // document to decide the no-op case.  Pull both from registered
    // state.
    let host = ctx.host();
    let sel_owner = host.document();
    let (dom, registry, sel_slot) = host.split_dom_live_ranges_and_selection();
    if sel_slot.is_none() {
        *sel_slot = Some(SelectionState::new());
    }
    let range_owner = registry
        .with_range(range_id, dom, |r, _| r.owner_document)
        .ok_or_else(|| {
            VmError::type_error(
                "Failed to execute 'addRange' on 'Selection': parameter is not of type 'Range'.",
            )
        })?;
    let changed =
        sel_slot
            .as_mut()
            .expect("just initialised")
            .add_range(range_owner, sel_owner, range_id);
    if changed {
        ctx.host().selectionchange_pending = true;
    }
    Ok(JsValue::Undefined)
}

fn native_selection_remove_range(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_selection_receiver(ctx, this, "removeRange")?;
    let range_id = arg_range(ctx, args.first().copied(), "removeRange")?;
    mutate_selection(ctx, "removeRange", |s, _reg, _dom, _doc| {
        s.remove_range(range_id)
    })?;
    Ok(JsValue::Undefined)
}

fn native_selection_remove_all_ranges(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_selection_receiver(ctx, this, "removeAllRanges")?;
    mutate_selection(ctx, "removeAllRanges", |s, _reg, _dom, _doc| {
        s.remove_all_ranges();
        Ok(())
    })?;
    Ok(JsValue::Undefined)
}

fn native_selection_empty(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_selection_receiver(ctx, this, "empty")?;
    mutate_selection(ctx, "empty", |s, _reg, _dom, _doc| {
        s.empty();
        Ok(())
    })?;
    Ok(JsValue::Undefined)
}

fn native_selection_collapse(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_selection_receiver(ctx, this, "collapse")?;
    // WebIDL §3.7.6 — arg conversion in declared order.
    let node = arg_node_required(ctx, args.first().copied(), "collapse")?;
    let offset = arg_offset_or_default(ctx, args.get(1).copied())?;
    mutate_selection(ctx, "collapse", |s, reg, dom, doc| {
        s.collapse(reg, dom, doc, node, offset)
    })?;
    Ok(JsValue::Undefined)
}

fn native_selection_collapse_to_start(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_selection_receiver(ctx, this, "collapseToStart")?;
    mutate_selection(ctx, "collapseToStart", |s, reg, dom, _doc| {
        s.collapse_to_start(reg, dom)
    })?;
    Ok(JsValue::Undefined)
}

fn native_selection_collapse_to_end(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_selection_receiver(ctx, this, "collapseToEnd")?;
    mutate_selection(ctx, "collapseToEnd", |s, reg, dom, _doc| {
        s.collapse_to_end(reg, dom)
    })?;
    Ok(JsValue::Undefined)
}

fn native_selection_extend(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_selection_receiver(ctx, this, "extend")?;
    let node = arg_node_required(ctx, args.first().copied(), "extend")?;
    let offset = arg_offset_or_default(ctx, args.get(1).copied())?;
    mutate_selection(ctx, "extend", |s, reg, dom, doc| {
        s.extend(reg, dom, doc, node, offset)
    })?;
    Ok(JsValue::Undefined)
}

fn native_selection_set_base_and_extent(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_selection_receiver(ctx, this, "setBaseAndExtent")?;
    // WebIDL declared-order coercion (lesson #245): brand-check both
    // node args + ToUint32 both offset args BEFORE any state probe.
    let anchor = arg_node_required(ctx, args.first().copied(), "setBaseAndExtent")?;
    let anchor_offset = arg_offset_required(ctx, args.get(1).copied(), "setBaseAndExtent")?;
    let focus = arg_node_required(ctx, args.get(2).copied(), "setBaseAndExtent")?;
    let focus_offset = arg_offset_required(ctx, args.get(3).copied(), "setBaseAndExtent")?;
    mutate_selection(ctx, "setBaseAndExtent", |s, reg, dom, doc| {
        s.set_base_and_extent(reg, dom, doc, anchor, anchor_offset, focus, focus_offset)
    })?;
    Ok(JsValue::Undefined)
}

fn native_selection_select_all_children(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_selection_receiver(ctx, this, "selectAllChildren")?;
    let parent = arg_node_required(ctx, args.first().copied(), "selectAllChildren")?;
    mutate_selection(ctx, "selectAllChildren", |s, reg, dom, doc| {
        s.select_all_children(reg, dom, doc, parent)
    })?;
    Ok(JsValue::Undefined)
}

fn native_selection_delete_from_document(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_selection_receiver(ctx, this, "deleteFromDocument")?;
    delete_selection_contents(ctx, "deleteFromDocument")?;
    Ok(JsValue::Undefined)
}

fn native_selection_contains_node(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_selection_receiver(ctx, this, "containsNode")?;
    let node = arg_node_required(ctx, args.first().copied(), "containsNode")?;
    let allow_partial = arg_bool_optional(ctx, args.get(1).copied())?;
    let res = read_selection(ctx, "containsNode", |s, reg, dom, doc| {
        s.contains_node(reg, dom, doc, node, allow_partial)
    })?;
    Ok(JsValue::Boolean(res))
}

fn native_selection_to_string(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_selection_receiver(ctx, this, "toString")?;
    let s = read_selection(ctx, "toString", |sel, reg, dom, _doc| {
        sel.to_string(reg, dom)
    })?;
    let sid = ctx.vm.strings.intern(&s);
    Ok(JsValue::String(sid))
}

// ---------------------------------------------------------------------------
// selectionchange dispatch (HTML §8.1.7.1 "selection task source")
// ---------------------------------------------------------------------------

/// Dispatch a coalesced `selectionchange` event at the bound Document
/// per Selection API §3.4 / HTML §8.1.7.1 ("selection task source") if
/// the per-document dirty bit is set.  Resets the bit on success.
///
/// Called from `VmInner::drain_tasks` after the regular pending-task
/// loop drains; one event per drain regardless of how many discrete
/// Selection mutations queued up.  Reentrancy gate matches the
/// surrounding drain — the helper short-circuits when no bit is set
/// or when the host is unbound.
///
/// Returns `true` when an event was actually fired (telemetry /
/// tests).
#[allow(clippy::missing_panics_doc)] // panics only on shape-table mis-init (impossible after register_globals)
pub(in crate::vm) fn dispatch_selectionchange_if_pending(vm: &mut VmInner) -> bool {
    // Read + reset the pending bit atomically (single-threaded VM).
    let should_fire = match vm.host_data.as_deref_mut() {
        Some(hd) if hd.selectionchange_pending && hd.is_bound() => {
            hd.selectionchange_pending = false;
            true
        }
        _ => false,
    };
    if !should_fire {
        return false;
    }
    let Some(host) = vm.host_data.as_deref() else {
        return false;
    };
    if !host.is_bound() {
        return false;
    }
    let document_entity = host.document();
    let type_sid = vm.well_known.selectionchange_event;

    // Build a minimal Event (selectionchange is non-bubbling,
    // non-cancelable, no payload per spec).  Reuse the `event_prototype`
    // chain directly — no UA-shape fold beyond core9.
    let event_proto = vm.event_prototype;
    let event_id = vm.alloc_object(Object {
        kind: ObjectKind::Event {
            default_prevented: false,
            propagation_stopped: false,
            immediate_propagation_stopped: false,
            cancelable: false,
            passive: false,
            type_sid,
            bubbles: false,
            composed: false,
            composed_path: None,
        },
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: event_proto,
        extensible: true,
    });
    // Pin the event across dispatch — without rooting, an
    // alloc-heavy listener could collect it mid-walk.
    let mut g = vm.push_temp_root(JsValue::Object(event_id));
    // Install the precomputed core-9 shape so `dispatch_script_event`
    // can write the target slot at the canonical offset.
    let core_shape = g
        .precomputed_event_shapes
        .as_ref()
        .expect("precomputed_event_shapes built during VM init")
        .core;
    let timestamp_ms = g.start_instant.elapsed().as_secs_f64() * 1000.0;
    let slots: Vec<PropertyValue> = vec![
        PropertyValue::Data(JsValue::String(type_sid)),
        PropertyValue::Data(JsValue::Boolean(false)),
        PropertyValue::Data(JsValue::Boolean(false)),
        PropertyValue::Data(JsValue::Number(0.0)),
        PropertyValue::Data(JsValue::Null), // target — filled by dispatch
        PropertyValue::Data(JsValue::Null), // currentTarget — filled by dispatch
        PropertyValue::Data(JsValue::Number(timestamp_ms)),
        PropertyValue::Data(JsValue::Boolean(false)),
        PropertyValue::Data(JsValue::Boolean(true)),
    ];
    g.define_with_precomputed_shape(event_id, core_shape, slots);
    g.dispatched_events.insert(event_id);
    {
        let mut ctx = NativeContext::new_call(&mut g);
        let _ = super::event_target_dispatch::dispatch_script_event(
            &mut ctx,
            event_id,
            document_entity,
        );
    }
    g.dispatched_events.remove(&event_id);
    true
}
