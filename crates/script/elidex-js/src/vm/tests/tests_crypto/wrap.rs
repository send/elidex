//! The wrap vertical (`wrapKey` / `unwrapKey`) for AES-KW + the AES-GCM /
//! CBC / CTR encrypt/decrypt fallback (`#11-crypto-subtle-full` PR-3b).
//!
//! These exercise the JS surface end-to-end (Promise settle + the marshal →
//! `elidex-api-crypto` → ArrayBuffer / CryptoKey pipeline, the §14.3.11 /
//! §14.3.12 wrap→encrypt / unwrap→decrypt normalize fallback, and the `jwk`
//! `JSON.stringify` / "parse a JWK" round-trip).  The RFC 3394 math itself is
//! KAT-validated in the crate's `tests/aes_kw.rs`.

use super::eval_global_string;

// ===========================================================================
// AES-KW raw wrap/unwrap round-trip (RFC 3394 through the JS surface)
// ===========================================================================

#[test]
fn wrap_unwrap_aes_kw_raw_roundtrip() {
    // Wrap a raw AES-GCM key under an AES-KW KEK, unwrap it, and confirm the
    // exported raw bytes are byte-identical (the KEK need not be extractable).
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.importKey('raw', new Uint8Array(16).fill(0x11), {name:'AES-KW'}, \
              false, ['wrapKey','unwrapKey']) \
           .then(kek => crypto.subtle.importKey('raw', new Uint8Array(16).fill(0x22), \
                 {name:'AES-GCM'}, true, ['encrypt','decrypt']) \
             .then(key => crypto.subtle.wrapKey('raw', key, kek, {name:'AES-KW'}) \
               .then(w => crypto.subtle.unwrapKey('raw', w, kek, {name:'AES-KW'}, \
                     {name:'AES-GCM'}, true, ['encrypt','decrypt'])) \
               .then(uk => crypto.subtle.exportKey('raw', uk)))) \
           .then(raw => { const a = new Uint8Array(raw); \
                 globalThis.r = a.length + ':' + a.every(b => b === 0x22); }, \
                 e => { globalThis.r = 'err:' + e.name; });";
    // RFC 3394 unwrap recovers the original 16-byte key material.
    assert_eq!(eval_global_string(src, "r"), "16:true");
}

/// A tampered AES-KW wrapped key fails the RFC 3394 integrity check on unwrap →
/// OperationError (WebCrypto §30.3.2 step 2).
#[test]
fn unwrap_aes_kw_tampered_is_operation_error() {
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.importKey('raw', new Uint8Array(16).fill(0x11), {name:'AES-KW'}, \
              false, ['wrapKey','unwrapKey']) \
           .then(kek => crypto.subtle.importKey('raw', new Uint8Array(16).fill(0x22), \
                 {name:'AES-GCM'}, true, ['encrypt','decrypt']) \
             .then(key => crypto.subtle.wrapKey('raw', key, kek, {name:'AES-KW'}) \
               .then(w => { const t = new Uint8Array(w); t[0] = t[0] ^ 0x01; \
                 return crypto.subtle.unwrapKey('raw', t, kek, {name:'AES-KW'}, \
                     {name:'AES-GCM'}, true, ['encrypt','decrypt']); }))) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "OperationError");
}

// ===========================================================================
// AES-GCM encrypt/decrypt fallback (§14.3.11 step 15 / §14.3.12 step 14)
// ===========================================================================

/// An AES-GCM key has no wrap key operation, so wrapKey/unwrapKey fall back to
/// its encrypt/decrypt operation.  Raw round-trip: the unwrapped key decrypts a
/// message the original key encrypted (same material).
#[test]
fn wrap_unwrap_aes_gcm_fallback_raw_roundtrip() {
    let src = "globalThis.r = 'pending'; \
         const iv = new Uint8Array(12).fill(7); \
         const msg = new TextEncoder().encode('wrapped secret'); \
         crypto.subtle.importKey('raw', new Uint8Array(32).fill(0x55), {name:'AES-GCM'}, \
              false, ['wrapKey','unwrapKey']) \
           .then(kek => crypto.subtle.importKey('raw', new Uint8Array(16).fill(0x22), \
                 {name:'AES-GCM'}, true, ['encrypt','decrypt']) \
             .then(key => crypto.subtle.encrypt({name:'AES-GCM', iv}, key, msg) \
               .then(ct => crypto.subtle.wrapKey('raw', key, kek, {name:'AES-GCM', iv}) \
                 .then(w => crypto.subtle.unwrapKey('raw', w, kek, {name:'AES-GCM', iv}, \
                       {name:'AES-GCM'}, true, ['encrypt','decrypt'])) \
                 .then(uk => crypto.subtle.decrypt({name:'AES-GCM', iv}, uk, ct))))) \
           .then(pt => { globalThis.r = new TextDecoder().decode(pt); }, \
                 e => { globalThis.r = 'err:' + e.name; });";
    assert_eq!(eval_global_string(src, "r"), "wrapped secret");
}

