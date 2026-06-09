//! RSASSA-PKCS1-v1_5 / RSA-PSS vertical (WebCrypto §20 / §21) JS-level tests:
//! the `generateKey` → `CryptoKeyPair` shape, the `RsaHashedKeyAlgorithm`
//! `[[algorithm]]` (`{name, modulusLength, publicExponent: Uint8Array,
//! hash:{name}}`), sign / verify end-to-end (both families, incl. the RSASSA
//! name-only params + the RSA-PSS `saltLength`), the jwk export / re-import
//! round-trip, and the required-member / registry-exclusion error paths.
//!
//! The algorithm-level correctness (padding, DER, the invalid-shape matrix)
//! lives in the crate's `tests::rsa`; this pins the VM marshalling surface.
//! Keys are 2048-bit (the realistic minimum), so each `generateKey` runs a
//! real RSA key generation.

use super::eval_global_string;

/// A 2048-bit RSASSA-PKCS1-v1_5 keygen algorithm literal (publicExponent
/// 65537) for the given JS `usages`.
const RSASSA_GEN: &str =
    "{name:'RSASSA-PKCS1-v1_5', modulusLength:2048, publicExponent:new Uint8Array([1,0,1]), hash:'SHA-256'}";
const RSAPSS_GEN: &str =
    "{name:'RSA-PSS', modulusLength:2048, publicExponent:new Uint8Array([1,0,1]), hash:'SHA-256'}";
/// A 2048-bit RSA-OAEP keygen algorithm literal (SHA-256) — the encrypt
/// family, reusing the RsaHashed key params (§22.4.3 reuses §20.4).
const RSAOAEP_GEN: &str =
    "{name:'RSA-OAEP', modulusLength:2048, publicExponent:new Uint8Array([1,0,1]), hash:'SHA-256'}";

#[test]
fn rsassa_generate_key_returns_crypto_key_pair_with_algorithm() {
    // §20.8.3: a CryptoKeyPair whose publicKey is verify-only + always
    // extractable, privateKey is sign-only.  The `[[algorithm]]` is the
    // RsaHashedKeyAlgorithm (§20.6) — note publicExponent is a Uint8Array.
    let src = format!(
        "globalThis.r = 'pending'; \
         crypto.subtle.generateKey({RSASSA_GEN}, false, ['sign','verify']) \
           .then(p => {{ const a = p.publicKey.algorithm; globalThis.r = [ \
             Object.keys(p).join(','), \
             a.name, a.modulusLength, a.hash.name, \
             (a.publicExponent instanceof Uint8Array), \
             Array.from(a.publicExponent).join('.'), \
             p.publicKey.usages.join('/'), p.privateKey.usages.join('/'), \
             p.publicKey.extractable, p.privateKey.extractable \
           ].join('|'); }}, e => {{ globalThis.r = 'ERR:' + e.name; }});"
    );
    assert_eq!(
        eval_global_string(&src, "r"),
        "privateKey,publicKey|RSASSA-PKCS1-v1_5|2048|SHA-256|true|1.0.1|verify|sign|true|false"
    );
}

#[test]
fn rsa_algorithm_keys_are_in_webidl_inheritance_order() {
    // Web IDL §2.7 orders dictionary members inherited-first (least- to
    // most-derived), lexicographic within a level.  For `RsaHashedKeyAlgorithm
    // : RsaKeyAlgorithm : KeyAlgorithm`, `Object.keys(key.algorithm)` is
    // `name, modulusLength, publicExponent, hash` — NOT the flat lexicographic
    // `hash, modulusLength, name, publicExponent` (Codex R5).  Both keys of the
    // pair share the `RsaHashedKeyAlgorithm` shape.
    let src = format!(
        "globalThis.r = 'pending'; \
         crypto.subtle.generateKey({RSASSA_GEN}, false, ['sign','verify']) \
           .then(p => {{ globalThis.r = [ \
             Object.keys(p.publicKey.algorithm).join(','), \
             Object.keys(p.privateKey.algorithm).join(',') \
           ].join('|'); }}, e => {{ globalThis.r = 'ERR:' + e.name; }});"
    );
    assert_eq!(
        eval_global_string(&src, "r"),
        "name,modulusLength,publicExponent,hash|name,modulusLength,publicExponent,hash"
    );
}

