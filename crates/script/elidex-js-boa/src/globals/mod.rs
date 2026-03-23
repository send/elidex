//! Global object registration for the boa JS context.

pub mod canvas;
pub mod console;
pub mod cssom;
pub mod custom_elements;
pub mod document;
pub mod element;
pub(crate) mod element_form;
pub mod events;
pub mod fetch;
pub mod history;
pub mod location;
pub mod observers;
pub mod timers;
pub mod wasm;
pub mod window;

use std::rc::Rc;

use boa_engine::{js_string, Context, JsNativeError, JsResult, JsValue};
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

/// Extract the `capture` flag from the third argument of addEventListener/removeEventListener.
///
/// Handles both the boolean form (`el.addEventListener('click', fn, true)`)
/// and the options object form (`el.addEventListener('click', fn, {capture: true})`).
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
    let capture = extract_capture(args, ctx)?;

    bridge.with(|_session, dom| {
        let is_duplicate =
            dom.world()
                .get::<&EventListeners>(entity)
                .ok()
                .is_some_and(|listeners| {
                    listeners.matching_all(&event_type).iter().any(|entry| {
                        entry.capture == capture && bridge.listener_matches(entry.id, &listener_fn)
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
                .add(&event_type, capture)
        } else {
            let mut listeners = EventListeners::new();
            let id = listeners.add(&event_type, capture);
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
    fetch::register_fetch(ctx, fetch_handle);
    wasm::register_wasm(ctx, bridge);
    observers::register_observers(ctx, bridge);
    custom_elements::register_custom_elements_global(ctx, bridge);
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