/// The `jwk` wrap/unwrap path through the AES-GCM fallback exercises the
/// `JSON.stringify` (§14.3.11 step 14) / "parse a JWK" (§14.3.12 step 15)
/// round-trip.  The unwrapped key self-encrypts/decrypts a message, proving the
/// JWK material survived the JSON round-trip. (AES-KW is unsuitable for the
/// `jwk` round-trip here: the serialized JSON is rarely a multiple of 64 bits,
/// which §30.3.1 step 1 rejects — the AES-GCM fallback handles any length.)
#[test]
fn wrap_unwrap_aes_gcm_jwk_json_roundtrip() {
    let src = "globalThis.r = 'pending'; \
         const iv = new Uint8Array(12).fill(9); \
         const msg = new TextEncoder().encode('jwk roundtrip'); \
         crypto.subtle.importKey('raw', new Uint8Array(32).fill(0x55), {name:'AES-GCM'}, \
              false, ['wrapKey','unwrapKey']) \
           .then(kek => crypto.subtle.importKey('raw', new Uint8Array(32).fill(0x33), \
                 {name:'AES-GCM'}, true, ['encrypt','decrypt']) \
             .then(key => crypto.subtle.wrapKey('jwk', key, kek, {name:'AES-GCM', iv}) \
               .then(w => crypto.subtle.unwrapKey('jwk', w, kek, {name:'AES-GCM', iv}, \
                     {name:'AES-GCM'}, true, ['encrypt','decrypt'])) \
               .then(uk => crypto.subtle.encrypt({name:'AES-GCM', iv}, uk, msg) \
                 .then(ct => crypto.subtle.decrypt({name:'AES-GCM', iv}, uk, ct))))) \
           .then(pt => { globalThis.r = new TextDecoder().decode(pt); }, \
                 e => { globalThis.r = 'err:' + e.name; });";
    assert_eq!(eval_global_string(src, "r"), "jwk roundtrip");
}

/// AES-KW can wrap a `jwk` key: the JWK JSON is padded to a multiple of 64 bits
/// (§14.3.11 step-14 Note), so the round-trip succeeds and the unwrapped key
/// self-encrypts/decrypts a message (Codex R-batch regression — AES-KW jwk was
/// previously rejected for the non-multiple-of-64-bits payload).
#[test]
fn wrap_unwrap_aes_kw_jwk_roundtrip() {
    let src = "globalThis.r = 'pending'; \
         const iv = new Uint8Array(16).fill(4); \
         const msg = new TextEncoder().encode('aes-kw jwk'); \
         crypto.subtle.importKey('raw', new Uint8Array(32).fill(0x55), {name:'AES-KW'}, \
              false, ['wrapKey','unwrapKey']) \
           .then(kek => crypto.subtle.importKey('raw', new Uint8Array(16).fill(0x22), \
                 {name:'AES-CBC'}, true, ['encrypt','decrypt']) \
             .then(key => crypto.subtle.wrapKey('jwk', key, kek, {name:'AES-KW'}) \
               .then(w => crypto.subtle.unwrapKey('jwk', w, kek, {name:'AES-KW'}, \
                     {name:'AES-CBC'}, true, ['encrypt','decrypt'])) \
               .then(uk => crypto.subtle.encrypt({name:'AES-CBC', iv}, uk, msg) \
                 .then(ct => crypto.subtle.decrypt({name:'AES-CBC', iv}, uk, ct))))) \
           .then(pt => { globalThis.r = new TextDecoder().decode(pt); }, \
                 e => { globalThis.r = 'err:' + e.name; });";
    assert_eq!(eval_global_string(src, "r"), "aes-kw jwk");
}

/// The unwrapped key's `[[algorithm]]` / `[[usages]]` / `[[extractable]]` come
/// from the unwrapKey arguments (the unwrappedKeyAlgorithm + extractable +
/// keyUsages), independent of the wrapped JWK's own `ext` / `key_ops`.
#[test]
fn unwrap_jwk_key_shape_from_arguments() {
    let src = "globalThis.r = 'pending'; \
         const iv = new Uint8Array(12).fill(9); \
         crypto.subtle.importKey('raw', new Uint8Array(32).fill(0x55), {name:'AES-GCM'}, \
              false, ['wrapKey','unwrapKey']) \
           .then(kek => crypto.subtle.importKey('raw', new Uint8Array(16).fill(0x33), \
                 {name:'AES-GCM'}, true, ['encrypt','decrypt']) \
             .then(key => crypto.subtle.wrapKey('jwk', key, kek, {name:'AES-GCM', iv}) \
               .then(w => crypto.subtle.unwrapKey('jwk', w, kek, {name:'AES-GCM', iv}, \
                     {name:'AES-GCM'}, false, ['decrypt'])))) \
           .then(uk => { globalThis.r = uk.algorithm.name + '|' + uk.algorithm.length \
                 + '|' + uk.extractable + '|' + uk.usages.join(','); }, \
                 e => { globalThis.r = 'err:' + e.name; });";
    assert_eq!(eval_global_string(src, "r"), "AES-GCM|128|false|decrypt");
}

