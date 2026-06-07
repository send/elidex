//! Unit tests for the engine-independent WebCrypto algorithms.
//!
//! HMAC vectors are from RFC 4231 (SHA-256/384/512) and RFC 2202
//! (SHA-1); digest vectors from FIPS 180 examples.

use crate::algorithm::{
    is_supported, normalize, params_shape, AesVariant, AlgorithmParams, NormalizedAlgorithm,
    Operation, RawAlgorithm,
};
use crate::error::AlgorithmError;
use crate::hash::HashAlgorithm;
use crate::jwk::JsonWebKey;
use crate::key::{KeyAlgorithm, KeyType, KeyUsage};
use crate::ops::{self, ExportedKey, KeyData, KeyFormat};
use crate::{aes, hash, hmac};

fn to_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

fn from_hex(s: &str) -> Vec<u8> {
    assert!(
        s.len().is_multiple_of(2),
        "hex string must have even length"
    );
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("valid hex"))
        .collect()
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
        normalize(Operation::Digest, raw).unwrap(),
        NormalizedAlgorithm::Digest(HashAlgorithm::Sha256)
    );
}

#[test]
fn normalize_unrecognized_is_not_supported() {
    let raw = RawAlgorithm::from_name("ROT13");
    assert!(matches!(
        normalize(Operation::Digest, raw),
        Err(AlgorithmError::NotSupported(_))
    ));
}

#[test]
fn normalize_hmac_keygen_nested_hash() {
    let raw = RawAlgorithm {
        name: "HMAC".into(),
        hash: Some(Box::new(RawAlgorithm::from_name("SHA-384"))),
        length: Some(256),
        ..RawAlgorithm::default()
    };
    assert_eq!(
        normalize(Operation::GenerateKey, raw).unwrap(),
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
        normalize(Operation::ImportKey, raw),
        Err(AlgorithmError::Type(_))
    ));
}

