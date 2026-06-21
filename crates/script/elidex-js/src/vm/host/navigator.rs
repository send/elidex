//! `navigator` global — the `Navigator` interface (WHATWG HTML §8.10.1).
//!
//! Phase 2 scope: fields are mostly static constants — no privacy /
//! language-negotiation / real UA string derivation yet.  The intent is
//! to answer feature-detection probes like `navigator.userAgent` or
//! `navigator.hardwareConcurrency` without coupling to a shell layer
//! that has not been designed yet.  The one value-derived member is
//! `cookieEnabled` (an RO accessor reading the bound `CookieJar`, A3).
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

        // --- Boolean + cookie fields, in WebIDL declaration order ---
        //
        // `cookieEnabled` (WHATWG HTML §8.10.1.5; see the getter for what it
        // reads) is an RO **accessor** — not a static data prop — because the jar
        // binds *after* navigator install. It is installed between `onLine` and
        // `javaEnabled`, its historical bool-field slot, so own-property
        // enumeration order is unchanged.
        let on_line = PropertyKey::String(self.strings.intern("onLine"));
        self.define_shaped_property(
            obj_id,
            on_line,
            PropertyValue::Data(JsValue::Boolean(true)),
            PropertyAttrs::WEBIDL_RO,
        );
        self.install_ro_accessors(obj_id, NAVIGATOR_RO_ACCESSORS);
        let java_enabled = PropertyKey::String(self.strings.intern("javaEnabled"));
        self.define_shaped_property(
            obj_id,
            java_enabled,
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
}

/// `navigator`'s value-derived RO accessors (WebIDL `readonly attribute`s whose
/// value is computed at access time, not fixed at install). Currently just
/// `cookieEnabled`; named const for parity with the sibling host globals
/// (`DOCUMENT_RO_ACCESSORS`, `WINDOW_RO_ACCESSORS`, …) so a future navigator
/// accessor extends the table rather than an inline literal.
const NAVIGATOR_RO_ACCESSORS: &[(&str, super::super::NativeFn)] =
    &[("cookieEnabled", native_navigator_get_cookie_enabled)];

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
