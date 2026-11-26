//! `performance` global — subset of the W3C High-Resolution Time
//! interface (HR-Time §7).
//!
//! Implemented:
//! - `performance.now()` — monotonic ms since `Vm::new`, real value.
//!
//! Deferred — surface installed for feature-detection compatibility,
//! but the real semantics ship later:
//! - `performance.timeOrigin` — installed as a stub constant `0.0`
//!   for feature detection. HR-Time §7.2 requires the epoch-relative time
//!   origin of the relevant global. The shell’s clock mapping is deferred.
//!
//! Deferred — surface not installed (require a `PerformanceObserver`
//! surface that is not worth building before the consuming code
//! paths land): `performance.mark`, `performance.measure`,
//! `getEntriesByType`, `getEntriesByName`, `clearMarks`,
//! `clearMeasures`.
//!
//! The clock is [`VmInner::start_instant`], which is *also* what
//! `Event.timeStamp` uses (DOM §2.5).  Sharing a single
//! `std::time::Instant` guarantees `performance.now()` and
//! `event.timeStamp` are directly comparable — the HR-Time spec
//! requires both to share a time origin.

#![cfg(feature = "engine")]

use super::super::shape::PropertyAttrs;
use super::super::value::{JsValue, NativeContext, PropertyKey, PropertyValue, VmError};
use super::super::VmInner;

/// `performance.now()` — returns the monotonic elapsed milliseconds
/// since `Vm::new`, with sub-millisecond precision.
pub(super) fn native_performance_now(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let ms = ctx.vm.start_instant.elapsed().as_secs_f64() * 1000.0;
    Ok(JsValue::Number(ms))
}

impl VmInner {
    /// Install `globalThis.performance = { now: <native> }`.
    ///
    /// Called from `register_globals()` after the prototype chain is
    /// finalised so the `performance` plain object inherits
    /// `Object.prototype` correctly.
    pub(in crate::vm) fn register_performance_global(&mut self) {
        let obj_id = self.create_object_with_methods(&[("now", native_performance_now)]);

        // See the module documentation for the timeOrigin placeholder.
        let key = PropertyKey::String(self.strings.intern("timeOrigin"));
        self.define_shaped_property(
            obj_id,
            key,
            PropertyValue::Data(JsValue::Number(0.0)),
            PropertyAttrs::WEBIDL_RO,
        );

        let name = self.well_known.performance;
        self.globals.insert(name, JsValue::Object(obj_id));
    }
}
