// ---------------------------------------------------------------------------
// ops: generate / sign / verify round-trip
// ---------------------------------------------------------------------------

use super::to_hex;
use crate::algorithm::NormalizedAlgorithm;
use crate::error::AlgorithmError;
use crate::hash::{self, HashAlgorithm};
use crate::jwk::JsonWebKey;
use crate::key::{KeyType, KeyUsage};
use crate::ops::{self, ExportedKey, KeyData, KeyFormat};

fn hmac_keygen_alg(hash: HashAlgorithm, length: Option<u32>) -> NormalizedAlgorithm {
    NormalizedAlgorithm::HmacKeyParams { hash, length }
}

#[test]
fn ops_generate_sign_verify_roundtrip() {
    let alg = hmac_keygen_alg(HashAlgorithm::Sha256, None);
    // deterministic "random": fill the crate-sized buffer with 0x42.
    let key = super::expect_single(ops::generate_key(
        alg,
        true,
        vec![KeyUsage::Sign, KeyUsage::Verify],
        |buf| {
            buf.fill(0x42);
            Ok(())
        },
    ));
    assert_eq!(key.key_type, KeyType::Secret);
    assert_eq!(key.material.as_bytes().len(), 64); // SHA-256 block size

    let sig = ops::sign(NormalizedAlgorithm::Hmac, &key, b"message").unwrap();
    assert!(ops::verify(NormalizedAlgorithm::Hmac, &key, &sig, b"message").unwrap());
    assert!(!ops::verify(NormalizedAlgorithm::Hmac, &key, &sig, b"tampered").unwrap());
}

#[test]
fn ops_generate_invalid_usage_kind_beats_length_error() {
    // HjuLU/Hlnbh / §31.6.3 step 1: a non-sign/verify usage is a
    // SyntaxError *before* the step-2 length handling — so `length: 0`
    // (which would otherwise be an OperationError) does not pre-empt it,
    // and the CSPRNG fill closure is never invoked.
    let alg = hmac_keygen_alg(HashAlgorithm::Sha256, Some(0));
    let mut filled = false;
    let result = ops::generate_key(alg, true, vec![KeyUsage::Encrypt], |_buf| {
        filled = true;
        Ok(())
    });
    assert!(matches!(result, Err(AlgorithmError::Syntax(_))));
    assert!(
        !filled,
        "fill closure must not run when usage validation fails"
    );
}

#[test]
fn ops_generate_empty_usages_is_syntax_error() {
    let alg = hmac_keygen_alg(HashAlgorithm::Sha256, None);
    assert!(matches!(
        ops::generate_key(alg, true, vec![], |_buf| Ok(())),
        Err(AlgorithmError::Syntax(_))
    ));
}

#[test]
fn ops_generate_invalid_usage_is_syntax_error() {
    let alg = hmac_keygen_alg(HashAlgorithm::Sha256, None);
    assert!(matches!(
        ops::generate_key(alg, true, vec![KeyUsage::Encrypt], |_buf| Ok(())),
        Err(AlgorithmError::Syntax(_))
    ));
}

#[test]
fn ops_import_valid_material_empty_usages_is_syntax_error() {
    // §14.3.9 step 10: a secret key with empty usages is a SyntaxError —
    // still raised once the (valid) material has been accepted.
    let alg = hmac_keygen_alg(HashAlgorithm::Sha256, None);
    assert!(matches!(
        ops::import_key(
            KeyFormat::Raw,
            alg,
            true,
            vec![],
            KeyData::Raw(vec![0x0b; 20])
        ),
        Err(AlgorithmError::Syntax(_))
    ));
}

#[test]
fn ops_import_empty_material_empty_usages_is_data_error() {
    // HjRqA: §31.6.4 empty-material DataError is validated *before* the
    // §14.3.9 empty-usages SyntaxError, so invalid material + empty usages
    // surfaces DataError (the material problem), not SyntaxError.
    let alg = hmac_keygen_alg(HashAlgorithm::Sha256, None);
    assert!(matches!(
        ops::import_key(KeyFormat::Raw, alg, true, vec![], KeyData::Raw(vec![])),
        Err(AlgorithmError::Data(_))
    ));
}

