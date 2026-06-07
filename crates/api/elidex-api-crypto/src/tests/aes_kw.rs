//! AES-KW key wrap / unwrap (WebCrypto §30) + the generic wrapKey / unwrapKey
//! composition (§14.3.11 / §14.3.12).
//!
//! Low-level RFC 3394 §4 known-answer vectors validate the `aes_kw` module;
//! the `ops_*` tests validate the spec-validation layer (usage gating, the
//! name-match + extractable gates, the export→wrap pipeline, and the
//! encrypt/decrypt fallback).

use super::{fill_seq, from_hex, to_hex};
use crate::aes_kw;
use crate::algorithm::{
    is_supported, normalize, params_shape, AesVariant, AlgorithmParams, NormalizedAlgorithm,
    Operation, RawAlgorithm,
};
use crate::error::AlgorithmError;
use crate::key::{KeyAlgorithm, KeyUsage};
use crate::ops::{self, ExportedKey, KeyData, KeyFormat};

// ===========================================================================
// RFC 3394 §4 known-answer vectors (Key Wrap with the default IV)
// ===========================================================================

/// RFC 3394 §4.1: wrap a 128-bit key with a 128-bit KEK.
#[test]
fn rfc3394_4_1_128kek_128key() {
    let kek = from_hex("000102030405060708090a0b0c0d0e0f");
    let key = from_hex("00112233445566778899aabbccddeeff");
    let expected = "1fa68b0a8112b447aef34bd8fb5a7b829d3e862371d2cfe5";
    let wrapped = aes_kw::wrap(&kek, &key).unwrap();
    assert_eq!(to_hex(&wrapped), expected);
    assert_eq!(
        to_hex(&aes_kw::unwrap(&kek, &wrapped).unwrap()),
        to_hex(&key)
    );
}

/// RFC 3394 §4.4: wrap a 192-bit key with a 192-bit KEK.
#[test]
fn rfc3394_4_4_192kek_192key() {
    let kek = from_hex("000102030405060708090a0b0c0d0e0f1011121314151617");
    let key = from_hex("00112233445566778899aabbccddeeff0001020304050607");
    let expected = "031d33264e15d33268f24ec260743edce1c6c7ddee725a936ba814915c6762d2";
    let wrapped = aes_kw::wrap(&kek, &key).unwrap();
    assert_eq!(to_hex(&wrapped), expected);
    assert_eq!(
        to_hex(&aes_kw::unwrap(&kek, &wrapped).unwrap()),
        to_hex(&key)
    );
}

/// RFC 3394 §4.6: wrap a 256-bit key with a 256-bit KEK.
#[test]
fn rfc3394_4_6_256kek_256key() {
    let kek = from_hex("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f");
    let key = from_hex("00112233445566778899aabbccddeeff000102030405060708090a0b0c0d0e0f");
    let expected =
        "28c9f404c4b810f4cbccb35cfb87f8263f5786e2d80ed326cbc7f0e71a99f43bfb988b9b7a02dd21";
    let wrapped = aes_kw::wrap(&kek, &key).unwrap();
    assert_eq!(to_hex(&wrapped), expected);
    assert_eq!(
        to_hex(&aes_kw::unwrap(&kek, &wrapped).unwrap()),
        to_hex(&key)
    );
}

// ===========================================================================
// AES-KW length + integrity error paths (WebCrypto §30.3.1 / §30.3.2)
// ===========================================================================

/// §30.3.1 step 1: a plaintext that is not a multiple of 64 bits (8 bytes) is
/// an OperationError.
#[test]
fn wrap_non_multiple_of_64_bits_is_operation_error() {
    let kek = from_hex("000102030405060708090a0b0c0d0e0f");
    for len in [1usize, 5, 7, 9, 15] {
        assert!(
            matches!(
                aes_kw::wrap(&kek, &vec![0u8; len]),
                Err(AlgorithmError::Operation(_))
            ),
            "wrapping {len} bytes should be an OperationError"
        );
    }
}

/// §30.3.2 step 2: a tampered wrapped key fails the RFC 3394 integrity check →
/// OperationError.
#[test]
fn unwrap_tampered_is_operation_error() {
    let kek = from_hex("000102030405060708090a0b0c0d0e0f");
    let key = from_hex("00112233445566778899aabbccddeeff");
    let mut wrapped = aes_kw::wrap(&kek, &key).unwrap();
    wrapped[0] ^= 0x01;
    assert!(matches!(
        aes_kw::unwrap(&kek, &wrapped),
        Err(AlgorithmError::Operation(_))
    ));
}

