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
fn get_image_data_rejects_zero_dims() {
    // HTML §4.12.5.1.16: getImageData with sw/sh == 0 throws IndexSizeError.
    with_vm(|vm, _dom| {
        assert!(vm
            .eval(
                "var ctx = document.createElement('canvas').getContext('2d'); \
                 ctx.getImageData(0, 0, 0, 4);"
            )
            .is_err());
        assert!(vm
            .eval(
                "var ctx = document.createElement('canvas').getContext('2d'); \
                 ctx.getImageData(0, 0, 4, 0);"
            )
            .is_err());
    });
}

#[test]
fn create_image_data_rejects_zero_dims() {
    with_vm(|vm, _dom| {
        assert!(vm
            .eval(
                "var ctx = document.createElement('canvas').getContext('2d'); \
                 ctx.createImageData(0, 4);"
            )
            .is_err());
    });
}

#[test]
fn image_data_constructor_rejects_non_clamped_typed_array() {
    // The data overload requires Uint8ClampedArray; a plain Uint8Array → TypeError.
    with_vm(|vm, _dom| {
        assert!(vm.eval("new ImageData(new Uint8Array(4), 1);").is_err());
    });
}

#[test]
fn put_image_data_rejects_spoofed_and_mismatched() {
    with_vm(|vm, _dom| {
        let ctx = "var ctx = document.createElement('canvas').getContext('2d'); ";
        // Plain object with wrong-typed data → not an ImageData.
        assert!(vm
            .eval(&format!(
                "{ctx} ctx.putImageData({{width: 1, height: 1, data: new Uint8Array(4)}}, 0, 0);"
            ))
            .is_err());
        // data.length inconsistent with width*height*4 → not an ImageData.
        assert!(vm
            .eval(&format!(
                "{ctx} ctx.putImageData({{width: 2, height: 2, data: new Uint8ClampedArray(4)}}, 0, 0);"
            ))
            .is_err());
        // Spoofed zero-dimension object (0*0*4 == data.length == 0) must NOT
        // slip the length check — real ImageData has positive integral dims.
        assert!(vm
            .eval(&format!(
                "{ctx} ctx.putImageData({{width: 0, height: 0, data: new Uint8ClampedArray(0)}}, 0, 0);"
            ))
            .is_err());
        // Fractional dimensions are likewise not a real ImageData.
        assert!(vm
            .eval(&format!(
                "{ctx} ctx.putImageData({{width: 2.5, height: 2, data: new Uint8ClampedArray(20)}}, 0, 0);"
            ))
            .is_err());
        // A real ImageData round-trips.
        assert!(vm
            .eval(&format!(
                "{ctx} ctx.putImageData(new ImageData(2, 2), 0, 0); true;"
            ))
            .is_ok());
    });
}

#[test]
fn canvas_width_preserves_large_integer_precision() {
    // > 2^24: an f32 round-trip would corrupt this; f64→u32 keeps it exact.
    with_vm(|vm, _dom| {
        assert!(eval_bool(
            vm,
            "var c = document.createElement('canvas'); c.width = 16777217; \
             c.width === 16777217;"
        ));
    });
}

#[test]
fn image_data_constructor_rejects_overflow_dims() {
    with_vm(|vm, _dom| {
        // width*height*4 overflows usize → RangeError, not a 0-length ImageData.
        assert!(vm.eval("new ImageData(4294967295, 4294967295);").is_err());
    });
}

#[test]
fn put_image_data_accepts_deep_prototype_chain() {
    // The ImageData brand walks the prototype chain up to the VM-wide
    // PROTO_CHAIN_LIMIT (10_000), not a smaller bespoke cap: a valid subclass
    // whose chain reaches ImageData.prototype only after >64 hops must still be
    // recognized, not falsely rejected.
    with_vm(|vm, _dom| {
        assert!(vm
            .eval(
                "var ctx = document.createElement('canvas').getContext('2d'); \
                 var base = new ImageData(2, 2); \
                 var proto = ImageData.prototype; \
                 for (var i = 0; i < 100; i++) { proto = Object.create(proto); } \
                 var sub = Object.create(proto); \
                 sub.width = 2; sub.height = 2; sub.data = base.data; \
                 ctx.putImageData(sub, 0, 0); true;"
            )
            .is_ok());
    });
}

