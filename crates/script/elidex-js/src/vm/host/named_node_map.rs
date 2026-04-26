//! `NamedNodeMap.prototype` intrinsic (WHATWG DOM §4.9.1).
//!
//! Backs `element.attributes`, exposing a **live** indexed /
//! named view over the owner Element's `Attributes` ECS component.
//! Mutations through `element.setAttribute` / `removeAttribute` /
//! `NamedNodeMap.{setNamedItem, removeNamedItem}` are visible to
//! previously-obtained NamedNodeMap instances; no caching layer.
//!
//! ## Backing state
//!
//! `ObjectKind::NamedNodeMap` is payload-free; the owner Entity
//! lives in [`VmInner::named_node_map_states`].  Every accessor
//! re-reads the owner's `Attributes` component on demand, keyed
//! by attribute name.  This keeps `NamedNodeMap` semantics
//! aligned with HTMLCollection / NodeList (spec-matching
//! per-access re-resolution, no invalidation surface).
//!
//! ## Phase 2 simplification
//!
//! NS-aware variants follow their WebIDL signatures:
//! `getNamedItemNS(namespace, localName)` and
//! `removeNamedItemNS(namespace, localName)` take an explicit
//! `namespaceURI` string / `null`; only the `null` namespace is
//! supported — any other `namespace` yields `null` / `NotFoundError`
//! respectively.  `setNamedItemNS(attr)` takes only an `Attr` (the
//! namespace lives on the Attr itself); since every Phase 2 Attr
//! has `namespaceURI = null` it is a straight alias for
//! `setNamedItem`.  Full XML namespace handling lands in Phase 3
//! (plan §Deferred #21).
//!
//! ## Brand check
//!
//! Every accessor / method routes through
//! [`require_named_node_map_receiver`]; non-NamedNodeMap
//! receivers throw "Illegal invocation" TypeError, matching
//! WebIDL brand semantics.

#![cfg(feature = "engine")]

use elidex_ecs::{Attributes, EcsDom, Entity};

use super::super::shape;
use super::super::value::ARRAY_ITER_KIND_VALUES;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, VmError,
};
use super::super::{NativeFn, StringId, VmInner};
use super::attr_proto::AttrState;

impl VmInner {
    /// Allocate `NamedNodeMap.prototype` with `Object.prototype`
    /// as parent.  Must run after `register_attr_prototype` (so
    /// NamedNodeMap methods can construct Attr wrappers).
    pub(in crate::vm) fn register_named_node_map_prototype(&mut self) {
        let obj_proto = self.object_prototype;
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: obj_proto,
            extensible: true,
        });
        self.named_node_map_prototype = Some(proto_id);

        // `length` getter.
        self.install_accessor_pair(
            proto_id,
            self.well_known.length,
            native_nnm_length_get,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );

        // Methods.
        for (name_sid, func) in [
            (self.well_known.item, native_nnm_item as NativeFn),
            (self.well_known.get_named_item, native_nnm_get_named_item),
            (self.well_known.set_named_item, native_nnm_set_named_item),
            (
                self.well_known.remove_named_item,
                native_nnm_remove_named_item,
            ),
            (
                self.well_known.get_named_item_ns,
                native_nnm_get_named_item_ns,
            ),
            (
                self.well_known.set_named_item_ns,
                native_nnm_set_named_item_ns,
            ),
            (
                self.well_known.remove_named_item_ns,
                native_nnm_remove_named_item_ns,
            ),
        ] {
            self.install_native_method(proto_id, name_sid, func, shape::PropertyAttrs::METHOD);
        }

        // `[Symbol.iterator]` — values iterator over attr wrappers.
        let iter_fn = self.create_native_function("[Symbol.iterator]", native_nnm_symbol_iterator);
        let iter_key = PropertyKey::Symbol(self.well_known_symbols.iterator);
        self.define_shaped_property(
            proto_id,
            iter_key,
            PropertyValue::Data(JsValue::Object(iter_fn)),
            shape::PropertyAttrs::METHOD,
        );
    }

    /// Allocate a `NamedNodeMap` wrapper for `owner`.
    pub(crate) fn alloc_named_node_map(&mut self, owner: Entity) -> ObjectId {
        let proto = self
            .named_node_map_prototype
            .expect("alloc_named_node_map before register_named_node_map_prototype");
        let id = self.alloc_object(Object {
            kind: ObjectKind::NamedNodeMap,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(proto),
            extensible: false,
        });
        self.named_node_map_states.insert(id, owner);
        id
    }
}

