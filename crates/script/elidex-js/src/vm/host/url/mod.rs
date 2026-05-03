//! `URL` interface (WHATWG URL §6.1).
//!
//! A `URL` instance is a WebIDL interface rooted at `Object`.
//! Prototype chain:
//!
//! ```text
//! URL instance (ObjectKind::URL, payload-free)
//!   → URL.prototype  (this module)
//!     → Object.prototype
//! ```
//!
//! ## State storage
//!
//! Per-instance state lives **out-of-band** in
//! [`super::super::VmInner::url_states`], keyed by the instance's
//! own `ObjectId`.  The variant
//! [`super::super::value::ObjectKind::URL`] is payload-free so the
//! per-variant size discipline of
//! [`super::super::value::ObjectKind`] is preserved.  Each
//! [`UrlState`] holds the parsed [`url::Url`] + the linked
//! `URLSearchParams` `ObjectId` (eagerly allocated by the
//! constructor for `searchParams` identity stability — `url
//! .searchParams === url.searchParams` is required by the spec).
//!
//! ## Implemented surface
//!
//! - `new URL(input, base?)` — both args coerced via
//!   [`super::super::coerce::to_string`].  Relative `input` resolves
//!   against `base` per WHATWG URL §4.4 ("URL parser").  A parse
//!   failure throws `TypeError` (matches V8 / Firefox 2023+).
//! - 12 IDL accessors: `href` / `origin` / `protocol` / `username`
//!   / `password` / `host` / `hostname` / `port` / `pathname` /
//!   `search` / `hash` / `searchParams`.  All are read/write
//!   except `origin` and `searchParams` (read-only per WHATWG).
//!   Setter mutations route through `url::Url::set_*`; the `href`
//!   and `search` setters additionally refresh the linked
//!   `URLSearchParams` entry list.  Getters live in [`accessors`]
//!   and setters in [`setters`].
//! - `searchParams` ↔ URL bidirectional linkage: the constructor
//!   eagerly allocates a `URLSearchParams` for each URL so
//!   `url.searchParams === url.searchParams` holds, and the
//!   `URLSearchParams` mutators (`append` / `delete` / `set` /
//!   `sort`) write back into the parent URL's query through
//!   `usp_parent_url`.
//! - `.toString()` / `.toJSON()` — both serialise the `[[URL]]`
//!   internal slot to its WHATWG URL §4.5 ("URL serializer") form.
//!   Aliased to the same native fn (the spec defines `toJSON` as a
//!   pointer to the same algorithm).
//! - `URL.canParse(input, base?)` / `URL.parse(input, base?)`
//!   constructor statics (WHATWG URL §6.1, added 2023).

#![cfg(feature = "engine")]

mod accessors;
mod setters;

use super::super::shape::{self, PropertyAttrs};
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, VmError,
};
use super::super::{NativeFn, VmInner};

/// Per-`URL` instance state, stored out-of-band in
/// [`super::super::VmInner::url_states`].
///
/// `url::Url` is `Clone + Send` and ~80 bytes; cheap to mutate
/// in-place via `set_*` setters.  `search_params` is `Some` for
/// every URL produced by [`native_url_constructor`] or
/// [`alloc_url_instance`] — both eagerly allocate the linked
/// `URLSearchParams` to make `url.searchParams ===
/// url.searchParams` hold (WHATWG URL §6.1 identity invariant).
/// The `Option` exists only to express the brief window between
/// `url_states.insert` and the back-edge link installation
/// inside the same constructor body.
pub(crate) struct UrlState {
    /// Parsed URL — every accessor reads through this; setters
    /// mutate it in place via the `url::Url::set_*` API.
    pub(crate) url: url::Url,
    /// Linked `URLSearchParams` instance `ObjectId`.  `None` only
    /// during the brief constructor-body window before the
    /// back-edge installation completes (between `url_states
    /// .insert` and `state.search_params = Some(sp_id)`); every
    /// observable URL has `Some(_)` here.
    pub(crate) search_params: Option<ObjectId>,
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

impl VmInner {
    /// Allocate `URL.prototype`, install its method suite, and
    /// expose the `URL` constructor on `globals`.
    ///
    /// Called from `register_globals()` after
    /// `register_url_search_params_global()` (with
    /// `register_form_data_global()` between them, since neither
    /// FormData nor URL depend on each other).  The
    /// `URLSearchParams` prototype must exist first because the
    /// `URL` constructor eagerly allocates a `URLSearchParams`
    /// instance for each URL's `searchParams` IDL attribute.
    ///
    /// # Panics
    ///
    /// Panics if `object_prototype` is `None` — would mean
    /// `register_prototypes` was skipped or the wrong order.
    pub(in crate::vm) fn register_url_global(&mut self) {
        let object_proto = self
            .object_prototype
            .expect("register_url_global called before register_prototypes");

        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });
        self.install_url_members(proto_id);
        self.url_prototype = Some(proto_id);

