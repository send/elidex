//! M4-12 D-22 `#11-resize-observer-vm` — `ResizeObserver` thin VM
//! binding tests.
//!
//! Covers prototype install, constructor + brand-check, init-dict
//! parsing (`box` enum), and the embedder API
//! `Vm::deliver_resize_observations` end-to-end against a synthetic
//! `LayoutBox` source.

#![cfg(feature = "engine")]

use elidex_ecs::EcsDom;
use elidex_plugin::Rect;
use elidex_script_session::SessionCore;

use super::super::test_helpers::{bind_vm, set_layout_box};
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

fn run(script: &str) -> String {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let result = vm.eval(script).unwrap();
    let JsValue::String(sid) = result else {
        panic!("expected string, got {result:?}")
    };
    let out = vm.inner.strings.get_utf8(sid);
    vm.unbind();
    out
}

fn run_throws(script: &str) -> String {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let err = vm.eval(script).expect_err("expected an error");
    vm.unbind();
    format!("{err:?}")
}

// --- Prototype / constructor / brand check --------------------------------

#[test]
fn resize_observer_prototype_installed() {
    let mut vm = Vm::new();
    assert!(
        vm.inner.resize_observer_prototype.is_some(),
        "ResizeObserver.prototype must be allocated during register_globals"
    );
    let result = vm
        .eval("typeof ResizeObserver === 'function'")
        .expect("typeof expression must not throw");
    assert_eq!(result, JsValue::Boolean(true));
}

#[test]
fn resize_observer_constructor_creates_instance() {
    let out = run("var ro = new ResizeObserver(function(){}); typeof ro;");
    assert_eq!(out, "object");
}

#[test]
fn resize_observer_constructor_without_host_data_throws() {
    let mut vm = Vm::new();
    let err = vm
        .eval("new ResizeObserver(function(){})")
        .expect_err("constructor must error pre-install_host_data");
    let err_text = format!("{err:?}");
    assert!(
        err_text.contains("host environment is not initialised"),
        "expected pre-init TypeError, got: {err_text}"
    );
}

#[test]
fn resize_observer_constructor_requires_callable() {
    let err = run_throws("new ResizeObserver(123);");
    assert!(
        err.contains("not of type 'Function'"),
        "expected ResizeObserver callable TypeError, got: {err}"
    );
}

#[test]
fn resize_observer_constructor_bare_call_throws() {
    let err = run_throws("ResizeObserver(function(){});");
    assert!(
        err.contains("'new' operator"),
        "expected bare-call TypeError, got: {err}"
    );
}

#[test]
fn resize_observer_instanceof_works() {
    let out = run("var ro = new ResizeObserver(function(){}); \
         (ro instanceof ResizeObserver) ? 'yes' : 'no';");
    assert_eq!(out, "yes");
}

#[test]
fn resize_observer_brand_check_disconnect() {
    let err = run_throws("ResizeObserver.prototype.disconnect.call({});");
    assert!(
        err.contains("Illegal invocation"),
        "expected brand-check TypeError, got: {err}"
    );
}

#[test]
fn resize_observer_brand_check_observe() {
    let err = run_throws("ResizeObserver.prototype.observe.call({}, document);");
    assert!(
        err.contains("Illegal invocation"),
        "expected brand-check TypeError, got: {err}"
    );
}

#[test]
fn resize_observer_brand_check_unobserve() {
    let err = run_throws("ResizeObserver.prototype.unobserve.call({}, document);");
    assert!(
        err.contains("Illegal invocation"),
        "expected brand-check TypeError, got: {err}"
    );
}

// --- observe argument validation -----------------------------------------

#[test]
fn resize_observer_observe_returns_undefined() {
    let out = run("var ro = new ResizeObserver(function(){}); \
         typeof ro.observe(document);");
    assert_eq!(out, "undefined");
}

