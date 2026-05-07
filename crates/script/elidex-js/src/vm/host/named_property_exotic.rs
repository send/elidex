//! Shared helpers for WebIDL §3.10 named-property exotic dispatch.
//!
//! Wrapper types like `DOMStringMap` (`HTMLElement.dataset`) and
//! `Storage` (`localStorage` / `sessionStorage`) implement the named-
//! property exotic protocol: a string-keyed [[Get]] / [[Set]] /
//! [[Delete]] / [[HasProperty]] / [[OwnPropertyKeys]] trap that maps
//! supported names to backing storage (data attributes / origin
//! storage / etc.).  The mechanical pieces — string-key coercion that
//! falls through for Symbol keys, prototype-chain shadowing per
//! WebIDL non-`[LegacyOverrideBuiltIns]` semantics, and the bound-
//! state guard for trap helpers that reach into engine-bound state —
//! are identical across implementations and live here so each
//! wrapper module stays focused on its backing-store dispatch.

#![cfg(feature = "engine")]

use super::super::value::{JsValue, ObjectId, PropertyKey, StringId, VmError};
use super::super::VmInner;

/// Coerce a property key into a `StringId`, returning `None` for
/// Symbol keys so the dispatch site falls through to the prototype-
/// chain walk.  Per ECMA §7.1.19 ToPropertyKey, every non-Symbol value
/// is stringified; the named-property exotic operates on string keys
/// only.
pub(super) fn coerce_key_or_none(
    vm: &mut VmInner,
    key: JsValue,
) -> Option<Result<StringId, VmError>> {
    match key {
        JsValue::Symbol(_) => None,
        _ => Some(super::super::coerce::to_string(vm, key)),
    }
}

/// WebIDL §3.10 named-property visibility — wrappers that are *not*
/// `[LegacyOverrideBuiltIns]` must defer to any prototype-chain
/// member that already exposes the key as an own property.  Returns
/// `true` when the dispatch site should fall through to the ordinary
/// trap so inherited members like `Object.prototype.toString` /
/// `Object.prototype.hasOwnProperty` / interface-prototype methods
/// (`getItem` / `setItem` / `length` for Storage; nothing for
/// DOMStringMap which has an empty prototype) are not shadowed by a
/// hypothetical supported-name collision.
pub(super) fn key_on_prototype_chain(
    vm: &VmInner,
    receiver_id: ObjectId,
    key_sid: StringId,
) -> bool {
    let mut current = vm.get_object(receiver_id).prototype;
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

/// Post-unbind tolerance helper: trap helpers must not reach into
/// engine-bound state when the VM is unbound.  Wrappers (`dataset` /
/// `localStorage` / `sessionStorage`) are plain JS objects, so user
/// code can retain them across a `Vm::unbind()` boundary; the trap
/// helpers check this guard before calling into `EcsDom` / the
/// storage backend.
pub(super) fn is_bound(vm: &VmInner) -> bool {
    vm.host_data
        .as_deref()
        .is_some_and(super::super::host_data::HostData::is_bound)
}
