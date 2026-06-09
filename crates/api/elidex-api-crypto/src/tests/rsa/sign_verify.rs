//! RSASSA-PKCS1-v1_5 / RSA-PSS sign / verify: round-trips, the PSS blinding
//! regression boundary, salt-length and cross-hash verification edges, and the
//! entropy-seam / oversized-salt robustness guards.

use super::*;
use crate::ops::{export_key, sign, verify};

#[test]
fn rsapss_signature_verifies_under_standard_unblinded_pss() {
    // `pss_scheme` signs with `Pss::new_blinded_with_salt` (R8 side-channel
    // blinding).  The rsa crate's doc comment frames `new_blinded` as
    // "RSA-BSSA", but the EMSA-PSS *encoding* is identical regardless of the
    // `blinded` flag — it only gates the modexp RNG (source-verified) — so the
    // output is a STANDARD RSASSA-PSS signature.  Prove interoperability: a
    // blinded-scheme signature verifies under the rsa crate's *unblinded*
    // `Pss::new_with_salt` (Codex R15: the BSSA concern is a docs-misread).
    use rsa::pkcs8::spki::DecodePublicKey;
    let (public, private) = generate_pair(
        RsaVariant::RsaPss,
        HashAlgorithm::Sha256,
        vec![KeyUsage::Sign, KeyUsage::Verify],
    );
    let salt_length = 32u32;
    let msg = b"standard-PSS interop";
    let sig = sign(
        sign_alg(RsaVariant::RsaPss, Some(salt_length)),
        &private,
        msg,
        seeded_fill(7),
    )
    .expect("RSA-PSS sign (blinded scheme)");
    let spki_der = expect_raw(export_key(KeyFormat::Spki, &public).expect("SPKI export"));
    let pubkey = rsa::RsaPublicKey::from_public_key_der(&spki_der).expect("decode SPKI");
    let digest = HashAlgorithm::Sha256.digest(msg);
    pubkey
        .verify(
            // `sha2_oid::Sha256` is the rsa-crate-compatible digest used by the
            // signing path; `new_with_salt` is the *standard* (unblinded) PSS.
            rsa::Pss::new_with_salt::<sha2_oid::Sha256>(salt_length as usize),
            &digest,
            &sig,
        )
        .expect("a blinded-scheme PSS signature MUST verify as standard (unblinded) RSASSA-PSS");
}

#[test]
fn rsassa_sign_verify_round_trip_all_hashes() {
    for hash in [
        HashAlgorithm::Sha256,
        HashAlgorithm::Sha384,
        HashAlgorithm::Sha512,
    ] {
        let (public, private) = generate_pair(
            RsaVariant::RsassaPkcs1V15,
            hash,
            vec![KeyUsage::Sign, KeyUsage::Verify],
        );
        let msg = b"RSASSA-PKCS1-v1_5 message";
        let sig = sign(
            sign_alg(RsaVariant::RsassaPkcs1V15, None),
            &private,
            msg,
            seeded_fill(1),
        )
        .expect("RSASSA sign");
        let ok = verify(
            verify_alg(RsaVariant::RsassaPkcs1V15, None),
            &public,
            &sig,
            msg,
        )
        .expect("RSASSA verify");
        assert!(
            ok,
            "RSASSA-PKCS1-v1_5 round-trip should verify for {hash:?}"
        );
    }
}

#[test]
fn rsapss_sign_verify_round_trip_salt_variants() {
    // RSA-PSS over SHA-256 (hLen = 32): saltLength 0 (deterministic) and 32.
    for salt_length in [0u32, 32] {
        let (public, private) = generate_pair(
            RsaVariant::RsaPss,
            HashAlgorithm::Sha256,
            vec![KeyUsage::Sign, KeyUsage::Verify],
        );
        let msg = b"RSA-PSS message";
        let sig = sign(
            sign_alg(RsaVariant::RsaPss, Some(salt_length)),
            &private,
            msg,
            seeded_fill(2),
        )
        .expect("RSA-PSS sign");
        let ok = verify(
            verify_alg(RsaVariant::RsaPss, Some(salt_length)),
            &public,
            &sig,
            msg,
        )
        .expect("RSA-PSS verify");
        assert!(
            ok,
            "RSA-PSS round-trip should verify (saltLength={salt_length})"
        );
    }
}

