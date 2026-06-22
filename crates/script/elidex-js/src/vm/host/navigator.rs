//! `navigator` global — the `Navigator` interface (WHATWG HTML §8.10.1
//! *The `Navigator` object*).
//!
//! Surface (mixin → spec section):
//!
//! - **`NavigatorID`** (§8.10.1.1 *Client identification*): `appCodeName`
//!   (constant `"Mozilla"`), `appName` (`"Netscape"`), `product` (`"Gecko"`),
//!   `vendorSub` (`""`) are spec-mandated constants; `appVersion` / `productSub`
//!   / `vendor` are UA/compat-mode-**derived** placeholders awaiting a shell UA
//!   source (see the slot note below); `userAgent` / `platform` are likewise
//!   placeholders. `productSub` / `vendor` / `vendorSub` are `[Exposed=Window]`
//!   — **absent on `WorkerNavigator`** (`host/worker_scope.rs`).
//! - **`NavigatorLanguage`** (§8.10.1.2): `language` / `languages`.
//! - **`NavigatorOnLine`** (§8.10.1.3): `onLine`.
//! - **`NavigatorCookies`** (§8.10.1.5): `cookieEnabled`, a value-derived RO
//!   accessor reading the bound `CookieJar` (A3). `Navigator`-only.
//! - **`NavigatorPlugins`** (§8.10.1.6 *PDF viewing support*): `plugins` /
//!   `mimeTypes` (empty collections — elidex's *PDF viewer supported* is
//!   `false`), `javaEnabled()` (a method returning `false`), `pdfViewerEnabled`
//!   (`false`). `Navigator`-only.
//! - **`NavigatorConcurrentHardware`** (§10.2.7): `hardwareConcurrency`.
//!
//! The intent is to answer feature-detection probes without coupling to a shell
//! layer that has not been designed yet, while keeping each member's WebIDL
//! *shape* spec-faithful (e.g. `javaEnabled` is a callable method, not a bool).
//!
//! Future work (slot `#11-navigator-spec-faithful-surface`):
//!
//! - `userAgent` / `appVersion` / `productSub` / `vendor` / `platform` wired to
//!   a shell-provided UA / compatibility-mode profile (§8.10.1.1) — placeholders
//!   until that source exists (same dependency as the E0/F6 mode work). The
//!   engine does **not** fabricate a real-browser UA string in the interim.
//! - `navigator.clipboard` / `storage` arrive alongside their respective
//!   primitives.
//! - `permissions` / `mediaDevices` are intentionally **not** surfaced so that
//!   hostile detection does not inflate feature-capability signals.

#![cfg(feature = "engine")]

use super::super::shape::PropertyAttrs;
use super::super::value::{JsValue, NativeContext, PropertyKey, PropertyValue, VmError};
use super::super::VmInner;

