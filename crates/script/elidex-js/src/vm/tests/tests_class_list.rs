//! M4-12 #11-classlist-dataset — `Element.classList` (DOMTokenList) tests.
//!
//! Covers:
//! - `el.classList` returns an identity-preserving `DOMTokenList` wrapper.
//! - `length`, `value` get/set, `item(i)`, `tokens[i]` indexed exotic.
//! - `add` / `remove` / `toggle` / `contains` / `replace` / `supports`.
//! - SyntaxError on empty token, InvalidCharacterError on whitespace.
//! - `for-of` iteration via `[Symbol.iterator]`.
//! - liveness: classList reflects `setAttribute('class', …)` mutations
//!   and vice versa.
//! - Brand check: `DOMTokenList.prototype.add.call({})` throws TypeError.

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
fn class_list_identity_preserved() {
    let out = run("var d = document.createElement('div'); \
         (d.classList === d.classList) ? 'same' : 'different';");
    assert_eq!(out, "same");
}

// --- length / value ----------------------------------------------

#[test]
fn class_list_length_and_value() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('class', 'foo bar baz'); \
         d.classList.length + ':' + d.classList.value;");
    assert_eq!(out, "3:foo bar baz");
}

#[test]
fn class_list_value_setter_replaces() {
    let out = run("var d = document.createElement('div'); \
         d.classList.value = 'a b c'; \
         d.getAttribute('class');");
    assert_eq!(out, "a b c");
}

#[test]
fn class_list_length_empty() {
    let out = run("var d = document.createElement('div'); \
         '' + d.classList.length;");
    assert_eq!(out, "0");
}

// --- item / indexed exotic ---------------------------------------

#[test]
fn class_list_item_and_indexed_access() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('class', 'a b c'); \
         d.classList.item(0) + '/' + d.classList[1] + '/' + d.classList.item(2);");
    assert_eq!(out, "a/b/c");
}

#[test]
fn class_list_item_oob_returns_null_indexed_oob_returns_undefined() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('class', 'x'); \
         (d.classList.item(5) === null) + '/' + (d.classList[5] === undefined);");
    assert_eq!(out, "true/true");
}

// --- contains / add / remove / toggle / replace ------------------

#[test]
fn class_list_contains() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('class', 'foo bar'); \
         d.classList.contains('foo') + '/' + d.classList.contains('missing');");
    assert_eq!(out, "true/false");
}

#[test]
fn class_list_add_multiple_tokens() {
    let out = run("var d = document.createElement('div'); \
         d.classList.add('a', 'b', 'c'); \
         d.getAttribute('class');");
    assert_eq!(out, "a b c");
}

#[test]
fn class_list_add_idempotent() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('class', 'foo'); \
         d.classList.add('foo'); \
         d.getAttribute('class');");
    assert_eq!(out, "foo");
}

#[test]
fn class_list_remove() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('class', 'a b c'); \
         d.classList.remove('b'); \
         d.getAttribute('class');");
    assert_eq!(out, "a c");
}

#[test]
fn class_list_toggle_no_force() {
    let out = run("var d = document.createElement('div'); \
         var added = d.classList.toggle('x'); \
         var removed = d.classList.toggle('x'); \
         added + '/' + removed + '/' + d.getAttribute('class');");
    assert_eq!(out, "true/false/");
}

#[test]
fn class_list_toggle_force_true_adds_only_when_absent() {
    let out = run("var d = document.createElement('div'); \
         d.classList.toggle('x', true); \
         d.classList.toggle('x', true); \
         d.getAttribute('class');");
    assert_eq!(out, "x");
}

#[test]
fn class_list_toggle_force_false_never_adds() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('class', 'a'); \
         var r = d.classList.toggle('a', false); \
         r + '/' + d.getAttribute('class');");
    assert_eq!(out, "false/");
}

#[test]
fn class_list_replace_returns_bool() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('class', 'foo bar'); \
         var r1 = d.classList.replace('foo', 'baz'); \
         var r2 = d.classList.replace('missing', 'x'); \
         r1 + '/' + r2 + '/' + d.getAttribute('class');");
    assert_eq!(out, "true/false/baz bar");
}

