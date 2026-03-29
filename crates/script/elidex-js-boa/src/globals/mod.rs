//! Global object registration for the boa JS context.

pub mod canvas;
pub mod console;
pub mod cssom;
pub mod custom_elements;
pub mod document;
pub mod element;
pub(crate) mod element_form;
pub mod encoding;
pub mod event_constructors;
pub mod event_source;
pub mod events;
pub mod navigator;
pub mod fetch;
pub mod history;
pub(crate) mod iframe;
pub mod location;
pub mod observers;
pub mod timers;
pub mod storage;
pub mod url;
pub mod wasm;
pub mod websocket;
pub mod window;

use std::rc::Rc;

use boa_engine::{js_string, Context, JsNativeError, JsObject, JsResult, JsValue};
use elidex_ecs::Entity;
use elidex_net::FetchHandle;
use elidex_plugin::JsValue as ElidexJsValue;
use elidex_script_session::EventListeners;

use crate::bridge::HostBridge;
use crate::error_conv::dom_error_to_js_error;
use crate::value_conv;
use console::ConsoleOutput;
use timers::TimerQueueHandle;

/// Extract a required string argument from boa args.
///
/// Returns `TypeError` if the argument is missing, matching browser behavior
/// for required DOM method parameters.
pub(crate) fn require_js_string_arg(
    args: &[JsValue],
    index: usize,
    method: &str,
    ctx: &mut Context,
) -> JsResult<String> {
    match args.get(index) {
        Some(v) => Ok(v.to_string(ctx)?.to_std_string_escaped()),
        None => Err(JsNativeError::typ()
            .with_message(format!("{method}: argument {index} is required"))
            .into()),
    }
}

/// Invoke a DOM API handler by name via the registry and return the converted boa `JsValue`.
#[must_use = "DOM handler result must be returned to the JS caller"]
pub(crate) fn invoke_dom_handler(
    name: &str,
    entity: Entity,
    args: &[ElidexJsValue],
    bridge: &HostBridge,
) -> JsResult<JsValue> {
    let handler = bridge
        .dom_registry()
        .resolve(name)
        .ok_or_else(|| JsNativeError::typ().with_message(format!("Unknown DOM method: {name}")))?;
    bridge.with(|session, dom| {
        let result = handler
            .invoke(entity, args, session, dom)
            .map_err(dom_error_to_js_error)?;
        Ok(value_conv::to_boa(&result))
    })
}

/// Invoke a DOM API handler by name and resolve `ObjectRef` results to element wrappers.
///
/// Use this for handlers that return entity references (tree navigation, cloneNode, etc.).
#[must_use = "DOM handler result must be returned to the JS caller"]
pub(crate) fn invoke_dom_handler_ref(
    name: &str,
    entity: Entity,
    args: &[ElidexJsValue],
    bridge: &HostBridge,
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let handler = bridge
        .dom_registry()
        .resolve(name)
        .ok_or_else(|| JsNativeError::typ().with_message(format!("Unknown DOM method: {name}")))?;
    let result = bridge.with(|session, dom| {
        handler
            .invoke(entity, args, session, dom)
            .map_err(dom_error_to_js_error)
    })?;
    Ok(element::resolve_object_ref(&result, bridge, ctx))
}

/// Invoke a DOM API handler by name via the registry, ignoring the return value.
#[must_use = "DOM handler result must be returned to the JS caller"]
pub(crate) fn invoke_dom_handler_void(
    name: &str,
    entity: Entity,
    args: &[ElidexJsValue],
    bridge: &HostBridge,
) -> JsResult<JsValue> {
    let handler = bridge
        .dom_registry()
        .resolve(name)
        .ok_or_else(|| JsNativeError::typ().with_message(format!("Unknown DOM method: {name}")))?;
    bridge.with(|session, dom| {
        handler
            .invoke(entity, args, session, dom)
            .map_err(dom_error_to_js_error)?;
        Ok(JsValue::undefined())
    })
}

