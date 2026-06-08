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
/// SPKI → decode error → DataError).
fn parse_spki(der: &[u8]) -> Result<RsaPublicKey, AlgorithmError> {
    RsaPublicKey::from_public_key_der(der)
        .map_err(|_| data("invalid SubjectPublicKeyInfo RSA public key"))
}

/// Parse a PKCS#8 PrivateKeyInfo (WebCrypto §20.8.4 pkcs8): `from_pkcs8_der`
/// validates the rsaEncryption OID + the RSA structure.
fn parse_pkcs8(der: &[u8]) -> Result<RsaPrivateKey, AlgorithmError> {
    RsaPrivateKey::from_pkcs8_der(der).map_err(|_| data("invalid PKCS#8 RSA private key"))
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

/// Import an RSA `jwk` (WebCrypto §20.8.4 / §21.4.4 jwk branch + RFC 7518
/// §6.3): validate the JWK shape (kty / use / key_ops / ext / alg), determine
/// the key type from the `d` member, then reconstruct the typed key from
/// n / e [/ d / p / q].  Multi-prime (`oth`) is NotSupported.
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
    // kty must be "RSA".
    if jwk.kty.as_deref() != Some("RSA") {
        return Err(data(
            "JWK 'kty' member must be 'RSA' for RSASSA-PKCS1-v1_5 / RSA-PSS",
        ));
    }
    // use, if present (and usages non-empty): "sig" (a signing key).
    if !usages.is_empty() {
        if let Some(use_) = jwk.use_.as_deref() {
            if use_ != "sig" {
                return Err(data(
                    "JWK 'use' member must be 'sig' for an RSA signing key",
                ));
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
    // Multi-prime keys (`oth` present + non-empty) are not supported — the rsa
    // crate's DER encoder rejects >2 primes (`#11-rsa-multiprime-jwk`).
    if jwk.oth.as_ref().is_some_and(|oth| !oth.is_empty()) {
        return Err(multiprime_unsupported());
    }
    // n / e are required for both public and private keys (RFC 7518 §6.3.1).
    let n = decode_biguint(jwk.n.as_deref(), "n")?;
    let e = decode_biguint(jwk.e.as_deref(), "e")?;
    match key_type {
        KeyType::Public => {
            let pubkey = RsaPublicKey::new(n, e)
                .map_err(|_| data("JWK RSA public key (n, e) is invalid"))?;
            public_imported(&pubkey)
        }
        KeyType::Private => {
            // d is required (the §-determined private branch); p / q / dp / dq /
            // qi are all-or-nothing (RFC 7518 §6.3.2 — see [`jwk_primes`]).
            let d = decode_biguint(jwk.d.as_deref(), "d")?;
            let primes = jwk_primes(jwk)?;
            let privkey = RsaPrivateKey::from_components(n, e, d, primes)
                .map_err(|_| data("JWK RSA private key is invalid or inconsistent"))?;
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
        KeyFormat::Jwk => Ok(ExportedKey::Jwk(export_jwk(variant, hash, key)?)),
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
    if usages.iter().all(|&u| u.is_rsa_sign_usage(key_type)) {
        Ok(())
    } else {
        Err(AlgorithmError::Syntax(usage_message(variant, key_type)))
    }
}

fn usage_message(variant: RsaVariant, key_type: KeyType) -> String {
    let kind = match key_type {
        KeyType::Public => "public keys support only the 'verify' usage",
        KeyType::Private | KeyType::Secret => "private keys support only the 'sign' usage",
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

/// Decode a required base64url `BigInteger` JWK member into a `BigUint`.
fn decode_biguint(value: Option<&str>, member: &str) -> Result<BigUint, AlgorithmError> {
    let value = value.ok_or_else(|| data_owned(format!("JWK '{member}' member is missing")))?;
    Ok(BigUint::from_bytes_be(&decode_b64(value)?))
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

/// The §20.8.5 step-2 "key material cannot be accessed → OperationError" —
/// used for the (unreachable, key already validated) re-encode / reconstruct
/// failures.
fn key_inaccessible() -> AlgorithmError {
    AlgorithmError::Operation("the RSA key material cannot be accessed".to_string())
}
