//! `%TypedArray%.of` / `%TypedArray%.from` static method bodies
//! (ES2024 §23.2.2).
//!
//! Both natives are installed on the abstract `%TypedArray%`
//! constructor — subclass ctors (`Uint8Array`, `Float64Array`, …)
//! pick them up via the constructor prototype chain
//! (`Object.getPrototypeOf(Uint8Array) === %TypedArray%`).  At call
//! time `this` is the calling subclass ctor; we resolve it back
//! to its [`ElementKind`] via [`ctor_to_element_kind`] (linear
//! scan over [`super::super::VmInner::subclass_array_ctors`]) and
//! materialise a fresh subclass instance.
//!
//! ## Scope (SP8a)
//!
//! - `%TypedArray%.of(...items)` (§23.2.2.2)
//! - `%TypedArray%.from(source, mapFn?, thisArg?)` (§23.2.2.1)
//!
//! Species-sensitive instance methods (`map`, `filter`, `findLast`,
//! `findLastIndex`, `sort`, `reduce`, `reduceRight`, `flatMap`,
//! `toLocaleString`) and the supporting [`SpeciesConstructor`] /
//! [`IsConstructor`] machinery land in SP8b/c.

#![cfg(feature = "engine")]

use super::super::value::{
    ElementKind, JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey,
    PropertyStorage, VmError,
};
use super::super::{shape, VmInner};
use super::typed_array::{allocate_fresh_buffer, write_element_raw};
use super::typed_array_methods::subclass_prototype_for;

/// Resolve a constructor `ObjectId` to its `ElementKind` by linear
/// scan over [`VmInner::subclass_array_ctors`].  Used by
/// [`native_typed_array_of`] / [`native_typed_array_from`] to
/// dispatch on `this` when invoked through a subclass ctor inherited
/// `of` / `from`.  Returns `None` for the abstract `%TypedArray%`
/// itself (registered separately) and for any other receiver — both
/// are treated by the callers as a TypeError ("not a TypedArray
/// constructor").
fn ctor_to_element_kind(vm: &VmInner, ctor_id: ObjectId) -> Option<ElementKind> {
    vm.subclass_array_ctors
        .iter()
        .position(|slot| *slot == Some(ctor_id))
        .and_then(ElementKind::from_index)
}

/// Resolve `this` (the caller of `%TypedArray%.of` / `.from`)
/// into the destination `(ElementKind, prototype_for_new_instance)`
/// pair.
///
/// For a built-in subclass ctor (`Uint8Array`, `Float64Array`,
/// …), `ek` comes from the direct `subclass_array_ctors` hit and
/// the new instance's prototype is the built-in subclass
/// prototype (`subclass_array_prototypes[ek.index()]`).
///
/// For a user-defined subclass (`class Sub extends Uint8Array
/// {}`), `Sub` itself is not in the registry — we walk the
/// constructor's `[[Prototype]]` chain to find the nearest
/// built-in TypedArray ctor (which gives the destination `ek`)
/// and then read the receiver's own `.prototype` data property
/// to use as the new instance's prototype (so
/// `(new Sub.of(...)).constructor === Sub` holds).  The full
/// spec `TypedArrayCreate(C, ⟨len⟩)` would invoke `Construct(C,
/// ⟨len⟩)` to let `Sub`'s ctor body run — that final step is
/// deferred to a follow-up PR which threads `new.target`
/// through; for the common `class Sub extends Uint8Array {}`
/// (no ctor override) the bypass is observably equivalent.
///
/// The abstract `%TypedArray%` itself and any other receiver
/// surface TypeError per §23.2.2.{1,2} step "If IsConstructor(C)
/// is false, throw".
fn require_subclass_ctor(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<(ElementKind, Option<ObjectId>), VmError> {
    let JsValue::Object(ctor_id) = this else {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'TypedArray': this is not a TypedArray constructor"
        )));
    };
    // Spec §23.2.2.{1,2} step "If IsConstructor(C) is false, throw
    // TypeError" — reject plain objects, arrow / async / generator
    // functions, and non-constructable natives BEFORE walking the
    // prototype chain.  Without this gate, a prototype-spoofed
    // receiver (e.g. `Object.setPrototypeOf({}, Uint8Array);
    // Uint8Array.of.call(o, 1)`) would slip through because the
    // walk finds Uint8Array at depth 1 and the receiver's
    // `prototype` data property resolution falls back to the
    // built-in subclass prototype.
    if !super::super::object_kind::is_constructor(ctx.vm, ctor_id) {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'TypedArray': this is not a TypedArray constructor"
        )));
    }
    // Walk the constructor `[[Prototype]]` chain looking for a
    // registered built-in TypedArray ctor.  No depth cap — a small
    // visited set catches `__proto__ = self`-style cycles without
    // imposing an arbitrary limit on legitimate deep subclass
    // towers (`class A extends Uint8Array {}; class B extends A
    // {}; …`).  Realistic chains are 1-3 entries; the
    // allocation is below measurement noise.
    let mut visited: std::collections::HashSet<ObjectId> = std::collections::HashSet::new();
    let mut current = Some(ctor_id);
    while let Some(id) = current {
        if !visited.insert(id) {
            // Cycle (`A.__proto__.__proto__... === A`) — give up
            // before re-traversing the same node forever.
            break;
        }
        if let Some(ek) = ctor_to_element_kind(ctx.vm, id) {
            // Found a registered ctor in the chain.  If the
            // original receiver IS the registered ctor, hand back
            // `None` to defer prototype resolution to
            // `subclass_prototype_for` (the standard path).
            // Otherwise (user-defined subclass), read the receiver's
            // own `.prototype` data property to preserve subclass
            // identity on the new instance.
            let proto_override = if id == ctor_id {
                None
            } else {
                receiver_prototype(ctx, ctor_id, ek)
            };
            return Ok((ek, proto_override));
        }
        current = ctx.vm.get_object(id).prototype;
    }
    Err(VmError::type_error(format!(
        "Failed to execute '{method}' on 'TypedArray': this is not a TypedArray constructor"
    )))
}

