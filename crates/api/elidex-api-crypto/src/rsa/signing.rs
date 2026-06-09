//! RSA signing / verification (WebCrypto §20.8.1/.2 RSASSA-PKCS1-v1_5 /
//! §21.4.1/.2 RSA-PSS) — the `sign` / `verify` op-set + the EMSA padding-scheme
//! selectors, on the pure-Rust `rsa` crate.
//!
//! Split from the key-management backend ([`super`]) as a cohesive vertical
//! (parallel to the OAEP op-set in [`super::oaep`]): the signing families are
//! the only `rsa`-crate consumers of the `Pkcs1v15Sign` / `Pss` EMSA schemes
//! and the digest OID marker types, so isolating them keeps the parent's key
//! infrastructure (import / export / generate / reconstruct) clear of the
//! per-scheme padding detail.  The key reconstruction + `[[type]]` gates live in
//! [`super`] (the canonical DER stores) and are reused here via `super::`.

use rsa::traits::PublicKeyParts;
use rsa::{Pkcs1v15Sign, Pss};
use sha1_oid::Sha1;
use sha2_oid::{Sha256, Sha384, Sha512};

use crate::algorithm::RsaVariant;
use crate::error::AlgorithmError;
use crate::hash::HashAlgorithm;
use crate::key::CryptoKeyData;
use crate::rng::ClosureRng;

use super::{
    operation, reconstruct_private, reconstruct_public, require_public, ENTROPY_PROBE_LEN,
};

/// RSA `sign` (WebCrypto §20.8.1 RSASSA-PKCS1-v1_5 / §21.4.1 RSA-PSS): digest
/// `message` with the key's `hash` (carried on `[[algorithm]]`, §20.6), then
/// apply the family padding — RSASSA-PKCS1-v1_5 (RFC 3447 §8.2) applies the
/// EMSA-PKCS1-v1_5 encoding (§9.2, deterministic); RSA-PSS (§8.1) applies
/// EMSA-PSS + MGF1 over a random `salt_length`-byte salt (§9.1).  The §14.3.3
/// name / `sign`-usage gate ran in
/// [`crate::ops::sign`]; this enforces step 1 ([[type]] must be private — via
/// the stored PKCS#8 DER).  `fill_random` is the VM entropy seam — both
/// families consume it: RSA-PSS for the salt, and RSASSA-PKCS1-v1_5 to
/// **blind** the private-key exponentiation (`sign_with_rng` masks the
/// modular exponentiation against timing sidechannels — important since
/// WebCrypto exposes signing to untrusted script; the signature output stays
/// deterministic).  An entropy failure surfaces as an OperationError (the
/// private-key op is never run unblinded).
pub(crate) fn sign<F>(
    variant: RsaVariant,
    hash: HashAlgorithm,
    key: &CryptoKeyData,
    message: &[u8],
    salt_length: Option<u32>,
    mut fill_random: F,
) -> Result<Vec<u8>, AlgorithmError>
where
    F: FnMut(&mut [u8]) -> Result<(), AlgorithmError>,
{
    // §20.8.1 / §21.4.1 step 1: the key must be private (reconstruct from the
    // stored PKCS#8 DER, InvalidAccessError if public).
    let privkey = reconstruct_private(key)?;
    // Fail BEFORE the private-key exponentiation if the entropy seam is down.
    // `sign_with_rng` blinds (and, for PSS, salts) via `ClosureRng`, whose
    // infallible `fill_bytes` falls back to a *deterministic* stream on a
    // `fill_random` error so the rsa op can still complete — acceptable for
    // keygen (the candidate key is discarded), but for *signing* an existing
    // key it would run the exponentiation with predictable blinding, exactly
    // the timing-observable private-key work the Marvin (RUSTSEC-2023-0071)
    // mitigation must avoid.  Probe the seam first so a down CSPRNG rejects the
    // sign before any private-key work runs.  (A CSPRNG seam is live or down
    // for the whole call; the per-arm `into_result()` below is the backstop for
    // the non-realistic case where the probe succeeds but a later draw fails —
    // the op then runs on the fallback but its signature is still rejected.)
    let mut entropy_probe = [0u8; ENTROPY_PROBE_LEN];
    fill_random(&mut entropy_probe)?;
    let digest = hash.digest(message);
    match variant {
        RsaVariant::RsassaPkcs1V15 => {
            // `sign_with_rng` (not `sign`) so the rsa crate blinds the
            // private-key operation with the entropy seam — `sign` uses a
            // DummyRng (no blinding).  The EMSA-PKCS1-v1_5 signature output is
            // still deterministic; only the exponentiation timing is masked.
            let mut rng = ClosureRng::new(&mut fill_random);
            let result = privkey.sign_with_rng(&mut rng, pkcs1v15_scheme(hash), &digest);
            // Backstop for a draw that fails after the pre-op probe: reject the
            // (fallback-blinded) signature rather than return it.
            rng.into_result()?;
            result.map_err(|_| operation("RSASSA-PKCS1-v1_5 signing failed"))
        }
        RsaVariant::RsaPss => {
            let salt_len = pss_salt_len(salt_length)?;
            // DoS ceiling (a cheap over-approximation, NOT the exact §9.1.1
            // validity bound): the rsa crate allocates + random-fills a
            // `vec![0; saltLength]` and only *then* checks the EMSA-PSS encoding
            // bound, so an attacker-supplied `saltLength = 2^32 − 1` would OOM the
            // thread first.  RFC 3447 §9.1.1 requires emLen ≥ hLen + saltLength +
            // 2, so any saltLength past the modulus byte size is certainly invalid
            // — reject those up front here; the rsa crate still rejects the narrow,
            // alloc-bounded window just below the ceiling as an OperationError.
            let modulus_bytes = privkey.n().bits().div_ceil(8);
            if salt_len > modulus_bytes {
                return Err(operation("RSA-PSS saltLength exceeds the modulus size"));
            }
            let mut rng = ClosureRng::new(&mut fill_random);
            let result = privkey.sign_with_rng(&mut rng, pss_scheme(hash, salt_len), &digest);
            // A `fill_random` error wins over the (otherwise opaque) PSS error.
            rng.into_result()?;
            result.map_err(|_| operation("RSA-PSS signing failed"))
        }
        // RSA-OAEP (WebCrypto §22) is an encrypt-only family: it never reaches
        // `sign`.  The registry resolves (Sign, "RSA-OAEP") to NotSupported, and
        // `ops::sign`'s name-match (RSASSA / RSA-PSS ≠ RSA-OAEP) rejects an OAEP
        // key before this dispatch.  Guard with an error rather than
        // `unreachable!` so a contract violation surfaces gracefully, not as a
        // panic (the OAEP encrypt / decrypt op-set lives off the signing path).
        RsaVariant::RsaOaep => Err(operation("RSA-OAEP keys do not support 'sign'")),
    }
}

