//! Native implementations of Symbol, Array iterator, and Object.prototype.toString.

use super::value::{
    ArrayIterState, JsValue, NativeContext, NativeFunction, Object, ObjectKind, Property,
    PropertyKey, VmError,
};

// -- Symbol constructor & methods -------------------------------------------

pub(super) fn native_symbol_constructor(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    // NOTE: `new Symbol()` should throw TypeError, but detecting constructor
    // calls requires knowing if invoked via the New opcode. Deferred.
    let desc = match args.first().copied() {
        Some(JsValue::Undefined) | None => None,
        Some(val) => Some(ctx.to_string_val(val)),
    };
    let sid = ctx.vm.alloc_symbol(desc);
    Ok(JsValue::Symbol(sid))
}

pub(super) fn native_symbol_for(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let key_id = ctx.to_string_val(args.first().copied().unwrap_or(JsValue::Undefined));
    if let Some(&sid) = ctx.vm.symbol_registry.get(&key_id) {
        return Ok(JsValue::Symbol(sid));
    }
    let sid = ctx.vm.alloc_symbol(Some(key_id));
    ctx.vm.symbol_registry.insert(key_id, sid);
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
    for (&key, &reg_sid) in &ctx.vm.symbol_registry {
        if reg_sid == sid {
            return Ok(JsValue::String(key));
        }
    }
    Ok(JsValue::Undefined)
}

pub(super) fn native_symbol_prototype_to_string(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let JsValue::Symbol(sid) = this else {
        return Err(VmError::type_error(
            "Symbol.prototype.toString requires a symbol value",
        ));
    };
    let desc = ctx.vm.symbols[sid.0 as usize]
        .description
        .map(|d| ctx.vm.strings.get_utf8(d));
    let result = match desc {
        Some(d) => format!("Symbol({d})"),
        None => "Symbol()".to_string(),
    };
    let id = ctx.intern(&result);
    Ok(JsValue::String(id))
}

// -- JSON stubs (M4-10) -----------------------------------------------------

// -- Array iterator (Symbol.iterator protocol) --------------------------------

/// `Array.prototype[Symbol.iterator]()` — creates an ArrayIterator.
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
    // Create a "next" native function object inline.
    let next_name = ctx.vm.well_known.next;
    let next_fn_id = ctx.alloc_object(Object {
        kind: ObjectKind::NativeFunction(NativeFunction {
            name: next_name,
            func: native_array_iterator_next,
        }),
        properties: Vec::new(),
        prototype: None,
    });
    let iter_obj = ctx.alloc_object(Object {
        kind: ObjectKind::ArrayIterator(ArrayIterState {
            array_id: arr_id,
            index: 0,
        }),
        properties: vec![(
            PropertyKey::String(next_name),
            Property::method(JsValue::Object(next_fn_id)),
        )],
        prototype: None,
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
    // Read state.
    let (array_id, idx) = {
        let iter_obj = ctx.get_object(iter_id);
        if let ObjectKind::ArrayIterator(state) = &iter_obj.kind {
            (state.array_id, state.index)
        } else {
            return create_iter_result(ctx, JsValue::Undefined, true);
        }
    };
    // Get value from array.
    let (value, done) = {
        let arr_obj = ctx.get_object(array_id);
        if let ObjectKind::Array { elements } = &arr_obj.kind {
            if idx < elements.len() {
                (elements[idx], false)
            } else {
                (JsValue::Undefined, true)
            }
        } else {
            (JsValue::Undefined, true)
        }
    };
    // Advance index.
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
        JsValue::Undefined => "Undefined",
        JsValue::Null => "Null",
        JsValue::Boolean(_) => "Boolean",
        JsValue::Number(_) => "Number",
        JsValue::String(_) => "String",
        JsValue::Symbol(_) => "Symbol",
        JsValue::Object(obj_id) => {
            // Check @@toStringTag
            let tag_key = PropertyKey::Symbol(ctx.vm.well_known_symbols.to_string_tag);
            if let Some(JsValue::String(tag_id)) =
                super::coerce::get_property(ctx.vm, obj_id, tag_key)
            {
                let tag_str = ctx.get_utf8(tag_id);
                let result = format!("[object {tag_str}]");
                let id = ctx.intern(&result);
                return Ok(JsValue::String(id));
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
                _ => "Object",
            }
        }
    };
    let result = format!("[object {tag}]");
    let id = ctx.intern(&result);
    Ok(JsValue::String(id))
}

/// Helper: create a `{ value, done }` iterator result object.
fn create_iter_result(
    ctx: &mut NativeContext<'_>,
    value: JsValue,
    done: bool,
) -> Result<JsValue, VmError> {
    let value_key = PropertyKey::String(ctx.vm.well_known.value);
    let done_key = PropertyKey::String(ctx.vm.well_known.done);
    let obj = ctx.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        properties: vec![
            (value_key, Property::data(value)),
            (done_key, Property::data(JsValue::Boolean(done))),
        ],
        prototype: None,
    });
    Ok(JsValue::Object(obj))
}
