//! `localStorage` and `sessionStorage` (WHATWG HTML §11.2).
//!
//! - `sessionStorage`: tab-scoped, `HashMap` in `HostBridgeInner`.
//! - `localStorage`: origin-scoped, disk-persisted via JSON files.

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsNativeError, JsValue, NativeFunction};

use crate::bridge::HostBridge;

/// 5 MB quota per origin (WHATWG HTML §11.2.1).
const STORAGE_QUOTA_BYTES: usize = 5 * 1024 * 1024;

/// Register `localStorage` and `sessionStorage` globals.
pub fn register_storage(ctx: &mut Context, bridge: &HostBridge) {
    let b = bridge.clone();
    let local = build_storage_object(ctx, &b, true);
    ctx.register_global_property(js_string!("localStorage"), local, Attribute::all())
        .expect("failed to register localStorage");

    let b = bridge.clone();
    let session = build_storage_object(ctx, &b, false);
    ctx.register_global_property(js_string!("sessionStorage"), session, Attribute::all())
        .expect("failed to register sessionStorage");
}

/// Build a Storage object (shared implementation for local/session).
fn build_storage_object(
    ctx: &mut Context,
    bridge: &HostBridge,
    is_local: bool,
) -> JsValue {
    let mut init = ObjectInitializer::new(ctx);
    let realm = init.context().realm().clone();

    // length — getter
    let b = bridge.clone();
    let length_getter = NativeFunction::from_copy_closure_with_captures(
        move |_this, _args, bridge, _ctx| {
            let len = if is_local {
                bridge.local_storage_len()
            } else {
                bridge.session_storage_len()
            };
            #[allow(clippy::cast_precision_loss)]
            Ok(JsValue::from(len as f64))
        },
        b,
    )
    .to_js_function(&realm);
    init.accessor(
        js_string!("length"),
        Some(length_getter),
        None,
        Attribute::CONFIGURABLE,
    );

    // getItem(key)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            move |_this, args, bridge, ctx| {
                let key = args
                    .first()
                    .map(|v| v.to_string(ctx))
                    .transpose()?
                    .map(|s| s.to_std_string_escaped())
                    .unwrap_or_default();
                let val = if is_local {
                    bridge.local_storage_get(&key)
                } else {
                    bridge.session_storage_get(&key)
                };
                Ok(val.map_or(JsValue::null(), |v| JsValue::from(js_string!(v.as_str()))))
            },
            b,
        ),
        js_string!("getItem"),
        1,
    );

    // setItem(key, value)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            move |_this, args, bridge, ctx| {
                let key = args
                    .first()
                    .map(|v| v.to_string(ctx))
                    .transpose()?
                    .map(|s| s.to_std_string_escaped())
                    .unwrap_or_default();
                let value = args
                    .get(1)
                    .map(|v| v.to_string(ctx))
                    .transpose()?
                    .map(|s| s.to_std_string_escaped())
                    .unwrap_or_default();

                // Quota check.
                let current_size = if is_local {
                    bridge.local_storage_byte_size()
                } else {
                    bridge.session_storage_byte_size()
                };
                let new_entry_size = key.len() + value.len();
                if current_size + new_entry_size > STORAGE_QUOTA_BYTES {
                    return Err(JsNativeError::eval()
                        .with_message("QuotaExceededError: storage quota exceeded")
                        .into());
                }

                if is_local {
                    bridge.local_storage_set(&key, &value);
                } else {
                    bridge.session_storage_set(&key, &value);
                }
                Ok(JsValue::undefined())
            },
            b,
        ),
        js_string!("setItem"),
        2,
    );

    // removeItem(key)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            move |_this, args, bridge, ctx| {
                let key = args
                    .first()
                    .map(|v| v.to_string(ctx))
                    .transpose()?
                    .map(|s| s.to_std_string_escaped())
                    .unwrap_or_default();
                if is_local {
                    bridge.local_storage_remove(&key);
                } else {
                    bridge.session_storage_remove(&key);
                }
                Ok(JsValue::undefined())
            },
            b,
        ),
        js_string!("removeItem"),
        1,
    );

    // clear()
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            move |_this, _args, bridge, _ctx| {
                if is_local {
                    bridge.local_storage_clear();
                } else {
                    bridge.session_storage_clear();
                }
                Ok(JsValue::undefined())
            },
            b,
        ),
        js_string!("clear"),
        0,
    );

    // key(index)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            move |_this, args, bridge, _ctx| {
                let index = args
                    .first()
                    .and_then(JsValue::as_number)
                    .unwrap_or(0.0);
                if !index.is_finite() || index < 0.0 {
                    return Ok(JsValue::null());
                }
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let idx = index as usize;
                let key = if is_local {
                    bridge.local_storage_key(idx)
                } else {
                    bridge.session_storage_key(idx)
                };
                Ok(key.map_or(JsValue::null(), |k| JsValue::from(js_string!(k.as_str()))))
            },
            b,
        ),
        js_string!("key"),
        1,
    );

    init.build().into()
}
