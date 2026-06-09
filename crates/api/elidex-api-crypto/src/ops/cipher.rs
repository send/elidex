//! The encrypt / decrypt / wrapKey / unwrapKey operation entry points
//! (WebCrypto §14.3.1 / §14.3.2 / §14.3.11 / §14.3.12) + their algorithm
//! dispatch.  Split from [`super`] (the operation-level boundary) so the
//! cipher op-set + the generalized key-algorithm dispatch (AES block-cipher
//! modes vs the RSA-OAEP `rsa::oaep` backend) stay one cohesive unit.
//!
//! The §14.3.x name / usage gate runs in the public entries; the `*_op`
//! helpers are the gate-free operation dispatch the wrapKey / unwrapKey
//! fallbacks reuse.  Every gate + the dispatch live here so the VM host only
//! marshals.

use crate::algorithm::{NormalizedAlgorithm, RsaVariant};
use crate::error::AlgorithmError;
use crate::key::{CryptoKeyData, KeyAlgorithm, KeyUsage};
use crate::{aes, aes_kw, jwk};

use super::{export_key, not_supported_op, require_key_usable, ExportedKey, KeyFormat};

/// `encrypt` (WebCrypto §14.3.1 → §27.7.1 / §28.4.1 / §29.4.1 AES /
/// §22.4.1 RSA-OAEP).  Consumes the normalized params, moving the `iv` /
/// `counter` / `additionalData` / `label` out and passing them straight to the
/// cipher (no copy beyond the marshal-time snapshot).
pub fn encrypt(
    algorithm: NormalizedAlgorithm,
    key: &CryptoKeyData,
    data: &[u8],
) -> Result<Vec<u8>, AlgorithmError> {
    require_key_usable(&algorithm, key, KeyUsage::Encrypt)?;
    encrypt_op(algorithm, key, data)
}

/// The encrypt *operation* dispatch (no §14.3.1 name/usage gate) — shared by
/// [`encrypt`] (after its gate) and the [`wrap_key`] §14.3.11 step-15 encrypt
/// fallback (whose gate is the `wrapKey` usage, not `encrypt`).
///
/// Branches on the key's algorithm and takes the **key** — NOT a pre-extracted
/// `as_bytes()`: an RSA key has no flat byte form (`as_bytes()` would panic,
/// `key.rs`), so the byte extraction must live inside the AES arm where it is
/// reachable.  The AES block-cipher modes read the flat material; RSA-OAEP
/// (§22.4.1) reconstructs the typed key from the stored DER in the `rsa::oaep`
/// backend, which also owns the §22.4.1 step-1 `[[type]]` = public
/// InvalidAccessError gate — so every entry point reaching here (`encrypt` /
/// `wrapKey`) inherits the correct gate without a per-entry type check.
fn encrypt_op(
    algorithm: NormalizedAlgorithm,
    key: &CryptoKeyData,
    data: &[u8],
) -> Result<Vec<u8>, AlgorithmError> {
    match key.algorithm {
        KeyAlgorithm::Aes { .. } => aes_encrypt_op(algorithm, key.material.as_bytes(), data),
        KeyAlgorithm::Rsa {
            variant: RsaVariant::RsaOaep,
            hash,
            ..
        } => {
            // The name-match in the caller admitted this RSA-OAEP key, so
            // `encrypt` / `wrapKey` normalized to `RsaOaep` (the only OAEP form);
            // its optional `label` rides into the backend by reference.
            let NormalizedAlgorithm::RsaOaep { label } = algorithm else {
                return Err(not_supported_op("encrypt"));
            };
            crate::rsa::oaep_encrypt(key, hash, label.as_deref(), data)
        }
        // `encrypt` normalizes only the AES block-cipher modes + RSA-OAEP, so
        // the name-match in the caller rejects any other key before reaching
        // here (an RSA signing key / AES-KW key never registers an encrypt op).
        KeyAlgorithm::Hmac { .. }
        | KeyAlgorithm::Hkdf
        | KeyAlgorithm::Pbkdf2
        | KeyAlgorithm::Ecdsa { .. }
        | KeyAlgorithm::Ecdh { .. }
        | KeyAlgorithm::Rsa { .. } => Err(not_supported_op("encrypt")),
    }
}

