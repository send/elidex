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

#[test]
fn eval_microtask_pipeline_succeeds() {
    // Verify that the eval+microtask pipeline works end-to-end:
    // a Promise .then() callback sets a flag, and both eval and
    // microtask processing succeed.
    let (mut runtime, mut session, mut dom, doc) = setup();

    let result = runtime.eval(
        "var microtaskRan = false; \
         Promise.resolve().then(function() { microtaskRan = true; });",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "Expected eval to succeed");

    // Verify microtask actually ran.
    let log_result = runtime.eval(
        "console.log('microtask=' + microtaskRan);",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(log_result.success, "Expected logging eval to succeed");
    let messages = runtime.console_output().messages();
    assert!(
        messages.iter().any(|m| m.1.contains("microtask=true")),
        "Expected microtask to have run, got: {messages:?}"
    );
}

#[test]
fn eval_top_level_error_takes_priority() {
    // When eval itself fails, the error should be reported regardless of
    // microtask queue status.
    let (mut runtime, mut session, mut dom, doc) = setup();

    let result = runtime.eval(
        "throw new Error('top-level-boom');",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(!result.success, "Expected eval to fail");
    assert!(
        result
            .error
            .as_deref()
            .is_some_and(|e| e.contains("top-level-boom")),
        "Expected top-level error message, got: {:?}",
        result.error
    );
}

// --- Observer API tests ---

#[test]
fn mutation_observer_construct_and_observe() {
    let (mut runtime, mut session, mut dom, doc) = setup();
    let div = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(doc, div);

    let result = runtime.eval(
        r"
        var mo = new MutationObserver(function(records) {
            console.log('mo-callback records=' + records.length);
        });
        var el = document.querySelector('div');
        mo.observe(el, { childList: true });
        console.log('mo-created');
        ",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(
        result.success,
        "MutationObserver creation failed: {:?}",
        result.error
    );

    let output = runtime.console_output().messages();
    assert!(output.iter().any(|m| m.1.contains("mo-created")));
}

#[test]
fn mutation_observer_disconnect() {
    let (mut runtime, mut session, mut dom, doc) = setup();
    let div = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(doc, div);

    let result = runtime.eval(
        r"
        var mo = new MutationObserver(function(records) {});
        var el = document.querySelector('div');
        mo.observe(el, { attributes: true });
        mo.disconnect();
        console.log('disconnected');
        ",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "disconnect failed: {:?}", result.error);
}

#[test]
fn mutation_observer_take_records() {
    let (mut runtime, mut session, mut dom, doc) = setup();
    let div = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(doc, div);

    let result = runtime.eval(
        r"
        var mo = new MutationObserver(function(records) {});
        var el = document.querySelector('div');
        mo.observe(el, { attributes: true });
        var records = mo.takeRecords();
        console.log('records-len=' + records.length);
        ",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "takeRecords failed: {:?}", result.error);

    let output = runtime.console_output().messages();
    assert!(output.iter().any(|m| m.1.contains("records-len=0")));
}

#[test]
fn mutation_observer_callback_delivery() {
    let (mut runtime, mut session, mut dom, doc) = setup();
    let div = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(doc, div);

    runtime.eval(
        r"
        var moRecords = [];
        var mo = new MutationObserver(function(records) {
            moRecords = records;
        });
        var el = document.querySelector('div');
        mo.observe(el, { childList: true });
        ",
        &mut session,
        &mut dom,
        doc,
    );

    // Create a child-list mutation.
    let child = dom.create_element("span", Attributes::default());
    let record = elidex_script_session::MutationRecord {
        kind: elidex_script_session::MutationKind::ChildList,
        target: div,
        added_nodes: vec![child],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: None,
        old_value: None,
    };

    runtime.deliver_mutation_records(&[record], &mut session, &mut dom, doc);

    // Check that the callback was invoked.
    runtime.eval(
        "console.log('delivered=' + moRecords.length);",
        &mut session,
        &mut dom,
        doc,
    );
    let output = runtime.console_output().messages();
    assert!(
        output.iter().any(|m| m.1.contains("delivered=1")),
        "Expected MutationObserver callback to receive 1 record, got: {output:?}"
    );
}

#[test]
fn resize_observer_construct_and_observe() {
    let (mut runtime, mut session, mut dom, doc) = setup();
    let div = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(doc, div);

    let result = runtime.eval(
        r"
        var ro = new ResizeObserver(function(entries) {
            console.log('ro-callback');
        });
        var el = document.querySelector('div');
        ro.observe(el);
        console.log('ro-created');
        ",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(
        result.success,
        "ResizeObserver creation failed: {:?}",
        result.error
    );
}

#[test]
fn resize_observer_disconnect() {
    let (mut runtime, mut session, mut dom, doc) = setup();
    let div = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(doc, div);

    let result = runtime.eval(
        r"
        var ro = new ResizeObserver(function(entries) {});
        var el = document.querySelector('div');
        ro.observe(el);
        ro.disconnect();
        console.log('ro-disconnected');
        ",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "disconnect failed: {:?}", result.error);
}

#[test]
fn intersection_observer_construct_and_observe() {
    let (mut runtime, mut session, mut dom, doc) = setup();
    let div = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(doc, div);

    let result = runtime.eval(
        r"
        var io = new IntersectionObserver(function(entries) {
            console.log('io-callback');
        }, { threshold: [0, 0.5, 1] });
        var el = document.querySelector('div');
        io.observe(el);
        console.log('io-created');
        ",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(
        result.success,
        "IntersectionObserver creation failed: {:?}",
        result.error
    );
}

#[test]
fn intersection_observer_disconnect() {
    let (mut runtime, mut session, mut dom, doc) = setup();
    let div = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(doc, div);

    let result = runtime.eval(
        r"
        var io = new IntersectionObserver(function(entries) {});
        var el = document.querySelector('div');
        io.observe(el);
        io.disconnect();
        console.log('io-disconnected');
        ",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "disconnect failed: {:?}", result.error);
}

#[test]
fn mutation_observer_requires_callback() {
    let (mut runtime, mut session, mut dom, doc) = setup();

    let result = runtime.eval("new MutationObserver();", &mut session, &mut dom, doc);
    assert!(!result.success, "Expected error for missing callback");
}

#[test]
fn mutation_observer_observe_requires_type() {
    let (mut runtime, mut session, mut dom, doc) = setup();
    let div = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(doc, div);

    let result = runtime.eval(
        r"
        var mo = new MutationObserver(function(){});
        var el = document.querySelector('div');
        mo.observe(el, {}); // no childList/attributes/characterData
        ",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(!result.success, "Expected error for empty observe options");
}

// --- EventQueue / checkValidity integration ---

#[test]
fn check_validity_fires_invalid_event_to_js_listener() {
    let (mut runtime, mut session, mut dom, doc) = setup();

    // Create a required text input (empty value = invalid).
    let mut attrs = Attributes::default();
    attrs.set("type", "text");
    attrs.set("name", "field");
    attrs.set("required", "");
    let input = dom.create_element("input", attrs.clone());
    let fcs = elidex_form::FormControlState::from_element("input", &attrs).unwrap();
    let _ = dom.world_mut().insert_one(input, fcs);
    let _ = dom.append_child(doc, input);

    // Register JS listener for "invalid" event + call checkValidity().
    // The invalid event is deferred (enqueued), so it fires after eval returns.
    let result = runtime.eval(
        r"
        var invalidFired = false;
        var el = document.querySelector('input');
        el.addEventListener('invalid', function(e) {
            invalidFired = true;
        });
        var valid = el.checkValidity();
        ",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "eval failed: {:?}", result.error);

    // After eval, the deferred invalid event has been drained and dispatched.
    // Check the flag in a second eval.
    runtime.eval(
        "console.log('valid=' + valid + ' invalidFired=' + invalidFired);",
        &mut session,
        &mut dom,
        doc,
    );

    let output = runtime.console_output().messages();
    assert!(
        output
            .iter()
            .any(|m| m.1.contains("valid=false") && m.1.contains("invalidFired=true")),
        "Expected checkValidity to return false and fire invalid event, got: {output:?}"
    );
}

// ---------------------------------------------------------------------------
// Lifecycle events (M4-3.8 Step 1)
// ---------------------------------------------------------------------------

#[test]
fn readystate_initial_is_loading() {
    let (_runtime, session, _dom, _doc) = setup();
    assert_eq!(session.document_ready_state.as_str(), "loading");
}

#[test]
fn readystate_transitions_via_pipeline() {
    let (mut runtime, mut session, mut dom, doc) = setup();

    // Register listener for readystatechange.
    runtime.eval(
        r"
        var states = [];
        document.addEventListener('readystatechange', function() {
            states.push(document.readyState);
        });
        ",
        &mut session,
        &mut dom,
        doc,
    );

    // Simulate lifecycle dispatch (same as pipeline.rs dispatch_lifecycle_events).
    // Transition to Interactive.
    session.document_ready_state = elidex_script_session::ReadyState::Interactive;
    let mut rs_event = elidex_script_session::DispatchEvent::new("readystatechange", doc);
    rs_event.bubbles = false;
    rs_event.cancelable = false;
    runtime.dispatch_event(&mut rs_event, &mut session, &mut dom, doc);
    session.flush(&mut dom);

    // DOMContentLoaded.
    let mut dcl = elidex_script_session::DispatchEvent::new("DOMContentLoaded", doc);
    dcl.cancelable = false;
    runtime.dispatch_event(&mut dcl, &mut session, &mut dom, doc);
    session.flush(&mut dom);

    // Transition to Complete.
    session.document_ready_state = elidex_script_session::ReadyState::Complete;
    let mut rs_event2 = elidex_script_session::DispatchEvent::new("readystatechange", doc);
    rs_event2.bubbles = false;
    rs_event2.cancelable = false;
    runtime.dispatch_event(&mut rs_event2, &mut session, &mut dom, doc);
    session.flush(&mut dom);

    // Verify states captured by listener.
    runtime.eval(
        "console.log('states=' + JSON.stringify(states));",
        &mut session,
        &mut dom,
        doc,
    );

    let output = runtime.console_output().messages();
    assert!(
        output
            .iter()
            .any(|m| m.1.contains(r#"states=["interactive","complete"]"#)),
        "Expected readystatechange to capture interactive then complete, got: {output:?}"
    );
}

#[test]
fn domcontentloaded_fires() {
    let (mut runtime, mut session, mut dom, doc) = setup();

    runtime.eval(
        r"
        var dclFired = false;
        document.addEventListener('DOMContentLoaded', function() {
            dclFired = true;
        });
        ",
        &mut session,
        &mut dom,
        doc,
    );

    let mut dcl = elidex_script_session::DispatchEvent::new("DOMContentLoaded", doc);
    dcl.cancelable = false;
    runtime.dispatch_event(&mut dcl, &mut session, &mut dom, doc);

    runtime.eval(
        "console.log('dcl=' + dclFired);",
        &mut session,
        &mut dom,
        doc,
    );

    let output = runtime.console_output().messages();
    assert!(
        output.iter().any(|m| m.1.contains("dcl=true")),
        "DOMContentLoaded should fire, got: {output:?}"
    );
}

#[test]
fn load_event_fires() {
    let (mut runtime, mut session, mut dom, doc) = setup();

    // load fires on window (document target), does NOT bubble.
    runtime.eval(
        r"
        var loadFired = false;
        document.addEventListener('load', function() {
            loadFired = true;
        });
        ",
        &mut session,
        &mut dom,
        doc,
    );

    let mut load = elidex_script_session::DispatchEvent::new("load", doc);
    load.bubbles = false;
    load.cancelable = false;
    runtime.dispatch_event(&mut load, &mut session, &mut dom, doc);

    runtime.eval(
        "console.log('load=' + loadFired);",
        &mut session,
        &mut dom,
        doc,
    );

    let output = runtime.console_output().messages();
    assert!(
        output.iter().any(|m| m.1.contains("load=true")),
        "load event should fire, got: {output:?}"
    );
}

#[test]
fn beforeunload_can_cancel() {
    let (mut runtime, mut session, mut dom, doc) = setup();

    runtime.eval(
        r"
        document.addEventListener('beforeunload', function(e) {
            e.preventDefault();
        });
        ",
        &mut session,
        &mut dom,
        doc,
    );

    let mut beforeunload = elidex_script_session::DispatchEvent::new("beforeunload", doc);
    beforeunload.cancelable = true;
    beforeunload.bubbles = false;
    let prevented = runtime.dispatch_event(&mut beforeunload, &mut session, &mut dom, doc);

    // dispatch_event returns true when preventDefault() was called.
    assert!(
        prevented,
        "beforeunload with preventDefault should be prevented (return true)"
    );
}

// ---------------------------------------------------------------------------
// innerHTML setter + insertAdjacentHTML (M4-3.8 Step 2)
// ---------------------------------------------------------------------------

#[test]
fn innerhtml_setter_replaces_children() {
    let (mut runtime, mut session, mut dom, doc) = setup();
    let div = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(doc, div);
    let text = dom.create_text("old content");
    let _ = dom.append_child(div, text);

    // Set innerHTML and read it back in the same eval (session.flush happens inside eval).
    runtime.eval(
        "
        var el = document.querySelector('div');
        el.innerHTML = '<span>new</span>';
        console.log('html=' + el.innerHTML);
        ",
        &mut session,
        &mut dom,
        doc,
    );

    let output = runtime.console_output().messages();
    // innerHTML getter reads from DOM which reflects the mutation after flush.
    // However, flush happens after eval, so the getter sees pre-flush state.
    // The mutation is deferred — innerHTML won't be updated until flush.
    // For now, verify the mutation was recorded and applied after flush.
    session.flush(&mut dom);

    // Re-read after explicit flush.
    runtime.eval(
        "console.log('post=' + document.querySelector('div').innerHTML);",
        &mut session,
        &mut dom,
        doc,
    );

    let output2 = runtime.console_output().messages();
    assert!(
        output2.iter().any(|m| m.1.contains("<span>")),
        "innerHTML setter should replace children after flush, got: {output:?} + {output2:?}"
    );
}

#[test]
fn innerhtml_setter_empty_clears_children() {
    let (mut runtime, mut session, mut dom, doc) = setup();
    let div = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(doc, div);
    let text = dom.create_text("content");
    let _ = dom.append_child(div, text);

    runtime.eval(
        "document.querySelector('div').innerHTML = '';",
        &mut session,
        &mut dom,
        doc,
    );
    session.flush(&mut dom);

    // After flush, children should be removed.
    let children: Vec<_> = dom.children(div);
    assert!(
        children.is_empty(),
        "innerHTML='' should clear children, got {} children",
        children.len()
    );
}

#[test]
fn insert_adjacent_html_beforeend() {
    let (mut runtime, mut session, mut dom, doc) = setup();
    let div = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(doc, div);
    let text = dom.create_text("existing");
    let _ = dom.append_child(div, text);

    runtime.eval(
        "document.querySelector('div').insertAdjacentHTML('beforeend', '<b>added</b>');",
        &mut session,
        &mut dom,
        doc,
    );
    // Flush deferred mutation before reading innerHTML.
    session.flush(&mut dom);

    runtime.eval(
        "console.log('html=' + document.querySelector('div').innerHTML);",
        &mut session,
        &mut dom,
        doc,
    );

    let output = runtime.console_output().messages();
    assert!(
        output.iter().any(|m| m.1.contains("added")),
        "insertAdjacentHTML beforeend should append, got: {output:?}"
    );
}

#[test]
fn parse_html_fragment_basic() {
    let mut dom = elidex_ecs::EcsDom::new();
    let root = dom.create_document_root();
    let div = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(root, div);

    let nodes =
        elidex_html_parser::parse_html_fragment("<p>hello</p><p>world</p>", "div", div, &mut dom);

    assert_eq!(nodes.len(), 2, "should parse 2 <p> elements");
    // Verify first child is a <p>.
    let first_tag = dom
        .world()
        .get::<&elidex_ecs::TagType>(nodes[0])
        .ok()
        .map(|t| t.0.clone());
    assert_eq!(first_tag.as_deref(), Some("p"));
}

// ---------------------------------------------------------------------------
// Window APIs (M4-3.8 Step 4)
// ---------------------------------------------------------------------------

#[test]
fn window_inner_width_height() {
    let (mut runtime, mut session, mut dom, doc) = setup();

    runtime.eval(
        "console.log('w=' + innerWidth + ',h=' + innerHeight);",
        &mut session,
        &mut dom,
        doc,
    );

    let output = runtime.console_output().messages();
    // Default viewport is 800x600 from HostBridge initialization.
    assert!(
        output.iter().any(|m| m.1.contains("w=800")),
        "innerWidth should be 800, got: {output:?}"
    );
}

#[test]
fn window_match_media() {
    let (mut runtime, mut session, mut dom, doc) = setup();

    runtime.eval(
        "var mql = matchMedia('(max-width: 600px)'); console.log('m=' + mql.matches + ',q=' + mql.media);",
        &mut session,
        &mut dom,
        doc,
    );

    let output = runtime.console_output().messages();
    assert!(
        output.iter().any(|m| m.1.contains("m=false")),
        "matchMedia should return matches=false, got: {output:?}"
    );
    assert!(
        output.iter().any(|m| m.1.contains("q=(max-width: 600px)")),
        "matchMedia should echo query, got: {output:?}"
    );
}

#[test]
fn window_scroll_x_y_initial() {
    let (mut runtime, mut session, mut dom, doc) = setup();

    runtime.eval(
        "console.log('sx=' + scrollX + ',sy=' + scrollY);",
        &mut session,
        &mut dom,
        doc,
    );

    let output = runtime.console_output().messages();
    assert!(
        output
            .iter()
            .any(|m| m.1.contains("sx=0") && m.1.contains("sy=0")),
        "scrollX/Y should be 0 initially, got: {output:?}"
    );
}

// ---------------------------------------------------------------------------
// CSSOM tests
// ---------------------------------------------------------------------------

#[test]
fn document_stylesheets_empty_by_default() {
    let (mut runtime, mut session, mut dom, doc) = setup();

    let result = runtime.eval(
        "console.log('len=' + document.styleSheets.length);",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "eval should succeed: {:?}", result.error);

    let output = runtime.console_output().messages();
    assert!(
        output.iter().any(|m| m.1.contains("len=0")),
        "styleSheets should be empty by default, got: {output:?}"
    );
}

#[test]
fn document_stylesheets_reflects_bridge_state() {
    let (mut runtime, mut session, mut dom, doc) = setup();

    // Set up a stylesheet in the bridge.
    let sheet = crate::bridge::CssomSheet {
        rules: vec![crate::bridge::CssomRule {
            selector_text: "div".to_string(),
            declarations: vec![("color".to_string(), "red".to_string())],
        }],
    };
    runtime.bridge().set_stylesheets(vec![sheet]);

    let result = runtime.eval(
        r"
        var sheets = document.styleSheets;
        console.log('count=' + sheets.length);
        var sheet = sheets.item(0);
        console.log('type=' + sheet.type);
        ",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "eval should succeed: {:?}", result.error);

    let output = runtime.console_output().messages();
    assert!(
        output.iter().any(|m| m.1.contains("count=1")),
        "should have 1 stylesheet, got: {output:?}"
    );
    assert!(
        output.iter().any(|m| m.1.contains("type=text/css")),
        "sheet type should be text/css, got: {output:?}"
    );
}

#[test]
fn css_rules_access() {
    let (mut runtime, mut session, mut dom, doc) = setup();

    let sheet = crate::bridge::CssomSheet {
        rules: vec![
            crate::bridge::CssomRule {
                selector_text: "div".to_string(),
                declarations: vec![("color".to_string(), "red".to_string())],
            },
            crate::bridge::CssomRule {
                selector_text: "p".to_string(),
                declarations: vec![("margin".to_string(), "10px".to_string())],
            },
        ],
    };
    runtime.bridge().set_stylesheets(vec![sheet]);

    let result = runtime.eval(
        r"
        var rules = document.styleSheets.item(0).cssRules;
        console.log('rlen=' + rules.length);
        console.log('sel0=' + rules[0].selectorText);
        console.log('sel1=' + rules[1].selectorText);
        console.log('type0=' + rules[0].type);
        console.log('css0=' + rules[0].cssText);
        ",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "eval should succeed: {:?}", result.error);

    let output = runtime.console_output().messages();
    assert!(
        output.iter().any(|m| m.1.contains("rlen=2")),
        "got: {output:?}"
    );
    assert!(
        output.iter().any(|m| m.1.contains("sel0=div")),
        "got: {output:?}"
    );
    assert!(
        output.iter().any(|m| m.1.contains("sel1=p")),
        "got: {output:?}"
    );
    assert!(
        output.iter().any(|m| m.1.contains("type0=1")),
        "got: {output:?}"
    );
    assert!(
        output
            .iter()
            .any(|m| m.1.contains("css0=div { color: red }")),
        "got: {output:?}"
    );
}

#[test]
fn css_rule_style_declaration() {
    let (mut runtime, mut session, mut dom, doc) = setup();

    let sheet = crate::bridge::CssomSheet {
        rules: vec![crate::bridge::CssomRule {
            selector_text: "body".to_string(),
            declarations: vec![
                ("color".to_string(), "blue".to_string()),
                ("font-size".to_string(), "16px".to_string()),
            ],
        }],
    };
    runtime.bridge().set_stylesheets(vec![sheet]);

    let result = runtime.eval(
        r"
        var style = document.styleSheets.item(0).cssRules[0].style;
        console.log('slen=' + style.length);
        console.log('item0=' + style.item(0));
        console.log('gpv=' + style.getPropertyValue('color'));
        console.log('csstext=' + style.cssText);
        ",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "eval should succeed: {:?}", result.error);

    let output = runtime.console_output().messages();
    assert!(
        output.iter().any(|m| m.1.contains("slen=2")),
        "got: {output:?}"
    );
    assert!(
        output.iter().any(|m| m.1.contains("gpv=blue")),
        "got: {output:?}"
    );
}

#[test]
fn insert_rule_and_delete_rule() {
    let (mut runtime, mut session, mut dom, doc) = setup();

    let sheet = crate::bridge::CssomSheet {
        rules: vec![crate::bridge::CssomRule {
            selector_text: "div".to_string(),
            declarations: vec![("color".to_string(), "red".to_string())],
        }],
    };
    runtime.bridge().set_stylesheets(vec![sheet]);

    // insertRule
    let result = runtime.eval(
        r"
        var sheet = document.styleSheets.item(0);
        var idx = sheet.insertRule('p { margin: 0 }', 1);
        console.log('idx=' + idx);
        // Read fresh cssRules after insert.
        var rules = sheet.cssRules;
        console.log('after_insert=' + rules.length);
        console.log('new_sel=' + rules[1].selectorText);
        ",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "eval should succeed: {:?}", result.error);

    let output = runtime.console_output().messages();
    assert!(
        output.iter().any(|m| m.1.contains("idx=1")),
        "got: {output:?}"
    );
    assert!(
        output.iter().any(|m| m.1.contains("after_insert=2")),
        "got: {output:?}"
    );
    assert!(
        output.iter().any(|m| m.1.contains("new_sel=p")),
        "got: {output:?}"
    );

    // Verify pending mutations.
    let mutations = runtime.bridge().take_cssom_mutations();
    assert_eq!(mutations.len(), 1, "should have 1 pending mutation");

    // deleteRule
    let result = runtime.eval(
        r"
        var sheet = document.styleSheets.item(0);
        sheet.deleteRule(0);
        var rules = sheet.cssRules;
        console.log('after_delete=' + rules.length);
        console.log('remaining_sel=' + rules[0].selectorText);
        ",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "eval should succeed: {:?}", result.error);

    let output2 = runtime.console_output().messages();
    assert!(
        output2.iter().any(|m| m.1.contains("after_delete=1")),
        "got: {output2:?}"
    );
    assert!(
        output2.iter().any(|m| m.1.contains("remaining_sel=p")),
        "got: {output2:?}"
    );
}

#[test]
fn insert_rule_invalid_index_throws() {
    let (mut runtime, mut session, mut dom, doc) = setup();

    let sheet = crate::bridge::CssomSheet { rules: vec![] };
    runtime.bridge().set_stylesheets(vec![sheet]);

    // Index 5 is out of range for an empty sheet.
    let result = runtime.eval(
        "document.styleSheets.item(0).insertRule('p { }', 5);",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(!result.success, "insertRule with invalid index should fail");
}

#[test]
fn delete_rule_invalid_index_throws() {
    let (mut runtime, mut session, mut dom, doc) = setup();

    let sheet = crate::bridge::CssomSheet { rules: vec![] };
    runtime.bridge().set_stylesheets(vec![sheet]);

    let result = runtime.eval(
        "document.styleSheets.item(0).deleteRule(0);",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(!result.success, "deleteRule on empty sheet should fail");
}

#[test]
fn style_css_text_getter() {
    let (mut runtime, mut session, mut dom, doc) = setup();
    let div = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(doc, div);

    let result = runtime.eval(
        r"
        var el = document.querySelector('div');
        el.style.setProperty('color', 'red');
        el.style.setProperty('margin', '10px');
        console.log('len=' + el.style.length);
        // cssText should contain both properties.
        var text = el.style.cssText;
        console.log('has_color=' + (text.indexOf('color') >= 0));
        console.log('has_margin=' + (text.indexOf('margin') >= 0));
        ",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "eval should succeed: {:?}", result.error);

    let output = runtime.console_output().messages();
    assert!(
        output.iter().any(|m| m.1.contains("len=2")),
        "got: {output:?}"
    );
    assert!(
        output.iter().any(|m| m.1.contains("has_color=true")),
        "got: {output:?}"
    );
    assert!(
        output.iter().any(|m| m.1.contains("has_margin=true")),
        "got: {output:?}"
    );
}

#[test]
fn style_css_text_setter() {
    let (mut runtime, mut session, mut dom, doc) = setup();
    let div = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(doc, div);

    let result = runtime.eval(
        r"
        var el = document.querySelector('div');
        el.style.cssText = 'color: blue; font-size: 14px';
        console.log('len=' + el.style.length);
        console.log('color=' + el.style.getPropertyValue('color'));
        console.log('fs=' + el.style.getPropertyValue('font-size'));
        ",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "eval should succeed: {:?}", result.error);

    let output = runtime.console_output().messages();
    assert!(
        output.iter().any(|m| m.1.contains("len=2")),
        "got: {output:?}"
    );
    assert!(
        output.iter().any(|m| m.1.contains("color=blue")),
        "got: {output:?}"
    );
    assert!(
        output.iter().any(|m| m.1.contains("fs=14px")),
        "got: {output:?}"
    );
}

#[test]
fn style_item_method() {
    let (mut runtime, mut session, mut dom, doc) = setup();
    let div = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(doc, div);

    let result = runtime.eval(
        r"
        var el = document.querySelector('div');
        el.style.setProperty('color', 'red');
        var name = el.style.item(0);
        // item(0) should return a property name string.
        console.log('type=' + typeof name);
        console.log('empty=' + (name.length > 0));
        // Out-of-range returns empty string.
        console.log('oob=' + el.style.item(99));
        ",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "eval should succeed: {:?}", result.error);

    let output = runtime.console_output().messages();
    assert!(
        output.iter().any(|m| m.1.contains("type=string")),
        "got: {output:?}"
    );
    assert!(
        output.iter().any(|m| m.1.contains("empty=true")),
        "got: {output:?}"
    );
}

#[test]
fn stylesheets_item_returns_null_for_invalid_index() {
    let (mut runtime, mut session, mut dom, doc) = setup();

    let result = runtime.eval(
        r"
        var result = document.styleSheets.item(99);
        console.log('null=' + (result === null));
        ",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "eval should succeed: {:?}", result.error);

    let output = runtime.console_output().messages();
    assert!(
        output.iter().any(|m| m.1.contains("null=true")),
        "got: {output:?}"
    );
}