#[test]
fn image_data_props_are_non_configurable() {
    // data/width/height are non-configurable own props: `delete` cannot remove
    // them (this VM throws a TypeError on deleting a non-configurable property),
    // so reads and the data array identity survive intact.
    with_vm(|vm, _dom| {
        assert!(eval_bool(
            vm,
            "var img = new ImageData(2, 2); var d = img.data; \
             var threw = false; \
             try { delete img.width; } catch (e) { threw = e instanceof TypeError; } \
             threw && img.width === 2 && img.height === 2 && img.data === d \
             && img.data.length === 16;"
        ));
        // Non-configurable → redefining the property throws a TypeError.
        assert!(eval_bool(
            vm,
            "var img = new ImageData(1, 1); \
             try { Object.defineProperty(img, 'width', {value: 99}); false; } \
             catch (e) { e instanceof TypeError && img.width === 1; }"
        ));
    });
}

#[test]
fn canvas_width_setter_uses_to_uint32() {
    // canvas.width is a reflected WebIDL `unsigned long` → ToUint32 (mod 2^32),
    // NOT clamp-to-0 / saturate. -1 wraps to u32::MAX; 2^32 wraps to 0.
    with_vm(|vm, _dom| {
        assert!(eval_bool(
            vm,
            "var c = document.createElement('canvas'); c.width = -1; \
             c.width === 4294967295;"
        ));
        assert!(eval_bool(
            vm,
            "var c = document.createElement('canvas'); c.height = 4294967296; \
             c.height === 0;"
        ));
    });
}

#[test]
fn get_context_rejects_non_canvas_receiver() {
    // getContext.call(<div>, '2d') must throw Illegal invocation, not attach
    // canvas state to a non-canvas element.
    with_vm(|vm, _dom| {
        assert!(vm
            .eval(
                "var d = document.createElement('div'); \
                 HTMLCanvasElement.prototype.getContext.call(d, '2d');"
            )
            .is_err());
    });
}

#[test]
fn width_accessor_rejects_non_canvas_receiver() {
    with_vm(|vm, _dom| {
        assert!(vm
            .eval(
                "var d = document.createElement('div'); \
                 var g = Object.getOwnPropertyDescriptor(HTMLCanvasElement.prototype, 'width').get; \
                 g.call(d);"
            )
            .is_err());
    });
}

#[test]
fn canvas_method_rejects_context_wrapper_receiver() {
    // The context wrapper shares its canvas entity, but an HTMLCanvasElement
    // method invoked with this = ctx must throw (it's not the element wrapper).
    with_vm(|vm, _dom| {
        assert!(vm
            .eval(
                "var c = document.createElement('canvas'); var ctx = c.getContext('2d'); \
                 HTMLCanvasElement.prototype.getContext.call(ctx, '2d');"
            )
            .is_err());
    });
}

#[test]
fn image_data_data_overload_preserves_identity() {
    // new ImageData(arr, w) must use `arr` as `data` by reference (no copy).
    with_vm(|vm, _dom| {
        assert!(eval_bool(
            vm,
            "var a = new Uint8ClampedArray(16); var img = new ImageData(a, 2); \
             img.data === a && img.width === 2 && img.height === 2;"
        ));
    });
}