#[test]
fn resize_observer_observe_target_must_be_node() {
    let err = run_throws("var ro = new ResizeObserver(function(){}); ro.observe({});");
    assert!(
        err.contains("not of type 'Node'"),
        "expected non-Node TypeError, got: {err}"
    );
}

#[test]
fn resize_observer_observe_requires_target() {
    let err = run_throws("var ro = new ResizeObserver(function(){}); ro.observe();");
    assert!(
        err.contains("1 argument required"),
        "expected missing-target TypeError, got: {err}"
    );
}

#[test]
fn resize_observer_options_box_enum_accepts_each_variant() {
    // All three WebIDL enum values must be accepted without throwing.
    let out = run("var ro = new ResizeObserver(function(){}); \
         try { \
            ro.observe(document, {box:'content-box'}); \
            ro.observe(document, {box:'border-box'}); \
            ro.observe(document, {box:'device-pixel-content-box'}); \
            'ok' \
         } catch(e) { 'threw: ' + e.message; }");
    assert_eq!(out, "ok");
}

#[test]
fn resize_observer_options_box_enum_rejects_invalid() {
    let err = run_throws(
        "var ro = new ResizeObserver(function(){}); \
         ro.observe(document, {box: 'unknown-box'});",
    );
    assert!(
        err.contains("not a valid enum value") && err.contains("'unknown-box'"),
        "expected enum-rejection TypeError, got: {err}"
    );
}

// --- Delivery (deliver_resize_observations) -------------------------------

#[test]
fn resize_observer_deliver_fires_callback_with_entry() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    let body = dom
        .first_child_with_tag(dom.first_child_with_tag(doc, "html").unwrap(), "body")
        .unwrap();
    let target = dom.create_element("div", elidex_ecs::Attributes::default());
    assert!(dom.append_child(body, target));

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let target_wrapper = vm.inner.create_element_wrapper(target);
    vm.set_global("target", JsValue::Object(target_wrapper));

    vm.eval(
        "globalThis.calls = []; \
         globalThis.ro = new ResizeObserver(function(entries){ \
             for (var i = 0; i < entries.length; i++) { \
                 calls.push(entries[i]); \
             } \
         }); \
         ro.observe(target);",
    )
    .unwrap();

    set_layout_box(&mut vm, target, Rect::new(0.0, 0.0, 100.0, 50.0));
    vm.deliver_resize_observations();

    let out = vm
        .eval(
            "calls.length + '|' + \
             calls[0].contentRect.width + 'x' + calls[0].contentRect.height + '|' + \
             calls[0].contentBoxSize[0].inlineSize + 'x' + \
             calls[0].contentBoxSize[0].blockSize + '|' + \
             (calls[0].target === target ? 'same' : 'diff') + '|' + \
             (calls[0].contentRect instanceof DOMRectReadOnly ? 'rect' : 'notrect')",
        )
        .unwrap();
    let JsValue::String(sid) = out else {
        panic!("expected string, got {out:?}")
    };
    let got = vm.inner.strings.get_utf8(sid);
    assert_eq!(got, "1|100x50|100x50|same|rect");

    vm.unbind();
}