/// Read `ctor.prototype` to obtain the prototype object that the
/// new TypedArray instance should inherit from.  Used by the
/// user-subclass branch of [`require_subclass_ctor`] so
/// `(class Sub extends Uint8Array {}).of(1, 2).constructor ===
/// Sub` holds.  Falls back to the built-in subclass prototype
/// if the receiver's `.prototype` slot is missing or non-Object
/// (defensive — the JS subclass ctor protocol always installs a
/// data `.prototype` property; only highly contrived
/// `Reflect.deleteProperty(Sub, 'prototype')` would expose this).
fn receiver_prototype(
    ctx: &NativeContext<'_>,
    ctor_id: ObjectId,
    ek: ElementKind,
) -> Option<ObjectId> {
    let proto_key = PropertyKey::String(ctx.vm.well_known.prototype);
    if let Some(super::super::coerce::PropertyResult::Data(JsValue::Object(p))) =
        super::super::coerce::get_property(ctx.vm, ctor_id, proto_key)
    {
        return Some(p);
    }
    subclass_prototype_for(ctx.vm, ek)
}

/// Allocate a fresh `<ek>Array(len)`-shaped TypedArray instance:
/// new buffer of `len * bpe` bytes, fresh view at `byte_offset = 0`
/// covering the whole buffer.  Approximates the spec's
/// `TypedArrayCreate(constructor, ⟨len⟩)` (§22.2.4.2.1).
///
/// `proto_override` selects the new instance's `[[Prototype]]`:
/// `None` uses the built-in subclass prototype for `ek` (the
/// path taken when `this` is the registered subclass ctor itself);
/// `Some(p)` uses `p` as the prototype (the user-subclass path —
/// `class Sub extends Uint8Array {}` resolves `p =
/// Sub.prototype` so `(new Sub.of(...)).constructor === Sub`).
fn create_typed_array_for_length(
    ctx: &mut NativeContext<'_>,
    ek: ElementKind,
    proto_override: Option<ObjectId>,
    len: u32,
) -> Result<ObjectId, VmError> {
    let bpe = u32::from(ek.bytes_per_element());
    let byte_len = len.checked_mul(bpe).ok_or_else(|| {
        VmError::range_error(format!(
            "Failed to allocate '{}': length too large",
            ek.name()
        ))
    })?;
    let (buf_id, _, _) = allocate_fresh_buffer(ctx, byte_len)?;
    // Root the freshly allocated buffer across the subsequent view
    // alloc — `alloc_object` could otherwise reclaim the buffer's
    // slot before the view links it.  Same RAII rooting pattern as
    // `init_from_typed_array` / `init_from_iterable_body`.
    let mut g = ctx.vm.push_temp_root(JsValue::Object(buf_id));
    let prototype = proto_override.or_else(|| subclass_prototype_for(&g, ek));
    let view_id = g.alloc_object(Object {
        kind: ObjectKind::TypedArray {
            buffer_id: buf_id,
            byte_offset: 0,
            byte_length: byte_len,
            element_kind: ek,
        },
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype,
        extensible: true,
    });
    drop(g);
    Ok(view_id)
}

