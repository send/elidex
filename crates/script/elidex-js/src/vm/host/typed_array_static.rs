//! `%TypedArray%.of` / `%TypedArray%.from` static method bodies
//! (ECMA-262 §23.2.2).
//!
//! Of and from resolve a constructable receiver through the registered constructor chain, then
//! allocate a view with the resolved kind and prototype. Map, filter and the VM flatMap
//! extension reuse the species and allocation helpers.
//!
//! ## Scope (SP8a + SP8b)
//!
//! - `%TypedArray%.of(...items)` (§23.2.2.2) — SP8a
//! - `%TypedArray%.from(source, mapFn?, thisArg?)` (§23.2.2.1) — SP8a
//! - [`species_constructor_for_typed_array`] +
//!   [`create_typed_array_for_length`] — SP8b shared infrastructure
//!   re-exported `pub(super)` for [`super::typed_array_hof`]
//!   (`map` / `filter`).
//!
//! Species-sensitive instance methods + accumulator HOFs +
//! `flatMap` land in [`super::typed_array_hof`] (SP8b / SP8c-A /
//! SP8c-B); `toLocaleString` lives in
//! [`super::typed_array_methods`] alongside `join` (SP8c-B).

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
/// [`register_typed_array_prototype_global`](crate::vm::VmInner::register_typed_array_prototype_global)
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

/// A registry miss means this node has no registered element kind. Continue the prototype
/// walk. Report a family error only if the walk ends without a hit.
fn ctor_to_element_kind(vm: &VmInner, ctor_id: ObjectId) -> Option<ElementKind> {
    vm.subclass_array_ctors
        .iter()
        .position(|slot| *slot == Some(ctor_id))
        .and_then(ElementKind::from_index)
}

/// The runtime depth limit belongs to the VM admission predicate. Bound `[[Construct]]`
/// (§10.4.1.2 steps 5–6) substitutes the target when newTarget is the bound function,
/// then delegates construction to the target.
fn unwrap_bound_chain(vm: &VmInner, id: ObjectId) -> ObjectId {
    let mut current = id;
    for _ in 0..=crate::vm::MAX_BIND_CHAIN_DEPTH {
        match &vm.get_object(current).kind {
            ObjectKind::BoundFunction { target, .. } => current = *target,
            _ => return current,
        }
    }
    current
}

/// For an admitted constructor with a linked built-in chain, read its prototype through Get. A
/// manually linked Sub can yield (Sub.of(...)).constructor === Sub. The VM skips Construct and
/// its result validation. The abstract intrinsic passes the spec predicate and throws if
/// constructed. A family miss is a separate VM rejection. The missing constructor invocation
/// is tracked by `#11-vm-typed-array-create-construct`.
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
    if !super::super::construct_admission::is_constructor(ctx.vm, ctor_id) {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'TypedArray': this is not a TypedArray constructor"
        )));
    }
    // TypedArrayCreateFromConstructor (§23.2.4.1 step 1) calls Construct. Bound
    // `[[Construct]]` (§10.4.1.2) prepends bound arguments in step 4 and delegates
    // in step 6. The VM unwraps only for kind discovery and reads the original wrapper
    // prototype. Use Uint8Array.of.call(Bound, ...) for this VM.
    resolve_typed_array_constructor_in_chain(ctx, ctor_id)?.ok_or_else(|| {
        VmError::type_error(format!(
            "Failed to execute '{method}' on 'TypedArray': this is not a TypedArray constructor"
        ))
    })
}

