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
    // §18.4.4 ASCII-case-insensitive match — all three should
    // produce the same hex digest of empty input.
    let lower = digest_hex("'sha-256'", "new Uint8Array(0)");
    let mixed = digest_hex("'Sha-256'", "new Uint8Array(0)");
    let upper = digest_hex("'SHA-256'", "new Uint8Array(0)");
    assert_eq!(lower, upper);
    assert_eq!(mixed, upper);
}

#[test]
fn digest_accepts_dict_form_with_extra_keys_ignored() {
    // §18.4.4: extra dict keys are IGNORED (only `name` consulted
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
    // Spec §18.4.4: preserve original-case name.
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
fn digest_illegal_receiver_rejects_promise() {
    // `SubtleCrypto.prototype.digest.call({}, ...)` returns a Promise
    // REJECTED with the brand-check TypeError — WebCrypto §14.3 reports
    // every error asynchronously, including the Web IDL receiver check, so
    // a promise-returning operation must not throw synchronously.
    let src = "globalThis.r = 'pending'; \
         SubtleCrypto.prototype.digest.call({}, 'SHA-256', new Uint8Array(0)) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "TypeError");
}

#[test]
fn sign_illegal_receiver_rejects_promise() {
    // Same async-error contract for the HMAC operations:
    // `crypto.subtle.sign.call({}, …)` rejects rather than throwing.
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.sign.call({}, 'HMAC', {}, new Uint8Array(1)) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "TypeError");
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

// ---------------------------------------------------------------------------
// Web IDL conversion conformance (Codex review batch 2)
// ---------------------------------------------------------------------------

#[test]
fn generate_key_accepts_non_array_iterable_usages() {
    // HjRp7 / Web IDL §3.2.21: `sequence<KeyUsage>` is built from any
    // iterable, not just an Array.  A custom `[Symbol.iterator]` yielding
    // 'sign' must be accepted.
    let src = "globalThis.r = 'pending'; \
         const it = { [Symbol.iterator]() { let n = 0; return { next() { \
             return n++ === 0 ? {value:'sign', done:false} : {value:undefined, done:true}; \
         } }; } }; \
         crypto.subtle.generateKey({name:'HMAC', hash:'SHA-256'}, true, it) \
           .then(k => { globalThis.r = k.usages.join(','); }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "sign");
}

#[test]
fn import_jwk_null_key_data_rejects_data_error() {
    // HjRp9 / Web IDL: `(BufferSource or JsonWebKey)` from null converts to
    // an empty JsonWebKey dictionary, so the HMAC import rejects with
    // DataError (missing kty/k), not a TypeError.
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.importKey('jwk', null, {name:'HMAC', hash:'SHA-256'}, true, ['sign']) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "DataError");
}

#[test]
fn import_jwk_null_alg_member_coerces_to_string_null() {
    // HjRp8 / Web IDL DOMString: a present `alg:null` converts to "null"
    // (not dropped), so it mismatches the requested HS256 → DataError.
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.importKey('jwk', {kty:'oct', k:'CwsLCwsLCwsLCwsLCwsLCwsLCws', alg:null}, \
              {name:'HMAC', hash:'SHA-256'}, true, ['sign']) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "DataError");
}

#[test]
fn import_jwk_reads_all_declared_members_firing_getters() {
    // HjRp- / Web IDL: dictionary conversion reads every declared
    // JsonWebKey member, even ones HMAC ignores (e.g. `x`), firing each
    // getter — a throwing getter on an unused member rejects the import.
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.importKey('jwk', \
              {kty:'oct', k:'CwsLCwsLCwsLCwsLCwsLCwsLCws', get x(){ throw new Error('x read'); }}, \
              {name:'HMAC', hash:'SHA-256'}, true, ['sign']) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = 'err:' + e.message; });";
    assert_eq!(eval_global_string(src, "r"), "err:x read");
}

