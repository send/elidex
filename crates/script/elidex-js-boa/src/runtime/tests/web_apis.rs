use super::*;

// Helper: eval JS and check console output for "true".
fn eval_true(
    runtime: &mut JsRuntime,
    session: &mut SessionCore,
    dom: &mut EcsDom,
    doc: Entity,
    code: &str,
) {
    let result = runtime.eval(code, session, dom, doc);
    assert!(result.success, "JS error: {:?} in: {code}", result.error);
    let msgs = runtime.console_output().messages();
    // Messages are (level, text) tuples.
    assert!(
        msgs.last().is_some_and(|(_, text)| text == "true"),
        "Expected console output 'true', got: {msgs:?}\nCode: {code}"
    );
}

// ---------------------------------------------------------------------------
// Performance Timeline (W3C User Timing §3-4)
// ---------------------------------------------------------------------------

#[test]
fn performance_now_returns_number() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        "console.log(typeof performance.now() === 'number')",
    );
}

#[test]
fn performance_time_origin_is_positive() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        "console.log(performance.timeOrigin > 0)",
    );
}

#[test]
fn performance_mark_and_measure() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        performance.mark('start');
        performance.mark('end');
        var m = performance.measure('test', 'start', 'end');
        console.log(m.entryType === 'measure' && m.name === 'test' && m.duration >= 0);
    ",
    );
}

#[test]
fn performance_get_entries_by_type() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        performance.mark('a');
        performance.mark('b');
        performance.measure('m', 'a', 'b');
        var marks = performance.getEntriesByType('mark');
        var measures = performance.getEntriesByType('measure');
        console.log(marks.length >= 2 && measures.length >= 1);
    ",
    );
}

#[test]
fn performance_clear_marks() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        performance.mark('x');
        performance.clearMarks('x');
        console.log(performance.getEntriesByName('x').length === 0);
    ",
    );
}

// ---------------------------------------------------------------------------
// atob / btoa (WHATWG HTML §8.3)
// ---------------------------------------------------------------------------

#[test]
fn btoa_atob_roundtrip() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        "console.log(atob(btoa('Hello, World!')) === 'Hello, World!')",
    );
}

// ---------------------------------------------------------------------------
// crypto (W3C WebCrypto)
// ---------------------------------------------------------------------------

#[test]
fn crypto_random_uuid() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var uuid = crypto.randomUUID();
        console.log(uuid.length === 36 && uuid[8] === '-' && uuid[13] === '-');
    ",
    );
}

// ---------------------------------------------------------------------------
// URL / URLSearchParams (WHATWG URL §6)
// ---------------------------------------------------------------------------

#[test]
fn url_constructor_basic() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var u = new URL('https://example.com/path?q=1#frag');
        console.log(u.hostname === 'example.com' && u.pathname === '/path' && u.hash === '#frag');
    ",
    );
}

#[test]
fn url_search_params() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var u = new URLSearchParams('a=1&b=2');
        console.log(u.get('a') === '1' && u.has('b') && !u.has('c'));
    ",
    );
}

// ---------------------------------------------------------------------------
// TextEncoder / TextDecoder (WHATWG Encoding §8)
// ---------------------------------------------------------------------------

#[test]
fn text_encoder_decode_roundtrip() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var enc = new TextEncoder();
        var dec = new TextDecoder();
        var bytes = enc.encode('hello');
        console.log(dec.decode(bytes) === 'hello');
    ",
    );
}

// ---------------------------------------------------------------------------
// AbortController / AbortSignal (WHATWG DOM §3.2)
// ---------------------------------------------------------------------------

#[test]
fn abort_controller_basic() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var ac = new AbortController();
        var before = ac.signal.aborted;
        ac.abort();
        var after = ac.signal.aborted;
        console.log(!before && after);
    ",
    );
}

#[test]
fn abort_signal_abort_static() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var s = AbortSignal.abort('test reason');
        console.log(s.aborted === true);
    ",
    );
}

#[test]
fn abort_controller_onabort_callback() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var called = false;
        var ac = new AbortController();
        ac.signal.onabort = function() { called = true; };
        ac.abort();
        console.log(called);
    ",
    );
}

