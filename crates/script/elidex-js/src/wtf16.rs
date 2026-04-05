//! WTF-16 string interner and utilities.

use std::hash::{Hash, Hasher};

use hashbrown::HashTable;

/// WTF-16 string interner with contiguous buffer and span-based indexing.
#[derive(Debug)]
pub struct Wtf16Interner {
    buffer: Vec<u16>,
    spans: Vec<(u32, u32)>, // (offset, length) in u16 units
    table: HashTable<u32>,
}

fn hash_u16(s: &[u16]) -> u64 {
    let mut h = std::hash::DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

impl Wtf16Interner {
    pub const MAX_STRINGS: usize = 16 * 1024 * 1024;
    const MAX_BUFFER_UNITS: usize = 128 * 1024 * 1024; // 256 MiB worth of u16

    pub fn new() -> Self {
        let mut si = Self {
            buffer: Vec::with_capacity(4096),
            spans: Vec::with_capacity(256),
            table: HashTable::with_capacity(256),
        };
        // Index 0 = empty string
        si.spans.push((0, 0));
        si.table.insert_unique(hash_u16(&[]), 0, |_| hash_u16(&[]));
        si
    }

    fn slice(&self, idx: u32) -> &[u16] {
        let (off, len) = self.spans[idx as usize];
        &self.buffer[off as usize..(off + len) as usize]
    }

    /// Intern from raw WTF-16 slice.
    pub fn intern_wtf16(&mut self, units: &[u16]) -> u32 {
        let h = hash_u16(units);
        let spans = &self.spans;
        let buffer: &[u16] = &self.buffer;
        if let Some(&idx) = self.table.find(h, |&idx| {
            let (off, len) = spans[idx as usize];
            &buffer[off as usize..(off + len) as usize] == units
        }) {
            return idx;
        }

        if self.spans.len() >= Self::MAX_STRINGS
            || self.buffer.len() + units.len() > Self::MAX_BUFFER_UNITS
        {
            return 0; // empty string fallback
        }

        let idx = self.spans.len() as u32;
        let offset = u32::try_from(self.buffer.len()).unwrap_or(u32::MAX);
        if offset == u32::MAX {
            return 0;
        }
        let slen = u32::try_from(units.len()).unwrap_or(u32::MAX);
        if slen == u32::MAX {
            return 0;
        }
        self.buffer.extend_from_slice(units);
        self.spans.push((offset, slen));

        let spans = &self.spans;
        let buffer: &[u16] = &self.buffer;
        self.table.insert_unique(h, idx, |&i| {
            let (off, len) = spans[i as usize];
            hash_u16(&buffer[off as usize..(off + len) as usize])
        });

        idx
    }

    /// Intern from UTF-8 &str (convenience -- converts internally).
    pub fn intern(&mut self, s: &str) -> u32 {
        let units: Vec<u16> = s.encode_utf16().collect();
        self.intern_wtf16(&units)
    }

    /// Get WTF-16 content for an interned string.
    #[inline]
    pub fn get(&self, id: u32) -> &[u16] {
        debug_assert!(
            (id as usize) < self.spans.len(),
            "Wtf16Interner::get: id {} out of bounds (len={})",
            id,
            self.spans.len()
        );
        self.slice(id)
    }

    /// Get as UTF-8 String (lossy for lone surrogates).
    pub fn get_utf8(&self, id: u32) -> String {
        String::from_utf16_lossy(self.get(id))
    }

    /// Look up an already-interned WTF-16 string without inserting.
    pub fn lookup_wtf16(&self, units: &[u16]) -> Option<u32> {
        let h = hash_u16(units);
        let spans = &self.spans;
        let buffer: &[u16] = &self.buffer;
        self.table
            .find(h, |&idx| {
                let (off, len) = spans[idx as usize];
                &buffer[off as usize..(off + len) as usize] == units
            })
            .copied()
    }

    /// Number of interned strings.
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.spans.len()
    }
}

impl Default for Wtf16Interner {
    fn default() -> Self {
        Self::new()
    }
}

// -- UTF-16 string operation helpers ------------------------------------------

/// Naive substring search on WTF-16 slices.
pub fn find_u16(haystack: &[u16], needle: &[u16]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    if needle.len() > haystack.len() {
        return None;
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// Check if haystack starts with needle at the given UTF-16 offset.
pub fn starts_with_u16(haystack: &[u16], needle: &[u16], offset: usize) -> bool {
    haystack
        .get(offset..offset + needle.len())
        .is_some_and(|s| s == needle)
}

/// Check if haystack[..end] ends with needle.
pub fn ends_with_u16(haystack: &[u16], needle: &[u16], end: usize) -> bool {
    end >= needle.len()
        && haystack
            .get(end - needle.len()..end)
            .is_some_and(|s| s == needle)
}

/// ES2020 whitespace check on a single UTF-16 code unit.
pub fn is_js_whitespace(u: u16) -> bool {
    matches!(
        u,
        0x0009 | 0x000A | 0x000B | 0x000C | 0x000D | 0x0020 | 0x00A0 | 0x1680 | 0x2000
            ..=0x200A | 0x2028 | 0x2029 | 0x202F | 0x205F | 0x3000 | 0xFEFF
    )
}

/// Trim leading and trailing JS whitespace from a WTF-16 slice.
pub fn trim_u16(s: &[u16]) -> &[u16] {
    let start = s
        .iter()
        .position(|&u| !is_js_whitespace(u))
        .unwrap_or(s.len());
    let end = s
        .iter()
        .rposition(|&u| !is_js_whitespace(u))
        .map_or(start, |i| i + 1);
    &s[start..end]
}

/// Apply a per-char case mapping to a WTF-16 slice.
/// Surrogate pairs and lone surrogates are preserved unchanged.
fn case_map_u16<I: Iterator<Item = char>>(units: &[u16], map: impl Fn(char) -> I) -> Vec<u16> {
    let mut result = Vec::with_capacity(units.len());
    let mut i = 0;
    while i < units.len() {
        let u = units[i];
        if (0xD800..=0xDBFF).contains(&u) {
            // High surrogate -- copy pair as-is (no case mapping for supplementary chars)
            result.push(u);
            if i + 1 < units.len() && (0xDC00..=0xDFFF).contains(&units[i + 1]) {
                // Valid surrogate pair — copy both
                result.push(units[i + 1]);
                i += 2;
            } else {
                // Lone high surrogate — advance by 1 only
                i += 1;
            }
        } else if (0xDC00..=0xDFFF).contains(&u) {
            // Lone low surrogate -- preserve
            result.push(u);
            i += 1;
        } else {
            // BMP character -- case map via char
            if let Some(ch) = char::from_u32(u32::from(u)) {
                let mut buf = [0u16; 2];
                for mapped in map(ch) {
                    let encoded = mapped.encode_utf16(&mut buf);
                    result.extend_from_slice(encoded);
                }
            } else {
                result.push(u);
            }
            i += 1;
        }
    }
    result
}

/// toLowerCase on WTF-16. Surrogate pairs/lone surrogates are preserved.
pub fn to_lower_u16(units: &[u16]) -> Vec<u16> {
    case_map_u16(units, char::to_lowercase)
}

/// toUpperCase on WTF-16. Surrogate pairs/lone surrogates are preserved.
pub fn to_upper_u16(units: &[u16]) -> Vec<u16> {
    case_map_u16(units, char::to_uppercase)
}
