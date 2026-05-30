//! M4-12 D-22 `#11-intersection-observer-vm` — `IntersectionObserver`
//! thin VM binding tests.
//!
//! Covers prototype install, constructor + brand-check, init-dict
//! parsing (`root` / `rootMargin` / `threshold`), and
//! `Vm::deliver_intersection_observations` end-to-end against a
//! synthetic `LayoutBox` source.

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
fn intersection_observer_prototype_installed() {
    let mut vm = Vm::new();
    assert!(
        vm.inner.intersection_observer_prototype.is_some(),
        "IntersectionObserver.prototype must be allocated during register_globals"
    );
    let result = vm
        .eval("typeof IntersectionObserver === 'function'")
        .expect("typeof expression must not throw");
    assert_eq!(result, JsValue::Boolean(true));
}

#[test]
fn intersection_observer_constructor_creates_instance() {
    let out = run("var io = new IntersectionObserver(function(){}); typeof io;");
    assert_eq!(out, "object");
}

#[test]
fn intersection_observer_constructor_without_host_data_throws() {
    let mut vm = Vm::new();
    let err = vm
        .eval("new IntersectionObserver(function(){})")
        .expect_err("constructor must error pre-install_host_data");
    let err_text = format!("{err:?}");
    assert!(
        err_text.contains("host environment is not initialised"),
        "expected pre-init TypeError, got: {err_text}"
    );
}

#[test]
fn intersection_observer_constructor_requires_callable() {
    let err = run_throws("new IntersectionObserver(123);");
    assert!(
        err.contains("not of type 'Function'"),
        "expected callable TypeError, got: {err}"
    );
}

#[test]
fn intersection_observer_instanceof_works() {
    let out = run("var io = new IntersectionObserver(function(){}); \
         (io instanceof IntersectionObserver) ? 'yes' : 'no';");
    assert_eq!(out, "yes");
}

#[test]
fn intersection_observer_brand_check_methods() {
    for method in ["observe", "unobserve", "disconnect", "takeRecords"] {
        let err = run_throws(&format!(
            "IntersectionObserver.prototype.{method}.call({{}});"
        ));
        assert!(
            err.contains("Illegal invocation"),
            "expected brand-check TypeError for {method}, got: {err}"
        );
    }
}

#[test]
fn intersection_observer_observe_returns_undefined() {
    let out = run("var io = new IntersectionObserver(function(){}); \
         typeof io.observe(document);");
    assert_eq!(out, "undefined");
}

#[test]
fn intersection_observer_observe_target_must_be_node() {
    let err = run_throws("var io = new IntersectionObserver(function(){}); io.observe({});");
    assert!(
        err.contains("not of type 'Node'"),
        "expected non-Node TypeError, got: {err}"
    );
}

#[test]
fn intersection_observer_take_records_returns_empty_array() {
    // The VM does not buffer queued entries between frames — per-frame
    // delivery consumes them directly.  takeRecords thus always
    // returns `[]` (spec-compliant: queue-of-zero is well-defined).
    let out = run("var io = new IntersectionObserver(function(){}); \
         var r = io.takeRecords(); \
         Array.isArray(r) + ':' + r.length;");
    assert_eq!(out, "true:0");
}

#[test]
fn intersection_observer_threshold_out_of_range_throws() {
    let err = run_throws("new IntersectionObserver(function(){}, {threshold: 1.5});");
    assert!(
        err.contains("finite numbers in [0, 1]"),
        "expected threshold range RangeError, got: {err}"
    );
    let err = run_throws("new IntersectionObserver(function(){}, {threshold: [-0.1]});");
    assert!(
        err.contains("finite numbers in [0, 1]"),
        "expected threshold range RangeError (array), got: {err}"
    );
}

#[test]
fn intersection_observer_root_margin_invalid_unit_throws_syntax_error() {
    // W3C Intersection Observer §2.2 — `rootMargin` must be a valid
    // `<length-percentage>{1,4}`.  `em` / `vh` / bare numbers fail
    // SyntaxError, NOT TypeError (regression guard for the strict
    // crate-side parser).
    for bad in ["10em", "10vh", "10", "NaNpx"] {
        let err = run_throws(&format!(
            "new IntersectionObserver(function(){{}}, {{rootMargin: '{bad}'}});"
        ));
        assert!(
            err.contains("rootMargin token") && err.contains(bad),
            "expected SyntaxError citing '{bad}', got: {err}"
        );
    }
}

#[test]
fn intersection_observer_threshold_accepts_single_number_or_sequence() {
    // Spec §2.4 accepts `double` or `sequence<double>` — the parser
    // routes through `webidl_iter_to_vec` (the WebIDL §3.10.16
    // sequence helper, already merged as #11-webidl-sequence-helper-extraction
    // / #202), so `Array.prototype[@@iterator]` overrides and other
    // iterables are honoured uniformly.
    let out = run("try { \
            new IntersectionObserver(function(){}, {threshold: 0.5}); \
            new IntersectionObserver(function(){}, {threshold: [0.25, 0.75]}); \
            'ok' \
         } catch(e) { 'threw: ' + e.message; }");
    assert_eq!(out, "ok");
}

