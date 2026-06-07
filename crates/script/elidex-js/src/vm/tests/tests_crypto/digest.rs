//! `SubtleCrypto.digest` — algorithm normalization + known-answer
//! vectors, the BufferSource type matrix, rejection paths, and Promise
//! identity + the async receiver brand check (WebCrypto §14.3.5).

use super::{eval_bool, eval_global_bool, eval_global_number, eval_global_string};

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

#[test]
fn digest_data_snapshot_after_normalize() {
    // WebCrypto §14.3.5: step 2 normalizes the algorithm (firing its `name`
    // getter), and step 4 *then* gets a copy of the data bytes.  So an
    // algorithm getter that mutates the data buffer during normalization is
    // reflected in the digest.  Here the `name` getter zeroes the buffer, so
    // the digest must be of `[0,0,0,0]`, not the original `[1,2,3,4]`.
    let mutated = digest_hex(
        "{ get name() { globalThis.__b.fill(0); return 'SHA-256'; } }",
        "(globalThis.__b = new Uint8Array([1, 2, 3, 4]))",
    );
    let zeroed = digest_hex("'SHA-256'", "new Uint8Array([0, 0, 0, 0])");
    assert_eq!(mutated, zeroed);
}
