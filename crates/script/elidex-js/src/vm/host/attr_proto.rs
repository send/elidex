//! `Attr.prototype` intrinsic + `Attr` wrapper allocation
//! (WHATWG DOM §4.9.2).
//!
//! `Attr` represents a single attribute on an Element — the
//! wrapper returned by `getAttributeNode` /
//! `NamedNodeMap.{item, getNamedItem}` and the argument type
//! accepted by `setAttributeNode` / `NamedNodeMap.setNamedItem`.
//!
//! ## Backing state
//!
//! `ObjectKind::Attr` is payload-free; the (owner Element,
//! qualified-name) tuple lives in [`VmInner::attr_states`] keyed
//! by this `ObjectId`.  Attribute values are stored on the owner's
//! `Attributes` ECS component — we never duplicate them here.
//! Removing the attribute via `element.removeAttribute(name)`
//! leaves the Attr wrapper intact but its `ownerElement` returns
//! `null` and `value` returns `""`.
//!
//! ## Identity
//!
//! Identity is **not** preserved across calls: repeated
//! `getAttributeNode('id')` allocates a fresh wrapper.  This
//! mirrors HTMLCollection / NodeList / NamedNodeMap and avoids the
//! GC root machinery a wrapper cache would demand.  Consumers that
//! need a stable handle can cache the wrapper on the JS side.
//!
//! ## Phase 2 simplification
//!
//! `namespaceURI` / `prefix` return `null` for every Attr;
//! `localName` equals the qualified name.  Full XML namespace
//! support lands in Phase 3 alongside XML document handling
//! (`m4-12-pr5b.md` Deferred #21).

#![cfg(feature = "engine")]

use elidex_ecs::Entity;

use super::super::shape;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, VmError,
};
use super::super::{NativeFn, StringId, VmInner};

/// `(owner Element, qualified-name StringId)` tuple identifying a
/// single attribute.  Qualified-name storage is the caller's
/// responsibility — `NamedNodeMap` normalises on HTML documents to
/// lowercase via `EcsDom::get_attribute` before insertion here.
pub(crate) struct AttrState {
    /// The Element that owned this attribute when the wrapper was
    /// allocated.  May have since had the attribute removed — the
    /// `ownerElement` accessor treats a missing entry in the
    /// Element's `Attributes` component as "detached" and reports
    /// `null` per WHATWG §4.9.2.
    pub(crate) owner: Entity,
    /// The qualified attribute name (e.g. `"id"`, `"data-foo"`).
    /// Pre-interned so `name` / `localName` / `NamedNodeMap`
    /// round-trips hit the same pool entry.
    pub(crate) qualified_name: StringId,
}

impl VmInner {
    /// Allocate `Attr.prototype` with `Object.prototype` as parent
    /// and install the `name` / `value` / `ownerElement` /
    /// `namespaceURI` / `prefix` / `localName` / `specified`
    /// accessor suite (WHATWG §4.9.2).
    pub(in crate::vm) fn register_attr_prototype(&mut self) {
        let obj_proto = self.object_prototype;
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: obj_proto,
            extensible: true,
        });
        self.attr_prototype = Some(proto_id);

        // Read-only accessors.
        for (name_sid, getter) in [
            (self.well_known.name, native_attr_get_name as NativeFn),
            (self.well_known.owner_element, native_attr_get_owner_element),
            (self.well_known.namespace_uri, native_attr_get_namespace_uri),
            (self.well_known.prefix, native_attr_get_prefix),
            (self.well_known.local_name, native_attr_get_local_name),
            (self.well_known.specified, native_attr_get_specified),
        ] {
            self.install_attr_ro_accessor(proto_id, name_sid, getter);
        }
        // `value` is read/write (§4.9.2).
        self.install_attr_rw_accessor(
            proto_id,
            self.well_known.value,
            native_attr_get_value,
            native_attr_set_value,
        );
    }

    /// Allocate a fresh `Attr` wrapper backed by `state`.  Called
    /// from NamedNodeMap and `element.getAttributeNode`.
    pub(crate) fn alloc_attr(&mut self, state: AttrState) -> ObjectId {
        let proto = self
            .attr_prototype
            .expect("alloc_attr before register_attr_prototype");
        let id = self.alloc_object(Object {
            kind: ObjectKind::Attr,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(proto),
            extensible: false,
        });
        self.attr_states.insert(id, state);
        id
    }

    fn install_attr_ro_accessor(
        &mut self,
        proto_id: ObjectId,
        name_sid: StringId,
        getter: NativeFn,
    ) {
        let display = self.strings.get_utf8(name_sid);
        let gid = self.create_native_function(&format!("get {display}"), getter);
        self.define_shaped_property(
            proto_id,
            PropertyKey::String(name_sid),
            PropertyValue::Accessor {
                getter: Some(gid),
                setter: None,
            },
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
    }

    fn install_attr_rw_accessor(
        &mut self,
        proto_id: ObjectId,
        name_sid: StringId,
        getter: NativeFn,
        setter: NativeFn,
    ) {
        let display = self.strings.get_utf8(name_sid);
        let gid = self.create_native_function(&format!("get {display}"), getter);
        let sid = self.create_native_function(&format!("set {display}"), setter);
        self.define_shaped_property(
            proto_id,
            PropertyKey::String(name_sid),
            PropertyValue::Accessor {
                getter: Some(gid),
                setter: Some(sid),
            },
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
    }
}

// -------------------------------------------------------------------------
// Brand check
// -------------------------------------------------------------------------

/// Recover the `AttrState` for a receiver, or throw
/// "Illegal invocation" TypeError when the receiver isn't an Attr.
fn require_attr_receiver<'a>(
    ctx: &'a NativeContext<'_>,
    this: JsValue,
    accessor: &'static str,
) -> Result<(ObjectId, &'a AttrState), VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "Failed to execute '{accessor}' on 'Attr': Illegal invocation"
        )));
    };
    if !matches!(ctx.vm.get_object(id).kind, ObjectKind::Attr) {
        return Err(VmError::type_error(format!(
            "Failed to execute '{accessor}' on 'Attr': Illegal invocation"
        )));
    }
    let state = ctx.vm.attr_states.get(&id).ok_or_else(|| {
        VmError::type_error(format!(
            "Failed to execute '{accessor}' on 'Attr': Illegal invocation"
        ))
    })?;
    Ok((id, state))
}

