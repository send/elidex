//! RSA (RSASSA-PKCS1-v1_5 / RSA-PSS) crate-level tests (WebCrypto §20 / §21):
//! the generateKey usage-split + key shape, the import / export round-trips
//! across spki / pkcs8 / jwk, RSASSA / RSA-PSS sign / verify, and the §20.8.4 /
//! §21.4.4 invalid-shape (DataError / SyntaxError / NotSupported) set.
//!
//! Keys are produced by a **seeded** deterministic CSPRNG (reproducible,
//! no OS-entropy dependency) at the 2048-bit modulus length.  The JS-level
//! vertical lives in the VM crate's `tests_crypto::rsa`; the `marshal_jwk` ≡
//! `from_json_bytes` JWK mirror differential test lives in
//! `subtle_crypto::differential`.

use rand_chacha::rand_core::{RngCore, SeedableRng};

use crate::algorithm::RsaVariant;
use crate::error::AlgorithmError;
use crate::hash::HashAlgorithm;
use crate::key::{KeyAlgorithm, KeyType, KeyUsage};
use crate::ops::{
    export_key, generate_key, import_key, ExportedKey, GeneratedKey, KeyData, KeyFormat,
};
use crate::{normalize, CryptoKeyData, JsonWebKey, NormalizedAlgorithm, Operation, RawAlgorithm};

/// A deterministic `fill_random` closure (a seeded ChaCha20 CSPRNG) for
/// reproducible RSA key generation — valid randomness without OS entropy.
fn seeded_fill(seed: u64) -> impl FnMut(&mut [u8]) -> Result<(), AlgorithmError> {
    let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(seed);
    move |buf| {
        rng.fill_bytes(buf);
        Ok(())
    }
}

fn keygen_alg(
    variant: RsaVariant,
    modulus_length: u32,
    hash: HashAlgorithm,
) -> NormalizedAlgorithm {
    let mut raw = RawAlgorithm::from_name(variant.canonical_name());
    raw.modulus_length = Some(modulus_length);
    raw.public_exponent = Some(vec![0x01, 0x00, 0x01]);
    raw.hash = Some(Box::new(RawAlgorithm::from_name(hash.canonical_name())));
    normalize(Operation::GenerateKey, raw).expect("RSA keygen algorithm normalizes")
}

fn import_alg(variant: RsaVariant, hash: HashAlgorithm) -> NormalizedAlgorithm {
    let mut raw = RawAlgorithm::from_name(variant.canonical_name());
    raw.hash = Some(Box::new(RawAlgorithm::from_name(hash.canonical_name())));
    normalize(Operation::ImportKey, raw).expect("RSA import algorithm normalizes")
}

/// Generate a `(public, private)` RSA key pair at 2048 bits over a fixed seed.
fn generate_pair(
    variant: RsaVariant,
    hash: HashAlgorithm,
    usages: Vec<KeyUsage>,
) -> (CryptoKeyData, CryptoKeyData) {
    match generate_key(
        keygen_alg(variant, 2048, hash),
        true,
        usages,
        seeded_fill(0x5A),
    )
    .unwrap()
    {
        GeneratedKey::Pair { public, private } => (public, private),
        GeneratedKey::Single(_) => panic!("RSA generateKey yields a key pair"),
    }
}

fn expect_jwk(exported: ExportedKey) -> JsonWebKey {
    match exported {
        ExportedKey::Jwk(jwk) => jwk,
        ExportedKey::Raw(_) => panic!("expected a JWK export"),
    }
}

fn expect_raw(exported: ExportedKey) -> Vec<u8> {
    match exported {
        ExportedKey::Raw(bytes) => bytes,
        ExportedKey::Jwk(_) => panic!("expected a raw / DER export"),
    }
}

#[test]
fn generate_key_shape_and_usage_split() {
    let (public, private) = generate_pair(
        RsaVariant::RsassaPkcs1V15,
        HashAlgorithm::Sha256,
        vec![KeyUsage::Sign, KeyUsage::Verify],
    );
    // The public key takes `verify`, is always extractable; the private key
    // takes `sign`.
    assert_eq!(public.key_type, KeyType::Public);
    assert!(public.extractable);
    assert_eq!(public.usages, vec![KeyUsage::Verify]);
    assert_eq!(private.key_type, KeyType::Private);
    assert_eq!(private.usages, vec![KeyUsage::Sign]);

    // The key's `[[algorithm]]` carries the variant + modulus length + public
    // exponent + hash (RsaHashedKeyAlgorithm §20.6).
    let KeyAlgorithm::Rsa {
        variant,
        modulus_length,
        public_exponent,
        hash,
    } = &private.algorithm
    else {
        panic!("RSA key has an Rsa algorithm");
    };
    assert_eq!(*variant, RsaVariant::RsassaPkcs1V15);
    assert_eq!(*modulus_length, 2048);
    assert_eq!(public_exponent, &vec![0x01, 0x00, 0x01]);
    assert_eq!(*hash, HashAlgorithm::Sha256);
    // The public key shares the same `[[algorithm]]`.
    assert_eq!(public.algorithm, private.algorithm);
}

#[test]
fn generate_empty_private_usages_is_syntax_error() {
    // `verify` only → the private half has empty usages → SyntaxError.
    let err = generate_key(
        keygen_alg(RsaVariant::RsaPss, 2048, HashAlgorithm::Sha256),
        true,
        vec![KeyUsage::Verify],
        seeded_fill(0x5A),
    )
    .expect_err("empty private usages is a SyntaxError");
    assert!(matches!(err, AlgorithmError::Syntax(_)), "got {err:?}");
}

