//! Registration of coroutine-family globals (Promise, Generator).
//!
//! Extracted from `globals.rs` to keep that file under the 1000-line
//! project convention.  These share the common pattern of introducing a
//! prototype object populated with native methods and (for Promise) a
//! constructable entry point.

use super::shape::PropertyAttrs;
use super::value::{JsValue, PropertyKey, PropertyValue};
use super::{NativeFn, VmInner};

impl VmInner {
    /// Promise constructor + prototype (ES2020 §25.6).
    ///
    /// Registers `Promise` as a constructable native function with static
    /// methods (`resolve`, `reject`, `all`, `allSettled`, `race`, `any`)
    /// and wires up `Promise.prototype.{then, catch, finally}`.  Promise
    /// instances are allocated with their prototype set by `do_new` via
    /// the `Promise.prototype` property lookup; the native constructor
    /// repurposes that pre-allocated Ordinary instance into
    /// `ObjectKind::Promise` in place.
    pub(super) fn register_promise_global(&mut self) {
        use super::natives_promise::{
            native_promise_constructor, native_promise_prototype_catch,
            native_promise_prototype_then, native_promise_reject, native_promise_resolve,
        };
        use super::natives_promise_combinator::{
            native_promise_all, native_promise_all_settled, native_promise_any,
            native_promise_prototype_finally, native_promise_race,
        };

        // Promise.prototype with `.then` / `.catch` / `.finally`.
        let proto_id = self.create_object_with_methods(&[
            ("then", native_promise_prototype_then as NativeFn),
            ("catch", native_promise_prototype_catch),
            ("finally", native_promise_prototype_finally),
        ]);
        self.promise_prototype = Some(proto_id);

        // Promise constructor (constructable via `new`).
        let ctor_id = self.create_constructable_function("Promise", native_promise_constructor);
        let proto_key = PropertyKey::String(self.well_known.prototype);
        self.define_shaped_property(
            ctor_id,
            proto_key,
            PropertyValue::Data(JsValue::Object(proto_id)),
            PropertyAttrs::BUILTIN,
        );
        // Promise.prototype.constructor = Promise
        let ctor_key = PropertyKey::String(self.well_known.constructor);
        self.define_shaped_property(
            proto_id,
            ctor_key,
            PropertyValue::Data(JsValue::Object(ctor_id)),
            PropertyAttrs::METHOD,
        );

        // Static methods: resolve / reject / all / allSettled / race / any.
        for (name, func) in [
            ("resolve", native_promise_resolve as NativeFn),
            ("reject", native_promise_reject as NativeFn),
            ("all", native_promise_all as NativeFn),
            ("allSettled", native_promise_all_settled as NativeFn),
            ("race", native_promise_race as NativeFn),
            ("any", native_promise_any as NativeFn),
        ] {
            let fn_id = self.create_native_function(name, func);
            let key = PropertyKey::String(self.strings.intern(name));
            self.define_shaped_property(
                ctor_id,
                key,
                PropertyValue::Data(JsValue::Object(fn_id)),
                PropertyAttrs::METHOD,
            );
        }

        let name = self.strings.intern("Promise");
        self.globals.insert(name, JsValue::Object(ctor_id));
    }

    /// Generator.prototype (§25.4.1) — shared prototype for generator
    /// iterator objects.  Holds `next` / `return` / `throw` and the
    /// `[Symbol.iterator]` that returns the generator itself.
    pub(super) fn register_generator_prototype(&mut self) {
        use super::natives_generator::{
            native_generator_iterator_self, native_generator_next, native_generator_return,
            native_generator_throw,
        };

        let proto_id = self.create_object_with_methods(&[
            ("next", native_generator_next as NativeFn),
            ("return", native_generator_return),
            ("throw", native_generator_throw),
        ]);
        // `[Symbol.iterator]` returns the generator itself (spec §25.4.1.5).
        let iter_fn =
            self.create_native_function("[Symbol.iterator]", native_generator_iterator_self);
        let sym_iter_key = PropertyKey::Symbol(self.well_known_symbols.iterator);
        self.define_shaped_property(
            proto_id,
            sym_iter_key,
            PropertyValue::Data(JsValue::Object(iter_fn)),
            PropertyAttrs::METHOD,
        );
        self.generator_prototype = Some(proto_id);
    }
}
