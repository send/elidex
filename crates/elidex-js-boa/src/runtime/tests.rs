use super::*;
use elidex_ecs::{Attributes, EcsDom};
use elidex_plugin::{EventPayload, MouseEventInit};

fn setup() -> (JsRuntime, SessionCore, EcsDom, Entity) {
    let runtime = JsRuntime::new();
    let session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    (runtime, session, dom, doc)
}

#[test]
fn add_event_listener_registers_in_ecs() {
    let (mut runtime, mut session, mut dom, doc) = setup();
    let div = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(doc, div);

    runtime.eval(
        r"
        var el = document.querySelector('div');
        el.addEventListener('click', function() {});
        ",
        &mut session,
        &mut dom,
        doc,
    );

    let listeners = dom
        .world()
        .get::<&elidex_script_session::EventListeners>(div)
        .unwrap();
    assert_eq!(listeners.len(), 1);
}

#[test]
fn remove_event_listener_clears() {
    let (mut runtime, mut session, mut dom, doc) = setup();
    let div = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(doc, div);

    runtime.eval(
        r"
        var handler = function() {};
        var el = document.querySelector('div');
        el.addEventListener('click', handler);
        el.removeEventListener('click', handler);
        ",
        &mut session,
        &mut dom,
        doc,
    );

    let listeners = dom
        .world()
        .get::<&elidex_script_session::EventListeners>(div)
        .unwrap();
    assert_eq!(listeners.len(), 0);
}

#[test]
fn duplicate_add_event_listener_ignored() {
    let (mut runtime, mut session, mut dom, doc) = setup();
    let div = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(doc, div);

    runtime.eval(
        r"
        var handler = function() {};
        var el = document.querySelector('div');
        el.addEventListener('click', handler);
        el.addEventListener('click', handler);
        ",
        &mut session,
        &mut dom,
        doc,
    );

    let listeners = dom
        .world()
        .get::<&elidex_script_session::EventListeners>(div)
        .unwrap();
    assert_eq!(listeners.len(), 1);
}

#[test]
fn capture_flag_mismatch_keeps_listener() {
    let (mut runtime, mut session, mut dom, doc) = setup();
    let div = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(doc, div);

    runtime.eval(
        r"
        var handler = function() {};
        var el = document.querySelector('div');
        el.addEventListener('click', handler, true);
        el.removeEventListener('click', handler, false);
        ",
        &mut session,
        &mut dom,
        doc,
    );

    let listeners = dom
        .world()
        .get::<&elidex_script_session::EventListeners>(div)
        .unwrap();
    assert_eq!(listeners.len(), 1);
}

#[test]
fn dispatch_event_invokes_listener() {
    let (mut runtime, mut session, mut dom, doc) = setup();
    let div = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(doc, div);

    runtime.eval(
        r"
        var el = document.querySelector('div');
        el.addEventListener('click', function(e) {
            e.target.textContent = 'clicked';
        });
        ",
        &mut session,
        &mut dom,
        doc,
    );
    session.flush(&mut dom);

    let mut event = DispatchEvent::new_composed("click", div);
    event.payload = EventPayload::Mouse(MouseEventInit {
        client_x: 50.0,
        client_y: 50.0,
        ..Default::default()
    });

    runtime.dispatch_event(&mut event, &mut session, &mut dom, doc);
    session.flush(&mut dom);

    let text = dom
        .world()
        .get::<&elidex_ecs::TextContent>(dom.get_first_child(div).unwrap())
        .map(|t| t.0.clone())
        .unwrap_or_default();
    assert_eq!(text, "clicked");
}

#[test]
fn dispatch_event_prevent_default() {
    let (mut runtime, mut session, mut dom, doc) = setup();
    let div = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(doc, div);

    runtime.eval(
        r"
        var el = document.querySelector('div');
        el.addEventListener('click', function(e) {
            e.preventDefault();
        });
        ",
        &mut session,
        &mut dom,
        doc,
    );
    session.flush(&mut dom);

    let mut event = DispatchEvent::new_composed("click", div);
    let prevented = runtime.dispatch_event(&mut event, &mut session, &mut dom, doc);
    assert!(prevented);
}

