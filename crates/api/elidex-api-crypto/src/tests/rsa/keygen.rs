//! RSA generateKey: key shape + usage-split, WebIDL member-order, the modulus
//! ceiling, and entropy / DoS bounds.  (The Marvin (RUSTSEC-2023-0071)
//! constant-time-decryption tripwire lives in `oaep`, next to the aws-lc-rs
//! OAEP backend it guards.)

use super::*;
use crate::key::{KeyAlgorithm, KeyType};

#[test]
fn generate_key_shape_and_usage_split() {
    let (public, private) = generate_pair(
        RsaVariant::RsassaPkcs1V15,
        HashAlgorithm::Sha256,
        vec![KeyUsage::Sign, KeyUsage::Verify],
    );
    // The public key takes `verify`, is always extractable; the private key
    // takes `sign`.
    assert_eq!(public.key_type, KeyType::Public);
    assert!(public.extractable);
    assert_eq!(public.usages, vec![KeyUsage::Verify]);
    assert_eq!(private.key_type, KeyType::Private);
    assert_eq!(private.usages, vec![KeyUsage::Sign]);

    // The key's `[[algorithm]]` carries the variant + modulus length + public
    // exponent + hash (RsaHashedKeyAlgorithm §20.7).
    let KeyAlgorithm::Rsa {
        variant,
        modulus_length,
        public_exponent,
        hash,
    } = &private.algorithm
    else {
        panic!("RSA key has an Rsa algorithm");
    };
    assert_eq!(*variant, RsaVariant::RsassaPkcs1V15);
    assert_eq!(*modulus_length, 2048);
    assert_eq!(public_exponent, &vec![0x01, 0x00, 0x01]);
    assert_eq!(*hash, HashAlgorithm::Sha256);
    // The public key shares the same `[[algorithm]]`.
    assert_eq!(public.algorithm, private.algorithm);
}

#[test]
fn generate_empty_private_usages_is_syntax_error() {
    // `verify` only → the private half has empty usages → SyntaxError.
    let err = generate_key(
        keygen_alg(RsaVariant::RsaPss, 2048, HashAlgorithm::Sha256),
        true,
        vec![KeyUsage::Verify],
        seeded_fill(0x5A),
    )
    .expect_err("empty private usages is a SyntaxError");
    assert!(matches!(err, AlgorithmError::Syntax(_)), "got {err:?}");
}

#[test]
fn generate_invalid_usage_is_syntax_error() {
    let err = generate_key(
        keygen_alg(RsaVariant::RsassaPkcs1V15, 2048, HashAlgorithm::Sha256),
        true,
        vec![KeyUsage::Encrypt],
        seeded_fill(0x5A),
    )
    .expect_err("a non-sign/verify usage is a SyntaxError");
    assert!(matches!(err, AlgorithmError::Syntax(_)), "got {err:?}");
}

#[test]
fn normalize_rsa_keygen_reports_members_in_webidl_order() {
    // RsaHashedKeyGenParams : RsaKeyGenParams : Algorithm — Web IDL validates
    // the inherited `modulusLength` / `publicExponent` before the derived
    // `hash`.  A malformed raw must report the inherited member first, matching
    // the spec + the VM marshaller (which fires getters in that order), not the
    // borrow-driven hash-first order (Codex R12).
    // modulusLength + hash absent (publicExponent present) → modulusLength first.
    let mut raw = RawAlgorithm::from_name(RsaVariant::RsassaPkcs1V15.canonical_name());
    raw.public_exponent = Some(vec![0x01, 0x00, 0x01]);
    let err = normalize(Operation::GenerateKey, raw).expect_err("missing modulusLength");
    assert!(
        matches!(&err, AlgorithmError::Type(m) if m.contains("modulusLength")),
        "expected the modulusLength TypeError first, got {err:?}"
    );
    // publicExponent + hash absent (modulusLength present) → publicExponent
    // (inherited) before hash (derived).
    let mut raw = RawAlgorithm::from_name(RsaVariant::RsaPss.canonical_name());
    raw.modulus_length = Some(2048);
    let err = normalize(Operation::GenerateKey, raw).expect_err("missing publicExponent");
    assert!(
        matches!(&err, AlgorithmError::Type(m) if m.contains("publicExponent")),
        "expected the publicExponent TypeError before hash, got {err:?}"
    );
}

#[test]
fn generate_modulus_above_8192_is_operation_error() {
    // The generate ceiling (`MAX_RSA_GENERATE_MODULUS_BITS` = 8192, Chrome-faithful)
    // is tighter than the 16384 import ceiling — keygen's prime search is the
    // expensive part.  A 16384-bit `modulusLength` is rejected BEFORE keygen
    // (fast), so an abusive request cannot hang the VM thread (§20.9.3 step-4
    // OperationError surface).
    let err = generate_key(
        keygen_alg(RsaVariant::RsassaPkcs1V15, 16384, HashAlgorithm::Sha256),
        true,
        vec![KeyUsage::Sign, KeyUsage::Verify],
        seeded_fill(7),
    )
    .expect_err("a modulusLength over the generate cap is an OperationError");
    assert!(matches!(err, AlgorithmError::Operation(_)), "got {err:?}");
}

