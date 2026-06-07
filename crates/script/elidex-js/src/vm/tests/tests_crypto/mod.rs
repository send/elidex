//! WebCrypto `Crypto` / `SubtleCrypto` tests (slots
//! `#11-crypto-subtle-min` + `#11-crypto-subtle-full` PR-1).
//!
//! The combined surface exceeds the 1000-line file convention, so the
//! tests are split topic-aligned into a directory module sharing the
//! `eval_*` harness helpers below:
//!
//! - [`crypto_global`] — `globalThis.crypto` + `Crypto` constructor
//!   stub, `getRandomValues` (type matrix / quota / identity / brand),
//!   `randomUUID`, and the `crypto.subtle` `[SameObject]` accessor.
//! - [`digest`] — `SubtleCrypto.digest` (algorithm normalization,
//!   BufferSource matrix, rejection paths, Promise identity + the
//!   async receiver brand check).
//! - [`hmac`] — the HMAC vertical (`generateKey` / `importKey` /
//!   `exportKey` / `sign` / `verify`) and the Web IDL argument /
//!   sequence / dictionary conversion conformance batches.
//! - [`aes`] — the AES-GCM / AES-CBC / AES-CTR vertical (`generateKey` /
//!   `importKey` / `exportKey` / `encrypt` / `decrypt`, PR-2).
//! - [`derive`] — the HKDF / PBKDF2 derive vertical (`importKey` /
//!   `deriveBits` / `deriveKey`, PR-3a).
//! - [`crypto_key`] — `CryptoKey` accessors + the `[[algorithm]]` /
//!   `[[usages]]` §13.4 caches and their GC / side-store invariants.

#![cfg(feature = "engine")]

mod aes;
mod crypto_global;
mod crypto_key;
mod derive;
mod digest;
mod hmac;

use super::super::value::JsValue;
use super::super::Vm;
use super::super::VmError;

pub(super) fn eval_string(source: &str) -> String {
    let mut vm = Vm::new();
    match vm.eval(source).unwrap() {
        JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string, got {other:?}"),
    }
}

pub(super) fn eval_bool(source: &str) -> bool {
    let mut vm = Vm::new();
    match vm.eval(source).unwrap() {
        JsValue::Boolean(b) => b,
        other => panic!("expected bool, got {other:?}"),
    }
}

pub(super) fn eval_number(source: &str) -> f64 {
    let mut vm = Vm::new();
    match vm.eval(source).unwrap() {
        JsValue::Number(n) => n,
        other => panic!("expected number, got {other:?}"),
    }
}

pub(super) fn eval_err(source: &str) -> VmError {
    let mut vm = Vm::new();
    vm.eval(source).unwrap_err()
}

/// Run `source` and return the value of `globalThis.<name>` after
/// the eval has drained its microtask queue (so `.then`-installed
/// values are visible).  Mirrors `tests_body_mixin::eval_global_*`.
pub(super) fn eval_global_string(source: &str, name: &str) -> String {
    let mut vm = Vm::new();
    vm.eval(source).unwrap();
    match vm.get_global(name) {
        Some(JsValue::String(id)) => vm.get_string(id),
        other => panic!("expected global {name} to be a string, got {other:?}"),
    }
}

pub(super) fn eval_global_number(source: &str, name: &str) -> f64 {
    let mut vm = Vm::new();
    vm.eval(source).unwrap();
    match vm.get_global(name) {
        Some(JsValue::Number(n)) => n,
        other => panic!("expected global {name} to be a number, got {other:?}"),
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

pub(super) fn assert_typeerror(err: &VmError) {
    // VmError::type_error renders as `TypeError: ...`.
    assert!(
        err.to_string().starts_with("TypeError"),
        "expected TypeError, got: {err}"
    );
}