/// §30.3.2: a wrong KEK fails the integrity check → OperationError (no
/// plaintext is returned).
#[test]
fn unwrap_wrong_kek_is_operation_error() {
    let kek = from_hex("000102030405060708090a0b0c0d0e0f");
    let other = from_hex("0f0e0d0c0b0a09080706050403020100");
    let wrapped = aes_kw::wrap(&kek, &from_hex("00112233445566778899aabbccddeeff")).unwrap();
    assert!(matches!(
        aes_kw::unwrap(&other, &wrapped),
        Err(AlgorithmError::Operation(_))
    ));
}

/// §30.3.2: a wrapped input that is not a multiple of 64 bits (or too short)
/// is an OperationError, not a panic.
#[test]
fn unwrap_bad_length_is_operation_error() {
    let kek = from_hex("000102030405060708090a0b0c0d0e0f");
    for len in [0usize, 4, 8, 12, 20] {
        assert!(
            matches!(
                aes_kw::unwrap(&kek, &vec![0u8; len]),
                Err(AlgorithmError::Operation(_))
            ),
            "unwrapping {len} bytes should be an OperationError"
        );
    }
}

// ===========================================================================
// AES-KW key vertical through the ops layer (generate / import / export /
// get-key-length) — reuses the AES arms via `AesVariant::Kw`
// ===========================================================================

fn aes_kw_gen(length: u32) -> NormalizedAlgorithm {
    NormalizedAlgorithm::AesKeyGen {
        variant: AesVariant::Kw,
        length,
    }
}

#[test]
fn ops_aes_kw_generate_sets_key_algorithm() {
    let key = ops::generate_key(
        aes_kw_gen(256),
        true,
        vec![KeyUsage::WrapKey, KeyUsage::UnwrapKey],
        fill_seq,
    )
    .unwrap();
    assert!(matches!(
        key.algorithm,
        KeyAlgorithm::Aes {
            variant: AesVariant::Kw,
            length: 256
        }
    ));
}

/// §30.3.3 step 1: AES-KW accepts ONLY {wrapKey, unwrapKey} — `encrypt` /
/// `decrypt` (valid for the block-cipher modes) are a SyntaxError here.
#[test]
fn ops_aes_kw_generate_encrypt_usage_is_syntax_error() {
    for usage in [KeyUsage::Encrypt, KeyUsage::Decrypt, KeyUsage::Sign] {
        assert!(
            matches!(
                ops::generate_key(aes_kw_gen(128), true, vec![usage], fill_seq),
                Err(AlgorithmError::Syntax(_))
            ),
            "AES-KW usage {usage:?} should be a SyntaxError"
        );
    }
}

#[test]
fn ops_aes_kw_import_raw_then_export_jwk_roundtrip() {
    let key = ops::import_key(
        KeyFormat::Raw,
        NormalizedAlgorithm::AesImport {
            variant: AesVariant::Kw,
        },
        true,
        vec![KeyUsage::WrapKey, KeyUsage::UnwrapKey],
        KeyData::Raw(vec![0x42u8; 24]),
    )
    .unwrap();
    assert!(matches!(
        key.algorithm,
        KeyAlgorithm::Aes {
            variant: AesVariant::Kw,
            length: 192
        }
    ));
    let ExportedKey::Jwk(jwk) = ops::export_key(KeyFormat::Jwk, &key).unwrap() else {
        panic!("expected a JWK export");
    };
    assert_eq!(jwk.kty.as_deref(), Some("oct"));
    assert_eq!(jwk.alg.as_deref(), Some("A192KW"));
    // Re-import the JWK and confirm the material round-trips.
    let reimported = ops::import_key(
        KeyFormat::Jwk,
        NormalizedAlgorithm::AesImport {
            variant: AesVariant::Kw,
        },
        true,
        vec![KeyUsage::WrapKey, KeyUsage::UnwrapKey],
        KeyData::Jwk(jwk),
    )
    .unwrap();
    assert_eq!(reimported.material, key.material);
}

/// §30.3.6: AES-KW get-key-length validates 128/192/256 (else OperationError),
/// reusing the AES `AesKeyGen{Kw,length}` form.
#[test]
fn ops_aes_kw_get_key_length() {
    for length in [128u32, 192, 256] {
        assert_eq!(
            ops::get_key_length(aes_kw_gen(length)).unwrap(),
            Some(length)
        );
    }
    assert!(matches!(
        ops::get_key_length(aes_kw_gen(200)),
        Err(AlgorithmError::Operation(_))
    ));
}

// ===========================================================================
// wrapKey / unwrapKey composition (WebCrypto §14.3.11 / §14.3.12)
// ===========================================================================

