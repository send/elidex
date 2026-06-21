//! `navigator` global — the `Navigator` interface (WHATWG HTML §8.10.1).
//!
//! Phase 2 scope: all fields are static constants — no privacy /
//! language-negotiation / real UA string derivation yet.  The intent is
//! to answer feature-detection probes like `navigator.userAgent` or
//! `navigator.hardwareConcurrency` without coupling to a shell layer
//! that has not been designed yet.
//!
//! Future work (PR5+):
//!
//! - `userAgent` / `language` read from a shell-provided profile.
//! - `navigator.serviceWorker`, `navigator.clipboard`, `storage`, etc.
//!   arrive alongside their respective primitives.
//! - `permissions` / `mediaDevices` are intentionally **not** surfaced
//!   so that hostile detection does not inflate feature-capability
//!   signals.

#![cfg(feature = "engine")]

use super::super::shape::PropertyAttrs;
use super::super::value::{JsValue, NativeContext, PropertyKey, PropertyValue, VmError};
use super::super::VmInner;

impl VmInner {
    /// Install `globalThis.navigator` — a plain object with the
    /// static `Navigator` fields listed in [`self`] module docs.
    ///
    /// Called from `register_globals()` after
    /// `register_event_target_prototype` / `register_window_prototype`
    /// so that the prototype chain (`navigator → Object.prototype`)
    /// is well-formed.
    pub(in crate::vm) fn register_navigator_global(&mut self) {
        // Navigator has no methods in Phase 2, only static fields — an
        // empty method slice gives us the ordinary plain-object
        // allocation + prototype wiring for free.
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

        // --- Boolean-valued fields ---
        //
        // `cookieEnabled` is NOT here — it is value-derived (a getter
        // reading the bound `CookieJar`), installed below. The other
        // booleans are static.
        let bool_fields: &[(&str, bool)] = &[("onLine", true), ("javaEnabled", false)];
        for &(name, value) in bool_fields {
            let key = PropertyKey::String(self.strings.intern(name));
            self.define_shaped_property(
                obj_id,
                key,
                PropertyValue::Data(JsValue::Boolean(value)),
                PropertyAttrs::WEBIDL_RO,
            );
        }

        // `cookieEnabled` (WHATWG HTML §8.10.1.5, NavigatorCookies) — a
        // value-derived RO accessor: `true` iff the UA handles cookies,
        // i.e. a `CookieJar` is bound to this session. This is the *cookie
        // handling* signal, independent of whether the `document.cookie`
        // script accessor is exposed: a `BrowserCore` / `App` session
        // processes HTTP cookies (jar bound → `true`) even though A3 hides
        // `document.cookie`. So `cookieEnabled === true` does NOT imply the
        // `document.cookie` write path succeeds (there the accessor is gated
        // off; only the HTTP/jar path persists). An accessor — not a static
        // data prop — because the jar binds *after* navigator install.
        self.install_ro_accessors(
            obj_id,
            &[("cookieEnabled", native_navigator_get_cookie_enabled)],
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
}

/// `navigator.cookieEnabled` getter (WHATWG HTML §8.10.1.5). Returns `true` iff a
/// `CookieJar` is bound to this session — the UA "handles cookies" signal. Reads
/// shared cross-cutting cookie state (always-compiled in every mode), so it is
/// independent of the `compat-webapi`-gated `document.cookie` accessor: a session
/// with HTTP cookies reports `true` even where `document.cookie` is hidden.
fn native_navigator_get_cookie_enabled(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let enabled = ctx.host_if_bound().and_then(|hd| hd.cookie_jar()).is_some();
    Ok(JsValue::Boolean(enabled))
}
