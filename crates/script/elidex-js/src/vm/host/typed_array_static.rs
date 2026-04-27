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

use super::super::shape::{self, PropertyAttrs};
use super::super::value::{
    ElementKind, JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey,
    PropertyStorage, VmError,
};
use super::super::VmInner;
use super::typed_array::{allocate_fresh_buffer, write_element_raw};
use super::typed_array_methods::subclass_prototype_for;

/// Install `%TypedArray%.of` and `%TypedArray%.from` on the
/// abstract constructor.  Subclass ctors (`Uint8Array`,
/// `Float64Array`, …) inherit both via the constructor prototype
/// chain (`Object.getPrototypeOf(Uint8Array) === %TypedArray%`),
/// so they need no per-subclass install.  The natives dispatch on
/// `this` (the calling subclass ctor) via `subclass_array_ctors`
/// to pick the destination [`ElementKind`].
///
/// Called once from
/// [`super::typed_array::register_typed_array_prototype_global`]
/// after the abstract ctor + the 11 subclass ctors are wired up
/// (so the registry that this module's `ctor_to_element_kind`
/// scans is fully populated).
pub(super) fn install_typed_array_static_methods(vm: &mut VmInner, abstract_ctor: ObjectId) {
    let of_sid = vm.strings.intern("of");
    vm.install_native_method(
        abstract_ctor,
        of_sid,
        native_typed_array_of,
        PropertyAttrs::METHOD,
    );
    let from_sid = vm.strings.intern("from");
    vm.install_native_method(
        abstract_ctor,
        from_sid,
        native_typed_array_from,
        PropertyAttrs::METHOD,
    );
}

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
    ctx: &mut NativeContext<'_>,
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
                receiver_prototype(ctx, ctor_id, ek)?
            };
            return Ok((ek, proto_override));
        }
        current = ctx.vm.get_object(id).prototype;
    }
    Err(VmError::type_error(format!(
        "Failed to execute '{method}' on 'TypedArray': this is not a TypedArray constructor"
    )))
}

/// Read `ctor.prototype` via spec `Get(C, "prototype")`
/// semantics — invokes a user-defined accessor (getter), honours
/// inherited `.prototype` properties, and only falls back to the
/// built-in subclass prototype when the get **succeeds** but
/// yields a missing or non-Object value.  An exception thrown
/// from the accessor (or from any inherited proxy trap) is
/// propagated as an abrupt completion per spec, NOT swallowed —
/// silent fallback would mask user errors.
///
/// Used by the user-subclass branch of [`require_subclass_ctor`]
/// so `(class Sub extends Uint8Array {}).of(1, 2).constructor ===
/// Sub` holds even when `Sub.prototype` is a getter rather than a
/// plain data property.
///
/// `&mut NativeContext<'_>` because `get_property_value` may fire
/// the user getter (which can run arbitrary JS).
fn receiver_prototype(
    ctx: &mut NativeContext<'_>,
    ctor_id: ObjectId,
    ek: ElementKind,
) -> Result<Option<ObjectId>, VmError> {
    let proto_key = PropertyKey::String(ctx.vm.well_known.prototype);
    match ctx.get_property_value(ctor_id, proto_key)? {
        JsValue::Object(p) => Ok(Some(p)),
        _ => Ok(subclass_prototype_for(ctx.vm, ek)),
    }
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

    // Drain into vm.stack and run alloc + per-element write inside
    // the SAME stack scope so collected `JsValue`s remain GC-rooted
    // through the destination TypedArray's allocation.  Snapshotting
    // to a `Vec<JsValue>` between drain and alloc would leave the
    // values invisible to GC; `create_typed_array_for_length`'s
    // `alloc_object` is a potential GC point that could collect any
    // object values referenced only from the snapshot — leaving
    // stale `ObjectId`s and a use-after-free in the write loop.
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
        return with_drained_iterator_on_stack(
            ctx,
            iter_val,
            map_fn,
            this_arg,
            |sub_ctx, elem_start, elems_len| {
                allocate_and_write_view(sub_ctx, ek, proto_override, elem_start, elems_len)
            },
        );
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
    with_drained_array_like_on_stack(
        ctx,
        source_obj,
        len_capped,
        map_fn,
        this_arg,
        |sub_ctx, elem_start, elems_len| {
            allocate_and_write_view(sub_ctx, ek, proto_override, elem_start, elems_len)
        },
    )
}

