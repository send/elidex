//! `CookieStore` interface + `cookieStore` global (Cookie Store API §3 *The
//! CookieStore interface*; the `window.cookieStore` attribute is §6.1 *The
//! Window interface*).
//!
//! `CookieStore` is an `EventTarget` (Cookie Store §3 `: EventTarget`) that is
//! *not* a `Node`, so its prototype chain mirrors `MediaQueryList` / `Window`:
//!
//! ```text
//! cookieStore singleton (ObjectKind::CookieStore, payload-free)
//!   → CookieStore.prototype   (this module)
//!     → EventTarget.prototype  (no Node members)
//!       → Object.prototype
//! ```
//!
//! ## S5-2 design
//!
//! `get` / `getAll` / `set` / `delete` are async (`Promise`-returning); the
//! cookie read/write is synchronous against the shell-owned
//! [`elidex_net::CookieJar`] (the same jar `document.cookie` uses, via
//! `HostData::cookie_jar`), so each method does its synchronous jar op and
//! returns an **already-resolved** Promise (the `Promise.resolve(value)` shape —
//! `create_promise` + `settle_promise`). Marshalling-only: the jar's
//! `cookie_details_for_script` / `set_cookie_from_script` are the canonical
//! script-cookie filter / `Set-Cookie` parser (`HttpOnly` + Secure-on-non-HTTPS
//! suppression live there — Layering mandate: no cookie-policy logic in
//! `vm/host/`).
//!
//! ### `CookieListItem` fields (VM ≥ boa)
//!
//! The current spec's `CookieListItem` dictionary (§3 *The CookieStore
//! interface*) + the "create a CookieListItem" algorithm (§7.1 *Query cookies*)
//! return only `{name, value}`, with the algorithm's note *"One implementation
//! is known to expose information beyond name and value."* boa returned the
//! Chromium superset (`domain` / `path` / `expires` / `secure` / `sameSite`).
//! Per the S5 **VM ≥ boa** cutover invariant (narrowing the surface would
//! regress vs boa — the same rule S5-1 applied), this returns boa's full field
//! set, citing the create-a-CookieListItem note for the beyond-`{name,value}`
//! extension. (The remaining Chromium field `partitioned` boa did not expose, so
//! it is omitted here too — boa parity is the bar.)
//!
//! ## Singleton + GC
//!
//! `window.cookieStore` is a per-window singleton (the `screen` / `navigator`
//! precedent): created once, held as a fixed `globals` entry (SameObject for
//! free + rooted by the `globals` GC root), so — like `VisualViewport` — it is
//! never listener-only-rooted and the S5-3 keepalive hazard does not apply. The
//! brand is payload-free (cookie state lives in the shared jar): GC has nothing
//! to trace or prune.
//!
//! ## Deferred (slots)
//!
//! - **`change` event delivery** — `addEventListener('change', …)` / `onchange`
//!   store listeners but never fire (no cookie-change producer; the same
//!   pre-producer state `MediaQueryList` had). Slot
//!   `#11-s5-2-window-parity-live-producers` (shared with the VisualViewport
//!   event + Screen monitor-fact producers — all S5-2 surfaces awaiting live
//!   shell-producer wiring).
//! - **ServiceWorker exposure** — Cookie Store is `[Exposed=(ServiceWorker,
//!   Window)]` (§6.2); this lands the Window surface only (S5-2 = *minor window
//!   parity*). boa DID expose `cookieStore` in its worker realm
//!   (`elidex-js-boa/.../worker_scope.rs`), so this is a deliberate, **tracked
//!   `VM < boa` delta in the ServiceWorker realm** — NOT a flip-time regression:
//!   the shell still runs boa for the live engine (incl. the SW realm), and the
//!   S5-6 FLIP cuts over the *Window* content path in `BrowserCompat` mode, so
//!   no shipping site regresses at S5-2. The SW-realm cookie context rides the
//!   separate SW-realm cutover; the delta MUST be carried into the
//!   `boa-vm-cutover-surface-parity-audit` so the eventual SW flip honours
//!   `VM ≥ boa`. Slot `#11-cookiestore-serviceworker-scope`.

#![cfg(feature = "engine")]

use std::fmt::Write as _;

use super::super::natives_promise::{create_promise, settle_promise};
use super::super::shape::{self, PropertyAttrs};
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, VmError,
};
use super::super::{coerce, NativeFn, VmInner};

