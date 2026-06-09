//! RSA key import / export + RSASSA-PKCS1-v1_5 / RSA-PSS sign / verify
//! (WebCrypto §20 RSASSA-PKCS1-v1_5 / §21 RSA-PSS), reached only through
//! [`crate::ops`] (which runs the §14.3.x name / usage / extractable gates),
//! so the rsa-typed APIs are `pub(crate)` — not a public surface.
//!
//! Mirrors `ec`: the engine-independent [`crate::key::KeyMaterial::Rsa`]
//! stores the canonical SubjectPublicKeyInfo + PKCS#8 DER, and the typed
//! `rsa::RsaPublicKey` / `RsaPrivateKey` is reconstructed here at op time (the
//! asymmetric analogue of `Raw(bytes)` → cipher).  RSA has no flat semantic
//! byte form (its key is 8+ BigUints), so its canonical multi-component
//! encoding *is* the DER — the faithful RSA analogue of EC's SEC1-bytes
//! storage.  Import re-encodes to canonical DER (so a single storage form
//! backs every format), which also gates multi-prime keys: the rsa crate's
//! `to_pkcs8_der` rejects >2 primes, so a multi-prime `pkcs8` / JWK `oth`
//! import is a NotSupportedError (`#11-rsa-multiprime-jwk`).

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use rsa::pkcs8::spki::{DecodePublicKey, EncodePublicKey};
use rsa::pkcs8::{DecodePrivateKey, EncodePrivateKey};
use rsa::traits::{PrivateKeyParts, PublicKeyParts};
use rsa::{BigUint, RsaPrivateKey, RsaPublicKey};

use crate::algorithm::RsaVariant;
use crate::error::AlgorithmError;
use crate::hash::HashAlgorithm;
use crate::jwk::{self, JsonWebKey};
use crate::key::{normalize_usages, CryptoKeyData, KeyMaterial, KeyType, KeyUsage};
use crate::ops::{format_data_mismatch, ExportedKey, KeyData, KeyFormat};
use crate::rng::ClosureRng;

/// RSA-OAEP encrypt / decrypt (WebCrypto §22) on the constant-time aws-lc-rs
/// backend — the encrypt family of the RSA keys, split from this module's
/// `rsa`-crate key-management backend (see [`oaep`]).
mod oaep;
pub(crate) use oaep::{oaep_decrypt, oaep_encrypt};

/// RSA `sign` / `verify` (WebCrypto §20.8.1/.2 / §21.4.1/.2) on the `rsa` crate
/// — the signing vertical, split from this module's key-management backend as a
/// cohesive op-set (parallel to [`oaep`]); see [`signing`].
mod signing;
pub(crate) use signing::{sign, verify};

/// Upper bound on an RSA modulus (bits) — the script-controlled
/// `generateKey` `modulusLength` (an unbounded value would DoS the synchronous
/// VM-thread keygen) and every imported modulus ([`check_modulus_bits`]).
///
/// Pinned to the rsa crate's `RsaPublicKey::MAX_SIZE` (4096): public keys are
/// reconstructed with `RsaPublicKey::new` / `from_public_key_der`, which reject
/// a modulus above `MAX_SIZE` with `ModulusTooLarge`.  Advertising a higher
/// ceiling would be inconsistent — `generateKey` (whose `new_with_exp` enforces
/// no size) could mint a key whose public half then fails to reconstruct, and
/// large imported verification keys would fail despite the stated policy.  4096
/// covers every standard RSA key; supporting larger moduli needs the
/// `new_with_max_size` constructor on every public-key path plus a custom SPKI
/// decode (follow-on `#11-rsa-modulus-above-4096`).  Kept in lockstep by
/// `tests::rsa::modulus_ceiling_matches_rsa_crate_max_size`.
pub(crate) const MAX_RSA_MODULUS_BITS: u32 = 4096;

/// Lower bound on an **RSA-OAEP** modulus (bits).  Unlike the signing variants
/// (RSASSA-PKCS1-v1_5 / RSA-PSS, which run on the pure-Rust `rsa` crate at any
/// size), RSA-OAEP `encrypt` / `decrypt` run on the constant-time aws-lc-rs
/// backend ([`oaep`]), whose OAEP keys are restricted to `2048..=8192` bits
/// (`OaepPublicEncryptingKey::new` / `OaepPrivateDecryptingKey::new`,
/// aws-lc-rs 1.17 `rsa/encryption.rs`).  A smaller modulus would `generateKey`
/// / `importKey` successfully on the `rsa` crate yet be **unusable** for every
/// OAEP op (the aws-lc-rs key construction returns `Err` → OperationError), so
/// an accepted RSA-OAEP key would not be usable.  Reject it at the gate instead
/// (generate + import), so a successfully-imported RSA-OAEP key is always
/// usable.  WebCrypto §22 sets no minimum (the floor is a backend capability),
/// so this is NotSupported / OperationError, not a spec requirement.  The floor
/// is OAEP-specific — the signing families keep no minimum.
pub(crate) const MIN_RSA_OAEP_MODULUS_BITS: u32 = 2048;

/// Whether an RSA modulus of `bits` is below the OAEP backend's minimum for
/// `variant`.  Only RSA-OAEP (the aws-lc-rs encrypt/decrypt backend) has a
/// floor; the signing variants accept any size the `rsa` crate produces.
fn rsa_oaep_modulus_below_minimum(variant: RsaVariant, bits: u32) -> bool {
    variant == RsaVariant::RsaOaep && bits < MIN_RSA_OAEP_MODULUS_BITS
}

