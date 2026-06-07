//! Hash algorithms (WebCrypto §32 SHA) — the engine-independent
//! `digest` driver relocated from the VM host's `DigestAlgo::compute`
//! (CLAUDE.md "Layering mandate": algorithm math belongs in the crate,
//! the VM only marshals bytes).

// `sha1::Sha1` and `sha2::Sha{256,384,512}` all implement the shared
// `digest::Digest` trait re-exported by `sha2`; importing it once
// brings the `digest` associated fn into scope for every hash.
use sha2::Digest as _;

/// A WebCrypto-recognized hash function.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HashAlgorithm {
    Sha1,
    Sha256,
    Sha384,
    Sha512,
}

impl HashAlgorithm {
    /// Compute the digest of `data`.
    pub fn digest(self, data: &[u8]) -> Vec<u8> {
        match self {
            Self::Sha1 => sha1::Sha1::digest(data).to_vec(),
            Self::Sha256 => sha2::Sha256::digest(data).to_vec(),
            Self::Sha384 => sha2::Sha384::digest(data).to_vec(),
            Self::Sha512 => sha2::Sha512::digest(data).to_vec(),
        }
    }

    /// The HMAC block size in bits (the default HMAC key length for this
    /// hash per WebCrypto §31 HMAC Generate Key): 512 for SHA-1/SHA-256,
    /// 1024 for SHA-384/SHA-512.
    pub fn block_size_bits(self) -> u32 {
        match self {
            Self::Sha1 | Self::Sha256 => 512,
            Self::Sha384 | Self::Sha512 => 1024,
        }
    }

    /// The digest output length in bytes (`HashLen`): SHA-1 → 20, SHA-256 →
    /// 32, SHA-384 → 48, SHA-512 → 64.  Used for the HKDF-Expand maximum
    /// output bound `255 × HashLen` (RFC 5869 §2.3 / WebCrypto §33.4.1).
    pub fn output_len_bytes(self) -> usize {
        match self {
            Self::Sha1 => 20,
            Self::Sha256 => 32,
            Self::Sha384 => 48,
            Self::Sha512 => 64,
        }
    }

    /// The canonical WebCrypto algorithm name (`"SHA-256"` etc.).
    pub fn canonical_name(self) -> &'static str {
        match self {
            Self::Sha1 => "SHA-1",
            Self::Sha256 => "SHA-256",
            Self::Sha384 => "SHA-384",
            Self::Sha512 => "SHA-512",
        }
    }

    /// The JWK `alg` value for an HMAC key using this hash (WebCrypto §31
    /// HMAC registration: SHA-1 → `"HS1"`, SHA-256 → `"HS256"`, …).
    pub fn jwk_hmac_alg(self) -> &'static str {
        match self {
            Self::Sha1 => "HS1",
            Self::Sha256 => "HS256",
            Self::Sha384 => "HS384",
            Self::Sha512 => "HS512",
        }
    }

    /// Recognize a hash from its canonical name, ASCII case-insensitively
    /// (WebCrypto §18.4.4 normalize, case-insensitive match).
    pub fn from_canonical_ci(name: &str) -> Option<Self> {
        if name.eq_ignore_ascii_case("SHA-1") {
            Some(Self::Sha1)
        } else if name.eq_ignore_ascii_case("SHA-256") {
            Some(Self::Sha256)
        } else if name.eq_ignore_ascii_case("SHA-384") {
            Some(Self::Sha384)
        } else if name.eq_ignore_ascii_case("SHA-512") {
            Some(Self::Sha512)
        } else {
            None
        }
    }
}
