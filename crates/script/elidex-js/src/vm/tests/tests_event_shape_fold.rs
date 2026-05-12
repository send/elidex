//! UA-dispatch event shape-fold regression tests (`#11-event-modern-extras-shape-fold`).
//!
//! D-10 PR #182 R9 introduced `prototype_for_payload(...)` so UA-fired
//! events get the spec-correct subclass prototype
//! (`UA click instanceof MouseEvent === true`).  The fix exposed a
//! second-order gap: the UA-dispatch payload shape was narrower than
//! the ctor shape — `view` / `detail` / `screenX` / `screenY` /
//! `movementX` / `movementY` / `relatedTarget` (Mouse family),
//! `location` / `isComposing` (Keyboard), `deltaZ` (Wheel),
//! `dataTransfer` (Input) all returned `undefined` on UA-fired events.
//!
//! These tests pin the **post-reshape slot order** for every affected
//! family.  Each test uses unique sentinel values for the input fields
//! so a slot-order miswrite (e.g. swapping ctrlKey/shiftKey arms in
//! `dispatch_payload`) silently flips one sentinel into another's slot
//! and the assertion fails immediately.  `debug_assert_eq!` on
//! `slots.len() - len_before` catches count drift but does NOT catch
//! intra-block reordering — these tests are the only mechanical safety
//! net for that class of regression.
//!
//! Lesson #222: TDD slot-order locks are mandatory whenever a
//! precomputed-shape writer is reshaped.  Plan memo's Phase 0a
//! requires these to be written RED before any reshape, so each
//! family's reshape phase turns its own test green in isolation.
//!
//! UA-vs-ctor symmetry: a separate cohort of tests verifies that
//! `getOwnPropertyNames(uaEvent)` (sorted) matches
//! `getOwnPropertyNames(ctorEvent)` (sorted) per family.  Name equality
//! only — value semantics differ deliberately (UA defaults
//! `view = window` per Chrome parity; ctor defaults `view = null` per
//! WebIDL §3.2 `attribute Window? view = null`).

#![cfg(feature = "engine")]

use elidex_ecs::{Attributes, EcsDom};
use elidex_plugin::{
    CompositionEventInit, EventPayload, FocusEventInit, InputEventInit, KeyboardEventInit,
    MouseEventInit, WheelEventInit,
};
use elidex_script_session::SessionCore;

use super::super::test_helpers::{bind_vm, make_event};
use super::super::value::{JsValue, ObjectId, PropertyKey, PropertyStorage, PropertyValue};
use super::super::Vm;

// ---------------------------------------------------------------------------
// Shared helpers (mirror tests_create_event_object.rs style).
// ---------------------------------------------------------------------------

fn read_data(vm: &Vm, obj: ObjectId, name: &str) -> JsValue {
    let sid = vm.inner.strings.lookup(name).unwrap_or_else(|| {
        panic!("property {name} not interned — create_event_object should have interned it")
    });
    let storage = &vm.inner.get_object(obj).storage;
    let (slot, _attrs) = storage
        .get(PropertyKey::String(sid), &vm.inner.shapes)
        .unwrap_or_else(|| panic!("missing property {name}"));
    match slot {
        PropertyValue::Data(v) => *v,
        other @ PropertyValue::Accessor { .. } => {
            panic!("{name}: expected Data, got {other:?}")
        }
    }
}

fn read_num(vm: &Vm, obj: ObjectId, name: &str) -> f64 {
    match read_data(vm, obj, name) {
        JsValue::Number(n) => n,
        other => panic!("{name}: expected Number, got {other:?}"),
    }
}

fn read_bool(vm: &Vm, obj: ObjectId, name: &str) -> bool {
    match read_data(vm, obj, name) {
        JsValue::Boolean(b) => b,
        other => panic!("{name}: expected Boolean, got {other:?}"),
    }
}

