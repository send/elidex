//! Operation-level entry points (WebCrypto §14.3.x) — the layering
//! boundary. Every spec-validation step (usages subset / empty usages /
//! extractable gate / length range / JWK shape / algorithm-name match)
//! lives here; the VM host only marshals JS ↔ these plain-Rust inputs
//! and settles the returned Promise.

use crate::algorithm::{AesVariant, EcAlgorithm, NormalizedAlgorithm};
use crate::error::AlgorithmError;
use crate::jwk::{self, JsonWebKey};
use crate::key::{normalize_usages, CryptoKeyData, KeyAlgorithm, KeyMaterial, KeyType, KeyUsage};
use crate::{aes, aes_kw, hkdf, hmac, pbkdf2};

/// The `KeyFormat` enum (WebCrypto §14.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyFormat {
    Raw,
    Pkcs8,
    Spki,
    Jwk,
}

/// `importKey` key material, already marshalled from JS by the VM:
/// `Raw` for the `raw` format (BufferSource bytes), `Jwk` for the `jwk`
/// format (the live JS object's members).
#[derive(Clone, Debug)]
pub enum KeyData {
    Raw(Vec<u8>),
    Jwk(JsonWebKey),
}

/// `exportKey` result — plain-Rust shapes the VM turns into an
/// `ArrayBuffer` or a JS object.
#[derive(Clone, Debug)]
pub enum ExportedKey {
    Raw(Vec<u8>),
    Jwk(JsonWebKey),
}

/// `generateKey` result (WebCrypto §14.3.6 `(CryptoKey or CryptoKeyPair)`
/// union): a [`Self::Single`] key for the symmetric algorithms (HMAC / AES),
/// or a [`Self::Pair`] for the asymmetric ones (EC).  The VM dispatches the
/// two shapes — one `CryptoKey` wrapper, or two wrappers assembled into a
/// `CryptoKeyPair` dictionary (§17).
#[derive(Clone, Debug)]
pub enum GeneratedKey {
    Single(CryptoKeyData),
    Pair {
        public: CryptoKeyData,
        private: CryptoKeyData,
    },
}

/// `generateKey` (WebCrypto §14.3.6) — a single key (symmetric: HMAC §31.6.3
/// / AES §27-§30) or a key pair (asymmetric: EC §23.7.3 / §24.4.1), returned
/// as the [`GeneratedKey`] union.
///
/// `fill_random` writes the OS CSPRNG bytes into the supplied buffer (the
/// VM owns the entropy source).  For the symmetric algorithms it is invoked
/// **after** the step-1 usage-kind check and step-2 length resolution, so an
/// invalid usage or zero length is rejected before any key-sized buffer is
/// allocated or filled; for EC it backs `SecretKey::random` via the crate's
/// `ClosureRng`.  All spec ordering + validation stays inside this crate
/// boundary (the VM only supplies entropy).  The bound is `FnMut` (not
/// `FnOnce`) because EC key generation may draw multiple times (rejection
/// sampling); the symmetric paths still call it once.
// The ops entry points take the freshly-normalized algorithm by value (the
// VM transfers ownership per call): `encrypt`/`decrypt` move the params'
// `iv`/`counter` out, and generate/import/sign/verify share that uniform
// signature even though they only inspect it.
#[allow(clippy::needless_pass_by_value)]
pub fn generate_key<F>(
    algorithm: NormalizedAlgorithm,
    extractable: bool,
    usages: Vec<KeyUsage>,
    fill_random: F,
) -> Result<GeneratedKey, AlgorithmError>
where
    F: FnMut(&mut [u8]) -> Result<(), AlgorithmError>,
{
    match algorithm {
        NormalizedAlgorithm::HmacKeyParams { hash, length } => {
            generate_hmac_key(hash, length, extractable, usages, fill_random)
                .map(GeneratedKey::Single)
        }
        NormalizedAlgorithm::AesKeyGen { variant, length } => {
            generate_aes_key(variant, length, extractable, usages, fill_random)
                .map(GeneratedKey::Single)
        }
        NormalizedAlgorithm::EcKeyGen {
            algorithm: ec_algorithm,
            curve,
        } => crate::ec::generate(ec_algorithm, curve, extractable, &usages, fill_random)
            .map(|(public, private)| GeneratedKey::Pair { public, private }),
        NormalizedAlgorithm::RsaKeyGen {
            variant,
            modulus_length,
            public_exponent,
            hash,
        } => crate::rsa::generate(
            variant,
            modulus_length,
            public_exponent,
            hash,
            extractable,
            &usages,
            fill_random,
        )
        .map(|(public, private)| GeneratedKey::Pair { public, private }),
        _ => Err(not_supported_op("generateKey")),
    }
}

fn generate_hmac_key<F>(
    hash: crate::hash::HashAlgorithm,
    length: Option<u32>,
    extractable: bool,
    usages: Vec<KeyUsage>,
    fill_random: F,
) -> Result<CryptoKeyData, AlgorithmError>
where
    F: FnOnce(&mut [u8]) -> Result<(), AlgorithmError>,
{
    // §31.6.3 step 1: a non-`sign`/`verify` usage is a SyntaxError —
    // before length sizing / buffer allocation (step 2+).
    validate_usage_kinds(&usages, KeyUsage::is_hmac_usage, HMAC_USAGE_MSG)?;
    // §31.6.3 step 2: resolve the key length (`length == 0` →
    // OperationError) and the byte count to fill.
    let byte_len = hmac::generate_key_byte_len(hash, length)?;
    let bit_len = hmac::generate_key_bit_len(hash, length);
    // §31.6.3 step 3 "Generate a key of length length bits": allocate +
    // fill only now that usage / length are valid.
    let mut material = vec![0u8; byte_len];
    fill_random(&mut material)?;
    // §14.3.6 generateKey generic step: a secret key with empty usages is
    // a SyntaxError — checked *after* the algorithm-specific op produced
    // the key (so an `OperationError`/`length` failure above wins).
    require_secret_usages_nonempty(&usages)?;
    let usages = normalize_usages(usages);
    // For a non-octet-aligned `length` the trailing low-order bits of the
    // final octet are not part of the key, so zero them.
    mask_to_bit_length(&mut material, bit_len);
    Ok(CryptoKeyData {
        key_type: KeyType::Secret,
        extractable,
        algorithm: KeyAlgorithm::Hmac {
            hash,
            length: bit_len,
        },
        usages,
        material: KeyMaterial::Raw(material),
    })
}

