//! Tests for `new UIEvent` / `new MouseEvent` / `new KeyboardEvent` /
//! `new FocusEvent` / `new InputEvent` (UI Events §3 / §5 / §6 / §7 /
//! §8).
//!
//! Covers:
//! - `[Constructor]` gate (call-mode throws TypeError) for all 5 ctors
//! - Required first argument (absent → TypeError)
//! - UIEventInit coercion (view + detail)
//! - Prototype chain: descendant → UIEvent.prototype → Event.prototype
//! - Own-data instance members (clientX / clientY / button / buttons /
//!   altKey / shiftKey / ctrlKey / metaKey / screenX / screenY /
//!   movementX / movementY / relatedTarget / key / code / repeat /
//!   isComposing / location / data / inputType)
//! - `view` resolution: null / undefined / globalThis allowed;
//!   other values throw TypeError
//! - `relatedTarget` resolution: null / undefined → null; DOM
//!   HostObject wrappers accepted; plain objects and primitives
//!   throw TypeError (WebIDL `EventTarget?` brand check)
//! - Inherited Event members (type / bubbles / cancelable / timeStamp /
//!   isTrusted) on UIEvent-family instances
//! - Symbol `type` argument propagates TypeError from `coerce::to_string`

#![cfg(feature = "engine")]

use super::super::value::JsValue;
use super::super::Vm;
use super::{eval_bool, eval_number, eval_string};

// ---------------------------------------------------------------------------
// [Constructor] gate + required args (shared across 5 ctors)
// ---------------------------------------------------------------------------

fn expect_type_error(vm: &mut Vm, source: &str) {
    let result = vm
        .eval(&format!(
            "var caught = null; \
             try {{ {source}; }} catch (e) {{ caught = e.name; }} caught;"
        ))
        .unwrap();
    match result {
        JsValue::String(id) => assert_eq!(vm.get_string(id), "TypeError"),
        other => panic!("expected TypeError, got {other:?} (source: {source})"),
    }
}

#[test]
fn all_ui_event_ctors_reject_call_mode() {
    let mut vm = Vm::new();
    for source in [
        "UIEvent('x')",
        "MouseEvent('x')",
        "KeyboardEvent('x')",
        "FocusEvent('x')",
        "InputEvent('x')",
    ] {
        expect_type_error(&mut vm, source);
    }
}

#[test]
fn all_ui_event_ctors_reject_missing_type() {
    let mut vm = Vm::new();
    for source in [
        "new UIEvent()",
        "new MouseEvent()",
        "new KeyboardEvent()",
        "new FocusEvent()",
        "new InputEvent()",
    ] {
        expect_type_error(&mut vm, source);
    }
}

#[test]
fn all_ui_event_ctors_reject_symbol_type() {
    // ToString on a Symbol throws.
    let mut vm = Vm::new();
    for source in [
        "new UIEvent(Symbol.iterator)",
        "new MouseEvent(Symbol.iterator)",
        "new KeyboardEvent(Symbol.iterator)",
        "new FocusEvent(Symbol.iterator)",
        "new InputEvent(Symbol.iterator)",
    ] {
        expect_type_error(&mut vm, source);
    }
}

// ---------------------------------------------------------------------------
// UIEvent basics
// ---------------------------------------------------------------------------

#[test]
fn ui_event_type_and_defaults() {
    assert_eq!(eval_string("new UIEvent('click').type"), "click");
    assert!(!eval_bool("new UIEvent('click').bubbles"));
    assert!(!eval_bool("new UIEvent('click').cancelable"));
    assert_eq!(eval_number("new UIEvent('click').detail"), 0.0);
    assert!(matches!(
        Vm::new().eval("new UIEvent('click').view").unwrap(),
        JsValue::Null
    ));
}

#[test]
fn ui_event_inherits_event_members() {
    // `type` / `isTrusted` resolve via the inherited core-9 slots.
    assert!(!eval_bool("new UIEvent('x').isTrusted"));
    assert_eq!(eval_number("new UIEvent('x').eventPhase"), 0.0);
    assert!(eval_bool("new UIEvent('x').timeStamp >= 0"));
}

