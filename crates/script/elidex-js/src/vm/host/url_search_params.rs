//! `URLSearchParams` interface (WHATWG URL §6).
//!
//! A `URLSearchParams` instance is a WebIDL interface rooted at
//! `Object` — not an `EventTarget`, not a `Node`.  Prototype chain:
//!
//! ```text
//! URLSearchParams instance (ObjectKind::URLSearchParams, payload-free)
//!   → URLSearchParams.prototype  (this module)
//!     → Object.prototype
//! ```
//!
//! ## State storage
//!
//! Per-instance state lives **out-of-band** in
//! [`super::super::VmInner::url_search_params_states`], keyed by the
//! instance's own `ObjectId`.  The variant
//! [`super::super::value::ObjectKind::URLSearchParams`] is payload-free
//! so the per-variant size discipline of
//! [`super::super::value::ObjectKind`] is preserved.  Entries carry
//! only interned `StringId`s (pool-permanent), so the GC trace step
//! has nothing to mark; the sweep tail prunes dead entries the same
//! way `headers_states` is pruned.
//!
//! ## Implemented
//!
//! - `new URLSearchParams(init?)` — `init` accepts a `String`
//!   (URL-encoded query, optional leading `?`), an `Array<[name,
//!   value]>` (sequence of pairs), an iterable of pairs, a record
//!   (own enumerable string keys), or another `URLSearchParams`
//!   instance.
//! - `.append(name, value)` / `.delete(name, value?)` /
//!   `.get(name)` / `.getAll(name)` / `.has(name, value?)` /
//!   `.set(name, value)`.
//! - `.sort()` — stable sort by name (insertion order tiebreak),
//!   per WHATWG §6.2.
//! - `.toString()` — re-serialises the entry list via
//!   `application/x-www-form-urlencoded`.
//! - `.forEach(callback, thisArg?)` / `.keys()` / `.values()` /
//!   `.entries()` / `[@@iterator]` (aliased to `.entries()`).
//! - `.size` IDL readonly attr — entry-list length.
//!
//! ## Wire format
//!
//! `toString()` returns the canonical
//! `application/x-www-form-urlencoded` serialisation that the Fetch
//! body extraction path consumes.  Both the Content-Type
//! (`application/x-www-form-urlencoded;charset=UTF-8`) and the body
//! bytes are agreed on by [`super::request_response::extract_body_bytes`]
//! and [`super::request_response::content_type_for_body`] via the
//! [`serialize`] helper here, so the `Request` / `Response` /
//! `fetch()` paths cannot drift from the JS-visible `toString()`
//! output.

#![cfg(feature = "engine")]

use super::super::shape::{self, PropertyAttrs};
use super::super::value::{
    ArrayIterState, JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey,
    PropertyStorage, PropertyValue, StringId, VmError,
};
use super::super::{NativeFn, VmInner};

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

