// boa skip: VM-only surface; the legacy boa runtime
// (`crates/script/elidex-js-boa/`) is on the deletion path per
// `m4-12-platform-gap-roadmap.md` §E-2 Round 20 PR7.  See
// `memory/project_boa_runtime_deletion.md`.

//! `StaticRange` interface (WHATWG DOM §5.4) — eager / immutable
//! AbstractRange holder.
//!
//! ## Layering
//!
//! `StaticRange` is **not** registered in
//! [`elidex_dom_api::LiveRangeRegistry`] (spec §5.4 deliberately
//! omits live-range tracking).  The four boundary fields are stored
//! inline in [`super::super::value::ObjectKind::StaticRange`] as
//! [`Entity`] bits + offsets; the wrapper has no other state.
//!
//! ## Constructor — eager validation
//!
//! `new StaticRange(init)` (spec §5.4 step 1) eagerly throws
//! `InvalidNodeTypeError` if either container is a `DocumentType` or
//! `Attr`.  Offsets are coerced via WebIDL `unsigned long`
//! (`coerce::to_uint32`) per the AbstractRange dictionary.
//!
//! ## `isValid()` — lazy validation
//!
//! `staticRange.isValid()` (spec §5.4 *is valid* criteria) returns true only if
//! all of:
//! - both containers share a root,
//! - `startOffset ≤ length(startContainer)`,
//! - `endOffset ≤ length(endContainer)`, and
//! - `(startContainer, startOffset)` is before-or-equal
//!   `(endContainer, endOffset)`.

#![cfg(feature = "engine")]

use elidex_dom_api::range::Range;
use elidex_ecs::{Attributes, DocTypeData, Entity, NodeKind};

use super::super::shape;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectKind, PropertyKey, PropertyStorage, PropertyValue,
    VmError,
};
use super::super::{NativeFn, VmInner};

impl VmInner {
    /// Allocate `StaticRange.prototype` chained to `Object.prototype`
    /// and expose the `StaticRange` constructor on `globalThis`.
    pub(in crate::vm) fn register_static_range_global(&mut self) {
        let object_proto = self
            .object_prototype
            .expect("register_static_range_global called before register_prototypes");

        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });

        // 4 readonly accessors per AbstractRange + 1 `collapsed`
        // computed from the 4 + `isValid()` method.
        let accessors: [(_, NativeFn); 5] = [
            (
                self.well_known.start_container,
                native_static_range_get_start_container as NativeFn,
            ),
            (
                self.well_known.start_offset,
                native_static_range_get_start_offset,
            ),
            (
                self.well_known.end_container,
                native_static_range_get_end_container,
            ),
            (
                self.well_known.end_offset,
                native_static_range_get_end_offset,
            ),
            (
                self.well_known.collapsed_attr,
                native_static_range_get_collapsed,
            ),
        ];
        let attrs = shape::PropertyAttrs::WEBIDL_RO_ACCESSOR;
        for (name_sid, getter) in accessors {
            self.install_accessor_pair(proto_id, name_sid, getter, None, attrs);
        }

        // `isValid()` — spec §5.4 *is valid* criteria.  Method (data property)
        // rather than accessor per WebIDL signature.  No SID dedicated
        // — intern at install time.
        let is_valid_sid = self.strings.intern("isValid");
        self.install_native_method(
            proto_id,
            is_valid_sid,
            native_static_range_is_valid,
            shape::PropertyAttrs::METHOD,
        );

        self.static_range_prototype = Some(proto_id);

        let ctor =
            self.create_constructor_only_function("StaticRange", native_static_range_constructor);
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
        let name_sid = self.well_known.static_range_global;
        self.globals.insert(name_sid, JsValue::Object(ctor));
    }
}

// ---------------------------------------------------------------------------
// Brand check
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct StaticRangeFields {
    start_container: Entity,
    start_offset: u32,
    end_container: Entity,
    end_offset: u32,
    bind_epoch: u32,
}

fn require_static_range_receiver(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &'static str,
) -> Result<StaticRangeFields, VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'StaticRange': Illegal invocation"
        )));
    };
    let ObjectKind::StaticRange {
        start_container_bits,
        start_offset,
        end_container_bits,
        end_offset,
        bind_epoch,
    } = ctx.vm.get_object(id).kind
    else {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'StaticRange': Illegal invocation"
        )));
    };
    let start_container = Entity::from_bits(start_container_bits).ok_or_else(|| {
        VmError::type_error(format!(
            "Failed to execute '{method}' on 'StaticRange': start container is detached"
        ))
    })?;
    let end_container = Entity::from_bits(end_container_bits).ok_or_else(|| {
        VmError::type_error(format!(
            "Failed to execute '{method}' on 'StaticRange': end container is detached"
        ))
    })?;
    Ok(StaticRangeFields {
        start_container,
        start_offset,
        end_container,
        end_offset,
        bind_epoch,
    })
}

