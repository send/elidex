//! RSA-OAEP (WebCrypto §22) front-end plumbing: the generateKey usage-split +
//! key shape, the import / export round-trips (spki / pkcs8 / jwk with the
//! `RSA-OAEP-*` `alg`), and the `RsaOaepParams.label` normalization.  The
//! `encrypt` / `decrypt` / `wrapKey` / `unwrapKey` op-set (the dual-backend
//! seam) lands with the backend; this commit only wires the registry + keys.

use super::*;
use crate::key::{KeyAlgorithm, KeyType};
use crate::ops::export_key;

#[test]
fn generate_oaep_key_shape_and_usage_split() {
    let (public, private) = generate_pair(
        RsaVariant::RsaOaep,
        HashAlgorithm::Sha256,
        vec![
            KeyUsage::Encrypt,
            KeyUsage::Decrypt,
            KeyUsage::WrapKey,
            KeyUsage::UnwrapKey,
        ],
    );
    // §22.4.3 usage split (distinct from the RSASSA / RSA-PSS sign families):
    // the public key takes {encrypt, wrapKey}, the private key {decrypt,
    // unwrapKey}; the public half is always extractable.
    assert_eq!(public.key_type, KeyType::Public);
    assert!(public.extractable);
    assert_eq!(public.usages, vec![KeyUsage::Encrypt, KeyUsage::WrapKey]);
    assert_eq!(private.key_type, KeyType::Private);
    assert_eq!(private.usages, vec![KeyUsage::Decrypt, KeyUsage::UnwrapKey]);

    // The key's `[[algorithm]]` is the shared `RsaHashedKeyAlgorithm` (§20.6,
    // reused by §22)
    // with the RsaOaep variant + the modulus / exponent / hash.
    let KeyAlgorithm::Rsa {
        variant,
        modulus_length,
        public_exponent,
        hash,
    } = &private.algorithm
    else {
        panic!("RSA-OAEP key has an Rsa algorithm");
    };
    assert_eq!(*variant, RsaVariant::RsaOaep);
    assert_eq!(*modulus_length, 2048);
    assert_eq!(public_exponent, &vec![0x01, 0x00, 0x01]);
    assert_eq!(*hash, HashAlgorithm::Sha256);
    assert_eq!(public.algorithm, private.algorithm);
}

#[test]
fn generate_oaep_with_sign_usage_is_syntax_error() {
    // §22.4.3 step 1: a usage outside {encrypt, decrypt, wrapKey, unwrapKey}
    // (e.g. the sign-family `sign`) is a SyntaxError.
    let err = generate_key(
        keygen_alg(RsaVariant::RsaOaep, 2048, HashAlgorithm::Sha256),
        true,
        vec![KeyUsage::Sign],
        seeded_fill(0x5A),
    )
    .expect_err("a sign usage is invalid for RSA-OAEP");
    assert!(matches!(err, AlgorithmError::Syntax(_)), "got {err:?}");
}

#[test]
fn oaep_private_pkcs8_round_trip() {
    let (_public, private) = generate_pair(
        RsaVariant::RsaOaep,
        HashAlgorithm::Sha256,
        vec![KeyUsage::Decrypt, KeyUsage::UnwrapKey],
    );
    let der = expect_raw(export_key(KeyFormat::Pkcs8, &private).expect("PKCS#8 export"));
    let reimported = import_key(
        KeyFormat::Pkcs8,
        import_alg(RsaVariant::RsaOaep, HashAlgorithm::Sha256),
        true,
        vec![KeyUsage::Decrypt],
        KeyData::Raw(der),
    )
    .expect("PKCS#8 re-import");
    assert_eq!(reimported.key_type, KeyType::Private);
    assert_eq!(reimported.material, private.material);
    assert_eq!(reimported.algorithm, private.algorithm);
}

#[test]
fn oaep_public_spki_round_trip() {
    let (public, _private) = generate_pair(
        RsaVariant::RsaOaep,
        HashAlgorithm::Sha384,
        vec![KeyUsage::Encrypt, KeyUsage::Decrypt],
    );
    let der = expect_raw(export_key(KeyFormat::Spki, &public).expect("SPKI export"));
    let reimported = import_key(
        KeyFormat::Spki,
        import_alg(RsaVariant::RsaOaep, HashAlgorithm::Sha384),
        true,
        vec![KeyUsage::Encrypt],
        KeyData::Raw(der),
    )
    .expect("SPKI re-import");
    assert_eq!(reimported.key_type, KeyType::Public);
    assert_eq!(reimported.material, public.material);
}