/// AES `generateKey` (WebCrypto §27.7.3 / §28.4.3 / §29.4.3 — identical
/// step shape across the three modes).
fn generate_aes_key<F>(
    variant: AesVariant,
    length: u32,
    extractable: bool,
    usages: Vec<KeyUsage>,
    fill_random: F,
) -> Result<CryptoKeyData, AlgorithmError>
where
    F: FnOnce(&mut [u8]) -> Result<(), AlgorithmError>,
{
    // step 1: an unsupported usage is a SyntaxError — before key sizing
    // (step 2+).  AES-KW accepts only {wrapKey, unwrapKey} (§30.3.3); the
    // block-cipher modes also accept {encrypt, decrypt} (§27.7.3 / §28.4.3 /
    // §29.4.3).
    let (allowed, msg) = aes_usage_rule(variant);
    validate_usage_kinds(&usages, allowed, msg)?;
    // step 2: the key length must be 128/192/256 bits, else OperationError.
    let byte_len = aes_key_byte_len(length)?;
    // step 3 "Generate an AES key of length length bits".
    let mut material = vec![0u8; byte_len];
    fill_random(&mut material)?;
    // §14.3.6 generic step: empty usages → SyntaxError, after the op
    // produced the key (so the length OperationError above wins).
    require_secret_usages_nonempty(&usages)?;
    let usages = normalize_usages(usages);
    Ok(CryptoKeyData {
        key_type: KeyType::Secret,
        extractable,
        algorithm: KeyAlgorithm::Aes { variant, length },
        usages,
        material: KeyMaterial::Raw(material),
    })
}

/// `importKey` (WebCrypto §14.3.9), dispatching on the normalized
/// algorithm family.
#[allow(clippy::needless_pass_by_value)] // uniform ops signature; see `generate_key`
pub fn import_key(
    format: KeyFormat,
    algorithm: NormalizedAlgorithm,
    extractable: bool,
    usages: Vec<KeyUsage>,
    key_data: KeyData,
) -> Result<CryptoKeyData, AlgorithmError> {
    match algorithm {
        NormalizedAlgorithm::HmacKeyParams { hash, length } => {
            import_hmac_key(format, hash, length, extractable, usages, key_data)
        }
        NormalizedAlgorithm::AesImport { variant } => {
            import_aes_key(format, variant, extractable, usages, key_data)
        }
        NormalizedAlgorithm::Hkdf => {
            import_kdf_key(format, KeyAlgorithm::Hkdf, extractable, usages, key_data)
        }
        NormalizedAlgorithm::Pbkdf2 => {
            import_kdf_key(format, KeyAlgorithm::Pbkdf2, extractable, usages, key_data)
        }
        NormalizedAlgorithm::EcImport { algorithm, curve } => {
            crate::ec::import(algorithm, curve, format, extractable, usages, key_data)
        }
        NormalizedAlgorithm::RsaImport { variant, hash } => {
            crate::rsa::import(variant, hash, format, extractable, usages, key_data)
        }
        _ => Err(not_supported_op("importKey")),
    }
}

fn import_hmac_key(
    format: KeyFormat,
    hash: crate::hash::HashAlgorithm,
    length: Option<u32>,
    extractable: bool,
    usages: Vec<KeyUsage>,
    key_data: KeyData,
) -> Result<CryptoKeyData, AlgorithmError> {
    // §31.6.4 Import Key step 2: a non-`sign`/`verify` usage is a
    // SyntaxError, checked before the key material is parsed.  The
    // *empty*-usages SyntaxError is a separate, later step
    // (§14.3.9 generic, below) — so empty usages must NOT short-circuit
    // material validation here, or `importKey('raw', new Uint8Array(0),
    // …, [])` would surface SyntaxError instead of the required DataError.
    validate_usage_kinds(&usages, KeyUsage::is_hmac_usage, HMAC_USAGE_MSG)?;

    let material = match (format, key_data) {
        (KeyFormat::Raw, KeyData::Raw(bytes)) => bytes,
        (KeyFormat::Jwk, KeyData::Jwk(jwk)) => {
            jwk::import_oct_hmac(&jwk, hash, extractable, &usages)?
        }
        (KeyFormat::Pkcs8 | KeyFormat::Spki, _) => {
            return Err(AlgorithmError::NotSupported(
                "HMAC import supports only the 'raw' and 'jwk' formats".to_string(),
            ));
        }
        // Format / data shape mismatch — the VM marshals them
        // consistently, so this is a defensive guard.
        _ => return Err(format_data_mismatch()),
    };

    // WebCrypto §31.6.4 HMAC Import Key (shared raw + jwk step): "Let
    // length be the length in bits of data. If length is zero then throw
    // a DataError." This fires regardless of the `length` member.
    if material.is_empty() {
        return Err(AlgorithmError::Data(
            "HMAC key material must not be empty".to_string(),
        ));
    }
    let length = resolve_import_length(material.len(), length)?;
    // §14.3.9 importKey generic step: a secret key with empty usages is a
    // SyntaxError — checked *after* the §31.6.4 op produced the key, so a
    // DataError/NotSupportedError from invalid material wins.
    require_secret_usages_nonempty(&usages)?;
    let usages = normalize_usages(usages);
    // §31.6.4 step 8 "an HMAC key with the first length bits of data":
    // step 7 admits a `length` member up to 7 bits below the data's bit
    // length, so zero the unused trailing bits of the final octet.
    let mut material = material;
    mask_to_bit_length(&mut material, length);
    Ok(CryptoKeyData {
        key_type: KeyType::Secret,
        extractable,
        algorithm: KeyAlgorithm::Hmac { hash, length },
        usages,
        material: KeyMaterial::Raw(material),
    })
}

