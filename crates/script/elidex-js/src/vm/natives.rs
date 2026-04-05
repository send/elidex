//! Native (Rust-implemented) JS built-in functions.
//!
//! These are free functions with the `NativeFn` signature, referenced by name
//! in `globals.rs` when registering built-in objects.

use super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, Property, PropertyKey, VmError,
};
use super::VmInner;

// -- Global functions -------------------------------------------------------

pub(super) fn native_parse_int(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let s_id = ctx.to_string_val(val)?;
    let s = ctx.get_utf8(s_id).trim().to_string();

    // ES2020 §18.2.5: strip sign first, then detect 0x prefix.
    let mut negative = false;
    let mut rest = s.as_str();
    if let Some(r) = rest.strip_prefix('-') {
        negative = true;
        rest = r;
    } else if let Some(r) = rest.strip_prefix('+') {
        rest = r;
    }

    let radix_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let (radix, rest) = if matches!(radix_arg, JsValue::Undefined) {
        if let Some(r) = rest.strip_prefix("0x").or_else(|| rest.strip_prefix("0X")) {
            (16u32, r)
        } else {
            (10u32, rest)
        }
    } else {
        let r = ctx.to_number(radix_arg)?;
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let ri = r as i32;
        // ES2020 §18.2.5: radix 0 (or undefined) → default (10, with 0x prefix detection).
        if ri == 0 {
            if let Some(r2) = rest.strip_prefix("0x").or_else(|| rest.strip_prefix("0X")) {
                (16u32, r2)
            } else {
                (10u32, rest)
            }
        } else if r.is_nan() || !(2..=36).contains(&ri) {
            return Ok(JsValue::Number(f64::NAN));
        } else {
            let ru = ri.cast_unsigned();
            let rest = if ru == 16 {
                rest.strip_prefix("0x")
                    .or_else(|| rest.strip_prefix("0X"))
                    .unwrap_or(rest)
            } else {
                rest
            };
            (ru, rest)
        }
    };

    if !(2..=36).contains(&radix) {
        return Ok(JsValue::Number(f64::NAN));
    }

    // Parse as many valid digits as possible (prefix parsing).
    let mut result: f64 = 0.0;
    let mut found = false;
    let chars = rest.chars();

    for ch in chars {
        let Some(digit) = ch.to_digit(radix) else {
            break;
        };
        found = true;
        result = result * f64::from(radix) + f64::from(digit);
    }

    if !found {
        return Ok(JsValue::Number(f64::NAN));
    }
    if negative {
        result = -result;
    }
    Ok(JsValue::Number(result))
}

pub(super) fn native_parse_float(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let s_id = ctx.to_string_val(val)?;
    let trimmed = ctx.get_utf8(s_id).trim_start().to_string();
    let n = parse_float_prefix(&trimmed);
    Ok(JsValue::Number(n))
}

/// Parse the longest valid float prefix from a string (ES2020 `parseFloat` semantics).
///
/// Recognises `[+-]? digits [. digits] [eE [+-] digits]` and the literal
/// `Infinity` / `+Infinity` / `-Infinity`. Rejects Rust-specific literals
/// such as `inf`, `nan`, etc.
fn parse_float_prefix(s: &str) -> f64 {
    if s.is_empty() {
        return f64::NAN;
    }

    // Check for Infinity literals (the only non-numeric token parseFloat accepts).
    if let Some(rest) = s.strip_prefix("Infinity") {
        let _ = rest;
        return f64::INFINITY;
    }
    if let Some(rest) = s.strip_prefix("+Infinity") {
        let _ = rest;
        return f64::INFINITY;
    }
    if s.starts_with("-Infinity") {
        return f64::NEG_INFINITY;
    }

    // Scan the longest valid numeric prefix: [+-]? digits [. digits] [eE [+-] digits]
    let bytes = s.as_bytes();
    let mut i = 0;

    // Optional sign
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }

    // Integer digits
    let mut has_digit = false;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        has_digit = true;
        i += 1;
    }

    // Decimal point + fraction (`.5` is valid — digits may appear only after the dot)
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            has_digit = true;
            i += 1;
        }
    }

    // Must have consumed at least one digit (a bare "." or sign is invalid).
    if !has_digit {
        return f64::NAN;
    }

    // Exponent part
    if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        let save = i;
        i += 1;
        if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
            i += 1;
        }
        let exp_start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i == exp_start {
            // No digits after 'e', roll back to before the exponent.
            i = save;
        }
    }

    s[..i].parse::<f64>().unwrap_or(f64::NAN)
}