#[test]
fn rsassa_sign_then_verify_round_trips_with_name_only_params() {
    // generateKey → sign(privateKey) → verify(publicKey).  RSASSA sign / verify
    // take a name-only `Algorithm` (the hash rides on the key, §20.6).
    let src = format!(
        "globalThis.r = 'pending'; \
         const data = new Uint8Array([1, 2, 3, 4, 5]); \
         crypto.subtle.generateKey({RSASSA_GEN}, false, ['sign','verify']) \
           .then(p => crypto.subtle.sign({{name:'RSASSA-PKCS1-v1_5'}}, p.privateKey, data) \
             .then(sig => crypto.subtle.verify({{name:'RSASSA-PKCS1-v1_5'}}, p.publicKey, sig, data) \
               .then(ok => crypto.subtle.verify({{name:'RSASSA-PKCS1-v1_5'}}, p.publicKey, sig, \
                       new Uint8Array([9, 9])) \
                 .then(bad => {{ globalThis.r = (ok === true && bad === false) ? 'ok' : \
                       ('ok=' + ok + ' bad=' + bad); }})))) \
           .catch(e => {{ globalThis.r = 'ERR:' + e.name; }});"
    );
    assert_eq!(eval_global_string(&src, "r"), "ok");
}

#[test]
fn rsapss_sign_then_verify_round_trips_with_salt_length() {
    // RSA-PSS sign / verify carry `saltLength` (§21.3) — marshalled through the
    // RsaPssParams arm.
    let src = format!(
        "globalThis.r = 'pending'; \
         const data = new Uint8Array([7, 7, 7]); \
         crypto.subtle.generateKey({RSAPSS_GEN}, false, ['sign','verify']) \
           .then(p => crypto.subtle.sign({{name:'RSA-PSS', saltLength:32}}, p.privateKey, data) \
             .then(sig => crypto.subtle.verify({{name:'RSA-PSS', saltLength:32}}, p.publicKey, sig, data) \
               .then(ok => {{ globalThis.r = ok === true ? 'ok' : ('bad:' + ok); }}))) \
           .catch(e => {{ globalThis.r = 'ERR:' + e.name; }});"
    );
    assert_eq!(eval_global_string(&src, "r"), "ok");
}

#[test]
fn rsapss_sign_missing_salt_length_is_type_error() {
    // `RsaPssParams.saltLength` is a required member → TypeError (async).
    let src = format!(
        "globalThis.r = 'pending'; \
         crypto.subtle.generateKey({RSAPSS_GEN}, false, ['sign','verify']) \
           .then(p => crypto.subtle.sign({{name:'RSA-PSS'}}, p.privateKey, new Uint8Array([1]))) \
           .then(() => {{ globalThis.r = 'resolved'; }}, e => {{ globalThis.r = e.name; }});"
    );
    assert_eq!(eval_global_string(&src, "r"), "TypeError");
}

#[test]
fn rsa_jwk_export_reimport_round_trips() {
    // generateKey → exportKey('jwk') → importKey('jwk'): the exported private
    // JWK carries kty='RSA', alg='RS256', and the n/e/d + CRT members in
    // lexicographic key order, and re-imports to a private key.
    let src = format!(
        "globalThis.r = 'pending'; \
         crypto.subtle.generateKey({RSASSA_GEN}, true, ['sign','verify']) \
           .then(p => crypto.subtle.exportKey('jwk', p.privateKey) \
             .then(jwk => crypto.subtle.importKey('jwk', jwk, \
                     {{name:'RSASSA-PKCS1-v1_5', hash:'SHA-256'}}, true, ['sign']) \
               .then(k => {{ globalThis.r = [ \
                 jwk.kty, jwk.alg, k.type, Object.keys(jwk).join(',') \
               ].join('|'); }}))) \
           .catch(e => {{ globalThis.r = 'ERR:' + e.name; }});"
    );
    assert_eq!(
        eval_global_string(&src, "r"),
        "RSA|RS256|private|alg,d,dp,dq,e,ext,key_ops,kty,n,p,q,qi"
    );
}

#[test]
fn rsa_generate_key_missing_modulus_length_is_type_error() {
    // `RsaHashedKeyGenParams.modulusLength` is required → TypeError.
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.generateKey({name:'RSASSA-PKCS1-v1_5', \
                 publicExponent:new Uint8Array([1,0,1]), hash:'SHA-256'}, false, ['sign','verify']) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "TypeError");
}

#[test]
fn rsa_generate_key_missing_public_exponent_is_type_error() {
    // `RsaHashedKeyGenParams.publicExponent` is required → TypeError.
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.generateKey({name:'RSASSA-PKCS1-v1_5', modulusLength:2048, hash:'SHA-256'}, \
                 false, ['sign','verify']) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "TypeError");
}

