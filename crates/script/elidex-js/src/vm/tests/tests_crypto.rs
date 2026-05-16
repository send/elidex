//! WebCrypto `Crypto` / `SubtleCrypto` tests (slot
//! `#11-crypto-subtle-min`).
//!
//! Phase 1 scope: `globalThis.crypto` data prop + `Crypto`
//! constructor stub + `Crypto.prototype.getRandomValues`.
//! Phase 2 (randomUUID) and Phase 3 (SubtleCrypto.digest) add
//! their own test sections below.

#![cfg(feature = "engine")]

use super::super::value::JsValue;
use super::super::Vm;
use super::super::VmError;

fn eval_string(source: &str) -> String {
    let mut vm = Vm::new();
    match vm.eval(source).unwrap() {
        JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string, got {other:?}"),
    }
}

fn eval_bool(source: &str) -> bool {
    let mut vm = Vm::new();
    match vm.eval(source).unwrap() {
        JsValue::Boolean(b) => b,
        other => panic!("expected bool, got {other:?}"),
    }
}

fn eval_number(source: &str) -> f64 {
    let mut vm = Vm::new();
    match vm.eval(source).unwrap() {
        JsValue::Number(n) => n,
        other => panic!("expected number, got {other:?}"),
    }
}

fn eval_err(source: &str) -> VmError {
    let mut vm = Vm::new();
    vm.eval(source).unwrap_err()
}

// ---------------------------------------------------------------------------
// `globalThis.crypto` + `Crypto` constructor stub
// ---------------------------------------------------------------------------

#[test]
fn crypto_is_an_object_on_global_this() {
    assert_eq!(eval_string("typeof globalThis.crypto;"), "object");
    assert!(eval_bool("globalThis.crypto !== null;"));
}

#[test]
fn crypto_is_instance_of_crypto() {
    assert!(eval_bool("globalThis.crypto instanceof Crypto;"));
}

#[test]
fn crypto_singleton_has_same_object_semantics() {
    // [SameObject] per WebIDL §10 — repeated reads return the
    // SAME ObjectId.
    assert!(eval_bool("globalThis.crypto === globalThis.crypto;"));
}

#[test]
fn crypto_constructor_throws_illegal_constructor() {
    let err = eval_err("new Crypto();");
    assert!(
        err.to_string().contains("Illegal constructor"),
        "unexpected error: {err}"
    );
}

#[test]
fn crypto_prototype_constructor_chain() {
    assert!(
        eval_bool("Object.getPrototypeOf(globalThis.crypto) === Crypto.prototype;"),
        "crypto -> Crypto.prototype chain broken"
    );
    assert!(
        eval_bool("Crypto.prototype.constructor === Crypto;"),
        "Crypto.prototype.constructor mismatch"
    );
    // Crypto.prototype chains through to the root prototype.  Walk
    // the chain to verify it terminates at null (i.e. the chain is
    // well-formed; the root prototype itself is opaque — see
    // `register_object_global` in `vm/globals.rs` which omits the
    // `Object.prototype` data prop on the `Object` global).
    assert!(
        eval_bool(
            "let p = Object.getPrototypeOf(Crypto.prototype); \
             p !== null && Object.getPrototypeOf(p) === null;"
        ),
        "Crypto.prototype's parent should be the root prototype (1-step from null)"
    );
}

// ---------------------------------------------------------------------------
// `getRandomValues` — type acceptance
// ---------------------------------------------------------------------------

#[test]
fn get_random_values_accepts_uint8_array() {
    // After call: at least one byte should be non-zero with very
    // high probability (1 - 1/2^64 for 8 bytes).
    assert!(eval_bool(
        "let v = new Uint8Array(8); \
         crypto.getRandomValues(v); \
         v.some(b => b !== 0);"
    ));
}

#[test]
fn get_random_values_accepts_uint8_clamped_array() {
    assert!(eval_bool(
        "let v = new Uint8ClampedArray(8); \
         crypto.getRandomValues(v); \
         v.some(b => b !== 0);"
    ));
}

