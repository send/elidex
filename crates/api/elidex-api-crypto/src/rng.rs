//! The VM entropy-seam adapters shared by the asymmetric key generators.
//!
//! Elliptic-curve (`ec::generate` -> `NonZeroScalar::try_generate_from_rng`)
//! and RSA (`rsa::generate` -> `RsaPrivateKey::new_with_exp`, plus RSA signing
//! via `sign_with_rng`) both draw their
//! randomness through RustCrypto `rand_core` traits, but from two
//! generations: EC (`elliptic-curve` 0.14) uses `rand_core` 0.10, RSA
//! (`rsa` 0.9) still uses 0.6.  Rather than each backend reaching for a
//! separate `getrandom` path, two thin adapters wrap the VM's single fallible
//! `fill_random` closure (which ultimately calls the OS CSPRNG), so all key
//! generation flows through the one VM entropy seam while the curve / RSA
//! crates still perform their own vetted rejection sampling:
//!
//! - [`TryClosureRng`]: `rand_core` 0.10 (EC), fallible, stateless.
//! - [`ClosureRng`]: `rand_core` 0.6 (RSA key generation and signing),
//!   infallible, with the error latch.  When `rsa` 0.10 is stable it moves to
//!   the `rand_core` 0.10 infallible traits (keeping the latch and SplitMix64
//!   fallback) and the 0.6 impl goes (`#11-closure-rng-rand-core-06-bridge`); [`TryClosureRng`] stays
//!   for EC.

use elliptic_curve::rand_core as rand_core10;

use crate::error::AlgorithmError;

/// A `rand_core` 0.6 RNG over the VM's `fill_random` closure, for RSA (key generation and signing).
///
/// The 0.6 `fill_bytes` is infallible, so a closure error is captured and
/// surfaced by [`Self::into_result`] **after** key generation.  Once the error
/// is latched, the buffer is filled from a deterministic but *varying*
/// SplitMix64 stream so the consumer's rejection-sampling loop still
/// **terminates**, and the resulting key is discarded because `into_result`
/// returns the captured error.
///
/// The `rand_core` 0.6 *impl* is what goes away with `rsa` 0.9 (see the removal
/// statement on it); the type stays, since `rsa` 0.10 takes an infallible
/// `rand_core` 0.10 `CryptoRng` and so still needs this latch / SplitMix64 /
/// `into_result` workaround.  The EC path uses [`TryClosureRng`].
///
/// The fill must *vary* per call, not be constant: RSA's
/// `RsaPrivateKey::new_with_exp` does iterative **prime search**, and a
/// *constant* candidate never becomes prime, so a fixed fill would spin its
/// loop forever (the entropy error would never surface). The SplitMix64
/// stream feeds the prime search fresh candidates so it converges, then the
/// key is discarded.
pub(crate) struct ClosureRng<'a> {
    fill: &'a mut dyn FnMut(&mut [u8]) -> Result<(), AlgorithmError>,
    error: Option<AlgorithmError>,
    /// SplitMix64 state for the post-error fallback fill (see the type doc).
    fallback: u64,
}

impl<'a> ClosureRng<'a> {
    pub(crate) fn new(fill: &'a mut dyn FnMut(&mut [u8]) -> Result<(), AlgorithmError>) -> Self {
        Self {
            fill,
            error: None,
            fallback: 0,
        }
    }

    /// The first captured `fill_random` error, if any, checked by the RSA
    /// caller after key generation to reject a key built over failed entropy.
    pub(crate) fn into_result(self) -> Result<(), AlgorithmError> {
        match self.error {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }
}

impl ClosureRng<'_> {
    /// The infallible fill behind the `rand_core` 0.6 (RSA) impl: latches the
    /// first closure error and keeps filling so the consumer terminates.
    fn fill(&mut self, dest: &mut [u8]) {
        if self.error.is_none() {
            if let Err(e) = (self.fill)(dest) {
                self.error = Some(e);
            }
        }
        if self.error.is_some() {
            // The entropy seam failed: emit a deterministic but *varying*
            // SplitMix64 stream so the consumer's rejection sampling converges
            // (RSA prime search needs fresh bytes per iteration — a constant
            // fill would spin keygen forever). The key built over these bytes
            // is discarded via `into_result`.
            for chunk in dest.chunks_mut(8) {
                self.fallback = self.fallback.wrapping_add(0x9E37_79B9_7F4A_7C15);
                let mut z = self.fallback;
                z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
                z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
                z ^= z >> 31;
                let bytes = z.to_le_bytes();
                chunk.copy_from_slice(&bytes[..chunk.len()]);
            }
        }
    }
}

/// `rand_core` 0.6 (RSA key generation and signing).  Exists only because
/// `rsa`'s stable line (0.9) is still on the 0.6 generation.  When `rsa` 0.10 is
/// stable (its `RsaPrivateKey::new_with_exp` / `sign_with_rng` take an infallible
/// `rand_core` 0.10 `CryptoRng`, which `rand_core` blanket-implements only for
/// `TryRng<Error = Infallible>`, so [`TryClosureRng`] cannot serve RSA),
/// `ClosureRng` moves to the 0.10 infallible traits, keeping its error latch and
/// SplitMix64 fallback; this 0.6 impl and the workspace `rand_core = "0.6"`
/// dependency are then removed (`#11-closure-rng-rand-core-06-bridge`).
impl rand_core::RngCore for ClosureRng<'_> {
    fn next_u32(&mut self) -> u32 {
        rand_core::impls::next_u32_via_fill(self)
    }

    fn next_u64(&mut self) -> u64 {
        rand_core::impls::next_u64_via_fill(self)
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.fill(dest);
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand_core::Error> {
        self.fill(dest);
        Ok(())
    }
}

impl rand_core::CryptoRng for ClosureRng<'_> {}

/// A fallible `rand_core` 0.10 RNG over the VM's `fill_random` closure, for the
/// EC key generators.  A closure failure is returned as-is (no latch, no
/// fallback stream), so the caller's `?` propagates the original error.
pub(crate) struct TryClosureRng<'a> {
    fill: &'a mut dyn FnMut(&mut [u8]) -> Result<(), AlgorithmError>,
}

impl<'a> TryClosureRng<'a> {
    pub(crate) fn new(fill: &'a mut dyn FnMut(&mut [u8]) -> Result<(), AlgorithmError>) -> Self {
        Self { fill }
    }
}

/// `rand_core` 0.10 `TryRng` / `TryCryptoRng` only.  The infallible `Rng` /
/// `CryptoRng` are blanket-implemented by `rand_core` solely for
/// `Error = Infallible`, so they do **not** follow here: consumers must use
/// the `try_*` entry points (e.g. `try_generate_from_rng`).
impl rand_core10::TryRng for TryClosureRng<'_> {
    type Error = AlgorithmError;

    fn try_next_u32(&mut self) -> Result<u32, Self::Error> {
        rand_core10::utils::next_word_via_fill(self)
    }

    fn try_next_u64(&mut self) -> Result<u64, Self::Error> {
        rand_core10::utils::next_word_via_fill(self)
    }

    fn try_fill_bytes(&mut self, dst: &mut [u8]) -> Result<(), Self::Error> {
        (self.fill)(dst)
    }
}

impl rand_core10::TryCryptoRng for TryClosureRng<'_> {}