/// The AES encrypt *operation* dispatch — the AES-key arm of [`encrypt_op`];
/// the caller has already validated the key length to 16/24/32 bytes.
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

/// `decrypt` (WebCrypto §14.3.2 → §27.7.2 / §28.4.2 / §29.4.2 AES /
/// §22.4.2 RSA-OAEP).
pub fn decrypt(
    algorithm: NormalizedAlgorithm,
    key: &CryptoKeyData,
    data: &[u8],
) -> Result<Vec<u8>, AlgorithmError> {
    require_key_usable(&algorithm, key, KeyUsage::Decrypt)?;
    decrypt_op(algorithm, key, data)
}

/// The decrypt *operation* dispatch (no §14.3.2 name/usage gate) — shared by
/// [`decrypt`] (after its gate) and the [`unwrap_key`] §14.3.12 step-14 decrypt
/// fallback (whose gate is the `unwrapKey` usage, not `decrypt`).  The mirror of
/// [`encrypt_op`]: takes the key (not `as_bytes()`), branches on the key
/// algorithm, and RSA-OAEP (§22.4.2) reconstructs the private key from the
/// stored PKCS#8 DER in the **constant-time** `rsa::oaep` backend, which owns
/// the §22.4.2 step-1 `[[type]]` = private InvalidAccessError gate (inherited by
/// both `decrypt` and `unwrapKey`).
fn decrypt_op(
    algorithm: NormalizedAlgorithm,
    key: &CryptoKeyData,
    data: &[u8],
) -> Result<Vec<u8>, AlgorithmError> {
    match key.algorithm {
        KeyAlgorithm::Aes { .. } => aes_decrypt_op(algorithm, key.material.as_bytes(), data),
        KeyAlgorithm::Rsa {
            variant: RsaVariant::RsaOaep,
            hash,
            ..
        } => {
            let NormalizedAlgorithm::RsaOaep { label } = algorithm else {
                return Err(not_supported_op("decrypt"));
            };
            crate::rsa::oaep_decrypt(key, hash, label.as_deref(), data)
        }
        KeyAlgorithm::Hmac { .. }
        | KeyAlgorithm::Hkdf
        | KeyAlgorithm::Pbkdf2
        | KeyAlgorithm::Ecdsa { .. }
        | KeyAlgorithm::Ecdh { .. }
        | KeyAlgorithm::Rsa { .. } => Err(not_supported_op("decrypt")),
    }
}

/// The AES decrypt *operation* dispatch — the AES-key arm of [`decrypt_op`].
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
    // step 15: wrap (AES-KW) or fall back to the encrypt op (AES-GCM/CBC/CTR or
    // RSA-OAEP — the generalized [`encrypt_op`] routes by the wrapping key's
    // algorithm and inherits RSA-OAEP's `[[type]]` = public gate).
    match algorithm {
        NormalizedAlgorithm::AesKwWrap => aes_kw::wrap(wrapping_key.material.as_bytes(), &bytes),
        _ => encrypt_op(algorithm, wrapping_key, &bytes),
    }
}

/// `unwrapKey` (WebCrypto §14.3.12) decrypt half: the name-equality (step 12)
/// and `unwrapKey`-usage (step 13) gate on `unwrapping_key`, then the unwrap
/// (or decrypt-fallback) operation (step 14) yielding the wrapped key's bytes.
///
/// The VM completes the method: "parse a JWK" (step 15, for `jwk` format) and
/// the `importKey` of the bytes (step 16, via [`super::import_key`], which also
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
    // step 14: unwrap (AES-KW) or fall back to the decrypt op (AES-GCM/CBC/CTR or
    // RSA-OAEP — the generalized [`decrypt_op`], constant-time for RSA-OAEP and
    // inheriting its `[[type]]` = private gate).
    match algorithm {
        NormalizedAlgorithm::AesKwWrap => {
            aes_kw::unwrap(unwrapping_key.material.as_bytes(), wrapped_key)
        }
        _ => decrypt_op(algorithm, unwrapping_key, wrapped_key),
    }
}