#[test]
fn get_random_values_accepts_int32_array() {
    assert!(eval_bool(
        "let v = new Int32Array(4); \
         crypto.getRandomValues(v); \
         v.some(b => b !== 0);"
    ));
}

#[test]
fn get_random_values_accepts_bigint64_array() {
    assert!(eval_bool(
        "let v = new BigInt64Array(2); \
         crypto.getRandomValues(v); \
         v.some(b => b !== 0n);"
    ));
}

// ---------------------------------------------------------------------------
// `getRandomValues` — type rejection (TypeError, NOT
// TypeMismatchError DOMException per modern WebCrypto + WPT)
// ---------------------------------------------------------------------------

#[test]
fn get_random_values_rejects_float32_array() {
    let err = eval_err("crypto.getRandomValues(new Float32Array(4));");
    assert!(err.to_string().contains("Float"), "unexpected error: {err}");
    assert_typeerror(&err);
}

#[test]
fn get_random_values_rejects_float64_array() {
    let err = eval_err("crypto.getRandomValues(new Float64Array(4));");
    assert!(err.to_string().contains("Float"), "unexpected error: {err}");
    assert_typeerror(&err);
}

#[test]
fn get_random_values_rejects_data_view() {
    let err = eval_err("crypto.getRandomValues(new DataView(new ArrayBuffer(8)));");
    assert_typeerror(&err);
}

#[test]
fn get_random_values_rejects_plain_array() {
    let err = eval_err("crypto.getRandomValues([1, 2, 3]);");
    assert_typeerror(&err);
}

#[test]
fn get_random_values_rejects_undefined() {
    let err = eval_err("crypto.getRandomValues();");
    assert_typeerror(&err);
}

// ---------------------------------------------------------------------------
// `getRandomValues` — quota boundary (QuotaExceededError DOMException)
// ---------------------------------------------------------------------------

#[test]
fn get_random_values_allows_65536_byte_view() {
    // 65,536 byte boundary is INCLUSIVE — last allowed size.
    assert_eq!(
        eval_number(
            "let v = new Uint8Array(65536); \
             crypto.getRandomValues(v); \
             v.length;"
        ),
        65536.0
    );
}

#[test]
fn get_random_values_rejects_65537_byte_view_with_quota_exceeded() {
    assert_eq!(
        eval_string(
            "try { \
               crypto.getRandomValues(new Uint8Array(65537)); \
               'no-throw'; \
             } catch (e) { e.name; }"
        ),
        "QuotaExceededError"
    );
}

#[test]
fn get_random_values_quota_exceeded_error_has_code_22() {
    // DOMException.code for QuotaExceededError is 22 (legacy
    // numeric code per WebIDL §3.6.8 table).
    assert_eq!(
        eval_number(
            "try { \
               crypto.getRandomValues(new Uint8Array(65537)); \
               -1; \
             } catch (e) { e.code; }"
        ),
        22.0
    );
}

// ---------------------------------------------------------------------------
// `getRandomValues` — return identity + zero-length
// ---------------------------------------------------------------------------

#[test]
fn get_random_values_returns_same_view_receiver() {
    assert!(eval_bool(
        "let v = new Uint8Array(8); \
         crypto.getRandomValues(v) === v;"
    ));
}

#[test]
fn get_random_values_zero_length_returns_receiver_without_alloc() {
    // Zero-length short-circuit — view is returned unchanged.
    assert!(eval_bool(
        "let v = new Uint8Array(0); \
         crypto.getRandomValues(v) === v && v.length === 0;"
    ));
}

// ---------------------------------------------------------------------------
// `getRandomValues` — brand check
// ---------------------------------------------------------------------------

#[test]
fn get_random_values_brand_checks_receiver() {
    let err = eval_err("Crypto.prototype.getRandomValues.call({}, new Uint8Array(1));");
    assert!(
        err.to_string().contains("Illegal invocation"),
        "unexpected error: {err}"
    );
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn assert_typeerror(err: &VmError) {
    // VmError::type_error renders as `TypeError: ...`.
    assert!(
        err.to_string().starts_with("TypeError"),
        "expected TypeError, got: {err}"
    );
}