impl VmInner {
    /// Allocate `CookieStore.prototype`, the illegal-constructor interface
    /// object, and the per-window singleton, exposing `window.cookieStore` + the
    /// `CookieStore` global.
    ///
    /// Called from `register_globals()` **after**
    /// [`Self::register_event_target_prototype`] (the prototype chains to
    /// `event_target_prototype`). Window realm only here (§6.1; the §6.2
    /// ServiceWorker exposure is a deferred slot).
    ///
    /// # Panics
    ///
    /// Panics if `event_target_prototype` is `None` — would mean
    /// `register_event_target_prototype` was skipped or run out of order.
    pub(in crate::vm) fn register_cookie_store_global(&mut self) {
        let event_target_proto = self
            .event_target_prototype
            .expect("register_cookie_store_global called before register_event_target_prototype");

        // ---- CookieStore.prototype ----
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(event_target_proto),
            extensible: true,
        });
        self.install_methods(proto_id, COOKIE_STORE_METHODS);
        // `onchange` event-handler IDL attribute (Cookie Store §3 — the member
        // is `[Exposed=Window]`, and this registration is Window-only). Bound to
        // the `'change'` event-type SID over the shared VmObject event-handler
        // backend (`MediaQueryList::onchange` precedent). `addEventListener` /
        // `removeEventListener` / `dispatchEvent` are INHERITED from
        // `EventTarget.prototype`.
        let onchange_sid = self.well_known.onchange;
        let change_sid = self.well_known.change;
        self.install_bound_accessor_pair(
            proto_id,
            onchange_sid,
            super::event_handler_attrs::native_vm_event_handler_get as NativeFn,
            Some(super::event_handler_attrs::native_vm_event_handler_set as NativeFn),
            change_sid,
            PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );

        // ---- CookieStore interface object ----
        // WebIDL: `CookieStore` declares NO constructor — illegal-constructor so
        // `cookieStore instanceof CookieStore` + `CookieStore.prototype` parity
        // hold (the `MediaQueryList` / `VisualViewport` precedent).
        let ctor = self.create_illegal_constructor_function(
            "CookieStore",
            super::super::value::native_illegal_constructor_unreachable,
        );
        self.wire_interface_ctor_prototype(ctor, proto_id);
        let ctor_name = self.strings.intern("CookieStore");
        self.globals.insert(ctor_name, JsValue::Object(ctor));

        // ---- the per-window singleton ----
        let instance = self.alloc_object(Object {
            kind: ObjectKind::CookieStore,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(proto_id),
            extensible: true,
        });
        let attr_name = self.strings.intern("cookieStore");
        self.globals.insert(attr_name, JsValue::Object(instance));
    }
}

const COOKIE_STORE_METHODS: &[(&str, NativeFn)] = &[
    ("get", native_cookie_store_get),
    ("getAll", native_cookie_store_get_all),
    ("set", native_cookie_store_set),
    ("delete", native_cookie_store_delete),
];

// ---------------------------------------------------------------------------
// Brand check + shared jar access
// ---------------------------------------------------------------------------

/// WebIDL branded-receiver gate for `CookieStore.prototype.*`. Throws a
/// TypeError ("illegal invocation") on a non-branded receiver.
fn require_cookie_store_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<(), VmError> {
    if let JsValue::Object(id) = this {
        if matches!(ctx.vm.get_object(id).kind, ObjectKind::CookieStore) {
            return Ok(());
        }
    }
    Err(VmError::type_error(format!(
        "Failed to execute '{method}' on 'CookieStore': illegal invocation"
    )))
}

/// Snapshot the script-visible cookies for the current browsing context, or the
/// empty list when no jar is installed (the cookie-averse fallback
/// `document.cookie` shares). Clones the `Arc<CookieJar>` (cheap atomic bump) so
/// the read can reach back into `ctx.vm.navigation` after the `host_if_bound`
/// borrow ends — the `document.cookie` accessor pattern.
fn current_cookies(ctx: &mut NativeContext<'_>) -> Vec<elidex_net::CookieSnapshot> {
    let jar = ctx.host_if_bound().and_then(|hd| hd.cookie_jar()).cloned();
    jar.map(|jar| jar.cookie_details_for_script(&ctx.vm.navigation.current_url))
        .unwrap_or_default()
}