#[test]
fn ui_event_prototype_chain() {
    // instance → UIEvent.prototype → Event.prototype.
    assert!(eval_bool(
        "Object.getPrototypeOf(new UIEvent('x')) === UIEvent.prototype"
    ));
    assert!(eval_bool(
        "Object.getPrototypeOf(UIEvent.prototype) === Event.prototype"
    ));
    assert!(eval_bool("new UIEvent('x').constructor === UIEvent"));
}

#[test]
fn ui_event_view_globalthis_is_accepted() {
    // `view === globalThis` is the only non-null accepted form.  The
    // return value is the same reference.
    assert!(eval_bool(
        "new UIEvent('x', {view: globalThis}).view === globalThis"
    ));
}

#[test]
fn ui_event_view_non_window_throws() {
    let mut vm = Vm::new();
    expect_type_error(&mut vm, "new UIEvent('x', {view: {}})");
    expect_type_error(&mut vm, "new UIEvent('x', {view: 42})");
    expect_type_error(&mut vm, "new UIEvent('x', {view: 'str'})");
}

#[test]
fn ui_event_detail_coerces_via_to_number() {
    assert_eq!(eval_number("new UIEvent('x', {detail: 7}).detail"), 7.0);
    assert_eq!(eval_number("new UIEvent('x', {detail: '42'}).detail"), 42.0);
    assert!(eval_bool(
        "Object.is(new UIEvent('x', {detail: NaN}).detail, NaN)"
    ));
}

// ---------------------------------------------------------------------------
// MouseEvent
// ---------------------------------------------------------------------------

#[test]
fn mouse_event_prototype_chain() {
    assert!(eval_bool(
        "Object.getPrototypeOf(new MouseEvent('click')) === MouseEvent.prototype"
    ));
    assert!(eval_bool(
        "Object.getPrototypeOf(MouseEvent.prototype) === UIEvent.prototype"
    ));
    assert!(eval_bool(
        "new MouseEvent('click').constructor === MouseEvent"
    ));
}

#[test]
fn mouse_event_coord_fields_default_zero() {
    // All numeric coordinate/movement fields default to 0.
    for name in [
        "clientX",
        "clientY",
        "screenX",
        "screenY",
        "movementX",
        "movementY",
        "button",
        "buttons",
    ] {
        let src = format!("new MouseEvent('click').{name}");
        assert_eq!(eval_number(&src), 0.0, "{name} should default to 0");
    }
    // Modifier booleans default to false.
    for name in ["altKey", "ctrlKey", "metaKey", "shiftKey"] {
        let src = format!("new MouseEvent('click').{name}");
        assert!(!eval_bool(&src), "{name} should default to false");
    }
}

#[test]
fn mouse_event_init_coords_pass_through() {
    let init =
        "{clientX: 10, clientY: 20, screenX: 100, screenY: 200, movementX: 3, movementY: -4}";
    assert_eq!(
        eval_number(&format!("new MouseEvent('m', {init}).clientX")),
        10.0
    );
    assert_eq!(
        eval_number(&format!("new MouseEvent('m', {init}).clientY")),
        20.0
    );
    assert_eq!(
        eval_number(&format!("new MouseEvent('m', {init}).screenX")),
        100.0
    );
    assert_eq!(
        eval_number(&format!("new MouseEvent('m', {init}).screenY")),
        200.0
    );
    assert_eq!(
        eval_number(&format!("new MouseEvent('m', {init}).movementX")),
        3.0
    );
    assert_eq!(
        eval_number(&format!("new MouseEvent('m', {init}).movementY")),
        -4.0
    );
}

#[test]
fn mouse_event_button_truncates_via_to_int16() {
    // `button` is WebIDL `short` — ToInt16 conversion.  2^15 overflows
    // to -2^15; non-integers truncate toward zero.
    assert_eq!(
        eval_number("new MouseEvent('m', {button: 2.7}).button"),
        2.0
    );
    assert_eq!(
        eval_number("new MouseEvent('m', {button: -1}).button"),
        -1.0
    );
    assert_eq!(
        eval_number("new MouseEvent('m', {button: 32768}).button"),
        -32768.0
    );
}

#[test]
fn mouse_event_buttons_truncates_via_to_uint16() {
    assert_eq!(
        eval_number("new MouseEvent('m', {buttons: 65536}).buttons"),
        0.0
    );
    assert_eq!(
        eval_number("new MouseEvent('m', {buttons: 0xFFFF}).buttons"),
        65535.0
    );
}

