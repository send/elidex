// ---------------------------------------------------------------------------
// ops: HKDF / PBKDF2 deriveBits + get-key-length + deriveKey compose
//
// HKDF vectors from RFC 5869 Appendix A (cases 1, 3, 4); PBKDF2 vectors from
// RFC 6070 (SHA-1) and RFC 7914 §11 (SHA-256).
// ---------------------------------------------------------------------------

use super::{no_rng, to_hex};
use crate::algorithm::{AesVariant, NormalizedAlgorithm};
use crate::error::AlgorithmError;
use crate::hash::HashAlgorithm;
use crate::key::{CryptoKeyData, KeyAlgorithm, KeyMaterial, KeyType, KeyUsage};
use crate::ops::{self, ExportedKey, KeyData, KeyFormat};

/// Import a raw HKDF / PBKDF2 key (the input keying material / password).
fn import_kdf(alg: NormalizedAlgorithm, material: &[u8], usages: Vec<KeyUsage>) -> CryptoKeyData {
    ops::import_key(
        KeyFormat::Raw,
        alg,
        false,
        usages,
        KeyData::Raw(material.to_vec()),
    )
    .expect("KDF raw import")
}

// --- HKDF deriveBits (RFC 5869 App. A) -------------------------------------

#[test]
fn hkdf_derive_bits_rfc5869_case1_sha256() {
    let ikm = vec![0x0b; 22];
    let salt = super::from_hex("000102030405060708090a0b0c");
    let info = super::from_hex("f0f1f2f3f4f5f6f7f8f9");
    let key = import_kdf(NormalizedAlgorithm::Hkdf, &ikm, vec![KeyUsage::DeriveBits]);
    let alg = NormalizedAlgorithm::HkdfParams {
        hash: HashAlgorithm::Sha256,
        salt,
        info,
    };
    // L = 42 octets → 336 bits.
    let out = ops::derive_bits(alg, &key, Some(336)).unwrap();
    assert_eq!(
        to_hex(&out),
        "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865"
    );
}

#[test]
fn hkdf_derive_bits_rfc5869_case3_sha256_empty_salt_info() {
    let ikm = vec![0x0b; 22];
    let key = import_kdf(NormalizedAlgorithm::Hkdf, &ikm, vec![KeyUsage::DeriveBits]);
    let alg = NormalizedAlgorithm::HkdfParams {
        hash: HashAlgorithm::Sha256,
        salt: Vec::new(),
        info: Vec::new(),
    };
    let out = ops::derive_bits(alg, &key, Some(336)).unwrap();
    assert_eq!(
        to_hex(&out),
        "8da4e775a563c18f715f802a063c5a31b8a11f5c5ee1879ec3454e5f3c738d2d9d201395faa4b61a96c8"
    );
}

#[test]
fn hkdf_derive_bits_rfc5869_case4_sha1() {
    let ikm = vec![0x0b; 11];
    let salt = super::from_hex("000102030405060708090a0b0c");
    let info = super::from_hex("f0f1f2f3f4f5f6f7f8f9");
    let key = import_kdf(NormalizedAlgorithm::Hkdf, &ikm, vec![KeyUsage::DeriveBits]);
    let alg = NormalizedAlgorithm::HkdfParams {
        hash: HashAlgorithm::Sha1,
        salt,
        info,
    };
    let out = ops::derive_bits(alg, &key, Some(336)).unwrap();
    assert_eq!(
        to_hex(&out),
        "085a01ea1b10f36933068b56efa5ad81a4f14b822f5b091568a9cdd4f155fda2c22e422478d305f3f896"
    );
}

#[test]
fn hkdf_derive_bits_zero_length_is_empty() {
    // §33.4.1: length 0 is a multiple of 8 → HKDF-Expand of L=0 → empty.
    let key = import_kdf(
        NormalizedAlgorithm::Hkdf,
        b"ikm",
        vec![KeyUsage::DeriveBits],
    );
    let alg = NormalizedAlgorithm::HkdfParams {
        hash: HashAlgorithm::Sha256,
        salt: b"s".to_vec(),
        info: b"i".to_vec(),
    };
    assert!(ops::derive_bits(alg, &key, Some(0)).unwrap().is_empty());
}