/// AES `importKey` (WebCrypto §27.7.4 / §28.4.4 / §29.4.4 — identical step
/// shape across the three modes; the key length derives from the material).
fn import_aes_key(
    format: KeyFormat,
    variant: AesVariant,
    extractable: bool,
    usages: Vec<KeyUsage>,
    key_data: KeyData,
) -> Result<CryptoKeyData, AlgorithmError> {
    // step 1: an unsupported usage is a SyntaxError, before the material is
    // parsed (empty-usages is the later generic step).  AES-KW accepts only
    // {wrapKey, unwrapKey} (§30.3.4); the block-cipher modes also accept
    // {encrypt, decrypt} (§27.7.4 / §28.4.4 / §29.4.4).
    let (allowed, msg) = aes_usage_rule(variant);
    validate_usage_kinds(&usages, allowed, msg)?;

    let material = match (format, key_data) {
        (KeyFormat::Raw, KeyData::Raw(bytes)) => {
            // raw substep 2: the length in bits must be 128/192/256.
            if !matches!(bytes.len(), 16 | 24 | 32) {
                return Err(AlgorithmError::Data(
                    "AES key material must be 16, 24 or 32 bytes (128/192/256-bit)".to_string(),
                ));
            }
            bytes
        }
        (KeyFormat::Jwk, KeyData::Jwk(jwk)) => {
            jwk::import_oct_aes(&jwk, variant, extractable, &usages)?
        }
        (KeyFormat::Pkcs8 | KeyFormat::Spki, _) => {
            return Err(AlgorithmError::NotSupported(
                "AES import supports only the 'raw' and 'jwk' formats".to_string(),
            ));
        }
        _ => return Err(format_data_mismatch()),
    };

    // Material length is validated to 16/24/32 bytes above (raw + jwk), so
    // the bit length is exactly 128/192/256 and fits a `u32`.
    let length = u32::try_from(material.len() * 8).expect("AES key length validated to ≤ 32 bytes");
    // §14.3.9 generic step: empty usages → SyntaxError, after the op
    // produced the key (so a DataError from invalid material wins).
    require_secret_usages_nonempty(&usages)?;
    let usages = normalize_usages(usages);
    Ok(CryptoKeyData {
        key_type: KeyType::Secret,
        extractable,
        algorithm: KeyAlgorithm::Aes { variant, length },
        usages,
        material: KeyMaterial::Raw(material),
    })
}

/// HKDF / PBKDF2 `importKey` (WebCrypto §33.4.2 / §34.4.2 — `raw` only).
/// `algorithm` is the name-only [`KeyAlgorithm::Hkdf`] / [`KeyAlgorithm::Pbkdf2`].
///
/// The imported material is the password / input keying material of any
/// length (no length validation — unlike AES/HMAC); it is **never**
/// extractable (step "extractable must be false"), so the key has no
/// `exportKey` path.
fn import_kdf_key(
    format: KeyFormat,
    algorithm: KeyAlgorithm,
    extractable: bool,
    usages: Vec<KeyUsage>,
    key_data: KeyData,
) -> Result<CryptoKeyData, AlgorithmError> {
    // §33.4.2 / §34.4.2: only the `raw` format is supported (the spec lists
    // no other branch — anything else is a NotSupportedError).
    let material = match (format, key_data) {
        (KeyFormat::Raw, KeyData::Raw(bytes)) => bytes,
        (KeyFormat::Raw, _) => return Err(format_data_mismatch()),
        _ => {
            return Err(AlgorithmError::NotSupported(
                "HKDF / PBKDF2 import supports only the 'raw' format".to_string(),
            ));
        }
    };
    // §33.4.2 / §34.4.2 step: a usage outside {deriveKey, deriveBits} is a
    // SyntaxError (the empty-usages SyntaxError is the later generic step).
    validate_usage_kinds(&usages, KeyUsage::is_kdf_usage, KDF_USAGE_MSG)?;
    // §33.4.2 / §34.4.2 step: `extractable` must be false.
    if extractable {
        return Err(AlgorithmError::Syntax(
            "HKDF / PBKDF2 keys must be imported with extractable set to false".to_string(),
        ));
    }
    // §14.3.9 importKey generic step: a secret key with empty usages is a
    // SyntaxError — checked after the algorithm-specific steps.
    require_secret_usages_nonempty(&usages)?;
    let usages = normalize_usages(usages);
    Ok(CryptoKeyData {
        key_type: KeyType::Secret,
        extractable,
        algorithm,
        usages,
        material: KeyMaterial::Raw(material),
    })
}

