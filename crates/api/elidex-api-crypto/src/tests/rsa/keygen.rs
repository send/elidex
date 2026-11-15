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

use crate::rsa::{
    validate_rsa_generate_capability as capability, validate_rsa_keygen_params as validate,
};

/// Run a `(row, ml, exponent, expected message substring)` table against `check`:
/// `None` expects Ok, `Some(m)` an OperationError whose message contains `m`.
fn run_table(
    rows: &[(&str, u32, &[u8], Option<&str>)],
    check: fn(u32, &[u8]) -> Result<(), AlgorithmError>,
) {
    for &(row, ml, e, msg) in rows {
        match (check(ml, e), msg) {
            (Ok(()), None) => {}
            (Err(AlgorithmError::Operation(m)), Some(want)) => {
                assert!(m.contains(want), "{row}: message {m:?} lacks {want:?}");
            }
            (got, _) => panic!("{row}: unexpected {got:?} (expected message {msg:?})"),
        }
    }
}

#[test]
fn keygen_params_boundary_table() {
    let rows: &[(&str, u32, &[u8], Option<&str>)] = &[
        ("B1 ml<4", 3, &[3], Some("at least 4")),
        ("B2 ml<4 pass", 4, &[3], None),
        ("B3 e<3", 2048, &[1], Some("at least 3")),
        ("B4 e<3 pass", 2048, &[3], None),
        ("B5 empty = 0", 2048, &[], Some("at least 3")),
        ("B6 all-zero = 0", 2048, &[0, 0], Some("at least 3")),
        ("B7 even", 2048, &[1, 0, 0], Some("odd")),
        ("B8 even pass", 2048, &[1, 0, 1], None),
        (
            "B9 e = 2^16-1",
            16,
            &[0xFF, 0xFF],
            Some("less than 2^modulusLength"),
        ),
        ("B10 bits == ml, not all ones", 16, &[0xFF, 0xFD], None),
        (
            "B11 bits > ml",
            16,
            &[1, 0, 1],
            Some("less than 2^modulusLength"),
        ),
        ("B12 bits < ml", 17, &[1, 0, 1], None),
        (
            "B13 leading zeros stripped",
            8,
            &[0, 0, 0xFF],
            Some("less than 2^modulusLength"),
        ),
        ("B14 leading zeros not bits", 17, &[0, 0, 1, 0, 1], None),
        ("B15 largest ml", u32::MAX, &[1, 0, 1], None),
    ];
    run_table(rows, validate);
}

/// 2^33 - 1 (the rsa crate's `MAX_PUB_EXPONENT`) and 2^33 + 1, big-endian.
const E_CAP: &[u8] = &[1, 0xFF, 0xFF, 0xFF, 0xFF];
const E_CAP_PLUS_2: &[u8] = &[2, 0, 0, 0, 1];

#[test]
fn keygen_capability_boundary_table() {
    let rows: &[(&str, u32, &[u8], Option<&str>)] = &[
        ("C1 floor 69", 69, &[1, 0, 1], Some("modulusLength")),
        ("C2 floor 70", 70, &[1, 0, 1], None),
        ("C3 cap 2^33-1", 70, E_CAP, None),
        ("C4 cap 2^33+1", 70, E_CAP_PLUS_2, Some("publicExponent")),
        (
            "C5 cap, leading zeros stripped",
            70,
            &[0, 0, 0, 0, 1, 0xFF, 0xFF, 0xFF, 0xFF],
            None,
        ),
        (
            "C6 cap, 9 bytes",
            70,
            &[1, 0, 0, 0, 0, 0, 0, 0, 1],
            Some("publicExponent"),
        ),
        ("C7 largest ml", u32::MAX, &[1, 0, 1], None),
    ];
    run_table(rows, capability);
    // The C3 / C4 / C2 inputs are all accepted by §20.5, so only the capability
    // bounds (not §20.5) decide them.
    for e in [E_CAP, E_CAP_PLUS_2, &[1, 0, 1][..]] {
        assert_eq!(validate(70, e), Ok(()), "{e:?}");
    }
}

#[derive(Clone, Copy)]
enum Src {
    Params,
    Capability,
}

fn sv() -> Vec<KeyUsage> {
    vec![KeyUsage::Sign, KeyUsage::Verify]
}

