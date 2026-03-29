use super::*;

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
// document.currentScript (WHATWG HTML §4.12.1.1)
// ---------------------------------------------------------------------------

#[test]
fn document_current_script_null_outside_script() {
    let (mut rt, mut s, mut d, doc) = setup();
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
    let script_elem = d.create_element("script", Attributes::default());
    let text = d.create_text("console.log('in script')");
    let _ = d.append_child(script_elem, text);
    let _ = d.append_child(doc, script_elem);

    rt.bridge().set_current_script_entity(Some(script_elem));
    eval_true(&mut rt, &mut s, &mut d, doc,
        "console.log(document.currentScript !== null && document.currentScript.tagName === 'SCRIPT')");
    rt.bridge().set_current_script_entity(None);
}
