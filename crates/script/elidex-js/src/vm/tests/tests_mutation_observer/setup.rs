//! Phase C2 / C3 — constructor, brand checks, and `observe()` init
//! parsing (including TypeError shape for invalid args).
//!
//! Companion to [`super::delivery`] (C4 + later additions, where the
//! actual `MutationRecord` plumbing is exercised) and
//! [`super::lifecycle`] (C5 unbind + rebind).

use super::super::super::value::JsValue;
use super::super::super::Vm;
use super::{run, run_throws};

// --- C2 — prototype + constructor brand check -----------------------

#[test]
fn mutation_observer_prototype_installed() {
    let mut vm = Vm::new();
    assert!(
        vm.inner.mutation_observer_prototype.is_some(),
        "MutationObserver.prototype must be allocated during register_globals"
    );
    // Global binding present.  `typeof X === 'function'` evaluates
    // to `false` (not a ReferenceError) for an undeclared `X`, so
    // `.is_ok()` alone would pass even if `MutationObserver` were
    // never installed — assert the returned value is `true`
    // instead (Copilot R3 PR #168).
    let result = vm
        .eval("typeof MutationObserver === 'function'")
        .expect("typeof expression must not throw");
    assert_eq!(
        result,
        JsValue::Boolean(true),
        "`MutationObserver` global binding must be installed by register_globals"
    );
}

#[test]
fn mutation_observer_constructor_creates_instance() {
    let out = run("var mo = new MutationObserver(function(){}); typeof mo;");
    assert_eq!(out, "object");
}

#[test]
fn mutation_observer_constructor_without_host_data_throws() {
    // Regression: prior implementation called `ctx.host()`
    // unconditionally, panicking when JS executed before
    // `Vm::install_host_data` (e.g. embedder ergonomics tests or
    // any pre-bind `vm.eval`).  The constructor must surface a
    // TypeError instead so `try { new MutationObserver(...) }
    // catch (e) {}` works pre-init.
    let mut vm = Vm::new();
    let err = vm
        .eval("new MutationObserver(function(){})")
        .expect_err("constructor must error pre-install_host_data");
    let err_text = format!("{err:?}");
    assert!(
        err_text.contains("host environment is not initialised"),
        "expected pre-init TypeError, got: {err_text}"
    );
}

#[test]
fn mutation_observer_constructor_requires_callable() {
    let err = run_throws("new MutationObserver(123);");
    assert!(
        err.contains("not of type 'Function'"),
        "expected MutationObserver callable TypeError, got: {err}"
    );
}

#[test]
fn mutation_observer_constructor_bare_call_throws() {
    let err = run_throws("MutationObserver(function(){});");
    assert!(
        err.contains("'new' operator"),
        "expected bare-call TypeError, got: {err}"
    );
}

#[test]
fn mutation_observer_instanceof_works() {
    let out = run("var mo = new MutationObserver(function(){}); \
         (mo instanceof MutationObserver) ? 'yes' : 'no';");
    assert_eq!(out, "yes");
}

#[test]
fn mutation_observer_method_brand_check_disconnect() {
    let err = run_throws("MutationObserver.prototype.disconnect.call({});");
    assert!(
        err.contains("Illegal invocation"),
        "expected brand-check TypeError, got: {err}"
    );
}

#[test]
fn mutation_observer_method_brand_check_take_records() {
    let err = run_throws("MutationObserver.prototype.takeRecords.call({});");
    assert!(
        err.contains("Illegal invocation"),
        "expected brand-check TypeError, got: {err}"
    );
}

#[test]
fn mutation_observer_method_brand_check_observe() {
    let err =
        run_throws("MutationObserver.prototype.observe.call({}, document, {childList:true});");
    assert!(
        err.contains("Illegal invocation"),
        "expected brand-check TypeError, got: {err}"
    );
}

#[test]
fn mutation_observer_take_records_initially_empty() {
    let out = run("var mo = new MutationObserver(function(){}); \
         var r = mo.takeRecords(); \
         Array.isArray(r) + ':' + r.length;");
    assert_eq!(out, "true:0");
}

#[test]
fn mutation_observer_disconnect_returns_undefined() {
    let out = run("var mo = new MutationObserver(function(){}); \
         typeof mo.disconnect();");
    assert_eq!(out, "undefined");
}

