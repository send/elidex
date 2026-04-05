//! Built-in global registration.
//!
//! All `register_*` methods that populate the VM's global environment with
//! standard JS built-ins (console, Math, JSON, Error constructors, etc.).

use super::natives::{
    native_array_is_array, native_array_values, native_console_error, native_console_log,
    native_console_warn, native_error_constructor, native_is_finite, native_is_nan,
    native_json_parse_stub, native_json_stringify_stub, native_math_abs, native_math_ceil,
    native_math_floor, native_math_log, native_math_max, native_math_min, native_math_pow,
    native_math_random, native_math_round, native_math_sqrt, native_object_assign,
    native_object_create, native_object_define_property, native_object_get_own_property_symbols,
    native_object_keys, native_object_prototype_to_string, native_object_values,
    native_parse_float, native_parse_int, native_range_error_constructor,
    native_reference_error_constructor, native_string_char_at, native_string_char_code_at,
    native_string_ends_with, native_string_includes, native_string_index_of,
    native_string_iterator, native_string_replace, native_string_slice, native_string_split,
    native_string_starts_with, native_string_substring, native_string_to_lower_case,
    native_string_to_upper_case, native_string_trim, native_symbol_constructor, native_symbol_for,
    native_symbol_key_for, native_symbol_prototype_to_string, native_type_error_constructor,
};
use super::value::{JsValue, NativeContext, Object, ObjectKind, Property, PropertyKey, VmError};
use super::{NativeFn, Vm};

impl Vm {
    // -- Global registration -------------------------------------------------

