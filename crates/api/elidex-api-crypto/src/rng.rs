//! The VM entropy-seam adapter shared by the asymmetric key generators.
//!
//! Elliptic-curve (`ec::generate` → `SecretKey::random`) and RSA
//! (`rsa::generate` → `RsaPrivateKey::new_with_exp`) both draw their
//! randomness through the RustCrypto `rand_core` 0.6 `RngCore` + `CryptoRng`
//! traits.  Rather than each backend reaching for a separate `getrandom`
//! path, [`ClosureRng`] adapts the VM's single fallible `fill_random` closure
//! (which ultimately calls the OS CSPRNG) into those traits, so all key
//! generation flows through the one VM entropy seam while the curve / RSA
//! crates still perform their own vetted rejection sampling.

use rand_core::{CryptoRng, RngCore};

use crate::error::AlgorithmError;

/// An RNG over the VM's `fill_random` closure (the single entropy seam).
///
/// `fill_random` is fallible but `RngCore::fill_bytes` is infallible, so a
/// closure error is captured and surfaced by [`Self::into_result`] **after**
/// key generation.  Once the error is latched, the buffer is filled from a
/// deterministic but *varying* SplitMix64 stream so the consumer's
/// rejection-sampling loop still **terminates** — and the resulting key is
/// discarded because `into_result` returns the captured error.
///
/// The fill must *vary* per call, not be constant: EC's `SecretKey::random`
/// would accept a constant valid scalar after one iteration, but RSA's
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

    /// The first captured `fill_random` error, if any — checked by the caller
    /// after key generation to reject a key built over failed entropy.
    pub(crate) fn into_result(self) -> Result<(), AlgorithmError> {
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
            // The entropy seam failed: emit a deterministic but *varying*
            // SplitMix64 stream so the consumer's rejection sampling converges
            // (EC scalar search AND RSA prime search both need fresh bytes per
            // iteration — a constant fill would spin RSA keygen forever). The
            // key built over these bytes is discarded via `into_result`.
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

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand_core::Error> {
        self.fill_bytes(dest);
        Ok(())
    }
}

impl CryptoRng for ClosureRng<'_> {}
