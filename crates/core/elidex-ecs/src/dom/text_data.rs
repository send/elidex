//! The `EcsDom` CharacterData text-data write cluster.
//!
//! Extracted from `dom/mod.rs` (which is over the 1000-line review
//! convention) so the text-data write chokepoints live in a focused
//! module — the CLAUDE.md "1000-line debt = touch-time split" seam for
//! the CharacterData data path. These are the sanctioned `TextContent` /
//! `CommentData` data-mutation entry points:
//! [`set_text_data`](EcsDom::set_text_data) (whole-string Text write,
//! fires [`MutationEvent::TextChange`]),
//! [`replace_text_data`](EcsDom::replace_text_data) (Text UTF-16 splice,
//! fires [`MutationEvent::ReplaceData`]), and
//! [`replace_comment_data`](EcsDom::replace_comment_data) (Comment UTF-16
//! splice, no event). All splice math delegates to the canonical
//! [`splice_utf16`](super::splice_utf16) helper. Rust permits the
//! inherent `impl EcsDom` to be split across files in the same module.

use hecs::Entity;

use super::{splice_utf16, EcsDom, MutationEvent};
use crate::components::{CommentData, TextContent};

impl EcsDom {
    /// Replace the `TextContent` of an entity. Returns the new UTF-16 length
    /// on success, or `None` if the entity has no `TextContent` component.
    ///
    /// On success, bumps [`Self::rev_version`] for `entity` (the canonical
    /// cache-invalidation step per the version-tracking docs above) and
    /// fires `after_text_change` on the mutation hook (if installed). This
    /// makes `set_text_data` self-contained: callers do not need to
    /// `rev_version` themselves after.
    ///
    /// This is the canonical write path for **Text / CData** mutations.
    /// `CharacterData` handlers in `elidex-dom-api` route `TextContent`
    /// updates through this method to ensure Range live-tracking hook fire
    /// consistency.
    ///
    /// Takes `&str` and uses [`str::clone_into`] so the existing
    /// `TextContent` buffer's capacity is reused — frequent CharacterData
    /// updates do not re-allocate.
    ///
    /// **NOT for Comment nodes** — Comment uses a separate `CommentData`
    /// component. Comment data writes go through
    /// [`Self::replace_comment_data`], which fires no live-range event:
    /// per the spec (§4.10 "replace data" steps 8-11) range fixup applies
    /// to ALL CharacterData, but elidex's range-fixup hook is wired for
    /// Text / CDATASection only (an implementation limitation, not a
    /// spec restriction).
    pub fn set_text_data(&mut self, entity: Entity, text: &str) -> Option<usize> {
        let new_utf16_len = {
            let mut tc = self.world.get::<&mut TextContent>(entity).ok()?;
            let len = text.encode_utf16().count();
            text.clone_into(&mut tc.0);
            len
        };
        self.rev_version(entity);
        let event = MutationEvent::TextChange {
            node: entity,
            new_utf16_len,
        };
        self.dispatch_event(&event);
        Some(new_utf16_len)
    }

