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
fn ecdsa_sign_then_verify_round_trips() {
    // generateKey → sign(privateKey) → verify(publicKey) end-to-end.
    let src = "globalThis.r = 'pending'; \
         const data = new Uint8Array([1, 2, 3, 4, 5]); \
         crypto.subtle.generateKey({name:'ECDSA', namedCurve:'P-256'}, false, ['sign','verify']) \
           .then(p => crypto.subtle.sign({name:'ECDSA', hash:'SHA-256'}, p.privateKey, data) \
             .then(sig => crypto.subtle.verify({name:'ECDSA', hash:'SHA-256'}, p.publicKey, sig, data) \
               .then(ok => crypto.subtle.verify({name:'ECDSA', hash:'SHA-256'}, p.publicKey, \
                       new Uint8Array(64), data) \
                 .then(bad => { globalThis.r = (ok === true && bad === false) ? 'ok' : \
                       ('ok=' + ok + ' bad=' + bad); })))) \
           .catch(e => { globalThis.r = 'ERR:' + e.name; });";
    assert_eq!(eval_global_string(src, "r"), "ok");
}

#[test]
fn ecdsa_sign_with_public_key_rejects_invalid_access() {
    // The public key lacks the `sign` usage → InvalidAccessError (async).
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.generateKey({name:'ECDSA', namedCurve:'P-256'}, true, ['sign','verify']) \
           .then(p => crypto.subtle.sign({name:'ECDSA', hash:'SHA-256'}, p.publicKey, \
                   new Uint8Array([1]))) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "InvalidAccessError");
}

#[test]
fn ecdh_derive_bits_is_symmetric() {
    // Two ECDH key pairs; deriveBits(A.priv, B.pub) == deriveBits(B.priv,
    // A.pub) — exercises the §24.3 `public` CryptoKey member marshalling.
    let src = "globalThis.r = 'pending'; \
         const gen = () => crypto.subtle.generateKey({name:'ECDH', namedCurve:'P-256'}, true, ['deriveBits']); \
         gen().then(a => gen().then(b => \
           crypto.subtle.deriveBits({name:'ECDH', public: b.publicKey}, a.privateKey, 256).then(s1 => \
             crypto.subtle.deriveBits({name:'ECDH', public: a.publicKey}, b.privateKey, 256).then(s2 => { \
               const u1 = new Uint8Array(s1), u2 = new Uint8Array(s2); \
               let eq = (u1.length === 32 && u2.length === 32); \
               for (let i = 0; i < u1.length; i++) { if (u1[i] !== u2[i]) eq = false; } \
               globalThis.r = eq ? 'equal' : 'diff'; \
             })))) \
           .catch(e => { globalThis.r = 'ERR:' + e.name; });";
    assert_eq!(eval_global_string(src, "r"), "equal");
}

#[test]
fn ecdh_derive_bits_non_crypto_key_public_is_type_error() {
    // §24.3 `public` is a required CryptoKey member — a plain object is a
    // WebIDL TypeError (settled on the Promise).
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.generateKey({name:'ECDH', namedCurve:'P-256'}, true, ['deriveBits']) \
           .then(p => crypto.subtle.deriveBits({name:'ECDH', public: {}}, p.privateKey, 128)) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "TypeError");
}

#[test]
fn ecdh_derive_bits_curve_mismatch_rejects_invalid_access() {
    // A P-384 peer against a P-256 base key → InvalidAccessError (§24.4.2
    // step 5).
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.generateKey({name:'ECDH', namedCurve:'P-256'}, true, ['deriveBits']) \
           .then(base => crypto.subtle.generateKey({name:'ECDH', namedCurve:'P-384'}, true, ['deriveBits']) \
             .then(peer => crypto.subtle.deriveBits({name:'ECDH', public: peer.publicKey}, base.privateKey, 128))) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "InvalidAccessError");
}

#[test]
fn ecdh_key_cannot_sign() {
    // ECDH registers no sign operation → NotSupportedError (the registry
    // excludes (Sign, ECDH)).
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.generateKey({name:'ECDH', namedCurve:'P-256'}, true, ['deriveBits']) \
           .then(p => crypto.subtle.sign({name:'ECDH'}, p.privateKey, new Uint8Array([1]))) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "NotSupportedError");
}

#[test]
fn ecdsa_key_cannot_derive_bits() {
    // ECDSA registers no deriveBits operation → NotSupportedError.
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.generateKey({name:'ECDSA', namedCurve:'P-256'}, true, ['sign','verify']) \
           .then(p => crypto.subtle.deriveBits({name:'ECDSA', public: p.publicKey}, p.privateKey, 128)) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "NotSupportedError");
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
