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

/// Run `source` and return the value of `globalThis.<name>` after
/// the eval has drained its microtask queue (so `.then`-installed
/// values are visible).  Mirrors `tests_body_mixin::eval_global_*`.
fn eval_global_string(source: &str, name: &str) -> String {
    let mut vm = Vm::new();
    vm.eval(source).unwrap();
    match vm.get_global(name) {
        Some(JsValue::String(id)) => vm.get_string(id),
        other => panic!("expected global {name} to be a string, got {other:?}"),
    }
}

fn eval_global_number(source: &str, name: &str) -> f64 {
    let mut vm = Vm::new();
    vm.eval(source).unwrap();
    match vm.get_global(name) {
        Some(JsValue::Number(n)) => n,
        other => panic!("expected global {name} to be a number, got {other:?}"),
    }
}

fn eval_global_bool(source: &str, name: &str) -> bool {
    let mut vm = Vm::new();
    vm.eval(source).unwrap();
    match vm.get_global(name) {
        Some(JsValue::Boolean(b)) => b,
        other => panic!("expected global {name} to be a bool, got {other:?}"),
    }
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

// Acceptance tests: pre-fill with a sentinel, call
// `getRandomValues`, then assert that AT LEAST ONE element
// changed (catches the constant-output / silently-no-op wiring
// bug).  Buffers are sized at 32 bytes / 32 elements so the
// false-negative probability — a CSPRNG happening to write back
// exactly the sentinel byte at every position — is 1/256^32 ≈
// 1/2^256, well below CPU bit-flip rates.  The sentinel `0xFF`
// (NOT `0`) ensures the assertion catches a "zero-fill" wiring
// bug that an `b !== 0` check on a `0`-initialised buffer would
// silently pass.
#[test]
fn get_random_values_accepts_uint8_array() {
    assert!(eval_bool(
        "let v = new Uint8Array(32); v.fill(0xFF); \
         crypto.getRandomValues(v); \
         v.some(b => b !== 0xFF);"
    ));
}

#[test]
fn get_random_values_accepts_uint8_clamped_array() {
    assert!(eval_bool(
        "let v = new Uint8ClampedArray(32); v.fill(0xFF); \
         crypto.getRandomValues(v); \
         v.some(b => b !== 0xFF);"
    ));
}

#[test]
fn get_random_values_accepts_int32_array() {
    assert!(eval_bool(
        "let v = new Int32Array(8); v.fill(-1); \
         crypto.getRandomValues(v); \
         v.some(b => b !== -1);"
    ));
}

#[test]
fn get_random_values_accepts_bigint64_array() {
    assert!(eval_bool(
        "let v = new BigInt64Array(4); v.fill(-1n); \
         crypto.getRandomValues(v); \
         v.some(b => b !== -1n);"
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
// `randomUUID` (WebCrypto §11.5)
// ---------------------------------------------------------------------------

#[test]
fn random_uuid_returns_string() {
    assert_eq!(eval_string("typeof crypto.randomUUID();"), "string");
}

#[test]
fn random_uuid_matches_v4_format() {
    // Spec §11.5: xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx
    // where x is hex and y in {8, 9, a, b} (RFC 4122 variant bits).
    assert!(eval_bool(
        "const u = crypto.randomUUID(); \
         /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/.test(u);"
    ));
}

#[test]
fn random_uuid_length_is_36() {
    assert_eq!(eval_number("crypto.randomUUID().length;"), 36.0);
}

#[test]
fn random_uuid_produces_non_degenerate_output() {
    // Constant / wiring-bug check: ALL outputs equal across 8
    // calls would indicate a constant-RNG bug (e.g. uuid::Uuid::nil,
    // or the v4 feature silently disabled).  The assertion is
    // "some call differs from the first", which is deterministically
    // true for any non-degenerate CSPRNG (false-negative
    // probability ~ 1/2^122 per pair — sub-cosmic-bit-flip).
    // Cheaper + clearer than the prior 100-iteration unique-set
    // sort, and addresses Copilot R3's probabilistic-assertion
    // concern by reframing as a degeneracy test.
    assert!(eval_bool(
        "const a = []; \
         for (let i = 0; i < 8; i++) a.push(crypto.randomUUID()); \
         a.some(u => u !== a[0]);"
    ));
}

#[test]
fn random_uuid_brand_checks_receiver() {
    let err = eval_err("Crypto.prototype.randomUUID.call({});");
    assert!(
        err.to_string().contains("Illegal invocation"),
        "unexpected error: {err}"
    );
}

// ---------------------------------------------------------------------------
// `crypto.subtle` accessor (WebCrypto §10, [SameObject])
// ---------------------------------------------------------------------------

#[test]
fn subtle_returns_subtle_crypto_instance() {
    assert!(eval_bool("crypto.subtle instanceof SubtleCrypto;"));
}

#[test]
fn subtle_is_same_object_across_reads() {
    // [SameObject] per WebIDL §10 — `crypto.subtle === crypto.subtle`.
    assert!(eval_bool("crypto.subtle === crypto.subtle;"));
}

#[test]
fn subtle_accessor_descriptor_is_accessor_not_data() {
    // §10 `readonly attribute SubtleCrypto subtle` — descriptor
    // must carry `get` (NOT `value`).
    assert!(eval_bool(
        "let d = Object.getOwnPropertyDescriptor(Crypto.prototype, 'subtle'); \
         typeof d.get === 'function' && !('value' in d);"
    ));
}

#[test]
fn subtle_crypto_constructor_throws_illegal_constructor() {
    let err = eval_err("new SubtleCrypto();");
    assert!(
        err.to_string().contains("Illegal constructor"),
        "unexpected error: {err}"
    );
}

// ---------------------------------------------------------------------------
// `SubtleCrypto.digest` — algorithm normalization + known-answer
// ---------------------------------------------------------------------------

/// Drive `crypto.subtle.digest(<algo_js>, <data_js>)` and return
/// the hex-encoded digest bytes.  Both arguments are interpolated
/// verbatim into the JS source so callers can pass either a quoted
/// string literal (`"'SHA-1'"`) or an object literal
/// (`"{name: 'SHA-256'}"`) for the algorithm.
fn digest_hex(algo_js: &str, data_js: &str) -> String {
    eval_global_string(
        &format!(
            "globalThis.r = ''; \
             crypto.subtle.digest({algo_js}, {data_js}) \
               .then(buf => {{ \
                 let v = new Uint8Array(buf); \
                 globalThis.r = Array.from(v) \
                   .map(b => b.toString(16).padStart(2, '0')).join(''); \
               }});"
        ),
        "r",
    )
}

#[test]
fn digest_sha1_known_answer_for_abc() {
    // RFC 3174 "abc" — SHA-1: a9993e364706816aba3e25717850c26c9cd0d89d
    assert_eq!(
        digest_hex("'SHA-1'", "new TextEncoder().encode('abc')"),
        "a9993e364706816aba3e25717850c26c9cd0d89d"
    );
}

#[test]
fn digest_sha256_known_answer_for_empty_string() {
    // RFC 6234 empty-input — SHA-256:
    // e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
    assert_eq!(
        digest_hex("'SHA-256'", "new Uint8Array(0)"),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn digest_sha384_returns_48_bytes() {
    assert_eq!(
        eval_global_number(
            "globalThis.r = 0; \
             crypto.subtle.digest('SHA-384', new Uint8Array(0)) \
               .then(buf => { globalThis.r = buf.byteLength; });",
            "r"
        ),
        48.0
    );
}

#[test]
fn digest_sha512_returns_64_bytes() {
    assert_eq!(
        eval_global_number(
            "globalThis.r = 0; \
             crypto.subtle.digest('SHA-512', new Uint8Array(0)) \
               .then(buf => { globalThis.r = buf.byteLength; });",
            "r"
        ),
        64.0
    );
}

#[test]
fn digest_accepts_mixed_case_algo_name() {
    // §18.2.1 ASCII-case-insensitive match — all three should
    // produce the same hex digest of empty input.
    let lower = digest_hex("'sha-256'", "new Uint8Array(0)");
    let mixed = digest_hex("'Sha-256'", "new Uint8Array(0)");
    let upper = digest_hex("'SHA-256'", "new Uint8Array(0)");
    assert_eq!(lower, upper);
    assert_eq!(mixed, upper);
}

#[test]
fn digest_accepts_dict_form_with_extra_keys_ignored() {
    // §18.2.1: extra dict keys are IGNORED (only `name` consulted
    // for `digest`).
    let plain = digest_hex("{name: 'SHA-256'}", "new Uint8Array(0)");
    let with_extra = digest_hex("{name: 'SHA-256', hash: 'ignored'}", "new Uint8Array(0)");
    assert_eq!(plain, with_extra);
}

// ---------------------------------------------------------------------------
// `SubtleCrypto.digest` — BufferSource type matrix
// ---------------------------------------------------------------------------

#[test]
fn digest_accepts_array_buffer() {
    assert_eq!(
        eval_global_number(
            "globalThis.r = 0; \
             crypto.subtle.digest('SHA-256', new ArrayBuffer(0)) \
               .then(buf => { globalThis.r = buf.byteLength; });",
            "r"
        ),
        32.0
    );
}

#[test]
fn digest_accepts_data_view() {
    assert_eq!(
        eval_global_number(
            "globalThis.r = 0; \
             crypto.subtle.digest('SHA-256', new DataView(new ArrayBuffer(4))) \
               .then(buf => { globalThis.r = buf.byteLength; });",
            "r"
        ),
        32.0
    );
}

#[test]
fn digest_accepts_int32_array() {
    assert_eq!(
        eval_global_number(
            "globalThis.r = 0; \
             crypto.subtle.digest('SHA-256', new Int32Array(2)) \
               .then(buf => { globalThis.r = buf.byteLength; });",
            "r"
        ),
        32.0
    );
}

// ---------------------------------------------------------------------------
// `SubtleCrypto.digest` — rejection paths
// ---------------------------------------------------------------------------

#[test]
fn digest_rejects_unknown_algorithm_with_not_supported_error() {
    // Unknown algo → Promise rejected with NotSupportedError.
    assert_eq!(
        eval_global_string(
            "globalThis.r = ''; \
             crypto.subtle.digest('SHA-9001', new Uint8Array(0)) \
               .catch(e => { globalThis.r = e.name; });",
            "r"
        ),
        "NotSupportedError"
    );
}

#[test]
fn digest_unknown_algorithm_preserves_user_supplied_name_in_message() {
    // Spec §18.2.1 step 9: preserve original-case name.
    assert!(eval_global_bool(
        "globalThis.r = false; \
         crypto.subtle.digest('NoSuchAlg', new Uint8Array(0)) \
           .catch(e => { globalThis.r = e.message.indexOf('NoSuchAlg') >= 0; });",
        "r"
    ));
}

#[test]
fn digest_truncates_long_unknown_algorithm_name_in_message() {
    // Security boundary: attacker-supplied algorithm name is
    // truncated at MAX_ECHOED_ALGO_NAME_LEN (64 bytes) when echoed
    // into the NotSupportedError message — bounds per-call DOMException
    // allocation against `'A'.repeat(N)` abuse.  The 1000-char name
    // here should NOT appear in full in the error.
    assert!(eval_global_bool(
        "globalThis.r = false; \
         let huge = 'A'.repeat(1000); \
         crypto.subtle.digest(huge, new Uint8Array(0)) \
           .catch(e => { globalThis.r = e.message.length < 200; });",
        "r"
    ));
}

#[test]
fn digest_rejects_non_buffer_source_data_with_type_error() {
    assert_eq!(
        eval_global_string(
            "globalThis.r = ''; \
             crypto.subtle.digest('SHA-256', 'not a buffer') \
               .catch(e => { globalThis.r = e.name; });",
            "r"
        ),
        "TypeError"
    );
}

#[test]
fn digest_rejects_missing_data_arg_with_type_error() {
    // WebCrypto §14.3.5 IDL signature is `digest(algorithm, data:
    // BufferSource)` — `data` is REQUIRED, so `digest('SHA-256')`
    // must throw TypeError per WebIDL §3.10.18 rather than silently
    // hashing empty input.  Copilot R2 finding.
    assert_eq!(
        eval_global_string(
            "globalThis.r = ''; \
             crypto.subtle.digest('SHA-256') \
               .catch(e => { globalThis.r = e.name; });",
            "r"
        ),
        "TypeError"
    );
}

#[test]
fn digest_rejects_explicit_undefined_data_with_type_error() {
    // Passing `undefined` explicitly is observably the same as
    // omitting the argument — both fail the required-BufferSource
    // conversion per WebIDL.  Locks the strict path against future
    // regression to "undefined → empty buffer".
    assert_eq!(
        eval_global_string(
            "globalThis.r = ''; \
             crypto.subtle.digest('SHA-256', undefined) \
               .catch(e => { globalThis.r = e.name; });",
            "r"
        ),
        "TypeError"
    );
}

#[test]
fn digest_dict_form_missing_name_rejects_with_type_error() {
    // WebCrypto §10.1 `dictionary Algorithm { required DOMString
    // name; }` — when the dict form omits `name`, the conversion
    // should throw TypeError, NOT ToString-coerce `undefined` to
    // the string `"undefined"` and reject with NotSupportedError.
    // Copilot R2 finding.
    assert_eq!(
        eval_global_string(
            "globalThis.r = ''; \
             crypto.subtle.digest({hash: 'SHA-256'}, new Uint8Array(0)) \
               .catch(e => { globalThis.r = e.name; });",
            "r"
        ),
        "TypeError"
    );
}

#[test]
fn digest_dict_form_explicit_undefined_name_rejects_with_type_error() {
    // Symmetry with the missing-property case: `{name: undefined}`
    // must also TypeError, not surface as `"undefined"` in a
    // NotSupportedError.
    assert_eq!(
        eval_global_string(
            "globalThis.r = ''; \
             crypto.subtle.digest({name: undefined}, new Uint8Array(0)) \
               .catch(e => { globalThis.r = e.name; });",
            "r"
        ),
        "TypeError"
    );
}

// ---------------------------------------------------------------------------
// `SubtleCrypto.digest` — Promise identity + brand check
// ---------------------------------------------------------------------------

#[test]
fn digest_returns_promise() {
    assert!(eval_bool(
        "crypto.subtle.digest('SHA-256', new Uint8Array(0)) instanceof Promise;"
    ));
}

#[test]
fn digest_returns_distinct_promises_per_call() {
    assert!(eval_bool(
        "crypto.subtle.digest('SHA-256', new Uint8Array(0)) !== \
         crypto.subtle.digest('SHA-256', new Uint8Array(0));"
    ));
}

#[test]
fn digest_resolves_with_array_buffer_constructor() {
    assert!(eval_global_bool(
        "globalThis.r = false; \
         crypto.subtle.digest('SHA-256', new Uint8Array(0)) \
           .then(b => { globalThis.r = (b.constructor === ArrayBuffer); });",
        "r"
    ));
}

#[test]
fn digest_brand_checks_receiver() {
    // `SubtleCrypto.prototype.digest.call({}, ...)` returns a
    // SYNCHRONOUSLY-thrown TypeError because the brand check runs
    // before the Promise is even allocated (matches Chrome).
    let err = eval_err("SubtleCrypto.prototype.digest.call({}, 'SHA-256', new Uint8Array(0));");
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

// ===========================================================================
// HMAC vertical: generateKey / importKey / exportKey / sign / verify
// (`#11-crypto-subtle-full` PR-1)
// ===========================================================================

/// JS helper installed at the top of each operation test: hex-encode an
/// ArrayBuffer.
const HEX_FN: &str = "globalThis.hex = b => Array.from(new Uint8Array(b)) \
     .map(x => x.toString(16).padStart(2,'0')).join('');";

#[test]
fn generate_sign_verify_roundtrip_true() {
    let src = format!(
        "{HEX_FN} globalThis.r = 'pending'; \
         const data = new Uint8Array([1,2,3,4]); \
         crypto.subtle.generateKey({{name:'HMAC', hash:'SHA-256'}}, true, ['sign','verify']) \
           .then(key => crypto.subtle.sign('HMAC', key, data) \
             .then(sig => crypto.subtle.verify('HMAC', key, sig, data))) \
           .then(ok => {{ globalThis.r = ok ? 'true' : 'false'; }}, \
                 e => {{ globalThis.r = 'err:' + e.name; }});"
    );
    assert_eq!(eval_global_string(&src, "r"), "true");
}

#[test]
fn verify_rejects_tampered_signature() {
    let src = "globalThis.r = 'pending'; \
         const data = new Uint8Array([1,2,3,4]); \
         crypto.subtle.generateKey({name:'HMAC', hash:'SHA-256'}, true, ['sign','verify']) \
           .then(key => crypto.subtle.sign('HMAC', key, data) \
             .then(sig => { const u = new Uint8Array(sig); u[0] = 255 - u[0]; \
                             return crypto.subtle.verify('HMAC', key, sig, data); })) \
           .then(ok => { globalThis.r = ok ? 'true' : 'false'; });";
    assert_eq!(eval_global_string(src, "r"), "false");
}

#[test]
fn import_raw_sign_matches_rfc4231_vector() {
    // RFC 4231 TC1: key = 0x0b×20, data = "Hi There", HMAC-SHA-256.
    let src = format!(
        "{HEX_FN} globalThis.r = 'pending'; \
         const key = new Uint8Array(20).fill(0x0b); \
         const data = new TextEncoder().encode('Hi There'); \
         crypto.subtle.importKey('raw', key, {{name:'HMAC', hash:'SHA-256'}}, false, ['sign']) \
           .then(k => crypto.subtle.sign('HMAC', k, data)) \
           .then(sig => {{ globalThis.r = hex(sig); }}, e => {{ globalThis.r = 'err:' + e.name; }});"
    );
    assert_eq!(
        eval_global_string(&src, "r"),
        "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
    );
}

#[test]
fn import_jwk_export_jwk_roundtrip() {
    let src = "globalThis.r = 'pending'; \
         const jwk = {kty:'oct', k:'CwsLCwsLCwsLCwsLCwsLCwsLCws', alg:'HS256', \
                      key_ops:['sign','verify'], ext:true}; \
         crypto.subtle.importKey('jwk', jwk, {name:'HMAC', hash:'SHA-256'}, true, ['sign','verify']) \
           .then(k => crypto.subtle.exportKey('jwk', k)) \
           .then(out => { globalThis.r = out.kty + '|' + out.k + '|' + out.alg + '|' + out.ext; }, \
                 e => { globalThis.r = 'err:' + e.name; });";
    assert_eq!(
        eval_global_string(src, "r"),
        "oct|CwsLCwsLCwsLCwsLCwsLCwsLCws|HS256|true"
    );
}

#[test]
fn export_raw_returns_key_bytes() {
    let src = format!(
        "{HEX_FN} globalThis.r = 'pending'; \
         const key = new Uint8Array(4).fill(0xab); \
         crypto.subtle.importKey('raw', key, {{name:'HMAC', hash:'SHA-256'}}, true, ['sign']) \
           .then(k => crypto.subtle.exportKey('raw', k)) \
           .then(out => {{ globalThis.r = hex(out); }});"
    );
    assert_eq!(eval_global_string(&src, "r"), "abababab");
}

#[test]
fn export_non_extractable_rejects_invalid_access() {
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.importKey('raw', new Uint8Array(20), {name:'HMAC', hash:'SHA-256'}, false, ['sign']) \
           .then(k => crypto.subtle.exportKey('raw', k)) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "InvalidAccessError");
}

#[test]
fn generate_empty_usages_rejects_syntax_error() {
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.generateKey({name:'HMAC', hash:'SHA-256'}, true, []) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "SyntaxError");
}

#[test]
fn import_unsupported_format_rejects_not_supported() {
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.importKey('pkcs8', new Uint8Array(20), {name:'HMAC', hash:'SHA-256'}, true, ['sign']) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "NotSupportedError");
}

#[test]
fn sign_without_sign_usage_rejects_invalid_access() {
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.importKey('raw', new Uint8Array(20), {name:'HMAC', hash:'SHA-256'}, true, ['verify']) \
           .then(k => crypto.subtle.sign('HMAC', k, new Uint8Array(1))) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "InvalidAccessError");
}

#[test]
fn import_jwk_bad_kty_rejects_data_error() {
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.importKey('jwk', {kty:'RSA', k:'CwsL'}, {name:'HMAC', hash:'SHA-256'}, true, ['sign']) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "DataError");
}

#[test]
fn import_missing_hash_rejects_type_error() {
    // HmacImportParams.hash is a required member → TypeError at normalize.
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.importKey('raw', new Uint8Array(20), {name:'HMAC'}, true, ['sign']) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "TypeError");
}

#[test]
fn unrecognized_algorithm_rejects_not_supported() {
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.generateKey({name:'AES-Magic', hash:'SHA-256'}, true, ['sign']) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "NotSupportedError");
}