    /// Primitive UTF-16 splice on a Text / CData entity's `TextContent`
    /// (WHATWG DOM §4.10 "replace data" steps 1-7 storage mutation,
    /// step 8-11 boundary adjustment is the hook consumer's
    /// responsibility). Returns the new UTF-16 length on success, or
    /// `None` if the entity has no `TextContent` component.
    ///
    /// **Bounds validation is the CALLER's responsibility** — this is
    /// the engine-level splice primitive that `CharacterData` handlers
    /// in `elidex-dom-api` (`appendData` / `insertData` / `deleteData`
    /// / `replaceData`) route through after raising `IndexSizeError`
    /// for `offset > utf16_len`. `count` IS clamped to `len - offset`
    /// here to match the spec's silent clamp (§4.10 "replace data"
    /// step 3: "if offset + count is greater than length, set count to
    /// length − offset").
    ///
    /// Splitting through a surrogate pair (offset / end mid-pair) is
    /// **spec-valid** — UTF-16 offsets ignore character boundaries —
    /// and produces lone surrogates in the intermediate `Vec<u16>`.
    /// Rust's `String` storage cannot represent lone surrogates, so the
    /// result is rendered through `from_utf16_lossy` which substitutes
    /// `U+FFFD` for each unpaired half. This matches the lossy-not-panic
    /// contract pinned by `tests_character_data::*surrogate_pair*`. The
    /// splice itself delegates to the canonical `splice_utf16` helper.
    ///
    /// On success:
    /// - splices the UTF-16 view of `TextContent` in place,
    /// - bumps [`Self::rev_version`] (cache invalidation),
    /// - fires [`MutationEvent::ReplaceData`] with
    ///   `(entity, offset, count, replacement_utf16_len)`.
    ///
    /// **NOT for Comment nodes** (Comment uses `CommentData`, not
    /// covered by WHATWG §5.5 Range live-tracking).
    pub fn replace_text_data(
        &mut self,
        entity: Entity,
        offset_utf16: usize,
        count_utf16: usize,
        replacement: &str,
    ) -> Option<usize> {
        let replacement_len = replacement.encode_utf16().count();
        let (new_utf16_len, clamped_count) = {
            let mut tc = self.world.get::<&mut TextContent>(entity).ok()?;
            let len = tc.0.encode_utf16().count();
            debug_assert!(
                offset_utf16 <= len,
                "replace_text_data: offset {offset_utf16} exceeds UTF-16 length {len}; \
                 caller must validate via `offset > utf16_len(&data)` before invocation"
            );
            // Span actually spliced (clamped per §4.10 step 3) — still needed
            // below for the `MutationEvent::ReplaceData` live-range payload.
            let clamped_count = offset_utf16.saturating_add(count_utf16).min(len) - offset_utf16;
            // Delegate the UTF-16 splice itself to the single canonical helper.
            let spliced = splice_utf16(&tc.0, offset_utf16, count_utf16, Some(replacement));
            // New UTF-16 length is derivable arithmetically (`len - removed +
            // inserted`); avoid a third full encode_utf16 pass over `spliced`.
            // No underflow: `clamped_count <= len - offset <= len`.
            let new_len = len - clamped_count + replacement_len;
            spliced.clone_into(&mut tc.0);
            (new_len, clamped_count)
        };
        self.rev_version(entity);
        // WHATWG DOM §4.10 "replace data" clamps `count` to `length −
        // offset` (step 3); the live-range adjustment steps (8-11) then
        // operate on that clamped span (`end - offset`), not the caller's
        // possibly-overflowing `count_utf16`. Passing the unclamped
        // value would make `adjust_ranges_for_replace_data` treat
        // boundaries near the OLD end as inside the splice region
        // and collapse them to `offset` instead of shifting by
        // `new_data_len - clamped_count` — PR186 R3 #1 fix.
        let event = MutationEvent::ReplaceData {
            node: entity,
            offset_utf16,
            count_utf16: clamped_count,
            new_data_len_utf16: replacement_len,
        };
        self.dispatch_event(&event);
        Some(new_utf16_len)
    }

    /// Primitive UTF-16 splice on a Comment entity's `CommentData`
    /// (WHATWG DOM §4.10 "replace data"). The Comment sibling of
    /// [`Self::replace_text_data`]: splices the data in place, bumps
    /// [`Self::rev_version`], and fires the **same** `MutationEvent::ReplaceData`
    /// so [`crate::MutationEvent::ReplaceData`] live-range fixup applies. Per
    /// §4.10 the "replace data" live-range steps (8-11) apply to **all**
    /// CharacterData incl. Comment, and `LiveRangeBridge::after_replace_data`
    /// is container-agnostic (operates on `(node, offset, count, new_len)`),
    /// so a live range anchored in a Comment IS now adjusted on data change
    /// (Codex PR426 R1 — previously a Text/CDATASection-only limitation; the
    /// F5-broadened `deleteContents` Comment path made the gap reachable). No
    /// other consumer (style / a11y / layout) reacts to a Comment, but they
    /// already ignore `ReplaceData` for non-participating nodes.
    ///
    /// Returns the new UTF-16 length on success, or `None` if the entity
    /// has no `CommentData` component (the `Option` doubles as the
    /// Text-vs-Comment discriminator for callers that try
    /// `replace_text_data` first). Bounds validation is the caller's
    /// responsibility, as for [`Self::replace_text_data`]; `count` is
    /// clamped to `len - offset` inside `splice_utf16`.
    pub fn replace_comment_data(
        &mut self,
        entity: Entity,
        offset_utf16: usize,
        count_utf16: usize,
        replacement: &str,
    ) -> Option<usize> {
        let replacement_len = replacement.encode_utf16().count();
        let (new_utf16_len, clamped_count) = {
            let mut cd = self.world.get::<&mut CommentData>(entity).ok()?;
            let len = cd.0.encode_utf16().count();
            debug_assert!(
                offset_utf16 <= len,
                "replace_comment_data: offset {offset_utf16} exceeds UTF-16 length {len}; \
                 caller must validate via `offset > utf16_len(&data)` before invocation"
            );
            // Span actually spliced (clamped per §4.10 step 3) — the
            // `ReplaceData` live-range payload (steps 8-11 operate on the
            // clamped span, not the caller's possibly-overflowing `count`).
            let clamped_count = offset_utf16.saturating_add(count_utf16).min(len) - offset_utf16;
            let spliced = splice_utf16(&cd.0, offset_utf16, count_utf16, Some(replacement));
            // new_len = len − removed + inserted (avoid a third encode pass).
            let new_len = len - clamped_count + replacement_len;
            spliced.clone_into(&mut cd.0);
            (new_len, clamped_count)
        };
        self.rev_version(entity);
        let event = MutationEvent::ReplaceData {
            node: entity,
            offset_utf16,
            count_utf16: clamped_count,
            new_data_len_utf16: replacement_len,
        };
        self.dispatch_event(&event);
        Some(new_utf16_len)
    }
}
