//! Interned string type for zero-copy token and AST string storage.
//!
//! `Atom` is a `Copy` handle into a `StringInterner`. All identifier names,
//! string literals, and other textual data in the lexer/parser/AST use `Atom`
//! instead of `String`, eliminating per-token heap allocations.
//!
//! Internally, all strings are stored in a single contiguous buffer.
//! Deduplication uses `hashbrown::HashTable` with external hash/eq
//! resolved through the buffer — zero per-string heap allocations.

use std::fmt;
use std::hash::{Hash, Hasher};

use hashbrown::HashTable;

/// An interned string handle. Copy + Eq + Hash + Ord.
///
/// Index 0 is reserved for the empty string.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Atom(u32);

impl Atom {
    /// The empty-string atom (always index 0).
    pub const EMPTY: Self = Self(0);
}

impl fmt::Debug for Atom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Atom({})", self.0)
    }
}

/// Append-only string interner backed by a contiguous buffer.
///
/// Deduplicates strings so that equal strings map to the same `Atom`.
/// Thread-local (not `Sync`); each parser instance owns one.
#[derive(Debug)]
pub struct StringInterner {
    /// All interned strings concatenated.
    buffer: String,
    /// `(byte_offset, byte_len)` for each `Atom` index.
    spans: Vec<(u32, u32)>,
    /// Hash table mapping string content → `Atom` index.
    table: HashTable<u32>,
}

impl Default for StringInterner {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute a hash for a string slice (used for hash table operations).
fn hash_str(s: &str) -> u64 {
    let mut h = std::hash::DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

impl StringInterner {
    /// Maximum number of interned strings — matches `Arena::MAX_NODES`.
    pub const MAX_STRINGS: usize = 16 * 1024 * 1024;

    /// Maximum cumulative buffer size (256 MiB).
    const MAX_BUFFER_BYTES: usize = 256 * 1024 * 1024;

    /// Create a new interner with the empty string pre-interned at index 0.
    #[must_use]
    pub fn new() -> Self {
        let mut si = Self {
            buffer: String::with_capacity(4096),
            spans: Vec::with_capacity(256),
            table: HashTable::with_capacity(256),
        };
        // Index 0 = empty string (offset 0, len 0 in an empty buffer).
        si.spans.push((0, 0));
        si.table.insert_unique(hash_str(""), 0, |_| hash_str(""));
        si
    }

    /// Resolve an `(offset, len)` span to a buffer slice.
    #[inline]
    fn slice(&self, idx: u32) -> &str {
        let (off, len) = self.spans[idx as usize];
        &self.buffer[off as usize..(off + len) as usize]
    }

    /// Intern a string, returning its `Atom`. Idempotent.
    ///
    /// Returns `Atom::EMPTY` if the interner is at capacity.
    pub fn intern(&mut self, s: &str) -> Atom {
        let h = hash_str(s);

        // Look up existing entry.
        let spans = &self.spans;
        let buffer: &str = &self.buffer;
        if let Some(&idx) = self.table.find(h, |&idx| {
            let (off, len) = spans[idx as usize];
            &buffer[off as usize..(off + len) as usize] == s
        }) {
            return Atom(idx);
        }

        if self.spans.len() >= Self::MAX_STRINGS
            || self.buffer.len() + s.len() > Self::MAX_BUFFER_BYTES
        {
            return Atom::EMPTY;
        }

        // Append to buffer and record span.
        let idx = self.spans.len() as u32;
        // Guard against u32 overflow on cumulative buffer length.
        let offset = u32::try_from(self.buffer.len()).unwrap_or(u32::MAX);
        if offset == u32::MAX {
            return Atom::EMPTY;
        }
        let slen = u32::try_from(s.len()).unwrap_or(u32::MAX);
        if slen == u32::MAX {
            return Atom::EMPTY;
        }
        self.buffer.push_str(s);
        self.spans.push((offset, slen));

        // Insert into hash table (reborrow after mutation).
        let spans = &self.spans;
        let buffer: &str = &self.buffer;
        self.table.insert_unique(h, idx, |&i| {
            let (off, len) = spans[i as usize];
            hash_str(&buffer[off as usize..(off + len) as usize])
        });

        Atom(idx)
    }

    /// Look up an already-interned string, returning its `Atom` or `Atom::EMPTY` if not found.
    #[must_use]
    pub fn lookup(&self, s: &str) -> Atom {
        let h = hash_str(s);
        let spans = &self.spans;
        let buffer: &str = &self.buffer;
        self.table
            .find(h, |&idx| {
                let (off, len) = spans[idx as usize];
                &buffer[off as usize..(off + len) as usize] == s
            })
            .copied()
            .map_or(Atom::EMPTY, Atom)
    }

    /// Resolve an `Atom` back to its string slice.
    #[inline]
    #[must_use]
    pub fn get(&self, atom: Atom) -> &str {
        debug_assert!(
            (atom.0 as usize) < self.spans.len(),
            "StringInterner::get: Atom({}) out of bounds (len={})",
            atom.0,
            self.spans.len()
        );
        self.slice(atom.0)
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
        assert_eq!(si.get(a), "hello");
    }

    #[test]
    fn empty_atom() {
        let si = StringInterner::new();
        assert_eq!(si.get(Atom::EMPTY), "");
    }

    #[test]
    fn distinct_strings() {
        let mut si = StringInterner::new();
        let a = si.intern("foo");
        let b = si.intern("bar");
        assert_ne!(a, b);
        assert_eq!(si.get(a), "foo");
        assert_eq!(si.get(b), "bar");
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
    fn contiguous_buffer() {
        let mut si = StringInterner::new();
        si.intern("abc");
        si.intern("def");
        si.intern("ghi");
        // All strings share the same contiguous buffer
        assert!(si.buffer.contains("abc"));
        assert!(si.buffer.contains("def"));
        assert!(si.buffer.contains("ghi"));
    }
}
