//! Function.prototype methods (ES2020 §19.2.3).
//!
//! - `call(thisArg, ...args)` — §19.2.3.3
//! - `apply(thisArg, argsArray)` — §19.2.3.1
//! - `bind(thisArg, ...args)` — §19.2.3.2
//! - `toString()` — §19.2.3.5

use super::shape::{self, PropertyAttrs};
use super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyStorage, VmError,
};

/// `{¬W, ¬E, C}` — used for `length` and `name` on bound functions (§19.2.3.2).
const NON_WRITABLE_CONFIGURABLE: PropertyAttrs = PropertyAttrs {
    writable: false,
    enumerable: false,
    configurable: true,
    is_accessor: false,
};

/// Validate that `this` is a callable object, returning its `ObjectId`.
fn require_callable_this(ctx: &NativeContext<'_>, this: JsValue) -> Result<ObjectId, VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error("not a function"));
    };
    if !ctx.get_object(id).kind.is_callable() {
        return Err(VmError::type_error("not a function"));
    }
    Ok(id)
}

/// Extract the display name from a function object kind.
/// Return the function's display name as WTF-16 code units, preserving lone
/// surrogates that may appear in user-defined function names.
fn function_display_name_u16(ctx: &NativeContext<'_>, kind: &ObjectKind) -> Vec<u16> {
    match kind {
        ObjectKind::Function(fo) => fo
            .name
            .map_or_else(Vec::new, |n| ctx.vm.strings.get(n).to_vec()),
        ObjectKind::NativeFunction(nf) => ctx.vm.strings.get(nf.name).to_vec(),
        _ => Vec::new(),
    }
}

/// `Function.prototype.call(thisArg, ...args)` — ES2020 §19.2.3.3
pub(super) fn native_function_call(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let callee_id = require_callable_this(ctx, this)?;
    let this_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let call_args = if args.len() > 1 { &args[1..] } else { &[] };
    ctx.call_function(callee_id, this_arg, call_args)
}

/// `Function.prototype.apply(thisArg, argsArray)` — ES2020 §19.2.3.1
pub(super) fn native_function_apply(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let callee_id = require_callable_this(ctx, this)?;
    let this_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let args_array = args.get(1).copied().unwrap_or(JsValue::Undefined);

    if args_array.is_nullish() {
        return ctx.call_function(callee_id, this_arg, &[]);
    }

    let JsValue::Object(arr_id) = args_array else {
        return Err(VmError::type_error(
            "CreateListFromArrayLike called on non-object",
        ));
    };
    let call_args = collect_array_like(ctx, arr_id)?;
    ctx.call_function(callee_id, this_arg, &call_args)
}

