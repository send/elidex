//! `Request` / `Response` interfaces (WHATWG Fetch §5.3 / §5.5).
//!
//! Both variants are payload-free ([`super::super::value::ObjectKind::Request`]
//! / [`super::super::value::ObjectKind::Response`]); per-instance state
//! lives in out-of-band side tables ([`RequestState`] / [`ResponseState`])
//! keyed by the instance's own `ObjectId`.  Body bytes — when present
//! — share a single `VmInner::body_data` map across both variants so
//! `clone()` clones the body `Vec<u8>` via the shared `body_data`
//! map.
//!
//! ## Prototype chain
//!
//! ```text
//! Request / Response instance
//!   → Request.prototype / Response.prototype  (this module)
//!     → Object.prototype
//! ```
//!
//! No EventTarget / Node ancestry — WebIDL lists Fetch objects as
//! interfaces rooted at `Object`, not `EventTarget`.
//!
//! ## IDL attribute authority
//!
//! Readonly IDL attributes (`url` / `method` / `status` / `statusText` /
//! `type` / `ok` / `redirected` / `headers` / `body` / `bodyUsed`)
//! **read from the out-of-band state**, not from own-data properties.
//! A user `delete resp.url` mutation does not affect `resp.url` reads
//! that hit the `Response.prototype` accessor — the accessor consults
//! `VmInner::response_states`, which is the authoritative source.
//! This matches PR5a2 R7.1 (internal-slot authoritative) and the
//! browser behaviour for attribute reflection.
//!
//! ## Module layout
//!
//! Split across this directory to keep each file under the project's
//! 1000-line convention:
//!
//! - [`mod@request_ctor`] — `new Request(input, init?)` constructor +
//!   the three private helpers it owns.
//! - [`mod@response_ctor`] — `new Response(body?, init?)` constructor +
//!   shared `build_response_instance` / `parse_response_init`.
//! - [`mod@response_statics`] — `Response.error()` / `.redirect()` /
//!   `.json()` static factories.
//! - [`mod@accessors`] — `Request.prototype` / `Response.prototype`
//!   IDL getters + the `.clone()` methods.
//! - [`mod@body_init`] — `BodyInit` byte extraction + default
//!   Content-Type derivation, shared between the two ctors and the
//!   `fetch()` host's URL-input init.body parsing path.
//!
//! Enums, side-table state structs, registration entry points, the
//! init-dict enum-string parsers, and the small headers helpers
//! shared across submodules live in this file.
//!
//! ## Scope
//!
//! - `new Request(input, init?)` — input = URL string or Request
//!   clone.  `init.body` accepts string / `ArrayBuffer` /
//!   `TypedArray` / `Blob` (landed with the PR5a-fetch C3 Body-
//!   mixin tranche).
//! - `new Response(body?, init?)` — same body types.
//! - All IDL getters listed above.
//! - `request.clone()` / `response.clone()` — copy body Vec via
//!   `body_data` map.
//! - `Response.error()` / `Response.redirect(url, status)` /
//!   `Response.json(data, init?)` static factories.
//! - `.text()` / `.json()` / `.arrayBuffer()` / `.blob()` read
//!   methods live in [`super::body_mixin`] and share the
//!   [`VmInner::body_data`] side table with this module.
//! - `.body` getter always returns `null` (Phase 2 non-streaming;
//!   a later PR5-streams tranche of the M4-12 boa → VM cutover
//!   supplies the `ReadableStream` replacement).
//! - `init.signal` is honoured by the `fetch()` path (pre-flight
//!   brand / aborted check — see [`super::fetch`]); the ctor
//!   itself silently accepts and stores the option.
//!
//! ## Deferred
//!
//! - `FormData` / `URLSearchParams` body init — no other surface
//!   consumes them yet.
//! - Strict forbidden-header / forbidden-method enforcement.
//!   Phase 2 is permissive; CORS-mode enforcement lands with the
//!   PR5-async-fetch refactor that threads through an Origin.