#[test]
fn crypto_key_accessors() {
    // type / extractable / algorithm.name / algorithm.hash.name / usages.
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.generateKey({name:'HMAC', hash:'SHA-384'}, true, ['sign','verify']) \
           .then(k => { globalThis.r = [k.type, k.extractable, k.algorithm.name, \
                        k.algorithm.hash.name, k.usages.join(','), \
                        k.algorithm.length].join('|'); });";
    assert_eq!(
        eval_global_string(src, "r"),
        // SHA-384 HMAC default length = block size 1024 bits.
        "secret|true|HMAC|SHA-384|sign,verify|1024"
    );
}

#[test]
fn crypto_key_constructor_is_illegal() {
    let err = eval_err("new CryptoKey();");
    assert_typeerror(&err);
}

#[test]
fn crypto_key_usages_is_fresh_array_each_read() {
    // Not [SameObject]: two reads yield distinct array objects.
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.generateKey({name:'HMAC', hash:'SHA-256'}, true, ['sign']) \
           .then(k => { globalThis.r = (k.usages === k.usages) ? 'same' : 'fresh'; });";
    assert_eq!(eval_global_string(src, "r"), "fresh");
}

#[test]
fn crypto_key_states_pruned_on_gc() {
    // I1 correctness invariant: a CryptoKey unreachable from any root is
    // pruned from `crypto_key_states` on collection (ObjectId slots are
    // reused, so a stale entry would bind another wrapper's material).
    use elidex_api_crypto::key::{CryptoKeyData, KeyAlgorithm, KeyMaterial, KeyType, KeyUsage};
    use elidex_api_crypto::HashAlgorithm;

    let mut vm = Vm::new();
    // Trigger global registration so `crypto_key_prototype` is set.
    vm.eval("void crypto.subtle;").unwrap();

    let data = CryptoKeyData {
        key_type: KeyType::Secret,
        extractable: true,
        algorithm: KeyAlgorithm::Hmac {
            hash: HashAlgorithm::Sha256,
            length: 160,
        },
        usages: vec![KeyUsage::Sign],
        material: KeyMaterial::Raw(vec![0xab; 20]),
    };
    let id = vm.inner.alloc_crypto_key(data);
    assert_eq!(vm.inner.crypto_key_states.len(), 1);

    // Root it via a global; GC keeps it.
    let key = vm.inner.strings.intern("rootedKey");
    vm.inner.globals.insert(key, JsValue::Object(id));
    vm.inner.collect_garbage();
    assert_eq!(
        vm.inner.crypto_key_states.len(),
        1,
        "rooted key survives GC"
    );

    // Drop the only root; GC prunes the side-store entry.
    vm.inner.globals.insert(key, JsValue::Undefined);
    vm.inner.collect_garbage();
    assert_eq!(
        vm.inner.crypto_key_states.len(),
        0,
        "unreachable key pruned from side-store"
    );
}
