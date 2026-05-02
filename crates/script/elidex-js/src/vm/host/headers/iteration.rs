//! Iteration support for `Headers.prototype.{forEach, keys, values,
//! entries}` plus the `combine` step shared with `Headers.get` and
//! the `body_mixin` content-type derivation.
//!
//! Split from [`super`] (`headers/mod.rs`) so the per-file
//! 1000-line convention is preserved when `Headers` grew the
//! sort-and-combine snapshot path (WHATWG Fetch §5.2).

#![cfg(feature = "engine")]

use super::super::super::shape;
use super::super::super::value::{
    ArrayIterState, JsValue, NativeContext, Object, ObjectKind, PropertyStorage, StringId,
};
use super::super::super::VmInner;
use super::ObjectId;

/// Join every `StringId` in `values` with `", "` into a single
/// interned `StringId` (WHATWG Fetch §5.2 `combine` algorithm).
/// Used by `Headers.get` for multi-valued headers and by
/// [`super::super::body_mixin::content_type_of`] so `Blob.type` and
/// `resp.headers.get('content-type')` always agree on the
/// combined form — `pub(super)` so body-mixin can share.
///
/// **Caller contract**: `values` must be non-empty.  A zero-length
/// input is a logic error (the caller should short-circuit to
/// the "no matching header" sentinel before calling); the body
/// still returns the empty interned string in that case, which
/// is harmless but not a defined output.
pub(in crate::vm::host) fn join_values_comma_space(
    vm: &mut VmInner,
    values: &[StringId],
) -> StringId {
    if values.len() == 1 {
        return values[0];
    }
    let mut joined = String::new();
    for (i, &sid) in values.iter().enumerate() {
        if i > 0 {
            joined.push_str(", ");
        }
        joined.push_str(&vm.strings.get_utf8(sid));
    }
    vm.strings.intern(&joined)
}

/// WHATWG Fetch §7.3 "sort and combine": return the iteration
/// entries in sorted-lowercase-name order, with same-name values
/// joined by `", "` except for `set-cookie` which produces one
/// output entry per occurrence.
pub(super) fn sort_and_combine(
    vm: &mut VmInner,
    headers_id: ObjectId,
) -> Vec<(StringId, StringId)> {
    let Some(state) = vm.headers_states.get(&headers_id) else {
        return Vec::new();
    };
    let set_cookie_sid = vm.well_known.set_cookie_header;
    let list = state.list.clone();
    // Gather the set of distinct lowercase names (preserving first
    // occurrence order is unnecessary — we sort anyway).  Use a
    // Vec+sort+dedup instead of HashSet so the downstream sort is
    // stable without extra bookkeeping.
    let mut name_ids: Vec<StringId> = list.iter().map(|(n, _)| *n).collect();
    // Sort by code-unit order (WHATWG Fetch §5.2 step 3.4:
    // "sort names in ascending order with a being less than b
    // if a is code-unit less than b").  Header-name validation
    // upstream restricts bytes to the RFC 7230 token set (ASCII
    // only), so code-unit order on `&[u16]` coincides with
    // byte order without any `String` allocation.  Duplicates
    // disappear in the dedup below.
    name_ids.sort_by(|a, b| vm.strings.get(*a).cmp(vm.strings.get(*b)));
    name_ids.dedup();

    let mut out: Vec<(StringId, StringId)> = Vec::with_capacity(list.len());
    for name_sid in name_ids {
        if name_sid == set_cookie_sid {
            for (n, v) in &list {
                if *n == set_cookie_sid {
                    out.push((*n, *v));
                }
            }
        } else {
            let values: Vec<StringId> = list
                .iter()
                .filter(|(n, _)| *n == name_sid)
                .map(|(_, v)| *v)
                .collect();
            let combined = join_values_comma_space(vm, &values);
            out.push((name_sid, combined));
        }
    }
    out
}

/// Wrap a snapshot `Vec<JsValue>` in an `ArrayIterator` (kind=0 =
/// Values) so it iterates the elements directly.  Used by `keys()`,
/// `values()`, `entries()` — the same snapshot strategy as
/// `Map.prototype.entries()` (S8 critical-review decision).
///
/// # GC safety
///
/// The `alloc_object` call for the iterator can trigger GC.
/// Between `create_array_object` (which returns `arr_id`) and
/// the iterator alloc, `arr_id` has no GC root — it is only
/// referenced by a Rust local, *not* yet by any live JS object.
/// Without the explicit temp-root guard below, a GC triggered
/// by the iterator's own allocation would reclaim the snapshot
/// array, leaving `ArrayIterState.array_id` pointing at a
/// freed slot.  The [`super::super::super::VmTempRoot`] RAII
/// guard holds `arr_id` on the VM stack until the iterator is
/// fully constructed and installed (at which point the iterator's
/// `ArrayIterState.array_id` field becomes a proper strong
/// reference that the GC trace picks up).
pub(super) fn wrap_in_array_iterator(
    ctx: &mut NativeContext<'_>,
    elements: Vec<JsValue>,
) -> JsValue {
    let arr_id = ctx.vm.create_array_object(elements);
    // Snapshot the prototype ObjectId first so the subsequent
    // `alloc_object(...)` call doesn't hold an immutable borrow
    // on `rooted` while also requesting a mutable one.
    let iter_proto = ctx.vm.array_iterator_prototype;
    let mut rooted = ctx.vm.push_temp_root(JsValue::Object(arr_id));
    let iter_id = rooted.alloc_object(Object {
        kind: ObjectKind::ArrayIterator(ArrayIterState {
            array_id: arr_id,
            index: 0,
            kind: 0, // Values
        }),
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: iter_proto,
        extensible: true,
    });
    // Dropping `rooted` here restores the stack; `arr_id` is
    // now reachable from `iter_id`'s `ArrayIterState` field so
    // the GC trace keeps it alive.
    drop(rooted);
    JsValue::Object(iter_id)
}