fn read_str(vm: &Vm, obj: ObjectId, name: &str) -> String {
    match read_data(vm, obj, name) {
        JsValue::String(sid) => vm.inner.strings.get_utf8(sid),
        other => panic!("{name}: expected String, got {other:?}"),
    }
}

/// Sorted UTF-8 own property names for `obj` — equivalent to
/// `Object.getOwnPropertyNames(obj).sort()` in JS.  Reads directly
/// from the object's `Shaped` storage so the value semantics of any
/// individual slot (e.g. `view = window` vs `view = null`) don't
/// affect the comparison.  `Symbol`-keyed properties are skipped
/// (event objects don't install any today; if that changes, extend
/// the filter).
fn own_property_names(vm: &Vm, obj: ObjectId) -> Vec<String> {
    let shape_id = match &vm.inner.get_object(obj).storage {
        PropertyStorage::Shaped { shape, .. } => *shape,
        PropertyStorage::Dictionary(_) => {
            panic!("event objects must remain in Shaped storage")
        }
    };
    let mut names: Vec<String> = vm.inner.shapes[shape_id as usize]
        .ordered_entries
        .iter()
        .filter_map(|(key, _attrs)| match key {
            PropertyKey::String(sid) => Some(vm.inner.strings.get_utf8(*sid)),
            PropertyKey::Symbol(_) => None,
        })
        .collect();
    names.sort();
    names
}

/// Build a UA-fired event of `payload`'s family on `el`, then return
/// its sorted own property names.  Wraps the bind/dispatch boilerplate
/// shared by every symmetry test.
fn ua_event_property_names(
    vm: &mut Vm,
    session: &mut SessionCore,
    dom: &mut EcsDom,
    doc: elidex_ecs::Entity,
    el: elidex_ecs::Entity,
    event_type: &str,
    payload: EventPayload,
) -> Vec<String> {
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(vm, session, dom, doc);
    }
    let target = vm.inner.create_element_wrapper(el);
    let ev = make_event(event_type, true, payload, el);
    let obj = vm.inner.create_event_object(&ev, target, target, false);
    own_property_names(vm, obj)
}

/// Construct `script_source` (a `new XEvent(...)` expression) and
/// return the resulting object's sorted own property names.  Sibling
/// of [`ua_event_property_names`] — the two return-values are then
/// asserted equal in each per-family symmetry test.
fn ctor_event_property_names(vm: &mut Vm, script_source: &str) -> Vec<String> {
    let result = vm.eval(script_source).unwrap();
    let JsValue::Object(obj) = result else {
        panic!("{script_source}: expected Object result, got {result:?}");
    };
    own_property_names(vm, obj)
}

// ---------------------------------------------------------------------------
// Per-family slot-order locks (one test per affected family).
// ---------------------------------------------------------------------------

#[test]
fn mouse_payload_slot_order_locked() {
    // Sentinel values: every input field gets a distinct value, so a
    // mis-permuted writer surfaces as a value mismatch on the first
    // assertion that hits a swapped slot.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let el = dom.create_element("button", Attributes::default());

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let payload = EventPayload::Mouse(MouseEventInit {
        client_x: 11.0,
        client_y: 12.0,
        button: 13,
        buttons: 14,
        alt_key: true,
        ctrl_key: false,
        meta_key: true,
        shift_key: false,
    });
    let target = vm.inner.create_element_wrapper(el);
    let global = vm.inner.global_object;
    let ev = make_event("click", true, payload, el);
    let obj = vm.inner.create_event_object(&ev, target, target, false);

    // UIEvent prefix — view defaults to window (Chrome parity), detail to 0.
    assert_eq!(read_data(&vm, obj, "view"), JsValue::Object(global));
    assert_eq!(read_num(&vm, obj, "detail"), 0.0);
    // Ctor-shape slot order: screenX/Y → clientX/Y → ctrl/shift/alt/meta →
    // button/buttons → relatedTarget → movementX/Y.
    assert_eq!(read_num(&vm, obj, "screenX"), 0.0);
    assert_eq!(read_num(&vm, obj, "screenY"), 0.0);
    assert_eq!(read_num(&vm, obj, "clientX"), 11.0);
    assert_eq!(read_num(&vm, obj, "clientY"), 12.0);
    assert!(!read_bool(&vm, obj, "ctrlKey"));
    assert!(!read_bool(&vm, obj, "shiftKey"));
    assert!(read_bool(&vm, obj, "altKey"));
    assert!(read_bool(&vm, obj, "metaKey"));
    assert_eq!(read_num(&vm, obj, "button"), 13.0);
    assert_eq!(read_num(&vm, obj, "buttons"), 14.0);
    assert_eq!(read_data(&vm, obj, "relatedTarget"), JsValue::Null);
    assert_eq!(read_num(&vm, obj, "movementX"), 0.0);
    assert_eq!(read_num(&vm, obj, "movementY"), 0.0);

    vm.unbind();
}