#[test]
fn mouse_event_modifiers_pass_through() {
    let init = "{ctrlKey: true, shiftKey: true, altKey: true, metaKey: true}";
    assert!(eval_bool(&format!("new MouseEvent('m', {init}).ctrlKey")));
    assert!(eval_bool(&format!("new MouseEvent('m', {init}).shiftKey")));
    assert!(eval_bool(&format!("new MouseEvent('m', {init}).altKey")));
    assert!(eval_bool(&format!("new MouseEvent('m', {init}).metaKey")));
}

#[test]
fn mouse_event_related_target_null_and_missing_map_to_null() {
    // null / missing → null
    assert!(matches!(
        Vm::new().eval("new MouseEvent('m').relatedTarget").unwrap(),
        JsValue::Null
    ));
    assert!(matches!(
        Vm::new()
            .eval("new MouseEvent('m', {relatedTarget: null}).relatedTarget")
            .unwrap(),
        JsValue::Null
    ));
}

#[test]
fn mouse_event_related_target_accepts_dom_wrapper() {
    // A real DOM HostObject (document wrapper) passes the
    // `EventTarget?` brand check and is identity-preserved.
    use super::super::test_helpers::setup_with_element;
    use elidex_ecs::EcsDom;
    use elidex_script_session::SessionCore;

    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    #[allow(unsafe_code)]
    unsafe {
        setup_with_element(&mut vm, &mut session, &mut dom, doc, "div");
    }
    assert!(matches!(
        vm.eval("new MouseEvent('m', {relatedTarget: el}).relatedTarget === el;")
            .unwrap(),
        JsValue::Boolean(true)
    ));
    vm.unbind();
}

#[test]
fn mouse_event_related_target_rejects_non_event_target() {
    // WebIDL `EventTarget? relatedTarget` — plain `{}` and non-DOM
    // objects (including primitives) are not EventTargets, so they
    // throw TypeError per browser behaviour.  Covers the regression
    // where the resolver accepted any `JsValue::Object(_)`.
    let mut vm = Vm::new();
    expect_type_error(&mut vm, "new MouseEvent('m', {relatedTarget: {}})");
    expect_type_error(&mut vm, "new MouseEvent('m', {relatedTarget: []})");
    expect_type_error(&mut vm, "new MouseEvent('m', {relatedTarget: 42})");
    expect_type_error(&mut vm, "new MouseEvent('m', {relatedTarget: 'x'})");
    expect_type_error(&mut vm, "new MouseEvent('m', {relatedTarget: true})");
}

// ---------------------------------------------------------------------------
// KeyboardEvent
// ---------------------------------------------------------------------------

#[test]
fn keyboard_event_prototype_chain() {
    assert!(eval_bool(
        "Object.getPrototypeOf(new KeyboardEvent('x')) === KeyboardEvent.prototype"
    ));
    assert!(eval_bool(
        "Object.getPrototypeOf(KeyboardEvent.prototype) === UIEvent.prototype"
    ));
}

#[test]
fn keyboard_event_defaults() {
    assert_eq!(eval_string("new KeyboardEvent('k').key"), "");
    assert_eq!(eval_string("new KeyboardEvent('k').code"), "");
    assert_eq!(eval_number("new KeyboardEvent('k').location"), 0.0);
    assert!(!eval_bool("new KeyboardEvent('k').repeat"));
    assert!(!eval_bool("new KeyboardEvent('k').isComposing"));
}

#[test]
fn keyboard_event_init_pass_through() {
    // `key` and `code` pass through ToString (strings stay as-is).
    let init = "{key: 'Enter', code: 'Enter', location: 0, repeat: true, isComposing: false}";
    assert_eq!(
        eval_string(&format!("new KeyboardEvent('k', {init}).key")),
        "Enter"
    );
    assert_eq!(
        eval_string(&format!("new KeyboardEvent('k', {init}).code")),
        "Enter"
    );
    assert!(eval_bool(&format!("new KeyboardEvent('k', {init}).repeat")));
    assert!(!eval_bool(&format!(
        "new KeyboardEvent('k', {init}).isComposing"
    )));
}