// ---------------------------------------------------------------------------
// Constructor — eager validation
// ---------------------------------------------------------------------------

/// `new StaticRange(init)` (WHATWG DOM §5.4).
///
/// Eager step 1: throw `InvalidNodeTypeError` if `init.startContainer`
/// or `init.endContainer` is a `DocumentType` or `Attr`.  Spec
/// AbstractRange dictionary fields are required (no defaults).
fn native_static_range_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let JsValue::Object(this_id) = this else {
        unreachable!("constructor `this` is always an Object after `do_new`");
    };
    // Copilot R6: `host_opt()` is true after `unbind()` (HostData
    // still installed, but `dom_ptr` is null).  Use `host_if_bound`
    // so post-unbind construction returns a JS error before the
    // subsequent `dom()` access (in `require_node_arg` /
    // `reject_invalid_container`) panics.
    if ctx.host_if_bound().is_none() {
        return Err(VmError::type_error(
            "Failed to construct 'StaticRange': host environment is not initialised",
        ));
    }
    let init_val = args.first().copied().unwrap_or(JsValue::Undefined);
    let JsValue::Object(init_id) = init_val else {
        return Err(VmError::type_error(
            "Failed to construct 'StaticRange': parameter 1 is not an object.",
        ));
    };

    let start_container_sid = ctx.vm.well_known.start_container;
    let start_offset_sid = ctx.vm.well_known.start_offset;
    let end_container_sid = ctx.vm.well_known.end_container;
    let end_offset_sid = ctx.vm.well_known.end_offset;

    // Copilot R3: WebIDL `AbstractRangeInit` dictionary lists all 4
    // members as `required`.  Per WebIDL §3.10.7 step 4, the
    // required check is by PRESENCE (HasProperty), not by value —
    // an explicit `startOffset: undefined` is "present" and goes
    // through ToUint32 → 0 normally.  Copilot R9 corrects R3:
    // distinguish absent (throw) from present-but-undefined (coerce).
    let start_container_val =
        require_dict_member(ctx, init_id, start_container_sid, "startContainer")?;
    let start_offset_val = require_dict_member(ctx, init_id, start_offset_sid, "startOffset")?;
    let end_container_val = require_dict_member(ctx, init_id, end_container_sid, "endContainer")?;
    let end_offset_val = require_dict_member(ctx, init_id, end_offset_sid, "endOffset")?;

    let start_container =
        super::node_proto::require_node_arg(ctx, start_container_val, "StaticRange")?;
    let end_container = super::node_proto::require_node_arg(ctx, end_container_val, "StaticRange")?;
    let start_offset = super::super::coerce::to_uint32(ctx.vm, start_offset_val)?;
    let end_offset = super::super::coerce::to_uint32(ctx.vm, end_offset_val)?;

    reject_invalid_container(ctx, start_container)?;
    reject_invalid_container(ctx, end_container)?;

    // Copilot R9: capture current bind epoch so retained instances
    // across `Vm::unbind`/rebind invalidate via `isValid()`.
    let bind_epoch = ctx.host().bind_epoch();
    ctx.vm.get_object_mut(this_id).kind = ObjectKind::StaticRange {
        start_container_bits: start_container.to_bits().into(),
        start_offset,
        end_container_bits: end_container.to_bits().into(),
        end_offset,
        bind_epoch,
    };
    Ok(JsValue::Object(this_id))
}

/// WebIDL §3.10.7 step 4 — required-dictionary-member lookup.
/// `HasProperty` check precedes `Get`: a member explicitly set to
/// `undefined` IS "present" (proceeds through type conversion); a
/// missing key throws TypeError.  Copilot R9 fix.
fn require_dict_member(
    ctx: &mut NativeContext<'_>,
    obj_id: super::super::value::ObjectId,
    key_sid: super::super::value::StringId,
    member_name: &'static str,
) -> Result<JsValue, VmError> {
    let key = PropertyKey::String(key_sid);
    match ctx.try_get_property_value(obj_id, key)? {
        Some(v) => Ok(v),
        None => Err(VmError::type_error(format!(
            "Failed to construct 'StaticRange': required member '{member_name}' is missing."
        ))),
    }
}

