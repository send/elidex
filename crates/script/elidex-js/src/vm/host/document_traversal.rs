// boa skip: VM-only surface; the legacy boa runtime
// (`crates/script/elidex-js-boa/`) is on the deletion path per
// `m4-12-platform-gap-roadmap.md` §E-2 Round 20 PR7.  See
// `memory/project_boa_runtime_deletion.md`.

//! `document.createRange` / `createTreeWalker` / `createNodeIterator`
//! factory methods (WHATWG DOM §4.4 / §6.4 / §6.1).
//!
//! ## Layering
//!
//! Each factory:
//!
//! 1. Brand-checks `this` as Document (handled by call-site of
//!    `DOCUMENT_METHODS` table — `native_*` entries already see a
//!    `document_receiver`-validated entity).
//! 2. Coerces args via WebIDL `to_uint32` / `require_node_arg`.
//! 3. Allocates engine-side state (`Range` / `TreeWalkerState` /
//!    `NodeIteratorState`) + a fresh wrapper, populates side tables.
//! 4. Returns the wrapper.
//!
//! No traversal algorithm here — those live in
//! [`elidex_dom_api::traversal`].

#![cfg(feature = "engine")]

use elidex_dom_api::{NodeIteratorState, Range};
use elidex_ecs::Entity;

use super::super::shape;
use super::super::value::{JsValue, NativeContext, Object, ObjectKind, PropertyStorage, VmError};

/// Strict WebIDL brand check for the document factory methods.
/// Copilot R16: `document_receiver` returns `Ok(None)` for any
/// non-HostObject receiver (so it can also cover the retained-
/// HostObject-after-`Vm::unbind` case), but WebIDL §3.7 mandates a
/// `TypeError` "Illegal invocation" for `document.createRange.call({})`
/// and similar misuse on a plain Object / primitive.  Pre-filter
/// those cases before deferring to the standard `document_receiver`
/// (which still handles the kind mismatch + unbound paths).
fn require_document_factory_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    let illegal = || {
        VmError::type_error(format!(
            "Failed to execute '{method}' on 'Document': Illegal invocation"
        ))
    };
    match this {
        JsValue::Object(id) => {
            if !matches!(ctx.vm.get_object(id).kind, ObjectKind::HostObject { .. }) {
                return Err(illegal());
            }
        }
        _ => return Err(illegal()),
    }
    // Unbound retained-HostObject case: `document_receiver` would
    // panic on `ctx.host().dom()`; gate before calling.
    if ctx.host_if_bound().is_none() {
        return Ok(None);
    }
    super::document::document_receiver(ctx, this, method)
}

/// `document.createRange()` — WHATWG DOM §4.4 step 3.
///
/// Returns a new live Range collapsed at `(receiver document, 0)` with
/// `owner_document = receiver document`.  WebIDL brand check rejects
/// non-Document receivers (Copilot R2): `document.createRange.call({})`
/// throws `TypeError`.
pub(super) fn native_document_create_range(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(doc) = require_document_factory_receiver(ctx, this, "createRange")? else {
        return Ok(JsValue::Null);
    };
    let range = Range::new_with_owner(doc, doc);
    let range_id = ctx.host().live_range_registry.register(range);
    let proto = ctx
        .vm
        .range_prototype
        .expect("Range.prototype installed during register_globals");
    let wrapper = ctx.vm.alloc_object(Object {
        kind: ObjectKind::Range {
            range_id: range_id.0,
        },
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: Some(proto),
        extensible: true,
    });
    ctx.host().range_instances.insert(range_id.0, wrapper);
    Ok(JsValue::Object(wrapper))
}

/// `document.createTreeWalker(root, whatToShow = SHOW_ALL, filter = null)`
/// — WHATWG DOM §6.4 createTreeWalker step.  WebIDL brand check
/// rejects non-Document receivers (Copilot R2).
pub(super) fn native_document_create_tree_walker(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if require_document_factory_receiver(ctx, this, "createTreeWalker")?.is_none() {
        return Ok(JsValue::Null);
    }
    let root_val = args.first().copied().unwrap_or(JsValue::Undefined);
    let root = super::node_proto::require_node_arg(ctx, root_val, "createTreeWalker")?;
    let what_to_show = parse_what_to_show(ctx, args.get(1).copied())?;
    let filter_id = parse_filter(args.get(2).copied(), "createTreeWalker")?;

    let proto = ctx
        .vm
        .tree_walker_prototype
        .expect("TreeWalker.prototype installed during register_globals");

    let walker_id = {
        let host = ctx.host();
        let id = host.next_tree_walker_id;
        host.next_tree_walker_id = host
            .next_tree_walker_id
            .checked_add(1)
            .expect("TreeWalker ID overflow");
        host.tree_walker_states.insert(
            id,
            crate::vm::host_data::TreeWalkerState {
                root,
                what_to_show,
                filter_object_id: filter_id,
                current: root,
                active: false,
            },
        );
        id
    };

    let wrapper = ctx.vm.alloc_object(Object {
        kind: ObjectKind::TreeWalker { walker_id },
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: Some(proto),
        extensible: true,
    });
    ctx.host().tree_walker_instances.insert(walker_id, wrapper);
    Ok(JsValue::Object(wrapper))
}