/// A KEK + a key-to-wrap, both AES through the ops layer.
fn import_raw(
    variant: AesVariant,
    material: Vec<u8>,
    usages: Vec<KeyUsage>,
) -> crate::CryptoKeyData {
    ops::import_key(
        KeyFormat::Raw,
        NormalizedAlgorithm::AesImport { variant },
        true,
        usages,
        KeyData::Raw(material),
    )
    .unwrap()
}

/// raw wrap/unwrap round-trip via the AES-KW path: export the key as raw bytes
/// (already a multiple of 8), RFC 3394-wrap, then unwrap back to the same bytes.
#[test]
fn ops_wrap_unwrap_raw_aes_kw_roundtrip() {
    let kek = import_raw(
        AesVariant::Kw,
        vec![0x11u8; 32],
        vec![KeyUsage::WrapKey, KeyUsage::UnwrapKey],
    );
    let key = import_raw(
        AesVariant::Gcm,
        vec![0x22u8; 16],
        vec![KeyUsage::Encrypt, KeyUsage::Decrypt],
    );
    let wrapped = ops::wrap_key(
        NormalizedAlgorithm::AesKwWrap,
        &kek,
        &key,
        KeyFormat::Raw,
        |_| unreachable!("raw format does not serialize a JWK"),
    )
    .unwrap();
    // RFC 3394 output is one 64-bit semiblock longer than the 16-byte key.
    assert_eq!(wrapped.len(), 24);
    let unwrapped = ops::unwrap_key(NormalizedAlgorithm::AesKwWrap, &kek, &wrapped).unwrap();
    assert_eq!(unwrapped, vec![0x22u8; 16]);
}

/// The §14.3.11 jwk path invokes the serialize closure with the exported oct
/// JWK and wraps its bytes; unwrap recovers them.  (The real JSON round-trip is
/// VM-side; here a controlled 16-byte serialization stands in.)
#[test]
fn ops_wrap_key_jwk_invokes_serializer() {
    let kek = import_raw(
        AesVariant::Kw,
        vec![0x33u8; 16],
        vec![KeyUsage::WrapKey, KeyUsage::UnwrapKey],
    );
    let key = import_raw(
        AesVariant::Cbc,
        vec![0x44u8; 16],
        vec![KeyUsage::Encrypt, KeyUsage::Decrypt],
    );
    let serialized = b"0123456789abcdef".to_vec(); // 16 bytes, a multiple of 8
    let wrapped = ops::wrap_key(
        NormalizedAlgorithm::AesKwWrap,
        &kek,
        &key,
        KeyFormat::Jwk,
        |jwk| {
            assert_eq!(jwk.kty.as_deref(), Some("oct"));
            assert_eq!(jwk.alg.as_deref(), Some("A128CBC"));
            serialized.clone()
        },
    )
    .unwrap();
    let unwrapped = ops::unwrap_key(NormalizedAlgorithm::AesKwWrap, &kek, &wrapped).unwrap();
    assert_eq!(unwrapped, b"0123456789abcdef");
}

/// The §14.3.11 step-15 encrypt fallback: an AES-GCM wrapping key wraps via its
/// encrypt operation; §14.3.12 step-14 decrypts.  Round-trips through the ops.
#[test]
fn ops_wrap_unwrap_aes_gcm_encrypt_fallback() {
    let kek = import_raw(
        AesVariant::Gcm,
        vec![0x55u8; 32],
        vec![KeyUsage::WrapKey, KeyUsage::UnwrapKey],
    );
    let key = import_raw(
        AesVariant::Ctr,
        vec![0x66u8; 16],
        vec![KeyUsage::Encrypt, KeyUsage::Decrypt],
    );
    let gcm = || NormalizedAlgorithm::AesGcm {
        iv: vec![0x07u8; 12],
        additional_data: None,
        tag_length: 128,
    };
    let wrapped = ops::wrap_key(gcm(), &kek, &key, KeyFormat::Raw, |_| {
        unreachable!("raw format does not serialize a JWK")
    })
    .unwrap();
    let unwrapped = ops::unwrap_key(gcm(), &kek, &wrapped).unwrap();
    assert_eq!(unwrapped, vec![0x66u8; 16]);
}

/// §14.3.11 step 10: a wrapping key without the `wrapKey` usage is an
/// InvalidAccessError.
#[test]
fn ops_wrap_key_missing_wrap_usage_is_invalid_access() {
    let kek = import_raw(AesVariant::Kw, vec![0x11u8; 16], vec![KeyUsage::UnwrapKey]);
    let key = import_raw(AesVariant::Gcm, vec![0x22u8; 16], vec![KeyUsage::Encrypt]);
    assert!(matches!(
        ops::wrap_key(
            NormalizedAlgorithm::AesKwWrap,
            &kek,
            &key,
            KeyFormat::Raw,
            |_| Vec::new()
        ),
        Err(AlgorithmError::InvalidAccess(_))
    ));
}