#[test]
fn intersection_observer_init_root_marshals_to_entity() {
    // Pass document as root — must be accepted without TypeError.  A
    // non-Node value must throw.
    let out = run(
        "try { new IntersectionObserver(function(){}, {root: document}); 'ok' } \
         catch(e) { 'threw: ' + e.message; }",
    );
    assert_eq!(out, "ok");
    let err = run_throws("new IntersectionObserver(function(){}, {root: 'not-a-node'});");
    assert!(
        err.contains("not of type 'Node'"),
        "expected non-Node root TypeError, got: {err}"
    );
}

// --- Delivery (deliver_intersection_observations) -------------------------

// VmInner default viewport (window.innerWidth × innerHeight) is 1024 × 768
// per `vm/host/window.rs::ViewportState::default`.  Tests that exercise
// rootMargin / cross-viewport positions are anchored against that.
const VIEWPORT_H: f32 = 768.0;

#[test]
fn intersection_observer_deliver_fires_callback_with_full_entry_shape() {
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
        "globalThis.calls = []; \
         globalThis.io = new IntersectionObserver(function(entries){ \
             for (var i = 0; i < entries.length; i++) calls.push(entries[i]); \
         }, {threshold: [0]}); \
         io.observe(target);",
    )
    .unwrap();

    // Fully-visible target inside the viewport.
    set_layout_box(&mut vm, target, Rect::new(10.0, 10.0, 100.0, 100.0));
    vm.deliver_intersection_observations();

    let out = vm
        .eval(
            "calls.length + '|' + \
             calls[0].isIntersecting + '|' + \
             calls[0].intersectionRatio + '|' + \
             calls[0].boundingClientRect.width + 'x' + \
             calls[0].boundingClientRect.height + '|' + \
             (calls[0].target === target ? 'same' : 'diff') + '|' + \
             (calls[0].rootBounds instanceof DOMRectReadOnly ? 'rect' : 'notrect') + '|' + \
             (typeof calls[0].time === 'number' ? 'numtime' : 'badtime')",
        )
        .unwrap();
    let JsValue::String(sid) = out else {
        panic!("expected string, got {out:?}")
    };
    assert_eq!(
        vm.inner.strings.get_utf8(sid),
        "1|true|1|100x100|same|rect|numtime"
    );

    vm.unbind();
}

#[test]
fn intersection_observer_deliver_box_less_target_delivers_initial_zero_once() {
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
        "globalThis.calls = 0; globalThis.last = null; \
         globalThis.io = new IntersectionObserver(function(entries){ \
             calls++; last = entries[0]; \
         }, {threshold: [0]}); \
         io.observe(target);",
    )
    .unwrap();

    // No LayoutBox attached → spec-mandated initial observation
    // (Intersection Observer §2.2).  Second deliver must not re-fire.
    vm.deliver_intersection_observations();
    vm.deliver_intersection_observations();

    let out = vm
        .eval(
            "calls + '|' + last.isIntersecting + '|' + last.intersectionRatio \
             + '|' + last.boundingClientRect.width",
        )
        .unwrap();
    let JsValue::String(sid) = out else {
        panic!("expected string, got {out:?}")
    };
    assert_eq!(vm.inner.strings.get_utf8(sid), "1|false|0|0");

    vm.unbind();
}

#[test]
fn intersection_observer_root_margin_expands_root_bounds() {
    // Target just below the viewport bottom; the 100px rootMargin
    // pulls it into the expanded root rect, so it intersects.
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
        "globalThis.calls = 0; globalThis.last = null; \
         globalThis.io = new IntersectionObserver(function(entries){ \
             calls++; last = entries[0]; \
         }, {rootMargin: '100px', threshold: [0]}); \
         io.observe(target);",
    )
    .unwrap();
    set_layout_box(
        &mut vm,
        target,
        Rect::new(10.0, VIEWPORT_H + 50.0, 100.0, 100.0),
    );
    vm.deliver_intersection_observations();

    let out = vm.eval("calls + '|' + last.isIntersecting").unwrap();
    let JsValue::String(sid) = out else {
        panic!("expected string, got {out:?}")
    };
    assert_eq!(vm.inner.strings.get_utf8(sid), "1|true");
    vm.unbind();
}