// --- spec error mapping ------------------------------------------

#[test]
fn class_list_add_empty_throws_syntax_error() {
    let out = run("var d = document.createElement('div'); \
         try { d.classList.add(''); 'no-throw'; } \
         catch (e) { e.name; }");
    assert_eq!(out, "SyntaxError");
}

#[test]
fn class_list_add_whitespace_throws_invalid_character_error() {
    let out = run("var d = document.createElement('div'); \
         try { d.classList.add('a b'); 'no-throw'; } \
         catch (e) { e.name; }");
    assert_eq!(out, "InvalidCharacterError");
}

#[test]
fn class_list_supports_always_throws_type_error() {
    let out = run("var d = document.createElement('div'); \
         try { d.classList.supports('foo'); 'no-throw'; } \
         catch (e) { (e instanceof TypeError) ? 'TypeError' : 'wrong:' + e.name; }");
    assert_eq!(out, "TypeError");
}

// --- stringifier (WebIDL `stringifier;` on WHATWG DOM §7.1) ------

#[test]
fn class_list_string_coercion_returns_value() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('class', 'foo bar'); \
         String(d.classList);");
    assert_eq!(out, "foo bar");
}

#[test]
fn class_list_template_interpolation() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('class', 'a b'); \
         `[${d.classList}]`;");
    assert_eq!(out, "[a b]");
}

#[test]
fn class_list_to_string_method_present() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('class', 'x y z'); \
         d.classList.toString();");
    assert_eq!(out, "x y z");
}

// --- iteration ---------------------------------------------------

#[test]
fn class_list_iterator_for_of() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('class', 'a b c'); \
         var r = []; for (var t of d.classList) r.push(t); \
         r.join(',');");
    assert_eq!(out, "a,b,c");
}

// --- liveness across attribute mutations -------------------------

#[test]
fn class_list_reflects_set_attribute() {
    let out = run("var d = document.createElement('div'); \
         var cl = d.classList; \
         d.setAttribute('class', 'x y'); \
         cl.length + ':' + cl.contains('x');");
    assert_eq!(out, "2:true");
}

#[test]
fn class_list_empty_after_remove_attribute() {
    let out = run("var d = document.createElement('div'); \
         d.classList.value = 'a b c'; \
         var cl = d.classList; \
         d.removeAttribute('class'); \
         '' + cl.length;");
    assert_eq!(out, "0");
}

// --- brand check -------------------------------------------------

// --- R3 regressions ----------------------------------------------
//
// Copilot R3 #1 — `toggle(token, force)` with `force === undefined`
// must apply WebIDL `boolean` coercion (→ false), behaving like
// `force=false` (never add, always return false), NOT like the
// 1-arg overload (flip current state).

#[test]
fn class_list_toggle_with_explicit_undefined_force_is_false() {
    // Element starts without 'a'; toggle('a', undefined) must NOT
    // add it (force=false → "remove if present, else no-op").
    let out = run("var d = document.createElement('div'); \
         var r1 = d.classList.toggle('a', undefined); \
         var present = d.classList.contains('a'); \
         '' + r1 + '/' + present;");
    assert_eq!(out, "false/false");
}

#[test]
fn class_list_toggle_no_arg_flips() {
    // Sanity: 1-arg overload still flips (regression guard for the
    // R3 #1 fix not breaking the no-force path).
    let out = run("var d = document.createElement('div'); \
         var r1 = d.classList.toggle('a'); \
         var present = d.classList.contains('a'); \
         '' + r1 + '/' + present;");
    assert_eq!(out, "true/true");
}

// Copilot R3 #2 — `try_indexed_get` must use exact integer
// round-trip rather than EPSILON tolerance.  A near-but-not-equal
// number like `1.0000000000000002` must fall through to ordinary
// property lookup (returns undefined for non-canonical key) instead
// of resolving to `tokens[1]`.

#[test]
fn class_list_indexed_get_rejects_non_canonical_numeric() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('class', 'first second third'); \
         var v = d.classList[1.0000000000000002]; \
         '' + (v === undefined);");
    assert_eq!(out, "true");
}

