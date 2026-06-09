//! RSA import / export round-trips (spki / pkcs8 / jwk) and the §20.8.4 /
//! §21.4.4 invalid-shape (DataError / SyntaxError / NotSupported) matrix.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;

use super::*;
use crate::key::KeyType;
use crate::ops::{export_key, sign, verify};

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
fn jwk_multiprime_oth_is_data_error() {
    // A *private* RSA JWK with a non-empty `oth` (multi-prime) is rejected
    // before the key is reconstructed: no browser supports multi-prime RSA and
    // the DER storage cannot encode >2 primes, so the key shape is a DataError
    // (§20.8.4 jwk step 10 / §6.3.2 — matching the `pkcs8` multi-prime path).
    // The reject is scoped to the private branch (see the public-import test
    // below).
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
    .expect_err("multi-prime RSA is a DataError");
    assert!(matches!(err, AlgorithmError::Data(_)), "got {err:?}");
}

#[test]
fn jwk_public_with_oth_member_is_ignored() {
    // A *public* RSA JWK (no `d`) carrying a stray `oth` member must import
    // successfully: WebCrypto interprets a public JWK per RFC 7518 §6.3.1
    // (n / e only — §20.8.4 / §21.4.4 "Otherwise" step), which never references
    // `oth`, so it is ignored exactly as p / q already are.  The multi-prime
    // rejection is private-branch-only (Codex R11).
    let (public, _private) = generate_pair(
        RsaVariant::RsassaPkcs1V15,
        HashAlgorithm::Sha256,
        vec![KeyUsage::Sign, KeyUsage::Verify],
    );
    let mut jwk = expect_jwk(export_key(KeyFormat::Jwk, &public).expect("JWK export"));
    assert!(jwk.d.is_none(), "the exported public JWK must have no `d`");
    jwk.oth = Some(vec![crate::RsaOtherPrimesInfo::default()]);
    let reimported = import_key(
        KeyFormat::Jwk,
        import_alg(RsaVariant::RsassaPkcs1V15, HashAlgorithm::Sha256),
        true,
        vec![KeyUsage::Verify],
        KeyData::Jwk(Box::new(jwk)),
    )
    .expect("a public JWK with a stray `oth` imports (oth ignored)");
    assert_eq!(reimported.key_type, KeyType::Public);
    assert_eq!(reimported.material, public.material);
}

#[test]
fn jwk_oversized_modulus_is_rejected_before_recovery() {
    // A modulus wider than `MAX_RSA_MODULUS_BITS` (4096) must be rejected
    // before the rsa crate validates / recovers — the d-only private path would
    // otherwise run `from_components` prime recovery on attacker-controlled
    // `n` / `e` / `d` (NIST SP 800-56B C.2), an engine DoS (Codex R5). 2200
    // octets = a 17600-bit modulus.
    let big_n = URL_SAFE_NO_PAD.encode(vec![0xFFu8; 2200]);
    // Public JWK (no `d`): rejected before `RsaPublicKey::new`.
    let public = JsonWebKey {
        kty: Some("RSA".to_string()),
        n: Some(big_n.clone()),
        e: Some("AQAB".to_string()),
        ..Default::default()
    };
    let err = import_key(
        KeyFormat::Jwk,
        import_alg(RsaVariant::RsassaPkcs1V15, HashAlgorithm::Sha256),
        true,
        vec![KeyUsage::Verify],
        KeyData::Jwk(Box::new(public)),
    )
    .expect_err("oversized public modulus is NotSupported");
    assert!(
        matches!(err, AlgorithmError::NotSupported(_)),
        "got {err:?}"
    );
    // d-only private JWK: rejected before the `from_components` recovery.
    let private = JsonWebKey {
        kty: Some("RSA".to_string()),
        n: Some(big_n),
        e: Some("AQAB".to_string()),
        d: Some("aaaa".to_string()),
        ..Default::default()
    };
    let err = import_key(
        KeyFormat::Jwk,
        import_alg(RsaVariant::RsassaPkcs1V15, HashAlgorithm::Sha256),
        true,
        vec![KeyUsage::Sign],
        KeyData::Jwk(Box::new(private)),
    )
    .expect_err("oversized d-only modulus is NotSupported");
    assert!(
        matches!(err, AlgorithmError::NotSupported(_)),
        "got {err:?}"
    );
}