#[test]
fn dispatch_event_stop_propagation() {
    let (mut runtime, mut session, mut dom, doc) = setup();
    let outer = dom.create_element("div", Attributes::default());
    let inner = dom.create_element("span", Attributes::default());
    let _ = dom.append_child(doc, outer);
    let _ = dom.append_child(outer, inner);

    // Listener on inner that stops propagation.
    runtime.eval(
        r"
        var inner = document.querySelector('span');
        inner.addEventListener('click', function(e) {
            e.stopPropagation();
            console.log('inner-click');
        });
        ",
        &mut session,
        &mut dom,
        doc,
    );
    // Register outer listener separately.
    runtime.eval(
        r"
        var outer = document.querySelector('div');
        outer.addEventListener('click', function(e) {
            console.log('outer-click');
        });
        ",
        &mut session,
        &mut dom,
        doc,
    );
    session.flush(&mut dom);

    let mut event = DispatchEvent::new_composed("click", inner);
    runtime.dispatch_event(&mut event, &mut session, &mut dom, doc);

    let output = runtime.console_output().messages();
    let has_inner = output.iter().any(|m| m.1.contains("inner-click"));
    let has_outer = output.iter().any(|m| m.1.contains("outer-click"));
    assert!(
        has_inner,
        "inner listener should fire, messages: {output:?}"
    );
    assert!(
        !has_outer,
        "outer listener should NOT fire due to stopPropagation, messages: {output:?}"
    );
}

#[test]
fn event_mouse_properties() {
    let (mut runtime, mut session, mut dom, doc) = setup();
    let div = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(doc, div);

    runtime.eval(
        r"
        var el = document.querySelector('div');
        el.addEventListener('click', function(e) {
            console.log('x=' + e.clientX + ' y=' + e.clientY);
        });
        ",
        &mut session,
        &mut dom,
        doc,
    );
    session.flush(&mut dom);

    let mut event = DispatchEvent::new_composed("click", div);
    event.payload = EventPayload::Mouse(MouseEventInit {
        client_x: 123.0,
        client_y: 456.0,
        ..Default::default()
    });
    runtime.dispatch_event(&mut event, &mut session, &mut dom, doc);

    let output = runtime.console_output().messages();
    assert!(output
        .iter()
        .any(|m| m.1.contains("x=123") && m.1.contains("y=456")));
}

#[test]
fn event_keyboard_properties() {
    use elidex_plugin::KeyboardEventInit;

    let (mut runtime, mut session, mut dom, doc) = setup();
    let div = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(doc, div);

    runtime.eval(
        r"
        var el = document.querySelector('div');
        el.addEventListener('keydown', function(e) {
            console.log('key=' + e.key + ' code=' + e.code);
        });
        ",
        &mut session,
        &mut dom,
        doc,
    );
    session.flush(&mut dom);

    let mut event = DispatchEvent::new_composed("keydown", div);
    event.payload = EventPayload::Keyboard(KeyboardEventInit {
        key: "Enter".into(),
        code: "Enter".into(),
        ..Default::default()
    });
    runtime.dispatch_event(&mut event, &mut session, &mut dom, doc);

    let output = runtime.console_output().messages();
    assert!(output
        .iter()
        .any(|m| m.1.contains("key=Enter") && m.1.contains("code=Enter")));
}

#[test]
fn event_bubbles_to_parent() {
    let (mut runtime, mut session, mut dom, doc) = setup();
    let outer = dom.create_element("div", Attributes::default());
    let inner = dom.create_element("span", Attributes::default());
    let _ = dom.append_child(doc, outer);
    let _ = dom.append_child(outer, inner);

    runtime.eval(
        r"
        var outer = document.querySelector('div');
        outer.addEventListener('click', function(e) {
            console.log('bubbled');
        });
        ",
        &mut session,
        &mut dom,
        doc,
    );
    session.flush(&mut dom);

    // Dispatch on inner — should bubble to outer.
    let mut event = DispatchEvent::new_composed("click", inner);
    runtime.dispatch_event(&mut event, &mut session, &mut dom, doc);

    let output = runtime.console_output().messages();
    assert!(output.iter().any(|m| m.1.contains("bubbled")));
}

