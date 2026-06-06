//! Unit tests for the engine-independent WebCrypto algorithms.
//!
//! HMAC vectors are from RFC 4231 (SHA-256/384/512) and RFC 2202
//! (SHA-1); digest vectors from FIPS 180 examples.

use crate::algorithm::{is_supported, normalize, NormalizedAlgorithm, Operation, RawAlgorithm};
use crate::error::AlgorithmError;
use crate::hash::HashAlgorithm;
use crate::jwk::JsonWebKey;
use crate::key::{KeyType, KeyUsage};
use crate::ops::{self, ExportedKey, KeyData, KeyFormat};
use crate::{hash, hmac};

fn to_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

// ---------------------------------------------------------------------------
// digest (relocated from the VM host)
// ---------------------------------------------------------------------------

#[test]
fn digest_sha256_abc() {
    // FIPS 180-4 SHA-256("abc").
    assert_eq!(
        to_hex(&HashAlgorithm::Sha256.digest(b"abc")),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn digest_sha1_abc() {
    assert_eq!(
        to_hex(&HashAlgorithm::Sha1.digest(b"abc")),
        "a9993e364706816aba3e25717850c26c9cd0d89d"
    );
}

#[test]
fn digest_lengths() {
    assert_eq!(HashAlgorithm::Sha1.digest(b"").len(), 20);
    assert_eq!(HashAlgorithm::Sha256.digest(b"").len(), 32);
    assert_eq!(HashAlgorithm::Sha384.digest(b"").len(), 48);
    assert_eq!(HashAlgorithm::Sha512.digest(b"").len(), 64);
}

// ---------------------------------------------------------------------------
// HMAC vectors (RFC 4231 TC1: key = 0x0b×20, data = "Hi There")
// ---------------------------------------------------------------------------

#[test]
fn hmac_rfc4231_tc1() {
    let key = vec![0x0b_u8; 20];
    let data = b"Hi There";
    assert_eq!(
        to_hex(&hmac::sign(HashAlgorithm::Sha256, &key, data)),
        "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
    );
    assert_eq!(
        to_hex(&hmac::sign(HashAlgorithm::Sha384, &key, data)),
        "afd03944d84895626b0825f4ab46907f15f9dadbe4101ec682aa034c7cebc59c\
         faea9ea9076ede7f4af152e8b2fa9cb6"
    );
    assert_eq!(
        to_hex(&hmac::sign(HashAlgorithm::Sha512, &key, data)),
        "87aa7cdea5ef619d4ff0b4241a1d6cb02379f4e2ce4ec2787ad0b30545e17cde\
         daa833b7d6b8a702038b274eaea3f4e4be9d914eeb61f1702e696c203a126854"
    );
}

#[test]
fn hmac_sha1_rfc2202_tc1() {
    // RFC 2202 TC1: key = 0x0b×20, data = "Hi There".
    let key = vec![0x0b_u8; 20];
    assert_eq!(
        to_hex(&hmac::sign(HashAlgorithm::Sha1, &key, b"Hi There")),
        "b617318655057264e28bc0b6fb378c8ef146be00"
    );
}

#[test]
fn hmac_sha256_rfc4231_tc2() {
    // key = "Jefe", data = "what do ya want for nothing?"
    assert_eq!(
        to_hex(&hmac::sign(
            HashAlgorithm::Sha256,
            b"Jefe",
            b"what do ya want for nothing?"
        )),
        "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
    );
}

#[test]
fn hmac_verify_constant_time_paths() {
    let key = vec![0x0b_u8; 20];
    let data = b"Hi There";
    let mac = hmac::sign(HashAlgorithm::Sha256, &key, data);
    assert!(hmac::verify(HashAlgorithm::Sha256, &key, &mac, data));
    // Tampered last byte.
    let mut bad = mac.clone();
    bad[31] ^= 0x01;
    assert!(!hmac::verify(HashAlgorithm::Sha256, &key, &bad, data));
    // Wrong length signature.
    assert!(!hmac::verify(HashAlgorithm::Sha256, &key, &mac[..31], data));
}

#[test]
fn hmac_block_size_defaults() {
    assert_eq!(HashAlgorithm::Sha1.block_size_bits(), 512);
    assert_eq!(HashAlgorithm::Sha256.block_size_bits(), 512);
    assert_eq!(HashAlgorithm::Sha384.block_size_bits(), 1024);
    assert_eq!(HashAlgorithm::Sha512.block_size_bits(), 1024);
    assert_eq!(
        hmac::generate_key_byte_len(HashAlgorithm::Sha256, None).unwrap(),
        64
    );
    assert_eq!(
        hmac::generate_key_byte_len(HashAlgorithm::Sha512, None).unwrap(),
        128
    );
    assert_eq!(
        hmac::generate_key_byte_len(HashAlgorithm::Sha256, Some(100)).unwrap(),
        13 // ceil(100/8)
    );
    assert!(matches!(
        hmac::generate_key_byte_len(HashAlgorithm::Sha256, Some(0)),
        Err(AlgorithmError::Operation(_))
    ));
}

// ---------------------------------------------------------------------------
// normalize registry
// ---------------------------------------------------------------------------

#[test]
fn normalize_digest() {
    let raw = RawAlgorithm::from_name("sha-256"); // case-insensitive
    assert_eq!(
        normalize(Operation::Digest, &raw).unwrap(),
        NormalizedAlgorithm::Digest(HashAlgorithm::Sha256)
    );
}

#[test]
fn normalize_unrecognized_is_not_supported() {
    let raw = RawAlgorithm::from_name("ROT13");
    assert!(matches!(
        normalize(Operation::Digest, &raw),
        Err(AlgorithmError::NotSupported(_))
    ));
}

#[test]
fn normalize_hmac_keygen_nested_hash() {
    let raw = RawAlgorithm {
        name: "HMAC".into(),
        hash: Some(Box::new(RawAlgorithm::from_name("SHA-384"))),
        length: Some(256),
    };
    assert_eq!(
        normalize(Operation::GenerateKey, &raw).unwrap(),
        NormalizedAlgorithm::HmacKeyParams {
            hash: HashAlgorithm::Sha384,
            length: Some(256),
        }
    );
}

#[test]
fn normalize_hmac_missing_hash_is_type_error() {
    // §6.3 row 6 — required `hash` absent → TypeError at normalize.
    let raw = RawAlgorithm::from_name("HMAC");
    assert!(matches!(
        normalize(Operation::ImportKey, &raw),
        Err(AlgorithmError::Type(_))
    ));
}

#[test]
fn normalize_hmac_sign_is_name_only() {
    let raw = RawAlgorithm::from_name("hmac");
    assert_eq!(
        normalize(Operation::Sign, &raw).unwrap(),
        NormalizedAlgorithm::Hmac
    );
}

#[test]
fn is_supported_mirrors_normalize_registry() {
    // §18.4.4 step 5 recognition gate. The marshalling layer reads
    // params getters only when this predicate is true, so it must agree
    // with `normalize`'s registry membership exactly (no drift): both
    // route through the same `resolve_registry` oracle.
    assert!(is_supported(Operation::Digest, "sha-256")); // case-insensitive
    assert!(is_supported(Operation::Sign, "HMAC"));
    assert!(is_supported(Operation::Verify, "HMAC"));
    assert!(is_supported(Operation::GenerateKey, "HMAC"));
    assert!(is_supported(Operation::ImportKey, "HMAC"));
    // Recognized name, wrong op for that name → not registered.
    assert!(!is_supported(Operation::GenerateKey, "SHA-256"));
    assert!(!is_supported(Operation::Digest, "HMAC"));
    // Unrecognized name → not registered for any op.
    assert!(!is_supported(Operation::GenerateKey, "AES-Magic"));
    assert!(!is_supported(Operation::ImportKey, "ROT13"));
}

#[test]
fn normalize_echo_truncated_to_64_bytes() {
    let raw = RawAlgorithm::from_name("A".repeat(10_000));
    let Err(AlgorithmError::NotSupported(msg)) = normalize(Operation::Digest, &raw) else {
        panic!("expected NotSupported");
    };
    // The echoed name is bounded; the message prefix + 64 chars.
    assert!(msg.len() < 200);
}

// ---------------------------------------------------------------------------
// ops: generate / sign / verify round-trip
// ---------------------------------------------------------------------------

fn hmac_keygen_alg(hash: HashAlgorithm, length: Option<u32>) -> NormalizedAlgorithm {
    NormalizedAlgorithm::HmacKeyParams { hash, length }
}

#[test]
fn ops_generate_sign_verify_roundtrip() {
    let alg = hmac_keygen_alg(HashAlgorithm::Sha256, None);
    // deterministic "random": fill the crate-sized buffer with 0x42.
    let key = ops::generate_key(alg, true, vec![KeyUsage::Sign, KeyUsage::Verify], |buf| {
        buf.fill(0x42);
        Ok(())
    })
    .unwrap();
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
    let key = ops::generate_key(alg, true, vec![KeyUsage::Sign], |buf| {
        buf.fill(0xFF);
        Ok(())
    })
    .unwrap();
    assert_eq!(key.material.as_bytes(), &[0x80]);
    let crate::key::KeyAlgorithm::Hmac { length, .. } = key.algorithm;
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
