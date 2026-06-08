//! EC (ECDSA / ECDH) crate-level tests (WebCrypto §23 / §24): the import /
//! export round-trips across the four formats over P-256 / P-384 / P-521, the
//! generateKey usage-split, ECDSA sign / verify, ECDH deriveBits / deriveKey,
//! and the §23.7.4 / §24.4.3 invalid-shape (DataError / SyntaxError) set.
//!
//! The JS-level vertical (the Promise / marshalling surface) lives in the VM
//! crate's `tests_crypto::ec`; the `marshal_jwk` ≡ `from_json_bytes` JWK
//! mirror differential test lives in `subtle_crypto::differential`.

use super::{fill_seq, no_rng};
use crate::algorithm::{EcAlgorithm, EcdhPeer, NamedCurve};
use crate::key::{KeyAlgorithm, KeyType, KeyUsage};
use crate::ops::{
    derive_bits, derive_key, export_key, generate_key, import_key, sign, verify, ExportedKey,
    GeneratedKey, KeyData, KeyFormat,
};
use crate::{normalize, CryptoKeyData, JsonWebKey, NormalizedAlgorithm, Operation};

// RFC 7515 Appendix A.3.1 — the ES256 (P-256) example key (x / y / d are the
// base64url coordinates / scalar).
const P256_X: &str = "f83OJ3D2xF1Bg8vub9tLe1gHMzV76e8Tus9uPHvRVEU";
const P256_Y: &str = "x_FEzRu9m36HLN_tue659LNpXW6pCyStikYjKIWI5a0";
const P256_D: &str = "jpsQnnGQmL-YBIffH1136cspYG6-0iY7X1fCE9-E9LI";

fn ec_alg(op: Operation, algorithm: EcAlgorithm, curve: NamedCurve) -> NormalizedAlgorithm {
    let name = match algorithm {
        EcAlgorithm::Ecdsa => "ECDSA",
        EcAlgorithm::Ecdh => "ECDH",
    };
    let mut raw = crate::RawAlgorithm::from_name(name);
    raw.named_curve = Some(curve.as_str().to_string());
    normalize(op, raw).expect("EC algorithm normalizes")
}

fn ec_import_alg(algorithm: EcAlgorithm, curve: NamedCurve) -> NormalizedAlgorithm {
    ec_alg(Operation::ImportKey, algorithm, curve)
}

