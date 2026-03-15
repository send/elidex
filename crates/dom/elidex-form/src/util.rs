//! Shared character boundary utilities for form controls.

/// Find the byte offset of the previous character boundary.
#[must_use]
pub fn prev_char_boundary(s: &str, pos: usize) -> usize {
    let mut p = pos.min(s.len()).saturating_sub(1);
    while p > 0 && !s.is_char_boundary(p) {
        p -= 1;
    }
    p
}

/// Find the byte offset of the next character boundary.
#[must_use]
pub fn next_char_boundary(s: &str, pos: usize) -> usize {
    let mut p = pos.min(s.len()).saturating_add(1);
    while p < s.len() && !s.is_char_boundary(p) {
        p += 1;
    }
    p.min(s.len())
}

/// Snap a byte position to the nearest valid char boundary (rounding down).
#[must_use]
pub fn snap_to_char_boundary(s: &str, pos: usize) -> usize {
    let pos = pos.min(s.len());
    let mut p = pos;
    while p > 0 && !s.is_char_boundary(p) {
        p -= 1;
    }
    p
}

/// Convert a byte offset to a UTF-16 code unit offset.
///
/// Each char in the string contributes 1 or 2 UTF-16 code units (surrogate pairs
/// for chars above U+FFFF). Returns the UTF-16 offset corresponding to the given
/// byte position.
#[must_use]
pub fn byte_offset_to_utf16(s: &str, byte_pos: usize) -> usize {
    let byte_pos = snap_to_char_boundary(s, byte_pos);
    s[..byte_pos].chars().map(char::len_utf16).sum()
}

/// Convert a UTF-16 code unit offset to a byte offset.
///
/// Returns the byte position corresponding to the given UTF-16 code unit index.
/// If `utf16_pos` exceeds the string length in UTF-16 units, returns `s.len()`.
#[must_use]
pub fn utf16_to_byte_offset(s: &str, utf16_pos: usize) -> usize {
    let mut utf16_count = 0;
    for (byte_idx, ch) in s.char_indices() {
        if utf16_count >= utf16_pos {
            return byte_idx;
        }
        utf16_count += ch.len_utf16();
    }
    s.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf16_ascii() {
        let s = "hello";
        assert_eq!(byte_offset_to_utf16(s, 0), 0);
        assert_eq!(byte_offset_to_utf16(s, 3), 3);
        assert_eq!(byte_offset_to_utf16(s, 5), 5);
        assert_eq!(utf16_to_byte_offset(s, 0), 0);
        assert_eq!(utf16_to_byte_offset(s, 3), 3);
        assert_eq!(utf16_to_byte_offset(s, 5), 5);
    }

    #[test]
    fn utf16_bmp_chars() {
        // "あいう" — each char is 3 bytes UTF-8 and 1 UTF-16 code unit.
        let s = "あいう";
        assert_eq!(s.len(), 9); // 3 chars × 3 bytes
        assert_eq!(byte_offset_to_utf16(s, 0), 0);
        assert_eq!(byte_offset_to_utf16(s, 3), 1); // after 'あ'
        assert_eq!(byte_offset_to_utf16(s, 6), 2); // after 'い'
        assert_eq!(byte_offset_to_utf16(s, 9), 3); // after 'う'

        assert_eq!(utf16_to_byte_offset(s, 0), 0);
        assert_eq!(utf16_to_byte_offset(s, 1), 3);
        assert_eq!(utf16_to_byte_offset(s, 2), 6);
        assert_eq!(utf16_to_byte_offset(s, 3), 9);
    }

    #[test]
    fn utf16_surrogate_pair() {
        // U+20BB7 (𠮷) — 4 bytes UTF-8, 2 UTF-16 code units (surrogate pair).
        let s = "𠮷";
        assert_eq!(s.len(), 4);
        assert_eq!(byte_offset_to_utf16(s, 0), 0);
        assert_eq!(byte_offset_to_utf16(s, 4), 2); // 2 UTF-16 code units

        assert_eq!(utf16_to_byte_offset(s, 0), 0);
        assert_eq!(utf16_to_byte_offset(s, 2), 4);
    }

    #[test]
    fn utf16_mixed() {
        // "a𠮷b" — 'a' (1 byte, 1 unit), '𠮷' (4 bytes, 2 units), 'b' (1 byte, 1 unit)
        let s = "a𠮷b";
        assert_eq!(s.len(), 6);
        assert_eq!(byte_offset_to_utf16(s, 0), 0); // before 'a'
        assert_eq!(byte_offset_to_utf16(s, 1), 1); // after 'a'
        assert_eq!(byte_offset_to_utf16(s, 5), 3); // after '𠮷' (1 + 2)
        assert_eq!(byte_offset_to_utf16(s, 6), 4); // after 'b'

        assert_eq!(utf16_to_byte_offset(s, 0), 0);
        assert_eq!(utf16_to_byte_offset(s, 1), 1);
        assert_eq!(utf16_to_byte_offset(s, 3), 5);
        assert_eq!(utf16_to_byte_offset(s, 4), 6);
    }

    #[test]
    fn utf16_beyond_end() {
        let s = "hi";
        assert_eq!(byte_offset_to_utf16(s, 100), 2);
        assert_eq!(utf16_to_byte_offset(s, 100), 2);
    }
}