impl VmInner {
    /// Allocate `URLSearchParams.prototype`, install its method
    /// suite + `[Symbol.iterator]`, and expose the
    /// `URLSearchParams` constructor on `globals`.
    ///
    /// Called from `register_globals()` after `register_prototypes`
    /// (which populates `object_prototype`).
    ///
    /// # Panics
    ///
    /// Panics if `object_prototype` is `None` — would mean
    /// `register_prototypes` was skipped or the wrong order.
    pub(in crate::vm) fn register_url_search_params_global(&mut self) {
        let object_proto = self
            .object_prototype
            .expect("register_url_search_params_global called before register_prototypes");

        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });
        self.install_url_search_params_members(proto_id);
        self.url_search_params_prototype = Some(proto_id);

        let ctor = self
            .create_constructable_function("URLSearchParams", native_url_search_params_constructor);
        let proto_key = PropertyKey::String(self.well_known.prototype);
        self.define_shaped_property(
            ctor,
            proto_key,
            PropertyValue::Data(JsValue::Object(proto_id)),
            PropertyAttrs::BUILTIN,
        );
        let ctor_key = PropertyKey::String(self.well_known.constructor);
        self.define_shaped_property(
            proto_id,
            ctor_key,
            PropertyValue::Data(JsValue::Object(ctor)),
            PropertyAttrs::METHOD,
        );
        let name_sid = self.well_known.url_search_params_global;
        self.globals.insert(name_sid, JsValue::Object(ctor));
    }

    fn install_url_search_params_members(&mut self, proto_id: ObjectId) {
        // `size` IDL readonly accessor — WHATWG URL §6 step "size getter".
        self.install_accessor_pair(
            proto_id,
            self.well_known.size,
            native_usp_get_size,
            None,
            PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );

        let wk = &self.well_known;
        let entries = [
            (wk.append, native_usp_append as NativeFn),
            (wk.delete_str, native_usp_delete as NativeFn),
            (wk.get, native_usp_get as NativeFn),
            (wk.get_all, native_usp_get_all as NativeFn),
            (wk.has, native_usp_has as NativeFn),
            (wk.set, native_usp_set as NativeFn),
            (wk.sort, native_usp_sort as NativeFn),
            (wk.to_string_method, native_usp_to_string as NativeFn),
            (wk.for_each, native_usp_for_each as NativeFn),
            (wk.keys, native_usp_keys as NativeFn),
            (wk.values, native_usp_values as NativeFn),
            (wk.entries, native_usp_entries as NativeFn),
        ];
        let entries_sid = wk.entries;
        let mut entries_fn_id: Option<ObjectId> = None;
        for (name_sid, func) in entries {
            let fn_id = self.install_native_method(proto_id, name_sid, func, PropertyAttrs::METHOD);
            if name_sid == entries_sid {
                entries_fn_id = Some(fn_id);
            }
        }
        let entries_fn_id = entries_fn_id.expect("entries method id not captured during install");

        // `URLSearchParams.prototype[Symbol.iterator] === .entries` —
        // WHATWG §6 IDL `iterable<USVString, USVString>` mirrors
        // `Map.prototype` precedent.  Reuse the exact `ObjectId` so
        // identity comparison holds.
        let sym_iter_key = PropertyKey::Symbol(self.well_known_symbols.iterator);
        self.define_shaped_property(
            proto_id,
            sym_iter_key,
            PropertyValue::Data(JsValue::Object(entries_fn_id)),
            PropertyAttrs::METHOD,
        );
    }
}

// ---------------------------------------------------------------------------
// Constructor
// ---------------------------------------------------------------------------

/// `new URLSearchParams(init?)` (WHATWG URL §6.2).
fn native_url_search_params_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if !ctx.is_construct() {
        return Err(VmError::type_error(
            "Failed to construct 'URLSearchParams': Please use the 'new' operator",
        ));
    }
    let JsValue::Object(id) = this else {
        unreachable!("constructor `this` is always an Object after `do_new`");
    };
    // Promote the pre-allocated Ordinary instance to URLSearchParams
    // — preserves the `new.target.prototype` chain installed by
    // `do_new` (PR5a2 R7.2/R7.3 lesson).
    ctx.vm.get_object_mut(id).kind = ObjectKind::URLSearchParams;
    ctx.vm.url_search_params_states.insert(id, Vec::new());

    let init = args.first().copied().unwrap_or(JsValue::Undefined);
    let entries = parse_init_entries(ctx, init)?;
    if let Some(state) = ctx.vm.url_search_params_states.get_mut(&id) {
        *state = entries;
    }
    Ok(JsValue::Object(id))
}