/// A constructor explicitly linked to Uint8Array resolves to Uint8. For a bound receiver, use
/// Uint8Array.of.call(Bound, ...). Instance identity depends on the resolved prototype and its
/// constructor property. The prototype walk has no depth cap; bound-target admission does.
fn resolve_typed_array_constructor_in_chain(
    ctx: &mut NativeContext<'_>,
    ctor_id: ObjectId,
) -> Result<Option<(ElementKind, Option<ObjectId>)>, VmError> {
    let chain_start = unwrap_bound_chain(ctx.vm, ctor_id);
    let mut visited: Vec<ObjectId> = Vec::new();
    let mut current = Some(chain_start);
    while let Some(id) = current {
        if visited.contains(&id) {
            break;
        }
        visited.push(id);
        if let Some(ek) = ctor_to_element_kind(ctx.vm, id) {
            let proto_override = receiver_prototype(ctx, ctor_id)?;
            return Ok(Some((ek, proto_override)));
        }
        current = ctx.vm.get_object(id).prototype;
    }
    Ok(None)
}

/// SpeciesConstructor steps 2 and 5 select the default. The inherited species getter returns
/// this and takes the explicit constructor branch. For a built-in, Construct reaches
/// AllocateTypedArray and GetPrototypeFromConstructor. The VM approximates this using its
/// family walk and original-constructor prototype read. Linked subclass identity does not
/// establish execution of its constructor or species result validation.
/// (TypedArraySpeciesCreate §23.2.4.3.) The missing constructor invocation is tracked by
/// `#11-vm-typed-array-create-construct`.
pub(super) fn species_constructor_for_typed_array(
    ctx: &mut NativeContext<'_>,
    receiver: ObjectId,
    default_ek: ElementKind,
    method: &str,
) -> Result<(ElementKind, Option<ObjectId>), VmError> {
    let constructor_key = PropertyKey::String(ctx.vm.well_known.constructor);
    let ctor_val = ctx.get_property_value(receiver, constructor_key)?;
    let c_id = match ctor_val {
        JsValue::Undefined => return default_typed_array_pair(ctx, default_ek),
        JsValue::Object(id) => id,
        _ => {
            return Err(VmError::type_error(format!(
                "Failed to execute '{method}' on 'TypedArray': constructor must be an object"
            )));
        }
    };
    let species_key = PropertyKey::Symbol(ctx.vm.well_known_symbols.species);
    let s_val = ctx.get_property_value(c_id, species_key)?;
    let s_id = match s_val {
        JsValue::Undefined | JsValue::Null => return default_typed_array_pair(ctx, default_ek),
        JsValue::Object(id) => id,
        _ => {
            return Err(VmError::type_error(format!(
                "Failed to execute '{method}' on 'TypedArray': @@species is not a constructor"
            )));
        }
    };
    if !super::super::construct_admission::is_constructor(ctx.vm, s_id) {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'TypedArray': @@species is not a constructor"
        )));
    }
    resolve_typed_array_constructor_in_chain(ctx, s_id)?.ok_or_else(|| {
        VmError::type_error(format!(
            "Failed to execute '{method}' on 'TypedArray': @@species is not a TypedArray constructor"
        ))
    })
}

/// TypedArraySpeciesCreate step 3 invokes TypedArrayCreateFromConstructor. Its Construct step
/// enters the built-in constructor. AllocateTypedArray steps 1–2 call
/// GetPrototypeFromConstructor and TypedArrayCreate(proto). The former reads prototype at step
/// 2. The VM reads the registered constructor directly. (TypedArraySpeciesCreate §23.2.4.3.)
fn default_typed_array_pair(
    ctx: &mut NativeContext<'_>,
    default_ek: ElementKind,
) -> Result<(ElementKind, Option<ObjectId>), VmError> {
    let proto_override = match ctx.vm.subclass_array_ctors[default_ek.index()] {
        Some(default_ctor_id) => receiver_prototype(ctx, default_ctor_id)?,
        None => None,
    };
    Ok((default_ek, proto_override))
}

/// Read the selected constructor’s prototype as in GetPrototypeFromConstructor step 2. Return
/// an optional object to both resolution paths. create_typed_array_for_length consumes that
/// result. Linked test constructors can preserve Sub.prototype. (GetPrototypeFromConstructor
/// §10.1.14.)
fn receiver_prototype(
    ctx: &mut NativeContext<'_>,
    ctor_id: ObjectId,
) -> Result<Option<ObjectId>, VmError> {
    let proto_key = PropertyKey::String(ctx.vm.well_known.prototype);
    match ctx.get_property_value(ctor_id, proto_key)? {
        JsValue::Object(p) => Ok(Some(p)),
        _ => Ok(None),
    }
}