/// Liveness probe drawn from the entropy seam **before** an RSA private-key
/// *signing* exponentiation (see [`sign`]).  A CSPRNG seam is live or down for
/// the whole call, so one byte suffices to detect a down seam and fail before
/// any private-key work runs.  `pub(crate)` so the blinding test can subtract
/// it from the seam-draw count.
pub(crate) const ENTROPY_PROBE_LEN: usize = 1;

// ---------------------------------------------------------------------------
// importKey (WebCrypto §20.8.4 / §21.4.4)
// ---------------------------------------------------------------------------

/// `importKey` for an RSA signing algorithm (WebCrypto §20.8.4
/// RSASSA-PKCS1-v1_5 / §21.4.4 RSA-PSS).  `hash` is the `RsaHashedImportParams`
/// hash (§20.7) carried on the produced key's `[[algorithm]]`.  `raw` is not a
/// registered RSA import format (§20.8.4 lists only spki / pkcs8 / jwk).
pub(crate) fn import(
    variant: RsaVariant,
    hash: HashAlgorithm,
    format: KeyFormat,
    extractable: bool,
    usages: Vec<KeyUsage>,
    key_data: KeyData,
) -> Result<CryptoKeyData, AlgorithmError> {
    // Each branch runs the §-step order: the usage SyntaxError check (which
    // depends on the key type the format implies) precedes the key-material
    // parse (the DataError set).  `jwk` determines its key type from the `d`
    // member, so it validates usages internally.
    let imported = match (format, key_data) {
        (KeyFormat::Spki, KeyData::Raw(der)) => {
            // §20.8.4 spki: a public-only format.
            validate_import_usages(variant, KeyType::Public, &usages)?;
            let pubkey = parse_spki(&der)?;
            public_imported(&pubkey)?
        }
        (KeyFormat::Pkcs8, KeyData::Raw(der)) => {
            // §20.8.4 pkcs8: a private-only format.
            validate_import_usages(variant, KeyType::Private, &usages)?;
            let privkey = parse_pkcs8(&der)?;
            private_imported(&privkey)?
        }
        (KeyFormat::Jwk, KeyData::Jwk(jwk)) => {
            import_jwk(variant, hash, extractable, &usages, &jwk)?
        }
        (KeyFormat::Raw, _) => {
            return Err(AlgorithmError::NotSupported(
                "RSA import supports only the 'spki', 'pkcs8' and 'jwk' formats".to_string(),
            ));
        }
        // Format / data shape mismatch — the VM marshals them consistently
        // (spki/pkcs8 → Raw, jwk → Jwk), so this is a defensive guard.
        _ => return Err(format_data_mismatch()),
    };
    // An RSA-OAEP key below the aws-lc-rs OAEP backend's minimum modulus parses
    // here but is unusable for encrypt/decrypt (see [`MIN_RSA_OAEP_MODULUS_BITS`])
    // — reject it as an unsupported capability, uniformly across spki/pkcs8/jwk
    // (every branch produces `imported.modulus_length`), so an imported RSA-OAEP
    // key is always usable.  The signing variants keep no floor.
    if rsa_oaep_modulus_below_minimum(variant, imported.modulus_length) {
        return Err(AlgorithmError::NotSupported(
            "RSA-OAEP modulus length is below the supported minimum (2048 bits)".to_string(),
        ));
    }
    // §14.3.9 importKey generic step: a private key with empty usages is a
    // SyntaxError — but an RSA *public* key may have empty usages.  Checked
    // after the algorithm-specific parse, so a DataError from invalid material
    // wins.
    if imported.key_type == KeyType::Private && usages.is_empty() {
        return Err(AlgorithmError::Syntax("usages cannot be empty".to_string()));
    }
    let usages = normalize_usages(usages);
    Ok(CryptoKeyData {
        key_type: imported.key_type,
        extractable,
        algorithm: variant.key_algorithm(imported.modulus_length, imported.public_exponent, hash),
        usages,
        material: imported.material,
    })
}

/// The parsed-key facts an import branch produces before the generic
/// empty-usages gate + the `CryptoKeyData` assembly.
struct Imported {
    key_type: KeyType,
    material: KeyMaterial,
    modulus_length: u32,
    public_exponent: Vec<u8>,
}

/// Parse a SubjectPublicKeyInfo (WebCrypto §20.8.4 spki): `from_public_key_der`
/// validates the rsaEncryption OID + the RSA structure (a non-RSA / malformed
/// SPKI → decode error → DataError).  It also enforces the modulus
/// (`RsaPublicKey::MAX_SIZE`) and exponent (`MAX_PUB_EXPONENT`) caps *during*
/// construction, so an oversized / large-exponent SPKI key is a DataError here
/// rather than the JWK path's NotSupported capability error — surfacing that as
/// NotSupported would need a custom SPKI decode of `n` / `e` before the rsa
/// crate's capped `try_from`, deferred to `#11-rsa-modulus-above-4096`
/// (the constructor caps make a separate `check_modulus_bits` here dead code).
fn parse_spki(der: &[u8]) -> Result<RsaPublicKey, AlgorithmError> {
    RsaPublicKey::from_public_key_der(der)
        .map_err(|_| data("invalid SubjectPublicKeyInfo RSA public key"))
}