/// Spec §5.4 step 1 — reject `DocumentType` / `Attr` container.
fn reject_invalid_container(ctx: &mut NativeContext<'_>, container: Entity) -> Result<(), VmError> {
    let dom = ctx.host().dom();
    let is_doctype = matches!(
        dom.node_kind_inferred(container),
        Some(NodeKind::DocumentType)
    ) || dom.world().get::<&DocTypeData>(container).is_ok();
    // `Attr` nodes are stored via the `Attributes` component on their
    // owner element + tracked as a separate Entity for the Attr
    // wrapper.  Detect by presence of a parent-less node whose role
    // is Attr — currently elidex represents Attr only as wrappers
    // around `(owner, name)` pairs, not as standalone Entities.
    // Therefore the Attr branch is a forward-stub: no Entity in
    // current elidex storage can fail this check, but the doc-comment
    // documents the spec intent so a future Attr-entity migration
    // does not regress §5.4 step 1.
    let _ = Attributes::default;
    if is_doctype {
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_invalid_node_type_error,
            "Failed to construct 'StaticRange': \
             the container is a DocumentType.",
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Accessors
// ---------------------------------------------------------------------------

fn native_static_range_get_start_container(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let f = require_static_range_receiver(ctx, this, "startContainer")?;
    Ok(JsValue::Object(
        ctx.vm.create_element_wrapper(f.start_container),
    ))
}

fn native_static_range_get_start_offset(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let f = require_static_range_receiver(ctx, this, "startOffset")?;
    Ok(JsValue::Number(f64::from(f.start_offset)))
}

fn native_static_range_get_end_container(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let f = require_static_range_receiver(ctx, this, "endContainer")?;
    Ok(JsValue::Object(
        ctx.vm.create_element_wrapper(f.end_container),
    ))
}

fn native_static_range_get_end_offset(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let f = require_static_range_receiver(ctx, this, "endOffset")?;
    Ok(JsValue::Number(f64::from(f.end_offset)))
}

fn native_static_range_get_collapsed(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let f = require_static_range_receiver(ctx, this, "collapsed")?;
    Ok(JsValue::Boolean(
        f.start_container == f.end_container && f.start_offset == f.end_offset,
    ))
}

// ---------------------------------------------------------------------------
// isValid()
// ---------------------------------------------------------------------------

fn native_static_range_is_valid(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let f = require_static_range_receiver(ctx, this, "isValid")?;
    // Copilot R9: a `Vm::unbind`/rebind cycle invalidates ALL
    // retained StaticRange instances because the stored
    // `Entity` bits may now resolve to an unrelated entity in
    // the fresh `EcsDom`.  Reject if the captured epoch differs
    // from the current bind epoch — even if `dom.contains`
    // happens to succeed against a recycled slot.
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Boolean(false));
    }
    if f.bind_epoch != ctx.host().bind_epoch() {
        return Ok(JsValue::Boolean(false));
    }
    let dom = ctx.host().dom();
    // Copilot R2: a stored Entity may have been despawned since
    // construction.  `dom.contains` is the canonical liveness
    // check — combined with the bind-epoch guard above, this
    // rejects both cross-rebind stale entities and same-bind
    // despawned ones.
    if !dom.contains(f.start_container) || !dom.contains(f.end_container) {
        return Ok(JsValue::Boolean(false));
    }
    // Clause (a): both containers must share a root.  Detached
    // containers (no longer in the tree) return false because
    // `find_tree_root` returns the entity itself for orphans, so two
    // unrelated orphans never match.
    if dom.find_tree_root(f.start_container) != dom.find_tree_root(f.end_container) {
        return Ok(JsValue::Boolean(false));
    }
    // Clause (b) + (c): offsets in bounds. Use the canonical engine-indep
    // `node_length` (WHATWG DOM §4.2 "length of a node") so StaticRange
    // bounds-checking agrees with the live Range path — incl. the
    // CharacterData (Text / CDATASection / Comment) data-length cases.
    if (f.start_offset as usize) > elidex_dom_api::range::node_length(f.start_container, dom) {
        return Ok(JsValue::Boolean(false));
    }
    if (f.end_offset as usize) > elidex_dom_api::range::node_length(f.end_container, dom) {
        return Ok(JsValue::Boolean(false));
    }
    // Clause (d): start ≤ end via boundary compare.  Use a transient
    // [`Range`] to leverage the existing `compare_boundary_points`
    // — START_TO_START on (self, self) compares start vs start,
    // but we want start vs end, so use `compare_point` style via
    // a synthesized Range whose start = (start_container, start_offset)
    // and end = (end_container, end_offset).  This route also handles
    // the cross-tree case correctly because `compare_points` falls
    // through to `tree_order_cmp`.
    let r = build_static_range(&f);
    // Strict "start point is BEFORE OR EQUAL to end point" check —
    // use the start->start vs end->end primitive: ordering relation
    // `compare_boundary_points(START_TO_END, other = self)` compares
    // self.start vs self.end and returns -1, 0, or 1.  spec is
    // `0 <= ordering <= 0` for an equal collapsed range, `< 0` for
    // a valid forward range, `> 0` for an inverted (invalid) range.
    let ordering = r.compare_boundary_points(elidex_dom_api::range::START_TO_END, &r, dom);
    Ok(JsValue::Boolean(ordering <= 0))
}

fn build_static_range(f: &StaticRangeFields) -> Range {
    let mut r = Range::new(f.start_container);
    r.start_container = f.start_container;
    r.start_offset = f.start_offset as usize;
    r.end_container = f.end_container;
    r.end_offset = f.end_offset as usize;
    r
}
