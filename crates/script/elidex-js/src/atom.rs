//! Interned string type for zero-copy token and AST string storage.
//!
//! `Atom` is a `Copy` handle into a `StringInterner`. All identifier names,
//! string literals, and other textual data in the lexer/parser/AST use `Atom`
//! instead of `String`, eliminating per-token heap allocations.
//!
//! Internally, all strings are stored in a WTF-16 contiguous buffer via
//! `Wtf16Interner`. Deduplication uses `hashbrown::HashTable` with external
//! hash/eq resolved through the buffer — zero per-string heap allocations.

use std::fmt;

use crate::wtf16::Wtf16Interner;

/// An interned string handle. Copy + Eq + Hash + Ord.
///
/// Index 0 is reserved for the empty string.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Atom(pub(crate) u32);

impl Atom {
    /// The empty-string atom (always index 0).
    pub const EMPTY: Self = Self(0);
}

impl fmt::Debug for Atom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Atom({})", self.0)
    }
}

/// Append-only string interner backed by a WTF-16 contiguous buffer.
///
/// Deduplicates strings so that equal strings map to the same `Atom`.
/// Thread-local (not `Sync`); each parser instance owns one.
#[derive(Debug)]
pub struct StringInterner(Wtf16Interner);

impl Default for StringInterner {
    fn default() -> Self {
        Self::new()
    }
}

impl StringInterner {
    /// Create a new interner with the empty string pre-interned at index 0.
    #[must_use]
    pub fn new() -> Self {
        Self(Wtf16Interner::new())
    }

    /// Intern a UTF-8 string, returning its `Atom`. Idempotent.
    pub fn intern(&mut self, s: &str) -> Atom {
        Atom(self.0.intern(s))
    }

    /// Intern a WTF-16 slice, returning its `Atom`. Idempotent.
    pub fn intern_wtf16(&mut self, units: &[u16]) -> Atom {
        Atom(self.0.intern_wtf16(units))
    }

    /// Resolve an `Atom` back to its WTF-16 content.
    #[inline]
    #[must_use]
    pub fn get(&self, atom: Atom) -> &[u16] {
        self.0.get(atom.0)
    }

    /// Resolve an `Atom` back to a UTF-8 String (lossy for lone surrogates).
    #[must_use]
    pub fn get_utf8(&self, atom: Atom) -> String {
        self.0.get_utf8(atom.0)
    }

    /// Look up an already-interned string, returning its `Atom` or `Atom::EMPTY` if not found.
    #[must_use]
    pub fn lookup(&self, s: &str) -> Atom {
        let units: Vec<u16> = s.encode_utf16().collect();
        self.0.lookup_wtf16(&units).map_or(Atom::EMPTY, Atom)
    }
}

/// Pre-interned atoms for frequently compared contextual keywords.
///
/// Comparing `Atom` values is a single `u32 == u32` operation, whereas
/// `at_contextual("async")` required a full string comparison through the interner.
#[derive(Debug, Clone, Copy)]
pub struct WellKnownAtoms {
    pub r#async: Atom,
    pub r#await: Atom,
    pub r#yield: Atom,
    pub get: Atom,
    pub set: Atom,
    pub of: Atom,
    pub from: Atom,
    pub r#as: Atom,
    pub meta: Atom,
    pub target: Atom,
    pub constructor: Atom,
    pub proto: Atom,
    pub prototype: Atom,
    pub default: Atom,
    pub eval: Atom,
    pub arguments: Atom,
}

impl WellKnownAtoms {
    /// Pre-intern all well-known atoms into the given interner.
    pub fn new(interner: &mut StringInterner) -> Self {
        Self {
            r#async: interner.intern("async"),
            r#await: interner.intern("await"),
            r#yield: interner.intern("yield"),
            get: interner.intern("get"),
            set: interner.intern("set"),
            of: interner.intern("of"),
            from: interner.intern("from"),
            r#as: interner.intern("as"),
            meta: interner.intern("meta"),
            target: interner.intern("target"),
            constructor: interner.intern("constructor"),
            proto: interner.intern("__proto__"),
            prototype: interner.intern("prototype"),
            default: interner.intern("default"),
            eval: interner.intern("eval"),
            arguments: interner.intern("arguments"),
        }
    }
}

/// Display implementation that shows `Atom(index)` — use `StringInterner::get()` for the string.
impl fmt::Display for Atom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Atom({})", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intern_dedup() {
        let mut si = StringInterner::new();
        let a = si.intern("hello");
        let b = si.intern("hello");
        assert_eq!(a, b);
        assert_eq!(si.get_utf8(a), "hello");
    }

    #[test]
    fn empty_atom() {
        let si = StringInterner::new();
        assert_eq!(si.get_utf8(Atom::EMPTY), "");
    }

    #[test]
    fn distinct_strings() {
        let mut si = StringInterner::new();
        let a = si.intern("foo");
        let b = si.intern("bar");
        assert_ne!(a, b);
        assert_eq!(si.get_utf8(a), "foo");
        assert_eq!(si.get_utf8(b), "bar");
    }

    #[test]
    fn atom_is_copy() {
        let mut si = StringInterner::new();
        let a = si.intern("x");
        let b = a; // Copy
        assert_eq!(a, b);
    }

    #[test]
    fn lookup_existing() {
        let mut si = StringInterner::new();
        let a = si.intern("test");
        assert_eq!(si.lookup("test"), a);
    }

    #[test]
    fn lookup_missing() {
        let si = StringInterner::new();
        assert_eq!(si.lookup("nonexistent"), Atom::EMPTY);
    }

    #[test]
    fn intern_wtf16_dedup() {
        let mut si = StringInterner::new();
        let units: Vec<u16> = "abc".encode_utf16().collect();
        let a = si.intern_wtf16(&units);
        let b = si.intern_wtf16(&units);
        assert_eq!(a, b);
        assert_eq!(si.get(a), &units[..]);
    }

    #[test]
    fn get_returns_u16_slice() {
        let mut si = StringInterner::new();
        let a = si.intern("hello");
        let expected: Vec<u16> = "hello".encode_utf16().collect();
        assert_eq!(si.get(a), &expected[..]);
    }
}
