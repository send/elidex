//! The recognized-algorithm + variant vocabulary: the canonical
//! [`AlgorithmName`] registry plus the per-family discriminators
//! ([`AesVariant`] / [`NamedCurve`] / [`EcAlgorithm`] / [`RsaVariant`]).
//! `recognize` / `as_hash` / `as_aes` are `pub(super)` so the
//! [`super::registry`] resolver and the [`super::normalize`] procedure can
//! consult them while the rest of the crate sees only the public types.

use crate::hash::HashAlgorithm;
use crate::key::KeyAlgorithm;

/// A canonical recognized algorithm name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlgorithmName {
    Sha1,
    Sha256,
    Sha384,
    Sha512,
    Hmac,
    AesCtr,
    AesCbc,
    AesGcm,
    /// HKDF (WebCrypto §33) — `importKey` (raw), `deriveBits`, and
    /// `get key length` (§33.4.3 → null, consumed by `deriveKey`).
    Hkdf,
    /// PBKDF2 (WebCrypto §34) — `importKey` (raw), `deriveBits`, and
    /// `get key length` (§34.4.3 → null, consumed by `deriveKey`).
    Pbkdf2,
    /// AES-KW (WebCrypto §30) — `generateKey` / `importKey` / `exportKey` /
    /// `wrapKey` / `unwrapKey` / `get key length`.  It is a key-wrap-only
    /// cipher: it registers no `encrypt` / `decrypt` operation.
    AesKw,
    /// ECDSA (WebCrypto §23) — `generateKey` / `importKey` / `exportKey` /
    /// `sign` / `verify`.  Asymmetric: no `get key length` (§23.2).
    Ecdsa,
    /// ECDH (WebCrypto §24) — `generateKey` / `importKey` / `exportKey` /
    /// `deriveBits` / `deriveKey`.  No `sign` / `verify` / `get key length`
    /// (§24.2).
    Ecdh,
    /// RSASSA-PKCS1-v1_5 (WebCrypto §20) — `generateKey` / `importKey` /
    /// `exportKey` / `sign` / `verify`.  Asymmetric: no `get key length`
    /// (§20.2).  The signature `hash` rides on the key
    /// (`RsaHashedKeyAlgorithm`, §20.6), and sign / verify take name-only
    /// params (no per-call dictionary).
    RsassaPkcs1V15,
    /// RSA-PSS (WebCrypto §21) — the same op-set as RSASSA-PKCS1-v1_5; sign
    /// / verify add only the `RsaPssParams.saltLength` (§21.3).
    RsaPss,
}

impl AlgorithmName {
    /// Recognize a name ASCII case-insensitively (§18.4.4 step:
    /// case-insensitive match against registered names).
    pub(super) fn recognize(name: &str) -> Option<Self> {
        if name.eq_ignore_ascii_case("SHA-1") {
            Some(Self::Sha1)
        } else if name.eq_ignore_ascii_case("SHA-256") {
            Some(Self::Sha256)
        } else if name.eq_ignore_ascii_case("SHA-384") {
            Some(Self::Sha384)
        } else if name.eq_ignore_ascii_case("SHA-512") {
            Some(Self::Sha512)
        } else if name.eq_ignore_ascii_case("HMAC") {
            Some(Self::Hmac)
        } else if name.eq_ignore_ascii_case("AES-CTR") {
            Some(Self::AesCtr)
        } else if name.eq_ignore_ascii_case("AES-CBC") {
            Some(Self::AesCbc)
        } else if name.eq_ignore_ascii_case("AES-GCM") {
            Some(Self::AesGcm)
        } else if name.eq_ignore_ascii_case("HKDF") {
            Some(Self::Hkdf)
        } else if name.eq_ignore_ascii_case("PBKDF2") {
            Some(Self::Pbkdf2)
        } else if name.eq_ignore_ascii_case("AES-KW") {
            Some(Self::AesKw)
        } else if name.eq_ignore_ascii_case("ECDSA") {
            Some(Self::Ecdsa)
        } else if name.eq_ignore_ascii_case("ECDH") {
            Some(Self::Ecdh)
        } else if name.eq_ignore_ascii_case("RSASSA-PKCS1-v1_5") {
            Some(Self::RsassaPkcs1V15)
        } else if name.eq_ignore_ascii_case("RSA-PSS") {
            Some(Self::RsaPss)
        } else {
            None
        }
    }

    pub(super) fn as_hash(self) -> Option<HashAlgorithm> {
        match self {
            Self::Sha1 => Some(HashAlgorithm::Sha1),
            Self::Sha256 => Some(HashAlgorithm::Sha256),
            Self::Sha384 => Some(HashAlgorithm::Sha384),
            Self::Sha512 => Some(HashAlgorithm::Sha512),
            Self::Hmac
            | Self::AesCtr
            | Self::AesCbc
            | Self::AesGcm
            | Self::AesKw
            | Self::Hkdf
            | Self::Pbkdf2
            | Self::Ecdsa
            | Self::Ecdh
            | Self::RsassaPkcs1V15
            | Self::RsaPss => None,
        }
    }

