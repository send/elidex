//! `Event` and `CustomEvent` constructors (WHATWG DOM §2.1, §2.2).
//!
//! Creates script-visible event objects that can be dispatched via
//! `element.dispatchEvent(event)`. Events created by constructors have
//! `isTrusted: false` and `initialized: true`.

use std::cell::Cell;
use std::rc::Rc;

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsNativeError, JsResult, JsValue, NativeFunction};

use crate::bridge::HostBridge;
use crate::globals::events::{register_flag_method, SharedFlag};

/// Hidden property keys for script-created event objects.
pub(crate) const EVENT_MARKER_KEY: &str = "__elidex_event__";
pub(crate) const EVENT_DISPATCHING_KEY: &str = "__elidex_dispatching__";
pub(crate) const EVENT_INITIALIZED_KEY: &str = "__elidex_initialized__";
pub(crate) const EVENT_TYPE_KEY: &str = "__elidex_event_type__";
pub(crate) const EVENT_BUBBLES_KEY: &str = "__elidex_event_bubbles__";
pub(crate) const EVENT_CANCELABLE_KEY: &str = "__elidex_event_cancelable__";
pub(crate) const EVENT_COMPOSED_KEY: &str = "__elidex_event_composed__";
pub(crate) const EVENT_PD_FLAG_KEY: &str = "__elidex_event_pd_flag__";
/// Hidden writable slot for `eventPhase` (updated during dispatch).
pub(crate) const EVENT_PHASE_SLOT: &str = "__elidex_event_phase__";
/// Hidden writable slot for `target` (updated during dispatch).
pub(crate) const EVENT_TARGET_SLOT: &str = "__elidex_event_target__";
/// Hidden writable slot for `currentTarget` (updated during dispatch).
pub(crate) const EVENT_CURRENT_TARGET_SLOT: &str = "__elidex_event_current_target__";

/// Read-only attribute shorthand.
const RO: Attribute = Attribute::READONLY;

/// Wrapper for `Rc<Cell<bool>>` used to store event flags in hidden properties.
///
/// Re-exports `SharedFlag` from events.rs and adds GC trace compatibility.
#[derive(Clone)]
struct SharedFlagStore(Rc<Cell<bool>>);

impl_empty_trace!(SharedFlagStore);

/// Register `Event` and `CustomEvent` global constructors.
pub fn register_event_constructors(ctx: &mut Context, _bridge: &HostBridge) {
    // Event constructor: new Event(type, eventInitDict?)
    ctx.register_global_builtin_callable(
        js_string!("Event"),
        1,
        NativeFunction::from_copy_closure(|_this, args, ctx| build_event_object(args, false, ctx)),
    )
    .expect("failed to register Event");

    // CustomEvent constructor: new CustomEvent(type, customEventInitDict?)
    ctx.register_global_builtin_callable(
        js_string!("CustomEvent"),
        1,
        NativeFunction::from_copy_closure(|_this, args, ctx| build_event_object(args, true, ctx)),
    )
    .expect("failed to register CustomEvent");
}