#[test]
fn resize_observer_content_rect_uses_element_local_coords() {
    // W3C Resize Observer §2.3: `contentRect` is in the element's own
    // coordinate space — origin = padding offsets, NOT document
    // coordinates.  Regression for Copilot R2: passing `lb.content`
    // (document-coord) directly to `gather_observations` produced
    // wrong x/y for any positioned-or-padded element.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    let body = dom
        .first_child_with_tag(dom.first_child_with_tag(doc, "html").unwrap(), "body")
        .unwrap();
    let target = dom.create_element("div", elidex_ecs::Attributes::default());
    assert!(dom.append_child(body, target));

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let target_wrapper = vm.inner.create_element_wrapper(target);
    vm.set_global("target", JsValue::Object(target_wrapper));

    vm.eval(
        "globalThis.calls = []; \
         globalThis.ro = new ResizeObserver(function(entries){ \
             calls.push(entries[0].contentRect.x + ',' + entries[0].contentRect.y + ',' + \
                        entries[0].contentRect.width + ',' + entries[0].contentRect.height); \
         }); \
         ro.observe(target);",
    )
    .unwrap();

    // Element positioned at document (50, 30) with padding {top: 7, left: 11}
    // and content size (100 x 50).  contentRect must be (11, 7, 100, 50) —
    // origin = padding offsets, NOT (50, 30, 100, 50).
    {
        let dom = vm.host_data().unwrap().dom();
        let _ = dom
            .world_mut()
            .remove_one::<elidex_plugin::LayoutBox>(target);
        let lb = elidex_plugin::LayoutBox {
            content: Rect::new(50.0, 30.0, 100.0, 50.0),
            padding: elidex_plugin::EdgeSizes::new(7.0, 13.0, 17.0, 11.0),
            ..elidex_plugin::LayoutBox::default()
        };
        dom.world_mut().insert_one(target, lb).unwrap();
    }
    vm.deliver_resize_observations();

    let out = vm.eval("calls.length + '|' + calls[0]").unwrap();
    let JsValue::String(sid) = out else {
        panic!("expected string, got {out:?}")
    };
    let got = vm.inner.strings.get_utf8(sid);
    assert_eq!(got, "1|11,7,100,50");

    vm.unbind();
}

#[test]
fn resize_observer_deliver_box_less_target_delivers_initial_zero_once() {
    // No LayoutBox attached → gather's `size_fn` returns None → spec
    // mandates a single initial 0×0 observation (Resize Observer §2.1).
    // The second deliver must not re-fire (last_size == ZERO).
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    let body = dom
        .first_child_with_tag(dom.first_child_with_tag(doc, "html").unwrap(), "body")
        .unwrap();
    let target = dom.create_element("div", elidex_ecs::Attributes::default());
    assert!(dom.append_child(body, target));

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let wrapper = vm.inner.create_element_wrapper(target);
    vm.set_global("target", JsValue::Object(wrapper));

    vm.eval(
        "globalThis.calls = 0; globalThis.lastWidth = -1; \
         globalThis.ro = new ResizeObserver(function(entries){ \
             calls++; lastWidth = entries[0].contentRect.width; \
         }); \
         ro.observe(target);",
    )
    .unwrap();

    // First deliver: spec-mandated initial 0×0 observation.
    vm.deliver_resize_observations();
    // Second deliver: still box-less, no re-delivery.
    vm.deliver_resize_observations();

    let out = vm.eval("calls + '|' + lastWidth").unwrap();
    let JsValue::String(sid) = out else {
        panic!("expected string, got {out:?}")
    };
    assert_eq!(vm.inner.strings.get_utf8(sid), "1|0");

    vm.unbind();
}

#[test]
fn resize_observer_disconnect_stops_delivery() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    let body = dom
        .first_child_with_tag(dom.first_child_with_tag(doc, "html").unwrap(), "body")
        .unwrap();
    let target = dom.create_element("div", elidex_ecs::Attributes::default());
    assert!(dom.append_child(body, target));

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let wrapper = vm.inner.create_element_wrapper(target);
    vm.set_global("target", JsValue::Object(wrapper));

    vm.eval(
        "globalThis.calls = 0; \
         globalThis.ro = new ResizeObserver(function(){ calls++; }); \
         ro.observe(target);",
    )
    .unwrap();

    set_layout_box(&mut vm, target, Rect::new(0.0, 0.0, 100.0, 50.0));
    vm.deliver_resize_observations();
    vm.eval("ro.disconnect();").unwrap();
    set_layout_box(&mut vm, target, Rect::new(0.0, 0.0, 200.0, 100.0));
    vm.deliver_resize_observations();

    let out = vm.eval("'' + calls").unwrap();
    let JsValue::String(sid) = out else {
        panic!("expected string, got {out:?}")
    };
    assert_eq!(vm.inner.strings.get_utf8(sid), "1");

    vm.unbind();
}

