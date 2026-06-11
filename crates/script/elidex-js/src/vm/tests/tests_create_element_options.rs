//! `document.createElement(localName, options)` option-flattening tests
//! (DOM §4.5 "flatten element creation options" + §4.9 step 6.3 / step
//! 5.1.3.10) — split out of `tests_custom_elements.rs` per the
//! 1000-line file rule.

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

/// Two-step run — see `tests_custom_elements::run_then_read`.
fn run_then_read(setup: &str, read_expr: &str) -> String {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval(setup).unwrap();
    let result = vm.eval(read_expr).unwrap();
    let JsValue::String(sid) = result else {
        panic!("expected string, got {result:?}")
    };
    let out = vm.inner.strings.get_utf8(sid);
    vm.unbind();
    out
}

#[test]
fn create_element_options_is_flattens_to_component_not_attribute() {
    // VM-side option flattening (DOM §4.5 createElement step 3): the
    // `is` value lands in the CustomElementState component — no `is`
    // content attribute is set — and the HTML §13.3 serializer
    // compensation emits it, so outerHTML round-trips the identity.
    let out = run(
        "var el = document.createElement('button', {is: 'my-btn'}); \
         (el.getAttribute('is') === null && el.outerHTML === '<button is=\"my-btn\"></button>') \
             ? 'ok' : ('fail:' + el.outerHTML);",
    );
    assert_eq!(out, "ok");
}

#[test]
fn create_element_options_is_tostring_coerces() {
    // WebIDL DOMString conversion at the marshalling layer: a numeric
    // `is` member ToString-coerces rather than being dropped.
    let out = run("var el = document.createElement('button', {is: 123}); \
         el.outerHTML === '<button is=\"123\"></button>' ? 'ok' : ('fail:' + el.outerHTML);");
    assert_eq!(out, "ok");
}

#[test]
fn create_element_is_with_custom_element_registry_throws_not_supported() {
    // DOM "flatten element creation options" step 3.2.1: a dictionary
    // carrying BOTH a non-null `is` and a `customElementRegistry`
    // member throws NotSupportedError — even when the registry is the
    // document's own (the conflict is about member presence).
    let out = run("var caught = ''; \
         try { document.createElement('button', \
                 {is: 'my-btn', customElementRegistry: customElements}); } \
         catch (e) { caught = '' + e; } \
         caught.indexOf('NotSupportedError') !== -1 ? 'ok' : ('fail:' + caught);");
    assert_eq!(out, "ok");
}

#[test]
fn sync_autonomous_create_element_nulls_is_value() {
    // DOM §4.9 create-an-element step 5.1.3.10: when an autonomous
    // definition is already registered, the synchronously created
    // element's is value is null — outerHTML must NOT emit a
    // synthetic is for it. (The async path — definition registered
    // later — retains the creation-time is value per spec.)
    let out = run_then_read(
        "class MyEl extends HTMLElement {} \
         customElements.define('my-el', MyEl); \
         globalThis.__el = document.createElement('my-el', {is: 'other-el'});",
        "globalThis.__el.outerHTML",
    );
    assert_eq!(out, "<my-el></my-el>");
}

#[test]
fn create_element_options_is_null_tostrings_to_null_string() {
    // Codex PR331 R5: `ElementCreationOptions.is` is a non-nullable
    // DOMString — WebIDL dictionary conversion ToStrings an explicit
    // null, so `{is: null}` yields the is value "null" (member absent
    // only when undefined).
    let out = run("var el = document.createElement('button', {is: null}); \
         el.outerHTML === '<button is=\"null\"></button>' ? 'ok' : ('fail:' + el.outerHTML);");
    assert_eq!(out, "ok");
}

#[test]
fn create_element_options_is_null_with_registry_still_conflicts() {
    // The ToString'd "null" is a non-null is value, so the flatten
    // step 3.2.1 conflict with customElementRegistry still throws.
    let out = run("var caught = ''; \
         try { document.createElement('button', \
                 {is: null, customElementRegistry: customElements}); } \
         catch (e) { caught = '' + e; } \
         caught.indexOf('NotSupportedError') !== -1 ? 'ok' : ('fail:' + caught);");
    assert_eq!(out, "ok");
}

