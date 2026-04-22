//! `Request` / `Response` interfaces (WHATWG Fetch §5.3 / §5.5).
//!
//! Both variants are payload-free ([`super::super::value::ObjectKind::Request`]
//! / [`super::super::value::ObjectKind::Response`]); per-instance state
//! lives in out-of-band side tables ([`RequestState`] / [`ResponseState`])
//! keyed by the instance's own `ObjectId`.  Body bytes — when present
//! — share a single `VmInner::body_data` map across both variants so
//! `clone()` can cheaply `Arc::clone` the backing buffer.
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
//! - `request.clone()` / `response.clone()` — shared body via `Arc`.
//! - `Response.error()` / `Response.redirect(url, status)` /
//!   `Response.json(data, init?)` static factories.
//! - `.text()` / `.json()` / `.arrayBuffer()` / `.blob()` read
//!   methods live in [`super::body_mixin`] and share the
//!   [`VmInner::body_data`] side table with this module.
//! - `.body` getter always returns `null` (Phase 2 non-streaming;
//!   a later PR supplies the `ReadableStream` replacement — see
//!   `~/.claude/plans/pr5a-fetch.md` §D10).
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

use std::sync::Arc;

use url::Url;

use super::super::shape::{self, PropertyAttrs};
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, StringId, VmError,
};
use super::super::{NativeFn, VmInner};
use super::headers::HeadersGuard;
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
// Enums — stored on state but not enforced in Phase 2
// ---------------------------------------------------------------------------

/// `RequestRedirect` (WHATWG §5.3).  Phase 2 stores the selected
/// mode verbatim but does not change `fetch()` behaviour yet —
/// the full redirect state machine lands with async fetch.  The
/// unused variants are kept so the upcoming `init.redirect`
/// parse path can select them without redefining the enum when
/// `fetch()` integration lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum RedirectMode {
    Follow,
    Error,
    Manual,
}

/// `RequestMode` (WHATWG §5.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum RequestMode {
    Cors,
    NoCors,
    SameOrigin,
    Navigate,
}

/// `RequestCredentials` (WHATWG §5.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum RequestCredentials {
    Omit,
    SameOrigin,
    Include,
}

/// `RequestCache` (WHATWG §5.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum RequestCache {
    Default,
    NoStore,
    Reload,
    NoCache,
    ForceCache,
    OnlyIfCached,
}

/// `ResponseType` (WHATWG §5.5).  Stored on Response state and
/// surfaced via the `.type` IDL attribute.  The `Basic` / `Cors`
/// / `Opaque` / `OpaqueRedirect` arms are selected only by the
/// upcoming `fetch()` path once CORS resolution runs, so they
/// appear unused here.
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

/// Tuple returned by [`resolve_request_input`]: the URL StringId
/// is canonicalised; method defaults to `GET` unless the input was
/// itself a `Request` (then its method carries over); `source_headers`
/// is `Some` for the Request-clone case; `source_body` is the cloned
/// body Arc (may be `None`).
type RequestInputParts = (StringId, StringId, Option<ObjectId>, Option<Arc<[u8]>>);

