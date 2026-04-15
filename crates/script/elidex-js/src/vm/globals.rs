//! Built-in global registration.
//!
//! All `register_*` methods that populate the VM's global environment with
//! standard JS built-ins (console, Math, JSON, Error constructors, etc.).

use super::natives::{
    native_array_constructor, native_array_is_array, native_array_iterator_next,
    native_array_values, native_console_error, native_console_log, native_console_warn,
    native_decode_uri, native_decode_uri_component, native_encode_uri, native_encode_uri_component,
    native_is_finite, native_is_nan, native_iterator_self, native_object_assign,
    native_object_create, native_object_define_property, native_object_entries,
    native_object_freeze, native_object_from_entries, native_object_get_own_property_descriptor,
    native_object_get_own_property_names, native_object_get_own_property_symbols,
    native_object_get_prototype_of, native_object_has_own_property, native_object_is,
    native_object_is_extensible, native_object_is_frozen, native_object_is_prototype_of,
    native_object_is_sealed, native_object_keys, native_object_prevent_extensions,
    native_object_property_is_enumerable, native_object_prototype_to_locale_string,
    native_object_prototype_to_string, native_object_seal, native_object_set_prototype_of,
    native_object_value_of, native_object_values, native_parse_float, native_parse_int,
    native_string_char_at, native_string_char_code_at, native_string_ends_with,
    native_string_includes, native_string_index_of, native_string_iterator,
    native_string_iterator_next, native_string_match, native_string_replace, native_string_search,
    native_string_slice, native_string_split, native_string_starts_with, native_string_substring,
    native_string_to_lower_case, native_string_to_upper_case, native_string_trim,
};
use super::natives::{
    native_symbol_constructor, native_symbol_for, native_symbol_key_for,
    native_symbol_prototype_to_string,
};
use super::natives_array::{
    native_array_concat, native_array_copy_within, native_array_fill, native_array_includes,
    native_array_index_of, native_array_join, native_array_last_index_of, native_array_pop,
    native_array_push, native_array_reverse, native_array_shift, native_array_slice,
    native_array_sort, native_array_splice, native_array_to_locale_string, native_array_to_string,
    native_array_unshift,
};
use super::natives_array_hof::{
    native_array_entries, native_array_every, native_array_filter, native_array_find,
    native_array_find_index, native_array_flat, native_array_flat_map, native_array_for_each,
    native_array_from, native_array_keys, native_array_map, native_array_of, native_array_reduce,
    native_array_reduce_right, native_array_some,
};
use super::natives_boolean::{native_boolean_to_string, native_boolean_value_of};
use super::natives_function::{
    native_function_apply, native_function_bind, native_function_call, native_function_to_string,
};
use super::natives_math::{
    native_math_abs, native_math_acos, native_math_asin, native_math_atan, native_math_atan2,
    native_math_cbrt, native_math_ceil, native_math_clz32, native_math_cos, native_math_exp,
    native_math_floor, native_math_fround, native_math_hypot, native_math_imul, native_math_log,
    native_math_log10, native_math_log2, native_math_max, native_math_min, native_math_pow,
    native_math_random, native_math_round, native_math_sign, native_math_sin, native_math_sqrt,
    native_math_tan, native_math_trunc,
};
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
use super::value::{
    JsValue, NativeContext, Object, ObjectKind, PropertyKey, PropertyStorage, PropertyValue,
    VmError,
};
use super::{NativeFn, VmInner};

/// §19.2.3 Function.prototype — accepts any arguments, returns undefined.
fn native_function_prototype_noop(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Undefined)
}

impl VmInner {
    // -- Global registration -------------------------------------------------