#[test]
fn sign_converts_key_argument_before_normalizing_algorithm() {
    // HjRp_ / Web IDL: the `key` (CryptoKey) argument is converted before
    // the sign operation normalizes the algorithm, so a non-CryptoKey
    // `key` rejects with TypeError, not NotSupportedError.
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.sign('NoSuchAlgo', {}, new Uint8Array(1)) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "TypeError");
}

#[test]
fn import_raw_empty_material_empty_usages_rejects_data_error() {
    // HjRqA / §31.6.4 + §14.3.9: invalid key material (empty → DataError)
    // is validated before the secret-key empty-usages SyntaxError (a later
    // generic step), so empty material + empty usages → DataError.
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.importKey('raw', new Uint8Array(0), {name:'HMAC', hash:'SHA-256'}, true, []) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "DataError");
}

// ---------------------------------------------------------------------------
// Web IDL conversion conformance (Codex review batch 3)
// ---------------------------------------------------------------------------

#[test]
fn generate_key_enforce_range_length_truncates_fraction() {
    // HjoAz / Web IDL §3.3.6 [EnforceRange]: a finite fractional `length`
    // truncates toward zero (31.9 → 31), it is NOT rejected; the resulting
    // key reports algorithm.length === 31.
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.generateKey({name:'HMAC', hash:'SHA-256', length: 31.9}, true, ['sign']) \
           .then(k => { globalThis.r = String(k.algorithm.length); }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "31");
}

#[test]
fn generate_key_enforce_range_rejects_below_lower_bound() {
    // [EnforceRange] step 3: IntegerPart(-8) = -8 < lowerBound 0 → TypeError
    // (a finite negative whose truncation falls below 0 is rejected, unlike
    // a fraction in (-1, 0] which truncates to 0).
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.generateKey({name:'HMAC', hash:'SHA-256', length: -8}, true, ['sign']) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "TypeError");
}

#[test]
fn import_jwk_empty_usages_bad_use_rejects_syntax_error() {
    // HjoA3 / §31.6.4 step 7: the JWK `use` check only fires when usages
    // is non-empty.  With empty usages, a present `use:'enc'` does NOT
    // pre-empt with DataError — the generic empty-secret-usages
    // SyntaxError (§14.3.9) is the correct rejection.
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.importKey('jwk', {kty:'oct', k:'CwsLCwsLCwsLCwsLCwsLCwsLCws', use:'enc'}, \
              {name:'HMAC', hash:'SHA-256'}, true, []) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "SyntaxError");
}

#[test]
fn import_jwk_non_sequence_oth_rejects_type_error() {
    // HjoA5 / Web IDL: the declared `sequence<RsaOtherPrimesInfo> oth`
    // member undergoes sequence conversion during dictionary conversion, so
    // a present non-iterable value (`oth:123`) rejects with a TypeError
    // before the HMAC import ignores RSA fields.
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.importKey('jwk', {kty:'oct', k:'CwsLCwsLCwsLCwsLCwsLCwsLCws', oth:123}, \
              {name:'HMAC', hash:'SHA-256'}, true, ['sign']) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "TypeError");
}

#[test]
fn import_raw_sub_byte_length_masks_then_signs_consistently() {
    // HjoA1 / §31.6.4 step 8: importing 4 raw bytes with length=25 keeps
    // the first 25 bits (top bit of the 4th octet), so exporting returns
    // the masked material.
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.importKey('raw', new Uint8Array([255,255,255,255]), \
              {name:'HMAC', hash:'SHA-256', length:25}, true, ['sign']) \
           .then(k => crypto.subtle.exportKey('raw', k)) \
           .then(buf => { globalThis.r = Array.from(new Uint8Array(buf)).join(','); }, \
                 e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "255,255,255,128");
}

// ---------------------------------------------------------------------------
// Web IDL conversion conformance (Codex review batch 4)
// ---------------------------------------------------------------------------

#[test]
fn digest_converts_data_before_normalizing_algorithm() {
    // HjuLU / Web IDL: the `data` (BufferSource) argument is converted
    // before the digest operation normalizes the algorithm, so an
    // unsupported algorithm + non-BufferSource `data` rejects with the data
    // TypeError, not NotSupportedError.
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.digest('NoSuchAlgo', 'not a buffer') \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "TypeError");
}