/// Tuple returned by [`parse_request_init`]: optional method
/// override, optional headers-init source (copied into the
/// companion Headers), optional body bytes.
type RequestInitParts = (Option<StringId>, Option<JsValue>, Option<Arc<[u8]>>);

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
    #[allow(dead_code)]
    pub(crate) redirect: RedirectMode,
    #[allow(dead_code)]
    pub(crate) mode: RequestMode,
    #[allow(dead_code)]
    pub(crate) credentials: RequestCredentials,
    #[allow(dead_code)]
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
        let clone_name_sid = self.well_known.clone;
        let clone_fn = self.create_native_function("clone", native_request_clone);
        self.define_shaped_property(
            proto_id,
            PropertyKey::String(clone_name_sid),
            PropertyValue::Data(JsValue::Object(clone_fn)),
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
            let name = self.strings.get_utf8(name_sid);
            let fn_id = self.create_native_function(&name, func);
            self.define_shaped_property(
                ctor,
                PropertyKey::String(name_sid),
                PropertyValue::Data(JsValue::Object(fn_id)),
                PropertyAttrs::METHOD,
            );
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
        let clone_name_sid = self.well_known.clone;
        let clone_fn = self.create_native_function("clone", native_response_clone);
        self.define_shaped_property(
            proto_id,
            PropertyKey::String(clone_name_sid),
            PropertyValue::Data(JsValue::Object(clone_fn)),
            PropertyAttrs::METHOD,
        );
    }

    /// Install a `[(name_sid, getter)]` batch as WEBIDL_RO
    /// accessors on `proto_id`.  Setters default to `None` (every
    /// readonly WebIDL attribute).  The getter's WebIDL display
    /// name is derived as `"get {name}"` to match browsers.
    fn install_ro_accessor_list(&mut self, proto_id: ObjectId, list: &[(StringId, NativeFn)]) {
        for &(name_sid, getter) in list {
            let name = self.strings.get_utf8(name_sid);
            let gid = self.create_native_function(&format!("get {name}"), getter);
            self.define_shaped_property(
                proto_id,
                PropertyKey::String(name_sid),
                PropertyValue::Accessor {
                    getter: Some(gid),
                    setter: None,
                },
                PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Request constructor
// ---------------------------------------------------------------------------

/// `new Request(input, init?)` (WHATWG §5.3).
fn native_request_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if !ctx.is_construct() {
        return Err(VmError::type_error(
            "Failed to construct 'Request': Please use the 'new' operator",
        ));
    }
    if args.is_empty() {
        return Err(VmError::type_error(
            "Failed to construct 'Request': 1 argument required, but only 0 present.",
        ));
    }
    let JsValue::Object(inst_id) = this else {
        unreachable!("constructor `this` is always an Object after `do_new`");
    };

    let input = args[0];
    let init = args.get(1).copied().unwrap_or(JsValue::Undefined);

    let (url_sid, method_sid, headers_source, body_bytes) = resolve_request_input(ctx, input)?;
    let (override_method, headers_init_arg, body_init_arg) = parse_request_init(ctx, init)?;
    let method_sid = override_method.unwrap_or(method_sid);

    // Allocate companion Headers (guard = None; a later PR tightens
    // to `request` guard once the forbidden-header list is enforced).
    let headers_id = ctx.vm.create_headers(HeadersGuard::None);
    // Copy entries from either the source Request's headers or the
    // init dict's `headers` value (if provided, it overrides).
    match headers_init_arg {
        Some(h) => fill_headers_like(ctx, headers_id, h)?,
        None => {
            if let Some(src_headers_id) = headers_source {
                copy_headers_entries(ctx, src_headers_id, headers_id);
            }
        }
    }

    // Body: `init.body` overrides; otherwise inherit from source
    // Request (with the same Arc).  Source inheritance is `None`
    // if no body was set.
    let body_bytes = body_init_arg.or(body_bytes);
    if let Some(bytes) = body_bytes {
        ctx.vm.body_data.insert(inst_id, bytes);
    }

    // Promote the pre-allocated Ordinary instance into Request.
    ctx.vm.get_object_mut(inst_id).kind = ObjectKind::Request;
    ctx.vm.request_states.insert(
        inst_id,
        RequestState {
            method_sid,
            url_sid,
            headers_id,
            redirect: RedirectMode::Follow,
            mode: RequestMode::Cors,
            credentials: RequestCredentials::SameOrigin,
            cache: RequestCache::Default,
        },
    );
    Ok(JsValue::Object(inst_id))
}

/// Resolve `input` (first arg of `new Request(...)`) into its
/// URL / default method / optional source-headers / optional
/// source-body.
///
/// - String → parse URL (relative → resolve against
///   `navigation.current_url`), method defaults to `"GET"`, no
///   source Headers, no source body.
/// - Request object → copy its state (URL / method / headers id
///   / body Arc).  Body is "taken" from the source per spec §5.3
///   step 37 — but Phase 2 shares the Arc without marking the
///   source as consumed, because the body-used tracking only
///   applies once the Body mixin read methods land.
/// - Anything else → `TypeError`.
fn resolve_request_input(
    ctx: &mut NativeContext<'_>,
    input: JsValue,
) -> Result<RequestInputParts, VmError> {
    match input {
        JsValue::String(sid) => {
            let raw = ctx.vm.strings.get_utf8(sid);
            let url = parse_url(ctx.vm, &raw)?;
            let url_sid = ctx.vm.strings.intern(url.as_str());
            let method_sid = ctx.vm.well_known.http_get;
            Ok((url_sid, method_sid, None, None))
        }
        JsValue::Object(obj_id) => {
            if !matches!(ctx.vm.get_object(obj_id).kind, ObjectKind::Request) {
                return Err(VmError::type_error(
                    "Failed to construct 'Request': input must be a URL string or Request",
                ));
            }
            let state = ctx
                .vm
                .request_states
                .get(&obj_id)
                .expect("Request without request_states entry");
            let url_sid = state.url_sid;
            let method_sid = state.method_sid;
            let headers_id = state.headers_id;
            let body = ctx.vm.body_data.get(&obj_id).map(Arc::clone);
            Ok((url_sid, method_sid, Some(headers_id), body))
        }
        _ => Err(VmError::type_error(
            "Failed to construct 'Request': input must be a URL string or Request",
        )),
    }
}

/// Parse the `init` dict (§5.3 step 27-38).  Returns the
/// optional method override, optional headers source, and
/// optional body bytes.  Unknown members are ignored silently.
fn parse_request_init(
    ctx: &mut NativeContext<'_>,
    init: JsValue,
) -> Result<RequestInitParts, VmError> {
    match init {
        JsValue::Undefined | JsValue::Null => Ok((None, None, None)),
        JsValue::Object(opts_id) => {
            let wk = &ctx.vm.well_known;
            let method_key = PropertyKey::String(wk.method);
            let headers_key = PropertyKey::String(wk.headers);
            let body_key = PropertyKey::String(wk.body);

            let method_val = ctx.get_property_value(opts_id, method_key)?;
            let headers_val = ctx.get_property_value(opts_id, headers_key)?;
            let body_val = ctx.get_property_value(opts_id, body_key)?;

            let method_override = match method_val {
                JsValue::Undefined => None,
                other => Some(normalise_method(ctx, other)?),
            };
            let headers_override = match headers_val {
                JsValue::Undefined => None,
                other => Some(other),
            };
            let body_override = extract_body_bytes(ctx, body_val)?;
            Ok((method_override, headers_override, body_override))
        }
        _ => Err(VmError::type_error(
            "Failed to construct 'Request': init must be an object",
        )),
    }
}

/// WHATWG §5.3 step 23 + §4.6 forbidden-method filter.
/// Uppercases canonical method names; rejects `CONNECT` / `TRACE` /
/// `TRACK` (forbidden).  Other tokens pass through verbatim — spec
/// also requires them to match RFC 7230 token syntax, which
/// Phase 2 defers (unknown methods that violate RFC 7230 are
/// accepted and relayed downstream).
fn normalise_method(ctx: &mut NativeContext<'_>, val: JsValue) -> Result<StringId, VmError> {
    let raw_sid = super::super::coerce::to_string(ctx.vm, val)?;
    let raw = ctx.vm.strings.get_utf8(raw_sid);
    let upper = validate_http_method(&raw, "Failed to construct 'Request'")?;
    let wk = &ctx.vm.well_known;
    Ok(match upper.as_str() {
        "GET" => wk.http_get,
        "HEAD" => wk.http_head,
        "POST" => wk.http_post,
        "PUT" => wk.http_put,
        "DELETE" => wk.http_delete,
        "OPTIONS" => wk.http_options,
        "PATCH" => wk.http_patch,
        _ => ctx.vm.strings.intern(&upper),
    })
}

/// Uppercase `raw` and reject WHATWG §4.6 forbidden methods
/// (`CONNECT` / `TRACE` / `TRACK`).  Returns the uppercase
/// `String` on success; error messages are prefixed with
/// `error_prefix` (e.g. `"Failed to construct 'Request'"` or
/// `"Failed to execute 'fetch'"`) so the caller's reporting
/// context is preserved.  Shared by `Request`'s ctor and
/// `fetch()`'s init parse.
pub(super) fn validate_http_method(raw: &str, error_prefix: &str) -> Result<String, VmError> {
    let upper = raw.to_ascii_uppercase();
    if matches!(upper.as_str(), "CONNECT" | "TRACE" | "TRACK") {
        return Err(VmError::type_error(format!(
            "{error_prefix}: '{raw}' HTTP method is unsupported."
        )));
    }
    Ok(upper)
}

/// Coerce a body init value into raw UTF-8 bytes.  Accepts
/// `String` / `ArrayBuffer` / `Blob` directly (per WHATWG §5
/// "extract a body"); any other non-null / non-undefined value is
/// `ToString`-coerced, matching browsers' forgiving
/// `new Request(url, {body: 42})` → `"42"` behaviour.
///
/// `FormData` / `URLSearchParams` / `ReadableStream` land with
/// their own tranches.
///
/// `pub(super)` so the `fetch()` host (`vm/host/fetch.rs`) can
/// reuse the exact same coercion path for `init.body` without
/// duplicating the ArrayBuffer / Blob extraction branches — the
/// two code paths would otherwise drift.
pub(super) fn extract_body_bytes(
    ctx: &mut NativeContext<'_>,
    val: JsValue,
) -> Result<Option<Arc<[u8]>>, VmError> {
    match val {
        JsValue::Undefined | JsValue::Null => Ok(None),
        JsValue::String(sid) => {
            let raw = ctx.vm.strings.get_utf8(sid);
            Ok(Some(Arc::from(raw.as_bytes())))
        }
        JsValue::Object(obj_id) => match ctx.vm.get_object(obj_id).kind {
            ObjectKind::ArrayBuffer => Ok(Some(super::array_buffer::array_buffer_bytes(
                ctx.vm, obj_id,
            ))),
            ObjectKind::Blob => Ok(Some(super::blob::blob_bytes(ctx.vm, obj_id))),
            _ => {
                // Generic fallback: stringify.  Covers plain
                // objects / Arrays / numbers once wrapped.
                let sid = super::super::coerce::to_string(ctx.vm, val)?;
                let raw = ctx.vm.strings.get_utf8(sid);
                Ok(Some(Arc::from(raw.as_bytes())))
            }
        },
        _ => {
            // String coercion covers number / bool / symbol-throws,
            // matching browsers' `new Request(url, {body: 42})` → "42".
            let sid = super::super::coerce::to_string(ctx.vm, val)?;
            let raw = ctx.vm.strings.get_utf8(sid);
            Ok(Some(Arc::from(raw.as_bytes())))
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
    let body_bytes = extract_body_bytes(ctx, body_arg)?;
    let body_default_content_type = content_type_for_body(ctx, body_arg);
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
    body_bytes: Option<Arc<[u8]>>,
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

    // Allocate the companion Headers as **Immutable** so post-ctor
    // `resp.headers.append(...)` throws TypeError per §5.5.
    // Copy entries from the init's `headers` member if present.
    let headers_id = ctx.vm.create_headers(HeadersGuard::None);
    if let Some(hval) = init_headers {
        fill_headers_like(ctx, headers_id, hval)?;
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
                // WebIDL `unsigned short` with [EnforceRange] —
                // Phase 2 implements the ToUint16 path and the
                // 200..=599 range check below; [EnforceRange]
                // rejection of out-of-bounds f64 before ToUint16
                // lands when `[EnforceRange]` becomes a general
                // coerce helper.
                let n = super::super::coerce::to_number(ctx.vm, status_val)?;
                let code = super::super::coerce::f64_to_uint16(n);
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
/// `type` (or nothing if the Blob's type is empty).  `ArrayBuffer`
/// has no default CT — matches spec (§5 step 4.7 "If object is a
/// BufferSource, ... set Content-Type to null").
fn content_type_for_body(ctx: &NativeContext<'_>, body: JsValue) -> Option<StringId> {
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
            // `ArrayBuffer` bodies fall through to `None` — spec
            // §5 step 4.7 "If object is a BufferSource, ... set
            // Content-Type to null".
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
fn fill_headers_like(
    ctx: &mut NativeContext<'_>,
    target_headers_id: ObjectId,
    init: JsValue,
) -> Result<(), VmError> {
    super::headers::fill_headers_from_init(ctx, target_headers_id, init)
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
fn ensure_content_type(
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
    let proto = ctx.vm.response_prototype;
    let inst_id = ctx.vm.alloc_object(Object {
        kind: ObjectKind::Response,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });
    let headers_id = ctx.vm.create_headers(HeadersGuard::Immutable);
    let wk = &ctx.vm.well_known;
    ctx.vm.response_states.insert(
        inst_id,
        ResponseState {
            status: 0,
            status_text_sid: wk.empty,
            url_sid: wk.empty,
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
            let n = super::super::coerce::to_number(ctx.vm, s)?;
            let code = super::super::coerce::f64_to_uint16(n);
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

    let proto = ctx.vm.response_prototype;
    let inst_id = ctx.vm.alloc_object(Object {
        kind: ObjectKind::Response,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });
    let headers_id = ctx.vm.create_headers(HeadersGuard::None);
    let location_name = ctx.vm.strings.intern("location");
    if let Some(state) = ctx.vm.headers_states.get_mut(&headers_id) {
        state.list.push((location_name, abs_url_sid));
        state.guard = HeadersGuard::Immutable;
    }
    let wk = &ctx.vm.well_known;
    ctx.vm.response_states.insert(
        inst_id,
        ResponseState {
            status,
            status_text_sid: wk.empty,
            url_sid: wk.empty,
            headers_id,
            response_type: ResponseType::Default,
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
            Some(Arc::from(raw.as_bytes()))
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