#[test]
fn resize_observer_reobserve_after_disconnect() {
    // W3C Resize Observer §3.5: `disconnect()` clears observation
    // targets but the observer stays usable.  A subsequent
    // `observe(other)` must therefore re-arm delivery — regression
    // guard against eagerly removing callback / instance maps on
    // disconnect (would silently swallow records here).
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    let body = dom
        .first_child_with_tag(dom.first_child_with_tag(doc, "html").unwrap(), "body")
        .unwrap();
    let a = dom.create_element("div", elidex_ecs::Attributes::default());
    let b = dom.create_element("section", elidex_ecs::Attributes::default());
    assert!(dom.append_child(body, a));
    assert!(dom.append_child(body, b));

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let wa = vm.inner.create_element_wrapper(a);
    let wb = vm.inner.create_element_wrapper(b);
    vm.set_global("a", JsValue::Object(wa));
    vm.set_global("b", JsValue::Object(wb));

    vm.eval(
        "globalThis.calls = 0; \
         globalThis.lastWidth = -1; \
         globalThis.ro = new ResizeObserver(function(entries){ \
             calls++; lastWidth = entries[0].contentRect.width; \
         }); \
         ro.observe(a);",
    )
    .unwrap();
    set_layout_box(&mut vm, a, Rect::new(0.0, 0.0, 100.0, 50.0));
    vm.deliver_resize_observations();

    // disconnect → re-observe a different target.
    vm.eval("ro.disconnect(); ro.observe(b);").unwrap();
    set_layout_box(&mut vm, b, Rect::new(0.0, 0.0, 222.0, 33.0));
    vm.deliver_resize_observations();

    let out = vm.eval("calls + '|' + lastWidth").unwrap();
    let JsValue::String(sid) = out else {
        panic!("expected string, got {out:?}")
    };
    assert_eq!(
        vm.inner.strings.get_utf8(sid),
        "2|222",
        "re-observe after disconnect must deliver to the new target"
    );

    vm.unbind();
}

#[test]
fn resize_observer_unobserve_drops_target() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    let body = dom
        .first_child_with_tag(dom.first_child_with_tag(doc, "html").unwrap(), "body")
        .unwrap();
    let target = dom.create_element("div", elidex_ecs::Attributes::default());
    assert!(dom.append_child(body, target));

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let wrapper = vm.inner.create_element_wrapper(target);
    vm.set_global("target", JsValue::Object(wrapper));

    vm.eval(
        "globalThis.calls = 0; \
         globalThis.ro = new ResizeObserver(function(){ calls++; }); \
         ro.observe(target);",
    )
    .unwrap();
    set_layout_box(&mut vm, target, Rect::new(0.0, 0.0, 100.0, 50.0));
    vm.deliver_resize_observations();
    vm.eval("ro.unobserve(target);").unwrap();
    set_layout_box(&mut vm, target, Rect::new(0.0, 0.0, 200.0, 200.0));
    vm.deliver_resize_observations();

    let out = vm.eval("'' + calls").unwrap();
    let JsValue::String(sid) = out else {
        panic!("expected string, got {out:?}")
    };
    assert_eq!(vm.inner.strings.get_utf8(sid), "1");
    vm.unbind();
}