#[test]
fn rsapss_sign_blinds_the_private_key_exponentiation() {
    // RSA-PSS signing MUST blind the private-key exponentiation — the
    // deny.toml RUSTSEC-2023-0071 (Marvin) rationale rests on signing being
    // blinded.  In rsa 0.9, `Pss` blinds the exponentiation only when its
    // `blinded` flag is set, so an *unblinded* `Pss::new_with_salt` would draw
    // exactly `saltLength` bytes from the seam (just the PSS salt), whereas the
    // blinded `new_blinded_with_salt` additionally draws the modulus-sized
    // blinding factor.  Asserting the seam is consumed *beyond* the salt (plus
    // the fixed pre-op entropy probe, R10) is a precise regression boundary for
    // "blinding is active" (Codex R8).
    let (_public, private) = generate_pair(
        RsaVariant::RsaPss,
        HashAlgorithm::Sha256,
        vec![KeyUsage::Sign],
    );
    let salt_length = 32u32;
    let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(0x99);
    let drawn = std::cell::Cell::new(0usize);
    let counting = |buf: &mut [u8]| {
        drawn.set(drawn.get() + buf.len());
        rng.fill_bytes(buf);
        Ok(())
    };
    sign(
        sign_alg(RsaVariant::RsaPss, Some(salt_length)),
        &private,
        b"RSA-PSS message",
        counting,
    )
    .expect("RSA-PSS sign");
    // The seam supplies the pre-op probe + the PSS salt + (iff blinded) the
    // modulus-sized blinding factor.  An unblinded scheme would draw only
    // probe + salt, so requiring strictly more proves blinding ran.
    let floor = salt_length as usize + crate::rsa::ENTROPY_PROBE_LEN;
    assert!(
        drawn.get() > floor,
        "RSA-PSS sign drew {} bytes (<= probe+saltLength {floor}): the private-key \
         exponentiation was NOT blinded",
        drawn.get(),
    );
}

#[test]
fn rsapss_sign_surfaces_entropy_failure() {
    // RSA-PSS sign must fail-fast on a down entropy seam — the pre-op probe
    // rejects before the private-key exponentiation, so the op never runs on
    // the deterministic `ClosureRng` fallback with predictable blinding (R10).
    let (_public, private) = generate_pair(
        RsaVariant::RsaPss,
        HashAlgorithm::Sha256,
        vec![KeyUsage::Sign],
    );
    let always_fail = |_: &mut [u8]| Err(AlgorithmError::Operation("entropy unavailable".into()));
    let err = sign(
        sign_alg(RsaVariant::RsaPss, Some(32)),
        &private,
        b"m",
        always_fail,
    )
    .expect_err("an entropy failure must surface, not sign on the deterministic fallback");
    assert!(matches!(err, AlgorithmError::Operation(_)), "got {err:?}");
}

#[test]
fn sign_with_public_key_is_invalid_access() {
    let (public, _private) = generate_pair(
        RsaVariant::RsassaPkcs1V15,
        HashAlgorithm::Sha256,
        vec![KeyUsage::Sign, KeyUsage::Verify],
    );
    // A public key has no private DER → InvalidAccessError (the crate gate; the
    // usage gate is the VM `ops::sign` layer).
    let err = sign(
        sign_alg(RsaVariant::RsassaPkcs1V15, None),
        &public,
        b"m",
        seeded_fill(3),
    )
    .expect_err("signing with a public key is rejected");
    assert!(
        matches!(err, AlgorithmError::InvalidAccess(_)),
        "got {err:?}"
    );
}

#[test]
fn verify_rejects_tampered_signature_and_message() {
    let (public, private) = generate_pair(
        RsaVariant::RsassaPkcs1V15,
        HashAlgorithm::Sha256,
        vec![KeyUsage::Sign, KeyUsage::Verify],
    );
    let msg = b"authentic";
    let mut sig = sign(
        sign_alg(RsaVariant::RsassaPkcs1V15, None),
        &private,
        msg,
        seeded_fill(4),
    )
    .expect("sign");
    // A flipped signature byte → false (no throw).
    sig[0] ^= 0xFF;
    assert!(!verify(
        verify_alg(RsaVariant::RsassaPkcs1V15, None),
        &public,
        &sig,
        msg
    )
    .unwrap());
    // A different message → false.
    sig[0] ^= 0xFF; // restore
    assert!(!verify(
        verify_alg(RsaVariant::RsassaPkcs1V15, None),
        &public,
        &sig,
        b"forged"
    )
    .unwrap());
}