#[test]
fn ops_import_invalid_usage_kind_beats_material() {
    // §31.6.4 step 2: a non-sign/verify usage is a SyntaxError raised
    // before key material is parsed (so it wins over an empty-material
    // DataError).
    let alg = hmac_keygen_alg(HashAlgorithm::Sha256, None);
    assert!(matches!(
        ops::import_key(
            KeyFormat::Raw,
            alg,
            true,
            vec![KeyUsage::Encrypt],
            KeyData::Raw(vec![]),
        ),
        Err(AlgorithmError::Syntax(_))
    ));
}

// ---------------------------------------------------------------------------
// ops: import raw + length range (F3) + length-as-metadata (F4 reversed)
// ---------------------------------------------------------------------------

#[test]
fn ops_import_raw_sign_matches_vector() {
    let alg = hmac_keygen_alg(HashAlgorithm::Sha256, None);
    let key = ops::import_key(
        KeyFormat::Raw,
        alg,
        true,
        vec![KeyUsage::Sign],
        KeyData::Raw(vec![0x0b; 20]),
    )
    .unwrap();
    let sig = ops::sign(NormalizedAlgorithm::Hmac, &key, b"Hi There").unwrap();
    assert_eq!(
        to_hex(&sig),
        "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
    );
}

#[test]
fn ops_import_raw_length_masks_trailing_bits() {
    // 4-byte (32-bit) key with length=28 → §31.6.4 step 8 "the first 28
    // bits of data": the final octet's low 4 bits are zeroed (0xFF→0xF0),
    // algorithm.length=28; export returns the masked material.
    let alg = hmac_keygen_alg(HashAlgorithm::Sha256, Some(28));
    let key = ops::import_key(
        KeyFormat::Raw,
        alg,
        true,
        vec![KeyUsage::Sign],
        KeyData::Raw(vec![0xFF, 0xFF, 0xFF, 0xFF]),
    )
    .unwrap();
    assert_eq!(key.material.as_bytes(), &[0xFF, 0xFF, 0xFF, 0xF0]);
    let ExportedKey::Raw(out) = ops::export_key(KeyFormat::Raw, &key).unwrap() else {
        panic!("expected raw export");
    };
    assert_eq!(out, vec![0xFF, 0xFF, 0xFF, 0xF0]);
}

#[test]
fn ops_generate_sub_byte_length_masks_trailing_bits() {
    // §31.6.3 step 3 "key of length length bits": generateKey with
    // length=1 keeps only the top bit of the single CSPRNG octet.
    let alg = hmac_keygen_alg(HashAlgorithm::Sha256, Some(1));
    let key = super::expect_single(ops::generate_key(alg, true, vec![KeyUsage::Sign], |buf| {
        buf.fill(0xFF);
        Ok(())
    }));
    assert_eq!(key.material.as_bytes(), &[0x80]);
    let crate::key::KeyAlgorithm::Hmac { length, .. } = key.algorithm else {
        panic!("expected an HMAC key algorithm");
    };
    assert_eq!(length, 1);
}

fn import_raw_len(len: u32) -> Result<crate::key::CryptoKeyData, AlgorithmError> {
    ops::import_key(
        KeyFormat::Raw,
        hmac_keygen_alg(HashAlgorithm::Sha256, Some(len)),
        true,
        vec![KeyUsage::Sign],
        KeyData::Raw(vec![0; 4]),
    )
}

#[test]
fn ops_import_raw_length_range() {
    // 4-byte key = 32 data bits; accept (24, 32], reject otherwise (F3).
    for ok in [25u32, 28, 31, 32] {
        assert!(import_raw_len(ok).is_ok(), "length {ok} should be accepted");
    }
    for bad in [0u32, 24, 33, 1000] {
        assert!(
            matches!(import_raw_len(bad), Err(AlgorithmError::Data(_))),
            "length {bad} should be DataError"
        );
    }
}