/// RSA `verify` (WebCrypto §20.8.2 RSASSA-PKCS1-v1_5 / §21.4.2 RSA-PSS): digest
/// `message`, then verify `signature` against the public key.  The §14.3.4 name
/// / `verify`-usage gate ran in [`crate::ops::verify`]; this enforces step 1
/// ([[type]] must be public) and returns **false** (not an error) on an invalid
/// signature.  For RSA-PSS the `salt_length` is enforced (RFC 3447 §9.1.2 — a
/// signature whose recovered salt length differs is invalid → false).
pub(crate) fn verify(
    variant: RsaVariant,
    hash: HashAlgorithm,
    key: &CryptoKeyData,
    signature: &[u8],
    message: &[u8],
    salt_length: Option<u32>,
) -> Result<bool, AlgorithmError> {
    // §20.8.2 / §21.4.2 step 1: the key must be public.
    require_public(key)?;
    let pubkey = reconstruct_public(key)?;
    let digest = hash.digest(message);
    let ok = match variant {
        RsaVariant::RsassaPkcs1V15 => pubkey
            .verify(pkcs1v15_scheme(hash), &digest, signature)
            .is_ok(),
        RsaVariant::RsaPss => {
            let salt_len = pss_salt_len(salt_length)?;
            pubkey
                .verify(pss_scheme(hash, salt_len), &digest, signature)
                .is_ok()
        }
        // RSA-OAEP (§22) is encrypt-only; it never reaches `verify` (guarded as
        // in `sign`).  Return an error rather than `unreachable!`.
        RsaVariant::RsaOaep => return Err(operation("RSA-OAEP keys do not support 'verify'")),
    };
    Ok(ok)
}

/// The `Pkcs1v15Sign` scheme for `hash` — `Pkcs1v15Sign::new::<D>()` derives
/// the RFC 3447 §9.2 DigestInfo prefix from the digest's OID (the `rsa::sha*`
/// 0.10 marker type), while the digest itself is the prehashed bytes from
/// hash.rs (sha2 0.11).
fn pkcs1v15_scheme(hash: HashAlgorithm) -> Pkcs1v15Sign {
    match hash {
        HashAlgorithm::Sha1 => Pkcs1v15Sign::new::<Sha1>(),
        HashAlgorithm::Sha256 => Pkcs1v15Sign::new::<Sha256>(),
        HashAlgorithm::Sha384 => Pkcs1v15Sign::new::<Sha384>(),
        HashAlgorithm::Sha512 => Pkcs1v15Sign::new::<Sha512>(),
    }
}

/// The `Pss` scheme for `hash` + `salt_len` — `Pss::new_blinded_with_salt::<D>`
/// sets the MGF1 hash + the enforced salt length (the EMSA-PSS encoding,
/// RFC 3447 §9.1).  The **`_blinded_`** constructor is load-bearing for signing:
/// in rsa 0.9 `Pss::sign` only blinds the private-key exponentiation when its
/// `blinded` flag is set (`sign(blind.then_some(rng), …)`), and `new_with_salt`
/// leaves it `false` — so a plain `Pss` would draw the RNG for the salt yet run
/// the exponentiation *unblinded*, leaving RSA-PSS `sign()` timing-observable
/// (the Marvin / RUSTSEC-2023-0071 surface the `deny.toml` rationale relies on
/// being mitigated; RSASSA's `Pkcs1v15Sign` blinds unconditionally under
/// `sign_with_rng`).  On the `verify` path the flag is inert (a public-key
/// operation has no private exponent to blind).
fn pss_scheme(hash: HashAlgorithm, salt_len: usize) -> Pss {
    match hash {
        HashAlgorithm::Sha1 => Pss::new_blinded_with_salt::<Sha1>(salt_len),
        HashAlgorithm::Sha256 => Pss::new_blinded_with_salt::<Sha256>(salt_len),
        HashAlgorithm::Sha384 => Pss::new_blinded_with_salt::<Sha384>(salt_len),
        HashAlgorithm::Sha512 => Pss::new_blinded_with_salt::<Sha512>(salt_len),
    }
}

/// The RSA-PSS `saltLength` as a `usize` — required (the registry guarantees
/// `RsaPssParams.saltLength` is present for a PSS sign / verify, §21.3), so its
/// absence is a defensive OperationError.
fn pss_salt_len(salt_length: Option<u32>) -> Result<usize, AlgorithmError> {
    Ok(salt_length.ok_or_else(|| operation("RSA-PSS requires a saltLength"))? as usize)
}