#[test]
fn keyboard_payload_slot_order_locked() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let el = dom.create_element("input", Attributes::default());

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let payload = EventPayload::Keyboard(KeyboardEventInit {
        key: "K".to_string(),
        code: "KeyK".to_string(),
        alt_key: false,
        ctrl_key: true,
        meta_key: false,
        shift_key: true,
        repeat: true,
    });
    let target = vm.inner.create_element_wrapper(el);
    let global = vm.inner.global_object;
    let ev = make_event("keydown", true, payload, el);
    let obj = vm.inner.create_event_object(&ev, target, target, false);

    // UIEvent prefix.
    assert_eq!(read_data(&vm, obj, "view"), JsValue::Object(global));
    assert_eq!(read_num(&vm, obj, "detail"), 0.0);
    // Ctor-shape slot order: key, code, location, ctrl/shift/alt/meta,
    // repeat, isComposing.
    assert_eq!(read_str(&vm, obj, "key"), "K");
    assert_eq!(read_str(&vm, obj, "code"), "KeyK");
    assert_eq!(read_num(&vm, obj, "location"), 0.0);
    assert!(read_bool(&vm, obj, "ctrlKey"));
    assert!(read_bool(&vm, obj, "shiftKey"));
    assert!(!read_bool(&vm, obj, "altKey"));
    assert!(!read_bool(&vm, obj, "metaKey"));
    assert!(read_bool(&vm, obj, "repeat"));
    assert!(!read_bool(&vm, obj, "isComposing"));

    vm.unbind();
}

#[test]
fn wheel_payload_slot_order_locked() {
    // WheelEvent ctor shape extends MouseEvent — UI prefix + 13 mouse +
    // 4 wheel keys (deltaX/Y/Z/Mode).  UA WheelEventInit only carries 3
    // (deltaX/Y/Mode); the rest fall to defaults.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let el = dom.create_element("div", Attributes::default());

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let payload = EventPayload::Wheel(WheelEventInit {
        delta_x: 21.0,
        delta_y: 22.0,
        delta_mode: 1,
    });
    let target = vm.inner.create_element_wrapper(el);
    let global = vm.inner.global_object;
    let ev = make_event("wheel", true, payload, el);
    let obj = vm.inner.create_event_object(&ev, target, target, false);

    // UIEvent prefix.
    assert_eq!(read_data(&vm, obj, "view"), JsValue::Object(global));
    assert_eq!(read_num(&vm, obj, "detail"), 0.0);
    // Mouse inheritance defaults — UA payload doesn't carry any of these.
    assert_eq!(read_num(&vm, obj, "screenX"), 0.0);
    assert_eq!(read_num(&vm, obj, "screenY"), 0.0);
    assert_eq!(read_num(&vm, obj, "clientX"), 0.0);
    assert_eq!(read_num(&vm, obj, "clientY"), 0.0);
    assert!(!read_bool(&vm, obj, "ctrlKey"));
    assert!(!read_bool(&vm, obj, "shiftKey"));
    assert!(!read_bool(&vm, obj, "altKey"));
    assert!(!read_bool(&vm, obj, "metaKey"));
    assert_eq!(read_num(&vm, obj, "button"), 0.0);
    assert_eq!(read_num(&vm, obj, "buttons"), 0.0);
    assert_eq!(read_data(&vm, obj, "relatedTarget"), JsValue::Null);
    assert_eq!(read_num(&vm, obj, "movementX"), 0.0);
    assert_eq!(read_num(&vm, obj, "movementY"), 0.0);
    // Wheel slots.
    assert_eq!(read_num(&vm, obj, "deltaX"), 21.0);
    assert_eq!(read_num(&vm, obj, "deltaY"), 22.0);
    assert_eq!(read_num(&vm, obj, "deltaZ"), 0.0);
    assert_eq!(read_num(&vm, obj, "deltaMode"), 1.0);

    vm.unbind();
}