#[test]
fn create_element_options_document_registry_accepted_without_is() {
    // Codex PR331 R8: a `customElementRegistry` member must be
    // inspected even when `is` is absent. The document's global
    // registry passes flatten step 3.3 (it IS the document's
    // registry), so creation proceeds normally.
    let out = run(
        "var el = document.createElement('div', {customElementRegistry: customElements}); \
         el.tagName === 'DIV' ? 'ok' : ('fail:' + el.tagName);",
    );
    assert_eq!(out, "ok");
}

#[test]
fn create_element_options_non_registry_value_throws_type_error() {
    // WebIDL conversion: the member is `CustomElementRegistry?` — a
    // plain object is neither null nor a registry platform object, so
    // dictionary conversion throws TypeError (Codex PR331 R8: this
    // previously fell through to the default registry silently).
    let out = run("var caught = ''; \
         try { document.createElement('div', {customElementRegistry: {}}); } \
         catch (e) { caught = '' + e; } \
         (caught.indexOf('TypeError') !== -1 \
          && caught.indexOf('CustomElementRegistry') !== -1) ? 'ok' : ('fail:' + caught);");
    assert_eq!(out, "ok");
}

#[test]
fn create_element_options_null_registry_creates_element() {
    // Codex PR331 R12: `{customElementRegistry: null}` is spec-legal —
    // flatten step 3.3 only rejects a NON-null foreign registry. The
    // element is created with a null registry association.
    let out = run(
        "var el = document.createElement('div', {customElementRegistry: null}); \
         el.tagName === 'DIV' ? 'ok' : ('fail:' + el.tagName);",
    );
    assert_eq!(out, "ok");
}

#[test]
fn create_element_null_registry_element_never_upgrades() {
    // DOM §4.9: definition lookup in a null registry is always null —
    // a null-registry custom element is never upgraded, neither at
    // define() time (queue/walk) nor by customElements.upgrade(),
    // while an identically named document-registry element upgrades
    // normally.
    let out = run_then_read(
        "globalThis.XNull = class extends HTMLElement { \
             constructor() { super(); this.upgraded = true; } }; \
         globalThis.__nullreg = document.createElement('x-nullreg', \
             {customElementRegistry: null}); \
         globalThis.__normal = document.createElement('x-nullreg'); \
         document.body.appendChild(globalThis.__nullreg); \
         customElements.define('x-nullreg', globalThis.XNull); \
         customElements.upgrade(document.body);",
        "(globalThis.__nullreg.upgraded ? 'null-upgraded' : 'null-kept') \
         + ':' + (globalThis.__normal.upgraded ? 'normal-upgraded' : 'normal-kept')",
    );
    assert_eq!(out, "null-kept:normal-upgraded");
}

#[test]
fn null_registry_element_not_upgraded_on_insertion_after_define() {
    // Codex PR331 R13: the insertion upgrade path must gate on the
    // registry association too — define() first, then create a
    // null-registry element and append it: the insertion-time
    // reaction must NOT upgrade it.
    let out = run_then_read(
        "globalThis.__count = 0; \
         customElements.define('x-ins', class extends HTMLElement { \
             constructor() { super(); globalThis.__count++; } }); \
         var n = document.createElement('x-ins', {customElementRegistry: null}); \
         document.body.appendChild(n);",
        "'count=' + globalThis.__count",
    );
    assert_eq!(out, "count=0");
}

#[test]
fn clone_of_defined_custom_element_upgrades_via_clone_reaction() {
    // Codex PR331 R13 / DOM §4.4 "clone a single node": a clone whose
    // definition lookup is non-null gets an upgrade reaction enqueued
    // at clone time — it must not stay Undefined until insertion or
    // customElements.upgrade(). The clone here is never inserted; the
    // reaction drains at the eval checkpoint.
    let out = run_then_read(
        "globalThis.__count = 0; \
         customElements.define('x-clup', class extends HTMLElement { \
             constructor() { super(); globalThis.__count++; } }); \
         var orig = document.createElement('x-clup'); \
         globalThis.__clone = orig.cloneNode(false);",
        "'count=' + globalThis.__count",
    );
    // One construction for the original (sync at createElement), one
    // for the disconnected clone (clone-time reaction).
    assert_eq!(out, "count=2");
}

#[test]
fn clone_of_null_registry_element_not_upgraded() {
    // The clone-time reaction seam resolves candidacy through
    // prepare_upgrade, so a null-registry clone (association
    // propagates per DOM §4.4) is skipped.
    let out = run_then_read(
        "globalThis.__count = 0; \
         customElements.define('x-clnull', class extends HTMLElement { \
             constructor() { super(); globalThis.__count++; } }); \
         var orig = document.createElement('x-clnull', {customElementRegistry: null}); \
         globalThis.__clone = orig.cloneNode(false);",
        "'count=' + globalThis.__count",
    );
    assert_eq!(out, "count=0");
}

