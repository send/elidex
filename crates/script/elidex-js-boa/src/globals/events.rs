//! DOM Event object creation for boa.
//!
//! Creates JS event objects with properties and methods matching the DOM
//! Event interface (type, bubbles, target, currentTarget, eventPhase,
//! preventDefault, stopPropagation, stopImmediatePropagation).

use std::cell::Cell;
use std::rc::Rc;

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsValue, NativeFunction};
use elidex_plugin::EventPayload;
use elidex_script_session::DispatchEvent;

/// Wrapper around `Rc<Cell<bool>>` that implements `boa_gc::Trace`.
///
/// The wrapped value contains no GC-managed objects, so trace is a no-op.
/// The `custom_trace!` macro requires `mark` to be referenced at least once;
/// we mark a unit value `()` as a harmless placeholder to satisfy this
/// constraint — it is not a real GC object and has no effect on collection.
#[derive(Clone)]
pub(crate) struct SharedFlag(pub Rc<Cell<bool>>);

// Safety: SharedFlag contains no GC-managed objects, trace is a no-op.
impl_empty_trace!(SharedFlag);

/// Wrapper around `Rc<JsValue>` for `composedPath()` captures.
///
/// The wrapped `JsValue` is a `JsArray`. Since we store it via `Rc` (not GC-managed),
/// trace is a no-op.
#[derive(Clone)]
struct SharedPathValue(Rc<JsValue>);

impl_empty_trace!(SharedPathValue);

/// Read-only attribute for DOM Event properties (per DOM spec).
const RO: Attribute = Attribute::READONLY;

/// Register a flag-setting method on an event object (e.g. `preventDefault`).
///
/// The method sets the shared `Rc<Cell<bool>>` flag to `true` when called.
fn register_flag_method(init: &mut ObjectInitializer<'_>, name: &str, flag: &Rc<Cell<bool>>) {
    let shared = SharedFlag(Rc::clone(flag));
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, _args, f, _ctx| {
                f.0.set(true);
                Ok(JsValue::undefined())
            },
            shared,
        ),
        js_string!(name),
        0,
    );
}

/// Shared event dispatch flags passed between the dispatch loop and JS event methods.
///
/// The `Rc<Cell<bool>>` flags allow JS code (e.g. `event.stopPropagation()`)
/// to communicate back to the dispatch loop immediately.
pub struct EventFlags {
    /// Set to `true` when `event.preventDefault()` is called.
    pub prevent_default: Rc<Cell<bool>>,
    /// Set to `true` when `event.stopPropagation()` is called.
    pub stop_propagation: Rc<Cell<bool>>,
    /// Set to `true` when `event.stopImmediatePropagation()` is called.
    pub stop_immediate: Rc<Cell<bool>>,
}