#[test]
fn listener_store_gc_trace() {
    // Verify that creating a runtime with listeners doesn't panic
    // during boa's GC cycle (which would happen if Trace is wrong).
    let (mut runtime, mut session, mut dom, doc) = setup();
    let div = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(doc, div);

    runtime.eval(
        r"
        var el = document.querySelector('div');
        el.addEventListener('click', function() {});
        el.addEventListener('keydown', function() {});
        // Force some allocations to potentially trigger GC.
        for (var i = 0; i < 100; i++) {
            var obj = { value: i };
        }
        ",
        &mut session,
        &mut dom,
        doc,
    );
    // If we get here without panic, GC trace is working.
}

// --- Promise / run_jobs integration tests ---

#[test]
fn eval_runs_promise_microtasks() {
    // Promise.resolve().then() callback should fire during eval
    // because run_jobs() is called while bridge is still bound.
    let (mut runtime, mut session, mut dom, doc) = setup();

    let result = runtime.eval(
        "var resolved = false;\
         Promise.resolve(42).then(function(v) { resolved = v; });",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success);

    // Check that the .then() callback ran.
    runtime.eval(
        "console.log('resolved=' + resolved);",
        &mut session,
        &mut dom,
        doc,
    );
    let messages = runtime.console_output().messages();
    assert!(
        messages.iter().any(|m| m.1.contains("resolved=42")),
        "Expected resolved=42 in console output, got: {messages:?}"
    );
}

#[test]
fn eval_promise_chain() {
    // Multi-step promise chain should fully resolve.
    let (mut runtime, mut session, mut dom, doc) = setup();

    runtime.eval(
        "var result = 0;\
         Promise.resolve(1)\
             .then(function(v) { return v + 1; })\
             .then(function(v) { return v * 3; })\
             .then(function(v) { result = v; });",
        &mut session,
        &mut dom,
        doc,
    );

    runtime.eval(
        "console.log('chain=' + result);",
        &mut session,
        &mut dom,
        doc,
    );
    let messages = runtime.console_output().messages();
    assert!(
        messages.iter().any(|m| m.1.contains("chain=6")),
        "Expected chain=6 (1+1=2, 2*3=6), got: {messages:?}"
    );
}

#[test]
fn dispatch_event_runs_promise_microtasks() {
    // Promise microtasks in event handlers should fire during dispatch.
    let (mut runtime, mut session, mut dom, doc) = setup();
    let div = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(doc, div);

    runtime.eval(
        "var asyncResult = '';\
         var el = document.querySelector('div');\
         el.addEventListener('click', function(e) {\
             Promise.resolve('async-ok').then(function(v) {\
                 asyncResult = v;\
             });\
         });",
        &mut session,
        &mut dom,
        doc,
    );
    session.flush(&mut dom);

    let mut event = DispatchEvent::new_composed("click", div);
    event.payload = EventPayload::Mouse(MouseEventInit {
        client_x: 10.0,
        client_y: 10.0,
        ..Default::default()
    });
    runtime.dispatch_event(&mut event, &mut session, &mut dom, doc);

    // Read the result.
    runtime.eval(
        "console.log('async=' + asyncResult);",
        &mut session,
        &mut dom,
        doc,
    );
    let messages = runtime.console_output().messages();
    assert!(
        messages.iter().any(|m| m.1.contains("async=async-ok")),
        "Expected async=async-ok, got: {messages:?}"
    );
}

#[test]
fn with_fetch_none_is_same_as_new() {
    // JsRuntime::new() and JsRuntime::with_fetch(None) should behave identically.
    let mut runtime = JsRuntime::with_fetch(None);
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();

    let result = runtime.eval("1 + 2", &mut session, &mut dom, doc);
    assert!(result.success);
}

