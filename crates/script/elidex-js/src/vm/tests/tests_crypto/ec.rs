//! ECDSA / ECDH vertical (WebCrypto §23 / §24) JS-level tests: the
//! `generateKey` → `CryptoKeyPair` shape, the EC `[[algorithm]]`
//! (`EcKeyAlgorithm`), and the usage split / extractable rules.
//!
//! The PR-4 commit-6 batch extends this with the sign / verify / deriveBits
//! verticals + the import / export per-format matrix + the invalid-shape set.

use super::{eval_err, eval_global_string};

#[test]
fn ecdsa_generate_key_returns_crypto_key_pair() {
    // §23.7.3: a CryptoKeyPair whose publicKey is verify-only + always
    // extractable, and privateKey is sign-only + extractable=requested.  Web
    // IDL dictionary→ES order is lexicographic: privateKey before publicKey.
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.generateKey({name:'ECDSA', namedCurve:'P-256'}, false, ['sign','verify']) \
           .then(p => { globalThis.r = [ \
             Object.keys(p).join(','), \
             p.publicKey.type, p.privateKey.type, \
             p.publicKey.extractable, p.privateKey.extractable, \
             p.publicKey.usages.join('/'), p.privateKey.usages.join('/'), \
             p.publicKey.algorithm.name, p.publicKey.algorithm.namedCurve \
           ].join('|'); }, e => { globalThis.r = 'ERR:' + e.name; });";
    assert_eq!(
        eval_global_string(src, "r"),
        "privateKey,publicKey|public|private|true|false|verify|sign|ECDSA|P-256"
    );
}

#[test]
fn ecdh_generate_key_public_has_empty_usages() {
    // §24.4.1: an ECDH public key has the empty usages list; the private key
    // carries the derive usages.
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.generateKey({name:'ECDH', namedCurve:'P-384'}, true, ['deriveBits','deriveKey']) \
           .then(p => { globalThis.r = [ \
             p.publicKey.usages.length, \
             p.privateKey.usages.join('/'), \
             p.privateKey.algorithm.name, p.privateKey.algorithm.namedCurve \
           ].join('|'); }, e => { globalThis.r = 'ERR:' + e.name; });";
    assert_eq!(
        eval_global_string(src, "r"),
        "0|deriveKey/deriveBits|ECDH|P-384"
    );
}

#[test]
fn ecdsa_generate_key_unknown_curve_rejects_not_supported() {
    // NamedCurve is a typedef (prose-validated), so an unknown value is a
    // NotSupportedError (asynchronously, on the Promise).
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.generateKey({name:'ECDSA', namedCurve:'P-999'}, true, ['sign']) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "NotSupportedError");
}

#[test]
fn ecdsa_generate_key_missing_named_curve_is_type_error() {
    // `EcKeyGenParams.namedCurve` is a required member → TypeError.
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.generateKey({name:'ECDSA'}, true, ['sign']) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "TypeError");
}

#[test]
fn crypto_key_pair_is_not_constructable() {
    // CryptoKeyPair is a plain dictionary, not an interface — there is no
    // global constructor for it.
    let err = eval_err("new CryptoKeyPair();");
    assert!(
        err.to_string().contains("CryptoKeyPair")
            || err.to_string().starts_with("ReferenceError")
            || err.to_string().starts_with("TypeError"),
        "unexpected error: {err}"
    );
}
