//! `Headers` interface (WHATWG Fetch §5.2).
//!
//! `Headers` is a WebIDL interface that is **not** an `EventTarget`
//! and **not** a `Node` — its prototype chain is:
//!
//! ```text
//! Headers instance (ObjectKind::Headers, payload-free)
//!   → Headers.prototype   (this module)
//!     → Object.prototype
//! ```
//!
//! ## State storage
//!
//! Per-instance state ([`HeadersState`]) lives **out-of-band** in
//! [`super::super::VmInner::headers_states`], keyed by the instance's
//! own `ObjectId`.  The variant [`super::super::value::ObjectKind::Headers`]
//! is payload-free so the per-variant size discipline of
//! [`super::super::value::ObjectKind`] is preserved.  Entries carry
//! only interned `StringId`s (pool-permanent), so the GC trace step
//! has nothing to mark; the sweep tail prunes dead entries the same
//! way `abort_signal_states` / `dom_exception_states` are pruned.
//!
//! ## Storage model
//!
//! The header list is stored in **insertion order** as
//! `Vec<(StringId, StringId)>` — the first component is the name
//! **already lowercased and interned**, the second is the value
//! (interned verbatim, CR/LF/NUL already rejected at the entry
//! point).  Lookups (`get` / `has` / `delete` / `getSetCookie`)
//! compare on the lowercase name `StringId` directly, which is an
//! O(n) sweep but acceptable given typical Fetch headers only
//! carry ~10-50 entries.  A HashMap on top would cost more per
//! small-headers request than it saves.
//!
//! Iteration (`entries` / `keys` / `values` / `forEach` / `@@iterator`)
//! uses WHATWG Fetch §5.2 "sort and combine": lowercase names are
//! sorted, entries with identical names are joined with `", "`,
//! **except** `set-cookie` which produces one iteration entry per
//! occurrence.  A snapshot `Array` is built at call time and wrapped
//! in the existing `ArrayIterator`, so subsequent mutations to the
//! `Headers` instance are not reflected — matches `Map.prototype.entries`
//! / `Set.prototype.entries` precedent.
//!
//! ## Guard
//!
//! The WebIDL `guard` enum (`none` / `immutable` / `request` /
//! `response` / `request-no-cors`) gates mutation.  Only `None`
//! (fully mutable) and `Immutable` (every mutating method throws
//! `TypeError`) are implemented here; the `Response` / `Request`
//! ctors will install `Immutable` on their own companion Headers,
//! and the `request` / `response` / `request-no-cors` variants
//! arrive with those ctors (they demand richer validation than the
//! plain mutable/immutable distinction can express).
//!
//! ## Implemented
//!
//! - `new Headers(init?)` — `init` is `Headers` / `Record<string,string>`
//!   / `Array<[string, string]>` / `null` / `undefined`.
//! - `headers.append(name, value)` / `.set(name, value)` /
//!   `.delete(name)` / `.get(name)` / `.has(name)` / `.getSetCookie()`
//!   / `.forEach(callback, thisArg?)` / `.keys()` / `.values()` /
//!   `.entries()` / `[@@iterator]` (aliased to `.entries()`).

#![cfg(feature = "engine")]

use super::super::shape::{self, PropertyAttrs};
use super::super::value::{
    ArrayIterState, JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey,
    PropertyStorage, PropertyValue, StringId, VmError,
};
use super::super::{NativeFn, VmInner};

/// WebIDL `Headers` guard — WHATWG Fetch §5.2.
///
/// Gates mutation.  `None` is fully mutable; `Immutable` rejects
/// every modifying method with `TypeError`.  `Request` rejects
/// **silently** (no throw) for WHATWG Fetch §4.6 forbidden request
/// header names; mutations on non-forbidden names succeed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HeadersGuard {
    /// Fully mutable (standalone `new Headers(...)` default).
    None,
    /// Companion Headers of a `Request` instance (WHATWG Fetch
    /// §5.3).  Mutating methods that target a §4.6 forbidden
    /// request header name silently return without modifying the
    /// list — the spec says these are "ignored", not "throw".
    /// Non-forbidden names mutate normally.  Switching guard from
    /// `None` to `Request` blocks late drift: a script that does
    /// `req.headers.set('Cookie', '...')` after construction is
    /// also silently dropped, matching browsers.
    Request,
    /// Every mutating method throws `TypeError`.  Installed by
    /// the `Response` ctor (WHATWG Fetch §5.5 step 11), by
    /// `Response.error()` / `.redirect()` / `.json()`, and by
    /// `fetch()` when wrapping a broker response.  Observable
    /// from script — e.g. `new Response('').headers.append(...)`
    /// throws.
    Immutable,
}

