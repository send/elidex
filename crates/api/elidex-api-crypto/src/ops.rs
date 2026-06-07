//! Operation-level entry points (WebCrypto §14.3.x) — the layering
//! boundary. Every spec-validation step (usages subset / empty usages /
//! extractable gate / length range / JWK shape / algorithm-name match)
//! lives here; the VM host only marshals JS ↔ these plain-Rust inputs
//! and settles the returned Promise.

use crate::aes;
use crate::algorithm::{AesVariant, NormalizedAlgorithm};
use crate::error::AlgorithmError;
use crate::hmac;
use crate::jwk::{self, JsonWebKey};
use crate::key::{normalize_usages, CryptoKeyData, KeyAlgorithm, KeyMaterial, KeyType, KeyUsage};

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

/// `generateKey` for HMAC (WebCrypto §14.3.6 + §31.6.3 Generate Key).
///
/// `fill_random` writes the OS CSPRNG bytes into the supplied buffer (the
/// VM owns the entropy source).  It is invoked **after** the §31.6.3
/// step-1 usage-kind check and step-2 length resolution, so an invalid
/// usage or zero length is rejected before any key-sized buffer is
/// allocated or filled — keeping all the spec ordering + validation inside
/// this crate boundary (the VM only supplies entropy).
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
) -> Result<CryptoKeyData, AlgorithmError>
where
    F: FnOnce(&mut [u8]) -> Result<(), AlgorithmError>,
{
    match algorithm {
        NormalizedAlgorithm::HmacKeyParams { hash, length } => {
            generate_hmac_key(hash, length, extractable, usages, fill_random)
        }
        NormalizedAlgorithm::AesKeyGen { variant, length } => {
            generate_aes_key(variant, length, extractable, usages, fill_random)
        }
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
    // step 1: a usage outside {encrypt, decrypt, wrapKey, unwrapKey} is a
    // SyntaxError — before key sizing (step 2+).
    validate_usage_kinds(&usages, KeyUsage::is_aes_usage, AES_USAGE_MSG)?;
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
    // step 1: a usage outside {encrypt, decrypt, wrapKey, unwrapKey} is a
    // SyntaxError, before the material is parsed (empty-usages is the later
    // generic step).
    validate_usage_kinds(&usages, KeyUsage::is_aes_usage, AES_USAGE_MSG)?;

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

/// `exportKey` (WebCrypto §14.3.10 + §31 Export Key). `extractable=false`
/// gates every format with `InvalidAccessError`.
pub fn export_key(format: KeyFormat, key: &CryptoKeyData) -> Result<ExportedKey, AlgorithmError> {
    if !key.extractable {
        return Err(AlgorithmError::InvalidAccess(
            "key is not extractable".to_string(),
        ));
    }
    match format {
        KeyFormat::Raw => Ok(ExportedKey::Raw(key.material.as_bytes().to_vec())),
        KeyFormat::Jwk => Ok(ExportedKey::Jwk(match key.algorithm {
            KeyAlgorithm::Hmac { hash, .. } => jwk::export_oct_hmac(key, hash),
            KeyAlgorithm::Aes { variant, length } => jwk::export_oct_aes(key, variant, length),
        })),
        KeyFormat::Pkcs8 | KeyFormat::Spki => Err(AlgorithmError::NotSupported(
            "symmetric key export supports only the 'raw' and 'jwk' formats".to_string(),
        )),
    }
}

/// `sign` (WebCrypto §14.3.3 + §31 Sign).
#[allow(clippy::needless_pass_by_value)] // uniform ops signature; see `generate_key`
pub fn sign(
    algorithm: NormalizedAlgorithm,
    key: &CryptoKeyData,
    data: &[u8],
) -> Result<Vec<u8>, AlgorithmError> {
    require_key_usable(&algorithm, key, KeyUsage::Sign)?;
    match key.algorithm {
        KeyAlgorithm::Hmac { hash, .. } => Ok(hmac::sign(hash, key.material.as_bytes(), data)),
        // `sign` only normalizes HMAC, so the name-match above rejects any
        // non-HMAC key before reaching here.
        KeyAlgorithm::Aes { .. } => Err(not_supported_op("sign")),
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
        KeyAlgorithm::Aes { .. } => Err(not_supported_op("verify")),
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
    let material = key.material.as_bytes();
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
        // `encrypt` only normalizes the AES modes, so the name-match above
        // rejects any other key/algorithm before reaching here.
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
    let material = key.material.as_bytes();
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

const HMAC_USAGE_MSG: &str = "HMAC keys support only the 'sign' and 'verify' usages";
const AES_USAGE_MSG: &str =
    "AES keys support only the 'encrypt', 'decrypt', 'wrapKey' and 'unwrapKey' usages";

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
/// consistently, so this is a defensive `TypeError`.
fn format_data_mismatch() -> AlgorithmError {
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
