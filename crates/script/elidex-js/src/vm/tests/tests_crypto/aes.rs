//! The AES vertical (`generateKey` / `importKey` / `exportKey` /
//! `encrypt` / `decrypt`) for AES-GCM / AES-CBC / AES-CTR
//! (`#11-crypto-subtle-full` PR-2).
//!
//! These exercise the JS surface end-to-end (Promise settle + the marshal
//! → `elidex-api-crypto` → ArrayBuffer pipeline); the cipher math itself is
//! KAT-validated in the crate's `tests.rs`.

use super::eval_global_string;

/// JS helpers installed at the top of each test: hex-encode an ArrayBuffer
/// + hex-decode to a Uint8Array.
const HEX: &str = "globalThis.hex = b => Array.from(new Uint8Array(b)) \
     .map(x => x.toString(16).padStart(2,'0')).join(''); \
     globalThis.fromHex = h => new Uint8Array(h.match(/../g).map(x => parseInt(x,16)));";

// ===========================================================================
// Round-trips (generate → encrypt → decrypt) for each mode
// ===========================================================================

#[test]
fn gcm_generate_encrypt_decrypt_roundtrip() {
    let src = "globalThis.r = 'pending'; \
         const iv = new Uint8Array(12).fill(7); \
         const msg = new TextEncoder().encode('secret message'); \
         crypto.subtle.generateKey({name:'AES-GCM', length:256}, true, ['encrypt','decrypt']) \
           .then(k => crypto.subtle.encrypt({name:'AES-GCM', iv}, k, msg) \
             .then(ct => crypto.subtle.decrypt({name:'AES-GCM', iv}, k, ct))) \
           .then(pt => { globalThis.r = new TextDecoder().decode(pt); }, \
                 e => { globalThis.r = 'err:' + e.name; });";
    assert_eq!(eval_global_string(src, "r"), "secret message");
}

#[test]
fn cbc_generate_encrypt_decrypt_roundtrip() {
    let src = "globalThis.r = 'pending'; \
         const iv = new Uint8Array(16).fill(3); \
         const msg = new TextEncoder().encode('cbc round trip'); \
         crypto.subtle.generateKey({name:'AES-CBC', length:128}, true, ['encrypt','decrypt']) \
           .then(k => crypto.subtle.encrypt({name:'AES-CBC', iv}, k, msg) \
             .then(ct => crypto.subtle.decrypt({name:'AES-CBC', iv}, k, ct))) \
           .then(pt => { globalThis.r = new TextDecoder().decode(pt); }, \
                 e => { globalThis.r = 'err:' + e.name; });";
    assert_eq!(eval_global_string(src, "r"), "cbc round trip");
}

#[test]
fn ctr_generate_encrypt_decrypt_roundtrip() {
    let src = "globalThis.r = 'pending'; \
         const counter = new Uint8Array(16); \
         const msg = new TextEncoder().encode('ctr round trip'); \
         crypto.subtle.generateKey({name:'AES-CTR', length:192}, true, ['encrypt','decrypt']) \
           .then(k => crypto.subtle.encrypt({name:'AES-CTR', counter, length:64}, k, msg) \
             .then(ct => crypto.subtle.decrypt({name:'AES-CTR', counter, length:64}, k, ct))) \
           .then(pt => { globalThis.r = new TextDecoder().decode(pt); }, \
                 e => { globalThis.r = 'err:' + e.name; });";
    assert_eq!(eval_global_string(src, "r"), "ctr round trip");
}

// ===========================================================================
// Known-answer vector through the JS surface (GCM Test Case 3)
// ===========================================================================

#[test]
fn gcm_import_raw_matches_nist_tc3() {
    let src = format!(
        "{HEX} globalThis.r = 'pending'; \
         const key = fromHex('feffe9928665731c6d6a8f9467308308'); \
         const iv = fromHex('cafebabefacedbaddecaf888'); \
         const pt = fromHex('d9313225f88406e5a55909c5aff5269a86a7a9531534f7da2e4c303d8a318a721c3c0c95956809532fcf0e2449a6b525b16aedf5aa0de657ba637b391aafd255'); \
         crypto.subtle.importKey('raw', key, {{name:'AES-GCM'}}, false, ['encrypt']) \
           .then(k => crypto.subtle.encrypt({{name:'AES-GCM', iv}}, k, pt)) \
           .then(ct => {{ globalThis.r = hex(ct); }}, e => {{ globalThis.r = 'err:' + e.name; }});"
    );
    assert_eq!(
        eval_global_string(&src, "r"),
        "42831ec2217774244b7221b784d0d49ce3aa212f2c02a4e035c17e2329aca12e\
         21d514b25466931c7d8f6a5aac84aa051ba30b396a0aac973d58e091473f5985\
         4d5c2af327cd64a62cf35abd2ba6fab4"
    );
}

// ===========================================================================
// JWK import / export
// ===========================================================================