/// Per-`Headers` mutable state.  Stored out-of-band keyed by the
/// instance's `ObjectId` in [`super::super::VmInner::headers_states`]
/// so [`super::super::value::ObjectKind::Headers`] stays payload-free.
#[derive(Debug)]
pub(crate) struct HeadersState {
    /// Header list in insertion order.  First component is the
    /// **lowercased, interned** name; second is the raw value
    /// (already validated at the entry point).  Same-name entries
    /// are allowed (`append` is a list-append, not a replace) and
    /// are joined with `", "` by `get` / iteration, except for
    /// `set-cookie` which yields separate iteration entries.
    pub(crate) list: Vec<(StringId, StringId)>,
    pub(crate) guard: HeadersGuard,
}

impl HeadersState {
    fn new(guard: HeadersGuard) -> Self {
        Self {
            list: Vec::new(),
            guard,
        }
    }
}

// ---------------------------------------------------------------------------
// Registration (called from register_globals)
// ---------------------------------------------------------------------------

impl VmInner {
    /// Allocate `Headers.prototype`, install its native methods +
    /// `[Symbol.iterator]`, and expose the `Headers` constructor
    /// on `globals`.
    ///
    /// Called from `register_globals()` after `register_prototypes`
    /// (which populates `object_prototype`).
    ///
    /// # Panics
    ///
    /// Panics if `object_prototype` is `None` — would mean
    /// `register_prototypes` was skipped or the wrong order.
    pub(in crate::vm) fn register_headers_global(&mut self) {
        let object_proto = self
            .object_prototype
            .expect("register_headers_global called before register_prototypes");

        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });
        self.install_headers_methods(proto_id);
        self.headers_prototype = Some(proto_id);

        let ctor = self.create_constructable_function("Headers", native_headers_constructor);
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
        let name_sid = self.well_known.headers_global;
        self.globals.insert(name_sid, JsValue::Object(ctor));
    }

    fn install_headers_methods(&mut self, proto_id: ObjectId) {
        let wk = &self.well_known;
        let entries = [
            (wk.append, native_headers_append as NativeFn),
            (wk.set, native_headers_set as NativeFn),
            (wk.delete_str, native_headers_delete as NativeFn),
            (wk.get, native_headers_get as NativeFn),
            (wk.has, native_headers_has as NativeFn),
            (wk.get_set_cookie, native_headers_get_set_cookie as NativeFn),
            (wk.for_each, native_headers_for_each as NativeFn),
            (wk.keys, native_headers_keys as NativeFn),
            (wk.values, native_headers_values as NativeFn),
            (wk.entries, native_headers_entries as NativeFn),
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

        // `Headers.prototype[Symbol.iterator] === Headers.prototype.entries`
        // (WHATWG §5.2, matching `Map.prototype` precedent).  Reuse the
        // exact `ObjectId` installed above so the identity comparison
        // holds — allocating a fresh native function for the symbol
        // slot would make `p[@@iterator] === p.entries` observably
        // `false`.
        let sym_iter_key = PropertyKey::Symbol(self.well_known_symbols.iterator);
        self.define_shaped_property(
            proto_id,
            sym_iter_key,
            PropertyValue::Data(JsValue::Object(entries_fn_id)),
            PropertyAttrs::METHOD,
        );
    }

    /// Allocate a fresh `Headers` instance with its state row
    /// installed in [`Self::headers_states`].  Used by the
    /// `Request` / `Response` / `fetch()` paths (and
    /// `Response.redirect` / `.error` / `.json` statics) to
    /// allocate their companion Headers with the appropriate
    /// guard; the JS-visible `Headers` constructor itself inlines
    /// this because it must repurpose the `do_new`-allocated
    /// receiver to preserve the `new.target.prototype` chain.
    pub(in crate::vm) fn create_headers(&mut self, guard: HeadersGuard) -> ObjectId {
        let proto = self.headers_prototype;
        let id = self.alloc_object(Object {
            kind: ObjectKind::Headers,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: proto,
            extensible: true,
        });
        self.headers_states.insert(id, HeadersState::new(guard));
        id
    }
}

