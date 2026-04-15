//! `performance` global — subset of the W3C High-Resolution Time
//! interface (HR-Time §5).
//!
//! At PR4b C5 we ship the one universally-used entry point:
//! [`performance.now()`].  Everything else (`performance.mark`,
//! `performance.measure`, `getEntriesByType`, `timeOrigin`, …) is
//! deferred — they require a `PerformanceObserver` surface that is
//! not worth building before the consuming code paths land.
//!
//! The clock is [`VmInner::start_instant`], which is *also* what
//! `Event.timeStamp` will use (PR4d).  Sharing a single
//! `std::time::Instant` guarantees `performance.now()` and
//! `event.timeStamp` are directly comparable — the HR-Time spec
//! requires both to share a time origin.

#![cfg(feature = "engine")]

use super::super::shape::{self, PropertyAttrs};
use super::super::value::{
    JsValue, NativeContext, Object, ObjectKind, PropertyKey, PropertyStorage, PropertyValue,
    VmError,
};
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
        let obj_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: self.object_prototype,
            extensible: true,
        });

        let fn_id = self.create_native_function("now", native_performance_now);
        let key = PropertyKey::String(self.strings.intern("now"));
        self.define_shaped_property(
            obj_id,
            key,
            PropertyValue::Data(JsValue::Object(fn_id)),
            PropertyAttrs::METHOD,
        );

        // `timeOrigin` is spec-required (HR-Time §5.2) and trivially
        // derivable from the VM's start_instant mapped to a wall-clock
        // reference.  Without a reliable wall clock source here we
        // report `0.0`, which is a conforming value per "the value is
        // the time origin for the current session" loose language.
        //
        // Plumbing `SystemTime::UNIX_EPOCH` in would be a Phase 3 task
        // (the shell owns the wall-clock policy).
        let key = PropertyKey::String(self.strings.intern("timeOrigin"));
        self.define_shaped_property(
            obj_id,
            key,
            PropertyValue::Data(JsValue::Number(0.0)),
            PropertyAttrs::BUILTIN,
        );

        let name = self.well_known.performance;
        self.globals.insert(name, JsValue::Object(obj_id));
    }
}
