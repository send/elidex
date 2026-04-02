//! Worker global scope registration (WHATWG HTML §10.3).
//!
//! Registers the `WorkerGlobalScope` APIs on the boa `Context` global object
//! for dedicated workers. Omits DOM APIs (`document`, `window`, etc.) and
//! registers worker-specific APIs (`self`, `name`, `postMessage`, `close`,
//! `importScripts`, `WorkerLocation`, `WorkerNavigator`).

use std::rc::Rc;

use crate::bridge::worker_state::OutgoingMessage;
use crate::bridge::HostBridge;
use crate::globals::console::ConsoleOutput;
use crate::globals::timers::TimerQueueHandle;
use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsNativeError, JsValue, NativeFunction};

/// JSON.stringify a JS value, returning the stringified result.
///
/// Used by both worker-side `postMessage` and parent-side `postMessage` to
/// serialize data for cross-thread transport.
///
/// Returns:
/// - `Ok(Some(string))` — successful serialization
/// - `Ok(None)` — `JSON.stringify` returned `undefined` (uncloneable value, e.g. a function)
/// - `Err(())` — `JSON.stringify` threw (e.g. circular reference)
pub(crate) fn js_json_stringify(data: &JsValue, ctx: &mut Context) -> Result<Option<String>, ()> {
    let json_global = ctx.global_object();
    let Ok(json_obj) = json_global.get(js_string!("JSON"), ctx) else {
        return Err(());
    };
    let Some(json_obj) = json_obj.as_object() else {
        return Err(());
    };
    let Ok(stringify_fn) = json_obj.get(js_string!("stringify"), ctx) else {
        return Err(());
    };
    let Some(stringify_fn) = stringify_fn.as_object() else {
        return Err(());
    };
    let Ok(result) = stringify_fn.call(&json_obj.clone().into(), std::slice::from_ref(data), ctx)
    else {
        return Err(());
    };
    // JSON.stringify returns undefined for functions, symbols, etc.
    // Treat undefined as a successful serialization of the JS `undefined` value,
    // encoding it as the string "undefined" so it round-trips through JSON.parse
    // (which will throw, but we catch that on the receiving side and deliver
    // `e.data === undefined`). This is an acceptable approximation until
    // structured clone is implemented.
    if result.is_undefined() {
        // Encode as JSON null — the closest JSON representation of undefined.
        // The receiver will get `null` instead of `undefined`, which is a known
        // limitation of the JSON-based serialization (same as structuredClone's
        // treatment of undefined in JSON contexts).
        return Ok(Some("null".to_string()));
    }
    let Ok(s) = result.to_string(ctx) else {
        return Err(());
    };
    Ok(Some(s.to_std_string_escaped()))
}

/// Register all worker globals on the boa `Context`.
///
/// This is the worker equivalent of `register_all_globals`. It registers
/// worker-specific APIs and a subset of the shared Web Platform APIs.
pub fn register_worker_globals(
    ctx: &mut Context,
    bridge: &HostBridge,
    console_output: &ConsoleOutput,
    timer_queue: &TimerQueueHandle,
    network_handle: Option<Rc<elidex_net::broker::NetworkHandle>>,
) {
    // Clone network_handle before register_fetch moves the original.
    let import_network_handle = network_handle.clone();

    // --- Shared Web Platform APIs ---
    crate::globals::console::register_console(ctx, console_output);
    crate::globals::timers::register_timers(ctx, timer_queue);
    crate::globals::fetch::register_fetch(ctx, network_handle);
    crate::globals::fetch::constructors::register_fetch_constructors(ctx);
    crate::globals::url::register_url_constructors(ctx, bridge);
    crate::globals::encoding::register_encoding(ctx, bridge);
    crate::globals::abort::register_abort_controller(ctx, bridge);
    crate::globals::blob::register_blob_file(ctx);
    crate::globals::form_data::register_form_data(ctx, bridge);
    crate::globals::event_constructors::register_event_constructors(ctx, bridge);
    crate::globals::window::encoding::register_atob_btoa(ctx);
    crate::globals::window::encoding::register_crypto(ctx);
    crate::globals::window::encoding::register_queue_microtask(ctx);
    crate::globals::window::performance::register_performance(ctx, bridge);
    crate::globals::window::dom_parser::register_structured_clone(ctx);

    crate::globals::cache::register_caches(ctx, bridge);
    crate::globals::cookie_store::register_cookie_store(ctx, bridge);

    // --- Worker-specific APIs ---
    register_worker_self(ctx);
    register_worker_name(ctx, bridge);
    register_worker_post_message(ctx, bridge);
    register_worker_close(ctx, bridge);
    register_import_scripts(ctx, bridge, import_network_handle);
    register_worker_location(ctx, bridge);
    register_worker_navigator(ctx);
    register_is_secure_context(ctx, bridge);
    register_worker_event_target(ctx, bridge);
}

