//! `IDBRequest` and `IDBOpenDBRequest` JS object builders.

use boa_engine::object::ObjectInitializer;
use boa_engine::{js_string, Context, JsObject, JsValue, NativeFunction};

/// `readyState` values.
pub const READY_STATE_PENDING: &str = "pending";
pub const READY_STATE_DONE: &str = "done";

/// Build a basic `IDBRequest` JS object.
///
/// Properties: `result`, `error`, `source`, `readyState`, `transaction`,
/// `onsuccess`, `onerror`.
pub fn build_request(ctx: &mut Context) -> JsObject {
    let obj = ObjectInitializer::new(ctx)
        .property(
            js_string!("result"),
            JsValue::undefined(),
            boa_engine::property::Attribute::all(),
        )
        .property(
            js_string!("error"),
            JsValue::null(),
            boa_engine::property::Attribute::all(),
        )
        .property(
            js_string!("source"),
            JsValue::null(),
            boa_engine::property::Attribute::all(),
        )
        .property(
            js_string!("readyState"),
            js_string!(READY_STATE_PENDING),
            boa_engine::property::Attribute::all(),
        )
        .property(
            js_string!("transaction"),
            JsValue::null(),
            boa_engine::property::Attribute::all(),
        )
        .property(
            js_string!("onsuccess"),
            JsValue::null(),
            boa_engine::property::Attribute::all(),
        )
        .property(
            js_string!("onerror"),
            JsValue::null(),
            boa_engine::property::Attribute::all(),
        )
        .build();

    // addEventListener / removeEventListener stubs
    register_event_target_stubs(&obj, ctx);

    obj
}

/// Build an `IDBOpenDBRequest` JS object (extends `IDBRequest`).
///
/// Additional properties: `onupgradeneeded`, `onblocked`.
pub fn build_open_request(ctx: &mut Context) -> JsObject {
    let obj = build_request(ctx);
    let _ = obj.set(js_string!("onupgradeneeded"), JsValue::null(), false, ctx);
    let _ = obj.set(js_string!("onblocked"), JsValue::null(), false, ctx);
    obj
}

/// Set the request to "done" state with a successful result.
///
/// W3C §5.6: result is set immediately; `onsuccess` is NOT fired inline.
/// The caller can use `fire_handler` separately for synchronous callback dispatch
/// (e.g., `onupgradeneeded`), or rely on `req.result` for the synchronous model.
pub fn resolve_request(request: &JsObject, result: JsValue, ctx: &mut Context) {
    let _ = request.set(
        js_string!("readyState"),
        js_string!(READY_STATE_DONE),
        false,
        ctx,
    );
    let _ = request.set(js_string!("result"), result, false, ctx);
    let _ = request.set(js_string!("error"), JsValue::null(), false, ctx);
}

/// Set the request to "done" state with an error.
///
/// `onerror` is NOT fired inline (same deferred model as `resolve_request`).
pub fn reject_request(request: &JsObject, error_msg: &str, ctx: &mut Context) {
    reject_request_with_name(request, "UnknownError", error_msg, ctx);
}

/// Reject with a `BackendError`, using its `DOMException` name.
#[allow(dead_code)] // Foundation for B-gap: callers will be updated to use this
pub fn reject_request_backend(
    request: &JsObject,
    err: &elidex_indexeddb::BackendError,
    ctx: &mut Context,
) {
    reject_request_with_name(request, err.dom_exception_name(), &err.to_string(), ctx);
}

/// Set the request to "done" state with a named `DOMException`.
pub fn reject_request_with_name(
    request: &JsObject,
    error_name: &str,
    error_msg: &str,
    ctx: &mut Context,
) {
    let _ = request.set(
        js_string!("readyState"),
        js_string!(READY_STATE_DONE),
        false,
        ctx,
    );
    let _ = request.set(js_string!("result"), JsValue::undefined(), false, ctx);

    let error = create_dom_exception(error_name, error_msg, ctx);
    let _ = request.set(js_string!("error"), error, false, ctx);
}

/// Fire an event handler property (e.g., `onsuccess`, `onerror`, `oncomplete`).
pub fn fire_handler(obj: &JsObject, handler_name: &str, ctx: &mut Context) {
    let handler = obj
        .get(js_string!(handler_name), ctx)
        .unwrap_or(JsValue::null());
    if let Some(func) = handler.as_callable() {
        // Create a minimal event object
        let event = ObjectInitializer::new(ctx)
            .property(
                js_string!("type"),
                js_string!(handler_name.strip_prefix("on").unwrap_or(handler_name)),
                boa_engine::property::Attribute::all(),
            )
            .property(
                js_string!("target"),
                JsValue::from(obj.clone()),
                boa_engine::property::Attribute::all(),
            )
            .build();

        let _ = func.call(&JsValue::from(obj.clone()), &[JsValue::from(event)], ctx);
    }
}

fn create_dom_exception(name: &str, message: &str, ctx: &mut Context) -> JsValue {
    let obj = ObjectInitializer::new(ctx)
        .property(
            js_string!("name"),
            js_string!(name),
            boa_engine::property::Attribute::all(),
        )
        .property(
            js_string!("message"),
            js_string!(message),
            boa_engine::property::Attribute::all(),
        )
        .build();
    JsValue::from(obj)
}

fn register_event_target_stubs(obj: &JsObject, ctx: &mut Context) {
    let noop = NativeFunction::from_copy_closure(|_, _, _| Ok(JsValue::undefined()));
    let _ = obj.set(
        js_string!("addEventListener"),
        JsValue::from(noop.to_js_function(ctx.realm())),
        false,
        ctx,
    );
    let noop2 = NativeFunction::from_copy_closure(|_, _, _| Ok(JsValue::undefined()));
    let _ = obj.set(
        js_string!("removeEventListener"),
        JsValue::from(noop2.to_js_function(ctx.realm())),
        false,
        ctx,
    );
}