// --- PBKDF2 deriveBits (RFC 6070 / RFC 7914) -------------------------------

fn pbkdf2_params(hash: HashAlgorithm, salt: &[u8], iterations: u32) -> NormalizedAlgorithm {
    NormalizedAlgorithm::Pbkdf2Params {
        salt: salt.to_vec(),
        iterations,
        hash,
    }
}

#[test]
fn pbkdf2_derive_bits_rfc6070_sha1() {
    let key = import_kdf(
        NormalizedAlgorithm::Pbkdf2,
        b"password",
        vec![KeyUsage::DeriveBits],
    );
    // c=1, dkLen=20 octets → 160 bits.
    let c1 = ops::derive_bits(
        pbkdf2_params(HashAlgorithm::Sha1, b"salt", 1),
        &key,
        Some(160),
    )
    .unwrap();
    assert_eq!(to_hex(&c1), "0c60c80f961f0e71f3a9b524af6012062fe037a6");
    let c2 = ops::derive_bits(
        pbkdf2_params(HashAlgorithm::Sha1, b"salt", 2),
        &key,
        Some(160),
    )
    .unwrap();
    assert_eq!(to_hex(&c2), "ea6c014dc72d6f8ccd1ed92ace1d41f0d8de8957");
    let c4096 = ops::derive_bits(
        pbkdf2_params(HashAlgorithm::Sha1, b"salt", 4096),
        &key,
        Some(160),
    )
    .unwrap();
    assert_eq!(to_hex(&c4096), "4b007901b765489abead49d926f721d065a429c1");
}

#[test]
fn pbkdf2_derive_bits_rfc7914_sha256() {
    let key = import_kdf(
        NormalizedAlgorithm::Pbkdf2,
        b"passwd",
        vec![KeyUsage::DeriveBits],
    );
    // c=1, dkLen=64 octets → 512 bits (RFC 7914 §11).
    let out = ops::derive_bits(
        pbkdf2_params(HashAlgorithm::Sha256, b"salt", 1),
        &key,
        Some(512),
    )
    .unwrap();
    assert_eq!(
        to_hex(&out),
        "55ac046e56e3089fec1691c22544b605f94185216dde0465e68b9d57c20dacbc\
         49ca9cccf179b645991664b39d77ef317c71b845b1e30bd509112041d3a19783"
    );
}

#[test]
fn hkdf_oversized_length_is_operation_error() {
    // §33.4.1 step 4 / RFC 5869 §2.3: HKDF-SHA-256 caps output at 255×32 =
    // 8160 bytes (65280 bits). A request above the cap is an OperationError —
    // and must reject WITHOUT allocating the oversized buffer (Codex R2 F3).
    let key = import_kdf(
        NormalizedAlgorithm::Hkdf,
        b"ikm",
        vec![KeyUsage::DeriveBits],
    );
    let over = NormalizedAlgorithm::HkdfParams {
        hash: HashAlgorithm::Sha256,
        salt: b"s".to_vec(),
        info: b"i".to_vec(),
    };
    assert!(matches!(
        ops::derive_bits(over, &key, Some(65288)), // 8161 bytes > 8160 cap
        Err(AlgorithmError::Operation(_))
    ));
    // At the cap (8160 bytes = 65280 bits) the derivation succeeds.
    let at_cap = NormalizedAlgorithm::HkdfParams {
        hash: HashAlgorithm::Sha256,
        salt: b"s".to_vec(),
        info: b"i".to_vec(),
    };
    assert_eq!(
        ops::derive_bits(at_cap, &key, Some(65280)).unwrap().len(),
        8160
    );
}

