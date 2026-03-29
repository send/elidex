//! `AbortController` / `AbortSignal` (WHATWG DOM §3.2).

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsNativeError, JsObject, JsResult, JsValue, NativeFunction};

use crate::bridge::HostBridge;

/// Hidden property key marking an object as an AbortSignal.
const SIGNAL_KEY: &str = "__elidex_abort_signal__";
/// Hidden property key for the aborted state.
const ABORTED_KEY: &str = "__elidex_abort_aborted__";
/// Hidden property key for the abort reason.
const REASON_KEY: &str = "__elidex_abort_reason__";
/// Hidden property key linking a controller to its signal.
const CONTROLLER_SIGNAL_KEY: &str = "__elidex_controller_signal__";

/// Register `AbortController` and `AbortSignal` globals.
pub fn register_abort_controller(ctx: &mut Context, bridge: &HostBridge) {
    register_abort_signal_statics(ctx, bridge);
    register_controller_constructor(ctx, bridge);
}

/// Create an `AbortSignal` JS object.
fn create_abort_signal(ctx: &mut Context, bridge: &HostBridge) -> JsObject {
    let b = bridge.clone();
    let mut init = ObjectInitializer::new(ctx);

    init.property(js_string!(SIGNAL_KEY), JsValue::from(true), Attribute::empty());
    init.property(js_string!(ABORTED_KEY), JsValue::from(false), Attribute::empty());
    init.property(
        js_string!(REASON_KEY),
        JsValue::undefined(),
        Attribute::empty(),
    );
    init.property(
        js_string!("onabort"),
        JsValue::null(),
        Attribute::WRITABLE | Attribute::CONFIGURABLE,
    );

    // aborted — getter.
    let realm = init.context().realm().clone();
    let aborted_getter = NativeFunction::from_copy_closure(|this, _args, ctx| {
        let obj = this.as_object().ok_or_else(|| {
            JsNativeError::typ().with_message("AbortSignal: this is not an object")
        })?;
        obj.get(js_string!(ABORTED_KEY), ctx)
    })
    .to_js_function(&realm);
    init.accessor(
        js_string!("aborted"),
        Some(aborted_getter),
        None,
        Attribute::CONFIGURABLE,
    );

    // reason — getter.
    let reason_getter = NativeFunction::from_copy_closure(|this, _args, ctx| {
        let obj = this.as_object().ok_or_else(|| {
            JsNativeError::typ().with_message("AbortSignal: this is not an object")
        })?;
        obj.get(js_string!(REASON_KEY), ctx)
    })
    .to_js_function(&realm);
    init.accessor(
        js_string!("reason"),
        Some(reason_getter),
        None,
        Attribute::CONFIGURABLE,
    );

    // throwIfAborted() — WHATWG DOM §3.2.
    init.function(
        NativeFunction::from_copy_closure(|this, _args, ctx| {
            let obj = this.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("AbortSignal: this is not an object")
            })?;
            let aborted = obj.get(js_string!(ABORTED_KEY), ctx)?.to_boolean();
            if aborted {
                let reason = obj.get(js_string!(REASON_KEY), ctx)?;
                if reason.is_undefined() {
                    return Err(JsNativeError::eval()
                        .with_message("AbortError: The operation was aborted")
                        .into());
                }
                return Err(JsNativeError::typ()
                    .with_message(
                        reason
                            .to_string(ctx)
                            .map_or("The operation was aborted".into(), |s| {
                                s.to_std_string_escaped()
                            }),
                    )
                    .into());
            }
            Ok(JsValue::undefined())
        }),
        js_string!("throwIfAborted"),
        0,
    );

    // addEventListener / removeEventListener on signal (EventTarget).
    let b2 = b.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = bridge.document_entity();
                // Store signal listeners on the signal object itself via a list.
                let event_type = crate::globals::require_js_string_arg(args, 0, "addEventListener", ctx)?;
                if event_type != "abort" {
                    return Ok(JsValue::undefined());
                }
                let listener = args.get(1).cloned().unwrap_or(JsValue::undefined());
                let obj = this.as_object().ok_or_else(|| {
                    JsNativeError::typ().with_message("AbortSignal: this is not an object")
                })?;
                // Store listeners in a hidden array.
                let listeners_key = js_string!("__abort_listeners__");
                let existing = obj.get(listeners_key.clone(), ctx)?;
                if existing.is_undefined() || existing.is_null() {
                    let arr = boa_engine::object::builtins::JsArray::new(ctx);
                    arr.push(listener, ctx)?;
                    obj.set(listeners_key, JsValue::from(arr), false, ctx)?;
                } else {
                    let arr = existing.as_object().ok_or_else(|| {
                        JsNativeError::typ().with_message("internal error")
                    })?;
                    let len = arr.get(js_string!("length"), ctx)?.to_number(ctx)? as u32;
                    arr.set(len, listener, false, ctx)?;
                }
                let _ = entity;
                Ok(JsValue::undefined())
            },
            b2,
        ),
        js_string!("addEventListener"),
        2,
    );

    init.function(
        NativeFunction::from_fn_ptr(|_this, _args, _ctx| Ok(JsValue::undefined())),
        js_string!("removeEventListener"),
        2,
    );

    init.build()
}