// -------------------------------------------------------------------------
// Brand check
// -------------------------------------------------------------------------

fn require_named_node_map_receiver(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &'static str,
) -> Result<(ObjectId, Entity), VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'NamedNodeMap': Illegal invocation"
        )));
    };
    if !matches!(ctx.vm.get_object(id).kind, ObjectKind::NamedNodeMap) {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'NamedNodeMap': Illegal invocation"
        )));
    }
    let owner = *ctx.vm.named_node_map_states.get(&id).ok_or_else(|| {
        VmError::type_error(format!(
            "Failed to execute '{method}' on 'NamedNodeMap': Illegal invocation"
        ))
    })?;
    Ok((id, owner))
}

// -------------------------------------------------------------------------
// Attribute-list accessors (shared by every method)
// -------------------------------------------------------------------------

/// Snapshot the owner's attribute names at call time, preserving
/// insertion order.  The Vec is short-lived; HTML documents
/// typically carry a handful of attributes per element, so the
/// per-access clone is negligible.
fn attribute_names_snapshot(dom: &EcsDom, owner: Entity) -> Vec<String> {
    dom.world()
        .get::<&Attributes>(owner)
        .ok()
        .map(|attrs| attrs.iter().map(|(k, _)| k.to_string()).collect())
        .unwrap_or_default()
}

/// Unbind-safe variant of [`attribute_names_snapshot`]: returns an
/// empty `Vec` when the VM has no bound `HostData`.  Required by
/// natives invoked on NamedNodeMap wrappers retained across
/// `Vm::unbind()` — `HostData::dom()` would otherwise assert
/// `is_bound()` and panic.
fn attribute_names_snapshot_if_bound(ctx: &mut NativeContext<'_>, owner: Entity) -> Vec<String> {
    let Some(host) = ctx.host_if_bound() else {
        return Vec::new();
    };
    attribute_names_snapshot(host.dom(), owner)
}

// -------------------------------------------------------------------------
// length
// -------------------------------------------------------------------------

fn native_nnm_length_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (_, owner) = require_named_node_map_receiver(ctx, this, "length")?;
    // Post-unbind access returns `0` — `HostData::dom()` would
    // otherwise assert `is_bound()` and panic.
    let count = ctx.host_if_bound().map_or(0, |host| {
        host.dom()
            .world()
            .get::<&Attributes>(owner)
            .map_or(0, |a| a.iter().count())
    });
    #[allow(clippy::cast_precision_loss)]
    Ok(JsValue::Number(count as f64))
}

// -------------------------------------------------------------------------
// item(index)
// -------------------------------------------------------------------------

fn native_nnm_item(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (_, owner) = require_named_node_map_receiver(ctx, this, "item")?;
    let index = match args.first() {
        Some(JsValue::Number(n)) if n.is_finite() && *n >= 0.0 => {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let idx = n.trunc() as usize;
            idx
        }
        Some(v) => {
            let n = super::super::coerce::to_int32(ctx.vm, *v)?;
            if n < 0 {
                return Ok(JsValue::Null);
            }
            #[allow(clippy::cast_sign_loss)]
            let idx = n as usize;
            idx
        }
        None => return Ok(JsValue::Null),
    };
    let names = attribute_names_snapshot_if_bound(ctx, owner);
    let Some(name) = names.get(index).cloned() else {
        return Ok(JsValue::Null);
    };
    let qname_sid = ctx.vm.strings.intern(&name);
    let attr_id = ctx.vm.alloc_attr(AttrState {
        owner,
        qualified_name: qname_sid,
        detached_value: None,
    });
    Ok(JsValue::Object(attr_id))
}

// -------------------------------------------------------------------------
// getNamedItem / setNamedItem / removeNamedItem
// -------------------------------------------------------------------------

