use super::*;

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
