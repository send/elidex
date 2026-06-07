//! EC (ECDSA / ECDH) import / export round-trip tests (WebCrypto §23 / §24).
//!
//! The comprehensive per-format / per-curve matrix + the invalid-shape set +
//! the JWK mirror differential test land in PR-4 commit 6; these are the
//! backend smoke tests proving import → export round-trips across the four
//! formats over a known P-256 vector.

use crate::algorithm::{EcAlgorithm, NamedCurve};
use crate::key::{KeyType, KeyUsage};
use crate::ops::{export_key, import_key, ExportedKey, KeyData, KeyFormat};
use crate::{normalize, JsonWebKey, NormalizedAlgorithm, Operation};

// RFC 7515 Appendix A.3.1 — the ES256 (P-256) example key (x / y / d are the
// base64url coordinates / scalar).
const P256_X: &str = "f83OJ3D2xF1Bg8vub9tLe1gHMzV76e8Tus9uPHvRVEU";
const P256_Y: &str = "x_FEzRu9m36HLN_tue659LNpXW6pCyStikYjKIWI5a0";
const P256_D: &str = "jpsQnnGQmL-YBIffH1136cspYG6-0iY7X1fCE9-E9LI";

fn ec_import_alg(algorithm: EcAlgorithm, curve: NamedCurve) -> NormalizedAlgorithm {
    let name = match algorithm {
        EcAlgorithm::Ecdsa => "ECDSA",
        EcAlgorithm::Ecdh => "ECDH",
    };
    let mut raw = crate::RawAlgorithm::from_name(name);
    raw.named_curve = Some(curve.as_str().to_string());
    normalize(Operation::ImportKey, raw).expect("EC import algorithm normalizes")
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