/// Forward a single `Set-Cookie`-syntax string to the shell-owned jar (no-op
/// when cookie-averse). The jar is the canonical attribute parser (rejecting
/// `HttpOnly` / Secure-over-HTTP) — Layering mandate.
///
/// `cookieStore.set`/`delete` marshal their structured args INTO a `Set-Cookie`
/// string here on purpose: routing both `cookieStore` and `document.cookie`
/// through the **one** `set_cookie_from_script(url, &str)` write chokepoint
/// (One-issue-one-way) keeps all cookie policy/parse in `elidex_net` and avoids
/// a second, divergent structured-set entry point. The string assembly + the
/// `expires`(ms)→`Max-Age`(s) unit conversion in the callers are the marshalling
/// boundary to that single canonical parser, not cookie-policy logic.
fn write_cookie(ctx: &mut NativeContext<'_>, set_cookie: &str) {
    let jar = ctx.host_if_bound().and_then(|hd| hd.cookie_jar()).cloned();
    if let Some(jar) = jar {
        jar.set_cookie_from_script(&ctx.vm.navigation.current_url, set_cookie);
    }
}

/// Wrap a synchronously-computed value in an already-fulfilled `Promise` (the
/// `Promise.resolve(value)` shape). `value` is rooted across `create_promise`'s
/// allocation; the fulfilled promise then holds it, and the promise is returned
/// onto the JS stack (rooted) before any further allocation.
fn resolved_promise(vm: &mut VmInner, value: JsValue) -> JsValue {
    let mut guard = vm.push_temp_root(value);
    let promise = create_promise(&mut guard);
    let _ = settle_promise(&mut guard, promise, false, value);
    JsValue::Object(promise)
}

// ---------------------------------------------------------------------------
// CookieListItem builder (Cookie Store "create a CookieListItem" algorithm,
// §7.1 Query cookies)
// ---------------------------------------------------------------------------

/// Build a `CookieListItem` plain object from a [`elidex_net::CookieSnapshot`].
/// GC-safe: the item is temp-rooted across its own property installs (each
/// `define_shaped_property` may reshape/allocate). Returns boa's field superset
/// (see the module-doc VM ≥ boa note).
#[allow(clippy::cast_precision_loss)]
fn build_cookie_list_item(vm: &mut VmInner, snap: &elidex_net::CookieSnapshot) -> JsValue {
    let proto = vm.object_prototype;
    let item = vm.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });
    let mut g = vm.push_temp_root(JsValue::Object(item));

    // name / value (the spec dictionary members).
    let name_v = JsValue::String(g.strings.intern(&snap.name));
    define_item_prop(&mut g, item, "name", name_v);
    let value_v = JsValue::String(g.strings.intern(&snap.value));
    define_item_prop(&mut g, item, "value", value_v);

    // Chromium-superset members (beyond {name, value}; create-a-CookieListItem
    // note). `domain` is `null` for a host-only cookie (empty domain).
    let domain_v = if snap.domain.is_empty() {
        JsValue::Null
    } else {
        JsValue::String(g.strings.intern(&snap.domain))
    };
    define_item_prop(&mut g, item, "domain", domain_v);
    let path_v = JsValue::String(g.strings.intern(&snap.path));
    define_item_prop(&mut g, item, "path", path_v);

    // `expires` — milliseconds since the epoch, or `null` for a session cookie.
    let expires_v = snap.expires.map_or(JsValue::Null, |exp| {
        let ms = exp
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .map_or(0.0, |d| d.as_millis() as f64);
        JsValue::Number(ms)
    });
    define_item_prop(&mut g, item, "expires", expires_v);

    define_item_prop(&mut g, item, "secure", JsValue::Boolean(snap.secure));
    let same_site_v = JsValue::String(g.strings.intern(&snap.same_site));
    define_item_prop(&mut g, item, "sameSite", same_site_v);

    JsValue::Object(item)
}

/// Install one `CookieListItem` member as a plain data property (`{W, E, C}` —
/// a dictionary-derived object's members are ordinary, matching boa's
/// `Attribute::all()`).
fn define_item_prop(vm: &mut VmInner, item: ObjectId, name: &str, value: JsValue) {
    let key = PropertyKey::String(vm.strings.intern(name));
    vm.define_shaped_property(item, key, PropertyValue::Data(value), PropertyAttrs::DATA);
}

