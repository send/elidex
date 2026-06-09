//! The §18.4.4 step-5 registry: the [`resolve_registry`] oracle that maps a
//! `(op, name)` pair to the IDL dictionary type it resolves to
//! ([`DesiredType`]), and the public [`params_shape`] / [`is_supported`]
//! views the VM marshalling layer consults. [`super::normalize`] routes
//! through the same oracle so membership and the normalized form cannot
//! drift.

use crate::hash::HashAlgorithm;

use super::names::{AesVariant, AlgorithmName, EcAlgorithm, RsaVariant};
use super::Operation;

/// The IDL dictionary type a recognized `(op, name)` pair resolves to
/// (§18.4.4 step 5 `desiredType`), plus the bits `normalize` needs to
/// build the result. This is the registry-membership oracle: a `Some`
/// means the pair is in `supportedAlgorithms[op]` (step 5 found a key),
/// a `None` means step 5 returns `NotSupportedError` before any
/// params-dictionary member is read.
///
/// Both [`crate::normalize`] and [`is_supported`] route through
/// [`resolve_registry`] so the two cannot drift: there is one place that
/// decides whether `(op, name)` is registered.
pub(super) enum DesiredType {
    /// `digest`: name-only `Algorithm` — the name fully determines the
    /// hash to compute.
    Digest(HashAlgorithm),
    /// `sign` / `verify` HMAC: name-only `Algorithm` (the hash comes from
    /// the key's `[[algorithm]]`).
    HmacSignVerify,
    /// `generateKey` / `importKey` / `getKeyLength` HMAC: an
    /// `HmacKeyGenParams` / `HmacImportParams` whose `hash` (required) and
    /// `length` (optional) members are read by step 6.
    HmacKeyParams,
    /// AES `generateKey` (`AesKeyGenParams`) or AES `get key length`
    /// (`AesDerivedKeyParams`) — both name a `length` (required
    /// `[EnforceRange] unsigned short`) read by step 6; the op (generate vs
    /// get-key-length) is the [`crate::ops`] entry, not the params shape.
    AesKeyGen(AesVariant),
    /// AES `importKey`: a name-only `Algorithm` (registration params =
    /// `None`); the key length derives from the imported material.
    AesImport(AesVariant),
    /// AES `encrypt` / `decrypt`: the mode's params dictionary
    /// (`AesGcmParams` / `AesCbcParams` / `AesCtrParams`).  Never carries
    /// `AesVariant::Kw` — AES-KW (§30) registers no encrypt/decrypt op, so
    /// [`resolve_registry`] filters it out of these pairs.
    AesEncryptDecrypt(AesVariant),
    /// AES-KW `wrapKey` / `unwrapKey` (WebCrypto §30.3.1 / §30.3.2): a
    /// name-only `Algorithm` — AES-KW takes no IV/nonce param (it uses the
    /// fixed RFC 3394 default IV), so the wrap algorithm carries nothing
    /// beyond `name`.
    AesKwWrap,
    /// HKDF / PBKDF2 name-only form (`importKey` + `get key length`): a
    /// name-only `Algorithm` (§33.4.2 / §34.4.2 import raw, §33.4.3 /
    /// §34.4.3 get-key-length null).
    KdfNameOnly(KdfKind),
    /// HKDF `deriveBits`: an `HkdfParams` (`hash` + `salt` + `info`, all
    /// required) read by step 6.
    HkdfDeriveBits,
    /// PBKDF2 `deriveBits`: a `Pbkdf2Params` (`salt` + `iterations` +
    /// `hash`, all required) read by step 6.
    Pbkdf2DeriveBits,
    /// EC `generateKey` (`EcKeyGenParams`, §23.4 / §24.4.1): a `namedCurve`
    /// (required `NamedCurve`) read by step 6.
    EcKeyGen(EcAlgorithm),
    /// EC `importKey` (`EcKeyImportParams`, §23.6 / §24.4.3): a `namedCurve`
    /// (required) read by step 6.
    EcImport(EcAlgorithm),
    /// ECDSA `sign` / `verify` (`EcdsaParams`, §23.3): a `hash` (required).
    EcdsaParams,
    /// ECDH `deriveBits` (`EcdhKeyDeriveParams`, §24.3): a `public` CryptoKey
    /// peer (required) — the novel CryptoKey-valued algorithm member.
    EcdhDerive,
    /// RSA `generateKey` (`RsaHashedKeyGenParams`, §20.4 / §21.4.3): a
    /// `modulusLength` (required `[EnforceRange] unsigned long`), a
    /// `publicExponent` (required `BigInteger`), and a `hash` (required).
    RsaKeyGen(RsaVariant),
    /// RSA `importKey` (`RsaHashedImportParams`, §20.7 / §21.4.4): a `hash`
    /// (required `HashAlgorithmIdentifier`).
    RsaImport(RsaVariant),
    /// RSASSA-PKCS1-v1_5 `sign` / `verify` (§20.8.1 / §20.8.2): a name-only
    /// `Algorithm` (the hash comes from the key's `[[algorithm]]`).
    RsassaParams,
    /// RSA-PSS `sign` / `verify` (`RsaPssParams`, §21.3): a `saltLength`
    /// (required `[EnforceRange] unsigned long`).
    RsaPssParams,
    /// RSA-OAEP `encrypt` / `decrypt` (`RsaOaepParams`, §22.3): an optional
    /// `label` (`BufferSource`).  §22.2 registers RSA-OAEP for `encrypt` /
    /// `decrypt` only — NOT `wrapKey` / `unwrapKey`, which carry no own op.
    /// A `wrapKey` / `unwrapKey` call therefore fails the §14.3.11 step-2 /
    /// §14.3.12 step-2 "normalize for wrapKey/unwrapKey" (NotSupported, before
    /// any `label` read) and falls back to normalizing for `encrypt` / `decrypt`
    /// (step 3), which resolves here — so the OAEP `label` is read exactly once,
    /// on the fallback, never twice.
    RsaOaepParams,
}