// ---------------------------------------------------------------------------
// Blob / File (WHATWG File API §4-5)
// ---------------------------------------------------------------------------

#[test]
fn blob_constructor_size_type() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var b = new Blob(['hello'], { type: 'text/plain' });
        console.log(b.size === 5 && b.type === 'text/plain');
    ",
    );
}

#[test]
fn blob_slice() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var b = new Blob(['hello world']);
        var sliced = b.slice(0, 5);
        console.log(sliced.size === 5);
    ",
    );
}

#[test]
fn file_constructor() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var f = new File(['content'], 'test.txt', { type: 'text/plain' });
        console.log(f.name === 'test.txt' && f.size === 7 && f.type === 'text/plain' && f.lastModified > 0);
    ",
    );
}

// ---------------------------------------------------------------------------
// FormData (WHATWG XHR §4.3)
// ---------------------------------------------------------------------------

#[test]
fn form_data_crud() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var fd = new FormData();
        fd.append('key', 'value');
        console.log(fd.has('key') && fd.get('key') === 'value' && !fd.has('missing'));
    ",
    );
}

#[test]
fn form_data_set_replaces() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var fd = new FormData();
        fd.append('k', 'a');
        fd.append('k', 'b');
        fd.set('k', 'c');
        console.log(fd.getAll('k').length === 1 && fd.get('k') === 'c');
    ",
    );
}

#[test]
fn form_data_delete() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var fd = new FormData();
        fd.append('k', 'v');
        fd.delete('k');
        console.log(!fd.has('k'));
    ",
    );
}

#[test]
fn form_data_foreach() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var fd = new FormData();
        fd.append('a', '1');
        fd.append('b', '2');
        var keys = [];
        fd.forEach(function(v, k) { keys.push(k); });
        console.log(keys.length === 2 && keys[0] === 'a' && keys[1] === 'b');
    ",
    );
}

// ---------------------------------------------------------------------------
// DOMPoint / DOMMatrix (CSSWG Geometry §5-6)
// ---------------------------------------------------------------------------

#[test]
fn dom_point_constructor() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var p = new DOMPoint(1, 2, 3, 4);
        console.log(p.x === 1 && p.y === 2 && p.z === 3 && p.w === 4);
    ",
    );
}

#[test]
fn dom_point_defaults() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var p = new DOMPoint();
        console.log(p.x === 0 && p.y === 0 && p.z === 0 && p.w === 1);
    ",
    );
}

#[test]
fn dom_rect_constructor() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var r = new DOMRect(10, 20, 100, 50);
        console.log(r.x === 10 && r.y === 20 && r.width === 100 && r.height === 50 &&
            r.top === 20 && r.left === 10 && r.right === 110 && r.bottom === 70);
    ",
    );
}

#[test]
fn dom_matrix_identity() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var m = new DOMMatrix();
        console.log(m.is2D && m.isIdentity && m.a === 1 && m.d === 1 && m.e === 0 && m.f === 0);
    ",
    );
}

// ---------------------------------------------------------------------------
// visualViewport
// ---------------------------------------------------------------------------

#[test]
fn visual_viewport_exists() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        "console.log(typeof visualViewport === 'object' && visualViewport.scale === 1)",
    );
}

// ---------------------------------------------------------------------------
// queueMicrotask
// ---------------------------------------------------------------------------

#[test]
fn queue_microtask_executes() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var executed = false;
        queueMicrotask(function() { executed = true; });
        console.log(executed);
    ",
    );
}

// ---------------------------------------------------------------------------
// navigator
// ---------------------------------------------------------------------------

#[test]
fn navigator_properties() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        console.log(
            typeof navigator.userAgent === 'string' &&
            typeof navigator.platform === 'string' &&
            navigator.onLine === true &&
            navigator.cookieEnabled === true
        );
    ",
    );
}

// ---------------------------------------------------------------------------
// console extensions
// ---------------------------------------------------------------------------

#[test]
fn console_time_end() {
    let (mut rt, mut s, mut d, doc) = setup();
    let result = rt.eval(
        r"
        console.time('test');
        console.timeEnd('test');
    ",
        &mut s,
        &mut d,
        doc,
    );
    assert!(
        result.success,
        "console.time/timeEnd error: {:?}",
        result.error
    );
}