// --- C3 — observe / init parsing / TypeErrors ----------------------

#[test]
fn mutation_observer_observe_returns_undefined() {
    let out = run("var mo = new MutationObserver(function(){}); \
         typeof mo.observe(document, {childList:true});");
    assert_eq!(out, "undefined");
}

#[test]
fn mutation_observer_observe_requires_at_least_one_flag() {
    let err = run_throws(
        "var mo = new MutationObserver(function(){}); \
         mo.observe(document, {});",
    );
    assert!(
        err.contains("at least one"),
        "expected 'at least one' TypeError, got: {err}"
    );
}

#[test]
fn mutation_observer_observe_target_must_be_node() {
    let err = run_throws(
        "var mo = new MutationObserver(function(){}); \
         mo.observe({}, {childList:true});",
    );
    assert!(
        err.contains("not of type 'Node'"),
        "expected non-Node TypeError, got: {err}"
    );
}

#[test]
fn mutation_observer_observe_requires_two_arguments() {
    // WebIDL: `observe(Node target, MutationObserverInit options)` —
    // both required.  Match Chrome/Firefox arg-count error message
    // before falling through to per-argument coercion errors.
    let err_zero = run_throws("var mo = new MutationObserver(function(){}); mo.observe();");
    assert!(
        err_zero.contains("2 arguments required") && err_zero.contains("only 0 present"),
        "expected '2 arguments required, but only 0 present', got: {err_zero}"
    );
    let err_one = run_throws("var mo = new MutationObserver(function(){}); mo.observe(document);");
    assert!(
        err_one.contains("2 arguments required") && err_one.contains("only 1 present"),
        "expected '2 arguments required, but only 1 present', got: {err_one}"
    );
}

#[test]
fn mutation_observer_observe_attributes_implicit_via_old_value() {
    // attributeOldValue alone should be sufficient (spec §4.3.2 step 3).
    let out = run("var mo = new MutationObserver(function(){}); \
         try { mo.observe(document, {attributeOldValue:true}); 'ok' } \
         catch(e) { 'threw: ' + e.message; }");
    assert_eq!(out, "ok");
}

#[test]
fn mutation_observer_observe_character_data_implicit_via_old_value() {
    let out = run("var mo = new MutationObserver(function(){}); \
         try { mo.observe(document, {characterDataOldValue:true}); 'ok' } \
         catch(e) { 'threw: ' + e.message; }");
    assert_eq!(out, "ok");
}

#[test]
fn mutation_observer_observe_explicit_attributes_false_with_old_value_throws() {
    // WHATWG DOM §4.3.2 step 6: `attributeOldValue: true` requires
    // `attributes: true` (or absent).  Browser-aligned: Chrome /
    // Firefox throw a TypeError citing both fields.
    let err = run_throws(
        "var mo = new MutationObserver(function(){}); \
         mo.observe(document, {childList:true, attributes:false, attributeOldValue:true});",
    );
    assert!(
        err.contains("'attributeOldValue'") && err.contains("'attributes'"),
        "expected attributeOldValue/attributes mismatch TypeError, got: {err}"
    );
}

#[test]
fn mutation_observer_observe_explicit_attributes_false_with_filter_throws() {
    // §4.3.2 step 7: `attributeFilter` requires `attributes: true`
    // (or absent).
    let err = run_throws(
        "var mo = new MutationObserver(function(){}); \
         mo.observe(document, {childList:true, attributes:false, attributeFilter:['class']});",
    );
    assert!(
        err.contains("'attributeFilter'") && err.contains("'attributes'"),
        "expected attributeFilter/attributes mismatch TypeError, got: {err}"
    );
}

#[test]
fn mutation_observer_observe_explicit_character_data_false_with_old_value_throws() {
    // §4.3.2 step 8: `characterDataOldValue: true` requires
    // `characterData: true` (or absent).
    let err = run_throws(
        "var mo = new MutationObserver(function(){}); \
         mo.observe(document, {childList:true, characterData:false, characterDataOldValue:true});",
    );
    assert!(
        err.contains("'characterDataOldValue'") && err.contains("'characterData'"),
        "expected characterDataOldValue/characterData mismatch TypeError, got: {err}"
    );
}

