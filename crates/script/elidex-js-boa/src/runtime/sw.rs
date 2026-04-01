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
    let skip_fn = NativeFunction::from_fn_ptr(|_, _, ctx| {
        let p = boa_engine::object::builtins::JsPromise::resolve(JsValue::undefined(), ctx);
        Ok(p.into())
    });
    ctx.register_global_builtin_callable(js_string!("skipWaiting"), 0, skip_fn)
        .expect("failed to register skipWaiting");

    // self.clients — all methods return Promises per spec.
    let clients = ObjectInitializer::new(ctx)
        .function(
            NativeFunction::from_fn_ptr(|_, _, ctx| {
                let p = boa_engine::object::builtins::JsPromise::resolve(JsValue::undefined(), ctx);
                Ok(p.into())
            }),
            js_string!("claim"),
            0,
        )
        .function(
            NativeFunction::from_fn_ptr(|_, _, ctx| {
                let arr = boa_engine::object::builtins::JsArray::new(ctx);
                let val: JsValue = arr.into();
                let p = boa_engine::object::builtins::JsPromise::resolve(val, ctx);
                Ok(p.into())
            }),
            js_string!("matchAll"),
            0,
        )
        .function(
            NativeFunction::from_fn_ptr(|_, _, ctx| {
                let p = boa_engine::object::builtins::JsPromise::resolve(JsValue::undefined(), ctx);
                Ok(p.into())
            }),
            js_string!("get"),
            1,
        )
        .function(
            NativeFunction::from_fn_ptr(|_, _, ctx| {
                let p = boa_engine::object::builtins::JsPromise::resolve(JsValue::null(), ctx);
                Ok(p.into())
            }),
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
    /// SW called respondWith() with a response.
    Responded {
        body: String,
        status: u16,
        status_text: String,
        headers: Vec<(String, String)>,
    },
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
    /// Phase 2 limitation: `waitUntil()` is a no-op stub. Promises passed to it
    /// resolve synchronously via `ctx.run_jobs()` after all callbacks execute.
    /// True promise tracking requires M4-10 (elidex-js VM with async support).
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
    ///
    /// Phase 2 limitation: `respondWith(promise)` is not awaited — only
    /// direct Response or string values are extracted. Promise-based responses
    /// (e.g., `respondWith(fetch(event.request))`) require M4-10 async support.
    #[allow(clippy::too_many_lines)]
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

        let event_obj = build_extendable_event(&mut self.ctx, "fetch", event_props);

        // respondWith() state tracked via a separate internal JS object.
        // Properties are writable (for the closure to update) but non-configurable
        // and non-enumerable. The state object is not exposed to SW scripts.
        let state_obj = ObjectInitializer::new(&mut self.ctx).build();
        let _ = state_obj.define_property_or_throw(
            js_string!("responded"),
            boa_engine::property::PropertyDescriptor::builder()
                .value(JsValue::from(false))
                .writable(true) // writable by respondWith closure only
                .configurable(false)
                .enumerable(false),
            &mut self.ctx,
        );
        let _ = state_obj.define_property_or_throw(
            js_string!("response"),
            boa_engine::property::PropertyDescriptor::builder()
                .value(JsValue::undefined())
                .writable(true)
                .configurable(false)
                .enumerable(false),
            &mut self.ctx,
        );

        let state_clone = state_obj.clone();
        let respond_fn = NativeFunction::from_copy_closure_with_captures(
            |_this, args, state, ctx| {
                let already = state.get(js_string!("responded"), ctx)?.to_boolean();
                if already {
                    return Err(boa_engine::JsNativeError::typ()
                        .with_message("InvalidStateError: respondWith() already called")
                        .into());
                }
                let _ = state.set(js_string!("responded"), JsValue::from(true), false, ctx);
                let response = args.first().cloned().unwrap_or(JsValue::undefined());
                let _ = state.set(js_string!("response"), response, false, ctx);
                Ok(JsValue::undefined())
            },
            state_clone,
        );
        let _ = event_obj.set(
            js_string!("respondWith"),
            respond_fn.to_js_function(self.ctx.realm()),
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

        let did_respond = state_obj
            .get(js_string!("responded"), &mut self.ctx)
            .map(|v| v.to_boolean())
            .unwrap_or(false);

        if did_respond {
            let response = state_obj
                .get(js_string!("response"), &mut self.ctx)
                .unwrap_or(JsValue::undefined());

            // Extract body, status, statusText, headers from the Response.
            // Response constructor stores body in "__body__" hidden property.
            // Plain strings are accepted as shorthand (body only, status 200).
            let (body, status, status_text, headers) = if let Some(s) = response.as_string() {
                (s.to_std_string_escaped(), 200, "OK".into(), vec![])
            } else if let Some(obj) = response.as_object() {
                // Body: try __body__ (Response constructor) then body (plain object).
                let body_str = obj
                    .get(js_string!("__body__"), &mut self.ctx)
                    .ok()
                    .or_else(|| obj.get(js_string!("body"), &mut self.ctx).ok())
                    .and_then(|v| v.as_string().map(|s| s.to_std_string_escaped()))
                    .unwrap_or_default();

                // Status: clamp to valid HTTP range [0, 999].
                let raw_status = obj
                    .get(js_string!("status"), &mut self.ctx)
                    .ok()
                    .and_then(|v| v.to_number(&mut self.ctx).ok())
                    .unwrap_or(200.0);
                let status = if raw_status.is_finite() && (0.0..=999.0).contains(&raw_status) {
                    raw_status as u16
                } else {
                    200
                };

                // StatusText.
                let status_text = obj
                    .get(js_string!("statusText"), &mut self.ctx)
                    .ok()
                    .and_then(|v| v.as_string().map(|s| s.to_std_string_escaped()))
                    .unwrap_or_else(|| "OK".into());

                // Headers: Response stores in __headers__ (NUL-delimited pairs, newline-separated).
                let headers = extract_response_headers(&obj, &mut self.ctx);

                (body_str, status, status_text, headers)
            } else {
                (String::new(), 200, "OK".into(), vec![])
            };

            FetchEventResult::Responded {
                body,
                status,
                status_text,
                headers,
            }
        } else {
            FetchEventResult::Passthrough
        }
    }
}

/// Extract headers from a Response object's Headers sub-object.
///
/// The fetch constructors store headers in a `__headers__` hidden property
/// on the Headers object, using NUL-delimited `name\0value` pairs separated
/// by newlines.
fn extract_response_headers(
    response_obj: &boa_engine::JsObject,
    ctx: &mut boa_engine::Context,
) -> Vec<(String, String)> {
    let Ok(headers_val) = response_obj.get(js_string!("headers"), ctx) else {
        return vec![];
    };
    let Some(headers_obj) = headers_val.as_object() else {
        return vec![];
    };
    let Ok(raw) = headers_obj.get(js_string!("__headers__"), ctx) else {
        return vec![];
    };
    let Some(raw_str) = raw.as_string() else {
        return vec![];
    };
    let raw_str = raw_str.to_std_string_escaped();
    raw_str
        .lines()
        .filter_map(|line| {
            let (name, value) = line.split_once('\0')?;
            Some((name.to_owned(), value.to_owned()))
        })
        .collect()
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