pub(super) fn native_is_nan(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = ctx.to_number(val)?;
    Ok(JsValue::Boolean(n.is_nan()))
}

pub(super) fn native_is_finite(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = ctx.to_number(val)?;
    Ok(JsValue::Boolean(n.is_finite()))
}

// -- Error constructors -----------------------------------------------------

fn error_ctor_impl(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
    error_name: &str,
) -> Result<JsValue, VmError> {
    if let JsValue::Object(id) = this {
        let name_key = PropertyKey::String(ctx.vm.well_known.name);
        let name_val = JsValue::String(ctx.intern(error_name));
        ctx.get_object_mut(id)
            .properties
            .push((name_key, Property::data(name_val)));
        let msg = args
            .first()
            .copied()
            .unwrap_or(JsValue::String(ctx.vm.well_known.empty));
        let msg_id = ctx.to_string_val(msg)?;
        let msg_key = PropertyKey::String(ctx.vm.well_known.message);
        ctx.get_object_mut(id)
            .properties
            .push((msg_key, Property::data(JsValue::String(msg_id))));
    }
    Ok(this)
}

pub(super) fn native_error_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    error_ctor_impl(ctx, this, args, "Error")
}

pub(super) fn native_type_error_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    error_ctor_impl(ctx, this, args, "TypeError")
}

pub(super) fn native_reference_error_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    error_ctor_impl(ctx, this, args, "ReferenceError")
}

pub(super) fn native_range_error_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    error_ctor_impl(ctx, this, args, "RangeError")
}

// -- Object static methods --------------------------------------------------

pub(super) fn native_object_keys(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let obj_val = args.first().copied().unwrap_or(JsValue::Undefined);
    let JsValue::Object(obj_id) = obj_val else {
        return Ok(JsValue::Object(ctx.alloc_object(Object {
            kind: ObjectKind::Array {
                elements: Vec::new(),
            },
            properties: Vec::new(),
            prototype: ctx.vm.array_prototype,
        })));
    };
    let keys: Vec<JsValue> = ctx
        .get_object(obj_id)
        .properties
        .iter()
        .filter(|(_, p)| p.enumerable)
        .filter_map(|(k, _)| {
            if let PropertyKey::String(sid) = k {
                Some(JsValue::String(*sid))
            } else {
                None
            }
        })
        .collect();
    Ok(JsValue::Object(ctx.alloc_object(Object {
        kind: ObjectKind::Array { elements: keys },
        properties: Vec::new(),
        prototype: ctx.vm.array_prototype,
    })))
}

pub(super) fn native_object_values(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let obj_val = args.first().copied().unwrap_or(JsValue::Undefined);
    let JsValue::Object(obj_id) = obj_val else {
        return Ok(JsValue::Object(ctx.alloc_object(Object {
            kind: ObjectKind::Array {
                elements: Vec::new(),
            },
            properties: Vec::new(),
            prototype: ctx.vm.array_prototype,
        })));
    };
    // §7.3.21 EnumerableOwnPropertyNames: snapshot keys, then Get per key.
    let keys: Vec<PropertyKey> = ctx
        .get_object(obj_id)
        .properties
        .iter()
        .filter(|(k, p)| p.enumerable && matches!(k, PropertyKey::String(_)))
        .map(|(k, _)| *k)
        .collect();
    let mut values = Vec::with_capacity(keys.len());
    for key in keys {
        values.push(ctx.get_property_value(obj_id, key)?);
    }
    Ok(JsValue::Object(ctx.alloc_object(Object {
        kind: ObjectKind::Array { elements: values },
        properties: Vec::new(),
        prototype: ctx.vm.array_prototype,
    })))
}