impl VmInner {
    /// Install `globalThis.navigator` — a plain object with the
    /// `Navigator` fields listed in [`self`] module docs (mostly static
    /// data props; `cookieEnabled` is a value-derived RO accessor).
    ///
    /// Called from `register_globals()` after
    /// `register_event_target_prototype` / `register_window_prototype`
    /// so that the prototype chain (`navigator → Object.prototype`)
    /// is well-formed.
    pub(in crate::vm) fn register_navigator_global(&mut self) {
        // Navigator has no methods — an empty method slice gives us the
        // ordinary plain-object allocation + prototype wiring for free.
        let obj_id = self.create_object_with_methods(&[]);

        // --- String-valued fields ---
        //
        // `available_parallelism` returns `NonZero<usize>`; clamp to
        // `u32` (web standard integer type) and convert losslessly to
        // `f64` via `From` to stay inside clippy's `cast_lossless`
        // rule.  Systems with >u32::MAX cores are not a thing we need
        // to represent.
        let hw = std::thread::available_parallelism()
            .map_or(1u32, |n| u32::try_from(n.get()).unwrap_or(u32::MAX));
        let hardware_concurrency = f64::from(hw);
        let string_fields: &[(&str, &str)] = &[
            // `appCodeName` is a NavigatorID spec constant (HTML §8.10.1.1): the
            // standard mandates the literal "Mozilla" for every UA.
            ("appCodeName", "Mozilla"),
            ("userAgent", "Mozilla/5.0 (compatible; Elidex/0.1)"),
            ("appName", "Netscape"),
            ("appVersion", "5.0 (compatible; Elidex/0.1)"),
            ("product", "Gecko"),
            ("productSub", "20030107"),
            ("vendor", ""),
            ("vendorSub", ""),
            ("platform", std::env::consts::OS),
            ("language", "en-US"),
        ];
        for &(name, value) in string_fields {
            let key = PropertyKey::String(self.strings.intern(name));
            let sid = self.strings.intern(value);
            self.define_shaped_property(
                obj_id,
                key,
                PropertyValue::Data(JsValue::String(sid)),
                PropertyAttrs::WEBIDL_RO,
            );
        }

        // --- `onLine` (NavigatorOnLine §8.10.1.3) + `cookieEnabled` ---
        //
        // `cookieEnabled` (NavigatorCookies, WHATWG HTML §8.10.1.5; see the getter
        // for what it reads) is an RO **accessor** — not a static data prop —
        // because the jar binds *after* navigator install.
        let on_line = PropertyKey::String(self.strings.intern("onLine"));
        self.define_shaped_property(
            obj_id,
            on_line,
            PropertyValue::Data(JsValue::Boolean(true)),
            PropertyAttrs::WEBIDL_RO,
        );
        self.install_ro_accessors(obj_id, NAVIGATOR_RO_ACCESSORS);

        // --- `NavigatorPlugins` mixin (WHATWG HTML §8.10.1.6 *PDF viewing
        // support*) — Navigator-only (NOT exposed on `WorkerNavigator`) ---
        //
        // elidex's *PDF viewer supported* boolean is `false`, so per §8.10.1.6
        // the `plugins` / `mime types` arrays are the empty list and
        // `pdfViewerEnabled` is `false`. `javaEnabled()` is a **method** whose
        // steps "are to return false" — installing it as a bool data property
        // (the historical mistake) made `navigator.javaEnabled()` a TypeError.
        self.install_methods(obj_id, NAVIGATOR_PLUGINS_METHODS);
        let plugins = self.create_empty_navigator_collection(PLUGIN_ARRAY_METHODS);
        let key = PropertyKey::String(self.strings.intern("plugins"));
        self.define_shaped_property(
            obj_id,
            key,
            PropertyValue::Data(JsValue::Object(plugins)),
            PropertyAttrs::WEBIDL_RO,
        );
        let mime_types = self.create_empty_navigator_collection(MIME_TYPE_ARRAY_METHODS);
        let key = PropertyKey::String(self.strings.intern("mimeTypes"));
        self.define_shaped_property(
            obj_id,
            key,
            PropertyValue::Data(JsValue::Object(mime_types)),
            PropertyAttrs::WEBIDL_RO,
        );
        let key = PropertyKey::String(self.strings.intern("pdfViewerEnabled"));
        self.define_shaped_property(
            obj_id,
            key,
            PropertyValue::Data(JsValue::Boolean(false)),
            PropertyAttrs::WEBIDL_RO,
        );

        // --- Number fields ---
        let key = PropertyKey::String(self.strings.intern("hardwareConcurrency"));
        self.define_shaped_property(
            obj_id,
            key,
            PropertyValue::Data(JsValue::Number(hardware_concurrency)),
            PropertyAttrs::WEBIDL_RO,
        );

        // --- `languages` is an Array ---
        //
        // Per spec this is a read-only frozen array; we install the
        // array but skip the freeze pass (no Proxy handlers yet).
        let en_us = self.strings.intern("en-US");
        let en = self.strings.intern("en");
        let lang_arr = self.create_array_object(vec![JsValue::String(en_us), JsValue::String(en)]);
        let key = PropertyKey::String(self.strings.intern("languages"));
        self.define_shaped_property(
            obj_id,
            key,
            PropertyValue::Data(JsValue::Object(lang_arr)),
            PropertyAttrs::WEBIDL_RO,
        );

        // `navigator.serviceWorker` (WHATWG SW §3.4; D-19 PR-3) — the
        // `ServiceWorkerContainer` singleton `register_service_worker_client`
        // built just before this call.  A `[SameObject]` readonly attribute, so
        // a stable readonly data property (the container's state is VM-level).
        if let Some(container) = self.sw_container {
            let key = PropertyKey::String(self.strings.intern("serviceWorker"));
            self.define_shaped_property(
                obj_id,
                key,
                PropertyValue::Data(JsValue::Object(container)),
                PropertyAttrs::WEBIDL_RO,
            );
        }

        let name = self.well_known.navigator;
        self.globals.insert(name, JsValue::Object(obj_id));
    }