/// Create a JS event object for the given dispatch event.
///
/// The returned object has:
/// - `type`, `bubbles`, `cancelable`, `eventPhase`, `defaultPrevented`
/// - `target`, `currentTarget` (passed as resolved JS values)
/// - `timeStamp` (always 0 for Phase 2)
/// - Mouse props: `clientX`, `clientY`, `button`, `buttons`
/// - Keyboard props: `key`, `code`
/// - Modifier props: `altKey`, `ctrlKey`, `metaKey`, `shiftKey`
/// - `preventDefault()`, `stopPropagation()`, `stopImmediatePropagation()`
/// - `composedPath()` — returns the event propagation path
///
/// The [`EventFlags`] are shared with the dispatch loop so that
/// calling `event.stopPropagation()` in JS immediately affects the loop.
///
/// `composed_path_array` is a pre-built JS array of element wrappers for
/// `composedPath()`. If `None`, `composedPath()` returns an empty array.
#[allow(clippy::too_many_lines)]
pub fn create_event_object(
    event: &DispatchEvent,
    target_wrapper: &JsValue,
    current_target_wrapper: &JsValue,
    flags: &EventFlags,
    composed_path_array: Option<JsValue>,
    ctx: &mut Context,
) -> JsValue {
    // Build composedPath value before borrowing ctx for ObjectInitializer.
    let path_val = composed_path_array
        .unwrap_or_else(|| boa_engine::object::builtins::JsArray::new(ctx).into());
    // Pre-build values that need ctx before ObjectInitializer borrows it.
    let empty_ports: JsValue = if matches!(event.payload, EventPayload::Message { .. }) {
        boa_engine::object::builtins::JsArray::new(ctx).into()
    } else {
        JsValue::undefined()
    };
    let realm = ctx.realm().clone();

    let mut init = ObjectInitializer::new(ctx);

    // Core event properties (read-only per DOM spec).
    init.property(
        js_string!("type"),
        JsValue::from(js_string!(event.event_type.as_str())),
        RO,
    );
    init.property(js_string!("bubbles"), JsValue::from(event.bubbles), RO);
    init.property(
        js_string!("cancelable"),
        JsValue::from(event.cancelable),
        RO,
    );
    init.property(
        js_string!("eventPhase"),
        JsValue::from(i32::from(event.phase as u8)),
        RO,
    );
    // Live getter: reads from the shared prevent_default flag so that
    // `event.defaultPrevented` reflects the current state even within
    // the same listener that called `preventDefault()`.
    let pd_flag = SharedFlag(Rc::clone(&flags.prevent_default));
    init.accessor(
        js_string!("defaultPrevented"),
        Some(
            NativeFunction::from_copy_closure_with_captures(
                |_this, _args, flag, _ctx| -> boa_engine::JsResult<JsValue> {
                    Ok(JsValue::from(flag.0.get()))
                },
                pd_flag,
            )
            .to_js_function(&realm),
        ),
        None,
        Attribute::CONFIGURABLE,
    );
    init.property(js_string!("target"), target_wrapper.clone(), RO);
    init.property(
        js_string!("currentTarget"),
        current_target_wrapper.clone(),
        RO,
    );
    init.property(js_string!("timeStamp"), JsValue::from(0), RO);
    init.property(js_string!("composed"), JsValue::from(event.composed), RO);
    // WHATWG DOM §2.1: isTrusted is true for events dispatched by the user agent.
    init.property(js_string!("isTrusted"), JsValue::from(true), RO);

    // Payload-specific properties (also read-only).
    set_payload_properties(&mut init, &event.payload, empty_ports);

    // preventDefault() only sets the flag when the event is cancelable (DOM §2.5).
    let pd_shared = SharedFlag(Rc::clone(&flags.prevent_default));
    let cancelable = event.cancelable;
    init.function(
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
    register_flag_method(&mut init, "stopPropagation", &flags.stop_propagation);

    // stopImmediatePropagation() sets both propagation + immediate flags.
    let stop_prop = SharedFlag(Rc::clone(&flags.stop_propagation));
    let stop_imm = SharedFlag(Rc::clone(&flags.stop_immediate));
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, _args, (sp, si), _ctx| {
                sp.0.set(true);
                si.0.set(true);
                Ok(JsValue::undefined())
            },
            (stop_prop, stop_imm),
        ),
        js_string!("stopImmediatePropagation"),
        0,
    );

    // returnValue — the setter behavior differs by event type:
    // - beforeunload (HTML §8.1.7.1): setting to truthy (non-empty string) cancels.
    // - general Event (legacy spec): setting to false cancels.
    let rv_pd = SharedFlag(Rc::clone(&flags.prevent_default));
    let rv_cancelable = cancelable;
    let event_type_for_rv = event.event_type.clone();
    init.accessor(
        js_string!("returnValue"),
        Some(
            NativeFunction::from_copy_closure_with_captures(
                |_this, _args, flag, _ctx| -> boa_engine::JsResult<JsValue> {
                    // Getter: per WHATWG DOM spec, returnValue returns false if the
                    // canceled flag is set, true otherwise.
                    Ok(JsValue::from(!flag.0.get()))
                },
                SharedFlag(Rc::clone(&flags.prevent_default)),
            )
            .to_js_function(&realm),
        ),
        Some(
            NativeFunction::from_copy_closure_with_captures(
                |_this, args, (flag, cancel, ev_type), _ctx| -> boa_engine::JsResult<JsValue> {
                    if *cancel {
                        if let Some(val) = args.first() {
                            if ev_type == "beforeunload" {
                                // HTML §8.1.7.1: non-empty string cancels.
                                let is_non_empty_string =
                                    val.as_string().is_some_and(|s| !s.is_empty());
                                if is_non_empty_string {
                                    flag.0.set(true);
                                }
                            } else {
                                // Legacy Event.returnValue: falsy value (false) cancels.
                                if !val.to_boolean() {
                                    flag.0.set(true);
                                }
                            }
                        }
                    }
                    Ok(JsValue::undefined())
                },
                (rv_pd, rv_cancelable, event_type_for_rv),
            )
            .to_js_function(&realm),
        ),
        Attribute::CONFIGURABLE,
    );

    // composedPath() — returns the pre-built propagation path array.
    // Wrap in SharedPathValue for GC tracing.
    let shared_path = SharedPathValue(Rc::new(path_val));
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, _args, path, _ctx| {
                // Clone the array reference so each call returns the same array.
                Ok((*path.0).clone())
            },
            shared_path,
        ),
        js_string!("composedPath"),
        0,
    );

    init.build().into()
}