#![cfg(feature = "engine")]

mod accessors;
mod body_init;
mod request_ctor;
mod response_ctor;
mod response_statics;

use url::Url;

use super::super::shape::{self, PropertyAttrs};
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, StringId, VmError,
};
use super::super::{NativeFn, VmInner};

use accessors::{
    native_request_clone, native_request_get_body, native_request_get_body_used,
    native_request_get_cache, native_request_get_credentials, native_request_get_headers,
    native_request_get_method, native_request_get_mode, native_request_get_redirect,
    native_request_get_url, native_response_clone, native_response_get_body,
    native_response_get_body_used, native_response_get_headers, native_response_get_ok,
    native_response_get_redirected, native_response_get_status, native_response_get_status_text,
    native_response_get_type, native_response_get_url,
};
use request_ctor::native_request_constructor;
use response_ctor::native_response_constructor;
use response_statics::{
    native_response_static_error, native_response_static_json, native_response_static_redirect,
};

// Re-export `extract_body_bytes` / `content_type_for_body` so the
// `fetch()` host (and url_search_params doc-links) keep working
// at `super::request_response::*` after the split.
pub(in crate::vm::host) use body_init::{content_type_for_body, extract_body_bytes};

// ---------------------------------------------------------------------------
// Enums (WHATWG Fetch §5.3 / §5.5)
// ---------------------------------------------------------------------------

/// `RequestRedirect` (WHATWG §5.3).  Honoured by the broker
/// `redirect::follow_redirects` loop: `Follow` auto-follows up
/// to `max_redirects`; `Error` rejects with `NetError` on the
/// first 3xx; `Manual` returns the 3xx as-is so the JS path can
/// surface an `OpaqueRedirect`-typed Response.
///
/// Re-exported from `elidex-net` so the broker `Request` field
/// and the JS-side state share a single type without round-trip
/// conversion.
pub(crate) use elidex_net::RedirectMode;

/// `RequestMode` (WHATWG §5.3).  `Cors` / `NoCors` / `SameOrigin`
/// are reachable from the Request constructor and `fetch()`;
/// `Navigate` is internal to navigation requests and rejected
/// with `TypeError` when set on `init` (spec §5.3 step 23).
///
/// Re-exported from `elidex-net` so the broker `Request` field
/// and the JS-side state share a single type without round-trip
/// conversion (mirrors the [`RedirectMode`] / [`RequestCredentials`]
/// re-export pattern).
pub(crate) use elidex_net::RequestMode;

/// `RequestCredentials` (WHATWG §5.3).  Threaded through the
/// broker so `Omit` suppresses the cookie attach, `SameOrigin`
/// (default) attaches only on same-origin, and `Include` always
/// attaches.  Re-exported from `elidex-net` (where the type is
/// named `CredentialsMode`) so the JS-side state and the broker
/// `Request` field share a single value without conversion.
pub(crate) use elidex_net::CredentialsMode as RequestCredentials;

/// `RequestCache` (WHATWG §5.3).  HTTP cache modes; PR5-cors
/// injects the spec-prescribed `Cache-Control` / `Pragma`
/// headers but does not implement an on-disk HTTP cache, so
/// `ForceCache` / `OnlyIfCached` round-trip without affecting
/// network behaviour (documented gap — see PR5-cors plan).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RequestCache {
    Default,
    NoStore,
    Reload,
    NoCache,
    ForceCache,
    OnlyIfCached,
}

/// `ResponseType` (WHATWG §5.5).  Surfaced via the `.type` IDL
/// attribute.  All six variants (`Basic` / `Cors` / `Default` /
/// `Error` / `Opaque` / `OpaqueRedirect`) are constructible
/// today: `Basic` / `Cors` / `Opaque` / `OpaqueRedirect` from
/// the fetch settlement path's CORS classifier
/// ([`super::cors::classify_response_type`]); `Default` from
/// `new Response(...)`; `Error` from `Response.error()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResponseType {
    Basic,
    Cors,
    Default,
    Error,
    Opaque,
    OpaqueRedirect,
}

