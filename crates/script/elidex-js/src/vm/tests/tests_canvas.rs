//! Slot `#11-canvas-2d-vm` — `HTMLCanvasElement.getContext('2d')` +
//! `CanvasRenderingContext2D` + `ImageData` coverage (HTML §4.12.5).

#![cfg(feature = "engine")]

use elidex_ecs::{EcsDom, Entity, ImageData};
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::{JsValue, ObjectKind};
use super::super::Vm;

fn build_doc(dom: &mut EcsDom) -> Entity {
    let doc = dom.create_document_root();
    let html = dom.create_element("html", elidex_ecs::Attributes::default());
    let body = dom.create_element("body", elidex_ecs::Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, body));
    doc
}

/// Bind a VM to a fresh document and run `f` with access to both the VM and the
/// owning `EcsDom` (so tests can inspect the `ImageData` component after a
/// draw + `sync_dirty_canvases`).
fn with_vm<R>(f: impl FnOnce(&mut Vm, &mut EcsDom) -> R) -> R {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let r = f(&mut vm, &mut dom);
    vm.unbind();
    r
}

/// Resolve the canvas `Entity` behind a `HostObject` wrapper value (the canvas
/// element returned as an eval's completion value).
fn entity_of(vm: &Vm, value: JsValue) -> Entity {
    let JsValue::Object(id) = value else {
        panic!("value is not an object: {value:?}")
    };
    let ObjectKind::HostObject { entity_bits } = vm.inner.get_object(id).kind else {
        panic!("value is not a HostObject")
    };
    Entity::from_bits(entity_bits).expect("valid entity bits")
}

fn eval_bool(vm: &mut Vm, src: &str) -> bool {
    match vm.eval(src).unwrap() {
        JsValue::Boolean(b) => b,
        other => panic!("expected bool, got {other:?}"),
    }
}

fn eval_string(vm: &mut Vm, src: &str) -> String {
    match vm.eval(src).unwrap() {
        JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string, got {other:?}"),
    }
}

#[test]
fn get_context_2d_same_object() {
    with_vm(|vm, _dom| {
        assert!(eval_bool(
            vm,
            "var c = document.createElement('canvas'); \
             c.getContext('2d') === c.getContext('2d');"
        ));
    });
}

#[test]
fn get_context_unknown_type_returns_null() {
    with_vm(|vm, _dom| {
        assert!(eval_bool(
            vm,
            "document.createElement('canvas').getContext('webgl') === null;"
        ));
    });
}

#[test]
fn context_instanceof_and_canvas_back_ref() {
    with_vm(|vm, _dom| {
        assert!(eval_bool(
            vm,
            "var c = document.createElement('canvas'); var ctx = c.getContext('2d'); \
             (ctx instanceof CanvasRenderingContext2D) && ctx.canvas === c;"
        ));
    });
}

#[test]
fn context_wrapper_rejected_as_node() {
    // Reverse half of the bidirectional brand: a context wrapper shares its
    // canvas entity (a node) but must not be graftable into the DOM tree.
    with_vm(|vm, _dom| {
        let err = vm.eval(
            "var c = document.createElement('canvas'); \
             document.body.appendChild(c.getContext('2d'));",
        );
        assert!(err.is_err(), "appendChild(ctx) must throw, got {err:?}");
    });
}

#[test]
fn canvas_element_rejected_as_context() {
    // Forward half: a plain canvas-element wrapper is not the interned context
    // wrapper, so a context method invoked on it brand-fails.
    with_vm(|vm, _dom| {
        let err = vm.eval(
            "var c = document.createElement('canvas'); c.getContext('2d'); \
             CanvasRenderingContext2D.prototype.fillRect.call(c, 0, 0, 1, 1);",
        );
        assert!(
            err.is_err(),
            "fillRect on the element must throw, got {err:?}"
        );
    });
}

#[test]
fn fill_style_round_trip() {
    with_vm(|vm, _dom| {
        let out = eval_string(
            vm,
            "var ctx = document.createElement('canvas').getContext('2d'); \
             ctx.fillStyle = 'red'; ctx.fillStyle;",
        );
        assert_eq!(out, "#ff0000");
    });
}

#[test]
fn line_width_and_global_alpha_round_trip() {
    with_vm(|vm, _dom| {
        assert!(eval_bool(
            vm,
            "var ctx = document.createElement('canvas').getContext('2d'); \
             ctx.lineWidth = 4; ctx.globalAlpha = 0.5; \
             ctx.lineWidth === 4 && ctx.globalAlpha === 0.5;"
        ));
    });
}

