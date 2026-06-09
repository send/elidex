//! RSA generateKey: key shape + usage-split, WebIDL member-order, the modulus
//! ceiling, entropy / DoS bounds, and the Marvin (RUSTSEC-2023-0071) tripwire.

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
    // exponent + hash (RsaHashedKeyAlgorithm §20.6).
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
fn modulus_ceiling_matches_rsa_crate_max_size() {
    // `MAX_RSA_MODULUS_BITS` must not exceed `rsa::RsaPublicKey::MAX_SIZE`:
    // public keys are reconstructed via `RsaPublicKey::new` /
    // `from_public_key_der`, which reject a modulus above MAX_SIZE.  A higher
    // ceiling would let `generateKey` mint a key whose public half can't be
    // reconstructed, and reject large imported keys despite the stated policy
    // (Codex R14).
    assert!(
        crate::rsa::MAX_RSA_MODULUS_BITS as usize <= rsa::RsaPublicKey::MAX_SIZE,
        "MAX_RSA_MODULUS_BITS ({}) exceeds rsa::RsaPublicKey::MAX_SIZE ({})",
        crate::rsa::MAX_RSA_MODULUS_BITS,
        rsa::RsaPublicKey::MAX_SIZE,
    );
}

#[test]
fn rsa_backend_has_no_decryption_while_marvin_advisory_is_ignored() {
    // Enforceable tripwire for the deny.toml RUSTSEC-2023-0071 (Marvin) ignore.
    // PR-5a is signing-only; RSA *decryption* (RSA-OAEP, the follow-on) is the
    // path the Marvin timing attack actually targets, and the deny.toml ignore
    // is workspace-wide so cargo-deny cannot fail when decryption lands — this
    // test does.  It scans the WHOLE crate source (not just rsa.rs) so a
    // decryption module added in any file still trips it (Codex R15).  If RSA
    // decryption appears, this fails: you MUST first address RUSTSEC-2023-0071
    // (drop the ignore for a fixed `rsa` release, or security-review the
    // decryption timing risk under the WebCrypto threat model — the deny.toml
    // gate).  Markers are code-syntactic (a method call / a type path) so they
    // never match prose like "RSA-OAEP"; `tests/` is skipped because this
    // test's own marker strings live there.
    fn scan(dir: &std::path::Path, markers: &[&str]) {
        for entry in std::fs::read_dir(dir).expect("read crate src dir") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                if path.file_name() == Some(std::ffi::OsStr::new("tests")) {
                    continue;
                }
                scan(&path, markers);
            } else if path.extension() == Some(std::ffi::OsStr::new("rs")) {
                let src = std::fs::read_to_string(&path).expect("read .rs source");
                for marker in markers {
                    assert!(
                        !src.contains(marker),
                        "RSA decryption marker `{marker}` found in {} — address \
                         RUSTSEC-2023-0071 (deny.toml Marvin gate) before adding RSA decryption",
                        path.display(),
                    );
                }
            }
        }
    }
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    scan(&src, &[".decrypt(", "Oaep::"]);
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
    // §20.8.3 step 7: the key's publicExponent reflects the caller's input
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
