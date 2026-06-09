//! RSA-OAEP (WebCrypto §22) front-end plumbing: the generateKey usage-split +
//! key shape, the import / export round-trips (spki / pkcs8 / jwk with the
//! `RSA-OAEP-*` `alg`), and the `RsaOaepParams.label` normalization.  The
//! `encrypt` / `decrypt` / `wrapKey` / `unwrapKey` op-set (the dual-backend
//! seam) lands with the backend; this commit only wires the registry + keys.

use super::*;
use crate::key::{KeyAlgorithm, KeyType};
use crate::ops::{decrypt, encrypt, export_key, unwrap_key, wrap_key};

/// The normalized RSA-OAEP encrypt / decrypt / wrapKey / unwrapKey algorithm
/// (§22.3 `RsaOaepParams`) with the optional `label`.
fn oaep_alg(op: Operation, label: Option<&[u8]>) -> NormalizedAlgorithm {
    let mut raw = RawAlgorithm::from_name("RSA-OAEP");
    raw.label = label.map(<[u8]>::to_vec);
    normalize(op, raw).expect("RSA-OAEP normalizes")
}

/// An extractable AES-GCM key (the wrap / unwrap payload).
fn aes_gcm_key(raw: &[u8]) -> CryptoKeyData {
    import_key(
        KeyFormat::Raw,
        normalize(Operation::ImportKey, RawAlgorithm::from_name("AES-GCM"))
            .expect("AES-GCM import normalizes"),
        true,
        vec![KeyUsage::Encrypt, KeyUsage::Decrypt],
        KeyData::Raw(raw.to_vec()),
    )
    .expect("AES-GCM raw import")
}

#[test]
fn generate_oaep_key_shape_and_usage_split() {
    let (public, private) = generate_pair(
        RsaVariant::RsaOaep,
        HashAlgorithm::Sha256,
        vec![
            KeyUsage::Encrypt,
            KeyUsage::Decrypt,
            KeyUsage::WrapKey,
            KeyUsage::UnwrapKey,
        ],
    );
    // §22.4.3 usage split (distinct from the RSASSA / RSA-PSS sign families):
    // the public key takes {encrypt, wrapKey}, the private key {decrypt,
    // unwrapKey}; the public half is always extractable.
    assert_eq!(public.key_type, KeyType::Public);
    assert!(public.extractable);
    assert_eq!(public.usages, vec![KeyUsage::Encrypt, KeyUsage::WrapKey]);
    assert_eq!(private.key_type, KeyType::Private);
    assert_eq!(private.usages, vec![KeyUsage::Decrypt, KeyUsage::UnwrapKey]);

    // The key's `[[algorithm]]` is the shared `RsaHashedKeyAlgorithm` (§20.6,
    // reused by §22)
    // with the RsaOaep variant + the modulus / exponent / hash.
    let KeyAlgorithm::Rsa {
        variant,
        modulus_length,
        public_exponent,
        hash,
    } = &private.algorithm
    else {
        panic!("RSA-OAEP key has an Rsa algorithm");
    };
    assert_eq!(*variant, RsaVariant::RsaOaep);
    assert_eq!(*modulus_length, 2048);
    assert_eq!(public_exponent, &vec![0x01, 0x00, 0x01]);
    assert_eq!(*hash, HashAlgorithm::Sha256);
    assert_eq!(public.algorithm, private.algorithm);
}

#[test]
fn generate_oaep_with_sign_usage_is_syntax_error() {
    // §22.4.3 step 1: a usage outside {encrypt, decrypt, wrapKey, unwrapKey}
    // (e.g. the sign-family `sign`) is a SyntaxError.
    let err = generate_key(
        keygen_alg(RsaVariant::RsaOaep, 2048, HashAlgorithm::Sha256),
        true,
        vec![KeyUsage::Sign],
        seeded_fill(0x5A),
    )
    .expect_err("a sign usage is invalid for RSA-OAEP");
    assert!(matches!(err, AlgorithmError::Syntax(_)), "got {err:?}");
}

#[test]
fn oaep_private_pkcs8_round_trip() {
    let (_public, private) = generate_pair(
        RsaVariant::RsaOaep,
        HashAlgorithm::Sha256,
        vec![KeyUsage::Decrypt, KeyUsage::UnwrapKey],
    );
    let der = expect_raw(export_key(KeyFormat::Pkcs8, &private).expect("PKCS#8 export"));
    let reimported = import_key(
        KeyFormat::Pkcs8,
        import_alg(RsaVariant::RsaOaep, HashAlgorithm::Sha256),
        true,
        vec![KeyUsage::Decrypt],
        KeyData::Raw(der),
    )
    .expect("PKCS#8 re-import");
    assert_eq!(reimported.key_type, KeyType::Private);
    assert_eq!(reimported.material, private.material);
    assert_eq!(reimported.algorithm, private.algorithm);
}