#[test]
fn ops_import_normalizes_usages_dedup_and_order() {
    // Codex #7: ['verify','sign','sign'] -> dedup + canonical order
    // [sign, verify] (WebCrypto normalize usages).
    let key = ops::import_key(
        KeyFormat::Raw,
        hmac_keygen_alg(HashAlgorithm::Sha256, None),
        true,
        vec![KeyUsage::Verify, KeyUsage::Sign, KeyUsage::Sign],
        KeyData::Raw(vec![0x0b; 20]),
    )
    .unwrap();
    assert_eq!(key.usages, vec![KeyUsage::Sign, KeyUsage::Verify]);
}

#[test]
fn ops_import_unsupported_format() {
    assert!(matches!(
        ops::import_key(
            KeyFormat::Pkcs8,
            hmac_keygen_alg(HashAlgorithm::Sha256, None),
            true,
            vec![KeyUsage::Sign],
            KeyData::Raw(vec![1, 2, 3]),
        ),
        Err(AlgorithmError::NotSupported(_))
    ));
}

#[test]
fn ops_import_empty_material_is_data_error() {
    // WebCrypto §31.6.4: zero-length HMAC import material → DataError
    // (the shared "if length is zero throw a DataError" step), for both
    // raw and jwk.
    assert!(matches!(
        import_raw_len_data(vec![]),
        Err(AlgorithmError::Data(_))
    ));
    let jwk = JsonWebKey {
        kty: Some("oct".into()),
        k: Some(String::new()), // base64url "" → empty material
        alg: Some("HS256".into()),
        use_: None,
        key_ops: None,
        ext: Some(true),
        ..Default::default()
    };
    assert!(matches!(
        ops::import_key(
            KeyFormat::Jwk,
            hmac_keygen_alg(HashAlgorithm::Sha256, None),
            true,
            vec![KeyUsage::Sign],
            KeyData::Jwk(jwk),
        ),
        Err(AlgorithmError::Data(_))
    ));
}

fn import_raw_len_data(bytes: Vec<u8>) -> Result<crate::key::CryptoKeyData, AlgorithmError> {
    ops::import_key(
        KeyFormat::Raw,
        hmac_keygen_alg(HashAlgorithm::Sha256, None),
        true,
        vec![KeyUsage::Sign],
        KeyData::Raw(bytes),
    )
}

// ---------------------------------------------------------------------------
// ops: export gate + jwk round-trip
// ---------------------------------------------------------------------------

#[test]
fn ops_export_non_extractable_is_invalid_access() {
    let key = ops::import_key(
        KeyFormat::Raw,
        hmac_keygen_alg(HashAlgorithm::Sha256, None),
        false, // not extractable
        vec![KeyUsage::Sign],
        KeyData::Raw(vec![0x0b; 20]),
    )
    .unwrap();
    assert!(matches!(
        ops::export_key(KeyFormat::Raw, &key),
        Err(AlgorithmError::InvalidAccess(_))
    ));
}

#[test]
fn ops_import_jwk_export_jwk_roundtrip() {
    let jwk = JsonWebKey {
        kty: Some("oct".into()),
        // base64url(no-pad) of 0x0b×20.
        k: Some("CwsLCwsLCwsLCwsLCwsLCwsLCws".into()),
        alg: Some("HS256".into()),
        use_: None,
        key_ops: Some(vec!["sign".into(), "verify".into()]),
        ext: Some(true),
        ..Default::default()
    };
    let key = ops::import_key(
        KeyFormat::Jwk,
        hmac_keygen_alg(HashAlgorithm::Sha256, None),
        true,
        vec![KeyUsage::Sign, KeyUsage::Verify],
        KeyData::Jwk(jwk),
    )
    .unwrap();
    assert_eq!(key.material.as_bytes(), &vec![0x0b; 20][..]);

    let ExportedKey::Jwk(out) = ops::export_key(KeyFormat::Jwk, &key).unwrap() else {
        panic!("expected jwk export");
    };
    assert_eq!(out.kty.as_deref(), Some("oct"));
    assert_eq!(out.alg.as_deref(), Some("HS256"));
    assert_eq!(out.k.as_deref(), Some("CwsLCwsLCwsLCwsLCwsLCwsLCws"));
    assert_eq!(out.ext, Some(true));
}

// ---------------------------------------------------------------------------
// ops: JWK invalid shapes (§6.3 matrix)
// ---------------------------------------------------------------------------