/// Register `self` as a reference to the global object.
fn register_worker_self(ctx: &mut Context) {
    let global = ctx.global_object();
    ctx.register_global_property(
        js_string!("self"),
        JsValue::from(global),
        Attribute::CONFIGURABLE,
    )
    .expect("failed to register self");
}

/// Register `name` as a read-only property (Worker name from constructor options).
fn register_worker_name(ctx: &mut Context, bridge: &HostBridge) {
    let name = bridge.worker_name();
    ctx.register_global_property(
        js_string!("name"),
        JsValue::from(js_string!(name.as_str())),
        Attribute::CONFIGURABLE,
    )
    .expect("failed to register name");
}

/// Register `postMessage(data)` on the worker global scope.
fn register_worker_post_message(ctx: &mut Context, bridge: &HostBridge) {
    let b = bridge.clone();
    let post_fn = NativeFunction::from_copy_closure_with_captures(
        |_this, args, bridge, ctx| {
            let data = args.first().cloned().unwrap_or(JsValue::undefined());

            match js_json_stringify(&data, ctx) {
                Ok(Some(json_str)) => {
                    bridge.worker_queue_message(OutgoingMessage::Data(json_str));
                    Ok(JsValue::undefined())
                }
                Ok(None) | Err(()) => Err(JsNativeError::typ()
                    .with_message("DataCloneError: Failed to serialize message")
                    .into()),
            }
        },
        b,
    );
    ctx.register_global_builtin_callable(js_string!("postMessage"), 1, post_fn)
        .expect("failed to register postMessage");
}

/// Register `close()` on the worker global scope (WHATWG HTML §10.3.1).
fn register_worker_close(ctx: &mut Context, bridge: &HostBridge) {
    let b = bridge.clone();
    let close_fn = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, _ctx| {
            bridge.worker_request_close();
            Ok(JsValue::undefined())
        },
        b,
    );
    ctx.register_global_builtin_callable(js_string!("close"), 0, close_fn)
        .expect("failed to register close");
}

/// Captures for `importScripts` closure.
#[derive(Clone)]
struct ImportScriptsCaptures {
    bridge: HostBridge,
    network_handle: Rc<elidex_net::broker::NetworkHandle>,
}

// Trace/Finalize: NetworkHandle + HostBridge contain only Rust types, no GC objects.
impl_empty_trace!(ImportScriptsCaptures);

/// Register `importScripts(...urls)` on the worker global scope (WHATWG HTML §10.3.2).
fn register_import_scripts(
    ctx: &mut Context,
    bridge: &HostBridge,
    network_handle: Option<Rc<elidex_net::broker::NetworkHandle>>,
) {
    let Some(nh) = network_handle else {
        return; // No network handle — importScripts unavailable.
    };
    let captures = ImportScriptsCaptures {
        bridge: bridge.clone(),
        network_handle: nh,
    };
    let import_fn = NativeFunction::from_copy_closure_with_captures(
        |_this, args, captures, ctx| {
            let script_url = captures.bridge.worker_script_url();
            let network_handle = &captures.network_handle;

            for arg in args {
                let url_str = arg.to_string(ctx)?.to_std_string_escaped();

                // Resolve URL relative to worker script URL.
                let resolved = script_url
                    .join(&url_str)
                    .map_err(|e| JsNativeError::typ().with_message(format!("Invalid URL: {e}")))?;
                let request = elidex_net::Request {
                    method: "GET".to_string(),
                    url: resolved.clone(),
                    headers: Vec::new(),
                    body: bytes::Bytes::new(),
                };
                let response = match network_handle.fetch_blocking(request) {
                    Ok(resp) => resp,
                    Err(e) => {
                        return Err(JsNativeError::typ()
                            .with_message(format!("NetworkError: Failed to fetch {resolved}: {e}"))
                            .into());
                    }
                };

                // Validate MIME type and HTTP status (WHATWG HTML §10.3.2).
                let source = crate::globals::worker_constructor::validate_worker_script_response(
                    &response, &resolved,
                )
                .map_err(|msg| JsNativeError::typ().with_message(format!("NetworkError: {msg}")))?;
                ctx.eval(boa_engine::Source::from_bytes(source.as_bytes()))?;
            }

            Ok(JsValue::undefined())
        },
        captures,
    );
    ctx.register_global_builtin_callable(js_string!("importScripts"), 0, import_fn)
        .expect("failed to register importScripts");
}