/// `%TypedArray%.of(...items)` (ES §23.2.2.2).
///
/// Allocates `new this(items.length)` and writes each `items[k]`
/// into the new TypedArray's `[k]` slot via the spec-mandated
/// per-element coerce ([`super::typed_array::write_element_raw`]).
/// `this` must be a built-in TypedArray subclass constructor —
/// the abstract `%TypedArray%` and any other receiver throws
/// TypeError ("If IsConstructor(C) is false").
pub(crate) fn native_typed_array_of(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (ek, proto_override) = require_subclass_ctor(ctx, this, "of")?;
    let len = u32::try_from(args.len()).map_err(|_| {
        VmError::range_error(format!(
            "Failed to execute 'of' on '{}': too many items",
            ek.name()
        ))
    })?;
    let view_id = create_typed_array_for_length(ctx, ek, proto_override, len)?;
    // Root the view across element writes (each `write_element_raw`
    // may run user-level `valueOf` / `Symbol.toPrimitive` and, on
    // BigInt subclasses, allocate fresh `BigIntId`s — both can
    // currently never trigger GC inside natives, but the rooting
    // matches the wider invariant).
    let mut g = ctx.vm.push_temp_root(JsValue::Object(view_id));
    let mut sub_ctx = NativeContext { vm: &mut g };
    let (buf_id, byte_offset) = match sub_ctx.vm.get_object(view_id).kind {
        ObjectKind::TypedArray {
            buffer_id,
            byte_offset,
            ..
        } => (buffer_id, byte_offset),
        _ => unreachable!("create_typed_array_for_length always produces ObjectKind::TypedArray"),
    };
    for (i, value) in args.iter().enumerate() {
        #[allow(clippy::cast_possible_truncation)]
        let idx = i as u32;
        write_element_raw(&mut sub_ctx, buf_id, byte_offset, idx, ek, *value)?;
    }
    drop(g);
    Ok(JsValue::Object(view_id))
}

/// `%TypedArray%.from(source, mapFn?, thisArg?)` (ES §23.2.2.1).
///
/// Iterates `source` (callable `@@iterator` first, falling back
/// to the array-like `length` + integer-indexed `[[Get]]` path),
/// optionally applies `mapFn(value, index)` per element with
/// `thisArg` as the callback's `this`, and writes the (possibly
/// mapped) values into a freshly allocated subclass instance whose
/// length matches the consumed element count.
///
/// `this` must be a built-in TypedArray subclass constructor;
/// `mapFn`, if present, must be callable.
pub(crate) fn native_typed_array_from(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (ek, proto_override) = require_subclass_ctor(ctx, this, "from")?;
    let source = args.first().copied().unwrap_or(JsValue::Undefined);
    let map_fn = match args.get(1).copied().unwrap_or(JsValue::Undefined) {
        JsValue::Undefined => None,
        JsValue::Object(id) if ctx.get_object(id).kind.is_callable() => Some(id),
        _ => {
            return Err(VmError::type_error(format!(
                "Failed to execute 'from' on '{}': mapFn is not a function",
                ek.name()
            )));
        }
    };
    let this_arg = args.get(2).copied().unwrap_or(JsValue::Undefined);

    // Drain `source` into a `Vec<JsValue>` first — the source-side
    // pass (iterator drain or array-like read) is decoupled from
    // the destination-side write so the two can run independently
    // (the destination buffer is sized once, after the count is
    // known, and rooted across the per-element write loop).  Mirror
    // of the `init_from_iterable` / `init_from_array_like` shape
    // in `typed_array_ctor.rs`.
    let values = collect_source_values(ctx, source, map_fn, this_arg, ek)?;
    let len = u32::try_from(values.len()).map_err(|_| {
        VmError::range_error(format!(
            "Failed to execute 'from' on '{}': too many elements in source",
            ek.name()
        ))
    })?;

    let view_id = create_typed_array_for_length(ctx, ek, proto_override, len)?;
    let mut g = ctx.vm.push_temp_root(JsValue::Object(view_id));
    let mut sub_ctx = NativeContext { vm: &mut g };
    let (buf_id, byte_offset) = match sub_ctx.vm.get_object(view_id).kind {
        ObjectKind::TypedArray {
            buffer_id,
            byte_offset,
            ..
        } => (buffer_id, byte_offset),
        _ => unreachable!("create_typed_array_for_length always produces ObjectKind::TypedArray"),
    };
    for (i, value) in values.into_iter().enumerate() {
        #[allow(clippy::cast_possible_truncation)]
        let idx = i as u32;
        write_element_raw(&mut sub_ctx, buf_id, byte_offset, idx, ek, value)?;
    }
    drop(g);
    Ok(JsValue::Object(view_id))
}