#[test]
fn oaep_public_spki_round_trip() {
    let (public, _private) = generate_pair(
        RsaVariant::RsaOaep,
        HashAlgorithm::Sha384,
        vec![KeyUsage::Encrypt, KeyUsage::Decrypt],
    );
    let der = expect_raw(export_key(KeyFormat::Spki, &public).expect("SPKI export"));
    let reimported = import_key(
        KeyFormat::Spki,
        import_alg(RsaVariant::RsaOaep, HashAlgorithm::Sha384),
        true,
        vec![KeyUsage::Encrypt],
        KeyData::Raw(der),
    )
    .expect("SPKI re-import");
    assert_eq!(reimported.key_type, KeyType::Public);
    assert_eq!(reimported.material, public.material);
}

#[test]
fn oaep_jwk_private_round_trip_uses_rsa_oaep_alg() {
    let (_public, private) = generate_pair(
        RsaVariant::RsaOaep,
        HashAlgorithm::Sha256,
        vec![KeyUsage::Decrypt, KeyUsage::UnwrapKey],
    );
    let jwk = expect_jwk(export_key(KeyFormat::Jwk, &private).expect("JWK export"));
    assert_eq!(jwk.kty.as_deref(), Some("RSA"));
    // §22.4.5 jwk export: SHA-256 → "RSA-OAEP-256" (the SHA-1 form is the bare
    // "RSA-OAEP").
    assert_eq!(jwk.alg.as_deref(), Some("RSA-OAEP-256"));
    let reimported = import_key(
        KeyFormat::Jwk,
        import_alg(RsaVariant::RsaOaep, HashAlgorithm::Sha256),
        true,
        vec![KeyUsage::Decrypt, KeyUsage::UnwrapKey],
        KeyData::Jwk(Box::new(jwk)),
    )
    .expect("JWK re-import");
    assert_eq!(reimported.material, private.material);
}

#[test]
fn oaep_jwk_public_round_trip_sha1_bare_alg() {
    let (public, _private) = generate_pair(
        RsaVariant::RsaOaep,
        HashAlgorithm::Sha1,
        // Both halves need a usage (the private half cannot be empty, §14.3.6),
        // so request encrypt (public) + decrypt (private); the public export
        // below carries only the public {encrypt} usage.
        vec![KeyUsage::Encrypt, KeyUsage::Decrypt],
    );
    let jwk = expect_jwk(export_key(KeyFormat::Jwk, &public).expect("JWK export"));
    // §22.4.5 jwk export: SHA-1 → the bare "RSA-OAEP".
    assert_eq!(jwk.alg.as_deref(), Some("RSA-OAEP"));
    assert!(jwk.n.is_some() && jwk.e.is_some());
    assert!(jwk.d.is_none());
    let reimported = import_key(
        KeyFormat::Jwk,
        import_alg(RsaVariant::RsaOaep, HashAlgorithm::Sha1),
        true,
        vec![KeyUsage::Encrypt],
        KeyData::Jwk(Box::new(jwk)),
    )
    .expect("public JWK re-import");
    assert_eq!(reimported.key_type, KeyType::Public);
    assert_eq!(reimported.material, public.material);
}

#[test]
fn import_oaep_public_with_decrypt_usage_is_syntax_error() {
    // §22.4.4: a public key accepts only {encrypt, wrapKey}; `decrypt` (a
    // private-half usage) on an spki import is a SyntaxError.
    let (public, _private) = generate_pair(
        RsaVariant::RsaOaep,
        HashAlgorithm::Sha256,
        vec![KeyUsage::Encrypt, KeyUsage::Decrypt],
    );
    let der = expect_raw(export_key(KeyFormat::Spki, &public).expect("SPKI export"));
    let err = import_key(
        KeyFormat::Spki,
        import_alg(RsaVariant::RsaOaep, HashAlgorithm::Sha256),
        true,
        vec![KeyUsage::Decrypt],
        KeyData::Raw(der),
    )
    .expect_err("decrypt is not a valid public-key usage");
    assert!(matches!(err, AlgorithmError::Syntax(_)), "got {err:?}");
}

