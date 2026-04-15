//! PR4b C6: `location` host-global tests.
//!
//! Covers initial state (`about:blank`), href round-trip via setter,
//! component getters (protocol/host/…/origin), assign / replace /
//! toString / reload stubs, and the history-entry bookkeeping that
//! `assign` vs `replace` performs.

#![cfg(feature = "engine")]

use super::super::value::JsValue;
use super::super::Vm;

fn eval_string(vm: &mut Vm, source: &str) -> String {
    match vm.eval(source).unwrap() {
        JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string, got {other:?}"),
    }
}

#[test]
fn location_is_object() {
    let mut vm = Vm::new();
    assert_eq!(eval_string(&mut vm, "typeof location;"), "object");
}

#[test]
fn location_href_defaults_to_about_blank() {
    let mut vm = Vm::new();
    assert_eq!(eval_string(&mut vm, "location.href;"), "about:blank");
}

#[test]
fn location_href_setter_updates_current_url() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "location.href = 'https://example.com/a?x=1#y'; location.href;"
        ),
        "https://example.com/a?x=1#y"
    );
}

#[test]
fn location_component_getters() {
    let mut vm = Vm::new();
    vm.eval("location.href = 'https://example.com:8443/a/b?x=1#y';")
        .unwrap();
    assert_eq!(eval_string(&mut vm, "location.protocol;"), "https:");
    assert_eq!(eval_string(&mut vm, "location.host;"), "example.com:8443");
    assert_eq!(eval_string(&mut vm, "location.hostname;"), "example.com");
    assert_eq!(eval_string(&mut vm, "location.port;"), "8443");
    assert_eq!(eval_string(&mut vm, "location.pathname;"), "/a/b");
    assert_eq!(eval_string(&mut vm, "location.search;"), "?x=1");
    assert_eq!(eval_string(&mut vm, "location.hash;"), "#y");
    assert_eq!(
        eval_string(&mut vm, "location.origin;"),
        "https://example.com:8443"
    );
}

#[test]
fn location_component_getters_no_port_no_query() {
    let mut vm = Vm::new();
    vm.eval("location.href = 'http://elidex.test/';").unwrap();
    assert_eq!(eval_string(&mut vm, "location.host;"), "elidex.test");
    assert_eq!(eval_string(&mut vm, "location.port;"), "");
    assert_eq!(eval_string(&mut vm, "location.search;"), "");
    assert_eq!(eval_string(&mut vm, "location.hash;"), "");
}

#[test]
fn location_origin_is_null_for_opaque_schemes() {
    let mut vm = Vm::new();
    // `about:blank` — no authority, opaque origin → "null".
    assert_eq!(eval_string(&mut vm, "location.origin;"), "null");
}

#[test]
fn location_to_string_matches_href() {
    let mut vm = Vm::new();
    vm.eval("location.href = 'https://example.com/';").unwrap();
    assert_eq!(
        eval_string(&mut vm, "location.toString();"),
        "https://example.com/"
    );
    // Note: `'' + location` would require `§7.1.12 step 9 ToPrimitive(val,
    // "string")` to call user `toString` — tracked as a VM-wide
    // follow-up (see `vm/coerce.rs` `to_string` KNOWN LIMITATION).
    // Until then it collapses to `"[object Object]"`; not a Location
    // defect.
}

#[test]
fn location_assign_pushes_new_history_entry() {
    let mut vm = Vm::new();
    // Start: about:blank (1 entry). `assign` adds one more each call.
    // `history` global lands in C7; read via the internal nav state.
    vm.eval("location.assign('https://a/')").unwrap();
    assert_eq!(vm.inner.navigation.history_entries.len(), 2);
    vm.eval("location.assign('https://b/')").unwrap();
    assert_eq!(vm.inner.navigation.history_entries.len(), 3);
    assert_eq!(vm.inner.navigation.history_index, 2);
}

#[test]
fn location_replace_overwrites_current_entry() {
    let mut vm = Vm::new();
    // `replace` overwrites in place — history length stays 1.
    vm.eval("location.replace('https://a/')").unwrap();
    assert_eq!(vm.inner.navigation.history_entries.len(), 1);
    vm.eval("location.replace('https://b/')").unwrap();
    assert_eq!(vm.inner.navigation.history_entries.len(), 1);
    assert_eq!(eval_string(&mut vm, "location.href;"), "https://b/");
}

#[test]
fn location_reload_is_no_op_but_callable() {
    let mut vm = Vm::new();
    vm.eval("location.reload();").unwrap();
    assert_eq!(eval_string(&mut vm, "location.href;"), "about:blank");
}