/// Fire the "abort" event on a signal object.
fn fire_abort_on_signal(signal: &JsObject, reason: &JsValue, ctx: &mut Context) -> JsResult<()> {
    // Set aborted = true, reason = value.
    signal.set(js_string!(ABORTED_KEY), JsValue::from(true), false, ctx)?;
    signal.set(js_string!(REASON_KEY), reason.clone(), false, ctx)?;

    // Call onabort if set.
    let onabort = signal.get(js_string!("onabort"), ctx)?;
    if let Some(func) = onabort.as_callable() {
        let _ = func.call(&JsValue::from(signal.clone()), &[], ctx);
    }

    // Call registered abort listeners.
    let listeners_key = js_string!("__abort_listeners__");
    let listeners = signal.get(listeners_key, ctx)?;
    if let Some(arr) = listeners.as_object() {
        let len = arr
            .get(js_string!("length"), ctx)?
            .to_number(ctx)
            .unwrap_or(0.0) as u32;
        for i in 0..len {
            let listener = arr.get(i, ctx)?;
            if let Some(func) = listener.as_callable() {
                let _ = func.call(&JsValue::from(signal.clone()), &[], ctx);
            }
        }
    }

    Ok(())
}

/// Register `AbortController` constructor.
fn register_controller_constructor(ctx: &mut Context, bridge: &HostBridge) {
    let b = bridge.clone();
    let constructor = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, ctx| {
            let signal = create_abort_signal(ctx, bridge);

            let signal_clone = signal.clone();
            let mut init = ObjectInitializer::new(ctx);

            // signal — read-only property.
            init.property(
                js_string!("signal"),
                JsValue::from(signal_clone.clone()),
                Attribute::READONLY | Attribute::CONFIGURABLE,
            );
            init.property(
                js_string!(CONTROLLER_SIGNAL_KEY),
                JsValue::from(signal_clone),
                Attribute::empty(),
            );

            // abort(reason?) — abort the signal.
            init.function(
                NativeFunction::from_copy_closure(|this, args, ctx| {
                    let obj = this.as_object().ok_or_else(|| {
                        JsNativeError::typ()
                            .with_message("AbortController: this is not an object")
                    })?;
                    let signal_val = obj.get(js_string!(CONTROLLER_SIGNAL_KEY), ctx)?;
                    let signal = signal_val.as_object().ok_or_else(|| {
                        JsNativeError::typ().with_message("AbortController: no signal")
                    })?;

                    // Already aborted — no-op.
                    let already = signal.get(js_string!(ABORTED_KEY), ctx)?.to_boolean();
                    if already {
                        return Ok(JsValue::undefined());
                    }

                    let reason = args.first().cloned().unwrap_or_else(|| {
                        JsValue::from(js_string!("AbortError: The operation was aborted"))
                    });

                    fire_abort_on_signal(&signal, &reason, ctx)?;
                    Ok(JsValue::undefined())
                }),
                js_string!("abort"),
                0,
            );

            Ok(JsValue::from(init.build()))
        },
        b,
    );

    ctx.register_global_builtin_callable(js_string!("AbortController"), 0, constructor)
        .expect("failed to register AbortController");
}

/// Register `AbortSignal` static methods.
fn register_abort_signal_statics(ctx: &mut Context, bridge: &HostBridge) {
    let b = bridge.clone();
    let mut init = ObjectInitializer::new(ctx);

    // AbortSignal.abort(reason?) — returns an already-aborted signal.
    let b2 = b.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, bridge, ctx| {
                let signal = create_abort_signal(ctx, bridge);
                let reason = args.first().cloned().unwrap_or_else(|| {
                    JsValue::from(js_string!("AbortError: The operation was aborted"))
                });
                signal.set(js_string!(ABORTED_KEY), JsValue::from(true), false, ctx)?;
                signal.set(js_string!(REASON_KEY), reason, false, ctx)?;
                Ok(JsValue::from(signal))
            },
            b2,
        ),
        js_string!("abort"),
        0,
    );

    // AbortSignal.timeout(ms) — returns a signal that aborts after ms milliseconds.
    let b3 = b;
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, bridge, ctx| {
                let ms = args
                    .first()
                    .and_then(JsValue::as_number)
                    .ok_or_else(|| {
                        JsNativeError::typ()
                            .with_message("AbortSignal.timeout: argument must be a number")
                    })?;

                let signal = create_abort_signal(ctx, bridge);

                if ms <= 0.0 {
                    // Immediately abort with TimeoutError.
                    let reason =
                        JsValue::from(js_string!("TimeoutError: The operation timed out"));
                    signal.set(js_string!(ABORTED_KEY), JsValue::from(true), false, ctx)?;
                    signal.set(js_string!(REASON_KEY), reason, false, ctx)?;
                }
                // For positive ms: the signal will not auto-abort in this implementation
                // since we don't have timer integration here. The signal is returned
                // un-aborted and callers can check/use it with fetch or manually.
                // TODO: integrate with TimerQueue for deferred abort.

                Ok(JsValue::from(signal))
            },
            b3,
        ),
        js_string!("timeout"),
        1,
    );

    let signal_obj = init.build();
    ctx.register_global_property(js_string!("AbortSignal"), signal_obj, Attribute::all())
        .expect("failed to register AbortSignal");
}

/// Check if a JS value is an AbortSignal object.
#[allow(dead_code)]
pub(crate) fn is_abort_signal(val: &JsValue, ctx: &mut Context) -> bool {
    val.as_object().is_some_and(|obj| {
        obj.get(js_string!(SIGNAL_KEY), ctx)
            .ok()
            .is_some_and(|v| v.to_boolean())
    })
}

/// Check if an AbortSignal is aborted.
#[allow(dead_code)]
pub(crate) fn is_signal_aborted(signal: &JsObject, ctx: &mut Context) -> bool {
    signal
        .get(js_string!(ABORTED_KEY), ctx)
        .ok()
        .is_some_and(|v| v.to_boolean())
}