/// Approximate TypedArrayCreateFromConstructor by allocating a fresh zero-offset view. Use an
/// object obtained from Get on the admitted constructor, with the cached built-in fallback for
/// a missing or non-object prototype. A manually linked Sub with the matching
/// prototype.constructor preserves identity.
pub(super) fn create_typed_array_for_length(
    ctx: &mut NativeContext<'_>,
    ek: ElementKind,
    proto_override: Option<ObjectId>,
    len: u32,
) -> Result<ObjectId, VmError> {
    let bpe = u32::from(ek.bytes_per_element());
    let byte_len = len.checked_mul(bpe).ok_or_else(|| {
        // The VM directly approximates TypedArrayCreateFromConstructor. Multiplying length by
        // element width overflowed the VM byte-length slot. Use the constructor helper’s
        // RangeError convention.
        VmError::range_error(format!(
            "Failed to construct '{}': length too large",
            ek.name()
        ))
    })?;
    // Single stack scope rooting both `proto_override` (when an
    // accessor-resolved prototype that may be reachable only via
    // a Rust local) and the freshly allocated `buf_id` across the
    // view's `alloc_object`.  Two `push_temp_root` guards would
    // overlap the `&mut ctx.vm` borrow; the looser stack scope
    // accepts an arbitrary number of pushed values and restores
    // the stack on every exit (success / `?` propagation /
    // panic) via the guard's `Drop`.
    let mut frame = ctx.vm.push_stack_scope();
    if let Some(p) = proto_override {
        frame.stack.push(JsValue::Object(p));
    }
    let mut sub_ctx = NativeContext::new_call(&mut frame);
    let (buf_id, _, _) = allocate_fresh_buffer(&mut sub_ctx, byte_len)?;
    sub_ctx.vm.stack.push(JsValue::Object(buf_id));
    let prototype = proto_override.or_else(|| subclass_prototype_for(sub_ctx.vm, ek));
    let view_id = sub_ctx.vm.alloc_object(Object {
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
    drop(frame);
    Ok(view_id)
}

/// Of steps 3–4 check IsConstructor and call TypedArrayCreateFromConstructor. The VM accepts
/// constructable receivers whose linked chain reaches a registered constructor and allocates
/// directly. The abstract constructor throws on construction in the spec. (%TypedArray%.of
/// §23.2.2.2.)
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
    let mut sub_ctx = NativeContext::new_call(&mut g);
    let ObjectKind::TypedArray {
        buffer_id: buf_id,
        byte_offset,
        ..
    } = sub_ctx.vm.get_object(view_id).kind
    else {
        unreachable!("create_typed_array_for_length always produces ObjectKind::TypedArray")
    };
    for (i, value) in args.iter().enumerate() {
        #[allow(clippy::cast_possible_truncation)]
        let idx = i as u32;
        write_element_raw(&mut sub_ctx, buf_id, byte_offset, idx, ek, *value)?;
    }
    drop(g);
    Ok(JsValue::Object(view_id))
}

/// From step 2 checks IsConstructor; the VM also requires a registered family ancestor.
/// Undefined mapper disables mapping. For iterables the VM maps during collection, while step
/// 6 drains, constructs, then maps. For array-like sources it obtains length, allocates, then
/// reads/maps/writes. (%TypedArray%.from §23.2.2.1.)
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

    // Step 8 calls ToObject (§7.1.20), preserving an object source and boxing a non-nullish
    // primitive. Step 9 obtains length, step 10 creates the target and step 12 reads/maps/writes. This VM rejects nullish explicitly here; the spec fails
    // earlier at GetMethod for nullish source. (%TypedArray%.from §23.2.2.1.)
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
    allocate_and_write_view_from_array_like(
        ctx,
        ek,
        proto_override,
        source_obj,
        len_capped,
        map_fn,
        this_arg,
    )
}