#[test]
fn generate_key_string_usages_rejects_type_error() {
    // HjuLW / Web IDL sequence conversion: a string primitive is not a
    // valid `sequence<KeyUsage>` source (Type(V) must be Object), so it is a
    // TypeError — NOT iterated into its characters.
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.generateKey({name:'HMAC', hash:'SHA-256'}, true, 'sign') \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "TypeError");
}

#[test]
fn import_jwk_string_key_ops_rejects_type_error() {
    // HjuLW: the JWK `key_ops` `sequence<DOMString>` member likewise
    // rejects a string primitive with a TypeError.
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.importKey('jwk', \
              {kty:'oct', k:'CwsLCwsLCwsLCwsLCwsLCwsLCws', key_ops:'sign'}, \
              {name:'HMAC', hash:'SHA-256'}, true, ['sign']) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "TypeError");
}

#[test]
fn import_jwk_oth_non_object_entry_rejects_type_error() {
    // HjuLV / Web IDL: each `oth` entry is converted to an
    // RsaOtherPrimesInfo dictionary, so a non-object entry (`oth:[123]`)
    // rejects with a TypeError during dictionary conversion.
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.importKey('jwk', \
              {kty:'oct', k:'CwsLCwsLCwsLCwsLCwsLCwsLCws', oth:[123]}, \
              {name:'HMAC', hash:'SHA-256'}, true, ['sign']) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "TypeError");
}

#[test]
fn import_jwk_oth_entry_getter_fires() {
    // HjuLV: a getter on an `oth` entry's RsaOtherPrimesInfo member fires
    // during dictionary conversion, so a throwing one rejects the import.
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.importKey('jwk', \
              {kty:'oct', k:'CwsLCwsLCwsLCwsLCwsLCwsLCws', \
               oth:[{ get r(){ throw new Error('oth r read'); } }]}, \
              {name:'HMAC', hash:'SHA-256'}, true, ['sign']) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = 'err:' + e.message; });";
    assert_eq!(eval_global_string(src, "r"), "err:oth r read");
}

#[test]
fn generate_key_name_getter_fires_twice_during_normalization() {
    // HjuLY / §18.4.4 step 6: converting to the params dictionary re-reads
    // the inherited `name` member, so a getter that throws on its second
    // access rejects the operation.
    let src = "globalThis.r = 'pending'; let n = 0; \
         crypto.subtle.generateKey( \
              {get name(){ if (++n === 2) throw new Error('second name read'); return 'HMAC'; }, \
               hash:'SHA-256'}, true, ['sign']) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = 'err:' + e.message; });";
    assert_eq!(eval_global_string(src, "r"), "err:second name read");
}

// ---------------------------------------------------------------------------
// Web IDL / algorithm conformance (Codex review batch 5)
// ---------------------------------------------------------------------------

#[test]
fn import_jwk_key_ops_allows_extension_values() {
    // Hlnbe / RFC 7517 §4.3 + §31.6.4 step 8: `key_ops` may carry
    // extension operations beyond WebCrypto's usages; as long as it is a
    // valid JWK array (no duplicates) containing every requested usage,
    // unknown entries are ignored, not rejected.
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.importKey('jwk', \
              {kty:'oct', k:'CwsLCwsLCwsLCwsLCwsLCwsLCws', key_ops:['sign','custom-op']}, \
              {name:'HMAC', hash:'SHA-256'}, true, ['sign']) \
           .then(k => { globalThis.r = k.usages.join(','); }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "sign");
}

#[test]
fn import_jwk_key_ops_duplicate_rejects_data_error() {
    // Hlnbe: duplicate key operation values are still invalid per RFC 7517
    // §4.3 → DataError.
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.importKey('jwk', \
              {kty:'oct', k:'CwsLCwsLCwsLCwsLCwsLCwsLCws', key_ops:['sign','sign']}, \
              {name:'HMAC', hash:'SHA-256'}, true, ['sign']) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "DataError");
}

