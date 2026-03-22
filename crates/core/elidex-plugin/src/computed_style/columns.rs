//! CSS Multi-column keyword enums and helpers.

use std::fmt;

use super::{ComputedStyle, Dimension, Display, WritingMode};

keyword_enum! {
    /// The CSS `column-fill` property (CSS Multi-column Layout Level 1 §7).
    ColumnFill {
        Balance => "balance",
        Auto => "auto",
    }
}

keyword_enum! {
    /// The CSS `column-span` property (CSS Multi-column Layout Level 1 §6).
    ColumnSpan {
        None => "none",
        All => "all",
    }
}

/// Check if an element is a multicol container.
///
/// CSS Multi-column L1 §2: "A multi-column container is established by …
/// a block container." Flex/Grid with column properties do NOT create multicol.
#[must_use]
pub fn is_multicol(style: &ComputedStyle) -> bool {
    matches!(
        style.display,
        Display::Block
            | Display::ListItem
            | Display::InlineBlock
            | Display::TableCell
            | Display::TableCaption
    ) && (style.column_count.is_some() || !matches!(style.column_width, Dimension::Auto))
}

/// Resolved multicol geometry, attached to multicol containers for rendering.
///
/// Stores per-segment info so column rules are drawn only between columns
/// that both have content, and not in spanner areas.
#[derive(Clone, Debug)]
pub struct MulticolInfo {
    /// Actual column width (inline extent per column).
    pub column_width: f32,
    /// Resolved column gap.
    pub column_gap: f32,
    /// Writing mode of the multicol container.
    pub writing_mode: WritingMode,
    /// Per-segment info for column rule rendering.
    ///
    /// Each entry: `(actual_column_count, segment_start, segment_extent)`.
    /// `segment_start` and `segment_extent` are in the block-axis direction.
    /// Spanner segments are excluded (rules are not drawn in spanner areas).
    pub segments: Vec<(u32, f32, f32)>,
}