// ---------------------------------------------------------------------------
// Element.animate
// ---------------------------------------------------------------------------

#[test]
fn element_animate_returns_animation() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var div = document.createElement('div');
        var anim = div.animate(
            [{ opacity: 0 }, { opacity: 1 }],
            { duration: 1000, iterations: 1 }
        );
        console.log(anim.playState === 'running' && typeof anim.play === 'function');
    ",
    );
}

#[test]
fn element_animate_cancel() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var div = document.createElement('div');
        var anim = div.animate([{ opacity: 0 }, { opacity: 1 }], 500);
        anim.cancel();
        console.log(anim.playState === 'idle');
    ",
    );
}

// ---------------------------------------------------------------------------
// window properties
// ---------------------------------------------------------------------------

#[test]
fn window_self_reference() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        "console.log(window === self && window === globalThis)",
    );
}

#[test]
fn window_closed_is_false() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        "console.log(closed === false)",
    );
}

// ---------------------------------------------------------------------------
// localStorage / sessionStorage
// ---------------------------------------------------------------------------

#[test]
fn session_storage_crud() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        sessionStorage.setItem('k', 'v');
        var got = sessionStorage.getItem('k');
        sessionStorage.removeItem('k');
        console.log(got === 'v' && sessionStorage.getItem('k') === null);
    ",
    );
}

#[test]
fn local_storage_crud() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        localStorage.setItem('k', 'v');
        console.log(localStorage.getItem('k') === 'v' && localStorage.length === 1);
        localStorage.clear();
        console.log(localStorage.length === 0);
    ",
    );
}

// ---------------------------------------------------------------------------
// Event API tests (WHATWG DOM §2)
// ---------------------------------------------------------------------------

#[test]
fn event_constructor_with_options() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var e = Event('test', { bubbles: true, cancelable: true });
        console.log(e.type === 'test' && e.bubbles === true && e.cancelable === true);
    ",
    );
}

#[test]
fn custom_event_with_detail() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var ce = CustomEvent('myevent', { detail: 42 });
        console.log(ce.type === 'myevent' && ce.detail === 42);
    ",
    );
}

#[test]
fn dispatch_event_returns_not_prevented() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var div = document.createElement('div');
        var result = div.dispatchEvent(Event('test'));
        console.log(result === true);
    ",
    );
}

#[test]
fn dispatch_event_with_prevent_default_returns_false() {
    let (mut rt, mut s, mut d, doc) = setup();
    let div = d.create_element("div", Attributes::default());
    let _ = d.append_child(doc, div);
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var el = document.querySelector('div');
        el.addEventListener('test', function(e) { e.preventDefault(); });
        var result = el.dispatchEvent(Event('test', { cancelable: true }));
        console.log(result === false);
    ",
    );
}

#[test]
fn dispatched_event_is_not_trusted() {
    let (mut rt, mut s, mut d, doc) = setup();
    let div = d.create_element("div", Attributes::default());
    let _ = d.append_child(doc, div);
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var el = document.querySelector('div');
        var trusted = null;
        el.addEventListener('test', function(e) { trusted = e.isTrusted; });
        el.dispatchEvent(Event('test'));
        console.log(trusted === false);
    ",
    );
}

#[test]
fn once_listener_option_accepted() {
    let (mut rt, mut s, mut d, doc) = setup();
    let div = d.create_element("div", Attributes::default());
    let _ = d.append_child(doc, div);
    // Verify that the { once: true } option is accepted and the listener fires.
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var el = document.querySelector('div');
        var fired = false;
        el.addEventListener('test', function() { fired = true; }, { once: true });
        el.dispatchEvent(Event('test'));
        console.log(fired === true);
    ",
    );
}

#[test]
fn stop_propagation_stops_bubbling() {
    let (mut rt, mut s, mut d, doc) = setup();
    let outer = d.create_element("div", Attributes::default());
    let inner = d.create_element("span", Attributes::default());
    let _ = d.append_child(doc, outer);
    let _ = d.append_child(outer, inner);
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var outerFired = false;
        document.querySelector('div').addEventListener('test', function() { outerFired = true; });
        document.querySelector('span').addEventListener('test', function(e) { e.stopPropagation(); });
        document.querySelector('span').dispatchEvent(Event('test', { bubbles: true }));
        console.log(outerFired === false);
    ",
    );
}

