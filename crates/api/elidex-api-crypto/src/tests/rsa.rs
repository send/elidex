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
    export_key, generate_key, import_key, sign, verify, ExportedKey, GeneratedKey, KeyData,
    KeyFormat,
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

fn sign_alg(variant: RsaVariant, salt_length: Option<u32>) -> NormalizedAlgorithm {
    let mut raw = RawAlgorithm::from_name(variant.canonical_name());
    raw.salt_length = salt_length;
    normalize(Operation::Sign, raw).expect("RSA sign algorithm normalizes")
}

fn verify_alg(variant: RsaVariant, salt_length: Option<u32>) -> NormalizedAlgorithm {
    let mut raw = RawAlgorithm::from_name(variant.canonical_name());
    raw.salt_length = salt_length;
    normalize(Operation::Verify, raw).expect("RSA verify algorithm normalizes")
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
        ExportedKey::Jwk(jwk) => *jwk,
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
        KeyData::Jwk(Box::new(jwk)),
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
        KeyData::Jwk(Box::new(jwk)),
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
        KeyData::Jwk(Box::new(jwk)),
    )
    .expect_err("multi-prime RSA is NotSupported");
    assert!(
        matches!(err, AlgorithmError::NotSupported(_)),
        "got {err:?}"
    );
}

// ---------------------------------------------------------------------------
// sign / verify
// ---------------------------------------------------------------------------

#[test]
fn rsassa_sign_verify_round_trip_all_hashes() {
    for hash in [
        HashAlgorithm::Sha256,
        HashAlgorithm::Sha384,
        HashAlgorithm::Sha512,
    ] {
        let (public, private) = generate_pair(
            RsaVariant::RsassaPkcs1V15,
            hash,
            vec![KeyUsage::Sign, KeyUsage::Verify],
        );
        let msg = b"RSASSA-PKCS1-v1_5 message";
        let sig = sign(
            sign_alg(RsaVariant::RsassaPkcs1V15, None),
            &private,
            msg,
            seeded_fill(1),
        )
        .expect("RSASSA sign");
        let ok = verify(
            verify_alg(RsaVariant::RsassaPkcs1V15, None),
            &public,
            &sig,
            msg,
        )
        .expect("RSASSA verify");
        assert!(
            ok,
            "RSASSA-PKCS1-v1_5 round-trip should verify for {hash:?}"
        );
    }
}

#[test]
fn rsapss_sign_verify_round_trip_salt_variants() {
    // RSA-PSS over SHA-256 (hLen = 32): saltLength 0 (deterministic) and 32.
    for salt_length in [0u32, 32] {
        let (public, private) = generate_pair(
            RsaVariant::RsaPss,
            HashAlgorithm::Sha256,
            vec![KeyUsage::Sign, KeyUsage::Verify],
        );
        let msg = b"RSA-PSS message";
        let sig = sign(
            sign_alg(RsaVariant::RsaPss, Some(salt_length)),
            &private,
            msg,
            seeded_fill(2),
        )
        .expect("RSA-PSS sign");
        let ok = verify(
            verify_alg(RsaVariant::RsaPss, Some(salt_length)),
            &public,
            &sig,
            msg,
        )
        .expect("RSA-PSS verify");
        assert!(
            ok,
            "RSA-PSS round-trip should verify (saltLength={salt_length})"
        );
    }
}

#[test]
fn sign_with_public_key_is_invalid_access() {
    let (public, _private) = generate_pair(
        RsaVariant::RsassaPkcs1V15,
        HashAlgorithm::Sha256,
        vec![KeyUsage::Sign, KeyUsage::Verify],
    );
    // A public key has no private DER → InvalidAccessError (the crate gate; the
    // usage gate is the VM `ops::sign` layer).
    let err = sign(
        sign_alg(RsaVariant::RsassaPkcs1V15, None),
        &public,
        b"m",
        seeded_fill(3),
    )
    .expect_err("signing with a public key is rejected");
    assert!(
        matches!(err, AlgorithmError::InvalidAccess(_)),
        "got {err:?}"
    );
}