#[test]
fn create_element_options_symbol_throws_type_error() {
    // Codex PR331 R12: a non-object `options` converts through the
    // DOMString arm of `(DOMString or ElementCreationOptions)` —
    // observably throwing for Symbol — before DOM §4.5 ignores the
    // string arm for web compatibility.
    let out = run("var caught = ''; \
         try { document.createElement('div', Symbol()); } \
         catch (e) { caught = '' + e; } \
         caught.indexOf('TypeError') !== -1 ? 'ok' : ('fail:' + caught);");
    assert_eq!(out, "ok");
}

#[test]
fn create_element_invalid_tag_beats_flatten_conflict() {
    // DOM §4.5 method step 1 (localName validity →
    // InvalidCharacterError) runs BEFORE step 3's flatten conflict
    // NotSupportedError. (WebIDL argument-conversion TypeErrors still
    // precede both — see the companion test below.)
    let out = run("var caught = ''; \
         try { document.createElement('bad tag', \
                 {is: 'x-a', customElementRegistry: customElements}); } \
         catch (e) { caught = '' + e; } \
         (caught.indexOf('InvalidCharacterError') !== -1 \
          && caught.indexOf('NotSupportedError') === -1) ? 'ok' : ('fail:' + caught);");
    assert_eq!(out, "ok");
}

#[test]
fn create_element_registry_conversion_beats_invalid_tag() {
    // WebIDL converts arguments BEFORE any method step: the
    // dictionary conversion of a non-registry `customElementRegistry`
    // TypeErrors before step 1's InvalidCharacterError is reached.
    let out = run("var caught = ''; \
         try { document.createElement('bad tag', {customElementRegistry: 42}); } \
         catch (e) { caught = '' + e; } \
         (caught.indexOf('TypeError') !== -1 \
          && caught.indexOf('CustomElementRegistry') !== -1 \
          && caught.indexOf('InvalidCharacterError') === -1) ? 'ok' : ('fail:' + caught);");
    assert_eq!(out, "ok");
}

#[test]
fn create_element_options_registry_conversion_precedes_is_conflict() {
    // WebIDL dictionary members convert in lexicographic order
    // (`customElementRegistry` < `is`), so a non-registry value
    // TypeErrors BEFORE the flatten step 3.2.1 NotSupportedError
    // conflict is reached.
    let out = run("var caught = ''; \
         try { document.createElement('button', \
                 {is: 'my-btn', customElementRegistry: 42}); } \
         catch (e) { caught = '' + e; } \
         caught.indexOf('TypeError') !== -1 ? 'ok' : ('fail:' + caught);");
    assert_eq!(out, "ok");
}

#[test]
fn create_element_options_registry_converts_before_is_getter_runs() {
    // Codex PR331 R10: WebIDL dictionary conversion gets AND converts
    // each member immediately in lexicographic order, so an invalid
    // `customElementRegistry` TypeErrors before the `is` getter is
    // even invoked — the getter's own throw must NOT win.
    let out = run("var caught = ''; \
         try { document.createElement('button', \
                 {customElementRegistry: 42, get is() { throw new Error('is-getter'); }}); } \
         catch (e) { caught = '' + e; } \
         (caught.indexOf('TypeError') !== -1 \
          && caught.indexOf('CustomElementRegistry') !== -1 \
          && caught.indexOf('is-getter') === -1) ? 'ok' : ('fail:' + caught);");
    assert_eq!(out, "ok");
}

#[test]
fn create_element_options_is_tostring_precedes_registry_conflict() {
    // The `is` ToString is part of dictionary CONVERSION, which
    // completes before the flatten algorithm runs — a throwing
    // toString wins over the step 3.2.1 conflict NotSupportedError.
    let out = run("var caught = ''; \
         try { document.createElement('button', \
                 {is: {toString: function() { throw new Error('tostr'); }}, \
                  customElementRegistry: customElements}); } \
         catch (e) { caught = '' + e; } \
         (caught.indexOf('tostr') !== -1 \
          && caught.indexOf('NotSupportedError') === -1) ? 'ok' : ('fail:' + caught);");
    assert_eq!(out, "ok");
}