/// Convert a boa JS value to an elidex `JsValue` for entity-accepting handlers.
///
/// If the value is an element object, extracts its entity and creates an `ObjectRef`.
/// If it's a string, returns a `String` value. Otherwise, converts to string.
pub(crate) fn boa_arg_to_elidex(
    arg: &JsValue,
    bridge: &HostBridge,
    ctx: &mut Context,
) -> JsResult<ElidexJsValue> {
    if arg.is_null() || arg.is_undefined() {
        return Ok(ElidexJsValue::Null);
    }
    if let Ok(entity) = element::extract_entity(arg, ctx) {
        let ref_val = bridge.with(|session, _dom| {
            let ref_ = session
                .get_or_create_wrapper(entity, elidex_script_session::ComponentKind::Element);
            ElidexJsValue::ObjectRef(ref_.to_raw())
        });
        return Ok(ref_val);
    }
    // Fall back to string conversion.
    let s = arg.to_string(ctx)?.to_std_string_escaped();
    Ok(ElidexJsValue::String(s))
}

/// Convert a slice of boa args to elidex values (for variadic Node/String methods).
pub(crate) fn boa_args_to_elidex(
    args: &[JsValue],
    bridge: &HostBridge,
    ctx: &mut Context,
) -> JsResult<Vec<ElidexJsValue>> {
    args.iter()
        .map(|a| boa_arg_to_elidex(a, bridge, ctx))
        .collect()
}

/// Parsed listener options from the third argument of addEventListener/removeEventListener.
pub(crate) struct ListenerOptions {
    pub capture: bool,
    pub once: bool,
    pub passive: bool,
}

/// Extract listener options from the third argument of addEventListener/removeEventListener.
///
/// Handles both the boolean form (`el.addEventListener('click', fn, true)`)
/// and the options object form (`el.addEventListener('click', fn, {capture: true, once: true})`).
/// WHATWG DOM §2.6.
pub(crate) fn extract_listener_options(
    args: &[JsValue],
    ctx: &mut Context,
) -> JsResult<ListenerOptions> {
    match args.get(2) {
        Some(v) if v.is_object() => {
            let obj = v.as_object().unwrap();
            Ok(ListenerOptions {
                capture: obj.get(js_string!("capture"), ctx)?.to_boolean(),
                once: obj.get(js_string!("once"), ctx)?.to_boolean(),
                passive: obj.get(js_string!("passive"), ctx)?.to_boolean(),
            })
        }
        Some(v) => Ok(ListenerOptions {
            capture: v.to_boolean(),
            once: false,
            passive: false,
        }),
        None => Ok(ListenerOptions {
            capture: false,
            once: false,
            passive: false,
        }),
    }
}

/// Extract only the `capture` flag (for removeEventListener which ignores once/passive).
pub(crate) fn extract_capture(args: &[JsValue], ctx: &mut Context) -> JsResult<bool> {
    match args.get(2) {
        Some(v) if v.is_object() => {
            let obj = v.as_object().unwrap();
            Ok(obj.get(js_string!("capture"), ctx)?.to_boolean())
        }
        Some(v) => Ok(v.to_boolean()),
        None => Ok(false),
    }
}

/// Shared implementation of `addEventListener` for both element and document.
///
/// Checks for duplicate listeners (same type, capture, and JS function identity),
/// registers in ECS `EventListeners`, and stores the JS function in the bridge.
pub(crate) fn add_event_listener_for(
    entity: Entity,
    args: &[JsValue],
    bridge: &HostBridge,
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let event_type = require_js_string_arg(args, 0, "addEventListener", ctx)?;
    let listener_fn = args.get(1).and_then(JsValue::as_callable).ok_or_else(|| {
        JsNativeError::typ().with_message("addEventListener: argument 1 must be a function")
    })?;
    let opts = extract_listener_options(args, ctx)?;

    bridge.with(|_session, dom| {
        // WHATWG DOM §2.6 step 4: duplicate check by (type, callback, capture).
        // Per spec, once/passive are NOT part of the duplicate check.
        let is_duplicate =
            dom.world()
                .get::<&EventListeners>(entity)
                .ok()
                .is_some_and(|listeners| {
                    listeners.matching_all(&event_type).iter().any(|entry| {
                        entry.capture == opts.capture
                            && bridge.listener_matches(entry.id, &listener_fn)
                    })
                });

        if is_duplicate {
            return;
        }

        let has_listeners = dom.world().get::<&EventListeners>(entity).is_ok();
        let id = if has_listeners {
            dom.world_mut()
                .get::<&mut EventListeners>(entity)
                .unwrap()
                .add_with_options(&event_type, opts.capture, opts.once, opts.passive)
        } else {
            let mut listeners = EventListeners::new();
            let id = listeners.add_with_options(&event_type, opts.capture, opts.once, opts.passive);
            let _ = dom.world_mut().insert_one(entity, listeners);
            id
        };
        bridge.store_listener(id, listener_fn);
    });

    Ok(JsValue::undefined())
}