#[test]
fn verify_rejects_tampered_signature_and_message() {
    let (public, private) = generate_pair(
        RsaVariant::RsassaPkcs1V15,
        HashAlgorithm::Sha256,
        vec![KeyUsage::Sign, KeyUsage::Verify],
    );
    let msg = b"authentic";
    let mut sig = sign(
        sign_alg(RsaVariant::RsassaPkcs1V15, None),
        &private,
        msg,
        seeded_fill(4),
    )
    .expect("sign");
    // A flipped signature byte → false (no throw).
    sig[0] ^= 0xFF;
    assert!(!verify(
        verify_alg(RsaVariant::RsassaPkcs1V15, None),
        &public,
        &sig,
        msg
    )
    .unwrap());
    // A different message → false.
    sig[0] ^= 0xFF; // restore
    assert!(!verify(
        verify_alg(RsaVariant::RsassaPkcs1V15, None),
        &public,
        &sig,
        b"forged"
    )
    .unwrap());
}

#[test]
fn rsapss_verify_wrong_salt_length_is_false() {
    let (public, private) = generate_pair(
        RsaVariant::RsaPss,
        HashAlgorithm::Sha256,
        vec![KeyUsage::Sign, KeyUsage::Verify],
    );
    let msg = b"salt-bound";
    // Sign with saltLength = 32, verify with saltLength = 0 → invalid → false.
    let sig = sign(
        sign_alg(RsaVariant::RsaPss, Some(32)),
        &private,
        msg,
        seeded_fill(5),
    )
    .expect("PSS sign");
    let ok = verify(verify_alg(RsaVariant::RsaPss, Some(0)), &public, &sig, msg).unwrap();
    assert!(!ok, "a saltLength mismatch must fail verification");
}

#[test]
fn cross_hash_verify_is_false() {
    // A signature made with a SHA-256 key does not verify under a public key of
    // the same material re-imported with hash = SHA-384 (the DigestInfo prefix
    // + digest length differ).
    let (public, private) = generate_pair(
        RsaVariant::RsassaPkcs1V15,
        HashAlgorithm::Sha256,
        vec![KeyUsage::Sign, KeyUsage::Verify],
    );
    let msg = b"hash-bound";
    let sig = sign(
        sign_alg(RsaVariant::RsassaPkcs1V15, None),
        &private,
        msg,
        seeded_fill(6),
    )
    .expect("sign");
    let spki = expect_raw(export_key(KeyFormat::Spki, &public).expect("spki"));
    let public384 = import_key(
        KeyFormat::Spki,
        import_alg(RsaVariant::RsassaPkcs1V15, HashAlgorithm::Sha384),
        true,
        vec![KeyUsage::Verify],
        KeyData::Raw(spki),
    )
    .expect("import public under SHA-384");
    let ok = verify(
        verify_alg(RsaVariant::RsassaPkcs1V15, None),
        &public384,
        &sig,
        msg,
    )
    .unwrap();
    assert!(!ok, "a SHA-384 verify of a SHA-256 signature must fail");
}

// ---------------------------------------------------------------------------
// invalid-shape matrix (§20.8.4 DataError / SyntaxError)
// ---------------------------------------------------------------------------

/// Import a public JWK (usages = `verify`) and return the error.
fn import_public_jwk_err(jwk: JsonWebKey) -> AlgorithmError {
    import_key(
        KeyFormat::Jwk,
        import_alg(RsaVariant::RsassaPkcs1V15, HashAlgorithm::Sha256),
        true,
        vec![KeyUsage::Verify],
        KeyData::Jwk(Box::new(jwk)),
    )
    .expect_err("invalid JWK is rejected")
}

/// Import a private JWK (usages = `sign`) and return the error.
fn import_private_jwk_err(jwk: JsonWebKey) -> AlgorithmError {
    import_key(
        KeyFormat::Jwk,
        import_alg(RsaVariant::RsassaPkcs1V15, HashAlgorithm::Sha256),
        true,
        vec![KeyUsage::Sign],
        KeyData::Jwk(Box::new(jwk)),
    )
    .expect_err("invalid JWK is rejected")
}