#[test]
fn normalize_oaep_label_present_absent_and_wrap_paths() {
    // §22.3: `label` is optional → absent normalizes to `None`.
    let alg = normalize(Operation::Encrypt, RawAlgorithm::from_name("RSA-OAEP"))
        .expect("RSA-OAEP encrypt normalizes");
    assert_eq!(alg, NormalizedAlgorithm::RsaOaep { label: None });
    // A present `label` rides into the normalized algorithm by value.
    let mut raw = RawAlgorithm::from_name("RSA-OAEP");
    raw.label = Some(vec![0xAA, 0xBB, 0xCC]);
    let alg = normalize(Operation::Decrypt, raw).expect("RSA-OAEP decrypt normalizes");
    assert_eq!(
        alg,
        NormalizedAlgorithm::RsaOaep {
            label: Some(vec![0xAA, 0xBB, 0xCC])
        }
    );
    // wrapKey / unwrapKey resolve to the same RsaOaep params (the generic
    // §14.3.11 / §14.3.12 encrypt / decrypt fallback target).
    for op in [Operation::WrapKey, Operation::UnwrapKey] {
        let alg = normalize(op, RawAlgorithm::from_name("rsa-oaep"))
            .expect("RSA-OAEP wrap/unwrap normalizes (case-insensitive)");
        assert_eq!(alg, NormalizedAlgorithm::RsaOaep { label: None });
    }
    // §22.2: RSA-OAEP registers no sign / verify / get-key-length.
    assert!(normalize(Operation::Sign, RawAlgorithm::from_name("RSA-OAEP")).is_err());
    assert!(normalize(Operation::GetKeyLength, RawAlgorithm::from_name("RSA-OAEP")).is_err());
}

// ---------------------------------------------------------------------------
// encrypt / decrypt op-set (the aws-lc-rs backend, §22.4.1 / §22.4.2)
// ---------------------------------------------------------------------------

#[test]
fn oaep_encrypt_decrypt_round_trip_across_hashes_and_labels() {
    // §22.4.1 → §22.4.2 round-trip through the PUBLIC ops API (ops::encrypt /
    // ops::decrypt), so it also pins the §2.2 op-dispatch generalization: the
    // op takes the KEY (RSA keys have no flat byte form), not a pre-extracted
    // `as_bytes()` — the pre-fix path `unreachable!`-panicked for an RSA key.
    // Every WebCrypto hash + an absent / ASCII / NON-UTF-8 label (the binary
    // `BufferSource` the `rsa` crate's String label cannot carry).
    let plaintext = b"a quick brown fox";
    for hash in [
        HashAlgorithm::Sha1,
        HashAlgorithm::Sha256,
        HashAlgorithm::Sha384,
        HashAlgorithm::Sha512,
    ] {
        let (public, private) = generate_pair(
            RsaVariant::RsaOaep,
            hash,
            vec![KeyUsage::Encrypt, KeyUsage::Decrypt],
        );
        for label in [
            None,
            Some(&b"app-label"[..]),
            Some(&[0x00, 0x01, 0xFE, 0xFF, 0x80][..]),
        ] {
            let ciphertext = encrypt(oaep_alg(Operation::Encrypt, label), &public, plaintext)
                .expect("RSA-OAEP encrypt");
            // RSA ciphertext is exactly the modulus size (2048-bit = 256 bytes)
            // and never the plaintext (OAEP is randomized — fresh seed each call).
            assert_eq!(ciphertext.len(), 256, "hash {hash:?}");
            assert_ne!(ciphertext.as_slice(), plaintext);
            let recovered = decrypt(oaep_alg(Operation::Decrypt, label), &private, &ciphertext)
                .expect("RSA-OAEP decrypt");
            assert_eq!(recovered, plaintext, "hash {hash:?}, label {label:?}");
        }
    }
}

#[test]
fn oaep_absent_and_empty_label_interoperate() {
    // WebCrypto treats an absent `label` as the empty label (RFC 3447 §7.1
    // default L = ""), so encrypt-absent ↔ decrypt-empty (and the reverse) must
    // round-trip — pinned by the backend's empty→None label canonicalization.
    let (public, private) = generate_pair(
        RsaVariant::RsaOaep,
        HashAlgorithm::Sha256,
        vec![KeyUsage::Encrypt, KeyUsage::Decrypt],
    );
    let plaintext = b"label parity";
    let empty = Some(&[][..]);
    let ct_absent = encrypt(oaep_alg(Operation::Encrypt, None), &public, plaintext).unwrap();
    assert_eq!(
        decrypt(oaep_alg(Operation::Decrypt, empty), &private, &ct_absent).unwrap(),
        plaintext
    );
    let ct_empty = encrypt(oaep_alg(Operation::Encrypt, empty), &public, plaintext).unwrap();
    assert_eq!(
        decrypt(oaep_alg(Operation::Decrypt, None), &private, &ct_empty).unwrap(),
        plaintext
    );
}

