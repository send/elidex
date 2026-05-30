// boa skip: VM-only surface; the legacy boa runtime
// (`crates/script/elidex-js-boa/`) is on the deletion path per
// `m4-12-platform-gap-roadmap.md` §E-2 Round 20 PR7.  See
// `memory/project_boa_runtime_deletion.md`.

//! `Range` interface (WHATWG DOM §4.4) — VM thin binding to the
//! engine-independent [`elidex_dom_api::Range`] +
//! [`elidex_dom_api::LiveRangeRegistry`].
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", this file performs only the
//! engine-bound responsibilities: prototype install, brand check,
//! argument coercion (JS → Rust marshalling), and one-line dispatch
//! into engine-indep helpers.  All Range algorithms (boundary
//! adjustment, point compare, intersects, etc.) live in
//! [`elidex_dom_api::range`].
//!
//! ## State storage
//!
//! Range state is split between two side tables:
//!
//! - [`super::super::host_data::HostData::live_range_registry`] — the
//!   [`elidex_dom_api::LiveRangeRegistry`] that owns the actual
//!   [`elidex_dom_api::Range`] records keyed by [`RangeId`].
//!   Mutation hooks fired by `EcsDom` apply boundary adjustments to
//!   the registered Ranges synchronously via the bound bridge.
//! - [`super::super::host_data::HostData::range_instances`] —
//!   `HashMap<u64, ObjectId>` from `RangeId` bits to the wrapper
//!   `ObjectId`.  Allows GC sweep to drop the registration when a
//!   wrapper becomes unreachable.
//!
//! [`super::super::value::ObjectKind::Range`] carries the
//! [`RangeId`] bits inline (`range_id: u64`); the wrapper has no
//! own data beyond the prototype chain.

#![cfg(feature = "engine")]

use elidex_dom_api::range::{
    RangePointError, END_TO_END, END_TO_START, START_TO_END, START_TO_START,
};
use elidex_dom_api::{Range, RangeId};
use elidex_ecs::Entity;

use super::super::shape;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, VmError,
};
use super::super::{NativeFn, VmInner};

impl VmInner {
    /// Allocate `Range.prototype` chained to `Object.prototype`,
    /// install its 23 method / accessor surface, and expose the
    /// `Range` constructor on `globalThis`.
    ///
    /// Called from `register_globals()` after `register_prototypes`
    /// and `register_mutation_observer_global` (no ordering
    /// constraint beyond `Object.prototype` existing).  Method count
    /// and boundary constants push past the 100-line clippy default;
    /// the install is a flat data-driven table so splitting would
    /// only obscure the structure.
    #[allow(clippy::too_many_lines)]
    pub(in crate::vm) fn register_range_global(&mut self) {
        let object_proto = self
            .object_prototype
            .expect("register_range_global called before register_prototypes");

        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });

        let methods: [(_, NativeFn); 22] = [
            (
                self.well_known.set_start_method,
                native_range_set_start as NativeFn,
            ),
            (self.well_known.set_end_method, native_range_set_end),
            (
                self.well_known.set_start_before_method,
                native_range_set_start_before,
            ),
            (
                self.well_known.set_start_after_method,
                native_range_set_start_after,
            ),
            (
                self.well_known.set_end_before_method,
                native_range_set_end_before,
            ),
            (
                self.well_known.set_end_after_method,
                native_range_set_end_after,
            ),
            (self.well_known.collapse_method, native_range_collapse),
            (self.well_known.select_node_method, native_range_select_node),
            (
                self.well_known.select_node_contents_method,
                native_range_select_node_contents,
            ),
            (
                self.well_known.compare_boundary_points_method,
                native_range_compare_boundary_points,
            ),
            (self.well_known.clone_range_method, native_range_clone_range),
            (
                self.well_known.clone_contents_method,
                super::range_proto_mutation::native_range_clone_contents,
            ),
            (
                self.well_known.extract_contents_method,
                super::range_proto_mutation::native_range_extract_contents,
            ),
            (
                self.well_known.delete_contents_method,
                super::range_proto_mutation::native_range_delete_contents,
            ),
            (
                self.well_known.insert_node_method,
                super::range_proto_mutation::native_range_insert_node,
            ),
            (
                self.well_known.surround_contents_method,
                super::range_proto_mutation::native_range_surround_contents,
            ),
            (self.well_known.detach_method, native_range_detach),
            (
                self.well_known.is_point_in_range_method,
                native_range_is_point_in_range,
            ),
            (
                self.well_known.compare_point_method,
                native_range_compare_point,
            ),
            (
                self.well_known.intersects_node_method,
                native_range_intersects_node,
            ),
            (self.well_known.to_string_method, native_range_to_string),
            (
                self.well_known.create_contextual_fragment_method,
                super::range_proto_mutation::native_range_create_contextual_fragment,
            ),
        ];
        for (name_sid, func) in methods {
            self.install_native_method(proto_id, name_sid, func, shape::PropertyAttrs::METHOD);
        }

        let accessors: [(_, NativeFn); 6] = [
            (
                self.well_known.start_container,
                native_range_get_start_container as NativeFn,
            ),
            (self.well_known.start_offset, native_range_get_start_offset),
            (
                self.well_known.end_container,
                native_range_get_end_container,
            ),
            (self.well_known.end_offset, native_range_get_end_offset),
            (self.well_known.collapsed_attr, native_range_get_collapsed),
            (
                self.well_known.common_ancestor_container,
                native_range_get_common_ancestor,
            ),
        ];
        let attrs = shape::PropertyAttrs::WEBIDL_RO_ACCESSOR;
        for (name_sid, getter) in accessors {
            self.install_accessor_pair(proto_id, name_sid, getter, None, attrs);
        }

        // START_TO_START / START_TO_END / END_TO_END / END_TO_START
        // on Range.prototype (per spec these live on both ctor +
        // prototype as readonly unsigned short constants).
        install_boundary_constants(self, proto_id);

        self.range_prototype = Some(proto_id);

        let ctor = self.create_constructor_only_function("Range", native_range_constructor);
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
        // Constants also live on the ctor object itself per spec.
        install_boundary_constants(self, ctor);
        let name_sid = self.well_known.range_global;
        self.globals.insert(name_sid, JsValue::Object(ctor));
    }
}