/// Create a standalone Event object for non-DOM event targets (`WebSocket`, `EventSource`).
///
/// Unlike [`create_event_object`], this does not require a DOM entity target or
/// shared dispatch flags. The event has `preventDefault()`/`stopPropagation()`
/// methods, `target`/`currentTarget` set to null, and payload-specific properties.
///
/// Used by `WebSocket` `onmessage`/`onclose`/`onopen`/`onerror` and `EventSource` events.
pub fn create_standalone_event(
    event_type: &str,
    payload: &EventPayload,
    cancelable: bool,
    ctx: &mut Context,
) -> JsValue {
    // Pre-build ports array for Message payloads.
    let empty_ports: JsValue = if matches!(payload, EventPayload::Message { .. }) {
        boa_engine::object::builtins::JsArray::new(ctx).into()
    } else {
        JsValue::undefined()
    };

    let prevent_default = Rc::new(Cell::new(false));
    let realm = ctx.realm().clone();

    let mut init = ObjectInitializer::new(ctx);

    // Core event properties.
    init.property(
        js_string!("type"),
        JsValue::from(js_string!(event_type)),
        RO,
    );
    init.property(js_string!("bubbles"), JsValue::from(false), RO);
    init.property(js_string!("cancelable"), JsValue::from(cancelable), RO);
    init.property(js_string!("target"), JsValue::null(), RO);
    init.property(js_string!("currentTarget"), JsValue::null(), RO);
    init.property(js_string!("composed"), JsValue::from(false), RO);
    init.property(js_string!("timeStamp"), JsValue::from(0), RO);
    init.property(
        js_string!("eventPhase"),
        JsValue::from(0_i32), // NONE
        RO,
    );
    // WHATWG DOM §2.1: isTrusted is true for events dispatched by the user agent.
    init.property(js_string!("isTrusted"), JsValue::from(true), RO);

    // defaultPrevented accessor.
    let pd_flag = SharedFlag(Rc::clone(&prevent_default));
    init.accessor(
        js_string!("defaultPrevented"),
        Some(
            NativeFunction::from_copy_closure_with_captures(
                |_this, _args, flag, _ctx| -> boa_engine::JsResult<JsValue> {
                    Ok(JsValue::from(flag.0.get()))
                },
                pd_flag,
            )
            .to_js_function(&realm),
        ),
        None,
        Attribute::CONFIGURABLE,
    );

    // preventDefault().
    let pd_shared = SharedFlag(Rc::clone(&prevent_default));
    init.function(
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

    // stopPropagation() / stopImmediatePropagation() — no-ops for standalone events.
    init.function(
        NativeFunction::from_copy_closure(|_this, _args, _ctx| Ok(JsValue::undefined())),
        js_string!("stopPropagation"),
        0,
    );
    init.function(
        NativeFunction::from_copy_closure(|_this, _args, _ctx| Ok(JsValue::undefined())),
        js_string!("stopImmediatePropagation"),
        0,
    );

    // composedPath() — empty array for standalone events.
    init.function(
        NativeFunction::from_copy_closure(|_this, _args, ctx| {
            Ok(boa_engine::object::builtins::JsArray::new(ctx).into())
        }),
        js_string!("composedPath"),
        0,
    );

    // Payload-specific properties.
    set_payload_properties(&mut init, payload, empty_ports);

    init.build().into()
}

/// Modifier key state (alt/ctrl/meta/shift).
#[allow(clippy::struct_excessive_bools)] // DOM UIEvent spec requires 4 modifier key booleans.
struct ModifierKeys {
    alt: bool,
    ctrl: bool,
    meta: bool,
    shift: bool,
}

fn set_modifier_keys(init: &mut ObjectInitializer<'_>, keys: &ModifierKeys) {
    init.property(js_string!("altKey"), JsValue::from(keys.alt), RO);
    init.property(js_string!("ctrlKey"), JsValue::from(keys.ctrl), RO);
    init.property(js_string!("metaKey"), JsValue::from(keys.meta), RO);
    init.property(js_string!("shiftKey"), JsValue::from(keys.shift), RO);
}

/// Set `elapsedTime` and `pseudoElement` properties shared by transition/animation events.
fn set_elapsed_and_pseudo(
    init: &mut ObjectInitializer<'_>,
    elapsed_time: f64,
    pseudo_element: &str,
) {
    init.property(js_string!("elapsedTime"), JsValue::from(elapsed_time), RO);
    init.property(
        js_string!("pseudoElement"),
        JsValue::from(js_string!(pseudo_element)),
        RO,
    );
}

fn set_transition_payload(
    init: &mut ObjectInitializer<'_>,
    t: &elidex_plugin::TransitionEventInit,
) {
    set_elapsed_and_pseudo(init, t.elapsed_time, &t.pseudo_element);
    init.property(
        js_string!("propertyName"),
        JsValue::from(js_string!(t.property_name.as_str())),
        RO,
    );
}

fn set_animation_payload(init: &mut ObjectInitializer<'_>, a: &elidex_plugin::AnimationEventInit) {
    set_elapsed_and_pseudo(init, a.elapsed_time, &a.pseudo_element);
    init.property(
        js_string!("animationName"),
        JsValue::from(js_string!(a.animation_name.as_str())),
        RO,
    );
}

fn set_payload_properties(
    init: &mut ObjectInitializer<'_>,
    payload: &EventPayload,
    empty_ports: JsValue,
) {
    match payload {
        EventPayload::Mouse(m) => set_mouse_payload(init, m),
        EventPayload::Keyboard(k) => set_keyboard_payload(init, k),
        EventPayload::Transition(t) => set_transition_payload(init, t),
        EventPayload::Animation(a) => set_animation_payload(init, a),
        EventPayload::Input(i) => set_input_payload(init, i),
        EventPayload::Clipboard(c) => set_clipboard_payload(init, c),
        EventPayload::Composition(c) => set_composition_payload(init, c),
        EventPayload::Focus(_f) => set_focus_payload(init),
        EventPayload::Message {
            data,
            origin,
            last_event_id,
        } => {
            set_message_payload(init, data, origin, last_event_id, empty_ports);
        }
        EventPayload::CloseEvent(c) => set_close_event_payload(init, c),
        EventPayload::None | _ => {}
    }
}

fn set_mouse_payload(init: &mut ObjectInitializer<'_>, m: &elidex_plugin::MouseEventInit) {
    init.property(js_string!("clientX"), JsValue::from(m.client_x), RO);
    init.property(js_string!("clientY"), JsValue::from(m.client_y), RO);
    init.property(js_string!("button"), JsValue::from(i32::from(m.button)), RO);
    init.property(
        js_string!("buttons"),
        JsValue::from(i32::from(m.buttons)),
        RO,
    );
    set_modifier_keys(
        init,
        &ModifierKeys {
            alt: m.alt_key,
            ctrl: m.ctrl_key,
            meta: m.meta_key,
            shift: m.shift_key,
        },
    );
}

fn set_keyboard_payload(init: &mut ObjectInitializer<'_>, k: &elidex_plugin::KeyboardEventInit) {
    init.property(
        js_string!("key"),
        JsValue::from(js_string!(k.key.as_str())),
        RO,
    );
    init.property(
        js_string!("code"),
        JsValue::from(js_string!(k.code.as_str())),
        RO,
    );
    init.property(js_string!("repeat"), JsValue::from(k.repeat), RO);
    set_modifier_keys(
        init,
        &ModifierKeys {
            alt: k.alt_key,
            ctrl: k.ctrl_key,
            meta: k.meta_key,
            shift: k.shift_key,
        },
    );
}

fn set_input_payload(init: &mut ObjectInitializer<'_>, i: &elidex_plugin::InputEventInit) {
    init.property(
        js_string!("inputType"),
        JsValue::from(js_string!(i.input_type.as_str())),
        RO,
    );
    init.property(
        js_string!("data"),
        i.data
            .as_deref()
            .map_or(JsValue::null(), |d| JsValue::from(js_string!(d))),
        RO,
    );
    init.property(js_string!("isComposing"), JsValue::from(i.is_composing), RO);
}

fn set_clipboard_payload(init: &mut ObjectInitializer<'_>, c: &elidex_plugin::ClipboardEventInit) {
    init.property(
        js_string!("clipboardData"),
        JsValue::from(js_string!(c.data.as_str())),
        RO,
    );
}

fn set_composition_payload(
    init: &mut ObjectInitializer<'_>,
    c: &elidex_plugin::CompositionEventInit,
) {
    init.property(
        js_string!("data"),
        JsValue::from(js_string!(c.data.as_str())),
        RO,
    );
}

fn set_message_payload(
    init: &mut ObjectInitializer<'_>,
    data: &str,
    origin: &str,
    last_event_id: &str,
    ports: JsValue,
) {
    // WHATWG HTML §9.4.3 / §9.2 / §9.3: MessageEvent properties.
    init.property(js_string!("data"), JsValue::from(js_string!(data)), RO);
    init.property(js_string!("origin"), JsValue::from(js_string!(origin)), RO);
    init.property(
        js_string!("lastEventId"),
        JsValue::from(js_string!(last_event_id)),
        RO,
    );
    // source: null (cross-context WindowProxy not available in boa).
    init.property(js_string!("source"), JsValue::null(), RO);
    // ports: empty frozen array (MessagePort not implemented).
    init.property(js_string!("ports"), ports, RO);
}

fn set_close_event_payload(init: &mut ObjectInitializer<'_>, c: &elidex_plugin::CloseEventInit) {
    // WHATWG HTML CloseEvent properties.
    init.property(js_string!("code"), JsValue::from(i32::from(c.code)), RO);
    init.property(
        js_string!("reason"),
        JsValue::from(js_string!(c.reason.as_str())),
        RO,
    );
    init.property(js_string!("wasClean"), JsValue::from(c.was_clean), RO);
}

fn set_focus_payload(init: &mut ObjectInitializer<'_>) {
    // UI Events §5.2: relatedTarget is the EventTarget losing/gaining focus.
    // Initialized as null here; JsRuntime::dispatch_event resolves the entity
    // bits to a JS wrapper and overwrites via obj.set().  Must be writable +
    // configurable so the runtime assignment succeeds.
    init.property(
        js_string!("relatedTarget"),
        JsValue::null(),
        Attribute::WRITABLE | Attribute::CONFIGURABLE,
    );
}