/// Source-side drain for [`native_typed_array_from`].  Resolves
/// `source` to either an iterator (callable `@@iterator`) or an
/// array-like (length + integer-indexed read), then collects each
/// value into the returned `Vec`, applying `map_fn(value, index)`
/// with `this_arg` as the callback receiver if present.
///
/// `@@iterator` is fetched **once** (a single
/// `GetMethod`-equivalent) — a user-defined getter for the symbol
/// MUST NOT run twice between probe and call (spec §7.3.10
/// `GetMethod` evaluates the getter once).  `null` / `undefined`
/// `@@iterator` falls through to the array-like branch rather
/// than throwing.
fn collect_source_values(
    ctx: &mut NativeContext<'_>,
    source: JsValue,
    map_fn: Option<ObjectId>,
    this_arg: JsValue,
    ek: ElementKind,
) -> Result<Vec<JsValue>, VmError> {
    let iter_method = lookup_iterator_method(ctx, source)?;
    if !iter_method.is_nullish() {
        let JsValue::Object(iter_fn_id) = iter_method else {
            return Err(VmError::type_error(format!(
                "Failed to execute 'from' on '{}': @@iterator is not callable",
                ek.name()
            )));
        };
        if !ctx.get_object(iter_fn_id).kind.is_callable() {
            return Err(VmError::type_error(format!(
                "Failed to execute 'from' on '{}': @@iterator is not callable",
                ek.name()
            )));
        }
        let iter_val = ctx.call_function(iter_fn_id, source, &[])?;
        let JsValue::Object(_) = iter_val else {
            return Err(VmError::type_error(format!(
                "Failed to execute 'from' on '{}': @@iterator must return an object",
                ek.name()
            )));
        };
        return drain_iterator_with_map(ctx, iter_val, map_fn, this_arg);
    }

    // Array-like fallback (§23.2.2.1 step 8.b): read `source.length`
    // → `ToLength` → drain `source[0..length]` through the shared
    // property path.  `null` / `undefined` source surfaces a
    // TypeError via `coerce::to_object`.
    let source_obj = match source {
        JsValue::Object(id) => id,
        JsValue::Null | JsValue::Undefined => {
            return Err(VmError::type_error(format!(
                "Failed to execute 'from' on '{}': source is null or undefined",
                ek.name()
            )));
        }
        _ => super::super::coerce::to_object(ctx.vm, source)?,
    };
    let len_key = PropertyKey::String(ctx.vm.well_known.length);
    let len_val = ctx.get_property_value(source_obj, len_key)?;
    let len_f = ctx.to_number(len_val)?;
    let len_capped = if len_f.is_nan() || len_f <= 0.0 {
        0_u32
    } else {
        let truncated = len_f.trunc();
        if truncated > f64::from(u32::MAX) {
            return Err(VmError::range_error(format!(
                "Failed to execute 'from' on '{}': source length exceeds the supported maximum",
                ek.name()
            )));
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let l = truncated as u32;
        l
    };
    // Cap the eagerly-reserved capacity to avoid an OOM panic on
    // attacker-controlled `source.length` values up to `u32::MAX`.
    // The actual element count usually matches `len_capped` exactly
    // for genuine array-like sources; the `Vec` will grow on
    // `push` for the rare case where we under-reserve.  Mirrors
    // the bounded-capacity convention used elsewhere in the
    // engine's array-like collectors.
    const INITIAL_CAPACITY_CAP: usize = 1024;
    let initial_capacity = (len_capped as usize).min(INITIAL_CAPACITY_CAP);
    let mut out: Vec<JsValue> = Vec::with_capacity(initial_capacity);
    for i in 0..len_capped {
        #[allow(clippy::cast_precision_loss)]
        let idx = JsValue::Number(f64::from(i));
        let raw = ctx.vm.get_element(JsValue::Object(source_obj), idx)?;
        let value = if let Some(fn_id) = map_fn {
            ctx.call_function(fn_id, this_arg, &[raw, idx])?
        } else {
            raw
        };
        out.push(value);
    }
    Ok(out)
}

/// Resolve `source`'s `@@iterator` method to a `JsValue` —
/// `Undefined` if no entry, the resolved property value otherwise.
/// For `Object` sources, looks up `@@iterator` on the object
/// itself (with prototype chain).  For `String` primitives,
/// looks up on `String.prototype` (the spec-mandated location).
/// All other primitives have no iterator and return `Undefined`.
///
/// The single `get_property` + `resolve_property` pair is the
/// `GetMethod(source, @@iterator)` per spec §7.3.10 — a user
/// getter on `@@iterator` is invoked exactly once.
fn lookup_iterator_method(
    ctx: &mut NativeContext<'_>,
    source: JsValue,
) -> Result<JsValue, VmError> {
    let lookup_id = match source {
        JsValue::Object(id) => Some(id),
        JsValue::String(_) => ctx.vm.string_prototype,
        _ => None,
    };
    let Some(obj_id) = lookup_id else {
        return Ok(JsValue::Undefined);
    };
    let iter_key = PropertyKey::Symbol(ctx.vm.well_known_symbols.iterator);
    match super::super::coerce::get_property(ctx.vm, obj_id, iter_key) {
        Some(prop) => ctx.vm.resolve_property(prop, source),
        None => Ok(JsValue::Undefined),
    }
}

/// Drain `iter_val` into a `Vec<JsValue>`, applying `map_fn` per
/// element.  Mirrors
/// [`super::super::natives_array_hof`]'s drain helper but inlined
/// here to keep the cross-module surface small (the
/// `IteratorClose`-on-abrupt path is straightforward enough that
/// duplicating it is cleaner than exposing the array helper).
fn drain_iterator_with_map(
    ctx: &mut NativeContext<'_>,
    iter_val: JsValue,
    map_fn: Option<ObjectId>,
    this_arg: JsValue,
) -> Result<Vec<JsValue>, VmError> {
    let mut out = Vec::new();
    while let Some(value) = ctx.vm.iter_next(iter_val)? {
        let mapped = if let Some(fn_id) = map_fn {
            #[allow(clippy::cast_precision_loss)]
            let idx = JsValue::Number(out.len() as f64);
            match ctx.call_function(fn_id, this_arg, &[value, idx]) {
                Ok(v) => v,
                Err(e) => {
                    // §7.4.6 IteratorClose: a throw from `mapFn` is
                    // an abrupt completion of the for-of-like body;
                    // close the iterator before propagating.  A
                    // throw from `.return()` itself wins.
                    return Err(close_iterator_with_precedence(ctx, iter_val, e));
                }
            }
        } else {
            value
        };
        out.push(mapped);
    }
    Ok(out)
}

/// Close `iter_val` via `.return()` and surface the higher-
/// precedence error — a throw from `.return()` wins over the
/// triggering abrupt completion (§7.4.6 IteratorClose step 6-7).
fn close_iterator_with_precedence(
    ctx: &mut NativeContext<'_>,
    iter_val: JsValue,
    fallback: VmError,
) -> VmError {
    ctx.vm.iter_close(iter_val).err().unwrap_or(fallback)
}