fn install_boundary_constants(vm: &mut VmInner, target: ObjectId) {
    let entries = [
        (vm.well_known.start_to_start, f64::from(START_TO_START)),
        (vm.well_known.start_to_end, f64::from(START_TO_END)),
        (vm.well_known.end_to_end, f64::from(END_TO_END)),
        (vm.well_known.end_to_start, f64::from(END_TO_START)),
    ];
    for (key, value) in entries {
        vm.define_shaped_property(
            target,
            PropertyKey::String(key),
            PropertyValue::Data(JsValue::Number(value)),
            shape::PropertyAttrs::WEBIDL_RO,
        );
    }
}

// ---------------------------------------------------------------------------
// Brand check
// ---------------------------------------------------------------------------

/// Recover the [`RangeId`] from `this`, returning `TypeError` if `this`
/// is not a `Range` instance.  Shared with sibling
/// [`super::range_proto_mutation`] mutating natives.
pub(super) fn require_range_receiver(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &'static str,
) -> Result<RangeId, VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'Range': Illegal invocation"
        )));
    };
    match ctx.vm.get_object(id).kind {
        ObjectKind::Range { range_id } => Ok(RangeId(range_id)),
        _ => Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'Range': Illegal invocation"
        ))),
    }
}

/// Recover a [`RangeId`] from a function argument (e.g.
/// `compareBoundaryPoints(how, sourceRange)`).  Errors with the
/// canonical Chrome/Firefox "parameter is not of type 'Range'" message.
fn require_range_arg(
    ctx: &NativeContext<'_>,
    arg: JsValue,
    method: &'static str,
) -> Result<RangeId, VmError> {
    let JsValue::Object(id) = arg else {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'Range': parameter is not of type 'Range'."
        )));
    };
    match ctx.vm.get_object(id).kind {
        ObjectKind::Range { range_id } => Ok(RangeId(range_id)),
        _ => Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'Range': parameter is not of type 'Range'."
        ))),
    }
}

// ---------------------------------------------------------------------------
// Constructor
// ---------------------------------------------------------------------------

