//! PR4b C2: `globalThis` / `window` host-object tests.
//!
//! The global object is a `HostObject` whose `entity_bits` point at a
//! dedicated Window ECS entity (distinct from the Document).  `Vm::bind`
//! allocates the entity lazily, retains it across unbind cycles, and
//! resets `entity_bits` back to `0` on `Vm::unbind` so post-unbind
//! `window.*` accesses silently no-op rather than panicking.

#![cfg(feature = "engine")]

use elidex_ecs::{EcsDom, NodeKind};
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::ObjectKind;
use super::super::Vm;

#[test]
fn global_object_is_host_object() {
    // Before any bind, `globalThis` is already a `HostObject` with the
    // sentinel `entity_bits = 0` so `entity_from_this` rejects access.
    let vm = Vm::new();
    match vm.inner.get_object(vm.inner.global_object).kind {
        ObjectKind::HostObject { entity_bits } => {
            assert_eq!(entity_bits, 0, "pre-bind globalThis must use sentinel");
        }
        _ => panic!("globalThis must be a HostObject"),
    }
}

#[test]
fn global_object_inherits_window_prototype_chain() {
    let vm = Vm::new();
    // globalThis → Window.prototype → EventTarget.prototype → Object.prototype.
    let g_proto = vm.inner.get_object(vm.inner.global_object).prototype;
    assert_eq!(
        g_proto, vm.inner.window_prototype,
        "globalThis.prototype must be Window.prototype"
    );
    let win_proto = vm.inner.window_prototype.unwrap();
    assert_eq!(
        vm.inner.get_object(win_proto).prototype,
        vm.inner.event_target_prototype,
        "Window.prototype.prototype must be EventTarget.prototype"
    );
    let et_proto = vm.inner.event_target_prototype.unwrap();
    assert_eq!(
        vm.inner.get_object(et_proto).prototype,
        vm.inner.object_prototype,
        "EventTarget.prototype.prototype must be Object.prototype"
    );
}

#[test]
fn bind_allocates_window_entity_and_threads_entity_bits() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();

    // SAFETY: `session` / `dom` are uniquely owned; we call `unbind`
    // before either is dropped.
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    // Window entity is allocated during bind.
    let win_entity = vm
        .host_data()
        .expect("HostData installed by bind_vm")
        .window_entity()
        .expect("bind must allocate a Window entity");

    // It is *not* the document entity.
    assert_ne!(win_entity, doc);

    // It carries only `NodeKind::Window` (no tree relation).
    let dom_ref = vm.host_data().unwrap().dom();
    assert_eq!(dom_ref.node_kind(win_entity), Some(NodeKind::Window));

    // globalThis.entity_bits == window_entity.to_bits().
    let ObjectKind::HostObject { entity_bits } = vm.inner.get_object(vm.inner.global_object).kind
    else {
        unreachable!()
    };
    assert_eq!(entity_bits, win_entity.to_bits().get());

    vm.unbind();
}

#[test]
fn unbind_resets_global_entity_bits_to_sentinel() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.unbind();

    let ObjectKind::HostObject { entity_bits } = vm.inner.get_object(vm.inner.global_object).kind
    else {
        unreachable!()
    };
    assert_eq!(entity_bits, 0, "unbind must reset to sentinel");
}

#[test]
fn window_entity_identity_is_stable_across_rebinds() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let first = vm.host_data().unwrap().window_entity().unwrap();
    vm.unbind();

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let second = vm.host_data().unwrap().window_entity().unwrap();
    assert_eq!(first, second, "rebind must reuse the Window entity");
    vm.unbind();
}

#[test]
fn globalthis_still_works_as_property_bag() {
    // Changing `globalThis` to a HostObject must not break the
    // existing `globalThis.foo = …` pattern (see `dispatch_objects.rs`
    // / `ops_property.rs` — they key on ObjectId, not ObjectKind).
    let mut vm = Vm::new();
    let v = vm.eval("globalThis.testGlobal = 42; testGlobal;").unwrap();
    match v {
        super::super::value::JsValue::Number(n) => assert_eq!(n, 42.0),
        other => panic!("unexpected: {other:?}"),
    }
}

// -- PR4b C8: window self-ref, viewport, scroll --------------------------

#[test]
fn window_is_self_reference_to_global_this() {
    let mut vm = Vm::new();
    // Identity: `window === globalThis` must hold (WHATWG HTML §7.2.4).
    let v = vm.eval("window === globalThis;").unwrap();
    match v {
        super::super::value::JsValue::Boolean(b) => {
            assert!(b, "window must be globalThis");
        }
        other => panic!("expected bool, got {other:?}"),
    }
}

#[test]
fn window_inner_width_and_height_default() {
    let mut vm = Vm::new();
    let v = vm
        .eval("window.innerWidth + ':' + window.innerHeight;")
        .unwrap();
    let super::super::value::JsValue::String(id) = v else {
        panic!("expected string");
    };
    assert_eq!(vm.get_string(id), "1024:768");
}

#[test]
fn window_scroll_x_and_y_default_to_zero() {
    let mut vm = Vm::new();
    let v = vm
        .eval("window.scrollX === 0 && window.scrollY === 0;")
        .unwrap();
    match v {
        super::super::value::JsValue::Boolean(b) => assert!(b),
        _ => panic!(),
    }
}

#[test]
fn window_scroll_to_updates_state() {
    let mut vm = Vm::new();
    vm.eval("window.scrollTo(50, 100);").unwrap();
    assert_eq!(vm.inner.viewport.scroll_x, 50.0);
    assert_eq!(vm.inner.viewport.scroll_y, 100.0);
}

#[test]
fn window_scroll_by_adds_delta() {
    let mut vm = Vm::new();
    vm.eval(
        "window.scrollTo(10, 20);
         window.scrollBy(5, 7);",
    )
    .unwrap();
    assert_eq!(vm.inner.viewport.scroll_x, 15.0);
    assert_eq!(vm.inner.viewport.scroll_y, 27.0);
}

#[test]
fn page_offset_aliases_scroll_xy() {
    let mut vm = Vm::new();
    vm.eval("window.scrollTo(42, 99);").unwrap();
    let v = vm
        .eval("window.pageXOffset === 42 && window.pageYOffset === 99;")
        .unwrap();
    match v {
        super::super::value::JsValue::Boolean(b) => assert!(b),
        _ => panic!(),
    }
}

#[test]
fn device_pixel_ratio_is_one() {
    let mut vm = Vm::new();
    let v = vm.eval("window.devicePixelRatio;").unwrap();
    match v {
        super::super::value::JsValue::Number(n) => assert_eq!(n, 1.0),
        _ => panic!(),
    }
}

// -- WHATWG separation: window listeners go to window entity -------------

#[test]
fn window_add_event_listener_targets_window_entity_not_document() {
    use super::super::test_helpers::{bind_vm, listeners_on};

    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    vm.eval(
        "globalThis.h = function () {};
         window.addEventListener('resize', globalThis.h);",
    )
    .unwrap();

    let window_entity = vm.host_data().unwrap().window_entity().unwrap();
    // Listener landed on the window entity.
    let win_count = listeners_on(&mut vm, window_entity)
        .matching_all("resize")
        .len();
    assert_eq!(win_count, 1, "window listener must land on window entity");

    // And *not* on the document entity (this was the bug that the
    // separate window entity guards against — PR3 C9 design note).
    let doc_count = listeners_on(&mut vm, doc).matching_all("resize").len();
    assert_eq!(doc_count, 0, "document must not see window's listener");

    vm.unbind();
}
