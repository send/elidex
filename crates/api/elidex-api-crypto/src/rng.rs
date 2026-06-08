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
/// key generation.  On error the buffer is filled with the canonical scalar
/// `1` (big-endian `…01`) — a valid non-zero value — so a rejection-sampling
/// loop (`SecretKey::random`) terminates rather than spinning on a zero /
/// out-of-range fill; the resulting key is then discarded because
/// `into_result` returns the captured error.
pub(crate) struct ClosureRng<'a> {
    fill: &'a mut dyn FnMut(&mut [u8]) -> Result<(), AlgorithmError>,
    error: Option<AlgorithmError>,
}

impl<'a> ClosureRng<'a> {
    pub(crate) fn new(fill: &'a mut dyn FnMut(&mut [u8]) -> Result<(), AlgorithmError>) -> Self {
        Self { fill, error: None }
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
            // Canonical value `1` so a rejection loop terminates; the key is
            // discarded via `into_result`.
            dest.fill(0);
            if let Some(last) = dest.last_mut() {
                *last = 1;
            }
        }
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand_core::Error> {
        self.fill_bytes(dest);
        Ok(())
    }
}

impl CryptoRng for ClosureRng<'_> {}