#[test]
fn kdf_export_key_is_not_supported_before_extractable() {
    // Codex R2 F2 / §14.3.10: the step-6 export-support check precedes the
    // step-7 extractable check, so a KDF key's exportKey is NotSupportedError
    // — even for a (hypothetically) extractable KDF key, NOT InvalidAccess.
    let extractable_kdf = CryptoKeyData {
        key_type: KeyType::Secret,
        extractable: true,
        algorithm: KeyAlgorithm::Hkdf,
        usages: vec![KeyUsage::DeriveBits],
        material: KeyMaterial::Raw(b"ikm".to_vec()),
    };
    assert!(matches!(
        ops::export_key(KeyFormat::Raw, &extractable_kdf),
        Err(AlgorithmError::NotSupported(_))
    ));
    assert!(matches!(
        ops::export_key(KeyFormat::Jwk, &extractable_kdf),
        Err(AlgorithmError::NotSupported(_))
    ));
    // The as-imported (non-extractable) PBKDF2 key likewise → NotSupported,
    // not InvalidAccess.
    let imported = import_kdf(
        NormalizedAlgorithm::Pbkdf2,
        b"pw",
        vec![KeyUsage::DeriveBits],
    );
    assert!(matches!(
        ops::export_key(KeyFormat::Raw, &imported),
        Err(AlgorithmError::NotSupported(_))
    ));
    // Sanity: an extractable HMAC key still exports (step 6 passes, step 7
    // passes) — the reordering didn't regress the supported algorithms.
    let hmac = ops::import_key(
        KeyFormat::Raw,
        NormalizedAlgorithm::HmacKeyParams {
            hash: HashAlgorithm::Sha256,
            length: None,
        },
        true,
        vec![KeyUsage::Sign],
        KeyData::Raw(vec![0x0b; 32]),
    )
    .unwrap();
    assert!(matches!(
        ops::export_key(KeyFormat::Raw, &hmac),
        Ok(ExportedKey::Raw(_))
    ));
}

#[test]
fn pbkdf2_zero_iterations_is_operation_error() {
    // §34.4.1 step 2.
    let key = import_kdf(
        NormalizedAlgorithm::Pbkdf2,
        b"pw",
        vec![KeyUsage::DeriveBits],
    );
    assert!(matches!(
        ops::derive_bits(
            pbkdf2_params(HashAlgorithm::Sha256, b"s", 0),
            &key,
            Some(128)
        ),
        Err(AlgorithmError::Operation(_))
    ));
}

#[test]
fn pbkdf2_zero_length_is_empty() {
    // §34.4.1 step 3.
    let key = import_kdf(
        NormalizedAlgorithm::Pbkdf2,
        b"pw",
        vec![KeyUsage::DeriveBits],
    );
    assert!(
        ops::derive_bits(pbkdf2_params(HashAlgorithm::Sha256, b"s", 1), &key, Some(0))
            .unwrap()
            .is_empty()
    );
}

// --- deriveBits length / name / usage gates --------------------------------

#[test]
fn derive_bits_null_length_is_operation_error() {
    // §33.4.1 / §34.4.1 step 1: a null length is an OperationError.
    let key = import_kdf(
        NormalizedAlgorithm::Hkdf,
        b"ikm",
        vec![KeyUsage::DeriveBits],
    );
    let alg = NormalizedAlgorithm::HkdfParams {
        hash: HashAlgorithm::Sha256,
        salt: b"s".to_vec(),
        info: b"i".to_vec(),
    };
    assert!(matches!(
        ops::derive_bits(alg, &key, None),
        Err(AlgorithmError::Operation(_))
    ));
}

#[test]
fn derive_bits_non_multiple_of_8_is_operation_error() {
    let key = import_kdf(
        NormalizedAlgorithm::Hkdf,
        b"ikm",
        vec![KeyUsage::DeriveBits],
    );
    let alg = NormalizedAlgorithm::HkdfParams {
        hash: HashAlgorithm::Sha256,
        salt: b"s".to_vec(),
        info: b"i".to_vec(),
    };
    assert!(matches!(
        ops::derive_bits(alg, &key, Some(100)), // 100 % 8 != 0
        Err(AlgorithmError::Operation(_))
    ));
}

