//! `navigator.serviceWorker` JS bindings (WHATWG SW §4.4).
//!
//! Registers the `ServiceWorkerContainer` on `navigator`.

use boa_engine::{
    js_string, object::ObjectInitializer, Context, JsNativeError, JsValue, NativeFunction,
};

use crate::bridge::HostBridge;

/// Register `navigator.serviceWorker` (ServiceWorkerContainer).
pub fn register_service_worker(ctx: &mut Context, bridge: &HostBridge) {
    let container = build_sw_container(ctx, bridge);

    // Get or create navigator object and set serviceWorker on it.
    let global = ctx.global_object();
    if let Ok(nav) = global.get(js_string!("navigator"), ctx) {
        if let Some(nav_obj) = nav.as_object() {
            let _ = nav_obj.set(js_string!("serviceWorker"), container, false, ctx);
        }
    }
}

fn build_sw_container(ctx: &mut Context, bridge: &HostBridge) -> JsValue {
    // register(scriptURL, options?) → queues SW registration request
    let b = bridge.clone();
    let register_fn = NativeFunction::from_copy_closure_with_captures(
        |_this, args, bridge, ctx| {
            let script_url = args
                .first()
                .and_then(JsValue::as_string)
                .map(|s| s.to_std_string_escaped())
                .ok_or_else(|| {
                    JsNativeError::typ()
                        .with_message("serviceWorker.register requires a script URL")
                })?;

            let scope = args
                .get(1)
                .and_then(JsValue::as_object)
                .and_then(|opts| opts.get(js_string!("scope"), ctx).ok())
                .and_then(|v| v.as_string().map(|s| s.to_std_string_escaped()));

            // Queue the registration request in the bridge (scope included).
            bridge.queue_sw_register(script_url, scope);

            // Return a resolved promise per spec (install is async).
            let promise =
                boa_engine::object::builtins::JsPromise::resolve(JsValue::undefined(), ctx);
            Ok(promise.into())
        },
        b,
    );

    // getRegistration(scope?) → returns undefined (stub)
    let get_registration_fn = NativeFunction::from_fn_ptr(|_, _, _| Ok(JsValue::undefined()));

    // getRegistrations() → returns empty array (stub)
    let get_registrations_fn = NativeFunction::from_fn_ptr(|_, _, ctx| {
        let arr = boa_engine::object::builtins::JsArray::new(ctx);
        Ok(arr.into())
    });

    // startMessages() → no-op (stub)
    let start_messages_fn = NativeFunction::from_fn_ptr(|_, _, _| Ok(JsValue::undefined()));

    let container = ObjectInitializer::new(ctx)
        .function(register_fn, js_string!("register"), 1)
        .function(get_registration_fn, js_string!("getRegistration"), 0)
        .function(get_registrations_fn, js_string!("getRegistrations"), 0)
        .function(start_messages_fn, js_string!("startMessages"), 0)
        .property(
            js_string!("controller"),
            JsValue::null(),
            boa_engine::property::Attribute::CONFIGURABLE,
        )
        .build();

    JsValue::from(container)
}