/// Parse a PKCS#8 PrivateKeyInfo (WebCrypto §20.8.4 pkcs8): `from_pkcs8_der`
/// validates the rsaEncryption OID + the RSA structure.  Unlike
/// `from_public_key_der`, `from_pkcs8_der` does NOT apply `RsaPublicKey::MAX_SIZE`,
/// so the explicit `check_modulus_bits` is *reached* and required — without it a
/// private key whose modulus exceeds `MAX_RSA_MODULUS_BITS` would import yet be
/// unusable (its public half can't be reconstructed via `RsaPublicKey::new`).
fn parse_pkcs8(der: &[u8]) -> Result<RsaPrivateKey, AlgorithmError> {
    let key =
        RsaPrivateKey::from_pkcs8_der(der).map_err(|_| data("invalid PKCS#8 RSA private key"))?;
    check_modulus_bits(key.n().bits())?;
    Ok(key)
}

/// Build the [`Imported`] facts for a public key: the canonical SPKI DER +
/// the modulus length / public exponent for the `[[algorithm]]`.
fn public_imported(pubkey: &RsaPublicKey) -> Result<Imported, AlgorithmError> {
    Ok(Imported {
        key_type: KeyType::Public,
        material: KeyMaterial::Rsa {
            public_spki_der: encode_spki(pubkey)?,
            private_pkcs8_der: None,
        },
        modulus_length: modulus_bits(pubkey)?,
        public_exponent: pubkey.e().to_bytes_be(),
    })
}

/// Build the [`Imported`] facts for a private key: the canonical PKCS#8 +
/// derived SPKI DER.  A multi-prime key (>2 primes) is a NotSupportedError —
/// the rsa crate's `to_pkcs8_der` cannot encode it (`#11-rsa-multiprime-jwk`).
fn private_imported(privkey: &RsaPrivateKey) -> Result<Imported, AlgorithmError> {
    if privkey.primes().len() > 2 {
        return Err(multiprime_unsupported());
    }
    let private_pkcs8_der = privkey
        .to_pkcs8_der()
        .map_err(|_| data("RSA private key cannot be encoded"))?
        .as_bytes()
        .to_vec();
    let public_spki_der = encode_spki(&RsaPublicKey::from(privkey))?;
    Ok(Imported {
        key_type: KeyType::Private,
        material: KeyMaterial::Rsa {
            public_spki_der,
            private_pkcs8_der: Some(private_pkcs8_der),
        },
        modulus_length: modulus_bits(privkey)?,
        public_exponent: privkey.e().to_bytes_be(),
    })
}