    /// The AES variant for this name (CTR / CBC / GCM / KW), or `None` for a
    /// non-AES name.  The three block-cipher modes participate in `encrypt` /
    /// `decrypt`; AES-KW (§30) is wrap-only, so the registry filters it out of
    /// the `encrypt` / `decrypt` pairs.
    pub(super) fn as_aes(self) -> Option<AesVariant> {
        match self {
            Self::AesCtr => Some(AesVariant::Ctr),
            Self::AesCbc => Some(AesVariant::Cbc),
            Self::AesGcm => Some(AesVariant::Gcm),
            Self::AesKw => Some(AesVariant::Kw),
            Self::Sha1
            | Self::Sha256
            | Self::Sha384
            | Self::Sha512
            | Self::Hmac
            | Self::Hkdf
            | Self::Pbkdf2
            | Self::Ecdsa
            | Self::Ecdh
            | Self::RsassaPkcs1V15
            | Self::RsaPss => None,
        }
    }
}

/// The four AES key kinds.  CTR / CBC / GCM (WebCrypto §27 / §28 / §29) are
/// the block-cipher modes that support `encrypt` / `decrypt`; KW (§30 AES-KW)
/// is a key-wrap-only cipher supporting `wrapKey` / `unwrapKey` (and **no**
/// `encrypt` / `decrypt`).  All four share `generateKey` / `importKey` /
/// `exportKey` / `get key length`, so the variant is the single discriminator
/// across the normalized generate/import forms, the key's
/// [`KeyAlgorithm`], and the JWK `alg` mapping —
/// dispatch stays typed rather than stringly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AesVariant {
    Ctr,
    Cbc,
    Gcm,
    Kw,
}

impl AesVariant {
    /// The canonical WebCrypto algorithm name (`"AES-GCM"` etc.) for the
    /// key's `[[algorithm]]` `name` attribute and the JWK `alg` mapping.
    pub fn canonical_name(self) -> &'static str {
        match self {
            Self::Ctr => "AES-CTR",
            Self::Cbc => "AES-CBC",
            Self::Gcm => "AES-GCM",
            Self::Kw => "AES-KW",
        }
    }

    pub(crate) fn algorithm_name(self) -> AlgorithmName {
        match self {
            Self::Ctr => AlgorithmName::AesCtr,
            Self::Cbc => AlgorithmName::AesCbc,
            Self::Gcm => AlgorithmName::AesGcm,
            Self::Kw => AlgorithmName::AesKw,
        }
    }

    /// The JWK `alg` value for an AES key of `length_bits` bits in this mode:
    /// the `alg` set by the AES import algorithms (WebCrypto §27.7.4 /
    /// §28.4.4 / §29.4.4 / §30.3.4) and emitted by the export algorithms
    /// (§27.7.5 / §28.4.5 / §29.4.5 / §30.3.5) — `A128GCM` / `A192CBC` /
    /// `A256KW` …, or `None` for a non-AES key length.
    pub fn jwk_alg(self, length_bits: u32) -> Option<&'static str> {
        Some(match (length_bits, self) {
            (128, Self::Ctr) => "A128CTR",
            (128, Self::Cbc) => "A128CBC",
            (128, Self::Gcm) => "A128GCM",
            (128, Self::Kw) => "A128KW",
            (192, Self::Ctr) => "A192CTR",
            (192, Self::Cbc) => "A192CBC",
            (192, Self::Gcm) => "A192GCM",
            (192, Self::Kw) => "A192KW",
            (256, Self::Ctr) => "A256CTR",
            (256, Self::Cbc) => "A256CBC",
            (256, Self::Gcm) => "A256GCM",
            (256, Self::Kw) => "A256KW",
            _ => return None,
        })
    }
}

/// A WebCrypto EC named curve (WebCrypto §23.4 `NamedCurve` typedef =
/// `DOMString`).  Unlike a Web IDL `enum`, an unrecognized value is a
/// `NotSupportedError` (prose-validated at the algorithm-specific step,
/// §23.7.3 / §24.4.1 / §23.7.4), NOT a WebIDL `TypeError` — so the VM
/// marshals the raw string and [`super::normalize`] recognizes it here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NamedCurve {
    P256,
    P384,
    P521,
}

impl NamedCurve {
    /// Recognize a `NamedCurve` value (exact match — the curve names are
    /// case-sensitive, unlike algorithm names).
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "P-256" => Self::P256,
            "P-384" => Self::P384,
            "P-521" => Self::P521,
            _ => return None,
        })
    }

    /// The canonical curve name for `[[algorithm]].namedCurve` + JWK `crv`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::P256 => "P-256",
            Self::P384 => "P-384",
            Self::P521 => "P-521",
        }
    }

    /// The field-element / coordinate length in bytes = `⌈log2(p) / 8⌉`:
    /// P-256 → 32, P-384 → 48, **P-521 → 66** (`⌈521 / 8⌉ = 66`, NOT 65 —
    /// the well-known P-521 edge).  Also the ECDH shared-secret length and
    /// each ECDSA signature half (`r`, `s`).
    pub fn coordinate_len(self) -> usize {
        match self {
            Self::P256 => 32,
            Self::P384 => 48,
            Self::P521 => 66,
        }
    }

    /// The raw ECDSA signature length (`r‖s`) = `2 * coordinate_len`
    /// (WebCrypto §23.7.1 / §23.7.2): P-256 → 64, P-384 → 96, P-521 → 132.
    pub fn signature_len(self) -> usize {
        2 * self.coordinate_len()
    }
}

