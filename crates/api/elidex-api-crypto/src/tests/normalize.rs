// ---------------------------------------------------------------------------
// normalize registry
// ---------------------------------------------------------------------------

use crate::algorithm::{is_supported, normalize, NormalizedAlgorithm, Operation, RawAlgorithm};
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
