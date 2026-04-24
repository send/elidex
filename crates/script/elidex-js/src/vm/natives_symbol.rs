//! Native implementations of Symbol, Array iterator, and Object.prototype.toString.

use super::value::{
    ArrayIterState, JsValue, NativeContext, Object, ObjectKind, PropertyKey, PropertyStorage,
    StringIterState, VmError,
};

// -- Symbol constructor & methods -------------------------------------------

pub(super) fn native_symbol_constructor(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    // `new Symbol()` is rejected by `do_new` via `constructable: false`.
    // This function only runs for direct calls: `Symbol('desc')`.
    let desc = match args.first().copied() {
        Some(JsValue::Undefined) | None => None,
        Some(val) => Some(ctx.to_string_val(val)?),
    };
    let sid = ctx.vm.alloc_symbol(desc);
    Ok(JsValue::Symbol(sid))
}

pub(super) fn native_symbol_for(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let key_id = ctx.to_string_val(args.first().copied().unwrap_or(JsValue::Undefined))?;
    if let Some(&sid) = ctx.vm.symbol_registry.get(&key_id) {
        return Ok(JsValue::Symbol(sid));
    }
    let sid = ctx.vm.alloc_symbol(Some(key_id));
    ctx.vm.symbol_registry.insert(key_id, sid);
    ctx.vm.symbol_reverse_registry.insert(sid, key_id);
    Ok(JsValue::Symbol(sid))
}

pub(super) fn native_symbol_key_for(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let JsValue::Symbol(sid) = val else {
        return Err(VmError::type_error("Symbol.keyFor requires a symbol"));
    };
    if let Some(&key) = ctx.vm.symbol_reverse_registry.get(&sid) {
        return Ok(JsValue::String(key));
    }
    Ok(JsValue::Undefined)
}

pub(super) fn native_symbol_prototype_to_string(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // §19.4.3.3 thisSymbolValue: accept both Symbol primitive and a
    // Symbol wrapper object (unwrap `[[SymbolData]]`).
    let sid = match this {
        JsValue::Symbol(sid) => sid,
        JsValue::Object(obj_id) => {
            if let ObjectKind::SymbolWrapper(sid) = ctx.get_object(obj_id).kind {
                sid
            } else {
                return Err(VmError::type_error(
                    "Symbol.prototype.toString requires a symbol value",
                ));
            }
        }
        _ => {
            return Err(VmError::type_error(
                "Symbol.prototype.toString requires a symbol value",
            ));
        }
    };
    // Build the output in WTF-16 so descriptions with unpaired surrogates
    // are preserved losslessly (UTF-8 round-trip via get_utf8 would
    // replace them with U+FFFD).
    let mut units: Vec<u16> = "Symbol(".encode_utf16().collect();
    if let Some(desc) = ctx.vm.symbols[sid.0 as usize].description {
        units.extend_from_slice(ctx.vm.strings.get(desc));
    }
    units.push(u16::from(b')'));
    let id = ctx.vm.strings.intern_utf16(&units);
    Ok(JsValue::String(id))
}

// -- JSON stubs (M4-10) -----------------------------------------------------

// -- Iterator @@iterator (returns `this`) ------------------------------------

/// Native function that returns `this`, used as `[Symbol.iterator]()` on
/// iterator objects so that iterators are themselves iterable.
pub(super) fn native_iterator_self(
    _ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(this)
}

// -- Array iterator (Symbol.iterator protocol) --------------------------------

/// `Array.prototype[Symbol.iterator]()` — creates an ArrayIterator.
///
/// Methods (`next`, `@@iterator`) live on the shared array iterator
/// prototype registered during VM initialisation, so individual
/// iterator objects carry no per-instance function allocations.
pub(super) fn native_array_values(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let JsValue::Object(arr_id) = this else {
        return Err(VmError::type_error(
            "Array.prototype[Symbol.iterator] called on non-object",
        ));
    };
    if !matches!(ctx.get_object(arr_id).kind, ObjectKind::Array { .. }) {
        return Err(VmError::type_error(
            "Array.prototype[Symbol.iterator] called on non-array",
        ));
    }
    let iter_obj = ctx.alloc_object(Object {
        kind: ObjectKind::ArrayIterator(ArrayIterState {
            array_id: arr_id,
            index: 0,
            kind: 0, // Values
        }),
        storage: PropertyStorage::shaped(super::shape::ROOT_SHAPE),
        prototype: ctx.vm.array_iterator_prototype,
        extensible: true,
    });
    Ok(JsValue::Object(iter_obj))
}