/// Register `self.location` as a `WorkerLocation` object (WHATWG HTML §10.3.3).
fn register_worker_location(ctx: &mut Context, bridge: &HostBridge) {
    let url = bridge.worker_script_url();

    let mut init = ObjectInitializer::new(ctx);

    let href = url.to_string();
    init.property(
        js_string!("href"),
        JsValue::from(js_string!(href.as_str())),
        Attribute::CONFIGURABLE,
    );

    let origin = url.origin().ascii_serialization();
    init.property(
        js_string!("origin"),
        JsValue::from(js_string!(origin.as_str())),
        Attribute::CONFIGURABLE,
    );

    let protocol = format!("{}:", url.scheme());
    init.property(
        js_string!("protocol"),
        JsValue::from(js_string!(protocol.as_str())),
        Attribute::CONFIGURABLE,
    );

    let host = url
        .host_str()
        .map(|h| {
            if let Some(port) = url.port() {
                format!("{h}:{port}")
            } else {
                h.to_string()
            }
        })
        .unwrap_or_default();
    init.property(
        js_string!("host"),
        JsValue::from(js_string!(host.as_str())),
        Attribute::CONFIGURABLE,
    );

    let hostname = url.host_str().unwrap_or("");
    init.property(
        js_string!("hostname"),
        JsValue::from(js_string!(hostname)),
        Attribute::CONFIGURABLE,
    );

    let port = url.port().map(|p| p.to_string()).unwrap_or_default();
    init.property(
        js_string!("port"),
        JsValue::from(js_string!(port.as_str())),
        Attribute::CONFIGURABLE,
    );

    init.property(
        js_string!("pathname"),
        JsValue::from(js_string!(url.path())),
        Attribute::CONFIGURABLE,
    );

    let search = url.query().map_or(String::new(), |q| format!("?{q}"));
    init.property(
        js_string!("search"),
        JsValue::from(js_string!(search.as_str())),
        Attribute::CONFIGURABLE,
    );

    let hash = url.fragment().map_or(String::new(), |f| format!("#{f}"));
    init.property(
        js_string!("hash"),
        JsValue::from(js_string!(hash.as_str())),
        Attribute::CONFIGURABLE,
    );

    // toString() returns href.
    let href_clone = url.to_string();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, _args, href, _ctx| Ok(JsValue::from(js_string!(href.as_str()))),
            href_clone,
        ),
        js_string!("toString"),
        0,
    );

    let location = init.build();
    ctx.register_global_property(js_string!("location"), location, Attribute::CONFIGURABLE)
        .expect("failed to register location");
}

/// Register `self.navigator` as a `WorkerNavigator` object (WHATWG HTML §10.3.4).
fn register_worker_navigator(ctx: &mut Context) {
    let language = sys_locale::get_locale().unwrap_or_else(|| "en-US".to_string());
    let languages_arr = boa_engine::object::builtins::JsArray::new(ctx);
    let _ = languages_arr.push(JsValue::from(js_string!(language.as_str())), ctx);
    let languages_val: JsValue = languages_arr.into();

    let mut init = ObjectInitializer::new(ctx);

    init.property(
        js_string!("userAgent"),
        JsValue::from(js_string!("elidex/0.1")),
        Attribute::CONFIGURABLE,
    );

    init.property(
        js_string!("language"),
        JsValue::from(js_string!(language.as_str())),
        Attribute::CONFIGURABLE,
    );

    init.property(
        js_string!("languages"),
        languages_val,
        Attribute::CONFIGURABLE,
    );

    init.property(
        js_string!("onLine"),
        JsValue::from(true),
        Attribute::CONFIGURABLE,
    );

    let cores = std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get);
    #[allow(clippy::cast_precision_loss)]
    init.property(
        js_string!("hardwareConcurrency"),
        JsValue::from(cores as f64),
        Attribute::CONFIGURABLE,
    );

    let navigator = init.build();
    ctx.register_global_property(js_string!("navigator"), navigator, Attribute::CONFIGURABLE)
        .expect("failed to register navigator");
}

/// Register `isSecureContext` on the worker global scope (WHATWG HTML §7.1.2).
fn register_is_secure_context(ctx: &mut Context, bridge: &HostBridge) {
    let url = bridge.worker_script_url();
    let is_secure = url.scheme() == "https"
        || url.host_str() == Some("localhost")
        || url.host_str() == Some("127.0.0.1")
        || url.host_str() == Some("::1")
        || url.scheme() == "file";

    ctx.register_global_property(
        js_string!("isSecureContext"),
        JsValue::from(is_secure),
        Attribute::CONFIGURABLE,
    )
    .expect("failed to register isSecureContext");
}

