//! PR3 C4: `create_event_object` + payload setter tests.
//!
//! Exercises the full property+method+accessor installation path by
//! constructing real `DispatchEvent`s (with various payload variants)
//! and reading back individual properties via the VM's property ops.
//!
//! Compiled only under the `engine` feature — `DispatchEvent` and
//! `HostData` binding live there.

#![cfg(feature = "engine")]

use elidex_ecs::{Attributes, EcsDom, Entity};
use elidex_plugin::{EventPayload, EventPhase, FocusEventInit, KeyboardEventInit, MouseEventInit};
use elidex_script_session::event_dispatch::DispatchEvent;
use elidex_script_session::SessionCore;

use super::super::host_data::HostData;
use super::super::value::{
    JsValue, NativeContext, ObjectId, ObjectKind, PropertyKey, PropertyValue,
};
use super::super::Vm;

/// Thin wrapper around `Vm::bind` to make call sites short.
#[allow(unsafe_code)]
unsafe fn bind_vm(vm: &mut Vm, session: &mut SessionCore, dom: &mut EcsDom, document: Entity) {
    vm.install_host_data(HostData::new());
    unsafe {
        vm.bind(session as *mut _, dom as *mut _, document);
    }
}

/// Build a minimal DispatchEvent with the given type/payload and
/// `target`/`current_target` set to `entity`.  Overrides the
/// constructor defaults to match event-object read-back expectations
/// (bubbles=false, phase=AtTarget, cancelable explicit).
fn make_event(
    event_type: &str,
    cancelable: bool,
    payload: EventPayload,
    entity: Entity,
) -> DispatchEvent {
    let mut ev = DispatchEvent::new(event_type, entity);
    ev.bubbles = false;
    ev.cancelable = cancelable;
    ev.payload = payload;
    ev.phase = EventPhase::AtTarget;
    ev.current_target = Some(entity);
    ev.dispatch_flag = true;
    ev
}

/// Read a named own property off an object and assert it is a Data
/// property with the expected `JsValue`.
fn expect_data(vm: &Vm, obj: ObjectId, name: &str) -> JsValue {
    let sid = vm.inner.strings.lookup(name).unwrap_or_else(|| {
        panic!("property {name} not interned — create_event_object should have interned it")
    });
    let storage = &vm.inner.get_object(obj).storage;
    let (slot, _attrs) = storage
        .get(PropertyKey::String(sid), &vm.inner.shapes)
        .unwrap_or_else(|| panic!("missing property {name}"));
    match slot {
        PropertyValue::Data(v) => *v,
        other => panic!("{name}: expected Data, got {other:?}"),
    }
}

fn js_string_eq(vm: &Vm, value: JsValue, expected: &str) -> bool {
    match value {
        JsValue::String(sid) => vm.inner.strings.get_utf8(sid) == expected,
        _ => false,
    }
}

#[test]
fn core_properties_installed_and_typed() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let el = dom.create_element("button", Attributes::default());

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let target = vm.inner.create_element_wrapper(el);
    let current = target;
    let ev = make_event("click", true, EventPayload::None, el);
    let obj = vm.inner.create_event_object(&ev, target, current, false);

    assert!(js_string_eq(&vm, expect_data(&vm, obj, "type"), "click"));
    assert_eq!(expect_data(&vm, obj, "bubbles"), JsValue::Boolean(false));
    assert_eq!(expect_data(&vm, obj, "cancelable"), JsValue::Boolean(true));
    assert_eq!(expect_data(&vm, obj, "eventPhase"), JsValue::Number(2.0));
    assert_eq!(expect_data(&vm, obj, "target"), JsValue::Object(target));
    assert_eq!(
        expect_data(&vm, obj, "currentTarget"),
        JsValue::Object(current)
    );
    assert_eq!(expect_data(&vm, obj, "timeStamp"), JsValue::Number(0.0));
    assert_eq!(expect_data(&vm, obj, "composed"), JsValue::Boolean(false));
    assert_eq!(expect_data(&vm, obj, "isTrusted"), JsValue::Boolean(true));

    vm.unbind();
}