/// Import an RSA `jwk` (WebCrypto §20.8.4 / §21.4.4 / §22.4.4 jwk branch +
/// RFC 7518 §6.3): validate the JWK shape (kty / use / key_ops / ext / alg),
/// determine the key type from the `d` member, then reconstruct the typed key
/// from n / e [/ d / p / q].  Multi-prime (`oth`) is NotSupported.
fn import_jwk(
    variant: RsaVariant,
    hash: HashAlgorithm,
    extractable: bool,
    usages: &[KeyUsage],
    jwk: &JsonWebKey,
) -> Result<Imported, AlgorithmError> {
    // step 2/3: the `d` member determines the key type; the usage SyntaxError
    // check runs before the DataError shape checks.
    let key_type = if jwk.d.is_some() {
        KeyType::Private
    } else {
        KeyType::Public
    };
    validate_import_usages(variant, key_type, usages)?;
    // kty must be "RSA" (§20.8.4 / §21.4.4 / §22.4.4).
    if jwk.kty.as_deref() != Some("RSA") {
        return Err(data("JWK 'kty' member must be 'RSA' for an RSA key"));
    }
    // use, if present (and usages non-empty), must match the family (§20.8.4
    // step 4 / §21.4.4 step 4 → "sig"; §22.4.4 step 5 RSA-OAEP → "enc").
    if !usages.is_empty() {
        if let Some(use_) = jwk.use_.as_deref() {
            if use_ != variant.jwk_use() {
                return Err(data_owned(format!(
                    "JWK 'use' member must be '{}' for an {} key",
                    variant.jwk_use(),
                    variant.canonical_name()
                )));
            }
        }
    }
    // key_ops must be a valid superset of the requested usages.
    if let Some(key_ops) = &jwk.key_ops {
        jwk::validate_key_ops(key_ops, usages)?;
    }
    // ext=false cannot satisfy an extractable=true import.
    if let Some(false) = jwk.ext {
        if extractable {
            return Err(data(
                "JWK 'ext' member is false but an extractable key was requested",
            ));
        }
    }
    // alg, if present, must match the variant + hash (RS256 / PS384 / …).
    if let Some(alg) = jwk.alg.as_deref() {
        if alg != variant.jwk_alg(hash) {
            return Err(data("JWK 'alg' member does not match the algorithm / hash"));
        }
    }
    // n / e are required for both public and private keys (RFC 7518 §6.3.1).
    let n = decode_biguint(jwk.n.as_deref(), "n")?;
    let e = decode_biguint(jwk.e.as_deref(), "e")?;
    // Bound the modulus + exponent BEFORE constructing the key, as NotSupported
    // capability boundaries (not the rsa crate's generic DataError): the private
    // branch's `from_components` runs prime recovery on a d-only JWK, which must
    // not be reached on an oversized attacker-controlled `n` (DoS), and a public
    // exponent over the rsa crate's cap is an unsupported capability, not
    // malformed material.
    check_modulus_bits(n.bits())?;
    check_public_exponent(&e)?;
    match key_type {
        KeyType::Public => {
            let pubkey = RsaPublicKey::new(n, e)
                .map_err(|_| data("JWK RSA public key (n, e) is invalid"))?;
            public_imported(&pubkey)
        }
        KeyType::Private => {
            // Multi-prime (`oth` present) private keys are not supported — the
            // rsa crate's DER encoder rejects >2 primes (`#11-rsa-multiprime-jwk`);
            // RFC 7518 §6.3.2.7 says `oth` MUST be absent for a two-prime key,
            // so even an empty `oth: []` is an unsupported multi-prime shape.
            // This is checked ONLY on the private branch: WebCrypto interprets a
            // *public* JWK per RFC 7518 §6.3.1 (n / e only — §20.8.4 / §21.4.4
            // "Otherwise" step), which never references `oth`, so a public import
            // ignores it exactly as it already ignores p / q / d.
            if jwk.oth.is_some() {
                return Err(multiprime_unsupported());
            }
            // d is required (the §-determined private branch); p / q / dp / dq /
            // qi are all-or-nothing (RFC 7518 §6.3.2 — see [`jwk_primes`]).
            let d = decode_biguint(jwk.d.as_deref(), "d")?;
            let primes = jwk_primes(jwk)?;
            // A CRT-less (d-only) JWK relies on `from_components`' prime
            // recovery (NIST SP 800-56B C.2), which the rsa crate supports only
            // for a public exponent in (2^16, 2^256).  A *valid* d-only key with
            // an out-of-range exponent (e.g. e=3) is a crate-capability
            // boundary, not malformed input → NotSupportedError
            // (`#11-rsa-jwk-d-only-small-exponent`); checked explicitly so it
            // stays distinct from the DataError below for an in-range-but-
            // inconsistent key (where `from_components` recovery genuinely fails
            // on bad n/e/d).
            if primes.is_empty() {
                // recover_primes requires 2^16 < e < 2^256.  2^16 = 65536;
                // 2^256 is a 1 byte followed by 32 zero bytes.
                let e_min = BigUint::from(65536u32);
                let mut e_max_bytes = [0u8; 33];
                e_max_bytes[0] = 1;
                let e_max = BigUint::from_bytes_be(&e_max_bytes);
                if e <= e_min || e >= e_max {
                    return Err(AlgorithmError::NotSupported(
                        "RSA private JWK without prime factors (p / q) is supported only for a \
                         public exponent in (2^16, 2^256)"
                            .to_string(),
                    ));
                }
            }
            let privkey = RsaPrivateKey::from_components(n, e, d, primes)
                .map_err(|_| data("JWK RSA private key is invalid or inconsistent"))?;
            // `from_components` recomputes the CRT values from p / q / d, so a
            // JWK carrying *corrupted* dp / dq / qi would otherwise import +
            // silently re-export with repaired values.  RFC 7518 §6.3.2 defines
            // those members as part of the private key, so reject inconsistent
            // material as a DataError rather than repairing it.
            validate_jwk_crt(jwk, &privkey)?;
            private_imported(&privkey)
        }
        KeyType::Secret => unreachable!("RSA keys are never secret"),
    }
}

/// The JWK private-key primes for `RsaPrivateKey::from_components` (RFC 7518
/// §6.3.2): the optional CRT members `p` / `q` / `dp` / `dq` / `qi` are
/// **all-or-nothing** ("if the producer includes any of the other private key
/// parameters, then all of the others MUST also be present").  When present,
/// `[p, q]` is returned (`from_components` recomputes dp / dq / qi); when
/// absent, an empty `Vec` lets `from_components` recover p / q from d.
fn jwk_primes(jwk: &JsonWebKey) -> Result<Vec<BigUint>, AlgorithmError> {
    let members = [&jwk.p, &jwk.q, &jwk.dp, &jwk.dq, &jwk.qi];
    let any = members.iter().any(|m| m.is_some());
    let all = members.iter().all(|m| m.is_some());
    if any && !all {
        return Err(data(
            "JWK RSA private key must include all of p / q / dp / dq / qi, or none",
        ));
    }
    if !any {
        return Ok(Vec::new());
    }
    let p = decode_biguint(jwk.p.as_deref(), "p")?;
    let q = decode_biguint(jwk.q.as_deref(), "q")?;
    Ok(vec![p, q])
}