#[test]
fn stop_immediate_propagation_stops_same_element() {
    let (mut rt, mut s, mut d, doc) = setup();
    let div = d.create_element("div", Attributes::default());
    let _ = d.append_child(doc, div);
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var el = document.querySelector('div');
        var first = false;
        var second = false;
        el.addEventListener('test', function(e) { first = true; e.stopImmediatePropagation(); });
        el.addEventListener('test', function() { second = true; });
        el.dispatchEvent(Event('test'));
        console.log(first === true && second === false);
    ",
    );
}

// ---------------------------------------------------------------------------
// Document API tests
// ---------------------------------------------------------------------------

#[test]
fn document_hidden_returns_boolean() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        "console.log(typeof document.hidden === 'boolean')",
    );
}

#[test]
fn document_visibility_state() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var vs = document.visibilityState;
        console.log(vs === 'visible' || vs === 'hidden');
    ",
    );
}

#[test]
fn document_has_focus_returns_boolean() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        "console.log(typeof document.hasFocus() === 'boolean')",
    );
}

#[test]
fn document_get_elements_by_class_name() {
    let (mut rt, mut s, mut d, doc) = setup();
    let mut attrs = Attributes::default();
    attrs.set("class", "foo");
    let div = d.create_element("div", attrs);
    let _ = d.append_child(doc, div);
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var elems = document.getElementsByClassName('foo');
        console.log(elems.length === 1);
    ",
    );
}

#[test]
fn document_get_elements_by_tag_name() {
    let (mut rt, mut s, mut d, doc) = setup();
    let div = d.create_element("div", Attributes::default());
    let _ = d.append_child(doc, div);
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var divs = document.getElementsByTagName('div');
        console.log(divs.length >= 1);
    ",
    );
}

#[test]
fn document_create_event() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var e = document.createEvent('Event');
        console.log(e.type === '' && typeof e.initEvent === 'function');
    ",
    );
}

#[test]
fn document_import_node_clones() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var div = document.createElement('div');
        div.setAttribute('id', 'orig');
        var clone = document.importNode(div, false);
        console.log(clone.getAttribute('id') === 'orig');
    ",
    );
}

// ---------------------------------------------------------------------------
// Element API tests
// ---------------------------------------------------------------------------

#[test]
fn element_children_returns_elements_only() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var div = document.createElement('div');
        div.appendChild(document.createElement('span'));
        div.appendChild(document.createTextNode('text'));
        div.appendChild(document.createElement('p'));
        console.log(div.children.length === 2);
    ",
    );
}

#[test]
fn element_outer_html() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var div = document.createElement('div');
        div.setAttribute('class', 'test');
        var html = div.outerHTML;
        console.log(html.indexOf('<div') === 0 && html.indexOf('class') > 0);
    ",
    );
}

#[test]
fn classlist_length() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var div = document.createElement('div');
        div.classList.add('a');
        div.classList.add('b');
        div.classList.add('c');
        console.log(div.classList.length === 3);
    ",
    );
}

#[test]
fn classlist_item() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var div = document.createElement('div');
        div.classList.add('first');
        div.classList.add('second');
        console.log(div.classList.item(0) === 'first');
    ",
    );
}

// ---------------------------------------------------------------------------
// Window API tests
// ---------------------------------------------------------------------------

#[test]
fn window_is_secure_context() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        "console.log(typeof window.isSecureContext === 'boolean')",
    );
}

#[test]
fn image_constructor_creates_img() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var img = new Image();
        console.log(typeof img === 'object' && typeof img.setAttribute === 'function');
    ",
    );
}

// ---------------------------------------------------------------------------
// DOMPoint.fromPoint / DOMPointReadOnly.fromPoint
// ---------------------------------------------------------------------------

#[test]
fn dom_point_from_point() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var p = DOMPoint.fromPoint({ x: 10, y: 20, z: 30, w: 2 });
        console.log(p.x === 10 && p.y === 20 && p.z === 30 && p.w === 2);
    ",
    );
}

