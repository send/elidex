//! `%TypedArray%` + concrete subclasses (ES2024 §23.2).
//!
//! `%TypedArray%` is an abstract base class — not exposed as a global,
//! reachable only via `Object.getPrototypeOf(Uint8Array)` etc. — that
//! carries shared IDL attrs (`buffer` / `byteOffset` / `byteLength` /
//! `length`) plus the prototype method suite (`fill` / `set` /
//! `subarray` / …, landing in PR5-typed-array §C4).  11 concrete
//! subclasses (`Int8Array` / `Uint8Array` / `Uint8ClampedArray` /
//! `Int16Array` / `Uint16Array` / `Int32Array` / `Uint32Array` /
//! `Float32Array` / `Float64Array` / `BigInt64Array` /
//! `BigUint64Array`) chain their prototype to `%TypedArray%.prototype`
//! and differ only by the [`super::super::value::ElementKind`] tag
//! baked into each instance's [`ObjectKind::TypedArray`] variant.
//!
//! ```text
//! new Uint8Array(n)           ObjectKind::TypedArray { element_kind: Uint8, … }
//!   → Uint8Array.prototype
//!     → %TypedArray%.prototype
//!       → Object.prototype
//! ```
//!
//! ## Byte-order convention
//!
//! TypedArray indexed reads / writes use **little-endian byte order
//! unconditionally** — an elidex implementation choice for
//! cross-platform determinism.  `IsLittleEndian()` (ES §25.1.3.1) is
//! implementation-defined, so a constant choice is spec-compliant.
//! [`super::data_view::DataView`] (PR5-typed-array §C5) exposes both
//! endiannesses explicitly via its `littleEndian` argument (ES
//! §25.3.4, default `false`).
//!
//! ## Backing storage
//!
//! A TypedArray is a **view**: the bytes live in the underlying
//! [`ObjectKind::ArrayBuffer`] (shared [`super::super::VmInner::body_data`]
//! entry), and every view over the same buffer mutates the same
//! bytes.  The view's `[[ByteOffset]]` / `[[ByteLength]]` slots
//! stored inline on `ObjectKind::TypedArray` translate JS indices to
//! buffer offsets.  No side-table is needed because all four spec
//! slots are immutable after construction in this PR —
//! `transfer()` / `resize()` / `detached` tracking (ES2024) are
//! deferred to the M4-12 cutover-residual tranche.

#![cfg(feature = "engine")]

use std::sync::Arc;

use super::super::shape::{self, PropertyAttrs};
use super::super::value::{
    ElementKind, JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey,
    PropertyStorage, PropertyValue, StringId, VmError,
};
use super::super::{NativeFn, VmInner};

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