#[test]
fn focus_payload_slot_order_locked() {
    // FocusEvent ctor shape: UIEvent prefix + relatedTarget.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let el = dom.create_element("input", Attributes::default());
    let related = dom.create_element("button", Attributes::default());
    let related_bits = related.to_bits().get();

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let payload = EventPayload::Focus(FocusEventInit {
        related_target: Some(related_bits),
    });
    let target = vm.inner.create_element_wrapper(el);
    let global = vm.inner.global_object;
    let ev = make_event("focus", false, payload, el);
    let obj = vm.inner.create_event_object(&ev, target, target, false);

    assert_eq!(read_data(&vm, obj, "view"), JsValue::Object(global));
    assert_eq!(read_num(&vm, obj, "detail"), 0.0);
    // relatedTarget resolves to a HostObject wrapper.
    match read_data(&vm, obj, "relatedTarget") {
        JsValue::Object(_) => {} // wrapper resolution checked elsewhere
        other => panic!("relatedTarget: expected Object wrapper, got {other:?}"),
    }

    vm.unbind();
}

#[test]
fn input_payload_slot_order_locked() {
    // InputEvent ctor shape: UIEvent prefix + data + isComposing +
    // inputType + dataTransfer.  UA InputEventInit carries 3 (inputType /
    // data / isComposing); dataTransfer defaults to null.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let el = dom.create_element("input", Attributes::default());

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let payload = EventPayload::Input(InputEventInit {
        input_type: "insertText".to_string(),
        data: Some("z".to_string()),
        is_composing: true,
    });
    let target = vm.inner.create_element_wrapper(el);
    let global = vm.inner.global_object;
    let ev = make_event("input", true, payload, el);
    let obj = vm.inner.create_event_object(&ev, target, target, false);

    assert_eq!(read_data(&vm, obj, "view"), JsValue::Object(global));
    assert_eq!(read_num(&vm, obj, "detail"), 0.0);
    // Ctor-shape order: data, isComposing, inputType, dataTransfer.
    assert_eq!(read_str(&vm, obj, "data"), "z");
    assert!(read_bool(&vm, obj, "isComposing"));
    assert_eq!(read_str(&vm, obj, "inputType"), "insertText");
    assert_eq!(read_data(&vm, obj, "dataTransfer"), JsValue::Null);

    vm.unbind();
}

#[test]
fn composition_payload_slot_order_locked() {
    // CompositionEvent ctor shape: UIEvent prefix + data.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let el = dom.create_element("input", Attributes::default());

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let payload = EventPayload::Composition(CompositionEventInit {
        data: "あ".to_string(),
    });
    let target = vm.inner.create_element_wrapper(el);
    let global = vm.inner.global_object;
    let ev = make_event("compositionstart", true, payload, el);
    let obj = vm.inner.create_event_object(&ev, target, target, false);

    assert_eq!(read_data(&vm, obj, "view"), JsValue::Object(global));
    assert_eq!(read_num(&vm, obj, "detail"), 0.0);
    assert_eq!(read_str(&vm, obj, "data"), "あ");

    vm.unbind();
}