/// Which KDF a [`DesiredType::KdfNameOnly`] resolves to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum KdfKind {
    Hkdf,
    Pbkdf2,
}

/// §18.4.4 step 5: does `supportedAlgorithms[op]` contain a
/// case-insensitive match for `name`, and if so, which IDL dictionary
/// type does it resolve to? `None` ⇒ the spec returns `NotSupportedError`
/// at step 5, *before* the step-6 WebIDL conversion reads any
/// params-dictionary member.
pub(super) fn resolve_registry(op: Operation, name: &str) -> Option<DesiredType> {
    let name = AlgorithmName::recognize(name)?;
    match (op, name) {
        (Operation::Digest, _) => name.as_hash().map(DesiredType::Digest),
        (Operation::Sign | Operation::Verify, AlgorithmName::Hmac) => {
            Some(DesiredType::HmacSignVerify)
        }
        (
            Operation::GenerateKey | Operation::ImportKey | Operation::GetKeyLength,
            AlgorithmName::Hmac,
        ) => Some(DesiredType::HmacKeyParams),
        // HKDF / PBKDF2 (WebCrypto §33 / §34): import (raw) + deriveBits +
        // get-key-length (null).  No generateKey / encrypt / export — those
        // fall through to the AES catch-alls below, where `as_aes()` returns
        // `None` for a KDF name, so they resolve to `None` (NotSupported).
        (Operation::ImportKey | Operation::GetKeyLength, AlgorithmName::Hkdf) => {
            Some(DesiredType::KdfNameOnly(KdfKind::Hkdf))
        }
        (Operation::ImportKey | Operation::GetKeyLength, AlgorithmName::Pbkdf2) => {
            Some(DesiredType::KdfNameOnly(KdfKind::Pbkdf2))
        }
        (Operation::DeriveBits, AlgorithmName::Hkdf) => Some(DesiredType::HkdfDeriveBits),
        (Operation::DeriveBits, AlgorithmName::Pbkdf2) => Some(DesiredType::Pbkdf2DeriveBits),
        // AES-KW (WebCrypto §30): wrapKey / unwrapKey take a name-only
        // algorithm (the fixed RFC 3394 default IV — no params).  Its
        // generateKey / importKey / get-key-length share the AES catch-alls
        // below (`as_aes()` maps "AES-KW" → `AesVariant::Kw`).
        (Operation::WrapKey | Operation::UnwrapKey, AlgorithmName::AesKw) => {
            Some(DesiredType::AesKwWrap)
        }
        // ECDSA / ECDH (WebCrypto §23 / §24).  generateKey + importKey carry
        // only `namedCurve` (EcKeyGenParams §23.4 / EcKeyImportParams §23.6);
        // ECDSA adds sign / verify (EcdsaParams §23.3) and ECDH adds
        // deriveBits (EcdhKeyDeriveParams §24.3).  Neither registers
        // get-key-length (§23.2 / §24.2): a `(GetKeyLength, Ecdsa|Ecdh)` pair
        // falls to the AES catch-all below where `as_aes()` returns `None`, so
        // it resolves to NotSupported.  These arms precede the AES catch-alls
        // (and the `(ImportKey, _)` catch-all) so an EC name never resolves to
        // an AES desiredType.
        (Operation::GenerateKey, AlgorithmName::Ecdsa) => {
            Some(DesiredType::EcKeyGen(EcAlgorithm::Ecdsa))
        }
        (Operation::GenerateKey, AlgorithmName::Ecdh) => {
            Some(DesiredType::EcKeyGen(EcAlgorithm::Ecdh))
        }
        (Operation::ImportKey, AlgorithmName::Ecdsa) => {
            Some(DesiredType::EcImport(EcAlgorithm::Ecdsa))
        }
        (Operation::ImportKey, AlgorithmName::Ecdh) => {
            Some(DesiredType::EcImport(EcAlgorithm::Ecdh))
        }
        (Operation::Sign | Operation::Verify, AlgorithmName::Ecdsa) => {
            Some(DesiredType::EcdsaParams)
        }
        (Operation::DeriveBits, AlgorithmName::Ecdh) => Some(DesiredType::EcdhDerive),
        // RSASSA-PKCS1-v1_5 / RSA-PSS (WebCrypto §20 / §21).  generateKey +
        // importKey carry the RsaHashed key params (RsaHashedKeyGenParams §20.4
        // / RsaHashedImportParams §20.7); RSASSA sign / verify are name-only
        // (§20.8.1 / §20.8.2) while RSA-PSS adds RsaPssParams (§21.3).  Neither
        // registers get-key-length (§20.2): a `(GetKeyLength, Rsassa|RsaPss)`
        // pair falls to the AES catch-all below where `as_aes()` returns `None`
        // → NotSupported.  These arms precede the AES catch-alls (and the
        // `(ImportKey, _)` catch-all) so an RSA name never resolves to an AES
        // desiredType.
        (Operation::GenerateKey, AlgorithmName::RsassaPkcs1V15) => {
            Some(DesiredType::RsaKeyGen(RsaVariant::RsassaPkcs1V15))
        }
        (Operation::GenerateKey, AlgorithmName::RsaPss) => {
            Some(DesiredType::RsaKeyGen(RsaVariant::RsaPss))
        }
        (Operation::ImportKey, AlgorithmName::RsassaPkcs1V15) => {
            Some(DesiredType::RsaImport(RsaVariant::RsassaPkcs1V15))
        }
        (Operation::ImportKey, AlgorithmName::RsaPss) => {
            Some(DesiredType::RsaImport(RsaVariant::RsaPss))
        }
        (Operation::Sign | Operation::Verify, AlgorithmName::RsassaPkcs1V15) => {
            Some(DesiredType::RsassaParams)
        }
        (Operation::Sign | Operation::Verify, AlgorithmName::RsaPss) => {
            Some(DesiredType::RsaPssParams)
        }
        // RSA-OAEP (WebCrypto §22).  generateKey + importKey reuse the RsaHashed
        // key params (RsaHashedKeyGenParams §20.4 / RsaHashedImportParams §20.7,
        // reused by §22); encrypt / decrypt take the optional RsaOaepParams.label
        // (§22.3).  §22.2 registers ONLY these — NOT wrapKey / unwrapKey (and no
        // get-key-length / sign / verify): a `(WrapKey | UnwrapKey, RSA-OAEP)`
        // pair is therefore unregistered here (`None` = NotSupported), so the VM's
        // §14.3.11 step-2 / §14.3.12 step-2 "normalize for wrapKey/unwrapKey" fails
        // (before reading `label`) and falls back to normalizing for encrypt /
        // decrypt (step 3) — which DOES resolve, reading `label` exactly once.
        // These arms precede the AES catch-alls (and the `(ImportKey, _)` /
        // `(Encrypt | Decrypt, _)` catch-alls) so an RSA-OAEP name never resolves
        // to an AES desiredType.
        (Operation::GenerateKey, AlgorithmName::RsaOaep) => {
            Some(DesiredType::RsaKeyGen(RsaVariant::RsaOaep))
        }
        (Operation::ImportKey, AlgorithmName::RsaOaep) => {
            Some(DesiredType::RsaImport(RsaVariant::RsaOaep))
        }
        (Operation::Encrypt | Operation::Decrypt, AlgorithmName::RsaOaep) => {
            Some(DesiredType::RsaOaepParams)
        }
        // AES generateKey / get-key-length both read a `length`-only dict
        // (`AesKeyGenParams` / `AesDerivedKeyParams`); `as_aes()` filters the
        // non-AES names (HMAC handled above, KDF handled above, SHA → None)
        // and admits all four AES variants incl. AES-KW (§30.3.3 / §30.3.6).
        (Operation::GenerateKey | Operation::GetKeyLength, _) => {
            name.as_aes().map(DesiredType::AesKeyGen)
        }
        (Operation::ImportKey, _) => name.as_aes().map(DesiredType::AesImport),
        // encrypt / decrypt: only the three block-cipher modes — AES-KW (§30)
        // is wrap-only and registers no encrypt/decrypt op, so it must stay
        // unregistered here (returns NotSupported), NOT fall to AesEncryptDecrypt.
        (Operation::Encrypt | Operation::Decrypt, _) => name
            .as_aes()
            .filter(|variant| !matches!(variant, AesVariant::Kw))
            .map(DesiredType::AesEncryptDecrypt),
        _ => None,
    }
}

