//! `init` parsing for `new Headers(init)` and the shared
//! `init.headers` path used by `Request` / `fetch`.
//!
//! Split from [`super`] (`headers/mod.rs`) so the per-file
//! 1000-line convention is preserved across the WHATWG Fetch §5.2
//! "fill a Headers object" branches (Headers source / Array
//! source / generic iterable / record).

#![cfg(feature = "engine")]

use super::super::super::value::{
    JsValue, NativeContext, ObjectKind, PropertyKey, StringId, VmError,
};
use super::{append_entry, validate_and_normalise, ObjectId};

/// Populate `headers_id` from an `init` value per WHATWG Fetch
/// §5.2 "fill a Headers object".
///
/// `pub(in crate::vm::host)` (re-exported via [`super`]) so the
/// `request_response` module can reuse this logic for its
/// `init.headers` member (avoids a parallel reimplementation —
/// validation / lowercase / revalidation skipping on
/// `Headers`-source all stay in one place).
pub(in crate::vm::host) fn fill_headers_from_init(
    ctx: &mut NativeContext<'_>,
    headers_id: ObjectId,
    init: JsValue,
    error_prefix: &str,
) -> Result<(), VmError> {
    let entries = parse_headers_init_entries(ctx, init, error_prefix)?;
    for (name_sid, value_sid) in entries {
        append_entry(ctx, headers_id, name_sid, value_sid)?;
    }
    Ok(())
}

/// Parse `init` per WHATWG Fetch §5.2 "fill a Headers object"
/// into an owned `Vec<(StringId, StringId)>` of
/// (lowercased-name, trimmed-value) pairs — **without**
/// allocating a `Headers` instance or touching
/// `headers_states`.  Used by the `fetch()` host when it needs
/// the parsed entries directly for the broker-facing
/// `elidex_net::Request`; that path formerly allocated a
/// throwaway `Headers` JS object, filled it, snapshotted its
/// list, and left the object to the next GC (R8.2).  The
/// shared [`fill_headers_from_init`] wrapper above now
/// delegates here so the validation / source-type branches do
/// not drift.
pub(in crate::vm::host) fn parse_headers_init_entries(
    ctx: &mut NativeContext<'_>,
    init: JsValue,
    error_prefix: &str,
) -> Result<Vec<(StringId, StringId)>, VmError> {
    match init {
        JsValue::Undefined | JsValue::Null => Ok(Vec::new()),
        JsValue::Object(obj_id) => {
            // Source `Headers`: copy entries directly.  Values are
            // already validated, so we can bypass revalidation.
            if matches!(ctx.vm.get_object(obj_id).kind, ObjectKind::Headers) {
                return Ok(ctx
                    .vm
                    .headers_states
                    .get(&obj_id)
                    .map(|s| s.list.clone())
                    .unwrap_or_default());
            }
            // Source `Array`: require each element to be a length-2
            // sequence.  Clone elements to release the borrow before
            // coercing / validating.
            if let ObjectKind::Array { elements } = &ctx.vm.get_object(obj_id).kind {
                let snapshot = elements.clone();
                let mut out = Vec::with_capacity(snapshot.len());
                for pair in snapshot {
                    out.push(validate_pair_entry(ctx, pair, error_prefix)?);
                }
                return Ok(out);
            }
            // WebIDL union resolution for `HeadersInit` (§Fetch 5.2:
            // `sequence<sequence<ByteString>> or record<ByteString,
            // ByteString>`): if `init` has a callable `[Symbol.iterator]`
            // it must be consumed as the sequence branch — iterate the
            // user-supplied iterator and validate each yielded pair.
            // Arrays already hit the fast path above; this branch picks
            // up generic iterables (user-defined `[Symbol.iterator]`
            // objects, Map-like wrappers, etc.) that would otherwise
            // fall through to the record path and silently produce an
            // empty Headers list (R17.1).  `GetMethod` semantics:
            // null/undefined → not iterable (record branch); any other
            // non-callable → TypeError.
            let iter_key = PropertyKey::Symbol(ctx.vm.well_known_symbols.iterator);
            let iter_method = ctx.get_property_value(obj_id, iter_key)?;
            let iter_fn = match iter_method {
                JsValue::Undefined | JsValue::Null => None,
                JsValue::Object(id) if ctx.vm.get_object(id).kind.is_callable() => {
                    Some(iter_method)
                }
                _ => {
                    return Err(VmError::type_error(format!(
                        "{error_prefix}: @@iterator is not callable"
                    )));
                }
            };
            if let Some(fn_val) = iter_fn {
                let iter = ctx.vm.call_value(fn_val, init, &[])?;
                if !matches!(iter, JsValue::Object(_)) {
                    return Err(VmError::type_error(format!(
                        "{error_prefix}: @@iterator must return an object"
                    )));
                }
                let mut out = Vec::new();
                loop {
                    // A throw from `iter_next` itself means the iterator's
                    // own `.next()` raised — per ES §7.4.6 the iterator is
                    // already considered closed, so `IteratorClose` must
                    // *not* be called.  Propagate directly.
                    let pair = match ctx.vm.iter_next(iter)? {
                        Some(p) => p,
                        None => break,
                    };
                    // A throw from `validate_pair_entry` is an abrupt
                    // completion of the for-of-like loop body; §7.4.6
                    // requires `IteratorClose` (i.e. call `.return()` on
                    // the iterator) before propagating.  A throw from
                    // `.return()` itself takes precedence over the
                    // triggering abrupt completion (§7.4.6 step 6-7).
                    match validate_pair_entry(ctx, pair, error_prefix) {
                        Ok(p) => out.push(p),
                        Err(err) => {
                            let close_err = ctx.vm.iter_close(iter).err();
                            return Err(close_err.unwrap_or(err));
                        }
                    }
                }
                return Ok(out);
            }
            // Record branch: no `@@iterator` — iterate own enumerable
            // string keys (§9.1.11.1 order) and coerce each value.
            let keys =
                super::super::super::coerce_format::collect_own_keys_es_order(ctx.vm, obj_id);
            let mut out = Vec::with_capacity(keys.len());
            for key_sid in keys {
                let value = ctx.get_property_value(obj_id, PropertyKey::String(key_sid))?;
                let value_sid = super::super::super::coerce::to_string(ctx.vm, value)?;
                let pair = validate_and_normalise(ctx.vm, key_sid, value_sid, error_prefix)?;
                out.push(pair);
            }
            Ok(out)
        }
        _ => Err(VmError::type_error(format!(
            "{error_prefix}: The provided value is not of type 'HeadersInit'."
        ))),
    }
}

