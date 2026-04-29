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

use url::Url;

use super::super::shape::{self, PropertyAttrs};
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, StringId, VmError,
};
use super::super::{NativeFn, VmInner};
use super::headers::HeadersGuard;
use super::request_ctor::native_request_constructor;
use super::request_response_accessors::{
    native_request_clone, native_request_get_body, native_request_get_body_used,
    native_request_get_cache, native_request_get_credentials, native_request_get_headers,
    native_request_get_method, native_request_get_mode, native_request_get_redirect,
    native_request_get_url, native_response_clone, native_response_get_body,
    native_response_get_body_used, native_response_get_headers, native_response_get_ok,
    native_response_get_redirected, native_response_get_status, native_response_get_status_text,
    native_response_get_type, native_response_get_url,
};

// ---------------------------------------------------------------------------
// Enums (WHATWG Fetch §5.3 / §5.5)
// ---------------------------------------------------------------------------

/// `RequestRedirect` (WHATWG §5.3).  Honoured by the broker
/// `redirect::follow_redirects` loop: `Follow` auto-follows up
/// to `max_redirects`; `Error` rejects with `NetError` on the
/// first 3xx; `Manual` returns the 3xx as-is so the JS path can
/// surface an `OpaqueRedirect`-typed Response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RedirectMode {
    Follow,
    Error,
    Manual,
}

/// `RequestMode` (WHATWG §5.3).  `Cors` / `NoCors` / `SameOrigin`
/// are reachable from the Request constructor and `fetch()`;
/// `Navigate` is internal to navigation requests and rejected
/// with `TypeError` when set on `init` (spec §5.3 step 23).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RequestMode {
    Cors,
    NoCors,
    SameOrigin,
    /// Reserved for the navigation pipeline's internal Request
    /// construction; never reached from JS-facing init parsing
    /// (the parser throws TypeError on the string).
    #[allow(dead_code)]
    Navigate,
}

/// `RequestCredentials` (WHATWG §5.3).  Threaded through the
/// broker so `Omit` suppresses the cookie attach, `SameOrigin`
/// (default) attaches only on same-origin, and `Include` always
/// attaches.  See `attach_cookies_for_credentials_mode` in the
/// elidex-net crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RequestCredentials {
    Omit,
    SameOrigin,
    Include,
}

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
/// attribute.  `Basic` / `Default` / `Error` / `OpaqueRedirect`
/// are constructible today from the broker fetch path and the
/// `Response.error()` factory; `Cors` and `Opaque` arms are
/// reserved for the CORS classifier landing in PR5-cors Stage 4.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
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