        let ctor = self.create_constructable_function("URL", native_url_constructor);
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
        // `URL.canParse` / `URL.parse` static methods (WHATWG URL §6.1
        // "URL static methods", added 2023).  Both share the
        // input-coercion + parse algorithm via [`coerce_url_args`] +
        // [`try_parse_url`] — `canParse` returns the result truthiness,
        // `parse` returns a fresh `URL` instance or `null`.  Statics
        // install onto the constructor object directly (mirrors
        // `Number.isFinite` / `Array.isArray` precedent).
        let can_parse_sid = self.well_known.can_parse;
        self.install_native_method(
            ctor,
            can_parse_sid,
            native_url_can_parse_static,
            PropertyAttrs::METHOD,
        );
        let parse_sid = self.well_known.parse_url;
        self.install_native_method(
            ctor,
            parse_sid,
            native_url_parse_static,
            PropertyAttrs::METHOD,
        );

        let name_sid = self.well_known.url_global;
        self.globals.insert(name_sid, JsValue::Object(ctor));
    }

    fn install_url_members(&mut self, proto_id: ObjectId) {
        // IDL accessor properties (WHATWG URL §6.1).  10 read/write
        // accessors (each paired with a `set_*` mutator on
        // [`url::Url`]) plus the read-only `origin` and `searchParams`.
        // `WEBIDL_RO_ACCESSOR` is used for both shapes — the
        // constant's `writable` bit is meaningless for accessor
        // properties and the read / read-write distinction is
        // encoded by the presence of the setter (see
        // `shape::PropertyAttrs::WEBIDL_RO_ACCESSOR` doc).
        //
        // Getter / setter native fns live in [`accessors`] /
        // [`setters`] respectively (file-split-d, slot #9.5 R6 —
        // 1000-line convention).
        let wk = &self.well_known;
        let pairs: [(super::super::value::StringId, NativeFn, Option<NativeFn>); 11] = [
            (
                wk.href,
                accessors::native_url_get_href,
                Some(setters::native_url_set_href),
            ),
            (wk.origin, accessors::native_url_get_origin, None),
            (
                wk.protocol,
                accessors::native_url_get_protocol,
                Some(setters::native_url_set_protocol),
            ),
            (
                wk.username,
                accessors::native_url_get_username,
                Some(setters::native_url_set_username),
            ),
            (
                wk.password,
                accessors::native_url_get_password,
                Some(setters::native_url_set_password),
            ),
            (
                wk.host_attr,
                accessors::native_url_get_host,
                Some(setters::native_url_set_host),
            ),
            (
                wk.hostname,
                accessors::native_url_get_hostname,
                Some(setters::native_url_set_hostname),
            ),
            (
                wk.port_attr,
                accessors::native_url_get_port,
                Some(setters::native_url_set_port),
            ),
            (
                wk.pathname,
                accessors::native_url_get_pathname,
                Some(setters::native_url_set_pathname),
            ),
            (
                wk.search_attr,
                accessors::native_url_get_search,
                Some(setters::native_url_set_search),
            ),
            (
                wk.hash_attr,
                accessors::native_url_get_hash,
                Some(setters::native_url_set_hash),
            ),
        ];
        for (name_sid, getter, setter) in pairs {
            self.install_accessor_pair(
                proto_id,
                name_sid,
                getter,
                setter,
                PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }

        // `searchParams` IDL attribute — read-only, returns the
        // eagerly-allocated linked `URLSearchParams` instance.
        // Identity stability (`url.searchParams === url.searchParams`)
        // is by virtue of the constructor allocating once and storing
        // the same `ObjectId` on every read.
        self.install_accessor_pair(
            proto_id,
            self.well_known.search_params,
            accessors::native_url_get_search_params,
            None,
            PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );

        // `toString` / `toJSON` — both alias the same native fn so
        // identity comparison holds (matches V8 + boa precedent).
        // `toJSON` is a separate property even though the algorithm
        // is identical; WHATWG URL §6.1 specifies the IDL `toJSON`
        // explicitly.
        let to_string_sid = self.well_known.to_string_method;
        let to_string_id = self.install_native_method(
            proto_id,
            to_string_sid,
            native_url_to_string,
            PropertyAttrs::METHOD,
        );
        let to_json_sid = self.well_known.to_json;
        self.define_shaped_property(
            proto_id,
            PropertyKey::String(to_json_sid),
            PropertyValue::Data(JsValue::Object(to_string_id)),
            PropertyAttrs::METHOD,
        );
    }
}