#[test]
fn import_raw_export_jwk_emits_aes_alg() {
    let src = "globalThis.r = 'pending'; \
         const key = new Uint8Array(32).fill(9); \
         crypto.subtle.importKey('raw', key, {name:'AES-GCM'}, true, ['encrypt','decrypt']) \
           .then(k => crypto.subtle.exportKey('jwk', k)) \
           .then(j => { globalThis.r = j.kty + '|' + j.alg + '|' + j.ext + '|' + (j.k.length > 0); }, \
                 e => { globalThis.r = 'err:' + e.name; });";
    assert_eq!(eval_global_string(src, "r"), "oct|A256GCM|true|true");
}

#[test]
fn import_jwk_then_encrypt_decrypt_roundtrip() {
    let src = "globalThis.r = 'pending'; \
         const jwk = {kty:'oct', k:'AAAAAAAAAAAAAAAAAAAAAA', alg:'A128CBC', \
                      key_ops:['encrypt','decrypt'], ext:true}; \
         const iv = new Uint8Array(16).fill(1); \
         const msg = new TextEncoder().encode('jwk cbc'); \
         crypto.subtle.importKey('jwk', jwk, {name:'AES-CBC'}, true, ['encrypt','decrypt']) \
           .then(k => crypto.subtle.encrypt({name:'AES-CBC', iv}, k, msg) \
             .then(ct => crypto.subtle.decrypt({name:'AES-CBC', iv}, k, ct))) \
           .then(pt => { globalThis.r = new TextDecoder().decode(pt); }, \
                 e => { globalThis.r = 'err:' + e.name; });";
    assert_eq!(eval_global_string(src, "r"), "jwk cbc");
}

// ===========================================================================
// `key.algorithm` shape (AES has no `hash` member)
// ===========================================================================

#[test]
fn aes_key_algorithm_shape_has_no_hash() {
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.generateKey({name:'AES-GCM', length:256}, true, ['encrypt']) \
           .then(k => { const a = k.algorithm; \
                        globalThis.r = a.name + '|' + a.length + '|' + ('hash' in a); }, \
                 e => { globalThis.r = 'err:' + e.name; });";
    assert_eq!(eval_global_string(src, "r"), "AES-GCM|256|false");
}

// ===========================================================================
// Validation / error paths
// ===========================================================================

#[test]
fn generate_invalid_length_rejects_operation_error() {
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.generateKey({name:'AES-GCM', length:200}, true, ['encrypt']) \
           .then(_ => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "OperationError");
}

#[test]
fn generate_invalid_usage_rejects_syntax_error() {
    // `sign` is not a valid AES usage.
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.generateKey({name:'AES-GCM', length:128}, true, ['sign']) \
           .then(_ => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "SyntaxError");
}

#[test]
fn encrypt_without_encrypt_usage_rejects_invalid_access() {
    let src = "globalThis.r = 'pending'; \
         const iv = new Uint8Array(12); \
         crypto.subtle.generateKey({name:'AES-GCM', length:128}, true, ['decrypt']) \
           .then(k => crypto.subtle.encrypt({name:'AES-GCM', iv}, k, new Uint8Array(1))) \
           .then(_ => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "InvalidAccessError");
}

#[test]
fn encrypt_mode_mismatch_rejects_invalid_access() {
    // An AES-GCM key used with AES-CBC params.
    let src = "globalThis.r = 'pending'; \
         const iv = new Uint8Array(16); \
         crypto.subtle.generateKey({name:'AES-GCM', length:128}, true, ['encrypt']) \
           .then(k => crypto.subtle.encrypt({name:'AES-CBC', iv}, k, new Uint8Array(1))) \
           .then(_ => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "InvalidAccessError");
}

#[test]
fn decrypt_tampered_ciphertext_rejects_operation_error() {
    let src = "globalThis.r = 'pending'; \
         const iv = new Uint8Array(12).fill(5); \
         const msg = new TextEncoder().encode('authentic'); \
         crypto.subtle.generateKey({name:'AES-GCM', length:128}, true, ['encrypt','decrypt']) \
           .then(k => crypto.subtle.encrypt({name:'AES-GCM', iv}, k, msg) \
             .then(ct => { const u = new Uint8Array(ct); u[0] = u[0] ^ 1; \
                            return crypto.subtle.decrypt({name:'AES-GCM', iv}, k, ct); })) \
           .then(_ => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "OperationError");
}

#[test]
fn gcm_invalid_tag_length_rejects_operation_error() {
    let src = "globalThis.r = 'pending'; \
         const iv = new Uint8Array(12); \
         crypto.subtle.generateKey({name:'AES-GCM', length:128}, true, ['encrypt']) \
           .then(k => crypto.subtle.encrypt({name:'AES-GCM', iv, tagLength:48}, k, new Uint8Array(1))) \
           .then(_ => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "OperationError");
}

#[test]
fn ctr_invalid_length_rejects_operation_error() {
    let src = "globalThis.r = 'pending'; \
         const counter = new Uint8Array(16); \
         crypto.subtle.generateKey({name:'AES-CTR', length:128}, true, ['encrypt']) \
           .then(k => crypto.subtle.encrypt({name:'AES-CTR', counter, length:200}, k, new Uint8Array(1))) \
           .then(_ => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "OperationError");
}