impl VmInner {
    /// Allocate `%TypedArray%.prototype`, the abstract
    /// `%TypedArray%` constructor, `@@toStringTag` / `@@species` /
    /// `constructor` links, and all 11 concrete subclass prototypes
    /// + constructors.  Must run during `register_globals()` after
    /// `register_array_buffer_global` (ArrayBuffer backs the
    /// TypedArray's bytes) and before
    /// `register_structured_clone_global` (so C6 clone arms can
    /// reach the subclass prototype ids).
    ///
    /// # Panics
    ///
    /// Panics if `object_prototype` is `None` — indicates a
    /// mis-ordered registration pass.
    pub(in crate::vm) fn register_typed_array_prototype_global(&mut self) {
        let object_proto = self
            .object_prototype
            .expect("register_typed_array_prototype_global called before register_prototypes");

        // Abstract `%TypedArray%.prototype` (ES §23.2.3).
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });
        self.typed_array_prototype = Some(proto_id);
        self.install_typed_array_prototype_members(proto_id);

        // Abstract `%TypedArray%` constructor (ES §23.2.1).
        // Callable, constructable — but both paths unconditionally
        // throw TypeError per §23.2.1.1 ("Abstract class TypedArray
        // not directly constructable").  The function still exists
        // so `Object.getPrototypeOf(Uint8Array) === %TypedArray%`
        // holds and so `@@species` can live on it.
        let abstract_ctor =
            self.create_constructable_function("TypedArray", native_abstract_typed_array_ctor);
        let proto_key = PropertyKey::String(self.well_known.prototype);
        self.define_shaped_property(
            abstract_ctor,
            proto_key,
            PropertyValue::Data(JsValue::Object(proto_id)),
            PropertyAttrs::BUILTIN,
        );
        let ctor_key = PropertyKey::String(self.well_known.constructor);
        self.define_shaped_property(
            proto_id,
            ctor_key,
            PropertyValue::Data(JsValue::Object(abstract_ctor)),
            PropertyAttrs::METHOD,
        );

        // `%TypedArray%[@@species]` accessor — returns `this`
        // (ES §23.2.2.4).  Enables spec-correct species lookup for
        // allocating methods (`map` / `filter` / `slice` — installed
        // in C4), though the current minimal method set uses
        // identity constructor (same subclass as receiver).
        let species_getter =
            self.create_native_function("get [Symbol.species]", native_typed_array_species_get);
        self.define_shaped_property(
            abstract_ctor,
            PropertyKey::Symbol(self.well_known_symbols.species),
            PropertyValue::Accessor {
                getter: Some(species_getter),
                setter: None,
            },
            PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );

        // Register each of the 11 concrete subclass prototypes +
        // constructors, chaining the prototype to the abstract's
        // prototype and the ctor to `%TypedArray%`.
        // Iteration order matches `ElementKind` declaration for
        // predictable global-install ordering.
        for entry in SUBCLASS_TABLE {
            self.register_typed_array_subclass(entry, proto_id, abstract_ctor);
        }
    }

    /// Install the generic accessors (`buffer` / `byteOffset` /
    /// `byteLength` / `length`) and the `@@toStringTag` getter on
    /// `%TypedArray%.prototype`.  The four accessors read the
    /// authoritative spec slots carried inline on
    /// [`ObjectKind::TypedArray`], so they work uniformly across
    /// every subclass instance without per-subclass install.
    fn install_typed_array_prototype_members(&mut self, proto_id: ObjectId) {
        let accessors: [(StringId, NativeFn); 4] = [
            (
                self.well_known.buffer,
                native_typed_array_get_buffer as NativeFn,
            ),
            (
                self.well_known.byte_offset,
                native_typed_array_get_byte_offset as NativeFn,
            ),
            (
                self.well_known.byte_length,
                native_typed_array_get_byte_length as NativeFn,
            ),
            (
                self.well_known.length,
                native_typed_array_get_length as NativeFn,
            ),
        ];
        for (name_sid, getter_fn) in accessors {
            let name = self.strings.get_utf8(name_sid);
            let gid = self.create_native_function(&format!("get {name}"), getter_fn);
            self.define_shaped_property(
                proto_id,
                PropertyKey::String(name_sid),
                PropertyValue::Accessor {
                    getter: Some(gid),
                    setter: None,
                },
                PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }

        // `%TypedArray%.prototype[@@toStringTag]` getter (ES
        // §23.2.3.32): returns the subclass name string from
        // `[[TypedArrayName]]` (derived from `element_kind`), or
        // `undefined` if `this` lacks the brand (does NOT throw,
        // per spec — the getter silently yields undefined so
        // `Object.prototype.toString.call(foreign)` returns
        // `"[object Object]"` rather than error).
        let tag_getter =
            self.create_native_function("get [Symbol.toStringTag]", native_typed_array_to_tag_get);
        self.define_shaped_property(
            proto_id,
            PropertyKey::Symbol(self.well_known_symbols.to_string_tag),
            PropertyValue::Accessor {
                getter: Some(tag_getter),
                setter: None,
            },
            PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );

        // Core method suite.  See `typed_array_methods` for the
        // spec-aligned bodies; each method is installed on
        // `%TypedArray%.prototype` (shared across all 11 subclasses
        // via prototype chain).
        let methods: [(StringId, NativeFn); 19] = [
            (
                self.strings.intern("fill"),
                super::typed_array_methods::native_typed_array_fill as NativeFn,
            ),
            (
                self.strings.intern("subarray"),
                super::typed_array_methods::native_typed_array_subarray as NativeFn,
            ),
            (
                self.strings.intern("slice"),
                super::typed_array_methods::native_typed_array_slice as NativeFn,
            ),
            (
                self.strings.intern("values"),
                super::typed_array_methods::native_typed_array_values as NativeFn,
            ),
            (
                self.strings.intern("keys"),
                super::typed_array_methods::native_typed_array_keys as NativeFn,
            ),
            (
                self.strings.intern("entries"),
                super::typed_array_methods::native_typed_array_entries as NativeFn,
            ),
            (
                self.strings.intern("set"),
                super::typed_array_methods::native_typed_array_set as NativeFn,
            ),
            (
                self.strings.intern("copyWithin"),
                super::typed_array_methods::native_typed_array_copy_within as NativeFn,
            ),
            (
                self.strings.intern("reverse"),
                super::typed_array_methods::native_typed_array_reverse as NativeFn,
            ),
            (
                self.strings.intern("indexOf"),
                super::typed_array_methods::native_typed_array_index_of as NativeFn,
            ),
            (
                self.strings.intern("lastIndexOf"),
                super::typed_array_methods::native_typed_array_last_index_of as NativeFn,
            ),
            (
                self.strings.intern("includes"),
                super::typed_array_methods::native_typed_array_includes as NativeFn,
            ),
            (
                self.strings.intern("at"),
                super::typed_array_methods::native_typed_array_at as NativeFn,
            ),
            (
                self.strings.intern("join"),
                super::typed_array_methods::native_typed_array_join as NativeFn,
            ),
            (
                self.strings.intern("forEach"),
                super::typed_array_methods::native_typed_array_for_each as NativeFn,
            ),
            (
                self.strings.intern("every"),
                super::typed_array_methods::native_typed_array_every as NativeFn,
            ),
            (
                self.strings.intern("some"),
                super::typed_array_methods::native_typed_array_some as NativeFn,
            ),
            (
                self.strings.intern("find"),
                super::typed_array_methods::native_typed_array_find as NativeFn,
            ),
            (
                self.strings.intern("findIndex"),
                super::typed_array_methods::native_typed_array_find_index as NativeFn,
            ),
        ];
        for (name_sid, fn_ptr) in methods {
            let name = self.strings.get_utf8(name_sid);
            let fn_id = self.create_native_function(&name, fn_ptr);
            self.define_shaped_property(
                proto_id,
                PropertyKey::String(name_sid),
                PropertyValue::Data(JsValue::Object(fn_id)),
                PropertyAttrs::METHOD,
            );
        }

        // `%TypedArray%.prototype[Symbol.iterator]` — spec-mandated
        // identity-equal to `.values` (ES §23.2.3.33).  Install
        // after `values` so we can reuse the same function id.
        let values_sid = self.strings.intern("values");
        let values_slot =
            super::super::coerce::get_property(self, proto_id, PropertyKey::String(values_sid));
        if let Some(super::super::coerce::PropertyResult::Data(JsValue::Object(values_fn))) =
            values_slot
        {
            self.define_shaped_property(
                proto_id,
                PropertyKey::Symbol(self.well_known_symbols.iterator),
                PropertyValue::Data(JsValue::Object(values_fn)),
                PropertyAttrs::METHOD,
            );
        }

        // `%TypedArray%.prototype.toString` — identity-equal to
        // `Array.prototype.toString` (ES §23.2.3.31 "same built-in
        // function object").  Install by copying the existing
        // function id rather than creating a new native, so
        // `Uint8Array.prototype.toString === Array.prototype.toString`.
        if let Some(array_proto) = self.array_prototype {
            let to_string_sid = self.strings.intern("toString");
            if let Some(super::super::coerce::PropertyResult::Data(JsValue::Object(
                array_to_string,
            ))) = super::super::coerce::get_property(
                self,
                array_proto,
                PropertyKey::String(to_string_sid),
            ) {
                self.define_shaped_property(
                    proto_id,
                    PropertyKey::String(to_string_sid),
                    PropertyValue::Data(JsValue::Object(array_to_string)),
                    PropertyAttrs::METHOD,
                );
            }
        }
    }

    /// Allocate one subclass `Xxx.prototype → %TypedArray%.prototype`
    /// + constructor `Xxx → %TypedArray%` (via prototype link),
    /// install `BYTES_PER_ELEMENT` on both ctor and prototype, and
    /// expose the ctor on `globals`.
    ///
    /// Per PR5b D4 per-interface brand wrappers lesson, each
    /// subclass gets its own static native fn so error messages
    /// carry the subclass name (e.g. `"Failed to construct
    /// 'Uint8Array': …"`).  The wrappers fan into a shared
    /// [`construct_typed_array`] impl parameterised by
    /// [`ElementKind`].
    fn register_typed_array_subclass(
        &mut self,
        entry: &SubclassEntry,
        abstract_proto: ObjectId,
        abstract_ctor: ObjectId,
    ) {
        let sub_proto = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(abstract_proto),
            extensible: true,
        });
        (entry.set_prototype)(self, sub_proto);

        let ctor = self.create_constructable_function(entry.name, entry.ctor_fn);
        // Prototype chain between subclass ctor and abstract ctor —
        // `Object.getPrototypeOf(Uint8Array) === %TypedArray%`
        // (ES §23.2.6).  `create_constructable_function` sets
        // `function_prototype` by default; override to the abstract
        // ctor.
        self.get_object_mut(ctor).prototype = Some(abstract_ctor);

        // Subclass prototype ↔ ctor cross-links.
        let proto_key = PropertyKey::String(self.well_known.prototype);
        self.define_shaped_property(
            ctor,
            proto_key,
            PropertyValue::Data(JsValue::Object(sub_proto)),
            PropertyAttrs::BUILTIN,
        );
        let ctor_key = PropertyKey::String(self.well_known.constructor);
        self.define_shaped_property(
            sub_proto,
            ctor_key,
            PropertyValue::Data(JsValue::Object(ctor)),
            PropertyAttrs::METHOD,
        );

        // `BYTES_PER_ELEMENT` on both prototype and constructor
        // (ES §23.2.6.1 / §23.2.7.1): `{writable: false,
        // enumerable: false, configurable: false}`.
        let bpe_attrs = PropertyAttrs {
            writable: false,
            enumerable: false,
            configurable: false,
            is_accessor: false,
        };
        let bpe_key = PropertyKey::String(self.well_known.bytes_per_element);
        let bpe_value = PropertyValue::Data(JsValue::Number(f64::from(
            entry.element_kind.bytes_per_element(),
        )));
        self.define_shaped_property(ctor, bpe_key, bpe_value, bpe_attrs);
        self.define_shaped_property(sub_proto, bpe_key, bpe_value, bpe_attrs);

        // Global ctor exposure (`Uint8Array`, `Int8Array`, …).
        let name_sid = (entry.global_name)(&self.well_known);
        self.globals.insert(name_sid, JsValue::Object(ctor));
    }
}

