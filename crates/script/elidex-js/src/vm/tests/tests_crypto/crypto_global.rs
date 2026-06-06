//! `globalThis.crypto` + `Crypto` constructor stub, `getRandomValues`
//! (type acceptance / rejection / quota boundary / return identity /
//! brand check), `randomUUID`, and the `crypto.subtle` `[SameObject]`
//! accessor (WebCrypto §10 / §11).

use super::{assert_typeerror, eval_bool, eval_err, eval_number, eval_string};

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