/// `new Range()` (WHATWG DOM §4.4 constructor steps).
///
/// Per spec, sets `(start, end)` to `(current global object's
/// associated Document, 0)` and `owner_document` to the same Document.
fn native_range_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let JsValue::Object(this_id) = this else {
        unreachable!("constructor `this` is always an Object after `do_new`");
    };
    // Copilot R6: `host_opt()` is true post-`unbind()` (HostData still
    // installed, but `dom_ptr` is null).  Use `host_if_bound()` so
    // post-unbind construction surfaces a JS TypeError rather than
    // panicking on the subsequent `ctx.host().document()` call.
    if ctx.host_if_bound().is_none() {
        return Err(VmError::type_error(
            "Failed to construct 'Range': host environment is not initialised",
        ));
    }
    let doc = ctx.host().document();
    let range = Range::new_with_owner(doc, doc);
    let range_id = ctx.host().live_range_registry.register(range);
    ctx.vm.get_object_mut(this_id).kind = ObjectKind::Range {
        range_id: range_id.0,
    };
    ctx.host().range_instances.insert(range_id.0, this_id);
    Ok(JsValue::Object(this_id))
}

// ---------------------------------------------------------------------------
// `read_range` helper — borrow split (Round 1 Arch CRIT-2)
// ---------------------------------------------------------------------------

/// Build the "Range has been detached" `InvalidStateError` per
/// WHATWG §4.4 + the local registry-cleared semantics on
/// `Vm::unbind()`.  Shared with sibling [`super::range_proto_mutation`].
pub(super) fn detached_range_error(ctx: &NativeContext<'_>, method: &'static str) -> VmError {
    VmError::dom_exception(
        ctx.vm.well_known.dom_exc_invalid_state_error,
        format!("Failed to execute '{method}' on 'Range': the Range has been detached"),
    )
}

/// Run `f` with a shared [`Range`] borrow + `&EcsDom`, returning
/// `Ok(value)` when the range is still registered.  Throws
/// `InvalidStateError` if the Range was unregistered (orphaned by GC,
/// post-`unbind` access of a retained reference).
///
/// `finalize_pending` runs internally inside
/// [`elidex_dom_api::LiveRangeRegistry::with_range`] so dangling-collapse
/// is applied transparently — VM-side code never calls
/// `finalize_pending` directly.
fn read_range<F, R>(
    ctx: &mut NativeContext<'_>,
    id: RangeId,
    method: &'static str,
    f: F,
) -> Result<R, VmError>
where
    F: FnOnce(&mut NativeContext<'_>, &Range) -> Result<R, VmError>,
{
    // Copilot R6: retained `Range` wrappers across `Vm::unbind()`
    // would otherwise panic in `split_dom_and_live_ranges` (which
    // asserts `is_bound()`).  Surface as the documented detached
    // semantics — `InvalidStateError` — so JS code can recover.
    if ctx.host_if_bound().is_none() {
        return Err(detached_range_error(ctx, method));
    }
    let host = ctx.host();
    let (dom, registry) = host.split_dom_and_live_ranges();
    let captured = registry.with_range(id, dom, |range, _dom| range.clone());
    let Some(range) = captured else {
        return Err(detached_range_error(ctx, method));
    };
    f(ctx, &range)
}

/// Apply a mutation `f` to the registered [`Range`].  Throws
/// `InvalidStateError` if the Range was unregistered or the VM is
/// unbound (Copilot R6).
fn write_range<F, R>(
    ctx: &mut NativeContext<'_>,
    id: RangeId,
    method: &'static str,
    f: F,
) -> Result<R, VmError>
where
    F: FnOnce(&mut Range, &elidex_ecs::EcsDom) -> R,
{
    if ctx.host_if_bound().is_none() {
        return Err(detached_range_error(ctx, method));
    }
    let host = ctx.host();
    let (dom, registry) = host.split_dom_and_live_ranges();
    let result = registry.with_range_mut(id, dom, f);
    match result {
        Some(v) => Ok(v),
        None => Err(detached_range_error(ctx, method)),
    }
}

// ---------------------------------------------------------------------------
// Accessors
// ---------------------------------------------------------------------------

fn native_range_get_start_container(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_range_receiver(ctx, this, "startContainer")?;
    let entity = read_range(ctx, id, "startContainer", |_ctx, r| Ok(r.start_container))?;
    Ok(JsValue::Object(ctx.vm.create_element_wrapper(entity)))
}

fn native_range_get_start_offset(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_range_receiver(ctx, this, "startOffset")?;
    let off = read_range(ctx, id, "startOffset", |_ctx, r| Ok(r.start_offset))?;
    #[allow(clippy::cast_precision_loss)]
    Ok(JsValue::Number(off as f64))
}

fn native_range_get_end_container(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_range_receiver(ctx, this, "endContainer")?;
    let entity = read_range(ctx, id, "endContainer", |_ctx, r| Ok(r.end_container))?;
    Ok(JsValue::Object(ctx.vm.create_element_wrapper(entity)))
}

fn native_range_get_end_offset(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_range_receiver(ctx, this, "endOffset")?;
    let off = read_range(ctx, id, "endOffset", |_ctx, r| Ok(r.end_offset))?;
    #[allow(clippy::cast_precision_loss)]
    Ok(JsValue::Number(off as f64))
}

fn native_range_get_collapsed(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_range_receiver(ctx, this, "collapsed")?;
    let collapsed = read_range(ctx, id, "collapsed", |_ctx, r| Ok(r.collapsed()))?;
    Ok(JsValue::Boolean(collapsed))
}

fn native_range_get_common_ancestor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_range_receiver(ctx, this, "commonAncestorContainer")?;
    let entity = read_range(ctx, id, "commonAncestorContainer", |ctx, r| {
        let dom = ctx.host().dom();
        Ok(r.common_ancestor_container(dom))
    })?;
    Ok(JsValue::Object(ctx.vm.create_element_wrapper(entity)))
}