/// Shared implementation of `removeEventListener` for both element and document.
///
/// Finds the matching listener by (type, capture, JS function identity),
/// removes from ECS `EventListeners`, and removes from the bridge.
pub(crate) fn remove_event_listener_for(
    entity: Entity,
    args: &[JsValue],
    bridge: &HostBridge,
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let event_type = require_js_string_arg(args, 0, "removeEventListener", ctx)?;
    let listener_fn = args.get(1).and_then(JsValue::as_callable).ok_or_else(|| {
        JsNativeError::typ().with_message("removeEventListener: argument 1 must be a function")
    })?;
    let capture = extract_capture(args, ctx)?;

    bridge.with(|_session, dom| {
        let matching_id = dom
            .world()
            .get::<&EventListeners>(entity)
            .ok()
            .and_then(|listeners| {
                listeners
                    .matching_all(&event_type)
                    .into_iter()
                    .find(|entry| {
                        entry.capture == capture && bridge.listener_matches(entry.id, &listener_fn)
                    })
                    .map(|entry| entry.id)
            });

        let Some(id) = matching_id else { return };

        if let Ok(mut listeners) = dom.world_mut().get::<&mut EventListeners>(entity) {
            listeners.remove(id);
        }
        bridge.remove_listener(id);
    });

    Ok(JsValue::undefined())
}

/// Shared implementation of `dispatchEvent` for element, document, and window.
///
/// WHATWG DOM §2.6: validates the event object, checks dispatch/initialized flags,
/// then dispatches synchronously through the propagation path.
/// Returns `!defaultPrevented` as a boolean.
pub(crate) fn dispatch_event_for(
    entity: Entity,
    args: &[JsValue],
    bridge: &HostBridge,
    ctx: &mut Context,
) -> JsResult<JsValue> {
    use crate::globals::event_constructors::*;

    let event_obj = args.first().ok_or_else(|| {
        JsNativeError::typ().with_message("dispatchEvent: argument 1 is required")
    })?;
    let obj = event_obj.as_object().ok_or_else(|| {
        JsNativeError::typ().with_message("dispatchEvent: argument must be an Event object")
    })?;

    // Verify this is an elidex Event object.
    let marker = obj.get(js_string!(EVENT_MARKER_KEY), ctx)?;
    if !marker.to_boolean() {
        return Err(JsNativeError::typ()
            .with_message("dispatchEvent: argument is not an Event object")
            .into());
    }

    // WHATWG DOM §2.10 step 1: empty type check.
    let event_type = obj
        .get(js_string!(EVENT_TYPE_KEY), ctx)?
        .to_string(ctx)?
        .to_std_string_escaped();
    if event_type.is_empty() {
        return Err(JsNativeError::eval()
            .with_message("InvalidStateError: event type must not be empty")
            .into());
    }

    // WHATWG DOM §2.6 step 1: dispatch flag check.
    let dispatching = obj
        .get(js_string!(EVENT_DISPATCHING_KEY), ctx)?
        .to_boolean();
    if dispatching {
        return Err(JsNativeError::eval()
            .with_message("InvalidStateError: event is already being dispatched")
            .into());
    }

    // WHATWG DOM §2.6 step 1: initialized flag check.
    let initialized = obj
        .get(js_string!(EVENT_INITIALIZED_KEY), ctx)?
        .to_boolean();
    if !initialized {
        return Err(JsNativeError::eval()
            .with_message("InvalidStateError: event has not been initialized")
            .into());
    }

    // Extract event metadata.
    let bubbles = obj
        .get(js_string!(EVENT_BUBBLES_KEY), ctx)?
        .to_boolean();
    let cancelable = obj
        .get(js_string!(EVENT_CANCELABLE_KEY), ctx)?
        .to_boolean();
    let composed = obj
        .get(js_string!(EVENT_COMPOSED_KEY), ctx)?
        .to_boolean();

    // Set dispatch flag.
    let _ = obj.set(
        js_string!(EVENT_DISPATCHING_KEY),
        JsValue::from(true),
        false,
        ctx,
    );

    // Create untrusted DispatchEvent.
    let mut dispatch_event =
        elidex_script_session::DispatchEvent::new_untrusted(&event_type, entity);
    dispatch_event.bubbles = bubbles;
    dispatch_event.cancelable = cancelable;
    dispatch_event.composed = composed;

    // Synchronous dispatch: enqueue as pending script dispatch for the runtime
    // to process immediately after this NativeFunction returns. The runtime's
    // eval loop calls drain_queued_events which will pick this up.
    //
    // For synchronous return value, we use the bridge's pending_script_dispatch
    // mechanism: store the DispatchEvent, let runtime process it, read the result.
    bridge.set_pending_script_dispatch(dispatch_event);

    // The dispatch happens synchronously when the runtime drains after eval.
    // For now, return true (not prevented) as a placeholder.
    // TODO: Wire synchronous dispatch through runtime for correct return value.
    let prevented = false;

    // WHATWG DOM §2.10 steps 10-14: post-dispatch cleanup.
    let _ = obj.set(
        js_string!(EVENT_DISPATCHING_KEY),
        JsValue::from(false),
        false,
        ctx,
    );
    let _ = obj.set(js_string!("target"), JsValue::null(), false, ctx);
    let _ = obj.set(js_string!("currentTarget"), JsValue::null(), false, ctx);

    // Return !defaultPrevented.
    Ok(JsValue::from(!prevented))
}