// -------------------------------------------------------------------------
// Accessors
// -------------------------------------------------------------------------

fn native_attr_get_name(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (_, state) = require_attr_receiver(ctx, this, "name")?;
    Ok(JsValue::String(state.qualified_name))
}

/// `localName` — Phase 2 simplification returns the same value as
/// `name` (XML namespace parsing lands in Phase 3).
fn native_attr_get_local_name(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (_, state) = require_attr_receiver(ctx, this, "localName")?;
    Ok(JsValue::String(state.qualified_name))
}

/// `namespaceURI` / `prefix` — always `null` under HTML-only
/// handling (plan §Deferred #21).
fn native_attr_get_namespace_uri(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_attr_receiver(ctx, this, "namespaceURI")?;
    Ok(JsValue::Null)
}

fn native_attr_get_prefix(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_attr_receiver(ctx, this, "prefix")?;
    Ok(JsValue::Null)
}

/// `specified` — legacy, always `true` per WHATWG §4.9.2.
fn native_attr_get_specified(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_attr_receiver(ctx, this, "specified")?;
    Ok(JsValue::Boolean(true))
}

fn native_attr_get_owner_element(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (_, state) = require_attr_receiver(ctx, this, "ownerElement")?;
    let owner = state.owner;
    let qname = state.qualified_name;
    // Resolve the attribute name outside the ECS borrow.
    let name_str = ctx.vm.strings.get_utf8(qname);
    let still_attached = ctx.host().dom().get_attribute(owner, &name_str).is_some();
    if still_attached {
        Ok(JsValue::Object(ctx.vm.create_element_wrapper(owner)))
    } else {
        Ok(JsValue::Null)
    }
}

fn native_attr_get_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (_, state) = require_attr_receiver(ctx, this, "value")?;
    let owner = state.owner;
    let qname = state.qualified_name;
    let name_str = ctx.vm.strings.get_utf8(qname);
    let value = ctx
        .host()
        .dom()
        .get_attribute(owner, &name_str)
        .unwrap_or_default();
    let sid = if value.is_empty() {
        ctx.vm.well_known.empty
    } else {
        ctx.vm.strings.intern(&value)
    };
    Ok(JsValue::String(sid))
}

fn native_attr_set_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (_, state) = require_attr_receiver(ctx, this, "value")?;
    let owner = state.owner;
    let qname = state.qualified_name;
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let value_sid = super::super::coerce::to_string(ctx.vm, val)?;
    // Write only when the attribute is still on the element — per
    // §4.9.2 an Attr with `ownerElement === null` ignores `value`
    // writes.  This avoids silently re-attaching a removed attr.
    let name_str = ctx.vm.strings.get_utf8(qname);
    let attached = ctx.host().dom().get_attribute(owner, &name_str).is_some();
    if attached {
        let new_value = ctx.vm.strings.get_utf8(value_sid);
        ctx.host().dom().set_attribute(owner, &name_str, new_value);
    }
    Ok(JsValue::Undefined)
}
