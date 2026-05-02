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
//! `response` / `request-no-cors`) gates mutation.  Three variants
//! are implemented:
//!
//! - `None` — fully mutable; standalone `new Headers(...)` default.
//! - `Immutable` — every mutating method throws `TypeError`;
//!   installed by the `Response` ctor (and by `Response.error()` /
//!   `.redirect()` / `.json()`, and by `fetch()` when wrapping a
//!   broker response).
//! - `Request` — silent no-op for WHATWG Fetch §4.6 forbidden
//!   request header names (`Cookie`, `Host`, `Origin`, `Referer`,
//!   `Set-Cookie`, the `Sec-` / `Proxy-` prefixes, …); mutations
//!   on non-forbidden names succeed normally.  Installed on the
//!   `Request` ctor's companion Headers.  Spec semantics are
//!   "ignore", not "throw" — verified against Chrome / Firefox /
//!   Safari and asserted in `tests_forbidden_headers.rs`.
//!
//! `response` / `request-no-cors` are not yet implemented; they
//! arrive with PR5-cors (full mode/credentials/redirect
//! enforcement) — see `m4-12-post-pr5a-fetch-roadmap.md`.
//!
//! ## Module layout
//!
//! Split across this directory to keep each file under the
//! project's 1000-line convention:
//!
//! - [`mod@parse_init`] — `init` parsing for `new Headers(init)` and
//!   the shared `init.headers` path used by `Request` / `fetch`.
//! - [`mod@methods`] — the JS-facing `Headers.prototype.*` natives.
//! - [`mod@iteration`] — `sort and combine` + `combine` (`get` join)
//!   + `ArrayIterator` snapshot wrapper.
//! - [`mod@validation`] — RFC 7230 / WHATWG §5.2 name + value
//!   normalisation and the §2.2.2 forbidden-name set.
//!
//! State enums + struct + the registration / allocation entry
//! points + the small helpers shared across submodules live in this
//! file.
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

mod iteration;
mod methods;
mod parse_init;
mod validation;

use super::super::shape::{self, PropertyAttrs};
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, StringId, VmError,
};
use super::super::{NativeFn, VmInner};

// Re-exports — preserve the external `super::headers::*` paths after
// the split so consumers in `host::*` (`request_response`, `fetch`,
// `body_mixin`) keep working unchanged.
pub(in crate::vm::host) use iteration::join_values_comma_space;
pub(in crate::vm::host) use parse_init::{fill_headers_from_init, parse_headers_init_entries};
pub(in crate::vm::host) use validation::{is_forbidden_request_header, validate_and_normalise};
// `validate_and_normalise_name` stays inside the headers/ module —
// only `methods.rs` consumes it (`Headers.prototype.{get,has,delete}`).
use validation::validate_and_normalise_name;

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
    pub(super) fn new(guard: HeadersGuard) -> Self {
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

        let ctor =
            self.create_constructable_function("Headers", methods::native_headers_constructor);
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
            (wk.append, methods::native_headers_append as NativeFn),
            (wk.set, methods::native_headers_set as NativeFn),
            (wk.delete_str, methods::native_headers_delete as NativeFn),
            (wk.get, methods::native_headers_get as NativeFn),
            (wk.has, methods::native_headers_has as NativeFn),
            (
                wk.get_set_cookie,
                methods::native_headers_get_set_cookie as NativeFn,
            ),
            (wk.for_each, methods::native_headers_for_each as NativeFn),
            (wk.keys, methods::native_headers_keys as NativeFn),
            (wk.values, methods::native_headers_values as NativeFn),
            (wk.entries, methods::native_headers_entries as NativeFn),
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
// Helpers shared across submodules
// ---------------------------------------------------------------------------

/// Resolve `this` to a `Headers` ObjectId.  Returns `TypeError` for
/// any other receiver — per WebIDL §3.2 "interface checks", off-brand
/// invocations (e.g. `Headers.prototype.get.call({})`) must throw
/// rather than silently producing `undefined`.
pub(super) fn require_headers_this(
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
pub(super) fn require_mutable(
    ctx: &NativeContext<'_>,
    id: ObjectId,
    method: &str,
) -> Result<(), VmError> {
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
/// already-lowercased `name_sid` should be silently ignored under
/// that guard.  Currently only `HeadersGuard::Request` short-
/// circuits (forbidden request header names per WHATWG Fetch §4.6);
/// other guards return `false` so existing append/set/delete
/// behaviour is unchanged.
///
/// Takes `StringId` (not `&str`) and resolves to UTF-8 *only* when
/// the guard is actually `Request` — the common `None`/`Immutable`
/// paths skip the [`super::super::pools::StringPool::get_utf8`]
/// allocation entirely (R10.2).
pub(super) fn is_blocked_by_guard(
    ctx: &NativeContext<'_>,
    headers_id: ObjectId,
    name_sid: StringId,
) -> bool {
    let guard = ctx
        .vm
        .headers_states
        .get(&headers_id)
        .map_or(HeadersGuard::None, |s| s.guard);
    if !matches!(guard, HeadersGuard::Request) {
        return false;
    }
    let name = ctx.vm.strings.get_utf8(name_sid);
    is_forbidden_request_header(&name)
}

/// Extract `(name, value)` args from a 2-argument method call,
/// coercing each via `ToString`.  Missing args → `TypeError`
/// matching Chromium's wording.
pub(super) fn take_name_value_args(
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

pub(super) fn take_name_arg(
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

/// Low-level append.  Callers are responsible for validation (we
/// deliberately do not re-validate here — e.g. `Headers` → `Headers`
/// copy skips revalidation because the source is already clean).
pub(super) fn append_entry(
    ctx: &mut NativeContext<'_>,
    headers_id: ObjectId,
    name_sid: StringId,
    value_sid: StringId,
) -> Result<(), VmError> {
    // Forbidden-name filter for the two callers that funnel here:
    // [`methods::native_headers_append`] (JS-facing `Headers.append`) and
    // [`fill_headers_from_init`] (init.headers ctor parse).
    // Spec semantics: silent ignore, not throw.
    //
    // Other internal mutators (`copy_headers_entries` /
    // `ensure_content_type` in `request_response.rs`) bypass this
    // filter by pushing onto `state.list` directly — the bypass is
    // safe because (a) `copy_headers_entries` only ever copies from
    // an already-filtered Request-guarded source, and (b)
    // `ensure_content_type` only adds the `content-type` header,
    // which is not in WHATWG Fetch §4.6's forbidden list.  Routing
    // those through `append_entry` would be redundant work on a
    // hot Request-clone path.
    if is_blocked_by_guard(ctx, headers_id, name_sid) {
        return Ok(());
    }
    if let Some(state) = ctx.vm.headers_states.get_mut(&headers_id) {
        state.list.push((name_sid, value_sid));
    }
    Ok(())
}