// ---------------------------------------------------------------------------
// Subclass dispatch table
// ---------------------------------------------------------------------------

/// Per-subclass registration data, keeping the 11 thin wrapper
/// functions (`native_uint8_array_ctor` et al.) in a compact table
/// rather than a match arm soup in `register_typed_array_prototype_global`.
/// Every field is `Copy` so the static array can live in `.rodata`.
#[derive(Clone, Copy)]
struct SubclassEntry {
    name: &'static str,
    element_kind: ElementKind,
    ctor_fn: NativeFn,
    set_prototype: fn(&mut VmInner, ObjectId),
    global_name: fn(&super::super::well_known::WellKnownStrings) -> StringId,
}

static SUBCLASS_TABLE: &[SubclassEntry] = &[
    SubclassEntry {
        name: "Int8Array",
        element_kind: ElementKind::Int8,
        ctor_fn: native_int8_array_ctor,
        set_prototype: |vm, id| vm.int8_array_prototype = Some(id),
        global_name: |w| w.int8_array_global,
    },
    SubclassEntry {
        name: "Uint8Array",
        element_kind: ElementKind::Uint8,
        ctor_fn: native_uint8_array_ctor,
        set_prototype: |vm, id| vm.uint8_array_prototype = Some(id),
        global_name: |w| w.uint8_array_global,
    },
    SubclassEntry {
        name: "Uint8ClampedArray",
        element_kind: ElementKind::Uint8Clamped,
        ctor_fn: native_uint8_clamped_array_ctor,
        set_prototype: |vm, id| vm.uint8_clamped_array_prototype = Some(id),
        global_name: |w| w.uint8_clamped_array_global,
    },
    SubclassEntry {
        name: "Int16Array",
        element_kind: ElementKind::Int16,
        ctor_fn: native_int16_array_ctor,
        set_prototype: |vm, id| vm.int16_array_prototype = Some(id),
        global_name: |w| w.int16_array_global,
    },
    SubclassEntry {
        name: "Uint16Array",
        element_kind: ElementKind::Uint16,
        ctor_fn: native_uint16_array_ctor,
        set_prototype: |vm, id| vm.uint16_array_prototype = Some(id),
        global_name: |w| w.uint16_array_global,
    },
    SubclassEntry {
        name: "Int32Array",
        element_kind: ElementKind::Int32,
        ctor_fn: native_int32_array_ctor,
        set_prototype: |vm, id| vm.int32_array_prototype = Some(id),
        global_name: |w| w.int32_array_global,
    },
    SubclassEntry {
        name: "Uint32Array",
        element_kind: ElementKind::Uint32,
        ctor_fn: native_uint32_array_ctor,
        set_prototype: |vm, id| vm.uint32_array_prototype = Some(id),
        global_name: |w| w.uint32_array_global,
    },
    SubclassEntry {
        name: "Float32Array",
        element_kind: ElementKind::Float32,
        ctor_fn: native_float32_array_ctor,
        set_prototype: |vm, id| vm.float32_array_prototype = Some(id),
        global_name: |w| w.float32_array_global,
    },
    SubclassEntry {
        name: "Float64Array",
        element_kind: ElementKind::Float64,
        ctor_fn: native_float64_array_ctor,
        set_prototype: |vm, id| vm.float64_array_prototype = Some(id),
        global_name: |w| w.float64_array_global,
    },
    SubclassEntry {
        name: "BigInt64Array",
        element_kind: ElementKind::BigInt64,
        ctor_fn: native_bigint64_array_ctor,
        set_prototype: |vm, id| vm.bigint64_array_prototype = Some(id),
        global_name: |w| w.bigint64_array_global,
    },
    SubclassEntry {
        name: "BigUint64Array",
        element_kind: ElementKind::BigUint64,
        ctor_fn: native_biguint64_array_ctor,
        set_prototype: |vm, id| vm.biguint64_array_prototype = Some(id),
        global_name: |w| w.biguint64_array_global,
    },
];