fn private_jwk() -> JsonWebKey {
    JsonWebKey {
        kty: Some("EC".to_string()),
        crv: Some("P-256".to_string()),
        x: Some(P256_X.to_string()),
        y: Some(P256_Y.to_string()),
        d: Some(P256_D.to_string()),
        ..Default::default()
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
fn ecdsa_jwk_private_import_export_round_trip() {
    let alg = ec_import_alg(EcAlgorithm::Ecdsa, NamedCurve::P256);
    let key = import_key(
        KeyFormat::Jwk,
        alg,
        true,
        vec![KeyUsage::Sign],
        KeyData::Jwk(private_jwk()),
    )
    .expect("private EC JWK imports");
    assert_eq!(key.key_type, KeyType::Private);

    let jwk = expect_jwk(export_key(KeyFormat::Jwk, &key).expect("JWK export"));
    assert_eq!(jwk.kty.as_deref(), Some("EC"));
    assert_eq!(jwk.crv.as_deref(), Some("P-256"));
    assert_eq!(jwk.x.as_deref(), Some(P256_X));
    assert_eq!(jwk.y.as_deref(), Some(P256_Y));
    assert_eq!(jwk.d.as_deref(), Some(P256_D));
    assert_eq!(jwk.ext, Some(true));
    assert_eq!(jwk.key_ops.as_deref(), Some(&["sign".to_string()][..]));
    // No `oct` members on an EC key.
    assert!(jwk.k.is_none() && jwk.alg.is_none());
}

#[test]
fn ecdsa_private_pkcs8_round_trip() {
    let key = import_key(
        KeyFormat::Jwk,
        ec_import_alg(EcAlgorithm::Ecdsa, NamedCurve::P256),
        true,
        vec![KeyUsage::Sign],
        KeyData::Jwk(private_jwk()),
    )
    .unwrap();

    // Export PKCS#8, then re-import — the recovered key matches.
    let der = expect_raw(export_key(KeyFormat::Pkcs8, &key).expect("PKCS#8 export"));
    let reimported = import_key(
        KeyFormat::Pkcs8,
        ec_import_alg(EcAlgorithm::Ecdsa, NamedCurve::P256),
        true,
        vec![KeyUsage::Sign],
        KeyData::Raw(der),
    )
    .expect("PKCS#8 re-import");
    assert_eq!(reimported.key_type, KeyType::Private);
    assert_eq!(reimported.material, key.material);
}

#[test]
fn ecdsa_public_raw_and_spki_round_trip() {
    // Import the private key, export its public point as `raw`, re-import as a
    // public verify key, then round-trip through SPKI.
    let private = import_key(
        KeyFormat::Jwk,
        ec_import_alg(EcAlgorithm::Ecdsa, NamedCurve::P256),
        true,
        vec![KeyUsage::Sign],
        KeyData::Jwk(private_jwk()),
    )
    .unwrap();
    // A private key cannot export `raw` (public-only) → InvalidAccessError.
    assert!(export_key(KeyFormat::Raw, &private).is_err());

    // Build the public JWK (drop `d`) and import as a verify key.
    let mut public_jwk = private_jwk();
    public_jwk.d = None;
    let public = import_key(
        KeyFormat::Jwk,
        ec_import_alg(EcAlgorithm::Ecdsa, NamedCurve::P256),
        true,
        vec![KeyUsage::Verify],
        KeyData::Jwk(public_jwk),
    )
    .unwrap();
    assert_eq!(public.key_type, KeyType::Public);

    let raw = expect_raw(export_key(KeyFormat::Raw, &public).expect("raw export"));
    assert_eq!(raw.first(), Some(&0x04)); // uncompressed SEC1
    assert_eq!(raw.len(), 1 + 2 * NamedCurve::P256.coordinate_len());

    let from_raw = import_key(
        KeyFormat::Raw,
        ec_import_alg(EcAlgorithm::Ecdsa, NamedCurve::P256),
        true,
        vec![KeyUsage::Verify],
        KeyData::Raw(raw),
    )
    .expect("raw re-import");
    assert_eq!(from_raw.material, public.material);

    let spki = expect_raw(export_key(KeyFormat::Spki, &public).expect("SPKI export"));
    let from_spki = import_key(
        KeyFormat::Spki,
        ec_import_alg(EcAlgorithm::Ecdsa, NamedCurve::P256),
        true,
        vec![KeyUsage::Verify],
        KeyData::Raw(spki),
    )
    .expect("SPKI re-import");
    assert_eq!(from_spki.material, public.material);
}

#[test]
fn ecdh_public_import_requires_empty_usages() {
    // ECDH public keys take no usages (§24.4.3 jwk step 3 SyntaxError).
    let mut public_jwk = private_jwk();
    public_jwk.d = None;
    let err = import_key(
        KeyFormat::Jwk,
        ec_import_alg(EcAlgorithm::Ecdh, NamedCurve::P256),
        true,
        vec![KeyUsage::DeriveBits],
        KeyData::Jwk(public_jwk.clone()),
    )
    .unwrap_err();
    assert!(matches!(err, crate::AlgorithmError::Syntax(_)));

    // With empty usages it imports.
    let ok = import_key(
        KeyFormat::Jwk,
        ec_import_alg(EcAlgorithm::Ecdh, NamedCurve::P256),
        true,
        vec![],
        KeyData::Jwk(public_jwk),
    )
    .expect("ECDH public JWK with empty usages imports");
    assert_eq!(ok.key_type, KeyType::Public);
}

fn expect_pair(generated: GeneratedKey) -> (crate::CryptoKeyData, crate::CryptoKeyData) {
    match generated {
        GeneratedKey::Pair { public, private } => (public, private),
        GeneratedKey::Single(_) => panic!("EC generateKey yields a CryptoKeyPair"),
    }
}

#[test]
fn ecdsa_generate_key_splits_usages_and_extractable() {
    let (public, private) = expect_pair(
        generate_key(
            ec_alg(Operation::GenerateKey, EcAlgorithm::Ecdsa, NamedCurve::P256),
            false, // private key extractable = false
            vec![KeyUsage::Sign, KeyUsage::Verify],
            fill_seq,
        )
        .expect("ECDSA keygen"),
    );
    // public: usages ∩ {verify}; always extractable.
    assert_eq!(public.key_type, KeyType::Public);
    assert!(public.extractable);
    assert_eq!(public.usages, vec![KeyUsage::Verify]);
    // private: usages ∩ {sign}; extractable = requested (false).
    assert_eq!(private.key_type, KeyType::Private);
    assert!(!private.extractable);
    assert_eq!(private.usages, vec![KeyUsage::Sign]);
    // Both halves share the public point; only the private carries the scalar.
    assert_eq!(
        public.material.ec_public_point(),
        private.material.ec_public_point()
    );
    assert!(private.material.ec_private_scalar().is_some());
    assert!(public.material.ec_private_scalar().is_none());
}

#[test]
fn ecdh_generate_key_public_has_no_usages() {
    let (public, private) = expect_pair(
        generate_key(
            ec_alg(Operation::GenerateKey, EcAlgorithm::Ecdh, NamedCurve::P384),
            true,
            vec![KeyUsage::DeriveBits, KeyUsage::DeriveKey],
            fill_seq,
        )
        .expect("ECDH keygen"),
    );
    assert!(public.usages.is_empty()); // §24.4.1 step 11: empty list
    assert!(public.extractable);
    assert_eq!(
        private.usages,
        vec![KeyUsage::DeriveKey, KeyUsage::DeriveBits]
    );
    assert!(private.extractable);
    // P-384 public point = 0x04 ‖ x ‖ y = 1 + 2·48 bytes.
    assert_eq!(
        public.material.ec_public_point().unwrap().len(),
        1 + 2 * NamedCurve::P384.coordinate_len()
    );
}

#[test]
fn ecdsa_generate_verify_only_leaves_private_empty_is_syntax_error() {
    // usages = [verify] → private ∩ {sign} = empty → §14.3.6 SyntaxError.
    let err = generate_key(
        ec_alg(Operation::GenerateKey, EcAlgorithm::Ecdsa, NamedCurve::P256),
        true,
        vec![KeyUsage::Verify],
        fill_seq,
    )
    .unwrap_err();
    assert!(matches!(err, crate::AlgorithmError::Syntax(_)));
}

#[test]
fn ec_generate_all_curves_round_trip_through_pkcs8() {
    for curve in [NamedCurve::P256, NamedCurve::P384, NamedCurve::P521] {
        let (_public, private) = expect_pair(
            generate_key(
                ec_alg(Operation::GenerateKey, EcAlgorithm::Ecdsa, curve),
                true,
                vec![KeyUsage::Sign],
                fill_seq,
            )
            .expect("keygen"),
        );
        let der = match export_key(KeyFormat::Pkcs8, &private).expect("PKCS#8 export") {
            ExportedKey::Raw(bytes) => bytes,
            ExportedKey::Jwk(_) => panic!("PKCS#8 export is DER bytes"),
        };
        let reimported = import_key(
            KeyFormat::Pkcs8,
            ec_import_alg(EcAlgorithm::Ecdsa, curve),
            true,
            vec![KeyUsage::Sign],
            KeyData::Raw(der),
        )
        .expect("PKCS#8 re-import");
        assert_eq!(reimported.material, private.material);
    }
}

fn ecdsa_params_alg(hash: &str) -> NormalizedAlgorithm {
    let mut raw = crate::RawAlgorithm::from_name("ECDSA");
    raw.hash = Some(Box::new(crate::RawAlgorithm::from_name(hash)));
    normalize(Operation::Sign, raw).expect("EcdsaParams normalizes")
}

#[test]
fn ecdsa_sign_verify_round_trip_all_curves() {
    // (curve, signature hash, raw r‖s length).
    for (curve, hash, sig_len) in [
        (NamedCurve::P256, "SHA-256", 64),
        (NamedCurve::P384, "SHA-384", 96),
        (NamedCurve::P521, "SHA-512", 132),
    ] {
        let (public, private) = expect_pair(
            generate_key(
                ec_alg(Operation::GenerateKey, EcAlgorithm::Ecdsa, curve),
                true,
                vec![KeyUsage::Sign, KeyUsage::Verify],
                fill_seq,
            )
            .expect("keygen"),
        );
        let msg = b"the quick brown fox jumps over the lazy dog";
        let sig = sign(ecdsa_params_alg(hash), &private, msg, no_rng).expect("sign");
        assert_eq!(sig.len(), sig_len, "raw r‖s length for {}", curve.as_str());
        // A genuine signature verifies.
        assert!(verify(ecdsa_params_alg(hash), &public, &sig, msg).unwrap());
        // A tampered message does not (returns false, not an error).
        assert!(!verify(ecdsa_params_alg(hash), &public, &sig, b"other message").unwrap());
        // A wrong-length signature returns false (§23.7.2 step 6.2), not error.
        assert!(!verify(ecdsa_params_alg(hash), &public, &sig[..sig_len - 1], msg).unwrap());
    }
}

#[test]
fn ecdsa_sign_with_public_key_is_invalid_access() {
    let (public, _private) = expect_pair(
        generate_key(
            ec_alg(Operation::GenerateKey, EcAlgorithm::Ecdsa, NamedCurve::P256),
            true,
            vec![KeyUsage::Sign, KeyUsage::Verify],
            fill_seq,
        )
        .unwrap(),
    );
    // The public key lacks the `sign` usage → InvalidAccessError at the gate.
    let err = sign(ecdsa_params_alg("SHA-256"), &public, b"m", no_rng).unwrap_err();
    assert!(matches!(err, crate::AlgorithmError::InvalidAccess(_)));
}

#[test]
fn ecdsa_sign_hash_mismatch_still_verifies_with_same_hash() {
    // The signature hash comes from the call params, not the key, so signing
    // with SHA-384 and verifying with SHA-384 round-trips on a P-256 key
    // (curve and hash are independent).
    let (public, private) = expect_pair(
        generate_key(
            ec_alg(Operation::GenerateKey, EcAlgorithm::Ecdsa, NamedCurve::P256),
            true,
            vec![KeyUsage::Sign, KeyUsage::Verify],
            fill_seq,
        )
        .unwrap(),
    );
    let sig = sign(ecdsa_params_alg("SHA-384"), &private, b"data", no_rng).unwrap();
    assert!(verify(ecdsa_params_alg("SHA-384"), &public, &sig, b"data").unwrap());
    // Verifying the SHA-384 signature under SHA-256 must fail.
    assert!(!verify(ecdsa_params_alg("SHA-256"), &public, &sig, b"data").unwrap());
}

// A second deterministic "random" fill, distinct from `fill_seq`, so two
// generated EC key pairs differ (a genuine A ≠ B for the ECDH symmetry test).
// (Same lints as `fill_seq`: the `fill_random` closure signature forces the
// `Result`, and the `i as u8` deterministic fill is an intentional truncation.)
#[allow(clippy::unnecessary_wraps, clippy::cast_possible_truncation)]
fn fill_offset(buf: &mut [u8]) -> Result<(), crate::error::AlgorithmError> {
    for (i, b) in buf.iter_mut().enumerate() {
        *b = (i as u8).wrapping_add(0x40);
    }
    Ok(())
}

fn ecdh_peer(key: &CryptoKeyData) -> EcdhPeer {
    EcdhPeer {
        key_type: key.key_type,
        algorithm: key.algorithm.name(),
        curve: key.algorithm.named_curve(),
        public_point: key.material.ec_public_point().map(<[u8]>::to_vec),
    }
}

fn ecdh_keypair<F>(curve: NamedCurve, fill: F) -> (CryptoKeyData, CryptoKeyData)
where
    F: FnMut(&mut [u8]) -> Result<(), crate::error::AlgorithmError>,
{
    expect_pair(
        generate_key(
            ec_alg(Operation::GenerateKey, EcAlgorithm::Ecdh, curve),
            true,
            vec![KeyUsage::DeriveBits, KeyUsage::DeriveKey],
            fill,
        )
        .expect("ECDH keygen"),
    )
}

#[test]
fn ecdh_derive_bits_is_symmetric() {
    let (a_pub, a_priv) = ecdh_keypair(NamedCurve::P256, fill_seq);
    let (b_pub, b_priv) = ecdh_keypair(NamedCurve::P256, fill_offset);
    // The two pairs must differ for a meaningful symmetry test.
    assert_ne!(a_pub.material, b_pub.material);

    let ab = derive_bits(
        NormalizedAlgorithm::EcdhDerive {
            peer: ecdh_peer(&b_pub),
        },
        &a_priv,
        None,
    )
    .expect("A.priv × B.pub");
    let ba = derive_bits(
        NormalizedAlgorithm::EcdhDerive {
            peer: ecdh_peer(&a_pub),
        },
        &b_priv,
        None,
    )
    .expect("B.priv × A.pub");
    // ECDH is symmetric, and the secret is the X coordinate (coordinate_len).
    assert_eq!(ab, ba);
    assert_eq!(ab.len(), NamedCurve::P256.coordinate_len());
}

#[test]
fn ecdh_derive_bits_length_truncation() {
    let (a_pub, _) = ecdh_keypair(NamedCurve::P256, fill_seq);
    let (_, b_priv) = ecdh_keypair(NamedCurve::P256, fill_offset);
    let peer = ecdh_peer(&a_pub);
    let full = derive_bits(
        NormalizedAlgorithm::EcdhDerive { peer: peer.clone() },
        &b_priv,
        None,
    )
    .unwrap();
    // First 128 bits = the first 16 bytes of the full secret.
    let short = derive_bits(
        NormalizedAlgorithm::EcdhDerive { peer: peer.clone() },
        &b_priv,
        Some(128),
    )
    .unwrap();
    assert_eq!(short, full[..16]);
    // length > secret bit length → OperationError (P-256 secret is 256 bits).
    let err =
        derive_bits(NormalizedAlgorithm::EcdhDerive { peer }, &b_priv, Some(264)).unwrap_err();
    assert!(matches!(err, crate::AlgorithmError::Operation(_)));
}

#[test]
fn ecdh_derive_bits_peer_mismatches_are_invalid_access() {
    let (_, base_priv) = ecdh_keypair(NamedCurve::P256, fill_seq);
    let (good_pub, _) = ecdh_keypair(NamedCurve::P256, fill_offset);
    let (wrong_curve_pub, _) = ecdh_keypair(NamedCurve::P384, fill_offset);

    // Peer on a different curve (§24.4.2 step 5).
    let err = derive_bits(
        NormalizedAlgorithm::EcdhDerive {
            peer: ecdh_peer(&wrong_curve_pub),
        },
        &base_priv,
        None,
    )
    .unwrap_err();
    assert!(matches!(err, crate::AlgorithmError::InvalidAccess(_)));

    // Peer with the wrong algorithm name — ECDSA (§24.4.2 step 4).
    let mut ecdsa_peer = ecdh_peer(&good_pub);
    ecdsa_peer.algorithm = crate::AlgorithmName::Ecdsa;
    let err = derive_bits(
        NormalizedAlgorithm::EcdhDerive { peer: ecdsa_peer },
        &base_priv,
        None,
    )
    .unwrap_err();
    assert!(matches!(err, crate::AlgorithmError::InvalidAccess(_)));

    // Peer that is a private (not public) key (§24.4.2 step 3).
    let mut private_peer = ecdh_peer(&good_pub);
    private_peer.key_type = KeyType::Private;
    let err = derive_bits(
        NormalizedAlgorithm::EcdhDerive { peer: private_peer },
        &base_priv,
        None,
    )
    .unwrap_err();
    assert!(matches!(err, crate::AlgorithmError::InvalidAccess(_)));
}

fn aes_import_alg() -> NormalizedAlgorithm {
    normalize(
        Operation::ImportKey,
        crate::RawAlgorithm::from_name("AES-GCM"),
    )
    .expect("AES import normalizes")
}

fn aes_get_key_length_alg(bits: u32) -> NormalizedAlgorithm {
    let mut raw = crate::RawAlgorithm::from_name("AES-GCM");
    raw.length = Some(bits);
    normalize(Operation::GetKeyLength, raw).expect("AES get-key-length normalizes")
}

#[test]
fn ecdh_derive_key_to_aes_gcm() {
    let (a_pub, _) = ecdh_keypair(NamedCurve::P256, fill_seq);
    let (_, b_priv) = ecdh_keypair(NamedCurve::P256, fill_offset);
    // §14.3.7 deriveKey: ECDH base + an AES-GCM derivedKeyType → a 256-bit AES
    // key (P-256 secret is exactly 256 bits).
    let derived = derive_key(
        NormalizedAlgorithm::EcdhDerive {
            peer: ecdh_peer(&a_pub),
        },
        &b_priv,
        aes_import_alg(),
        aes_get_key_length_alg(256),
        true,
        vec![KeyUsage::Encrypt, KeyUsage::Decrypt],
    )
    .expect("ECDH deriveKey → AES-GCM");
    assert_eq!(derived.key_type, KeyType::Secret);
    assert!(matches!(
        derived.algorithm,
        KeyAlgorithm::Aes { length: 256, .. }
    ));
}

// --- invalid-shape matrix (the §23.7.4 / §24.4.3 DataError / SyntaxError sets) ---

fn import_ecdsa_jwk(jwk: JsonWebKey, usages: Vec<KeyUsage>) -> crate::error::AlgorithmError {
    import_key(
        KeyFormat::Jwk,
        ec_import_alg(EcAlgorithm::Ecdsa, NamedCurve::P256),
        true,
        usages,
        KeyData::Jwk(jwk),
    )
    .expect_err("import should fail")
}

#[test]
fn ec_jwk_invalid_shapes_are_data_errors() {
    use crate::error::AlgorithmError::Data;
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};

    // kty not "EC".
    let mut jwk = private_jwk();
    jwk.kty = Some("oct".to_string());
    assert!(matches!(
        import_ecdsa_jwk(jwk, vec![KeyUsage::Sign]),
        Data(_)
    ));

    // x not valid base64url.
    let mut jwk = private_jwk();
    jwk.x = Some("not base64!!".to_string());
    assert!(matches!(
        import_ecdsa_jwk(jwk, vec![KeyUsage::Sign]),
        Data(_)
    ));

    // x the wrong length for the curve (31 bytes ≠ 32).
    let mut jwk = private_jwk();
    jwk.x = Some(URL_SAFE_NO_PAD.encode([0u8; 31]));
    assert!(matches!(
        import_ecdsa_jwk(jwk, vec![KeyUsage::Sign]),
        Data(_)
    ));

    // d present but inconsistent with x / y (a different valid scalar).
    let mut jwk = private_jwk();
    jwk.d = Some(URL_SAFE_NO_PAD.encode([0x42u8; 32]));
    assert!(matches!(
        import_ecdsa_jwk(jwk, vec![KeyUsage::Sign]),
        Data(_)
    ));

    // ECDSA `alg` mismatched with the curve (ES384 on a P-256 key).
    let mut jwk = private_jwk();
    jwk.alg = Some("ES384".to_string());
    assert!(matches!(
        import_ecdsa_jwk(jwk, vec![KeyUsage::Sign]),
        Data(_)
    ));
}

#[test]
fn ecdsa_jwk_private_with_verify_usage_is_syntax_error() {
    // §23.7.4 jwk step 2: `d` present + a usage other than "sign" → SyntaxError.
    let err = import_ecdsa_jwk(private_jwk(), vec![KeyUsage::Verify]);
    assert!(matches!(err, crate::error::AlgorithmError::Syntax(_)));
}

#[test]
fn ec_import_garbage_der_and_point_are_data_errors() {
    use crate::error::AlgorithmError::Data;
    let garbage = vec![0x01, 0x02, 0x03, 0x04];
    // raw: not a valid SEC1 point.
    assert!(matches!(
        import_key(
            KeyFormat::Raw,
            ec_import_alg(EcAlgorithm::Ecdsa, NamedCurve::P256),
            true,
            vec![KeyUsage::Verify],
            KeyData::Raw(garbage.clone()),
        )
        .unwrap_err(),
        Data(_)
    ));
    // spki: not a valid SubjectPublicKeyInfo.
    assert!(matches!(
        import_key(
            KeyFormat::Spki,
            ec_import_alg(EcAlgorithm::Ecdsa, NamedCurve::P256),
            true,
            vec![KeyUsage::Verify],
            KeyData::Raw(garbage.clone()),
        )
        .unwrap_err(),
        Data(_)
    ));
    // pkcs8: not a valid PrivateKeyInfo.
    assert!(matches!(
        import_key(
            KeyFormat::Pkcs8,
            ec_import_alg(EcAlgorithm::Ecdsa, NamedCurve::P256),
            true,
            vec![KeyUsage::Sign],
            KeyData::Raw(garbage),
        )
        .unwrap_err(),
        Data(_)
    ));
}

#[test]
fn ec_import_curve_mismatch_is_data_error() {
    // The JWK declares P-256 but the algorithm asks for P-384 → DataError.
    let err = import_key(
        KeyFormat::Jwk,
        ec_import_alg(EcAlgorithm::Ecdsa, NamedCurve::P384),
        true,
        vec![KeyUsage::Sign],
        KeyData::Jwk(private_jwk()),
    )
    .unwrap_err();
    assert!(matches!(err, crate::AlgorithmError::Data(_)));
}
