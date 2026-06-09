//! RSA-OAEP encrypt / decrypt (WebCrypto §22.4.1 / §22.4.2, RFC 3447 §7.1
//! RSAES-OAEP), on the **constant-time aws-lc-rs** (AWS-LC) backend.
//!
//! Split from the `rsa`-crate signing / key-management backend ([`super`])
//! because the OAEP op-set runs on aws-lc-rs rather than the `rsa` crate for
//! two reasons:
//!
//! 1. **Constant-time decryption.**  `decrypt` / `unwrapKey` is the private-key
//!    op RUSTSEC-2023-0071 (Marvin) targets — a chosen-ciphertext timing oracle
//!    a malicious same-origin script could use to recover a non-extractable
//!    private key.  aws-lc-rs (a FIPS-family BoringSSL fork) performs the
//!    private-key exponentiation + OAEP unpadding in constant time; the pure-Rust
//!    `rsa` crate carries Marvin on its decryption and ships no constant-time
//!    decrypt ([`super::sign`]'s blinding has no rsa-crate *decryption*
//!    analogue).  This is the SOLE aws-lc-rs call site, isolated so a future swap
//!    back to a constant-time pure-Rust `rsa` (RustCrypto/RSA#390) touches only
//!    this file.
//! 2. **Arbitrary-byte labels.**  WebCrypto's `RsaOaepParams.label` (§22.3) is a
//!    `BufferSource` (arbitrary octets); the `rsa` crate models the OAEP label as
//!    a UTF-8 `String` (`rsa::Oaep.label: Option<String>`), so it cannot carry a
//!    non-UTF-8 label.  aws-lc-rs takes `Option<&[u8]>`, so the whole OAEP op-set
//!    is faithful (and `encrypt` ↔ `decrypt` share one OAEP implementation =
//!    guaranteed interop).  `encrypt` is a public-key op (Marvin-safe on either
//!    backend); it runs here only to share that implementation + the byte label.
//!
//! Both ops reconstruct the typed key from the canonical DER [`super`] stores
//! ([`crate::key::KeyMaterial::Rsa`]) — the cross-backend bridge, so the split
//! needs no new key state (the asymmetric analogue of `Raw(bytes)` → cipher).

use aws_lc_rs::rsa::{
    OaepAlgorithm, OaepPrivateDecryptingKey, OaepPublicEncryptingKey, PrivateDecryptingKey,
    PublicEncryptingKey, OAEP_SHA1_MGF1SHA1, OAEP_SHA256_MGF1SHA256, OAEP_SHA384_MGF1SHA384,
    OAEP_SHA512_MGF1SHA512,
};

use crate::error::AlgorithmError;
use crate::hash::HashAlgorithm;
use crate::key::CryptoKeyData;

use super::{key_inaccessible, operation, require_public, rsa_private_der, rsa_public_der};

/// RSA-OAEP `encrypt` (WebCrypto §22.4.1, RFC 3447 §7.1 RSAES-OAEP-ENCRYPT):
/// OAEP-pad `plaintext` (the key's `hash` is both the label digest and the MGF1
/// hash) then apply RSAEP under the public half.  §22.4.1 step 1 ([[type]] must
/// be public → InvalidAccessError) is enforced HERE via [`require_public`] — the
/// stored SPKI DER is present even on a private key, so without this gate a
/// private key would silently encrypt — NOT in [`crate::ops::encrypt`], whose
/// `require_key_usable` checks only name + usage.  Plaintext longer than the
/// OAEP limit (`k − 2·hLen − 2`) → OperationError (aws-lc-rs returns Err).
pub(crate) fn oaep_encrypt(
    key: &CryptoKeyData,
    hash: HashAlgorithm,
    label: Option<&[u8]>,
    plaintext: &[u8],
) -> Result<Vec<u8>, AlgorithmError> {
    require_public(key)?;
    // Reconstruct from this crate's own canonical SPKI DER, so a parse failure
    // is the "key material cannot be accessed" OperationError, not a DataError.
    let public_key =
        PublicEncryptingKey::from_der(rsa_public_der(key)).map_err(|_| key_inaccessible())?;
    let oaep_key = OaepPublicEncryptingKey::new(public_key).map_err(|_| key_inaccessible())?;
    let mut out = vec![0u8; oaep_key.ciphertext_size()];
    let ciphertext = oaep_key
        .encrypt(
            oaep_algorithm(hash),
            plaintext,
            &mut out,
            normalize_label(label),
        )
        .map_err(|_| operation("RSA-OAEP encryption failed"))?;
    Ok(ciphertext.to_vec())
}

/// RSA-OAEP `decrypt` (WebCrypto §22.4.2, RFC 3447 §7.1 RSAES-OAEP-DECRYPT):
/// RSADP under the private half then OAEP-decode.  §22.4.2 step 1 ([[type]] must
/// be private → InvalidAccessError) is enforced HERE via the stored PKCS#8 DER
/// ([`rsa_private_der`] → `None` ⇒ InvalidAccessError on a public-only key), NOT
/// in [`crate::ops::decrypt`] (name + usage only).  **The sole constant-time
/// CT-bridge site** (see the module docs): malformed ciphertext / wrong label /
/// wrong size → OperationError (aws-lc-rs returns Err; never a panic), so the
/// Marvin chosen-ciphertext oracle the advisory warns about is closed.
pub(crate) fn oaep_decrypt(
    key: &CryptoKeyData,
    hash: HashAlgorithm,
    label: Option<&[u8]>,
    ciphertext: &[u8],
) -> Result<Vec<u8>, AlgorithmError> {
    let pkcs8_der = rsa_private_der(key)?;
    let private_key =
        PrivateDecryptingKey::from_pkcs8(pkcs8_der).map_err(|_| key_inaccessible())?;
    let oaep_key = OaepPrivateDecryptingKey::new(private_key).map_err(|_| key_inaccessible())?;
    let mut out = vec![0u8; oaep_key.min_output_size()];
    let plaintext = oaep_key
        .decrypt(
            oaep_algorithm(hash),
            ciphertext,
            &mut out,
            normalize_label(label),
        )
        .map_err(|_| operation("RSA-OAEP decryption failed"))?;
    Ok(plaintext.to_vec())
}

/// The aws-lc-rs OAEP algorithm for `hash`.  WebCrypto RSA-OAEP uses the key's
/// `hash` for BOTH the OAEP label digest and MGF1 (§22.4.1 / §22.4.2 reference
/// the key's `hash` for the whole RSAES-OAEP scheme), which maps cleanly to
/// aws-lc-rs's `OAEP_SHA*_MGF1SHA*` (same hash on both legs).
fn oaep_algorithm(hash: HashAlgorithm) -> &'static OaepAlgorithm {
    match hash {
        HashAlgorithm::Sha1 => &OAEP_SHA1_MGF1SHA1,
        HashAlgorithm::Sha256 => &OAEP_SHA256_MGF1SHA256,
        HashAlgorithm::Sha384 => &OAEP_SHA384_MGF1SHA384,
        HashAlgorithm::Sha512 => &OAEP_SHA512_MGF1SHA512,
    }
}

/// Normalize the OAEP label: an absent OR empty label is the RFC 3447 §7.1
/// default `L = ""` (so `lHash = Hash("")`).  Collapsing `Some(&[])` to `None`
/// routes "no label" and "empty label" through one path, so they are
/// byte-identical regardless of how aws-lc-rs treats `Some(&[])` — matching the
/// WebCrypto semantic that an absent `label` member equals an empty one.
fn normalize_label(label: Option<&[u8]>) -> Option<&[u8]> {
    label.filter(|l| !l.is_empty())
}
