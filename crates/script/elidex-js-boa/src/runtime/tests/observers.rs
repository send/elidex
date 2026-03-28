use super::*;

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