#[test]
fn rsa_key_cannot_derive_bits() {
    // RSASSA registers no deriveBits operation → NotSupportedError (the registry
    // excludes (DeriveBits, RSASSA-PKCS1-v1_5)).
    let src = format!(
        "globalThis.r = 'pending'; \
         crypto.subtle.generateKey({RSASSA_GEN}, true, ['sign','verify']) \
           .then(p => crypto.subtle.deriveBits({{name:'RSASSA-PKCS1-v1_5'}}, p.privateKey, 128)) \
           .then(() => {{ globalThis.r = 'resolved'; }}, e => {{ globalThis.r = e.name; }});"
    );
    assert_eq!(eval_global_string(&src, "r"), "NotSupportedError");
}

#[test]
fn rsa_generate_key_non_uint8array_public_exponent_is_type_error() {
    // §20.3: `RsaKeyGenParams.publicExponent` is a `BigInteger` (the `Uint8Array`
    // typedef), NOT any `BufferSource` — an `ArrayBuffer` is a Web IDL TypeError.
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.generateKey({name:'RSASSA-PKCS1-v1_5', modulusLength:2048, \
                 publicExponent: new ArrayBuffer(3), hash:'SHA-256'}, false, ['sign','verify']) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "TypeError");
}

#[test]
fn rsa_generate_key_reads_inherited_members_before_hash() {
    // RsaHashedKeyGenParams : RsaKeyGenParams — Web IDL converts the inherited
    // modulusLength / publicExponent before the derived hash, so the getter
    // firing order is modulusLength, publicExponent, hash (NOT hash-first).
    let src = "globalThis.order = []; globalThis.r = 'pending'; \
         crypto.subtle.generateKey({ \
             name: 'RSASSA-PKCS1-v1_5', \
             get modulusLength() { globalThis.order.push('ml'); return 2048; }, \
             get publicExponent() { globalThis.order.push('pe'); return new Uint8Array([1,0,1]); }, \
             get hash() { globalThis.order.push('h'); return 'SHA-256'; } \
           }, false, ['sign','verify']) \
           .then(() => { globalThis.r = globalThis.order.join(','); }, \
                 e => { globalThis.r = 'ERR:' + globalThis.order.join(','); });";
    assert_eq!(eval_global_string(src, "r"), "ml,pe,h");
}

// ===========================================================================
// RSA-OAEP vertical (WebCrypto §22) — the encrypt / decrypt / wrapKey /
// unwrapKey op-set on the aws-lc-rs backend.  The algorithm-level coverage
// (padding, labels, the dual-backend seam) lives in the crate's
// `tests::rsa::oaep`; these pin the VM marshalling surface (the `RsaOaepParams`
// label read + the generic cipher / wrap natives routing RSA-OAEP).
// ===========================================================================

#[test]
fn rsa_oaep_generate_key_pair_shape_and_usage_split() {
    // §22.4.3: a CryptoKeyPair whose publicKey is {encrypt, wrapKey} + always
    // extractable, privateKey is {decrypt, unwrapKey}; name = "RSA-OAEP", the
    // hash rides on the key (RsaHashedKeyAlgorithm §20.6, reused by §22).
    let src = format!(
        "globalThis.r = 'pending'; \
         crypto.subtle.generateKey({RSAOAEP_GEN}, false, ['encrypt','decrypt','wrapKey','unwrapKey']) \
           .then(p => {{ const a = p.publicKey.algorithm; globalThis.r = [ \
             a.name, a.modulusLength, a.hash.name, \
             p.publicKey.usages.join('/'), p.privateKey.usages.join('/'), \
             p.publicKey.extractable, p.privateKey.extractable \
           ].join('|'); }}, e => {{ globalThis.r = 'ERR:' + e.name; }});"
    );
    assert_eq!(
        eval_global_string(&src, "r"),
        "RSA-OAEP|2048|SHA-256|encrypt/wrapKey|decrypt/unwrapKey|true|false"
    );
}

#[test]
fn rsa_oaep_encrypt_then_decrypt_round_trips() {
    // generateKey → encrypt(publicKey) → decrypt(privateKey).  RSA-OAEP
    // encrypt / decrypt take the optional name-only `RsaOaepParams` (no label).
    let src = format!(
        "globalThis.r = 'pending'; \
         crypto.subtle.generateKey({RSAOAEP_GEN}, true, ['encrypt','decrypt']) \
           .then(p => {{ globalThis.pair = p; \
             return crypto.subtle.encrypt({{name:'RSA-OAEP'}}, p.publicKey, new Uint8Array([10,20,30,40])); }}) \
           .then(ct => crypto.subtle.decrypt({{name:'RSA-OAEP'}}, globalThis.pair.privateKey, ct)) \
           .then(pt => {{ globalThis.r = Array.from(new Uint8Array(pt)).join('.'); }}, \
                 e => {{ globalThis.r = 'ERR:' + e.name; }});"
    );
    assert_eq!(eval_global_string(&src, "r"), "10.20.30.40");
}

