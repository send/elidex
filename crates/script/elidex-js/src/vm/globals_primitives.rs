//! Per-primitive prototype + constructor registrations.
//!
//! Extracted from `vm/globals.rs` to keep that file under the
//! 1000-line convention.  Covers `String` / `Number` / `Boolean` /
//! `RegExp` / `Symbol` / `BigInt` prototypes and globals, plus the
//! `console` namespace.  `Object.prototype` / `Array.prototype` /
//! `Function.prototype` and the Error constructor family stay in
//! `globals.rs` because they bootstrap the prototype chain itself
//! (Object.prototype must exist before any other prototype's
//! `create_object_with_methods` call).

use super::natives::{
    native_console_error, native_console_log, native_console_warn, native_string_char_at,
    native_string_char_code_at, native_string_ends_with, native_string_includes,
    native_string_index_of, native_string_iterator, native_string_match, native_string_replace,
    native_string_search, native_string_slice, native_string_split, native_string_starts_with,
    native_string_substring, native_string_to_lower_case, native_string_to_upper_case,
    native_string_trim, native_symbol_constructor, native_symbol_for, native_symbol_key_for,
    native_symbol_prototype_to_string,
};
use super::natives_boolean::{native_boolean_to_string, native_boolean_value_of};
use super::natives_number::{
    native_number_is_finite, native_number_is_integer, native_number_is_nan,
    native_number_is_safe_integer, native_number_to_exponential, native_number_to_fixed,
    native_number_to_precision, native_number_to_string, native_number_value_of,
};
use super::natives_regexp::{native_regexp_exec, native_regexp_test, native_regexp_to_string};
use super::natives_string::native_string_value_of;
use super::natives_string_ext::{
    native_string_code_point_at, native_string_concat, native_string_from_char_code,
    native_string_from_code_point, native_string_last_index_of, native_string_pad_end,
    native_string_pad_start, native_string_repeat, native_string_replace_all,
    native_string_trim_end, native_string_trim_start,
};
use super::shape::{self, PropertyAttrs};
use super::value::{JsValue, Object, ObjectKind, PropertyKey, PropertyStorage, PropertyValue};
use super::{NativeFn, VmInner};

impl VmInner {
    pub(super) fn register_string_prototype(&mut self) {
        let proto_id = self.create_object_with_methods(&[
            ("charAt", native_string_char_at),
            ("charCodeAt", native_string_char_code_at),
            ("codePointAt", native_string_code_point_at),
            ("indexOf", native_string_index_of),
            ("lastIndexOf", native_string_last_index_of),
            ("includes", native_string_includes),
            ("slice", native_string_slice),
            ("substring", native_string_substring),
            ("toLowerCase", native_string_to_lower_case),
            ("toUpperCase", native_string_to_upper_case),
            ("trim", native_string_trim),
            ("trimStart", native_string_trim_start),
            ("trimEnd", native_string_trim_end),
            ("trimLeft", native_string_trim_start),
            ("trimRight", native_string_trim_end),
            ("repeat", native_string_repeat),
            ("padStart", native_string_pad_start),
            ("padEnd", native_string_pad_end),
            ("concat", native_string_concat),
            ("replaceAll", native_string_replace_all),
            ("split", native_string_split),
            ("startsWith", native_string_starts_with),
            ("endsWith", native_string_ends_with),
            ("replace", native_string_replace),
            ("match", native_string_match),
            ("search", native_string_search),
            ("valueOf", native_string_value_of),
            // §21.1.3.25: String.prototype.toString is identical to valueOf.
            ("toString", native_string_value_of),
        ]);
        // String.prototype[Symbol.iterator] = native_string_iterator
        let iter_fn_id = self.create_native_function("[Symbol.iterator]", native_string_iterator);
        let sym_iter_key = PropertyKey::Symbol(self.well_known_symbols.iterator);
        self.define_shaped_property(
            proto_id,
            sym_iter_key,
            PropertyValue::Data(JsValue::Object(iter_fn_id)),
            PropertyAttrs::METHOD,
        );
        self.string_prototype = Some(proto_id);

        // String constructor — constructable NativeFunction so `new String()` works.
        let ctor_id = self.create_constructable_function(
            "String",
            super::natives_string::native_string_constructor,
        );
        let proto_key = PropertyKey::String(self.well_known.prototype);
        self.define_shaped_property(
            ctor_id,
            proto_key,
            PropertyValue::Data(JsValue::Object(proto_id)),
            PropertyAttrs::BUILTIN,
        );
        // String.prototype.constructor = String
        let ctor_key = PropertyKey::String(self.well_known.constructor);
        self.define_shaped_property(
            proto_id,
            ctor_key,
            PropertyValue::Data(JsValue::Object(ctor_id)),
            PropertyAttrs::METHOD,
        );
        let ctor_name = self.strings.intern("String");
        self.globals.insert(ctor_name, JsValue::Object(ctor_id));

        // String.fromCharCode / String.fromCodePoint — static methods on constructor
        let from_char_code_fn =
            self.create_native_function("fromCharCode", native_string_from_char_code);
        let key = PropertyKey::String(self.strings.intern("fromCharCode"));
        self.define_shaped_property(
            ctor_id,
            key,
            PropertyValue::Data(JsValue::Object(from_char_code_fn)),
            PropertyAttrs::METHOD,
        );
        let from_code_point_fn =
            self.create_native_function("fromCodePoint", native_string_from_code_point);
        let key = PropertyKey::String(self.strings.intern("fromCodePoint"));
        self.define_shaped_property(
            ctor_id,
            key,
            PropertyValue::Data(JsValue::Object(from_code_point_fn)),
            PropertyAttrs::METHOD,
        );
    }