#[test]
fn cbc_bad_iv_length_rejects_operation_error() {
    let src = "globalThis.r = 'pending'; \
         const iv = new Uint8Array(12); \
         crypto.subtle.generateKey({name:'AES-CBC', length:128}, true, ['encrypt']) \
           .then(k => crypto.subtle.encrypt({name:'AES-CBC', iv}, k, new Uint8Array(1))) \
           .then(_ => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "OperationError");
}

#[test]
fn import_raw_bad_length_rejects_data_error() {
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.importKey('raw', new Uint8Array(20), {name:'AES-GCM'}, true, ['encrypt']) \
           .then(_ => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "DataError");
}

// ===========================================================================
// Web IDL argument conversion order + the §18.4.4 recognition gate
// ===========================================================================

#[test]
fn symbol_algorithm_rejects_type_error_before_key() {
    // The `(object or DOMString)` algorithm conversion runs first, so a
    // Symbol throws a TypeError even though `key` is also invalid.
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.generateKey({name:'AES-GCM', length:128}, true, ['encrypt']) \
           .then(k => crypto.subtle.encrypt(Symbol(), k, new Uint8Array(1))) \
           .then(_ => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "TypeError");
}

#[test]
fn non_cryptokey_key_rejects_type_error() {
    let src = "globalThis.r = 'pending'; \
         const iv = new Uint8Array(12); \
         crypto.subtle.encrypt({name:'AES-GCM', iv}, {}, new Uint8Array(1)) \
           .then(_ => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "TypeError");
}

#[test]
fn unsupported_name_does_not_fire_params_getter() {
    // §18.4.4 step 5: an unregistered `(op, name)` rejects with
    // NotSupportedError *before* step 6 converts the params dictionary, so
    // the `iv` getter must never fire.
    let src = "globalThis.fired = false; globalThis.r = 'pending'; \
         crypto.subtle.generateKey({name:'AES-GCM', length:128}, true, ['encrypt']) \
           .then(k => crypto.subtle.encrypt( \
                  {name:'AES-Magic', get iv() { globalThis.fired = true; return new Uint8Array(12); }}, \
                  k, new Uint8Array(1))) \
           .then(_ => { globalThis.r = 'resolved'; }, \
                 e => { globalThis.r = e.name + '|' + globalThis.fired; });";
    assert_eq!(eval_global_string(src, "r"), "NotSupportedError|false");
}

#[test]
fn missing_iv_rejects_type_error() {
    // `iv` is a required AesGcmParams member.
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.generateKey({name:'AES-GCM', length:128}, true, ['encrypt']) \
           .then(k => crypto.subtle.encrypt({name:'AES-GCM'}, k, new Uint8Array(1))) \
           .then(_ => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "TypeError");
}

// ===========================================================================
// AES-GCM additionalData + non-96-bit IV + non-default tagLength marshalling
// (host member-read coverage beyond the crate-only KATs)
// ===========================================================================

#[test]
fn gcm_additional_data_long_iv_truncated_tag_roundtrip() {
    // End-to-end exercise of the host marshalling of `additionalData` + a
    // non-96-bit IV (16 bytes) + a non-default `tagLength` (96); the crate
    // KATs cover the algorithm, this covers the VM member reads + plumbing.
    let src = "globalThis.r = 'pending'; \
         const iv = new Uint8Array(16).fill(2); \
         const aad = new TextEncoder().encode('header'); \
         const msg = new TextEncoder().encode('aead payload'); \
         crypto.subtle.generateKey({name:'AES-GCM', length:256}, true, ['encrypt','decrypt']) \
           .then(k => crypto.subtle.encrypt({name:'AES-GCM', iv, additionalData: aad, tagLength: 96}, k, msg) \
             .then(ct => { globalThis.tagBits = (ct.byteLength - msg.byteLength) * 8; \
                            return crypto.subtle.decrypt({name:'AES-GCM', iv, additionalData: aad, tagLength: 96}, k, ct); })) \
           .then(pt => { globalThis.r = new TextDecoder().decode(pt) + '|tag' + globalThis.tagBits; }, \
                 e => { globalThis.r = 'err:' + e.name; });";
    // The 96-bit tag means ciphertext = plaintext_len + 12 bytes.
    assert_eq!(eval_global_string(src, "r"), "aead payload|tag96");
}

#[test]
fn gcm_wrong_additional_data_fails_auth() {
    // additionalData participates in the tag, so a mismatch on decrypt must
    // fail authentication (validates the host threads aad into the op).
    let src = "globalThis.r = 'pending'; \
         const iv = new Uint8Array(12).fill(4); \
         const msg = new TextEncoder().encode('x'); \
         crypto.subtle.generateKey({name:'AES-GCM', length:128}, true, ['encrypt','decrypt']) \
           .then(k => crypto.subtle.encrypt({name:'AES-GCM', iv, additionalData: new Uint8Array([1,2,3])}, k, msg) \
             .then(ct => crypto.subtle.decrypt({name:'AES-GCM', iv, additionalData: new Uint8Array([9,9,9])}, k, ct))) \
           .then(_ => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "OperationError");
}