/// Parse `init` per WHATWG URL §6.2 "URLSearchParams(init)" into
/// the entry list.  Accepts `undefined` / `null` / `String` /
/// `URLSearchParams` / array of pairs / generic iterable of pairs /
/// record (object).
fn parse_init_entries(
    ctx: &mut NativeContext<'_>,
    init: JsValue,
) -> Result<Vec<(StringId, StringId)>, VmError> {
    match init {
        JsValue::Undefined | JsValue::Null => Ok(Vec::new()),
        JsValue::String(sid) => {
            let raw = ctx.vm.strings.get_utf8(sid);
            Ok(parse_query_string(ctx.vm, &raw))
        }
        JsValue::Object(obj_id) => {
            // Source `URLSearchParams` (clone entries directly).
            if matches!(ctx.vm.get_object(obj_id).kind, ObjectKind::URLSearchParams) {
                return Ok(ctx
                    .vm
                    .url_search_params_states
                    .get(&obj_id)
                    .cloned()
                    .unwrap_or_default());
            }
            // Source `Array`: each element must be a length-2
            // sequence of (name, value).
            if let ObjectKind::Array { elements } = &ctx.vm.get_object(obj_id).kind {
                let snapshot = elements.clone();
                let mut out = Vec::with_capacity(snapshot.len());
                for pair in snapshot {
                    out.push(validate_pair_entry(ctx, pair)?);
                }
                return Ok(out);
            }
            // Generic iterable of pairs: per WebIDL §3.10.34 the
            // sequence branch wins over the record branch when
            // `@@iterator` is callable.
            let iter_key = PropertyKey::Symbol(ctx.vm.well_known_symbols.iterator);
            let iter_method = ctx.get_property_value(obj_id, iter_key)?;
            let iter_fn = match iter_method {
                JsValue::Undefined | JsValue::Null => None,
                JsValue::Object(id) if ctx.vm.get_object(id).kind.is_callable() => {
                    Some(iter_method)
                }
                _ => {
                    return Err(VmError::type_error(
                        "Failed to construct 'URLSearchParams': @@iterator is not callable",
                    ));
                }
            };
            if let Some(fn_val) = iter_fn {
                let iter = ctx.vm.call_value(fn_val, init, &[])?;
                if !matches!(iter, JsValue::Object(_)) {
                    return Err(VmError::type_error(
                        "Failed to construct 'URLSearchParams': @@iterator must return an object",
                    ));
                }
                let mut out = Vec::new();
                loop {
                    let pair = match ctx.vm.iter_next(iter)? {
                        Some(p) => p,
                        None => break,
                    };
                    match validate_pair_entry(ctx, pair) {
                        Ok(p) => out.push(p),
                        Err(err) => {
                            let close_err = ctx.vm.iter_close(iter).err();
                            return Err(close_err.unwrap_or(err));
                        }
                    }
                }
                return Ok(out);
            }
            // Record branch: own enumerable string keys + `ToString`
            // each value.
            let keys = super::super::coerce_format::collect_own_keys_es_order(ctx.vm, obj_id);
            let mut out = Vec::with_capacity(keys.len());
            for key_sid in keys {
                let value = ctx.get_property_value(obj_id, PropertyKey::String(key_sid))?;
                let value_sid = super::super::coerce::to_string(ctx.vm, value)?;
                out.push((key_sid, value_sid));
            }
            Ok(out)
        }
        // Other primitives (number / boolean / symbol-throws) fall
        // through to ToString — matches browsers' forgiving init.
        _ => {
            let sid = super::super::coerce::to_string(ctx.vm, init)?;
            let raw = ctx.vm.strings.get_utf8(sid);
            Ok(parse_query_string(ctx.vm, &raw))
        }
    }
}