/// `exportKey` (WebCrypto §14.3.10 + §31 / §23.7.5 / §24.4.4 Export Key).
pub fn export_key(format: KeyFormat, key: &CryptoKeyData) -> Result<ExportedKey, AlgorithmError> {
    // §14.3.10 step 6: the key's algorithm must support the export key
    // operation — checked BEFORE the step-7 extractable gate.  HKDF / PBKDF2
    // register no exportKey (§33.4 / §34.4), so exporting one is a
    // NotSupportedError regardless of (its always-false) extractability.  HMAC
    // / AES (§31 / §27-§30) and EC (§23.7.5 / §24.4.4) all support it.
    match key.algorithm {
        KeyAlgorithm::Hmac { .. }
        | KeyAlgorithm::Aes { .. }
        | KeyAlgorithm::Ecdsa { .. }
        | KeyAlgorithm::Ecdh { .. }
        | KeyAlgorithm::Rsa { .. } => {}
        KeyAlgorithm::Hkdf | KeyAlgorithm::Pbkdf2 => {
            return Err(AlgorithmError::NotSupported(
                "HKDF / PBKDF2 keys do not support the exportKey operation".to_string(),
            ));
        }
    }
    // §14.3.10 step 7: a non-extractable key is an InvalidAccessError.
    if !key.extractable {
        return Err(AlgorithmError::InvalidAccess(
            "key is not extractable".to_string(),
        ));
    }
    // Per-family export dispatch (the format → bytes / JWK mapping differs by
    // family): symmetric is raw-octets / oct-JWK; EC is SEC1 / SPKI / PKCS#8 /
    // EC-JWK (the `ec` backend, PR-4 commit 2).
    match key.algorithm {
        KeyAlgorithm::Hmac { .. } | KeyAlgorithm::Aes { .. } => export_symmetric(format, key),
        KeyAlgorithm::Ecdsa { curve } => crate::ec::export(EcAlgorithm::Ecdsa, curve, format, key),
        KeyAlgorithm::Ecdh { curve } => crate::ec::export(EcAlgorithm::Ecdh, curve, format, key),
        // RSA export (§20.8.5 / §21.4.5) — the variant + hash drive the jwk
        // `alg` (RS256 / PS384 / …); spki / pkcs8 are verbatim canonical DER.
        KeyAlgorithm::Rsa { variant, hash, .. } => crate::rsa::export(variant, hash, format, key),
        KeyAlgorithm::Hkdf | KeyAlgorithm::Pbkdf2 => unreachable!("KDF rejected at step 6"),
    }
}

/// Export a symmetric (HMAC / AES) key (WebCrypto §31 / §27-§30 Export Key)
/// — `raw` octets verbatim or the `oct` JWK; `pkcs8` / `spki` are
/// asymmetric-only (NotSupportedError).  Called only for HMAC / AES (the
/// step-6/step-7 gates ran in [`export_key`]).
fn export_symmetric(format: KeyFormat, key: &CryptoKeyData) -> Result<ExportedKey, AlgorithmError> {
    match format {
        KeyFormat::Raw => Ok(ExportedKey::Raw(key.material.as_bytes().to_vec())),
        KeyFormat::Jwk => Ok(ExportedKey::Jwk(match key.algorithm {
            KeyAlgorithm::Hmac { hash, .. } => jwk::export_oct_hmac(key, hash),
            KeyAlgorithm::Aes { variant, length } => jwk::export_oct_aes(key, variant, length),
            // Only HMAC / AES reach `export_symmetric`.
            KeyAlgorithm::Hkdf
            | KeyAlgorithm::Pbkdf2
            | KeyAlgorithm::Ecdsa { .. }
            | KeyAlgorithm::Ecdh { .. }
            | KeyAlgorithm::Rsa { .. } => {
                unreachable!("export_symmetric called only for HMAC/AES")
            }
        })),
        KeyFormat::Pkcs8 | KeyFormat::Spki => Err(AlgorithmError::NotSupported(
            "symmetric key export supports only the 'raw' and 'jwk' formats".to_string(),
        )),
    }
}

/// `sign` (WebCrypto §14.3.3 + §31 HMAC / §23.7.1 ECDSA / §20.8.1
/// RSASSA-PKCS1-v1_5 / §21.4.1 RSA-PSS).  `fill_random` is the VM entropy seam
/// — only the RSA-PSS salt draws from it (HMAC / ECDSA / RSASSA-PKCS1-v1_5 are
/// deterministic), so for those algorithms the closure is never invoked.
#[allow(clippy::needless_pass_by_value)] // uniform ops signature; see `generate_key`
pub fn sign<F>(
    algorithm: NormalizedAlgorithm,
    key: &CryptoKeyData,
    data: &[u8],
    fill_random: F,
) -> Result<Vec<u8>, AlgorithmError>
where
    F: FnMut(&mut [u8]) -> Result<(), AlgorithmError>,
{
    require_key_usable(&algorithm, key, KeyUsage::Sign)?;
    match key.algorithm {
        KeyAlgorithm::Hmac { hash, .. } => Ok(hmac::sign(hash, key.material.as_bytes(), data)),
        KeyAlgorithm::Ecdsa { curve } => {
            // The name-match above admitted this ECDSA key, so `sign`
            // normalized to `EcdsaParams` (the only ECDSA sign form); its
            // `hash` is the signature hash.
            let NormalizedAlgorithm::EcdsaParams { hash } = algorithm else {
                return Err(not_supported_op("sign"));
            };
            crate::ec::sign(curve, hash, key, data)
        }
        // RSA: the variant + hash ride on the key (§20.6); RSA-PSS reads its
        // `saltLength` from the normalized params (RSASSA params are name-only).
        KeyAlgorithm::Rsa { variant, hash, .. } => {
            let salt_length = match algorithm {
                NormalizedAlgorithm::RsaPssParams { salt_length } => Some(salt_length),
                _ => None,
            };
            crate::rsa::sign(variant, hash, key, data, salt_length, fill_random)
        }
        // `sign` normalizes only HMAC + ECDSA + RSA, so the name-match above
        // rejects any other key before reaching here.
        KeyAlgorithm::Aes { .. }
        | KeyAlgorithm::Hkdf
        | KeyAlgorithm::Pbkdf2
        | KeyAlgorithm::Ecdh { .. } => Err(not_supported_op("sign")),
    }
}