fn native_nnm_get_named_item(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (_, owner) = require_named_node_map_receiver(ctx, this, "getNamedItem")?;
    let key_value = args.first().copied().unwrap_or(JsValue::Undefined);
    let key_sid = super::super::coerce::to_string(ctx.vm, key_value)?;
    let key = ctx.vm.strings.get_utf8(key_sid);
    // Post-unbind lookup returns null — no attribute visible.
    let exists = ctx
        .host_if_bound()
        .is_some_and(|host| host.dom().has_attribute(owner, &key));
    if !exists {
        return Ok(JsValue::Null);
    }
    let qname_sid = ctx.vm.strings.intern(&key);
    let attr_id = ctx.vm.alloc_attr(AttrState {
        owner,
        qualified_name: qname_sid,
        detached_value: None,
    });
    Ok(JsValue::Object(attr_id))
}

/// `setNamedItem(attr)` — stores the Attr's value onto the owner
/// element under the Attr's name (§4.9.1.2 step 3).  Accepts any
/// Attr wrapper; a plain object with a `name` + `value` string
/// pair is *not* valid (unlike the pre-spec polyfill we used to
/// tolerate in boa) — WebIDL requires a true `Attr` argument.
fn native_nnm_set_named_item(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (_, owner) = require_named_node_map_receiver(ctx, this, "setNamedItem")?;
    let arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let JsValue::Object(attr_id) = arg else {
        return Err(VmError::type_error(
            "Failed to execute 'setNamedItem' on 'NamedNodeMap': argument is not an Attr"
                .to_string(),
        ));
    };
    if !matches!(ctx.vm.get_object(attr_id).kind, ObjectKind::Attr) {
        return Err(VmError::type_error(
            "Failed to execute 'setNamedItem' on 'NamedNodeMap': argument is not an Attr"
                .to_string(),
        ));
    }
    let Some(state) = ctx.vm.attr_states.get(&attr_id) else {
        return Err(VmError::type_error(
            "Failed to execute 'setNamedItem' on 'NamedNodeMap': Attr has no backing state"
                .to_string(),
        ));
    };
    let source_owner = state.owner;
    let qname = state.qualified_name;
    let source_detached = state.detached_value;
    // Per §4.9.2, the Attr's observable value is the detached
    // snapshot when present (`Attr.prototype.value` returns that),
    // otherwise the owner's current attribute value keyed by the
    // qualified name.  Mirrors the `value` accessor precedence so
    // `setNamedItem(detachedAttr)` writes the snapshot — not the
    // former owner's empty/stale slot.
    let empty = ctx.vm.well_known.empty;
    // Capture the source value + previous-target-value snapshot in
    // a single split-borrow pass so the prev_value can be interned
    // directly from the borrowed `&str` (no `String::from` clone).
    // Live source: read through the source owner's `Attributes`
    // component; otherwise a `source_detached` snapshot has already
    // been substituted via the `if let Some(...)` arm.  Unbound
    // receivers can't write, so return Null without mutating.
    let outcome = match ctx.dom_and_strings_if_bound() {
        Some((dom, strings)) => {
            let name_str = strings.get_utf8(qname);
            let value = if let Some(snapshot_sid) = source_detached {
                strings.get_utf8(snapshot_sid)
            } else {
                dom.with_attribute(source_owner, &name_str, |v| {
                    v.map(str::to_owned).unwrap_or_default()
                })
            };
            let prev_sid = dom.with_attribute(owner, &name_str, |v| match v {
                Some("") => Some(empty),
                Some(s) => Some(strings.intern(s)),
                None => None,
            });
            Some((name_str, value, prev_sid))
        }
        None => None,
    };
    let Some((name_str, value, prev_sid)) = outcome else {
        return Ok(JsValue::Null);
    };
    // Apply the write through `host_if_bound`.  Both this codebase
    // and the broader VM treat post-unbind access as a recoverable
    // state — match that contract here: if the host happens to be
    // unbound between the snapshot and the write, surface `Null`
    // (no mutation, no "previous" Attr) instead of panicking.  The
    // path is unreachable under current control flow but keeping
    // it recoverable avoids any future-bug embedding crash.
    let Some(host) = ctx.host_if_bound() else {
        return Ok(JsValue::Null);
    };
    host.dom().set_attribute(owner, &name_str, value);
    Ok(match prev_sid {
        Some(sid) => {
            let prev = ctx.vm.alloc_attr(AttrState {
                owner,
                qualified_name: qname,
                detached_value: Some(sid),
            });
            JsValue::Object(prev)
        }
        None => JsValue::Null,
    })
}