#[test]
fn derive_bits_name_mismatch_is_invalid_access() {
    // §14.3.8 step 8: HKDF deriveBits on a PBKDF2 key.
    let key = import_kdf(
        NormalizedAlgorithm::Pbkdf2,
        b"pw",
        vec![KeyUsage::DeriveBits],
    );
    let alg = NormalizedAlgorithm::HkdfParams {
        hash: HashAlgorithm::Sha256,
        salt: b"s".to_vec(),
        info: b"i".to_vec(),
    };
    assert!(matches!(
        ops::derive_bits(alg, &key, Some(128)),
        Err(AlgorithmError::InvalidAccess(_))
    ));
}

#[test]
fn derive_bits_missing_usage_is_invalid_access() {
    // §14.3.8 step 9: a key without the "deriveBits" usage.
    let key = import_kdf(NormalizedAlgorithm::Hkdf, b"ikm", vec![KeyUsage::DeriveKey]);
    let alg = NormalizedAlgorithm::HkdfParams {
        hash: HashAlgorithm::Sha256,
        salt: b"s".to_vec(),
        info: b"i".to_vec(),
    };
    assert!(matches!(
        ops::derive_bits(alg, &key, Some(128)),
        Err(AlgorithmError::InvalidAccess(_))
    ));
}

// --- get key length (§27.7.6 / §28.4.6 / §29.4.6 / §31.6.6 / §33.4.3 /
//     §34.4.3) ---------------------------------------------------------------

#[test]
fn get_key_length_aes_valid_and_invalid() {
    for length in [128, 192, 256] {
        let alg = NormalizedAlgorithm::AesKeyGen {
            variant: AesVariant::Gcm,
            length,
        };
        assert_eq!(ops::get_key_length(alg).unwrap(), Some(length));
    }
    let bad = NormalizedAlgorithm::AesKeyGen {
        variant: AesVariant::Cbc,
        length: 200,
    };
    assert!(matches!(
        ops::get_key_length(bad),
        Err(AlgorithmError::Operation(_))
    ));
}

#[test]
fn get_key_length_hmac_block_size_default() {
    // §31.6.6: absent length → hash block size in bits.
    assert_eq!(
        ops::get_key_length(NormalizedAlgorithm::HmacKeyParams {
            hash: HashAlgorithm::Sha256,
            length: None,
        })
        .unwrap(),
        Some(512)
    );
    assert_eq!(
        ops::get_key_length(NormalizedAlgorithm::HmacKeyParams {
            hash: HashAlgorithm::Sha512,
            length: None,
        })
        .unwrap(),
        Some(1024)
    );
}

#[test]
fn get_key_length_hmac_explicit_and_zero() {
    // §31.6.6: present non-zero → that value; zero → TypeError.
    assert_eq!(
        ops::get_key_length(NormalizedAlgorithm::HmacKeyParams {
            hash: HashAlgorithm::Sha256,
            length: Some(256),
        })
        .unwrap(),
        Some(256)
    );
    assert!(matches!(
        ops::get_key_length(NormalizedAlgorithm::HmacKeyParams {
            hash: HashAlgorithm::Sha256,
            length: Some(0),
        }),
        Err(AlgorithmError::Type(_))
    ));
}

#[test]
fn get_key_length_kdf_is_null() {
    // §33.4.3 / §34.4.3: HKDF / PBKDF2 → null.
    assert_eq!(
        ops::get_key_length(NormalizedAlgorithm::Hkdf).unwrap(),
        None
    );
    assert_eq!(
        ops::get_key_length(NormalizedAlgorithm::Pbkdf2).unwrap(),
        None
    );
}

// --- importKey constraints (§33.4.2 / §34.4.2) -----------------------------

#[test]
fn kdf_import_raw_success() {
    let key = ops::import_key(
        KeyFormat::Raw,
        NormalizedAlgorithm::Hkdf,
        false,
        vec![KeyUsage::DeriveBits, KeyUsage::DeriveKey],
        KeyData::Raw(b"secret".to_vec()),
    )
    .unwrap();
    assert_eq!(key.algorithm, KeyAlgorithm::Hkdf);
    assert_eq!(key.key_type, KeyType::Secret);
    assert!(!key.extractable);
    assert_eq!(key.material.as_bytes(), b"secret");
}