/// `verify` (WebCrypto §14.3.4 + §31 Verify).
#[allow(clippy::needless_pass_by_value)] // uniform ops signature; see `generate_key`
pub fn verify(
    algorithm: NormalizedAlgorithm,
    key: &CryptoKeyData,
    signature: &[u8],
    data: &[u8],
) -> Result<bool, AlgorithmError> {
    require_key_usable(&algorithm, key, KeyUsage::Verify)?;
    match key.algorithm {
        KeyAlgorithm::Hmac { hash, .. } => {
            Ok(hmac::verify(hash, key.material.as_bytes(), signature, data))
        }
        KeyAlgorithm::Ecdsa { curve } => {
            let NormalizedAlgorithm::EcdsaParams { hash } = algorithm else {
                return Err(not_supported_op("verify"));
            };
            crate::ec::verify(curve, hash, key, signature, data)
        }
        // RSA: variant + hash from the key; RSA-PSS `saltLength` from the params.
        KeyAlgorithm::Rsa { variant, hash, .. } => {
            let salt_length = match algorithm {
                NormalizedAlgorithm::RsaPssParams { salt_length } => Some(salt_length),
                _ => None,
            };
            crate::rsa::verify(variant, hash, key, signature, data, salt_length)
        }
        // `verify` normalizes only HMAC + ECDSA + RSA.
        KeyAlgorithm::Aes { .. }
        | KeyAlgorithm::Hkdf
        | KeyAlgorithm::Pbkdf2
        | KeyAlgorithm::Ecdh { .. } => Err(not_supported_op("verify")),
    }
}

/// `encrypt` (WebCrypto §14.3.1 → §27.7.1 / §28.4.1 / §29.4.1).  Consumes
/// the normalized params, moving the `iv` / `counter` / `additionalData`
/// out and passing them straight to the cipher (no copy beyond the
/// marshal-time snapshot).
pub fn encrypt(
    algorithm: NormalizedAlgorithm,
    key: &CryptoKeyData,
    data: &[u8],
) -> Result<Vec<u8>, AlgorithmError> {
    require_key_usable(&algorithm, key, KeyUsage::Encrypt)?;
    aes_encrypt_op(algorithm, key.material.as_bytes(), data)
}

/// The AES encrypt *operation* dispatch (no §14.3.1 name/usage gate) — shared
/// by [`encrypt`] (after its gate) and the [`wrap_key`] §14.3.11 step-15
/// encrypt fallback (whose gate is the `wrapKey` usage, not `encrypt`).  The
/// caller has already validated the key length to 16/24/32 bytes.
fn aes_encrypt_op(
    algorithm: NormalizedAlgorithm,
    material: &[u8],
    data: &[u8],
) -> Result<Vec<u8>, AlgorithmError> {
    match algorithm {
        NormalizedAlgorithm::AesGcm {
            iv,
            additional_data,
            tag_length,
        } => aes::encrypt_gcm(
            material,
            &iv,
            additional_data.as_deref().unwrap_or(&[]),
            data,
            tag_length,
        ),
        NormalizedAlgorithm::AesCbc { iv } => aes::encrypt_cbc(material, &iv, data),
        NormalizedAlgorithm::AesCtr { counter, length } => {
            aes::encrypt_ctr(material, &counter, length, data)
        }
        // Only the AES modes normalize to an encrypt op; the name-match in the
        // caller rejects any other key/algorithm before reaching here.
        _ => Err(not_supported_op("encrypt")),
    }
}

/// `decrypt` (WebCrypto §14.3.2 → §27.7.2 / §28.4.2 / §29.4.2).
pub fn decrypt(
    algorithm: NormalizedAlgorithm,
    key: &CryptoKeyData,
    data: &[u8],
) -> Result<Vec<u8>, AlgorithmError> {
    require_key_usable(&algorithm, key, KeyUsage::Decrypt)?;
    aes_decrypt_op(algorithm, key.material.as_bytes(), data)
}

/// The AES decrypt *operation* dispatch (no §14.3.2 name/usage gate) — shared
/// by [`decrypt`] (after its gate) and the [`unwrap_key`] §14.3.12 step-14
/// decrypt fallback (whose gate is the `unwrapKey` usage, not `decrypt`).
fn aes_decrypt_op(
    algorithm: NormalizedAlgorithm,
    material: &[u8],
    data: &[u8],
) -> Result<Vec<u8>, AlgorithmError> {
    match algorithm {
        NormalizedAlgorithm::AesGcm {
            iv,
            additional_data,
            tag_length,
        } => aes::decrypt_gcm(
            material,
            &iv,
            additional_data.as_deref().unwrap_or(&[]),
            data,
            tag_length,
        ),
        NormalizedAlgorithm::AesCbc { iv } => aes::decrypt_cbc(material, &iv, data),
        NormalizedAlgorithm::AesCtr { counter, length } => {
            aes::decrypt_ctr(material, &counter, length, data)
        }
        _ => Err(not_supported_op("decrypt")),
    }
}

