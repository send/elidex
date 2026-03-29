use super::*;

// Helper: eval JS and check console output for "true".
fn eval_true(runtime: &mut JsRuntime, session: &mut SessionCore, dom: &mut EcsDom, doc: Entity, code: &str) {
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
    eval_true(&mut rt, &mut s, &mut d, doc,
        "console.log(typeof performance.now() === 'number')");
}

#[test]
fn performance_time_origin_is_positive() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(&mut rt, &mut s, &mut d, doc,
        "console.log(performance.timeOrigin > 0)");
}

#[test]
fn performance_mark_and_measure() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(&mut rt, &mut s, &mut d, doc, r"
        performance.mark('start');
        performance.mark('end');
        var m = performance.measure('test', 'start', 'end');
        console.log(m.entryType === 'measure' && m.name === 'test' && m.duration >= 0);
    ");
}

#[test]
fn performance_get_entries_by_type() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(&mut rt, &mut s, &mut d, doc, r"
        performance.mark('a');
        performance.mark('b');
        performance.measure('m', 'a', 'b');
        var marks = performance.getEntriesByType('mark');
        var measures = performance.getEntriesByType('measure');
        console.log(marks.length >= 2 && measures.length >= 1);
    ");
}

#[test]
fn performance_clear_marks() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(&mut rt, &mut s, &mut d, doc, r"
        performance.mark('x');
        performance.clearMarks('x');
        console.log(performance.getEntriesByName('x').length === 0);
    ");
}

// ---------------------------------------------------------------------------
// atob / btoa (WHATWG HTML §8.3)
// ---------------------------------------------------------------------------

#[test]
fn btoa_atob_roundtrip() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(&mut rt, &mut s, &mut d, doc,
        "console.log(atob(btoa('Hello, World!')) === 'Hello, World!')");
}

// ---------------------------------------------------------------------------
// crypto (W3C WebCrypto)
// ---------------------------------------------------------------------------

#[test]
fn crypto_random_uuid() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(&mut rt, &mut s, &mut d, doc, r"
        var uuid = crypto.randomUUID();
        console.log(uuid.length === 36 && uuid[8] === '-' && uuid[13] === '-');
    ");
}

// ---------------------------------------------------------------------------
// URL / URLSearchParams (WHATWG URL §6)
// ---------------------------------------------------------------------------

#[test]
fn url_constructor_basic() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(&mut rt, &mut s, &mut d, doc, r"
        var u = new URL('https://example.com/path?q=1#frag');
        console.log(u.hostname === 'example.com' && u.pathname === '/path' && u.hash === '#frag');
    ");
}

#[test]
fn url_search_params() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(&mut rt, &mut s, &mut d, doc, r"
        var u = new URLSearchParams('a=1&b=2');
        console.log(u.get('a') === '1' && u.has('b') && !u.has('c'));
    ");
}

// ---------------------------------------------------------------------------
// TextEncoder / TextDecoder (WHATWG Encoding §8)
// ---------------------------------------------------------------------------

#[test]
fn text_encoder_decode_roundtrip() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(&mut rt, &mut s, &mut d, doc, r"
        var enc = new TextEncoder();
        var dec = new TextDecoder();
        var bytes = enc.encode('hello');
        console.log(dec.decode(bytes) === 'hello');
    ");
}

// ---------------------------------------------------------------------------
// AbortController / AbortSignal (WHATWG DOM §3.2)
// ---------------------------------------------------------------------------

#[test]
fn abort_controller_basic() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(&mut rt, &mut s, &mut d, doc, r"
        var ac = new AbortController();
        var before = ac.signal.aborted;
        ac.abort();
        var after = ac.signal.aborted;
        console.log(!before && after);
    ");
}

#[test]
fn abort_signal_abort_static() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(&mut rt, &mut s, &mut d, doc, r"
        var s = AbortSignal.abort('test reason');
        console.log(s.aborted === true);
    ");
}

#[test]
fn abort_controller_onabort_callback() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(&mut rt, &mut s, &mut d, doc, r"
        var called = false;
        var ac = new AbortController();
        ac.signal.onabort = function() { called = true; };
        ac.abort();
        console.log(called);
    ");
}

// ---------------------------------------------------------------------------
// Blob / File (WHATWG File API §4-5)
// ---------------------------------------------------------------------------

#[test]
fn blob_constructor_size_type() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(&mut rt, &mut s, &mut d, doc, r"
        var b = new Blob(['hello'], { type: 'text/plain' });
        console.log(b.size === 5 && b.type === 'text/plain');
    ");
}

#[test]
fn blob_slice() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(&mut rt, &mut s, &mut d, doc, r"
        var b = new Blob(['hello world']);
        var sliced = b.slice(0, 5);
        console.log(sliced.size === 5);
    ");
}

#[test]
fn file_constructor() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(&mut rt, &mut s, &mut d, doc, r"
        var f = new File(['content'], 'test.txt', { type: 'text/plain' });
        console.log(f.name === 'test.txt' && f.size === 7 && f.type === 'text/plain' && f.lastModified > 0);
    ");
}

// ---------------------------------------------------------------------------
// FormData (WHATWG XHR §4.3)
// ---------------------------------------------------------------------------

#[test]
fn form_data_crud() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(&mut rt, &mut s, &mut d, doc, r"
        var fd = new FormData();
        fd.append('key', 'value');
        console.log(fd.has('key') && fd.get('key') === 'value' && !fd.has('missing'));
    ");
}