/// Step 10 calls TypedArrayCreateFromConstructor; the VM allocates directly and can fail its
/// byte-length check before step 12 indexed reads or mapper calls. (%TypedArray%.from
/// §23.2.2.1.)
fn allocate_and_write_view_from_array_like(
    ctx: &mut NativeContext<'_>,
    ek: ElementKind,
    proto_override: Option<ObjectId>,
    source_obj: ObjectId,
    elems_len: u32,
    map_fn: Option<ObjectId>,
    this_arg: JsValue,
) -> Result<JsValue, VmError> {
    let view_id = create_typed_array_for_length(ctx, ek, proto_override, elems_len)?;
    let mut g = ctx.vm.push_temp_root(JsValue::Object(view_id));
    let mut sub_ctx = NativeContext::new_call(&mut g);
    let ObjectKind::TypedArray {
        buffer_id: buf_id,
        byte_offset,
        ..
    } = sub_ctx.vm.get_object(view_id).kind
    else {
        unreachable!("create_typed_array_for_length always produces ObjectKind::TypedArray")
    };
    for i in 0..elems_len {
        #[allow(clippy::cast_precision_loss)]
        let idx = JsValue::Number(f64::from(i));
        let raw = sub_ctx.vm.get_element(JsValue::Object(source_obj), idx)?;
        let value = if let Some(fn_id) = map_fn {
            sub_ctx.call_function(fn_id, this_arg, &[raw, idx])?
        } else {
            raw
        };
        write_element_raw(&mut sub_ctx, buf_id, byte_offset, i, ek, value)?;
    }
    drop(g);
    Ok(JsValue::Object(view_id))
}

/// Copy values collected by the VM drain to the allocated destination via write_element_raw.
/// From step 6.e uses TypedArraySetElement. This VM may already have mapped the values.
/// (%TypedArray%.from §23.2.2.1.)
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
    let mut sub_ctx = NativeContext::new_call(&mut g);
    let ObjectKind::TypedArray {
        buffer_id: buf_id,
        byte_offset,
        ..
    } = sub_ctx.vm.get_object(view_id).kind
    else {
        unreachable!("create_typed_array_for_length always produces ObjectKind::TypedArray")
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
/// The VM maps during drain and closes on mapper error. From step 6.a exhausts the iterator
/// before allocation or mapping, so mapper failure does not close it. (%TypedArray%.from
/// §23.2.2.1.)
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
    let mut sub_ctx = NativeContext::new_call(&mut frame);
    drain_iterator_loop(&mut sub_ctx, iter_val, map_fn, this_arg, elem_start)?;
    let elems_len = sub_ctx.vm.stack.len() - elem_start;
    let result = body(&mut sub_ctx, elem_start, elems_len);
    drop(frame);
    result
}

/// Inner loop of [`with_drained_iterator_on_stack`], split so the
/// outer stack scope can `truncate(saved_len)` on every exit
/// (success + `?` propagation + panic unwinding) via the guard's
/// `Drop`.
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
                    // VM-only early mapping closes the iterator here. From step 6.e maps after
                    // exhaustion. IteratorClose step 5 preserves an incoming throw if closing
                    // also throws. (%TypedArray%.from §23.2.2.1.)
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

/// From step 5 calls GetMethod(source, %Symbol.iterator%). GetV uses ToObject (§7.1.20)
/// for lookup: an object keeps its identity and a non-nullish primitive is boxed.
/// The original source is the receiver. This helper returns undefined for nullish sources and the caller
/// reports TypeError later. (GetMethod §7.3.10.)
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

/// This VM gives a close error precedence. IteratorClose step 5 retains the incoming throw.
/// From does its mapping after exhaustion. (IteratorClose §7.4.11.)
fn close_iterator_with_precedence(
    ctx: &mut NativeContext<'_>,
    iter_val: JsValue,
    fallback: VmError,
) -> VmError {
    ctx.vm.iter_close(iter_val).err().unwrap_or(fallback)
}
