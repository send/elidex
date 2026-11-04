//! ECMA-262 fundamental-object registration: the prototype chain roots
//! (`Object.prototype`, `Function.prototype`, …), iterator prototypes, and the
//! `Object` / `Array` / `Math` / `JSON` globals.
//!
//! Split from `globals.rs` (pure move) to keep that file under the project's
//! 1000-line convention.

use super::natives::{
    native_array_constructor, native_array_is_array, native_array_iterator_next,
    native_array_values, native_iterator_self, native_object_assign, native_object_constructor,
    native_object_create, native_object_define_property, native_object_entries,
    native_object_freeze, native_object_from_entries, native_object_get_own_property_descriptor,
    native_object_get_own_property_names, native_object_get_own_property_symbols,
    native_object_get_prototype_of, native_object_has_own_property, native_object_is,
    native_object_is_extensible, native_object_is_frozen, native_object_is_prototype_of,
    native_object_is_sealed, native_object_keys, native_object_prevent_extensions,
    native_object_property_is_enumerable, native_object_prototype_to_locale_string,
    native_object_prototype_to_string, native_object_seal, native_object_set_prototype_of,
    native_object_value_of, native_object_values, native_string_iterator_next,
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
use super::shape::{self, PropertyAttrs};
use super::value::{
    JsValue, NativeContext, Object, ObjectKind, PropertyKey, PropertyStorage, PropertyValue,
    VmError,
};
use super::{NativeFn, VmInner};

/// §20.2.3 Function.prototype — accepts any arguments, returns undefined.
fn native_function_prototype_noop(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Undefined)
}

impl VmInner {
    #[allow(clippy::too_many_lines)]
    pub(super) fn register_prototypes(&mut self) {
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
        // §20.2.3: Function.prototype is a callable function that accepts
        // any arguments and returns undefined.
        let fp_name = self.strings.intern("");
        let func_proto = self.alloc_object(Object {
            kind: ObjectKind::NativeFunction(super::NativeFunction {
                name: fp_name,
                func: native_function_prototype_noop,
                shape: super::value::CallShape::CallableOnly,
                bound_key: None,
            }),
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(obj_proto),
            extensible: true,
        });
        self.function_prototype = Some(func_proto);

        // Function.prototype methods (ECMA-262 §20.2.3)
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

        // Object.prototype methods (ECMA-262 §20.1.3)
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

        // Array.prototype methods (ECMA-262 §23.1.3)
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

    pub(super) fn register_iterator_prototypes(&mut self) {
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

    pub(super) fn register_object_global(&mut self) {
        // §20.1.1 The Object Constructor — a constructable function so both
        // `Object(x)` (call form, §20.1.1.1) and `new Object(x)` / subclass `new`
        // (construct form) work, wired to %Object.prototype% like every sibling
        // constructor. (The historic namespace-object shape — `ObjectKind::Ordinary`
        // via `create_object_with_methods` — left `Object` non-callable and
        // `Object.prototype`/`Object.prototype.constructor` unreachable.)
        let ctor_id = self.create_constructable_function("Object", native_object_constructor);
        // Record the intrinsic id so `native_object_constructor` can distinguish a
        // direct `new Object()` (NewTarget === %Object%) from a subclass `new`.
        self.object_constructor = Some(ctor_id);
        // §20.1.2 static methods.
        self.install_methods(
            ctor_id,
            &[
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
            ],
        );
        // §20.1.2.21 `Object.prototype` (BUILTIN) + §20.1.3.1
        // `Object.prototype.constructor` (METHOD) + the `Object` global binding,
        // via the shared sibling helper. %Object.prototype% is created by
        // `register_prototypes` (called at globals.rs `register()` before this).
        let obj_proto = self.object_prototype.expect(
            "object_prototype is populated by register_prototypes before register_object_global",
        );
        self.wire_constructor_global("Object", ctor_id, obj_proto);
        // `Object.name` = "Object" comes free from `create_native_function_keyed`.
        // `Object.length` is intentionally NOT installed: no built-in ctor in elidex
        // installs `.length` (a uniform gap — a per-ctor `.length` pass is a
        // separate concern, tracked as a candidate slot in this PR's plan-memo §7),
        // so adding it only to `Object` would be a strangler.
    }

    pub(super) fn register_array_global(&mut self) {
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

    pub(super) fn register_math_global(&mut self) {
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

    pub(super) fn register_json_global(&mut self) {
        let obj_id = self.create_object_with_methods(&[
            ("stringify", super::natives_json::native_json_stringify),
            ("parse", super::natives_json::native_json_parse),
        ]);
        let name = self.strings.intern("JSON");
        self.globals.insert(name, JsValue::Object(obj_id));
    }
}
