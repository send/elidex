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

use super::super::value::{ElementKind, JsValue, NativeContext, ObjectId, PropertyKey, VmError};
use super::super::VmInner;
use super::typed_array::{allocate_fresh_buffer, write_element_raw};
use super::typed_array_methods::alloc_typed_array_view;

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

/// Resolve `this` (the caller of `%TypedArray%.of` / `.from`) to
/// the destination [`ElementKind`].  Mirrors the spec's
/// `IsConstructor` step at the level of "is this a built-in
/// TypedArray subclass ctor we know about?": user-defined
/// subclasses + the abstract `%TypedArray%` ctor itself both land
/// in the `None` branch and surface a TypeError per
/// §23.2.2.{1,2} step "If IsConstructor(C) is false, throw".
fn require_subclass_ctor(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<ElementKind, VmError> {
    let JsValue::Object(ctor_id) = this else {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on '%TypedArray%': this is not a TypedArray constructor"
        )));
    };
    ctor_to_element_kind(ctx.vm, ctor_id).ok_or_else(|| {
        VmError::type_error(format!(
            "Failed to execute '{method}' on '%TypedArray%': this is not a built-in TypedArray subclass constructor"
        ))
    })
}

/// Allocate a fresh `<ek>Array(len)`-shaped TypedArray instance:
/// new buffer of `len * bpe` bytes, fresh view at `byte_offset = 0`
/// covering the whole buffer.  Mirrors the spec's
/// `TypedArrayCreate(constructor, ⟨len⟩)` (§22.2.4.2.1) for the
/// built-in subclass case — `constructor` is implied by the
/// already-resolved `ek`.
fn create_typed_array_for_length(
    ctx: &mut NativeContext<'_>,
    ek: ElementKind,
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
    // alloc — `alloc_typed_array_view` allocates an `Object` and
    // (under the future GC-enabled path) could otherwise reclaim
    // the buffer's slot before the view links it.  Same RAII
    // rooting pattern as `init_from_typed_array` /
    // `init_from_iterable_body`.
    let mut g = ctx.vm.push_temp_root(JsValue::Object(buf_id));
    let mut sub_ctx = NativeContext { vm: &mut g };
    let view_id = alloc_typed_array_view(&mut sub_ctx, ek, buf_id, 0, byte_len);
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
    let ek = require_subclass_ctor(ctx, this, "of")?;
    let len = u32::try_from(args.len()).map_err(|_| {
        VmError::range_error(format!(
            "Failed to execute 'of' on '{}': too many items",
            ek.name()
        ))
    })?;
    let view_id = create_typed_array_for_length(ctx, ek, len)?;
    // Root the view across element writes (each `write_element_raw`
    // may run user-level `valueOf` / `Symbol.toPrimitive` and, on
    // BigInt subclasses, allocate fresh `BigIntId`s — both can
    // currently never trigger GC inside natives, but the rooting
    // matches the wider invariant).
    let mut g = ctx.vm.push_temp_root(JsValue::Object(view_id));
    let mut sub_ctx = NativeContext { vm: &mut g };
    let (buf_id, byte_offset) = match sub_ctx.vm.get_object(view_id).kind {
        super::super::value::ObjectKind::TypedArray {
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
    let ek = require_subclass_ctor(ctx, this, "from")?;
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

    let view_id = create_typed_array_for_length(ctx, ek, len)?;
    let mut g = ctx.vm.push_temp_root(JsValue::Object(view_id));
    let mut sub_ctx = NativeContext { vm: &mut g };
    let (buf_id, byte_offset) = match sub_ctx.vm.get_object(view_id).kind {
        super::super::value::ObjectKind::TypedArray {
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
/// with `this_arg` as the callback receiver if present.  Mirrors
/// `Array.from`'s [`super::super::natives_array_hof::native_array_from`]
/// dispatch — `null` / `undefined` `@@iterator` falls through to
/// the array-like branch rather than throwing.
fn collect_source_values(
    ctx: &mut NativeContext<'_>,
    source: JsValue,
    map_fn: Option<ObjectId>,
    this_arg: JsValue,
    ek: ElementKind,
) -> Result<Vec<JsValue>, VmError> {
    // `@@iterator` resolves to a callable for objects, strings,
    // arrays, sets, maps, generators, etc.  Primitives without
    // wrappers (numbers, booleans, etc.) fall through to the
    // array-like branch, which then `ToObject`-wraps via
    // `get_property_value` semantics.
    let has_iterator = match source {
        JsValue::Object(obj_id) => {
            let iter_key = PropertyKey::Symbol(ctx.vm.well_known_symbols.iterator);
            match super::super::coerce::get_property(ctx.vm, obj_id, iter_key) {
                Some(prop) => !ctx.vm.resolve_property(prop, source)?.is_nullish(),
                None => false,
            }
        }
        JsValue::String(_) => true,
        _ => false,
    };

    if has_iterator {
        let Some(iter_val) = ctx.vm.resolve_iterator(source)? else {
            return Ok(Vec::new());
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
    let mut out: Vec<JsValue> = Vec::with_capacity(len_capped as usize);
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