// ---------------------------------------------------------------------------
// Boundary setters
// ---------------------------------------------------------------------------

/// Coerce a required `offset: unsigned long` argument per WebIDL.
/// Copilot R3: a MISSING arg must throw `TypeError`, not silently
/// default to 0 — `Range.prototype.setStart(node)` (no second arg)
/// is `Range.prototype.setStart.length === 2`.
fn arg_offset_required(
    ctx: &mut NativeContext<'_>,
    arg: Option<JsValue>,
    method: &'static str,
) -> Result<usize, VmError> {
    let Some(val) = arg else {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'Range': 2 arguments required"
        )));
    };
    let n = super::super::coerce::to_uint32(ctx.vm, val)?;
    Ok(n as usize)
}

pub(super) fn arg_node(
    ctx: &mut NativeContext<'_>,
    arg: Option<JsValue>,
    method: &'static str,
) -> Result<Entity, VmError> {
    let value = arg.ok_or_else(|| {
        VmError::type_error(format!(
            "Failed to execute '{method}' on 'Range': 1 argument required"
        ))
    })?;
    // Copilot R12: `require_node_arg` dereferences `ctx.host().dom()`
    // for the brand check.  A retained Range wrapper across
    // `Vm::unbind()` can still pass `ObjectKind::Range`, so any
    // node-taking Range method (`setStart`, `insertNode`, `surroundContents`,
    // etc.) would panic instead of surfacing `InvalidStateError` here.
    // Gate on `host_if_bound` BEFORE the dom-touching helper.
    if ctx.host_if_bound().is_none() {
        return Err(detached_range_error(ctx, method));
    }
    super::node_proto::require_node_arg(ctx, value, method)
}

/// WHATWG DOM §4.4 "set the start or end of a Range to a boundary point"
/// — step 1: throw `InvalidNodeTypeError` when `node` is a `DocumentType`,
/// step 2: throw `IndexSizeError` when `offset > node's length`.
///
/// Used by `setStart` / `setEnd` (boundary-point setters with explicit
/// offset).  `selectNodeContents` reuses the same doctype check but
/// has no offset arg.
fn validate_boundary_node_and_offset(
    ctx: &mut NativeContext<'_>,
    node: Entity,
    offset: usize,
    method: &'static str,
) -> Result<(), VmError> {
    // Copilot R11: gate on `host_if_bound` BEFORE `ctx.host().dom()`
    // call — a retained Range method reaches here via the receiver
    // brand check (which only requires HostData installed, not
    // bound).  Surface as detached-range InvalidStateError.
    if ctx.host_if_bound().is_none() {
        return Err(detached_range_error(ctx, method));
    }
    reject_doctype(ctx, node, method)?;
    let len = elidex_dom_api::range::node_length(node, ctx.host().dom());
    if offset > len {
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_index_size_error,
            format!(
                "Failed to execute '{method}' on 'Range': \
                 The offset {offset} is larger than the node's length ({len})."
            ),
        ));
    }
    Ok(())
}

