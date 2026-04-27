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

    /// Return `alias` when `s` is empty; otherwise intern `s` via
    /// [`Self::intern`] and return the resulting `StringId`.  Lets call sites
    /// route the WHATWG "value attribute resolves to empty string"
    /// fast-path through a single pre-interned sentinel
    /// (`well_known.empty`) instead of paying the per-call hash
    /// lookup.  Centralises the alias-or-intern shape that
    /// `setNamedItem` / `removeNamedItem` / `setAttributeNode` /
    /// `removeAttributeNode` snapshot the prior attribute value
    /// through.
    pub fn intern_or_alias(&mut self, alias: StringId, s: &str) -> StringId {
        if s.is_empty() {
            alias
        } else {
            self.intern(s)
        }
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

/// Pool of arbitrary-precision BigInt values.  Allocated BigInts are
/// permanent (not garbage-collected) but **deduplicated by value**:
/// repeated `alloc(v)` for the same `v` returns the same `BigIntId`.
/// Canonical `0n` and `1n` are pre-allocated and short-circuit
/// before the dedup lookup to avoid hashing in common
/// increment/decrement patterns (`i + 1n` etc.).
///
/// Dedup motivation: every TypedArray BigInt read
/// (`%TypedArray%.prototype.{forEach, every, some, find, findIndex,
/// findLast, findLastIndex, map, filter, reduce, reduceRight, sort,
/// at, includes, indexOf, lastIndexOf, join, values, keys, entries,
/// copyWithin, reverse, fill, ...}` on `BigInt64Array` /
/// `BigUint64Array`) goes through `read_element_raw` which calls
/// `alloc`.  Without dedup the pool grows by one entry per
/// per-element read across the entire `%TypedArray%.prototype`
/// surface, producing slow unbounded growth in long-running
/// workloads (Copilot SP8c-A R5 surfaced this on `sort`).  With
/// dedup, a `BigInt64Array(N).map(...)` allocates at most `N` new
/// pool entries the first time each unique value appears, and
/// subsequent passes — including the rest of the typed-array
/// surface — find existing entries via the `dedup` lookup.
pub(crate) struct BigIntPool {
    values: Vec<num_bigint::BigInt>,
    /// Value → `BigIntId` index for deduplication.  Kept in sync
    /// with `values` (every push also inserts here); never
    /// removed because pool entries are permanent.
    ///
    /// `0n` and `1n` are NOT in `dedup` — they short-circuit in
    /// [`Self::alloc`] before the lookup, so a `Hash` round-trip
    /// for the hot increment/decrement path is avoided.
    dedup: std::collections::HashMap<num_bigint::BigInt, value::BigIntId>,
    /// Pre-allocated ID for `0n`.
    pub(crate) zero: value::BigIntId,
    /// Pre-allocated ID for `1n`.
    pub(crate) one: value::BigIntId,
}

impl BigIntPool {
    pub(crate) fn new() -> Self {
        Self {
            values: vec![num_bigint::BigInt::from(0), num_bigint::BigInt::from(1)],
            dedup: std::collections::HashMap::new(),
            zero: value::BigIntId(0),
            one: value::BigIntId(1),
        }
    }

    /// Allocate or reuse a `BigIntId` for `val`.  Hot-path order:
    ///
    /// 1. `0n` / `1n` short-circuit (sign / one check) — no hash.
    /// 2. `dedup` lookup — value-equal BigInts share an id.
    /// 3. Fresh push + `dedup.insert(val.clone(), id)`.
    ///
    /// `num_traits::One::is_one()` avoids constructing a temporary
    /// `BigInt::from(1)` on every call (hot for `i + 1n` /
    /// incrementing loops).  The clone in step 3 is unavoidable
    /// because `dedup` and `values` both need to own the value;
    /// for typical 1-2 limb BigInts the clone is ~16 bytes.
    pub(crate) fn alloc(&mut self, val: num_bigint::BigInt) -> value::BigIntId {
        use num_bigint::Sign;
        use num_traits::One;
        match val.sign() {
            Sign::NoSign => return self.zero,
            Sign::Plus if val.is_one() => return self.one,
            _ => {}
        }
        if let Some(&id) = self.dedup.get(&val) {
            return id;
        }
        let id = value::BigIntId(self.values.len() as u32);
        self.dedup.insert(val.clone(), id);
        self.values.push(val);
        id
    }

    /// Get a reference to a BigInt by its ID.
    #[inline]
    pub(crate) fn get(&self, id: value::BigIntId) -> &num_bigint::BigInt {
        &self.values[id.0 as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_bigint::BigInt;

    #[test]
    fn bigint_pool_dedups_repeated_values() {
        // Pool dedup contract: `alloc(v)` for the same `v`
        // returns the same `BigIntId`, so per-element TypedArray
        // BigInt reads (`%TypedArray%.prototype.{forEach, every,
        // some, find, map, filter, reduce, sort, ...}` on
        // BigInt64Array / BigUint64Array) don't grow the pool
        // unboundedly across repeated invocations.
        let mut pool = BigIntPool::new();
        let pre_len = pool.values.len();

        // Three reads of `42n` share an id.
        let id_42_first = pool.alloc(BigInt::from(42));
        let id_42_second = pool.alloc(BigInt::from(42));
        let id_42_third = pool.alloc(BigInt::from(42));
        assert_eq!(id_42_first, id_42_second);
        assert_eq!(id_42_first, id_42_third);
        assert_eq!(pool.values.len(), pre_len + 1, "deduped to one entry");

        // A different value gets a fresh id.
        let id_neg7_first = pool.alloc(BigInt::from(-7));
        assert_ne!(id_42_first, id_neg7_first);
        assert_eq!(pool.values.len(), pre_len + 2);

        // Re-allocating the new value also dedups.
        let id_neg7_second = pool.alloc(BigInt::from(-7));
        assert_eq!(id_neg7_first, id_neg7_second);
        assert_eq!(pool.values.len(), pre_len + 2, "still two entries");
    }

    #[test]
    fn bigint_pool_short_circuits_zero_and_one() {
        // `0n` and `1n` are pre-allocated and short-circuit
        // before the dedup hash lookup — verify no pool growth
        // for repeated `0n`/`1n` allocations (hot path for
        // `i + 1n` increment loops).
        let mut pool = BigIntPool::new();
        let pre_len = pool.values.len();
        for _ in 0..100 {
            assert_eq!(pool.alloc(BigInt::from(0)), pool.zero);
            assert_eq!(pool.alloc(BigInt::from(1)), pool.one);
        }
        assert_eq!(pool.values.len(), pre_len, "no growth for canonical values");
    }
}