#[test]
fn class_list_indexed_get_exact_integer_still_works() {
    // Sanity: exact integer indexing is unchanged.
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('class', 'a b c'); \
         d.classList[1];");
    assert_eq!(out, "b");
}

// Copilot R7 #1 — `DOMTokenList.item(unsigned long)` per WebIDL
// §3.10.13: negative inputs are coerced via ToUint32 (mod 2^32),
// which for `-1` yields `4294967295`, well past any token list
// length → handler must return `null`.  Previously a generic
// `to_number` would have passed `-1` through and the handler's
// float→usize cast would map it to `0`, returning the first token.

#[test]
fn class_list_item_negative_index_returns_null_not_first_token() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('class', 'first second'); \
         '' + (d.classList.item(-1) === null);");
    assert_eq!(out, "true");
}

#[test]
fn class_list_item_huge_index_returns_null() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('class', 'a b'); \
         '' + (d.classList.item(2147483647) === null);");
    assert_eq!(out, "true");
}

#[test]
fn class_list_method_brand_check() {
    let out = run("var d = document.createElement('div'); \
         try { d.classList.add.call({}, 'foo'); 'no-throw'; } \
         catch (e) { (e instanceof TypeError) ? 'TypeError' : 'wrong:' + e.name; }");
    assert_eq!(out, "TypeError");
}

// --- post-unbind tolerance (R1 #3) -------------------------------
//
// `el.classList` retained across `Vm::unbind()` must not panic.
// `length` → 0, `value` → "", `contains/toggle/replace` → false,
// `item(i)` / `tokens[i]` → null/undefined, mutators no-op,
// `supports` throws TypeError matching the bound-state message.

#[test]
fn class_list_methods_after_unbind_return_safe_defaults() {
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
         d.setAttribute('class', 'a b c'); \
         globalThis.cl = d.classList;",
    )
    .unwrap();
    vm.unbind();

    let r = vm.eval("globalThis.cl.length;").unwrap();
    assert!(matches!(r, JsValue::Number(n) if n == 0.0), "{r:?}");
    let r = vm.eval("globalThis.cl.value;").unwrap();
    if let JsValue::String(sid) = r {
        assert_eq!(vm.inner.strings.get_utf8(sid), "");
    } else {
        panic!("expected String, got {r:?}");
    }
    let r = vm.eval("globalThis.cl.contains('a');").unwrap();
    assert!(matches!(r, JsValue::Boolean(false)), "{r:?}");
    let r = vm.eval("globalThis.cl.item(0);").unwrap();
    assert!(matches!(r, JsValue::Null), "{r:?}");
    // Indexed exotic also tolerates unbound.
    let r = vm.eval("globalThis.cl[0];").unwrap();
    assert!(matches!(r, JsValue::Undefined), "{r:?}");
    let r = vm.eval("globalThis.cl.toggle('a');").unwrap();
    assert!(matches!(r, JsValue::Boolean(false)), "{r:?}");
    let r = vm.eval("globalThis.cl.replace('a', 'b');").unwrap();
    assert!(matches!(r, JsValue::Boolean(false)), "{r:?}");
    // Mutators no-op (does not panic).
    vm.eval("globalThis.cl.add('z'); globalThis.cl.remove('a');")
        .unwrap();
    // `value` setter no-op.
    vm.eval("globalThis.cl.value = 'x';").unwrap();
    // Iterator yields zero values.
    let r = vm
        .eval("var n = 0; for (var t of globalThis.cl) n++; n;")
        .unwrap();
    assert!(matches!(r, JsValue::Number(n) if n == 0.0), "{r:?}");
    // Stringifier returns "".
    let r = vm.eval("'' + globalThis.cl;").unwrap();
    if let JsValue::String(sid) = r {
        assert_eq!(vm.inner.strings.get_utf8(sid), "");
    } else {
        panic!("expected String, got {r:?}");
    }
    // `supports` still throws TypeError (matches bound-state spec).
    let r = vm
        .eval("try { globalThis.cl.supports('x'); 'no-throw'; } catch(e) { e.name; }")
        .unwrap();
    if let JsValue::String(sid) = r {
        assert_eq!(vm.inner.strings.get_utf8(sid), "TypeError");
    } else {
        panic!("expected String, got {r:?}");
    }
}