pub(super) fn native_object_assign(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let target = args.first().copied().unwrap_or(JsValue::Undefined);
    let JsValue::Object(target_id) = target else {
        return Ok(target);
    };
    let is_global = target_id == ctx.vm.global_object;
    for &source in args.iter().skip(1) {
        let JsValue::Object(src_id) = source else {
            continue;
        };
        // §19.1.2.1 step 5c: snapshot keys, then Get per key.
        let keys: Vec<PropertyKey> = ctx
            .get_object(src_id)
            .properties
            .iter()
            .filter(|(_, p)| p.enumerable)
            .map(|(k, _)| *k)
            .collect();
        let mut props = Vec::with_capacity(keys.len());
        for key in keys {
            props.push((key, ctx.get_property_value(src_id, key)?));
        }
        for (key, value) in &props {
            // Sync global object writes to the globals HashMap.
            if is_global {
                if let PropertyKey::String(sid) = key {
                    ctx.vm.globals.insert(*sid, *value);
                }
            }
            // Update existing or push new.
            let target_obj = ctx.get_object_mut(target_id);
            if let Some(prop) = target_obj.properties.iter_mut().find(|(k, _)| *k == *key) {
                prop.1.slot = super::value::PropertyValue::Data(*value);
            } else {
                target_obj.properties.push((*key, Property::data(*value)));
            }
        }
    }
    Ok(target)
}

pub(super) fn native_object_create(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let proto = args.first().copied().unwrap_or(JsValue::Null);
    let prototype = if let JsValue::Object(id) = proto {
        Some(id)
    } else {
        None
    };
    let obj_id = ctx.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        properties: Vec::new(),
        prototype,
    });
    Ok(JsValue::Object(obj_id))
}