/// Register `addEventListener`, `removeEventListener` on the worker global
/// scope, plus `onmessage`/`onerror`/`onmessageerror` setter/getter.
fn register_worker_event_target(ctx: &mut Context, bridge: &HostBridge) {
    let b_add = bridge.clone();
    let add_fn = NativeFunction::from_copy_closure_with_captures(
        |_this, args, bridge, ctx| {
            let event_type = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            if let Some(cb) = args.get(1).and_then(JsValue::as_object) {
                bridge.worker_add_event_listener(event_type, cb.clone());
            }
            Ok(JsValue::undefined())
        },
        b_add,
    );
    ctx.register_global_builtin_callable(js_string!("addEventListener"), 2, add_fn)
        .expect("failed to register addEventListener");

    let b_remove = bridge.clone();
    let remove_fn = NativeFunction::from_copy_closure_with_captures(
        |_this, args, bridge, ctx| {
            let event_type = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            if let Some(cb) = args.get(1).and_then(JsValue::as_object) {
                bridge.worker_remove_event_listener(&event_type, &cb);
            }
            Ok(JsValue::undefined())
        },
        b_remove,
    );
    ctx.register_global_builtin_callable(js_string!("removeEventListener"), 2, remove_fn)
        .expect("failed to register removeEventListener");

    // dispatchEvent(event) — fires event on the worker global scope.
    let b_dispatch = bridge.clone();
    let dispatch_fn = NativeFunction::from_copy_closure_with_captures(
        |_this, args, bridge, ctx| {
            let event = args.first().cloned().unwrap_or(JsValue::undefined());
            let event_obj = event.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("dispatchEvent: argument is not an object")
            })?;
            let event_type = event_obj
                .get(js_string!("type"), ctx)?
                .to_string(ctx)?
                .to_std_string_escaped();

            let callbacks = bridge.worker_get_callbacks(&event_type);
            let global = ctx.global_object();
            for cb in callbacks {
                let _ = cb.call(
                    &JsValue::from(global.clone()),
                    std::slice::from_ref(&event),
                    ctx,
                );
            }
            // Return true (not cancelled) — simplified per spec.
            Ok(JsValue::from(true))
        },
        b_dispatch,
    );
    ctx.register_global_builtin_callable(js_string!("dispatchEvent"), 1, dispatch_fn)
        .expect("failed to register dispatchEvent");

    // onmessage / onerror / onmessageerror — IDL event handler attributes.
    register_event_handler_property(ctx, bridge, "onmessage", "message");
    register_event_handler_property(ctx, bridge, "onerror", "error");
    register_event_handler_property(ctx, bridge, "onmessageerror", "messageerror");

    // Additional WorkerGlobalScope event handler attributes (WHATWG HTML §10.3).
    register_event_handler_property(ctx, bridge, "onlanguagechange", "languagechange");
    register_event_handler_property(ctx, bridge, "onoffline", "offline");
    register_event_handler_property(ctx, bridge, "ononline", "online");
    register_event_handler_property(ctx, bridge, "onrejectionhandled", "rejectionhandled");
    register_event_handler_property(ctx, bridge, "onunhandledrejection", "unhandledrejection");
}

/// Register an IDL event handler attribute (e.g., `onmessage`) that acts as
/// a setter/getter backed by `WorkerBridgeState.event_handlers`.
fn register_event_handler_property(
    ctx: &mut Context,
    bridge: &HostBridge,
    prop_name: &str,
    event_type: &str,
) {
    let b_set = bridge.clone();
    let et_set = event_type.to_string();
    let setter = NativeFunction::from_copy_closure_with_captures(
        |_this, args, captures: &(HostBridge, String), _ctx| {
            let callback = args.first().and_then(JsValue::as_object);
            captures
                .0
                .worker_set_event_handler(captures.1.clone(), callback);
            Ok(JsValue::undefined())
        },
        (b_set, et_set),
    );

    let b_get = bridge.clone();
    let et_get = event_type.to_string();
    let getter = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, captures: &(HostBridge, String), _ctx| {
            let handler = captures.0.worker_get_event_handler(&captures.1);
            Ok(handler.map_or(JsValue::null(), JsValue::from))
        },
        (b_get, et_get),
    );

    let global = ctx.global_object();
    let desc = boa_engine::property::PropertyDescriptor::builder()
        .set(setter.to_js_function(ctx.realm()))
        .get(getter.to_js_function(ctx.realm()))
        .enumerable(true)
        .configurable(true)
        .build();
    global
        .define_property_or_throw(js_string!(prop_name), desc, ctx)
        .expect("failed to define event handler property");
}