/// Extract `[name, value]` from an inner pair — fast-path on Array,
/// fall back to the iterator protocol for any other iterable.
/// Mirror of [`super::headers::collect_header_pair_values`] —
/// arity-2 contract is identical, only the validation differs.
fn validate_pair_entry(
    ctx: &mut NativeContext<'_>,
    pair: JsValue,
) -> Result<(StringId, StringId), VmError> {
    let JsValue::Object(pair_id) = pair else {
        return Err(VmError::type_error(
            "Failed to construct 'URLSearchParams': Sequence init must contain iterables of length 2",
        ));
    };
    if let ObjectKind::Array { elements } = &ctx.vm.get_object(pair_id).kind {
        if elements.len() != 2 {
            return Err(VmError::type_error(
                "Failed to construct 'URLSearchParams': Sequence init must contain iterables of length 2",
            ));
        }
        // Snapshot the two values before passing `&mut ctx.vm` to
        // `to_string` (each call may grow the string pool, which
        // re-borrows `vm`).
        let (a, b) = (elements[0], elements[1]);
        let name_sid = super::super::coerce::to_string(ctx.vm, a)?;
        let value_sid = super::super::coerce::to_string(ctx.vm, b)?;
        return Ok((name_sid, value_sid));
    }
    let iter = match ctx.vm.resolve_iterator(pair)? {
        Some(iter @ JsValue::Object(_)) => iter,
        Some(_) => {
            return Err(VmError::type_error(
                "Failed to construct 'URLSearchParams': @@iterator must return an object",
            ));
        }
        None => {
            return Err(VmError::type_error(
                "Failed to construct 'URLSearchParams': Sequence init must contain iterables of length 2",
            ));
        }
    };
    let mut values: Vec<JsValue> = Vec::with_capacity(2);
    loop {
        let v = match ctx.vm.iter_next(iter)? {
            Some(v) => v,
            None => break,
        };
        values.push(v);
        if values.len() > 2 {
            let close_err = ctx.vm.iter_close(iter).err();
            let arity_err = VmError::type_error(
                "Failed to construct 'URLSearchParams': Sequence init must contain iterables of length 2",
            );
            return Err(close_err.unwrap_or(arity_err));
        }
    }
    if values.len() != 2 {
        return Err(VmError::type_error(
            "Failed to construct 'URLSearchParams': Sequence init must contain iterables of length 2",
        ));
    }
    let name_sid = super::super::coerce::to_string(ctx.vm, values[0])?;
    let value_sid = super::super::coerce::to_string(ctx.vm, values[1])?;
    Ok((name_sid, value_sid))
}

/// Parse a URL query string per WHATWG URL §5.1 "application/
/// x-www-form-urlencoded parser" into `(name, value)` pairs.
/// Strips a leading `?` (constructor convenience), then delegates
/// to `url::form_urlencoded::parse`, which already handles
/// `+ → space` and percent decoding.
fn parse_query_string(vm: &mut VmInner, raw: &str) -> Vec<(StringId, StringId)> {
    let stripped = raw.strip_prefix('?').unwrap_or(raw);
    url::form_urlencoded::parse(stripped.as_bytes())
        .map(|(name, value)| (vm.strings.intern(&name), vm.strings.intern(&value)))
        .collect()
}

// ---------------------------------------------------------------------------
// Methods
// ---------------------------------------------------------------------------

fn native_usp_get_size(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_usp_this(ctx, this, "size")?;
    #[allow(clippy::cast_precision_loss)]
    let len = ctx
        .vm
        .url_search_params_states
        .get(&id)
        .map_or(0.0, |s| s.len() as f64);
    Ok(JsValue::Number(len))
}

fn native_usp_append(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_usp_this(ctx, this, "append")?;
    let (name_sid, value_sid) = take_name_value_args(ctx, args, "append")?;
    if let Some(state) = ctx.vm.url_search_params_states.get_mut(&id) {
        state.push((name_sid, value_sid));
    }
    super::url::rewrite_url_query_from_search_params(ctx.vm, id);
    Ok(JsValue::Undefined)
}

/// `delete(name, value?)`.  WHATWG URL §6 (post 2023 update): when
/// `value` is provided, only entries matching both name AND value
/// are removed; otherwise all entries with that name are removed.
fn native_usp_delete(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_usp_this(ctx, this, "delete")?;
    let name_sid = take_name_arg(ctx, args, "delete")?;
    let value_sid = match args.get(1).copied() {
        Some(JsValue::Undefined) | None => None,
        Some(v) => Some(super::super::coerce::to_string(ctx.vm, v)?),
    };
    if let Some(state) = ctx.vm.url_search_params_states.get_mut(&id) {
        match value_sid {
            None => state.retain(|(n, _)| *n != name_sid),
            Some(v) => state.retain(|(n, val)| !(*n == name_sid && *val == v)),
        }
    }
    super::url::rewrite_url_query_from_search_params(ctx.vm, id);
    Ok(JsValue::Undefined)
}