// ---------------------------------------------------------------------------
// Argument coercion (overload resolution helpers)
// ---------------------------------------------------------------------------

/// Resolve the `get` / `getAll` name filter from the `(USVString name)` vs
/// `(CookieStoreGetOptions options)` overloads (Cookie Store §3): a string/number
/// first arg is the `name`; an object is the options dictionary (read `.name`);
/// `undefined`/absent is the empty-options default (no filter → match all).
/// Returns `None` for "no name filter".
fn cookie_name_filter(
    ctx: &mut NativeContext<'_>,
    args: &[JsValue],
) -> Result<Option<String>, VmError> {
    match args.first().copied().unwrap_or(JsValue::Undefined) {
        JsValue::Undefined | JsValue::Null => Ok(None),
        JsValue::Object(id) => {
            let key = PropertyKey::String(ctx.vm.strings.intern("name"));
            match ctx.get_property_value(id, key)? {
                JsValue::Undefined => Ok(None),
                v => Ok(Some(coerce_to_string(ctx, v)?)),
            }
        }
        other => Ok(Some(coerce_to_string(ctx, other)?)),
    }
}

/// `ToString`-coerce a value to an owned `String` (USVString boundary).
fn coerce_to_string(ctx: &mut NativeContext<'_>, value: JsValue) -> Result<String, VmError> {
    let sid = coerce::to_string(ctx.vm, value)?;
    Ok(ctx.vm.strings.get_utf8(sid))
}

/// Read an optional `DOMString` dictionary member via `[[Get]]`, returning
/// `None` when the member is absent / `undefined`.
fn optional_string_member(
    ctx: &mut NativeContext<'_>,
    options: ObjectId,
    name: &str,
) -> Result<Option<String>, VmError> {
    let key = PropertyKey::String(ctx.vm.strings.intern(name));
    match ctx.get_property_value(options, key)? {
        JsValue::Undefined => Ok(None),
        v => Ok(Some(coerce_to_string(ctx, v)?)),
    }
}

/// Read a required `DOMString` dictionary member, throwing a TypeError when
/// absent (boa parity — `options.name` / `options.value` are required on the
/// `CookieInit` set overload).
fn required_string_member(
    ctx: &mut NativeContext<'_>,
    options: ObjectId,
    name: &str,
    op: &str,
) -> Result<String, VmError> {
    optional_string_member(ctx, options, name)?.ok_or_else(|| {
        VmError::type_error(format!(
            "Failed to execute '{op}' on 'CookieStore': required member '{name}' is undefined."
        ))
    })
}

// ---------------------------------------------------------------------------
// get / getAll
// ---------------------------------------------------------------------------

/// `cookieStore.get(name)` / `get(options)` → `Promise<CookieListItem?>` (Cookie
/// Store §3). Resolves with the first matching cookie, or `null`.
fn native_cookie_store_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_cookie_store_this(ctx, this, "get")?;
    let filter = cookie_name_filter(ctx, args)?;
    let cookies = current_cookies(ctx);
    let found = cookies
        .iter()
        .find(|c| filter.as_ref().is_none_or(|n| &c.name == n));
    let value = match found {
        Some(snap) => build_cookie_list_item(ctx.vm, snap),
        None => JsValue::Null,
    };
    Ok(resolved_promise(ctx.vm, value))
}

/// `cookieStore.getAll(name?)` / `getAll(options?)` → `Promise<CookieList>`
/// (Cookie Store §3). Resolves with every matching cookie as an array.
fn native_cookie_store_get_all(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_cookie_store_this(ctx, this, "getAll")?;
    let filter = cookie_name_filter(ctx, args)?;
    let cookies: Vec<elidex_net::CookieSnapshot> = current_cookies(ctx)
        .into_iter()
        .filter(|c| filter.as_ref().is_none_or(|n| &c.name == n))
        .collect();
    // GC-safe array build (roots the array; each item temp-rooted during its
    // own construction inside `build_cookie_list_item`).
    let array = super::observer_common::build_marshalled_array(ctx.vm, &cookies, |vm, snap| {
        build_cookie_list_item(vm, snap)
    });
    Ok(resolved_promise(ctx.vm, array))
}

// ---------------------------------------------------------------------------
// set / delete
// ---------------------------------------------------------------------------