#[test]
fn oaep_jwk_private_round_trip_uses_rsa_oaep_alg() {
    let (_public, private) = generate_pair(
        RsaVariant::RsaOaep,
        HashAlgorithm::Sha256,
        vec![KeyUsage::Decrypt, KeyUsage::UnwrapKey],
    );
    let jwk = expect_jwk(export_key(KeyFormat::Jwk, &private).expect("JWK export"));
    assert_eq!(jwk.kty.as_deref(), Some("RSA"));
    // §22.4.5 jwk export: SHA-256 → "RSA-OAEP-256" (the SHA-1 form is the bare
    // "RSA-OAEP").
    assert_eq!(jwk.alg.as_deref(), Some("RSA-OAEP-256"));
    let reimported = import_key(
        KeyFormat::Jwk,
        import_alg(RsaVariant::RsaOaep, HashAlgorithm::Sha256),
        true,
        vec![KeyUsage::Decrypt, KeyUsage::UnwrapKey],
        KeyData::Jwk(Box::new(jwk)),
    )
    .expect("JWK re-import");
    assert_eq!(reimported.material, private.material);
}

#[test]
fn oaep_jwk_public_round_trip_sha1_bare_alg() {
    let (public, _private) = generate_pair(
        RsaVariant::RsaOaep,
        HashAlgorithm::Sha1,
        // Both halves need a usage (the private half cannot be empty, §14.3.6),
        // so request encrypt (public) + decrypt (private); the public export
        // below carries only the public {encrypt} usage.
        vec![KeyUsage::Encrypt, KeyUsage::Decrypt],
    );
    let jwk = expect_jwk(export_key(KeyFormat::Jwk, &public).expect("JWK export"));
    // §22.4.5 jwk export: SHA-1 → the bare "RSA-OAEP".
    assert_eq!(jwk.alg.as_deref(), Some("RSA-OAEP"));
    assert!(jwk.n.is_some() && jwk.e.is_some());
    assert!(jwk.d.is_none());
    let reimported = import_key(
        KeyFormat::Jwk,
        import_alg(RsaVariant::RsaOaep, HashAlgorithm::Sha1),
        true,
        vec![KeyUsage::Encrypt],
        KeyData::Jwk(Box::new(jwk)),
    )
    .expect("public JWK re-import");
    assert_eq!(reimported.key_type, KeyType::Public);
    assert_eq!(reimported.material, public.material);
}

#[test]
fn import_oaep_public_with_decrypt_usage_is_syntax_error() {
    // §22.4.4: a public key accepts only {encrypt, wrapKey}; `decrypt` (a
    // private-half usage) on an spki import is a SyntaxError.
    let (public, _private) = generate_pair(
        RsaVariant::RsaOaep,
        HashAlgorithm::Sha256,
        vec![KeyUsage::Encrypt, KeyUsage::Decrypt],
    );
    let der = expect_raw(export_key(KeyFormat::Spki, &public).expect("SPKI export"));
    let err = import_key(
        KeyFormat::Spki,
        import_alg(RsaVariant::RsaOaep, HashAlgorithm::Sha256),
        true,
        vec![KeyUsage::Decrypt],
        KeyData::Raw(der),
    )
    .expect_err("decrypt is not a valid public-key usage");
    assert!(matches!(err, AlgorithmError::Syntax(_)), "got {err:?}");
}

#[test]
fn normalize_oaep_label_present_absent_and_wrap_paths() {
    // §22.3: `label` is optional → absent normalizes to `None`.
    let alg = normalize(Operation::Encrypt, RawAlgorithm::from_name("RSA-OAEP"))
        .expect("RSA-OAEP encrypt normalizes");
    assert_eq!(alg, NormalizedAlgorithm::RsaOaep { label: None });
    // A present `label` rides into the normalized algorithm by value.
    let mut raw = RawAlgorithm::from_name("RSA-OAEP");
    raw.label = Some(vec![0xAA, 0xBB, 0xCC]);
    let alg = normalize(Operation::Decrypt, raw).expect("RSA-OAEP decrypt normalizes");
    assert_eq!(
        alg,
        NormalizedAlgorithm::RsaOaep {
            label: Some(vec![0xAA, 0xBB, 0xCC])
        }
    );
    // wrapKey / unwrapKey resolve to the same RsaOaep params (the generic
    // §14.3.11 / §14.3.12 encrypt / decrypt fallback target).
    for op in [Operation::WrapKey, Operation::UnwrapKey] {
        let alg = normalize(op, RawAlgorithm::from_name("rsa-oaep"))
            .expect("RSA-OAEP wrap/unwrap normalizes (case-insensitive)");
        assert_eq!(alg, NormalizedAlgorithm::RsaOaep { label: None });
    }
    // §22.2: RSA-OAEP registers no sign / verify / get-key-length.
    assert!(normalize(Operation::Sign, RawAlgorithm::from_name("RSA-OAEP")).is_err());
    assert!(normalize(Operation::GetKeyLength, RawAlgorithm::from_name("RSA-OAEP")).is_err());
}