#[test]
fn import_jwk_key_ops_missing_requested_usage_rejects_data_error() {
    // Hlnbe: §31.6.4 step 8 still requires key_ops to contain every
    // requested usage — `['verify']` lacks the requested `sign`.
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.importKey('jwk', \
              {kty:'oct', k:'CwsLCwsLCwsLCwsLCwsLCwsLCws', key_ops:['verify']}, \
              {name:'HMAC', hash:'SHA-256'}, true, ['sign']) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "DataError");
}

#[test]
fn generate_key_invalid_usage_beats_zero_length_error() {
    // Hlnbh / §31.6.3 step 1: a non-sign/verify usage is a SyntaxError
    // before the step-2 length handling, so `length:0` + `['encrypt']`
    // rejects with SyntaxError, not the OperationError of a zero length.
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.generateKey({name:'HMAC', hash:'SHA-256', length:0}, true, ['encrypt']) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "SyntaxError");
}

// ---------------------------------------------------------------------------
// WebIDL sequence + arg-conversion conformance (Codex review batch 6)
// ---------------------------------------------------------------------------

#[test]
fn generate_key_runaway_usages_iterator_is_capped() {
    // Hl17H: a custom `keyUsages` iterable whose `.next()` never reports
    // `done` must NOT hang the Promise — the shared sequence converter caps
    // it and rejects with a TypeError.
    let src = "globalThis.r = 'pending'; \
         const it = { [Symbol.iterator]() { return { next() { \
             return {value:'sign', done:false}; } }; } }; \
         crypto.subtle.generateKey({name:'HMAC', hash:'SHA-256'}, true, it) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "TypeError");
}

#[test]
fn generate_key_usages_iterator_close_error_takes_precedence() {
    // Hl17E / ECMA-262 §7.4.11: when an element fails conversion AND the
    // iterator's `.return()` throws, the `.return()` error wins over the
    // element error.
    let src = "globalThis.r = 'pending'; \
         const it = { [Symbol.iterator]() { return { \
             next() { return {value:'not-a-usage', done:false}; }, \
             return() { throw new Error('return threw'); } }; } }; \
         crypto.subtle.generateKey({name:'HMAC', hash:'SHA-256'}, true, it) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = 'err:' + e.message; });";
    assert_eq!(eval_global_string(src, "r"), "err:return threw");
}

#[test]
fn digest_converts_symbol_algorithm_before_data() {
    // Hl17G / Web IDL: the `algorithm` `(object or DOMString)` conversion
    // (arg 1) runs before the `data` (arg 2) conversion, so a `Symbol()`
    // algorithm rejects with the Symbol-to-string TypeError (its message
    // mentions Symbol), not the `data` TypeError.
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.digest(Symbol(), 123) \
           .then(() => { globalThis.r = 'resolved'; }, e => { \
             globalThis.r = (e instanceof TypeError && /[Ss]ymbol/.test(e.message)) \
                 ? 'symbol-type-error' : ('other:' + e.message); });";
    assert_eq!(eval_global_string(src, "r"), "symbol-type-error");
}

// ---------------------------------------------------------------------------
// WebIDL dictionary member-order conformance (Codex review batch 7)
// ---------------------------------------------------------------------------

#[test]
fn generate_key_missing_hash_rejects_before_reading_length() {
    // Hl-BT / Web IDL: `hash` is a required member read (lexicographically)
    // before the optional `length`, so an omitted `hash` rejects with the
    // missing-required-member TypeError without firing a throwing `length`
    // getter.
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.generateKey( \
              {name:'HMAC', get length(){ throw new Error('length read'); }}, true, ['sign']) \
           .then(() => { globalThis.r = 'resolved'; }, e => { \
             globalThis.r = (e instanceof TypeError && !/length read/.test(e.message)) \
                 ? 'hash-required' : ('other:' + e.message); });";
    assert_eq!(eval_global_string(src, "r"), "hash-required");
}