fn native_usp_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_usp_this(ctx, this, "get")?;
    let name_sid = take_name_arg(ctx, args, "get")?;
    let found = ctx
        .vm
        .url_search_params_states
        .get(&id)
        .and_then(|s| s.iter().find(|(n, _)| *n == name_sid).map(|(_, v)| *v));
    Ok(match found {
        Some(v) => JsValue::String(v),
        None => JsValue::Null,
    })
}

fn native_usp_get_all(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_usp_this(ctx, this, "getAll")?;
    let name_sid = take_name_arg(ctx, args, "getAll")?;
    let values: Vec<JsValue> = ctx
        .vm
        .url_search_params_states
        .get(&id)
        .map(|s| {
            s.iter()
                .filter(|(n, _)| *n == name_sid)
                .map(|(_, v)| JsValue::String(*v))
                .collect()
        })
        .unwrap_or_default();
    Ok(JsValue::Object(ctx.vm.create_array_object(values)))
}

/// `has(name, value?)`.  WHATWG URL §6 (post 2023 update): when
/// `value` is provided, returns `true` iff an entry with matching
/// name AND value exists.
fn native_usp_has(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_usp_this(ctx, this, "has")?;
    let name_sid = take_name_arg(ctx, args, "has")?;
    let value_sid = match args.get(1).copied() {
        Some(JsValue::Undefined) | None => None,
        Some(v) => Some(super::super::coerce::to_string(ctx.vm, v)?),
    };
    let present = ctx
        .vm
        .url_search_params_states
        .get(&id)
        .is_some_and(|s| match value_sid {
            None => s.iter().any(|(n, _)| *n == name_sid),
            Some(v) => s.iter().any(|(n, val)| *n == name_sid && *val == v),
        });
    Ok(JsValue::Boolean(present))
}

/// `set(name, value)`.  WHATWG URL §6: replace the *first* matching
/// entry's value and remove *all* other matching entries; if no
/// match exists, append.  Position of the replacement is the
/// position of the first prior occurrence.
fn native_usp_set(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_usp_this(ctx, this, "set")?;
    let (name_sid, value_sid) = take_name_value_args(ctx, args, "set")?;
    if let Some(state) = ctx.vm.url_search_params_states.get_mut(&id) {
        let mut replaced = false;
        state.retain_mut(|(n, v)| {
            if *n != name_sid {
                return true;
            }
            if replaced {
                false
            } else {
                *v = value_sid;
                replaced = true;
                true
            }
        });
        if !replaced {
            state.push((name_sid, value_sid));
        }
    }
    super::url::rewrite_url_query_from_search_params(ctx.vm, id);
    Ok(JsValue::Undefined)
}

/// `sort()` — stable sort by name in code-unit order; values
/// preserve insertion order within each name group (WHATWG URL §6
/// "URLSearchParams sort").
fn native_usp_sort(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_usp_this(ctx, this, "sort")?;
    // Sort by name `&[u16]` code-units — stable so same-name
    // entries keep their insertion order.  `sort_by` borrows the
    // pool immutably; `state` borrow is `&mut` so split via a
    // local clone on the names being compared.  Cheaper: read each
    // name once into a `Vec<&[u16]>` for the comparison key.
    // Field-disjoint borrows: `url_search_params_states` and
    // `strings` are independent `VmInner` fields, so we can hold a
    // `&mut` on the entry list while the comparator reads through
    // `&strings`.  In-place stable `sort_by` avoids the extra
    // index permutation + clone the prior implementation needed.
    {
        let pool = &ctx.vm.strings;
        let Some(state) = ctx.vm.url_search_params_states.get_mut(&id) else {
            return Ok(JsValue::Undefined);
        };
        state.sort_by(|a, b| pool.get(a.0).cmp(pool.get(b.0)));
    }
    super::url::rewrite_url_query_from_search_params(ctx.vm, id);
    Ok(JsValue::Undefined)
}