/// `ArrayIterator.prototype.next()` — returns `{ value, done }`.
pub(super) fn native_array_iterator_next(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let JsValue::Object(iter_id) = this else {
        return create_iter_result(ctx, JsValue::Undefined, true);
    };
    let (array_id, idx, kind) = {
        let iter_obj = ctx.get_object(iter_id);
        if let ObjectKind::ArrayIterator(state) = &iter_obj.kind {
            (state.array_id, state.index, state.kind)
        } else {
            return create_iter_result(ctx, JsValue::Undefined, true);
        }
    };
    let len = match &ctx.get_object(array_id).kind {
        ObjectKind::Array { elements } => Some(elements.len()),
        #[cfg(feature = "engine")]
        &ObjectKind::TypedArray {
            byte_length,
            element_kind,
            ..
        } => {
            // TypedArray iteration reuses `ArrayIterState` with
            // `array_id` pointing at the TypedArray instance.  The
            // element count is byte_length / bpe — computed here
            // so the value-fetch path below (`get_element`) sees
            // the correct length and defers to the TypedArray
            // integer-indexed get (installed in C3).
            let bpe = u32::from(element_kind.bytes_per_element());
            Some((byte_length / bpe) as usize)
        }
        _ => None,
    };
    let (value, done) = if let Some(len) = len {
        if idx < len {
            let idx_val = super::natives_array::index_to_number(idx);
            let val = match kind {
                1 => idx_val, // Keys
                2 => {
                    // Entries: [index, value]
                    let elem = ctx.vm.get_element(JsValue::Object(array_id), idx_val)?;
                    let pair = vec![idx_val, elem];
                    super::natives_array::create_array(ctx, pair)
                }
                _ => ctx.vm.get_element(JsValue::Object(array_id), idx_val)?, // Values
            };
            (val, false)
        } else {
            (JsValue::Undefined, true)
        }
    } else {
        (JsValue::Undefined, true)
    };
    if !done {
        let iter_obj = ctx.get_object_mut(iter_id);
        if let ObjectKind::ArrayIterator(state) = &mut iter_obj.kind {
            state.index += 1;
        }
    }
    create_iter_result(ctx, value, done)
}

// -- Object.prototype.toString (ES2020 §19.1.3.6) -------------------------

pub(super) fn native_object_prototype_to_string(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let tag = match this {
        JsValue::Empty | JsValue::Undefined => "Undefined",
        JsValue::Null => "Null",
        JsValue::Boolean(_) => "Boolean",
        JsValue::Number(_) => "Number",
        JsValue::String(_) => "String",
        JsValue::Symbol(_) => "Symbol",
        JsValue::BigInt(_) => "BigInt",
        JsValue::Object(obj_id) => {
            // Check @@toStringTag (invoke getter if accessor).
            let tag_key = PropertyKey::Symbol(ctx.vm.well_known_symbols.to_string_tag);
            if let Some(result) = super::coerce::get_property(ctx.vm, obj_id, tag_key) {
                let tag_val = match result {
                    super::coerce::PropertyResult::Data(v) => v,
                    super::coerce::PropertyResult::Getter(g) => ctx.call_function(g, this, &[])?,
                };
                if let JsValue::String(tag_id) = tag_val {
                    // WTF-16 concat preserves lone surrogates in the tag.
                    let mut units: Vec<u16> = "[object ".encode_utf16().collect();
                    units.extend_from_slice(ctx.vm.strings.get(tag_id));
                    units.push(u16::from(b']'));
                    let id = ctx.vm.strings.intern_utf16(&units);
                    return Ok(JsValue::String(id));
                }
            }
            // Default tags based on object kind
            let obj = ctx.get_object(obj_id);
            match &obj.kind {
                ObjectKind::Array { .. } => "Array",
                ObjectKind::Function(_)
                | ObjectKind::NativeFunction(_)
                | ObjectKind::BoundFunction { .. } => "Function",
                ObjectKind::Error { .. } => "Error",
                ObjectKind::RegExp { .. } => "RegExp",
                ObjectKind::Promise(_) => "Promise",
                _ => "Object",
            }
        }
    };
    let result = format!("[object {tag}]");
    let id = ctx.intern(&result);
    Ok(JsValue::String(id))
}

