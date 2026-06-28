//! Unit tests for the canonical [`splice_utf16`](crate::dom::splice_utf16)
//! UTF-16 splice helper (moved here from `elidex-dom-api` per B1.3-i §2.1).

use crate::dom::splice_utf16;

#[test]
fn mid_splice_replaces_span() {
    // "hello" replace offset 1, count 3 ("ell") with "XY" → "hXYo".
    assert_eq!(splice_utf16("hello", 1, 3, Some("XY")), "hXYo");
}

#[test]
fn append_at_len_inserts_tail() {
    // appendData shape: (len, 0, Some("!")).
    let s = "hello";
    let len = s.encode_utf16().count();
    assert_eq!(splice_utf16(s, len, 0, Some("!")), "hello!");
}

#[test]
fn delete_removes_span() {
    // deleteData shape: (offset, count, None).
    assert_eq!(splice_utf16("hello", 1, 3, None), "ho");
}

#[test]
fn count_clamped_to_length() {
    // §4.10 "replace data" step 3 silent clamp: offset+count > length → length.
    assert_eq!(splice_utf16("hello", 2, 100, Some("X")), "heX");
}

#[test]
fn surrogate_pair_split_is_lossy_not_panic() {
    // U+1F600 (😀) is a surrogate pair (2 UTF-16 code units). Deleting just
    // the high surrogate (offset 0, count 1) leaves a lone low surrogate,
    // which cannot be stored in a Rust String and renders as U+FFFD by
    // from_utf16_lossy rather than panicking.
    let emoji = "\u{1F600}";
    let out = splice_utf16(emoji, 0, 1, None);
    assert_eq!(out, "\u{FFFD}");
}