fn native_nnm_remove_named_item(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (_, owner) = require_named_node_map_receiver(ctx, this, "removeNamedItem")?;
    let key_value = args.first().copied().unwrap_or(JsValue::Undefined);
    let key_sid = super::super::coerce::to_string(ctx.vm, key_value)?;
    let key = ctx.vm.strings.get_utf8(key_sid);
    let empty = ctx.vm.well_known.empty;
    // Post-unbind: the attribute is not visible → NotFoundError
    // (treat as absent per spec step 3 rather than panicking via
    // `HostData::dom()` is_bound assert).  Snapshot the prior value
    // through the split-borrow path so the intern happens directly
    // on the borrowed `&str`.
    let prev_sid = ctx.dom_and_strings_if_bound().and_then(|(dom, strings)| {
        dom.with_attribute(owner, &key, |v| match v {
            Some("") => Some(empty),
            Some(s) => Some(strings.intern(s)),
            None => None,
        })
    });
    let Some(prev_sid) = prev_sid else {
        // Spec §4.9.1.2 step 3: throw NotFoundError when the
        // attribute is absent.  Our current DOMException surface
        // covers this via the well-known name; reuse the same
        // factory as `element.removeAttributeNode`.
        let not_found = ctx.vm.well_known.dom_exc_not_found_error;
        return Err(VmError::dom_exception(
            not_found,
            format!("Failed to execute 'removeNamedItem' on 'NamedNodeMap': '{key}' not found"),
        ));
    };
    // Reuse `key_sid` directly as the qualified-name StringId — it
    // was produced from the original argument's
    // `coerce::to_string` and is the canonical interned form;
    // re-interning the lossy `get_utf8(...)` round-trip would
    // duplicate work and risk a different code-unit sequence for
    // strings containing lone surrogates.
    let qname_sid = key_sid;
    // Apply the removal through `host_if_bound`.  Treat post-snapshot
    // unbind as the absent path (raise `NotFoundError`) rather than
    // panicking — matches how the rest of this codepath surfaces
    // post-unbind state.
    let Some(host) = ctx.host_if_bound() else {
        let not_found = ctx.vm.well_known.dom_exc_not_found_error;
        return Err(VmError::dom_exception(
            not_found,
            format!("Failed to execute 'removeNamedItem' on 'NamedNodeMap': '{key}' not found"),
        ));
    };
    host.dom().remove_attribute(owner, &key);
    let returned = ctx.vm.alloc_attr(AttrState {
        owner,
        qualified_name: qname_sid,
        detached_value: Some(prev_sid),
    });
    Ok(JsValue::Object(returned))
}

// -------------------------------------------------------------------------
// NS variants — Phase 2 supports null namespace only.
// -------------------------------------------------------------------------

/// Return `true` if `ns_value` is a null namespace reference
/// (`null` / `undefined` / empty string).  Any non-null namespace
/// argument is treated as an unsupported XML namespace request
/// in Phase 2 — callers map to `null` / no-op depending on the
/// operation.
fn is_null_namespace(ctx: &mut NativeContext<'_>, ns: JsValue) -> Result<bool, VmError> {
    match ns {
        JsValue::Null | JsValue::Undefined => Ok(true),
        JsValue::String(sid) => Ok(ctx.vm.strings.get_utf8(sid).is_empty()),
        other => {
            let sid = super::super::coerce::to_string(ctx.vm, other)?;
            Ok(ctx.vm.strings.get_utf8(sid).is_empty())
        }
    }
}

fn native_nnm_get_named_item_ns(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_named_node_map_receiver(ctx, this, "getNamedItemNS")?;
    let ns = args.first().copied().unwrap_or(JsValue::Undefined);
    if !is_null_namespace(ctx, ns)? {
        return Ok(JsValue::Null);
    }
    // Delegate to the non-namespace path — localName == qualified
    // name in Phase 2.
    let local = args.get(1).copied().unwrap_or(JsValue::Undefined);
    native_nnm_get_named_item(ctx, this, &[local])
}

fn native_nnm_set_named_item_ns(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    // WebIDL: argument is an Attr; namespace is intrinsic to the
    // Attr itself.  Under Phase 2 every Attr has `namespaceURI =
    // null`, so this is a straight alias for `setNamedItem`.
    native_nnm_set_named_item(ctx, this, args)
}