#[test]
fn default_prevented_is_accessor_and_reflects_flag_live() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let el = dom.create_element("a", Attributes::default());

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let target = vm.inner.create_element_wrapper(el);
    let ev = make_event("click", true, EventPayload::None, el);
    let obj = vm.inner.create_event_object(&ev, target, target, false);

    // `defaultPrevented` lives on `event_methods_prototype` (PR3
    // simplify pass): the accessor is shared across every event
    // instance so we look it up on the prototype rather than as an
    // own property.
    let proto_id = vm
        .inner
        .get_object(obj)
        .prototype
        .expect("event must have prototype = event_methods_prototype");
    let dp_sid = vm.inner.strings.lookup("defaultPrevented").unwrap();
    let (slot, attrs) = vm
        .inner
        .get_object(proto_id)
        .storage
        .get(PropertyKey::String(dp_sid), &vm.inner.shapes)
        .unwrap();
    assert!(attrs.is_accessor, "defaultPrevented must be an accessor");
    let getter_id = match slot {
        PropertyValue::Accessor {
            getter: Some(g), ..
        } => *g,
        other => panic!("defaultPrevented: expected accessor with getter, got {other:?}"),
    };

    // Pre-mutation: getter returns false.
    {
        let mut ctx = NativeContext { vm: &mut vm.inner };
        let got = match &ctx.vm.get_object(getter_id).kind {
            ObjectKind::NativeFunction(nf) => {
                (nf.func)(&mut ctx, JsValue::Object(obj), &[]).unwrap()
            }
            _ => panic!("getter must be NativeFunction"),
        };
        assert_eq!(got, JsValue::Boolean(false));
    }

    // Flip the internal-slot flag.
    if let ObjectKind::Event {
        ref mut default_prevented,
        ..
    } = &mut vm.inner.get_object_mut(obj).kind
    {
        *default_prevented = true;
    }

    // Post-mutation: getter returns true (live, not stale).
    {
        let mut ctx = NativeContext { vm: &mut vm.inner };
        let got = match &ctx.vm.get_object(getter_id).kind {
            ObjectKind::NativeFunction(nf) => {
                (nf.func)(&mut ctx, JsValue::Object(obj), &[]).unwrap()
            }
            _ => panic!("getter must be NativeFunction"),
        };
        assert_eq!(got, JsValue::Boolean(true));
    }

    vm.unbind();
}

#[test]
fn four_methods_installed_on_event_methods_prototype() {
    // Methods live on the shared `event_methods_prototype` intrinsic
    // (PR3 simplify pass: avoids 4 native-fn allocs + 4 shape
    // transitions per event).  Verify they're reachable via prototype
    // chain by inspecting the prototype's own properties directly.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let el = dom.create_element("div", Attributes::default());

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let target = vm.inner.create_element_wrapper(el);
    let ev = make_event("click", true, EventPayload::None, el);
    let obj = vm.inner.create_event_object(&ev, target, target, false);
    let proto_id = vm.inner.get_object(obj).prototype.expect("must have proto");

    for name in [
        "preventDefault",
        "stopPropagation",
        "stopImmediatePropagation",
        "composedPath",
    ] {
        let sid = vm.inner.strings.lookup(name).unwrap();
        let (slot, _) = vm
            .inner
            .get_object(proto_id)
            .storage
            .get(PropertyKey::String(sid), &vm.inner.shapes)
            .unwrap_or_else(|| panic!("{name} missing from event_methods_prototype"));
        let PropertyValue::Data(JsValue::Object(fn_id)) = slot else {
            panic!("{name}: expected Data(Object), got {slot:?}");
        };
        assert!(
            matches!(
                vm.inner.get_object(*fn_id).kind,
                ObjectKind::NativeFunction(_)
            ),
            "{name}: expected NativeFunction"
        );
    }

    vm.unbind();
}