// ---------------------------------------------------------------------------
// Constructor
// ---------------------------------------------------------------------------

/// `new URL(input, base?)` (WHATWG URL §6.1.1).
///
/// Coerces both arguments via `ToString` *first*, then runs the
/// URL parser.  Coercion before allocation matters for GC safety —
/// user-defined `toString` can trigger arbitrary code (and a GC
/// cycle), and the freshly allocated URL instance must be the
/// only thing on the work list when we call back into
/// `get_object_mut`.  `this` is rooted by the call frame so it
/// survives the coercion calls.
///
/// Eagerly allocates the linked `URLSearchParams` instance for
/// the IDL `searchParams` attribute: WHATWG URL §6.1 requires
/// `url.searchParams === url.searchParams` to hold across reads,
/// and a lazy-create path would either need a per-instance cache
/// or a shape-property install.  Eager creation avoids both —
/// the cost is one `alloc_object` per `new URL()` call.
fn native_url_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if !ctx.is_construct() {
        return Err(VmError::type_error(
            "Failed to construct 'URL': Please use the 'new' operator",
        ));
    }
    let JsValue::Object(id) = this else {
        unreachable!("constructor `this` is always an Object after `do_new`");
    };

    let input_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let base_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let parsed = parse_url_arguments(ctx, input_arg, base_arg)?;

    // Promote the pre-allocated Ordinary instance to URL — preserves
    // the `new.target.prototype` chain installed by `do_new` (PR5a2
    // R7.2/R7.3 lesson, mirrors `URLSearchParams` ctor).
    ctx.vm.get_object_mut(id).kind = ObjectKind::URL;
    let query_string = parsed.query().unwrap_or("").to_owned();
    ctx.vm.url_states.insert(
        id,
        UrlState {
            url: parsed,
            search_params: None,
        },
    );

    // Linked `URLSearchParams` allocation — `id` is already
    // promoted to `ObjectKind::URL` and reachable via `this` on the
    // call frame; the new sp_id is reachable via the
    // `url_states[id].search_params` link below.  Both halves of
    // the back-edge (`url_states.search_params` + `usp_parent_url`)
    // are written before any further allocation can occur, so a
    // GC during `alloc_url_search_params_from_query` cannot orphan
    // either side.
    let sp_id = super::url_search_params::alloc_url_search_params_from_query(
        ctx.vm,
        Some(query_string.as_str()),
    );
    if let Some(state) = ctx.vm.url_states.get_mut(&id) {
        state.search_params = Some(sp_id);
    }
    ctx.vm.usp_parent_url.insert(sp_id, id);

    Ok(JsValue::Object(id))
}

/// Parse `input` against optional `base`, mirroring boa-side
/// `globals/url.rs:38-49` exactly so the cutover is observably
/// identical.  When `base` is `undefined` / `null` the parser
/// requires `input` to be absolute; otherwise `base` must itself
/// parse and `input` is joined relative to it (WHATWG URL §4.4).
///
/// Used by the constructor.  `URL.canParse` / `URL.parse` use
/// [`coerce_url_args`] + [`try_parse_url`] directly so they don't
/// allocate the discarded TypeError message on parse failure.
pub(super) fn parse_url_arguments(
    ctx: &mut NativeContext<'_>,
    input: JsValue,
    base: JsValue,
) -> Result<url::Url, VmError> {
    let (input_str, base_str) = coerce_url_args(ctx, input, base)?;
    if let Some(base_ref) = base_str.as_deref() {
        let base_url = url::Url::parse(base_ref)
            .map_err(|_| VmError::type_error(format!("URL: invalid base URL: {base_ref}")))?;
        base_url
            .join(&input_str)
            .map_err(|_| VmError::type_error(format!("URL: invalid URL: {input_str}")))
    } else {
        url::Url::parse(&input_str)
            .map_err(|_| VmError::type_error(format!("URL: invalid URL: {input_str}")))
    }
}

