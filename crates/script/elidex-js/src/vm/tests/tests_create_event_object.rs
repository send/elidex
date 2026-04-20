//! PR3 C4: `create_event_object` + payload setter tests.
//!
//! Exercises the full property+method+accessor installation path by
//! constructing real `DispatchEvent`s (with various payload variants)
//! and reading back individual properties via the VM's property ops.
//!
//! Compiled only under the `engine` feature — `DispatchEvent` and
//! `HostData` binding live there.

#![cfg(feature = "engine")]

use elidex_ecs::{Attributes, EcsDom};
use elidex_plugin::{EventPayload, FocusEventInit, KeyboardEventInit, MouseEventInit};
use elidex_script_session::SessionCore;

use super::super::test_helpers::{bind_vm, make_event};
use super::super::value::{
    JsValue, NativeContext, ObjectId, ObjectKind, PropertyKey, PropertyStorage, PropertyValue,
};
use super::super::Vm;

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
    let ts = match expect_data(&vm, obj, "timeStamp") {
        JsValue::Number(n) => n,
        other => panic!("timeStamp: expected Number, got {other:?}"),
    };
    assert!(
        ts.is_finite() && ts >= 0.0,
        "timeStamp must be a finite non-negative number, got {ts}"
    );
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

    // `defaultPrevented` lives on `Event.prototype`: the accessor
    // is shared across every event instance so we look it up on the
    // prototype rather than as an own property.
    let proto_id = vm
        .inner
        .get_object(obj)
        .prototype
        .expect("event must have prototype = Event.prototype");
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
fn four_methods_installed_on_event_prototype() {
    // Methods live on the shared `Event.prototype` (PR3 simplify pass:
    // avoids 4 native-fn allocs + 4 shape transitions per event).
    // Verify they're reachable via prototype chain by inspecting the
    // prototype's own properties directly.
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
            .unwrap_or_else(|| panic!("{name} missing from Event.prototype"));
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

#[test]
fn timestamp_is_monotonic_and_shares_origin_with_performance_now() {
    // PR4d C1: `Event.timeStamp` must use the same `start_instant`
    // clock as `performance.now()` (HR-Time §5: identical time origin
    // means values inside the same listener body are directly
    // comparable).  Two back-to-back events read non-decreasing
    // values, and a `performance.now()` reading sandwiched between
    // them lies in the same range.
    //
    // The interleaved `performance.now()` reading goes through
    // `vm.eval("performance.now()")` rather than reading
    // `start_instant` in Rust directly — calling the JS-visible
    // surface is what actually verifies the two clocks share an
    // origin.  Reading `start_instant` would pass even if a future
    // refactor wired `performance.now()` to a different `Instant`.
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

    let obj1 = vm.inner.create_event_object(&ev, target, target, false);
    let now_ms = match vm.eval("performance.now();").unwrap() {
        JsValue::Number(n) => n,
        other => panic!("performance.now() returned {other:?}"),
    };
    let obj2 = vm.inner.create_event_object(&ev, target, target, false);

    let JsValue::Number(ts1) = expect_data(&vm, obj1, "timeStamp") else {
        unreachable!()
    };
    let JsValue::Number(ts2) = expect_data(&vm, obj2, "timeStamp") else {
        unreachable!()
    };
    assert!(ts1 >= 0.0 && ts1.is_finite(), "ts1 = {ts1}");
    assert!(ts2 >= ts1, "non-monotonic: ts1={ts1} ts2={ts2}");
    // The JS-side performance.now() reading must fall inside the
    // event timestamps' span — proves both surfaces consult the
    // same monotonic clock origin.
    assert!(
        now_ms >= ts1 && now_ms <= ts2 + 1e-3,
        "performance.now()={now_ms} not within [ts1={ts1}, ts2={ts2}]"
    );

    vm.unbind();
}

// ---------------------------------------------------------------------
// Precomputed shape sharing.
//
// `create_event_object` allocates at the terminal shape for the
// payload variant; two events with the same variant must share the
// same `ShapeId` so that the hidden-class fast path (PIC etc.) sees
// them as a single type.  A separate variant must land at a different
// ShapeId — cross-type shape sharing would cause hidden-class
// polymorphism and defeat the precomputed-shape optimisation.
// ---------------------------------------------------------------------

fn shape_of(vm: &Vm, obj: ObjectId) -> u32 {
    match &vm.inner.get_object(obj).storage {
        PropertyStorage::Shaped { shape, .. } => *shape,
        PropertyStorage::Dictionary(_) => {
            panic!("event objects must remain in Shaped storage mode")
        }
    }
}

#[test]
fn two_mouse_events_share_one_shape() {
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
        client_x: 1.0,
        client_y: 2.0,
        button: 0,
        buttons: 0,
        alt_key: false,
        ctrl_key: false,
        meta_key: false,
        shift_key: false,
    });
    let target = vm.inner.create_element_wrapper(el);
    let ev1 = make_event("click", true, payload.clone(), el);
    let ev2 = make_event("click", true, payload, el);

    let obj1 = vm.inner.create_event_object(&ev1, target, target, false);
    let obj2 = vm.inner.create_event_object(&ev2, target, target, false);

    assert_eq!(
        shape_of(&vm, obj1),
        shape_of(&vm, obj2),
        "two Mouse events must share the precomputed terminal ShapeId",
    );

    vm.unbind();
}

#[test]
fn different_payload_variants_use_different_shapes() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let el = dom.create_element("input", Attributes::default());

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let mouse_payload = EventPayload::Mouse(MouseEventInit::default());
    let kbd_payload = EventPayload::Keyboard(KeyboardEventInit {
        key: "a".to_string(),
        code: "KeyA".to_string(),
        alt_key: false,
        ctrl_key: false,
        meta_key: false,
        shift_key: false,
        repeat: false,
    });
    let none_payload = EventPayload::None;

    let target = vm.inner.create_element_wrapper(el);
    let ev_m = make_event("click", true, mouse_payload, el);
    let ev_k = make_event("keydown", true, kbd_payload, el);
    let ev_n = make_event("load", false, none_payload, el);

    let obj_m = vm.inner.create_event_object(&ev_m, target, target, false);
    let obj_k = vm.inner.create_event_object(&ev_k, target, target, false);
    let obj_n = vm.inner.create_event_object(&ev_n, target, target, false);

    let sm = shape_of(&vm, obj_m);
    let sk = shape_of(&vm, obj_k);
    let sn = shape_of(&vm, obj_n);
    assert_ne!(sm, sk, "Mouse and Keyboard must have distinct shapes");
    assert_ne!(sm, sn, "Mouse and None must have distinct shapes");
    assert_ne!(sk, sn, "Keyboard and None must have distinct shapes");

    vm.unbind();
}

#[test]
fn scroll_and_none_share_core_shape() {
    // Both variants have zero payload keys; they terminate at the
    // shared `core` shape per `PrecomputedEventShapes::shape_for`.
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
    let ev_scroll = make_event("scroll", false, EventPayload::Scroll, el);
    let ev_none = make_event("load", false, EventPayload::None, el);

    let obj_s = vm
        .inner
        .create_event_object(&ev_scroll, target, target, false);
    let obj_n = vm
        .inner
        .create_event_object(&ev_none, target, target, false);

    assert_eq!(
        shape_of(&vm, obj_s),
        shape_of(&vm, obj_n),
        "Scroll and None must share the core-only shape",
    );

    vm.unbind();
}