/// Extract a numeric connection ID from a hidden property on `this`.
///
/// Shared by `WebSocket` (`__elidex_ws_id__`) and `EventSource` (`__elidex_sse_id__`).
pub(crate) fn extract_connection_id(
    this: &JsValue,
    key: &str,
    type_name: &str,
    ctx: &mut Context,
) -> JsResult<u64> {
    let obj = this.as_object().ok_or_else(|| {
        JsNativeError::typ().with_message(format!("{type_name}: not a {type_name} object"))
    })?;
    let id_val = obj.get(js_string!(key), ctx)?;
    let id = id_val.as_number().ok_or_else(|| {
        JsNativeError::typ().with_message(format!("{type_name}: missing connection ID"))
    })?;
    Ok(id as u64)
}

/// Define a hidden, non-writable connection ID property on a JS object.
///
/// Shared by `WebSocket` and `EventSource` to store the internal connection ID
/// as a tamper-proof property.
pub(crate) fn define_connection_id(obj: &JsObject, key: &str, id: u64, ctx: &mut Context) {
    let _ = obj.define_property_or_throw(
        js_string!(key),
        boa_engine::property::PropertyDescriptor::builder()
            .value(JsValue::from(id as f64))
            .writable(false)
            .enumerable(false)
            .configurable(false)
            .build(),
        ctx,
    );
}

/// Set readyState constants on a JS object.
///
/// Shared by `WebSocket` and `EventSource` for setting CONNECTING/OPEN/CLOSING/CLOSED
/// constants on both the constructor and instance objects.
pub(crate) fn set_readystate_constants(obj: &JsObject, names: &[(&str, i32)], ctx: &mut Context) {
    for &(name, value) in names {
        let _ = obj.set(js_string!(name), JsValue::from(value), false, ctx);
    }
}

/// Register all elidex globals on the boa context.
pub fn register_all_globals(
    ctx: &mut Context,
    bridge: &HostBridge,
    console_output: &ConsoleOutput,
    timer_queue: &TimerQueueHandle,
    fetch_handle: Option<Rc<FetchHandle>>,
) {
    console::register_console(ctx, console_output);
    document::register_document(ctx, bridge);
    window::register_window(ctx, bridge);
    timers::register_timers(ctx, timer_queue);
    fetch::register_fetch(ctx, fetch_handle.clone());
    wasm::register_wasm(ctx, bridge);
    observers::register_observers(ctx, bridge);
    custom_elements::register_custom_elements_global(ctx, bridge);
    websocket::register_websocket(ctx, bridge);
    event_source::register_event_source(ctx, bridge, fetch_handle);
    event_constructors::register_event_constructors(ctx, bridge);
    navigator::register_navigator(ctx, bridge);
    url::register_url_constructors(ctx, bridge);
    encoding::register_encoding(ctx, bridge);
    storage::register_storage(ctx, bridge);
    // Register location and history as global properties.
    let location_obj = location::register_location(ctx, bridge);
    let history_obj = history::register_history(ctx, bridge);
    let global = ctx.global_object();
    global
        .set(js_string!("location"), location_obj, false, ctx)
        .expect("failed to register location");
    global
        .set(js_string!("history"), history_obj, false, ctx)
        .expect("failed to register history");
}