/// ToString-coerce the constructor / static-method arguments,
/// returning the input + optional base as owned UTF-8 strings.
/// Factored so [`parse_url_arguments`] (throwing) and the
/// `URL.canParse` / `URL.parse` statics (non-throwing on parse
/// failure) share the IDL-conversion half without re-throwing the
/// formatted-message half.  ToString errors still propagate — they
/// happen at the WebIDL boundary regardless of what the URL parser
/// would do.
pub(super) fn coerce_url_args(
    ctx: &mut NativeContext<'_>,
    input: JsValue,
    base: JsValue,
) -> Result<(String, Option<String>), VmError> {
    let input_sid = super::super::coerce::to_string(ctx.vm, input)?;
    let input_str = ctx.vm.strings.get_utf8(input_sid);
    let base_str: Option<String> = match base {
        JsValue::Undefined | JsValue::Null => None,
        other => {
            let sid = super::super::coerce::to_string(ctx.vm, other)?;
            Some(ctx.vm.strings.get_utf8(sid))
        }
    };
    Ok((input_str, base_str))
}

/// Try-parse `input` against optional `base` without allocating an
/// error message on failure.  `Some(Url)` on success, `None` on
/// parse failure.  Used by the `URL.canParse` / `URL.parse`
/// statics — both discard the failure cause (`canParse` returns a
/// boolean, `parse` returns `null`), so the constructor's
/// `format!("URL: invalid …")` cost would be wasted there.
pub(super) fn try_parse_url(input: &str, base: Option<&str>) -> Option<url::Url> {
    match base {
        Some(b) => url::Url::parse(b).ok().and_then(|bu| bu.join(input).ok()),
        None => url::Url::parse(input).ok(),
    }
}

// ---------------------------------------------------------------------------
// Statics
// ---------------------------------------------------------------------------

/// `URL.canParse(input, base?)` (WHATWG URL §6.1).  Equivalent to
/// `try { new URL(input, base); true } catch { false }` — but
/// avoids both the `URL` allocation on success and the discarded
/// TypeError-message allocation on parse failure (composes
/// `coerce_url_args` + `try_parse_url` directly so no
/// `format!("URL: invalid …")` runs on the failure path).
fn native_url_can_parse_static(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let input_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let base_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let (input_str, base_str) = coerce_url_args(ctx, input_arg, base_arg)?;
    Ok(JsValue::Boolean(
        try_parse_url(&input_str, base_str.as_deref()).is_some(),
    ))
}

/// `URL.parse(input, base?)` (WHATWG URL §6.1).  Returns a fresh
/// `URL` instance on success, `null` on parse failure.  Same perf
/// shape as `URL.canParse`: skips the discarded TypeError
/// allocation on failure.
fn native_url_parse_static(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let input_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let base_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let (input_str, base_str) = coerce_url_args(ctx, input_arg, base_arg)?;
    let Some(parsed) = try_parse_url(&input_str, base_str.as_deref()) else {
        return Ok(JsValue::Null);
    };
    let id = alloc_url_instance(ctx.vm, parsed);
    Ok(JsValue::Object(id))
}

/// Allocate a fresh `URL` wrapper + its eagerly-linked
/// `URLSearchParams`, returning the URL's `ObjectId`.  Used by the
/// `URL.parse` static (and any future caller that needs a URL
/// without going through the JS-level constructor).
///
/// GC contract: between `alloc_object` for the URL and the second
/// `alloc_object` (inside
/// [`super::url_search_params::alloc_url_search_params_from_query`]),
/// the new URL has no caller-side root — only the
/// [`super::super::VmInner::push_temp_root`] guard pushed below
/// keeps it pinned through the inner allocation's potential GC
/// cycle.  Without the guard the URL could be swept and the
/// `url_states[id]` entry pruned before we attach the linked
/// `searchParams`.
pub(super) fn alloc_url_instance(vm: &mut VmInner, parsed: url::Url) -> ObjectId {
    let proto = vm.url_prototype;
    let query_string = parsed.query().unwrap_or("").to_owned();
    let id = vm.alloc_object(Object {
        kind: ObjectKind::URL,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });
    vm.url_states.insert(
        id,
        UrlState {
            url: parsed,
            search_params: None,
        },
    );
    let mut g = vm.push_temp_root(JsValue::Object(id));
    let sp_id = super::url_search_params::alloc_url_search_params_from_query(
        &mut g,
        Some(query_string.as_str()),
    );
    if let Some(state) = g.url_states.get_mut(&id) {
        state.search_params = Some(sp_id);
    }
    g.usp_parent_url.insert(sp_id, id);
    drop(g);
    id
}