/// Which EC algorithm family a generate / import resolves to (ECDSA vs
/// ECDH).  `EcKeyGenParams` (§23.4) and `EcKeyImportParams` (§23.6) carry
/// only `namedCurve`, so this discriminator rides alongside the curve to
/// decide the produced key's `[[algorithm]]` — the EC analogue of
/// [`AesVariant`] inside `AesKeyGen` / `AesImport`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EcAlgorithm {
    Ecdsa,
    Ecdh,
}

impl EcAlgorithm {
    pub(crate) fn algorithm_name(self) -> AlgorithmName {
        match self {
            Self::Ecdsa => AlgorithmName::Ecdsa,
            Self::Ecdh => AlgorithmName::Ecdh,
        }
    }

    /// The key's `[[algorithm]]` for this EC family + curve (WebCrypto §23.5
    /// / §24 `EcKeyAlgorithm`) — used by both EC import and generateKey.
    pub(crate) fn key_algorithm(self, curve: NamedCurve) -> KeyAlgorithm {
        match self {
            Self::Ecdsa => KeyAlgorithm::Ecdsa { curve },
            Self::Ecdh => KeyAlgorithm::Ecdh { curve },
        }
    }
}

/// Which RSA signing family a generate / import / sign / verify resolves to
/// (RSASSA-PKCS1-v1_5 §20 vs RSA-PSS §21).  The §20.4 `RsaHashedKeyGenParams`
/// / §20.7 `RsaHashedImportParams` dicts carry no family marker, so this
/// discriminator rides alongside the key params to decide the produced key's
/// `[[algorithm]]` and the sign / verify padding — the RSA analogue of
/// [`EcAlgorithm`] / [`AesVariant`].  (RSA-OAEP §22 — the encrypt family —
/// lands its variant in PR-5b.)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RsaVariant {
    RsassaPkcs1V15,
    RsaPss,
}

impl RsaVariant {
    /// The canonical WebCrypto algorithm name (`"RSASSA-PKCS1-v1_5"` /
    /// `"RSA-PSS"`) for the key's `[[algorithm]]` `name` attribute.
    pub fn canonical_name(self) -> &'static str {
        match self {
            Self::RsassaPkcs1V15 => "RSASSA-PKCS1-v1_5",
            Self::RsaPss => "RSA-PSS",
        }
    }

    pub(crate) fn algorithm_name(self) -> AlgorithmName {
        match self {
            Self::RsassaPkcs1V15 => AlgorithmName::RsassaPkcs1V15,
            Self::RsaPss => AlgorithmName::RsaPss,
        }
    }

    /// The key's `[[algorithm]]` for this RSA family (WebCrypto §20.6 / §21
    /// `RsaHashedKeyAlgorithm`) — used by both RSA import and generateKey.
    pub(crate) fn key_algorithm(
        self,
        modulus_length: u32,
        public_exponent: Vec<u8>,
        hash: HashAlgorithm,
    ) -> KeyAlgorithm {
        KeyAlgorithm::Rsa {
            variant: self,
            modulus_length,
            public_exponent,
            hash,
        }
    }

    /// The JWK `alg` value for this RSA family + `hash`, emitted on export
    /// (WebCrypto §20.8.5 RSASSA / §21.4.5 RSA-PSS jwk) and matched on import
    /// (§20.8.4 / §21.4.4): `RS1` / `RS256` / `RS384` / `RS512` for
    /// RSASSA-PKCS1-v1_5, `PS1` / `PS256` / `PS384` / `PS512` for RSA-PSS.
    /// Total over the four hashes — WebCrypto defines the SHA-1 `RS1` / `PS1`
    /// values explicitly (unlike RFC 7518, which omits them).
    pub fn jwk_alg(self, hash: HashAlgorithm) -> &'static str {
        match (self, hash) {
            (Self::RsassaPkcs1V15, HashAlgorithm::Sha1) => "RS1",
            (Self::RsassaPkcs1V15, HashAlgorithm::Sha256) => "RS256",
            (Self::RsassaPkcs1V15, HashAlgorithm::Sha384) => "RS384",
            (Self::RsassaPkcs1V15, HashAlgorithm::Sha512) => "RS512",
            (Self::RsaPss, HashAlgorithm::Sha1) => "PS1",
            (Self::RsaPss, HashAlgorithm::Sha256) => "PS256",
            (Self::RsaPss, HashAlgorithm::Sha384) => "PS384",
            (Self::RsaPss, HashAlgorithm::Sha512) => "PS512",
        }
    }
}