#[test]
fn mutation_observer_observe_attribute_filter_non_iterable_throws() {
    // WebIDL §3.10.20 sequence conversion: a non-iterable
    // `attributeFilter` must TypeError, not silently fall through to
    // a stale-empty filter.
    let err = run_throws(
        "var mo = new MutationObserver(function(){}); \
         mo.observe(document, {attributeFilter: 'class'});",
    );
    assert!(
        err.contains("'attributeFilter' is not iterable"),
        "expected attributeFilter non-iterable TypeError, got: {err}"
    );
    let err = run_throws(
        "var mo = new MutationObserver(function(){}); \
         mo.observe(document, {attributeFilter: 42});",
    );
    assert!(
        err.contains("'attributeFilter' is not iterable"),
        "expected attributeFilter non-iterable TypeError, got: {err}"
    );
}

#[test]
fn mutation_observer_observe_attribute_filter_negative_length_clamps_to_zero() {
    // Regression: prior `to_uint32` length conversion wrapped
    // `length: -1` to `u32::MAX`, triggering a ~4 GiB
    // `Vec::with_capacity` and an OOM abort.  ToLength clamps
    // negative / NaN to 0; the iteration produces an empty filter.
    let out = run("var mo = new MutationObserver(function(){}); \
         try { mo.observe(document, {attributes:true, attributeFilter:{length:-1}}); 'ok' } \
         catch(e) { 'threw: ' + e.message; }");
    assert_eq!(out, "ok");
    let out = run("var mo = new MutationObserver(function(){}); \
         try { mo.observe(document, {attributes:true, attributeFilter:{length:NaN}}); 'ok' } \
         catch(e) { 'threw: ' + e.message; }");
    assert_eq!(out, "ok");
}

#[test]
fn mutation_observer_observe_attribute_filter_oversize_length_throws() {
    // Lengths exceeding u32::MAX (4_294_967_295) are not
    // representable as array indices in this VM; surface a
    // RangeError rather than silently truncating or attempting a
    // pathological allocation.
    let err = run_throws(
        "var mo = new MutationObserver(function(){}); \
         mo.observe(document, {attributes:true, attributeFilter:{length:4294967296}});",
    );
    assert!(
        err.contains("supported maximum"),
        "expected attributeFilter oversize RangeError, got: {err}"
    );
}

#[test]
fn mutation_observer_observe_attribute_filter_dos_length_cap_enforced() {
    // Regression: prior code accepted any `length` up to `u32::MAX`
    // and iterated `"0".."N-1"` interning each numeric index into
    // the permanent `StringPool` — a hostile length (e.g. 4 billion)
    // would have ballooned CPU + memory on a single observe() call.
    // The new cap matches the IntersectionObserver threshold cap
    // (65_536).  A length far below `u32::MAX` but above the cap
    // must throw RangeError, NOT enter the per-index loop.
    let err = run_throws(
        "var mo = new MutationObserver(function(){}); \
         mo.observe(document, {attributes:true, attributeFilter:{length:100000}});",
    );
    assert!(
        err.contains("supported maximum"),
        "expected attributeFilter DoS cap RangeError, got: {err}"
    );
}

#[test]
fn mutation_observer_observe_attribute_filter_implies_attributes() {
    let out = run("var mo = new MutationObserver(function(){}); \
         try { mo.observe(document, {attributeFilter: ['class']}); 'ok' } \
         catch(e) { 'threw: ' + e.message; }");
    assert_eq!(out, "ok");
}

// --- Argument validation edge cases ----------------------------------

#[test]
fn mutation_observer_observe_null_target_throws() {
    let err = run_throws(
        "var mo = new MutationObserver(function(){}); \
         mo.observe(null, {childList:true});",
    );
    assert!(
        err.contains("not of type 'Node'"),
        "expected null-target TypeError, got: {err}"
    );
}

#[test]
fn mutation_observer_observe_null_options_uses_defaults() {
    // WebIDL §3.10.7: null and undefined both yield the default-init
    // dictionary; the subsequent at-least-one-flag check then fires.
    let err = run_throws(
        "var mo = new MutationObserver(function(){}); \
         mo.observe(document, null);",
    );
    assert!(
        err.contains("at least one"),
        "null options should default-init then fail at-least-one-flag, got: {err}"
    );
}
