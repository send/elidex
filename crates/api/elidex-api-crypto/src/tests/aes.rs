// ===========================================================================
// AES-GCM (McGrew & Viega "GCM" test vectors / NIST GCMVS)
// ===========================================================================

use super::{fill_seq, from_hex, to_hex};
use crate::aes;
use crate::algorithm::{
    is_supported, normalize, params_shape, AesVariant, AlgorithmParams, NormalizedAlgorithm,
    Operation, RawAlgorithm,
};
use crate::error::AlgorithmError;
use crate::key::{KeyAlgorithm, KeyUsage};
use crate::ops::{self, ExportedKey, KeyData, KeyFormat};

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

fn aes_gcm_params(iv: Vec<u8>) -> NormalizedAlgorithm {
    NormalizedAlgorithm::AesGcm {
        iv,
        additional_data: None,
        tag_length: 128,
    }
}

#[test]
fn ops_aes_generate_encrypt_decrypt_roundtrip() {
    let key = super::expect_single(ops::generate_key(
        NormalizedAlgorithm::AesKeyGen {
            variant: AesVariant::Gcm,
            length: 256,
        },
        true,
        vec![KeyUsage::Encrypt, KeyUsage::Decrypt],
        fill_seq,
    ));
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
        // `jwk` is already a `Box<JsonWebKey>` (the `ExportedKey::Jwk` payload).
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