#[allow(clippy::too_many_lines)]
pub(super) fn native_object_define_property(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let obj_val = args.first().copied().unwrap_or(JsValue::Undefined);
    let prop_val = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let desc_val = args.get(2).copied().unwrap_or(JsValue::Undefined);

    let JsValue::Object(obj_id) = obj_val else {
        return Err(VmError::type_error(
            "Object.defineProperty called on non-object",
        ));
    };
    let key = match prop_val {
        JsValue::Symbol(sid) => PropertyKey::Symbol(sid),
        other => PropertyKey::String(ctx.to_string_val(other)?),
    };

    // Extract descriptor fields.
    let new_prop = if let JsValue::Object(desc_id) = desc_val {
        let get_key = PropertyKey::String(ctx.intern("get"));
        let set_key = PropertyKey::String(ctx.intern("set"));
        let value_key = PropertyKey::String(ctx.intern("value"));
        let enumerable_key = PropertyKey::String(ctx.intern("enumerable"));
        let configurable_key = PropertyKey::String(ctx.intern("configurable"));
        let writable_key = PropertyKey::String(ctx.intern("writable"));

        // Snapshot which keys exist on the descriptor, then do a fresh Get
        // per key (§7.3.1).  A getter on one field may mutate another field,
        // so we must not cache values across calls.
        let desc_keys: Vec<PropertyKey> = ctx
            .get_object(desc_id)
            .properties
            .iter()
            .map(|(k, _)| *k)
            .collect();
        let get_field =
            |ctx: &mut NativeContext<'_>, key: PropertyKey| -> Result<Option<JsValue>, VmError> {
                if !desc_keys.contains(&key) {
                    return Ok(None);
                }
                Ok(Some(ctx.get_property_value(desc_id, key)?))
            };
        let has_get = get_field(ctx, get_key)?;
        let has_set = get_field(ctx, set_key)?;
        let has_value = get_field(ctx, value_key)?;
        let has_writable = get_field(ctx, writable_key)?;
        let has_enumerable = get_field(ctx, enumerable_key)?;
        let has_configurable = get_field(ctx, configurable_key)?;
        // ToBoolean coercion for boolean descriptor fields (§6.2.5.1).
        let enumerable = has_enumerable.map(|v| super::coerce::to_boolean(ctx.vm, v));
        let configurable = has_configurable.map(|v| super::coerce::to_boolean(ctx.vm, v));

        let has_accessor = has_get.is_some() || has_set.is_some();
        let has_data = has_value.is_some() || has_writable.is_some();

        // §9.1.6.3 step 2: mixing accessor and data fields is a TypeError.
        if has_accessor && has_data {
            return Err(VmError::type_error(
                "Invalid property descriptor. Cannot both specify accessors and a value or writable attribute",
            ));
        }

        // §9.1.6.3 step 7-8: get/set must be callable or undefined.
        let validate_accessor = |v: JsValue, role: &str| -> Result<Option<ObjectId>, VmError> {
            match v {
                JsValue::Undefined => Ok(None),
                JsValue::Object(id) => {
                    let is_callable = matches!(
                        &ctx.get_object(id).kind,
                        ObjectKind::Function(_)
                            | ObjectKind::NativeFunction(_)
                            | ObjectKind::BoundFunction { .. }
                    );
                    if is_callable {
                        Ok(Some(id))
                    } else {
                        Err(VmError::type_error(format!(
                            "Property descriptor {role} must be a function or undefined"
                        )))
                    }
                }
                _ => Err(VmError::type_error(format!(
                    "Property descriptor {role} must be a function or undefined"
                ))),
            }
        };

        // Look up the existing property to merge partial descriptors.
        let existing = ctx
            .get_object(obj_id)
            .properties
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, p)| *p);

        if has_accessor {
            let getter = match has_get {
                Some(v) => validate_accessor(v, "get")?,
                None => existing.and_then(|p| match p.slot {
                    super::value::PropertyValue::Accessor { getter, .. } => getter,
                    super::value::PropertyValue::Data(_) => None,
                }),
            };
            let setter = match has_set {
                Some(v) => validate_accessor(v, "set")?,
                None => existing.and_then(|p| match p.slot {
                    super::value::PropertyValue::Accessor { setter, .. } => setter,
                    super::value::PropertyValue::Data(_) => None,
                }),
            };
            Property {
                slot: super::value::PropertyValue::Accessor { getter, setter },
                writable: false,
                enumerable: enumerable.unwrap_or_else(|| existing.is_some_and(|p| p.enumerable)),
                configurable: configurable
                    .unwrap_or_else(|| existing.is_some_and(|p| p.configurable)),
            }
        } else {
            let value = has_value
                .unwrap_or_else(|| existing.map_or(JsValue::Undefined, |p| p.data_value()));
            let writable = has_writable.map_or_else(
                || existing.is_some_and(|p| p.writable),
                |v| super::coerce::to_boolean(ctx.vm, v),
            );
            Property {
                slot: super::value::PropertyValue::Data(value),
                writable,
                enumerable: enumerable.unwrap_or_else(|| existing.is_some_and(|p| p.enumerable)),
                configurable: configurable
                    .unwrap_or_else(|| existing.is_some_and(|p| p.configurable)),
            }
        }
    } else {
        return Err(VmError::type_error(
            "Property description must be an object",
        ));
    };

    // §9.1.6.3: Validate attribute changes against existing non-configurable property.
    if let Some(existing) = ctx
        .get_object(obj_id)
        .properties
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, p)| *p)
    {
        if !existing.configurable {
            // Cannot change configurable from false to true.
            if new_prop.configurable {
                return Err(VmError::type_error(
                    "Cannot redefine property: configurable is false",
                ));
            }
            // Cannot change enumerable on non-configurable property.
            if new_prop.enumerable != existing.enumerable {
                return Err(VmError::type_error(
                    "Cannot redefine property: cannot change enumerable",
                ));
            }
            // Cannot change from data to accessor or vice versa.
            let existing_is_accessor =
                matches!(existing.slot, super::value::PropertyValue::Accessor { .. });
            let new_is_accessor =
                matches!(new_prop.slot, super::value::PropertyValue::Accessor { .. });
            if existing_is_accessor != new_is_accessor {
                return Err(VmError::type_error(
                    "Cannot redefine property: cannot convert between data and accessor",
                ));
            }
            match (existing.slot, new_prop.slot) {
                // Non-configurable data: cannot change writable false→true
                // or value if non-writable.
                (
                    super::value::PropertyValue::Data(existing_val),
                    super::value::PropertyValue::Data(new_val),
                ) => {
                    if !existing.writable {
                        if new_prop.writable {
                            return Err(VmError::type_error(
                                "Cannot redefine property: cannot make non-writable property writable",
                            ));
                        }
                        if !super::value::same_value(existing_val, new_val) {
                            return Err(VmError::type_error(
                                "Cannot redefine property: cannot change value of non-writable, non-configurable property",
                            ));
                        }
                    }
                }
                // Non-configurable accessor: cannot change getter or setter
                // unless SameValue (§9.1.6.3 step 11).
                (
                    super::value::PropertyValue::Accessor {
                        getter: eg,
                        setter: es,
                    },
                    super::value::PropertyValue::Accessor {
                        getter: ng,
                        setter: ns,
                    },
                ) => {
                    let obj_or_undef =
                        |o: Option<ObjectId>| o.map_or(JsValue::Undefined, JsValue::Object);
                    if !super::value::same_value(obj_or_undef(eg), obj_or_undef(ng)) {
                        return Err(VmError::type_error(
                            "Cannot redefine property: cannot change getter of non-configurable accessor",
                        ));
                    }
                    if !super::value::same_value(obj_or_undef(es), obj_or_undef(ns)) {
                        return Err(VmError::type_error(
                            "Cannot redefine property: cannot change setter of non-configurable accessor",
                        ));
                    }
                }
                _ => {}
            }
        }
    }

    // Sync globals HashMap: data properties are cached for GetGlobal fast path;
    // accessor properties remove any stale data entry so GetGlobal falls back
    // to the global object.
    if obj_id == ctx.vm.global_object {
        if let PropertyKey::String(sid) = key {
            match new_prop.slot {
                super::value::PropertyValue::Data(v) => {
                    ctx.vm.globals.insert(sid, v);
                }
                super::value::PropertyValue::Accessor { .. } => {
                    ctx.vm.globals.remove(&sid);
                }
            }
        }
    }
    let obj = ctx.get_object_mut(obj_id);
    if let Some(prop) = obj.properties.iter_mut().find(|(k, _)| *k == key) {
        prop.1 = new_prop;
    } else {
        obj.properties.push((key, new_prop));
    }
    Ok(obj_val)
}