// ---------------------------------------------------------------------------
// Per-subclass constructor thin wrappers
// ---------------------------------------------------------------------------

// Per-subclass wrappers (vs a single generic ctor with a lookup) are
// what surfaces interface-specific error messages like
// `"Failed to construct 'Uint8Array': …"`.
macro_rules! typed_array_ctor_wrapper {
    ($fn_name:ident, $ek:expr) => {
        fn $fn_name(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            args: &[JsValue],
        ) -> Result<JsValue, VmError> {
            construct_typed_array(ctx, this, args, $ek)
        }
    };
}

typed_array_ctor_wrapper!(native_int8_array_ctor, ElementKind::Int8);
typed_array_ctor_wrapper!(native_uint8_array_ctor, ElementKind::Uint8);
typed_array_ctor_wrapper!(native_uint8_clamped_array_ctor, ElementKind::Uint8Clamped);
typed_array_ctor_wrapper!(native_int16_array_ctor, ElementKind::Int16);
typed_array_ctor_wrapper!(native_uint16_array_ctor, ElementKind::Uint16);
typed_array_ctor_wrapper!(native_int32_array_ctor, ElementKind::Int32);
typed_array_ctor_wrapper!(native_uint32_array_ctor, ElementKind::Uint32);
typed_array_ctor_wrapper!(native_float32_array_ctor, ElementKind::Float32);
typed_array_ctor_wrapper!(native_float64_array_ctor, ElementKind::Float64);
typed_array_ctor_wrapper!(native_bigint64_array_ctor, ElementKind::BigInt64);
typed_array_ctor_wrapper!(native_biguint64_array_ctor, ElementKind::BigUint64);

// ---------------------------------------------------------------------------
// Abstract ctor + species/toStringTag getters
// ---------------------------------------------------------------------------

/// Abstract `%TypedArray%` constructor (ES §23.2.1.1).
/// Throws TypeError in BOTH call-mode and new-mode — the spec
/// explicitly forbids direct invocation of the abstract intrinsic
/// (step 2: `"Abstract class TypedArray not directly
/// constructable"`).  Subclasses (Uint8Array, …) are the only
/// allocating entry points.
fn native_abstract_typed_array_ctor(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Err(VmError::type_error(
        "Abstract class TypedArray not directly constructable",
    ))
}

/// `get %TypedArray% [ @@species ]` (ES §23.2.2.4) — returns `this`.
/// Allocating methods (`map` / `filter` / `slice`) that use
/// SpeciesConstructor read this accessor; subclasses inherit it
/// unchanged, so `Uint8Array[Symbol.species] === Uint8Array`.
fn native_typed_array_species_get(
    _ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(this)
}

/// `%TypedArray%.prototype[@@toStringTag]` getter (ES §23.2.3.32).
/// Reads `[[TypedArrayName]]` — the subclass name string derived
/// from `element_kind`.  Returns `undefined` (NOT throws) if
/// `this` lacks the TypedArray brand, so
/// `Object.prototype.toString.call(plain)` still yields
/// `"[object Object]"` on foreign receivers.
fn native_typed_array_to_tag_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let JsValue::Object(id) = this else {
        return Ok(JsValue::Undefined);
    };
    let ObjectKind::TypedArray { element_kind, .. } = ctx.vm.get_object(id).kind else {
        return Ok(JsValue::Undefined);
    };
    let name_sid = ctx.vm.strings.intern(element_kind.name());
    Ok(JsValue::String(name_sid))
}

// ---------------------------------------------------------------------------
// Generic prototype accessors
// ---------------------------------------------------------------------------

fn require_typed_array_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<ObjectId, VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "TypedArray.prototype.{method} called on non-TypedArray"
        )));
    };
    if matches!(ctx.vm.get_object(id).kind, ObjectKind::TypedArray { .. }) {
        Ok(id)
    } else {
        Err(VmError::type_error(format!(
            "TypedArray.prototype.{method} called on non-TypedArray"
        )))
    }
}

fn native_typed_array_get_buffer(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_typed_array_this(ctx, this, "buffer")?;
    let ObjectKind::TypedArray { buffer_id, .. } = ctx.vm.get_object(id).kind else {
        unreachable!("brand-check passed");
    };
    Ok(JsValue::Object(buffer_id))
}

fn native_typed_array_get_byte_offset(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_typed_array_this(ctx, this, "byteOffset")?;
    let ObjectKind::TypedArray { byte_offset, .. } = ctx.vm.get_object(id).kind else {
        unreachable!("brand-check passed");
    };
    Ok(JsValue::Number(f64::from(byte_offset)))
}

fn native_typed_array_get_byte_length(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_typed_array_this(ctx, this, "byteLength")?;
    let ObjectKind::TypedArray { byte_length, .. } = ctx.vm.get_object(id).kind else {
        unreachable!("brand-check passed");
    };
    Ok(JsValue::Number(f64::from(byte_length)))
}

