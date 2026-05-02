//! `ReadableStream` + DefaultReader + DefaultController tests
//! (WHATWG Streams §4, Phase-2 read-output-only).
//!
//! Split into theme-based submodules to keep each file under the
//! project's 1000-line convention (PR-file-split-a, slot #10.5):
//!
//! - [`basics`] — constructor, brand checks, start callback,
//!   enqueue/read pairing, getReader/releaseLock/locked, error
//!   path, desiredSize, cancel.
//! - [`strategies`] — `CountQueuingStrategy` /
//!   `ByteLengthQueuingStrategy` (§6.1 / §6.2) + `highWaterMark`
//!   validation.
//! - [`body`] — `Response.body` / `Request.body` /
//!   `Blob.prototype.stream()` integration, body-input rejection,
//!   stream-level invariants (post-close enqueue, double-close,
//!   reader.closed resolve/reject).
//! - [`regressions`] — per-Copilot-round spec/correctness fixes
//!   carried in PR5-streams (#138) R1-R10 and PR-file-split-a
//!   R3 / R6 / R7.

#![cfg(feature = "engine")]

use crate::vm::value::JsValue;
use crate::vm::Vm;

mod basics;
mod body;
mod regressions;
mod strategies;

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

pub(super) fn eval_bool(vm: &mut Vm, source: &str) -> bool {
    match vm.eval(source).unwrap() {
        JsValue::Boolean(b) => b,
        other => panic!("expected bool, got {other:?}"),
    }
}

pub(super) fn eval_number(vm: &mut Vm, source: &str) -> f64 {
    match vm.eval(source).unwrap() {
        JsValue::Number(n) => n,
        other => panic!("expected number, got {other:?}"),
    }
}

pub(super) fn eval_global_string(source: &str, name: &str) -> String {
    let mut vm = Vm::new();
    vm.eval(source).unwrap();
    match vm.get_global(name) {
        Some(JsValue::String(id)) => vm.get_string(id),
        other => panic!("expected global {name} to be a string, got {other:?}"),
    }
}

pub(super) fn eval_global_bool(source: &str, name: &str) -> bool {
    let mut vm = Vm::new();
    vm.eval(source).unwrap();
    match vm.get_global(name) {
        Some(JsValue::Boolean(b)) => b,
        other => panic!("expected global {name} to be a bool, got {other:?}"),
    }
}
