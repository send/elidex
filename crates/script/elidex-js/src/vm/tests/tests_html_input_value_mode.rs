//! `HTMLInputElement.prototype.value` HTML §4.10.5.4 value-mode dispatch
//! (`#11-input-idl-value-mode-dispatch`) — getter/setter mode dispatch +
//! the §4.10.5 type-change value migration.  Split out of
//! `tests_html_input_proto.rs` to keep that file under the 1000-line
//! convention.

#![cfg(feature = "engine")]

use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
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

#[test]
fn input_value_default_on_getter_falls_back_to_on() {
    // checkbox/radio = default/on mode: getter returns "on" with no
    // `value` content attribute.
    let out = run("var i = document.createElement('input'); i.type='checkbox'; i.value;");
    assert_eq!(out, "on");
}

#[test]
fn input_value_default_on_setter_writes_content_attribute() {
    // Setting `.value` in default/on mode sets the `value` CONTENT
    // attribute (not the live value); the getter then reads it back.
    let out = run(
        "var i = document.createElement('input'); i.type='checkbox'; \
         i.value = 'x'; \
         i.value + '|' + i.getAttribute('value');",
    );
    assert_eq!(out, "x|x");
}

#[test]
fn input_value_default_getter_falls_back_to_empty() {
    // hidden/submit/etc = default mode: getter returns "" with no attr.
    let out = run("var i = document.createElement('input'); i.type='hidden'; i.value;");
    assert_eq!(out, "");
}

#[test]
fn input_value_default_setter_round_trips_via_content_attribute() {
    let out = run("var i = document.createElement('input'); i.type='hidden'; \
         i.value = 'secret'; \
         i.value + '|' + i.getAttribute('value');");
    assert_eq!(out, "secret|secret");
}

#[test]
fn input_value_filename_setter_nonempty_throws_invalid_state() {
    let out = run("var i = document.createElement('input'); i.type='file'; \
         try { i.value = 'x'; 'no-throw'; } \
         catch (e) { (e.name === 'InvalidStateError') ? 'ok' : 'other:' + e.name; }");
    assert_eq!(out, "ok");
}

#[test]
fn input_value_filename_setter_empty_is_noop() {
    // Empty-string set on a file control empties the (empty) file list —
    // no throw; getter stays "" (list not modeled).
    let out = run("var i = document.createElement('input'); i.type='file'; \
         i.value = ''; i.value;");
    assert_eq!(out, "");
}

#[test]
fn input_value_filename_null_clears_does_not_throw() {
    // `HTMLInputElement.value` is `[LegacyNullToEmptyString]`, so `null`
    // assignment is the empty string — for a file input that clears the
    // selected files (no throw), exactly like `value = ""` (NOT the literal
    // "null", which would take the non-empty throwing branch).
    let out = run("var i = document.createElement('input'); i.type='file'; \
         try { i.value = null; i.value; } catch (e) { 'throw:' + e.name; }");
    assert_eq!(out, "");
}

#[test]
fn input_value_filename_getter_empty_list_is_empty_string() {
    let out = run("var i = document.createElement('input'); i.type='file'; i.value;");
    assert_eq!(out, "");
}

#[test]
fn input_value_value_mode_unchanged() {
    // Regression: text (value mode) reads/writes the live value directly.
    let out = run("var i = document.createElement('input'); i.value = 'hi'; i.value;");
    assert_eq!(out, "hi");
}

#[test]
fn input_value_null_sets_empty_string() {
    // `[LegacyNullToEmptyString]`: `value = null` sets "" (not "null").
    let out = run("var i = document.createElement('input'); i.value = null; i.value;");
    assert_eq!(out, "");
}

#[test]
fn input_type_change_value_to_default_migrates_live_value_to_attr() {
    // §4.10.5 type-change step 1 via the IDL `type` setter (which routes
    // through the canonical reconciler site): a dirty live value migrates
    // into the `value` content attribute, then the default-mode getter
    // reads it back.
    let out = run("var i = document.createElement('input'); i.value = 'abc'; \
         i.type = 'hidden'; \
         i.getAttribute('value') + '|' + i.value;");
    assert_eq!(out, "abc|abc");
}

#[test]
fn input_type_change_default_to_value_adopts_content_attr() {
    // §4.10.5 type-change step 2: hidden (with a `value` attr) → text
    // adopts the content attribute as the live value.
    let out = run("var i = document.createElement('input'); i.type='hidden'; \
         i.setAttribute('value', 'x'); \
         i.type = 'text'; \
         i.value;");
    assert_eq!(out, "x");
}