fn ed() -> Vec<KeyUsage> {
    vec![KeyUsage::Encrypt, KeyUsage::Decrypt]
}

/// `generate_key` over the panic seam: a check before the rsa crate must reject
/// (a regression panics instead of hanging).
fn keygen_rejected(
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
    .expect_err("rejected before key generation")
}

type GenRow = (
    &'static str,
    RsaVariant,
    u32,
    &'static [u8],
    Vec<KeyUsage>,
    Src,
);

#[allow(clippy::too_many_lines)] // a flat case table
fn rejection_rows() -> Vec<GenRow> {
    use RsaVariant::{RsaOaep, RsaPss, RsassaPkcs1V15 as Rsassa};
    vec![
        (
            "G1 e=2 at 1024 (measured hang)",
            Rsassa,
            1024,
            &[2],
            sv(),
            Src::Params,
        ),
        (
            "G2 even, RSASSA",
            Rsassa,
            2048,
            &[1, 0, 0],
            sv(),
            Src::Params,
        ),
        (
            "G3 even, RSA-PSS",
            RsaPss,
            2048,
            &[1, 0, 0],
            sv(),
            Src::Params,
        ),
        (
            "G4 even, RSA-OAEP",
            RsaOaep,
            2048,
            &[1, 0, 0],
            ed(),
            Src::Params,
        ),
        (
            "G6 step 2 before the OAEP range",
            RsaOaep,
            1024,
            &[1, 0, 0],
            ed(),
            Src::Params,
        ),
        (
            "G6b step 2 before the ceiling",
            Rsassa,
            16384,
            &[1, 0, 0],
            sv(),
            Src::Params,
        ),
        (
            "G8 step 2 before the floor",
            Rsassa,
            3,
            &[3],
            sv(),
            Src::Params,
        ),
        (
            "G9 ml 12 e=39 hang, RSASSA",
            Rsassa,
            12,
            &[39],
            sv(),
            Src::Capability,
        ),
        (
            "G10 ml 12 e=39 hang, RSA-PSS",
            RsaPss,
            12,
            &[39],
            sv(),
            Src::Capability,
        ),
        (
            "G11 floor 69",
            RsaPss,
            69,
            &[1, 0, 1],
            sv(),
            Src::Capability,
        ),
        (
            "G12 cap 2^33+1",
            Rsassa,
            1024,
            E_CAP_PLUS_2,
            sv(),
            Src::Capability,
        ),
        (
            "G13 cap before the ceiling",
            Rsassa,
            16384,
            E_CAP_PLUS_2,
            sv(),
            Src::Capability,
        ),
        (
            "G14 cap before the OAEP range",
            RsaOaep,
            1024,
            E_CAP_PLUS_2,
            ed(),
            Src::Capability,
        ),
    ]
}

#[test]
fn generate_rejects_before_the_rsa_crate() {
    for (row, variant, ml, e, usages, src) in rejection_rows() {
        let want = match src {
            Src::Params => validate(ml, e),
            Src::Capability => capability(ml, e),
        }
        .expect_err(row);
        assert_eq!(keygen_rejected(variant, ml, e, usages), want, "{row}");
    }
}

#[test]
fn generate_check_order_kinds() {
    // (row, variant, ml, exponent, usages, expect Syntax? else Operation)
    let rows = [
        // Step 1 (usages) before step 2 (§20.5).
        (
            "G5",
            RsaVariant::RsassaPkcs1V15,
            2048,
            &[1u8, 0, 0][..],
            vec![KeyUsage::Encrypt],
            true,
        ),
        // §14.3.6 empty-private-usages SyntaxError comes after the operation.
        (
            "G6c",
            RsaVariant::RsaPss,
            2048,
            &[1, 0, 0],
            vec![KeyUsage::Verify],
            false,
        ),
        (
            "G6d floor",
            RsaVariant::RsaPss,
            12,
            &[39],
            vec![KeyUsage::Verify],
            false,
        ),
    ];
    for (row, variant, ml, e, usages, syntax) in rows {
        let err = keygen_rejected(variant, ml, e, usages);
        let ok = if syntax {
            matches!(err, AlgorithmError::Syntax(_))
        } else {
            matches!(err, AlgorithmError::Operation(_))
        };
        assert!(ok, "{row}: {err:?}");
    }
}