/// `document.createNodeIterator(root, whatToShow = SHOW_ALL, filter = null)`
/// — WHATWG DOM §6.1 createNodeIterator step.  WebIDL brand check
/// rejects non-Document receivers (Copilot R2).
pub(super) fn native_document_create_node_iterator(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if require_document_factory_receiver(ctx, this, "createNodeIterator")?.is_none() {
        return Ok(JsValue::Null);
    }
    let root_val = args.first().copied().unwrap_or(JsValue::Undefined);
    let root = super::node_proto::require_node_arg(ctx, root_val, "createNodeIterator")?;
    let what_to_show = parse_what_to_show(ctx, args.get(1).copied())?;
    let filter_id = parse_filter(args.get(2).copied(), "createNodeIterator")?;

    let proto = ctx
        .vm
        .node_iterator_prototype
        .expect("NodeIterator.prototype installed during register_globals");

    let iterator_id = {
        let host = ctx.host();
        let id = host.next_node_iterator_id;
        host.next_node_iterator_id = host
            .next_node_iterator_id
            .checked_add(1)
            .expect("NodeIterator ID overflow");
        let state = NodeIteratorState {
            root,
            what_to_show,
            filter_object_id: filter_id,
            reference: root,
            pointer_before: true,
            active: false,
        };
        host.node_iterator_states_shared
            .lock()
            .expect("NodeIterator state mutex poisoned")
            .insert(id, state);
        id
    };

    let wrapper = ctx.vm.alloc_object(Object {
        kind: ObjectKind::NodeIterator { iterator_id },
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: Some(proto),
        extensible: true,
    });
    ctx.host()
        .node_iterator_instances
        .insert(iterator_id, wrapper);
    Ok(JsValue::Object(wrapper))
}

/// Parse the `whatToShow` arg with WebIDL `unsigned long` coercion +
/// default `SHOW_ALL` when absent.
fn parse_what_to_show(ctx: &mut NativeContext<'_>, arg: Option<JsValue>) -> Result<u32, VmError> {
    match arg {
        None | Some(JsValue::Undefined) => Ok(elidex_dom_api::traversal::SHOW_ALL),
        Some(v) => super::super::coerce::to_uint32(ctx.vm, v),
    }
}

/// Parse the `filter` arg per WebIDL `NodeFilter?` (nullable callback
/// interface).  Acceptable shapes:
/// - missing / `undefined` / `null` → `None` (no filter)
/// - Object → captured as opaque ObjectId bits; `acceptNode` lookup
///   happens lazily at dispatch time per
///   `node_filter_dispatch::pick_callable`
/// - any non-nullish primitive (Boolean / Number / BigInt / String /
///   Symbol) → `TypeError` per WebIDL §3.10 callback interface
///   conversion ("If Type(V) is not Object, then throw a TypeError")
///
/// Copilot R17: the previous impl swallowed primitives as `None`,
/// silently creating an unfiltered walker for `createTreeWalker(root,
/// SHOW_ALL, 42)` instead of throwing.
fn parse_filter(arg: Option<JsValue>, method: &str) -> Result<Option<u64>, VmError> {
    match arg {
        None | Some(JsValue::Undefined | JsValue::Null) => Ok(None),
        Some(JsValue::Object(id)) => Ok(Some(u64::from(id.0))),
        Some(_) => Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'Document': \
             parameter 3 is not of type 'NodeFilter'."
        ))),
    }
}

/// Wrapper helpers — these are wired into the `DOCUMENT_METHODS`
/// table in `document.rs` so call routing happens through the
/// existing `document_receiver` brand check.
pub(super) const FACTORIES: &[(&str, super::super::NativeFn)] = &[
    ("createRange", native_document_create_range),
    ("createTreeWalker", native_document_create_tree_walker),
    ("createNodeIterator", native_document_create_node_iterator),
];