// -- Array static methods ---------------------------------------------------

pub(super) fn native_array_is_array(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let result = if let JsValue::Object(id) = val {
        matches!(ctx.get_object(id).kind, ObjectKind::Array { .. })
    } else {
        false
    };
    Ok(JsValue::Boolean(result))
}

// -- Math methods -----------------------------------------------------------

pub(super) fn native_math_abs(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let n = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined))?;
    Ok(JsValue::Number(n.abs()))
}

pub(super) fn native_math_ceil(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let n = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined))?;
    Ok(JsValue::Number(n.ceil()))
}

pub(super) fn native_math_floor(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let n = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined))?;
    Ok(JsValue::Number(n.floor()))
}

pub(super) fn native_math_round(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let n = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined))?;
    // ES2020 §20.2.2.28: if n is in [-0.5, 0), result is -0.
    let result = if (-0.5..0.0).contains(&n) {
        -0.0_f64
    } else {
        (n + 0.5).floor()
    };
    Ok(JsValue::Number(result))
}

pub(super) fn native_math_max(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if args.is_empty() {
        return Ok(JsValue::Number(f64::NEG_INFINITY));
    }
    let mut result = f64::NEG_INFINITY;
    for &arg in args {
        let n = ctx.to_number(arg)?;
        if n.is_nan() {
            return Ok(JsValue::Number(f64::NAN));
        }
        if n > result {
            result = n;
        }
    }
    Ok(JsValue::Number(result))
}

pub(super) fn native_math_min(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if args.is_empty() {
        return Ok(JsValue::Number(f64::INFINITY));
    }
    let mut result = f64::INFINITY;
    for &arg in args {
        let n = ctx.to_number(arg)?;
        if n.is_nan() {
            return Ok(JsValue::Number(f64::NAN));
        }
        if n < result {
            result = n;
        }
    }
    Ok(JsValue::Number(result))
}

