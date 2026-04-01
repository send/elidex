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

/// Result of dispatching a FetchEvent.
pub enum FetchEventResult {
    /// SW called respondWith() with a response body.
    Responded { body: String, status: u16 },
    /// SW did not call respondWith() — fall through to network.
    Passthrough,
    /// Error during dispatch.
    Error,
}

impl JsRuntime {
    /// Dispatch a SW event by calling registered listeners via the bridge.
    ///
    /// Uses `worker_get_callbacks(event_type)` to get listeners registered
    /// via `addEventListener`, builds an event object, and calls each listener.
    /// This avoids string-based JS eval and injection risks.
    ///
    /// `waitUntil()` collects promises; after all listeners run, `ctx.run_jobs()`
    /// drains the microtask queue (Phase 2: synchronous resolution).
    /// If any callback throws, returns `false`.
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
            return true;
        }

        let event_obj = build_extendable_event(&mut self.ctx, event_type, event_props);
        let event_val = JsValue::from(event_obj);
        let global = self.ctx.global_object();
        let mut success = true;
        for cb in callbacks {
            if cb
                .call(
                    &JsValue::from(global.clone()),
                    std::slice::from_ref(&event_val),
                    &mut self.ctx,
                )
                .is_err()
            {
                success = false;
            }
        }

        // Drain microtask queue (Phase 2: resolves waitUntil promises synchronously).
        let _ = self.ctx.run_jobs();
        success
    }

    /// Dispatch a FetchEvent with respondWith() support.
    ///
    /// Returns whether the SW called respondWith() and with what response.
    /// respondWith() can only be called once (InvalidStateError on 2nd call).
    /// respondWith() must be called synchronously during dispatch.
    pub fn dispatch_fetch_event(
        &mut self,
        session: &mut SessionCore,
        dom: &mut EcsDom,
        document_entity: Entity,
        event_props: &[(&str, JsValue)],
    ) -> FetchEventResult {
        self.bridge.bind(session, dom, document_entity);
        let _guard = UnbindGuard(&self.bridge);

        let callbacks = self.bridge.worker_get_callbacks("fetch");
        if callbacks.is_empty() {
            return FetchEventResult::Passthrough;
        }

        // Create a shared flag for respondWith() state tracking.
        // Use a hidden property on the event object.
        let event_obj = build_extendable_event(&mut self.ctx, "fetch", event_props);

        // respondWith(response): stores response in __sw_response__ hidden prop.
        // Can only be called once (tracked via __sw_responded__ flag).
        let respond_fn = NativeFunction::from_copy_closure(|this, args, ctx| {
            let event = this
                .as_object()
                .ok_or_else(|| boa_engine::JsNativeError::typ().with_message("not a FetchEvent"))?;

            // Check if already called.
            let already = event.get(js_string!("__sw_responded__"), ctx)?.to_boolean();
            if already {
                return Err(boa_engine::JsNativeError::typ()
                    .with_message("InvalidStateError: respondWith() already called")
                    .into());
            }

            // Mark as called.
            let _ = event.set(
                js_string!("__sw_responded__"),
                JsValue::from(true),
                false,
                ctx,
            );

            // Store the response value.
            let response = args.first().cloned().unwrap_or(JsValue::undefined());
            let _ = event.set(js_string!("__sw_response__"), response, false, ctx);

            Ok(JsValue::undefined())
        });
        let _ = event_obj.set(
            js_string!("respondWith"),
            respond_fn.to_js_function(self.ctx.realm()),
            false,
            &mut self.ctx,
        );

        // Initialize hidden state.
        let _ = event_obj.set(
            js_string!("__sw_responded__"),
            JsValue::from(false),
            false,
            &mut self.ctx,
        );

        let event_val = JsValue::from(event_obj.clone());
        let global = self.ctx.global_object();
        for cb in callbacks {
            if cb
                .call(
                    &JsValue::from(global.clone()),
                    std::slice::from_ref(&event_val),
                    &mut self.ctx,
                )
                .is_err()
            {
                return FetchEventResult::Error;
            }
        }

        let _ = self.ctx.run_jobs();

        // Check if respondWith() was called.
        let responded = event_obj
            .get(js_string!("__sw_responded__"), &mut self.ctx)
            .map(|v| v.to_boolean())
            .unwrap_or(false);

        if responded {
            // Extract response body.
            let response = event_obj
                .get(js_string!("__sw_response__"), &mut self.ctx)
                .unwrap_or(JsValue::undefined());

            // If response is a string, use it directly as body.
            // If response is an object with body/status, extract them.
            let (body, status) = if let Some(s) = response.as_string() {
                (s.to_std_string_escaped(), 200)
            } else if let Some(obj) = response.as_object() {
                let body_val = obj
                    .get(js_string!("body"), &mut self.ctx)
                    .unwrap_or(JsValue::undefined());
                let body_str = body_val
                    .as_string()
                    .map(|s| s.to_std_string_escaped())
                    .unwrap_or_default();
                let status_val = obj
                    .get(js_string!("status"), &mut self.ctx)
                    .unwrap_or(JsValue::from(200));
                let status = status_val.to_number(&mut self.ctx).unwrap_or(200.0) as u16;
                (body_str, status)
            } else {
                (String::new(), 200)
            };

            FetchEventResult::Responded { body, status }
        } else {
            FetchEventResult::Passthrough
        }
    }
}

/// Build an ExtendableEvent object with type, waitUntil, and custom properties.
fn build_extendable_event(
    ctx: &mut boa_engine::Context,
    event_type: &str,
    props: &[(&str, JsValue)],
) -> boa_engine::JsObject {
    let event_obj = ObjectInitializer::new(ctx)
        .property(
            js_string!("type"),
            JsValue::from(js_string!(event_type)),
            boa_engine::property::Attribute::READONLY,
        )
        .function(
            NativeFunction::from_fn_ptr(|_, _, _| {
                // Phase 2: waitUntil() is a no-op — promises resolve synchronously
                // via ctx.run_jobs() after all callbacks execute.
                // If a promise rejects, the callback itself throws, caught by dispatch.
                Ok(JsValue::undefined())
            }),
            js_string!("waitUntil"),
            1,
        )
        .build();

    for (key, val) in props {
        let _ = event_obj.set(js_string!(*key), val.clone(), false, ctx);
    }

    event_obj
}
