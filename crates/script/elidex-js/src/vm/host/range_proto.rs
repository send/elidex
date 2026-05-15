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
                native_range_clone_contents,
            ),
            (
                self.well_known.extract_contents_method,
                native_range_extract_contents,
            ),
            (
                self.well_known.delete_contents_method,
                native_range_delete_contents,
            ),
            (self.well_known.insert_node_method, native_range_insert_node),
            (
                self.well_known.surround_contents_method,
                native_range_surround_contents,
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
                native_range_create_contextual_fragment,
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

        let ctor = self.create_constructable_function("Range", native_range_constructor);
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
/// is not a `Range` instance.
fn require_range_receiver(
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
    if !ctx.is_construct() {
        return Err(VmError::type_error(
            "Failed to construct 'Range': Please use the 'new' operator",
        ));
    }
    let JsValue::Object(this_id) = this else {
        unreachable!("constructor `this` is always an Object after `do_new`");
    };
    if ctx.host_opt().is_none() {
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
    let host = ctx.host();
    let (dom, registry) = host.split_dom_and_live_ranges();
    let captured = registry.with_range(id, dom, |range, _dom| range.clone());
    let Some(range) = captured else {
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_invalid_state_error,
            format!("Failed to execute '{method}' on 'Range': the Range has been detached"),
        ));
    };
    f(ctx, &range)
}