/// WHATWG DOM §4.4 — methods that anchor at `node`'s sibling slot
/// (`setStartBefore` / `setStartAfter` / `setEndBefore` / `setEndAfter`
/// / `selectNode`) throw `InvalidNodeTypeError` when `node`'s parent is
/// null.
fn require_attached_node(
    ctx: &mut NativeContext<'_>,
    node: Entity,
    method: &'static str,
) -> Result<(), VmError> {
    // Copilot R11: same `host_if_bound` gate as
    // `validate_boundary_node_and_offset`.
    if ctx.host_if_bound().is_none() {
        return Err(detached_range_error(ctx, method));
    }
    if ctx.host().dom().get_parent(node).is_none() {
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_invalid_node_type_error,
            format!(
                "Failed to execute '{method}' on 'Range': \
                 The given node has no parent."
            ),
        ));
    }
    Ok(())
}

fn native_range_set_start(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_range_receiver(ctx, this, "setStart")?;
    let node = arg_node(ctx, args.first().copied(), "setStart")?;
    let offset = arg_offset_required(ctx, args.get(1).copied(), "setStart")?;
    validate_boundary_node_and_offset(ctx, node, offset, "setStart")?;
    // Copilot R2: spec step 4 collapse-on-cross-root / after-end.
    write_range(ctx, id, "setStart", |r, dom| {
        r.set_start_to_boundary(node, offset, dom);
    })?;
    Ok(JsValue::Undefined)
}

fn native_range_set_end(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_range_receiver(ctx, this, "setEnd")?;
    let node = arg_node(ctx, args.first().copied(), "setEnd")?;
    let offset = arg_offset_required(ctx, args.get(1).copied(), "setEnd")?;
    validate_boundary_node_and_offset(ctx, node, offset, "setEnd")?;
    write_range(ctx, id, "setEnd", |r, dom| {
        r.set_end_to_boundary(node, offset, dom);
    })?;
    Ok(JsValue::Undefined)
}

fn native_range_set_start_before(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_range_receiver(ctx, this, "setStartBefore")?;
    let node = arg_node(ctx, args.first().copied(), "setStartBefore")?;
    require_attached_node(ctx, node, "setStartBefore")?;
    write_range(ctx, id, "setStartBefore", |r, dom| {
        r.set_start_before(node, dom);
    })?;
    Ok(JsValue::Undefined)
}

fn native_range_set_start_after(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_range_receiver(ctx, this, "setStartAfter")?;
    let node = arg_node(ctx, args.first().copied(), "setStartAfter")?;
    require_attached_node(ctx, node, "setStartAfter")?;
    write_range(ctx, id, "setStartAfter", |r, dom| {
        r.set_start_after(node, dom);
    })?;
    Ok(JsValue::Undefined)
}

fn native_range_set_end_before(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_range_receiver(ctx, this, "setEndBefore")?;
    let node = arg_node(ctx, args.first().copied(), "setEndBefore")?;
    require_attached_node(ctx, node, "setEndBefore")?;
    write_range(ctx, id, "setEndBefore", |r, dom| {
        r.set_end_before(node, dom);
    })?;
    Ok(JsValue::Undefined)
}

fn native_range_set_end_after(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_range_receiver(ctx, this, "setEndAfter")?;
    let node = arg_node(ctx, args.first().copied(), "setEndAfter")?;
    require_attached_node(ctx, node, "setEndAfter")?;
    write_range(ctx, id, "setEndAfter", |r, dom| {
        r.set_end_after(node, dom);
    })?;
    Ok(JsValue::Undefined)
}

fn native_range_collapse(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_range_receiver(ctx, this, "collapse")?;
    // WebIDL `optional boolean toStart = false` — coerce via ToBoolean
    // when present, default false when absent / undefined.
    let to_start = match args.first().copied() {
        None | Some(JsValue::Undefined) => false,
        Some(v) => super::super::coerce::to_boolean(ctx.vm, v),
    };
    write_range(ctx, id, "collapse", |r, _dom| r.collapse(to_start))?;
    Ok(JsValue::Undefined)
}

fn native_range_select_node(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_range_receiver(ctx, this, "selectNode")?;
    let node = arg_node(ctx, args.first().copied(), "selectNode")?;
    // WHATWG §4.4 selectNode step 1 — parent must not be null.
    require_attached_node(ctx, node, "selectNode")?;
    write_range(ctx, id, "selectNode", |r, dom| r.select_node(node, dom))?;
    Ok(JsValue::Undefined)
}

