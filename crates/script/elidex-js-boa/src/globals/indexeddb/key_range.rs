//! `IDBKeyRange` global object with static factory methods.

use std::cmp::Ordering;

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsNativeError, JsResult, JsValue, NativeFunction};

use super::factory::js_value_to_idb_key;

/// Register `IDBKeyRange` as a global constructor with static methods.
#[allow(clippy::redundant_closure)] // closures needed to discard `this` parameter
pub fn register_idb_key_range(ctx: &mut Context) {
    let only_fn = NativeFunction::from_copy_closure(|_, args, ctx| kr_only(args, ctx));
    let lower_fn = NativeFunction::from_copy_closure(|_, args, ctx| kr_lower_bound(args, ctx));
    let upper_fn = NativeFunction::from_copy_closure(|_, args, ctx| kr_upper_bound(args, ctx));
    let bound_fn = NativeFunction::from_copy_closure(|_, args, ctx| kr_bound(args, ctx));

    let kr = ObjectInitializer::new(ctx)
        .function(only_fn, js_string!("only"), 1)
        .function(lower_fn, js_string!("lowerBound"), 2)
        .function(upper_fn, js_string!("upperBound"), 2)
        .function(bound_fn, js_string!("bound"), 4)
        .build();

    ctx.register_global_property(js_string!("IDBKeyRange"), kr, Attribute::all())
        .expect("failed to register IDBKeyRange");
}

/// Build a JS representation of an `IDBKeyRange`.
fn build_range_object(
    ctx: &mut Context,
    lower: JsValue,
    upper: JsValue,
    lower_open: bool,
    upper_open: bool,
) -> JsValue {
    let obj = ObjectInitializer::new(ctx)
        .property(js_string!("lower"), lower, Attribute::all())
        .property(js_string!("upper"), upper, Attribute::all())
        .property(
            js_string!("lowerOpen"),
            JsValue::from(lower_open),
            Attribute::all(),
        )
        .property(
            js_string!("upperOpen"),
            JsValue::from(upper_open),
            Attribute::all(),
        )
        .property(
            js_string!("__elidex_idb_range__"),
            JsValue::from(true),
            Attribute::empty(),
        )
        .build();

    // includes(key) method
    let range_ref = obj.clone();
    let includes_fn = NativeFunction::from_copy_closure_with_captures(
        |_, args, range_obj, ctx| {
            let key = js_value_to_idb_key(args.first().unwrap_or(&JsValue::undefined()), ctx)?;
            let range =
                super::object_store::extract_range(Some(&JsValue::from(range_obj.clone())), ctx)?;
            match range {
                Some(r) => Ok(JsValue::from(r.includes(&key))),
                None => Ok(JsValue::from(false)),
            }
        },
        range_ref,
    );
    let _ = obj.set(
        js_string!("includes"),
        JsValue::from(includes_fn.to_js_function(ctx.realm())),
        false,
        ctx,
    );

    JsValue::from(obj)
}

/// `IDBKeyRange.only(value)`
fn kr_only(args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let val = args
        .first()
        .ok_or_else(|| JsNativeError::typ().with_message("IDBKeyRange.only requires a value"))?;
    let _ = js_value_to_idb_key(val, ctx)?; // validate it's a valid key
    Ok(build_range_object(
        ctx,
        val.clone(),
        val.clone(),
        false,
        false,
    ))
}

/// `IDBKeyRange.lowerBound(lower, open?)`
fn kr_lower_bound(args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let lower = args.first().ok_or_else(|| {
        JsNativeError::typ().with_message("IDBKeyRange.lowerBound requires a lower bound")
    })?;
    let _ = js_value_to_idb_key(lower, ctx)?;
    let open = args.get(1).is_some_and(JsValue::to_boolean);
    Ok(build_range_object(
        ctx,
        lower.clone(),
        JsValue::undefined(),
        open,
        false,
    ))
}

/// `IDBKeyRange.upperBound(upper, open?)`
fn kr_upper_bound(args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let upper = args.first().ok_or_else(|| {
        JsNativeError::typ().with_message("IDBKeyRange.upperBound requires an upper bound")
    })?;
    let _ = js_value_to_idb_key(upper, ctx)?;
    let open = args.get(1).is_some_and(JsValue::to_boolean);
    Ok(build_range_object(
        ctx,
        JsValue::undefined(),
        upper.clone(),
        false,
        open,
    ))
}

/// `IDBKeyRange.bound(lower, upper, lowerOpen?, upperOpen?)`
fn kr_bound(args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let lower = args.first().ok_or_else(|| {
        JsNativeError::typ().with_message("IDBKeyRange.bound requires lower and upper bounds")
    })?;
    let upper = args.get(1).ok_or_else(|| {
        JsNativeError::typ().with_message("IDBKeyRange.bound requires lower and upper bounds")
    })?;
    let lower_key = js_value_to_idb_key(lower, ctx)?;
    let upper_key = js_value_to_idb_key(upper, ctx)?;
    let lower_open = args.get(2).is_some_and(JsValue::to_boolean);
    let upper_open = args.get(3).is_some_and(JsValue::to_boolean);

    // W3C §4.7: If lower > upper, throw DataError. If equal and either open, throw DataError.
    match lower_key.cmp(&upper_key) {
        Ordering::Greater => {
            return Err(JsNativeError::typ()
                .with_message("DataError: lower bound is greater than upper bound")
                .into());
        }
        Ordering::Equal if lower_open || upper_open => {
            return Err(JsNativeError::typ()
                .with_message("DataError: bounds are equal but one is open")
                .into());
        }
        _ => {}
    }

    Ok(build_range_object(
        ctx,
        lower.clone(),
        upper.clone(),
        lower_open,
        upper_open,
    ))
}