/// Validate the JWK's supplied CRT members (`dp` / `dq` / `qi`) against the
/// values recomputed from p / q / d (RFC 7518 §6.3.2.4-.6).  Absent → ok (the
/// CRT members are all-or-nothing, [`jwk_primes`]); present but inconsistent →
/// DataError (the key material is malformed — do not silently repair it).
// `dp` / `dq` (and `expected_dp` / `expected_dq`) are the canonical RFC 7518
// CRT-exponent member names — renaming them to satisfy `similar_names` would
// obscure the spec mapping.
#[allow(clippy::similar_names)]
fn validate_jwk_crt(jwk: &JsonWebKey, privkey: &RsaPrivateKey) -> Result<(), AlgorithmError> {
    // All-or-nothing: if `dp` is absent so are `dq` / `qi` (jwk_primes gate).
    if jwk.dp.is_none() {
        return Ok(());
    }
    let expected_dp = privkey.dp().ok_or_else(key_inaccessible)?;
    let expected_dq = privkey.dq().ok_or_else(key_inaccessible)?;
    let expected_qi = privkey.crt_coefficient().ok_or_else(key_inaccessible)?;
    if &decode_biguint(jwk.dp.as_deref(), "dp")? != expected_dp {
        return Err(data("JWK 'dp' is inconsistent with the RSA private key"));
    }
    if &decode_biguint(jwk.dq.as_deref(), "dq")? != expected_dq {
        return Err(data("JWK 'dq' is inconsistent with the RSA private key"));
    }
    if decode_biguint(jwk.qi.as_deref(), "qi")? != expected_qi {
        return Err(data("JWK 'qi' is inconsistent with the RSA private key"));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// generateKey (WebCrypto §20.8.3 / §21.4.3)
// ---------------------------------------------------------------------------

/// `generateKey` for an RSA algorithm (WebCrypto §20.8.3 RSASSA-PKCS1-v1_5 /
/// §21.4.3 RSA-PSS / §22.4.3 RSA-OAEP — `variant` selects the family, all three
/// share this `RsaHashedKeyGenParams` keygen) — returns the `(publicKey,
/// privateKey)` pair (the §17 `CryptoKeyPair`).  `fill_random` is the VM entropy seam,
/// fed through [`ClosureRng`] into `RsaPrivateKey::new_with_exp`'s vetted prime
/// generation.  `public_exponent` is the `RsaKeyGenParams.publicExponent`
/// big-endian `BigInteger`; WebCrypto does not constrain its value, so an even
/// / `< 3` exponent (or an unusable modulus length) surfaces as the §20.8.3
/// step-3 OperationError from the rsa crate (honored as-is).
pub(crate) fn generate<F>(
    variant: RsaVariant,
    modulus_length: u32,
    public_exponent: &[u8],
    hash: HashAlgorithm,
    extractable: bool,
    usages: &[KeyUsage],
    mut fill_random: F,
) -> Result<(CryptoKeyData, CryptoKeyData), AlgorithmError>
where
    F: FnMut(&mut [u8]) -> Result<(), AlgorithmError>,
{
    // §20.8.3 step 1 / §21.4.3 step 1: a usage outside {sign, verify} is a
    // SyntaxError — before key generation.
    validate_generate_usages(variant, usages)?;
    // Bound the script-controlled `modulusLength` BEFORE the rsa crate
    // allocates + prime-searches at that bit size.  WebCrypto §20.8.3 sets no
    // maximum, but `modulusLength` is `[EnforceRange] unsigned long`, so an
    // untrusted `generateKey({modulusLength: 2**32 - 1})` would otherwise
    // OOM / hang the engine (the keygen runs on the VM thread).  No real RSA
    // key approaches `MAX_RSA_MODULUS_BITS`, so this rejects only abuse.
    if modulus_length > MAX_RSA_MODULUS_BITS {
        return Err(operation("RSA modulusLength exceeds the supported maximum"));
    }
    // RSA-OAEP runs on aws-lc-rs, whose OAEP keys are 2048..=8192 bits
    // ([`MIN_RSA_OAEP_MODULUS_BITS`]); a smaller modulus would generate here yet
    // be unusable for encrypt/decrypt — reject before keygen so an accepted
    // RSA-OAEP key is always usable.  (Checked on the requested length, which
    // `new_with_exp` reproduces exactly; the signing variants keep no floor.)
    if rsa_oaep_modulus_below_minimum(variant, modulus_length) {
        return Err(operation(
            "RSA-OAEP modulusLength is below the supported minimum (2048 bits)",
        ));
    }
    // §20.8.3 step 2-3: generate the RSA key pair (failure → OperationError).
    let exp = BigUint::from_bytes_be(public_exponent);
    let privkey = {
        let mut rng = ClosureRng::new(&mut fill_random);
        let result = RsaPrivateKey::new_with_exp(&mut rng, modulus_length as usize, &exp);
        // Surface any `fill_random` error before the (otherwise opaque)
        // generation error.
        rng.into_result()?;
        result.map_err(|_| AlgorithmError::Operation("RSA key generation failed".to_string()))?
    };
    // The canonical DER + the actual modulus length / exponent for the key's
    // `[[algorithm]]` (the rsa crate guarantees a 2-prime key, so this never
    // hits the multi-prime branch).
    let imported = private_imported(&privkey)?;
    let KeyMaterial::Rsa {
        public_spki_der,
        private_pkcs8_der,
    } = imported.material
    else {
        unreachable!("private_imported always returns KeyMaterial::Rsa");
    };
    // §20.8.3 step 7: the key's `publicExponent` is the `publicExponent`
    // member of the *normalized algorithm* (the caller's input bytes), NOT the
    // canonical form re-derived from the parsed key — so a non-minimal input
    // (e.g. a leading-zero `[0, 1, 0, 1]`) round-trips byte-identical.  The
    // modulus length is the generated key's actual bit length.
    let key_alg = variant.key_algorithm(imported.modulus_length, public_exponent.to_vec(), hash);
    // §20.8.3 steps 9-13: the public key — [[extractable]] always true (step 12),
    // usages = ∩(usages, {verify}) (step 13).
    let public = CryptoKeyData {
        key_type: KeyType::Public,
        extractable: true,
        algorithm: key_alg.clone(),
        usages: split_usages(variant, KeyType::Public, usages),
        material: KeyMaterial::Rsa {
            public_spki_der: public_spki_der.clone(),
            private_pkcs8_der: None,
        },
    };
    // §20.8.3 steps 14-18: the private key — [[extractable]] = the requested
    // value (step 17), usages = ∩(usages, {sign}) (step 18).
    let private_usages = split_usages(variant, KeyType::Private, usages);
    // §14.3.6 generateKey generic step: a CryptoKeyPair whose privateKey has
    // empty usages is a SyntaxError.
    if private_usages.is_empty() {
        return Err(AlgorithmError::Syntax("usages cannot be empty".to_string()));
    }
    let private = CryptoKeyData {
        key_type: KeyType::Private,
        extractable,
        algorithm: key_alg,
        usages: private_usages,
        material: KeyMaterial::Rsa {
            public_spki_der,
            private_pkcs8_der,
        },
    };
    Ok((public, private))
}

/// Whether `usage` is permitted for an RSA key of `variant` + `key_type`: the
/// §20.8.3/.4 + §21.4.3/.4 sign-family split (public → `verify`, private →
/// `sign`) for RSASSA-PKCS1-v1_5 / RSA-PSS, or the §22.4.3/.4 OAEP split
/// (public → `encrypt` / `wrapKey`, private → `decrypt` / `unwrapKey`) for
/// RSA-OAEP.  One predicate so the generate / import / pair-split usage checks
/// can't drift by variant.
fn rsa_usage_permitted(variant: RsaVariant, usage: KeyUsage, key_type: KeyType) -> bool {
    match variant {
        RsaVariant::RsassaPkcs1V15 | RsaVariant::RsaPss => usage.is_rsa_sign_usage(key_type),
        RsaVariant::RsaOaep => usage.is_rsa_oaep_usage(key_type),
    }
}

/// The allowed-usages clause for an RSA `variant`'s generate-time SyntaxError
/// message (the union of the public + private halves' usages).
fn rsa_allowed_usages(variant: RsaVariant) -> &'static str {
    match variant {
        RsaVariant::RsassaPkcs1V15 | RsaVariant::RsaPss => "the 'sign' and 'verify' usages",
        RsaVariant::RsaOaep => "the 'encrypt', 'decrypt', 'wrapKey' and 'unwrapKey' usages",
    }
}

/// The §20.8.3 / §21.4.3 / §22.4.3 step-1 usage check: every requested usage
/// must be valid for one half of the produced pair (the variant's usage split).
fn validate_generate_usages(
    variant: RsaVariant,
    usages: &[KeyUsage],
) -> Result<(), AlgorithmError> {
    let permitted = |u: KeyUsage| {
        rsa_usage_permitted(variant, u, KeyType::Public)
            || rsa_usage_permitted(variant, u, KeyType::Private)
    };
    if usages.iter().copied().all(permitted) {
        Ok(())
    } else {
        Err(AlgorithmError::Syntax(format!(
            "{} keys support only {}",
            variant.canonical_name(),
            rsa_allowed_usages(variant)
        )))
    }
}

/// The usage intersection for the `key_type` half of a generated key pair
/// (§20.8.3 steps 13 / 18, §22.4.3): keep the requested usages permitted for
/// that key type + `variant` (sign family public → {verify} / private →
/// {sign}; OAEP public → {encrypt, wrapKey} / private → {decrypt, unwrapKey}),
/// deduplicated + canonically ordered.
fn split_usages(variant: RsaVariant, key_type: KeyType, usages: &[KeyUsage]) -> Vec<KeyUsage> {
    normalize_usages(
        usages
            .iter()
            .copied()
            .filter(|&u| rsa_usage_permitted(variant, u, key_type))
            .collect(),
    )
}

// ---------------------------------------------------------------------------
// exportKey (WebCrypto §20.8.5 / §21.4.5)
// ---------------------------------------------------------------------------

/// `exportKey` for an RSA key (WebCrypto §20.8.5 / §21.4.5).  The §14.3.10
/// step-6 export-support + step-7 extractable gates already ran in
/// [`crate::ops::export_key`]; this performs the per-format `[[type]]` check
/// (InvalidAccessError) + encoding.  `raw` is not a registered RSA export
/// format.
pub(crate) fn export(
    variant: RsaVariant,
    hash: HashAlgorithm,
    format: KeyFormat,
    key: &CryptoKeyData,
) -> Result<ExportedKey, AlgorithmError> {
    match format {
        KeyFormat::Spki => {
            // §20.8.5 spki: [[type]] must be public.
            require_public(key)?;
            Ok(ExportedKey::Raw(rsa_public_der(key).to_vec()))
        }
        KeyFormat::Pkcs8 => {
            // §20.8.5 pkcs8: [[type]] must be private.
            Ok(ExportedKey::Raw(rsa_private_der(key)?.to_vec()))
        }
        KeyFormat::Jwk => Ok(ExportedKey::Jwk(Box::new(export_jwk(variant, hash, key)?))),
        KeyFormat::Raw => Err(AlgorithmError::NotSupported(
            "RSA export supports only the 'spki', 'pkcs8' and 'jwk' formats".to_string(),
        )),
    }
}

/// Build the RSA `jwk` (WebCrypto §20.8.5 / §21.4.5 jwk): `kty`="RSA", `n` /
/// `e` (public), plus `d` / `p` / `q` / `dp` / `dq` / `qi` (private), `alg`
/// (RS256 / PS384 / …), `key_ops`, `ext`.
fn export_jwk(
    variant: RsaVariant,
    hash: HashAlgorithm,
    key: &CryptoKeyData,
) -> Result<JsonWebKey, AlgorithmError> {
    let alg = Some(variant.jwk_alg(hash).to_string());
    let key_ops = Some(key.usages.iter().map(|u| u.as_str().to_string()).collect());
    let ext = Some(key.extractable);
    match key.key_type {
        KeyType::Public => {
            let pubkey = reconstruct_public(key)?;
            Ok(JsonWebKey {
                kty: Some("RSA".to_string()),
                n: Some(b64(pubkey.n())),
                e: Some(b64(pubkey.e())),
                alg,
                key_ops,
                ext,
                ..JsonWebKey::default()
            })
        }
        KeyType::Private => {
            let privkey = reconstruct_private(key)?;
            let primes = privkey.primes();
            let p = primes.first().ok_or_else(key_inaccessible)?;
            let q = primes.get(1).ok_or_else(key_inaccessible)?;
            let dp = privkey.dp().ok_or_else(key_inaccessible)?;
            let dq = privkey.dq().ok_or_else(key_inaccessible)?;
            let qi = privkey.crt_coefficient().ok_or_else(key_inaccessible)?;
            Ok(JsonWebKey {
                kty: Some("RSA".to_string()),
                n: Some(b64(privkey.n())),
                e: Some(b64(privkey.e())),
                d: Some(b64(privkey.d())),
                p: Some(b64(p)),
                q: Some(b64(q)),
                dp: Some(b64(dp)),
                dq: Some(b64(dq)),
                qi: Some(b64(&qi)),
                alg,
                key_ops,
                ext,
                ..JsonWebKey::default()
            })
        }
        KeyType::Secret => unreachable!("RSA keys are never secret"),
    }
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Reconstruct the typed `RsaPublicKey` from a key's stored canonical SPKI DER
/// (export / verify).  The DER was produced by this crate, so a parse failure
/// is the §20.8.5 step-2 "key material cannot be accessed" OperationError, not
/// a DataError.
fn reconstruct_public(key: &CryptoKeyData) -> Result<RsaPublicKey, AlgorithmError> {
    RsaPublicKey::from_public_key_der(rsa_public_der(key)).map_err(|_| key_inaccessible())
}

/// Reconstruct the typed `RsaPrivateKey` from a key's stored canonical PKCS#8
/// DER (export / sign).  Requires `[[type]]` = private (InvalidAccessError).
fn reconstruct_private(key: &CryptoKeyData) -> Result<RsaPrivateKey, AlgorithmError> {
    RsaPrivateKey::from_pkcs8_der(rsa_private_der(key)?).map_err(|_| key_inaccessible())
}

/// Validate the import usages for `(variant, key_type)` (WebCrypto §20.8.3 /
/// §20.8.4 "usages contains an entry which is not 'sign' / 'verify' →
/// SyntaxError"): a public key accepts only `verify`, a private key only
/// `sign`.
fn validate_import_usages(
    variant: RsaVariant,
    key_type: KeyType,
    usages: &[KeyUsage],
) -> Result<(), AlgorithmError> {
    if usages
        .iter()
        .all(|&u| rsa_usage_permitted(variant, u, key_type))
    {
        Ok(())
    } else {
        Err(AlgorithmError::Syntax(usage_message(variant, key_type)))
    }
}

fn usage_message(variant: RsaVariant, key_type: KeyType) -> String {
    let kind = match (variant, key_type) {
        (RsaVariant::RsaOaep, KeyType::Public) => {
            "public keys support only the 'encrypt' and 'wrapKey' usages"
        }
        (RsaVariant::RsaOaep, _) => {
            "private keys support only the 'decrypt' and 'unwrapKey' usages"
        }
        (_, KeyType::Public) => "public keys support only the 'verify' usage",
        (_, _) => "private keys support only the 'sign' usage",
    };
    format!("{} {}", variant.canonical_name(), kind)
}

/// The §20.8.5 spki "If [[type]] is not 'public' → InvalidAccessError" gate.
fn require_public(key: &CryptoKeyData) -> Result<(), AlgorithmError> {
    if key.key_type == KeyType::Public {
        Ok(())
    } else {
        Err(AlgorithmError::InvalidAccess(
            "the key is not a public key".to_string(),
        ))
    }
}

/// The stored canonical SPKI DER (always present for an RSA key).
fn rsa_public_der(key: &CryptoKeyData) -> &[u8] {
    key.material
        .rsa_public_der()
        .expect("an RSA key always stores its public SPKI DER")
}

/// The stored canonical PKCS#8 DER of an RSA **private** key — the §20.8.5
/// pkcs8 "If [[type]] is not 'private' → InvalidAccessError" gate.
fn rsa_private_der(key: &CryptoKeyData) -> Result<&[u8], AlgorithmError> {
    key.material
        .rsa_private_der()
        .ok_or_else(|| AlgorithmError::InvalidAccess("the key is not a private key".to_string()))
}

/// The canonical SubjectPublicKeyInfo DER of a public key.
fn encode_spki(pubkey: &RsaPublicKey) -> Result<Vec<u8>, AlgorithmError> {
    Ok(pubkey
        .to_public_key_der()
        .map_err(|_| data("RSA public key cannot be encoded"))?
        .as_ref()
        .to_vec())
}

/// The modulus bit length (`RsaHashedKeyAlgorithm.modulusLength`, §20.6).
fn modulus_bits<K: PublicKeyParts>(key: &K) -> Result<u32, AlgorithmError> {
    u32::try_from(key.n().bits()).map_err(|_| data("RSA modulus length is too large"))
}

/// Reject an imported RSA modulus wider than [`MAX_RSA_MODULUS_BITS`] — the same
/// bound [`generate`] applies to a script-controlled `modulusLength`.  WebCrypto
/// §20.8.4 sets no import maximum, but every import path validates / operates on
/// the modulus on the VM thread, and the d-only JWK path runs NIST SP 800-56B
/// C.2 prime *recovery* on attacker-controlled `n` / `e` / `d`, so an unbounded
/// modulus is an engine-hang / OOM DoS via untrusted script.  Checked *before*
/// the rsa crate does that work (the JWK path checks the decoded `n`; the
/// spki / pkcs8 paths re-check the parsed key, keeping import↔generate
/// symmetric).  A capability boundary, not malformed material → NotSupported.
fn check_modulus_bits(bits: usize) -> Result<(), AlgorithmError> {
    if bits > MAX_RSA_MODULUS_BITS as usize {
        return Err(AlgorithmError::NotSupported(
            "RSA modulus length exceeds the supported maximum".to_string(),
        ));
    }
    Ok(())
}

/// Reject a public exponent above the rsa crate's `RsaPublicKey::MAX_PUB_EXPONENT`
/// (2^33 − 1) as a NotSupported capability boundary, rather than letting
/// `RsaPublicKey::new` surface it as a generic `DataError`.  WebCrypto / JWA
/// accept any valid `Base64urlUInt` `e`, but the rsa crate caps it (every real
/// key uses e=65537 ≪ 2^33), so an over-cap `e` is an unsupported capability,
/// not malformed material.  Checked on the JWK path (where `e` is decoded
/// before construction); the spki / pkcs8 DER paths hit the rsa crate's cap
/// first (DataError) — see [`parse_spki`].  (Codex R16.)
fn check_public_exponent(e: &BigUint) -> Result<(), AlgorithmError> {
    if *e > BigUint::from(RsaPublicKey::MAX_PUB_EXPONENT) {
        return Err(AlgorithmError::NotSupported(
            "RSA public exponent exceeds the supported maximum".to_string(),
        ));
    }
    Ok(())
}

/// Decode a required RFC 7518 §2 `Base64urlUInt` JWK member into a `BigUint`.
/// The encoding is the **minimal-length** big-endian octets (zero is `"AA"` =
/// `[0x00]`), so an empty value or a non-minimal leading-zero octet is a
/// malformed member → DataError (do not silently canonicalize it).
fn decode_biguint(value: Option<&str>, member: &str) -> Result<BigUint, AlgorithmError> {
    let value = value.ok_or_else(|| data_owned(format!("JWK '{member}' member is missing")))?;
    let bytes = decode_b64(value)?;
    if bytes.is_empty() || (bytes.len() > 1 && bytes[0] == 0) {
        return Err(data_owned(format!(
            "JWK '{member}' member is not a minimal Base64urlUInt"
        )));
    }
    Ok(BigUint::from_bytes_be(&bytes))
}

/// base64url (no padding) of a `BigUint`'s minimal big-endian octets (the
/// RFC 7518 §2 `Base64urlUInt` encoding).
fn b64(value: &BigUint) -> String {
    URL_SAFE_NO_PAD.encode(value.to_bytes_be())
}

fn decode_b64(s: &str) -> Result<Vec<u8>, AlgorithmError> {
    URL_SAFE_NO_PAD
        .decode(s)
        .map_err(|_| data("JWK RSA member is not valid base64url"))
}

/// A multi-prime RSA key (>2 primes — `pkcs8` otherPrimeInfos / JWK `oth`) is
/// not supported: the rsa crate's DER encoder rejects it, and DER is the
/// canonical storage form (`#11-rsa-multiprime-jwk`).
fn multiprime_unsupported() -> AlgorithmError {
    AlgorithmError::NotSupported(
        "multi-prime RSA keys (more than two primes) are not supported".to_string(),
    )
}

fn data(msg: &str) -> AlgorithmError {
    AlgorithmError::Data(msg.to_string())
}

fn data_owned(msg: String) -> AlgorithmError {
    AlgorithmError::Data(msg)
}

fn operation(msg: &str) -> AlgorithmError {
    AlgorithmError::Operation(msg.to_string())
}

/// The §20.8.5 step-2 "key material cannot be accessed → OperationError" —
/// used for the (unreachable, key already validated) re-encode / reconstruct
/// failures.
fn key_inaccessible() -> AlgorithmError {
    AlgorithmError::Operation("the RSA key material cannot be accessed".to_string())
}
