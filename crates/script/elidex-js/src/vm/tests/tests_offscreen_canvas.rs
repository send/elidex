//! Slot `#11-offscreen-canvas-vm` — `OffscreenCanvas` + `OffscreenCanvasRenderingContext2D` +
//! `convertToBlob` + `transferControlToOffscreen` coverage (HTML §4.12.5.1.7).
//! Main-thread side only; worker-side transferable receipt is deferred.

#![cfg(feature = "engine")]

use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;

fn build_doc(dom: &mut EcsDom) -> elidex_ecs::Entity {
    let doc = dom.create_document_root();
    let html = dom.create_element("html", elidex_ecs::Attributes::default());
    let body = dom.create_element("body", elidex_ecs::Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, body));
    doc
}

/// Single-shot eval against a fresh VM bound to a minimal document — most OC
/// tests don't need the DOM (ctor / getContext / convertToBlob work without
/// it), but `transferControlToOffscreen` does, so the harness binds
/// unconditionally for uniformity. `f` receives the bound `Vm` and returns
/// whatever the test cares about.
fn with_vm<R>(f: impl FnOnce(&mut Vm) -> R) -> R {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let r = f(&mut vm);
    vm.unbind();
    r
}

fn eval_bool(vm: &mut Vm, src: &str) -> bool {
    match vm.eval(src).unwrap() {
        JsValue::Boolean(b) => b,
        other => panic!("expected bool, got {other:?}"),
    }
}

fn eval_number(vm: &mut Vm, src: &str) -> f64 {
    match vm.eval(src).unwrap() {
        JsValue::Number(n) => n,
        other => panic!("expected number, got {other:?}"),
    }
}

/// Run `src` (which is expected to write to `globalThis[name]`, typically via a
/// `.then` callback) and return that global as a String AFTER the microtask
/// drain at the end of `vm.eval`. Mirrors the `tests_body_mixin::
/// eval_global_string` pattern — Promise callbacks fire during the drain, so
/// reading the global inline in the same script would see pre-drain state.
fn eval_global_string(vm: &mut Vm, src: &str, name: &str) -> String {
    vm.eval(src).unwrap();
    match vm.get_global(name) {
        Some(JsValue::String(id)) => vm.get_string(id),
        other => panic!("expected global {name} to be a string, got {other:?}"),
    }
}

fn eval_global_bool(vm: &mut Vm, src: &str, name: &str) -> bool {
    vm.eval(src).unwrap();
    match vm.get_global(name) {
        Some(JsValue::Boolean(b)) => b,
        other => panic!("expected global {name} to be a bool, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Constructor + IDL accessors
// ---------------------------------------------------------------------------

#[test]
fn ctor_creates_oc_with_dims() {
    with_vm(|vm| {
        assert_eq!(
            eval_number(vm, "new OffscreenCanvas(640, 480).width"),
            640.0
        );
    });
}

#[test]
fn ctor_creates_oc_with_height() {
    with_vm(|vm| {
        assert_eq!(
            eval_number(vm, "new OffscreenCanvas(640, 480).height"),
            480.0
        );
    });
}

#[test]
fn ctor_requires_two_args() {
    with_vm(|vm| {
        let err = vm.eval("new OffscreenCanvas(100)").unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("2 arguments required") || msg.contains("only 1 present"),
            "expected arg-count TypeError, got {msg}"
        );
    });
}

#[test]
fn ctor_requires_new_operator() {
    with_vm(|vm| {
        let err = vm.eval("OffscreenCanvas(100, 100)").unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("'new' operator"),
            "expected 'new' operator TypeError, got {msg}"
        );
    });
}

#[test]
fn instanceof_offscreen_canvas() {
    with_vm(|vm| {
        assert!(eval_bool(
            vm,
            "new OffscreenCanvas(10, 10) instanceof OffscreenCanvas"
        ));
    });
}

#[test]
fn width_setter_updates_idl_value() {
    with_vm(|vm| {
        assert_eq!(
            eval_number(
                vm,
                "var oc = new OffscreenCanvas(10, 10); oc.width = 200; oc.width"
            ),
            200.0
        );
    });
}

// ---------------------------------------------------------------------------
// getContext('2d') + SameObject identity
// ---------------------------------------------------------------------------

#[test]
fn get_context_2d_same_object() {
    with_vm(|vm| {
        assert!(eval_bool(
            vm,
            "var oc = new OffscreenCanvas(50, 50); \
             oc.getContext('2d') === oc.getContext('2d')"
        ));
    });
}

#[test]
fn get_context_unknown_returns_null() {
    with_vm(|vm| {
        assert!(eval_bool(
            vm,
            "new OffscreenCanvas(10, 10).getContext('webgl') === null"
        ));
        assert!(eval_bool(
            vm,
            "new OffscreenCanvas(10, 10).getContext('bitmaprenderer') === null"
        ));
    });
}

#[test]
fn context_canvas_backref_round_trip() {
    with_vm(|vm| {
        // canvas backref returns the same OC wrapper (cache_wrapper guarantees
        // SameObject identity).
        assert!(eval_bool(
            vm,
            "var oc = new OffscreenCanvas(10, 10); \
             var ctx = oc.getContext('2d'); ctx.canvas === oc"
        ));
    });
}

#[test]
fn context_instanceof_ocrc2d() {
    with_vm(|vm| {
        assert!(eval_bool(
            vm,
            "new OffscreenCanvas(10, 10).getContext('2d') \
             instanceof OffscreenCanvasRenderingContext2D"
        ));
    });
}

#[test]
fn ocrc2d_constructor_throws_illegal() {
    with_vm(|vm| {
        let err = vm
            .eval("new OffscreenCanvasRenderingContext2D()")
            .unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.contains("Illegal constructor"), "got {msg}");
    });
}