#[test]
fn kdf_import_extractable_true_is_syntax_error() {
    assert!(matches!(
        ops::import_key(
            KeyFormat::Raw,
            NormalizedAlgorithm::Pbkdf2,
            true,
            vec![KeyUsage::DeriveBits],
            KeyData::Raw(b"pw".to_vec()),
        ),
        Err(AlgorithmError::Syntax(_))
    ));
}

#[test]
fn kdf_import_non_derive_usage_is_syntax_error() {
    assert!(matches!(
        ops::import_key(
            KeyFormat::Raw,
            NormalizedAlgorithm::Hkdf,
            false,
            vec![KeyUsage::Sign],
            KeyData::Raw(b"ikm".to_vec()),
        ),
        Err(AlgorithmError::Syntax(_))
    ));
}

#[test]
fn kdf_import_empty_usages_is_syntax_error() {
    assert!(matches!(
        ops::import_key(
            KeyFormat::Raw,
            NormalizedAlgorithm::Hkdf,
            false,
            vec![],
            KeyData::Raw(b"ikm".to_vec()),
        ),
        Err(AlgorithmError::Syntax(_))
    ));
}

#[test]
fn kdf_import_non_raw_format_is_not_supported() {
    // §33.4.2 / §34.4.2 register only the "raw" format.
    assert!(matches!(
        ops::import_key(
            KeyFormat::Pkcs8,
            NormalizedAlgorithm::Hkdf,
            false,
            vec![KeyUsage::DeriveBits],
            KeyData::Raw(b"ikm".to_vec()),
        ),
        Err(AlgorithmError::NotSupported(_))
    ));
}

// --- deriveKey compose (§14.3.7) -------------------------------------------

#[test]
fn derive_key_pbkdf2_to_aes_gcm_kat() {
    // PBKDF2-HMAC-SHA-256("passwd", "salt", 1) → AES-256-GCM key: the
    // material is the first 32 bytes of the RFC 7914 §11 dkLen=64 vector
    // (PBKDF2 is prefix-stable in dkLen).
    let base = import_kdf(
        NormalizedAlgorithm::Pbkdf2,
        b"passwd",
        vec![KeyUsage::DeriveKey],
    );
    let derived = ops::derive_key(
        pbkdf2_params(HashAlgorithm::Sha256, b"salt", 1),
        &base,
        NormalizedAlgorithm::AesImport {
            variant: AesVariant::Gcm,
        },
        NormalizedAlgorithm::AesKeyGen {
            variant: AesVariant::Gcm,
            length: 256,
        },
        true,
        vec![KeyUsage::Encrypt, KeyUsage::Decrypt],
    )
    .unwrap();
    assert_eq!(
        derived.algorithm,
        KeyAlgorithm::Aes {
            variant: AesVariant::Gcm,
            length: 256
        }
    );
    assert_eq!(
        to_hex(derived.material.as_bytes()),
        "55ac046e56e3089fec1691c22544b605f94185216dde0465e68b9d57c20dacbc"
    );
}

#[test]
fn derive_key_hkdf_to_hmac_signs() {
    // HKDF → HMAC-SHA-256 (length from get-key-length = block size 512 bits =
    // 64 bytes), then the derived key signs + verifies.
    let base = import_kdf(
        NormalizedAlgorithm::Hkdf,
        &[0x0b; 22],
        vec![KeyUsage::DeriveKey],
    );
    let derived = ops::derive_key(
        NormalizedAlgorithm::HkdfParams {
            hash: HashAlgorithm::Sha256,
            salt: super::from_hex("000102030405060708090a0b0c"),
            info: super::from_hex("f0f1f2f3f4f5f6f7f8f9"),
        },
        &base,
        NormalizedAlgorithm::HmacKeyParams {
            hash: HashAlgorithm::Sha256,
            length: None,
        },
        NormalizedAlgorithm::HmacKeyParams {
            hash: HashAlgorithm::Sha256,
            length: None,
        },
        true,
        vec![KeyUsage::Sign, KeyUsage::Verify],
    )
    .unwrap();
    assert_eq!(
        derived.algorithm,
        KeyAlgorithm::Hmac {
            hash: HashAlgorithm::Sha256,
            length: 512
        }
    );
    assert_eq!(derived.material.as_bytes().len(), 64);
    let sig = ops::sign(NormalizedAlgorithm::Hmac, &derived, b"hi", no_rng).unwrap();
    assert!(ops::verify(NormalizedAlgorithm::Hmac, &derived, &sig, b"hi").unwrap());
}