/// `cookieStore.set(name, value)` / `set(CookieInit)` → `Promise<undefined>`
/// (Cookie Store §3). Forwards a `Set-Cookie` string to the jar.
fn native_cookie_store_set(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_cookie_store_this(ctx, this, "set")?;
    let set_cookie = match args.first().copied().unwrap_or(JsValue::Undefined) {
        // `set(CookieInit options)` — {name, value, domain?, path?, expires?,
        // secure?, sameSite?}. `name` / `value` are required (boa parity).
        JsValue::Object(id) => {
            let name = required_string_member(ctx, id, "name", "set")?;
            let value = required_string_member(ctx, id, "value", "set")?;
            let mut s = format!("{name}={value}");
            if let Some(domain) = optional_string_member(ctx, id, "domain")? {
                write!(s, "; Domain={domain}").ok();
            }
            if let Some(path) = optional_string_member(ctx, id, "path")? {
                write!(s, "; Path={path}").ok();
            }
            if let Some(max_age) = optional_expires_max_age(ctx, id)? {
                write!(s, "; Max-Age={max_age}").ok();
            }
            if optional_bool_member(ctx, id, "secure")? {
                s.push_str("; Secure");
            }
            if let Some(same_site) = optional_string_member(ctx, id, "sameSite")? {
                write!(s, "; SameSite={same_site}").ok();
            }
            s
        }
        // `set(name, value)`.
        first => {
            let name = coerce_to_string(ctx, first)?;
            let value = coerce_to_string(ctx, args.get(1).copied().unwrap_or(JsValue::Undefined))?;
            format!("{name}={value}")
        }
    };
    write_cookie(ctx, &set_cookie);
    Ok(resolved_promise(ctx.vm, JsValue::Undefined))
}

/// `cookieStore.delete(name)` / `delete(CookieStoreDeleteOptions)` →
/// `Promise<undefined>` (Cookie Store §3). Expires the cookie via `Max-Age=0`,
/// honouring the `path` / `domain` scope on the options overload (so a
/// path/domain-scoped cookie is actually removed).
fn native_cookie_store_delete(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_cookie_store_this(ctx, this, "delete")?;
    let (name, path, domain) = match args.first().copied().unwrap_or(JsValue::Undefined) {
        JsValue::Object(id) => {
            let name = required_string_member(ctx, id, "name", "delete")?;
            let path = optional_string_member(ctx, id, "path")?;
            let domain = optional_string_member(ctx, id, "domain")?;
            (name, path, domain)
        }
        first => (coerce_to_string(ctx, first)?, None, None),
    };
    let mut s = format!("{name}=; Max-Age=0");
    if let Some(path) = path {
        write!(s, "; Path={path}").ok();
    }
    if let Some(domain) = domain {
        write!(s, "; Domain={domain}").ok();
    }
    write_cookie(ctx, &s);
    Ok(resolved_promise(ctx.vm, JsValue::Undefined))
}

/// Read the `CookieInit.expires` member (milliseconds since the epoch) and
/// convert it to a `Max-Age` in seconds from now (boa parity). `None` when the
/// member is absent / `undefined`.
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
fn optional_expires_max_age(
    ctx: &mut NativeContext<'_>,
    options: ObjectId,
) -> Result<Option<u64>, VmError> {
    let key = PropertyKey::String(ctx.vm.strings.intern("expires"));
    match ctx.get_property_value(options, key)? {
        JsValue::Undefined | JsValue::Null => Ok(None),
        v => {
            let exp_ms = coerce::to_number(ctx.vm, v)?;
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .map_or(0.0, |d| d.as_millis() as f64);
            let max_age = ((exp_ms - now_ms) / 1000.0).max(0.0) as u64;
            Ok(Some(max_age))
        }
    }
}

/// Read an optional `boolean` dictionary member via `[[Get]]`, defaulting to
/// `false` when absent / `undefined` (the `CookieInit.secure` default).
fn optional_bool_member(
    ctx: &mut NativeContext<'_>,
    options: ObjectId,
    name: &str,
) -> Result<bool, VmError> {
    let key = PropertyKey::String(ctx.vm.strings.intern(name));
    match ctx.get_property_value(options, key)? {
        JsValue::Undefined => Ok(false),
        v => Ok(coerce::to_boolean(ctx.vm, v)),
    }
}