fn native_typed_array_get_length(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_typed_array_this(ctx, this, "length")?;
    let ObjectKind::TypedArray {
        byte_length,
        element_kind,
        ..
    } = ctx.vm.get_object(id).kind
    else {
        unreachable!("brand-check passed");
    };
    let len = u32::from(element_kind.bytes_per_element())
        .max(1)
        .try_into()
        .map(|bpe: u32| byte_length / bpe)
        .unwrap_or(0);
    Ok(JsValue::Number(f64::from(len)))
}

// ---------------------------------------------------------------------------
// Shared constructor dispatch (ES §23.2.5)
// ---------------------------------------------------------------------------

/// Shared body of every TypedArray subclass ctor.  Dispatches on
/// `args[0]` shape per ES §23.2.5:
/// 1. `() / (undefined)` → empty view over fresh zero-byte buffer.
/// 2. `(number)` → `ToIndex(n)`, fresh zero-filled buffer of
///    `n * bpe` bytes (§23.2.5.1.1).
/// 3. `(ArrayBuffer, byteOffset?, length?)` → share buffer bytes,
///    validate alignment (`byteOffset % bpe === 0` — RangeError)
///    and range (§23.2.5.1.3).
/// 4. `(TypedArray)` → fresh buffer of `src.length * dst.bpe` bytes,
///    element-copy with type conversion (§23.2.5.1.2).
/// 5. `(iterable)` where `@@iterator` resolves → consume iterator,
///    allocate buffer, write each element (§23.2.5.1.4).  Any
///    throw during body is closed with `IteratorClose` per
///    §7.4.6 (but a throw from `iter_next` itself is NOT
///    closed — §7.4.7, PR5b R13 lesson).
/// 6. Otherwise → TypeError.
///
/// The pre-allocated receiver carries `new.target.prototype` via
/// `do_new`; we promote its `kind` in-place rather than reassigning
/// `prototype`, so subclasses of our builtins (`class X extends
/// Uint8Array`) inherit correctly (PR5a2 R7.2/R7.3 lesson).
fn construct_typed_array(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
    ek: ElementKind,
) -> Result<JsValue, VmError> {
    if !ctx.is_construct() {
        return Err(VmError::type_error(format!(
            "Failed to construct '{}': Please use the 'new' operator",
            ek.name()
        )));
    }
    let JsValue::Object(inst_id) = this else {
        unreachable!("constructor `this` is always an Object after `do_new`");
    };

    let arg0 = args.first().copied().unwrap_or(JsValue::Undefined);
    let arg1 = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let arg2 = args.get(2).copied().unwrap_or(JsValue::Undefined);

    let (buffer_id, byte_offset, byte_length) = match arg0 {
        JsValue::Undefined => allocate_fresh_buffer(ctx, 0)?,
        JsValue::Object(src_id) => match ctx.vm.get_object(src_id).kind {
            ObjectKind::ArrayBuffer => init_from_array_buffer(ctx, src_id, arg1, arg2, ek)?,
            ObjectKind::TypedArray { .. } => init_from_typed_array(ctx, src_id, ek)?,
            _ => init_from_iterable(ctx, arg0, ek)?,
        },
        // Plain number (or anything number-coercible via ToNumber)
        // → length form.  Strings like `"5"` coerce too; this
        // matches V8 / SpiderMonkey.  NaN → 0-length per ToIndex.
        _ => {
            let n = ctx.to_number(arg0)?;
            let length = to_index_u32(n, ek.name(), "length")?;
            let byte_len = length
                .checked_mul(u32::from(ek.bytes_per_element()))
                .ok_or_else(|| {
                    VmError::range_error(format!(
                        "Failed to construct '{}': length too large",
                        ek.name()
                    ))
                })?;
            allocate_fresh_buffer(ctx, byte_len)?
        }
    };

    // Preserve `prototype` on the pre-allocated instance so
    // `new.target.prototype` chains work for subclasses (PR5a2 R7.2/R7.3).
    ctx.vm.get_object_mut(inst_id).kind = ObjectKind::TypedArray {
        buffer_id,
        byte_offset,
        byte_length,
        element_kind: ek,
    };
    Ok(JsValue::Object(inst_id))
}

/// Allocate a fresh `ArrayBuffer` of `byte_len` zero bytes and
/// return its `(ObjectId, byte_offset=0, byte_length)` triple.
/// Uses the shared `body_data` store so GC sweep prunes it
/// alongside other ArrayBuffers.
pub(super) fn allocate_fresh_buffer(
    ctx: &mut NativeContext<'_>,
    byte_len: u32,
) -> Result<(ObjectId, u32, u32), VmError> {
    let bytes: Arc<[u8]> = if byte_len == 0 {
        Arc::from(&[][..])
    } else {
        vec![0_u8; byte_len as usize].into()
    };
    let buf_id = super::array_buffer::create_array_buffer_from_bytes(ctx.vm, bytes);
    Ok((buf_id, 0, byte_len))
}