/// `wrapKey` (WebCrypto §14.3.11), composing the full export-then-wrap pipeline
/// in the engine-independent crate so the VM only marshals + normalizes.
///
/// Steps, on `wrapping_key` then `key`: name-equality (step 9) with the
/// `wrapKey` usage check (step 10); the export-support and extractable gates
/// plus the export itself (steps 11-13, via [`export_key`]); the serialization
/// (step 14, raw bytes verbatim, or — for `jwk` — [`crate::jwk::to_json_bytes`],
/// which serializes the `oct` JWK to JSON over the Rust struct **isolated from
/// the page realm** per the §14.3.11 step-14 "new global object" requirement —
/// no page `Object.prototype.toJSON` is invoked); then the wrap-or-encrypt
/// dispatch (step 15), where AES-KW wraps via RFC 3394 while an AES-GCM/CBC/CTR
/// wrapping key falls back to its encrypt *operation* (with no `encrypt`-usage
/// recheck — §14.3.11 step 15 invokes the operation, not the `encrypt` method).
///
/// The wrappingKey gate (steps 9-10) runs **before** the key export (steps
/// 11-13), matching the spec order (so a wrappingKey name/usage `InvalidAccess`
/// wins over a non-exportable `key` `NotSupported`).
pub fn wrap_key(
    algorithm: NormalizedAlgorithm,
    wrapping_key: &CryptoKeyData,
    key: &CryptoKeyData,
    format: KeyFormat,
) -> Result<Vec<u8>, AlgorithmError> {
    // §14.3.11 step 9 (name equality) + step 10 (wrapKey usage) on wrappingKey.
    require_key_usable(&algorithm, wrapping_key, KeyUsage::WrapKey)?;
    // steps 11-13: export-support check + extractable gate + export the key.
    let bytes = match export_key(format, key)? {
        // step 14 (non-jwk): the exported bytes are the plaintext verbatim.
        ExportedKey::Raw(raw) => raw,
        // step 14 (jwk): serialize the exported oct JWK to JSON bytes, in-crate
        // and realm-isolated (no page `toJSON`).
        ExportedKey::Jwk(jwk) => jwk::to_json_bytes(&jwk),
    };
    // step 15: wrap (AES-KW) or fall back to the encrypt op (AES-GCM/CBC/CTR).
    match algorithm {
        NormalizedAlgorithm::AesKwWrap => aes_kw::wrap(wrapping_key.material.as_bytes(), &bytes),
        _ => aes_encrypt_op(algorithm, wrapping_key.material.as_bytes(), &bytes),
    }
}

/// `unwrapKey` (WebCrypto §14.3.12) decrypt half: the name-equality (step 12)
/// and `unwrapKey`-usage (step 13) gate on `unwrapping_key`, then the unwrap
/// (or decrypt-fallback) operation (step 14) yielding the wrapped key's bytes.
///
/// The VM completes the method: "parse a JWK" (step 15, for `jwk` format) and
/// the `importKey` of the bytes (step 16, via [`import_key`], which also
/// enforces the step-17 empty-secret-usages SyntaxError).  Unlike [`wrap_key`]
/// this needs no closure — the JSON parse + import happen entirely *after* this
/// op returns, so the op stays a plain gate-then-decrypt.
pub fn unwrap_key(
    algorithm: NormalizedAlgorithm,
    unwrapping_key: &CryptoKeyData,
    wrapped_key: &[u8],
) -> Result<Vec<u8>, AlgorithmError> {
    // §14.3.12 step 12 (name equality) + step 13 (unwrapKey usage).
    require_key_usable(&algorithm, unwrapping_key, KeyUsage::UnwrapKey)?;
    // step 14: unwrap (AES-KW) or fall back to the decrypt op (AES-GCM/CBC/CTR).
    match algorithm {
        NormalizedAlgorithm::AesKwWrap => {
            aes_kw::unwrap(unwrapping_key.material.as_bytes(), wrapped_key)
        }
        _ => aes_decrypt_op(algorithm, unwrapping_key.material.as_bytes(), wrapped_key),
    }
}

/// `deriveBits` method-level op (WebCrypto §14.3.8): the name-equality
/// (step 8) + `deriveBits`-usage (step 9) gates, then the algorithm-internal
/// derive (§33.4.1 HKDF / §34.4.1 PBKDF2).  `length` is the `unsigned long?`
/// argument (`None` ⇒ the §33.4.1 / §34.4.1 step-1 OperationError).
#[allow(clippy::needless_pass_by_value)] // uniform ops signature; see `generate_key`
pub fn derive_bits(
    algorithm: NormalizedAlgorithm,
    base_key: &CryptoKeyData,
    length: Option<u32>,
) -> Result<Vec<u8>, AlgorithmError> {
    require_key_usable(&algorithm, base_key, KeyUsage::DeriveBits)?;
    derive_secret_bits(&algorithm, base_key, length)
}

/// The algorithm-internal derive-bits dispatch shared by [`derive_bits`] and
/// [`derive_key`] (after their §14.3.8 / §14.3.7 name + usage gates): ECDH
/// (§24.4.2) takes the base key + peer (a non-flat key material), while the
/// KDFs (HKDF §33.4.1 / PBKDF2 §34.4.1) take the flat key material.  Routing
/// here keeps the EC path off [`derive_bits_raw`]'s `as_bytes` + multiple-of-8
/// length rule (the ECDH length semantics differ).
fn derive_secret_bits(
    algorithm: &NormalizedAlgorithm,
    base_key: &CryptoKeyData,
    length: Option<u32>,
) -> Result<Vec<u8>, AlgorithmError> {
    match algorithm {
        NormalizedAlgorithm::EcdhDerive { peer } => crate::ec::derive_bits(base_key, peer, length),
        _ => derive_bits_raw(algorithm, base_key.material.as_bytes(), length),
    }
}

/// The algorithm-internal derive-bits operation (WebCrypto §33.4.1 HKDF /
/// §34.4.1 PBKDF2), shared by [`derive_bits`] and [`derive_key`].
///
/// `derive_key` calls this directly (NOT [`derive_bits`]) because §14.3.7
/// step 15 performs the derive-bits *operation* without re-checking the
/// `deriveBits` usage — the §14.3.7 step-13 gate is `deriveKey`, not
/// `deriveBits` — and without re-matching the name (already done at step 12).
///
/// The §33.4.1 / §34.4.1 step-1 `length` constraint (null or not a multiple
/// of 8 → OperationError) is common to both KDFs and enforced here, before
/// dispatch; the PBKDF2-specific `iterations == 0` / `length == 0` steps
/// live in [`crate::pbkdf2::derive_bits`].
fn derive_bits_raw(
    algorithm: &NormalizedAlgorithm,
    key_material: &[u8],
    length: Option<u32>,
) -> Result<Vec<u8>, AlgorithmError> {
    // §33.4.1 step 1 / §34.4.1 step 1: a null or non-multiple-of-8 length is
    // an OperationError (a `deriveKey` whose `derivedKeyType` is itself a KDF
    // gets `length = None` from get-key-length and degenerates here, which is
    // spec-correct — there is no fixed-length KDF key to derive).
    let length_bits = match length {
        Some(l) if l % 8 == 0 => l,
        _ => {
            return Err(AlgorithmError::Operation(
                "derived bit length must be a non-null multiple of 8".to_string(),
            ));
        }
    };
    match algorithm {
        NormalizedAlgorithm::HkdfParams { hash, salt, info } => {
            hkdf::derive_bits(*hash, key_material, salt, info, length_bits)
        }
        NormalizedAlgorithm::Pbkdf2Params {
            salt,
            iterations,
            hash,
        } => pbkdf2::derive_bits(*hash, key_material, salt, *iterations, length_bits),
        // `derive_bits` / `derive_key` only normalize HKDF / PBKDF2 for the
        // derive algorithm, so the name-match upstream rejects anything else.
        _ => Err(not_supported_op("deriveBits")),
    }
}