/// Materialise a fresh `<ek>Array` of length `elems_len` and copy
/// each value through the destination's `[[Set]]` coercion.  Lives
/// inside the source-drain stack scope so the source values
/// (`ctx.vm.stack[elem_start..elem_start + elems_len]`) stay
/// rooted across the `alloc_object` GC point in
/// [`create_typed_array_for_length`] and the per-element
/// [`write_element_raw`] loop.  Reads from the rooted stack
/// range directly — no intermediate `Vec` clone (`JsValue` is
/// `Copy`, so the per-element `let value = ...` snapshot ends
/// before the next mutable borrow).
fn allocate_and_write_view(
    ctx: &mut NativeContext<'_>,
    ek: ElementKind,
    proto_override: Option<ObjectId>,
    elem_start: usize,
    elems_len: usize,
) -> Result<JsValue, VmError> {
    let len = u32::try_from(elems_len).map_err(|_| {
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
    for i in 0..elems_len {
        // `JsValue` is `Copy`; this snapshot ends before the
        // mutable `&mut sub_ctx` borrow on the next line.  The
        // stack entry remains rooted under the parent
        // `with_drained_*_on_stack` scope.
        let value = sub_ctx.vm.stack[elem_start + i];
        #[allow(clippy::cast_possible_truncation)]
        let idx = i as u32;
        write_element_raw(&mut sub_ctx, buf_id, byte_offset, idx, ek, value)?;
    }
    drop(g);
    Ok(JsValue::Object(view_id))
}

/// Drain `iter_val` onto `vm.stack`, then run `body` while every
/// drained `JsValue` (and the iterator object) remains rooted on
/// the stack.  `body` is the alloc + per-element-write phase;
/// running it inside the same stack scope as the drain ensures
/// `alloc_object` GC points cannot collect any object values
/// referenced from the stack range — snapshotting those values
/// into an unrooted `Vec<JsValue>` between drain and alloc would
/// leave them invisible to GC and produce stale `ObjectId`s
/// (Copilot R7 lesson).  Mirrors the `init_from_iterable_body`
/// pattern in `typed_array_ctor.rs`.
///
/// `body` receives `(ctx, elem_start, elems_len)` and reads
/// individual elements directly from `ctx.vm.stack[elem_start +
/// i]` — no intermediate `Vec` clone (Copilot R8 lesson).
/// `body`'s own `push_temp_root` allocations grow the stack
/// **above** `elem_start + elems_len` and shrink back on guard
/// drop, so the rooted element range is undisturbed.
///
/// IteratorClose (§7.4.6) runs on `map_fn` abrupt completion
/// before the stack scope drops, so the iterator's `.return()`
/// observes a still-rooted iter; `iter_next` throw is spec-exempt
/// and propagates without close.
fn with_drained_iterator_on_stack<R, F>(
    ctx: &mut NativeContext<'_>,
    iter_val: JsValue,
    map_fn: Option<ObjectId>,
    this_arg: JsValue,
    body: F,
) -> Result<R, VmError>
where
    F: FnOnce(&mut NativeContext<'_>, usize, usize) -> Result<R, VmError>,
{
    let mut frame = ctx.vm.push_stack_scope();
    let iter_slot = frame.saved_len();
    frame.stack.push(iter_val);
    let elem_start = iter_slot + 1;
    let mut sub_ctx = NativeContext { vm: &mut frame };
    drain_iterator_loop(&mut sub_ctx, iter_val, map_fn, this_arg, elem_start)?;
    let elems_len = sub_ctx.vm.stack.len() - elem_start;
    let result = body(&mut sub_ctx, elem_start, elems_len);
    drop(frame);
    result
}

/// Inner loop of [`drain_iterator_into_stack`], split so the outer
/// stack scope can `truncate(saved_len)` on every exit (success +
/// `?` propagation + panic unwinding) via the guard's `Drop`.
fn drain_iterator_loop(
    ctx: &mut NativeContext<'_>,
    iter_val: JsValue,
    map_fn: Option<ObjectId>,
    this_arg: JsValue,
    elem_start: usize,
) -> Result<(), VmError> {
    while let Some(value) = ctx.vm.iter_next(iter_val)? {
        let mapped = if let Some(fn_id) = map_fn {
            #[allow(clippy::cast_precision_loss)]
            let idx = JsValue::Number((ctx.vm.stack.len() - elem_start) as f64);
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
        ctx.vm.stack.push(mapped);
    }
    Ok(())
}

/// Drain the array-like `source_obj[0..len_capped]` onto
/// `vm.stack`, then run `body` while drained values remain rooted
/// — same GC-rooting + direct-slice-read invariants as
/// [`with_drained_iterator_on_stack`] (no iterator close needed
/// because the source isn't an iterator).
fn with_drained_array_like_on_stack<R, F>(
    ctx: &mut NativeContext<'_>,
    source_obj: ObjectId,
    len_capped: u32,
    map_fn: Option<ObjectId>,
    this_arg: JsValue,
    body: F,
) -> Result<R, VmError>
where
    F: FnOnce(&mut NativeContext<'_>, usize, usize) -> Result<R, VmError>,
{
    let mut frame = ctx.vm.push_stack_scope();
    let elem_start = frame.saved_len();
    let mut sub_ctx = NativeContext { vm: &mut frame };
    drain_array_like_loop(&mut sub_ctx, source_obj, len_capped, map_fn, this_arg)?;
    let elems_len = sub_ctx.vm.stack.len() - elem_start;
    let result = body(&mut sub_ctx, elem_start, elems_len);
    drop(frame);
    result
}

/// Inner loop of [`with_drained_array_like_on_stack`].  Pushed
/// elements are released by the parent's stack-scope `Drop` on
/// every exit.
fn drain_array_like_loop(
    ctx: &mut NativeContext<'_>,
    source_obj: ObjectId,
    len_capped: u32,
    map_fn: Option<ObjectId>,
    this_arg: JsValue,
) -> Result<(), VmError> {
    for i in 0..len_capped {
        #[allow(clippy::cast_precision_loss)]
        let idx = JsValue::Number(f64::from(i));
        let raw = ctx.vm.get_element(JsValue::Object(source_obj), idx)?;
        let value = if let Some(fn_id) = map_fn {
            ctx.call_function(fn_id, this_arg, &[raw, idx])?
        } else {
            raw
        };
        ctx.vm.stack.push(value);
    }
    Ok(())
}

/// Resolve `source`'s `@@iterator` method to a `JsValue` per spec
/// `GetMethod(ToObject(source), @@iterator)` (§7.3.10): `null` /
/// `undefined` source returns `Undefined` immediately (no
/// iterator); every other value (`Object`, `String`, `Number`,
/// `Boolean`, `BigInt`, `Symbol`) is boxed via `ToObject` so
/// prototype-installed iterators on **any** primitive's wrapper
/// prototype are honoured (e.g. user-defined
/// `Number.prototype[Symbol.iterator]`).
///
/// `resolve_property(prop, source)` keeps the **original**
/// `source` as the receiver passed to a `@@iterator` accessor,
/// matching `GetV` semantics — the wrapper is a transient lookup
/// vehicle, not the `this` binding of any user getter.
///
/// The single `get_property` + `resolve_property` pair preserves
/// the spec's "read once" behaviour — a user getter on
/// `@@iterator` is invoked exactly once between probe and call.
fn lookup_iterator_method(
    ctx: &mut NativeContext<'_>,
    source: JsValue,
) -> Result<JsValue, VmError> {
    let obj_id = match source {
        JsValue::Undefined | JsValue::Null => return Ok(JsValue::Undefined),
        _ => super::super::coerce::to_object(ctx.vm, source)?,
    };
    let iter_key = PropertyKey::Symbol(ctx.vm.well_known_symbols.iterator);
    match super::super::coerce::get_property(ctx.vm, obj_id, iter_key) {
        Some(prop) => ctx.vm.resolve_property(prop, source),
        None => Ok(JsValue::Undefined),
    }
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
