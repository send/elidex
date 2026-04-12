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
fn function_display_name(ctx: &NativeContext<'_>, kind: &ObjectKind) -> String {
    match kind {
        ObjectKind::Function(fo) => fo.name.map_or_else(String::new, |n| ctx.get_utf8(n)),
        ObjectKind::NativeFunction(nf) => ctx.get_utf8(nf.name),
        _ => String::new(),
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

    // §19.2.3.2 steps 4-5: compute length and name for the bound function.
    let (target_length, target_name) = target_function_length_name(ctx, target_id);
    let bound_length = target_length.saturating_sub(bound_args.len());
    let name_str = format!("bound {target_name}");
    let name_id = ctx.intern(&name_str);

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
        #[allow(clippy::cast_precision_loss)]
        super::value::PropertyValue::Data(JsValue::Number(bound_length as f64)),
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

/// Extract the effective `length` (param count) and `name` from a function
/// target.  Iterative over BoundFunction chains to avoid stack overflow on
/// deep `.bind()` chains; capped at `MAX_BIND_CHAIN_DEPTH` to prevent O(N²)
/// name string growth from attacker-controlled input.
fn target_function_length_name(ctx: &NativeContext<'_>, target_id: ObjectId) -> (usize, String) {
    let mut current = target_id;
    let mut bound_arg_total = 0usize;
    let mut bound_depth = 0usize;
    loop {
        let obj = ctx.get_object(current);
        match &obj.kind {
            ObjectKind::Function(fo) => {
                let len = ctx.vm.get_compiled(fo.func_id).param_count as usize;
                let base_name = function_display_name(ctx, &obj.kind);
                let len = len.saturating_sub(bound_arg_total);
                let name = prepend_bound(bound_depth, &base_name);
                return (len, name);
            }
            ObjectKind::NativeFunction(nf) => {
                let base_name = ctx.get_utf8(nf.name);
                let name = prepend_bound(bound_depth, &base_name);
                return (0, name);
            }
            ObjectKind::BoundFunction {
                target, bound_args, ..
            } => {
                bound_arg_total = bound_arg_total.saturating_add(bound_args.len());
                bound_depth += 1;
                if bound_depth > crate::vm::MAX_BIND_CHAIN_DEPTH {
                    return (0, String::from("bound"));
                }
                current = *target;
            }
            _ => return (0, String::new()),
        }
    }
}

/// Prepend `"bound "` `n` times to a name (for BoundFunction.name derivation).
fn prepend_bound(n: usize, base: &str) -> String {
    if n == 0 {
        return base.to_string();
    }
    let mut s = String::with_capacity(n * 6 + base.len());
    for _ in 0..n {
        s.push_str("bound ");
    }
    s.push_str(base);
    s
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
    let mut current = obj_id;
    let mut bound_depth = 0u32;
    while let ObjectKind::BoundFunction { target, .. } = &ctx.get_object(current).kind {
        bound_depth += 1;
        current = *target;
    }
    let base_name = function_display_name(ctx, &ctx.get_object(current).kind);
    let name_str = if bound_depth == 0 {
        base_name
    } else {
        let prefix = "bound ".repeat(bound_depth as usize);
        format!("{prefix}{base_name}")
    };
    let result = format!("function {name_str}() {{ [native code] }}");
    let sid = ctx.intern(&result);
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