    pub(super) fn register_number_prototype(&mut self) {
        let proto_id = self.create_object_with_methods(&[
            ("toString", native_number_to_string),
            ("valueOf", native_number_value_of),
            ("toFixed", native_number_to_fixed),
            ("toExponential", native_number_to_exponential),
            ("toPrecision", native_number_to_precision),
        ]);
        self.number_prototype = Some(proto_id);
        self.register_constructor_global("Number", proto_id);

        // Add static methods and constants to the Number constructor object.
        let ctor_name = self.strings.intern("Number");
        let Some(&JsValue::Object(ctor_id)) = self.globals.get(&ctor_name) else {
            return;
        };

        // Static methods
        let statics: &[(&str, NativeFn)] = &[
            ("isFinite", native_number_is_finite),
            ("isInteger", native_number_is_integer),
            ("isNaN", native_number_is_nan),
            ("isSafeInteger", native_number_is_safe_integer),
        ];
        for &(name, func) in statics {
            let fn_id = self.create_native_function(name, func);
            let key = PropertyKey::String(self.strings.intern(name));
            self.define_shaped_property(
                ctor_id,
                key,
                PropertyValue::Data(JsValue::Object(fn_id)),
                PropertyAttrs::METHOD,
            );
        }

        // Constants
        let consts: &[(&str, f64)] = &[
            ("POSITIVE_INFINITY", f64::INFINITY),
            ("NEGATIVE_INFINITY", f64::NEG_INFINITY),
            ("MAX_SAFE_INTEGER", 9_007_199_254_740_991.0),
            ("MIN_SAFE_INTEGER", -9_007_199_254_740_991.0),
            ("MAX_VALUE", f64::MAX),
            ("MIN_VALUE", f64::MIN_POSITIVE),
            ("EPSILON", f64::EPSILON),
        ];
        for &(name, value) in consts {
            let key = PropertyKey::String(self.strings.intern(name));
            self.define_shaped_property(
                ctor_id,
                key,
                PropertyValue::Data(JsValue::Number(value)),
                PropertyAttrs::BUILTIN,
            );
        }

        // Number.NaN
        let nan_key = PropertyKey::String(self.strings.intern("NaN"));
        self.define_shaped_property(
            ctor_id,
            nan_key,
            PropertyValue::Data(JsValue::Number(f64::NAN)),
            PropertyAttrs::BUILTIN,
        );
    }

    pub(super) fn register_boolean_prototype(&mut self) {
        let proto_id = self.create_object_with_methods(&[
            ("toString", native_boolean_to_string),
            ("valueOf", native_boolean_value_of),
        ]);
        self.boolean_prototype = Some(proto_id);
        self.register_constructor_global("Boolean", proto_id);
    }

    pub(super) fn register_regexp_prototype(&mut self) {
        let proto_id = self.create_object_with_methods(&[
            ("test", native_regexp_test),
            ("exec", native_regexp_exec),
            ("toString", native_regexp_to_string),
        ]);
        self.regexp_prototype = Some(proto_id);
    }

    pub(super) fn register_symbol_prototype(&mut self) {
        let proto_id =
            self.create_object_with_methods(&[("toString", native_symbol_prototype_to_string)]);
        self.symbol_prototype = Some(proto_id);
    }