#[test]
fn resize_observer_border_box_size_matches_layout() {
    // Sanity: padding/border absent → border_box.size == content.size.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    let body = dom
        .first_child_with_tag(dom.first_child_with_tag(doc, "html").unwrap(), "body")
        .unwrap();
    let target = dom.create_element("div", elidex_ecs::Attributes::default());
    assert!(dom.append_child(body, target));

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let wrapper = vm.inner.create_element_wrapper(target);
    vm.set_global("target", JsValue::Object(wrapper));

    vm.eval(
        "globalThis.last = null; \
         globalThis.ro = new ResizeObserver(function(entries){ last = entries[0]; }); \
         ro.observe(target);",
    )
    .unwrap();
    set_layout_box(&mut vm, target, Rect::new(0.0, 0.0, 80.0, 40.0));
    vm.deliver_resize_observations();
    let out = vm
        .eval(
            "last.borderBoxSize[0].inlineSize + 'x' + last.borderBoxSize[0].blockSize \
             + '|' + last.borderBoxSize.length",
        )
        .unwrap();
    let JsValue::String(sid) = out else {
        panic!("expected string, got {out:?}")
    };
    assert_eq!(vm.inner.strings.get_utf8(sid), "80x40|1");

    vm.unbind();
}

#[test]
fn resize_observer_two_observers_on_same_target_both_fire() {
    // The `ResizeObservedBy::0: Vec<ResizeObservation>` design supports
    // multiple observers per target.  Both callbacks must fire on a
    // single delivery tick.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    let body = dom
        .first_child_with_tag(dom.first_child_with_tag(doc, "html").unwrap(), "body")
        .unwrap();
    let target = dom.create_element("div", elidex_ecs::Attributes::default());
    assert!(dom.append_child(body, target));

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let wrapper = vm.inner.create_element_wrapper(target);
    vm.set_global("target", JsValue::Object(wrapper));

    vm.eval(
        "globalThis.callsA = 0; globalThis.callsB = 0; \
         globalThis.roA = new ResizeObserver(function(){ callsA++; }); \
         globalThis.roB = new ResizeObserver(function(){ callsB++; }); \
         roA.observe(target); roB.observe(target);",
    )
    .unwrap();
    set_layout_box(&mut vm, target, Rect::new(0.0, 0.0, 100.0, 50.0));
    vm.deliver_resize_observations();

    let out = vm.eval("callsA + '|' + callsB").unwrap();
    let JsValue::String(sid) = out else {
        panic!("expected string, got {out:?}")
    };
    assert_eq!(vm.inner.strings.get_utf8(sid), "1|1");

    vm.unbind();
}

#[test]
fn resize_observer_callback_survives_gc_via_root_chain() {
    // Even with no JS-stack reference to `ro` (assignment to a local
    // `var` inside a `function` that has returned), the
    // `gc_root_object_ids` chain via `HostData::resize_observer_bindings`
    // must keep the callback + instance alive across a GC cycle so the
    // next deliver tick still fires.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    let body = dom
        .first_child_with_tag(dom.first_child_with_tag(doc, "html").unwrap(), "body")
        .unwrap();
    let target = dom.create_element("div", elidex_ecs::Attributes::default());
    assert!(dom.append_child(body, target));

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let wrapper = vm.inner.create_element_wrapper(target);
    vm.set_global("target", JsValue::Object(wrapper));

    // Build the observer inside a scope that drops the local ref
    // immediately; the callback closes over `globalThis.calls` so the
    // only retention path is the binding map's GC root.
    vm.eval(
        "globalThis.calls = 0; \
         (function(){ \
             var ro = new ResizeObserver(function(){ calls++; }); \
             ro.observe(target); \
         })();",
    )
    .unwrap();
    // Force a GC immediately, before any delivery would re-root via
    // the JS-stack `entries`/`observer` args.
    vm.inner.collect_garbage();
    set_layout_box(&mut vm, target, Rect::new(0.0, 0.0, 100.0, 50.0));
    vm.deliver_resize_observations();

    let out = vm.eval("'' + calls").unwrap();
    let JsValue::String(sid) = out else {
        panic!("expected string, got {out:?}")
    };
    assert_eq!(
        vm.inner.strings.get_utf8(sid),
        "1",
        "callback must survive GC via HostData::resize_observer_bindings root"
    );

    vm.unbind();
}