// --- document.addEventListener / removeEventListener ---

#[test]
fn document_add_event_listener() {
    let (mut runtime, mut session, mut dom, doc) = setup();

    runtime.eval(
        r"
        document.addEventListener('DOMContentLoaded', function() {
            console.log('dcl-handler');
        });
        ",
        &mut session,
        &mut dom,
        doc,
    );

    let listeners = dom
        .world()
        .get::<&elidex_script_session::EventListeners>(doc)
        .unwrap();
    assert_eq!(listeners.len(), 1);
}

#[test]
fn document_remove_event_listener() {
    let (mut runtime, mut session, mut dom, doc) = setup();

    runtime.eval(
        r"
        var handler = function() {};
        document.addEventListener('load', handler);
        document.removeEventListener('load', handler);
        ",
        &mut session,
        &mut dom,
        doc,
    );

    let listeners = dom
        .world()
        .get::<&elidex_script_session::EventListeners>(doc)
        .unwrap();
    assert_eq!(listeners.len(), 0);
}

#[test]
fn document_event_listener_dispatch() {
    let (mut runtime, mut session, mut dom, doc) = setup();

    runtime.eval(
        r"
        document.addEventListener('DOMContentLoaded', function() {
            console.log('dcl-fired');
        });
        ",
        &mut session,
        &mut dom,
        doc,
    );
    session.flush(&mut dom);

    let mut event = DispatchEvent::new("DOMContentLoaded", doc);
    event.cancelable = false;
    runtime.dispatch_event(&mut event, &mut session, &mut dom, doc);

    let output = runtime.console_output().messages();
    assert!(
        output.iter().any(|m| m.1.contains("dcl-fired")),
        "Expected dcl-fired in console output, got: {output:?}"
    );
}

// --- M3.5-3: Legacy DOM API stubs ---

#[test]
fn document_all_is_undefined() {
    let (mut runtime, mut session, mut dom, doc) = setup();
    runtime.eval(
        r"console.log(typeof document.all);",
        &mut session,
        &mut dom,
        doc,
    );
    let output = runtime.console_output().messages();
    assert!(
        output.iter().any(|m| m.1.contains("undefined")),
        "Expected document.all to be undefined, got: {output:?}"
    );
}

#[test]
fn document_write_does_not_throw() {
    let (mut runtime, mut session, mut dom, doc) = setup();
    runtime.eval(
        r"
        document.write('<p>test</p>');
        document.writeln('test');
        console.log('survived');
        ",
        &mut session,
        &mut dom,
        doc,
    );
    let output = runtime.console_output().messages();
    assert!(
        output.iter().any(|m| m.1.contains("survived")),
        "Expected document.write to not throw, got: {output:?}"
    );
}

// --- M3.5-8: WebAssembly API ---

#[test]
fn webassembly_instantiate_and_call_export() {
    let (mut runtime, mut session, mut dom, doc) = setup();

    // Minimal Wasm module that exports an `add` function: (i32, i32) -> i32.
    // Pre-compiled WAT: (module (func (export "add") (param i32 i32) (result i32)
    //                      (i32.add (local.get 0) (local.get 1))))
    let wasm_bytes = wat::parse_str(
        r#"(module
            (func (export "add") (param i32 i32) (result i32)
                (i32.add (local.get 0) (local.get 1))
            )
        )"#,
    )
    .unwrap();

    // Build a JS array of the Wasm bytes.
    let bytes_array_str = format!(
        "[{}]",
        wasm_bytes
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>()
            .join(",")
    );

    let js = format!(
        r"
        var wasmBytes = {bytes_array_str};
        var result = null;
        WebAssembly.instantiate(wasmBytes).then(function(mod) {{
            result = mod.instance.exports.add(3, 4);
            console.log('wasm_result=' + result);
        }});
        "
    );

    runtime.eval(&js, &mut session, &mut dom, doc);

    let output = runtime.console_output().messages();
    assert!(
        output.iter().any(|m| m.1.contains("wasm_result=7")),
        "Expected wasm_result=7, got: {output:?}"
    );
}