/// `deriveKey` (WebCrypto §14.3.7), composing the three normalized
/// algorithms the VM supplies: `derive_algorithm` (op `deriveBits`),
/// `import_algorithm` + `length_algorithm` (both the `derivedKeyType`,
/// normalized for op `importKey` / `get key length`).
///
/// Steps: name-equality (step 12) + `deriveKey`-usage (step 13) on
/// `base_key`; `length` = get-key-length(`length_algorithm`) (step 14);
/// `secret` = derive-bits (step 15, no usage/name recheck — via the shared
/// `derive_bits_raw`); `result` = importKey("raw", secret,
/// `import_algorithm`, …) (step 16, which enforces the step-17 empty-usages
/// SyntaxError for a secret key via `require_secret_usages_nonempty`).
#[allow(clippy::needless_pass_by_value)] // uniform ops signature; see `generate_key`
pub fn derive_key(
    derive_algorithm: NormalizedAlgorithm,
    base_key: &CryptoKeyData,
    import_algorithm: NormalizedAlgorithm,
    length_algorithm: NormalizedAlgorithm,
    extractable: bool,
    usages: Vec<KeyUsage>,
) -> Result<CryptoKeyData, AlgorithmError> {
    // §14.3.7 step 12 (name equality) + step 13 (deriveKey usage).
    require_name_match(&derive_algorithm, base_key)?;
    require_usage(base_key, KeyUsage::DeriveKey)?;
    // step 14: length = get key length of the derivedKeyType.
    let length = get_key_length(length_algorithm)?;
    // step 15: derive `secret` (no deriveBits-usage / name recheck) — ECDH or
    // a KDF, via the shared dispatch.
    let secret = derive_secret_bits(&derive_algorithm, base_key, length)?;
    // step 16: importKey("raw", secret, derivedKeyType, extractable, usages)
    // — also raises the step-17 empty-usages SyntaxError for a secret key.
    import_key(
        KeyFormat::Raw,
        import_algorithm,
        extractable,
        usages,
        KeyData::Raw(secret),
    )
}

/// `get key length` (WebCrypto §27.7.6 / §28.4.6 / §29.4.6 AES, §31.6.6
/// HMAC, §33.4.3 / §34.4.3 HKDF / PBKDF2), run on a `derivedKeyType` during
/// `deriveKey` (§14.3.7 step 14).  `Ok(None)` is the KDF "null" length.
#[allow(clippy::needless_pass_by_value)] // uniform ops signature; see `generate_key`
pub fn get_key_length(algorithm: NormalizedAlgorithm) -> Result<Option<u32>, AlgorithmError> {
    match algorithm {
        // §27.7.6 / §28.4.6 / §29.4.6: 128/192/256 else OperationError. The
        // `variant` is irrelevant to the length (AES-CTR/CBC/GCM share the
        // rule); it rides along only so `NormalizedAlgorithm::name` is total.
        NormalizedAlgorithm::AesKeyGen { length, .. } => match length {
            128 | 192 | 256 => Ok(Some(length)),
            _ => Err(AlgorithmError::Operation(
                "AES key length must be 128, 192 or 256 bits".to_string(),
            )),
        },
        // §31.6.6: `length` absent → the hash block size; present & non-zero
        // → that value; zero → TypeError (HMAC is a common derivedKeyType, so
        // PBKDF2/HKDF → HMAC signing keys exercise this arm).
        NormalizedAlgorithm::HmacKeyParams { hash, length } => match length {
            None => Ok(Some(hash.block_size_bits())),
            Some(0) => Err(AlgorithmError::Type(
                "HMAC key length must be greater than zero".to_string(),
            )),
            Some(l) => Ok(Some(l)),
        },
        // §33.4.3 / §34.4.3: KDFs return null (a `deriveKey` whose
        // `derivedKeyType` is a KDF then degenerates at derive-bits).
        NormalizedAlgorithm::Hkdf | NormalizedAlgorithm::Pbkdf2 => Ok(None),
        _ => Err(not_supported_op("get key length")),
    }
}

const HMAC_USAGE_MSG: &str = "HMAC keys support only the 'sign' and 'verify' usages";
const AES_USAGE_MSG: &str =
    "AES keys support only the 'encrypt', 'decrypt', 'wrapKey' and 'unwrapKey' usages";
const AES_KW_USAGE_MSG: &str = "AES-KW keys support only the 'wrapKey' and 'unwrapKey' usages";
const KDF_USAGE_MSG: &str =
    "HKDF / PBKDF2 keys support only the 'deriveKey' and 'deriveBits' usages";

/// The generate/import usage predicate + its error message for an AES key of
/// `variant` (WebCrypto generate/import step 1).  AES-KW (§30) is wrap-only
/// ({wrapKey, unwrapKey}); the three block-cipher modes also accept
/// {encrypt, decrypt} (§27 / §28 / §29).
fn aes_usage_rule(variant: AesVariant) -> (fn(KeyUsage) -> bool, &'static str) {
    match variant {
        AesVariant::Kw => (KeyUsage::is_aes_kw_usage, AES_KW_USAGE_MSG),
        AesVariant::Ctr | AesVariant::Cbc | AesVariant::Gcm => {
            (KeyUsage::is_aes_usage, AES_USAGE_MSG)
        }
    }
}