/// The params-dictionary members the VM must read for a recognized
/// `(op, name)` pair (WebCrypto §18.4.4 step 6 "convert `alg` to the IDL
/// dictionary"), so the registry — not the VM marshalling layer — owns
/// which members each operation consults.  Returned by [`params_shape`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlgorithmParams {
    /// No params-dictionary members beyond `name` (digest / sign / verify /
    /// AES importKey).
    NameOnly,
    /// HMAC generateKey / importKey: `hash` (required) + `length` (optional
    /// `unsigned long`).
    HmacKeyParams,
    /// AES generateKey: `length` (required `[EnforceRange] unsigned short`).
    AesKeyGen,
    /// AES-GCM encrypt / decrypt: `iv` (required), `additionalData`
    /// (optional), `tagLength` (optional `[EnforceRange] octet`).
    AesGcmParams,
    /// AES-CBC encrypt / decrypt: `iv` (required `BufferSource`).
    AesCbcParams,
    /// AES-CTR encrypt / decrypt: `counter` (required `BufferSource`) +
    /// `length` (required `[EnforceRange] octet`).
    AesCtrParams,
    /// HKDF deriveBits (`HkdfParams`): `hash` (required), `info` (required
    /// `BufferSource`), `salt` (required `BufferSource`).
    HkdfParams,
    /// PBKDF2 deriveBits (`Pbkdf2Params`): `hash` (required), `iterations`
    /// (required `[EnforceRange] unsigned long`), `salt` (required
    /// `BufferSource`).
    Pbkdf2Params,
    /// EC generateKey / importKey (`EcKeyGenParams` §23.4 / `EcKeyImportParams`
    /// §23.6): `namedCurve` (required `NamedCurve` = DOMString).
    EcKeyGen,
    /// ECDSA sign / verify (`EcdsaParams` §23.3): `hash` (required
    /// `HashAlgorithmIdentifier`).
    EcdsaParams,
    /// ECDH deriveBits (`EcdhKeyDeriveParams` §24.3): `public` (required
    /// `CryptoKey` — the peer public key; the novel CryptoKey-valued member).
    EcdhKeyDeriveParams,
    /// RSA generateKey (`RsaHashedKeyGenParams` §20.4 / §21.4.3):
    /// `modulusLength` (required `[EnforceRange] unsigned long`),
    /// `publicExponent` (required `BigInteger` = `Uint8Array`), `hash`
    /// (required `HashAlgorithmIdentifier`).
    RsaHashedKeyGen,
    /// RSA importKey (`RsaHashedImportParams` §20.7 / §21.4.4): `hash`
    /// (required `HashAlgorithmIdentifier`).
    RsaHashedImport,
    /// RSA-PSS sign / verify (`RsaPssParams` §21.3): `saltLength` (required
    /// `[EnforceRange] unsigned long`).
    RsaPssParams,
    /// RSA-OAEP encrypt / decrypt (`RsaOaepParams` §22.3): `label` (optional
    /// `BufferSource`).  §22.2 registers RSA-OAEP for encrypt / decrypt only;
    /// wrapKey / unwrapKey reach this shape via the §14.3.11 / §14.3.12 encrypt /
    /// decrypt fallback, not a direct registration.
    RsaOaepParams,
}

