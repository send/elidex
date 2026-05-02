//! `Headers.prototype` native methods + the `new Headers(init)`
//! constructor.
//!
//! Split from [`super`] (`headers/mod.rs`) so the per-file
//! 1000-line convention is preserved.  Each native function in
//! this module is referenced from `super::install_headers_methods`
//! (and `super::register_headers_global` for the constructor).

#![cfg(feature = "engine")]

use super::super::super::value::{JsValue, NativeContext, ObjectKind, StringId, VmError};
use super::iteration::{sort_and_combine, wrap_in_array_iterator};
use super::{
    append_entry, fill_headers_from_init, is_blocked_by_guard, join_values_comma_space,
    require_headers_this, require_mutable, take_name_arg, take_name_value_args,
    validate_and_normalise, validate_and_normalise_name, HeadersGuard, HeadersState,
};

/// `new Headers(init?)` (WHATWG Fetch §5.2).
///
/// Accepts:
/// - `undefined` / `null` / missing → empty list.
/// - Another `Headers` instance → copy entries (lowercased name
///   preserved from the source).
/// - `Array<[name, value]>` pairs → iterate length-2 pairs,
///   `append` each.  Non-length-2 sub-array → `TypeError`.
/// - Plain object (Record<string,string>) → iterate own enumerable
///   string keys in ES spec order, `append` each.
/// - Any other primitive / Symbol → `TypeError` (WebIDL `record<…>`
///   dictionary coercion §3.10.34).
pub(super) fn native_headers_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if !ctx.is_construct() {
        return Err(VmError::type_error(
            "Failed to construct 'Headers': Please use the 'new' operator",
        ));
    }
    // `do_new` already pre-allocated an Ordinary instance; repurpose
    // it so the `new.target.prototype` chain stays intact (PR5a2
    // R7.2/R7.3 lesson — helpers must not reassign `prototype`).
    let JsValue::Object(id) = this else {
        unreachable!("constructor `this` is always an Object after `do_new`");
    };
    ctx.vm.get_object_mut(id).kind = ObjectKind::Headers;
    ctx.vm
        .headers_states
        .insert(id, HeadersState::new(HeadersGuard::None));

    let init = args.first().copied().unwrap_or(JsValue::Undefined);
    fill_headers_from_init(ctx, id, init, "Failed to construct 'Headers'")?;
    Ok(JsValue::Object(id))
}

pub(super) fn native_headers_append(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_headers_this(ctx, this, "append")?;
    require_mutable(ctx, id, "append")?;
    let (name_sid, value_sid) = take_name_value_args(ctx, args, "append")?;
    let (name_sid, value_sid) = validate_and_normalise(
        ctx.vm,
        name_sid,
        value_sid,
        "Failed to execute 'append' on 'Headers'",
    )?;
    // Forbidden-name filter lives inside `append_entry`, which is
    // the funnel for the JS-facing `Headers.append` (this site)
    // and the `init.headers` parse path
    // (`fill_headers_from_init` → `parse_headers_init_entries`).
    // No second check here.
    append_entry(ctx, id, name_sid, value_sid)?;
    Ok(JsValue::Undefined)
}

pub(super) fn native_headers_set(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_headers_this(ctx, this, "set")?;
    require_mutable(ctx, id, "set")?;
    let (name_sid, value_sid) = take_name_value_args(ctx, args, "set")?;
    let (name_sid, value_sid) = validate_and_normalise(
        ctx.vm,
        name_sid,
        value_sid,
        "Failed to execute 'set' on 'Headers'",
    )?;
    if is_blocked_by_guard(ctx, id, name_sid) {
        return Ok(JsValue::Undefined);
    }
    if let Some(state) = ctx.vm.headers_states.get_mut(&id) {
        // Remove every existing entry with the same lowercase name,
        // then append once — WHATWG Fetch §5.2 "set a header".
        state.list.retain(|(n, _)| *n != name_sid);
        state.list.push((name_sid, value_sid));
    }
    Ok(JsValue::Undefined)
}

pub(super) fn native_headers_delete(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_headers_this(ctx, this, "delete")?;
    require_mutable(ctx, id, "delete")?;
    let name_sid = take_name_arg(ctx, args, "delete")?;
    let name_sid =
        validate_and_normalise_name(ctx.vm, name_sid, "Failed to execute 'delete' on 'Headers'")?;
    if is_blocked_by_guard(ctx, id, name_sid) {
        return Ok(JsValue::Undefined);
    }
    if let Some(state) = ctx.vm.headers_states.get_mut(&id) {
        state.list.retain(|(n, _)| *n != name_sid);
    }
    Ok(JsValue::Undefined)
}

