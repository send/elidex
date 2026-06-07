//! The KDF derive vertical (`importKey` / `deriveBits` / `deriveKey`) for
//! HKDF / PBKDF2 (`#11-crypto-subtle-full` PR-3a).
//!
//! These exercise the JS surface end-to-end (Promise settle + the marshal →
//! `elidex-api-crypto` → ArrayBuffer / CryptoKey pipeline, including the
//! §14.3.7 two-algorithm `derivedKeyType` normalization); the KDF math
//! itself is RFC-KAT-validated in the crate's `tests/derive.rs`.

use super::eval_global_string;

/// JS helpers: hex-encode an ArrayBuffer + hex-decode to a Uint8Array.
const HEX: &str = "globalThis.hex = b => Array.from(new Uint8Array(b)) \
     .map(x => x.toString(16).padStart(2,'0')).join(''); \
     globalThis.fromHex = h => new Uint8Array(h.match(/../g).map(x => parseInt(x,16)));";

// ===========================================================================
// deriveBits — HKDF known-answer (RFC 5869 Appendix A case 1) through JS
// ===========================================================================

#[test]
fn hkdf_derive_bits_rfc5869_case1() {
    let src = format!(
        "{HEX} globalThis.r = 'pending'; \
         const ikm = new Uint8Array(22).fill(0x0b); \
         const salt = fromHex('000102030405060708090a0b0c'); \
         const info = fromHex('f0f1f2f3f4f5f6f7f8f9'); \
         crypto.subtle.importKey('raw', ikm, {{name:'HKDF'}}, false, ['deriveBits']) \
           .then(k => crypto.subtle.deriveBits({{name:'HKDF', hash:'SHA-256', salt, info}}, k, 336)) \
           .then(bits => {{ globalThis.r = hex(bits); }}, e => {{ globalThis.r = 'err:' + e.name; }});"
    );
    assert_eq!(
        eval_global_string(&src, "r"),
        "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865"
    );
}

// ===========================================================================
// deriveKey — PBKDF2 → AES-GCM (derive then encrypt/decrypt with the key)
// ===========================================================================

#[test]
fn derive_key_pbkdf2_to_aes_gcm_roundtrip() {
    let src = "globalThis.r = 'pending'; \
         const pw = new TextEncoder().encode('password'); \
         const salt = new TextEncoder().encode('salt'); \
         const iv = new Uint8Array(12).fill(7); \
         const msg = new TextEncoder().encode('derived secret'); \
         crypto.subtle.importKey('raw', pw, {name:'PBKDF2'}, false, ['deriveKey']) \
           .then(baseKey => crypto.subtle.deriveKey( \
              {name:'PBKDF2', salt, iterations:1000, hash:'SHA-256'}, baseKey, \
              {name:'AES-GCM', length:256}, false, ['encrypt','decrypt'])) \
           .then(aesKey => crypto.subtle.encrypt({name:'AES-GCM', iv}, aesKey, msg) \
              .then(ct => crypto.subtle.decrypt({name:'AES-GCM', iv}, aesKey, ct))) \
           .then(pt => { globalThis.r = new TextDecoder().decode(pt); }, \
                 e => { globalThis.r = 'err:' + e.name; });";
    assert_eq!(eval_global_string(src, "r"), "derived secret");
}

#[test]
fn derive_key_pbkdf2_to_aes_gcm_key_shape() {
    // The derived key's `[[algorithm]]` is the AES derivedKeyType (length
    // from get-key-length), with the deriveKey usages.
    let src = "globalThis.r = 'pending'; \
         const pw = new TextEncoder().encode('pw'); \
         const salt = new TextEncoder().encode('s'); \
         crypto.subtle.importKey('raw', pw, {name:'PBKDF2'}, false, ['deriveKey']) \
           .then(baseKey => crypto.subtle.deriveKey( \
              {name:'PBKDF2', salt, iterations:1, hash:'SHA-256'}, baseKey, \
              {name:'AES-CBC', length:128}, true, ['encrypt','decrypt'])) \
           .then(k => { globalThis.r = k.algorithm.name + '|' + k.algorithm.length + '|' \
                + k.type + '|' + k.extractable + '|' + k.usages.join(','); }, \
                 e => { globalThis.r = 'err:' + e.name; });";
    assert_eq!(
        eval_global_string(src, "r"),
        "AES-CBC|128|secret|true|encrypt,decrypt"
    );
}

// ===========================================================================
// deriveKey — HKDF → HMAC (derive then sign/verify with the key)
// ===========================================================================