/// §14.3.11 step 12: a non-extractable key cannot be wrapped (wrap effectively
/// exports) → InvalidAccessError, after the wrappingKey gate passes.
#[test]
fn ops_wrap_key_non_extractable_is_invalid_access() {
    let kek = import_raw(
        AesVariant::Kw,
        vec![0x11u8; 16],
        vec![KeyUsage::WrapKey, KeyUsage::UnwrapKey],
    );
    let key = ops::import_key(
        KeyFormat::Raw,
        NormalizedAlgorithm::AesImport {
            variant: AesVariant::Gcm,
        },
        false, // non-extractable
        vec![KeyUsage::Encrypt],
        KeyData::Raw(vec![0x22u8; 16]),
    )
    .unwrap();
    assert!(matches!(
        ops::wrap_key(
            NormalizedAlgorithm::AesKwWrap,
            &kek,
            &key,
            KeyFormat::Raw,
            |_| Vec::new()
        ),
        Err(AlgorithmError::InvalidAccess(_))
    ));
}

/// §14.3.11 step order: the wrappingKey name gate (step 9, InvalidAccessError)
/// runs BEFORE the key export-support check (step 11, NotSupportedError) — so
/// an AES-KW algorithm against an AES-GCM wrapping key is InvalidAccess even
/// when the key-to-wrap is a non-exportable HKDF key.
#[test]
fn ops_wrap_key_name_mismatch_precedes_export_support() {
    let gcm_kek = import_raw(
        AesVariant::Gcm,
        vec![0x11u8; 16],
        vec![KeyUsage::WrapKey, KeyUsage::UnwrapKey],
    );
    let hkdf_key = ops::import_key(
        KeyFormat::Raw,
        NormalizedAlgorithm::Hkdf,
        false,
        vec![KeyUsage::DeriveBits],
        KeyData::Raw(vec![0x22u8; 16]),
    )
    .unwrap();
    // AesKwWrap (name "AES-KW") against an AES-GCM wrapping key.
    assert!(matches!(
        ops::wrap_key(
            NormalizedAlgorithm::AesKwWrap,
            &gcm_kek,
            &hkdf_key,
            KeyFormat::Raw,
            |_| Vec::new()
        ),
        Err(AlgorithmError::InvalidAccess(_))
    ));
}

/// §14.3.12 step 12: an unwrapping key without the `unwrapKey` usage is an
/// InvalidAccessError.
#[test]
fn ops_unwrap_key_missing_unwrap_usage_is_invalid_access() {
    let kek = import_raw(AesVariant::Kw, vec![0x11u8; 16], vec![KeyUsage::WrapKey]);
    assert!(matches!(
        ops::unwrap_key(NormalizedAlgorithm::AesKwWrap, &kek, &[0u8; 24]),
        Err(AlgorithmError::InvalidAccess(_))
    ));
}

// ===========================================================================
// Registry: AES-KW is registered for wrap/unwrap/generate/import/get-key-length
// but NOT encrypt/decrypt (WebCrypto §30)
// ===========================================================================

#[test]
fn registry_aes_kw_operations() {
    assert!(is_supported(Operation::WrapKey, "AES-KW"));
    assert!(is_supported(Operation::UnwrapKey, "AES-KW"));
    assert!(is_supported(Operation::GenerateKey, "AES-KW"));
    assert!(is_supported(Operation::ImportKey, "AES-KW"));
    assert!(is_supported(Operation::GetKeyLength, "AES-KW"));
    // AES-KW is wrap-only: no encrypt/decrypt operation.
    assert!(!is_supported(Operation::Encrypt, "AES-KW"));
    assert!(!is_supported(Operation::Decrypt, "AES-KW"));
    // Case-insensitive recognition (WebCrypto §18.4.4).
    assert!(is_supported(Operation::WrapKey, "aes-kw"));
}

#[test]
fn registry_aes_kw_wrap_is_name_only() {
    // wrapKey / unwrapKey carry no params dictionary (the RFC 3394 default IV).
    assert_eq!(
        params_shape(Operation::WrapKey, "AES-KW"),
        Some(AlgorithmParams::NameOnly)
    );
    let normalized = normalize(Operation::WrapKey, RawAlgorithm::from_name("AES-KW")).unwrap();
    assert_eq!(normalized, NormalizedAlgorithm::AesKwWrap);
    // The other block-cipher modes do NOT register wrap/unwrap (they reach the
    // wrap surface only through the §14.3.11 encrypt fallback).
    assert!(!is_supported(Operation::WrapKey, "AES-GCM"));
}