/// Apply a mutation `f` to the registered [`Range`].  Throws
/// `InvalidStateError` if the Range was unregistered.
fn write_range<F, R>(
    ctx: &mut NativeContext<'_>,
    id: RangeId,
    method: &'static str,
    f: F,
) -> Result<R, VmError>
where
    F: FnOnce(&mut Range, &elidex_ecs::EcsDom) -> R,
{
    let host = ctx.host();
    let (dom, registry) = host.split_dom_and_live_ranges();
    let result = registry.with_range_mut(id, dom, f);
    result.ok_or_else(|| {
        VmError::dom_exception(
            ctx.vm.well_known.dom_exc_invalid_state_error,
            format!("Failed to execute '{method}' on 'Range': the Range has been detached"),
        )
    })
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

fn arg_offset(ctx: &mut NativeContext<'_>, arg: Option<JsValue>) -> Result<usize, VmError> {
    let val = arg.unwrap_or(JsValue::Undefined);
    let n = super::super::coerce::to_uint32(ctx.vm, val)?;
    Ok(n as usize)
}

fn arg_node(
    ctx: &mut NativeContext<'_>,
    arg: Option<JsValue>,
    method: &'static str,
) -> Result<Entity, VmError> {
    let value = arg.ok_or_else(|| {
        VmError::type_error(format!(
            "Failed to execute '{method}' on 'Range': 1 argument required"
        ))
    })?;
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
    let offset = arg_offset(ctx, args.get(1).copied())?;
    validate_boundary_node_and_offset(ctx, node, offset, "setStart")?;
    write_range(ctx, id, "setStart", |r, _dom| r.set_start(node, offset))?;
    Ok(JsValue::Undefined)
}

fn native_range_set_end(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_range_receiver(ctx, this, "setEnd")?;
    let node = arg_node(ctx, args.first().copied(), "setEnd")?;
    let offset = arg_offset(ctx, args.get(1).copied())?;
    validate_boundary_node_and_offset(ctx, node, offset, "setEnd")?;
    write_range(ctx, id, "setEnd", |r, _dom| r.set_end(node, offset))?;
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
    let how_val = args.first().copied().unwrap_or(JsValue::Undefined);
    let how = super::super::coerce::to_uint16(ctx.vm, how_val)?;
    // WebIDL §3.10.7 — `how` must be one of the 4 spec constants;
    // any other value throws NotSupportedError per spec.
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
    let other_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let other_id = require_range_arg(ctx, other_arg, "compareBoundaryPoints")?;
    let host = ctx.host();
    let (dom, registry) = host.split_dom_and_live_ranges();
    let other_range = registry
        .with_range(other_id, dom, |r, _| r.clone())
        .ok_or_else(|| {
            VmError::dom_exception(
                ctx.vm.well_known.dom_exc_invalid_state_error,
                "Failed to execute 'compareBoundaryPoints' on 'Range': \
                 the source Range has been detached",
            )
        })?;
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
    let offset = arg_offset(ctx, args.get(1).copied())?;
    reject_doctype(ctx, node, "isPointInRange")?;
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
    let offset = arg_offset(ctx, args.get(1).copied())?;
    reject_doctype(ctx, node, "comparePoint")?;
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
    let cloned = {
        let host = ctx.host();
        let (dom, registry) = host.split_dom_and_live_ranges();
        registry
            .with_range(id, dom, |r, _| r.clone())
            .ok_or_else(|| {
                VmError::dom_exception(
                    ctx.vm.well_known.dom_exc_invalid_state_error,
                    "Failed to execute 'cloneRange' on 'Range': \
                     the Range has been detached",
                )
            })?
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

// ---------------------------------------------------------------------------
// Mutating methods
// ---------------------------------------------------------------------------

/// Snapshot the registered Range, then commit `mutated` back over it.
/// Copilot R1: `deleteContents` / `extractContents` / `insertNode`
/// engine-indep impls perform boundary updates (notably the
/// post-delete collapse to the start point per WHATWG §4.4
/// `deleteContents` step 3) that are NOT recoverable from the
/// mutation hooks alone — `set_text_data` hooks only clamp offsets to
/// the new length, missing the spec-required collapse.  Persist the
/// post-op boundary state explicitly.
fn commit_range_after_mutation(
    ctx: &mut NativeContext<'_>,
    id: RangeId,
    method: &'static str,
    mutated: &Range,
) -> Result<(), VmError> {
    let host = ctx.host();
    let (dom, registry) = host.split_dom_and_live_ranges();
    let applied = registry.with_range_mut(id, dom, |r, _| {
        r.start_container = mutated.start_container;
        r.start_offset = mutated.start_offset;
        r.end_container = mutated.end_container;
        r.end_offset = mutated.end_offset;
    });
    if applied.is_none() {
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_invalid_state_error,
            format!(
                "Failed to execute '{method}' on 'Range': \
                 the Range has been detached"
            ),
        ));
    }
    Ok(())
}

fn native_range_delete_contents(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_range_receiver(ctx, this, "deleteContents")?;
    // Mutating Range methods need `&mut EcsDom` + `&mut LiveRangeRegistry`
    // — the with_range_mut split gives `&EcsDom` only.  Snapshot the
    // Range, run the deletion through `&mut EcsDom`, then commit the
    // post-op boundary state back to the registry (Copilot R1).
    let mut range = {
        let host = ctx.host();
        let (dom, registry) = host.split_dom_and_live_ranges();
        registry
            .with_range(id, dom, |r, _| r.clone())
            .ok_or_else(|| {
                VmError::dom_exception(
                    ctx.vm.well_known.dom_exc_invalid_state_error,
                    "Failed to execute 'deleteContents' on 'Range': \
                     the Range has been detached",
                )
            })?
    };
    let host = ctx.host();
    let dom = host.dom();
    range.delete_contents(dom);
    commit_range_after_mutation(ctx, id, "deleteContents", &range)?;
    Ok(JsValue::Undefined)
}

fn native_range_extract_contents(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_range_receiver(ctx, this, "extractContents")?;
    let mut range = {
        let host = ctx.host();
        let (dom, registry) = host.split_dom_and_live_ranges();
        registry
            .with_range(id, dom, |r, _| r.clone())
            .ok_or_else(|| {
                VmError::dom_exception(
                    ctx.vm.well_known.dom_exc_invalid_state_error,
                    "Failed to execute 'extractContents' on 'Range': \
                     the Range has been detached",
                )
            })?
    };
    let host = ctx.host();
    let dom = host.dom();
    let fragment = range.extract_contents(dom);
    commit_range_after_mutation(ctx, id, "extractContents", &range)?;
    Ok(JsValue::Object(ctx.vm.create_element_wrapper(fragment)))
}

fn native_range_insert_node(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_range_receiver(ctx, this, "insertNode")?;
    let node = arg_node(ctx, args.first().copied(), "insertNode")?;
    let mut range = {
        let host = ctx.host();
        let (dom, registry) = host.split_dom_and_live_ranges();
        registry
            .with_range(id, dom, |r, _| r.clone())
            .ok_or_else(|| {
                VmError::dom_exception(
                    ctx.vm.well_known.dom_exc_invalid_state_error,
                    "Failed to execute 'insertNode' on 'Range': \
                     the Range has been detached",
                )
            })?
    };
    let host = ctx.host();
    let dom = host.dom();
    range.insert_node(dom, node);
    commit_range_after_mutation(ctx, id, "insertNode", &range)?;
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// Stubs: cloneContents / surroundContents / createContextualFragment
// ---------------------------------------------------------------------------

/// elidex-specific `NotSupportedError` placeholder pending deep-clone
/// infrastructure; spec returns a `DocumentFragment` per WHATWG §4.4.
/// Full impl tracked at `#11-range-full-impl`.
fn native_range_clone_contents(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let _ = require_range_receiver(ctx, this, "cloneContents")?;
    Err(VmError::dom_exception(
        ctx.vm.well_known.dom_exc_not_supported_error,
        "Failed to execute 'cloneContents' on 'Range': \
         deep-clone of Range contents is not yet implemented.",
    ))
}

/// elidex-specific `NotSupportedError` placeholder pending deep-clone
/// infrastructure; spec wraps the contents in `newParent` per WHATWG §4.4.
/// Full impl tracked at `#11-range-full-impl`.
fn native_range_surround_contents(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let _ = require_range_receiver(ctx, this, "surroundContents")?;
    // Validate new_parent shape per spec step 2 (Element / Text / Comment /
    // ProcessingInstruction allowed; Document / DocumentType /
    // DocumentFragment rejected).  Currently unimplemented past brand
    // check.
    let _ = arg_node(ctx, args.first().copied(), "surroundContents")?;
    Err(VmError::dom_exception(
        ctx.vm.well_known.dom_exc_not_supported_error,
        "Failed to execute 'surroundContents' on 'Range': \
         surround-with-parent is not yet implemented.",
    ))
}

/// elidex-specific `NotSupportedError` placeholder pending parser
/// wiring; spec returns a `DocumentFragment` per WHATWG §3.2 step 7.
/// Full impl tracked at `#11-range-full-impl`.
fn native_range_create_contextual_fragment(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let _ = require_range_receiver(ctx, this, "createContextualFragment")?;
    let _ = args.first().copied().unwrap_or(JsValue::Undefined);
    Err(VmError::dom_exception(
        ctx.vm.well_known.dom_exc_not_supported_error,
        "Failed to execute 'createContextualFragment' on 'Range': \
         HTML-parser wiring for fragment parsing is not yet implemented.",
    ))
}
