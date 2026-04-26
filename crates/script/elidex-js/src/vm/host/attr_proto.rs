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
//! Live `Attr` wrappers are interned in
//! [`VmInner::attr_wrapper_cache`] keyed by
//! `(owner Element entity, qualified-name StringId)`, so repeated
//! `getAttributeNode('id')` returns the same `ObjectId` (matches
//! Chrome / Firefox / Safari).  The qualified-name `StringId` is
//! always `intern(get_utf8(...))` — the UTF-8 form the DOM itself
//! stores via [`elidex_ecs::EcsDom::set_attribute`].  Every hit
//! site (`getAttributeNode`, `nnm.{item, getNamedItem,
//! [Symbol.iterator]}`, `nnm[k]`) and every invalidation site
//! agrees on this key shape, so identity holds across all paths
//! even for inputs containing lone surrogates (which collapse to
//! the same UTF-8 representation the DOM stored).
//!
//! The cache is invalidated whenever the named attribute leaves
//! the owner's attribute list (`removeAttribute`,
//! `removeAttributeNode`, `toggleAttribute(off)`,
//! `removeNamedItem`); subsequent `setAttribute` allocates a fresh
//! wrapper for the new attribute, distinct from any wrapper the
//! caller still holds for the prior incarnation.  *Detached*
//! wrappers (`detached_value == Some(_)`) are never cached — each
//! detachment site allocates its own snapshot wrapper.
//!
//! `setAttributeNode` / `setNamedItem` invalidate the cache only
//! when the passed-in Attr cannot remain canonical for the target
//! `(owner, qname)` pair — that is, when the Attr's
//! `AttrState.owner` differs from the receiving element or when
//! the Attr is detached.  Live Attrs already attached to the
//! receiving element are left in place, so
//! `el.setAttributeNode(el.getAttributeNode("id"))` preserves
//! identity.  Cross-element or detached arguments cannot be
//! retargeted (the engine path does not mutate the passed-in
//! Attr's `AttrState.owner`), so the cache drops the entry and
//! the next `getAttributeNode` allocates a fresh wrapper — same
//! Deferred #21 bucket as the namespace work below.
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
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyStorage, VmError,
};
use super::super::{NativeFn, StringId, VmInner};