#[test]
fn oaep_decrypt_wrong_label_is_operation_error() {
    let (public, private) = generate_pair(
        RsaVariant::RsaOaep,
        HashAlgorithm::Sha256,
        vec![KeyUsage::Encrypt, KeyUsage::Decrypt],
    );
    let ciphertext = encrypt(
        oaep_alg(Operation::Encrypt, Some(b"label-a")),
        &public,
        b"secret",
    )
    .expect("encrypt with label-a");
    let err = decrypt(
        oaep_alg(Operation::Decrypt, Some(b"label-b")),
        &private,
        &ciphertext,
    )
    .expect_err("decrypt with the wrong label fails");
    assert!(matches!(err, AlgorithmError::Operation(_)), "got {err:?}");
}

#[test]
fn oaep_decrypt_garbage_ciphertext_is_operation_error_not_panic() {
    let (_public, private) = generate_pair(
        RsaVariant::RsaOaep,
        HashAlgorithm::Sha256,
        vec![KeyUsage::Encrypt, KeyUsage::Decrypt],
    );
    // A wrong-size ciphertext and a modulus-size all-zero block both fail OAEP
    // decode as an OperationError (the aws-lc-rs backend returns Err — no panic).
    for ciphertext in [vec![0xAB; 10], vec![0x00; 256]] {
        let err = decrypt(oaep_alg(Operation::Decrypt, None), &private, &ciphertext)
            .expect_err("garbage ciphertext fails");
        assert!(matches!(err, AlgorithmError::Operation(_)), "got {err:?}");
    }
}

#[test]
fn oaep_encrypt_plaintext_too_long_is_operation_error() {
    // §22.4.1 / RFC 3447 §7.1: the message must be ≤ k − 2·hLen − 2 bytes
    // (2048-bit, SHA-256 → 190).  An over-length plaintext is an OperationError.
    let (public, _private) = generate_pair(
        RsaVariant::RsaOaep,
        HashAlgorithm::Sha256,
        vec![KeyUsage::Encrypt, KeyUsage::Decrypt],
    );
    let err = encrypt(oaep_alg(Operation::Encrypt, None), &public, &vec![0u8; 256])
        .expect_err("an over-length plaintext fails");
    assert!(matches!(err, AlgorithmError::Operation(_)), "got {err:?}");
}

#[test]
fn oaep_encrypt_requires_public_key_invalid_access() {
    // §22.4.1 step 1: the backend's `require_public` gate (the type check the
    // op-dispatch inherits) rejects a private key with InvalidAccessError — pin
    // it directly, since the ops usage-split keeps a private key from carrying
    // the `encrypt` usage that would otherwise reach here.
    let (_public, private) = generate_pair(
        RsaVariant::RsaOaep,
        HashAlgorithm::Sha256,
        vec![KeyUsage::Decrypt, KeyUsage::UnwrapKey],
    );
    let err = crate::rsa::oaep_encrypt(&private, HashAlgorithm::Sha256, None, b"x")
        .expect_err("a private key cannot OAEP-encrypt");
    assert!(
        matches!(err, AlgorithmError::InvalidAccess(_)),
        "got {err:?}"
    );
}

#[test]
fn oaep_decrypt_requires_private_key_invalid_access() {
    // §22.4.2 step 1: the backend's private-DER gate rejects a public-only key
    // with InvalidAccessError.  (Both halves need a usage — §14.3.6 forbids an
    // empty private half — so request encrypt + decrypt and take the public.)
    let (public, _private) = generate_pair(
        RsaVariant::RsaOaep,
        HashAlgorithm::Sha256,
        vec![KeyUsage::Encrypt, KeyUsage::Decrypt],
    );
    let err = crate::rsa::oaep_decrypt(&public, HashAlgorithm::Sha256, None, &[0u8; 256])
        .expect_err("a public key cannot OAEP-decrypt");
    assert!(
        matches!(err, AlgorithmError::InvalidAccess(_)),
        "got {err:?}"
    );
}