// Request constructor tuple aliases (`RequestInputParts` /
// `RequestInitParts`) live in [`super::request_ctor`] alongside
// their sole users.

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
pub(super) fn validate_http_method(raw: &str, error_prefix: &str) -> Result<String, VmError> {
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

/// Coerce a body init value into raw UTF-8 bytes plus, when
/// applicable, an override Content-Type that supersedes the
/// generic [`content_type_for_body`] mapping (the encoder-derived
/// boundary for `multipart/form-data` is only known after the body
/// is serialised, so it has to be returned together with the
/// bytes).  Accepts `String` / `ArrayBuffer` / `Blob` /
/// `URLSearchParams` / `FormData` / `BufferSource` views (per
/// WHATWG §5 "extract a body"); any other non-null / non-undefined
/// value is `ToString`-coerced, matching browsers' forgiving
/// `new Request(url, {body: 42})` → `"42"` behaviour.
///
/// `ReadableStream` lands with the PR5-streams tranche.
///
/// `pub(super)` so the `fetch()` host (`vm/host/fetch.rs`) can
/// reuse the exact same coercion path for `init.body` without
/// duplicating the ArrayBuffer / Blob extraction branches — the
/// two code paths would otherwise drift.
pub(super) fn extract_body_bytes(
    ctx: &mut NativeContext<'_>,
    val: JsValue,
) -> Result<Option<(Vec<u8>, Option<StringId>)>, VmError> {
    match val {
        JsValue::Undefined | JsValue::Null => Ok(None),
        JsValue::String(sid) => {
            let raw = ctx.vm.strings.get_utf8(sid);
            Ok(Some((raw.into_bytes(), None)))
        }
        JsValue::Object(obj_id) => match ctx.vm.get_object(obj_id).kind {
            ObjectKind::ArrayBuffer => Ok(Some((
                super::array_buffer::array_buffer_bytes(ctx.vm, obj_id),
                None,
            ))),
            ObjectKind::Blob => {
                // `BlobData.bytes` is the source of truth as
                // `Arc<[u8]>` (per-spec immutable).  Snapshot the
                // bytes into a fresh Vec at the pool boundary so
                // the new body owns its bytes independently.
                Ok(Some((
                    super::blob::blob_bytes(ctx.vm, obj_id).to_vec(),
                    None,
                )))
            }
            // TypedArray / DataView as BufferSource (WHATWG Fetch
            // §5 — BodyInit union accepts any BufferSource).
            // Extract the view's byte range from the underlying
            // ArrayBuffer.
            ObjectKind::TypedArray {
                buffer_id,
                byte_offset,
                byte_length,
                ..
            }
            | ObjectKind::DataView {
                buffer_id,
                byte_offset,
                byte_length,
            } => Ok(Some((
                super::array_buffer::array_buffer_view_bytes(
                    ctx.vm,
                    buffer_id,
                    byte_offset,
                    byte_length,
                ),
                None,
            ))),
            ObjectKind::URLSearchParams => {
                // Always serialise via the `serialize_for_body`
                // helper so the wire bytes match `toString()`'s
                // observable output.
                let serialized = super::url_search_params::serialize_for_body(ctx.vm, obj_id);
                Ok(Some((serialized.into_bytes(), None)))
            }
            ObjectKind::FormData => {
                // Snapshot the entry list because the multipart
                // encoder needs `&VmInner` (read-only) and we
                // cannot keep a `&mut` borrow open across the
                // subsequent `intern` of the boundary string.
                let entries = ctx
                    .vm
                    .form_data_states
                    .get(&obj_id)
                    .cloned()
                    .unwrap_or_default();
                let (body, boundary) = super::multipart::encode(ctx.vm, &entries);
                let prefix = ctx
                    .vm
                    .strings
                    .get_utf8(ctx.vm.well_known.multipart_form_data_prefix);
                let ct_string = format!("{prefix}{boundary}");
                let ct_sid = ctx.vm.strings.intern(&ct_string);
                Ok(Some((body, Some(ct_sid))))
            }
            _ => {
                // Generic fallback: stringify.  Covers plain
                // objects / Arrays / numbers once wrapped.
                let sid = super::super::coerce::to_string(ctx.vm, val)?;
                let raw = ctx.vm.strings.get_utf8(sid);
                Ok(Some((raw.into_bytes(), None)))
            }
        },
        _ => {
            // String coercion covers number / bool / symbol-throws,
            // matching browsers' `new Request(url, {body: 42})` → "42".
            let sid = super::super::coerce::to_string(ctx.vm, val)?;
            let raw = ctx.vm.strings.get_utf8(sid);
            Ok(Some((raw.into_bytes(), None)))
        }
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

// ---------------------------------------------------------------------------
// Response constructor
// ---------------------------------------------------------------------------

/// `new Response(body?, init?)` (WHATWG §5.5).
fn native_response_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if !ctx.is_construct() {
        return Err(VmError::type_error(
            "Failed to construct 'Response': Please use the 'new' operator",
        ));
    }
    let JsValue::Object(inst_id) = this else {
        unreachable!("constructor `this` is always an Object after `do_new`");
    };
    let body_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let init_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let extracted = extract_body_bytes(ctx, body_arg)?;
    let body_default_content_type = match &extracted {
        // FormData returns a boundary-bearing override that
        // supersedes the static `content_type_for_body` mapping
        // (the boundary is only known after serialisation).  For
        // every other body kind the helper agrees with
        // `content_type_for_body`, so falling back to the latter
        // keeps the wiring deterministic.
        Some((_, Some(ct))) => Some(*ct),
        _ => content_type_for_body(ctx, body_arg),
    };
    let body_bytes = extracted.map(|(b, _)| b);
    build_response_instance(
        ctx,
        inst_id,
        body_bytes,
        body_default_content_type,
        init_arg,
        ResponseType::Default,
        0,
        false,
    )?;
    Ok(JsValue::Object(inst_id))
}

/// Build a Response on `inst_id` from parsed body bytes + init
/// dict.  Shared between the public `new Response(...)` path and
/// the `Response.redirect` / `Response.json` static factories.
///
/// `redirected` / `synthetic_status` override the init status when
/// non-zero (used by `Response.redirect(...)`).
#[allow(clippy::too_many_arguments)]
fn build_response_instance(
    ctx: &mut NativeContext<'_>,
    inst_id: ObjectId,
    body_bytes: Option<Vec<u8>>,
    body_default_content_type: Option<StringId>,
    init_arg: JsValue,
    response_type: ResponseType,
    synthetic_status: u16,
    redirected: bool,
) -> Result<(), VmError> {
    let (status_from_init, status_text_sid, init_headers) = parse_response_init(ctx, init_arg)?;
    let status = if synthetic_status != 0 {
        synthetic_status
    } else {
        status_from_init.unwrap_or(200)
    };

    // WHATWG §5.5 step "initialize a response" → reject null body
    // statuses (204 / 205 / 304) with an attached body (spec
    // prescribes `TypeError`).
    if matches!(status, 204 | 205 | 304) && body_bytes.is_some() {
        return Err(VmError::type_error(
            "Failed to construct 'Response': Response with null body status cannot have body",
        ));
    }

    // Allocate the companion Headers as `None` (mutable) so the
    // subsequent `init.headers` copy and default `Content-Type`
    // splice can succeed, then flip the guard to `Immutable` in
    // the block below — WHATWG Fetch §5.5 step 11 demands the
    // post-ctor surface be immutable so `resp.headers.append(...)`
    // throws TypeError.
    // Root `headers_id` across `fill_headers_like` (may invoke
    // user-supplied iterables' `.next()` / `.return()`) +
    // `ensure_content_type` + `body_data.insert` +
    // `response_states.insert`.  `headers_states` is not a GC
    // root on its own — the Headers is reached only via
    // `response_states[inst_id].headers_id`, which isn't installed
    // until the end of this helper.  `inst_id` is the ctor receiver
    // and is already rooted by the caller.  Same invariant as
    // R18.2 / Audit 1 (R18-audit).
    let headers_id = ctx.vm.create_headers(HeadersGuard::None);
    let mut g = ctx.vm.push_temp_root(JsValue::Object(headers_id));
    let mut rooted_holder = super::super::value::NativeContext { vm: &mut *g };
    let ctx = &mut rooted_holder;
    if let Some(hval) = init_headers {
        fill_headers_like(ctx, headers_id, hval, "Failed to construct 'Response'")?;
    }
    // If the caller supplied a default `Content-Type` and the user
    // didn't already set one via `init.headers`, populate it —
    // mirrors §5.5 "initialize a response" extract-body step 2.
    if let Some(ct_sid) = body_default_content_type {
        ensure_content_type(ctx, headers_id, ct_sid);
    }
    // Promote the guard to immutable only after we're done
    // mutating — the public Headers handle will refuse further
    // mutation from script.
    if let Some(state) = ctx.vm.headers_states.get_mut(&headers_id) {
        state.guard = HeadersGuard::Immutable;
    }

    if let Some(bytes) = body_bytes {
        ctx.vm.body_data.insert(inst_id, bytes);
    }

    ctx.vm.get_object_mut(inst_id).kind = ObjectKind::Response;
    let url_sid = ctx.vm.well_known.empty;
    ctx.vm.response_states.insert(
        inst_id,
        ResponseState {
            status,
            status_text_sid,
            url_sid,
            headers_id,
            response_type,
            redirected,
        },
    );
    Ok(())
}

/// Parse a `ResponseInit` dict.  Returns `(status, statusText,
/// headers)` (each optional where spec permits a default).
fn parse_response_init(
    ctx: &mut NativeContext<'_>,
    init: JsValue,
) -> Result<(Option<u16>, StringId, Option<JsValue>), VmError> {
    let default_status_text = ctx.vm.well_known.empty;
    match init {
        JsValue::Undefined | JsValue::Null => Ok((None, default_status_text, None)),
        JsValue::Object(opts_id) => {
            let wk = &ctx.vm.well_known;
            let status_key = PropertyKey::String(wk.status);
            let status_text_key = PropertyKey::String(wk.status_text);
            let headers_key = PropertyKey::String(wk.headers);

            let status_val = ctx.get_property_value(opts_id, status_key)?;
            let status_text_val = ctx.get_property_value(opts_id, status_text_key)?;
            let headers_val = ctx.get_property_value(opts_id, headers_key)?;

            let status = if matches!(status_val, JsValue::Undefined) {
                None
            } else {
                // WebIDL `[EnforceRange] unsigned short` — reject
                // NaN / ±∞ / out-of-[0,65535] as TypeError *before*
                // the spec's 200..=599 RangeError check (§5.5
                // "initialize a response" step 1 implicitly relies
                // on the earlier conversion rejecting wraps).
                let n = super::super::coerce::to_number(ctx.vm, status_val)?;
                let code = super::super::coerce::enforce_range_unsigned_short(
                    n,
                    "Failed to construct 'Response'",
                )?;
                if !(200..=599).contains(&code) {
                    return Err(VmError::range_error(format!(
                        "Failed to construct 'Response': The status provided ({code}) is outside the range [200, 599]."
                    )));
                }
                Some(code)
            };
            let status_text_sid = match status_text_val {
                JsValue::Undefined => default_status_text,
                other => {
                    let sid = super::super::coerce::to_string(ctx.vm, other)?;
                    // WHATWG §5.5 statusText must match HTTP reason-phrase
                    // grammar (ASCII without CR/LF/NUL).  Phase 2 only
                    // rejects the obvious CR/LF/NUL case to match the
                    // spec's normative error path.
                    let raw = ctx.vm.strings.get_utf8(sid);
                    if raw.bytes().any(|b| matches!(b, 0x00 | 0x0A | 0x0D)) {
                        return Err(VmError::type_error(
                            "Failed to construct 'Response': Invalid statusText",
                        ));
                    }
                    sid
                }
            };
            let headers_override = match headers_val {
                JsValue::Undefined => None,
                other => Some(other),
            };
            Ok((status, status_text_sid, headers_override))
        }
        _ => Err(VmError::type_error(
            "Failed to construct 'Response': init must be an object",
        )),
    }
}

/// Default `Content-Type` for a body argument (WHATWG §5 "extract
/// a body").  `String` bodies default to
/// `"text/plain;charset=UTF-8"`; `Blob` bodies carry their own
/// `type` (or nothing if the Blob's type is empty);
/// `URLSearchParams` bodies default to
/// `"application/x-www-form-urlencoded;charset=UTF-8"`.
/// `ArrayBuffer` has no default CT — matches spec (§5 step 4.7
/// "If object is a BufferSource, ... set Content-Type to null").
///
/// `FormData` is **not** handled here — its boundary-bearing
/// `Content-Type` is computed inline by [`extract_body_bytes`]
/// because the boundary is only known after serialisation.  Builds
/// that consult `content_type_for_body` for a FormData body
/// receive `None`; the [`build_response_instance`] /
/// [`request_ctor`] paths thread the override returned by
/// `extract_body_bytes` ahead of this fallback.
pub(super) fn content_type_for_body(ctx: &NativeContext<'_>, body: JsValue) -> Option<StringId> {
    match body {
        JsValue::String(_) => Some(ctx.vm.well_known.text_plain_charset_utf8),
        JsValue::Object(obj_id) => match ctx.vm.get_object(obj_id).kind {
            ObjectKind::Blob => {
                let ty = super::blob::blob_type(ctx.vm, obj_id);
                // An empty type means "don't expose a Content-Type"
                // per WHATWG §5 step 4.4.3 "If object's type
                // attribute is not the empty string, set
                // Content-Type to its value".
                if ty == ctx.vm.well_known.empty {
                    None
                } else {
                    Some(ty)
                }
            }
            ObjectKind::URLSearchParams => Some(ctx.vm.well_known.application_form_urlencoded),
            // `ArrayBuffer` / `FormData` / others fall through to
            // `None` — see fn doc.
            _ => None,
        },
        _ => None,
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
///
/// `pub(super)` so the accessors module (which hosts the two
/// `clone()` implementations) can share the splice path.
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

// ---------------------------------------------------------------------------
// Response static factories
// ---------------------------------------------------------------------------

/// `Response.error()` (WHATWG §5.5.6).  Network-error response —
/// `status === 0`, `type === "error"`, immutable empty headers.
fn native_response_static_error(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // Allocate a raw Response instance (not via `new Response()`
    // because the ctor rejects status 0 → "outside [200, 599]").
    //
    // Root `inst_id` across the subsequent `create_headers` call:
    // the new Response is reachable only via this Rust local until
    // `response_states.insert(...)` links it at the end of the
    // function, and `create_headers` itself allocates an object
    // that can trigger GC under a future refactor (R18-audit).
    let proto = ctx.vm.response_prototype;
    let inst_id = ctx.vm.alloc_object(Object {
        kind: ObjectKind::Response,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });
    let mut g = ctx.vm.push_temp_root(JsValue::Object(inst_id));
    let headers_id = g.create_headers(HeadersGuard::Immutable);
    let empty_sid = g.well_known.empty;
    g.response_states.insert(
        inst_id,
        ResponseState {
            status: 0,
            status_text_sid: empty_sid,
            url_sid: empty_sid,
            headers_id,
            response_type: ResponseType::Error,
            redirected: false,
        },
    );
    Ok(JsValue::Object(inst_id))
}

/// `Response.redirect(url, status?)` (WHATWG §5.5.7).
fn native_response_static_redirect(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if args.is_empty() {
        return Err(VmError::type_error(
            "Failed to execute 'redirect' on 'Response': 1 argument required, but only 0 present.",
        ));
    }
    let url_sid = super::super::coerce::to_string(ctx.vm, args[0])?;
    let raw_url = ctx.vm.strings.get_utf8(url_sid);
    let url = parse_url(ctx.vm, &raw_url).map_err(|_| {
        VmError::type_error(format!(
            "Failed to execute 'redirect' on 'Response': Invalid URL '{raw_url}'"
        ))
    })?;
    let abs_url_sid = ctx.vm.strings.intern(url.as_str());

    let status = if let Some(s) = args.get(1).copied() {
        if matches!(s, JsValue::Undefined) {
            302
        } else {
            // WebIDL `[EnforceRange] unsigned short` — NaN / ±∞ /
            // out-of-[0,65535] is TypeError, the subsequent
            // redirect-code membership check is RangeError.
            let n = super::super::coerce::to_number(ctx.vm, s)?;
            let code = super::super::coerce::enforce_range_unsigned_short(
                n,
                "Failed to execute 'redirect' on 'Response'",
            )?;
            if !matches!(code, 301 | 302 | 303 | 307 | 308) {
                return Err(VmError::range_error(format!(
                    "Failed to execute 'redirect' on 'Response': Invalid status code {code}"
                )));
            }
            code
        }
    } else {
        302
    };

    // Root `inst_id` across `create_headers` (which allocates an
    // object and would otherwise collect the newly-allocated
    // Response under a future GC-enabled refactor).  `strings
    // .intern` + `headers_states` mutation + `well_known` access
    // after `create_headers` are alloc-free, so `headers_id`
    // itself reaches `response_states` without a separate root
    // (R18-audit).
    let proto = ctx.vm.response_prototype;
    let inst_id = ctx.vm.alloc_object(Object {
        kind: ObjectKind::Response,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });
    let mut g = ctx.vm.push_temp_root(JsValue::Object(inst_id));
    let headers_id = g.create_headers(HeadersGuard::None);
    let location_name = g.strings.intern("location");
    if let Some(state) = g.headers_states.get_mut(&headers_id) {
        state.list.push((location_name, abs_url_sid));
        state.guard = HeadersGuard::Immutable;
    }
    let empty_sid = g.well_known.empty;
    g.response_states.insert(
        inst_id,
        ResponseState {
            status,
            status_text_sid: empty_sid,
            url_sid: empty_sid,
            headers_id,
            // WHATWG Fetch §5.5 step 7: `Response.redirect(...)`
            // produces an opaque-redirect response.  `type` must
            // therefore expose `"opaqueredirect"`; not `"default"`.
            response_type: ResponseType::OpaqueRedirect,
            redirected: false,
        },
    );
    Ok(JsValue::Object(inst_id))
}

/// `Response.json(data, init?)` (WHATWG §5.5.8, ES2023
/// addition).  Stringifies `data` via `JSON.stringify`, uses the
/// result as the body, and sets `Content-Type:
/// application/json`.
fn native_response_static_json(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    // Defer to `native_json_stringify` for the serialisation —
    // re-using the spec-compliant path keeps us in sync with
    // `JSON.stringify` semantics (cycle detection, replacer
    // fn / list, Number/BigInt / toJSON etc.).
    let data = args.first().copied().unwrap_or(JsValue::Undefined);
    let json_val =
        super::super::natives_json::native_json_stringify(ctx, JsValue::Undefined, &[data])?;
    let body_bytes = match json_val {
        JsValue::String(sid) => {
            let raw = ctx.vm.strings.get_utf8(sid);
            Some(raw.into_bytes())
        }
        _ => {
            // `JSON.stringify(undefined)` → `undefined` → body is
            // absent.  Matches browsers which pass through
            // undefined literally: `Response.json(undefined).text()`
            // resolves to `""`.
            None
        }
    };
    let init = args.get(1).copied().unwrap_or(JsValue::Undefined);

    let proto = ctx.vm.response_prototype;
    let inst_id = ctx.vm.alloc_object(Object {
        kind: ObjectKind::Response,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });
    build_response_instance(
        ctx,
        inst_id,
        body_bytes,
        Some(ctx.vm.well_known.application_json_utf8),
        init,
        ResponseType::Default,
        0,
        false,
    )?;
    Ok(JsValue::Object(inst_id))
}