fn rsa_jwk(members: &[(&str, &str)]) -> JsonWebKey {
    let mut jwk = JsonWebKey {
        kty: Some("RSA".to_string()),
        ..Default::default()
    };
    for &(k, v) in members {
        let v = Some(v.to_string());
        match k {
            "n" => jwk.n = v,
            "e" => jwk.e = v,
            "d" => jwk.d = v,
            "p" => jwk.p = v,
            "q" => jwk.q = v,
            "dp" => jwk.dp = v,
            "dq" => jwk.dq = v,
            "qi" => jwk.qi = v,
            other => panic!("unknown JWK member {other}"),
        }
    }
    jwk
}

#[test]
fn garbage_spki_der_is_data_error() {
    let err = import_key(
        KeyFormat::Spki,
        import_alg(RsaVariant::RsassaPkcs1V15, HashAlgorithm::Sha256),
        true,
        vec![KeyUsage::Verify],
        KeyData::Raw(vec![0xDE, 0xAD, 0xBE, 0xEF]),
    )
    .expect_err("garbage SPKI is a DataError");
    assert!(matches!(err, AlgorithmError::Data(_)), "got {err:?}");
}

#[test]
fn garbage_pkcs8_der_is_data_error() {
    let err = import_key(
        KeyFormat::Pkcs8,
        import_alg(RsaVariant::RsassaPkcs1V15, HashAlgorithm::Sha256),
        true,
        vec![KeyUsage::Sign],
        KeyData::Raw(vec![0x30, 0x03, 0x02, 0x01, 0x00]),
    )
    .expect_err("garbage PKCS#8 is a DataError");
    assert!(matches!(err, AlgorithmError::Data(_)), "got {err:?}");
}

#[test]
fn jwk_wrong_kty_is_data_error() {
    let jwk = JsonWebKey {
        kty: Some("EC".to_string()),
        n: Some("AQID".to_string()),
        e: Some("AQAB".to_string()),
        ..Default::default()
    };
    assert!(
        matches!(import_public_jwk_err(jwk), AlgorithmError::Data(_)),
        "wrong kty must be a DataError"
    );
}

#[test]
fn jwk_public_missing_n_or_e_is_data_error() {
    // n missing.
    assert!(matches!(
        import_public_jwk_err(rsa_jwk(&[("e", "AQAB")])),
        AlgorithmError::Data(_)
    ));
    // e missing.
    assert!(matches!(
        import_public_jwk_err(rsa_jwk(&[("n", "AQID")])),
        AlgorithmError::Data(_)
    ));
}

#[test]
fn jwk_private_partial_crt_members_is_data_error() {
    // `p` present without q / dp / dq / qi → all-or-nothing violation.
    let jwk = rsa_jwk(&[("n", "AQID"), ("e", "AQAB"), ("d", "AQID"), ("p", "AQ")]);
    assert!(
        matches!(import_private_jwk_err(jwk), AlgorithmError::Data(_)),
        "a partial CRT member set must be a DataError"
    );
}

#[test]
fn jwk_private_inconsistent_d_is_data_error() {
    // n / e / d that do not form a valid RSA key → from_components rejects it.
    let jwk = rsa_jwk(&[("n", "AQID"), ("e", "AQAB"), ("d", "AQID")]);
    assert!(
        matches!(import_private_jwk_err(jwk), AlgorithmError::Data(_)),
        "an inconsistent private key must be a DataError"
    );
}

#[test]
fn jwk_member_not_base64url_is_data_error() {
    // A `+` / `/` (standard base64, not URL-safe) in `n` → decode failure.
    let jwk = rsa_jwk(&[("n", "a+/b"), ("e", "AQAB")]);
    assert!(
        matches!(import_public_jwk_err(jwk), AlgorithmError::Data(_)),
        "non-base64url members are a DataError"
    );
}

#[test]
fn public_exponent_round_trips_byte_identical() {
    // The stored publicExponent is the canonical big-endian 65537 = [1, 0, 1].
    let (public, _private) = generate_pair(
        RsaVariant::RsassaPkcs1V15,
        HashAlgorithm::Sha256,
        vec![KeyUsage::Sign, KeyUsage::Verify],
    );
    let KeyAlgorithm::Rsa {
        public_exponent, ..
    } = &public.algorithm
    else {
        panic!("RSA key");
    };
    assert_eq!(public_exponent, &vec![0x01, 0x00, 0x01]);
}