#[test]
fn export_jwk_emits_members_in_lexicographic_order() {
    // Hl-BU / Web IDL "convert dictionary to ES value": the exported
    // `oct` JWK's own keys are created in lexicographic member order.
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.generateKey({name:'HMAC', hash:'SHA-256'}, true, ['sign','verify']) \
           .then(k => crypto.subtle.exportKey('jwk', k)) \
           .then(jwk => { globalThis.r = Object.keys(jwk).join(','); }, e => { globalThis.r = e.name; });";
    // Present members for an extractable HMAC oct export: alg, ext, k,
    // key_ops, kty (no `use`).
    assert_eq!(eval_global_string(src, "r"), "alg,ext,k,key_ops,kty");
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
fn import_cyclic_algorithm_object_does_not_recurse() {
    // C1 regression: a self-referential `hash` member must NOT recurse
    // (the nested `hash` is marshalled as a name-only leaf). It rejects
    // (`hash` "HMAC" is not a recognized digest), it does not crash.
    let src = "globalThis.r = 'pending'; \
         const a = {name:'HMAC'}; a.hash = a; \
         crypto.subtle.importKey('raw', new Uint8Array(20), a, true, ['sign']) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "NotSupportedError");
}

#[test]
fn sign_does_not_read_hash_member_getter() {
    // C6 regression: sign's algorithm is name-only (the spec never reads
    // `hash`/`length` for sign), so a throwing `hash` getter must NOT fire.
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.importKey('raw', new Uint8Array(20), {name:'HMAC', hash:'SHA-256'}, true, ['sign']) \
           .then(k => crypto.subtle.sign( \
              {name:'HMAC', get hash(){ throw new Error('should not read'); }}, \
              k, new Uint8Array(1))) \
           .then(() => { globalThis.r = 'signed'; }, e => { globalThis.r = 'err:' + e.message; });";
    assert_eq!(eval_global_string(src, "r"), "signed");
}

#[test]
fn generate_key_unsupported_name_does_not_read_hash_getter() {
    // §18.4.4 step 5/6 ordering: an unregistered `(generateKey, name)`
    // pair is rejected as NotSupportedError at step 5 — *before* step 6's
    // params-dictionary conversion reads `hash` — so a throwing `hash`
    // getter on an unsupported algorithm must NOT fire.
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.generateKey( \
            {name:'AES-Magic', get hash(){ throw new Error('should not read'); }}, \
            true, ['sign']) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "NotSupportedError");
}

#[test]
fn crypto_key_algorithm_and_usages_are_cached_objects() {
    // §13.4: the `algorithm` / `usages` getters return the *cached*
    // ECMAScript object (`[[algorithm_cached]]` / `[[usages_cached]]`), so
    // identity is stable across reads — `key.algorithm === key.algorithm`
    // and `key.usages === key.usages` are both `true`.
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.generateKey({name:'HMAC', hash:'SHA-256'}, true, ['sign']) \
           .then(k => { globalThis.r = [k.algorithm === k.algorithm, \
                        k.usages === k.usages, \
                        k.algorithm.hash === k.algorithm.hash].join(','); });";
    assert_eq!(eval_global_string(src, "r"), "true,true,true");
}