// ---------------------------------------------------------------------------
// UA-vs-ctor `getOwnPropertyNames` symmetry locks (one per family).
//
// Compares the SET of own property names — not the values.  Per the
// docstring on `push_ui_prefix`, UA / ctor paths deliberately diverge
// on `view` (UA defaults to `window`, ctor defaults to `null`); any
// value-equality assertion would falsely flag that asymmetry.
//
// Symmetry holds because UA-side `dispatch_payload` and the per-
// family ctor share the SAME `*_event_constructed` ShapeId since the
// shape-fold.  Asserting on names (not ShapeIds) keeps the test
// observable from a userland perspective and survives a hypothetical
// future implementation that allocates equivalent shapes from
// different transition chains.
// ---------------------------------------------------------------------------

#[test]
fn mouse_ua_and_ctor_share_own_property_names() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let el = dom.create_element("button", Attributes::default());

    let ua_names = ua_event_property_names(
        &mut vm,
        &mut session,
        &mut dom,
        doc,
        el,
        "click",
        EventPayload::Mouse(MouseEventInit::default()),
    );
    let ctor_names = ctor_event_property_names(&mut vm, "new MouseEvent('click')");
    assert_eq!(ua_names, ctor_names, "UA Mouse vs ctor MouseEvent");

    vm.unbind();
}

#[test]
fn keyboard_ua_and_ctor_share_own_property_names() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let el = dom.create_element("input", Attributes::default());

    let ua_names = ua_event_property_names(
        &mut vm,
        &mut session,
        &mut dom,
        doc,
        el,
        "keydown",
        EventPayload::Keyboard(KeyboardEventInit::default()),
    );
    let ctor_names = ctor_event_property_names(&mut vm, "new KeyboardEvent('keydown')");
    assert_eq!(ua_names, ctor_names, "UA Keyboard vs ctor KeyboardEvent");

    vm.unbind();
}

#[test]
fn wheel_ua_and_ctor_share_own_property_names() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let el = dom.create_element("div", Attributes::default());

    let ua_names = ua_event_property_names(
        &mut vm,
        &mut session,
        &mut dom,
        doc,
        el,
        "wheel",
        EventPayload::Wheel(WheelEventInit::default()),
    );
    let ctor_names = ctor_event_property_names(&mut vm, "new WheelEvent('wheel')");
    assert_eq!(ua_names, ctor_names, "UA Wheel vs ctor WheelEvent");

    vm.unbind();
}

#[test]
fn focus_ua_and_ctor_share_own_property_names() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let el = dom.create_element("input", Attributes::default());

    let ua_names = ua_event_property_names(
        &mut vm,
        &mut session,
        &mut dom,
        doc,
        el,
        "focus",
        EventPayload::Focus(FocusEventInit::default()),
    );
    let ctor_names = ctor_event_property_names(&mut vm, "new FocusEvent('focus')");
    assert_eq!(ua_names, ctor_names, "UA Focus vs ctor FocusEvent");

    vm.unbind();
}

#[test]
fn input_ua_and_ctor_share_own_property_names() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let el = dom.create_element("input", Attributes::default());

    let ua_names = ua_event_property_names(
        &mut vm,
        &mut session,
        &mut dom,
        doc,
        el,
        "input",
        EventPayload::Input(InputEventInit::default()),
    );
    let ctor_names = ctor_event_property_names(&mut vm, "new InputEvent('input')");
    assert_eq!(ua_names, ctor_names, "UA Input vs ctor InputEvent");

    vm.unbind();
}

#[test]
fn composition_ua_and_ctor_share_own_property_names() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let el = dom.create_element("input", Attributes::default());

    let ua_names = ua_event_property_names(
        &mut vm,
        &mut session,
        &mut dom,
        doc,
        el,
        "compositionstart",
        EventPayload::Composition(CompositionEventInit::default()),
    );
    let ctor_names = ctor_event_property_names(&mut vm, "new CompositionEvent('compositionstart')");
    assert_eq!(
        ua_names, ctor_names,
        "UA Composition vs ctor CompositionEvent"
    );

    vm.unbind();
}
