//! Elliptic-curve key import / export + ECDSA / ECDH operations (WebCrypto
//! §23 ECDSA / §24 ECDH), reached only through [`crate::ops`] (which runs
//! the §14.3.x name / usage / extractable gates), so the curve-typed APIs
//! are `pub(crate)` — not a public surface.
//!
//! Each operation dispatches on [`NamedCurve`] to the matching RustCrypto
//! curve crate (`p256` / `p384` / `p521`) via the [`with_curve!`] macro:
//! the curve crates re-export `PublicKey` / `SecretKey` / `EncodedPoint`
//! specialized to their curve, so one macro body type-checks for all three.
//! The engine-independent [`crate::key::KeyMaterial::Ec`] stores the SEC1
//! uncompressed public point (`0x04‖x‖y`) plus the optional big-endian
//! private scalar; the typed curve key is reconstructed here at op time
//! (the asymmetric analogue of `Raw(bytes)` → cipher).

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use ecdsa::signature::hazmat::{PrehashSigner, PrehashVerifier};
use elliptic_curve::pkcs8::spki::{DecodePublicKey, EncodePublicKey};
use elliptic_curve::pkcs8::{DecodePrivateKey, EncodePrivateKey};
use elliptic_curve::sec1::ToEncodedPoint;

use crate::algorithm::{EcAlgorithm, EcdhPeer, NamedCurve};
use crate::error::AlgorithmError;
use crate::hash::HashAlgorithm;
use crate::jwk::{self, JsonWebKey};
use crate::key::{normalize_usages, CryptoKeyData, KeyMaterial, KeyType, KeyUsage};
use crate::ops::{format_data_mismatch, ExportedKey, KeyData, KeyFormat};
use crate::rng::ClosureRng;

/// Dispatch to the RustCrypto curve crate matching `$curve`, binding it as
/// `$cc` inside `$body`.  Each curve crate (`p256` / `p384` / `p521`)
/// re-exports `PublicKey` / `SecretKey` / `EncodedPoint` / `FieldBytes`
/// specialized to its curve, so one `$body` type-checks for all three and
/// the `?` / `return` inside it act on the enclosing function.
macro_rules! with_curve {
    ($curve:expr, $cc:ident, $body:block) => {
        match $curve {
            NamedCurve::P256 => {
                use ::p256 as $cc;
                $body
            }
            NamedCurve::P384 => {
                use ::p384 as $cc;
                $body
            }
            NamedCurve::P521 => {
                use ::p521 as $cc;
                $body
            }
        }
    };
}

// ---------------------------------------------------------------------------
// importKey (WebCrypto §23.7.4 / §24.4.3)
// ---------------------------------------------------------------------------

/// `importKey` for an EC algorithm (WebCrypto §23.7.4 ECDSA / §24.4.3 ECDH).
/// `curve` is the `namedCurve` member of the normalized algorithm; the
/// imported key's curve must match it.
pub(crate) fn import(
    algorithm: EcAlgorithm,
    curve: NamedCurve,
    format: KeyFormat,
    extractable: bool,
    usages: Vec<KeyUsage>,
    key_data: KeyData,
) -> Result<CryptoKeyData, AlgorithmError> {
    // Each branch runs the §-step order: the usage SyntaxError check (which
    // depends on the key type the format implies) precedes the key-material
    // parse (the DataError set).  `jwk` determines its key type from the `d`
    // member, so it validates usages internally.
    let (key_type, material) = match (format, key_data) {
        (KeyFormat::Raw, KeyData::Raw(bytes)) => {
            // §23.7.4 / §24.4.3 raw: a public-only format.
            validate_import_usages(algorithm, KeyType::Public, &usages)?;
            (
                KeyType::Public,
                public_material(import_sec1_point(curve, &bytes)?),
            )
        }
        (KeyFormat::Spki, KeyData::Raw(bytes)) => {
            validate_import_usages(algorithm, KeyType::Public, &usages)?;
            (
                KeyType::Public,
                public_material(import_spki(curve, &bytes)?),
            )
        }
        (KeyFormat::Pkcs8, KeyData::Raw(bytes)) => {
            validate_import_usages(algorithm, KeyType::Private, &usages)?;
            (KeyType::Private, import_pkcs8(curve, &bytes)?)
        }
        (KeyFormat::Jwk, KeyData::Jwk(jwk)) => {
            import_jwk(algorithm, curve, extractable, &usages, &jwk)?
        }
        // Format / data shape mismatch — the VM marshals them consistently
        // (raw/spki/pkcs8 → Raw, jwk → Jwk), so this is a defensive guard.
        _ => return Err(format_data_mismatch()),
    };
    // §14.3.9 importKey generic step: a private (or secret) key with empty
    // usages is a SyntaxError — but an EC *public* key may have empty usages
    // (an ECDH public key MUST).  Checked after the algorithm-specific parse,
    // so a DataError from invalid material wins.
    if key_type == KeyType::Private && usages.is_empty() {
        return Err(AlgorithmError::Syntax("usages cannot be empty".to_string()));
    }
    let usages = normalize_usages(usages);
    Ok(CryptoKeyData {
        key_type,
        extractable,
        algorithm: algorithm.key_algorithm(curve),
        usages,
        material,
    })
}