// -- Object.prototype.toLocaleString (ES2020 §19.1.3.5) -------------------
//
// Spec: `Return ? Invoke(O, "toString")`.  Previously wired as an alias of
// Object.prototype.toString, which bypassed per-type overrides and made
// `(5).toLocaleString()` yield "[object Number]" instead of "5".
pub(super) fn native_object_prototype_to_locale_string(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // Invoke(V, "toString") = ? Call(? GetV(V, "toString"), V).  GetV boxes
    // primitives via ToObject for property lookup but passes the original V
    // as the receiver, so Number.prototype.toString etc. apply correctly.
    let (obj_id, receiver) = match this {
        JsValue::Object(obj_id) => (obj_id, this),
        JsValue::Null | JsValue::Undefined | JsValue::Empty => {
            return Err(VmError::type_error(
                "Object.prototype.toLocaleString called on null or undefined",
            ));
        }
        primitive => (super::coerce::to_object(ctx.vm, primitive)?, primitive),
    };
    let to_string_key = PropertyKey::String(ctx.intern("toString"));
    let method = ctx
        .try_get_property_value(obj_id, to_string_key)?
        .unwrap_or(JsValue::Undefined);
    let JsValue::Object(fn_id) = method else {
        return Err(VmError::type_error("toString is not callable"));
    };
    if !ctx.get_object(fn_id).kind.is_callable() {
        return Err(VmError::type_error("toString is not callable"));
    }
    ctx.call_function(fn_id, receiver, &[])
}

// -- String iterator (Symbol.iterator protocol) ------------------------------

/// `String.prototype[Symbol.iterator]()` — creates a StringIterator.
///
/// Methods (`next`, `@@iterator`) live on the shared string iterator
/// prototype registered during VM initialisation.
pub(super) fn native_string_iterator(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let JsValue::String(sid) = this else {
        return Err(VmError::type_error(
            "String.prototype[Symbol.iterator] called on non-string",
        ));
    };
    let iter_obj = ctx.alloc_object(Object {
        kind: ObjectKind::StringIterator(StringIterState {
            string_id: sid,
            index: 0,
        }),
        storage: PropertyStorage::shaped(super::shape::ROOT_SHAPE),
        prototype: ctx.vm.string_iterator_prototype,
        extensible: true,
    });
    Ok(JsValue::Object(iter_obj))
}

/// `StringIterator.prototype.next()` — returns `{ value, done }`.
///
/// Yields individual code points (combining surrogate pairs for supplementary
/// characters per ES2020 §21.1.5.2.1).
pub(super) fn native_string_iterator_next(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let JsValue::Object(iter_id) = this else {
        return create_iter_result(ctx, JsValue::Undefined, true);
    };
    // Step 1: read only the needed code units under an immutable borrow (O(1)).
    let (first, second) = {
        let iter_obj = ctx.get_object(iter_id);
        if let ObjectKind::StringIterator(state) = &iter_obj.kind {
            let units = ctx.vm.strings.get(state.string_id);
            if state.index >= units.len() {
                return create_iter_result(ctx, JsValue::Undefined, true);
            }
            (units[state.index], units.get(state.index + 1).copied())
        } else {
            return create_iter_result(ctx, JsValue::Undefined, true);
        }
    };
    // Step 2: compute character and advance amount (no borrow held).
    // Use a stack buffer to avoid heap allocation.
    let is_surrogate_pair = (0xD800..=0xDBFF).contains(&first)
        && second.is_some_and(|low| (0xDC00..=0xDFFF).contains(&low));
    let (buf, len, advance) = if is_surrogate_pair {
        ([first, second.unwrap()], 2, 2)
    } else {
        ([first, 0], 1, 1)
    };
    // Step 3: advance index (mutable borrow).
    {
        let iter_obj = ctx.get_object_mut(iter_id);
        if let ObjectKind::StringIterator(state) = &mut iter_obj.kind {
            state.index += advance;
        }
    }
    // Step 4: create result.
    let str_id = ctx.intern_utf16(&buf[..len]);
    create_iter_result(ctx, JsValue::String(str_id), false)
}

/// Local wrapper around [`VmInner::create_iter_result`] that fits the
/// `NativeContext` call shape.  The shared helper on `VmInner` anchors
/// the prototype at `%Object.prototype%` (§7.4.8) and is used by every
/// IteratorResult-shaped allocation in the VM (generator, array, string
/// iterators).
fn create_iter_result(
    ctx: &mut NativeContext<'_>,
    value: JsValue,
    done: bool,
) -> Result<JsValue, VmError> {
    Ok(JsValue::Object(ctx.vm.create_iter_result(value, done)))
}
