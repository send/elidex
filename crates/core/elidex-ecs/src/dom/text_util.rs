//! Pure UTF-16 text-splice primitive shared across the DOM stack.
//!
//! Lives in `elidex-ecs` (the lowest crate that needs it) so both
//! [`EcsDom::replace_text_data`](super::EcsDom::replace_text_data) /
//! [`EcsDom::replace_comment_data`](super::EcsDom::replace_comment_data)
//! and the engine-independent
//! `elidex_script_session::apply_replace_data` /
//! `elidex_dom_api` CharacterData handlers call the single canonical
//! helper rather than each carrying their own splice loop.

/// Splice a UTF-16 view of `original` and return the result as a Rust
/// `String`.
///
/// **Caller contract**: `offset` MUST be ≤ the UTF-16 length of
/// `original`. This helper is **not** a spec-validating primitive —
/// the WHATWG DOM "replace data" algorithm (§4.10, `#concept-cd-replace`)
/// requires `offset > length` to raise `IndexSizeError` (step 2), and
/// that check lives in every caller (`InsertData` / `DeleteData` /
/// `ReplaceData` / `SubstringData`). Adding a new caller? Validate
/// `offset` first. Debug builds enforce the contract via `debug_assert!`;
/// release builds rely on the slice indexing below to panic on violation
/// rather than silently clamp.
///
/// `count` IS clamped to `len - offset` to match the spec's silent
/// clamp (§4.10 "replace data" step 3: "if offset + count is greater
/// than length, set count to length − offset"). `replacement` is `None`
/// for delete, `Some` for replace / insert / append.
///
/// Splitting through a surrogate pair (offset / end mid-pair) is
/// **spec-valid** — UTF-16 offsets ignore character boundaries — and
/// produces lone surrogates in the intermediate `Vec<u16>`. Rust's
/// `String` storage cannot represent lone surrogates, so the result is
/// rendered through `from_utf16_lossy` which substitutes `U+FFFD` for
/// each unpaired half. This intentionally degrades into a known-lossy
/// shape rather than panicking; matches the pre-arch-hoist VM-side
/// behaviour and the lossy-not-panic contract pinned by
/// `tests_character_data::*surrogate_pair*`.
#[must_use]
pub(crate) fn splice_utf16(
    original: &str,
    offset: usize,
    count: usize,
    replacement: Option<&str>,
) -> String {
    let units: Vec<u16> = original.encode_utf16().collect();
    let len = units.len();
    debug_assert!(
        offset <= len,
        "splice_utf16: offset {offset} exceeds UTF-16 length {len}; caller must \
         validate via `if offset > utf16_len(&data)` before invocation"
    );
    let end = offset.saturating_add(count).min(len);
    // Capacity hint via a count pass (no intermediate Vec); the replacement
    // is streamed directly into `out` to avoid an extra allocation per splice.
    let replacement_len = replacement.map_or(0, |r| r.encode_utf16().count());
    let mut out: Vec<u16> = Vec::with_capacity(len - (end - offset) + replacement_len);
    out.extend_from_slice(&units[..offset]);
    if let Some(rep) = replacement {
        out.extend(rep.encode_utf16());
    }
    out.extend_from_slice(&units[end..]);
    String::from_utf16_lossy(&out)
}