// ===========================================================================
// Error paths (WebCrypto §14.3.11 steps 9-12)
// ===========================================================================

/// §14.3.11 step 12: a non-extractable key cannot be wrapped (wrap effectively
/// exports it) → InvalidAccessError.
#[test]
fn wrap_non_extractable_key_is_invalid_access() {
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.importKey('raw', new Uint8Array(16).fill(0x11), {name:'AES-KW'}, \
              false, ['wrapKey','unwrapKey']) \
           .then(kek => crypto.subtle.importKey('raw', new Uint8Array(16).fill(0x22), \
                 {name:'AES-GCM'}, false, ['encrypt','decrypt']) \
             .then(key => crypto.subtle.wrapKey('raw', key, kek, {name:'AES-KW'}))) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "InvalidAccessError");
}

/// §14.3.11 step 9: the wrap algorithm name must equal the wrapping key's
/// algorithm name → InvalidAccessError (AES-KW algorithm, AES-GCM wrappingKey).
#[test]
fn wrap_name_mismatch_is_invalid_access() {
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.importKey('raw', new Uint8Array(16).fill(0x11), {name:'AES-GCM'}, \
              false, ['wrapKey','unwrapKey']) \
           .then(kek => crypto.subtle.importKey('raw', new Uint8Array(16).fill(0x22), \
                 {name:'AES-GCM'}, true, ['encrypt','decrypt']) \
             .then(key => crypto.subtle.wrapKey('raw', key, kek, {name:'AES-KW'}))) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "InvalidAccessError");
}

/// §14.3.11 step 10: a wrapping key whose usages omit `wrapKey` →
/// InvalidAccessError.
#[test]
fn wrap_missing_wrap_usage_is_invalid_access() {
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.importKey('raw', new Uint8Array(16).fill(0x11), {name:'AES-KW'}, \
              false, ['unwrapKey']) \
           .then(kek => crypto.subtle.importKey('raw', new Uint8Array(16).fill(0x22), \
                 {name:'AES-GCM'}, true, ['encrypt','decrypt']) \
             .then(key => crypto.subtle.wrapKey('raw', key, kek, {name:'AES-KW'}))) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "InvalidAccessError");
}

/// AES-KW import rejects an `encrypt` usage (§30.3.4 step 1: wrap-only) →
/// SyntaxError, distinguishing it from the block-cipher modes.
#[test]
fn import_aes_kw_encrypt_usage_is_syntax_error() {
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.importKey('raw', new Uint8Array(16).fill(0x11), {name:'AES-KW'}, \
              false, ['encrypt']) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "SyntaxError");
}

/// A non-CryptoKey `key` / `wrappingKey` argument is a WebIDL conversion
/// TypeError (settled on the Promise, WebCrypto §14.3 async contract).
#[test]
fn wrap_non_cryptokey_argument_is_type_error() {
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.importKey('raw', new Uint8Array(16).fill(0x11), {name:'AES-KW'}, \
              false, ['wrapKey','unwrapKey']) \
           .then(kek => crypto.subtle.wrapKey('raw', {}, kek, {name:'AES-KW'})) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "TypeError");
}

// ===========================================================================
// Realm isolation (WebCrypto §14.3.11 step 14 / §9 parse-a-JWK "new global
// object") — regression for the Codex R1 findings (#314).
//
// The fix makes the `jwk` wrap/unwrap JSON round-trip run entirely in the
// engine-independent crate over the `JsonWebKey` struct — it never builds or
// reads a JS object in the page realm — so a page-mutated `Object.prototype`
// (`toJSON`, or a throwing inherited getter for an absent member) can neither
// hijack a wrap nor spuriously reject an unwrap.  That isolation is enforced
// *structurally* (the path has no JS-object step) and verified at the crate
// level: `elidex-api-crypto::jwk::{to_json_bytes,from_json_bytes}` round-trip
// + `ops_wrap_unwrap_jwk_roundtrip_via_gcm` in `tests/aes_kw.rs`.  A direct
// VM-level prototype-pollution test is not expressible here — elidex does not
// currently surface `Object.prototype` as a mutable property (a separate
// core-engine gap; until it lands the attack is unreachable, but the spec
// mandates the isolation unconditionally, so the crate owns it).
// ===========================================================================
