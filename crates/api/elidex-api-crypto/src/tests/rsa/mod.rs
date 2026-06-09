//! RSA (RSASSA-PKCS1-v1_5 / RSA-PSS) crate-level tests (WebCrypto §20 / §21):
//! the generateKey usage-split + key shape, the import / export round-trips
//! across spki / pkcs8 / jwk, RSASSA / RSA-PSS sign / verify, and the §20.8.4 /
//! §21.4.4 invalid-shape (DataError / SyntaxError / NotSupported) set.
//!
//! Keys are produced by a **seeded** deterministic CSPRNG (reproducible,
//! no OS-entropy dependency) at the 2048-bit modulus length.  The JS-level
//! vertical lives in the VM crate's `tests_crypto::rsa`; the `marshal_jwk` ≡
//! `from_json_bytes` JWK mirror differential test lives in
//! `subtle_crypto::differential`.
//!
//! The surface exceeds the 1000-line file convention, so it is split into a
//! directory module by theme — the shared key/algorithm helpers live here and
//! are reused by the submodules:
//!
//! - `keygen` — generateKey shape / usage-split / WebIDL member order /
//!   modulus ceiling / entropy + DoS bounds, and the Marvin tripwire.
//! - `roundtrip` — import / export round-trips (spki / pkcs8 / jwk) and the
//!   §20.8.4 / §21.4.4 invalid-shape (DataError / SyntaxError / NotSupported)
//!   matrix.
//! - `sign_verify` — RSASSA / RSA-PSS sign / verify, blinding, salt-length and
//!   cross-hash boundaries.

use rand_chacha::rand_core::{RngCore, SeedableRng};

use crate::algorithm::RsaVariant;
use crate::error::AlgorithmError;
use crate::hash::HashAlgorithm;
use crate::key::KeyUsage;
use crate::ops::{generate_key, import_key, ExportedKey, GeneratedKey, KeyData, KeyFormat};
use crate::{normalize, CryptoKeyData, JsonWebKey, NormalizedAlgorithm, Operation, RawAlgorithm};

mod keygen;
mod roundtrip;
mod sign_verify;

/// A deterministic `fill_random` closure (a seeded ChaCha20 CSPRNG) for
/// reproducible RSA key generation — valid randomness without OS entropy.
fn seeded_fill(seed: u64) -> impl FnMut(&mut [u8]) -> Result<(), AlgorithmError> {
    let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(seed);
    move |buf| {
        rng.fill_bytes(buf);
        Ok(())
    }
}

fn keygen_alg(
    variant: RsaVariant,
    modulus_length: u32,
    hash: HashAlgorithm,
) -> NormalizedAlgorithm {
    let mut raw = RawAlgorithm::from_name(variant.canonical_name());
    raw.modulus_length = Some(modulus_length);
    raw.public_exponent = Some(vec![0x01, 0x00, 0x01]);
    raw.hash = Some(Box::new(RawAlgorithm::from_name(hash.canonical_name())));
    normalize(Operation::GenerateKey, raw).expect("RSA keygen algorithm normalizes")
}

fn import_alg(variant: RsaVariant, hash: HashAlgorithm) -> NormalizedAlgorithm {
    let mut raw = RawAlgorithm::from_name(variant.canonical_name());
    raw.hash = Some(Box::new(RawAlgorithm::from_name(hash.canonical_name())));
    normalize(Operation::ImportKey, raw).expect("RSA import algorithm normalizes")
}

fn sign_alg(variant: RsaVariant, salt_length: Option<u32>) -> NormalizedAlgorithm {
    let mut raw = RawAlgorithm::from_name(variant.canonical_name());
    raw.salt_length = salt_length;
    normalize(Operation::Sign, raw).expect("RSA sign algorithm normalizes")
}

fn verify_alg(variant: RsaVariant, salt_length: Option<u32>) -> NormalizedAlgorithm {
    let mut raw = RawAlgorithm::from_name(variant.canonical_name());
    raw.salt_length = salt_length;
    normalize(Operation::Verify, raw).expect("RSA verify algorithm normalizes")
}

/// Generate a `(public, private)` RSA key pair at 2048 bits over a fixed seed.
fn generate_pair(
    variant: RsaVariant,
    hash: HashAlgorithm,
    usages: Vec<KeyUsage>,
) -> (CryptoKeyData, CryptoKeyData) {
    match generate_key(
        keygen_alg(variant, 2048, hash),
        true,
        usages,
        seeded_fill(0x5A),
    )
    .unwrap()
    {
        GeneratedKey::Pair { public, private } => (public, private),
        GeneratedKey::Single(_) => panic!("RSA generateKey yields a key pair"),
    }
}

fn expect_jwk(exported: ExportedKey) -> JsonWebKey {
    match exported {
        ExportedKey::Jwk(jwk) => *jwk,
        ExportedKey::Raw(_) => panic!("expected a JWK export"),
    }
}

fn expect_raw(exported: ExportedKey) -> Vec<u8> {
    match exported {
        ExportedKey::Raw(bytes) => bytes,
        ExportedKey::Jwk(_) => panic!("expected a raw / DER export"),
    }
}

/// Import a public JWK (usages = `verify`) and return the error.
fn import_public_jwk_err(jwk: JsonWebKey) -> AlgorithmError {
    import_key(
        KeyFormat::Jwk,
        import_alg(RsaVariant::RsassaPkcs1V15, HashAlgorithm::Sha256),
        true,
        vec![KeyUsage::Verify],
        KeyData::Jwk(Box::new(jwk)),
    )
    .expect_err("invalid JWK is rejected")
}

/// Import a private JWK (usages = `sign`) and return the error.
fn import_private_jwk_err(jwk: JsonWebKey) -> AlgorithmError {
    import_key(
        KeyFormat::Jwk,
        import_alg(RsaVariant::RsassaPkcs1V15, HashAlgorithm::Sha256),
        true,
        vec![KeyUsage::Sign],
        KeyData::Jwk(Box::new(jwk)),
    )
    .expect_err("invalid JWK is rejected")
}

fn rsa_jwk(members: &[(&str, &str)]) -> JsonWebKey {
    let mut jwk = JsonWebKey {
        kty: Some("RSA".to_string()),
        ..Default::default()
    };
    for &(k, v) in members {
        let v = Some(v.to_string());
        match k {
            "n" => jwk.n = v,
            "e" => jwk.e = v,
            "d" => jwk.d = v,
            "p" => jwk.p = v,
            "q" => jwk.q = v,
            "dp" => jwk.dp = v,
            "dq" => jwk.dq = v,
            "qi" => jwk.qi = v,
            other => panic!("unknown JWK member {other}"),
        }
    }
    jwk
}