#[test]
fn intersection_observer_threshold_string_coerces_to_double() {
    // WebIDL §3.10.25 union resolution for `(double or sequence<double>)`:
    // a primitive String routes to the `double` branch (ToNumber), NOT
    // the sequence branch — even though `String.prototype[@@iterator]`
    // exists (code-point iteration).  Regression guard: a bug where
    // `parse_threshold` probed `@@iterator` on any value would iterate
    // `'.', '5'` and RangeError on the first NaN.
    let out = run("try { \
            var io = new IntersectionObserver(function(){}, {threshold: '0.5'}); \
            'ok' \
         } catch(e) { 'threw: ' + e.message; }");
    assert_eq!(out, "ok");
    // And a clearly invalid string still RangeErrors after ToNumber→NaN.
    let err = run_throws("new IntersectionObserver(function(){}, {threshold: 'not-a-number'});");
    assert!(
        err.contains("finite numbers in [0, 1]"),
        "expected NaN-coerced RangeError, got: {err}"
    );
}

#[test]
fn intersection_observer_reobserve_after_disconnect() {
    // W3C Intersection Observer §2.2: `disconnect()` stops observing
    // all targets but the observer stays usable.  Re-observe must
    // re-arm delivery — regression guard against eagerly removing
    // callback / instance maps on disconnect.
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
        "globalThis.calls = 0; globalThis.lastWidth = -1; \
         globalThis.io = new IntersectionObserver(function(entries){ \
             calls++; lastWidth = entries[0].boundingClientRect.width; \
         }, {threshold: [0]}); \
         io.observe(a);",
    )
    .unwrap();
    set_layout_box(&mut vm, a, Rect::new(10.0, 10.0, 100.0, 100.0));
    vm.deliver_intersection_observations();

    vm.eval("io.disconnect(); io.observe(b);").unwrap();
    set_layout_box(&mut vm, b, Rect::new(20.0, 20.0, 333.0, 50.0));
    vm.deliver_intersection_observations();

    let out = vm.eval("calls + '|' + lastWidth").unwrap();
    let JsValue::String(sid) = out else {
        panic!("expected string, got {out:?}")
    };
    assert_eq!(
        vm.inner.strings.get_utf8(sid),
        "2|333",
        "re-observe after disconnect must deliver to the new target"
    );

    vm.unbind();
}

#[test]
fn intersection_observer_disconnect_stops_delivery() {
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
         globalThis.io = new IntersectionObserver(function(){ calls++; }, {threshold: [0]}); \
         io.observe(target);",
    )
    .unwrap();

    set_layout_box(&mut vm, target, Rect::new(10.0, 10.0, 100.0, 100.0));
    vm.deliver_intersection_observations();
    vm.eval("io.disconnect();").unwrap();
    set_layout_box(&mut vm, target, Rect::new(2000.0, 2000.0, 100.0, 100.0));
    vm.deliver_intersection_observations();

    let out = vm.eval("'' + calls").unwrap();
    let JsValue::String(sid) = out else {
        panic!("expected string, got {out:?}")
    };
    assert_eq!(vm.inner.strings.get_utf8(sid), "1");
    vm.unbind();
}

#[test]
fn intersection_observer_two_observers_on_same_target_both_fire() {
    // `IntersectionObservedBy::0: Vec<IntersectionObservation>` design
    // supports multiple observers per target.  Both callbacks must
    // fire on a single delivery tick when the target crosses each
    // observer's threshold.
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
         globalThis.ioA = new IntersectionObserver(function(){ callsA++; }, {threshold:[0]}); \
         globalThis.ioB = new IntersectionObserver(function(){ callsB++; }, {threshold:[0]}); \
         ioA.observe(target); ioB.observe(target);",
    )
    .unwrap();
    set_layout_box(&mut vm, target, Rect::new(10.0, 10.0, 100.0, 100.0));
    vm.deliver_intersection_observations();

    let out = vm.eval("callsA + '|' + callsB").unwrap();
    let JsValue::String(sid) = out else {
        panic!("expected string, got {out:?}")
    };
    assert_eq!(vm.inner.strings.get_utf8(sid), "1|1");

    vm.unbind();
}

#[test]
fn intersection_observer_callback_survives_gc_via_root_chain() {
    // No JS-stack ref to `io` — the callback retention path is the
    // `gc_root_object_ids` chain via
    // `HostData::intersection_observer_bindings`.
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
         (function(){ \
             var io = new IntersectionObserver(function(){ calls++; }, {threshold:[0]}); \
             io.observe(target); \
         })();",
    )
    .unwrap();
    vm.inner.collect_garbage();
    set_layout_box(&mut vm, target, Rect::new(10.0, 10.0, 100.0, 100.0));
    vm.deliver_intersection_observations();

    let out = vm.eval("'' + calls").unwrap();
    let JsValue::String(sid) = out else {
        panic!("expected string, got {out:?}")
    };
    assert_eq!(
        vm.inner.strings.get_utf8(sid),
        "1",
        "callback must survive GC via HostData::intersection_observer_bindings root"
    );

    vm.unbind();
}

#[test]
fn intersection_observer_ctor_requires_new() {
    super::assert_ctor_requires_new("IntersectionObserver(function(){})", "IntersectionObserver");
}