fn native_usp_to_string(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_usp_this(ctx, this, "toString")?;
    let pairs: Vec<(String, String)> = ctx
        .vm
        .url_search_params_states
        .get(&id)
        .map(|s| {
            s.iter()
                .map(|(n, v)| (ctx.vm.strings.get_utf8(*n), ctx.vm.strings.get_utf8(*v)))
                .collect()
        })
        .unwrap_or_default();
    let serialized = serialize_pairs(&pairs);
    let sid = ctx.vm.strings.intern(&serialized);
    Ok(JsValue::String(sid))
}

fn native_usp_for_each(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_usp_this(ctx, this, "forEach")?;
    let callback = match args.first().copied().unwrap_or(JsValue::Undefined) {
        JsValue::Object(cb_id) if ctx.vm.get_object(cb_id).kind.is_callable() => cb_id,
        _ => {
            return Err(VmError::type_error(
                "Failed to execute 'forEach' on 'URLSearchParams': \
                 parameter 1 is not of type 'Function'.",
            ));
        }
    };
    let this_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    // Snapshot the entry list so callback-side mutations do not
    // affect the remaining iteration (matches `Map.prototype.forEach`
    // / `Headers.prototype.forEach` precedent).
    let entries: Vec<(StringId, StringId)> = ctx
        .vm
        .url_search_params_states
        .get(&id)
        .cloned()
        .unwrap_or_default();
    let usp_val = JsValue::Object(id);
    for (name_sid, value_sid) in entries {
        ctx.call_function(
            callback,
            this_arg,
            &[
                JsValue::String(value_sid),
                JsValue::String(name_sid),
                usp_val,
            ],
        )?;
    }
    Ok(JsValue::Undefined)
}

fn native_usp_keys(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_usp_this(ctx, this, "keys")?;
    let arr: Vec<JsValue> = ctx
        .vm
        .url_search_params_states
        .get(&id)
        .map(|s| s.iter().map(|(n, _)| JsValue::String(*n)).collect())
        .unwrap_or_default();
    Ok(wrap_in_array_iterator(ctx, arr))
}

fn native_usp_values(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_usp_this(ctx, this, "values")?;
    let arr: Vec<JsValue> = ctx
        .vm
        .url_search_params_states
        .get(&id)
        .map(|s| s.iter().map(|(_, v)| JsValue::String(*v)).collect())
        .unwrap_or_default();
    Ok(wrap_in_array_iterator(ctx, arr))
}

fn native_usp_entries(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_usp_this(ctx, this, "entries")?;
    let entries: Vec<(StringId, StringId)> = ctx
        .vm
        .url_search_params_states
        .get(&id)
        .cloned()
        .unwrap_or_default();
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

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn require_usp_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<ObjectId, VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "URLSearchParams.prototype.{method} called on non-URLSearchParams"
        )));
    };
    if matches!(ctx.vm.get_object(id).kind, ObjectKind::URLSearchParams) {
        Ok(id)
    } else {
        Err(VmError::type_error(format!(
            "URLSearchParams.prototype.{method} called on non-URLSearchParams"
        )))
    }
}

fn take_name_value_args(
    ctx: &mut NativeContext<'_>,
    args: &[JsValue],
    method: &str,
) -> Result<(StringId, StringId), VmError> {
    if args.len() < 2 {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'URLSearchParams': 2 arguments required, but only {} present.",
            args.len()
        )));
    }
    let name_sid = super::super::coerce::to_string(ctx.vm, args[0])?;
    let value_sid = super::super::coerce::to_string(ctx.vm, args[1])?;
    Ok((name_sid, value_sid))
}

fn take_name_arg(
    ctx: &mut NativeContext<'_>,
    args: &[JsValue],
    method: &str,
) -> Result<StringId, VmError> {
    if args.is_empty() {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'URLSearchParams': 1 argument required, but only 0 present."
        )));
    }
    super::super::coerce::to_string(ctx.vm, args[0])
}