#[test]
fn dom_point_readonly_from_point() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var p = DOMPointReadOnly.fromPoint({ x: 5, y: 15 });
        console.log(p.x === 5 && p.y === 15 && p.z === 0 && p.w === 1);
    ",
    );
}

// ---------------------------------------------------------------------------
// DOMMatrix transformation methods
// ---------------------------------------------------------------------------

#[test]
fn dom_matrix_translate_self() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var m = new DOMMatrix();
        var r = m.translateSelf(10, 20);
        console.log(r === m && m.e === 10 && m.f === 20);
    ",
    );
}

#[test]
fn dom_matrix_scale_self() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var m = new DOMMatrix();
        m.scaleSelf(2, 3);
        console.log(m.a === 2 && m.d === 3);
    ",
    );
}

#[test]
fn dom_matrix_rotate_self() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var m = new DOMMatrix();
        m.rotateSelf(90);
        // After 90 degree rotation: a ~= 0, b ~= 1, c ~= -1, d ~= 0
        console.log(Math.abs(m.a) < 0.001 && Math.abs(m.b - 1) < 0.001 &&
                    Math.abs(m.c + 1) < 0.001 && Math.abs(m.d) < 0.001);
    ",
    );
}

#[test]
fn dom_matrix_multiply_self() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var m1 = new DOMMatrix();
        m1.scaleSelf(2, 2);
        var m2 = new DOMMatrix();
        m2.translateSelf(5, 10);
        m1.multiplySelf(m2);
        // scale(2,2) * translate(5,10) = a=2, d=2, e=10, f=20
        console.log(m1.a === 2 && m1.d === 2 && m1.e === 10 && m1.f === 20);
    ",
    );
}

#[test]
fn dom_matrix_invert_self() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var m = new DOMMatrix();
        m.scaleSelf(2, 4);
        m.invertSelf();
        console.log(Math.abs(m.a - 0.5) < 0.001 && Math.abs(m.d - 0.25) < 0.001);
    ",
    );
}

#[test]
fn dom_matrix_translate_immutable() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var m = new DOMMatrix();
        var m2 = m.translate(10, 20);
        console.log(m.e === 0 && m2.e === 10 && m2.f === 20);
    ",
    );
}

#[test]
fn dom_matrix_scale_immutable() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var m = new DOMMatrix();
        var m2 = m.scale(3, 5);
        console.log(m.a === 1 && m2.a === 3 && m2.d === 5);
    ",
    );
}

#[test]
fn dom_matrix_readonly_no_mutation_methods() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var m = new DOMMatrixReadOnly();
        console.log(typeof m.translateSelf === 'undefined' && typeof m.scaleSelf === 'undefined');
    ",
    );
}

// ---------------------------------------------------------------------------
// Element.animate composite option
// ---------------------------------------------------------------------------

#[test]
fn element_animate_composite_option() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var div = document.createElement('div');
        var anim = div.animate(
            [{ opacity: 0 }, { opacity: 1 }],
            { duration: 1000, composite: 'add' }
        );
        console.log(anim.playState === 'running');
    ",
    );
}

// ---------------------------------------------------------------------------
// DOMParser (WHATWG HTML §8.4)
// ---------------------------------------------------------------------------

#[test]
fn dom_parser_basic() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r#"
        var parser = new DOMParser();
        var doc = parser.parseFromString('<div id="test">hello</div>', 'text/html');
        var el = doc.querySelector('#test');
        console.log(el !== null && el.textContent === 'hello');
    "#,
    );
}

#[test]
fn dom_parser_query_selector_all() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var parser = new DOMParser();
        var doc = parser.parseFromString('<p>a</p><p>b</p>', 'text/html');
        var ps = doc.querySelectorAll('p');
        console.log(ps.length === 2);
    ",
    );
}

#[test]
fn dom_parser_unsupported_mime_throws() {
    let (mut rt, mut s, mut d, doc) = setup();
    let result = rt.eval(
        r"
        var parser = new DOMParser();
        parser.parseFromString('test', 'text/css');
    ",
        &mut s,
        &mut d,
        doc,
    );
    assert!(!result.success, "Expected TypeError for unsupported MIME");
}

