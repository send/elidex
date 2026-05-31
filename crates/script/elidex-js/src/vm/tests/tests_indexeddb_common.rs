//! Shared fixtures for the IndexedDB JS-surface test suite (slot
//! `#11-indexed-db-vm` / D-20a), split out of `tests_indexeddb.rs` so each
//! IDB test module stays under the repo's ~1000-line convention.
//!
//! The async model is the focus of these tests: a request's `success` /
//! `error` event fires from a **database task** drained at the `drain_tasks`
//! tail (§5.6 step 5.6), *not* inline (the boa bridge fired inline = bug, not
//! copied).  `Vm::eval` drains tasks after the top-level script returns, so
//! the pattern is: run a setup script that wires `onupgradeneeded` /
//! `onsuccess` callbacks writing into a persistent `globalThis.__log`, then
//! read `__log` in a second `eval` (by which point the post-eval drain has
//! run every queued task).

#![cfg(feature = "engine")]

use elidex_ecs::{Attributes, EcsDom};
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;

fn build_min_fixture(dom: &mut EcsDom) -> elidex_ecs::Entity {
    let doc = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    let body = dom.create_element("body", Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, body));
    doc
}

struct UnbindOnDrop<'a>(&'a mut Vm);

impl Drop for UnbindOnDrop<'_> {
    fn drop(&mut self) {
        self.0.unbind();
    }
}

pub(crate) fn with_vm<F, R>(f: F) -> R
where
    F: FnOnce(&mut Vm) -> R,
{
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_min_fixture(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let guard = UnbindOnDrop(&mut vm);
    f(guard.0)
}

pub(crate) fn eval_bool(vm: &mut Vm, source: &str) -> bool {
    match vm.eval(source).unwrap() {
        JsValue::Boolean(b) => b,
        other => panic!("expected bool, got {other:?} for `{source}`"),
    }
}

pub(crate) fn eval_string(vm: &mut Vm, source: &str) -> String {
    match vm.eval(source).unwrap() {
        JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string, got {other:?} for `{source}`"),
    }
}

pub(crate) fn eval_number(vm: &mut Vm, source: &str) -> f64 {
    match vm.eval(source).unwrap() {
        JsValue::Number(n) => n,
        other => panic!("expected number, got {other:?} for `{source}`"),
    }
}