/// Variant 3: share an existing `ArrayBuffer`.  Validates
/// `byteOffset % bpe === 0` (RangeError) and range coverage
/// (`byteOffset + byteLength ≤ buffer.byteLength`).  `length`
/// in elements is multiplied by `bpe` to derive the byte count.
fn init_from_array_buffer(
    ctx: &mut NativeContext<'_>,
    buffer_id: ObjectId,
    byte_offset_arg: JsValue,
    length_arg: JsValue,
    ek: ElementKind,
) -> Result<(ObjectId, u32, u32), VmError> {
    let bpe = u32::from(ek.bytes_per_element());
    let buf_len_usize = super::array_buffer::array_buffer_byte_length(ctx.vm, buffer_id);
    // Clamp the buffer byte length into `u32` — anything larger
    // can't be addressed by the `[[ByteLength]]` slot anyway
    // (u32 on the TypedArray variant).
    let buf_len: u32 = buf_len_usize.try_into().map_err(|_| {
        VmError::range_error(format!(
            "Failed to construct '{}': ArrayBuffer is larger than 4 GiB",
            ek.name()
        ))
    })?;

    let byte_offset = match byte_offset_arg {
        JsValue::Undefined => 0,
        other => {
            let n = ctx.to_number(other)?;
            to_index_u32(n, ek.name(), "byteOffset")?
        }
    };
    if byte_offset % bpe != 0 {
        return Err(VmError::range_error(format!(
            "Failed to construct '{}': start offset must be a multiple of {bpe}",
            ek.name()
        )));
    }
    if byte_offset > buf_len {
        return Err(VmError::range_error(format!(
            "Failed to construct '{}': byteOffset {byte_offset} exceeds ArrayBuffer length {buf_len}",
            ek.name()
        )));
    }

    let byte_length = match length_arg {
        JsValue::Undefined => {
            // Auto-length: `buffer.byteLength - byteOffset`, and
            // that remainder must itself be aligned.
            let remainder = buf_len - byte_offset;
            if remainder % bpe != 0 {
                return Err(VmError::range_error(format!(
                    "Failed to construct '{}': byte length of buffer should be a multiple of {bpe}",
                    ek.name()
                )));
            }
            remainder
        }
        other => {
            let n = ctx.to_number(other)?;
            let length = to_index_u32(n, ek.name(), "length")?;
            let byte_len = length.checked_mul(bpe).ok_or_else(|| {
                VmError::range_error(format!(
                    "Failed to construct '{}': length too large",
                    ek.name()
                ))
            })?;
            if byte_offset
                .checked_add(byte_len)
                .map_or(true, |end| end > buf_len)
            {
                return Err(VmError::range_error(format!(
                    "Failed to construct '{}': length out of range of buffer",
                    ek.name()
                )));
            }
            byte_len
        }
    };

    Ok((buffer_id, byte_offset, byte_length))
}

/// Variant 4: `new Xxx(otherTypedArray)`.  Allocates a fresh
/// buffer sized to the source's `length`, then copies each
/// element through the destination's type coercion.  Source and
/// destination may have different ElementKinds (e.g. `new
/// Uint8Array(new Float32Array([1.7, 2.9]))` coerces to `[1, 2]`).
fn init_from_typed_array(
    ctx: &mut NativeContext<'_>,
    src_id: ObjectId,
    dst_ek: ElementKind,
) -> Result<(ObjectId, u32, u32), VmError> {
    let (src_buf_id, src_offset, src_byte_len, src_ek) = match ctx.vm.get_object(src_id).kind {
        ObjectKind::TypedArray {
            buffer_id,
            byte_offset,
            byte_length,
            element_kind,
        } => (buffer_id, byte_offset, byte_length, element_kind),
        _ => unreachable!("caller confirmed TypedArray kind"),
    };
    // Content-type compatibility: BigInt ↔ Number mixing throws
    // TypeError per ES §23.2.5.1.2 step 17 (subclass check).
    if src_ek.is_bigint() != dst_ek.is_bigint() {
        return Err(VmError::type_error(format!(
            "Failed to construct '{}': Cannot mix BigInt and other types",
            dst_ek.name()
        )));
    }
    let src_len_elem = src_byte_len / u32::from(src_ek.bytes_per_element());
    let dst_byte_len = src_len_elem
        .checked_mul(u32::from(dst_ek.bytes_per_element()))
        .ok_or_else(|| {
            VmError::range_error(format!(
                "Failed to construct '{}': length too large",
                dst_ek.name()
            ))
        })?;
    let (dst_buf_id, dst_offset, _) = allocate_fresh_buffer(ctx, dst_byte_len)?;

    // Element-by-element copy with per-kind coercion.  Same
    // underlying buffer case (`src_buf_id == dst_buf_id`) isn't
    // possible here because `dst_buf_id` is freshly allocated.
    // BigInt read path needs &mut VmInner for the alloc — the
    // two borrows never overlap because `read_element_raw` returns
    // a JsValue before `write_element_raw` touches the VM again.
    for i in 0..src_len_elem {
        let elem = read_element_raw(ctx.vm, src_buf_id, src_offset, i, src_ek);
        write_element_raw(ctx, dst_buf_id, dst_offset, i, dst_ek, elem)?;
    }

    Ok((dst_buf_id, dst_offset, dst_byte_len))
}

/// Variant 5: `new Xxx(object)`.  ES §23.2.5.1 steps 7-12: if the
/// source has a callable `@@iterator`, iterate; otherwise fall back
/// to the array-like path (`length` + integer-indexed `[[Get]]`s).
fn init_from_iterable(
    ctx: &mut NativeContext<'_>,
    source: JsValue,
    ek: ElementKind,
) -> Result<(ObjectId, u32, u32), VmError> {
    let JsValue::Object(src_id) = source else {
        unreachable!("init_from_iterable called on non-Object");
    };
    // §23.2.5.1 step 7 / GetMethod(object, @@iterator): an absent
    // or `undefined` / `null` value for `@@iterator` falls through
    // to the array-like branch rather than throwing.
    let iter_key = super::super::value::PropertyKey::Symbol(ctx.vm.well_known_symbols.iterator);
    let using_iter = match super::super::coerce::get_property(ctx.vm, src_id, iter_key) {
        Some(pr) => ctx.vm.resolve_property(pr, source)?,
        None => JsValue::Undefined,
    };
    if matches!(using_iter, JsValue::Null | JsValue::Undefined) {
        return init_from_array_like(ctx, src_id, ek);
    }
    let iter = match ctx.vm.call_value(using_iter, source, &[])? {
        it @ JsValue::Object(_) => it,
        _ => {
            return Err(VmError::type_error(format!(
                "Failed to construct '{}': @@iterator must return an object",
                ek.name()
            )));
        }
    };

    // Collect elements using the stack as GC-safe scratch: each
    // `iter_next` may execute user code that triggers GC, and Rust
    // locals holding `JsValue`s are invisible to the scanner.  The
    // iterator itself lives on the stack below the elements so it
    // survives any intervening GC even when the outer `args` slice
    // no longer reaches it (the `@@iterator` call's return value is
    // freshly allocated and not transitively rooted via `source`).
    // Every exit path — success, RangeError, or a `?` propagation
    // from inside `iter_next` / `write_element_raw` — truncates
    // back to `iter_slot` via the outer-scope helper call.
    let iter_slot = ctx.vm.stack.len();
    ctx.vm.stack.push(iter);
    let elem_start = iter_slot + 1;
    let outcome = init_from_iterable_body(ctx, iter, elem_start, ek);
    ctx.vm.stack.truncate(iter_slot);
    outcome
}