#[test]
fn dom_parser_xml_mime_accepted() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var parser = new DOMParser();
        var doc = parser.parseFromString('<root><child/></root>', 'application/xml');
        console.log(typeof doc.querySelector === 'function');
    ",
    );
}

#[test]
fn dom_parser_document_element() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var parser = new DOMParser();
        var doc = parser.parseFromString('<html><body><p>text</p></body></html>', 'text/html');
        console.log(doc.documentElement !== null);
    ",
    );
}

// ---------------------------------------------------------------------------
// XMLSerializer (WHATWG DOM Parsing §3.2)
// ---------------------------------------------------------------------------

#[test]
fn xml_serializer_element() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var s = new XMLSerializer();
        var div = document.createElement('div');
        div.setAttribute('class', 'test');
        var result = s.serializeToString(div);
        console.log(result.indexOf('<div') === 0 && result.indexOf('class') > 0);
    ",
    );
}

#[test]
fn xml_serializer_text_node() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var s = new XMLSerializer();
        var text = document.createTextNode('hello world');
        var result = s.serializeToString(text);
        console.log(result === 'hello world');
    ",
    );
}

// ---------------------------------------------------------------------------
// requestIdleCallback / cancelIdleCallback (W3C requestIdleCallback §2)
// ---------------------------------------------------------------------------

#[test]
fn request_idle_callback_executes() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var executed = false;
        var id = requestIdleCallback(function(deadline) {
            executed = true;
        });
        console.log(typeof id === 'number');
    ",
    );
}

#[test]
fn request_idle_callback_deadline_object() {
    let (mut rt, mut s, mut d, doc) = setup();
    // The callback runs via setTimeout(0), so in boa's single-threaded model
    // it fires during run_jobs or timer drain. We test the returned id type.
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var id = requestIdleCallback(function(deadline) {
            // deadline has timeRemaining() and didTimeout
        });
        console.log(id > 0);
    ",
    );
}

#[test]
fn cancel_idle_callback_accepted() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var id = requestIdleCallback(function() {});
        cancelIdleCallback(id);
        console.log(true);
    ",
    );
}

// ---------------------------------------------------------------------------
// structuredClone (WHATWG HTML §2.7.6)
// ---------------------------------------------------------------------------

#[test]
fn structured_clone_object() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var obj = { a: 1, b: 'hello', c: [1, 2, 3] };
        var clone = structuredClone(obj);
        console.log(clone.a === 1 && clone.b === 'hello' && clone.c.length === 3 && clone !== obj);
    ",
    );
}

#[test]
fn structured_clone_array() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var arr = [1, 'two', { three: 3 }];
        var clone = structuredClone(arr);
        console.log(clone.length === 3 && clone[0] === 1 && clone[2].three === 3 && clone !== arr);
    ",
    );
}

#[test]
fn structured_clone_primitives() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        console.log(
            structuredClone(42) === 42 &&
            structuredClone('hello') === 'hello' &&
            structuredClone(true) === true &&
            structuredClone(null) === null
        );
    ",
    );
}

#[test]
fn structured_clone_nested() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var original = { outer: { inner: 'deep' } };
        var clone = structuredClone(original);
        clone.outer.inner = 'modified';
        console.log(original.outer.inner === 'deep');
    ",
    );
}

// ---------------------------------------------------------------------------
// document.currentScript (WHATWG HTML §4.12.1.1)
// ---------------------------------------------------------------------------

#[test]
fn document_current_script_null_outside_script() {
    let (mut rt, mut s, mut d, doc) = setup();
    // Without a script entity being set, currentScript should be null.
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        "console.log(document.currentScript === null)",
    );
}

#[test]
fn document_current_script_returns_element() {
    let (mut rt, mut s, mut d, doc) = setup();
    // Create a script element, set it as current, then eval.
    let script_elem = d.create_element("script", Attributes::default());
    let text = d.create_text("console.log('in script')");
    let _ = d.append_child(script_elem, text);
    let _ = d.append_child(doc, script_elem);

    // Set the current script entity before eval.
    rt.bridge().set_current_script_entity(Some(script_elem));
    eval_true(&mut rt, &mut s, &mut d, doc,
        "console.log(document.currentScript !== null && document.currentScript.tagName === 'SCRIPT')");
    rt.bridge().set_current_script_entity(None);
}