/// `Function.prototype.bind(thisArg, ...args)` — ES2020 §19.2.3.2
pub(super) fn native_function_bind(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let target_id = require_callable_this(ctx, this)?;
    let bound_this = args.first().copied().unwrap_or(JsValue::Undefined);
    let bound_args: Vec<JsValue> = if args.len() > 1 {
        args[1..].to_vec()
    } else {
        Vec::new()
    };

    // §19.2.3.2 steps 4-11: read target's `length` and `name` via property
    // Get so that `defineProperty(fn, 'name', {value: ...})` is honored.  A
    // BoundFunction's own `.name` is already "bound foo" from the prior bind,
    // so this naturally yields "bound bound foo" without walking the chain.
    // Abrupt completions from user-defined getters propagate (spec `?`).
    let (target_length, target_name): (f64, Vec<u16>) =
        target_function_length_name(ctx, target_id)?;
    // §19.2.3.2 step 8: L = max(0, targetLen - argCount).  Keep as f64
    // so that `+Infinity - argCount` stays `+Infinity` per spec.
    #[allow(clippy::cast_precision_loss)]
    let bound_length = (target_length - bound_args.len() as f64).max(0.0);
    // Prefix "bound " in WTF-16 so names with lone surrogates round-trip
    // losslessly.
    let mut name_units: Vec<u16> = "bound ".encode_utf16().collect();
    name_units.extend_from_slice(&target_name);
    let name_id = ctx.vm.strings.intern_utf16(&name_units);

    let func_proto = ctx.vm.function_prototype;
    let bound_id = ctx.alloc_object(Object {
        kind: ObjectKind::BoundFunction {
            target: target_id,
            bound_this,
            bound_args,
        },
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: func_proto,
        extensible: true,
    });

    // Set length (non-writable, non-enumerable, configurable).
    let length_key = super::value::PropertyKey::String(ctx.vm.well_known.length);
    ctx.vm.define_shaped_property(
        bound_id,
        length_key,
        super::value::PropertyValue::Data(JsValue::Number(bound_length)),
        NON_WRITABLE_CONFIGURABLE,
    );
    // Set name (non-writable, non-enumerable, configurable).
    let name_key = super::value::PropertyKey::String(ctx.vm.well_known.name);
    ctx.vm.define_shaped_property(
        bound_id,
        name_key,
        super::value::PropertyValue::Data(JsValue::String(name_id)),
        NON_WRITABLE_CONFIGURABLE,
    );

    Ok(JsValue::Object(bound_id))
}

/// Read the target's effective `length` and `name` per §19.2.3.2 steps 4-11.
/// Uses property `Get` so that `defineProperty(fn, 'length'|'name', ...)`
/// overrides are honored.  Falls back to the function's internal slots
/// (`FunctionObject.name`, `NativeFunction.name`, `CompiledFunction.param_count`)
/// when the property is absent or not of the expected type — these are
/// authoritative for user-defined functions whose `.name`/`.length` is not
/// yet exposed as a data property.  Abrupt completions (e.g. accessor
/// getter throws) are propagated to the caller.
fn target_function_length_name(
    ctx: &mut NativeContext<'_>,
    target_id: ObjectId,
) -> Result<(f64, Vec<u16>), VmError> {
    let length_key = super::value::PropertyKey::String(ctx.vm.well_known.length);
    let name_key = super::value::PropertyKey::String(ctx.vm.well_known.name);

    // §19.2.3.2 step 4-5: ToIntegerOrInfinity on the Number value.  NaN
    // → 0; ±Infinity propagate; other numbers truncate to integer.  Any
    // non-Number type → 0.  Absent property → internal param_count
    // (authoritative for user functions whose `.length` is not yet a
    // data property).
    #[allow(clippy::cast_precision_loss)]
    let length: f64 = match ctx.try_get_property_value(target_id, length_key)? {
        Some(JsValue::Number(n)) if n.is_nan() => 0.0,
        Some(JsValue::Number(n)) if n.is_infinite() => n,
        Some(JsValue::Number(n)) if n > 0.0 => n.trunc(),
        // Non-positive finite Numbers and non-Number values → 0.
        Some(_) => 0.0,
        None => internal_function_length(ctx, target_id) as f64,
    };
    // §19.2.3.2 step 11-13: `targetName` is `? Get(target, "name")`.
    // Non-String results get `ToString`-coerced (e.g. `{value: 42}` → "42");
    // Symbols fall back to empty string per §19.2.3.2 step 13 ("If Type is
    // not String, let targetName be the empty String").
    // Return WTF-16 units so lone surrogates in the name round-trip losslessly.
    let name: Vec<u16> = match ctx.try_get_property_value(target_id, name_key)? {
        Some(JsValue::String(sid)) => ctx.vm.strings.get(sid).to_vec(),
        Some(JsValue::Symbol(_)) => Vec::new(),
        Some(other) => {
            let sid = ctx.to_string_val(other)?;
            ctx.vm.strings.get(sid).to_vec()
        }
        None => internal_function_name_u16(ctx, target_id),
    };
    Ok((length, name))
}