// ---------------------------------------------------------------------------
// FocusEvent
// ---------------------------------------------------------------------------

#[test]
fn focus_event_prototype_chain() {
    assert!(eval_bool(
        "Object.getPrototypeOf(new FocusEvent('focus')) === FocusEvent.prototype"
    ));
    assert!(eval_bool(
        "Object.getPrototypeOf(FocusEvent.prototype) === UIEvent.prototype"
    ));
}

#[test]
fn focus_event_related_target_default_null() {
    assert!(matches!(
        Vm::new()
            .eval("new FocusEvent('focus').relatedTarget")
            .unwrap(),
        JsValue::Null
    ));
}

#[test]
fn focus_event_related_target_rejects_non_event_target() {
    // WebIDL `EventTarget? relatedTarget` — plain objects and
    // primitives are not EventTargets (shared brand check with
    // MouseEvent's resolver).
    let mut vm = Vm::new();
    expect_type_error(&mut vm, "new FocusEvent('focus', {relatedTarget: {}})");
    expect_type_error(&mut vm, "new FocusEvent('focus', {relatedTarget: 42})");
}

#[test]
fn focus_event_related_target_accepts_dom_wrapper() {
    // DOM HostObject wrapper is accepted and identity-preserved.
    use super::super::test_helpers::setup_with_element;
    use elidex_ecs::EcsDom;
    use elidex_script_session::SessionCore;

    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    #[allow(unsafe_code)]
    unsafe {
        setup_with_element(&mut vm, &mut session, &mut dom, doc, "input");
    }
    assert!(matches!(
        vm.eval("new FocusEvent('focus', {relatedTarget: el}).relatedTarget === el;")
            .unwrap(),
        JsValue::Boolean(true)
    ));
    vm.unbind();
}

// ---------------------------------------------------------------------------
// InputEvent
// ---------------------------------------------------------------------------

#[test]
fn input_event_prototype_chain() {
    assert!(eval_bool(
        "Object.getPrototypeOf(new InputEvent('input')) === InputEvent.prototype"
    ));
    assert!(eval_bool(
        "Object.getPrototypeOf(InputEvent.prototype) === UIEvent.prototype"
    ));
}

#[test]
fn input_event_defaults() {
    // `data` default is null (nullable DOMString in WebIDL), not empty string.
    assert!(matches!(
        Vm::new().eval("new InputEvent('i').data").unwrap(),
        JsValue::Null
    ));
    assert_eq!(eval_string("new InputEvent('i').inputType"), "");
    assert!(!eval_bool("new InputEvent('i').isComposing"));
}

#[test]
fn input_event_init_pass_through() {
    assert_eq!(
        eval_string("new InputEvent('i', {data: 'hi', inputType: 'insertText'}).data"),
        "hi"
    );
    assert_eq!(
        eval_string("new InputEvent('i', {data: 'hi', inputType: 'insertText'}).inputType"),
        "insertText"
    );
    assert!(eval_bool(
        "new InputEvent('i', {isComposing: true}).isComposing"
    ));
}

// ---------------------------------------------------------------------------
// Cross-cutting: brand-check + getter-propagation
// ---------------------------------------------------------------------------

#[test]
fn mouse_event_init_getter_throw_propagates() {
    // WHATWG dictionary coercion: getters on the init object may
    // fire and their exceptions propagate out of the constructor.
    let mut vm = Vm::new();
    let check = vm
        .eval(
            "var caught = null; \
             try { \
                new MouseEvent('x', { get clientX() { throw new Error('boom'); } }); \
             } catch (e) { caught = e.message; } caught;",
        )
        .unwrap();
    match check {
        JsValue::String(id) => assert_eq!(vm.get_string(id), "boom"),
        other => panic!("expected string, got {other:?}"),
    }
}

#[test]
fn ui_event_family_shares_event_prototype_reach() {
    // Event.prototype.preventDefault is reachable via prototype chain
    // on every UIEvent-family instance.
    for ctor in [
        "UIEvent",
        "MouseEvent",
        "KeyboardEvent",
        "FocusEvent",
        "InputEvent",
    ] {
        let src = format!("typeof new {ctor}('x').preventDefault === 'function'");
        assert!(eval_bool(&src), "{ctor}.preventDefault not reachable");
    }
}
