//! `FormData` API (WHATWG XHR §4.3).

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsNativeError, JsObject, JsResult, JsValue, NativeFunction};

use crate::bridge::HostBridge;

/// Hidden property key storing the entries Vec serialized as JSON-like pairs.
const ENTRIES_KEY: &str = "__elidex_formdata_entries__";
/// Hidden property key marking an object as FormData.
const FORMDATA_MARKER: &str = "__elidex_formdata__";

/// Register `FormData` constructor on the global object.
pub fn register_form_data(ctx: &mut Context, bridge: &HostBridge) {
    let b = bridge.clone();
    let constructor = NativeFunction::from_copy_closure_with_captures(
        |_this, args, bridge, ctx| {
            let fd = create_form_data_object(ctx)?;

            // If a form element is passed, collect its control values (WHATWG XHR §4.3).
            if let Some(form_val) = args.first() {
                if let Some(form_obj) = form_val.as_object() {
                    collect_form_controls(&fd, &form_obj, bridge, ctx)?;
                }
            }

            Ok(JsValue::from(fd))
        },
        b,
    );
    ctx.register_global_callable(js_string!("FormData"), 0, constructor)
        .expect("failed to register FormData");
}

/// Walk form controls under a form element via the bridge and extract name/value pairs.
fn collect_form_controls(
    fd: &JsObject,
    form_obj: &JsObject,
    bridge: &HostBridge,
    ctx: &mut Context,
) -> JsResult<()> {
    use crate::globals::element::ENTITY_KEY;

    // Check if the argument is an element (has __elidex_entity__).
    let entity_val = form_obj.get(js_string!(ENTITY_KEY), ctx)?;
    let entity_bits = match entity_val.as_number() {
        Some(n) if n > 0.0 => n as u64,
        _ => return Ok(()), // Not an element — silently ignore.
    };

    // Use the bridge to walk form controls and collect name/value pairs.
    let pairs = bridge.collect_form_data(entity_bits);

    let entries = fd.get(js_string!(ENTRIES_KEY), ctx)?;
    let arr = entries.as_object().ok_or_else(|| {
        JsNativeError::typ().with_message("FormData: internal error")
    })?;

    for (name, value) in pairs {
        let len = arr.get(js_string!("length"), ctx)?.to_number(ctx)? as u32;
        let pair = boa_engine::object::builtins::JsArray::new(ctx);
        pair.push(JsValue::from(js_string!(name.as_str())), ctx)?;
        pair.push(JsValue::from(js_string!(value.as_str())), ctx)?;
        arr.set(len, JsValue::from(pair), false, ctx)?;
    }

    Ok(())
}

