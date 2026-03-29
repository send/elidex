use super::*;

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
