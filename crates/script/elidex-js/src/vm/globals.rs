//! Built-in global registration.
//!
//! All `register_*` methods that populate the VM's global environment with
//! standard JS built-ins (console, Math, JSON, Error constructors, etc.).

use super::natives::{
    native_array_constructor, native_array_is_array, native_array_iterator_next,
    native_array_values, native_decode_uri, native_decode_uri_component, native_encode_uri,
    native_encode_uri_component, native_is_finite, native_is_nan, native_iterator_self,
    native_object_assign, native_object_create, native_object_define_property,
    native_object_entries, native_object_freeze, native_object_from_entries,
    native_object_get_own_property_descriptor, native_object_get_own_property_names,
    native_object_get_own_property_symbols, native_object_get_prototype_of,
    native_object_has_own_property, native_object_is, native_object_is_extensible,
    native_object_is_frozen, native_object_is_prototype_of, native_object_is_sealed,
    native_object_keys, native_object_prevent_extensions, native_object_property_is_enumerable,
    native_object_prototype_to_locale_string, native_object_prototype_to_string,
    native_object_seal, native_object_set_prototype_of, native_object_value_of,
    native_object_values, native_parse_float, native_parse_int, native_string_iterator_next,
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
        // Allocate the global object (`globalThis` / `window`).  It is a
        // `HostObject` backed by the Window ECS entity so that
        // `window.addEventListener(...)` targets a distinct entity from
        // `document.addEventListener(...)` (WHATWG HTML §7.2).  The
        // initial `entity_bits` is `0`, which `Entity::from_bits`
        // rejects — `entity_from_this` therefore treats any
        // pre-bind/post-unbind access as a silent no-op rather than
        // panicking.  `Vm::bind` overwrites `entity_bits` with the
        // `HostData::window_entity()` value on every bind; `Vm::unbind`
        // resets it back to `0`.
        //
        // Writes through this object are mirrored into the `globals`
        // HashMap, and reads fall back to `globals` so `this.<prop>`
        // stays consistent with bare identifier access in non-strict
        // functions (§9.2.1.2).
        let global_obj = self.alloc_object(Object {
            kind: ObjectKind::HostObject { entity_bits: 0 },
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: None, // chain finalised after Window.prototype exists
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

        // EventTarget.prototype — shared root of the DOM wrapper
        // prototype chain (WHATWG DOM §2.7).  Carries only the three
        // EventTarget methods; Node-level members live on
        // Node.prototype one level up.
        self.register_event_target_prototype();

        // Node.prototype — chained to EventTarget.prototype.  Carries
        // Node-common accessors (parentNode, nodeType, textContent, …)
        // and mutation methods (appendChild, …).  Every DOM Node
        // wrapper reaches this prototype; Window (EventTarget but
        // not a Node) does not — Window.prototype chains directly to
        // EventTarget.prototype.
        #[cfg(feature = "engine")]
        self.register_node_prototype();

        // CharacterData.prototype — chained to Node.prototype.  Holds
        // the Text / Comment shared members (`data`, `length`,
        // `appendData` / `insertData` / ...).  Wrappers for Text and
        // Comment entities route through here via
        // `create_element_wrapper`'s `PrototypeKind` branch; Text
        // wrappers further chain through `Text.prototype`.
        #[cfg(feature = "engine")]
        self.register_character_data_prototype();

        // Text.prototype — chained to CharacterData.prototype.  Carries
        // Text-only members (`splitText` today; `wholeText` /
        // `assignedSlot` land in PR4f / PR5b).
        #[cfg(feature = "engine")]
        self.register_text_prototype();

        // Element.prototype — chained to Node.prototype.  Holds
        // Element-specific members (tree nav, attributes, matches).
        // Wrappers for entities carrying a `TagType` component pick
        // it up automatically via `create_element_wrapper`'s
        // per-entity prototype branch; Text / Comment wrappers skip
        // this level.
        #[cfg(feature = "engine")]
        self.register_element_prototype();

        // Window.prototype — prototype for the `globalThis` `HostObject`
        // (WHATWG HTML §7.2).  Must run *after*
        // `register_event_target_prototype` because the Window prototype
        // chains to it.  After creation, splice Window.prototype into
        // globalThis's prototype slot, finalising
        // `globalThis → Window.prototype → EventTarget.prototype →
        // Object.prototype`.
        #[cfg(feature = "engine")]
        {
            self.register_window_prototype();
            self.get_object_mut(self.global_object).prototype = self.window_prototype;
            // `navigator` — static Navigator object (WHATWG HTML §8.1.5).
            // Installed after the Window prototype chain is in place so
            // `navigator.hasOwnProperty`, `Object.getPrototypeOf(navigator)`
            // etc. resolve against `Object.prototype` as expected.
            self.register_navigator_global();
            // `performance` — HR-Time §5.  Shares the time origin
            // (`VmInner::start_instant`) with `Event.timeStamp`.
            self.register_performance_global();
            // `location` — WHATWG HTML §7.1.  Reads/writes
            // `VmInner::navigation` (in-memory only at Phase 2).
            self.register_location_global();
            // `history` — WHATWG HTML §7.4.  Shares navigation state
            // with `location`.
            self.register_history_global();
            // `window === globalThis` (WHATWG HTML §7.2.4).
            self.install_window_self_ref();
        }

        // Internal Event-methods prototype (PR3) — `event_methods_prototype`
        // is set under the `engine` feature only; without engine there are
        // no DOM events to dispatch and the methods are unused.
        #[cfg(feature = "engine")]
        self.register_event_methods_prototype();

        // `AbortController` constructor + `AbortSignal` global +
        // `AbortSignal.prototype` (WHATWG DOM §3.1).  Must run after
        // `register_event_target_prototype` (the prototype chains
        // there) and after `register_error_constructors` (the default
        // abort reason allocates against `error_prototype`).
        #[cfg(feature = "engine")]
        self.register_abort_signal_global();

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
    pub(super) fn register_constructor_global(
        &mut self,
        name: &str,
        proto_id: super::value::ObjectId,
    ) {
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
        self.install_methods(obj_id, methods);
        obj_id
    }

    /// Install each `(name, native)` as a `DATA`/`METHOD`-attributed
    /// property on `obj_id`.  Shared by `create_object_with_methods`,
    /// the Window prototype registration (`host/window.rs`), and the
    /// per-bind document-method installer (`host/document.rs`) —
    /// everywhere we need to attach a batch of methods to an object
    /// that already exists.
    pub(crate) fn install_methods(
        &mut self,
        obj_id: super::value::ObjectId,
        methods: &[(&str, NativeFn)],
    ) {
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
    }

    /// Install each `(name, getter)` as a read-only accessor on
    /// `obj_id` with the WebIDL-default attrs (non-writable,
    /// enumerable, configurable).  The getter's WebIDL name
    /// (`"get foo"`) is derived from `name`.
    ///
    /// Engine-only — every call site lives behind `#[cfg(feature =
    /// "engine")]` (host globals: `location`, `history`, `window`,
    /// `document`).  Non-engine builds omit the helper entirely.
    #[cfg(feature = "engine")]
    pub(crate) fn install_ro_accessors(
        &mut self,
        obj_id: super::value::ObjectId,
        accessors: &[(&str, NativeFn)],
    ) {
        for &(name, getter) in accessors {
            let gid = self.create_native_function(&format!("get {name}"), getter);
            let key = PropertyKey::String(self.strings.intern(name));
            self.define_shaped_property(
                obj_id,
                key,
                PropertyValue::Accessor {
                    getter: Some(gid),
                    setter: None,
                },
                PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }
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

    // String / Number / Boolean / RegExp / Symbol / BigInt prototypes
    // and the `console` namespace are registered from
    // `vm/globals_primitives.rs` — extracted to keep this file under
    // the 1000-line convention.
}