/// Inner body of [`init_from_iterable`], extracted so the caller can
/// truncate the GC-rooting stack prefix unconditionally on every
/// exit (spec `?` throws + inlined RangeError short-circuits alike).
fn init_from_iterable_body(
    ctx: &mut NativeContext<'_>,
    iter: JsValue,
    elem_start: usize,
    ek: ElementKind,
) -> Result<(ObjectId, u32, u32), VmError> {
    loop {
        let next = ctx.vm.iter_next(iter)?;
        match next {
            Some(v) => ctx.vm.stack.push(v),
            None => break,
        }
    }
    let count = ctx.vm.stack.len() - elem_start;
    let count_u32 = u32::try_from(count).map_err(|_| {
        VmError::range_error(format!(
            "Failed to construct '{}': too many elements in source iterable",
            ek.name()
        ))
    })?;
    let byte_len = count_u32
        .checked_mul(u32::from(ek.bytes_per_element()))
        .ok_or_else(|| {
            VmError::range_error(format!(
                "Failed to construct '{}': length too large",
                ek.name()
            ))
        })?;
    let (buf_id, offset, _) = allocate_fresh_buffer(ctx, byte_len)?;

    // Drain elements off the stack into the buffer.  A throw
    // during element write (e.g. `ToBigInt` on a Number for a
    // BigInt64Array) is a body-level abrupt completion — the
    // iterator has already been drained to exhaustion above, so
    // there is nothing to `IteratorClose`.  (IteratorClose is
    // only relevant when we exit MID-iteration — `iter_next`
    // throw is spec-exempt per §7.4.7, and the full-drain
    // pattern here never leaves the iterator open.)
    for i in 0..count_u32 {
        let elem = ctx.vm.stack[elem_start + i as usize];
        write_element_raw(ctx, buf_id, offset, i, ek, elem)?;
    }

    Ok((buf_id, offset, byte_len))
}

/// Variant 5b: §23.2.5.1 array-like fallback when `source` has no
/// callable `@@iterator`.  Reads `source.length` → `ToIndex` →
/// allocates buffer → drains `source[i]` through the shared property
/// path (§23.2.5.1 steps 9-12).
fn init_from_array_like(
    ctx: &mut NativeContext<'_>,
    src_id: ObjectId,
    ek: ElementKind,
) -> Result<(ObjectId, u32, u32), VmError> {
    let length_sid = ctx.vm.well_known.length;
    let len_val =
        ctx.get_property_value(src_id, super::super::value::PropertyKey::String(length_sid))?;
    let len_f = ctx.to_number(len_val)?;
    let length = to_index_u32(len_f, ek.name(), "length")?;
    let byte_len = length
        .checked_mul(u32::from(ek.bytes_per_element()))
        .ok_or_else(|| {
            VmError::range_error(format!(
                "Failed to construct '{}': length too large",
                ek.name()
            ))
        })?;
    let (buf_id, offset, _) = allocate_fresh_buffer(ctx, byte_len)?;
    let source = JsValue::Object(src_id);
    for i in 0..length {
        // `get_element` dispatches through the full element-get
        // pipeline (Array dense, TypedArray integer-indexed,
        // prototype chain), matching what a plain `source[i]` would
        // see from user code.
        let elem = ctx.vm.get_element(source, JsValue::Number(f64::from(i)))?;
        write_element_raw(ctx, buf_id, offset, i, ek, elem)?;
    }
    Ok((buf_id, offset, byte_len))
}

// ---------------------------------------------------------------------------
// Low-level element read/write (shared with indexed access in C3)
// ---------------------------------------------------------------------------

/// Read the element at `index` from the buffer backing a
/// TypedArray, decoded per `ek`.  Little-endian byte order
/// (elidex convention, documented at module header).  Missing
/// body_data entry (e.g. freshly allocated zero-byte buffer) is
/// treated as all zeros.
///
/// Takes `&mut VmInner` because BigInt element reads need to
/// `alloc` a BigIntId on the interning pool.  For non-BigInt
/// subclasses no allocation occurs.
pub(crate) fn read_element_raw(
    vm: &mut VmInner,
    buffer_id: ObjectId,
    byte_offset: u32,
    index: u32,
    ek: ElementKind,
) -> JsValue {
    let bpe = u32::from(ek.bytes_per_element());
    let abs = (byte_offset + index * bpe) as usize;
    // Snapshot up to 8 bytes (max element size) into scratch so
    // the subsequent `alloc` (BigInt branch) doesn't conflict with
    // a live borrow of `body_data`.
    let mut scratch = [0_u8; 8];
    let scratch_len = bpe as usize;
    if let Some(bytes) = vm.body_data.get(&buffer_id) {
        if let Some(slice) = bytes.get(abs..abs + scratch_len) {
            scratch[..scratch_len].copy_from_slice(slice);
        }
    }
    match ek {
        ElementKind::Int8 => JsValue::Number(f64::from(scratch[0] as i8)),
        ElementKind::Uint8 | ElementKind::Uint8Clamped => JsValue::Number(f64::from(scratch[0])),
        ElementKind::Int16 => {
            let v = i16::from_le_bytes([scratch[0], scratch[1]]);
            JsValue::Number(f64::from(v))
        }
        ElementKind::Uint16 => {
            let v = u16::from_le_bytes([scratch[0], scratch[1]]);
            JsValue::Number(f64::from(v))
        }
        ElementKind::Int32 => {
            let v = i32::from_le_bytes([scratch[0], scratch[1], scratch[2], scratch[3]]);
            JsValue::Number(f64::from(v))
        }
        ElementKind::Uint32 => {
            let v = u32::from_le_bytes([scratch[0], scratch[1], scratch[2], scratch[3]]);
            JsValue::Number(f64::from(v))
        }
        ElementKind::Float32 => {
            let v = f32::from_le_bytes([scratch[0], scratch[1], scratch[2], scratch[3]]);
            JsValue::Number(f64::from(v))
        }
        ElementKind::Float64 => JsValue::Number(f64::from_le_bytes(scratch)),
        ElementKind::BigInt64 => {
            let v = i64::from_le_bytes(scratch);
            let bi = num_bigint::BigInt::from(v);
            JsValue::BigInt(vm.bigints.alloc(bi))
        }
        ElementKind::BigUint64 => {
            let v = u64::from_le_bytes(scratch);
            let bi = num_bigint::BigInt::from(v);
            JsValue::BigInt(vm.bigints.alloc(bi))
        }
    }
}

