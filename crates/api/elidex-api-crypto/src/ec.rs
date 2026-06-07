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
use elliptic_curve::pkcs8::spki::{DecodePublicKey, EncodePublicKey};
use elliptic_curve::pkcs8::{DecodePrivateKey, EncodePrivateKey};
use elliptic_curve::rand_core::{CryptoRng, RngCore};
use elliptic_curve::sec1::ToEncodedPoint;

use crate::algorithm::{EcAlgorithm, NamedCurve};
use crate::error::AlgorithmError;
use crate::jwk::{self, JsonWebKey};
use crate::key::{normalize_usages, CryptoKeyData, KeyMaterial, KeyType, KeyUsage};
use crate::ops::{format_data_mismatch, ExportedKey, KeyData, KeyFormat};

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
        algorithm: key_alg,
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

/// An RNG adapter over the VM's `fill_random` closure, so EC key generation
/// draws from the single VM entropy seam (the closure ultimately calls the OS
/// CSPRNG) rather than a separate `getrandom` path, while `SecretKey::random`
/// still does the vetted rejection sampling.
///
/// `fill_random` is fallible but `RngCore::fill_bytes` is infallible, so a
/// closure error is captured and surfaced by [`Self::into_result`] after
/// keygen.  On error the buffer is filled with the canonical scalar `1`
/// (big-endian `…01`) — a valid non-zero scalar on every supported curve —
/// so `SecretKey::random`'s rejection loop terminates (a zero / out-of-range
/// fill would loop forever); the resulting key is then discarded because
/// `into_result` returns the captured error.
struct ClosureRng<'a> {
    fill: &'a mut dyn FnMut(&mut [u8]) -> Result<(), AlgorithmError>,
    error: Option<AlgorithmError>,
}

impl<'a> ClosureRng<'a> {
    fn new(fill: &'a mut dyn FnMut(&mut [u8]) -> Result<(), AlgorithmError>) -> Self {
        Self { fill, error: None }
    }

    fn into_result(self) -> Result<(), AlgorithmError> {
        match self.error {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }
}

impl RngCore for ClosureRng<'_> {
    fn next_u32(&mut self) -> u32 {
        let mut b = [0u8; 4];
        self.fill_bytes(&mut b);
        u32::from_le_bytes(b)
    }

    fn next_u64(&mut self) -> u64 {
        let mut b = [0u8; 8];
        self.fill_bytes(&mut b);
        u64::from_le_bytes(b)
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        if self.error.is_none() {
            if let Err(e) = (self.fill)(dest) {
                self.error = Some(e);
            }
        }
        if self.error.is_some() {
            // Canonical scalar `1` so the rejection loop terminates; the key is
            // discarded via `into_result`.
            dest.fill(0);
            if let Some(last) = dest.last_mut() {
                *last = 1;
            }
        }
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), elliptic_curve::rand_core::Error> {
        self.fill_bytes(dest);
        Ok(())
    }
}

impl CryptoRng for ClosureRng<'_> {}

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
        // The `oct` members are absent for an EC key.
        k: None,
        alg: None,
        use_: None,
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

/// The §23.7.5 / §24.4.4 step-2 "key material cannot be accessed →
/// OperationError" — used for the (unreachable, key already validated)
/// re-encode failures.
fn key_inaccessible() -> AlgorithmError {
    AlgorithmError::Operation("the elliptic-curve key material cannot be accessed".to_string())
}
