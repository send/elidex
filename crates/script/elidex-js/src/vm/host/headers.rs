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
/// every modifying method with `TypeError`.  The request /
/// response / request-no-cors variants will arrive with the
/// `Request` / `Response` ctors when they grow their own
/// companion Headers — those guards demand richer validation than
/// the plain mutable/immutable distinction can express.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HeadersGuard {
    /// Fully mutable (standalone `new Headers(...)` default).
    None,
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
            let name = self.strings.get_utf8(name_sid);
            let fn_id = self.create_native_function(&name, func);
            if name_sid == entries_sid {
                entries_fn_id = Some(fn_id);
            }
            self.define_shaped_property(
                proto_id,
                PropertyKey::String(name_sid),
                PropertyValue::Data(JsValue::Object(fn_id)),
                PropertyAttrs::METHOD,
            );
        }

        // `Headers.prototype[Symbol.iterator] === Headers.prototype.entries`
        // (WHATWG §5.2, matching `Map.prototype` precedent).  Reuse the
        // exact `ObjectId` installed above so the identity comparison
        // holds — allocating a fresh native function for the symbol
        // slot would make `p[@@iterator] === p.entries` observably
        // `false`.
        let iter_fn = entries_fn_id.expect("entries method id not captured during install");
        let sym_iter_key = PropertyKey::Symbol(self.well_known_symbols.iterator);
        self.define_shaped_property(
            proto_id,
            sym_iter_key,
            PropertyValue::Data(JsValue::Object(iter_fn)),
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
            // Otherwise treat as Record<ByteString, ByteString>:
            // iterate own enumerable string keys (§9.1.11.1 order).
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

/// Validate one `[name, value]` pair from the Array-init form
/// and return the normalised `(name_sid, value_sid)` tuple.
fn validate_pair_entry(
    ctx: &mut NativeContext<'_>,
    pair: JsValue,
    error_prefix: &str,
) -> Result<(StringId, StringId), VmError> {
    let JsValue::Object(pair_id) = pair else {
        return Err(VmError::type_error(format!(
            "{error_prefix}: Sequence header init must contain arrays of length 2"
        )));
    };
    let pair_elems = match &ctx.vm.get_object(pair_id).kind {
        ObjectKind::Array { elements } if elements.len() == 2 => elements.clone(),
        _ => {
            return Err(VmError::type_error(format!(
                "{error_prefix}: Sequence header init must contain arrays of length 2"
            )));
        }
    };
    let name_sid = super::super::coerce::to_string(ctx.vm, pair_elems[0])?;
    let value_sid = super::super::coerce::to_string(ctx.vm, pair_elems[1])?;
    validate_and_normalise(ctx.vm, name_sid, value_sid, error_prefix)
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

/// RFC 7230 ABNF `tchar` — the permitted bytes in a header field
/// name.  Used by [`is_valid_header_name`]; any other byte
/// (including non-ASCII, CR, LF, NUL, whitespace, or the delimiter
/// characters) disqualifies the name.
#[inline]
fn is_tchar(b: u8) -> bool {
    matches!(
        b,
        b'!' | b'#'
            | b'$'
            | b'%'
            | b'&'
            | b'\''
            | b'*'
            | b'+'
            | b'-'
            | b'.'
            | b'^'
            | b'_'
            | b'`'
            | b'|'
            | b'~'
            | b'0'..=b'9'
            | b'A'..=b'Z'
            | b'a'..=b'z'
    )
}

fn is_valid_header_name(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(is_tchar)
}

/// Spec §5.2 "header value": no `0x0D` (CR), `0x0A` (LF), or
/// `0x00` (NUL).  Leading / trailing HTAB (`0x09`) and SP (`0x20`)
/// must be trimmed *before* validation per §5.2 "normalize a byte
/// sequence"; [`validate_and_normalise`] handles the trim.
fn is_valid_header_value_content(s: &str) -> bool {
    !s.bytes().any(|b| matches!(b, 0x00 | 0x0A | 0x0D))
}

/// Trim leading/trailing HTAB (`0x09`) + SP (`0x20`) per WHATWG
/// Fetch §5.2 "normalize a byte sequence".  Works on bytes directly
/// because the spec only trims ASCII whitespace, and the input
/// must already be ASCII for validation to have any chance of
/// passing.
fn trim_http_whitespace(s: &str) -> &str {
    s.trim_matches(|c: char| c == '\t' || c == ' ')
}

/// Combined validation + interning pass: lowercase the name, trim
/// the value, validate both, return interned `(name, value)`.
///
/// Returns `TypeError` per WHATWG Fetch §5.2 validation steps —
/// `DOMException` would be the wrong choice here (spec says
/// `TypeError`).
/// `pub(super)` so the `fetch` module can route broker-delivered
/// response headers through the same name/value invariants as
/// script-constructed Headers (§5.2 normalisation — lowercased
/// name, trimmed value, no CR/LF/NUL).
pub(super) fn validate_and_normalise(
    vm: &mut VmInner,
    name_sid: StringId,
    value_sid: StringId,
    error_prefix: &str,
) -> Result<(StringId, StringId), VmError> {
    let name_sid = validate_and_normalise_name(vm, name_sid, error_prefix)?;
    let value_raw = vm.strings.get_utf8(value_sid);
    let trimmed = trim_http_whitespace(&value_raw);
    if !is_valid_header_value_content(trimmed) {
        return Err(VmError::type_error(format!(
            "{error_prefix}: Invalid header value — contains CR, LF, or NUL"
        )));
    }
    // Re-intern the trimmed form only if trimming changed bytes —
    // otherwise keep the original StringId so repeated adds share
    // pool entries.
    let value_sid = if trimmed.len() == value_raw.len() {
        value_sid
    } else {
        vm.strings.intern(trimmed)
    };
    Ok((name_sid, value_sid))
}

fn validate_and_normalise_name(
    vm: &mut VmInner,
    name_sid: StringId,
    error_prefix: &str,
) -> Result<StringId, VmError> {
    let raw = vm.strings.get_utf8(name_sid);
    if !is_valid_header_name(&raw) {
        return Err(VmError::type_error(format!(
            "{error_prefix}: Invalid header name '{raw}' — must match RFC 7230 token syntax"
        )));
    }
    let lower = raw.to_ascii_lowercase();
    // Avoid the re-intern if already lowercase (common case for
    // wire-format names like `content-type`).
    let name_sid = if lower == raw {
        name_sid
    } else {
        vm.strings.intern(&lower)
    };
    Ok(name_sid)
}

/// Low-level append.  Callers are responsible for validation (we
/// deliberately do not re-validate here — e.g. `Headers` → `Headers`
/// copy skips revalidation because the source is already clean).
fn append_entry(
    ctx: &mut NativeContext<'_>,
    headers_id: ObjectId,
    name_sid: StringId,
    value_sid: StringId,
) -> Result<(), VmError> {
    if let Some(state) = ctx.vm.headers_states.get_mut(&headers_id) {
        state.list.push((name_sid, value_sid));
    }
    Ok(())
}

/// Join every `StringId` in `values` with `", "` into a single
/// interned `StringId`.  Used by `get` for multi-valued headers.
fn join_values_comma_space(vm: &mut VmInner, values: &[StringId]) -> StringId {
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
    // if b is code-unit less than a").  Header-name validation
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
