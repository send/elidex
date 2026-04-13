//! Registration of Error-family global constructors + prototypes.
//!
//! Split from `globals.rs` to keep that file under the project's
//! 1000-line convention; all Error subclasses (`Error`, `TypeError`, …,
//! `AggregateError`) and their shared / own prototypes live here.

use super::natives::{
    native_aggregate_error_constructor, native_error_constructor, native_range_error_constructor,
    native_reference_error_constructor, native_syntax_error_constructor,
    native_type_error_constructor, native_uri_error_constructor,
};
use super::shape::{self, PropertyAttrs};
use super::value::{JsValue, Object, ObjectKind, PropertyKey, PropertyStorage, PropertyValue};
use super::{NativeFn, VmInner};

impl VmInner {
    /// Allocate `Error.prototype` + all error-subclass constructors
    /// (`Error`, `TypeError`, `ReferenceError`, `RangeError`,
    /// `SyntaxError`, `URIError`, `AggregateError`) and publish them on
    /// `globalThis`.  Stores `error_prototype` and
    /// `aggregate_error_prototype` on `VmInner` so later registrations
    /// (Promise.any, AggregateError constructor) can chain to them.
    pub(super) fn register_error_constructors(&mut self) {
        // §19.5.3 Error.prototype — shared by Error and the built-in
        // error subclasses (TypeError, RangeError, …) in elidex.  Not
        // fully spec-compliant (each subclass should have its own
        // prototype chained to Error.prototype), but sufficient for
        // `String(new TypeError(...))` to produce "TypeError: msg" via
        // inherited .toString.  AggregateError *does* get its own
        // prototype chained to Error.prototype — see below — because
        // its signature (`(errors, message)`) differs enough from the
        // shared `error_ctor_impl` path that tests routinely check
        // `instanceof AggregateError` distinctly.
        let error_proto = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: self.object_prototype,
            extensible: true,
        });
        let to_string_fn =
            self.create_native_function("toString", super::natives::native_error_to_string);
        let to_string_key = PropertyKey::String(self.strings.intern("toString"));
        self.define_shaped_property(
            error_proto,
            to_string_key,
            PropertyValue::Data(JsValue::Object(to_string_fn)),
            PropertyAttrs::METHOD,
        );
        let default_name_key = PropertyKey::String(self.well_known.name);
        let default_name_val = JsValue::String(self.strings.intern("Error"));
        self.define_shaped_property(
            error_proto,
            default_name_key,
            PropertyValue::Data(default_name_val),
            PropertyAttrs::METHOD,
        );
        let default_msg_key = PropertyKey::String(self.well_known.message);
        self.define_shaped_property(
            error_proto,
            default_msg_key,
            PropertyValue::Data(JsValue::String(self.well_known.empty)),
            PropertyAttrs::METHOD,
        );
        self.error_prototype = Some(error_proto);

        let ctors: &[(&str, NativeFn)] = &[
            ("Error", native_error_constructor),
            ("TypeError", native_type_error_constructor),
            ("ReferenceError", native_reference_error_constructor),
            ("RangeError", native_range_error_constructor),
            ("SyntaxError", native_syntax_error_constructor),
            ("URIError", native_uri_error_constructor),
        ];
        let proto_key = PropertyKey::String(self.well_known.prototype);
        for &(name, func) in ctors {
            let fn_id = self.create_constructable_function(name, func);
            // Set ctor.prototype = error_proto so `new Error(...)` instances
            // inherit from Error.prototype (via do_new's lookup chain).
            self.define_shaped_property(
                fn_id,
                proto_key,
                PropertyValue::Data(JsValue::Object(error_proto)),
                PropertyAttrs::BUILTIN,
            );
            let name_id = self.strings.intern(name);
            self.globals.insert(name_id, JsValue::Object(fn_id));
        }

        // AggregateError.prototype (§20.5.7.3) — its own prototype
        // chained to Error.prototype so `instanceof Error` is true for
        // AggregateError instances.  Own `.name` = "AggregateError",
        // `.message` = "".
        let agg_proto = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(error_proto),
            extensible: true,
        });
        let agg_name_val = JsValue::String(self.well_known.aggregate_error);
        self.define_shaped_property(
            agg_proto,
            default_name_key,
            PropertyValue::Data(agg_name_val),
            PropertyAttrs::METHOD,
        );
        self.define_shaped_property(
            agg_proto,
            default_msg_key,
            PropertyValue::Data(JsValue::String(self.well_known.empty)),
            PropertyAttrs::METHOD,
        );
        self.aggregate_error_prototype = Some(agg_proto);

        let agg_ctor = self
            .create_constructable_function("AggregateError", native_aggregate_error_constructor);
        self.define_shaped_property(
            agg_ctor,
            proto_key,
            PropertyValue::Data(JsValue::Object(agg_proto)),
            PropertyAttrs::BUILTIN,
        );
        let agg_ctor_key = PropertyKey::String(self.well_known.constructor);
        self.define_shaped_property(
            agg_proto,
            agg_ctor_key,
            PropertyValue::Data(JsValue::Object(agg_ctor)),
            PropertyAttrs::BUILTIN,
        );
        self.globals
            .insert(self.well_known.aggregate_error, JsValue::Object(agg_ctor));
    }
}