#[test]
fn normalize_hmac_sign_is_name_only() {
    let raw = RawAlgorithm::from_name("hmac");
    assert_eq!(
        normalize(Operation::Sign, raw).unwrap(),
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
    let Err(AlgorithmError::NotSupported(msg)) = normalize(Operation::Digest, raw) else {
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

// ===========================================================================
// AES-GCM (McGrew & Viega "GCM" test vectors / NIST GCMVS)
// ===========================================================================

#[test]
fn aes_gcm_tc3_aes128_no_aad() {
    // GCM Test Case 3 (AES-128, 64-byte plaintext, no AAD, 128-bit tag).
    let key = from_hex("feffe9928665731c6d6a8f9467308308");
    let iv = from_hex("cafebabefacedbaddecaf888");
    let pt = from_hex(
        "d9313225f88406e5a55909c5aff5269a86a7a9531534f7da2e4c303d8a318a72\
         1c3c0c95956809532fcf0e2449a6b525b16aedf5aa0de657ba637b391aafd255",
    );
    let ct = from_hex(
        "42831ec2217774244b7221b784d0d49ce3aa212f2c02a4e035c17e2329aca12e\
         21d514b25466931c7d8f6a5aac84aa051ba30b396a0aac973d58e091473f5985",
    );
    let tag = from_hex("4d5c2af327cd64a62cf35abd2ba6fab4");
    let expected = [ct.clone(), tag].concat();

    let out = aes::encrypt_gcm(&key, &iv, &[], &pt, 128).unwrap();
    assert_eq!(to_hex(&out), to_hex(&expected));
    let back = aes::decrypt_gcm(&key, &iv, &[], &out, 128).unwrap();
    assert_eq!(to_hex(&back), to_hex(&pt));
}

#[test]
fn aes_gcm_tc4_aes128_with_aad_partial_block() {
    // GCM Test Case 4 (AES-128, 60-byte plaintext + AAD): exercises AAD and
    // a non-block-aligned final block.
    let key = from_hex("feffe9928665731c6d6a8f9467308308");
    let iv = from_hex("cafebabefacedbaddecaf888");
    let aad = from_hex("feedfacedeadbeeffeedfacedeadbeefabaddad2");
    let pt = from_hex(
        "d9313225f88406e5a55909c5aff5269a86a7a9531534f7da2e4c303d8a318a72\
         1c3c0c95956809532fcf0e2449a6b525b16aedf5aa0de657ba637b39",
    );
    let ct = from_hex(
        "42831ec2217774244b7221b784d0d49ce3aa212f2c02a4e035c17e2329aca12e\
         21d514b25466931c7d8f6a5aac84aa051ba30b396a0aac973d58e091",
    );
    let tag = from_hex("5bc94fbc3221a5db94fae95ae7121a47");
    let out = aes::encrypt_gcm(&key, &iv, &aad, &pt, 128).unwrap();
    assert_eq!(to_hex(&out), to_hex(&[ct, tag].concat()));
    assert_eq!(
        to_hex(&aes::decrypt_gcm(&key, &iv, &aad, &out, 128).unwrap()),
        to_hex(&pt)
    );
}

#[test]
fn aes_gcm_tc6_aes128_non_96bit_iv() {
    // GCM Test Case 6 (AES-128, 60-byte IV): exercises the GHASH-based J0
    // derivation for an IV that is not 96 bits.
    let key = from_hex("feffe9928665731c6d6a8f9467308308");
    let iv = from_hex(
        "9313225df88406e555909c5aff5269aa6a7a9538534f7da1e4c303d2a318a728\
         c3c0c95156809539fcf0e2429a6b525416aedbf5a0de6a57a637b39b",
    );
    let aad = from_hex("feedfacedeadbeeffeedfacedeadbeefabaddad2");
    let pt = from_hex(
        "d9313225f88406e5a55909c5aff5269a86a7a9531534f7da2e4c303d8a318a72\
         1c3c0c95956809532fcf0e2449a6b525b16aedf5aa0de657ba637b39",
    );
    let ct = from_hex(
        "8ce24998625615b603a033aca13fb894be9112a5c3a211a8ba262a3cca7e2ca7\
         01e4a9a4fba43c90ccdcb281d48c7c6fd62875d2aca417034c34aee5",
    );
    let tag = from_hex("619cc5aefffe0bfa462af43c1699d050");
    let out = aes::encrypt_gcm(&key, &iv, &aad, &pt, 128).unwrap();
    assert_eq!(to_hex(&out), to_hex(&[ct, tag].concat()));
    assert_eq!(
        to_hex(&aes::decrypt_gcm(&key, &iv, &aad, &out, 128).unwrap()),
        to_hex(&pt)
    );
}

#[test]
fn aes_gcm_tc9_aes192() {
    // GCM Test Case 9 (AES-192, 64-byte plaintext, no AAD).
    let key = from_hex("feffe9928665731c6d6a8f9467308308feffe9928665731c");
    let iv = from_hex("cafebabefacedbaddecaf888");
    let pt = from_hex(
        "d9313225f88406e5a55909c5aff5269a86a7a9531534f7da2e4c303d8a318a72\
         1c3c0c95956809532fcf0e2449a6b525b16aedf5aa0de657ba637b391aafd255",
    );
    let ct = from_hex(
        "3980ca0b3c00e841eb06fac4872a2757859e1ceaa6efd984628593b40ca1e19c\
         7d773d00c144c525ac619d18c84a3f4718e2448b2fe324d9ccda2710acade256",
    );
    let tag = from_hex("9924a7c8587336bfb118024db8674a14");
    let out = aes::encrypt_gcm(&key, &iv, &[], &pt, 128).unwrap();
    assert_eq!(to_hex(&out), to_hex(&[ct, tag].concat()));
    assert_eq!(
        to_hex(&aes::decrypt_gcm(&key, &iv, &[], &out, 128).unwrap()),
        to_hex(&pt)
    );
}

#[test]
fn aes_gcm_tc16_aes256_with_aad() {
    // GCM Test Case 16 (AES-256, 60-byte plaintext + AAD).
    let key = from_hex("feffe9928665731c6d6a8f9467308308feffe9928665731c6d6a8f9467308308");
    let iv = from_hex("cafebabefacedbaddecaf888");
    let aad = from_hex("feedfacedeadbeeffeedfacedeadbeefabaddad2");
    let pt = from_hex(
        "d9313225f88406e5a55909c5aff5269a86a7a9531534f7da2e4c303d8a318a72\
         1c3c0c95956809532fcf0e2449a6b525b16aedf5aa0de657ba637b39",
    );
    let ct = from_hex(
        "522dc1f099567d07f47f37a32a84427d643a8cdcbfe5c0c97598a2bd2555d1aa\
         8cb08e48590dbb3da7b08b1056828838c5f61e6393ba7a0abcc9f662",
    );
    let tag = from_hex("76fc6ece0f4e1768cddf8853bb2d551b");
    let out = aes::encrypt_gcm(&key, &iv, &aad, &pt, 128).unwrap();
    assert_eq!(to_hex(&out), to_hex(&[ct, tag].concat()));
    assert_eq!(
        to_hex(&aes::decrypt_gcm(&key, &iv, &aad, &out, 128).unwrap()),
        to_hex(&pt)
    );
}

#[test]
fn aes_gcm_truncated_tag_is_leading_bits() {
    // A truncated tag is the leading `tagLength` bits of the full 128-bit
    // tag (NIST SP 800-38D), so a 96-bit tag == the first 12 tag bytes of
    // TC3 — and it round-trips, while verifying it as a 128-bit tag fails.
    let key = from_hex("feffe9928665731c6d6a8f9467308308");
    let iv = from_hex("cafebabefacedbaddecaf888");
    let pt = from_hex("d9313225f88406e5a55909c5aff5269a");
    let full = aes::encrypt_gcm(&key, &iv, &[], &pt, 128).unwrap();
    let trunc = aes::encrypt_gcm(&key, &iv, &[], &pt, 96).unwrap();
    // ciphertext identical; the 96-bit tag is the leading 12 of the 16.
    assert_eq!(to_hex(&trunc), to_hex(&full[..full.len() - 4]));
    assert_eq!(
        to_hex(&aes::decrypt_gcm(&key, &iv, &[], &trunc, 96).unwrap()),
        to_hex(&pt)
    );
    // Decrypting a 96-bit-tag ciphertext as 128-bit must fail.
    assert!(matches!(
        aes::decrypt_gcm(&key, &iv, &[], &trunc, 128),
        Err(AlgorithmError::Operation(_))
    ));
}

#[test]
fn aes_gcm_tampered_tag_and_ciphertext_fail() {
    let key = from_hex("feffe9928665731c6d6a8f9467308308");
    let iv = from_hex("cafebabefacedbaddecaf888");
    let pt = from_hex("d9313225f88406e5a55909c5aff5269a");
    let mut ct = aes::encrypt_gcm(&key, &iv, &[], &pt, 128).unwrap();
    // Flip a tag bit.
    *ct.last_mut().unwrap() ^= 0x01;
    assert!(matches!(
        aes::decrypt_gcm(&key, &iv, &[], &ct, 128),
        Err(AlgorithmError::Operation(_))
    ));
    // Flip a ciphertext byte.
    let mut ct = aes::encrypt_gcm(&key, &iv, &[], &pt, 128).unwrap();
    ct[0] ^= 0x01;
    assert!(matches!(
        aes::decrypt_gcm(&key, &iv, &[], &ct, 128),
        Err(AlgorithmError::Operation(_))
    ));
}

#[test]
fn aes_gcm_invalid_tag_length_is_operation_error() {
    let key = from_hex("feffe9928665731c6d6a8f9467308308");
    let iv = from_hex("cafebabefacedbaddecaf888");
    for bad in [0u32, 8, 16, 48, 127, 129, 256] {
        assert!(
            matches!(
                aes::encrypt_gcm(&key, &iv, &[], b"", bad),
                Err(AlgorithmError::Operation(_))
            ),
            "tagLength {bad} should be OperationError"
        );
    }
}

#[test]
fn aes_gcm_empty_plaintext_roundtrips() {
    let key = from_hex("feffe9928665731c6d6a8f9467308308");
    let iv = from_hex("cafebabefacedbaddecaf888");
    let out = aes::encrypt_gcm(&key, &iv, &[], &[], 128).unwrap();
    assert_eq!(out.len(), 16); // tag only
    assert_eq!(
        aes::decrypt_gcm(&key, &iv, &[], &out, 128).unwrap(),
        Vec::<u8>::new()
    );
}

// ===========================================================================
// AES-CBC (NIST SP 800-38A F.2)
// ===========================================================================

#[test]
fn aes_cbc_first_block_matches_nist_f2() {
    // F.2.1 CBC-AES128.Encrypt block 1.  WebCrypto adds PKCS#7 padding, so
    // a single 16-byte plaintext yields 32 bytes (one ciphertext block +
    // one full padding block); the first block equals the NIST vector.
    let key = from_hex("2b7e151628aed2a6abf7158809cf4f3c");
    let iv = from_hex("000102030405060708090a0b0c0d0e0f");
    let pt = from_hex("6bc1bee22e409f96e93d7e117393172a");
    let out = aes::encrypt_cbc(&key, &iv, &pt).unwrap();
    assert_eq!(out.len(), 32);
    assert_eq!(to_hex(&out[..16]), "7649abac8119b246cee98e9b12e9197d");
    assert_eq!(
        to_hex(&aes::decrypt_cbc(&key, &iv, &out).unwrap()),
        to_hex(&pt)
    );
}

#[test]
fn aes_cbc_roundtrip_all_key_sizes() {
    let iv = from_hex("000102030405060708090a0b0c0d0e0f");
    let pt = b"the quick brown fox jumps"; // 25 bytes (non-block-aligned)
    for klen in [16usize, 24, 32] {
        let key = vec![0x42u8; klen];
        let ct = aes::encrypt_cbc(&key, &iv, pt).unwrap();
        // PKCS#7 always pads to the next block boundary.
        assert_eq!(ct.len() % 16, 0);
        assert!(ct.len() > pt.len());
        assert_eq!(aes::decrypt_cbc(&key, &iv, &ct).unwrap(), pt);
    }
}

#[test]
fn aes_cbc_bad_iv_length_is_operation_error() {
    let key = from_hex("2b7e151628aed2a6abf7158809cf4f3c");
    assert!(matches!(
        aes::encrypt_cbc(&key, &[0u8; 12], b"abc"),
        Err(AlgorithmError::Operation(_))
    ));
    assert!(matches!(
        aes::decrypt_cbc(&key, &[0u8; 17], &[0u8; 16]),
        Err(AlgorithmError::Operation(_))
    ));
}

#[test]
fn aes_cbc_bad_ciphertext_length_is_operation_error() {
    let key = from_hex("2b7e151628aed2a6abf7158809cf4f3c");
    let iv = from_hex("000102030405060708090a0b0c0d0e0f");
    // Empty + non-multiple-of-16 ciphertexts are rejected before unpadding.
    assert!(matches!(
        aes::decrypt_cbc(&key, &iv, &[]),
        Err(AlgorithmError::Operation(_))
    ));
    assert!(matches!(
        aes::decrypt_cbc(&key, &iv, &[0u8; 17]),
        Err(AlgorithmError::Operation(_))
    ));
}

#[test]
fn aes_cbc_invalid_padding_is_operation_error() {
    let key = from_hex("2b7e151628aed2a6abf7158809cf4f3c");
    let iv = from_hex("000102030405060708090a0b0c0d0e0f");
    // A block-aligned ciphertext that decrypts to invalid PKCS#7 padding.
    let bogus = vec![0u8; 16];
    assert!(matches!(
        aes::decrypt_cbc(&key, &iv, &bogus),
        Err(AlgorithmError::Operation(_))
    ));
}

// ===========================================================================
// AES-CTR (NIST SP 800-38A F.5)
// ===========================================================================

#[test]
fn aes_ctr_full_counter_matches_nist_f5() {
    // F.5.1 CTR-AES128.Encrypt, length = 128 (full-block counter).
    let key = from_hex("2b7e151628aed2a6abf7158809cf4f3c");
    let counter = from_hex("f0f1f2f3f4f5f6f7f8f9fafbfcfdfeff");
    let pt = from_hex(
        "6bc1bee22e409f96e93d7e117393172aae2d8a571e03ac9c9eb76fac45af8e51\
         30c81c46a35ce411e5fbc1191a0a52eff69f2445df4f9b17ad2b417be66c3710",
    );
    let ct = from_hex(
        "874d6191b620e3261bef6864990db6ce9806f66b7970fdff8617187bb9fffdff\
         5ae4df3edbd5d35e5b4f09020db03eab1e031dda2fbe03d1792170a0f3009cee",
    );
    assert_eq!(
        to_hex(&aes::encrypt_ctr(&key, &counter, 128, &pt).unwrap()),
        to_hex(&ct)
    );
    assert_eq!(
        to_hex(&aes::decrypt_ctr(&key, &counter, 128, &ct).unwrap()),
        to_hex(&pt)
    );
}

#[test]
fn aes_ctr_roundtrip_partial_counter_width_and_key_sizes() {
    let counter = from_hex("00000000000000000000000000000000");
    let pt = vec![0xABu8; 70]; // > 4 blocks, non-aligned tail
    for klen in [16usize, 24, 32] {
        let key = vec![0x11u8; klen];
        // A narrow counter width (e.g. 32 bits) still round-trips.
        let ct = aes::encrypt_ctr(&key, &counter, 32, &pt).unwrap();
        assert_eq!(ct.len(), pt.len());
        assert_eq!(aes::decrypt_ctr(&key, &counter, 32, &ct).unwrap(), pt);
    }
}

#[test]
fn aes_ctr_partial_counter_wraps_within_width() {
    // With a 16-bit counter at 0xFFFF, the next block reuses counter 0x0000
    // while the upper 112 nonce bits are preserved.  Two blocks at 0xFFFF
    // and 0x0000 therefore use distinct keystreams; the round-trip confirms
    // the counter increment honours the narrow width.
    let key = from_hex("2b7e151628aed2a6abf7158809cf4f3c");
    let counter = from_hex("aabbccddeeff00112233445566ffffff");
    let pt = vec![0u8; 48];
    let ct = aes::encrypt_ctr(&key, &counter, 24, &pt).unwrap();
    assert_eq!(aes::decrypt_ctr(&key, &counter, 24, &ct).unwrap(), pt);
}

#[test]
fn aes_ctr_invalid_params_are_operation_errors() {
    let key = from_hex("2b7e151628aed2a6abf7158809cf4f3c");
    let counter = from_hex("f0f1f2f3f4f5f6f7f8f9fafbfcfdfeff");
    // counter must be 16 bytes
    assert!(matches!(
        aes::encrypt_ctr(&key, &[0u8; 8], 64, b"x"),
        Err(AlgorithmError::Operation(_))
    ));
    // length ∈ [1, 128]
    assert!(matches!(
        aes::encrypt_ctr(&key, &counter, 0, b"x"),
        Err(AlgorithmError::Operation(_))
    ));
    assert!(matches!(
        aes::encrypt_ctr(&key, &counter, 129, b"x"),
        Err(AlgorithmError::Operation(_))
    ));
}

#[test]
fn aes_ctr_message_exceeding_counter_capacity_is_operation_error() {
    // §27.7.1 step 3 / NIST SP 800-38A: a message of more than 2^length
    // blocks wraps the counter and reuses keystream → reject (OperationError).
    let key = from_hex("2b7e151628aed2a6abf7158809cf4f3c");
    let counter = [0u8; 16];
    // length=8 → counter space = 2^8 = 256 blocks = 4096 bytes. Exactly at
    // capacity round-trips (all 256 counter values distinct).
    let exact = vec![0u8; 256 * 16];
    let ct = aes::encrypt_ctr(&key, &counter, 8, &exact).unwrap();
    assert_eq!(aes::decrypt_ctr(&key, &counter, 8, &ct).unwrap(), exact);
    // One block past capacity (257 blocks) would reuse counter 0 → reject.
    assert!(matches!(
        aes::encrypt_ctr(&key, &counter, 8, &[0u8; 256 * 16 + 1]),
        Err(AlgorithmError::Operation(_))
    ));
    // A wide counter (length=128) imposes no practical limit.
    assert!(aes::encrypt_ctr(&key, &counter, 128, &[0u8; 64]).is_ok());
}

// ===========================================================================
// AES ops (generate / import / export + encrypt / decrypt validation)
// ===========================================================================

// Deterministic key material for the ops tests; the `Result` shape + the
// truncating index cast match the `fill_random` closure contract (the key
// is at most 32 bytes, so the index never exceeds a `u8`).
#[allow(clippy::unnecessary_wraps, clippy::cast_possible_truncation)]
fn fill_seq(buf: &mut [u8]) -> Result<(), AlgorithmError> {
    for (i, b) in buf.iter_mut().enumerate() {
        *b = i as u8;
    }
    Ok(())
}

fn aes_gcm_params(iv: Vec<u8>) -> NormalizedAlgorithm {
    NormalizedAlgorithm::AesGcm {
        iv,
        additional_data: None,
        tag_length: 128,
    }
}

#[test]
fn ops_aes_generate_encrypt_decrypt_roundtrip() {
    let key = ops::generate_key(
        NormalizedAlgorithm::AesKeyGen {
            variant: AesVariant::Gcm,
            length: 256,
        },
        true,
        vec![KeyUsage::Encrypt, KeyUsage::Decrypt],
        fill_seq,
    )
    .unwrap();
    assert!(matches!(
        key.algorithm,
        KeyAlgorithm::Aes {
            variant: AesVariant::Gcm,
            length: 256
        }
    ));
    let iv = vec![0x24u8; 12];
    let msg = b"attack at dawn";
    let ct = ops::encrypt(aes_gcm_params(iv.clone()), &key, msg).unwrap();
    let pt = ops::decrypt(aes_gcm_params(iv), &key, &ct).unwrap();
    assert_eq!(pt, msg);
}

#[test]
fn ops_aes_generate_invalid_length_is_operation_error() {
    assert!(matches!(
        ops::generate_key(
            NormalizedAlgorithm::AesKeyGen {
                variant: AesVariant::Cbc,
                length: 200,
            },
            true,
            vec![KeyUsage::Encrypt],
            fill_seq,
        ),
        Err(AlgorithmError::Operation(_))
    ));
}

#[test]
fn ops_aes_generate_invalid_usage_is_syntax_error() {
    // `sign` is not a valid AES usage.
    assert!(matches!(
        ops::generate_key(
            NormalizedAlgorithm::AesKeyGen {
                variant: AesVariant::Gcm,
                length: 128,
            },
            true,
            vec![KeyUsage::Sign],
            fill_seq,
        ),
        Err(AlgorithmError::Syntax(_))
    ));
    // wrapKey / unwrapKey ARE valid AES usages (even though the wrap ops
    // land in `#11-crypto-subtle-full` PR-3).
    assert!(ops::generate_key(
        NormalizedAlgorithm::AesKeyGen {
            variant: AesVariant::Gcm,
            length: 128,
        },
        true,
        vec![KeyUsage::WrapKey, KeyUsage::UnwrapKey],
        fill_seq,
    )
    .is_ok());
}

#[test]
fn ops_aes_import_raw_bad_length_is_data_error() {
    for len in [8usize, 15, 20, 33] {
        assert!(
            matches!(
                ops::import_key(
                    KeyFormat::Raw,
                    NormalizedAlgorithm::AesImport {
                        variant: AesVariant::Ctr,
                    },
                    true,
                    vec![KeyUsage::Encrypt],
                    KeyData::Raw(vec![0u8; len]),
                ),
                Err(AlgorithmError::Data(_))
            ),
            "raw AES key of {len} bytes should be DataError"
        );
    }
}

#[test]
fn ops_aes_import_raw_then_encrypt_matches_known_vector() {
    // Importing the NIST CTR key + using it through the op layer reproduces
    // the F.5 keystream.
    let key = ops::import_key(
        KeyFormat::Raw,
        NormalizedAlgorithm::AesImport {
            variant: AesVariant::Ctr,
        },
        true,
        vec![KeyUsage::Encrypt, KeyUsage::Decrypt],
        KeyData::Raw(from_hex("2b7e151628aed2a6abf7158809cf4f3c")),
    )
    .unwrap();
    let ct = ops::encrypt(
        NormalizedAlgorithm::AesCtr {
            counter: from_hex("f0f1f2f3f4f5f6f7f8f9fafbfcfdfeff"),
            length: 128,
        },
        &key,
        &from_hex("6bc1bee22e409f96e93d7e117393172a"),
    )
    .unwrap();
    assert_eq!(to_hex(&ct), "874d6191b620e3261bef6864990db6ce");
}

#[test]
fn ops_aes_export_jwk_roundtrip() {
    let key = ops::import_key(
        KeyFormat::Raw,
        NormalizedAlgorithm::AesImport {
            variant: AesVariant::Cbc,
        },
        true,
        vec![KeyUsage::Encrypt, KeyUsage::Decrypt],
        KeyData::Raw(vec![0x7u8; 32]),
    )
    .unwrap();
    let ExportedKey::Jwk(jwk) = ops::export_key(KeyFormat::Jwk, &key).unwrap() else {
        panic!("expected a JWK export");
    };
    assert_eq!(jwk.kty.as_deref(), Some("oct"));
    assert_eq!(jwk.alg.as_deref(), Some("A256CBC"));
    assert_eq!(jwk.ext, Some(true));
    // Re-import the exported JWK and confirm the material round-trips.
    let reimported = ops::import_key(
        KeyFormat::Jwk,
        NormalizedAlgorithm::AesImport {
            variant: AesVariant::Cbc,
        },
        true,
        vec![KeyUsage::Encrypt, KeyUsage::Decrypt],
        KeyData::Jwk(jwk),
    )
    .unwrap();
    assert_eq!(reimported.material, key.material);
}

#[test]
fn ops_aes_encrypt_wrong_usage_is_invalid_access() {
    let key = ops::import_key(
        KeyFormat::Raw,
        NormalizedAlgorithm::AesImport {
            variant: AesVariant::Gcm,
        },
        true,
        vec![KeyUsage::Decrypt], // decrypt only
        KeyData::Raw(vec![0u8; 16]),
    )
    .unwrap();
    assert!(matches!(
        ops::encrypt(aes_gcm_params(vec![0u8; 12]), &key, b"x"),
        Err(AlgorithmError::InvalidAccess(_))
    ));
}

#[test]
fn ops_aes_encrypt_mode_mismatch_is_invalid_access() {
    // An AES-GCM key used with AES-CBC params → name mismatch.
    let key = ops::import_key(
        KeyFormat::Raw,
        NormalizedAlgorithm::AesImport {
            variant: AesVariant::Gcm,
        },
        true,
        vec![KeyUsage::Encrypt],
        KeyData::Raw(vec![0u8; 16]),
    )
    .unwrap();
    assert!(matches!(
        ops::encrypt(
            NormalizedAlgorithm::AesCbc { iv: vec![0u8; 16] },
            &key,
            b"x"
        ),
        Err(AlgorithmError::InvalidAccess(_))
    ));
}

#[test]
fn aes_jwk_alg_names() {
    assert_eq!(AesVariant::Gcm.jwk_alg(128), Some("A128GCM"));
    assert_eq!(AesVariant::Cbc.jwk_alg(192), Some("A192CBC"));
    assert_eq!(AesVariant::Ctr.jwk_alg(256), Some("A256CTR"));
    assert_eq!(AesVariant::Gcm.jwk_alg(200), None);
}

#[test]
fn aes_normalize_and_params_shape() {
    // generateKey reads AesKeyGenParams (length required).
    assert_eq!(
        params_shape(Operation::GenerateKey, "AES-GCM"),
        Some(AlgorithmParams::AesKeyGen)
    );
    // importKey is name-only (length derives from material).
    assert_eq!(
        params_shape(Operation::ImportKey, "aes-cbc"),
        Some(AlgorithmParams::NameOnly)
    );
    // encrypt reads the mode's params dictionary.
    assert_eq!(
        params_shape(Operation::Encrypt, "AES-CTR"),
        Some(AlgorithmParams::AesCtrParams)
    );
    assert_eq!(
        params_shape(Operation::Decrypt, "AES-GCM"),
        Some(AlgorithmParams::AesGcmParams)
    );
    // AES is not a digest/sign algorithm.
    assert!(params_shape(Operation::Sign, "AES-GCM").is_none());
    assert!(!is_supported(Operation::Digest, "AES-CBC"));

    // A missing required AES-CTR length normalizes to a TypeError.
    let raw = RawAlgorithm {
        name: "AES-CTR".to_string(),
        counter: Some(vec![0u8; 16]),
        ..RawAlgorithm::default()
    };
    assert!(matches!(
        normalize(Operation::Encrypt, raw),
        Err(AlgorithmError::Type(_))
    ));
}