#[test]
fn rsapss_verify_wrong_salt_length_is_false() {
    let (public, private) = generate_pair(
        RsaVariant::RsaPss,
        HashAlgorithm::Sha256,
        vec![KeyUsage::Sign, KeyUsage::Verify],
    );
    let msg = b"salt-bound";
    // Sign with saltLength = 32, verify with saltLength = 0 → invalid → false.
    let sig = sign(
        sign_alg(RsaVariant::RsaPss, Some(32)),
        &private,
        msg,
        seeded_fill(5),
    )
    .expect("PSS sign");
    let ok = verify(verify_alg(RsaVariant::RsaPss, Some(0)), &public, &sig, msg).unwrap();
    assert!(!ok, "a saltLength mismatch must fail verification");
}

#[test]
fn cross_hash_verify_is_false() {
    // A signature made with a SHA-256 key does not verify under a public key of
    // the same material re-imported with hash = SHA-384 (the DigestInfo prefix
    // + digest length differ).
    let (public, private) = generate_pair(
        RsaVariant::RsassaPkcs1V15,
        HashAlgorithm::Sha256,
        vec![KeyUsage::Sign, KeyUsage::Verify],
    );
    let msg = b"hash-bound";
    let sig = sign(
        sign_alg(RsaVariant::RsassaPkcs1V15, None),
        &private,
        msg,
        seeded_fill(6),
    )
    .expect("sign");
    let spki = expect_raw(export_key(KeyFormat::Spki, &public).expect("spki"));
    let public384 = import_key(
        KeyFormat::Spki,
        import_alg(RsaVariant::RsassaPkcs1V15, HashAlgorithm::Sha384),
        true,
        vec![KeyUsage::Verify],
        KeyData::Raw(spki),
    )
    .expect("import public under SHA-384");
    let ok = verify(
        verify_alg(RsaVariant::RsassaPkcs1V15, None),
        &public384,
        &sig,
        msg,
    )
    .unwrap();
    assert!(!ok, "a SHA-384 verify of a SHA-256 signature must fail");
}

#[test]
fn rsassa_sign_surfaces_entropy_failure() {
    // RSASSA-PKCS1-v1_5 sign blinds the private-key op via the entropy seam
    // (`sign_with_rng`), so a `fill_random` failure surfaces as an error rather
    // than running the exponentiation unblinded.
    let (_public, private) = generate_pair(
        RsaVariant::RsassaPkcs1V15,
        HashAlgorithm::Sha256,
        vec![KeyUsage::Sign],
    );
    let always_fail = |_: &mut [u8]| Err(AlgorithmError::Operation("entropy unavailable".into()));
    let err = sign(
        sign_alg(RsaVariant::RsassaPkcs1V15, None),
        &private,
        b"m",
        always_fail,
    )
    .expect_err("an entropy failure must surface, not sign unblinded");
    assert!(matches!(err, AlgorithmError::Operation(_)), "got {err:?}");
}

#[test]
fn rsapss_sign_oversized_salt_length_is_rejected_without_oom() {
    // A saltLength larger than the modulus is always an invalid PSS signature
    // (RFC 3447 §9.1.1) and must be rejected BEFORE the rsa crate allocates a
    // salt-sized buffer — an attacker-supplied saltLength = u32::MAX would
    // otherwise allocate / fill ~4 GiB.
    let (_public, private) = generate_pair(
        RsaVariant::RsaPss,
        HashAlgorithm::Sha256,
        vec![KeyUsage::Sign],
    );
    let err = sign(
        sign_alg(RsaVariant::RsaPss, Some(u32::MAX)),
        &private,
        b"m",
        seeded_fill(9),
    )
    .expect_err("an oversized saltLength must be rejected up front");
    assert!(matches!(err, AlgorithmError::Operation(_)), "got {err:?}");
}
