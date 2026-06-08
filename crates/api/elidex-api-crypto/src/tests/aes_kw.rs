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

/// §30.3.1 step 1 + NIST SP 800-38F §5.3.1: a plaintext that is not a multiple
/// of 64 bits, OR is fewer than two 64-bit semiblocks (< 16 bytes — e.g. an
/// exported 8-byte HMAC key), is an OperationError.  `8` is a multiple of 8 but
/// a single semiblock, so it must still reject (the `aes-kw` crate would
/// otherwise emit a nonstandard 16-byte wrap).
#[test]
fn wrap_invalid_length_is_operation_error() {
    let kek = from_hex("000102030405060708090a0b0c0d0e0f");
    for len in [0usize, 1, 5, 7, 8, 9, 15] {
        assert!(
            matches!(
                aes_kw::wrap(&kek, &vec![0u8; len]),
                Err(AlgorithmError::Operation(_))
            ),
            "wrapping {len} bytes should be an OperationError"
        );
    }
    // The smallest valid AES-KW input is two semiblocks (16 bytes).
    assert!(aes_kw::wrap(&kek, &[0u8; 16]).is_ok());
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

/// §30.3.2 + NIST SP 800-38F: a wrapped input that is not a multiple of 64 bits
/// OR fewer than three semiblocks (< 24 bytes — IV + two plaintext semiblocks)
/// is an OperationError, not a panic.  `8` (the bare IV) and `16` (IV + one
/// semiblock — the `aes-kw` crate's nonstandard single-block wrap) are multiples
/// of 8 but still too short, so they must reject.
#[test]
fn unwrap_invalid_length_is_operation_error() {
    let kek = from_hex("000102030405060708090a0b0c0d0e0f");
    for len in [0usize, 4, 8, 12, 16, 20] {
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
    let key = super::expect_single(ops::generate_key(
        aes_kw_gen(256),
        true,
        vec![KeyUsage::WrapKey, KeyUsage::UnwrapKey],
        fill_seq,
    ));
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
        // `jwk` is already a `Box<JsonWebKey>` (the `ExportedKey::Jwk` payload).
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
    let wrapped =
        ops::wrap_key(NormalizedAlgorithm::AesKwWrap, &kek, &key, KeyFormat::Raw).unwrap();
    // RFC 3394 output is one 64-bit semiblock longer than the 16-byte key.
    assert_eq!(wrapped.len(), 24);
    let unwrapped = ops::unwrap_key(NormalizedAlgorithm::AesKwWrap, &kek, &wrapped).unwrap();
    assert_eq!(unwrapped, vec![0x22u8; 16]);
}

/// The `jwk` wrap path serializes the exported oct JWK to JSON in-crate
/// (realm-isolated, §14.3.11 step 14) and wraps the bytes; unwrap recovers the
/// JSON, which parses back to the same `oct` JWK.  Uses AES-GCM (the encrypt
/// fallback handles arbitrary lengths — an AES-KW wrap of the JSON would
/// usually fail §30.3.1's multiple-of-64-bits rule, see the dedicated test).
#[test]
fn ops_wrap_unwrap_jwk_roundtrip_via_gcm() {
    let kek = import_raw(
        AesVariant::Gcm,
        vec![0x33u8; 32],
        vec![KeyUsage::WrapKey, KeyUsage::UnwrapKey],
    );
    let key = import_raw(
        AesVariant::Cbc,
        vec![0x44u8; 16],
        vec![KeyUsage::Encrypt, KeyUsage::Decrypt],
    );
    let gcm = || NormalizedAlgorithm::AesGcm {
        iv: vec![0x07u8; 12],
        additional_data: None,
        tag_length: 128,
    };
    let wrapped = ops::wrap_key(gcm(), &kek, &key, KeyFormat::Jwk).unwrap();
    let json = ops::unwrap_key(gcm(), &kek, &wrapped).unwrap();
    let jwk = crate::jwk::from_json_bytes(&json).unwrap();
    assert_eq!(jwk.kty.as_deref(), Some("oct"));
    assert_eq!(jwk.alg.as_deref(), Some("A128CBC"));
    // Re-import the recovered JWK and confirm the key material round-trips.
    let reimported = ops::import_key(
        KeyFormat::Jwk,
        NormalizedAlgorithm::AesImport {
            variant: AesVariant::Cbc,
        },
        true,
        vec![KeyUsage::Encrypt, KeyUsage::Decrypt],
        KeyData::Jwk(Box::new(jwk)),
    )
    .unwrap();
    assert_eq!(reimported.material, key.material);
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
    let wrapped = ops::wrap_key(gcm(), &kek, &key, KeyFormat::Raw).unwrap();
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
        ops::wrap_key(NormalizedAlgorithm::AesKwWrap, &kek, &key, KeyFormat::Raw),
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
        ops::wrap_key(NormalizedAlgorithm::AesKwWrap, &kek, &key, KeyFormat::Raw),
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
            KeyFormat::Raw
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
// JWK JSON round-trip (WebCrypto §14.3.11 step 14 / §9 parse-a-JWK) — done in
// the crate over the `JsonWebKey` struct, isolated from any JS realm
// ===========================================================================

#[test]
fn jwk_json_round_trip_omits_absent_members() {
    let jwk = crate::JsonWebKey {
        kty: Some("oct".to_string()),
        k: Some("AAECAwQFBgcICQoLDA0ODw".to_string()),
        alg: Some("A128KW".to_string()),
        use_: None,
        key_ops: Some(vec!["wrapKey".to_string(), "unwrapKey".to_string()]),
        ext: Some(true),
        ..Default::default()
    };
    let json = crate::jwk::to_json_bytes(&jwk);
    let text = std::str::from_utf8(&json).unwrap();
    // Absent `use` is omitted (skip_serializing_if), `use` member name is the
    // `use_` field renamed.
    assert!(
        !text.contains("\"use\""),
        "absent `use` must be omitted: {text}"
    );
    assert!(text.contains("\"key_ops\""));
    assert_eq!(crate::jwk::from_json_bytes(&json).unwrap(), jwk);
}

#[test]
fn jwk_from_json_ignores_unknown_members() {
    // A full (EC) JWK parses to its `oct` subset; the EC fields are ignored,
    // not rejected (so a wrapped key from another impl round-trips).
    let json = br#"{"kty":"oct","k":"AAEC","crv":"P-256","x":"abc","ext":false}"#;
    let jwk = crate::jwk::from_json_bytes(json).unwrap();
    assert_eq!(jwk.kty.as_deref(), Some("oct"));
    assert_eq!(jwk.k.as_deref(), Some("AAEC"));
    assert_eq!(jwk.ext, Some(false));
}

/// Codex R2 regression: an explicit `null` is a *present* member converted per
/// WebIDL, NOT an absent one.  `ext: null` → `Some(false)` (ToBoolean), so a
/// wrapped `{…,"ext":null}` unwrapped with `extractable=true` is rejected
/// (DataError) exactly as the normal `importKey` path rejects it — not silently
/// accepted.
#[test]
fn jwk_from_json_present_null_ext_is_false() {
    let jwk =
        crate::jwk::from_json_bytes(br#"{"kty":"oct","k":"AAECAwQFBgcICQoLDA0ODw","ext":null}"#)
            .unwrap();
    assert_eq!(jwk.ext, Some(false));
    // End-to-end: importing it as a 128-bit AES-GCM key with extractable=true
    // is a DataError (ext=false cannot satisfy extractable=true).
    assert!(matches!(
        ops::import_key(
            KeyFormat::Jwk,
            NormalizedAlgorithm::AesImport {
                variant: AesVariant::Gcm,
            },
            true,
            vec![KeyUsage::Encrypt],
            KeyData::Jwk(Box::new(jwk)),
        ),
        Err(AlgorithmError::Data(_))
    ));
}

/// Codex R2 regression: a present non-array `key_ops` (incl. explicit `null`)
/// is not a sequence → `TypeError` (matching `importKey`'s
/// `webidl_sequence_to_vec` step-1, so it cannot silently bypass the key_ops
/// usage-superset check), whereas an absent `key_ops` is `None`.
#[test]
fn jwk_from_json_present_non_array_key_ops_is_type_error() {
    for body in [
        &br#"{"kty":"oct","k":"AAEC","key_ops":null}"#[..],
        &br#"{"kty":"oct","k":"AAEC","key_ops":"sign"}"#[..],
        &br#"{"kty":"oct","k":"AAEC","key_ops":{}}"#[..],
    ] {
        assert!(
            matches!(
                crate::jwk::from_json_bytes(body),
                Err(AlgorithmError::Type(_))
            ),
            "non-array key_ops should be a TypeError: {:?}",
            std::str::from_utf8(body)
        );
    }
    // Absent key_ops parses cleanly to None.
    let jwk = crate::jwk::from_json_bytes(br#"{"kty":"oct","k":"AAEC"}"#).unwrap();
    assert_eq!(jwk.key_ops, None);
}

/// Codex R3 regression: a `DOMString` JWK member that parses to a JSON array /
/// object is coerced via ECMAScript `ToString` (matching the live `importKey`
/// path), NOT serialized as JSON text — so `kty:["oct"]` coerces to `"oct"`,
/// `["a","b"]` → `"a,b"`, `[null]` → `""`, and an object → `"[object Object]"`.
#[test]
fn jwk_from_json_domstring_array_coercion_matches_tostring() {
    let jwk =
        crate::jwk::from_json_bytes(br#"{"kty":["oct"],"k":["AAEC"],"alg":["a","b"]}"#).unwrap();
    assert_eq!(jwk.kty.as_deref(), Some("oct"));
    assert_eq!(jwk.k.as_deref(), Some("AAEC"));
    assert_eq!(jwk.alg.as_deref(), Some("a,b"));
    // `[null]` joins to "" and a `{}` member stringifies to "[object Object]".
    let jwk2 = crate::jwk::from_json_bytes(br#"{"kty":[null],"k":{}}"#).unwrap();
    assert_eq!(jwk2.kty.as_deref(), Some(""));
    assert_eq!(jwk2.k.as_deref(), Some("[object Object]"));
}

/// Codex R-batch regression: `to_json_bytes` pads the serialization to a
/// multiple of the AES-KW semiblock (8 bytes) with trailing JSON whitespace
/// (§14.3.11 step-14 Note), so a `jwk` key can be AES-KW-wrapped; the padding
/// is still valid JSON that `from_json_bytes` parses.
#[test]
fn jwk_to_json_bytes_is_padded_to_aes_kw_block() {
    let jwk = crate::JsonWebKey {
        kty: Some("oct".to_string()),
        k: Some("AAECAwQFBgcICQoLDA0ODw".to_string()),
        alg: Some("A128KW".to_string()),
        use_: None,
        key_ops: Some(vec!["wrapKey".to_string()]),
        ext: Some(true),
        ..Default::default()
    };
    let bytes = crate::jwk::to_json_bytes(&jwk);
    assert_eq!(
        bytes.len() % 8,
        0,
        "JWK JSON must be padded to a multiple of 8"
    );
    // Trailing whitespace is ignored on parse → round-trips.
    assert_eq!(crate::jwk::from_json_bytes(&bytes).unwrap(), jwk);
}

/// Codex R-batch regression: AES-KW can now wrap a `jwk` key (the JSON is
/// padded to a multiple of 64 bits), round-tripping through wrap → unwrap →
/// parse.
#[test]
fn ops_wrap_unwrap_jwk_roundtrip_via_aes_kw() {
    let kek = import_raw(
        AesVariant::Kw,
        vec![0x33u8; 32],
        vec![KeyUsage::WrapKey, KeyUsage::UnwrapKey],
    );
    let key = import_raw(
        AesVariant::Cbc,
        vec![0x44u8; 16],
        vec![KeyUsage::Encrypt, KeyUsage::Decrypt],
    );
    let wrapped =
        ops::wrap_key(NormalizedAlgorithm::AesKwWrap, &kek, &key, KeyFormat::Jwk).unwrap();
    assert_eq!(wrapped.len() % 8, 0);
    let json = ops::unwrap_key(NormalizedAlgorithm::AesKwWrap, &kek, &wrapped).unwrap();
    let jwk = crate::jwk::from_json_bytes(&json).unwrap();
    assert_eq!(jwk.kty.as_deref(), Some("oct"));
    assert_eq!(jwk.alg.as_deref(), Some("A128CBC"));
}

/// Codex R-batch regression: `from_json_bytes` converts (for validation) the
/// `oth` member like the live `importKey` path — a non-array `oth` or a
/// primitive entry is a `TypeError`; an array of objects / `null` (incl. an
/// empty array) is accepted (the values are discarded for `oct` keys).
#[test]
fn jwk_from_json_oth_matches_import_validation() {
    // Malformed `oth` → TypeError.
    for bad in [
        &br#"{"kty":"oct","k":"AAEC","oth":123}"#[..],
        &br#"{"kty":"oct","k":"AAEC","oth":"x"}"#[..],
        &br#"{"kty":"oct","k":"AAEC","oth":[123]}"#[..],
        &br#"{"kty":"oct","k":"AAEC","oth":["s"]}"#[..],
    ] {
        assert!(
            matches!(
                crate::jwk::from_json_bytes(bad),
                Err(AlgorithmError::Type(_))
            ),
            "malformed oth should be a TypeError: {:?}",
            std::str::from_utf8(bad)
        );
    }
    // Well-formed / empty `oth` is accepted (ignored for oct keys).
    for ok in [
        &br#"{"kty":"oct","k":"AAEC","oth":[]}"#[..],
        &br#"{"kty":"oct","k":"AAEC","oth":[{"r":"x","d":"y","t":"z"}]}"#[..],
        &br#"{"kty":"oct","k":"AAEC","oth":[null]}"#[..],
    ] {
        assert!(
            crate::jwk::from_json_bytes(ok).is_ok(),
            "well-formed oth should parse: {:?}",
            std::str::from_utf8(ok)
        );
    }
}

#[test]
fn jwk_from_json_malformed_is_data_error() {
    for bad in [
        &b"not json"[..],         // invalid JSON syntax
        &b"{ \"kty\": }"[..],     // invalid JSON syntax
        &b"{}"[..],               // §9 step 6: object with no kty
        &b"{\"k\":\"AAEC\"}"[..], // §9 step 6: kty member missing
        &b"[1,2,3]"[..],          // array → empty dict → kty missing
        &b"null"[..],             // null → empty dict → kty missing
    ] {
        assert!(
            matches!(
                crate::jwk::from_json_bytes(bad),
                Err(AlgorithmError::Data(_))
            ),
            "malformed / kty-less JWK JSON should be a DataError: {:?}",
            std::str::from_utf8(bad)
        );
    }
}

/// Codex R4 regression (§9 step 5 / WebIDL §3.2.17): valid JSON that is a
/// primitive (string / number / boolean) is not a dictionary → `TypeError`
/// (NOT `DataError`), distinguishing it from `null` / arrays (missing-kty
/// `DataError`, covered above).
#[test]
fn jwk_from_json_primitive_is_type_error() {
    for bad in [&b"\"a-string\""[..], &b"123"[..], &b"true"[..]] {
        assert!(
            matches!(
                crate::jwk::from_json_bytes(bad),
                Err(AlgorithmError::Type(_))
            ),
            "a primitive JWK document should be a TypeError: {:?}",
            std::str::from_utf8(bad)
        );
    }
}

/// Codex R4 regression (§9 step 6): a `kty`-less JWK rejects with a parse-time
/// `DataError` regardless of the requested import algorithm — e.g. an HKDF
/// unwrap of `{}` is a DataError, not HKDF's later "jwk unsupported"
/// NotSupportedError.
#[test]
fn jwk_missing_kty_rejects_before_import() {
    assert!(matches!(
        crate::jwk::from_json_bytes(br#"{"k":"AAEC","alg":"A128KW"}"#),
        Err(AlgorithmError::Data(_))
    ));
}

/// Codex R4 regression (§9 step 5): numeric JWK members are `ToString`-ed as
/// the JS `f64` value, so `1` and `1.0` both become `"1"` (a duplicate
/// `key_ops` the WebIDL conversion would catch), not the source spelling.
#[test]
fn jwk_from_json_number_uses_js_f64_stringification() {
    let jwk =
        crate::jwk::from_json_bytes(br#"{"kty":"oct","k":"AAEC","key_ops":[1,1.0]}"#).unwrap();
    assert_eq!(jwk.key_ops, Some(vec!["1".to_string(), "1".to_string()]));
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