/// Build an Event (or `CustomEvent`) JS object from constructor arguments.
///
/// WHATWG DOM §2.1/§2.2: `new Event(type, { bubbles, cancelable, composed })`
/// and `new CustomEvent(type, { bubbles, cancelable, composed, detail })`.
#[allow(clippy::too_many_lines)]
fn build_event_object(args: &[JsValue], is_custom: bool, ctx: &mut Context) -> JsResult<JsValue> {
    // Argument 0: type (required).
    let event_type = args
        .first()
        .map(|v| v.to_string(ctx))
        .transpose()?
        .map(|s| s.to_std_string_escaped())
        .ok_or_else(|| JsNativeError::typ().with_message("Event: argument 1 (type) is required"))?;

    // Argument 1: eventInitDict (optional).
    let init = args.get(1);
    let bubbles = extract_bool_opt(init, "bubbles", ctx)?;
    let cancelable = extract_bool_opt(init, "cancelable", ctx)?;
    let composed = extract_bool_opt(init, "composed", ctx)?;

    // CustomEvent.detail (default: null).
    let detail = if is_custom {
        init.and_then(boa_engine::JsValue::as_object)
            .map(|obj| obj.get(js_string!("detail"), ctx))
            .transpose()?
            .unwrap_or(JsValue::null())
    } else {
        JsValue::undefined()
    };

    // Shared flags for preventDefault / stopPropagation / stopImmediatePropagation.
    let pd_flag = Rc::new(Cell::new(false));
    let stop_prop_flag = Rc::new(Cell::new(false));
    let stop_imm_flag = Rc::new(Cell::new(false));

    let realm = ctx.realm().clone();
    let mut init_obj = ObjectInitializer::new(ctx);

    // Core event properties (read-only per DOM spec).
    init_obj.property(
        js_string!("type"),
        JsValue::from(js_string!(event_type.as_str())),
        RO,
    );
    init_obj.property(js_string!("bubbles"), JsValue::from(bubbles), RO);
    init_obj.property(js_string!("cancelable"), JsValue::from(cancelable), RO);
    init_obj.property(js_string!("composed"), JsValue::from(composed), RO);
    // Script-created events are untrusted ([LegacyUnforgeable]).
    init_obj.property(js_string!("isTrusted"), JsValue::from(false), RO);
    init_obj.property(js_string!("timeStamp"), JsValue::from(0), RO);
    // eventPhase / target / currentTarget — getter-backed by hidden writable slots.
    // dispatch_event_for updates the hidden slots during dispatch; the getters read them.
    init_obj.accessor(
        js_string!("eventPhase"),
        Some(
            NativeFunction::from_copy_closure(|this, _args, ctx| {
                let Some(obj) = this.as_object() else {
                    return Ok(JsValue::from(0_i32));
                };
                Ok(obj
                    .get(js_string!(EVENT_PHASE_SLOT), ctx)
                    .unwrap_or(JsValue::from(0_i32)))
            })
            .to_js_function(&realm),
        ),
        None,
        Attribute::CONFIGURABLE,
    );
    init_obj.accessor(
        js_string!("target"),
        Some(
            NativeFunction::from_copy_closure(|this, _args, ctx| {
                let Some(obj) = this.as_object() else {
                    return Ok(JsValue::null());
                };
                let val = obj
                    .get(js_string!(EVENT_TARGET_SLOT), ctx)
                    .unwrap_or(JsValue::null());
                if val.is_undefined() {
                    return Ok(JsValue::null());
                }
                Ok(val)
            })
            .to_js_function(&realm),
        ),
        None,
        Attribute::CONFIGURABLE,
    );
    init_obj.accessor(
        js_string!("currentTarget"),
        Some(
            NativeFunction::from_copy_closure(|this, _args, ctx| {
                let Some(obj) = this.as_object() else {
                    return Ok(JsValue::null());
                };
                let val = obj
                    .get(js_string!(EVENT_CURRENT_TARGET_SLOT), ctx)
                    .unwrap_or(JsValue::null());
                if val.is_undefined() {
                    return Ok(JsValue::null());
                }
                Ok(val)
            })
            .to_js_function(&realm),
        ),
        None,
        Attribute::CONFIGURABLE,
    );

    // defaultPrevented — live accessor reading from shared flag.
    let pd_accessor = SharedFlag(Rc::clone(&pd_flag));
    init_obj.accessor(
        js_string!("defaultPrevented"),
        Some(
            NativeFunction::from_copy_closure_with_captures(
                |_this, _args, flag, _ctx| -> JsResult<JsValue> { Ok(JsValue::from(flag.0.get())) },
                pd_accessor,
            )
            .to_js_function(&realm),
        ),
        None,
        Attribute::CONFIGURABLE,
    );

    // preventDefault() — only sets flag when cancelable (DOM §2.5).
    let pd_shared = SharedFlag(Rc::clone(&pd_flag));
    init_obj.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, _args, (f, cancel), _ctx| {
                if *cancel {
                    f.0.set(true);
                }
                Ok(JsValue::undefined())
            },
            (pd_shared, cancelable),
        ),
        js_string!("preventDefault"),
        0,
    );

    // stopPropagation / stopImmediatePropagation.
    register_flag_method(&mut init_obj, "stopPropagation", &stop_prop_flag);
    register_flag_method(&mut init_obj, "stopImmediatePropagation", &stop_imm_flag);

    // composedPath() — returns empty array (populated during dispatch).
    init_obj.function(
        NativeFunction::from_copy_closure(|_this, _args, ctx| {
            Ok(boa_engine::object::builtins::JsArray::new(ctx).into())
        }),
        js_string!("composedPath"),
        0,
    );

    // returnValue accessor (legacy, mirrors defaultPrevented inverted).
    let rv_flag = SharedFlag(Rc::clone(&pd_flag));
    init_obj.accessor(
        js_string!("returnValue"),
        Some(
            NativeFunction::from_copy_closure_with_captures(
                |_this, _args, flag, _ctx| -> JsResult<JsValue> {
                    Ok(JsValue::from(!flag.0.get()))
                },
                rv_flag,
            )
            .to_js_function(&realm),
        ),
        None,
        Attribute::CONFIGURABLE,
    );

    // CustomEvent.detail (read-only).
    if is_custom {
        init_obj.property(js_string!("detail"), detail, RO);
    }

    // initEvent(type, bubbles, cancelable) — legacy method (WHATWG DOM §2.5).
    // No-op if dispatch flag is set. Resets stop/canceled/propagation flags, sets initialized.
    let init_pd = SharedFlagStore(Rc::clone(&pd_flag));
    let init_stop_prop = SharedFlagStore(Rc::clone(&stop_prop_flag));
    let init_stop_imm = SharedFlagStore(Rc::clone(&stop_imm_flag));
    init_obj.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, (pd_f, sp_f, si_f), ctx| {
                apply_init_event(this, args, pd_f, sp_f, si_f, ctx)
            },
            (init_pd, init_stop_prop, init_stop_imm),
        ),
        js_string!("initEvent"),
        3,
    );

    // initCustomEvent (legacy, WHATWG DOM §2.5) — same as initEvent + sets detail.
    if is_custom {
        let ice_pd = SharedFlagStore(Rc::clone(&pd_flag));
        let ice_stop_prop = SharedFlagStore(Rc::clone(&stop_prop_flag));
        let ice_stop_imm = SharedFlagStore(Rc::clone(&stop_imm_flag));
        init_obj.function(
            NativeFunction::from_copy_closure_with_captures(
                |this, args, (pd_f, sp_f, si_f), ctx| {
                    apply_init_event(this, args, pd_f, sp_f, si_f, ctx)?;
                    // Set detail (WHATWG DOM §2.5 — initCustomEvent extension).
                    if let Some(obj) = this.as_object() {
                        let dispatching = obj
                            .get(js_string!(EVENT_DISPATCHING_KEY), ctx)?
                            .to_boolean();
                        if !dispatching {
                            let detail_val = args.get(3).cloned().unwrap_or(JsValue::null());
                            let _ = obj.set(js_string!("detail"), detail_val, false, ctx);
                        }
                    }
                    Ok(JsValue::undefined())
                },
                (ice_pd, ice_stop_prop, ice_stop_imm),
            ),
            js_string!("initCustomEvent"),
            4,
        );
    }

    // Hidden properties for dispatch infrastructure.
    let hidden = Attribute::empty();
    init_obj.property(js_string!(EVENT_MARKER_KEY), JsValue::from(true), hidden);
    init_obj.property(
        js_string!(EVENT_DISPATCHING_KEY),
        JsValue::from(false),
        hidden,
    );
    init_obj.property(
        js_string!(EVENT_INITIALIZED_KEY),
        JsValue::from(true), // Constructor-created events are initialized.
        hidden,
    );
    init_obj.property(
        js_string!(EVENT_TYPE_KEY),
        JsValue::from(js_string!(event_type.as_str())),
        hidden,
    );
    init_obj.property(
        js_string!(EVENT_BUBBLES_KEY),
        JsValue::from(bubbles),
        hidden,
    );
    init_obj.property(
        js_string!(EVENT_CANCELABLE_KEY),
        JsValue::from(cancelable),
        hidden,
    );
    init_obj.property(
        js_string!(EVENT_COMPOSED_KEY),
        JsValue::from(composed),
        hidden,
    );

    // Hidden writable slots for eventPhase/target/currentTarget (updated during dispatch).
    init_obj.property(
        js_string!(EVENT_PHASE_SLOT),
        JsValue::from(0_i32),
        Attribute::WRITABLE,
    );
    init_obj.property(
        js_string!(EVENT_TARGET_SLOT),
        JsValue::null(),
        Attribute::WRITABLE,
    );
    init_obj.property(
        js_string!(EVENT_CURRENT_TARGET_SLOT),
        JsValue::null(),
        Attribute::WRITABLE,
    );

    // Store shared flags as hidden properties for dispatch_event_for() to extract.
    init_obj.property(
        js_string!(EVENT_PD_FLAG_KEY),
        JsValue::from(0.0), // Placeholder — actual Rc<Cell> stored separately.
        hidden,
    );

    Ok(init_obj.build().into())
}