/// `KeyMaterial::Ec` for a public key (no private scalar).
fn public_material(public_point: Vec<u8>) -> KeyMaterial {
    KeyMaterial::Ec {
        public_point,
        private_scalar: None,
    }
}

/// Decode a SEC1 §2.3.4 point (the `raw` format / a SPKI `subjectPublicKey`)
/// and re-encode it canonically uncompressed.  `from_sec1_bytes` validates
/// the point is on the curve and rejects the identity point; a compressed
/// point is accepted (RustCrypto supports decompression — WebCrypto requires
/// only that uncompressed be supported), then normalized to uncompressed.  A
/// decode error / off-curve / identity point is a DataError (§23.7.4 /
/// §24.4.3 raw substeps "decode error or identity point").
fn import_sec1_point(curve: NamedCurve, bytes: &[u8]) -> Result<Vec<u8>, AlgorithmError> {
    with_curve!(curve, cc, {
        let pk = cc::PublicKey::from_sec1_bytes(bytes)
            .map_err(|_| data("invalid elliptic-curve public key point"))?;
        Ok(pk.to_encoded_point(false).as_bytes().to_vec())
    })
}

/// Parse a SubjectPublicKeyInfo (WebCrypto §23.7.4 / §24.4.3 spki): the
/// curve-typed `PublicKey::from_public_key_der` validates the id-ecPublicKey
/// OID + the embedded `namedCurve` OID equals this curve (a mismatching curve
/// → decode error → DataError, subsuming the §-step "namedCurve ≠
/// normalizedAlgorithm.namedCurve → DataError").
fn import_spki(curve: NamedCurve, der: &[u8]) -> Result<Vec<u8>, AlgorithmError> {
    with_curve!(curve, cc, {
        let pk = cc::PublicKey::from_public_key_der(der)
            .map_err(|_| data("invalid SubjectPublicKeyInfo elliptic-curve public key"))?;
        Ok(pk.to_encoded_point(false).as_bytes().to_vec())
    })
}

/// Parse a PKCS#8 PrivateKeyInfo (WebCrypto §23.7.4 / §24.4.3 pkcs8): the
/// curve-typed `SecretKey::from_pkcs8_der` validates the OID + curve, and the
/// public point is derived from the scalar.  Returns the `KeyMaterial::Ec`.
fn import_pkcs8(curve: NamedCurve, der: &[u8]) -> Result<KeyMaterial, AlgorithmError> {
    with_curve!(curve, cc, {
        let sk = cc::SecretKey::from_pkcs8_der(der)
            .map_err(|_| data("invalid PKCS#8 elliptic-curve private key"))?;
        Ok(KeyMaterial::Ec {
            public_point: sk.public_key().to_encoded_point(false).as_bytes().to_vec(),
            private_scalar: Some(sk.to_bytes().to_vec()),
        })
    })
}

