//! M4-12 #11-classlist-dataset — `HTMLElement.dataset` (DOMStringMap) tests.
//!
//! Covers:
//! - Identity: `el.dataset === el.dataset`.
//! - Named-property [[Get]] with camelCase ↔ kebab-case mapping.
//! - Named-property [[Set]] with ToString coercion.
//! - Named-property [[Delete]] (idempotent).
//! - `for-in dataset` enumerates camelCase keys in attribute order.
//! - `Object.keys(dataset)` returns only camelCase data-* keys.
//! - Round-trip with `setAttribute('data-foo-bar', …)` /
//!   `getAttribute('data-foo-bar')`.
//! - Prototype fallthrough — `dataset.toString` resolves to
//!   `Object.prototype.toString` since `data-toString` is absent.

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

// --- identity ----------------------------------------------------

#[test]
fn dataset_identity_preserved() {
    let out = run("var d = document.createElement('div'); \
         (d.dataset === d.dataset) ? 'same' : 'different';");
    assert_eq!(out, "same");
}

// --- read / write / delete --------------------------------------

#[test]
fn dataset_set_creates_data_attr() {
    let out = run("var d = document.createElement('div'); \
         d.dataset.foo = 'bar'; \
         d.getAttribute('data-foo');");
    assert_eq!(out, "bar");
}

#[test]
fn dataset_get_reads_data_attr() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('data-x', 'y'); \
         d.dataset.x;");
    assert_eq!(out, "y");
}

#[test]
fn dataset_camel_to_kebab_roundtrip() {
    let out = run("var d = document.createElement('div'); \
         d.dataset.fooBar = '1'; \
         d.getAttribute('data-foo-bar') + '/' + d.dataset.fooBar;");
    assert_eq!(out, "1/1");
}

#[test]
fn dataset_kebab_to_camel_roundtrip() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('data-foo-bar-baz', 'q'); \
         d.dataset.fooBarBaz;");
    assert_eq!(out, "q");
}

#[test]
fn dataset_delete_removes_data_attr() {
    let out = run("var d = document.createElement('div'); \
         d.dataset.foo = 'x'; \
         var r = delete d.dataset.foo; \
         r + '/' + d.hasAttribute('data-foo');");
    assert_eq!(out, "true/false");
}

#[test]
fn dataset_delete_absent_succeeds() {
    let out = run("var d = document.createElement('div'); \
         '' + (delete d.dataset.missing);");
    assert_eq!(out, "true");
}

// --- ToString coercion on values --------------------------------

#[test]
fn dataset_set_coerces_number_to_string() {
    let out = run("var d = document.createElement('div'); \
         d.dataset.foo = 42; \
         d.getAttribute('data-foo');");
    assert_eq!(out, "42");
}

#[test]
fn dataset_set_coerces_null() {
    let out = run("var d = document.createElement('div'); \
         d.dataset.foo = null; \
         d.getAttribute('data-foo');");
    assert_eq!(out, "null");
}

#[test]
fn dataset_set_coerces_undefined() {
    let out = run("var d = document.createElement('div'); \
         d.dataset.foo = undefined; \
         d.getAttribute('data-foo');");
    assert_eq!(out, "undefined");
}

#[test]
fn dataset_set_symbol_throws_type_error() {
    let out = run("var d = document.createElement('div'); \
         try { d.dataset.foo = Symbol(); 'no-throw'; } \
         catch (e) { (e instanceof TypeError) ? 'TypeError' : 'wrong:' + e.name; }");
    assert_eq!(out, "TypeError");
}

#[test]
fn dataset_different_elements_have_different_wrappers() {
    let out = run("var e1 = document.createElement('div'); \
         var e2 = document.createElement('span'); \
         '' + (e1.dataset !== e2.dataset);");
    assert_eq!(out, "true");
}

#[test]
fn dataset_get_own_property_descriptor_returns_data_descriptor() {
    let out = run("var d = document.createElement('div'); \
         d.dataset.fooBar = 'x'; \
         var desc = Object.getOwnPropertyDescriptor(d.dataset, 'fooBar'); \
         desc.value + '/' + desc.writable + '/' + desc.enumerable + '/' + desc.configurable;");
    assert_eq!(out, "x/true/true/true");
}

#[test]
fn dataset_get_own_property_descriptor_absent_falls_through() {
    let out = run("var d = document.createElement('div'); \
         var desc = Object.getOwnPropertyDescriptor(d.dataset, 'missing'); \
         '' + (desc === undefined);");
    assert_eq!(out, "true");
}

#[test]
fn dataset_not_iterable() {
    let out = run("var d = document.createElement('div'); \
         d.dataset.foo = 'bar'; \
         try { for (var x of d.dataset) {} 'iterable'; } \
         catch (e) { (e instanceof TypeError) ? 'TypeError' : 'wrong:' + e.name; }");
    assert_eq!(out, "TypeError");
}