fn internal_function_length(ctx: &NativeContext<'_>, target_id: ObjectId) -> usize {
    match &ctx.get_object(target_id).kind {
        ObjectKind::Function(fo) => ctx.vm.get_compiled(fo.func_id).param_count as usize,
        _ => 0,
    }
}

fn internal_function_name_u16(ctx: &NativeContext<'_>, target_id: ObjectId) -> Vec<u16> {
    match &ctx.get_object(target_id).kind {
        ObjectKind::Function(fo) => fo
            .name
            .map_or_else(Vec::new, |n| ctx.vm.strings.get(n).to_vec()),
        ObjectKind::NativeFunction(nf) => ctx.vm.strings.get(nf.name).to_vec(),
        _ => Vec::new(),
    }
}

/// `Function.prototype.toString()` — ES2020 §19.2.3.5
pub(super) fn native_function_to_string(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let JsValue::Object(obj_id) = this else {
        return Err(VmError::type_error(
            "Function.prototype.toString requires a function",
        ));
    };
    if !ctx.get_object(obj_id).kind.is_callable() {
        return Err(VmError::type_error(
            "Function.prototype.toString requires a function",
        ));
    }
    // Unwrap BoundFunction chain iteratively, counting "bound " prefixes.
    // Attacker-controlled bind depth would otherwise drive O(N²) name
    // string construction; cap via MAX_BIND_CHAIN_DEPTH (same policy as
    // call / construct paths).
    let mut current = obj_id;
    let mut bound_depth = 0usize;
    while let ObjectKind::BoundFunction { target, .. } = &ctx.get_object(current).kind {
        bound_depth += 1;
        if bound_depth > crate::vm::MAX_BIND_CHAIN_DEPTH {
            return Err(VmError::range_error("Maximum bind chain depth exceeded"));
        }
        current = *target;
    }
    let base_name = function_display_name_u16(ctx, &ctx.get_object(current).kind);
    // Assemble in WTF-16 so user-defined names with lone surrogates
    // round-trip losslessly.  Pre-compute "bound " once instead of
    // re-encoding per iteration.
    let bound_prefix: Vec<u16> = "bound ".encode_utf16().collect();
    let suffix: Vec<u16> = "() { [native code] }".encode_utf16().collect();
    let total = "function ".encode_utf16().count()
        + bound_prefix.len() * bound_depth
        + base_name.len()
        + suffix.len();
    let mut units: Vec<u16> = Vec::with_capacity(total);
    units.extend("function ".encode_utf16());
    for _ in 0..bound_depth {
        units.extend_from_slice(&bound_prefix);
    }
    units.extend_from_slice(&base_name);
    units.extend_from_slice(&suffix);
    let sid = ctx.vm.strings.intern_utf16(&units);
    Ok(JsValue::String(sid))
}

/// Collect elements from an array-like object into a Vec.
fn collect_array_like(
    ctx: &mut NativeContext<'_>,
    obj_id: ObjectId,
) -> Result<Vec<JsValue>, VmError> {
    let obj = ctx.get_object(obj_id);
    if let ObjectKind::Array { elements } = &obj.kind {
        return Ok(elements.iter().map(|v| v.or_undefined()).collect());
    }
    let length_key = super::value::PropertyKey::String(ctx.vm.well_known.length);
    let len_val = ctx.get_property_value(obj_id, length_key)?;
    let len_f = ctx.to_number(len_val)?.trunc();
    if len_f.is_nan() || len_f < 0.0 {
        return Ok(Vec::new());
    }
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    let len = (len_f as usize).min(super::ops::DENSE_ARRAY_LEN_LIMIT);
    let mut result = Vec::with_capacity(len.min(1024));
    for i in 0..len {
        let key_str = ctx.intern(&i.to_string());
        let key = super::value::PropertyKey::String(key_str);
        let val = ctx.get_property_value(obj_id, key)?;
        result.push(val);
    }
    Ok(result)
}
