//! `DOMStringMap.prototype` intrinsic + `HTMLElement.dataset`
//! accessor plumbing (WHATWG HTML §3.2.6 / WebIDL §3.10).
//!
//! Thin binding to the engine-independent
//! `elidex_dom_api::element::attrs` dataset handlers
//! (`dataset.get` / `dataset.set` / `dataset.delete` /
//! `dataset.keys`).  The `data-*` ↔ camelCase conversion lives in
//! the handler crate (`camel_to_data_attr` / `data_attr_to_camel`);
//! this file only routes the named-property exotic
//! [[Get]] / [[Set]] / [[Delete]] / [[OwnPropertyKeys]] traps to the
//! handler dispatch.
//!
//! ## Backing state
//!
//! [`ObjectKind::DOMStringMap`] carries the owner `Entity` inline
//! (`entity_bits`); there is no per-wrapper side table.  Identity
//! is preserved via [`VmInner::dataset_wrapper_cache`] keyed by the
//! owner `Entity`.
//!
//! ## Named-property exotic dispatch
//!
//! The four traps land at four different VM dispatch sites:
//!
//! - `[[Get]]` — `ops_element::get_element` — [`try_get`]
//! - `[[Set]]` — `ops_element::set_element` — [`try_set`]
//! - `[[Delete]]` — `ops_property::try_delete_property` — [`try_delete`]
//! - `[[OwnPropertyKeys]]` / for-in — `coerce_format::collect_own_keys_es_order`
//!   + `dispatch_iter::op_for_in_iterator` — [`collect_keys`]
//!
//! Each helper returns `None` when the receiver is not a
//! `DOMStringMap` (so the dispatch site falls through to ordinary
//! property semantics for non-string keys / Symbol keys / etc.) and
//! `Some(_)` to short-circuit the trap.

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage, StringId,
    VmError,
};
use super::super::VmInner;
use super::dom_bridge::invoke_dom_api;

use elidex_ecs::Entity;

impl VmInner {
    /// Allocate `DOMStringMap.prototype` chained to `Object.prototype`.
    /// The prototype carries no own members — every named-property
    /// access dispatches via `ObjectKind` in the four trap sites.
    pub(in crate::vm) fn register_dom_string_map_prototype(&mut self) {
        let obj_proto = self.object_prototype;
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: obj_proto,
            extensible: true,
        });
        self.dom_string_map_prototype = Some(proto_id);
    }

    /// Allocate a `DOMStringMap` wrapper for `owner`, caching by
    /// `owner` so `el.dataset === el.dataset` (WHATWG HTML §3.2.6).
    pub(crate) fn alloc_or_cached_dataset(&mut self, owner: Entity) -> ObjectId {
        if let Some(&id) = self.dataset_wrapper_cache.get(&owner) {
            return id;
        }
        let proto = self
            .dom_string_map_prototype
            .expect("alloc_or_cached_dataset before register_dom_string_map_prototype");
        let id = self.alloc_object(Object {
            kind: ObjectKind::DOMStringMap {
                entity_bits: owner.to_bits().get(),
            },
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(proto),
            extensible: false,
        });
        self.dataset_wrapper_cache.insert(owner, id);
        id
    }
}

// ---------------------------------------------------------------------------
// Trap helpers — one per named-property-exotic operation.
// ---------------------------------------------------------------------------

fn entity_from_id(vm: &VmInner, id: ObjectId) -> Option<Entity> {
    let ObjectKind::DOMStringMap { entity_bits } = vm.get_object(id).kind else {
        return None;
    };
    Entity::from_bits(entity_bits)
}

/// Coerce a property key into a `StringId` for handler dispatch, or
/// signal fall-through.  ECMA §7.1.19 ToPropertyKey turns every
/// non-Symbol value into a string, so `dataset[true]`, `dataset[null]`,
/// `'fooBar' in dataset`, and `Object.getOwnPropertyDescriptor(dataset,
/// 0)` all need to reach the named-property exotic with a string key.
/// Symbol keys return `None` so the dispatch site falls through to the
/// ordinary property path (Symbol-keyed access resolves via the
/// prototype chain, never via `data-*`).
fn coerce_key_or_none(vm: &mut VmInner, key: JsValue) -> Option<Result<StringId, VmError>> {
    match key {
        JsValue::Symbol(_) => None,
        _ => Some(super::super::coerce::to_string(vm, key)),
    }
}