    pub(super) fn register_symbol_global(&mut self) {
        // The Symbol "constructor" is callable but not constructable.
        // Register it as a native function, then attach static methods and
        // well-known symbol properties.
        let sym_fn_id = self.create_native_function("Symbol", native_symbol_constructor);
        let name_id = self.strings.intern("Symbol");
        self.globals.insert(name_id, JsValue::Object(sym_fn_id));

        // Symbol.for
        let for_fn = self.create_native_function("for", native_symbol_for);
        let for_key = PropertyKey::String(self.strings.intern("for"));
        self.define_shaped_property(
            sym_fn_id,
            for_key,
            PropertyValue::Data(JsValue::Object(for_fn)),
            PropertyAttrs::METHOD,
        );

        // Symbol.keyFor
        let key_for_fn = self.create_native_function("keyFor", native_symbol_key_for);
        let key_for_key = PropertyKey::String(self.strings.intern("keyFor"));
        self.define_shaped_property(
            sym_fn_id,
            key_for_key,
            PropertyValue::Data(JsValue::Object(key_for_fn)),
            PropertyAttrs::METHOD,
        );

        // Well-known symbols as properties
        let wk = &self.well_known_symbols;
        let well_known_props = [
            ("iterator", wk.iterator),
            ("asyncIterator", wk.async_iterator),
            ("hasInstance", wk.has_instance),
            ("toPrimitive", wk.to_primitive),
            ("toStringTag", wk.to_string_tag),
            ("species", wk.species),
            ("isConcatSpreadable", wk.is_concat_spreadable),
        ];
        for (prop_name, sid) in well_known_props {
            let key = PropertyKey::String(self.strings.intern(prop_name));
            self.define_shaped_property(
                sym_fn_id,
                key,
                PropertyValue::Data(JsValue::Symbol(sid)),
                PropertyAttrs::BUILTIN,
            );
        }

        // Symbol.prototype (non-enumerable, non-configurable, non-writable per spec)
        if let Some(proto_id) = self.symbol_prototype {
            let proto_key = PropertyKey::String(self.well_known.prototype);
            self.define_shaped_property(
                sym_fn_id,
                proto_key,
                PropertyValue::Data(JsValue::Object(proto_id)),
                PropertyAttrs::BUILTIN,
            );

            // Symbol.prototype.constructor = Symbol
            let ctor_key = PropertyKey::String(self.well_known.constructor);
            self.define_shaped_property(
                proto_id,
                ctor_key,
                PropertyValue::Data(JsValue::Object(sym_fn_id)),
                PropertyAttrs::METHOD,
            );
        }
    }

    pub(super) fn register_bigint_global(&mut self) {
        use super::natives_bigint::{
            native_bigint_as_int_n, native_bigint_as_uint_n, native_bigint_constructor,
            native_bigint_to_string, native_bigint_value_of,
        };

        // BigInt is callable but NOT constructable (like Symbol).
        let bigint_fn = self.create_native_function("BigInt", native_bigint_constructor);

        // Static methods: BigInt.asIntN, BigInt.asUintN
        for (name, func) in [
            ("asIntN", native_bigint_as_int_n as NativeFn),
            ("asUintN", native_bigint_as_uint_n as NativeFn),
        ] {
            let method_fn = self.create_native_function(name, func);
            let key = PropertyKey::String(self.strings.intern(name));
            self.define_shaped_property(
                bigint_fn,
                key,
                PropertyValue::Data(JsValue::Object(method_fn)),
                PropertyAttrs::METHOD,
            );
        }

        // BigInt.prototype
        let proto_id = self.create_object_with_methods(&[
            ("toString", native_bigint_to_string),
            ("valueOf", native_bigint_value_of),
            ("toLocaleString", native_bigint_to_string),
        ]);
        self.bigint_prototype = Some(proto_id);
        let proto_key = PropertyKey::String(self.well_known.prototype);
        self.define_shaped_property(
            bigint_fn,
            proto_key,
            PropertyValue::Data(JsValue::Object(proto_id)),
            PropertyAttrs::BUILTIN,
        );

        // BigInt.prototype.constructor = BigInt
        let ctor_key = PropertyKey::String(self.well_known.constructor);
        self.define_shaped_property(
            proto_id,
            ctor_key,
            PropertyValue::Data(JsValue::Object(bigint_fn)),
            PropertyAttrs::METHOD,
        );

        let bigint_name = self.strings.intern("BigInt");
        self.globals.insert(bigint_name, JsValue::Object(bigint_fn));
    }

    pub(super) fn register_console(&mut self) {
        use super::natives::{native_console_debug, native_console_info, native_console_trace};
        // Namespace object; omit Object.prototype so console behaves like a
        // direct-property host object with only its own methods, matching
        // most engines' layout more closely.
        let console_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: None,
            extensible: true,
        });
        // WHATWG Console §2.  Signature parity with `log` — variadic,
        // returns undefined.  Output routes through `eprintln!` for now;
        // the shell will swap in `host.session().log(level, ...)` later.
        let methods: &[(&str, NativeFn)] = &[
            ("log", native_console_log),
            ("error", native_console_error),
            ("warn", native_console_warn),
            ("info", native_console_info),
            ("debug", native_console_debug),
            ("trace", native_console_trace),
        ];
        for &(name, func) in methods {
            let fn_id = self.create_native_function(name, func);
            let key = PropertyKey::String(self.strings.intern(name));
            self.define_shaped_property(
                console_id,
                key,
                PropertyValue::Data(JsValue::Object(fn_id)),
                PropertyAttrs::METHOD,
            );
        }

        let console_name = self.strings.intern("console");
        self.globals
            .insert(console_name, JsValue::Object(console_id));
    }
}
