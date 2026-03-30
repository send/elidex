//! Global object registration for the boa JS context.

pub mod abort;
pub mod blob;
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
pub mod fetch;
pub mod form_data;
pub mod history;
pub(crate) mod iframe;
pub mod indexeddb;
pub mod location;
pub mod navigator;
pub mod observers;
pub mod storage;
pub mod timers;
pub mod url;
pub mod wasm;
pub mod websocket;
pub mod window;
pub mod worker_constructor;
pub mod worker_scope;

use std::rc::Rc;

use boa_engine::{js_string, Context, JsNativeError, JsObject, JsResult, JsValue, NativeFunction};
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
    /// An `AbortSignal` object. If present and already aborted, the listener must not be added.
    /// If not yet aborted, an abort callback is registered to auto-remove the listener.
    pub signal: Option<JsObject>,
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
            let signal_val = obj.get(js_string!("signal"), ctx)?;
            let signal = if abort::is_abort_signal(&signal_val, ctx) {
                signal_val.as_object()
            } else {
                None
            };
            Ok(ListenerOptions {
                capture: obj.get(js_string!("capture"), ctx)?.to_boolean(),
                once: obj.get(js_string!("once"), ctx)?.to_boolean(),
                passive: obj.get(js_string!("passive"), ctx)?.to_boolean(),
                signal,
            })
        }
        Some(v) => Ok(ListenerOptions {
            capture: v.to_boolean(),
            once: false,
            passive: false,
            signal: None,
        }),
        None => Ok(ListenerOptions {
            capture: false,
            once: false,
            passive: false,
            signal: None,
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
#[allow(clippy::too_many_lines)]
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

    // WHATWG DOM §2.6 step 3: if signal is already aborted, do not add the listener.
    if let Some(ref signal) = opts.signal {
        if abort::is_signal_aborted(signal, ctx) {
            return Ok(JsValue::undefined());
        }
    }

    // Clone listener_fn for potential signal abort callback (before bridge.with consumes it).
    let listener_fn_for_signal = if opts.signal.is_some() {
        Some(listener_fn.clone())
    } else {
        None
    };

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

    // WHATWG DOM §2.6 step 5: if signal is present, register an abort callback
    // that removes the listener when the signal fires.
    if let Some(signal) = opts.signal {
        let listeners_key = js_string!("__abort_listeners__");
        let existing = signal.get(listeners_key.clone(), ctx)?;

        // Build a wrapper object that holds the removal context. We store the
        // entity, event type, and listener function reference as properties so
        // the abort callback can use them. This avoids Trace requirements on
        // non-GC types like Entity and String.
        let mut removal_ctx_init = boa_engine::object::ObjectInitializer::new(ctx);
        removal_ctx_init.property(
            js_string!("__entity__"),
            JsValue::from(entity.to_bits().get() as f64),
            boa_engine::property::Attribute::empty(),
        );
        removal_ctx_init.property(
            js_string!("__event_type__"),
            JsValue::from(js_string!(event_type.as_str())),
            boa_engine::property::Attribute::empty(),
        );
        removal_ctx_init.property(
            js_string!("__listener__"),
            JsValue::from(listener_fn_for_signal.unwrap()),
            boa_engine::property::Attribute::empty(),
        );
        removal_ctx_init.property(
            js_string!("__capture__"),
            JsValue::from(opts.capture),
            boa_engine::property::Attribute::empty(),
        );
        let removal_ctx = removal_ctx_init.build();

        let b = bridge.clone();
        let remove_callback = NativeFunction::from_copy_closure_with_captures(
            move |_this, _args, (bridge, removal_obj), ctx| {
                let entity_bits = removal_obj
                    .get(js_string!("__entity__"), ctx)?
                    .to_number(ctx)? as u64;
                let Some(entity) = Entity::from_bits(entity_bits) else {
                    return Ok(JsValue::undefined());
                };
                let event_type = removal_obj
                    .get(js_string!("__event_type__"), ctx)?
                    .to_string(ctx)?
                    .to_std_string_escaped();
                let listener_val = removal_obj.get(js_string!("__listener__"), ctx)?;
                let Some(listener_fn) = listener_val.as_callable() else {
                    return Ok(JsValue::undefined());
                };
                let capture = removal_obj
                    .get(js_string!("__capture__"), ctx)?
                    .to_boolean();

                bridge.with(|_session, dom| {
                    let matching_id =
                        dom.world()
                            .get::<&EventListeners>(entity)
                            .ok()
                            .and_then(|listeners| {
                                listeners
                                    .matching_all(&event_type)
                                    .into_iter()
                                    .find(|entry| {
                                        entry.capture == capture
                                            && bridge.listener_matches(entry.id, &listener_fn)
                                    })
                                    .map(|entry| entry.id)
                            });
                    if let Some(id) = matching_id {
                        if let Ok(mut listeners) =
                            dom.world_mut().get::<&mut EventListeners>(entity)
                        {
                            listeners.remove(id);
                        }
                        bridge.remove_listener(id);
                    }
                });
                Ok(JsValue::undefined())
            },
            (b, removal_ctx),
        )
        .to_js_function(ctx.realm());

        if existing.is_undefined() || existing.is_null() {
            let arr = boa_engine::object::builtins::JsArray::new(ctx);
            let _ = arr.push(JsValue::from(remove_callback), ctx);
            let _ = signal.set(listeners_key, JsValue::from(arr), false, ctx);
        } else if let Some(arr) = existing.as_object() {
            let len = arr
                .get(js_string!("length"), ctx)?
                .to_number(ctx)
                .unwrap_or(0.0) as u32;
            let _ = arr.set(len, JsValue::from(remove_callback), false, ctx);
        }
    }

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
#[allow(clippy::too_many_lines)]
pub(crate) fn dispatch_event_for(
    entity: Entity,
    args: &[JsValue],
    bridge: &HostBridge,
    ctx: &mut Context,
) -> JsResult<JsValue> {
    use crate::globals::event_constructors::{
        EVENT_BUBBLES_KEY, EVENT_CANCELABLE_KEY, EVENT_COMPOSED_KEY, EVENT_CURRENT_TARGET_SLOT,
        EVENT_DISPATCHING_KEY, EVENT_INITIALIZED_KEY, EVENT_MARKER_KEY, EVENT_PHASE_SLOT,
        EVENT_TARGET_SLOT, EVENT_TYPE_KEY,
    };

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
    let bubbles = obj.get(js_string!(EVENT_BUBBLES_KEY), ctx)?.to_boolean();
    let cancelable = obj.get(js_string!(EVENT_CANCELABLE_KEY), ctx)?.to_boolean();
    let composed = obj.get(js_string!(EVENT_COMPOSED_KEY), ctx)?.to_boolean();

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

    // WHATWG DOM §2.6: pass the SAME JS event object to all listeners.
    // Before dispatch, we install new stopPropagation/stopImmediatePropagation
    // methods and a preventDefault method on the user's event object that write
    // to Rc<Cell> flags we own. This allows us to read back flag state between
    // listener invocations while reusing the same JS object identity.
    let user_event_obj = obj.clone();

    // Finding 5: preserve any pre-dispatch defaultPrevented state set before
    // dispatchEvent() was called (e.g. event.preventDefault() called before dispatch).
    let pre_dispatch_pd = obj
        .get(js_string!("defaultPrevented"), ctx)
        .ok()
        .is_some_and(|v| v.to_boolean());

    let pd_flag = Rc::new(std::cell::Cell::new(pre_dispatch_pd));
    let stop_prop_flag = Rc::new(std::cell::Cell::new(false));
    let stop_imm_flag = Rc::new(std::cell::Cell::new(false));

    // Install dispatch-owned flag methods on the user's event object.
    // These override the constructor-created methods for the duration of dispatch.
    {
        use crate::globals::events::SharedFlag;

        let pd_shared = SharedFlag(Rc::clone(&pd_flag));
        let _ = user_event_obj.set(
            js_string!("preventDefault"),
            NativeFunction::from_copy_closure_with_captures(
                |_this, _args, (f, cancel), _ctx| {
                    if *cancel {
                        f.0.set(true);
                    }
                    Ok(JsValue::undefined())
                },
                (pd_shared, cancelable),
            )
            .to_js_function(ctx.realm()),
            false,
            ctx,
        );

        let stop_prop_shared = SharedFlag(Rc::clone(&stop_prop_flag));
        let _ = user_event_obj.set(
            js_string!("stopPropagation"),
            NativeFunction::from_copy_closure_with_captures(
                |_this, _args, f, _ctx| {
                    f.0.set(true);
                    Ok(JsValue::undefined())
                },
                stop_prop_shared,
            )
            .to_js_function(ctx.realm()),
            false,
            ctx,
        );

        let imm_shared_prop = SharedFlag(Rc::clone(&stop_prop_flag));
        let imm_shared_imm = SharedFlag(Rc::clone(&stop_imm_flag));
        let _ = user_event_obj.set(
            js_string!("stopImmediatePropagation"),
            NativeFunction::from_copy_closure_with_captures(
                |_this, _args, (sp, si), _ctx| {
                    sp.0.set(true);
                    si.0.set(true);
                    Ok(JsValue::undefined())
                },
                (imm_shared_prop, imm_shared_imm),
            )
            .to_js_function(ctx.realm()),
            false,
            ctx,
        );
    }

    // Pre-compute the dispatch plan and composed path via bridge.with(),
    // releasing the DOM borrow before any listener callbacks execute.
    // This avoids the UB of with_dom_ref (creating &EcsDom) while bridge.with()
    // inside callbacks creates &mut EcsDom.
    let (plan, composed_path) = bridge.with(|_session, dom| {
        let plan = elidex_script_session::build_dispatch_plan(dom, &dispatch_event);
        let path =
            elidex_script_session::build_propagation_path(dom, dispatch_event.target, composed);
        (plan, path)
    });

    dispatch_event.composed_path = composed_path;
    dispatch_event.dispatch_flag = true;
    dispatch_event.flags.default_prevented = pre_dispatch_pd;

    let saved_target = dispatch_event.target;

    // Helper closure: invoke a list of listeners on an entity.
    let mut invoke_phase_listeners =
        |ids: &[elidex_script_session::ListenerId],
         phase_entity: elidex_ecs::Entity,
         ev: &mut elidex_script_session::DispatchEvent| {
            for &listener_id in ids {
                if ev.flags.immediate_propagation_stopped {
                    break;
                }

                // Look up listener metadata for once/passive options.
                let (is_once, is_passive) = bridge.with(|_session, dom| {
                    dom.world()
                        .get::<&EventListeners>(phase_entity)
                        .ok()
                        .map_or((false, false), |listeners| {
                            listeners
                                .find_entry(listener_id)
                                .map_or((false, false), |entry| (entry.once, entry.passive))
                        })
                });

                let Some(js_func) = bridge.get_listener(listener_id) else {
                    continue;
                };

                // WHATWG DOM §2.10 step 15: remove once listeners BEFORE invoking.
                if is_once {
                    bridge.with(|_session, dom| {
                        if let Ok(mut listeners) =
                            dom.world_mut().get::<&mut EventListeners>(phase_entity)
                        {
                            listeners.remove(listener_id);
                        }
                    });
                    bridge.remove_listener(listener_id);
                }

                // Update eventPhase, target, currentTarget hidden slots.
                let _ = user_event_obj.set(
                    js_string!(EVENT_PHASE_SLOT),
                    JsValue::from(i32::from(ev.phase as u8)),
                    false,
                    ctx,
                );
                let target_val = bridge.with(|session, _dom| {
                    let obj_ref = session.get_or_create_wrapper(
                        ev.target,
                        elidex_script_session::ComponentKind::Element,
                    );
                    crate::globals::element::create_element_wrapper(
                        ev.target, bridge, obj_ref, ctx, false,
                    )
                });
                let _ = user_event_obj.set(js_string!(EVENT_TARGET_SLOT), target_val, false, ctx);
                if let Some(ct) = ev.current_target {
                    let ct_val = bridge.with(|session, _dom| {
                        let obj_ref = session.get_or_create_wrapper(
                            ct,
                            elidex_script_session::ComponentKind::Element,
                        );
                        crate::globals::element::create_element_wrapper(
                            ct, bridge, obj_ref, ctx, false,
                        )
                    });
                    let _ = user_event_obj.set(
                        js_string!(EVENT_CURRENT_TARGET_SLOT),
                        ct_val,
                        false,
                        ctx,
                    );
                } else {
                    let _ = user_event_obj.set(
                        js_string!(EVENT_CURRENT_TARGET_SLOT),
                        JsValue::null(),
                        false,
                        ctx,
                    );
                }

                // Sync dispatch flags into our Rc<Cell>s before each listener.
                pd_flag.set(ev.flags.default_prevented);
                stop_prop_flag.set(ev.flags.propagation_stopped);
                stop_imm_flag.set(ev.flags.immediate_propagation_stopped);

                // WHATWG DOM §2.6: passive listeners cannot call preventDefault().
                let saved_cancelable = ev.cancelable;
                if is_passive {
                    ev.cancelable = false;
                    let pd_shared = crate::globals::events::SharedFlag(Rc::clone(&pd_flag));
                    let _ = user_event_obj.set(
                        js_string!("preventDefault"),
                        NativeFunction::from_copy_closure_with_captures(
                            |_this, _args, (f, cancel), _ctx| {
                                if *cancel {
                                    f.0.set(true);
                                }
                                Ok(JsValue::undefined())
                            },
                            (pd_shared, false),
                        )
                        .to_js_function(ctx.realm()),
                        false,
                        ctx,
                    );
                }

                let event_val: JsValue = user_event_obj.clone().into();
                if let Err(err) = js_func.call(&JsValue::undefined(), &[event_val], ctx) {
                    eprintln!("[JS dispatchEvent Error] {err}");
                }

                // Microtask checkpoint (HTML §8.1.7.3).
                let _ = ctx.run_jobs();

                // Restore cancelable after passive listener override.
                if is_passive {
                    ev.cancelable = saved_cancelable;
                    let pd_shared = crate::globals::events::SharedFlag(Rc::clone(&pd_flag));
                    let _ = user_event_obj.set(
                        js_string!("preventDefault"),
                        NativeFunction::from_copy_closure_with_captures(
                            |_this, _args, (f, cancel), _ctx| {
                                if *cancel {
                                    f.0.set(true);
                                }
                                Ok(JsValue::undefined())
                            },
                            (pd_shared, saved_cancelable),
                        )
                        .to_js_function(ctx.realm()),
                        false,
                        ctx,
                    );
                }

                // Read back flag state from our owned Rc<Cell>s.
                ev.flags.default_prevented = pd_flag.get();
                ev.flags.propagation_stopped = stop_prop_flag.get();
                ev.flags.immediate_propagation_stopped = stop_imm_flag.get();
            }
        };

    // Phase 1: Capture (root → target, exclusive).
    dispatch_event.phase = elidex_plugin::EventPhase::Capturing;
    for (phase_entity, ids) in &plan.capture {
        if dispatch_event.flags.propagation_stopped
            || dispatch_event.flags.immediate_propagation_stopped
        {
            break;
        }
        // Per-listener retarget via bridge.with().
        bridge.with(|_session, dom| {
            elidex_script_session::apply_retarget(
                &mut dispatch_event,
                *phase_entity,
                saved_target,
                dom,
            );
        });
        dispatch_event.current_target = Some(*phase_entity);
        invoke_phase_listeners(ids, *phase_entity, &mut dispatch_event);
    }

    // Phase 2: At-target.
    if !dispatch_event.flags.propagation_stopped
        && !dispatch_event.flags.immediate_propagation_stopped
    {
        if let Some((target, ref ids)) = plan.at_target {
            dispatch_event.phase = elidex_plugin::EventPhase::AtTarget;
            dispatch_event.target = saved_target;
            dispatch_event.original_target = None;
            dispatch_event.current_target = Some(target);
            invoke_phase_listeners(ids, target, &mut dispatch_event);
        }
    }

    // Phase 3: Bubble (target → root, exclusive, reversed).
    if dispatch_event.bubbles
        && !dispatch_event.flags.propagation_stopped
        && !dispatch_event.flags.immediate_propagation_stopped
    {
        dispatch_event.phase = elidex_plugin::EventPhase::Bubbling;
        for (phase_entity, ids) in &plan.bubble {
            if dispatch_event.flags.propagation_stopped
                || dispatch_event.flags.immediate_propagation_stopped
            {
                break;
            }
            bridge.with(|_session, dom| {
                elidex_script_session::apply_retarget(
                    &mut dispatch_event,
                    *phase_entity,
                    saved_target,
                    dom,
                );
            });
            dispatch_event.current_target = Some(*phase_entity);
            invoke_phase_listeners(ids, *phase_entity, &mut dispatch_event);
        }
    }

    // Post-dispatch cleanup.
    dispatch_event.phase = elidex_plugin::EventPhase::None;
    dispatch_event.current_target = None;
    dispatch_event.target = saved_target;
    dispatch_event.original_target = None;
    dispatch_event.dispatch_flag = false;

    let prevented = dispatch_event.flags.default_prevented;

    // WHATWG DOM §2.10 steps 10-14: post-dispatch cleanup.
    let _ = obj.set(
        js_string!(EVENT_DISPATCHING_KEY),
        JsValue::from(false),
        false,
        ctx,
    );
    let _ = obj.set(
        js_string!(EVENT_PHASE_SLOT),
        JsValue::from(0_i32),
        false,
        ctx,
    );
    let _ = obj.set(js_string!(EVENT_TARGET_SLOT), JsValue::null(), false, ctx);
    let _ = obj.set(
        js_string!(EVENT_CURRENT_TARGET_SLOT),
        JsValue::null(),
        false,
        ctx,
    );

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
    fetch::constructors::register_fetch_constructors(ctx);
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
    abort::register_abort_controller(ctx, bridge);
    blob::register_blob_file(ctx);
    form_data::register_form_data(ctx, bridge);
    worker_constructor::register_worker_constructor(ctx, bridge);
    indexeddb::register_indexeddb(ctx, bridge);
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