#[test]
fn crypto_key_cached_algorithm_mutation_persists() {
    // A consequence of caching (§13.4): because the same object is
    // returned each read, a property written onto `key.algorithm` is
    // observable on the next read (it is not rebuilt fresh).
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.generateKey({name:'HMAC', hash:'SHA-256'}, true, ['sign']) \
           .then(k => { k.algorithm.marker = 42; globalThis.r = String(k.algorithm.marker); });";
    assert_eq!(eval_global_string(src, "r"), "42");
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

#[test]
fn crypto_key_cached_algorithm_survives_gc_via_trace_arm() {
    // The cached `[[algorithm_cached]]` object (§13.4) is reachable ONLY
    // through `crypto_key_js_cache` after the callback returns — no JS var
    // holds it.  A GC with the key still rooted must keep it alive via the
    // `ObjectKind::CryptoKey` trace arm; otherwise the tagged property
    // would be lost (the getter would rebuild a fresh object).
    let mut vm = Vm::new();
    vm.eval(
        "crypto.subtle.generateKey({name:'HMAC', hash:'SHA-256'}, true, ['sign']) \
           .then(key => { globalThis.k = key; key.algorithm.marker = 7; });",
    )
    .unwrap();
    vm.inner.collect_garbage();
    let r = vm.eval("String(globalThis.k.algorithm.marker)").unwrap();
    match r {
        JsValue::String(id) => assert_eq!(vm.get_string(id), "7", "cached object survived GC"),
        other => panic!("expected string, got {other:?}"),
    }
}

#[test]
fn crypto_key_js_cache_pruned_on_gc() {
    // The `algorithm` / `usages` cache (`crypto_key_js_cache`) is pruned
    // alongside `crypto_key_states` when the key is collected — `ObjectId`
    // slots are reused, so a stale cache entry would alias another
    // wrapper's accessors.  Root the key directly via a global (not via a
    // settled `generateKey` Promise, whose `[[PromiseResult]]` would keep
    // the key reachable past the global drop).
    use elidex_api_crypto::key::{CryptoKeyData, KeyAlgorithm, KeyMaterial, KeyType, KeyUsage};
    use elidex_api_crypto::HashAlgorithm;

    let mut vm = Vm::new();
    // First eval registers the globals so `crypto_key_prototype` is set.
    vm.eval("void crypto.subtle;").unwrap();
    let id = vm.inner.alloc_crypto_key(CryptoKeyData {
        key_type: KeyType::Secret,
        extractable: true,
        algorithm: KeyAlgorithm::Hmac {
            hash: HashAlgorithm::Sha256,
            length: 160,
        },
        usages: vec![KeyUsage::Sign],
        material: KeyMaterial::Raw(vec![0xab; 20]),
    });
    let k = vm.inner.strings.intern("k");
    vm.inner.globals.insert(k, JsValue::Object(id));

    // Read both accessors (via JS, so the real getter populates the cache).
    vm.eval("void globalThis.k.algorithm; void globalThis.k.usages;")
        .unwrap();
    assert!(
        vm.inner.crypto_key_js_cache.contains_key(&id),
        "both accessors populated the cache"
    );

    // Rooted → cache entry survives.
    vm.inner.collect_garbage();
    assert!(
        vm.inner.crypto_key_js_cache.contains_key(&id),
        "cache survives while key rooted"
    );

    // Drop the root → cache + key state both pruned.
    vm.inner.globals.insert(k, JsValue::Undefined);
    vm.inner.collect_garbage();
    assert!(
        !vm.inner.crypto_key_js_cache.contains_key(&id),
        "cache pruned with collected key"
    );
    assert!(
        !vm.inner.crypto_key_states.contains_key(&id),
        "key state pruned with collected key"
    );
}

#[test]
fn crypto_key_accessor_with_missing_side_store_entry_is_illegal_invocation() {
    // Copilot #1 regression: a `CryptoKey` brand surviving WITHOUT its
    // side-store entry (e.g. a reference retained across `Vm::unbind`,
    // which clears the side-store) must surface as a TypeError, not a
    // panic / stale-material read.
    use elidex_api_crypto::key::{CryptoKeyData, KeyAlgorithm, KeyMaterial, KeyType, KeyUsage};
    use elidex_api_crypto::HashAlgorithm;

    let mut vm = Vm::new();
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
    let key = vm.inner.strings.intern("k");
    vm.inner.globals.insert(key, JsValue::Object(id));
    // Simulate the invariant violation (entry gone, wrapper retained).
    vm.inner.crypto_key_states.remove(&id);

    let r = vm
        .eval("(() => { try { globalThis.k.type; return 'no-throw'; } catch (e) { return e.name; } })();")
        .unwrap();
    match r {
        JsValue::String(sid) => assert_eq!(vm.get_string(sid), "TypeError"),
        other => panic!("expected TypeError name, got {other:?}"),
    }
}