/// `(owner Element, qualified-name StringId, detached snapshot)`
/// tuple identifying a single attribute.  Qualified-name storage
/// is the caller's responsibility — `NamedNodeMap` normalises on
/// HTML documents to lowercase via `EcsDom::get_attribute` before
/// insertion here.
///
/// ## Live vs detached
///
/// An `AttrState` with `detached_value == None` is *live*: accessors
/// read the current value from the owner Element's `Attributes`
/// component on each call, and `ownerElement` returns the owner as
/// long as the attribute is still present.
///
/// An `AttrState` with `detached_value == Some(sid)` is *detached*:
/// `value` returns the snapshot `sid` and `ownerElement` returns
/// `null`.  Detached wrappers are produced by operations that
/// return the "previous / removed" Attr —
/// `Element.removeAttributeNode(attr)`,
/// `Element.setAttributeNode(new)` when replacing,
/// `NamedNodeMap.setNamedItem(new)` when replacing,
/// `NamedNodeMap.removeNamedItem(name)`.  Detaching captures the
/// value at detach-time so a subsequent same-name `setAttribute` on
/// the former owner cannot make the detached wrapper appear to
/// "re-attach" (WHATWG §4.9.2 semantics).
pub(crate) struct AttrState {
    /// The Element that owned this attribute when the wrapper was
    /// allocated.  For live wrappers, `ownerElement` returns this
    /// entity when the attribute is still present on the owner;
    /// otherwise `null` (the attribute was removed via an API that
    /// did not flow through this wrapper's detachment path).  For
    /// detached wrappers, `ownerElement` unconditionally returns
    /// `null` regardless of the owner's current attribute set.
    pub(crate) owner: Entity,
    /// The qualified attribute name (e.g. `"id"`, `"data-foo"`).
    /// Pre-interned so `name` / `localName` / `NamedNodeMap`
    /// round-trips hit the same pool entry.
    pub(crate) qualified_name: StringId,
    /// Snapshot value captured at detach time.  `None` → live
    /// wrapper (accessors read the owner's current attribute
    /// value).  `Some(sid)` → detached wrapper (accessors return
    /// the snapshot; `ownerElement` returns `null`).
    pub(crate) detached_value: Option<StringId>,
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
            self.install_accessor_pair(
                proto_id,
                name_sid,
                getter,
                None,
                shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }
        // `value` is read/write (§4.9.2).
        self.install_accessor_pair(
            proto_id,
            self.well_known.value,
            native_attr_get_value,
            Some(native_attr_set_value),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
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

    /// Identity-preserving allocation for *live* `Attr` wrappers
    /// (WHATWG DOM §4.9.2).  Returns the cached `ObjectId` for
    /// `(owner, qualified_name)` if one already exists, otherwise
    /// allocates a fresh wrapper via [`Self::alloc_attr`] and
    /// caches it.  Callers must only invoke this for attributes
    /// known to be present on the owner — detached wrappers are
    /// never cached and must go through [`Self::alloc_attr`]
    /// directly with `detached_value: Some(_)`.
    pub(crate) fn cached_or_alloc_attr_live(
        &mut self,
        owner: Entity,
        qualified_name: StringId,
    ) -> ObjectId {
        if let Some(&id) = self.attr_wrapper_cache.get(&(owner, qualified_name)) {
            return id;
        }
        let id = self.alloc_attr(AttrState {
            owner,
            qualified_name,
            detached_value: None,
        });
        self.attr_wrapper_cache.insert((owner, qualified_name), id);
        id
    }

    /// Drop the cached `Attr` wrapper for `(owner, qualified_name)`
    /// when the named attribute is leaving the owner's attribute
    /// list (`removeAttribute` / `removeAttributeNode` /
    /// `toggleAttribute(off)` / `removeNamedItem` /
    /// `setAttributeNode` / `setNamedItem`).  A subsequent
    /// `getAttributeNode` for the same name allocates a fresh
    /// wrapper distinct from any caller-held handle to the prior
    /// incarnation.  No-op when no entry exists.
    pub(crate) fn invalidate_attr_cache_entry(&mut self, owner: Entity, qualified_name: StringId) {
        self.attr_wrapper_cache.remove(&(owner, qualified_name));
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
    // Detached wrappers always report `null` — the snapshot is
    // frozen and ignores any subsequent same-name mutations on
    // the former owner.
    if state.detached_value.is_some() {
        return Ok(JsValue::Null);
    }
    let owner = state.owner;
    let qname = state.qualified_name;
    // Resolve the attribute name outside the ECS borrow.
    let name_str = ctx.vm.strings.get_utf8(qname);
    // Post-unbind: treat as detached — return `null` rather than
    // panicking via `HostData::dom()` is_bound assert.
    let still_attached = ctx
        .host_if_bound()
        .is_some_and(|host| host.dom().has_attribute(owner, &name_str));
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
    // Detached wrappers return the snapshot captured at detach
    // time, regardless of any post-detachment state on the former
    // owner.
    if let Some(snapshot_sid) = state.detached_value {
        return Ok(JsValue::String(snapshot_sid));
    }
    let owner = state.owner;
    let qname = state.qualified_name;
    let empty = ctx.vm.well_known.empty;
    // Post-unbind live Attr: report empty string rather than
    // panicking via `HostData::dom()` is_bound assert.  Split the
    // dom + strings borrow so the borrowed value can be interned
    // without the per-call `String::from` clone the owned getter
    // would allocate.
    let sid = match ctx.dom_and_strings_if_bound() {
        Some((dom, strings)) => {
            let name_str = strings.get_utf8(qname);
            dom.with_attribute(owner, &name_str, |v| match v {
                Some("") | None => empty,
                Some(s) => strings.intern(s),
            })
        }
        None => empty,
    };
    Ok(JsValue::String(sid))
}

fn native_attr_set_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (attr_id, state) = require_attr_receiver(ctx, this, "value")?;
    let owner = state.owner;
    let qname = state.qualified_name;
    let is_detached = state.detached_value.is_some();
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let value_sid = super::super::coerce::to_string(ctx.vm, val)?;
    // Detached wrappers update the snapshot in place (WHATWG §4.9.2
    // "change an attribute" on a detached Attr mutates the Attr's
    // value but does NOT reach the former owner).  Live wrappers
    // with a still-attached backing attribute write through to the
    // owner's `Attributes` component.  Live wrappers whose
    // attribute was already removed are ignored — this avoids
    // silently re-attaching a removed attr.
    if is_detached {
        if let Some(state_mut) = ctx.vm.attr_states.get_mut(&attr_id) {
            state_mut.detached_value = Some(value_sid);
        }
        return Ok(JsValue::Undefined);
    }
    let name_str = ctx.vm.strings.get_utf8(qname);
    // Post-unbind live Attr: setter is a no-op — matches the
    // detached-Attr doc ("setter does not reach the former
    // owner") and avoids panicking via `HostData::dom()`.
    let Some(host) = ctx.host_if_bound() else {
        return Ok(JsValue::Undefined);
    };
    let attached = host.dom().has_attribute(owner, &name_str);
    if attached {
        let new_value = ctx.vm.strings.get_utf8(value_sid);
        // Re-borrow `ctx.host_if_bound()` since we need a fresh
        // `&mut` after the shared-read above.
        if let Some(host) = ctx.host_if_bound() {
            host.dom().set_attribute(owner, &name_str, new_value);
        }
    }
    Ok(JsValue::Undefined)
}