#[test]
fn jwk_public_exponent_over_cap_is_not_supported() {
    // A public JWK whose `e` exceeds the rsa crate's `MAX_PUB_EXPONENT`
    // (2^33 − 1) is a NotSupported capability boundary, not the rsa crate's
    // generic DataError (Codex R16).  Real keys use e=65537; this `e` is 2^48−1.
    let jwk = JsonWebKey {
        kty: Some("RSA".to_string()),
        // A small valid modulus so the size cap passes and the exponent check
        // is reached.
        n: Some("AQAB".to_string()),
        e: Some(URL_SAFE_NO_PAD.encode([0xFFu8; 6])),
        ..Default::default()
    };
    let err = import_key(
        KeyFormat::Jwk,
        import_alg(RsaVariant::RsassaPkcs1V15, HashAlgorithm::Sha256),
        true,
        vec![KeyUsage::Verify],
        KeyData::Jwk(Box::new(jwk)),
    )
    .expect_err("a public exponent over the cap is NotSupported");
    assert!(
        matches!(err, AlgorithmError::NotSupported(_)),
        "got {err:?}"
    );
}

#[test]
fn from_json_bytes_oversized_oth_is_rejected() {
    // The `unwrapKey` bytes parser must cap `oth` at `MAX_CRYPTO_SEQUENCE_LEN`,
    // mirroring the live `importKey` marshaller — otherwise a huge `oth` array
    // DoSes the parse before RSA rejects it as multi-prime (Codex R5).
    let entries = vec!["{}"; crate::MAX_CRYPTO_SEQUENCE_LEN + 1].join(",");
    let json = format!(r#"{{"kty":"RSA","n":"AQAB","e":"AQAB","oth":[{entries}]}}"#);
    let err = crate::jwk::from_json_bytes(json.as_bytes()).expect_err("oversized oth is rejected");
    assert!(matches!(err, AlgorithmError::Type(_)), "got {err:?}");
}

#[test]
fn from_json_bytes_oversized_key_ops_is_rejected() {
    // The sibling `key_ops` sequence shares the same cap (the audit half of the
    // oth finding) — the live↔bytes mirror must hold for every JWK sequence.
    let entries = vec![r#""sign""#; crate::MAX_CRYPTO_SEQUENCE_LEN + 1].join(",");
    let json = format!(r#"{{"kty":"oct","k":"AAEC","key_ops":[{entries}]}}"#);
    let err =
        crate::jwk::from_json_bytes(json.as_bytes()).expect_err("oversized key_ops is rejected");
    assert!(matches!(err, AlgorithmError::Type(_)), "got {err:?}");
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
fn jwk_empty_oth_is_data_error() {
    // A *present* `oth` (even empty `[]`) is an unsupported multi-prime shape
    // (RFC 7518 §6.3.2.7: `oth` MUST be absent for a two-prime key) → DataError.
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
    .expect_err("a present empty oth is a DataError");
    assert!(matches!(err, AlgorithmError::Data(_)), "got {err:?}");
}

#[test]
fn jwk_non_minimal_base64urluint_is_data_error() {
    // RFC 7518 §2: Base64urlUInt is the minimal big-endian encoding.  A
    // leading-zero `n` ("AAEAAQ" = [0,1,0,1]) is malformed → DataError.
    let jwk = rsa_jwk(&[("n", "AAEAAQ"), ("e", "AQAB")]);
    assert!(
        matches!(import_public_jwk_err(jwk), AlgorithmError::Data(_)),
        "a non-minimal Base64urlUInt must be a DataError"
    );
}

#[test]
fn jwk_d_only_imports_via_prime_recovery() {
    // A CRT-less (d-only) private JWK with the standard exponent (65537) imports
    // via from_components' prime recovery; it is functionally equivalent (a
    // signature verifies under the original public key).
    let (public, private) = generate_pair(
        RsaVariant::RsassaPkcs1V15,
        HashAlgorithm::Sha256,
        vec![KeyUsage::Sign, KeyUsage::Verify],
    );
    let mut jwk = expect_jwk(export_key(KeyFormat::Jwk, &private).expect("jwk export"));
    jwk.p = None;
    jwk.q = None;
    jwk.dp = None;
    jwk.dq = None;
    jwk.qi = None;
    let recovered = import_key(
        KeyFormat::Jwk,
        import_alg(RsaVariant::RsassaPkcs1V15, HashAlgorithm::Sha256),
        true,
        vec![KeyUsage::Sign],
        KeyData::Jwk(Box::new(jwk)),
    )
    .expect("d-only JWK imports via recovery (e=65537)");
    assert_eq!(recovered.key_type, KeyType::Private);
    let sig = sign(
        sign_alg(RsaVariant::RsassaPkcs1V15, None),
        &recovered,
        b"d-only",
        seeded_fill(11),
    )
    .expect("sign with the recovered key");
    assert!(verify(
        verify_alg(RsaVariant::RsassaPkcs1V15, None),
        &public,
        &sig,
        b"d-only"
    )
    .unwrap());
}