// ---------------------------------------------------------------------------
// robustness: entropy failure + oversized PSS salt (must not hang / OOM)
// ---------------------------------------------------------------------------

#[test]
fn generate_surfaces_entropy_failure_without_hanging() {
    // When the VM `fill_random` seam errors, generateKey must TERMINATE with
    // the OperationError — not spin RSA prime-search forever (the ClosureRng
    // fallback must vary so `new_with_exp` converges, then `into_result` errors
    // and the key is discarded).  A 512-bit modulus keeps the discarded keygen
    // fast.  (Pre-fix: a constant fallback fill hangs this test.)
    let always_fail = |_: &mut [u8]| Err(AlgorithmError::Operation("entropy unavailable".into()));
    let err = generate_key(
        keygen_alg(RsaVariant::RsassaPkcs1V15, 512, HashAlgorithm::Sha256),
        true,
        vec![KeyUsage::Sign, KeyUsage::Verify],
        always_fail,
    )
    .expect_err("an entropy-seam failure must surface as an error");
    assert!(matches!(err, AlgorithmError::Operation(_)), "got {err:?}");
}

#[test]
fn rsapss_sign_oversized_salt_length_is_rejected_without_oom() {
    // A saltLength larger than the modulus is always an invalid PSS signature
    // (RFC 3447 §9.1.1) and must be rejected BEFORE the rsa crate allocates a
    // salt-sized buffer — an attacker-supplied saltLength = u32::MAX would
    // otherwise allocate / fill ~4 GiB.
    let (_public, private) = generate_pair(
        RsaVariant::RsaPss,
        HashAlgorithm::Sha256,
        vec![KeyUsage::Sign],
    );
    let err = sign(
        sign_alg(RsaVariant::RsaPss, Some(u32::MAX)),
        &private,
        b"m",
        seeded_fill(9),
    )
    .expect_err("an oversized saltLength must be rejected up front");
    assert!(matches!(err, AlgorithmError::Operation(_)), "got {err:?}");
}

// ---------------------------------------------------------------------------
// Codex R1: CRT consistency + empty-oth rejection
// ---------------------------------------------------------------------------

#[test]
fn jwk_inconsistent_crt_member_is_data_error() {
    // A private JWK whose `dp` does not match the value recomputed from p/q/d
    // is malformed key material → DataError (not silently repaired).
    let (_public, private) = generate_pair(
        RsaVariant::RsassaPkcs1V15,
        HashAlgorithm::Sha256,
        vec![KeyUsage::Sign],
    );
    let mut jwk = expect_jwk(export_key(KeyFormat::Jwk, &private).expect("jwk export"));
    // Corrupt `dp` to a clearly-wrong value (65537) while leaving dq/qi intact
    // (the CRT members stay all-present, so the all-or-nothing gate passes and
    // the consistency check is what rejects it).
    jwk.dp = Some("AQAB".to_string());
    let err = import_key(
        KeyFormat::Jwk,
        import_alg(RsaVariant::RsassaPkcs1V15, HashAlgorithm::Sha256),
        true,
        vec![KeyUsage::Sign],
        KeyData::Jwk(Box::new(jwk)),
    )
    .expect_err("an inconsistent CRT member is a DataError");
    assert!(matches!(err, AlgorithmError::Data(_)), "got {err:?}");
}

#[test]
fn jwk_empty_oth_is_not_supported() {
    // A *present* `oth` (even empty `[]`) is an unsupported multi-prime shape
    // (RFC 7518 §6.3.2.7: `oth` MUST be absent for a two-prime key).
    let jwk = JsonWebKey {
        kty: Some("RSA".to_string()),
        d: Some("aaaa".to_string()),
        n: Some("aaaa".to_string()),
        e: Some("AQAB".to_string()),
        oth: Some(vec![]),
        ..Default::default()
    };
    let err = import_key(
        KeyFormat::Jwk,
        import_alg(RsaVariant::RsassaPkcs1V15, HashAlgorithm::Sha256),
        true,
        vec![KeyUsage::Sign],
        KeyData::Jwk(Box::new(jwk)),
    )
    .expect_err("a present empty oth is NotSupported");
    assert!(
        matches!(err, AlgorithmError::NotSupported(_)),
        "got {err:?}"
    );
}