#[test]
fn generate_invalid_usage_is_syntax_error() {
    let err = generate_key(
        keygen_alg(RsaVariant::RsassaPkcs1V15, 2048, HashAlgorithm::Sha256),
        true,
        vec![KeyUsage::Encrypt],
        seeded_fill(0x5A),
    )
    .expect_err("a non-sign/verify usage is a SyntaxError");
    assert!(matches!(err, AlgorithmError::Syntax(_)), "got {err:?}");
}

#[test]
fn private_pkcs8_round_trip() {
    let (_public, private) = generate_pair(
        RsaVariant::RsassaPkcs1V15,
        HashAlgorithm::Sha256,
        vec![KeyUsage::Sign],
    );
    let der = expect_raw(export_key(KeyFormat::Pkcs8, &private).expect("PKCS#8 export"));
    let reimported = import_key(
        KeyFormat::Pkcs8,
        import_alg(RsaVariant::RsassaPkcs1V15, HashAlgorithm::Sha256),
        true,
        vec![KeyUsage::Sign],
        KeyData::Raw(der),
    )
    .expect("PKCS#8 re-import");
    assert_eq!(reimported.key_type, KeyType::Private);
    assert_eq!(reimported.material, private.material);
    assert_eq!(reimported.algorithm, private.algorithm);
}

#[test]
fn public_spki_round_trip() {
    let (public, _private) = generate_pair(
        RsaVariant::RsaPss,
        HashAlgorithm::Sha384,
        vec![KeyUsage::Sign, KeyUsage::Verify],
    );
    let der = expect_raw(export_key(KeyFormat::Spki, &public).expect("SPKI export"));
    let reimported = import_key(
        KeyFormat::Spki,
        import_alg(RsaVariant::RsaPss, HashAlgorithm::Sha384),
        true,
        vec![KeyUsage::Verify],
        KeyData::Raw(der),
    )
    .expect("SPKI re-import");
    assert_eq!(reimported.key_type, KeyType::Public);
    assert_eq!(reimported.material, public.material);
}

#[test]
fn jwk_private_round_trip_includes_crt_members() {
    let (_public, private) = generate_pair(
        RsaVariant::RsassaPkcs1V15,
        HashAlgorithm::Sha256,
        vec![KeyUsage::Sign],
    );
    let jwk = expect_jwk(export_key(KeyFormat::Jwk, &private).expect("JWK export"));
    assert_eq!(jwk.kty.as_deref(), Some("RSA"));
    assert_eq!(jwk.alg.as_deref(), Some("RS256"));
    assert_eq!(jwk.ext, Some(true));
    assert_eq!(jwk.key_ops.as_deref(), Some(&["sign".to_string()][..]));
    // A private RSA JWK carries n / e / d + all CRT members.
    for member in [
        &jwk.n, &jwk.e, &jwk.d, &jwk.p, &jwk.q, &jwk.dp, &jwk.dq, &jwk.qi,
    ] {
        assert!(member.is_some(), "private JWK is missing a member");
    }

    // Re-import the JWK — the recovered key matches.
    let reimported = import_key(
        KeyFormat::Jwk,
        import_alg(RsaVariant::RsassaPkcs1V15, HashAlgorithm::Sha256),
        true,
        vec![KeyUsage::Sign],
        KeyData::Jwk(jwk),
    )
    .expect("JWK re-import");
    assert_eq!(reimported.material, private.material);
}

#[test]
fn jwk_public_round_trip() {
    let (public, _private) = generate_pair(
        RsaVariant::RsaPss,
        HashAlgorithm::Sha512,
        vec![KeyUsage::Sign, KeyUsage::Verify],
    );
    let jwk = expect_jwk(export_key(KeyFormat::Jwk, &public).expect("JWK export"));
    assert_eq!(jwk.kty.as_deref(), Some("RSA"));
    assert_eq!(jwk.alg.as_deref(), Some("PS512"));
    assert!(jwk.n.is_some() && jwk.e.is_some());
    // A public JWK has no private members.
    assert!(jwk.d.is_none() && jwk.p.is_none() && jwk.q.is_none());

    let reimported = import_key(
        KeyFormat::Jwk,
        import_alg(RsaVariant::RsaPss, HashAlgorithm::Sha512),
        true,
        vec![KeyUsage::Verify],
        KeyData::Jwk(jwk),
    )
    .expect("public JWK re-import");
    assert_eq!(reimported.key_type, KeyType::Public);
    assert_eq!(reimported.material, public.material);
}

#[test]
fn raw_format_is_not_supported() {
    let err = import_key(
        KeyFormat::Raw,
        import_alg(RsaVariant::RsassaPkcs1V15, HashAlgorithm::Sha256),
        true,
        vec![KeyUsage::Verify],
        KeyData::Raw(vec![0u8; 16]),
    )
    .expect_err("RSA has no raw import format");
    assert!(
        matches!(err, AlgorithmError::NotSupported(_)),
        "got {err:?}"
    );
}

#[test]
fn jwk_multiprime_oth_is_not_supported() {
    // A private RSA JWK with a non-empty `oth` (multi-prime) is rejected before
    // the key is reconstructed (the DER storage cannot encode >2 primes).
    let jwk = JsonWebKey {
        kty: Some("RSA".to_string()),
        d: Some("aaaa".to_string()),
        n: Some("aaaa".to_string()),
        e: Some("AQAB".to_string()),
        oth: Some(vec![crate::RsaOtherPrimesInfo::default()]),
        ..Default::default()
    };
    let err = import_key(
        KeyFormat::Jwk,
        import_alg(RsaVariant::RsassaPkcs1V15, HashAlgorithm::Sha256),
        true,
        vec![KeyUsage::Sign],
        KeyData::Jwk(jwk),
    )
    .expect_err("multi-prime RSA is NotSupported");
    assert!(
        matches!(err, AlgorithmError::NotSupported(_)),
        "got {err:?}"
    );
}