// ---------------------------------------------------------------------------
// Constructor
// ---------------------------------------------------------------------------

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
fn native_headers_constructor(
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

/// Populate `headers_id` from an `init` value per WHATWG Fetch
/// §5.2 "fill a Headers object".
///
/// `pub(super)` so the `request_response` module can reuse this
/// logic for its `init.headers` member (avoids a parallel
/// reimplementation — validation / lowercase / revalidation
/// skipping on `Headers`-source all stay in one place).
pub(super) fn fill_headers_from_init(
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
pub(super) fn parse_headers_init_entries(
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
            let keys = super::super::coerce_format::collect_own_keys_es_order(ctx.vm, obj_id);
            let mut out = Vec::with_capacity(keys.len());
            for key_sid in keys {
                let value = ctx.get_property_value(obj_id, PropertyKey::String(key_sid))?;
                let value_sid = super::super::coerce::to_string(ctx.vm, value)?;
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
    let name_sid = super::super::coerce::to_string(ctx.vm, name_val)?;
    let value_sid = super::super::coerce::to_string(ctx.vm, value_val)?;
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

// ---------------------------------------------------------------------------
// Methods
// ---------------------------------------------------------------------------

fn native_headers_append(
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
    // Forbidden-name filter lives inside `append_entry` so internal
    // callers (init.headers parse / `copy_headers_entries` / default
    // Content-Type splice) get the same silent-drop behaviour as
    // `Headers.append`.  No second check here.
    append_entry(ctx, id, name_sid, value_sid)?;
    Ok(JsValue::Undefined)
}

fn native_headers_set(
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
    let lower = ctx.vm.strings.get_utf8(name_sid);
    if is_blocked_by_guard(ctx, id, &lower) {
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

fn native_headers_delete(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_headers_this(ctx, this, "delete")?;
    require_mutable(ctx, id, "delete")?;
    let name_sid = take_name_arg(ctx, args, "delete")?;
    let name_sid =
        validate_and_normalise_name(ctx.vm, name_sid, "Failed to execute 'delete' on 'Headers'")?;
    let lower = ctx.vm.strings.get_utf8(name_sid);
    if is_blocked_by_guard(ctx, id, &lower) {
        return Ok(JsValue::Undefined);
    }
    if let Some(state) = ctx.vm.headers_states.get_mut(&id) {
        state.list.retain(|(n, _)| *n != name_sid);
    }
    Ok(JsValue::Undefined)
}

fn native_headers_get(
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

fn native_headers_has(
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

fn native_headers_get_set_cookie(
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

fn native_headers_for_each(
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

fn native_headers_keys(
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

fn native_headers_values(
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

fn native_headers_entries(
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

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Resolve `this` to a `Headers` ObjectId.  Returns `TypeError` for
/// any other receiver — per WebIDL §3.2 "interface checks", off-brand
/// invocations (e.g. `Headers.prototype.get.call({})`) must throw
/// rather than silently producing `undefined`.
fn require_headers_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<ObjectId, VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "Headers.prototype.{method} called on non-Headers"
        )));
    };
    if matches!(ctx.vm.get_object(id).kind, ObjectKind::Headers) {
        Ok(id)
    } else {
        Err(VmError::type_error(format!(
            "Headers.prototype.{method} called on non-Headers"
        )))
    }
}

/// Reject mutations on an `Immutable`-guarded Headers (WHATWG Fetch
/// §5.2 step 1 of `append` / `set` / `delete`).
fn require_mutable(ctx: &NativeContext<'_>, id: ObjectId, method: &str) -> Result<(), VmError> {
    let guard = ctx
        .vm
        .headers_states
        .get(&id)
        .map_or(HeadersGuard::None, |s| s.guard);
    if matches!(guard, HeadersGuard::Immutable) {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'Headers': Headers are immutable."
        )));
    }
    Ok(())
}

/// Look up the guard on `headers_id` and return `true` if the
/// already-lowercased `name` should be silently ignored under that
/// guard.  Currently only `HeadersGuard::Request` short-circuits
/// (forbidden request header names per WHATWG Fetch §4.6); other
/// guards return `false` so existing append/set/delete behaviour
/// is unchanged.
fn is_blocked_by_guard(ctx: &NativeContext<'_>, headers_id: ObjectId, name: &str) -> bool {
    let guard = ctx
        .vm
        .headers_states
        .get(&headers_id)
        .map_or(HeadersGuard::None, |s| s.guard);
    matches!(guard, HeadersGuard::Request) && is_forbidden_request_header(name)
}

/// Extract `(name, value)` args from a 2-argument method call,
/// coercing each via `ToString`.  Missing args → `TypeError`
/// matching Chromium's wording.
fn take_name_value_args(
    ctx: &mut NativeContext<'_>,
    args: &[JsValue],
    method: &str,
) -> Result<(StringId, StringId), VmError> {
    if args.len() < 2 {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'Headers': 2 arguments required, but only {} present.",
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
            "Failed to execute '{method}' on 'Headers': 1 argument required, but only 0 present."
        )));
    }
    super::super::coerce::to_string(ctx.vm, args[0])
}

// The RFC 7230 `tchar` classifier + `validate_and_normalise` / its
// name-only companion live in the dedicated
// [`headers_validation`](super::headers_validation) module — pure
// functions over the string pool, no Headers-state dependency.
// Re-export the one symbol the rest of `host::` consumes (fetch.rs
// routes broker response headers through it) so external call
// sites stay at `super::headers::validate_and_normalise` (R23.1
// split keeps this file under the project's 1000-line convention).
pub(super) use super::headers_validation::{
    is_forbidden_request_header, validate_and_normalise, validate_and_normalise_name,
};

/// Low-level append.  Callers are responsible for validation (we
/// deliberately do not re-validate here — e.g. `Headers` → `Headers`
/// copy skips revalidation because the source is already clean).
fn append_entry(
    ctx: &mut NativeContext<'_>,
    headers_id: ObjectId,
    name_sid: StringId,
    value_sid: StringId,
) -> Result<(), VmError> {
    // Forbidden-name filter applies to every internal append site
    // (init.headers parse during ctor, default Content-Type splice,
    // copy_headers_entries from another Headers instance — all of
    // which call here).  Spec semantics: silent ignore, not throw.
    let lower = ctx.vm.strings.get_utf8(name_sid);
    if is_blocked_by_guard(ctx, headers_id, &lower) {
        return Ok(());
    }
    if let Some(state) = ctx.vm.headers_states.get_mut(&headers_id) {
        state.list.push((name_sid, value_sid));
    }
    Ok(())
}

/// Join every `StringId` in `values` with `", "` into a single
/// interned `StringId` (WHATWG Fetch §5.2 `combine` algorithm).
/// Used by `Headers.get` for multi-valued headers and by
/// [`super::body_mixin::content_type_of`] so `Blob.type` and
/// `resp.headers.get('content-type')` always agree on the
/// combined form — `pub(super)` so body-mixin can share.
///
/// **Caller contract**: `values` must be non-empty.  A zero-length
/// input is a logic error (the caller should short-circuit to
/// the "no matching header" sentinel before calling); the body
/// still returns the empty interned string in that case, which
/// is harmless but not a defined output.
pub(super) fn join_values_comma_space(vm: &mut VmInner, values: &[StringId]) -> StringId {
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
fn sort_and_combine(vm: &mut VmInner, headers_id: ObjectId) -> Vec<(StringId, StringId)> {
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
/// freed slot.  The [`VmTempRoot`] RAII guard holds `arr_id`
/// on the VM stack until the iterator is fully constructed
/// and installed (at which point the iterator's
/// `ArrayIterState.array_id` field becomes a proper strong
/// reference that the GC trace picks up).
fn wrap_in_array_iterator(ctx: &mut NativeContext<'_>, elements: Vec<JsValue>) -> JsValue {
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
