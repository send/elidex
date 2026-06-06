//! The HMAC vertical (`generateKey` / `importKey` / `exportKey` /
//! `sign` / `verify`) and the Web IDL argument / sequence / dictionary
//! conversion conformance batches (`#11-crypto-subtle-full` PR-1, Codex
//! review batches 2–7).

use super::eval_global_string;

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
