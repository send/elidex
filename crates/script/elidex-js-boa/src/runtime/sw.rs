//! Service Worker-specific runtime support.
//!
//! - `register_sw_dispatch_helpers`: registers `registration`, `skipWaiting`, `clients` globals
//! - `dispatch_sw_event`: dispatches SW events via bridge callback storage

use boa_engine::{js_string, object::ObjectInitializer, Context, JsValue, NativeFunction};
use elidex_ecs::{EcsDom, Entity};
use elidex_script_session::SessionCore;

use super::{JsRuntime, UnbindGuard};

/// Register SW-specific globals via NativeFunction closures.
///
/// `addEventListener`/`removeEventListener`/`dispatchEvent` are already
/// registered by `register_worker_event_target()` and use the bridge's
/// worker event listener storage. This function adds SW-only globals:
/// `registration`, `skipWaiting`, `clients`.
#[allow(clippy::too_many_lines)]
pub(super) fn register_sw_dispatch_helpers(ctx: &mut Context, scope: &url::Url) {
    let scope_str = scope.to_string();

    // self.registration (ServiceWorkerRegistration stub)
    let sync_obj = ObjectInitializer::new(ctx)
        .function(
            NativeFunction::from_fn_ptr(|_, _, _| Ok(JsValue::undefined())),
            js_string!("register"),
            1,
        )
        .function(
            NativeFunction::from_fn_ptr(|_, _, ctx| {
                let arr = boa_engine::object::builtins::JsArray::new(ctx);
                Ok(arr.into())
            }),
            js_string!("getTags"),
            0,
        )
        .build();

    let registration = ObjectInitializer::new(ctx)
        .property(
            js_string!("scope"),
            JsValue::from(js_string!(scope_str)),
            boa_engine::property::Attribute::READONLY,
        )
        .property(
            js_string!("active"),
            JsValue::null(),
            boa_engine::property::Attribute::CONFIGURABLE,
        )
        .property(
            js_string!("waiting"),
            JsValue::null(),
            boa_engine::property::Attribute::CONFIGURABLE,
        )
        .property(
            js_string!("installing"),
            JsValue::null(),
            boa_engine::property::Attribute::CONFIGURABLE,
        )
        .property(
            js_string!("updateViaCache"),
            JsValue::from(js_string!("imports")),
            boa_engine::property::Attribute::READONLY,
        )
        .function(
            NativeFunction::from_fn_ptr(|_, _, _| Ok(JsValue::undefined())),
            js_string!("update"),
            0,
        )
        .function(
            NativeFunction::from_fn_ptr(|_, _, _| Ok(JsValue::from(true))),
            js_string!("unregister"),
            0,
        )
        .function(
            NativeFunction::from_fn_ptr(|_, _, _| Ok(JsValue::undefined())),
            js_string!("showNotification"),
            2,
        )
        .function(
            NativeFunction::from_fn_ptr(|_, _, ctx| {
                let arr = boa_engine::object::builtins::JsArray::new(ctx);
                Ok(arr.into())
            }),
            js_string!("getNotifications"),
            0,
        )
        .property(
            js_string!("sync"),
            JsValue::from(sync_obj),
            boa_engine::property::Attribute::READONLY,
        )
        .build();

    let global = ctx.global_object();
    global
        .set(
            js_string!("registration"),
            JsValue::from(registration),
            false,
            ctx,
        )
        .expect("failed to register registration");

    // self.skipWaiting()
    let skip_fn = NativeFunction::from_fn_ptr(|_, _, _| Ok(JsValue::undefined()));
    ctx.register_global_builtin_callable(js_string!("skipWaiting"), 0, skip_fn)
        .expect("failed to register skipWaiting");

    // self.clients
    let clients = ObjectInitializer::new(ctx)
        .function(
            NativeFunction::from_fn_ptr(|_, _, _| Ok(JsValue::undefined())),
            js_string!("claim"),
            0,
        )
        .function(
            NativeFunction::from_fn_ptr(|_, _, ctx| {
                let arr = boa_engine::object::builtins::JsArray::new(ctx);
                Ok(arr.into())
            }),
            js_string!("matchAll"),
            0,
        )
        .function(
            NativeFunction::from_fn_ptr(|_, _, _| Ok(JsValue::undefined())),
            js_string!("get"),
            1,
        )
        .function(
            NativeFunction::from_fn_ptr(|_, _, _| Ok(JsValue::null())),
            js_string!("openWindow"),
            1,
        )
        .build();

    global
        .set(js_string!("clients"), JsValue::from(clients), false, ctx)
        .expect("failed to register clients");
}

impl JsRuntime {
    /// Dispatch a SW event by calling registered listeners via the bridge.
    ///
    /// Uses `worker_get_callbacks(event_type)` to get listeners registered
    /// via `addEventListener`, builds an event object, and calls each listener.
    /// This avoids string-based JS eval and injection risks.
    pub fn dispatch_sw_event(
        &mut self,
        session: &mut SessionCore,
        dom: &mut EcsDom,
        document_entity: Entity,
        event_type: &str,
        event_props: &[(&str, JsValue)],
    ) -> bool {
        self.bridge.bind(session, dom, document_entity);
        let _guard = UnbindGuard(&self.bridge);

        let callbacks = self.bridge.worker_get_callbacks(event_type);
        if callbacks.is_empty() {
            return true; // No listeners — success
        }

        // Build event object with type and custom properties.
        let event_obj = ObjectInitializer::new(&mut self.ctx)
            .property(
                js_string!("type"),
                JsValue::from(js_string!(event_type)),
                boa_engine::property::Attribute::READONLY,
            )
            .function(
                NativeFunction::from_fn_ptr(|_, _, _| Ok(JsValue::undefined())),
                js_string!("waitUntil"),
                1,
            )
            .build();

        for (key, val) in event_props {
            let _ = event_obj.set(js_string!(*key), val.clone(), false, &mut self.ctx);
        }

        let event_val = JsValue::from(event_obj);
        let global = self.ctx.global_object();
        let mut success = true;
        for cb in callbacks {
            let result = cb.call(
                &JsValue::from(global.clone()),
                std::slice::from_ref(&event_val),
                &mut self.ctx,
            );
            if result.is_err() {
                success = false;
            }
        }

        let _ = self.ctx.run_jobs();
        success
    }
}