/// §18.4.4 step 5 + step-6 member plan: for a registered `(op, name)` pair
/// return which params-dictionary members the VM should read; `None` ⇒ the
/// pair is unregistered, so the spec returns `NotSupportedError` at step 5
/// *before* any getter fires.  The VM marshalling layer routes through this
/// (rather than re-deriving the dictionary shape from the name string), so
/// the registry stays the single source of truth and an unregistered name
/// never triggers a user-defined member getter.
pub fn params_shape(op: Operation, name: &str) -> Option<AlgorithmParams> {
    resolve_registry(op, name).map(|d| match d {
        DesiredType::Digest(_)
        | DesiredType::HmacSignVerify
        | DesiredType::AesImport(_)
        | DesiredType::KdfNameOnly(_)
        | DesiredType::AesKwWrap
        | DesiredType::RsassaParams => AlgorithmParams::NameOnly,
        DesiredType::HmacKeyParams => AlgorithmParams::HmacKeyParams,
        DesiredType::AesKeyGen(_) => AlgorithmParams::AesKeyGen,
        DesiredType::AesEncryptDecrypt(variant) => match variant {
            AesVariant::Gcm => AlgorithmParams::AesGcmParams,
            AesVariant::Cbc => AlgorithmParams::AesCbcParams,
            AesVariant::Ctr => AlgorithmParams::AesCtrParams,
            // `resolve_registry` never builds `AesEncryptDecrypt(Kw)`.
            AesVariant::Kw => unreachable!("AES-KW has no encrypt/decrypt params"),
        },
        DesiredType::HkdfDeriveBits => AlgorithmParams::HkdfParams,
        DesiredType::Pbkdf2DeriveBits => AlgorithmParams::Pbkdf2Params,
        // EC generateKey + importKey share `EcKeyGenParams` / `EcKeyImportParams`
        // (both name-only `namedCurve`); the op (generate vs import) is the
        // `crate::ops` entry, not the params shape.
        DesiredType::EcKeyGen(_) | DesiredType::EcImport(_) => AlgorithmParams::EcKeyGen,
        DesiredType::EcdsaParams => AlgorithmParams::EcdsaParams,
        DesiredType::EcdhDerive => AlgorithmParams::EcdhKeyDeriveParams,
        // RSA generateKey + importKey carry distinct dictionaries
        // (`RsaHashedKeyGenParams` §20.4 has modulusLength + publicExponent +
        // hash; `RsaHashedImportParams` §20.7 has only hash).  RSA-PSS sign /
        // verify read `saltLength`; RSASSA sign / verify are name-only
        // (folded into the `NameOnly` group above).
        DesiredType::RsaKeyGen(_) => AlgorithmParams::RsaHashedKeyGen,
        DesiredType::RsaImport(_) => AlgorithmParams::RsaHashedImport,
        DesiredType::RsaPssParams => AlgorithmParams::RsaPssParams,
        DesiredType::RsaOaepParams => AlgorithmParams::RsaOaepParams,
    })
}

/// §18.4.4 step 5 as a predicate: is `(op, name)` a registered pair?
/// (`params_shape(op, name).is_some()`.)
pub fn is_supported(op: Operation, name: &str) -> bool {
    params_shape(op, name).is_some()
}