    /// Build an **empty** `PluginArray` / `MimeTypeArray` (WHATWG HTML §8.10.1.6):
    /// `length === 0`, with the `methods` table for that interface (`item` /
    /// `namedItem`, plus `refresh` for `PluginArray`). Both arrays are the empty
    /// list because elidex's *PDF viewer supported* boolean is `false`; the objects
    /// exist only so feature-detection probes (`navigator.plugins.length`,
    /// `…item(0)`) see the member shape rather than `undefined`. Installed as
    /// `[SameObject]` readonly attributes — a stable data property is SameObject for
    /// free. (Interface-object branding — `instanceof PluginArray`, receiver
    /// brand-checks — is deferred to slot `#11-navigator-interface-object-branding`.)
    fn create_empty_navigator_collection(
        &mut self,
        methods: &[(&str, super::super::NativeFn)],
    ) -> super::super::value::ObjectId {
        let obj_id = self.create_object_with_methods(methods);
        let key = PropertyKey::String(self.strings.intern("length"));
        self.define_shaped_property(
            obj_id,
            key,
            PropertyValue::Data(JsValue::Number(0.0)),
            PropertyAttrs::WEBIDL_RO,
        );
        obj_id
    }
}

/// `navigator`'s value-derived RO accessors (WebIDL `readonly attribute`s whose
/// value is computed at access time, not fixed at install). Currently just
/// `cookieEnabled`; named const for parity with the sibling host globals
/// (`DOCUMENT_RO_ACCESSORS`, `WINDOW_RO_ACCESSORS`, …) so a future navigator
/// accessor extends the table rather than an inline literal.
const NAVIGATOR_RO_ACCESSORS: &[(&str, super::super::NativeFn)] =
    &[("cookieEnabled", native_navigator_get_cookie_enabled)];

/// `NavigatorPlugins` mixin methods (WHATWG HTML §8.10.1.6) — currently just
/// `javaEnabled()`. `plugins` / `mimeTypes` are `[SameObject]` data attributes
/// (built by [`create_empty_navigator_collection`](VmInner::create_empty_navigator_collection)),
/// not methods; `pdfViewerEnabled` is a plain bool. Navigator-only (the mixin is
/// not included by `WorkerNavigator`).
const NAVIGATOR_PLUGINS_METHODS: &[(&str, super::super::NativeFn)] =
    &[("javaEnabled", native_navigator_java_enabled)];

/// `navigator.javaEnabled()` (WHATWG HTML §8.10.1.6): the spec defines the
/// method's steps "are to return false". It is a **method**, not a bool data
/// property — the latter (the historical shape) made `navigator.javaEnabled()`
/// throw a TypeError.
fn native_navigator_java_enabled(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Boolean(false))
}

/// `PluginArray` method table (WHATWG HTML §8.10.1.6): `item` / `namedItem` +
/// the PluginArray-only `refresh()`. Per-interface natives (vs `MimeTypeArray`'s)
/// so the WebIDL missing-required-argument `TypeError` names the right interface.
const PLUGIN_ARRAY_METHODS: &[(&str, super::super::NativeFn)] = &[
    ("item", native_plugin_array_item),
    ("namedItem", native_plugin_array_named_item),
    ("refresh", native_plugin_array_refresh),
];

/// `MimeTypeArray` method table (WHATWG HTML §8.10.1.6): `item` / `namedItem`
/// (no `refresh` — that is PluginArray-only).
const MIME_TYPE_ARRAY_METHODS: &[(&str, super::super::NativeFn)] = &[
    ("item", native_mime_type_array_item),
    ("namedItem", native_mime_type_array_named_item),
];

fn native_plugin_array_item(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    nav_collection_item(ctx, args, "PluginArray")
}