// --- enumeration -------------------------------------------------

#[test]
fn dataset_for_in_yields_camel_case_keys() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('data-foo', '1'); \
         d.setAttribute('data-bar-baz', '2'); \
         d.setAttribute('class', 'ignored'); \
         var keys = []; for (var k in d.dataset) keys.push(k); \
         keys.join(',');");
    assert_eq!(out, "foo,barBaz");
}

#[test]
fn dataset_object_keys_yields_camel_case() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('data-a-b', 'x'); \
         d.setAttribute('data-c', 'y'); \
         Object.keys(d.dataset).join(',');");
    assert_eq!(out, "aB,c");
}

// --- spec edge cases --------------------------------------------

#[test]
fn dataset_get_absent_returns_undefined() {
    let out = run("var d = document.createElement('div'); \
         '' + (d.dataset.missing === undefined);");
    assert_eq!(out, "true");
}

#[test]
fn dataset_prototype_fallthrough_for_unset_keys() {
    // `dataset.toString` should resolve to `Object.prototype.toString`,
    // not return undefined — DOMStringMap is *not* `[OverrideBuiltIns]`.
    let out = run("var d = document.createElement('div'); \
         (typeof d.dataset.toString === 'function') ? 'function' : 'wrong';");
    assert_eq!(out, "function");
}

#[test]
fn dataset_in_operator_reflects_attr_presence() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('data-foo', '1'); \
         ('foo' in d.dataset) + '/' + ('missing' in d.dataset);");
    assert_eq!(out, "true/false");
}

#[test]
fn dataset_in_operator_falls_through_to_prototype() {
    // `'toString' in dataset` must walk the prototype chain and find
    // `Object.prototype.toString`.  The named-property exotic
    // [[HasProperty]] trap returns `Some(true)` only for present
    // data-* keys; absent keys return `None` to enable fall-through.
    let out = run("var d = document.createElement('div'); \
         '' + ('toString' in d.dataset);");
    assert_eq!(out, "true");
}

#[test]
fn dataset_in_operator_inherited_methods_visible() {
    // Confirms the prototype-chain fall-through across multiple
    // canonical Object.prototype names.
    let out = run("var d = document.createElement('div'); \
         ('hasOwnProperty' in d.dataset) + '/' \
         + ('isPrototypeOf' in d.dataset) + '/' \
         + ('valueOf' in d.dataset);");
    assert_eq!(out, "true/true/true");
}

// --- WebIDL §3.10 LegacyOverrideBuiltIns absent (R5 #1) ----------
//
// DOMStringMap is *not* `[LegacyOverrideBuiltIns]`; a `data-*`
// attribute whose camelCase key collides with an `Object.prototype`
// member MUST NOT shadow the inherited member — the named-property
// exotic [[Get]] / [[HasProperty]] / [[Delete]] / [[Set]] /
// [[OwnPropertyKeys]] traps fall through to the ordinary path.
// Tests use kebab-case attribute names (`data-to-string`) since
// HTML attribute storage lowercases names; the camel-conversion
// (`data-to-string` → `toString`) is what materialises the
// prototype-collision case.

#[test]
fn dataset_does_not_shadow_object_prototype_to_string() {
    // Even with a `data-to-string` attribute set, `dataset.toString`
    // must remain a function (`Object.prototype.toString`), not the
    // attribute value.
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('data-to-string', 'shadow'); \
         (typeof d.dataset.toString === 'function') ? 'function' : 'string';");
    assert_eq!(out, "function");
}

#[test]
fn dataset_does_not_shadow_object_prototype_has_own_property() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('data-has-own-property', 'shadow'); \
         (typeof d.dataset.hasOwnProperty === 'function') ? 'function' : 'string';");
    assert_eq!(out, "function");
}

#[test]
fn dataset_in_operator_with_shadowed_key_returns_true_via_prototype() {
    // `'toString' in dataset` must be true (Object.prototype has it),
    // even when `data-to-string` exists.
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('data-to-string', 'shadow'); \
         '' + ('toString' in d.dataset);");
    assert_eq!(out, "true");
}

#[test]
fn dataset_object_keys_filters_shadowed_keys() {
    // Object.keys(dataset) must NOT surface 'toString' even when
    // `data-to-string` is set.
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('data-to-string', 'shadow'); \
         d.setAttribute('data-foo', '1'); \
         Object.keys(d.dataset).join(',');");
    assert_eq!(out, "foo");
}

#[test]
fn dataset_set_with_shadowed_key_does_not_create_data_attr() {
    // `dataset.toString = 'x'` must fall through to ordinary [[Set]]
    // on the sealed wrapper.  Whether that throws TypeError or
    // silently fails depends on strict-mode context; in either case
    // the `data-to-string` attribute MUST NOT be created.
    let out = run("var d = document.createElement('div'); \
         try { d.dataset.toString = 'x'; } catch (e) {} \
         '' + d.hasAttribute('data-to-string');");
    assert_eq!(out, "false");
}

