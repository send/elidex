//! String and BigInt pools backing the VM's handle-based value model.
//!
//! Both pools allocate values permanently (no GC): strings and BigInts are
//! referenced via `StringId` / `BigIntId` handles that never dangle.

use super::value;
use super::value::StringId;
use crate::wtf16::Wtf16Interner;

// ---------------------------------------------------------------------------
// StringPool
// ---------------------------------------------------------------------------

/// Interned string pool backed by a WTF-16 contiguous buffer. All runtime
/// strings are stored here and referenced by `StringId`. Deduplication
/// ensures that property-name comparisons are O(1) integer equality.
pub struct StringPool(Wtf16Interner);

impl StringPool {
    pub(crate) fn new() -> Self {
        Self(Wtf16Interner::new())
    }

    /// Intern a string from UTF-8, returning its `StringId`.
    pub fn intern(&mut self, s: &str) -> StringId {
        StringId(self.0.intern(s))
    }

    /// Intern a string from raw WTF-16 code units.
    pub fn intern_utf16(&mut self, units: &[u16]) -> StringId {
        StringId(self.0.intern_wtf16(units))
    }

    /// Look up a string by its ID, returning WTF-16 code units.
    #[inline]
    pub fn get(&self, id: StringId) -> &[u16] {
        self.0.get(id.0)
    }

    /// Check if a string is already interned (without inserting), returning
    /// its `StringId` if found. O(1) hash lookup.
    pub fn lookup(&self, s: &str) -> Option<StringId> {
        let units: Vec<u16> = s.encode_utf16().collect();
        self.0.lookup_wtf16(&units).map(StringId)
    }

    /// Look up a string by its ID, returning a UTF-8 `String` (lossy for
    /// lone surrogates).
    pub fn get_utf8(&self, id: StringId) -> String {
        self.0.get_utf8(id.0)
    }

    /// Returns the number of interned strings.
    #[allow(dead_code, clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

// ---------------------------------------------------------------------------
// BigIntPool
// ---------------------------------------------------------------------------

/// Pool of arbitrary-precision BigInt values. Allocated BigInts are permanent
/// (not garbage-collected), following the same strategy as `StringPool`.
/// Canonical 0n and 1n are pre-allocated to avoid repeated allocation in
/// common patterns like `i + 1n`.
pub(crate) struct BigIntPool {
    values: Vec<num_bigint::BigInt>,
    /// Pre-allocated ID for `0n`.
    pub(crate) zero: value::BigIntId,
    /// Pre-allocated ID for `1n`.
    pub(crate) one: value::BigIntId,
}

impl BigIntPool {
    pub(crate) fn new() -> Self {
        Self {
            values: vec![num_bigint::BigInt::from(0), num_bigint::BigInt::from(1)],
            zero: value::BigIntId(0),
            one: value::BigIntId(1),
        }
    }

    /// Allocate a new BigInt, returning its `BigIntId`.
    /// Returns cached IDs for 0 and 1.  `num_traits::One::is_one()` avoids
    /// constructing a temporary `BigInt::from(1)` on every call (hot for
    /// `i + 1n` / incrementing loops).
    pub(crate) fn alloc(&mut self, val: num_bigint::BigInt) -> value::BigIntId {
        use num_bigint::Sign;
        use num_traits::One;
        match val.sign() {
            Sign::NoSign => return self.zero,
            Sign::Plus if val.is_one() => return self.one,
            _ => {}
        }
        let id = value::BigIntId(self.values.len() as u32);
        self.values.push(val);
        id
    }

    /// Get a reference to a BigInt by its ID.
    #[inline]
    pub(crate) fn get(&self, id: value::BigIntId) -> &num_bigint::BigInt {
        &self.values[id.0 as usize]
    }
}