/// Post-unbind tolerance helper: `DOMStringMap` wrappers are plain JS
/// objects (not `HostObject`), so user code can retain `el.dataset`
/// across a `Vm::unbind()` boundary.  When the VM is unbound, the
/// trap helpers must not call [`invoke_dom_api`] (it panics via
/// `HostData::with_session_and_dom`'s `is_bound()` assert).  Mirrors
/// [`super::named_node_map::attribute_names_snapshot_if_bound`].
fn is_bound(vm: &VmInner) -> bool {
    vm.host_data
        .as_deref()
        .is_some_and(super::super::host_data::HostData::is_bound)
}

/// WebIDL §3.10 named-property visibility — DOMStringMap is *not*
/// `[LegacyOverrideBuiltIns]`, so a key already exposed as an own
/// property on any object in the prototype chain (`DOMStringMap.prototype`
/// → `Object.prototype` → null) MUST take precedence over the
/// supported-name → `data-*` mapping.  Returns `true` when the
/// dispatch sites should fall through to the ordinary [[Get]] /
/// [[HasProperty]] / [[Delete]] / [[Set]] / [[OwnPropertyKeys]]
/// path so inherited members like `Object.prototype.toString` /
/// `Object.prototype.hasOwnProperty` are not shadowed by a
/// hypothetical `data-toString` / `data-hasOwnProperty` attribute.
fn key_on_prototype_chain(vm: &VmInner, dataset_id: ObjectId, key_sid: StringId) -> bool {
    let mut current = vm.get_object(dataset_id).prototype;
    while let Some(proto_id) = current {
        let proto = vm.get_object(proto_id);
        if proto
            .storage
            .get(PropertyKey::String(key_sid), &vm.shapes)
            .is_some()
        {
            return true;
        }
        current = proto.prototype;
    }
    false
}

/// `[[HasProperty]]` trap (WebIDL §3.10 named-property exotic).
/// Returns `Some(true)` when the key names a present `data-*`
/// attribute (so `'fooBar' in el.dataset` is true after
/// `el.setAttribute('data-foo-bar', _)`).  Returns `None` in every
/// other case — Symbol / non-coercible keys, absent `data-*` keys,
/// and post-unbind — so the dispatch site falls through to the
/// ordinary `[[HasProperty]]` / prototype walk.  Without this
/// fall-through, inherited methods like `'toString' in el.dataset`
/// would incorrectly return `false` because the wrapper itself is
/// sealed: the prototype-chain walk is the only path that surfaces
/// `Object.prototype` members.
pub(crate) fn try_has(
    vm: &mut VmInner,
    id: ObjectId,
    key: JsValue,
) -> Option<Result<bool, VmError>> {
    let entity = entity_from_id(vm, id)?;
    let sid = match coerce_key_or_none(vm, key)? {
        Ok(sid) => sid,
        Err(e) => return Some(Err(e)),
    };
    if key_on_prototype_chain(vm, id, sid) {
        return None;
    }
    if !is_bound(vm) {
        return None;
    }
    let mut ctx = NativeContext { vm };
    let result = invoke_dom_api(&mut ctx, "dataset.get", entity, &[JsValue::String(sid)]);
    match result {
        Ok(JsValue::Undefined) => None,
        Ok(_) => Some(Ok(true)),
        Err(e) => Some(Err(e)),
    }
}

/// `[[Get]]` trap (WebIDL §3.10 named-property getter).  Returns
/// `Some(Ok(value))` only when the key names a present `data-*`
/// attribute — `dataset.fooBar` with `data-foo-bar` set.  Returns
/// `None` for Symbol keys, non-string non-numeric keys, and string
/// keys that miss the `data-*` set, so the dispatch site falls
/// through to the ordinary [[Get]] / prototype-chain walk.
///
/// This matches WebIDL: a non-`[LegacyOverrideBuiltIns]` named
/// property exotic provides the supported-name → value mapping, but
/// non-matching keys still resolve through `Object.prototype`
/// (`dataset.toString === Object.prototype.toString`,
/// `dataset.hasOwnProperty('foo')` works).
pub(crate) fn try_get(
    vm: &mut VmInner,
    id: ObjectId,
    key: JsValue,
) -> Option<Result<JsValue, VmError>> {
    let entity = entity_from_id(vm, id)?;
    let sid = match coerce_key_or_none(vm, key)? {
        Ok(sid) => sid,
        Err(e) => return Some(Err(e)),
    };
    if key_on_prototype_chain(vm, id, sid) {
        return None;
    }
    if !is_bound(vm) {
        return None;
    }
    let mut ctx = NativeContext { vm };
    let result = invoke_dom_api(&mut ctx, "dataset.get", entity, &[JsValue::String(sid)]);
    match result {
        Ok(JsValue::Undefined) => None,
        Ok(v) => Some(Ok(v)),
        Err(e) => Some(Err(e)),
    }
}