// ---------------------------------------------------------------------------
// `init.*` enum-value parsing (WHATWG WebIDL §3.10.7 dictionary
// member conversion: invalid enum strings throw TypeError; an
// absent member falls through to the caller-provided default).
// ---------------------------------------------------------------------------

/// Parse `init.mode`'s string into a [`RequestMode`].  Returns
/// `Ok(None)` when the input is `undefined` (member absent).
/// `null` is rejected: WebIDL enum members are not nullable so
/// `null` ToString-coerces to `"null"`, which is not a valid
/// enum value (throws).  `"navigate"` is also rejected from the
/// JS-facing init dictionary per WHATWG Fetch §5.3 step 23
/// (navigate is reserved for navigation requests built
/// internally).
pub(crate) fn parse_request_mode(
    ctx: &mut NativeContext<'_>,
    value: JsValue,
    operation: &str,
) -> Result<Option<RequestMode>, VmError> {
    if matches!(value, JsValue::Undefined) {
        return Ok(None);
    }
    let sid = super::super::coerce::to_string(ctx.vm, value)?;
    let raw = ctx.vm.strings.get_utf8(sid);
    Ok(Some(match raw.as_str() {
        "cors" => RequestMode::Cors,
        "no-cors" => RequestMode::NoCors,
        "same-origin" => RequestMode::SameOrigin,
        "navigate" => {
            return Err(VmError::type_error(format!(
                "{operation}: 'navigate' is not a valid request mode"
            )));
        }
        other => {
            return Err(VmError::type_error(format!(
                "{operation}: '{other}' is not a valid request mode"
            )));
        }
    }))
}

/// Parse `init.credentials` into a [`RequestCredentials`].  `Ok(None)`
/// for `undefined`; invalid strings throw TypeError per WebIDL.
pub(crate) fn parse_request_credentials(
    ctx: &mut NativeContext<'_>,
    value: JsValue,
    operation: &str,
) -> Result<Option<RequestCredentials>, VmError> {
    if matches!(value, JsValue::Undefined) {
        return Ok(None);
    }
    let sid = super::super::coerce::to_string(ctx.vm, value)?;
    let raw = ctx.vm.strings.get_utf8(sid);
    Ok(Some(match raw.as_str() {
        "omit" => RequestCredentials::Omit,
        "same-origin" => RequestCredentials::SameOrigin,
        "include" => RequestCredentials::Include,
        other => {
            return Err(VmError::type_error(format!(
                "{operation}: '{other}' is not a valid credentials mode"
            )));
        }
    }))
}

/// Parse `init.redirect` into a [`RedirectMode`].  `Ok(None)` for
/// `undefined`; invalid strings throw TypeError per WebIDL.
pub(crate) fn parse_request_redirect(
    ctx: &mut NativeContext<'_>,
    value: JsValue,
    operation: &str,
) -> Result<Option<RedirectMode>, VmError> {
    if matches!(value, JsValue::Undefined) {
        return Ok(None);
    }
    let sid = super::super::coerce::to_string(ctx.vm, value)?;
    let raw = ctx.vm.strings.get_utf8(sid);
    Ok(Some(match raw.as_str() {
        "follow" => RedirectMode::Follow,
        "error" => RedirectMode::Error,
        "manual" => RedirectMode::Manual,
        other => {
            return Err(VmError::type_error(format!(
                "{operation}: '{other}' is not a valid redirect mode"
            )));
        }
    }))
}