#[test]
fn derive_key_hkdf_to_hmac_sign_verify() {
    let src = "globalThis.r = 'pending'; \
         const ikm = new Uint8Array(22).fill(0x0b); \
         const salt = new Uint8Array([1,2,3]); \
         const info = new Uint8Array([4,5,6]); \
         const msg = new TextEncoder().encode('mac me'); \
         crypto.subtle.importKey('raw', ikm, {name:'HKDF'}, false, ['deriveKey']) \
           .then(baseKey => crypto.subtle.deriveKey( \
              {name:'HKDF', hash:'SHA-256', salt, info}, baseKey, \
              {name:'HMAC', hash:'SHA-256'}, false, ['sign','verify'])) \
           .then(macKey => crypto.subtle.sign('HMAC', macKey, msg) \
              .then(sig => crypto.subtle.verify('HMAC', macKey, sig, msg))) \
           .then(ok => { globalThis.r = ok ? 'verified' : 'bad'; }, \
                 e => { globalThis.r = 'err:' + e.name; });";
    assert_eq!(eval_global_string(src, "r"), "verified");
}

#[test]
fn derive_key_hkdf_to_hmac_length_is_block_size() {
    // get-key-length(HMAC, no length) → hash block size (SHA-256 → 512).
    let src = "globalThis.r = 'pending'; \
         const ikm = new Uint8Array(8).fill(0x0b); \
         const salt = new Uint8Array([1]); const info = new Uint8Array([2]); \
         crypto.subtle.importKey('raw', ikm, {name:'HKDF'}, false, ['deriveKey']) \
           .then(baseKey => crypto.subtle.deriveKey( \
              {name:'HKDF', hash:'SHA-256', salt, info}, baseKey, \
              {name:'HMAC', hash:'SHA-256'}, true, ['sign'])) \
           .then(k => { globalThis.r = k.algorithm.name + '|' + k.algorithm.length \
                + '|' + k.algorithm.hash.name; }, \
                 e => { globalThis.r = 'err:' + e.name; });";
    assert_eq!(eval_global_string(src, "r"), "HMAC|512|SHA-256");
}

// ===========================================================================
// importKey constraints (§33.4.2 / §34.4.2)
// ===========================================================================

#[test]
fn kdf_import_key_algorithm_is_name_only() {
    // HKDF / PBKDF2 keys expose a name-only `[[algorithm]]` (no hash/length).
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.importKey('raw', new Uint8Array([1,2,3]), {name:'HKDF'}, false, ['deriveBits']) \
           .then(k => { globalThis.r = k.algorithm.name + '|' + (k.algorithm.hash===undefined) \
                + '|' + (k.algorithm.length===undefined) + '|' + k.type + '|' + k.extractable; }, \
                 e => { globalThis.r = 'err:' + e.name; });";
    assert_eq!(eval_global_string(src, "r"), "HKDF|true|true|secret|false");
}

#[test]
fn pbkdf2_import_extractable_true_rejects_syntax() {
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.importKey('raw', new Uint8Array([1,2,3]), {name:'PBKDF2'}, true, ['deriveBits']) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "SyntaxError");
}

#[test]
fn hkdf_import_non_derive_usage_rejects_syntax() {
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.importKey('raw', new Uint8Array([1]), {name:'HKDF'}, false, ['encrypt']) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "SyntaxError");
}

#[test]
fn hkdf_export_key_rejects_not_supported() {
    // Codex R2 F2 / §14.3.10: the export-support check (step 6) precedes the
    // extractable check (step 7), so a KDF key's exportKey is NotSupportedError
    // (NOT InvalidAccessError from the always-false extractable gate).
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.importKey('raw', new Uint8Array([1,2,3]), {name:'HKDF'}, false, ['deriveBits']) \
           .then(k => crypto.subtle.exportKey('raw', k)) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "NotSupportedError");
}

#[test]
fn hkdf_derivebits_member_getter_order() {
    // Codex R2 F1 / §18.4.4: step 6 converts the whole dictionary (top-level
    // getters in lexicographic order hash < info < salt), then step 10
    // normalizes the nested HashAlgorithmIdentifier (`hash.name`). So `hash.name`
    // must fire AFTER the info/salt getters, not immediately after `hash`.
    let src = "globalThis.order = []; globalThis.r = 'pending'; \
         const ikm = new Uint8Array(8).fill(0x0b); \
         const alg = { name:'HKDF', \
           get hash(){ globalThis.order.push('hash'); \
             return { get name(){ globalThis.order.push('hash.name'); return 'SHA-256'; } }; }, \
           get info(){ globalThis.order.push('info'); return new Uint8Array(1); }, \
           get salt(){ globalThis.order.push('salt'); return new Uint8Array(1); } }; \
         crypto.subtle.importKey('raw', ikm, {name:'HKDF'}, false, ['deriveBits']) \
           .then(k => crypto.subtle.deriveBits(alg, k, 256)) \
           .then(() => { globalThis.r = globalThis.order.join(','); }, \
                 e => { globalThis.r = 'err:' + e.name + '|' + globalThis.order.join(','); });";
    assert_eq!(eval_global_string(src, "r"), "hash,info,salt,hash.name");
}