/// Import an EC `jwk` (WebCrypto §23.7.4 / §24.4.3 jwk branch + JWA §6.2):
/// validate the JWK shape (kty / use / key_ops / ext / crv / [ECDSA] alg),
/// determine the key type from the `d` member, then reconstruct the point
/// (and scalar) via the curve crate.  Returns the key type + material; the
/// caller applies the generic empty-usages SyntaxError.
fn import_jwk(
    algorithm: EcAlgorithm,
    curve: NamedCurve,
    extractable: bool,
    usages: &[KeyUsage],
    jwk: &JsonWebKey,
) -> Result<(KeyType, KeyMaterial), AlgorithmError> {
    // step 2/3: the `d` member determines the key type; the usage SyntaxError
    // check (which usages are valid for that type) runs before the DataError
    // shape checks.
    let key_type = if jwk.d.is_some() {
        KeyType::Private
    } else {
        KeyType::Public
    };
    validate_import_usages(algorithm, key_type, usages)?;
    // kty must be "EC".
    if jwk.kty.as_deref() != Some("EC") {
        return Err(data("JWK 'kty' member must be 'EC' for ECDSA / ECDH"));
    }
    // use, if present (and usages non-empty): ECDSA → "sig", ECDH → "enc".
    if !usages.is_empty() {
        if let Some(use_) = jwk.use_.as_deref() {
            let expected = match algorithm {
                EcAlgorithm::Ecdsa => "sig",
                EcAlgorithm::Ecdh => "enc",
            };
            if use_ != expected {
                return Err(data("JWK 'use' member does not match the algorithm"));
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
    // crv must equal the normalized algorithm's namedCurve.
    let crv = jwk
        .crv
        .as_deref()
        .ok_or_else(|| data("JWK 'crv' member is missing"))?;
    if NamedCurve::from_name(crv) != Some(curve) {
        return Err(data("JWK 'crv' member does not match the requested curve"));
    }
    // ECDSA: a present `alg` (ES256 / ES384 / ES512) must match the curve
    // (§23.7.4 jwk step 9.2).  ECDH has no such `alg` requirement (§24.4.3).
    if matches!(algorithm, EcAlgorithm::Ecdsa) {
        if let Some(alg) = jwk.alg.as_deref() {
            let alg_curve = match alg {
                "ES256" => NamedCurve::P256,
                "ES384" => NamedCurve::P384,
                "ES512" => NamedCurve::P521,
                _ => return Err(data("JWK 'alg' member is not a valid ECDSA algorithm")),
            };
            if alg_curve != curve {
                return Err(data("JWK 'alg' member does not match the curve"));
            }
        }
    }
    // Reconstruct the point from x / y (and the scalar from d), validating it
    // lies on the curve (§6.2.1 / §6.2.2 "meets the requirements" → DataError).
    let x = decode_b64(
        jwk.x
            .as_deref()
            .ok_or_else(|| data("JWK 'x' member is missing"))?,
    )?;
    let y = decode_b64(
        jwk.y
            .as_deref()
            .ok_or_else(|| data("JWK 'y' member is missing"))?,
    )?;
    let clen = curve.coordinate_len();
    if x.len() != clen || y.len() != clen {
        return Err(data(
            "JWK 'x' / 'y' member has the wrong length for the curve",
        ));
    }
    // Build the uncompressed SEC1 encoding `0x04‖x‖y` and validate via the
    // curve crate (on-curve + non-identity).
    let mut sec1 = Vec::with_capacity(1 + 2 * clen);
    sec1.push(0x04);
    sec1.extend_from_slice(&x);
    sec1.extend_from_slice(&y);
    let public_point = import_sec1_point(curve, &sec1)?;

    let private_scalar = match &jwk.d {
        None => None,
        Some(d_b64) => {
            let d = decode_b64(d_b64)?;
            with_curve!(curve, cc, {
                // §6.2.2: `d` must be the private scalar; `from_slice`
                // validates its length + range [1, n-1].
                let sk = cc::SecretKey::from_slice(&d)
                    .map_err(|_| data("JWK 'd' member is not a valid private scalar"))?;
                // The public key derived from `d` must match x / y (§6.2.2
                // "the public key must be consistent with the private key").
                if sk.public_key().to_encoded_point(false).as_bytes() != public_point.as_slice() {
                    return Err(data("JWK 'd' member is inconsistent with 'x' / 'y'"));
                }
                Some(sk.to_bytes().to_vec())
            })
        }
    };
    Ok((
        key_type,
        KeyMaterial::Ec {
            public_point,
            private_scalar,
        },
    ))
}

// ---------------------------------------------------------------------------
// generateKey (WebCrypto §23.7.3 / §24.4.1)
// ---------------------------------------------------------------------------

/// `generateKey` for an EC algorithm (WebCrypto §23.7.3 ECDSA / §24.4.1 ECDH)
/// — returns the `(publicKey, privateKey)` pair (the §14.3.6 `CryptoKeyPair`).
/// `fill_random` is the VM entropy seam, fed through [`ClosureRng`] into
/// `SecretKey::random`'s vetted rejection sampling.
pub(crate) fn generate<F>(
    algorithm: EcAlgorithm,
    curve: NamedCurve,
    extractable: bool,
    usages: &[KeyUsage],
    fill_random: F,
) -> Result<(CryptoKeyData, CryptoKeyData), AlgorithmError>
where
    F: FnMut(&mut [u8]) -> Result<(), AlgorithmError>,
{
    // §23.7.3 step 1 / §24.4.1 step 1: a usage outside the algorithm's set is a
    // SyntaxError — before key generation.
    validate_generate_usages(algorithm, usages)?;
    // §23.7.3 step 2-3 / §24.4.1 step 2-3: generate the curve key pair (a
    // generation failure is an OperationError, surfaced by the ClosureRng).
    let (public_point, private_scalar) = generate_keypair(curve, fill_random)?;
    let key_alg = algorithm.key_algorithm(curve);
    // steps 7-11: the public key — usages = ∩(usages, public-permitted),
    // [[extractable]] always true.
    let public = CryptoKeyData {
        key_type: KeyType::Public,
        extractable: true,
        algorithm: key_alg.clone(),
        usages: split_usages(algorithm, KeyType::Public, usages),
        material: KeyMaterial::Ec {
            public_point: public_point.clone(),
            private_scalar: None,
        },
    };
    // steps 12-16: the private key — usages = ∩(usages, private-permitted),
    // [[extractable]] = the requested value.
    let private_usages = split_usages(algorithm, KeyType::Private, usages);
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
        material: KeyMaterial::Ec {
            public_point,
            private_scalar: Some(private_scalar),
        },
    };
    Ok((public, private))
}

/// Generate a curve key pair, returning the SEC1 uncompressed public point +
/// the big-endian private scalar.  The scalar is produced by
/// `SecretKey::random`'s rejection sampling over the [`ClosureRng`] (the VM
/// entropy seam); a `fill_random` failure surfaces as an OperationError.
fn generate_keypair<F>(
    curve: NamedCurve,
    mut fill_random: F,
) -> Result<(Vec<u8>, Vec<u8>), AlgorithmError>
where
    F: FnMut(&mut [u8]) -> Result<(), AlgorithmError>,
{
    with_curve!(curve, cc, {
        let mut rng = ClosureRng::new(&mut fill_random);
        let sk = cc::SecretKey::random(&mut rng);
        // Surface any `fill_random` error before using the (otherwise garbage)
        // key.
        rng.into_result()?;
        Ok((
            sk.public_key().to_encoded_point(false).as_bytes().to_vec(),
            sk.to_bytes().to_vec(),
        ))
    })
}

/// The §23.7.3 / §24.4.1 step-1 usage check: every requested usage must be
/// valid for *either* the public or the private key of the pair (the union of
/// the split sets).
fn validate_generate_usages(
    algorithm: EcAlgorithm,
    usages: &[KeyUsage],
) -> Result<(), AlgorithmError> {
    let permitted = |u: KeyUsage| {
        ec_usage_permitted(algorithm, KeyType::Public, u)
            || ec_usage_permitted(algorithm, KeyType::Private, u)
    };
    if usages.iter().all(|&u| permitted(u)) {
        Ok(())
    } else {
        Err(AlgorithmError::Syntax(generate_usage_message(algorithm)))
    }
}

fn generate_usage_message(algorithm: EcAlgorithm) -> String {
    match algorithm {
        EcAlgorithm::Ecdsa => "ECDSA keys support only the 'sign' and 'verify' usages",
        EcAlgorithm::Ecdh => "ECDH keys support only the 'deriveKey' and 'deriveBits' usages",
    }
    .to_string()
}

/// The usage intersection for the `key_type` half of a generated key pair
/// (§23.7.3 steps 11 / 16, §24.4.1 steps 11 / 16): keep the requested usages
/// permitted for that key type, deduplicated + canonically ordered.
fn split_usages(algorithm: EcAlgorithm, key_type: KeyType, usages: &[KeyUsage]) -> Vec<KeyUsage> {
    normalize_usages(
        usages
            .iter()
            .copied()
            .filter(|&u| ec_usage_permitted(algorithm, key_type, u))
            .collect(),
    )
}

// ---------------------------------------------------------------------------
// ECDSA sign / verify (WebCrypto §23.7.1 / §23.7.2)
// ---------------------------------------------------------------------------

/// ECDSA `sign` (WebCrypto §23.7.1): hash `message` with `hash`, then sign the
/// digest, returning the raw `r‖s` concatenation (each `coordinate_len` bytes,
/// NOT DER).  The §14.3.3 name / `sign`-usage gate ran in [`crate::ops::sign`];
/// this enforces step 1 ([[type]] must be private) + the signing.  Uses
/// `sign_prehash` (RFC 6979 deterministic — spec-acceptable, no RNG) so our
/// `sha2 0.11` digest stays decoupled from ecdsa's digest generation.
pub(crate) fn sign(
    curve: NamedCurve,
    hash: HashAlgorithm,
    key: &CryptoKeyData,
    message: &[u8],
) -> Result<Vec<u8>, AlgorithmError> {
    // §23.7.1 step 1: the key must be private.
    let scalar = ec_private_scalar(key)?;
    // step 3: M = digest(hash, message); steps 4-6: ECDSA sign → raw r‖s.
    let digest = hash.digest(message);
    with_curve!(curve, cc, {
        let signing_key =
            cc::ecdsa::SigningKey::from_slice(scalar).map_err(|_| key_inaccessible())?;
        let sig: cc::ecdsa::Signature = signing_key
            .sign_prehash(&digest)
            .map_err(|_| AlgorithmError::Operation("ECDSA signing failed".to_string()))?;
        // `to_bytes` is the fixed-size `r‖s` (each coordinate_len bytes,
        // big-endian, zero-padded) — exactly the WebCrypto §23.7.1 format.
        Ok(sig.to_bytes().to_vec())
    })
}

/// ECDSA `verify` (WebCrypto §23.7.2): hash `message`, parse the raw `r‖s`
/// signature, and verify.  The §14.3.4 name / `verify`-usage gate ran in
/// [`crate::ops::verify`]; this enforces step 1 ([[type]] must be public),
/// returns **false** (not an error) when the signature is not `2n` bytes or
/// `(r, s)` is out of range, and otherwise the verification result.
pub(crate) fn verify(
    curve: NamedCurve,
    hash: HashAlgorithm,
    key: &CryptoKeyData,
    signature: &[u8],
    message: &[u8],
) -> Result<bool, AlgorithmError> {
    // §23.7.2 step 1: the key must be public.
    require_public(key)?;
    // step 6.2: a signature that is not exactly 2·coordinate_len bytes → false.
    if signature.len() != curve.signature_len() {
        return Ok(false);
    }
    let point = ec_public_point(key);
    let digest = hash.digest(message);
    with_curve!(curve, cc, {
        let Ok(verifying_key) = cc::ecdsa::VerifyingKey::from_sec1_bytes(point) else {
            // The stored point was validated at import / generate, so this is
            // unreachable; a parse failure means no valid signature → false.
            return Ok(false);
        };
        // A malformed `(r, s)` (zero / ≥ n) is an invalid signature → false.
        let Ok(sig) = cc::ecdsa::Signature::from_slice(signature) else {
            return Ok(false);
        };
        Ok(verifying_key.verify_prehash(&digest, &sig).is_ok())
    })
}

// ---------------------------------------------------------------------------
// ECDH deriveBits (WebCrypto §24.4.2)
// ---------------------------------------------------------------------------

/// ECDH `deriveBits` (WebCrypto §24.4.2): validate the §24.4.2
/// InvalidAccessError precedence against the base key (steps 1, 3, 4, 5),
/// perform the ECDH primitive (RFC 6090 §4) yielding the shared secret (the
/// field-element-to-octet-string X coordinate, `coordinate_len` bytes), then
/// apply the step-8 length semantics.  `peer` is the VM-extracted
/// [`EcdhKeyDeriveParams.public`][EcdhPeer]; the §14.3.8 name + `deriveBits`-
/// usage gate ran in [`crate::ops::derive_bits`] / [`crate::ops::derive_key`].
///
/// `length` semantics differ from the KDFs: `None` returns the **full**
/// secret (not an OperationError); `Some(len)` returns the first `len` bits
/// (sub-byte aligned, masking the final octet), or an OperationError if `len`
/// exceeds the secret's bit length.
pub(crate) fn derive_bits(
    base_key: &CryptoKeyData,
    peer: &EcdhPeer,
    length: Option<u32>,
) -> Result<Vec<u8>, AlgorithmError> {
    // step 1: the base key must be private.
    let scalar = ec_private_scalar(base_key)?;
    let curve = base_key
        .algorithm
        .named_curve()
        .expect("an ECDH base key has a curve");
    // step 3: the peer must be public.
    if peer.key_type != KeyType::Public {
        return Err(invalid_access("the ECDH public key is not a public key"));
    }
    // step 4: the peer's algorithm name must equal the base key's (ECDH).
    if peer.algorithm != base_key.algorithm.name() {
        return Err(invalid_access(
            "the ECDH public key algorithm does not match the base key",
        ));
    }
    // step 5: the peer's curve must equal the base key's.
    if peer.curve != Some(curve) {
        return Err(invalid_access(
            "the ECDH public key curve does not match the base key",
        ));
    }
    let peer_point = peer
        .public_point
        .as_deref()
        .ok_or_else(|| invalid_access("the ECDH public key has no point"))?;
    // step 6: the ECDH primitive → the shared secret (X coordinate octets).
    let secret = ecdh_shared_secret(curve, scalar, peer_point)?;
    // step 8: the length semantics (null → full; else first `length` bits).
    truncate_to_length(secret, length)
}

/// The ECDH primitive (WebCrypto §24.4.2 step 6 / RFC 6090 §4): scalar-
/// multiply the peer point by the base scalar, returning the shared secret's
/// X coordinate as octets (RFC 6090 §6.2, `coordinate_len` bytes big-endian).
fn ecdh_shared_secret(
    curve: NamedCurve,
    scalar: &[u8],
    peer_point: &[u8],
) -> Result<Vec<u8>, AlgorithmError> {
    with_curve!(curve, cc, {
        let sk = cc::SecretKey::from_slice(scalar).map_err(|_| key_inaccessible())?;
        let peer_pk = cc::PublicKey::from_sec1_bytes(peer_point)
            .map_err(|_| operation("the ECDH public key point is invalid"))?;
        let shared = cc::ecdh::diffie_hellman(sk.to_nonzero_scalar(), peer_pk.as_affine());
        Ok(shared.raw_secret_bytes().to_vec())
    })
}

/// WebCrypto §24.4.2 step 8: `None` returns the full secret; `Some(len)`
/// returns the first `len` bits (masking the final partial octet), or an
/// OperationError if `len` exceeds the secret's bit length.
fn truncate_to_length(secret: Vec<u8>, length: Option<u32>) -> Result<Vec<u8>, AlgorithmError> {
    let Some(length) = length else {
        return Ok(secret);
    };
    let length = length as usize;
    if length > secret.len() * 8 {
        return Err(operation(
            "the requested length exceeds the ECDH shared secret",
        ));
    }
    let full_bytes = length / 8;
    let rem_bits = length % 8; // 0..8
    let mut out = secret[..full_bytes].to_vec();
    if rem_bits != 0 {
        // Keep the high `rem_bits` of the next octet, zero the rest.
        out.push(secret[full_bytes] & (0xFFu8 << (8 - rem_bits)));
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// exportKey (WebCrypto §23.7.5 / §24.4.4)
// ---------------------------------------------------------------------------

/// `exportKey` for an EC key (WebCrypto §23.7.5 ECDSA / §24.4.4 ECDH).  The
/// §14.3.10 step-6 export-support + step-7 extractable gates already ran in
/// [`crate::ops::export_key`]; this performs the per-format `[[type]]` check
/// (InvalidAccessError) + encoding.
pub(crate) fn export(
    algorithm: EcAlgorithm,
    curve: NamedCurve,
    format: KeyFormat,
    key: &CryptoKeyData,
) -> Result<ExportedKey, AlgorithmError> {
    match format {
        KeyFormat::Raw => {
            require_public(key)?;
            // The stored public point is already canonical uncompressed SEC1
            // (§2.3.3) — the raw export form.
            Ok(ExportedKey::Raw(ec_public_point(key).to_vec()))
        }
        KeyFormat::Spki => {
            require_public(key)?;
            Ok(ExportedKey::Raw(export_spki(curve, ec_public_point(key))?))
        }
        KeyFormat::Pkcs8 => {
            let scalar = ec_private_scalar(key)?;
            Ok(ExportedKey::Raw(export_pkcs8(curve, scalar)?))
        }
        KeyFormat::Jwk => Ok(ExportedKey::Jwk(export_jwk(algorithm, curve, key)?)),
    }
}

/// SPKI DER for an EC public key (WebCrypto §23.7.5 / §24.4.4 spki).
fn export_spki(curve: NamedCurve, point: &[u8]) -> Result<Vec<u8>, AlgorithmError> {
    with_curve!(curve, cc, {
        let pk = cc::PublicKey::from_sec1_bytes(point).map_err(|_| key_inaccessible())?;
        Ok(pk
            .to_public_key_der()
            .map_err(|_| key_inaccessible())?
            .as_bytes()
            .to_vec())
    })
}

/// PKCS#8 DER for an EC private key (WebCrypto §23.7.5 / §24.4.4 pkcs8); the
/// RustCrypto encoder includes the public key in the inner RFC 5915
/// ECPrivateKey, as the spec requires.
fn export_pkcs8(curve: NamedCurve, scalar: &[u8]) -> Result<Vec<u8>, AlgorithmError> {
    with_curve!(curve, cc, {
        let sk = cc::SecretKey::from_slice(scalar).map_err(|_| key_inaccessible())?;
        Ok(sk
            .to_pkcs8_der()
            .map_err(|_| key_inaccessible())?
            .as_bytes()
            .to_vec())
    })
}

/// Build the EC `jwk` (WebCrypto §23.7.5 / §24.4.4 jwk): `kty`="EC", `crv`,
/// `x` / `y` from the public point, `d` from the scalar (private only),
/// `key_ops` from the usages, `ext` from extractability.  ECDSA / ECDH set no
/// `alg` member on export (§23.7.5 / §24.4.4 omit it).
fn export_jwk(
    _algorithm: EcAlgorithm,
    curve: NamedCurve,
    key: &CryptoKeyData,
) -> Result<JsonWebKey, AlgorithmError> {
    let point = ec_public_point(key);
    let clen = curve.coordinate_len();
    // The stored point is uncompressed `0x04‖x‖y` (length 1 + 2·clen).
    if point.len() != 1 + 2 * clen {
        return Err(key_inaccessible());
    }
    let x = &point[1..=clen];
    let y = &point[1 + clen..=2 * clen];
    Ok(JsonWebKey {
        kty: Some("EC".to_string()),
        crv: Some(curve.as_str().to_string()),
        x: Some(URL_SAFE_NO_PAD.encode(x)),
        y: Some(URL_SAFE_NO_PAD.encode(y)),
        d: key
            .material
            .ec_private_scalar()
            .map(|s| URL_SAFE_NO_PAD.encode(s)),
        key_ops: Some(key.usages.iter().map(|u| u.as_str().to_string()).collect()),
        ext: Some(key.extractable),
        // The `oct` / RSA members are absent for an EC key.
        k: None,
        alg: None,
        use_: None,
        n: None,
        e: None,
        p: None,
        q: None,
        dp: None,
        dq: None,
        qi: None,
        oth: None,
    })
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Validate the import usages for `(algorithm, key_type)` (WebCrypto §23.7.4
/// ECDSA / §24.4.3 ECDH per-format "usages contains … → SyntaxError"): ECDSA
/// public → {verify}, private → {sign}; ECDH public → {} (none), private →
/// {deriveKey, deriveBits}.
fn validate_import_usages(
    algorithm: EcAlgorithm,
    key_type: KeyType,
    usages: &[KeyUsage],
) -> Result<(), AlgorithmError> {
    if usages
        .iter()
        .all(|&u| ec_usage_permitted(algorithm, key_type, u))
    {
        Ok(())
    } else {
        Err(AlgorithmError::Syntax(ec_usage_message(
            algorithm, key_type,
        )))
    }
}

/// Whether `usage` is permitted for an EC key of `(algorithm, key_type)`
/// (the per-family, per-key-type usage rules — §23.7 ECDSA / §24.4 ECDH).
fn ec_usage_permitted(algorithm: EcAlgorithm, key_type: KeyType, usage: KeyUsage) -> bool {
    match algorithm {
        EcAlgorithm::Ecdsa => usage.is_ecdsa_usage(key_type),
        EcAlgorithm::Ecdh => usage.is_ecdh_usage(key_type),
    }
}

fn ec_usage_message(algorithm: EcAlgorithm, key_type: KeyType) -> String {
    match (algorithm, key_type) {
        (EcAlgorithm::Ecdsa, KeyType::Public) => {
            "ECDSA public keys support only the 'verify' usage"
        }
        (EcAlgorithm::Ecdsa, _) => "ECDSA private keys support only the 'sign' usage",
        (EcAlgorithm::Ecdh, KeyType::Public) => "ECDH public keys support no key usages",
        (EcAlgorithm::Ecdh, _) => {
            "ECDH private keys support only the 'deriveKey' and 'deriveBits' usages"
        }
    }
    .to_string()
}

/// The §23.7.5 / §24.4.4 raw / spki "If [[type]] is not 'public' →
/// InvalidAccessError" gate.
fn require_public(key: &CryptoKeyData) -> Result<(), AlgorithmError> {
    if key.key_type == KeyType::Public {
        Ok(())
    } else {
        Err(AlgorithmError::InvalidAccess(
            "the key is not a public key".to_string(),
        ))
    }
}

/// The §23.7.5 / §24.4.4 pkcs8 "If [[type]] is not 'private' →
/// InvalidAccessError" gate, returning the private scalar.
fn ec_private_scalar(key: &CryptoKeyData) -> Result<&[u8], AlgorithmError> {
    key.material
        .ec_private_scalar()
        .ok_or_else(|| AlgorithmError::InvalidAccess("the key is not a private key".to_string()))
}

/// The stored SEC1 uncompressed public point (always present for an EC key).
fn ec_public_point(key: &CryptoKeyData) -> &[u8] {
    key.material
        .ec_public_point()
        .expect("an EC key always stores its public point")
}

fn decode_b64(s: &str) -> Result<Vec<u8>, AlgorithmError> {
    URL_SAFE_NO_PAD
        .decode(s)
        .map_err(|_| data("JWK EC member is not valid base64url"))
}

fn data(msg: &str) -> AlgorithmError {
    AlgorithmError::Data(msg.to_string())
}

fn invalid_access(msg: &str) -> AlgorithmError {
    AlgorithmError::InvalidAccess(msg.to_string())
}

fn operation(msg: &str) -> AlgorithmError {
    AlgorithmError::Operation(msg.to_string())
}

/// The §23.7.5 / §24.4.4 step-2 "key material cannot be accessed →
/// OperationError" — used for the (unreachable, key already validated)
/// re-encode failures.
fn key_inaccessible() -> AlgorithmError {
    AlgorithmError::Operation("the elliptic-curve key material cannot be accessed".to_string())
}