#[test]
fn form_data_set_replaces() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(&mut rt, &mut s, &mut d, doc, r"
        var fd = new FormData();
        fd.append('k', 'a');
        fd.append('k', 'b');
        fd.set('k', 'c');
        console.log(fd.getAll('k').length === 1 && fd.get('k') === 'c');
    ");
}

#[test]
fn form_data_delete() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(&mut rt, &mut s, &mut d, doc, r"
        var fd = new FormData();
        fd.append('k', 'v');
        fd.delete('k');
        console.log(!fd.has('k'));
    ");
}

#[test]
fn form_data_foreach() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(&mut rt, &mut s, &mut d, doc, r"
        var fd = new FormData();
        fd.append('a', '1');
        fd.append('b', '2');
        var keys = [];
        fd.forEach(function(v, k) { keys.push(k); });
        console.log(keys.length === 2 && keys[0] === 'a' && keys[1] === 'b');
    ");
}

// ---------------------------------------------------------------------------
// DOMPoint / DOMMatrix (CSSWG Geometry §5-6)
// ---------------------------------------------------------------------------

#[test]
fn dom_point_constructor() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(&mut rt, &mut s, &mut d, doc, r"
        var p = new DOMPoint(1, 2, 3, 4);
        console.log(p.x === 1 && p.y === 2 && p.z === 3 && p.w === 4);
    ");
}

#[test]
fn dom_point_defaults() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(&mut rt, &mut s, &mut d, doc, r"
        var p = new DOMPoint();
        console.log(p.x === 0 && p.y === 0 && p.z === 0 && p.w === 1);
    ");
}

#[test]
fn dom_rect_constructor() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(&mut rt, &mut s, &mut d, doc, r"
        var r = new DOMRect(10, 20, 100, 50);
        console.log(r.x === 10 && r.y === 20 && r.width === 100 && r.height === 50 &&
            r.top === 20 && r.left === 10 && r.right === 110 && r.bottom === 70);
    ");
}

#[test]
fn dom_matrix_identity() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(&mut rt, &mut s, &mut d, doc, r"
        var m = new DOMMatrix();
        console.log(m.is2D && m.isIdentity && m.a === 1 && m.d === 1 && m.e === 0 && m.f === 0);
    ");
}

// ---------------------------------------------------------------------------
// visualViewport
// ---------------------------------------------------------------------------

#[test]
fn visual_viewport_exists() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(&mut rt, &mut s, &mut d, doc,
        "console.log(typeof visualViewport === 'object' && visualViewport.scale === 1)");
}

// ---------------------------------------------------------------------------
// queueMicrotask
// ---------------------------------------------------------------------------

#[test]
fn queue_microtask_executes() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(&mut rt, &mut s, &mut d, doc, r"
        var executed = false;
        queueMicrotask(function() { executed = true; });
        console.log(executed);
    ");
}

// ---------------------------------------------------------------------------
// navigator
// ---------------------------------------------------------------------------

#[test]
fn navigator_properties() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(&mut rt, &mut s, &mut d, doc, r"
        console.log(
            typeof navigator.userAgent === 'string' &&
            typeof navigator.platform === 'string' &&
            navigator.onLine === true &&
            navigator.cookieEnabled === true
        );
    ");
}

// ---------------------------------------------------------------------------
// console extensions
// ---------------------------------------------------------------------------

#[test]
fn console_time_end() {
    let (mut rt, mut s, mut d, doc) = setup();
    let result = rt.eval(r"
        console.time('test');
        console.timeEnd('test');
    ", &mut s, &mut d, doc);
    assert!(result.success, "console.time/timeEnd error: {:?}", result.error);
}

// ---------------------------------------------------------------------------
// Element.animate
// ---------------------------------------------------------------------------

#[test]
fn element_animate_returns_animation() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(&mut rt, &mut s, &mut d, doc, r"
        var div = document.createElement('div');
        var anim = div.animate(
            [{ opacity: 0 }, { opacity: 1 }],
            { duration: 1000, iterations: 1 }
        );
        console.log(anim.playState === 'running' && typeof anim.play === 'function');
    ");
}

#[test]
fn element_animate_cancel() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(&mut rt, &mut s, &mut d, doc, r"
        var div = document.createElement('div');
        var anim = div.animate([{ opacity: 0 }, { opacity: 1 }], 500);
        anim.cancel();
        console.log(anim.playState === 'idle');
    ");
}

// ---------------------------------------------------------------------------
// window properties
// ---------------------------------------------------------------------------

#[test]
fn window_self_reference() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(&mut rt, &mut s, &mut d, doc,
        "console.log(window === self && window === globalThis)");
}

#[test]
fn window_closed_is_false() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(&mut rt, &mut s, &mut d, doc, "console.log(closed === false)");
}

// ---------------------------------------------------------------------------
// localStorage / sessionStorage
// ---------------------------------------------------------------------------

#[test]
fn session_storage_crud() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(&mut rt, &mut s, &mut d, doc, r"
        sessionStorage.setItem('k', 'v');
        var got = sessionStorage.getItem('k');
        sessionStorage.removeItem('k');
        console.log(got === 'v' && sessionStorage.getItem('k') === null);
    ");
}

#[test]
fn local_storage_crud() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(&mut rt, &mut s, &mut d, doc, r"
        localStorage.setItem('k', 'v');
        console.log(localStorage.getItem('k') === 'v' && localStorage.length === 1);
        localStorage.clear();
        console.log(localStorage.length === 0);
    ");
}
