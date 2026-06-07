// ---------------------------------------------------------------------------
// normalize registry
// ---------------------------------------------------------------------------

use crate::algorithm::{
    is_supported, normalize, AesVariant, NormalizedAlgorithm, Operation, RawAlgorithm,
};
use crate::error::AlgorithmError;
use crate::hash::HashAlgorithm;

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
// KDF + AES/HMAC get-key-length registry rows (PR-3a)
// ---------------------------------------------------------------------------

#[test]
fn normalize_kdf_import_is_name_only() {
    assert_eq!(
        normalize(Operation::ImportKey, RawAlgorithm::from_name("hkdf")).unwrap(),
        NormalizedAlgorithm::Hkdf
    );
    assert_eq!(
        normalize(Operation::ImportKey, RawAlgorithm::from_name("PBKDF2")).unwrap(),
        NormalizedAlgorithm::Pbkdf2
    );
}

#[test]
fn normalize_kdf_get_key_length_is_name_only() {
    // §33.4.3 / §34.4.3 — get-key-length resolves to the same name-only form
    // (the op interprets it as the null length).
    assert_eq!(
        normalize(Operation::GetKeyLength, RawAlgorithm::from_name("HKDF")).unwrap(),
        NormalizedAlgorithm::Hkdf
    );
    assert_eq!(
        normalize(Operation::GetKeyLength, RawAlgorithm::from_name("pbkdf2")).unwrap(),
        NormalizedAlgorithm::Pbkdf2
    );
}

#[test]
fn normalize_hkdf_derive_bits_reads_hash_salt_info() {
    let raw = RawAlgorithm {
        name: "HKDF".into(),
        hash: Some(Box::new(RawAlgorithm::from_name("SHA-256"))),
        salt: Some(vec![1, 2, 3]),
        info: Some(vec![4, 5]),
        ..RawAlgorithm::default()
    };
    assert_eq!(
        normalize(Operation::DeriveBits, raw).unwrap(),
        NormalizedAlgorithm::HkdfParams {
            hash: HashAlgorithm::Sha256,
            salt: vec![1, 2, 3],
            info: vec![4, 5],
        }
    );
}

#[test]
fn normalize_pbkdf2_derive_bits_reads_salt_iterations_hash() {
    let raw = RawAlgorithm {
        name: "PBKDF2".into(),
        hash: Some(Box::new(RawAlgorithm::from_name("SHA-512"))),
        salt: Some(vec![9]),
        iterations: Some(1000),
        ..RawAlgorithm::default()
    };
    assert_eq!(
        normalize(Operation::DeriveBits, raw).unwrap(),
        NormalizedAlgorithm::Pbkdf2Params {
            salt: vec![9],
            iterations: 1000,
            hash: HashAlgorithm::Sha512,
        }
    );
}

#[test]
fn normalize_hkdf_derive_bits_missing_members_is_type_error() {
    // §33.3 `salt` / `info` are required.
    let raw = RawAlgorithm {
        name: "HKDF".into(),
        hash: Some(Box::new(RawAlgorithm::from_name("SHA-256"))),
        salt: Some(vec![1]),
        // info absent
        ..RawAlgorithm::default()
    };
    assert!(matches!(
        normalize(Operation::DeriveBits, raw),
        Err(AlgorithmError::Type(_))
    ));
}

#[test]
fn normalize_pbkdf2_missing_iterations_is_type_error() {
    let raw = RawAlgorithm {
        name: "PBKDF2".into(),
        hash: Some(Box::new(RawAlgorithm::from_name("SHA-256"))),
        salt: Some(vec![1]),
        // iterations absent
        ..RawAlgorithm::default()
    };
    assert!(matches!(
        normalize(Operation::DeriveBits, raw),
        Err(AlgorithmError::Type(_))
    ));
}

#[test]
fn normalize_aes_get_key_length_reads_length() {
    let raw = RawAlgorithm {
        name: "AES-GCM".into(),
        length: Some(256),
        ..RawAlgorithm::default()
    };
    assert_eq!(
        normalize(Operation::GetKeyLength, raw).unwrap(),
        NormalizedAlgorithm::AesKeyGen {
            variant: AesVariant::Gcm,
            length: 256,
        }
    );
}

#[test]
fn is_supported_kdf_and_get_key_length_registry() {
    // HKDF / PBKDF2 register import + deriveBits + get-key-length only.
    assert!(is_supported(Operation::ImportKey, "HKDF"));
    assert!(is_supported(Operation::DeriveBits, "HKDF"));
    assert!(is_supported(Operation::GetKeyLength, "HKDF"));
    assert!(is_supported(Operation::ImportKey, "PBKDF2"));
    assert!(is_supported(Operation::DeriveBits, "PBKDF2"));
    assert!(is_supported(Operation::GetKeyLength, "PBKDF2"));
    // AES + HMAC get-key-length are now registered (deriveKey derivedKeyType).
    assert!(is_supported(Operation::GetKeyLength, "AES-GCM"));
    assert!(is_supported(Operation::GetKeyLength, "AES-CBC"));
    assert!(is_supported(Operation::GetKeyLength, "AES-CTR"));
    assert!(is_supported(Operation::GetKeyLength, "HMAC"));
    // KDFs have NO generateKey / encrypt / decrypt / sign — the as_aes()-
    // guarded catch-alls correctly exclude them (NotSupported).
    assert!(!is_supported(Operation::GenerateKey, "HKDF"));
    assert!(!is_supported(Operation::GenerateKey, "PBKDF2"));
    assert!(!is_supported(Operation::Encrypt, "HKDF"));
    assert!(!is_supported(Operation::Decrypt, "PBKDF2"));
    assert!(!is_supported(Operation::Sign, "HKDF"));
    // AES has no deriveBits.
    assert!(!is_supported(Operation::DeriveBits, "AES-GCM"));
}