/// Common initEvent logic (WHATWG DOM §2.5).
///
/// Checks dispatch flag, resets internal flags, updates type/bubbles/cancelable,
/// and resets target to null. Used by both initEvent and initCustomEvent.
#[allow(clippy::similar_names)]
fn apply_init_event(
    this: &JsValue,
    args: &[JsValue],
    pd_f: &SharedFlagStore,
    sp_f: &SharedFlagStore,
    si_f: &SharedFlagStore,
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let Some(obj) = this.as_object() else {
        return Ok(JsValue::undefined());
    };
    // Check dispatch flag — no-op during dispatch.
    let dispatching = obj
        .get(js_string!(EVENT_DISPATCHING_KEY), ctx)?
        .to_boolean();
    if dispatching {
        return Ok(JsValue::undefined());
    }
    // Set initialized flag.
    let _ = obj.set(
        js_string!(EVENT_INITIALIZED_KEY),
        JsValue::from(true),
        false,
        ctx,
    );
    // WHATWG DOM §2.5: reset stopPropagation, stopImmediatePropagation,
    // and canceled (defaultPrevented) flags.
    pd_f.0.set(false);
    sp_f.0.set(false);
    si_f.0.set(false);
    // Update type/bubbles/cancelable from arguments.
    if let Some(t) = args.first() {
        let type_str = t.to_string(ctx)?;
        let s = type_str.to_std_string_escaped();
        let _ = obj.set(
            js_string!(EVENT_TYPE_KEY),
            JsValue::from(js_string!(s.as_str())),
            false,
            ctx,
        );
        let _ = obj.set(
            js_string!("type"),
            JsValue::from(js_string!(s.as_str())),
            false,
            ctx,
        );
    }
    if let Some(b) = args.get(1) {
        let _ = obj.set(
            js_string!(EVENT_BUBBLES_KEY),
            JsValue::from(b.to_boolean()),
            false,
            ctx,
        );
    }
    if let Some(c) = args.get(2) {
        let _ = obj.set(
            js_string!(EVENT_CANCELABLE_KEY),
            JsValue::from(c.to_boolean()),
            false,
            ctx,
        );
    }
    // Reset target to null (WHATWG DOM §2.5).
    let _ = obj.set(js_string!(EVENT_TARGET_SLOT), JsValue::null(), false, ctx);
    let _ = obj.set(
        js_string!(EVENT_CURRENT_TARGET_SLOT),
        JsValue::null(),
        false,
        ctx,
    );
    let _ = obj.set(
        js_string!(EVENT_PHASE_SLOT),
        JsValue::from(0_i32),
        false,
        ctx,
    );
    Ok(JsValue::undefined())
}

/// Extract a boolean from an optional options object property, defaulting to `false`.
fn extract_bool_opt(init: Option<&JsValue>, key: &str, ctx: &mut Context) -> JsResult<bool> {
    match init.and_then(JsValue::as_object) {
        Some(obj) => Ok(obj.get(js_string!(key), ctx)?.to_boolean()),
        None => Ok(false),
    }
}

/// Build an uninitialized event for `document.createEvent()` (WHATWG DOM §4.1).
///
/// The event has `initialized=false`, so `initEvent()` must be called before dispatch.
pub(crate) fn build_uninit_event(ctx: &mut Context) -> JsResult<JsValue> {
    // Build a minimal Event with empty type and initialized=false.
    let event = build_event_object(&[JsValue::from(js_string!(""))], false, ctx)?;
    // Override initialized to false.
    if let Some(obj) = event.as_object() {
        let _ = obj.set(
            js_string!(EVENT_INITIALIZED_KEY),
            JsValue::from(false),
            false,
            ctx,
        );
    }
    Ok(event)
}