#[test]
fn hkdf_import_jwk_format_rejects_not_supported() {
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.importKey('jwk', {kty:'oct', k:'AAAA'}, {name:'HKDF'}, false, ['deriveBits']) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "NotSupportedError");
}

// ===========================================================================
// deriveBits error paths (§14.3.8 + §33.4.1)
// ===========================================================================

#[test]
fn derive_bits_null_length_rejects_operation() {
    let src = "globalThis.r = 'pending'; \
         const salt = new Uint8Array([1]); const info = new Uint8Array([2]); \
         crypto.subtle.importKey('raw', new Uint8Array([9]), {name:'HKDF'}, false, ['deriveBits']) \
           .then(k => crypto.subtle.deriveBits({name:'HKDF', hash:'SHA-256', salt, info}, k)) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "OperationError");
}

#[test]
fn derive_bits_without_derive_bits_usage_rejects_invalid_access() {
    // §14.3.8 step 9: a key imported with only deriveKey usage.
    let src = "globalThis.r = 'pending'; \
         const salt = new Uint8Array([1]); const info = new Uint8Array([2]); \
         crypto.subtle.importKey('raw', new Uint8Array([9]), {name:'HKDF'}, false, ['deriveKey']) \
           .then(k => crypto.subtle.deriveBits({name:'HKDF', hash:'SHA-256', salt, info}, k, 256)) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "InvalidAccessError");
}

#[test]
fn pbkdf2_derive_bits_zero_iterations_rejects_operation() {
    let src = "globalThis.r = 'pending'; \
         const salt = new Uint8Array([1]); \
         crypto.subtle.importKey('raw', new Uint8Array([9]), {name:'PBKDF2'}, false, ['deriveBits']) \
           .then(k => crypto.subtle.deriveBits({name:'PBKDF2', salt, iterations:0, hash:'SHA-256'}, k, 128)) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "OperationError");
}

// ===========================================================================
// Web IDL argument conversion order + two-algorithm marshalling (§14.3.7)
// ===========================================================================

#[test]
fn derive_key_symbol_algorithm_rejects_type_error_first() {
    // arg conversion is left-to-right: a `Symbol()` algorithm (arg 1)
    // ToString-throws before the baseKey brand / derivedKeyType / usages.
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.deriveKey(Symbol(), {}, Symbol(), false, ['x']) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "TypeError");
}

#[test]
fn derive_key_symbol_derived_key_type_rejects_type_error() {
    // The second (derivedKeyType) algorithm conversion also ToString-throws
    // for a Symbol — exercising the §14.3.7 two-algorithm marshalling.
    let src = "globalThis.r = 'pending'; \
         const pw = new Uint8Array([1,2,3]); \
         crypto.subtle.importKey('raw', pw, {name:'PBKDF2'}, false, ['deriveKey']) \
           .then(k => crypto.subtle.deriveKey( \
              {name:'PBKDF2', salt:pw, iterations:1, hash:'SHA-256'}, k, Symbol(), false, ['encrypt'])) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "TypeError");
}

#[test]
fn derive_bits_non_crypto_key_rejects_type_error() {
    let src = "globalThis.r = 'pending'; \
         const salt = new Uint8Array([1]); const info = new Uint8Array([2]); \
         crypto.subtle.deriveBits({name:'HKDF', hash:'SHA-256', salt, info}, {}, 256) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "TypeError");
}

// ===========================================================================
// §18.4.4 recognition gate: an unsupported deriveBits algorithm name rejects
// with NotSupportedError WITHOUT firing a params-dictionary getter.
// ===========================================================================

#[test]
fn derive_bits_unsupported_name_skips_params_getter() {
    let src = "globalThis.fired = false; globalThis.r = 'pending'; \
         crypto.subtle.importKey('raw', new Uint8Array([1]), {name:'HKDF'}, false, ['deriveBits']) \
           .then(k => crypto.subtle.deriveBits( \
              {name:'BOGUS-KDF', get salt(){ globalThis.fired = true; throw new Error('x'); }}, k, 256)) \
           .then(() => { globalThis.r = 'resolved'; }, \
                 e => { globalThis.r = e.name + '|' + globalThis.fired; });";
    assert_eq!(eval_global_string(src, "r"), "NotSupportedError|false");
}