#[test]
fn derive_key_name_mismatch_is_invalid_access() {
    // §14.3.7 step 12: a PBKDF2 derive algorithm on an HKDF base key.
    let base = import_kdf(NormalizedAlgorithm::Hkdf, b"ikm", vec![KeyUsage::DeriveKey]);
    let result = ops::derive_key(
        pbkdf2_params(HashAlgorithm::Sha256, b"salt", 1),
        &base,
        NormalizedAlgorithm::AesImport {
            variant: AesVariant::Gcm,
        },
        NormalizedAlgorithm::AesKeyGen {
            variant: AesVariant::Gcm,
            length: 128,
        },
        true,
        vec![KeyUsage::Encrypt],
    );
    assert!(matches!(result, Err(AlgorithmError::InvalidAccess(_))));
}

#[test]
fn derive_key_missing_derive_key_usage_is_invalid_access() {
    // §14.3.7 step 13: the base key lacks the "deriveKey" usage.
    let base = import_kdf(
        NormalizedAlgorithm::Hkdf,
        b"ikm",
        vec![KeyUsage::DeriveBits],
    );
    let result = ops::derive_key(
        NormalizedAlgorithm::HkdfParams {
            hash: HashAlgorithm::Sha256,
            salt: b"s".to_vec(),
            info: b"i".to_vec(),
        },
        &base,
        NormalizedAlgorithm::AesImport {
            variant: AesVariant::Gcm,
        },
        NormalizedAlgorithm::AesKeyGen {
            variant: AesVariant::Gcm,
            length: 128,
        },
        true,
        vec![KeyUsage::Encrypt],
    );
    assert!(matches!(result, Err(AlgorithmError::InvalidAccess(_))));
}

#[test]
fn derive_key_empty_usages_is_syntax_error() {
    // §14.3.7 step 17 (via the importKey generic empty-usages step): the
    // derived secret key has empty usages.
    let base = import_kdf(
        NormalizedAlgorithm::Hkdf,
        &[0x0b; 22],
        vec![KeyUsage::DeriveKey],
    );
    let result = ops::derive_key(
        NormalizedAlgorithm::HkdfParams {
            hash: HashAlgorithm::Sha256,
            salt: b"s".to_vec(),
            info: b"i".to_vec(),
        },
        &base,
        NormalizedAlgorithm::AesImport {
            variant: AesVariant::Gcm,
        },
        NormalizedAlgorithm::AesKeyGen {
            variant: AesVariant::Gcm,
            length: 128,
        },
        true,
        vec![],
    );
    assert!(matches!(result, Err(AlgorithmError::Syntax(_))));
}

#[test]
fn derive_key_kdf_derived_key_type_degenerates_to_operation_error() {
    // §14.3.7 step 14/15: a KDF `derivedKeyType` gives get-key-length = null,
    // so derive-bits sees length = None → OperationError (spec-correct: there
    // is no fixed-length KDF key to derive).
    let base = import_kdf(
        NormalizedAlgorithm::Pbkdf2,
        b"pw",
        vec![KeyUsage::DeriveKey],
    );
    let result = ops::derive_key(
        pbkdf2_params(HashAlgorithm::Sha256, b"salt", 1),
        &base,
        NormalizedAlgorithm::Hkdf,
        NormalizedAlgorithm::Hkdf,
        false,
        vec![KeyUsage::DeriveBits],
    );
    assert!(matches!(result, Err(AlgorithmError::Operation(_))));
}