#[test]
fn mouse_payload_installs_coords_and_modifier_keys() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let el = dom.create_element("div", Attributes::default());

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let payload = EventPayload::Mouse(MouseEventInit {
        client_x: 42.0,
        client_y: 17.0,
        button: 2,
        buttons: 4,
        alt_key: true,
        ctrl_key: false,
        meta_key: false,
        shift_key: true,
    });
    let target = vm.inner.create_element_wrapper(el);
    let ev = make_event("click", true, payload, el);
    let obj = vm.inner.create_event_object(&ev, target, target, false);

    assert_eq!(expect_data(&vm, obj, "clientX"), JsValue::Number(42.0));
    assert_eq!(expect_data(&vm, obj, "clientY"), JsValue::Number(17.0));
    assert_eq!(expect_data(&vm, obj, "button"), JsValue::Number(2.0));
    assert_eq!(expect_data(&vm, obj, "buttons"), JsValue::Number(4.0));
    assert_eq!(expect_data(&vm, obj, "altKey"), JsValue::Boolean(true));
    assert_eq!(expect_data(&vm, obj, "ctrlKey"), JsValue::Boolean(false));
    assert_eq!(expect_data(&vm, obj, "metaKey"), JsValue::Boolean(false));
    assert_eq!(expect_data(&vm, obj, "shiftKey"), JsValue::Boolean(true));

    vm.unbind();
}

#[test]
fn keyboard_payload_installs_key_code_and_repeat() {
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
        key: "Enter".to_string(),
        code: "Enter".to_string(),
        alt_key: false,
        ctrl_key: true,
        meta_key: false,
        shift_key: false,
        repeat: true,
    });
    let target = vm.inner.create_element_wrapper(el);
    let ev = make_event("keydown", true, payload, el);
    let obj = vm.inner.create_event_object(&ev, target, target, false);

    assert!(js_string_eq(&vm, expect_data(&vm, obj, "key"), "Enter"));
    assert!(js_string_eq(&vm, expect_data(&vm, obj, "code"), "Enter"));
    assert_eq!(expect_data(&vm, obj, "ctrlKey"), JsValue::Boolean(true));
    assert_eq!(expect_data(&vm, obj, "repeat"), JsValue::Boolean(true));

    vm.unbind();
}

#[test]
fn focus_payload_resolves_related_target_to_wrapper() {
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

    let target = vm.inner.create_element_wrapper(el);
    let payload = EventPayload::Focus(FocusEventInit {
        related_target: Some(related_bits),
    });
    let ev = make_event("focus", false, payload, el);
    let obj = vm.inner.create_event_object(&ev, target, target, false);

    let related_val = expect_data(&vm, obj, "relatedTarget");
    let JsValue::Object(related_wrapper_id) = related_val else {
        panic!("relatedTarget must resolve to a HostObject wrapper, got {related_val:?}");
    };
    assert!(
        matches!(
            vm.inner.get_object(related_wrapper_id).kind,
            ObjectKind::HostObject { .. }
        ),
        "relatedTarget wrapper must be a HostObject"
    );

    // Absent relatedTarget → null.
    let ev2 = make_event(
        "focus",
        false,
        EventPayload::Focus(FocusEventInit::default()),
        el,
    );
    let obj2 = vm.inner.create_event_object(&ev2, target, target, false);
    assert_eq!(expect_data(&vm, obj2, "relatedTarget"), JsValue::Null);

    vm.unbind();
}

#[test]
fn event_object_kind_carries_flag_seed() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let el = dom.create_element("div", Attributes::default());

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let target = vm.inner.create_element_wrapper(el);
    let mut ev = make_event("click", true, EventPayload::None, el);
    // Seed the incoming DispatchFlags with a preset value — e.g. a
    // prior listener already called preventDefault — and verify the
    // new event object's internal slot reflects it.
    ev.flags.default_prevented = true;
    let obj = vm
        .inner
        .create_event_object(&ev, target, target, /* passive */ true);

    let ObjectKind::Event {
        default_prevented,
        passive,
        cancelable,
        ..
    } = vm.inner.get_object(obj).kind
    else {
        panic!("expected Event variant");
    };
    assert!(default_prevented, "flag carried over from DispatchFlags");
    assert!(passive, "passive propagated from argument");
    assert!(cancelable, "cancelable copied from DispatchEvent");

    vm.unbind();
}