/// Wrap a snapshot `Vec<JsValue>` in an `ArrayIterator` (kind=0 =
/// Values).  Same shape + GC rooting contract as
/// [`super::headers::wrap_in_array_iterator`]; duplicated here
/// because the helper is `fn`-private to its module.  See that
/// module's docstring for the GC rationale.
fn wrap_in_array_iterator(ctx: &mut NativeContext<'_>, elements: Vec<JsValue>) -> JsValue {
    let arr_id = ctx.vm.create_array_object(elements);
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
    drop(rooted);
    JsValue::Object(iter_id)
}

/// Serialise a list of `(name, value)` pairs per WHATWG URL §5.2
/// "application/x-www-form-urlencoded serializer".  Used by
/// `toString()` and by [`super::request_response::extract_body_bytes`]
/// when wiring a `URLSearchParams` body — both routes go through
/// here so the JS-visible output and the Fetch wire bytes cannot
/// drift.
pub(super) fn serialize_pairs<K: AsRef<str>, V: AsRef<str>>(pairs: &[(K, V)]) -> String {
    let mut serializer = url::form_urlencoded::Serializer::new(String::new());
    for (k, v) in pairs {
        serializer.append_pair(k.as_ref(), v.as_ref());
    }
    serializer.finish()
}

/// Resolve a `URLSearchParams` instance's entry list to UTF-8
/// strings + serialise it for the Fetch body extraction path.
/// `pub(super)` so [`super::request_response::extract_body_bytes`]
/// can call it directly without going through the JS-visible
/// `toString()` round-trip (avoids re-interning the result).
pub(super) fn serialize_for_body(vm: &VmInner, id: ObjectId) -> String {
    let pairs: Vec<(String, String)> = vm
        .url_search_params_states
        .get(&id)
        .map(|s| {
            s.iter()
                .map(|(n, v)| (vm.strings.get_utf8(*n), vm.strings.get_utf8(*v)))
                .collect()
        })
        .unwrap_or_default();
    serialize_pairs(&pairs)
}

/// Allocate a fresh `URLSearchParams` instance, seed its entry list
/// from the parsed `query` (`None` or `""` ⇒ empty list), and
/// register it in [`VmInner::url_search_params_states`].  Returns
/// the new instance's `ObjectId`.
///
/// Used by the `URL` constructor (slot #9.5 Phase 4) to eagerly
/// create the linked `searchParams` so identity is stable
/// (`url.searchParams === url.searchParams` per WHATWG URL §6.1).
/// `pub(super)` keeps the `ObjectKind::URLSearchParams` allocation
/// path single-sourced.
pub(super) fn alloc_url_search_params_from_query(
    vm: &mut VmInner,
    query: Option<&str>,
) -> ObjectId {
    // GC contract: `alloc_object` may run a collection cycle
    // BEFORE returning the new slot, but the `prototype` ObjectId
    // we pass through (`vm.url_search_params_prototype`) is kept
    // alive by the intrinsic-prototype root list (collect.rs
    // proto_roots), and the entry strings are pure StringIds
    // (pool-permanent).  Order of "intern entries" vs
    // "alloc_object" is therefore a pure ergonomics choice — done
    // first here so the entry list is ready to insert in a single
    // tail step rather than re-borrowing `&mut vm.strings` after
    // the allocation.
    let entries: Vec<(StringId, StringId)> = match query {
        Some(q) if !q.is_empty() => url::form_urlencoded::parse(q.as_bytes())
            .map(|(k, v)| (vm.strings.intern(&k), vm.strings.intern(&v)))
            .collect(),
        _ => Vec::new(),
    };
    let proto = vm.url_search_params_prototype;
    let sp_id = vm.alloc_object(Object {
        kind: ObjectKind::URLSearchParams,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });
    vm.url_search_params_states.insert(sp_id, entries);
    sp_id
}