// ---------------------------------------------------------------------------
// 2D drawing methods (brand-checked into OC entity)
// ---------------------------------------------------------------------------

#[test]
fn drawing_methods_run_without_throwing() {
    with_vm(|vm| {
        // A full surface smoke test — all 17 CONTEXT_METHODS dispatch into
        // the brand-checked OC entity. This catches a broken brand check or
        // a wrong-arg shape silently dropping the dispatch.
        assert!(eval_bool(
            vm,
            "var ctx = new OffscreenCanvas(10, 10).getContext('2d'); \
             ctx.save(); ctx.beginPath(); ctx.moveTo(0,0); ctx.lineTo(5,5); \
             ctx.rect(0,0,5,5); ctx.arc(5,5,3,0,6.28,false); ctx.closePath(); \
             ctx.fillRect(0,0,3,3); ctx.strokeRect(0,0,3,3); ctx.clearRect(0,0,1,1); \
             ctx.translate(1,1); ctx.rotate(0.1); ctx.scale(2,2); \
             ctx.fill(); ctx.stroke(); ctx.restore(); \
             ctx.measureText('hi').width >= 0",
        ));
    });
}

#[test]
fn context_method_brand_check_rejects_canvas_2d_context() {
    // OC method called with a <canvas> 2D context as `this` must throw —
    // brand-check rejects the wrong-interface receiver.
    with_vm(|vm| {
        let err = vm
            .eval(
                "var c2d = document.createElement('canvas').getContext('2d'); \
                 OffscreenCanvasRenderingContext2D.prototype.fillRect.call(c2d, 0,0,1,1);",
            )
            .unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("Illegal invocation") && msg.contains("OffscreenCanvasRenderingContext2D"),
            "got {msg}"
        );
    });
}

// ---------------------------------------------------------------------------
// convertToBlob (Promise + format dispatch)
// ---------------------------------------------------------------------------
//
// Promise reactions run via microtask scheduling — the inline `.then` callback
// captures into a `globalThis` slot which we read after eval completes (same
// pattern as `tests_body_mixin::response_text_round_trip`).

#[test]
fn convert_to_blob_default_png_resolves_with_blob() {
    with_vm(|vm| {
        assert_eq!(
            eval_global_string(
                vm,
                "globalThis.r = ''; \
                 var oc = new OffscreenCanvas(4, 4); oc.getContext('2d'); \
                 oc.convertToBlob().then(b => { globalThis.r = b.type; });",
                "r",
            ),
            "image/png"
        );
    });
}

#[test]
fn convert_to_blob_jpeg_via_type_option() {
    with_vm(|vm| {
        assert_eq!(
            eval_global_string(
                vm,
                "globalThis.r = ''; \
                 var oc = new OffscreenCanvas(4, 4); oc.getContext('2d'); \
                 oc.convertToBlob({type: 'image/jpeg'}).then(b => { globalThis.r = b.type; });",
                "r",
            ),
            "image/jpeg"
        );
    });
}