fn import_jwk(jwk: JsonWebKey, usages: Vec<KeyUsage>) -> Result<(), AlgorithmError> {
    ops::import_key(
        KeyFormat::Jwk,
        hmac_keygen_alg(HashAlgorithm::Sha256, None),
        true,
        usages,
        KeyData::Jwk(jwk),
    )
    .map(|_| ())
}

fn base_jwk() -> JsonWebKey {
    JsonWebKey {
        kty: Some("oct".into()),
        k: Some("CwsLCwsLCwsLCwsLCwsLCwsLCws".into()),
        alg: Some("HS256".into()),
        use_: None,
        key_ops: None,
        ext: Some(true),
        ..Default::default()
    }
}

#[test]
fn jwk_invalid_shapes() {
    // kty ≠ oct (row 1)
    let mut j = base_jwk();
    j.kty = Some("RSA".into());
    assert!(matches!(
        import_jwk(j, vec![KeyUsage::Sign]),
        Err(AlgorithmError::Data(_))
    ));

    // k missing (row 2)
    let mut j = base_jwk();
    j.k = None;
    assert!(matches!(
        import_jwk(j, vec![KeyUsage::Sign]),
        Err(AlgorithmError::Data(_))
    ));

    // k not base64url (row 2)
    let mut j = base_jwk();
    j.k = Some("not valid base64url!!!".into());
    assert!(matches!(
        import_jwk(j, vec![KeyUsage::Sign]),
        Err(AlgorithmError::Data(_))
    ));

    // alg mismatch (row 5)
    let mut j = base_jwk();
    j.alg = Some("HS512".into());
    assert!(matches!(
        import_jwk(j, vec![KeyUsage::Sign]),
        Err(AlgorithmError::Data(_))
    ));

    // use ≠ sig (row 7)
    let mut j = base_jwk();
    j.use_ = Some("enc".into());
    assert!(matches!(
        import_jwk(j, vec![KeyUsage::Sign]),
        Err(AlgorithmError::Data(_))
    ));

    // key_ops not a superset (row 8)
    let mut j = base_jwk();
    j.key_ops = Some(vec!["verify".into()]);
    assert!(matches!(
        import_jwk(j, vec![KeyUsage::Sign]),
        Err(AlgorithmError::Data(_))
    ));

    // key_ops invalid identifier (row 8)
    let mut j = base_jwk();
    j.key_ops = Some(vec!["frobnicate".into()]);
    assert!(matches!(
        import_jwk(j, vec![KeyUsage::Sign]),
        Err(AlgorithmError::Data(_))
    ));

    // key_ops duplicate (row 8)
    let mut j = base_jwk();
    j.key_ops = Some(vec!["sign".into(), "sign".into()]);
    assert!(matches!(
        import_jwk(j, vec![KeyUsage::Sign]),
        Err(AlgorithmError::Data(_))
    ));

    // ext false but extractable requested (row 9)
    let mut j = base_jwk();
    j.ext = Some(false);
    assert!(matches!(
        import_jwk(j, vec![KeyUsage::Sign]),
        Err(AlgorithmError::Data(_))
    ));
}

#[test]
fn ops_sign_without_sign_usage_is_invalid_access() {
    let key = ops::import_key(
        KeyFormat::Raw,
        hmac_keygen_alg(HashAlgorithm::Sha256, None),
        true,
        vec![KeyUsage::Verify], // verify only
        KeyData::Raw(vec![0x0b; 20]),
    )
    .unwrap();
    assert!(matches!(
        ops::sign(NormalizedAlgorithm::Hmac, &key, b"x"),
        Err(AlgorithmError::InvalidAccess(_))
    ));
}

#[test]
fn jwk_hmac_alg_names() {
    assert_eq!(hash::HashAlgorithm::Sha1.jwk_hmac_alg(), "HS1");
    assert_eq!(hash::HashAlgorithm::Sha256.jwk_hmac_alg(), "HS256");
    assert_eq!(hash::HashAlgorithm::Sha384.jwk_hmac_alg(), "HS384");
    assert_eq!(hash::HashAlgorithm::Sha512.jwk_hmac_alg(), "HS512");
}
