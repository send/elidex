//! Source span (byte offsets) for AST nodes and tokens.

/// A byte-offset range in the source text.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    /// Create a new span.
    #[must_use]
    pub fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    /// A zero-width span at a position.
    #[must_use]
    pub fn empty(pos: u32) -> Self {
        Self {
            start: pos,
            end: pos,
        }
    }

    /// Merge two spans into the smallest span covering both.
    #[must_use]
    pub fn merge(self, other: Self) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    /// Length in bytes.
    #[must_use]
    pub fn len(self) -> u32 {
        debug_assert!(self.start <= self.end, "span start > end");
        self.end.saturating_sub(self.start)
    }

    /// Whether this span is zero-width.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.start == self.end
    }
}

impl std::fmt::Debug for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

/// Line/column location computed on demand from byte offset + `line_starts` table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceLocation {
    /// 1-based line number.
    pub line: u32,
    /// 0-based column (byte offset from line start).
    pub column: u32,
}

impl SourceLocation {
    /// Compute location from byte offset and a sorted `line_starts` table.
    #[must_use]
    pub fn from_offset(offset: u32, line_starts: &[u32]) -> Self {
        if line_starts.is_empty() {
            return Self {
                line: 1,
                column: offset,
            };
        }
        let line_idx = match line_starts.binary_search(&offset) {
            Ok(i) => i,
            Err(i) => i.saturating_sub(1),
        };
        let column = offset.saturating_sub(line_starts[line_idx]);
        Self {
            line: (line_idx as u32) + 1,
            column,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_basics() {
        let s = Span::new(5, 10);
        assert_eq!(s.len(), 5);
        assert!(!s.is_empty());

        let e = Span::empty(3);
        assert_eq!(e.len(), 0);
        assert!(e.is_empty());
    }

    #[test]
    fn span_merge() {
        let a = Span::new(2, 5);
        let b = Span::new(8, 12);
        let m = a.merge(b);
        assert_eq!(m, Span::new(2, 12));
    }

    #[test]
    fn source_location() {
        // "ab\ncd\nef"  line_starts = [0, 3, 6]
        let starts = &[0u32, 3, 6];
        assert_eq!(
            SourceLocation::from_offset(0, starts),
            SourceLocation { line: 1, column: 0 }
        );
        assert_eq!(
            SourceLocation::from_offset(4, starts),
            SourceLocation { line: 2, column: 1 }
        );
        assert_eq!(
            SourceLocation::from_offset(6, starts),
            SourceLocation { line: 3, column: 0 }
        );
    }
}