pub(super) fn native_math_random(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // xorshift64 PRNG — not cryptographically secure but sufficient for
    // Math.random(). State is stored in VmInner so successive calls produce
    // distinct values.
    let mut s = ctx.vm.rng_state;
    s ^= s << 13;
    s ^= s >> 7;
    s ^= s << 17;
    ctx.vm.rng_state = s;
    // The shift produces a 53-bit value that fits in f64's mantissa exactly.
    #[allow(clippy::cast_precision_loss)]
    let n = (s >> 11) as f64 / (1u64 << 53) as f64;
    Ok(JsValue::Number(n))
}

pub(super) fn native_math_sqrt(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let n = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined))?;
    Ok(JsValue::Number(n.sqrt()))
}

pub(super) fn native_math_pow(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let base = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined))?;
    let exp = ctx.to_number(args.get(1).copied().unwrap_or(JsValue::Undefined))?;
    Ok(JsValue::Number(base.powf(exp)))
}

pub(super) fn native_math_log(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let n = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined))?;
    Ok(JsValue::Number(n.ln()))
}

// -- Object.getOwnPropertySymbols ---------------------------------------------

pub(super) fn native_object_get_own_property_symbols(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let obj_val = args.first().copied().unwrap_or(JsValue::Undefined);
    // §19.1.2.10.1: ToObject — throw TypeError for null/undefined
    if matches!(obj_val, JsValue::Null | JsValue::Undefined) {
        return Err(VmError::type_error(
            "Cannot convert undefined or null to object",
        ));
    }
    let JsValue::Object(obj_id) = obj_val else {
        // Primitives (number, string, boolean, symbol): ToObject would wrap,
        // but primitive wrappers have no own symbol properties.
        return Ok(JsValue::Object(ctx.alloc_object(Object {
            kind: ObjectKind::Array {
                elements: Vec::new(),
            },
            properties: Vec::new(),
            prototype: ctx.vm.array_prototype,
        })));
    };
    let syms: Vec<JsValue> = ctx
        .get_object(obj_id)
        .properties
        .iter()
        .filter_map(|(k, _)| {
            if let PropertyKey::Symbol(sid) = k {
                Some(JsValue::Symbol(*sid))
            } else {
                None
            }
        })
        .collect();
    Ok(JsValue::Object(ctx.alloc_object(Object {
        kind: ObjectKind::Array { elements: syms },
        properties: Vec::new(),
        prototype: ctx.vm.array_prototype,
    })))
}

// -- JSON stubs ---------------------------------------------------------------

pub(super) fn native_json_stringify_stub(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Undefined)
}

pub(super) fn native_json_parse_stub(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Undefined)
}

// -- Console ----------------------------------------------------------------

fn format_value_for_console(vm: &mut VmInner, val: JsValue) -> String {
    let id = super::coerce::to_display_string(vm, val);
    vm.strings.get_utf8(id)
}

fn console_output(
    ctx: &mut NativeContext<'_>,
    args: &[JsValue],
    prefix: &str,
) -> Result<JsValue, VmError> {
    let parts: Vec<String> = args
        .iter()
        .map(|v| format_value_for_console(ctx.vm, *v))
        .collect();
    eprintln!("{prefix}{}", parts.join(" "));
    Ok(JsValue::Undefined)
}

pub(super) fn native_console_log(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    console_output(ctx, args, "")
}

pub(super) fn native_console_error(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    console_output(ctx, args, "[error] ")
}

pub(super) fn native_console_warn(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    console_output(ctx, args, "[warn] ")
}

// Re-exports from split modules.
pub(super) use super::natives_string::{
    native_string_char_at, native_string_char_code_at, native_string_ends_with,
    native_string_includes, native_string_index_of, native_string_match, native_string_replace,
    native_string_search, native_string_slice, native_string_split, native_string_starts_with,
    native_string_substring, native_string_to_lower_case, native_string_to_upper_case,
    native_string_trim,
};
pub(super) use super::natives_symbol::{
    native_array_iterator_next, native_array_values, native_iterator_self,
    native_object_prototype_to_string, native_string_iterator, native_string_iterator_next,
    native_symbol_constructor, native_symbol_for, native_symbol_key_for,
    native_symbol_prototype_to_string,
};
