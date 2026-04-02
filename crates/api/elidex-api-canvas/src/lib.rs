//! Engine-independent Canvas 2D API protocol types.
//!
//! Provides constants and helpers shared across script engine bindings.
//! The actual rendering logic lives in `elidex-web-canvas`.

/// Default canvas width in pixels (HTML spec §4.12.5.1).
pub const DEFAULT_WIDTH: u32 = 300;

/// Default canvas height in pixels (HTML spec §4.12.5.1).
pub const DEFAULT_HEIGHT: u32 = 150;

/// Split a 64-bit entity ID into (high, low) 32-bit parts.
///
/// Entity bits are split to avoid f64 precision loss for values > 2^53
/// when stored in JS number properties.
#[must_use]
pub fn split_entity_bits(bits: u64) -> (u32, u32) {
    let hi = (bits >> 32) as u32;
    let lo = bits as u32;
    (hi, lo)
}

/// Reconstruct a 64-bit entity ID from (high, low) 32-bit parts.
#[must_use]
pub fn join_entity_bits(hi: u32, lo: u32) -> u64 {
    (u64::from(hi) << 32) | u64::from(lo)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults() {
        assert_eq!(DEFAULT_WIDTH, 300);
        assert_eq!(DEFAULT_HEIGHT, 150);
    }

    #[test]
    fn split_and_join_roundtrip() {
        let bits: u64 = 0xDEAD_BEEF_CAFE_BABEu64;
        let (hi, lo) = split_entity_bits(bits);
        assert_eq!(hi, 0xDEAD_BEEF);
        assert_eq!(lo, 0xCAFE_BABE);
        assert_eq!(join_entity_bits(hi, lo), bits);
    }

    #[test]
    fn split_zero() {
        let (hi, lo) = split_entity_bits(0);
        assert_eq!(hi, 0);
        assert_eq!(lo, 0);
        assert_eq!(join_entity_bits(hi, lo), 0);
    }

    #[test]
    fn split_max() {
        let (hi, lo) = split_entity_bits(u64::MAX);
        assert_eq!(hi, u32::MAX);
        assert_eq!(lo, u32::MAX);
        assert_eq!(join_entity_bits(hi, lo), u64::MAX);
    }
}