pub(super) fn native_headers_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_headers_this(ctx, this, "get")?;
    let name_sid = take_name_arg(ctx, args, "get")?;
    let name_sid =
        validate_and_normalise_name(ctx.vm, name_sid, "Failed to execute 'get' on 'Headers'")?;
    // Collect every matching value in insertion order, then join
    // with `", "` (WHATWG §5.2 "get a header").  `set-cookie` is
    // not specially handled here — `get("set-cookie")` returns the
    // joined string; callers wanting separate cookies use
    // `getSetCookie()`.
    let matched: Vec<StringId> = ctx
        .vm
        .headers_states
        .get(&id)
        .map(|s| {
            s.list
                .iter()
                .filter(|(n, _)| *n == name_sid)
                .map(|(_, v)| *v)
                .collect()
        })
        .unwrap_or_default();
    if matched.is_empty() {
        return Ok(JsValue::Null);
    }
    let joined = join_values_comma_space(ctx.vm, &matched);
    Ok(JsValue::String(joined))
}

pub(super) fn native_headers_has(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_headers_this(ctx, this, "has")?;
    let name_sid = take_name_arg(ctx, args, "has")?;
    let name_sid =
        validate_and_normalise_name(ctx.vm, name_sid, "Failed to execute 'has' on 'Headers'")?;
    let present = ctx
        .vm
        .headers_states
        .get(&id)
        .is_some_and(|s| s.list.iter().any(|(n, _)| *n == name_sid));
    Ok(JsValue::Boolean(present))
}

pub(super) fn native_headers_get_set_cookie(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_headers_this(ctx, this, "getSetCookie")?;
    let set_cookie_sid = ctx.vm.well_known.set_cookie_header;
    // Per WHATWG Fetch §5.2 "get set-cookie": return list of each
    // `set-cookie` header's value in insertion order as separate
    // strings (no joining, unlike `get`).
    let values: Vec<JsValue> = ctx
        .vm
        .headers_states
        .get(&id)
        .map(|s| {
            s.list
                .iter()
                .filter(|(n, _)| *n == set_cookie_sid)
                .map(|(_, v)| JsValue::String(*v))
                .collect()
        })
        .unwrap_or_default();
    Ok(JsValue::Object(ctx.vm.create_array_object(values)))
}

pub(super) fn native_headers_for_each(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_headers_this(ctx, this, "forEach")?;
    let callback = match args.first().copied().unwrap_or(JsValue::Undefined) {
        JsValue::Object(cb_id) if ctx.vm.get_object(cb_id).kind.is_callable() => cb_id,
        _ => {
            return Err(VmError::type_error(
                "Failed to execute 'forEach' on 'Headers': \
                 parameter 1 is not of type 'Function'.",
            ));
        }
    };
    let this_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    // Snapshot the sort-and-combine output so user mutations inside
    // the callback do not affect the remaining iteration (matches
    // `Map.prototype.forEach` precedent).
    let entries = sort_and_combine(ctx.vm, id);
    let headers_val = JsValue::Object(id);
    for (name_sid, value_sid) in entries {
        // WHATWG §5.2 forEach order: callback(value, name, headers).
        ctx.call_function(
            callback,
            this_arg,
            &[
                JsValue::String(value_sid),
                JsValue::String(name_sid),
                headers_val,
            ],
        )?;
    }
    Ok(JsValue::Undefined)
}

pub(super) fn native_headers_keys(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_headers_this(ctx, this, "keys")?;
    let entries = sort_and_combine(ctx.vm, id);
    let arr: Vec<JsValue> = entries
        .into_iter()
        .map(|(n, _)| JsValue::String(n))
        .collect();
    Ok(wrap_in_array_iterator(ctx, arr))
}

pub(super) fn native_headers_values(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_headers_this(ctx, this, "values")?;
    let entries = sort_and_combine(ctx.vm, id);
    let arr: Vec<JsValue> = entries
        .into_iter()
        .map(|(_, v)| JsValue::String(v))
        .collect();
    Ok(wrap_in_array_iterator(ctx, arr))
}

pub(super) fn native_headers_entries(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_headers_this(ctx, this, "entries")?;
    let entries = sort_and_combine(ctx.vm, id);
    let pairs: Vec<JsValue> = entries
        .into_iter()
        .map(|(n, v)| {
            JsValue::Object(
                ctx.vm
                    .create_array_object(vec![JsValue::String(n), JsValue::String(v)]),
            )
        })
        .collect();
    Ok(wrap_in_array_iterator(ctx, pairs))
}