#[test]
fn convert_to_blob_webp_via_type_option() {
    with_vm(|vm| {
        assert_eq!(
            eval_global_string(
                vm,
                "globalThis.r = ''; \
                 var oc = new OffscreenCanvas(4, 4); oc.getContext('2d'); \
                 oc.convertToBlob({type: 'image/webp'}).then(b => { globalThis.r = b.type; });",
                "r",
            ),
            "image/webp"
        );
    });
}

#[test]
fn convert_to_blob_unknown_type_falls_back_to_png() {
    // Spec: unknown / unsupported type defaults to image/png.
    with_vm(|vm| {
        assert_eq!(
            eval_global_string(
                vm,
                "globalThis.r = ''; \
                 var oc = new OffscreenCanvas(4, 4); oc.getContext('2d'); \
                 oc.convertToBlob({type: 'image/bogus'}).then(b => { globalThis.r = b.type; });",
                "r",
            ),
            "image/png"
        );
    });
}

#[test]
fn convert_to_blob_type_with_mime_params_maps_essence() {
    // WHATWG MIME parser: `"image/jpeg; charset=utf-8"` essence is
    // `image/jpeg` → JPEG encoder. The Blob.type is the canonical encoder
    // MIME (no params), per spec.
    with_vm(|vm| {
        assert_eq!(
            eval_global_string(
                vm,
                "globalThis.r = ''; \
                 var oc = new OffscreenCanvas(4, 4); oc.getContext('2d'); \
                 oc.convertToBlob({type: 'image/jpeg; charset=utf-8'}).then(b => { globalThis.r = b.type; });",
                "r",
            ),
            "image/jpeg"
        );
    });
}

#[test]
fn convert_to_blob_before_get_context_rejects_invalid_state_error() {
    // No `getContext('2d')` was called — context mode is set to none — per
    // HTML §4.12.5.1.7 convertToBlob "If this OffscreenCanvas object's
    // context mode is set to none" the Promise rejects with InvalidStateError.
    with_vm(|vm| {
        assert_eq!(
            eval_global_string(
                vm,
                "globalThis.errName = ''; \
                 var oc = new OffscreenCanvas(4, 4); \
                 oc.convertToBlob().catch(e => { globalThis.errName = e.name; });",
                "errName",
            ),
            "InvalidStateError"
        );
    });
}

#[test]
fn convert_to_blob_zero_dim_rejects_index_size_error() {
    // HTML §4.12.5.1.7 convertToBlob "If this OffscreenCanvas object's bitmap
    // has no pixels (i.e. either its horizontal dimension or its vertical
    // dimension is zero) then return a promise rejected with an
    // IndexSizeError DOMException." — exercised on width=0 and height=0.
    with_vm(|vm| {
        assert_eq!(
            eval_global_string(
                vm,
                "globalThis.errName = ''; \
                 var oc = new OffscreenCanvas(0, 4); oc.getContext('2d'); \
                 oc.convertToBlob().catch(e => { globalThis.errName = e.name; });",
                "errName",
            ),
            "IndexSizeError"
        );
        assert_eq!(
            eval_global_string(
                vm,
                "globalThis.errName = ''; \
                 var oc = new OffscreenCanvas(4, 0); oc.getContext('2d'); \
                 oc.convertToBlob().catch(e => { globalThis.errName = e.name; });",
                "errName",
            ),
            "IndexSizeError"
        );
    });
}

#[test]
fn convert_to_blob_result_is_blob_instance() {
    with_vm(|vm| {
        assert!(eval_global_bool(
            vm,
            "globalThis.is_blob = false; \
             var oc = new OffscreenCanvas(4, 4); oc.getContext('2d'); \
             oc.convertToBlob().then(b => { globalThis.is_blob = b instanceof Blob; });",
            "is_blob",
        ));
    });
}

// ---------------------------------------------------------------------------
// transferControlToOffscreen
// ---------------------------------------------------------------------------

#[test]
fn transfer_returns_offscreen_canvas_with_canvas_dims() {
    with_vm(|vm| {
        assert!(eval_bool(
            vm,
            "var c = document.createElement('canvas'); \
             c.setAttribute('width', '320'); c.setAttribute('height', '200'); \
             var oc = c.transferControlToOffscreen(); \
             oc instanceof OffscreenCanvas && oc.width === 320 && oc.height === 200"
        ));
    });
}