#[test]
fn draw_then_sync_writes_image_data() {
    with_vm(|vm, dom| {
        let canvas = vm
            .eval(
                "var c = document.createElement('canvas'); c.width = 4; c.height = 3; \
                 var ctx = c.getContext('2d'); ctx.fillStyle = 'rgb(0, 0, 255)'; \
                 ctx.fillRect(0, 0, 4, 3); c",
            )
            .unwrap();
        let entity = entity_of(vm, canvas);
        // No ImageData until the per-frame sync runs.
        assert!(dom.world().get::<&ImageData>(entity).is_err());
        vm.sync_dirty_canvases();
        let img = dom
            .world()
            .get::<&ImageData>(entity)
            .expect("ImageData synced");
        assert_eq!((img.width, img.height), (4, 3));
        assert_eq!(img.pixels.len(), 4 * 3 * 4);
        assert_eq!(&img.pixels[0..4], &[0, 0, 255, 255]);
    });
}

#[test]
fn width_attribute_change_resets_bitmap() {
    with_vm(|vm, dom| {
        // Draw on a 4×2 canvas, then resize via the `width` IDL setter — which
        // routes through the `set_attribute` chokepoint, so the `CanvasReconciler`
        // (AttributeChange SoT) clears the bitmap to transparent black + re-marks
        // it dirty.
        let canvas = vm
            .eval(
                "var c = document.createElement('canvas'); c.width = 4; c.height = 2; \
                 var ctx = c.getContext('2d'); ctx.fillStyle = 'red'; \
                 ctx.fillRect(0, 0, 4, 2); c.width = 8; c",
            )
            .unwrap();
        let entity = entity_of(vm, canvas);
        vm.sync_dirty_canvases();
        let img = dom
            .world()
            .get::<&ImageData>(entity)
            .expect("ImageData synced");
        assert_eq!((img.width, img.height), (8, 2));
        assert!(
            img.pixels.iter().all(|&b| b == 0),
            "bitmap reset to transparent black on width change"
        );
    });
}

#[test]
fn image_data_constructable() {
    with_vm(|vm, _dom| {
        assert!(eval_bool(
            vm,
            "var d = new ImageData(2, 3); \
             d.width === 2 && d.height === 3 && d.data.length === 24 && \
             (d.data instanceof Uint8ClampedArray);"
        ));
    });
}

#[test]
fn image_data_from_typed_array() {
    with_vm(|vm, _dom| {
        // Derived height: 8 px of data ÷ width 4 = height 2.
        assert!(eval_bool(
            vm,
            "var d = new ImageData(new Uint8ClampedArray(32), 4); \
             d.width === 4 && d.height === 2 && d.data.length === 32;"
        ));
    });
}

#[test]
fn image_data_constructor_rejects_inconsistent_data() {
    with_vm(|vm, _dom| {
        // Data length not a multiple of 4 → InvalidStateError.
        assert!(vm
            .eval("new ImageData(new Uint8ClampedArray(6), 1);")
            .is_err());
        // width × height does not match data length → IndexSizeError.
        assert!(vm
            .eval("new ImageData(new Uint8ClampedArray(8), 2, 5);")
            .is_err());
        // Data not divisible by width → IndexSizeError.
        assert!(vm
            .eval("new ImageData(new Uint8ClampedArray(12), 2);")
            .is_err());
    });
}

#[test]
fn get_put_image_data_round_trip() {
    with_vm(|vm, _dom| {
        assert!(eval_bool(
            vm,
            "var ctx = document.createElement('canvas').getContext('2d'); \
             ctx.fillStyle = 'rgb(10, 20, 30)'; ctx.fillRect(0, 0, 2, 2); \
             var d = ctx.getImageData(0, 0, 2, 2); \
             d.width === 2 && d.height === 2 && \
             d.data[0] === 10 && d.data[1] === 20 && d.data[2] === 30 && d.data[3] === 255;"
        ));
    });
}

#[test]
fn create_image_data_is_transparent_black() {
    with_vm(|vm, _dom| {
        assert!(eval_bool(
            vm,
            "var ctx = document.createElement('canvas').getContext('2d'); \
             var d = ctx.createImageData(3, 2); \
             d.width === 3 && d.height === 2 && d.data.length === 24 && \
             d.data[0] === 0 && d.data[23] === 0;"
        ));
    });
}