/// Reject any usage the algorithm does not accept (WebCrypto generate/import
/// step 1 — algorithm-specific, runs *before* key material is produced).
/// Empty usages pass here; the empty-usages SyntaxError is a separate, later
/// generic step ([`require_secret_usages_nonempty`]).
fn validate_usage_kinds(
    usages: &[KeyUsage],
    allowed: impl Fn(KeyUsage) -> bool,
    msg: &str,
) -> Result<(), AlgorithmError> {
    for &usage in usages {
        if !allowed(usage) {
            return Err(AlgorithmError::Syntax(msg.to_string()));
        }
    }
    Ok(())
}

/// AES generate-key length validation (WebCrypto §27.7.3 / §28.4.3 /
/// §29.4.3 step 2): 128/192/256 bits → 16/24/32 bytes, else OperationError.
fn aes_key_byte_len(length_bits: u32) -> Result<usize, AlgorithmError> {
    match length_bits {
        128 => Ok(16),
        192 => Ok(24),
        256 => Ok(32),
        _ => Err(AlgorithmError::Operation(
            "AES key length must be 128, 192 or 256 bits".to_string(),
        )),
    }
}

/// A `format` / `keyData` shape mismatch — the VM marshals them
/// consistently, so this is a defensive `TypeError`.  Shared with the EC
/// import backend (`crate::ec`).
pub(crate) fn format_data_mismatch() -> AlgorithmError {
    AlgorithmError::Type("keyData does not match the requested format".to_string())
}

/// A secret (or private) key with empty usages is a SyntaxError
/// (WebCrypto §14.3.6 generateKey / §14.3.9 importKey generic step).  This
/// runs *after* the algorithm-specific op has produced the key, so a
/// DataError / OperationError from invalid key material takes precedence.
fn require_secret_usages_nonempty(usages: &[KeyUsage]) -> Result<(), AlgorithmError> {
    if usages.is_empty() {
        return Err(AlgorithmError::Syntax("usages cannot be empty".to_string()));
    }
    Ok(())
}

/// Zero the unused trailing (low-order) bits of the final octet so the
/// material represents exactly the first `length_bits` bits of the key
/// (WebCrypto §31.6.3 step 3 "key of length length bits" / §31.6.4 step 8
/// "first length bits of data").
///
/// Both callers supply exactly `ceil(length_bits / 8)` octets — generate
/// fills that many CSPRNG bytes, and the import range check (§31.6.4
/// step 7) bounds `length_bits` to within 7 bits of the data — so the only
/// partial octet is the last one, holding `length_bits mod 8` significant
/// (high-order) bits.  An octet-aligned `length_bits` is a no-op.
fn mask_to_bit_length(material: &mut [u8], length_bits: u32) {
    let used_bits_in_last = (length_bits % 8) as u8;
    if used_bits_in_last == 0 {
        return; // octet-aligned (or empty) — nothing to mask
    }
    if let Some(last) = material.last_mut() {
        // Keep the high `used` bits, zero the low `8 - used`.
        *last &= 0xFFu8 << (8 - used_bits_in_last);
    }
}

/// Resolve + range-check the HMAC import `length` member against the
/// `data` octet length (WebCrypto §31 Import Key): accept
/// `8·len − 8 < length ≤ 8·len`, else `DataError`. `length` is metadata
/// only — the full `material` is the key.
fn resolve_import_length(material_len: usize, length: Option<u32>) -> Result<u32, AlgorithmError> {
    // Callers reject empty material first (§31.6.4 zero-length DataError),
    // so `material_len >= 1` and `data_bits >= 8` here.
    let data_bits = u32::try_from(material_len)
        .ok()
        .and_then(|n| n.checked_mul(8))
        .ok_or_else(|| AlgorithmError::Data("HMAC key material is too large".to_string()))?;
    match length {
        None => Ok(data_bits),
        Some(l) => {
            if l > data_bits || l <= data_bits - 8 {
                return Err(AlgorithmError::Data(
                    "HMAC import 'length' is out of range for the supplied key material"
                        .to_string(),
                ));
            }
            Ok(l)
        }
    }
}

/// Gate a sign/verify/encrypt/decrypt op on the key (WebCrypto §14.3.x):
/// the normalized algorithm `name` must equal the key's `[[algorithm]]`
/// name, then the key's `[[usages]]` must contain the op's usage — both
/// `InvalidAccessError`, in that spec step order.
fn require_key_usable(
    algorithm: &NormalizedAlgorithm,
    key: &CryptoKeyData,
    usage: KeyUsage,
) -> Result<(), AlgorithmError> {
    require_name_match(algorithm, key)?;
    require_usage(key, usage)?;
    Ok(())
}

fn require_usage(key: &CryptoKeyData, usage: KeyUsage) -> Result<(), AlgorithmError> {
    if key.has_usage(usage) {
        Ok(())
    } else {
        Err(AlgorithmError::InvalidAccess(format!(
            "key does not support the '{}' operation",
            usage.as_str()
        )))
    }
}

fn require_name_match(
    algorithm: &NormalizedAlgorithm,
    key: &CryptoKeyData,
) -> Result<(), AlgorithmError> {
    if algorithm.name() == key.algorithm.name() {
        Ok(())
    } else {
        Err(AlgorithmError::InvalidAccess(
            "algorithm does not match the key's algorithm".to_string(),
        ))
    }
}

fn not_supported_op(op: &str) -> AlgorithmError {
    AlgorithmError::NotSupported(format!("algorithm is not supported for {op}"))
}
