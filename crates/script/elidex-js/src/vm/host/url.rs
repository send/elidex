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
//! ## Implemented surface (Phase 1)
//!
//! - `new URL(input, base?)` — both args coerced via
//!   [`super::super::coerce::to_string`].  Relative `input` resolves
//!   against `base` per WHATWG URL §4.4 ("URL parser").  A parse
//!   failure throws `TypeError` (matches V8 / Firefox 2023+).
//! - `.toString()` / `.toJSON()` — both serialise the `[[URL]]`
//!   internal slot to its WHATWG URL §4.5 ("URL serializer") form.
//!   Aliased to the same native fn (the spec defines `toJSON` as a
//!   pointer to the same algorithm).
//!
//! Phases 2-5 add accessors, setters, the `searchParams` link, and
//! the `URL.canParse` / `URL.parse` statics.

#![cfg(feature = "engine")]

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
/// in-place via `set_*` setters (Phase 3).  `search_params` is
/// `None` until the constructor lazily creates the linked
/// `URLSearchParams` instance (Phase 4); accessor reads
/// (`searchParams` IDL attribute) ensure the same `ObjectId` is
/// returned on every read.
pub(crate) struct UrlState {
    /// Parsed URL — every accessor reads through this; setters
    /// mutate it in place via the `url::Url::set_*` API.
    pub(crate) url: url::Url,
    /// Linked `URLSearchParams` instance `ObjectId` (Phase 4).
    /// `None` in Phase 1 — the `searchParams` IDL attribute is
    /// not yet wired up.
    pub(crate) search_params: Option<ObjectId>,
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

impl VmInner {
    /// Allocate `URL.prototype`, install its method suite, and
    /// expose the `URL` constructor on `globals`.
    ///
    /// Called from `register_globals()` immediately after
    /// `register_url_search_params_global()` so the `URL`
    /// constructor's eager `URLSearchParams` allocation in Phase 4
    /// can rely on the prototype existing.
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
        // "URL static methods", added 2023).  Both share the same
        // input-parsing algorithm via [`parse_url_arguments`] —
        // `canParse` returns the result truthiness, `parse` returns
        // a fresh `URL` instance or `null`.  Statics install onto
        // the constructor object directly (mirrors `Number.isFinite`
        // / `Array.isArray` precedent).
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
        // [`url::Url`]) plus the read-only `origin`.
        // `searchParams` is installed separately in Phase 4
        // alongside the bidirectional linkage.  `WEBIDL_RO_ACCESSOR`
        // is used for both shapes — the constant's `writable` bit
        // is meaningless for accessor properties and the read /
        // read-write distinction is encoded by the presence of the
        // setter (see `shape::PropertyAttrs::WEBIDL_RO_ACCESSOR`
        // doc).
        let wk = &self.well_known;
        let accessors: [(super::super::value::StringId, NativeFn, Option<NativeFn>); 11] = [
            (wk.href, native_url_get_href, Some(native_url_set_href)),
            (wk.origin, native_url_get_origin, None),
            (
                wk.protocol,
                native_url_get_protocol,
                Some(native_url_set_protocol),
            ),
            (
                wk.username,
                native_url_get_username,
                Some(native_url_set_username),
            ),
            (
                wk.password,
                native_url_get_password,
                Some(native_url_set_password),
            ),
            (wk.host_attr, native_url_get_host, Some(native_url_set_host)),
            (
                wk.hostname,
                native_url_get_hostname,
                Some(native_url_set_hostname),
            ),
            (wk.port_attr, native_url_get_port, Some(native_url_set_port)),
            (
                wk.pathname,
                native_url_get_pathname,
                Some(native_url_set_pathname),
            ),
            (
                wk.search_attr,
                native_url_get_search,
                Some(native_url_set_search),
            ),
            (wk.hash_attr, native_url_get_hash, Some(native_url_set_hash)),
        ];
        for (name_sid, getter, setter) in accessors {
            self.install_accessor_pair(
                proto_id,
                name_sid,
                getter,
                setter,
                PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }

        // `searchParams` IDL attribute — read-only, returns the
        // eagerly-allocated linked `URLSearchParams` instance
        // (Phase 4 of slot #9.5).  Identity stability
        // (`url.searchParams === url.searchParams`) is by virtue
        // of the constructor allocating once and storing the same
        // `ObjectId` on every read.
        self.install_accessor_pair(
            proto_id,
            self.well_known.search_params,
            native_url_get_search_params,
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
/// the IDL `searchParams` attribute (Phase 4 of slot #9.5):
/// WHATWG URL §6.1 requires `url.searchParams ===
/// url.searchParams` to hold across reads, and a lazy-create path
/// would either need a per-instance cache or a shape-property
/// install.  Eager creation avoids both — Phase 4 chooses this for
/// design simplicity given that the cost is one `alloc_object`
/// per `new URL()` call.
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
pub(super) fn parse_url_arguments(
    ctx: &mut NativeContext<'_>,
    input: JsValue,
    base: JsValue,
) -> Result<url::Url, VmError> {
    let input_sid = super::super::coerce::to_string(ctx.vm, input)?;
    let input_str = ctx.vm.strings.get_utf8(input_sid);
    let base_str: Option<String> = match base {
        JsValue::Undefined | JsValue::Null => None,
        other => {
            let sid = super::super::coerce::to_string(ctx.vm, other)?;
            Some(ctx.vm.strings.get_utf8(sid))
        }
    };
    parse_url_strings(input_str.as_str(), base_str.as_deref())
}

/// Parse `input` against optional `base`, returning `Ok(Url)` on
/// success and a `TypeError` on parse failure.  Factored out of
/// [`parse_url_arguments`] so the `URL.canParse` / `URL.parse`
/// statics (Phase 5) can share the algorithm without re-coercing
/// already-strings.
pub(super) fn parse_url_strings(input: &str, base: Option<&str>) -> Result<url::Url, VmError> {
    if let Some(base_str) = base {
        let base_url = url::Url::parse(base_str)
            .map_err(|_| VmError::type_error(format!("URL: invalid base URL: {base_str}")))?;
        base_url
            .join(input)
            .map_err(|_| VmError::type_error(format!("URL: invalid URL: {input}")))
    } else {
        url::Url::parse(input)
            .map_err(|_| VmError::type_error(format!("URL: invalid URL: {input}")))
    }
}

// ---------------------------------------------------------------------------
// Statics
// ---------------------------------------------------------------------------

/// `URL.canParse(input, base?)` (WHATWG URL §6.1).  Equivalent to
/// `try { new URL(input, base); true } catch { false }` — but
/// avoids the `URL` allocation when the parse succeeds.
fn native_url_can_parse_static(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let input_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let base_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    Ok(JsValue::Boolean(
        parse_url_arguments(ctx, input_arg, base_arg).is_ok(),
    ))
}

/// `URL.parse(input, base?)` (WHATWG URL §6.1).  Returns a fresh
/// `URL` instance on success, `null` on parse failure — replaces
/// the `try { new URL(...) } catch { null }` idiom without
/// touching the throw machinery.
fn native_url_parse_static(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let input_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let base_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let parsed = match parse_url_arguments(ctx, input_arg, base_arg) {
        Ok(u) => u,
        Err(_) => return Ok(JsValue::Null),
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
/// [`VmTempRoot`] guard pushed below keeps it pinned through the
/// inner allocation's potential GC cycle.  Without the guard the
/// URL could be swept and the `url_states[id]` entry pruned
/// before we attach the linked `searchParams`.
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
// Read-only accessor getters (WHATWG URL §6.1)
// ---------------------------------------------------------------------------

fn native_url_get_href(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_url_this(ctx, this, "href")?;
    url_component(ctx, id, |u| u.as_str().to_string())
}

/// `URL.prototype.origin` (WHATWG URL §6.1).  Read-only.  Returns
/// the ASCII serialisation of the URL's origin tuple — for opaque
/// origins (`file:` / `data:` / `blob:` / `about:`) this is the
/// literal `"null"` string.
fn native_url_get_origin(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_url_this(ctx, this, "origin")?;
    url_component(ctx, id, |u| u.origin().ascii_serialization())
}

fn native_url_get_protocol(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_url_this(ctx, this, "protocol")?;
    // Spec mandates the trailing `:` (WHATWG URL §6.1 "protocol getter").
    url_component(ctx, id, |u| format!("{}:", u.scheme()))
}

fn native_url_get_username(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_url_this(ctx, this, "username")?;
    url_component(ctx, id, |u| u.username().to_string())
}

fn native_url_get_password(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_url_this(ctx, this, "password")?;
    url_component(ctx, id, |u| u.password().unwrap_or("").to_string())
}

/// `host` IDL attribute — `hostname[:port]`.  Port is omitted when
/// the URL has no explicit port (default-port stripping is handled
/// by `url::Url` at parse time).
fn native_url_get_host(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_url_this(ctx, this, "host")?;
    url_component(ctx, id, |u| match (u.host_str(), u.port()) {
        (Some(h), Some(p)) => format!("{h}:{p}"),
        (Some(h), None) => h.to_string(),
        (None, _) => String::new(),
    })
}

fn native_url_get_hostname(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_url_this(ctx, this, "hostname")?;
    url_component(ctx, id, |u| u.host_str().unwrap_or("").to_string())
}

fn native_url_get_port(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_url_this(ctx, this, "port")?;
    url_component(ctx, id, |u| {
        u.port().map_or(String::new(), |p| p.to_string())
    })
}

fn native_url_get_pathname(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_url_this(ctx, this, "pathname")?;
    url_component(ctx, id, |u| u.path().to_string())
}

/// `search` IDL attribute — `?`-prefixed query when non-empty,
/// empty string when the query is absent or empty (matches WHATWG
/// URL §6.1 "search getter").
fn native_url_get_search(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_url_this(ctx, this, "search")?;
    url_component(ctx, id, |u| match u.query() {
        Some(q) if !q.is_empty() => format!("?{q}"),
        _ => String::new(),
    })
}

/// `searchParams` IDL attribute (read-only) — return the linked
/// `URLSearchParams` `ObjectId` allocated eagerly by the
/// constructor (Phase 4 of slot #9.5).  WHATWG URL §6.1 mandates
/// that `url.searchParams === url.searchParams` holds, so the same
/// `ObjectId` is returned on every access.
fn native_url_get_search_params(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_url_this(ctx, this, "searchParams")?;
    let sp = ctx
        .vm
        .url_states
        .get(&id)
        .and_then(|state| state.search_params);
    match sp {
        Some(sp_id) => Ok(JsValue::Object(sp_id)),
        // The constructor always populates `search_params`, so the
        // `None` branch is only reachable if a downstream caller
        // synthesises a `URL` instance without going through
        // `native_url_constructor`.  Treat as a defensive
        // TypeError rather than silently returning undefined —
        // matches the brand-check intent.
        None => Err(VmError::type_error(
            "URL.prototype.searchParams: missing internal slot",
        )),
    }
}

/// `hash` IDL attribute — `#`-prefixed fragment when non-empty,
/// empty string when absent or empty (WHATWG URL §6.1 "hash getter").
fn native_url_get_hash(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_url_this(ctx, this, "hash")?;
    url_component(ctx, id, |u| match u.fragment() {
        Some(f) if !f.is_empty() => format!("#{f}"),
        _ => String::new(),
    })
}

// ---------------------------------------------------------------------------
// Read/write accessor setters (WHATWG URL §6.1)
// ---------------------------------------------------------------------------

/// `url.href = …` — full re-parse.  Throws `TypeError` when the
/// new value does not parse as an absolute URL (matches V8 / Firefox
/// 2023+; the WHATWG spec is to silently ignore but every browser
/// throws here, and boa-side throws too).  Re-parse refreshes the
/// linked `URLSearchParams` entry list (Phase 4 of slot #9.5).
fn native_url_set_href(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_url_this(ctx, this, "href")?;
    let val = take_url_setter_arg(ctx, args)?;
    let parsed = url::Url::parse(&val)
        .map_err(|_| VmError::type_error(format!("URL: invalid URL: {val}")))?;
    if let Some(state) = ctx.vm.url_states.get_mut(&id) {
        state.url = parsed;
    }
    rebuild_linked_search_params(ctx.vm, id);
    Ok(JsValue::Undefined)
}

/// `url.protocol = …` — strip a single trailing `:` then call
/// `url::Url::set_scheme`.  WHATWG URL §6.1 silently ignores
/// scheme changes that violate the URL parser invariants
/// (mismatched special / non-special schemes); we mirror that by
/// discarding the `Result<(), ()>` return.
fn native_url_set_protocol(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_url_this(ctx, this, "protocol")?;
    let val = take_url_setter_arg(ctx, args)?;
    let scheme = val.strip_suffix(':').unwrap_or(&val).to_owned();
    if let Some(state) = ctx.vm.url_states.get_mut(&id) {
        let _ = state.url.set_scheme(&scheme);
    }
    Ok(JsValue::Undefined)
}

fn native_url_set_username(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_url_this(ctx, this, "username")?;
    let val = take_url_setter_arg(ctx, args)?;
    if let Some(state) = ctx.vm.url_states.get_mut(&id) {
        let _ = state.url.set_username(&val);
    }
    Ok(JsValue::Undefined)
}

/// `url.password = …` — empty string clears the password
/// (WHATWG URL §6.1 "password setter": `Some(empty)` would emit
/// the trailing `:` separator, so map to `None`).
fn native_url_set_password(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_url_this(ctx, this, "password")?;
    let val = take_url_setter_arg(ctx, args)?;
    if let Some(state) = ctx.vm.url_states.get_mut(&id) {
        let pw = if val.is_empty() {
            None
        } else {
            Some(val.as_str())
        };
        let _ = state.url.set_password(pw);
    }
    Ok(JsValue::Undefined)
}

/// `url.host = …` — WHATWG URL §6.1 "host setter" parses the
/// input as `host[:port]`.  The `url` crate's `set_host` only
/// understands the host portion, so split on the first `:` and
/// stamp the port half through `set_port` when present (omitted
/// inputs leave the existing port untouched, matching WHATWG).
fn native_url_set_host(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_url_this(ctx, this, "host")?;
    let val = take_url_setter_arg(ctx, args)?;
    let (host_part, port_part) = match val.split_once(':') {
        Some((h, p)) => (h.to_owned(), Some(p.to_owned())),
        None => (val, None),
    };
    if let Some(state) = ctx.vm.url_states.get_mut(&id) {
        let _ = state.url.set_host(Some(&host_part));
        if let Some(p) = port_part {
            if let Ok(parsed_port) = p.parse::<u16>() {
                let _ = state.url.set_port(Some(parsed_port));
            }
        }
    }
    Ok(JsValue::Undefined)
}

fn native_url_set_hostname(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_url_this(ctx, this, "hostname")?;
    let val = take_url_setter_arg(ctx, args)?;
    if let Some(state) = ctx.vm.url_states.get_mut(&id) {
        let _ = state.url.set_host(Some(&val));
    }
    Ok(JsValue::Undefined)
}

/// `url.port = …` — empty string clears the port; otherwise parse
/// as `u16` and silently ignore parse failures (matches WHATWG URL
/// §6.1 "port setter" which short-circuits on invalid values).
fn native_url_set_port(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_url_this(ctx, this, "port")?;
    let val = take_url_setter_arg(ctx, args)?;
    if let Some(state) = ctx.vm.url_states.get_mut(&id) {
        if val.is_empty() {
            let _ = state.url.set_port(None);
        } else if let Ok(p) = val.parse::<u16>() {
            let _ = state.url.set_port(Some(p));
        }
    }
    Ok(JsValue::Undefined)
}

fn native_url_set_pathname(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_url_this(ctx, this, "pathname")?;
    let val = take_url_setter_arg(ctx, args)?;
    if let Some(state) = ctx.vm.url_states.get_mut(&id) {
        state.url.set_path(&val);
    }
    Ok(JsValue::Undefined)
}

/// `url.search = …` — strip a single leading `?` then update the
/// query.  Empty value clears the query entirely.  Refreshes the
/// linked `URLSearchParams` entry list so
/// `url.search = "x=1"; url.searchParams.get("x")` returns `"1"`
/// (Phase 4 of slot #9.5).
fn native_url_set_search(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_url_this(ctx, this, "search")?;
    let val = take_url_setter_arg(ctx, args)?;
    let query = val.strip_prefix('?').unwrap_or(&val).to_owned();
    if let Some(state) = ctx.vm.url_states.get_mut(&id) {
        if query.is_empty() {
            state.url.set_query(None);
        } else {
            state.url.set_query(Some(&query));
        }
    }
    rebuild_linked_search_params(ctx.vm, id);
    Ok(JsValue::Undefined)
}

/// `url.hash = …` — strip a single leading `#` then update the
/// fragment.  Empty value clears the fragment entirely.
fn native_url_set_hash(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_url_this(ctx, this, "hash")?;
    let val = take_url_setter_arg(ctx, args)?;
    let frag = val.strip_prefix('#').unwrap_or(&val).to_owned();
    if let Some(state) = ctx.vm.url_states.get_mut(&id) {
        if frag.is_empty() {
            state.url.set_fragment(None);
        } else {
            state.url.set_fragment(Some(&frag));
        }
    }
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Coerce the first positional argument to a `String` via
/// [`super::super::coerce::to_string`].  Used by every URL setter
/// (the WHATWG IDL declares all these IDL attrs `attribute USVString
/// …` so the receiver-side coercion shape matches `to_string`'s
/// abstract semantics).  An owned `String` is returned because
/// each setter then drops the `&strings` borrow before calling
/// `url_states.get_mut` — overlap on `vm.strings` and
/// `vm.url_states` would conflict with the simultaneous `&mut
/// state.url` operation otherwise.
fn take_url_setter_arg(ctx: &mut NativeContext<'_>, args: &[JsValue]) -> Result<String, VmError> {
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = super::super::coerce::to_string(ctx.vm, val)?;
    Ok(ctx.vm.strings.get_utf8(sid))
}

/// `URLSearchParams` mutation hook — re-serialise the entry list
/// and stamp it back onto the parent URL's query (Phase 4 of slot
/// #9.5).  Called at the tail of every `URLSearchParams` mutator
/// (`append` / `delete` / `set` / `sort`) by
/// [`super::url_search_params`].  No-op for standalone
/// `URLSearchParams` instances (those with no `usp_parent_url`
/// entry).
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
/// has no linked `URLSearchParams` (Phase 1 path; Phase 4
/// installs the eager link).
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
fn url_component(
    ctx: &mut NativeContext<'_>,
    id: ObjectId,
    extract: impl FnOnce(&url::Url) -> String,
) -> Result<JsValue, VmError> {
    let s = ctx
        .vm
        .url_states
        .get(&id)
        .map(|state| extract(&state.url))
        .unwrap_or_default();
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