/// Write `value` into the buffer at `index`, coerced per `ek`.
/// Returns `Err` when BigInt coercion fails (writing a Number into
/// a `BigInt64Array` — `ToBigInt` rejects) or when user-level
/// coercion (valueOf / Symbol.toPrimitive) throws.
///
/// The backing `Arc<[u8]>` is treated as immutable: every write
/// produces a fresh `Arc::from(Vec)` that replaces the map entry.
/// This keeps the spec-mandated "views share bytes" invariant —
/// every view reads through `body_data[buffer_id]`, which always
/// holds the latest snapshot.  The O(N)-per-write cost is
/// acceptable for C2; a byte-level interior-mutability refactor
/// lands with PR-spec-polish SP9.
pub(crate) fn write_element_raw(
    ctx: &mut NativeContext<'_>,
    buffer_id: ObjectId,
    byte_offset: u32,
    index: u32,
    ek: ElementKind,
    value: JsValue,
) -> Result<(), VmError> {
    let bpe = u32::from(ek.bytes_per_element());
    let abs = (byte_offset + index * bpe) as usize;

    // Coerce first — user code may run (valueOf / toPrimitive)
    // and throw.  Scratch buffer holds the encoded little-endian
    // bytes; the actual write (which replaces the Arc entry)
    // only runs if coercion succeeds.
    let mut scratch = [0_u8; 8];
    let written_len = match ek {
        ElementKind::Int8 => {
            let v = super::super::coerce::to_int8(ctx.vm, value)?;
            scratch[0] = v as u8;
            1
        }
        ElementKind::Uint8 => {
            let v = super::super::coerce::to_uint8(ctx.vm, value)?;
            scratch[0] = v;
            1
        }
        ElementKind::Uint8Clamped => {
            let v = super::super::coerce::to_uint8_clamp(ctx.vm, value)?;
            scratch[0] = v;
            1
        }
        ElementKind::Int16 => {
            let v = super::super::coerce::to_int16(ctx.vm, value)?;
            scratch[..2].copy_from_slice(&v.to_le_bytes());
            2
        }
        ElementKind::Uint16 => {
            let v = super::super::coerce::to_uint16(ctx.vm, value)?;
            scratch[..2].copy_from_slice(&v.to_le_bytes());
            2
        }
        ElementKind::Int32 => {
            let v = super::super::coerce::to_int32(ctx.vm, value)?;
            scratch[..4].copy_from_slice(&v.to_le_bytes());
            4
        }
        ElementKind::Uint32 => {
            let v = super::super::coerce::to_uint32(ctx.vm, value)?;
            scratch[..4].copy_from_slice(&v.to_le_bytes());
            4
        }
        ElementKind::Float32 => {
            let n = super::super::coerce::to_number(ctx.vm, value)?;
            #[allow(clippy::cast_possible_truncation)]
            let v = n as f32;
            scratch[..4].copy_from_slice(&v.to_le_bytes());
            4
        }
        ElementKind::Float64 => {
            let n = super::super::coerce::to_number(ctx.vm, value)?;
            scratch[..8].copy_from_slice(&n.to_le_bytes());
            8
        }
        ElementKind::BigInt64 => {
            let v = super::super::natives_bigint::to_bigint64(ctx, value)?;
            scratch[..8].copy_from_slice(&v.to_le_bytes());
            8
        }
        ElementKind::BigUint64 => {
            let v = super::super::natives_bigint::to_biguint64(ctx, value)?;
            scratch[..8].copy_from_slice(&v.to_le_bytes());
            8
        }
    };

    let needed_len = abs + written_len;
    // Clone the existing buffer into a fresh `Vec<u8>`, grow if
    // needed, apply the element write, install the new Arc.
    // Other views over the same `buffer_id` read via
    // `body_data.get(&buffer_id)` and will see the new bytes on
    // their next access.
    let current: &[u8] = ctx
        .vm
        .body_data
        .get(&buffer_id)
        .map(AsRef::as_ref)
        .unwrap_or(&[]);
    let mut new_bytes: Vec<u8> = current.to_vec();
    if new_bytes.len() < needed_len {
        new_bytes.resize(needed_len, 0);
    }
    new_bytes[abs..abs + written_len].copy_from_slice(&scratch[..written_len]);
    ctx.vm.body_data.insert(buffer_id, Arc::from(new_bytes));
    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// `ToIndex` (ES §7.1.22) with `u32` target.  NaN → 0, fractional
/// values truncate toward zero per `ToIntegerOrInfinity`, and
/// negative / non-finite / > u32::MAX values throw `RangeError`.
/// Used by TypedArray ctor length / byteOffset / byteLength args
/// (each spec'd as `unsigned long long` + `[EnforceRange]`, but the
/// u32-bound byte_length slot constrains us to u32 here).
fn to_index_u32(n: f64, ctor_name: &str, what: &str) -> Result<u32, VmError> {
    if n.is_nan() {
        return Ok(0);
    }
    let truncated = n.trunc();
    if !truncated.is_finite() || truncated < 0.0 {
        return Err(VmError::range_error(format!(
            "Failed to construct '{ctor_name}': {what} must be a non-negative safe integer"
        )));
    }
    if truncated > f64::from(u32::MAX) {
        return Err(VmError::range_error(format!(
            "Failed to construct '{ctor_name}': {what} exceeds the supported maximum"
        )));
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let as_u32 = truncated as u32;
    Ok(as_u32)
}