fn native_range_select_node_contents(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_range_receiver(ctx, this, "selectNodeContents")?;
    let node = arg_node(ctx, args.first().copied(), "selectNodeContents")?;
    // WHATWG §4.4 selectNodeContents step 1 — node must not be a doctype.
    reject_doctype(ctx, node, "selectNodeContents")?;
    write_range(ctx, id, "selectNodeContents", |r, dom| {
        r.select_node_contents(node, dom);
    })?;
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// Compare / point query methods
// ---------------------------------------------------------------------------

fn native_range_compare_boundary_points(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_range_receiver(ctx, this, "compareBoundaryPoints")?;
    // Copilot R20: WebIDL §3.7.6 mandates argument conversion in
    // declared order, so brand-check + coerce BOTH args BEFORE
    // running the DOM `how`-validation step.  The previous order
    // (validate `how` → coerce `sourceRange`) made
    // `compareBoundaryPoints(999, {})` throw NotSupportedError when
    // browsers throw TypeError ("Failed to execute ... parameter 2
    // is not of type 'Range'").
    let how_val = args.first().copied().unwrap_or(JsValue::Undefined);
    let how = super::super::coerce::to_uint16(ctx.vm, how_val)?;
    let other_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let other_id = require_range_arg(ctx, other_arg, "compareBoundaryPoints")?;
    // WebIDL §3.10.7 — `how` must be one of the 4 spec constants;
    // any other value throws NotSupportedError per spec.  Runs AFTER
    // both arg conversions per the precedence above.
    if !matches!(
        how,
        START_TO_START | START_TO_END | END_TO_END | END_TO_START
    ) {
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_not_supported_error,
            "Failed to execute 'compareBoundaryPoints' on 'Range': \
             The value provided is not a valid 'how' constant.",
        ));
    }
    // Copilot R7: gate on `host_if_bound` BEFORE
    // `split_dom_and_live_ranges` — the latter asserts on unbound
    // `dom_ptr` and would panic for retained Range refs across
    // `Vm::unbind()`.
    if ctx.host_if_bound().is_none() {
        return Err(detached_range_error(ctx, "compareBoundaryPoints"));
    }
    let host = ctx.host();
    let (dom, registry) = host.split_dom_and_live_ranges();
    let other_range = registry
        .with_range(other_id, dom, |r, _| r.clone())
        .ok_or_else(|| detached_range_error(ctx, "compareBoundaryPoints"))?;
    let result = read_range(ctx, id, "compareBoundaryPoints", |ctx, r| {
        let dom = ctx.host().dom();
        // WHATWG §4.4 step 2 — `WrongDocumentError` when the two
        // Ranges have different roots.
        if dom.find_tree_root(r.start_container) != dom.find_tree_root(other_range.start_container)
        {
            return Err(VmError::dom_exception(
                ctx.vm.well_known.dom_exc_wrong_document_error,
                "Failed to execute 'compareBoundaryPoints' on 'Range': \
                 The two Ranges are in different trees.",
            ));
        }
        Ok(r.compare_boundary_points(how, &other_range, dom))
    })?;
    Ok(JsValue::Number(f64::from(result)))
}

fn native_range_is_point_in_range(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_range_receiver(ctx, this, "isPointInRange")?;
    let node = arg_node(ctx, args.first().copied(), "isPointInRange")?;
    let offset = arg_offset_required(ctx, args.get(1).copied(), "isPointInRange")?;
    // Copilot R4: doctype check moved into engine-indep
    // `Range::is_point_in_range` so spec step ORDER (root → doctype
    // → offset) is enforced — cross-root DocumentType now returns
    // `false` per spec step 1 instead of throwing.
    let result = read_range(ctx, id, "isPointInRange", |ctx, r| {
        let dom = ctx.host().dom();
        r.is_point_in_range(node, offset, dom)
            .map_err(|e| point_error_to_vm(ctx, e, "isPointInRange"))
    })?;
    Ok(JsValue::Boolean(result))
}

fn native_range_compare_point(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_range_receiver(ctx, this, "comparePoint")?;
    let node = arg_node(ctx, args.first().copied(), "comparePoint")?;
    let offset = arg_offset_required(ctx, args.get(1).copied(), "comparePoint")?;
    // Copilot R4: doctype check moved into engine-indep
    // `Range::compare_point` so spec step ORDER (root → doctype
    // → offset) is enforced via `WrongDocumentError` precedence.
    let result = read_range(ctx, id, "comparePoint", |ctx, r| {
        let dom = ctx.host().dom();
        r.compare_point(node, offset, dom)
            .map_err(|e| point_error_to_vm(ctx, e, "comparePoint"))
    })?;
    Ok(JsValue::Number(f64::from(result)))
}