#[test]
fn rsa_oaep_label_round_trips_and_wrong_label_rejects() {
    // §22.3 `RsaOaepParams.label` (the optional BufferSource the VM snapshots):
    // a same-label decrypt recovers the plaintext; a mismatched label fails the
    // OAEP decode as an OperationError.
    let src = format!(
        "globalThis.r = 'pending'; \
         crypto.subtle.generateKey({RSAOAEP_GEN}, true, ['encrypt','decrypt']) \
           .then(p => {{ globalThis.pair = p; \
             return crypto.subtle.encrypt({{name:'RSA-OAEP', label:new Uint8Array([9,9])}}, p.publicKey, new Uint8Array([7,7,7])); }}) \
           .then(ct => {{ globalThis.ct = ct; \
             return crypto.subtle.decrypt({{name:'RSA-OAEP', label:new Uint8Array([9,9])}}, globalThis.pair.privateKey, ct); }}) \
           .then(pt => {{ globalThis.good = Array.from(new Uint8Array(pt)).join('.'); \
             return crypto.subtle.decrypt({{name:'RSA-OAEP', label:new Uint8Array([8,8])}}, globalThis.pair.privateKey, globalThis.ct) \
               .then(() => 'NOFAIL', e => e.name); }}) \
           .then(bad => {{ globalThis.r = globalThis.good + '|' + bad; }}, \
                 e => {{ globalThis.r = 'ERR:' + e.name; }});"
    );
    assert_eq!(eval_global_string(&src, "r"), "7.7.7|OperationError");
}

#[test]
fn rsa_oaep_wrap_unwrap_aes_key_round_trips() {
    // §14.3.11 / §14.3.12: RSA-OAEP wraps an AES-GCM key (export → OAEP-encrypt;
    // OAEP-decrypt → import) — the generic wrap / unwrap natives route RSA-OAEP
    // through the same encrypt / decrypt op as the cipher path.
    let src = format!(
        "globalThis.r = 'pending'; \
         Promise.all([ \
           crypto.subtle.generateKey({RSAOAEP_GEN}, true, ['wrapKey','unwrapKey']), \
           crypto.subtle.generateKey({{name:'AES-GCM', length:128}}, true, ['encrypt','decrypt']) \
         ]).then(([rsa, aes]) => {{ globalThis.rsa = rsa; \
             return crypto.subtle.wrapKey('raw', aes, rsa.publicKey, {{name:'RSA-OAEP'}}); }}) \
           .then(wrapped => crypto.subtle.unwrapKey('raw', wrapped, globalThis.rsa.privateKey, \
                 {{name:'RSA-OAEP'}}, {{name:'AES-GCM'}}, true, ['encrypt','decrypt'])) \
           .then(k => {{ globalThis.r = [k.type, k.algorithm.name, k.algorithm.length, k.usages.join('/')].join('|'); }}, \
                 e => {{ globalThis.r = 'ERR:' + e.name; }});"
    );
    assert_eq!(
        eval_global_string(&src, "r"),
        "secret|AES-GCM|128|encrypt/decrypt"
    );
}

#[test]
fn rsa_oaep_export_jwk_uses_rsa_oaep_alg() {
    // §22.4.5 jwk export: SHA-256 → alg "RSA-OAEP-256", kty "RSA".
    let src = format!(
        "globalThis.r = 'pending'; \
         crypto.subtle.generateKey({RSAOAEP_GEN}, true, ['encrypt','decrypt']) \
           .then(p => crypto.subtle.exportKey('jwk', p.publicKey)) \
           .then(jwk => {{ globalThis.r = jwk.kty + '|' + jwk.alg; }}, \
                 e => {{ globalThis.r = 'ERR:' + e.name; }});"
    );
    assert_eq!(eval_global_string(&src, "r"), "RSA|RSA-OAEP-256");
}

#[test]
fn rsa_oaep_sign_is_not_supported() {
    // §22.2: RSA-OAEP registers no sign operation, so `sign` normalization
    // returns NotSupportedError (the registry exclusion, distinct from the
    // RSASSA / RSA-PSS signing families).
    let src = format!(
        "globalThis.r = 'pending'; \
         crypto.subtle.generateKey({RSAOAEP_GEN}, true, ['encrypt','decrypt']) \
           .then(p => crypto.subtle.sign({{name:'RSA-OAEP'}}, p.privateKey, new Uint8Array([1,2,3]))) \
           .then(() => {{ globalThis.r = 'resolved'; }}, e => {{ globalThis.r = e.name; }});"
    );
    assert_eq!(eval_global_string(&src, "r"), "NotSupportedError");
}