    pub(super) fn register_globals(&mut self) {
        // Allocate the global object (globalThis). Writes through this object
        // are mirrored into the globals HashMap, and reads fall back to
        // globals so `this.<prop>` stays consistent with bare identifier
        // access in non-strict functions (§9.2.1.2).
        let global_obj = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: None, // will be set after Object.prototype exists
            extensible: true,
        });
        self.global_object = global_obj;

        // undefined, NaN, Infinity
        let undef_name = self.well_known.undefined;
        self.globals.insert(undef_name, JsValue::Undefined);

        let nan_name = self.well_known.nan;
        self.globals.insert(nan_name, JsValue::Number(f64::NAN));

        let inf_name = self.well_known.infinity;
        self.globals
            .insert(inf_name, JsValue::Number(f64::INFINITY));

        // console object
        self.register_console();

        // Global functions: parseInt, parseFloat, isNaN, isFinite, URI encoding
        self.register_global_function("parseInt", native_parse_int);
        self.register_global_function("parseFloat", native_parse_float);
        self.register_global_function("isNaN", native_is_nan);
        self.register_global_function("isFinite", native_is_finite);
        self.register_global_function("encodeURI", native_encode_uri);
        self.register_global_function("decodeURI", native_decode_uri);
        self.register_global_function("encodeURIComponent", native_encode_uri_component);
        self.register_global_function("decodeURIComponent", native_decode_uri_component);
        // queueMicrotask (HTML §8.1.4.3).  Registered early so later built-in
        // setup that wants to defer work can rely on it if needed.
        self.register_global_function(
            "queueMicrotask",
            super::natives_promise::native_queue_microtask,
        );
        // Timers (WHATWG §8.7): setTimeout/setInterval schedule a callback
        // on the VM's timer heap; clearTimeout/clearInterval cancel by id.
        // Drain is driven by the shell via VmInner::drain_timers (PR6).
        self.register_global_function("setTimeout", super::natives_timer::native_set_timeout);
        self.register_global_function("setInterval", super::natives_timer::native_set_interval);
        self.register_global_function("clearTimeout", super::natives_timer::native_clear_timeout);
        self.register_global_function("clearInterval", super::natives_timer::native_clear_interval);

        // globalThis (§18.1) — points to the global object
        let global_this_name = self.strings.intern("globalThis");
        self.globals
            .insert(global_this_name, JsValue::Object(global_obj));

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

        // Iterator prototypes (array + string)
        self.register_iterator_prototypes();

        // String.prototype
        self.register_string_prototype();

        // Number.prototype + Boolean.prototype + RegExp.prototype
        self.register_number_prototype();
        self.register_boolean_prototype();
        self.register_regexp_prototype();

        // Symbol.prototype + Symbol global
        self.register_symbol_prototype();
        self.register_symbol_global();

        // BigInt global (not a constructor)
        self.register_bigint_global();

        // Promise global (constructable) + prototype
        self.register_promise_global();

        // Generator.prototype — shared prototype for generator iterator
        // objects. No constructable `Generator` global is exposed (spec);
        // users obtain generators by calling `function* g() { ... }` forms.
        self.register_generator_prototype();

        // EventTarget.prototype — shared prototype for every DOM wrapper
        // (WHATWG DOM §2.7).  No `EventTarget` constructable global is
        // exposed yet; wrappers obtain the prototype via
        // `create_element_wrapper` (PR3 C2).
        self.register_event_target_prototype();

        // Internal Event-methods prototype (PR3) — `event_methods_prototype`
        // is set under the `engine` feature only; without engine there are
        // no DOM events to dispatch and the methods are unused.
        #[cfg(feature = "engine")]
        self.register_event_methods_prototype();

        // Precomputed Shape terminals per EventPayload variant.
        // Must run *after* payload-key WellKnownStrings are interned
        // (done in `Vm::new` before `register_globals`) so the
        // shape-transition walk uses the interned StringIds.  Also
        // after `event_methods_prototype` so no field ordering assumes
        // shapes exist without the prototype.
        #[cfg(feature = "engine")]
        {
            let shapes = self.build_precomputed_event_shapes();
            self.precomputed_event_shapes = Some(shapes);
        }
    }

    /// Helper: register a native function as a global.
    fn register_global_function(
        &mut self,
        name: &str,
        func: fn(&mut NativeContext<'_>, JsValue, &[JsValue]) -> Result<JsValue, VmError>,
    ) {
        let fn_id = self.create_native_function(name, func);
        let name_id = self.strings.intern(name);
        self.globals.insert(name_id, JsValue::Object(fn_id));
    }

    /// Helper: register a constructor-like global object with a `.prototype` property.
    fn register_constructor_global(&mut self, name: &str, proto_id: super::value::ObjectId) {
        let ctor_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: self.object_prototype,
            extensible: true,
        });
        let proto_key = PropertyKey::String(self.well_known.prototype);
        self.define_shaped_property(
            ctor_id,
            proto_key,
            PropertyValue::Data(JsValue::Object(proto_id)),
            PropertyAttrs::BUILTIN,
        );
        let name_id = self.strings.intern(name);
        self.globals.insert(name_id, JsValue::Object(ctor_id));
    }

    /// Helper: create a global object with named native methods.
    pub(super) fn create_object_with_methods(
        &mut self,
        methods: &[(&str, NativeFn)],
    ) -> super::value::ObjectId {
        let obj_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: self.object_prototype,
            extensible: true,
        });
        for &(name, func) in methods {
            let fn_id = self.create_native_function(name, func);
            let key = PropertyKey::String(self.strings.intern(name));
            self.define_shaped_property(
                obj_id,
                key,
                PropertyValue::Data(JsValue::Object(fn_id)),
                PropertyAttrs::METHOD,
            );
        }
        obj_id
    }

    #[allow(clippy::too_many_lines)]
    fn register_prototypes(&mut self) {
        // Object.prototype — root of the prototype chain.
        let obj_proto = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: None,
            extensible: true,
        });
        self.object_prototype = Some(obj_proto);

        // Function.prototype — prototype for all function objects.
        // Must be registered before any native function is created so that
        // `create_native_function` can set the prototype automatically.
        // §19.2.3: Function.prototype is a callable function that accepts
        // any arguments and returns undefined.
        let fp_name = self.strings.intern("");
        let func_proto = self.alloc_object(Object {
            kind: ObjectKind::NativeFunction(super::NativeFunction {
                name: fp_name,
                func: native_function_prototype_noop,
                constructable: false,
            }),
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(obj_proto),
            extensible: true,
        });
        self.function_prototype = Some(func_proto);

        // Function.prototype methods (ES2020 §19.2.3)
        let fp_methods: &[(&str, NativeFn)] = &[
            ("call", native_function_call),
            ("apply", native_function_apply),
            ("bind", native_function_bind),
            ("toString", native_function_to_string),
        ];
        for &(name, func) in fp_methods {
            let fn_id = self.create_native_function(name, func);
            let key = PropertyKey::String(self.strings.intern(name));
            self.define_shaped_property(
                func_proto,
                key,
                PropertyValue::Data(JsValue::Object(fn_id)),
                PropertyAttrs::METHOD,
            );
        }

        // Object.prototype methods (ES2020 §19.1.3)
        let op_methods: &[(&str, NativeFn)] = &[
            ("toString", native_object_prototype_to_string),
            ("toLocaleString", native_object_prototype_to_locale_string),
            ("hasOwnProperty", native_object_has_own_property),
            ("valueOf", native_object_value_of),
            ("isPrototypeOf", native_object_is_prototype_of),
            ("propertyIsEnumerable", native_object_property_is_enumerable),
        ];
        for &(name, func) in op_methods {
            let fn_id = self.create_native_function(name, func);
            let key = PropertyKey::String(self.strings.intern(name));
            self.define_shaped_property(
                obj_proto,
                key,
                PropertyValue::Data(JsValue::Object(fn_id)),
                PropertyAttrs::METHOD,
            );
        }

        // Set the global object's prototype now that Object.prototype exists.
        self.get_object_mut(self.global_object).prototype = Some(obj_proto);

        // Array.prototype — inherits from Object.prototype.
        let arr_proto = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(obj_proto),
            extensible: true,
        });
        self.array_prototype = Some(arr_proto);

        // Array.prototype[Symbol.iterator] = native_array_values
        let iter_fn_id = self.create_native_function("[Symbol.iterator]", native_array_values);
        let sym_iter_key = PropertyKey::Symbol(self.well_known_symbols.iterator);
        self.define_shaped_property(
            arr_proto,
            sym_iter_key,
            PropertyValue::Data(JsValue::Object(iter_fn_id)),
            PropertyAttrs::METHOD,
        );
        // Array.prototype.values === Array.prototype[Symbol.iterator] per spec.
        let values_key = PropertyKey::String(self.strings.intern("values"));
        self.define_shaped_property(
            arr_proto,
            values_key,
            PropertyValue::Data(JsValue::Object(iter_fn_id)),
            PropertyAttrs::METHOD,
        );

        // Array.prototype methods (ES2020 §22.1.3)
        let methods: &[(&str, NativeFn)] = &[
            ("push", native_array_push),
            ("pop", native_array_pop),
            ("shift", native_array_shift),
            ("unshift", native_array_unshift),
            ("reverse", native_array_reverse),
            ("sort", native_array_sort),
            ("splice", native_array_splice),
            ("fill", native_array_fill),
            ("copyWithin", native_array_copy_within),
            ("slice", native_array_slice),
            ("concat", native_array_concat),
            ("join", native_array_join),
            ("indexOf", native_array_index_of),
            ("lastIndexOf", native_array_last_index_of),
            ("includes", native_array_includes),
            ("forEach", native_array_for_each),
            ("map", native_array_map),
            ("filter", native_array_filter),
            ("every", native_array_every),
            ("some", native_array_some),
            ("reduce", native_array_reduce),
            ("reduceRight", native_array_reduce_right),
            ("find", native_array_find),
            ("findIndex", native_array_find_index),
            ("flat", native_array_flat),
            ("flatMap", native_array_flat_map),
            ("entries", native_array_entries),
            ("keys", native_array_keys),
            ("toString", native_array_to_string),
            ("toLocaleString", native_array_to_locale_string),
        ];
        for &(name, func) in methods {
            let fn_id = self.create_native_function(name, func);
            let key = PropertyKey::String(self.strings.intern(name));
            self.define_shaped_property(
                arr_proto,
                key,
                PropertyValue::Data(JsValue::Object(fn_id)),
                PropertyAttrs::METHOD,
            );
        }
    }

    fn register_iterator_prototypes(&mut self) {
        let next_key = PropertyKey::String(self.well_known.next);
        let sym_iter_key = PropertyKey::Symbol(self.well_known_symbols.iterator);

        // Array iterator prototype with `next` + `@@iterator`
        let arr_next_fn = self.create_native_function("next", native_array_iterator_next);
        let arr_iter_self_fn =
            self.create_native_function("[Symbol.iterator]", native_iterator_self);
        let arr_iter_proto = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: self.object_prototype,
            extensible: true,
        });
        self.define_shaped_property(
            arr_iter_proto,
            next_key,
            PropertyValue::Data(JsValue::Object(arr_next_fn)),
            PropertyAttrs::METHOD,
        );
        self.define_shaped_property(
            arr_iter_proto,
            sym_iter_key,
            PropertyValue::Data(JsValue::Object(arr_iter_self_fn)),
            PropertyAttrs::METHOD,
        );
        self.array_iterator_prototype = Some(arr_iter_proto);

        // String iterator prototype with `next` + `@@iterator`
        let str_next_fn = self.create_native_function("next", native_string_iterator_next);
        let str_iter_self_fn =
            self.create_native_function("[Symbol.iterator]", native_iterator_self);
        let str_iter_proto = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: self.object_prototype,
            extensible: true,
        });
        self.define_shaped_property(
            str_iter_proto,
            next_key,
            PropertyValue::Data(JsValue::Object(str_next_fn)),
            PropertyAttrs::METHOD,
        );
        self.define_shaped_property(
            str_iter_proto,
            sym_iter_key,
            PropertyValue::Data(JsValue::Object(str_iter_self_fn)),
            PropertyAttrs::METHOD,
        );
        self.string_iterator_prototype = Some(str_iter_proto);
    }

    fn register_object_global(&mut self) {
        let obj_id = self.create_object_with_methods(&[
            ("keys", native_object_keys),
            ("values", native_object_values),
            ("entries", native_object_entries),
            ("assign", native_object_assign),
            ("create", native_object_create),
            ("defineProperty", native_object_define_property),
            ("is", native_object_is),
            ("getPrototypeOf", native_object_get_prototype_of),
            ("setPrototypeOf", native_object_set_prototype_of),
            (
                "getOwnPropertyDescriptor",
                native_object_get_own_property_descriptor,
            ),
            ("getOwnPropertyNames", native_object_get_own_property_names),
            (
                "getOwnPropertySymbols",
                native_object_get_own_property_symbols,
            ),
            ("freeze", native_object_freeze),
            ("seal", native_object_seal),
            ("isFrozen", native_object_is_frozen),
            ("isSealed", native_object_is_sealed),
            ("preventExtensions", native_object_prevent_extensions),
            ("isExtensible", native_object_is_extensible),
            ("fromEntries", native_object_from_entries),
        ]);
        let name = self.strings.intern("Object");
        self.globals.insert(name, JsValue::Object(obj_id));
    }

    fn register_array_global(&mut self) {
        // Array is a callable constructor with static methods (isArray).
        let ctor_id = self.create_constructable_function("Array", native_array_constructor);
        // Attach Array.prototype
        let proto_key = PropertyKey::String(self.well_known.prototype);
        if let Some(arr_proto) = self.array_prototype {
            self.define_shaped_property(
                ctor_id,
                proto_key,
                PropertyValue::Data(JsValue::Object(arr_proto)),
                PropertyAttrs::BUILTIN,
            );
        }
        // Attach static methods: Array.isArray, Array.from, Array.of
        for (method_name, func) in [
            ("isArray", native_array_is_array as NativeFn),
            ("from", native_array_from as NativeFn),
            ("of", native_array_of as NativeFn),
        ] {
            let fn_id = self.create_native_function(method_name, func);
            let key = PropertyKey::String(self.strings.intern(method_name));
            self.define_shaped_property(
                ctor_id,
                key,
                PropertyValue::Data(JsValue::Object(fn_id)),
                PropertyAttrs::METHOD,
            );
        }
        // Array.prototype.constructor = Array
        if let Some(arr_proto) = self.array_prototype {
            let ctor_key = PropertyKey::String(self.well_known.constructor);
            self.define_shaped_property(
                arr_proto,
                ctor_key,
                PropertyValue::Data(JsValue::Object(ctor_id)),
                PropertyAttrs::METHOD,
            );
        }
        let name = self.strings.intern("Array");
        self.globals.insert(name, JsValue::Object(ctor_id));
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
            ("trunc", native_math_trunc),
            ("sign", native_math_sign),
            ("sin", native_math_sin),
            ("cos", native_math_cos),
            ("tan", native_math_tan),
            ("asin", native_math_asin),
            ("acos", native_math_acos),
            ("atan", native_math_atan),
            ("atan2", native_math_atan2),
            ("log2", native_math_log2),
            ("log10", native_math_log10),
            ("exp", native_math_exp),
            ("cbrt", native_math_cbrt),
            ("hypot", native_math_hypot),
            ("clz32", native_math_clz32),
            ("imul", native_math_imul),
            ("fround", native_math_fround),
        ]);
        // Math constants
        let consts: &[(&str, f64)] = &[
            ("PI", std::f64::consts::PI),
            ("E", std::f64::consts::E),
            ("LN2", std::f64::consts::LN_2),
            ("LN10", std::f64::consts::LN_10),
            ("LOG2E", std::f64::consts::LOG2_E),
            ("LOG10E", std::f64::consts::LOG10_E),
            ("SQRT2", std::f64::consts::SQRT_2),
            ("SQRT1_2", std::f64::consts::FRAC_1_SQRT_2),
        ];
        for &(name, value) in consts {
            let key = PropertyKey::String(self.strings.intern(name));
            self.define_shaped_property(
                obj_id,
                key,
                PropertyValue::Data(JsValue::Number(value)),
                PropertyAttrs::BUILTIN,
            );
        }
        let name = self.strings.intern("Math");
        self.globals.insert(name, JsValue::Object(obj_id));
    }

    fn register_json_global(&mut self) {
        let obj_id = self.create_object_with_methods(&[
            ("stringify", super::natives_json::native_json_stringify),
            ("parse", super::natives_json::native_json_parse),
        ]);
        let name = self.strings.intern("JSON");
        self.globals.insert(name, JsValue::Object(obj_id));
    }

    fn register_string_prototype(&mut self) {
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

    fn register_number_prototype(&mut self) {
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
        let statics: &[(&str, super::NativeFn)] = &[
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

    fn register_boolean_prototype(&mut self) {
        let proto_id = self.create_object_with_methods(&[
            ("toString", native_boolean_to_string),
            ("valueOf", native_boolean_value_of),
        ]);
        self.boolean_prototype = Some(proto_id);
        self.register_constructor_global("Boolean", proto_id);
    }

    fn register_regexp_prototype(&mut self) {
        let proto_id = self.create_object_with_methods(&[
            ("test", native_regexp_test),
            ("exec", native_regexp_exec),
            ("toString", native_regexp_to_string),
        ]);
        self.regexp_prototype = Some(proto_id);
    }

    fn register_symbol_prototype(&mut self) {
        let proto_id =
            self.create_object_with_methods(&[("toString", native_symbol_prototype_to_string)]);
        self.symbol_prototype = Some(proto_id);
    }

    fn register_symbol_global(&mut self) {
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

    fn register_bigint_global(&mut self) {
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

    fn register_console(&mut self) {
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
        // PR6 will swap in host.session().log(level, ...).
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