#[test]
fn generate_surfaces_entropy_failure_without_hanging() {
    // When the VM `fill_random` seam errors, generateKey must TERMINATE with
    // the OperationError — not spin RSA prime-search forever (the ClosureRng
    // fallback must vary so `new_with_exp` converges, then `into_result` errors
    // and the key is discarded).  A 512-bit modulus keeps the discarded keygen
    // fast.  (Pre-fix: a constant fallback fill hangs this test.)
    let always_fail = |_: &mut [u8]| Err(AlgorithmError::Operation("entropy unavailable".into()));
    let err = generate_key(
        keygen_alg(RsaVariant::RsassaPkcs1V15, 512, HashAlgorithm::Sha256),
        true,
        vec![KeyUsage::Sign, KeyUsage::Verify],
        always_fail,
    )
    .expect_err("an entropy-seam failure must surface as an error");
    assert!(matches!(err, AlgorithmError::Operation(_)), "got {err:?}");
}

#[test]
fn generate_preserves_non_minimal_public_exponent() {
    // §20.9.3 step 8: the key's publicExponent reflects the caller's input
    // bytes (normalizedAlgorithm.publicExponent) — a non-minimal `[0,1,0,1]`
    // is NOT canonicalized to `[1,0,1]`.
    let mut raw = RawAlgorithm::from_name(RsaVariant::RsassaPkcs1V15.canonical_name());
    raw.modulus_length = Some(2048);
    raw.public_exponent = Some(vec![0x00, 0x01, 0x00, 0x01]);
    raw.hash = Some(Box::new(RawAlgorithm::from_name(
        HashAlgorithm::Sha256.canonical_name(),
    )));
    let alg = normalize(Operation::GenerateKey, raw).expect("normalizes");
    let public = match generate_key(
        alg,
        true,
        vec![KeyUsage::Sign, KeyUsage::Verify],
        seeded_fill(7),
    )
    .unwrap()
    {
        GeneratedKey::Pair { public, .. } => public,
        GeneratedKey::Single(_) => panic!("RSA keygen yields a pair"),
    };
    let KeyAlgorithm::Rsa {
        public_exponent, ..
    } = &public.algorithm
    else {
        panic!("RSA key");
    };
    assert_eq!(public_exponent, &vec![0x00, 0x01, 0x00, 0x01]);
}

#[test]
fn generate_oversized_modulus_length_is_rejected() {
    // An untrusted `generateKey({modulusLength: 2^32-1})` is rejected BEFORE the
    // rsa crate prime-searches at that size (engine DoS guard).
    let mut raw = RawAlgorithm::from_name(RsaVariant::RsassaPkcs1V15.canonical_name());
    raw.modulus_length = Some(u32::MAX);
    raw.public_exponent = Some(vec![0x01, 0x00, 0x01]);
    raw.hash = Some(Box::new(RawAlgorithm::from_name(
        HashAlgorithm::Sha256.canonical_name(),
    )));
    let alg = normalize(Operation::GenerateKey, raw).expect("normalizes");
    let err = generate_key(
        alg,
        true,
        vec![KeyUsage::Sign, KeyUsage::Verify],
        seeded_fill(8),
    )
    .expect_err("an oversized modulusLength must be rejected");
    assert!(matches!(err, AlgorithmError::Operation(_)), "got {err:?}");
}

#[test]
fn public_exponent_round_trips_byte_identical() {
    // The stored publicExponent is the canonical big-endian 65537 = [1, 0, 1].
    let (public, _private) = generate_pair(
        RsaVariant::RsassaPkcs1V15,
        HashAlgorithm::Sha256,
        vec![KeyUsage::Sign, KeyUsage::Verify],
    );
    let KeyAlgorithm::Rsa {
        public_exponent, ..
    } = &public.algorithm
    else {
        panic!("RSA key");
    };
    assert_eq!(public_exponent, &vec![0x01, 0x00, 0x01]);
}

// ---------------------------------------------------------------------------
// WebCrypto §20.5 RSA key generation parameter validation
// ---------------------------------------------------------------------------

use crate::rsa::validate_rsa_keygen_params as validate;