#[test]
fn oaep_wrap_unwrap_aes_key_round_trip() {
    // §14.3.11 / §14.3.12: RSA-OAEP registers no own wrap op, so wrapKey /
    // unwrapKey fall back to the encrypt / decrypt op — this pins the generic
    // wrap/unwrap path reaches the RSA-OAEP arm of the generalized dispatch
    // (the §2.2 reachability invariant, distinct from the direct encrypt test).
    let (public, private) = generate_pair(
        RsaVariant::RsaOaep,
        HashAlgorithm::Sha256,
        vec![KeyUsage::WrapKey, KeyUsage::UnwrapKey],
    );
    let aes_raw = vec![0x24u8; 16];
    let aes_key = aes_gcm_key(&aes_raw);
    let wrapped = wrap_key(
        oaep_alg(Operation::WrapKey, None),
        &public,
        &aes_key,
        KeyFormat::Raw,
    )
    .expect("RSA-OAEP wrapKey");
    assert_eq!(wrapped.len(), 256);
    let unwrapped = unwrap_key(oaep_alg(Operation::UnwrapKey, None), &private, &wrapped)
        .expect("RSA-OAEP unwrapKey");
    // The §14.3.12 op returns the decrypted key bytes (the raw AES material the
    // VM then imports); for the `raw` format that is the AES key verbatim.
    assert_eq!(unwrapped, aes_raw);
}

#[test]
fn rsa_oaep_decryption_runs_on_constant_time_aws_lc_not_rsa_crate() {
    // Enforceable tripwire for the deny.toml RUSTSEC-2023-0071 (Marvin) ignore
    // under the C1 dual-backend.  The ignore persists (the `rsa` crate still
    // backs keygen / sign / import / export), so cargo-deny cannot distinguish a
    // safe build from one that ships rsa-crate decryption — this test does, by a
    // POSITIVE ∧ NEGATIVE contract:
    //
    //  (positive) RSA-OAEP decryption is performed by the constant-time
    //  aws-lc-rs backend (`rsa/oaep.rs` uses `aws_lc_rs` +
    //  `OaepPrivateDecryptingKey`); and
    //  (negative) NO rsa-crate private-key decryption appears anywhere in src —
    //  not the rsa-crate decryption padding types (`Oaep::`, `Pkcs1v15Encrypt`),
    //  nor an `RsaPrivateKey` `.decrypt(` call.
    //
    // The markers are rsa-crate-specific: aws-lc-rs's decrypt is
    // `OaepPrivateDecryptingKey::decrypt` (the type name has no `Oaep::`
    // substring) on a key that is NOT an `RsaPrivateKey`, so the constant-time
    // backend never false-matches.  If this fails you MUST resolve
    // RUSTSEC-2023-0071 first (drop the ignore for a fixed `rsa` release, move
    // the decryption to the CT backend, or security-review the timing risk — the
    // deny.toml gate).  `tests/` is skipped (this test's own marker strings live
    // here).
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");

    // (positive) the OAEP decrypt path actually runs on aws-lc-rs.
    let oaep = std::fs::read_to_string(src.join("rsa").join("oaep.rs")).expect("read rsa/oaep.rs");
    assert!(
        oaep.contains("aws_lc_rs") && oaep.contains("OaepPrivateDecryptingKey"),
        "rsa/oaep.rs must perform RSA-OAEP decryption via the constant-time \
         aws-lc-rs OaepPrivateDecryptingKey (the Marvin mitigation)"
    );

    // (negative) no rsa-crate private-key decryption anywhere in src.
    fn scan_no_rsa_crate_decrypt(dir: &std::path::Path) {
        for entry in std::fs::read_dir(dir).expect("read src dir") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                if path.file_name() == Some(std::ffi::OsStr::new("tests")) {
                    continue;
                }
                scan_no_rsa_crate_decrypt(&path);
            } else if path.extension() == Some(std::ffi::OsStr::new("rs")) {
                let body = std::fs::read_to_string(&path).expect("read .rs source");
                for marker in ["Oaep::", "Pkcs1v15Encrypt"] {
                    assert!(
                        !body.contains(marker),
                        "rsa-crate decryption padding `{marker}` in {} — resolve \
                         RUSTSEC-2023-0071 (deny.toml Marvin gate) before adding it",
                        path.display(),
                    );
                }
                // The rsa crate's only `.decrypt(` is a method on `RsaPrivateKey`;
                // aws-lc-rs decrypts on `OaepPrivateDecryptingKey` (no
                // `RsaPrivateKey` in that file), so this co-occurrence is exact.
                assert!(
                    !(body.contains("RsaPrivateKey") && body.contains(".decrypt(")),
                    "rsa-crate `RsaPrivateKey` decryption in {} — resolve \
                     RUSTSEC-2023-0071 (deny.toml Marvin gate) before adding it",
                    path.display(),
                );
            }
        }
    }
    scan_no_rsa_crate_decrypt(&src);
}