/// Parse `init.cache` into a [`RequestCache`].  `Ok(None)` for
/// `undefined`; invalid strings throw TypeError per WebIDL.
pub(crate) fn parse_request_cache(
    ctx: &mut NativeContext<'_>,
    value: JsValue,
    operation: &str,
) -> Result<Option<RequestCache>, VmError> {
    if matches!(value, JsValue::Undefined) {
        return Ok(None);
    }
    let sid = super::super::coerce::to_string(ctx.vm, value)?;
    let raw = ctx.vm.strings.get_utf8(sid);
    Ok(Some(match raw.as_str() {
        "default" => RequestCache::Default,
        "no-store" => RequestCache::NoStore,
        "reload" => RequestCache::Reload,
        "no-cache" => RequestCache::NoCache,
        "force-cache" => RequestCache::ForceCache,
        "only-if-cached" => RequestCache::OnlyIfCached,
        other => {
            return Err(VmError::type_error(format!(
                "{operation}: '{other}' is not a valid cache mode"
            )));
        }
    }))
}

// ---------------------------------------------------------------------------
// State structs
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub(crate) struct RequestState {
    /// Uppercased normalised method per §5.3 step 23 (unknown
    /// methods preserved verbatim).
    pub(crate) method_sid: StringId,
    /// Absolute URL serialisation (Url::to_string has already
    /// normalised the form).
    pub(crate) url_sid: StringId,
    /// Paired Headers instance (guard = `request` in full spec;
    /// Phase 2 uses `none` with loose validation).
    pub(crate) headers_id: ObjectId,
    pub(crate) redirect: RedirectMode,
    pub(crate) mode: RequestMode,
    pub(crate) credentials: RequestCredentials,
    pub(crate) cache: RequestCache,
}

#[derive(Debug)]
pub(crate) struct ResponseState {
    pub(crate) status: u16,
    pub(crate) status_text_sid: StringId,
    /// Absolute URL of the response; empty for synthetic
    /// `new Response(...)` / `Response.error()` responses.
    pub(crate) url_sid: StringId,
    /// Paired Headers instance (guard = `response` in full spec;
    /// Phase 2 uses `Immutable` so user `.headers.append` throws).
    pub(crate) headers_id: ObjectId,
    pub(crate) response_type: ResponseType,
    pub(crate) redirected: bool,
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

impl VmInner {
    /// Allocate `Request.prototype` + install accessors + expose
    /// the `Request` constructor on `globals`.  Runs during
    /// `register_globals()` after `register_prototypes` (for
    /// `object_prototype`) and after `register_headers_global`
    /// (so `headers_prototype` is live when Request allocates its
    /// companion Headers instance).
    pub(in crate::vm) fn register_request_global(&mut self) {
        let object_proto = self
            .object_prototype
            .expect("register_request_global called before register_prototypes");
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });
        self.install_request_members(proto_id);
        self.request_prototype = Some(proto_id);