#[test]
fn keygen_params_boundary_table() {
    // (row, modulusLength, publicExponent bytes, expect Ok)
    let rows: &[(&str, u32, &[u8], bool)] = &[
        ("B1 ml<4", 3, &[3], false),
        ("B2 ml<4 pass", 4, &[3], true),
        ("B3 e<3", 2048, &[1], false),
        ("B4 e<3 pass", 2048, &[3], true),
        ("B5 empty = 0", 2048, &[], false),
        ("B6 all-zero = 0", 2048, &[0, 0], false),
        ("B7 even", 2048, &[1, 0, 0], false),
        ("B8 even pass", 2048, &[1, 0, 1], true),
        ("B9 e = 2^16-1", 16, &[0xFF, 0xFF], false),
        ("B10 bits == ml, not all ones", 16, &[0xFF, 0xFD], true),
        ("B11 bits > ml", 16, &[1, 0, 1], false),
        ("B12 bits < ml", 17, &[1, 0, 1], true),
        ("B13 leading zeros stripped", 8, &[0, 0, 0xFF], false),
        ("B14 leading zeros not bits", 17, &[0, 0, 1, 0, 1], true),
        ("B15 largest ml", u32::MAX, &[1, 0, 1], true),
    ];
    for &(row, ml, e, ok) in rows {
        let got = validate(ml, e);
        if ok {
            assert_eq!(got, Ok(()), "{row}");
        } else {
            assert!(
                matches!(got, Err(AlgorithmError::Operation(_))),
                "{row}: {got:?}"
            );
        }
    }
}

#[test]
fn keygen_params_messages_name_the_condition() {
    let ml = validate(3, &[3]).unwrap_err();
    assert!(
        matches!(&ml, AlgorithmError::Operation(m) if m.contains("modulusLength")),
        "{ml:?}"
    );
    for e in [&[1u8][..], &[1, 0, 0], &[0xFF, 0xFF]] {
        let err = validate(16, e).unwrap_err();
        assert!(
            matches!(&err, AlgorithmError::Operation(m) if m.contains("publicExponent")),
            "{err:?}"
        );
    }
}

/// `generate_key` over the panic seam: §20.5 must reject before the rsa crate.
fn keygen_rejected_by_params(
    variant: RsaVariant,
    ml: u32,
    exp: &[u8],
    usages: Vec<KeyUsage>,
) -> AlgorithmError {
    generate_key(
        keygen_alg_exp(variant, ml, exp, HashAlgorithm::Sha256),
        true,
        usages,
        panicking_fill,
    )
    .expect_err("§20.5 rejects these parameters")
}

#[test]
fn generate_even_exponent_rejected_at_once_for_all_variants() {
    let sv = || vec![KeyUsage::Sign, KeyUsage::Verify];
    let ed = || vec![KeyUsage::Encrypt, KeyUsage::Decrypt];
    // G1: the measured hang input (modulusLength 1024, e = 2).
    assert_eq!(
        keygen_rejected_by_params(RsaVariant::RsassaPkcs1V15, 1024, &[2], sv()),
        validate(1024, &[2]).unwrap_err(),
        "G1"
    );
    // G2-G4: the three variants reach the same validation.
    for (row, variant, usages) in [
        ("G2", RsaVariant::RsassaPkcs1V15, sv()),
        ("G3", RsaVariant::RsaPss, sv()),
        ("G4", RsaVariant::RsaOaep, ed()),
    ] {
        assert_eq!(
            keygen_rejected_by_params(variant, 2048, &[1, 0, 0], usages),
            validate(2048, &[1, 0, 0]).unwrap_err(),
            "{row}"
        );
    }
}

#[test]
fn generate_usages_syntax_error_precedes_params() {
    // G5: Generate Key step 1 (usages) before step 2 (§20.5).
    let err = keygen_rejected_by_params(
        RsaVariant::RsassaPkcs1V15,
        2048,
        &[1, 0, 0],
        vec![KeyUsage::Encrypt],
    );
    assert!(matches!(err, AlgorithmError::Syntax(_)), "got {err:?}");
}

#[test]
fn generate_params_error_precedes_oaep_range_and_ceiling() {
    // G6: step 2 before the RSA-OAEP range.
    let err = keygen_rejected_by_params(
        RsaVariant::RsaOaep,
        1024,
        &[1, 0, 0],
        vec![KeyUsage::Encrypt, KeyUsage::Decrypt],
    );
    assert_eq!(err, validate(1024, &[1, 0, 0]).unwrap_err(), "G6");
    // G6b: step 2 before the generate ceiling.
    let err = keygen_rejected_by_params(
        RsaVariant::RsassaPkcs1V15,
        16384,
        &[1, 0, 0],
        vec![KeyUsage::Sign, KeyUsage::Verify],
    );
    assert_eq!(err, validate(16384, &[1, 0, 0]).unwrap_err(), "G6b");
}

#[test]
fn generate_params_error_precedes_empty_private_usages_error() {
    // G6c: the §14.3.6 empty-private-usages SyntaxError comes after the
    // operation, so an even exponent with `verify` only is an OperationError.
    let err =
        keygen_rejected_by_params(RsaVariant::RsaPss, 2048, &[1, 0, 0], vec![KeyUsage::Verify]);
    assert!(matches!(err, AlgorithmError::Operation(_)), "got {err:?}");
}