fn native_nnm_remove_named_item_ns(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_named_node_map_receiver(ctx, this, "removeNamedItemNS")?;
    let ns = args.first().copied().unwrap_or(JsValue::Undefined);
    if !is_null_namespace(ctx, ns)? {
        let not_found = ctx.vm.well_known.dom_exc_not_found_error;
        return Err(VmError::dom_exception(
            not_found,
            "Failed to execute 'removeNamedItemNS' on 'NamedNodeMap': non-null namespace is not supported"
                .to_string(),
        ));
    }
    let local = args.get(1).copied().unwrap_or(JsValue::Undefined);
    native_nnm_remove_named_item(ctx, this, &[local])
}

// -------------------------------------------------------------------------
// `[Symbol.iterator]` — values iterator over Attr wrappers.
// -------------------------------------------------------------------------

fn native_nnm_symbol_iterator(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (_, owner) = require_named_node_map_receiver(ctx, this, "@@iterator")?;
    let names = attribute_names_snapshot_if_bound(ctx, owner);
    let mut values = Vec::with_capacity(names.len());
    for name in names {
        let qname_sid = ctx.vm.strings.intern(&name);
        let attr_id = ctx.vm.alloc_attr(AttrState {
            owner,
            qualified_name: qname_sid,
            detached_value: None,
        });
        values.push(JsValue::Object(attr_id));
    }
    let array_id = ctx.vm.create_array_object(values);
    let proto = ctx.vm.array_iterator_prototype;
    let iter_obj = ctx.vm.alloc_object(Object {
        kind: ObjectKind::ArrayIterator(super::super::value::ArrayIterState {
            array_id,
            index: 0,
            kind: ARRAY_ITER_KIND_VALUES,
        }),
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });
    Ok(JsValue::Object(iter_obj))
}

// -------------------------------------------------------------------------
// Indexed + named property access (called from `ops_property::get_element`)
// -------------------------------------------------------------------------

/// Try to resolve `nnm[key]` for a NamedNodeMap receiver.
/// Returns `Some((owner, qname_sid))` when the key is a valid
/// numeric index or a string matching an attribute name; `None`
/// when the caller should fall through to the prototype chain
/// (so `.length` / `.getNamedItem` still see the prototype
/// accessor / method).
///
/// Returns pre-wrapper data (owner Entity + qualified-name
/// `StringId`) rather than allocating the `Attr` wrapper inline.
/// `alloc_attr` mutably borrows `VmInner::attr_states`, which
/// aliases through the same `VmInner` that owns the shared
/// reborrow chain backing the caller's `&EcsDom` — splitting the
/// phases lets the caller drop the DOM borrow before allocation.
pub(crate) fn try_indexed_get(
    vm: &mut VmInner,
    dom: &EcsDom,
    id: ObjectId,
    key: JsValue,
) -> Option<(Entity, StringId)> {
    let owner = *vm.named_node_map_states.get(&id)?;
    let names = attribute_names_snapshot(dom, owner);

    match key {
        JsValue::Number(n) if n.is_finite() => {
            let trunc = n.trunc();
            if (trunc - n).abs() > f64::EPSILON || trunc < 0.0 {
                return None;
            }
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let idx = trunc as usize;
            let name = names.get(idx)?;
            let qname_sid = vm.strings.intern(name);
            Some((owner, qname_sid))
        }
        JsValue::String(sid) => {
            // Canonical array-index parse (ES §6.1.7 / §7.1.21):
            // rejects "01" / "+1" / "1.0" so `attrs['01']` falls
            // through to attribute-name lookup rather than aliasing
            // `attrs[1]`.  Mirrors the HTMLCollection / NodeList
            // indexed-string path in `dom_collection.rs`.
            let key_units = vm.strings.get(sid);
            if let Some(idx_u32) = super::super::coerce_format::parse_array_index_u32(key_units) {
                let idx = idx_u32 as usize;
                let name = names.get(idx)?;
                let qname_sid = vm.strings.intern(name);
                return Some((owner, qname_sid));
            }
            let key_str = vm.strings.get_utf8(sid);
            // Match by exact attribute name — HTML documents store
            // names lowercase via `EcsDom::set_attribute`, so a
            // lookup for `"id"` hits the normalised key.
            if !names.iter().any(|n| n == key_str.as_str()) {
                return None;
            }
            let qname_sid = vm.strings.intern(&key_str);
            Some((owner, qname_sid))
        }
        _ => None,
    }
}