#[test]
fn put_image_data_accepts_image_data_subtype_chain() {
    // Brand is prototype-CHAIN membership: an object reaching ImageData.prototype
    // via an intermediate prototype (a subclass instance) is a valid ImageData.
    with_vm(|vm, _dom| {
        assert!(vm
            .eval(
                "var ctx = document.createElement('canvas').getContext('2d'); \
                 var base = new ImageData(2, 2); \
                 var sub = Object.create(Object.create(ImageData.prototype)); \
                 sub.width = 2; sub.height = 2; sub.data = base.data; \
                 ctx.putImageData(sub, 0, 0); true;"
            )
            .is_ok());
    });
}

#[test]
fn put_image_data_rejects_oversized_branded_object() {
    // A branded ImageData-like object (ImageData.prototype on its chain) with
    // huge own width/height must be capped BEFORE copying its `data` — untrusted
    // JS putImageData must not OOM the process. width*height*4 > 2 GiB → throw.
    with_vm(|vm, _dom| {
        assert!(vm
            .eval(
                "var ctx = document.createElement('canvas').getContext('2d'); \
                 var sub = Object.create(ImageData.prototype); \
                 sub.width = 100000; sub.height = 100000; \
                 sub.data = new Uint8ClampedArray(4); \
                 ctx.putImageData(sub, 0, 0);"
            )
            .is_err());
    });
}

#[test]
fn put_image_data_rejects_small_dims_huge_data_without_copy() {
    // Inverse OOM vector to the huge-dims case: a branded object with tiny dims
    // (1×1 → expected 4 bytes) but a giant `data` array. The view's byte length
    // is checked BEFORE the copy, so the mismatch rejects it without allocating
    // gigabytes. 64 MiB is large enough that an accidental copy is observable
    // yet cheap enough to allocate once for the test.
    with_vm(|vm, _dom| {
        assert!(vm
            .eval(
                "var ctx = document.createElement('canvas').getContext('2d'); \
                 var sub = Object.create(ImageData.prototype); \
                 sub.width = 1; sub.height = 1; \
                 sub.data = new Uint8ClampedArray(64 * 1024 * 1024); \
                 ctx.putImageData(sub, 0, 0);"
            )
            .is_err());
    });
}

#[test]
fn get_image_data_normalizes_negative_dimensions() {
    // HTML getImageData: a negative sw/sh shifts the origin and flips the rect.
    // Fill x∈[0,5) red; getImageData(5,0,-5,1) must read the SAME region as
    // getImageData(0,0,5,1) (x∈[0,5)), not x∈[5,10).
    with_vm(|vm, _dom| {
        assert!(eval_bool(
            vm,
            "var ctx = document.createElement('canvas').getContext('2d'); \
             ctx.fillStyle = 'rgb(255, 0, 0)'; ctx.fillRect(0, 0, 5, 1); \
             var neg = ctx.getImageData(5, 0, -5, 1); \
             var pos = ctx.getImageData(0, 0, 5, 1); \
             neg.width === 5 && neg.height === 1 \
             && neg.data[0] === 255 && neg.data[1] === 0 && neg.data[3] === 255 \
             && Array.from(neg.data).join() === Array.from(pos.data).join();"
        ));
    });
}

#[test]
fn get_image_data_rejects_oversized_request() {
    // Untrusted JS must not OOM the process: a huge sw*sh is capped → RangeError.
    with_vm(|vm, _dom| {
        assert!(vm
            .eval(
                "var ctx = document.createElement('canvas').getContext('2d'); \
                 ctx.getImageData(0, 0, 100000, 100000);"
            )
            .is_err());
    });
}

#[test]
fn image_data_constructor_rejects_oversized() {
    with_vm(|vm, _dom| {
        assert!(vm.eval("new ImageData(100000, 100000);").is_err());
    });
}

#[test]
fn get_image_data_dim_uses_to_int32() {
    // WebIDL `long` sw is ToInt32 (mod 2^32): 4294967301 → 5, not saturated.
    with_vm(|vm, _dom| {
        assert!(eval_bool(
            vm,
            "var ctx = document.createElement('canvas').getContext('2d'); \
             ctx.getImageData(0, 0, 4294967301, 1).width === 5;"
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