#[test]
fn dataset_shadowed_key_still_reachable_via_get_attribute() {
    // The `data-to-string` attribute is still reachable through
    // `getAttribute('data-to-string')` — only the dataset trap layer
    // is required to fall through to inherited members.
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('data-to-string', 'attr-value'); \
         d.getAttribute('data-to-string');");
    assert_eq!(out, "attr-value");
}

#[test]
fn dataset_has_own_property_reflects_attr() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('data-foo', '1'); \
         d.dataset.hasOwnProperty('foo') + '/' + d.dataset.hasOwnProperty('missing');");
    assert_eq!(out, "true/false");
}

// --- liveness across attribute mutations ------------------------

#[test]
fn dataset_reflects_set_attribute() {
    let out = run("var d = document.createElement('div'); \
         var ds = d.dataset; \
         d.setAttribute('data-late', 'arrived'); \
         ds.late;");
    assert_eq!(out, "arrived");
}

#[test]
fn dataset_set_visible_through_get_attribute() {
    let out = run("var d = document.createElement('div'); \
         d.dataset.aAa = 'v'; \
         d.getAttribute('data-a-aa');");
    assert_eq!(out, "v");
}

// --- ToPropertyKey coercion (R1 #1) ------------------------------
//
// ECMA §7.1.19 ToPropertyKey turns every non-Symbol value into a
// string before the named-property exotic dispatch.  Boolean / null /
// undefined / numeric keys must reflect the corresponding `data-*`
// attribute (or fall through to ordinary [[Get]] when absent).

#[test]
fn dataset_bracket_access_with_boolean_key() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('data-true', 'yes'); \
         d.dataset[true];");
    assert_eq!(out, "yes");
}

#[test]
fn dataset_bracket_access_with_null_key() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('data-null', 'nil'); \
         d.dataset[null];");
    assert_eq!(out, "nil");
}

#[test]
fn dataset_bracket_access_with_undefined_key() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('data-undefined', 'u'); \
         d.dataset[undefined];");
    assert_eq!(out, "u");
}

#[test]
fn dataset_in_operator_with_boolean_key() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('data-false', '1'); \
         (false in d.dataset) + '/' + (true in d.dataset);");
    assert_eq!(out, "true/false");
}

#[test]
fn dataset_set_with_boolean_key() {
    let out = run("var d = document.createElement('div'); \
         d.dataset[true] = 'present'; \
         d.getAttribute('data-true');");
    assert_eq!(out, "present");
}

#[test]
fn dataset_delete_with_null_key_vacuous_true() {
    // [[Delete]] returns true even for an absent key (WebIDL §3.10
    // deleter).  `null` coerces to "null" via ToPropertyKey.
    let out = run("var d = document.createElement('div'); \
         '' + (delete d.dataset[null]);");
    assert_eq!(out, "true");
}

// --- post-unbind tolerance (R1 #2) -------------------------------
//
// `el.dataset` retained across `Vm::unbind()` must not panic.
// [[Get]] / [[Has]] / [[OwnKeys]] fall through to ordinary semantics
// (sealed wrapper → no own keys); [[Set]] is a silent no-op;
// [[Delete]] returns vacuous true.

#[test]
fn dataset_traps_after_unbind_do_not_panic() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval(
        "var d = document.createElement('div'); \
         d.setAttribute('data-foo', '1'); \
         globalThis.ds = d.dataset;",
    )
    .unwrap();
    vm.unbind();

    // [[Get]] falls through → undefined for data-* keys post-unbind.
    let r = vm.eval("globalThis.ds.foo;").unwrap();
    assert!(matches!(r, JsValue::Undefined), "{r:?}");
    // [[Has]] falls through → false (sealed wrapper, no inherited
    // 'foo' on Object.prototype).
    let r = vm.eval("'foo' in globalThis.ds;").unwrap();
    assert!(matches!(r, JsValue::Boolean(false)), "{r:?}");
    // [[Set]] is a silent no-op (does not panic).
    vm.eval("globalThis.ds.bar = 'x';").unwrap();
    // [[Delete]] returns vacuous true.
    let r = vm.eval("delete globalThis.ds.foo;").unwrap();
    assert!(matches!(r, JsValue::Boolean(true)), "{r:?}");
    // for-in yields no keys (collect_keys returns Some(Ok([]))
    // post-unbind; ordinary for-in fallback would also yield 0).
    let r = vm
        .eval("var n = 0; for (var k in globalThis.ds) n++; n;")
        .unwrap();
    assert!(matches!(r, JsValue::Number(n) if n == 0.0), "{r:?}");
}
