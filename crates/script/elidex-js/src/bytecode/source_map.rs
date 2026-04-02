//! Source map for mapping bytecode offsets to source spans.

use crate::span::Span;

/// Source map for a single [`CompiledFunction`](super::CompiledFunction).
///
/// Maps bytecode offsets to source code spans. Entries are sorted by
/// `bytecode_offset` and looked up via binary search.
#[derive(Debug, Default, Clone)]
pub struct SourceMap {
    entries: Vec<SourceMapEntry>,
}

/// A single source map entry.
#[derive(Debug, Clone, Copy)]
pub struct SourceMapEntry {
    /// Byte offset into the bytecode `Vec`.
    pub bytecode_offset: u32,
    /// Source span (byte offsets into original source text).
    pub span: Span,
}

impl SourceMap {
    /// Create an empty source map.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Add an entry. Entries should be added in order of bytecode emission,
    /// but duplicate offsets are deduplicated (last wins).
    pub fn add(&mut self, bytecode_offset: u32, span: Span) {
        if let Some(last) = self.entries.last() {
            if last.bytecode_offset == bytecode_offset {
                // Update existing entry at same offset.
                self.entries.last_mut().unwrap().span = span;
                return;
            }
            // Skip if span is identical to previous (reduces entries).
            if last.span == span {
                return;
            }
        }
        self.entries.push(SourceMapEntry {
            bytecode_offset,
            span,
        });
    }

    /// Look up the source span for a bytecode offset.
    ///
    /// Returns the span of the entry whose `bytecode_offset` is ≤ the given
    /// offset (the instruction that "owns" that bytecode position).
    #[must_use]
    pub fn lookup(&self, bytecode_offset: u32) -> Option<Span> {
        let idx = self
            .entries
            .partition_point(|e| e.bytecode_offset <= bytecode_offset);
        if idx == 0 {
            None
        } else {
            Some(self.entries[idx - 1].span)
        }
    }

    /// Number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the source map is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_lookup() {
        let sm = SourceMap::new();
        assert!(sm.lookup(0).is_none());
    }

    #[test]
    fn single_entry() {
        let mut sm = SourceMap::new();
        sm.add(0, Span::new(10, 20));
        assert_eq!(sm.lookup(0), Some(Span::new(10, 20)));
        assert_eq!(sm.lookup(5), Some(Span::new(10, 20)));
    }

    #[test]
    fn multiple_entries() {
        let mut sm = SourceMap::new();
        sm.add(0, Span::new(0, 5));
        sm.add(3, Span::new(10, 15));
        sm.add(8, Span::new(20, 30));

        assert_eq!(sm.lookup(0), Some(Span::new(0, 5)));
        assert_eq!(sm.lookup(2), Some(Span::new(0, 5)));
        assert_eq!(sm.lookup(3), Some(Span::new(10, 15)));
        assert_eq!(sm.lookup(7), Some(Span::new(10, 15)));
        assert_eq!(sm.lookup(8), Some(Span::new(20, 30)));
        assert_eq!(sm.lookup(100), Some(Span::new(20, 30)));
    }

    #[test]
    fn dedup_same_offset() {
        let mut sm = SourceMap::new();
        sm.add(0, Span::new(0, 5));
        sm.add(0, Span::new(10, 15)); // overwrites
        assert_eq!(sm.len(), 1);
        assert_eq!(sm.lookup(0), Some(Span::new(10, 15)));
    }

    #[test]
    fn dedup_same_span() {
        let mut sm = SourceMap::new();
        sm.add(0, Span::new(0, 5));
        sm.add(3, Span::new(0, 5)); // same span, skipped
        assert_eq!(sm.len(), 1);
    }
}
