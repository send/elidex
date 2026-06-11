use super::*;

#[test]
fn custom_elements_define_and_get() {
    let (mut runtime, mut session, mut dom, doc) = setup();
    let result = runtime.eval(
        r"
        class MyElement {
            connectedCallback() {}
        }
        customElements.define('my-element', MyElement);
        var ctor = customElements.get('my-element');
        console.log('defined=' + (ctor === MyElement));
        console.log('undefined=' + (customElements.get('not-defined') === undefined));
        ",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "eval should succeed: {:?}", result.error);
    let output = runtime.console_output().messages();
    assert!(
        output.iter().any(|m| m.1.contains("defined=true")),
        "got: {output:?}"
    );
    assert!(
        output.iter().any(|m| m.1.contains("undefined=true")),
        "got: {output:?}"
    );
}

#[test]
fn custom_elements_define_invalid_name_throws() {
    let (mut runtime, mut session, mut dom, doc) = setup();
    let result = runtime.eval(
        r"
        try {
            customElements.define('div', class {});
            console.log('error=no_throw');
        } catch (e) {
            console.log('error=' + e.constructor.name);
        }
        ",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "eval should succeed: {:?}", result.error);
    let output = runtime.console_output().messages();
    assert!(
        output.iter().any(|m| m.1.contains("error=SyntaxError")),
        "got: {output:?}"
    );
}

#[test]
fn custom_elements_define_duplicate_throws() {
    let (mut runtime, mut session, mut dom, doc) = setup();
    let result = runtime.eval(
        r"
        customElements.define('my-dup', class {});
        try {
            customElements.define('my-dup', class {});
            console.log('error=no_throw');
        } catch (e) {
            console.log('error=' + e.constructor.name);
        }
        ",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "eval should succeed: {:?}", result.error);
    let output = runtime.console_output().messages();
    assert!(
        output.iter().any(|m| m.1.contains("error=TypeError")),
        "duplicate define should throw NotSupportedError (TypeError), got: {output:?}"
    );
}

#[test]
fn custom_elements_when_defined() {
    let (mut runtime, mut session, mut dom, doc) = setup();
    let result = runtime.eval(
        r"
        customElements.define('my-when', class {});
        var p = customElements.whenDefined('my-when');
        console.log('promise=' + (typeof p.then === 'function'));
        ",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "eval should succeed: {:?}", result.error);
    let output = runtime.console_output().messages();
    assert!(
        output.iter().any(|m| m.1.contains("promise=true")),
        "got: {output:?}"
    );
}

#[test]
fn create_custom_element_invokes_constructor() {
    let (mut runtime, mut session, mut dom, doc) = setup();
    // First eval: define and create element — enqueues upgrade reaction.
    let result = runtime.eval(
        r"
        var constructed = false;
        class MyButton {
            constructor() {
                constructed = true;
            }
        }
        MyButton.prototype.connectedCallback = function() {};
        customElements.define('my-button', MyButton);
        var el = document.createElement('my-button');
        ",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "eval should succeed: {:?}", result.error);
    // Second eval: check the result after reactions are drained.
    let result = runtime.eval(
        r"console.log('constructed=' + constructed);",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "eval should succeed: {:?}", result.error);
    let output = runtime.console_output().messages();
    assert!(
        output.iter().any(|m| m.1.contains("constructed=true")),
        "got: {output:?}"
    );
}

#[test]
fn create_undefined_element_upgrades_on_define() {
    let (mut runtime, mut session, mut dom, doc) = setup();
    let result = runtime.eval(
        r"
        var el = document.createElement('my-later');
        var upgraded = false;
        class MyLater {
            constructor() { upgraded = true; }
        }
        customElements.define('my-later', MyLater);
        ",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "eval should succeed: {:?}", result.error);
    let result = runtime.eval(
        r"console.log('upgraded=' + upgraded);",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "eval should succeed: {:?}", result.error);
    let output = runtime.console_output().messages();
    assert!(
        output.iter().any(|m| m.1.contains("upgraded=true")),
        "got: {output:?}"
    );
}

#[test]
fn connected_callback_fires_on_append() {
    let (mut runtime, mut session, mut dom, doc) = setup();

    // Set up html > body structure for document.body accessor.
    let html = dom.create_element("html", Attributes::default());
    let body = dom.create_element("body", Attributes::default());
    let _ = dom.append_child(doc, html);
    let _ = dom.append_child(html, body);

    // First eval: define, create, append — CE reactions enqueued in JS bindings.
    let result = runtime.eval(
        r"
        var connected = false;
        class MyConn {
            connectedCallback() { connected = true; }
        }
        customElements.define('my-conn', MyConn);
        var el = document.createElement('my-conn');
        document.body.appendChild(el);
        ",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "eval should succeed: {:?}", result.error);

    // Reactions are drained after eval; check the result.
    let result = runtime.eval(
        r"console.log('connected=' + connected);",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "eval should succeed: {:?}", result.error);
    let output = runtime.console_output().messages();
    assert!(
        output.iter().any(|m| m.1.contains("connected=true")),
        "got: {output:?}"
    );
}

#[test]
fn disconnected_callback_fires_on_remove() {
    let (mut runtime, mut session, mut dom, doc) = setup();

    let html = dom.create_element("html", Attributes::default());
    let body = dom.create_element("body", Attributes::default());
    let _ = dom.append_child(doc, html);
    let _ = dom.append_child(html, body);

    // Define and create element — upgrade happens after first eval.
    let result = runtime.eval(
        r"
        var disconnected = false;
        class MyDisc {
            disconnectedCallback() { disconnected = true; }
        }
        customElements.define('my-disc', MyDisc);
        var el = document.createElement('my-disc');
        document.body.appendChild(el);
        ",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "eval should succeed: {:?}", result.error);
    // After first eval: element is upgraded to Custom and connectedCallback fired.

    // Second eval: remove the now-upgraded element.
    let result = runtime.eval(
        r"
        document.body.removeChild(el);
        ",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "eval should succeed: {:?}", result.error);

    // Third eval: check disconnected flag.
    let result = runtime.eval(
        r"console.log('disconnected=' + disconnected);",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "eval should succeed: {:?}", result.error);
    let output = runtime.console_output().messages();
    assert!(
        output.iter().any(|m| m.1.contains("disconnected=true")),
        "got: {output:?}"
    );
}

#[test]
fn attribute_changed_callback_fires() {
    let (mut runtime, mut session, mut dom, doc) = setup();
    // Define, create, setAttribute — CE attributeChanged enqueued in JS binding.
    let result = runtime.eval(
        r"
        var attrLog = [];
        class MyAttr {
            static get observedAttributes() { return ['title']; }
            attributeChangedCallback(name, oldVal, newVal) {
                attrLog.push(name + ':' + oldVal + '->' + newVal);
            }
        }
        customElements.define('my-attr', MyAttr);
        var el = document.createElement('my-attr');
        el.setAttribute('title', 'hello');
        ",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "eval should succeed: {:?}", result.error);

    let result = runtime.eval(
        r"console.log('attrLog=' + attrLog.join(','));",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "eval should succeed: {:?}", result.error);
    let output = runtime.console_output().messages();
    assert!(
        output
            .iter()
            .any(|m| m.1.contains("attrLog=title:null->hello")),
        "got: {output:?}"
    );
}

#[test]
fn html_parser_marks_custom_elements() {
    let html = "<html><body><my-widget></my-widget></body></html>";
    let parse_result = elidex_html_parser::parse_html(html);

    // Walk the DOM to find the custom element entity.
    let doc = parse_result.document;
    let dom = &parse_result.dom;

    let mut found = false;
    let mut stack = vec![doc];
    while let Some(entity) = stack.pop() {
        if let Ok(ce_state) = dom
            .world()
            .get::<&elidex_custom_elements::CustomElementState>(entity)
        {
            assert_eq!(ce_state.definition_name, "my-widget");
            assert_eq!(ce_state.state, elidex_custom_elements::CEState::Undefined);
            found = true;
        }
        let mut child = dom.get_first_child(entity);
        while let Some(c) = child {
            stack.push(c);
            child = dom.get_next_sibling(c);
        }
    }
    assert!(
        found,
        "custom element should be marked with CustomElementState"
    );
}

#[test]
fn custom_elements_upgrade_walk() {
    let (mut runtime, mut session, mut dom, doc) = setup();

    // First eval: create elements and define — enqueues upgrades.
    let result = runtime.eval(
        r"
        var count = 0;
        var el1 = document.createElement('my-walk');
        var el2 = document.createElement('my-walk');

        function MyWalk() { count++; }
        customElements.define('my-walk', MyWalk);
        ",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "eval should succeed: {:?}", result.error);

    // Second eval: reactions have been drained after first eval, count updated.
    let result = runtime.eval(
        r"console.log('count=' + count);",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "eval should succeed: {:?}", result.error);
    let output = runtime.console_output().messages();
    assert!(
        output.iter().any(|m| m.1.contains("count=2")),
        "got: {output:?}"
    );
}

#[test]
fn constructor_exception_sets_failed_state() {
    let (mut runtime, mut session, mut dom, doc) = setup();
    let result = runtime.eval(
        r"
        class BadElement {
            constructor() { throw new Error('fail'); }
        }
        customElements.define('bad-el', BadElement);
        var el = document.createElement('bad-el');
        // el should exist but state should be Failed
        console.log('created=' + (el !== null));
        ",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "eval should succeed: {:?}", result.error);

    // After reactions drain, the element exists but state is Failed.
    let result = runtime.eval(
        r"console.log('created2=' + (el !== null));",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "eval should succeed: {:?}", result.error);
    let output = runtime.console_output().messages();
    assert!(
        output.iter().any(|m| m.1.contains("created=true")),
        "element should be created even if constructor throws, got: {output:?}"
    );

    // Verify that the ECS entity has CEState::Failed.
    // The element was created but not appended, so we query all entities.
    let mut found_failed = false;
    #[allow(clippy::explicit_iter_loop)]
    for ce_state in dom
        .world()
        .query::<&elidex_custom_elements::CustomElementState>()
        .iter()
    {
        if ce_state.definition_name == "bad-el" {
            assert_eq!(
                ce_state.state,
                elidex_custom_elements::CEState::Failed,
                "constructor threw, state should be Failed"
            );
            found_failed = true;
        }
    }
    assert!(
        found_failed,
        "should find a bad-el entity with Failed state"
    );
}

#[test]
fn inner_html_marks_custom_element_state() {
    let (mut runtime, mut session, mut dom, doc) = setup();

    let html = dom.create_element("html", Attributes::default());
    let body = dom.create_element("body", Attributes::default());
    let div = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(doc, html);
    let _ = dom.append_child(html, body);
    let _ = dom.append_child(body, div);

    // Define a custom element with connectedCallback.
    let result = runtime.eval(
        r"
        class InnerEl {
            connectedCallback() { console.log('inner-connected'); }
        }
        customElements.define('inner-el', InnerEl);
        var div = document.querySelector('div');
        div.innerHTML = '<inner-el></inner-el>';
        ",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "eval should succeed: {:?}", result.error);
    session.flush(&mut dom);

    // After flush, the inner-el should be created with CE state.
    // Note: Full CE reaction pipeline (enqueue + drain) requires runtime binding.
    // Here we verify the parser correctly marks custom elements; upgrade
    // processing is tested in create_undefined_element_upgrades_on_define.
    // Check that a CustomElementState exists for inner-el.
    let mut found = false;
    let mut stack = vec![doc];
    while let Some(entity) = stack.pop() {
        if let Ok(ce_state) = dom
            .world()
            .get::<&elidex_custom_elements::CustomElementState>(entity)
        {
            if ce_state.definition_name == "inner-el" {
                found = true;
            }
        }
        let mut child = dom.get_first_child(entity);
        while let Some(c) = child {
            stack.push(c);
            child = dom.get_next_sibling(c);
        }
    }
    assert!(
        found,
        "innerHTML should create inner-el with CustomElementState"
    );
}

#[test]
fn is_attribute_customized_builtin() {
    let (mut runtime, mut session, mut dom, doc) = setup();
    let result = runtime.eval(
        r"
        var upgraded = false;
        class MyDiv {
            constructor() { upgraded = true; }
        }
        customElements.define('my-div', MyDiv, { extends: 'div' });
        var el = document.createElement('div', { is: 'my-div' });
        ",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "eval should succeed: {:?}", result.error);
    let result = runtime.eval(
        r"console.log('upgraded=' + upgraded);",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "eval should succeed: {:?}", result.error);
    let output = runtime.console_output().messages();
    assert!(
        output.iter().any(|m| m.1.contains("upgraded=true")),
        "got: {output:?}"
    );
}

#[test]
fn nested_ce_connected_disconnected_callbacks() {
    let (mut runtime, mut session, mut dom, doc) = setup();

    let html = dom.create_element("html", Attributes::default());
    let body = dom.create_element("body", Attributes::default());
    let _ = dom.append_child(doc, html);
    let _ = dom.append_child(html, body);

    let result = runtime.eval(
        r"
        class OuterEl {
            connectedCallback() { console.log('outer-connected'); }
            disconnectedCallback() { console.log('outer-disconnected'); }
        }
        class InnerEl {
            connectedCallback() { console.log('inner-connected'); }
            disconnectedCallback() { console.log('inner-disconnected'); }
        }
        customElements.define('outer-el', OuterEl);
        customElements.define('inner-el', InnerEl);

        var outer = document.createElement('outer-el');
        var inner = document.createElement('inner-el');
        outer.appendChild(inner);
        document.body.appendChild(outer);
        ",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "eval should succeed: {:?}", result.error);

    // Drain reactions; check connected callbacks fired for both.
    let result = runtime.eval(r"0;", &mut session, &mut dom, doc);
    assert!(result.success);

    let output = runtime.console_output().messages();
    assert!(
        output.iter().any(|m| m.1.contains("outer-connected")),
        "outer connectedCallback should fire, got: {output:?}"
    );
    assert!(
        output.iter().any(|m| m.1.contains("inner-connected")),
        "inner connectedCallback should fire, got: {output:?}"
    );

    // Now remove the outer element — both should get disconnectedCallback.
    let result = runtime.eval(
        r"
        document.body.removeChild(outer);
        ",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "eval should succeed: {:?}", result.error);

    let result = runtime.eval(r"0;", &mut session, &mut dom, doc);
    assert!(result.success);

    let output = runtime.console_output().messages();
    assert!(
        output.iter().any(|m| m.1.contains("outer-disconnected")),
        "outer disconnectedCallback should fire, got: {output:?}"
    );
    assert!(
        output.iter().any(|m| m.1.contains("inner-disconnected")),
        "inner disconnectedCallback should fire, got: {output:?}"
    );
}

#[test]
fn create_element_is_round_trips_through_outer_html() {
    // boa outerHTML now routes through the canonical HTML §13.3
    // serializer (no hand-rolled opening tag): a customized built-in
    // created via createElement(tag, {is}) carries NO is content
    // attribute (DOM §4.5 sets none) yet its outerHTML emits the
    // is-value compensation, so serialize→parse keeps the identity.
    let (mut runtime, mut session, mut dom, doc) = setup();
    let result = runtime.eval(
        r"
        var el = document.createElement('button', { is: 'my-btn' });
        console.log('attr=' + el.getAttribute('is') + ' html=' + el.outerHTML);
        ",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "eval should succeed: {:?}", result.error);
    let output = runtime.console_output().messages();
    assert!(
        output.iter().any(|m| m
            .1
            .contains(r#"attr=null html=<button is="my-btn"></button>"#)),
        "expected is-emit without an is attribute, got: {output:?}"
    );
}

#[test]
fn create_element_is_with_registry_member_throws() {
    // DOM "flatten element creation options" step 3.2.1: non-null `is`
    // + `customElementRegistry` member → NotSupportedError, even when
    // the registry is the document's own (presence is the conflict).
    let (mut runtime, mut session, mut dom, doc) = setup();
    let result = runtime.eval(
        r"
        var caught = '';
        try { document.createElement('button',
                {is: 'my-btn', customElementRegistry: customElements}); }
        catch (e) { caught = '' + e; }
        console.log('caught=' + caught);
        ",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "eval should succeed: {:?}", result.error);
    let output = runtime.console_output().messages();
    assert!(
        output
            .iter()
            .any(|m| m.1.contains("caught=") && m.1.contains("NotSupportedError")),
        "expected NotSupportedError, got: {output:?}"
    );
}

#[test]
fn create_element_registry_converts_before_is_getter_runs() {
    // Codex PR331 R10: WebIDL dictionary conversion gets AND converts
    // members in lexicographic order (`customElementRegistry` before
    // `is`), so an invalid registry TypeErrors before the `is` getter
    // is even invoked -- the getter's own throw must NOT win.
    let (mut runtime, mut session, mut dom, doc) = setup();
    let result = runtime.eval(
        r"
        var caught = '';
        try { document.createElement('div',
                {customElementRegistry: 42, get is() { throw new Error('is-getter'); }}); }
        catch (e) { caught = '' + e; }
        console.log('caught=' + caught);
        ",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "eval should succeed: {:?}", result.error);
    let output = runtime.console_output().messages();
    assert!(
        output.iter().any(|m| m
            .1
            .contains("Failed to convert value to 'CustomElementRegistry'")
            && !m.1.contains("is-getter")),
        "expected registry conversion TypeError to precede the is getter, got: {output:?}"
    );
}

#[test]
fn create_element_registry_member_without_is_validated() {
    // Codex PR331 R8: the `customElementRegistry` member is inspected
    // even when `is` is absent — the document's registry passes
    // (flatten step 3.3), a non-registry value is the WebIDL
    // conversion TypeError, and a null registry is rejected loudly
    // (per-element registry association deferred, slot
    // `#11-shadow-scoped-custom-element-registry`).
    let (mut runtime, mut session, mut dom, doc) = setup();
    let result = runtime.eval(
        r"
        var ok = document.createElement('div',
            {customElementRegistry: customElements}).tagName;
        var bogus = '';
        try { document.createElement('div', {customElementRegistry: {}}); }
        catch (e) { bogus = '' + e; }
        var nul = '';
        try { document.createElement('div', {customElementRegistry: null}); }
        catch (e) { nul = '' + e; }
        console.log('ok=' + ok + ' bogus=' + bogus + ' nul=' + nul);
        ",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "eval should succeed: {:?}", result.error);
    let output = runtime.console_output().messages();
    assert!(
        output.iter().any(|m| m.1.contains("ok=DIV")
            && m.1
                .contains("Failed to convert value to 'CustomElementRegistry'")
            && m.1.contains("null customElementRegistry is not supported")),
        "expected accept/TypeError/NotSupportedError triple, got: {output:?}"
    );
}

#[test]
fn sync_autonomous_create_element_nulls_is_value() {
    // DOM §4.9 step 5.1.3.10: an autonomous element created while its
    // definition is already registered has a null is value — no
    // synthetic is= in outerHTML. (Async-created elements keep theirs.)
    let (mut runtime, mut session, mut dom, doc) = setup();
    let result = runtime.eval(
        r"
        class MyEl {}
        customElements.define('my-el', MyEl);
        var el = document.createElement('my-el', {is: 'other-el'});
        console.log('html=' + el.outerHTML);
        ",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "eval should succeed: {:?}", result.error);
    let output = runtime.console_output().messages();
    assert!(
        output.iter().any(|m| m.1.contains("html=<my-el></my-el>")),
        "sync autonomous must not emit synthetic is, got: {output:?}"
    );
}

#[test]
fn create_element_is_null_tostrings_to_null_string() {
    // WebIDL: `ElementCreationOptions.is` is a non-nullable DOMString —
    // an explicit `{is: null}` converts via ToString(null) = "null"
    // (member absent only when undefined).
    let (mut runtime, mut session, mut dom, doc) = setup();
    let result = runtime.eval(
        r"
        var el = document.createElement('button', { is: null });
        console.log('html=' + el.outerHTML);
        ",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "eval should succeed: {:?}", result.error);
    let output = runtime.console_output().messages();
    assert!(
        output
            .iter()
            .any(|m| m.1.contains(r#"html=<button is="null"></button>"#)),
        "explicit null is must ToString to \"null\", got: {output:?}"
    );
}

#[test]
fn name_sharing_builtin_definition_does_not_clear_is_value() {
    // Codex PR331 R6: define('plastic-button', C, {extends:'button'})
    // does not match createElement('plastic-button', {is}) for that
    // local name — the no-definition branch keeps the is value.
    let (mut runtime, mut session, mut dom, doc) = setup();
    let result = runtime.eval(
        r"
        class PB {}
        customElements.define('plastic-button', PB, { extends: 'button' });
        var el = document.createElement('plastic-button', { is: 'other-el' });
        console.log('html=' + el.outerHTML);
        ",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "eval should succeed: {:?}", result.error);
    let output = runtime.console_output().messages();
    assert!(
        output.iter().any(|m| m
            .1
            .contains(r#"html=<plastic-button is="other-el"></plastic-button>"#)),
        "name-sharing built-in definition must not clear the is value, got: {output:?}"
    );
}
