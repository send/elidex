//! `navigator` global — the `Navigator` interface (WHATWG HTML §8.1.5).
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
use super::super::value::{JsValue, PropertyKey, PropertyValue};
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
                PropertyAttrs::BUILTIN,
            );
        }

        // --- Boolean-valued fields ---
        //
        // `cookieEnabled` is deliberately `false` until Phase 3: the VM
        // has no cookie jar yet, and reporting `true` would lead
        // scripts to call `document.cookie` setters whose writes we
        // silently drop (a worse failure mode than "disabled").
        let bool_fields: &[(&str, bool)] = &[
            ("onLine", true),
            ("cookieEnabled", false),
            ("javaEnabled", false),
        ];
        for &(name, value) in bool_fields {
            let key = PropertyKey::String(self.strings.intern(name));
            self.define_shaped_property(
                obj_id,
                key,
                PropertyValue::Data(JsValue::Boolean(value)),
                PropertyAttrs::BUILTIN,
            );
        }

        // --- Number fields ---
        let key = PropertyKey::String(self.strings.intern("hardwareConcurrency"));
        self.define_shaped_property(
            obj_id,
            key,
            PropertyValue::Data(JsValue::Number(hardware_concurrency)),
            PropertyAttrs::BUILTIN,
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
            PropertyAttrs::BUILTIN,
        );

        let name = self.well_known.navigator;
        self.globals.insert(name, JsValue::Object(obj_id));
    }
}