/// Create a new `FormData` JS object with all methods.
fn create_form_data_object(ctx: &mut Context) -> JsResult<boa_engine::JsObject> {
    // Entries stored as a JsArray of [name, value] pairs.
    let entries = boa_engine::object::builtins::JsArray::new(ctx);

    let mut init = ObjectInitializer::new(ctx);

    init.property(js_string!(FORMDATA_MARKER), JsValue::from(true), Attribute::empty());
    init.property(
        js_string!(ENTRIES_KEY),
        JsValue::from(entries),
        Attribute::WRITABLE | Attribute::CONFIGURABLE,
    );

    // append(name, value, filename?)
    init.function(
        NativeFunction::from_copy_closure(|this, args, ctx| {
            let obj = this.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("FormData: this is not an object")
            })?;
            let name = crate::globals::require_js_string_arg(args, 0, "FormData.append", ctx)?;
            let value = args.get(1).cloned().unwrap_or(JsValue::undefined());

            let entries = obj.get(js_string!(ENTRIES_KEY), ctx)?;
            let arr = entries.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("FormData: internal error")
            })?;
            let len = arr.get(js_string!("length"), ctx)?.to_number(ctx)? as u32;

            let pair = boa_engine::object::builtins::JsArray::new(ctx);
            pair.push(JsValue::from(js_string!(name.as_str())), ctx)?;
            pair.push(value, ctx)?;
            arr.set(len, JsValue::from(pair), false, ctx)?;

            Ok(JsValue::undefined())
        }),
        js_string!("append"),
        2,
    );

    // delete(name)
    init.function(
        NativeFunction::from_copy_closure(|this, args, ctx| {
            let obj = this.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("FormData: this is not an object")
            })?;
            let name = crate::globals::require_js_string_arg(args, 0, "FormData.delete", ctx)?;

            let entries = obj.get(js_string!(ENTRIES_KEY), ctx)?;
            let arr = entries.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("FormData: internal error")
            })?;

            // Rebuild array without matching entries.
            let new_arr = boa_engine::object::builtins::JsArray::new(ctx);
            let len = arr.get(js_string!("length"), ctx)?.to_number(ctx)? as u32;
            for i in 0..len {
                let pair = arr.get(i, ctx)?;
                if let Some(pair_obj) = pair.as_object() {
                    let entry_name = pair_obj.get(0, ctx)?.to_string(ctx)?.to_std_string_escaped();
                    if entry_name != name {
                        let new_len =
                            new_arr.get(js_string!("length"), ctx)?.to_number(ctx)? as u32;
                        new_arr.push(pair, ctx)?;
                        let _ = new_len;
                    }
                }
            }
            obj.set(js_string!(ENTRIES_KEY), JsValue::from(new_arr), false, ctx)?;
            Ok(JsValue::undefined())
        }),
        js_string!("delete"),
        1,
    );

    // get(name) → value or null
    init.function(
        NativeFunction::from_copy_closure(|this, args, ctx| {
            let obj = this.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("FormData: this is not an object")
            })?;
            let name = crate::globals::require_js_string_arg(args, 0, "FormData.get", ctx)?;

            let entries = obj.get(js_string!(ENTRIES_KEY), ctx)?;
            let arr = entries.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("FormData: internal error")
            })?;
            let len = arr.get(js_string!("length"), ctx)?.to_number(ctx)? as u32;
            for i in 0..len {
                let pair = arr.get(i, ctx)?;
                if let Some(pair_obj) = pair.as_object() {
                    let entry_name = pair_obj.get(0, ctx)?.to_string(ctx)?.to_std_string_escaped();
                    if entry_name == name {
                        return pair_obj.get(1, ctx);
                    }
                }
            }
            Ok(JsValue::null())
        }),
        js_string!("get"),
        1,
    );

    // getAll(name) → array
    init.function(
        NativeFunction::from_copy_closure(|this, args, ctx| {
            let obj = this.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("FormData: this is not an object")
            })?;
            let name = crate::globals::require_js_string_arg(args, 0, "FormData.getAll", ctx)?;

            let entries = obj.get(js_string!(ENTRIES_KEY), ctx)?;
            let arr = entries.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("FormData: internal error")
            })?;
            let result = boa_engine::object::builtins::JsArray::new(ctx);
            let len = arr.get(js_string!("length"), ctx)?.to_number(ctx)? as u32;
            for i in 0..len {
                let pair = arr.get(i, ctx)?;
                if let Some(pair_obj) = pair.as_object() {
                    let entry_name = pair_obj.get(0, ctx)?.to_string(ctx)?.to_std_string_escaped();
                    if entry_name == name {
                        let val = pair_obj.get(1, ctx)?;
                        result.push(val, ctx)?;
                    }
                }
            }
            Ok(JsValue::from(result))
        }),
        js_string!("getAll"),
        1,
    );

    // has(name) → bool
    init.function(
        NativeFunction::from_copy_closure(|this, args, ctx| {
            let obj = this.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("FormData: this is not an object")
            })?;
            let name = crate::globals::require_js_string_arg(args, 0, "FormData.has", ctx)?;

            let entries = obj.get(js_string!(ENTRIES_KEY), ctx)?;
            let arr = entries.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("FormData: internal error")
            })?;
            let len = arr.get(js_string!("length"), ctx)?.to_number(ctx)? as u32;
            for i in 0..len {
                let pair = arr.get(i, ctx)?;
                if let Some(pair_obj) = pair.as_object() {
                    let entry_name = pair_obj.get(0, ctx)?.to_string(ctx)?.to_std_string_escaped();
                    if entry_name == name {
                        return Ok(JsValue::from(true));
                    }
                }
            }
            Ok(JsValue::from(false))
        }),
        js_string!("has"),
        1,
    );

    // set(name, value, filename?)
    init.function(
        NativeFunction::from_copy_closure(|this, args, ctx| {
            let obj = this.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("FormData: this is not an object")
            })?;
            let name = crate::globals::require_js_string_arg(args, 0, "FormData.set", ctx)?;
            let value = args.get(1).cloned().unwrap_or(JsValue::undefined());

            let entries = obj.get(js_string!(ENTRIES_KEY), ctx)?;
            let arr = entries.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("FormData: internal error")
            })?;

            // Remove all existing entries with this name, then add at the position
            // of the first removed entry (WHATWG XHR §4.3).
            let new_arr = boa_engine::object::builtins::JsArray::new(ctx);
            let len = arr.get(js_string!("length"), ctx)?.to_number(ctx)? as u32;
            let mut inserted = false;
            for i in 0..len {
                let pair = arr.get(i, ctx)?;
                if let Some(pair_obj) = pair.as_object() {
                    let entry_name = pair_obj.get(0, ctx)?.to_string(ctx)?.to_std_string_escaped();
                    if entry_name == name {
                        if !inserted {
                            // Insert the new value at the first occurrence position.
                            let new_pair = boa_engine::object::builtins::JsArray::new(ctx);
                            new_pair.push(JsValue::from(js_string!(name.as_str())), ctx)?;
                            new_pair.push(value.clone(), ctx)?;
                            new_arr.push(JsValue::from(new_pair), ctx)?;
                            inserted = true;
                        }
                        // Skip subsequent entries with the same name.
                    } else {
                        new_arr.push(pair, ctx)?;
                    }
                }
            }
            if !inserted {
                let new_pair = boa_engine::object::builtins::JsArray::new(ctx);
                new_pair.push(JsValue::from(js_string!(name.as_str())), ctx)?;
                new_pair.push(value, ctx)?;
                new_arr.push(JsValue::from(new_pair), ctx)?;
            }
            obj.set(js_string!(ENTRIES_KEY), JsValue::from(new_arr), false, ctx)?;
            Ok(JsValue::undefined())
        }),
        js_string!("set"),
        2,
    );

    // entries() → iterator-like array of [name, value]
    init.function(
        NativeFunction::from_copy_closure(|this, _args, ctx| {
            let obj = this.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("FormData: this is not an object")
            })?;
            let entries = obj.get(js_string!(ENTRIES_KEY), ctx)?;
            // Return a copy of the entries array.
            Ok(entries)
        }),
        js_string!("entries"),
        0,
    );

    // keys() → array of names
    init.function(
        NativeFunction::from_copy_closure(|this, _args, ctx| {
            let obj = this.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("FormData: this is not an object")
            })?;
            let entries = obj.get(js_string!(ENTRIES_KEY), ctx)?;
            let arr = entries.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("FormData: internal error")
            })?;
            let result = boa_engine::object::builtins::JsArray::new(ctx);
            let len = arr.get(js_string!("length"), ctx)?.to_number(ctx)? as u32;
            for i in 0..len {
                let pair = arr.get(i, ctx)?;
                if let Some(pair_obj) = pair.as_object() {
                    let name = pair_obj.get(0, ctx)?;
                    result.push(name, ctx)?;
                }
            }
            Ok(JsValue::from(result))
        }),
        js_string!("keys"),
        0,
    );

    // values() → array of values
    init.function(
        NativeFunction::from_copy_closure(|this, _args, ctx| {
            let obj = this.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("FormData: this is not an object")
            })?;
            let entries = obj.get(js_string!(ENTRIES_KEY), ctx)?;
            let arr = entries.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("FormData: internal error")
            })?;
            let result = boa_engine::object::builtins::JsArray::new(ctx);
            let len = arr.get(js_string!("length"), ctx)?.to_number(ctx)? as u32;
            for i in 0..len {
                let pair = arr.get(i, ctx)?;
                if let Some(pair_obj) = pair.as_object() {
                    let val = pair_obj.get(1, ctx)?;
                    result.push(val, ctx)?;
                }
            }
            Ok(JsValue::from(result))
        }),
        js_string!("values"),
        0,
    );

    // forEach(callback, thisArg?)
    init.function(
        NativeFunction::from_copy_closure(|this, args, ctx| {
            let obj = this.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("FormData: this is not an object")
            })?;
            let callback = args.first().and_then(JsValue::as_callable).ok_or_else(|| {
                JsNativeError::typ()
                    .with_message("FormData.forEach: argument must be a function")
            })?;
            let this_arg = args.get(1).cloned().unwrap_or(JsValue::undefined());

            let entries = obj.get(js_string!(ENTRIES_KEY), ctx)?;
            let arr = entries.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("FormData: internal error")
            })?;
            let len = arr.get(js_string!("length"), ctx)?.to_number(ctx)? as u32;
            for i in 0..len {
                let pair = arr.get(i, ctx)?;
                if let Some(pair_obj) = pair.as_object() {
                    let name = pair_obj.get(0, ctx)?;
                    let value = pair_obj.get(1, ctx)?;
                    callback.call(&this_arg, &[value, name, JsValue::from(obj.clone())], ctx)?;
                }
            }
            Ok(JsValue::undefined())
        }),
        js_string!("forEach"),
        1,
    );

    Ok(init.build())
}

/// Check if a JS value is a FormData object.
#[allow(dead_code)]
pub(crate) fn is_form_data(val: &JsValue, ctx: &mut Context) -> bool {
    val.as_object().is_some_and(|obj| {
        obj.get(js_string!(FORMDATA_MARKER), ctx)
            .ok()
            .is_some_and(|v| v.to_boolean())
    })
}