/// `[[Set]]` trap.  Symbol keys fall through; string keys route to
/// `dataset.set` after ToString-coercing the value.  Numeric keys
/// are stringified and routed too — same rationale as `try_get`.
pub(crate) fn try_set(
    vm: &mut VmInner,
    id: ObjectId,
    key: JsValue,
    value: JsValue,
) -> Option<Result<(), VmError>> {
    let entity = entity_from_id(vm, id)?;
    let key_sid = match coerce_key_or_none(vm, key)? {
        Ok(sid) => sid,
        Err(e) => return Some(Err(e)),
    };
    if key_on_prototype_chain(vm, id, key_sid) {
        return None;
    }
    let val_sid = match super::super::coerce::to_string(vm, value) {
        Ok(sid) => sid,
        Err(e) => return Some(Err(e)),
    };
    if !is_bound(vm) {
        return Some(Ok(()));
    }
    let mut ctx = NativeContext { vm };
    let result = invoke_dom_api(
        &mut ctx,
        "dataset.set",
        entity,
        &[JsValue::String(key_sid), JsValue::String(val_sid)],
    );
    Some(result.map(|_| ()))
}

/// `[[Delete]]` trap.  String / numeric keys route to
/// `dataset.delete`; Symbol keys / non-string-coercible keys fall
/// through.  WebIDL §3.10 deletion is total-success (returns `true`
/// even when the key was absent), matching how `dataset.delete`
/// resolves: the handler calls `attrs.remove` which is idempotent.
pub(crate) fn try_delete(
    vm: &mut VmInner,
    id: ObjectId,
    key: JsValue,
) -> Option<Result<bool, VmError>> {
    let entity = entity_from_id(vm, id)?;
    let key_sid = match coerce_key_or_none(vm, key)? {
        Ok(sid) => sid,
        Err(e) => return Some(Err(e)),
    };
    if key_on_prototype_chain(vm, id, key_sid) {
        return None;
    }
    if !is_bound(vm) {
        return Some(Ok(true));
    }
    let mut ctx = NativeContext { vm };
    let result = invoke_dom_api(
        &mut ctx,
        "dataset.delete",
        entity,
        &[JsValue::String(key_sid)],
    );
    Some(result.map(|_| true))
}

/// Collect the supported property names (camelCase keys backing
/// `data-*` attributes) for a DOMStringMap receiver.  Returns
/// `None` when `id` is not a DOMStringMap.  The handler returns the
/// keys joined by `\0`; this helper splits and re-interns each
/// segment as a `StringId` so the OwnPropertyKeys / for-in dispatch
/// sites can iterate without re-parsing.
pub(crate) fn collect_keys(
    vm: &mut VmInner,
    id: ObjectId,
) -> Option<Result<Vec<super::super::value::StringId>, VmError>> {
    let entity = entity_from_id(vm, id)?;
    if !is_bound(vm) {
        return Some(Ok(Vec::new()));
    }
    let mut ctx = NativeContext { vm };
    let result = invoke_dom_api(&mut ctx, "dataset.keys", entity, &[]);
    Some(result.and_then(|raw| match raw {
        JsValue::String(sid) => {
            let joined = vm.strings.get_utf8(sid);
            let keys: Vec<_> = if joined.is_empty() {
                Vec::new()
            } else {
                joined.split('\0').map(|s| vm.strings.intern(s)).collect()
            };
            // WebIDL §3.10 named-property visibility — drop any key
            // whose own-property descriptor would have been
            // shadowed by `Object.prototype` so `Object.keys(dataset)`
            // never surfaces a stub like `"toString"` even if a
            // `data-to-string` attribute exists (HTML attribute names
            // are lowercased on storage; the camel↔kebab mapping
            // turns `data-to-string` into the camelCase key
            // `toString`).  The shadowed attribute remains
            // accessible via `el.getAttribute("data-to-string")`.
            let filtered = keys
                .into_iter()
                .filter(|sid| !key_on_prototype_chain(vm, id, *sid))
                .collect();
            Ok(filtered)
        }
        _ => Err(VmError::type_error(
            "DOMStringMap dataset.keys must return a string",
        )),
    }))
}
