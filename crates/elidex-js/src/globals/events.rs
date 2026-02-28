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

impl boa_gc::Finalize for SharedFlag {
    fn finalize(&self) {}
}

// Safety: SharedFlag contains no GC-managed objects, trace is a no-op.
// The `mark(&())` call is a no-op placeholder required by the macro.
#[allow(unsafe_code)]
unsafe impl boa_gc::Trace for SharedFlag {
    boa_gc::custom_trace!(this, mark, {
        mark(&());
        let _ = this;
    });
}

/// Read-only attribute for DOM Event properties (per DOM spec).
const RO: Attribute = Attribute::READONLY;

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
///
/// The `Rc<Cell<bool>>` flags are shared with the dispatch loop so that
/// calling `event.stopPropagation()` in JS immediately affects the loop.
pub fn create_event_object(
    event: &DispatchEvent,
    target_wrapper: &JsValue,
    current_target_wrapper: &JsValue,
    prevent_default_flag: &Rc<Cell<bool>>,
    stop_propagation_flag: &Rc<Cell<bool>>,
    stop_immediate_flag: &Rc<Cell<bool>>,
    ctx: &mut Context,
) -> JsValue {
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
    // TODO(Phase 3): replace with an accessor (live getter) that reads from
    // prevent_default_flag, so `event.defaultPrevented` reflects the current
    // state even within the same listener that called `preventDefault()`.
    init.property(
        js_string!("defaultPrevented"),
        JsValue::from(event.default_prevented),
        RO,
    );
    init.property(js_string!("target"), target_wrapper.clone(), RO);
    init.property(
        js_string!("currentTarget"),
        current_target_wrapper.clone(),
        RO,
    );
    init.property(js_string!("timeStamp"), JsValue::from(0), RO);

    // Payload-specific properties (also read-only).
    set_payload_properties(&mut init, &event.payload);

    // preventDefault()
    let flag = SharedFlag(Rc::clone(prevent_default_flag));
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, _args, flag, _ctx| {
                flag.0.set(true);
                Ok(JsValue::undefined())
            },
            flag,
        ),
        js_string!("preventDefault"),
        0,
    );

    // stopPropagation()
    let flag = SharedFlag(Rc::clone(stop_propagation_flag));
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, _args, flag, _ctx| {
                flag.0.set(true);
                Ok(JsValue::undefined())
            },
            flag,
        ),
        js_string!("stopPropagation"),
        0,
    );

    // stopImmediatePropagation()
    let stop_prop = SharedFlag(Rc::clone(stop_propagation_flag));
    let stop_imm = SharedFlag(Rc::clone(stop_immediate_flag));
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

    init.build().into()
}

/// Modifier key state (alt/ctrl/meta/shift).
#[allow(clippy::struct_excessive_bools)]
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

fn set_payload_properties(init: &mut ObjectInitializer<'_>, payload: &EventPayload) {
    match payload {
        EventPayload::Mouse(m) => {
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
        EventPayload::Keyboard(k) => {
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
        EventPayload::None | _ => {}
    }
}
