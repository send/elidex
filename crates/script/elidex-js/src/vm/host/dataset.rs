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
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyStorage, StringId, VmError,
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
/// signal fall-through.  Strings pass through; numeric keys are
/// stringified per ECMA §10.5 ToPropertyKey conversion (an integer
/// key exists as a string in the supported-name set).  Symbol keys /
/// non-coercible keys return `None` so the dispatch site falls
/// through to the ordinary property path / prototype chain.
fn coerce_key_or_none(vm: &mut VmInner, key: JsValue) -> Option<Result<StringId, VmError>> {
    match key {
        JsValue::String(sid) => Some(Ok(sid)),
        JsValue::Number(_) => Some(super::super::coerce::to_string(vm, key)),
        _ => None,
    }
}

/// `[[HasProperty]]` trap (WebIDL §3.10 named-property exotic).
/// Returns `Some(true)` when the key names a present `data-*`
/// attribute (so `'fooBar' in el.dataset` is true after
/// `el.setAttribute('data-foo-bar', _)`); `Some(false)` for a
/// string-coercible key whose data-* attribute is absent (so
/// non-supported names short-circuit before the prototype walk —
/// the wrapper is sealed and inherits no enumerable members);
/// `None` for Symbol / non-string keys so the dispatch site falls
/// through to ordinary `[[HasProperty]]` (Symbol membership and
/// inherited methods like `toString` resolve via the prototype
/// chain).
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
    let mut ctx = NativeContext { vm };
    let result = invoke_dom_api(&mut ctx, "dataset.get", entity, &[JsValue::String(sid)]);
    Some(result.map(|v| !matches!(v, JsValue::Undefined)))
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
    let val_sid = match super::super::coerce::to_string(vm, value) {
        Ok(sid) => sid,
        Err(e) => return Some(Err(e)),
    };
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
            Ok(keys)
        }
        _ => Err(VmError::type_error(
            "DOMStringMap dataset.keys must return a string",
        )),
    }))
}