    pub(super) fn register_globals(&mut self) {
        // Allocate the global object (globalThis). Properties are not
        // synchronised with the globals HashMap — this object is only
        // used when `this` needs to be coerced to the global object in
        // non-strict functions (§9.2.1.2).
        let global_obj = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            properties: Vec::new(),
            prototype: None, // will be set after Object.prototype exists
        });
        self.inner.global_object = global_obj;

        // undefined, NaN, Infinity
        let undef_name = self.inner.well_known.undefined;
        self.inner.globals.insert(undef_name, JsValue::Undefined);

        let nan_name = self.inner.well_known.nan;
        self.inner
            .globals
            .insert(nan_name, JsValue::Number(f64::NAN));

        let inf_name = self.inner.well_known.infinity;
        self.inner
            .globals
            .insert(inf_name, JsValue::Number(f64::INFINITY));

        // console object
        self.register_console();

        // Global functions: parseInt, parseFloat, isNaN, isFinite
        self.register_global_function("parseInt", native_parse_int);
        self.register_global_function("parseFloat", native_parse_float);
        self.register_global_function("isNaN", native_is_nan);
        self.register_global_function("isFinite", native_is_finite);

        // Object.prototype and Array.prototype
        self.register_prototypes();

        // Error constructors
        self.register_error_constructors();

        // Object global
        self.register_object_global();

        // Array global
        self.register_array_global();

        // Math global
        self.register_math_global();

        // JSON global (stubs for M4-10)
        self.register_json_global();

        // String.prototype
        self.register_string_prototype();

        // Symbol.prototype + Symbol global
        self.register_symbol_prototype();
        self.register_symbol_global();
    }

    /// Helper: register a native function as a global.
    fn register_global_function(
        &mut self,
        name: &str,
        func: fn(&mut NativeContext<'_>, JsValue, &[JsValue]) -> Result<JsValue, VmError>,
    ) {
        let fn_id = self.create_native_function(name, func);
        let name_id = self.inner.strings.intern(name);
        self.inner.globals.insert(name_id, JsValue::Object(fn_id));
    }

    /// Helper: create a global object with named native methods.
    fn create_object_with_methods(
        &mut self,
        methods: &[(&str, NativeFn)],
    ) -> super::value::ObjectId {
        let obj_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            properties: Vec::new(),
            prototype: self.inner.object_prototype,
        });
        for &(name, func) in methods {
            let fn_id = self.create_native_function(name, func);
            let key = PropertyKey::String(self.inner.strings.intern(name));
            self.get_object_mut(obj_id)
                .properties
                .push((key, Property::method(JsValue::Object(fn_id))));
        }
        obj_id
    }

    fn register_prototypes(&mut self) {
        // Object.prototype — root of the prototype chain.
        let obj_proto = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            properties: Vec::new(),
            prototype: None,
        });
        self.inner.object_prototype = Some(obj_proto);

        // Object.prototype.toString (ES2020 §19.1.3.6)
        let to_str_fn = self.create_native_function("toString", native_object_prototype_to_string);
        let to_str_key = PropertyKey::String(self.inner.strings.intern("toString"));
        self.get_object_mut(obj_proto)
            .properties
            .push((to_str_key, Property::method(JsValue::Object(to_str_fn))));

        // Set the global object's prototype now that Object.prototype exists.
        self.get_object_mut(self.inner.global_object).prototype = Some(obj_proto);

        // Array.prototype — inherits from Object.prototype.
        let arr_proto = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            properties: Vec::new(),
            prototype: Some(obj_proto),
        });
        self.inner.array_prototype = Some(arr_proto);

        // Array.prototype[Symbol.iterator] = native_array_values
        let iter_fn_id = self.create_native_function("[Symbol.iterator]", native_array_values);
        let sym_iter_key = PropertyKey::Symbol(self.inner.well_known_symbols.iterator);
        self.get_object_mut(arr_proto)
            .properties
            .push((sym_iter_key, Property::method(JsValue::Object(iter_fn_id))));
    }

    fn register_error_constructors(&mut self) {
        let ctors: &[(&str, NativeFn)] = &[
            ("Error", native_error_constructor),
            ("TypeError", native_type_error_constructor),
            ("ReferenceError", native_reference_error_constructor),
            ("RangeError", native_range_error_constructor),
        ];
        for &(name, func) in ctors {
            let fn_id = self.create_native_function(name, func);
            let name_id = self.inner.strings.intern(name);
            self.inner.globals.insert(name_id, JsValue::Object(fn_id));
        }
    }

    fn register_object_global(&mut self) {
        let obj_id = self.create_object_with_methods(&[
            ("keys", native_object_keys),
            ("values", native_object_values),
            ("assign", native_object_assign),
            ("create", native_object_create),
            ("defineProperty", native_object_define_property),
            (
                "getOwnPropertySymbols",
                native_object_get_own_property_symbols,
            ),
        ]);
        let name = self.inner.strings.intern("Object");
        self.inner.globals.insert(name, JsValue::Object(obj_id));
    }

    fn register_array_global(&mut self) {
        let obj_id = self.create_object_with_methods(&[("isArray", native_array_is_array)]);
        let name = self.inner.strings.intern("Array");
        self.inner.globals.insert(name, JsValue::Object(obj_id));
    }

    fn register_math_global(&mut self) {
        let obj_id = self.create_object_with_methods(&[
            ("abs", native_math_abs),
            ("ceil", native_math_ceil),
            ("floor", native_math_floor),
            ("round", native_math_round),
            ("max", native_math_max),
            ("min", native_math_min),
            ("random", native_math_random),
            ("sqrt", native_math_sqrt),
            ("pow", native_math_pow),
            ("log", native_math_log),
        ]);
        // Math.PI, Math.E
        let pi_key = PropertyKey::String(self.inner.strings.intern("PI"));
        self.get_object_mut(obj_id).properties.push((
            pi_key,
            Property::builtin(JsValue::Number(std::f64::consts::PI)),
        ));
        let e_key = PropertyKey::String(self.inner.strings.intern("E"));
        self.get_object_mut(obj_id).properties.push((
            e_key,
            Property::builtin(JsValue::Number(std::f64::consts::E)),
        ));
        let name = self.inner.strings.intern("Math");
        self.inner.globals.insert(name, JsValue::Object(obj_id));
    }

    fn register_json_global(&mut self) {
        let obj_id = self.create_object_with_methods(&[
            ("stringify", native_json_stringify_stub),
            ("parse", native_json_parse_stub),
        ]);
        let name = self.inner.strings.intern("JSON");
        self.inner.globals.insert(name, JsValue::Object(obj_id));
    }

    fn register_string_prototype(&mut self) {
        let proto_id = self.create_object_with_methods(&[
            ("charAt", native_string_char_at),
            ("charCodeAt", native_string_char_code_at),
            ("indexOf", native_string_index_of),
            ("includes", native_string_includes),
            ("slice", native_string_slice),
            ("substring", native_string_substring),
            ("toLowerCase", native_string_to_lower_case),
            ("toUpperCase", native_string_to_upper_case),
            ("trim", native_string_trim),
            ("split", native_string_split),
            ("startsWith", native_string_starts_with),
            ("endsWith", native_string_ends_with),
            ("replace", native_string_replace),
        ]);
        // String.prototype[Symbol.iterator] = native_string_iterator
        let iter_fn_id = self.create_native_function("[Symbol.iterator]", native_string_iterator);
        let sym_iter_key = PropertyKey::Symbol(self.inner.well_known_symbols.iterator);
        self.get_object_mut(proto_id)
            .properties
            .push((sym_iter_key, Property::method(JsValue::Object(iter_fn_id))));
        self.inner.string_prototype = Some(proto_id);
    }

    fn register_symbol_prototype(&mut self) {
        let proto_id =
            self.create_object_with_methods(&[("toString", native_symbol_prototype_to_string)]);
        self.inner.symbol_prototype = Some(proto_id);
    }

    fn register_symbol_global(&mut self) {
        // The Symbol "constructor" is callable but not constructable.
        // Register it as a native function, then attach static methods and
        // well-known symbol properties.
        let sym_fn_id = self.create_native_function("Symbol", native_symbol_constructor);
        let name_id = self.inner.strings.intern("Symbol");
        self.inner
            .globals
            .insert(name_id, JsValue::Object(sym_fn_id));

        // Symbol.for
        let for_fn = self.create_native_function("for", native_symbol_for);
        let for_key = PropertyKey::String(self.inner.strings.intern("for"));
        self.get_object_mut(sym_fn_id)
            .properties
            .push((for_key, Property::method(JsValue::Object(for_fn))));

        // Symbol.keyFor
        let key_for_fn = self.create_native_function("keyFor", native_symbol_key_for);
        let key_for_key = PropertyKey::String(self.inner.strings.intern("keyFor"));
        self.get_object_mut(sym_fn_id)
            .properties
            .push((key_for_key, Property::method(JsValue::Object(key_for_fn))));

        // Well-known symbols as properties
        let wk = &self.inner.well_known_symbols;
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
            let key = PropertyKey::String(self.inner.strings.intern(prop_name));
            self.get_object_mut(sym_fn_id)
                .properties
                .push((key, Property::builtin(JsValue::Symbol(sid))));
        }
    }

    fn register_console(&mut self) {
        let console_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            properties: Vec::new(),
            prototype: None,
        });

        // console.log
        let log_fn = self.create_native_function("log", native_console_log);
        let log_key = PropertyKey::String(self.inner.well_known.log);
        self.get_object_mut(console_id)
            .properties
            .push((log_key, Property::method(JsValue::Object(log_fn))));

        // console.error
        let error_fn = self.create_native_function("error", native_console_error);
        let error_key = PropertyKey::String(self.inner.well_known.error);
        self.get_object_mut(console_id)
            .properties
            .push((error_key, Property::method(JsValue::Object(error_fn))));

        // console.warn
        let warn_fn = self.create_native_function("warn", native_console_warn);
        let warn_key = PropertyKey::String(self.inner.well_known.warn);
        self.get_object_mut(console_id)
            .properties
            .push((warn_key, Property::method(JsValue::Object(warn_fn))));

        let console_name = self.inner.strings.intern("console");
        self.inner
            .globals
            .insert(console_name, JsValue::Object(console_id));
    }
}