#[test]
fn transfer_then_get_context_throws_invalid_state() {
    with_vm(|vm| {
        let err = vm
            .eval(
                "var c = document.createElement('canvas'); \
                 c.transferControlToOffscreen(); \
                 c.getContext('2d');",
            )
            .unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("transferred"),
            "expected placeholder-related error, got {msg}"
        );
    });
}

#[test]
fn transfer_after_get_context_throws_invalid_state() {
    with_vm(|vm| {
        let err = vm
            .eval(
                "var c = document.createElement('canvas'); \
                 c.getContext('2d'); \
                 c.transferControlToOffscreen();",
            )
            .unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("rendering context") || msg.contains("AlreadyHasContext"),
            "expected already-has-context error, got {msg}"
        );
    });
}

#[test]
fn transfer_double_throws_invalid_state() {
    with_vm(|vm| {
        let err = vm
            .eval(
                "var c = document.createElement('canvas'); \
                 c.transferControlToOffscreen(); \
                 c.transferControlToOffscreen();",
            )
            .unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("already been transferred") || msg.contains("AlreadyPlaceholder"),
            "expected double-transfer error, got {msg}"
        );
    });
}

// ---------------------------------------------------------------------------
// Width/height setter resets bitmap (shared `reset_canvas_bitmap` chokepoint)
// ---------------------------------------------------------------------------

#[test]
fn width_setter_after_get_context_resets_bitmap_dims() {
    with_vm(|vm| {
        // After getContext + a fill, then a width setter, dims update.
        assert_eq!(
            eval_number(
                vm,
                "var oc = new OffscreenCanvas(10, 10); \
                 var ctx = oc.getContext('2d'); \
                 ctx.fillStyle = 'red'; ctx.fillRect(0, 0, 10, 10); \
                 oc.width = 20; oc.width"
            ),
            20.0
        );
    });
}

// ---------------------------------------------------------------------------
// is_node() == false (Node-arg coercion exclusion)
// ---------------------------------------------------------------------------

#[test]
fn offscreen_canvas_rejected_as_node_argument() {
    // appendChild(oc) must throw — OC is an EventTarget but NOT a Node
    // (NodeKind::OffscreenCanvas::is_node() == false).
    with_vm(|vm| {
        let err = vm
            .eval(
                "var oc = new OffscreenCanvas(10, 10); \
                 document.body.appendChild(oc);",
            )
            .unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("Node") || msg.contains("argument") || msg.contains("appendChild"),
            "expected Node-coercion failure, got {msg}"
        );
    });
}

// ---------------------------------------------------------------------------
// [EnforceRange] unsigned long long coercion (WebIDL §3.10.4)
// ---------------------------------------------------------------------------

#[test]
fn ctor_throws_range_error_on_overflow() {
    // Above u32::MAX → RangeError per `[EnforceRange]` (spec strict path).
    with_vm(|vm| {
        let err = vm.eval("new OffscreenCanvas(4294967296, 10)").unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("RangeError") || msg.contains("out of range"),
            "expected RangeError, got {msg}"
        );
    });
}

#[test]
fn ctor_throws_range_error_on_negative() {
    // Negative → out of [0, 2^32-1] → RangeError.
    with_vm(|vm| {
        let err = vm.eval("new OffscreenCanvas(-1, 10)").unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("RangeError") || msg.contains("out of range"),
            "expected RangeError, got {msg}"
        );
    });
}

#[test]
fn ctor_throws_type_error_on_nan() {
    // NaN / non-finite → TypeError per `[EnforceRange]` step 1.
    with_vm(|vm| {
        let err = vm.eval("new OffscreenCanvas(NaN, 10)").unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("TypeError") || msg.contains("finite"),
            "expected TypeError, got {msg}"
        );
    });
}

#[test]
fn width_setter_throws_range_error_on_overflow() {
    with_vm(|vm| {
        let err = vm
            .eval("var oc = new OffscreenCanvas(10, 10); oc.width = 4294967296;")
            .unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("RangeError") || msg.contains("out of range"),
            "expected RangeError, got {msg}"
        );
    });
}