/// Validate one `[name, value]` pair from the sequence-init form
/// and return the normalised `(name_sid, value_sid)` tuple.
///
/// Per WebIDL `sequence<sequence<ByteString>>`, the **inner** pair
/// is converted via the iterator protocol just like the outer
/// sequence — any iterable yielding exactly two items qualifies
/// (plain `[name, value]` arrays, `new Set([name, value])`,
/// user-defined `[Symbol.iterator]` objects, etc.).  Arity ≠ 2
/// is TypeError; iteration abrupt completion closes the inner
/// iterator via `.return()` (§7.4.6) (R22.1).
fn validate_pair_entry(
    ctx: &mut NativeContext<'_>,
    pair: JsValue,
    error_prefix: &str,
) -> Result<(StringId, StringId), VmError> {
    let [name_val, value_val] = collect_header_pair_values(ctx, pair, error_prefix)?;
    let name_sid = super::super::super::coerce::to_string(ctx.vm, name_val)?;
    let value_sid = super::super::super::coerce::to_string(ctx.vm, value_val)?;
    validate_and_normalise(ctx.vm, name_sid, value_sid, error_prefix)
}

/// Extract exactly two values from an inner pair.  Fast-path a
/// plain VM `Array` (skips the iterator protocol overhead); fall
/// back to `[Symbol.iterator]` for any other iterable.  Early-exit
/// on the third yielded item to bound cost on pathological
/// iterables — spec allows this since the arity check fails either
/// way.  `IteratorClose` is called on abrupt completion so custom
/// iterables can run `.return()` cleanup.
fn collect_header_pair_values(
    ctx: &mut NativeContext<'_>,
    pair: JsValue,
    error_prefix: &str,
) -> Result<[JsValue; 2], VmError> {
    let JsValue::Object(pair_id) = pair else {
        return Err(VmError::type_error(format!(
            "{error_prefix}: Sequence header init must contain iterables of length 2"
        )));
    };
    // Fast path: VM `Array` with exactly two elements.  Same kind
    // of optimisation as the outer Array fast path in
    // `parse_headers_init_entries`; an array's `@@iterator` would
    // yield these same two values.
    if let ObjectKind::Array { elements } = &ctx.vm.get_object(pair_id).kind {
        if elements.len() != 2 {
            return Err(VmError::type_error(format!(
                "{error_prefix}: Sequence header init must contain iterables of length 2"
            )));
        }
        return Ok([elements[0], elements[1]]);
    }
    // Generic iterator protocol for any other iterable (Set,
    // custom `[Symbol.iterator]`, etc.).  `resolve_iterator` None →
    // non-iterable → TypeError, matching WebIDL sequence
    // conversion.
    let iter = match ctx.vm.resolve_iterator(pair)? {
        Some(iter @ JsValue::Object(_)) => iter,
        Some(_) => {
            return Err(VmError::type_error(format!(
                "{error_prefix}: @@iterator must return an object"
            )));
        }
        None => {
            return Err(VmError::type_error(format!(
                "{error_prefix}: Sequence header init must contain iterables of length 2"
            )));
        }
    };
    let mut values: Vec<JsValue> = Vec::with_capacity(2);
    loop {
        // `iter_next` throw → iterator already considered closed
        // (§7.4.6); propagate without `.return()`.
        let v = match ctx.vm.iter_next(iter)? {
            Some(v) => v,
            None => break,
        };
        values.push(v);
        if values.len() > 2 {
            // Early exit on arity overflow.  Closing the iterator
            // lets its `.return()` run; a throw from `.return()`
            // wins over the triggering arity error (§7.4.6
            // step 6-7).
            let close_err = ctx.vm.iter_close(iter).err();
            let arity_err = VmError::type_error(format!(
                "{error_prefix}: Sequence header init must contain iterables of length 2"
            ));
            return Err(close_err.unwrap_or(arity_err));
        }
    }
    if values.len() != 2 {
        // Exhaustion with <2 items: the iterator has already
        // reported `done=true` so it is already closed per
        // §7.4.6 "normal completion"; no `.return()` call needed.
        return Err(VmError::type_error(format!(
            "{error_prefix}: Sequence header init must contain iterables of length 2"
        )));
    }
    Ok([values[0], values[1]])
}