fn native_mime_type_array_item(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    nav_collection_item(ctx, args, "MimeTypeArray")
}

fn native_plugin_array_named_item(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    nav_collection_named_item(ctx, args, "PluginArray")
}

fn native_mime_type_array_named_item(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    nav_collection_named_item(ctx, args, "MimeTypeArray")
}

/// `PluginArray`/`MimeTypeArray` `item(unsigned long index)` getter (WHATWG HTML
/// §8.10.1.6). elidex's collections are always empty (*PDF viewer supported* is
/// `false`), so the lookup is unconditionally `null` — but the spec-declared
/// **required** `index` is honoured per WebIDL §3.7: a *missing* argument throws
/// `TypeError` (browser/WPT parity, and the VM-wide convention — see `storage.rs`
/// `getItem` / `url_search_params.rs` / `Selection` `arg_offset_required`), and a
/// *present* argument is run through the `unsigned long` conversion (observable
/// side effects, e.g. a throwing `valueOf`) before the empty-list `null`.
fn nav_collection_item(
    ctx: &mut NativeContext<'_>,
    args: &[JsValue],
    interface: &str,
) -> Result<JsValue, VmError> {
    let Some(v) = args.first() else {
        return Err(VmError::type_error(format!(
            "Failed to execute 'item' on '{interface}': 1 argument required, but only 0 present."
        )));
    };
    super::super::coerce::to_int32(ctx.vm, *v)?;
    Ok(JsValue::Null)
}

/// `PluginArray`/`MimeTypeArray` `namedItem(DOMString name)` getter (WHATWG HTML
/// §8.10.1.6). Always `null` for elidex's empty collections, but the required
/// `name` is honoured: a missing argument throws `TypeError`, a present argument
/// is run through the `DOMString` conversion first. Mirrors `nav_collection_item`.
fn nav_collection_named_item(
    ctx: &mut NativeContext<'_>,
    args: &[JsValue],
    interface: &str,
) -> Result<JsValue, VmError> {
    let Some(v) = args.first() else {
        return Err(VmError::type_error(format!(
            "Failed to execute 'namedItem' on '{interface}': 1 argument required, but only 0 present."
        )));
    };
    super::super::coerce::to_string(ctx.vm, *v)?;
    Ok(JsValue::Null)
}

/// `PluginArray.refresh()` (WHATWG HTML §8.10.1.6): a no-op — there are no
/// plugins to re-enumerate (*PDF viewer supported* is `false`).
fn native_plugin_array_refresh(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Undefined)
}

/// `navigator.cookieEnabled` getter (WHATWG HTML §8.10.1.5): returns `true` iff
/// the user agent attempts to handle cookies — i.e. a `CookieJar` is bound to this
/// session (the UA-level cookie capability). It is deliberately **not** narrowed
/// by the current document's origin: a cookie-capable session reports `true` even
/// at host-less `about:blank` / `data:` or before the first HTTP navigation,
/// matching real browsers (which expose `cookieEnabled` as the global cookie
/// setting, not per-document write-eligibility) and the normative §8.10.1.5 text
/// ("the user agent attempts to handle cookies", not "a write at this URL would
/// succeed"). The host-less `document.cookie` write behavior is the separate
/// `#11-cookie-opaque-origin-securityerror` concern, not this signal. Reads shared
/// cross-cutting cookie state (always-compiled in every mode), so it is independent
/// of the `compat-webapi`-gated `document.cookie` accessor: a session that handles
/// cookies reports `true` even where `document.cookie` is hidden.
///
/// Reads the installed `HostData` via [`host_opt`](NativeContext::host_opt), NOT
/// [`host_if_bound`](NativeContext::host_if_bound): the cookie jar is a session
/// resource documented to persist across bind/unbind cycles, and this is a
/// resource-presence check (no DOM operation), so it must not be gated on a
/// current DOM bind — a jar-installed session between bind cycles still handles
/// cookies and must report `true`. (`host_if_bound` is for natives that perform a
/// `host.dom()` operation and need a bound DOM.)
fn native_navigator_get_cookie_enabled(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let enabled = ctx.host_opt().and_then(|hd| hd.cookie_jar()).is_some();
    Ok(JsValue::Boolean(enabled))
}