fn native_range_intersects_node(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_range_receiver(ctx, this, "intersectsNode")?;
    let node = arg_node(ctx, args.first().copied(), "intersectsNode")?;
    let result = read_range(ctx, id, "intersectsNode", |ctx, r| {
        let dom = ctx.host().dom();
        Ok(r.intersects_node(node, dom))
    })?;
    Ok(JsValue::Boolean(result))
}

fn reject_doctype(
    ctx: &mut NativeContext<'_>,
    node: Entity,
    method: &'static str,
) -> Result<(), VmError> {
    use elidex_ecs::{DocTypeData, NodeKind};
    // Copilot R11: ctx.host().dom() panics post-unbind.  Gate first.
    if ctx.host_if_bound().is_none() {
        return Err(detached_range_error(ctx, method));
    }
    let dom = ctx.host().dom();
    let is_doctype = matches!(dom.node_kind_inferred(node), Some(NodeKind::DocumentType))
        || dom.world().get::<&DocTypeData>(node).is_ok();
    if is_doctype {
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_invalid_node_type_error,
            format!(
                "Failed to execute '{method}' on 'Range': \
                 The given node is a DocumentType."
            ),
        ));
    }
    Ok(())
}

fn point_error_to_vm(
    ctx: &NativeContext<'_>,
    err: RangePointError,
    method: &'static str,
) -> VmError {
    let wk = &ctx.vm.well_known;
    match err {
        RangePointError::WrongDocument => VmError::dom_exception(
            wk.dom_exc_wrong_document_error,
            format!(
                "Failed to execute '{method}' on 'Range': \
                 The two nodes are in different documents."
            ),
        ),
        RangePointError::InvalidNodeType => VmError::dom_exception(
            wk.dom_exc_invalid_node_type_error,
            format!(
                "Failed to execute '{method}' on 'Range': \
                 The given node is a DocumentType."
            ),
        ),
        RangePointError::IndexSize => VmError::dom_exception(
            wk.dom_exc_index_size_error,
            format!(
                "Failed to execute '{method}' on 'Range': \
                 The offset is larger than the node's length."
            ),
        ),
    }
}

// ---------------------------------------------------------------------------
// Clone / detach / toString
// ---------------------------------------------------------------------------

fn native_range_clone_range(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_range_receiver(ctx, this, "cloneRange")?;
    // Copilot R7: gate on `host_if_bound` BEFORE
    // `split_dom_and_live_ranges` (same fix as `compareBoundaryPoints`).
    if ctx.host_if_bound().is_none() {
        return Err(detached_range_error(ctx, "cloneRange"));
    }
    let cloned = {
        let host = ctx.host();
        let (dom, registry) = host.split_dom_and_live_ranges();
        registry
            .with_range(id, dom, |r, _| r.clone())
            .ok_or_else(|| detached_range_error(ctx, "cloneRange"))?
    };
    let new_id = ctx.host().live_range_registry.register(cloned);
    let proto = ctx.vm.range_prototype.expect("range_prototype installed");
    let wrapper = ctx.vm.alloc_object(Object {
        kind: ObjectKind::Range { range_id: new_id.0 },
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: Some(proto),
        extensible: true,
    });
    ctx.host().range_instances.insert(new_id.0, wrapper);
    Ok(JsValue::Object(wrapper))
}

fn native_range_detach(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // WHATWG DOM §4.4 — legacy no-op since 2014, but as a WebIDL
    // operation it still must reject non-Range receivers per WebIDL
    // §3.10 "operations" + brand-check requirement (Copilot R1).
    let _ = require_range_receiver(ctx, this, "detach")?;
    Ok(JsValue::Undefined)
}

fn native_range_to_string(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_range_receiver(ctx, this, "toString")?;
    let text = read_range(ctx, id, "toString", |ctx, r| {
        let dom = ctx.host().dom();
        Ok(r.to_string(dom))
    })?;
    let sid = ctx.vm.strings.intern(&text);
    Ok(JsValue::String(sid))
}

// Mutating methods + Phase-A stubs live in sibling
// [`super::range_proto_mutation`] — split out to keep this module
// under the ~1000-line convention (Copilot R3 MIN).
