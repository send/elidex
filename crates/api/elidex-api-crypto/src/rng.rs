//! The VM entropy-seam adapter shared by the asymmetric key generators.
//!
//! Elliptic-curve (`ec::generate` → `SecretKey::try_generate_from_rng`) and RSA
//! (`rsa::generate` → `RsaPrivateKey::new_with_exp`) both draw their
//! randomness through the RustCrypto `rand_core` `RngCore` + `CryptoRng`
//! traits — but from two generations: EC (`elliptic-curve` 0.14) uses
//! `rand_core` 0.10, RSA (`rsa` 0.9) still uses 0.6.  Rather than each backend
//! reaching for a separate `getrandom` path, [`ClosureRng`] adapts the VM's single fallible `fill_random` closure
//! (which ultimately calls the OS CSPRNG) into those traits, so all key
//! generation flows through the one VM entropy seam while the curve / RSA
//! crates still perform their own vetted rejection sampling.

use elliptic_curve::rand_core as rand_core10;

use crate::error::AlgorithmError;

/// An RNG over the VM's `fill_random` closure (the single entropy seam).
///
/// The `rand_core` 0.10 impl (EC) is fallible, so a closure error is returned
/// directly.  The `rand_core` 0.6 impl (RSA) has an infallible `fill_bytes`, so
/// there a closure error is captured and surfaced by [`Self::into_result`]
/// **after** key generation.  Once the error is latched, the buffer is filled
/// from a deterministic but *varying* SplitMix64 stream so the consumer's
/// rejection-sampling loop still **terminates** — and the resulting key is
/// discarded because `into_result` returns the captured error.
///
/// This capture / SplitMix64 / `into_result` workaround exists for the 0.6
/// (RSA) path only and goes away with it (see the removal trigger on the 0.6
/// impl).
///
/// The fill must *vary* per call, not be constant: RSA's
/// `RsaPrivateKey::new_with_exp` does iterative **prime search** — a *constant*
/// candidate never becomes prime, so a fixed fill would spin its loop forever
/// (the entropy error would never surface). The SplitMix64 stream feeds the
/// prime search fresh candidates so it converges, then the key is discarded.
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

    /// The first captured `fill_random` error, if any — checked by the RSA
    /// caller after key generation to reject a key built over failed entropy
    /// (the 0.6 path only; the 0.10 EC impl returns the error directly).
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

/// `rand_core` 0.6 (RSA keygen).  Exists only because `rsa`'s stable line (0.9)
/// is still on the 0.6 generation; when `rsa` 0.10 is stable (rand_core 0.10),
/// bump it, delete this impl and the workspace `rand_core = "0.6"` dependency.
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

/// `rand_core` 0.10 (EC keygen); `Rng` / `CryptoRng` follow by blanket impl.
/// Fallible: a `fill_random` failure is returned as-is (no latch / fallback).
impl rand_core10::TryRng for ClosureRng<'_> {
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

impl rand_core10::TryCryptoRng for ClosureRng<'_> {}
