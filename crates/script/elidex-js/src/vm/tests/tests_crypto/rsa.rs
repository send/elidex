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