// ---------------------------------------------------------------------------
// Methods
// ---------------------------------------------------------------------------

/// `URL.prototype.toString()` and `URL.prototype.toJSON()`
/// (WHATWG URL §6.1).  Both algorithms are "return the
/// serialization of `[[URL]]`" — `url::Url::as_str()` returns the
/// already-canonical representation maintained by every setter.
fn native_url_to_string(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_url_this(ctx, this, "toString")?;
    url_component(ctx, id, |u| u.as_str().to_string())
}

// ---------------------------------------------------------------------------
// Cross-module helpers
// ---------------------------------------------------------------------------

/// `URLSearchParams` mutation hook — re-serialise the entry list
/// and stamp it back onto the parent URL's query.  Called at the
/// tail of every `URLSearchParams` mutator (`append` / `delete` /
/// `set` / `sort`) by [`super::url_search_params`].  No-op for
/// standalone `URLSearchParams` instances (those with no
/// `usp_parent_url` entry).
pub(crate) fn rewrite_url_query_from_search_params(vm: &mut VmInner, sp_id: ObjectId) {
    let Some(&url_id) = vm.usp_parent_url.get(&sp_id) else {
        return;
    };
    let serialized = super::url_search_params::serialize_for_body(vm, sp_id);
    if let Some(url_state) = vm.url_states.get_mut(&url_id) {
        if serialized.is_empty() {
            url_state.url.set_query(None);
        } else {
            url_state.url.set_query(Some(&serialized));
        }
    }
}

/// Re-parse the URL's query and refresh the linked
/// `URLSearchParams` entry list so `url.search = "x=1"; url
/// .searchParams.get("x") === "1"` and `url.href = "…"; url
/// .searchParams` reflects the new query.  No-op when the URL
/// has no linked `URLSearchParams`.
///
/// Field-disjoint borrows (`url_states` immutable, then
/// `url_search_params_states` mutable) are sequenced through a
/// snapshot of `(sp_id, query_string)` taken under the immutable
/// borrow so each side touches the VM in turn rather than
/// simultaneously.
pub(super) fn rebuild_linked_search_params(vm: &mut VmInner, url_id: ObjectId) {
    let (sp_id, query) = match vm.url_states.get(&url_id) {
        Some(state) => match state.search_params {
            Some(sp_id) => (sp_id, state.url.query().unwrap_or("").to_owned()),
            None => return,
        },
        None => return,
    };
    let pairs: Vec<(super::super::value::StringId, super::super::value::StringId)> =
        url::form_urlencoded::parse(query.as_bytes())
            .map(|(k, v)| (vm.strings.intern(&k), vm.strings.intern(&v)))
            .collect();
    if let Some(sp_state) = vm.url_search_params_states.get_mut(&sp_id) {
        *sp_state = pairs;
    }
}

/// Shared body for every URL-component getter: borrow the URL's
/// state, run `extract` to assemble the output string, intern the
/// result.  Mirror of [`super::location::url_component`] — the
/// `extract` closure produces an owned `String` so the borrow on
/// `url_states` ends before `intern` takes `&mut self.strings`.
///
/// Throws `TypeError` (`missing internal slot`) when `url_states`
/// has no entry despite the brand check passing.  Such a state
/// would indicate a GC-rooting bug or a manually-promoted
/// `ObjectKind::URL` instance — fail loudly rather than silently
/// returning `""` (matches the `searchParams` getter's defensive
/// posture).
pub(super) fn url_component(
    ctx: &mut NativeContext<'_>,
    id: ObjectId,
    extract: impl FnOnce(&url::Url) -> String,
) -> Result<JsValue, VmError> {
    let Some(s) = ctx.vm.url_states.get(&id).map(|state| extract(&state.url)) else {
        return Err(VmError::type_error("URL: missing internal slot"));
    };
    let sid = ctx.vm.strings.intern(&s);
    Ok(JsValue::String(sid))
}

/// Brand check — every `URL.prototype.*` native rejects receivers
/// whose `ObjectKind` is not `URL` with a `TypeError`.  Mirror of
/// [`super::url_search_params::require_usp_this`].
pub(super) fn require_url_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<ObjectId, VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "URL.prototype.{method} called on non-URL"
        )));
    };
    if matches!(ctx.vm.get_object(id).kind, ObjectKind::URL) {
        Ok(id)
    } else {
        Err(VmError::type_error(format!(
            "URL.prototype.{method} called on non-URL"
        )))
    }
}