        let ctor = self.create_constructable_function("Request", native_request_constructor);
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
        let name_sid = self.well_known.request;
        self.globals.insert(name_sid, JsValue::Object(ctor));
    }

    fn install_request_members(&mut self, proto_id: ObjectId) {
        // Accessors (WebIDL RO attrs).  Capture the StringIds up
        // front so the subsequent `&mut self` call to
        // `install_ro_accessor_list` doesn't conflict with a held
        // `&self.well_known` borrow (E0502).
        let accessors: [(StringId, NativeFn); 9] = [
            (
                self.well_known.method,
                native_request_get_method as NativeFn,
            ),
            (self.well_known.url, native_request_get_url as NativeFn),
            (
                self.well_known.headers,
                native_request_get_headers as NativeFn,
            ),
            (self.well_known.body, native_request_get_body as NativeFn),
            (
                self.well_known.body_used,
                native_request_get_body_used as NativeFn,
            ),
            (
                self.well_known.redirect,
                native_request_get_redirect as NativeFn,
            ),
            (self.well_known.mode, native_request_get_mode as NativeFn),
            (
                self.well_known.credentials,
                native_request_get_credentials as NativeFn,
            ),
            (self.well_known.cache, native_request_get_cache as NativeFn),
        ];
        self.install_ro_accessor_list(proto_id, &accessors);
        self.install_native_method(
            proto_id,
            self.well_known.clone,
            native_request_clone,
            PropertyAttrs::METHOD,
        );
    }

    /// Allocate `Response.prototype`, install accessors + `clone`,
    /// attach the three static factories (`error` / `redirect` /
    /// `json`), and expose the `Response` constructor on `globals`.
    pub(in crate::vm) fn register_response_global(&mut self) {
        let object_proto = self
            .object_prototype
            .expect("register_response_global called before register_prototypes");
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });
        self.install_response_members(proto_id);
        self.response_prototype = Some(proto_id);

        let ctor = self.create_constructable_function("Response", native_response_constructor);
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
        // Static factories on the ctor object itself.  StringIds
        // snapshotted so each loop body can take `&mut self`.
        let statics: [(StringId, NativeFn); 3] = [
            (
                self.well_known.error,
                native_response_static_error as NativeFn,
            ),
            (
                self.well_known.redirect,
                native_response_static_redirect as NativeFn,
            ),
            (
                self.well_known.json,
                native_response_static_json as NativeFn,
            ),
        ];
        for (name_sid, func) in statics {
            self.install_native_method(ctor, name_sid, func, PropertyAttrs::METHOD);
        }
        let name_sid = self.well_known.response;
        self.globals.insert(name_sid, JsValue::Object(ctor));
    }

    fn install_response_members(&mut self, proto_id: ObjectId) {
        let accessors: [(StringId, NativeFn); 9] = [
            (
                self.well_known.status,
                native_response_get_status as NativeFn,
            ),
            (self.well_known.ok, native_response_get_ok as NativeFn),
            (
                self.well_known.status_text,
                native_response_get_status_text as NativeFn,
            ),
            (self.well_known.url, native_response_get_url as NativeFn),
            (
                self.well_known.event_type,
                native_response_get_type as NativeFn,
            ),
            (
                self.well_known.headers,
                native_response_get_headers as NativeFn,
            ),
            (self.well_known.body, native_response_get_body as NativeFn),
            (
                self.well_known.body_used,
                native_response_get_body_used as NativeFn,
            ),
            (
                self.well_known.redirected,
                native_response_get_redirected as NativeFn,
            ),
        ];
        self.install_ro_accessor_list(proto_id, &accessors);
        self.install_native_method(
            proto_id,
            self.well_known.clone,
            native_response_clone,
            PropertyAttrs::METHOD,
        );
    }

    /// Install a `[(name_sid, getter)]` batch as WEBIDL_RO
    /// accessors on `proto_id`.  Setters default to `None` (every
    /// readonly WebIDL attribute).  The getter's WebIDL display
    /// name is derived as `"get {name}"` to match browsers.
    fn install_ro_accessor_list(&mut self, proto_id: ObjectId, list: &[(StringId, NativeFn)]) {
        for &(name_sid, getter) in list {
            self.install_accessor_pair(
                proto_id,
                name_sid,
                getter,
                None,
                PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Shared HTTP / URL helpers
// ---------------------------------------------------------------------------

/// Apply WHATWG §5.3 step 24 method canonicalisation + §4.6
/// forbidden-method filter.  Returns the canonical form:
/// - the uppercase token for the seven byte-case-insensitive
///   canonical methods (`GET` / `HEAD` / `POST` / `PUT` /
///   `DELETE` / `OPTIONS` / `PATCH`),
/// - the original case otherwise (unknown extensions like
///   `CustomOperation` or `MKCOL` pass through verbatim —
///   matches Chromium / Firefox).
///
/// The forbidden-method check runs on the uppercased token so
/// `connect` / `Trace` / `track` all reject case-insensitively.
/// Error messages are prefixed with `error_prefix` (e.g.
/// `"Failed to construct 'Request'"` or
/// `"Failed to execute 'fetch'"`) so the caller's reporting
/// context is preserved.
pub(in crate::vm::host) fn validate_http_method(
    raw: &str,
    error_prefix: &str,
) -> Result<String, VmError> {
    let upper = raw.to_ascii_uppercase();
    if matches!(upper.as_str(), "CONNECT" | "TRACE" | "TRACK") {
        return Err(VmError::type_error(format!(
            "{error_prefix}: '{raw}' HTTP method is unsupported."
        )));
    }
    if matches!(
        upper.as_str(),
        "GET" | "HEAD" | "POST" | "PUT" | "DELETE" | "OPTIONS" | "PATCH"
    ) {
        Ok(upper)
    } else {
        // Non-canonical extension — preserve the original casing
        // (spec §5.3 step 24 only canonicalises the seven known
        // methods; unknown tokens bypass the uppercase step).
        Ok(raw.to_string())
    }
}

/// Parse `url` (relative or absolute) against the current
/// `navigation.current_url` base.  Failure → `TypeError`
/// (WHATWG Fetch §5.3 step 11 requires a URL parse that may yield
/// `failure`; the ctor then throws `TypeError`).
///
/// `pub(super)` so the `fetch()` host (`vm/host/fetch.rs`) can
/// reuse the same relative-resolution path.  `about:blank` as a
/// base makes `Url::join` fail for relative input — the caller
/// surfaces this as a `TypeError` per WHATWG Fetch §5.1.
pub(super) fn parse_url(vm: &VmInner, input: &str) -> Result<Url, VmError> {
    if let Ok(abs) = Url::parse(input) {
        return Ok(abs);
    }
    match vm.navigation.current_url.join(input) {
        Ok(u) => Ok(u),
        Err(_) => Err(VmError::type_error(format!(
            "Failed to construct 'Request': Invalid URL '{input}'"
        ))),
    }
}

/// Populate `headers_id` from a `HeadersInit`-shaped value
/// (Headers / Record / Array-of-pairs / null / undefined).
/// Delegates to [`super::headers::fill_headers_from_init`] so the
/// WHATWG §5.2 "fill a Headers object" algorithm lives in one
/// place and name / value validation does not drift.
/// `error_prefix` is the caller's reporting context (e.g.
/// `"Failed to construct 'Request'"`) threaded into any validation
/// error so users see the surface that triggered the failure.
pub(super) fn fill_headers_like(
    ctx: &mut NativeContext<'_>,
    target_headers_id: ObjectId,
    init: JsValue,
    error_prefix: &str,
) -> Result<(), VmError> {
    super::headers::fill_headers_from_init(ctx, target_headers_id, init, error_prefix)
}

/// Splice every entry from `src_id` into `dst_id`.  Entries are
/// already validated on the source side, so no re-check here.
pub(super) fn copy_headers_entries(
    ctx: &mut NativeContext<'_>,
    src_id: ObjectId,
    dst_id: ObjectId,
) {
    if src_id == dst_id {
        return;
    }
    let entries = ctx
        .vm
        .headers_states
        .get(&src_id)
        .map(|s| s.list.clone())
        .unwrap_or_default();
    if let Some(state) = ctx.vm.headers_states.get_mut(&dst_id) {
        state.list.extend(entries);
    }
}

/// Set `Content-Type: <ct_sid>` on `headers_id` only when the user
/// did not already provide a `content-type` entry.  Mirrors the
/// §5 "extract a body" default-content-type logic.
pub(super) fn ensure_content_type(
    ctx: &mut NativeContext<'_>,
    headers_id: ObjectId,
    default_ct_sid: StringId,
) {
    let ct_name_sid = ctx.vm.well_known.content_type;
    if let Some(state) = ctx.vm.headers_states.get_mut(&headers_id) {
        if !state.list.iter().any(|(n, _)| *n == ct_name_sid) {
            state.list.push((ct_name_sid, default_ct_sid));
        }
    }
}
