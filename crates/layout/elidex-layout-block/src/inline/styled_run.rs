//! `StyledRun` — a segment of text with its originating element's style, and the
//! [`InlineItem`] enum that carries it through an inline formatting context.
//!
//! CSS 2 §9.2.2 "Inline-level elements and inline boxes" is what governs these types.
//! Not §9.4.2: that governs line-box formation, and `StyledRun` / `InlineItem` are its
//! INPUT — collected before packing — so citing it here would claim a scope the module
//! does not have.

use elidex_ecs::Entity;
use elidex_plugin::{ComputedStyle, TextTransform, WhiteSpace};
use elidex_text::{to_fontdb_style, FontStyle, TextMeasureParams};

/// An item in an inline formatting context — either text or an atomic inline box.
pub(crate) enum InlineItem {
    /// A text run with per-element style.
    Text(StyledRun),
    /// An atomic inline-level box (e.g. `inline-block`, replaced element).
    /// The entity has already been laid out; its dimensions are used as-is.
    Atomic {
        entity: Entity,
        /// Inline-axis size (width for horizontal).
        inline_size: f32,
        /// Block-axis size (height for horizontal).
        block_size: f32,
        /// Which render-run-group this atomic's `AtomicBox` member persists under
        /// (see [`StyledRun::group_key`]). `None` = not recorded.
        /// Ignored when `positioned` (a positioned atomic is never a flow member).
        group_key: Option<Entity>,
        /// `true` for a `position:relative`/`sticky` atomic. Such an atomic advances
        /// the IFC line cursor (in-flow, CSS 2 §9.4.3) but is painted in render's
        /// Layer 6 from its own `LayoutBox`, so it is NOT recorded as an
        /// [`InlineFlowRun::AtomicBox`] flow member (that would double-paint —
        /// `emit_inline_flow` walks every member in Layer 5 AND Layer 6 walks the
        /// positioned box). Instead `LinePacker` records its on-line position in a
        /// separate per-pass bucket and layout repositions its `LayoutBox`
        /// preserving the applied relative offset (slice 3p-b-2).
        ///
        /// [`InlineFlowRun::AtomicBox`]: elidex_ecs::InlineFlowRun
        positioned: bool,
    },
    /// Absolutely positioned element placeholder (zero-width, zero-height).
    /// Used to record static position for CSS 2.1 §10.3.7 / §10.6.4.
    Placeholder(Entity),
}

/// A segment of text within an inline formatting context, preserving the
/// originating element's style for measurement.
pub struct StyledRun {
    /// Entity this text belongs to (element or pseudo-element).
    pub entity: Entity,
    /// The text content.
    pub text: String,
    /// Font families for measurement.
    pub families: Vec<String>,
    /// Font size in px.
    pub font_size: f32,
    /// Font weight (100–900).
    pub font_weight: u16,
    /// Font style (Normal/Italic/Oblique).
    pub font_style: FontStyle,
    /// Letter spacing in px.
    pub letter_spacing: f32,
    /// Word spacing in px.
    pub word_spacing: f32,
    /// Resolved line height in px.
    pub line_height: f32,
    /// CSS `white-space` (drives §4.1.1 collapsing / segment-break handling).
    pub white_space: WhiteSpace,
    /// CSS `text-transform` (CSS Text 3 §2.1). Applied to `text` *after* §4.1.1
    /// collapse and *before* measuring/packing (§2.1.2 order of operations), so
    /// the persisted positions are for the final transformed glyphs and render
    /// paints `text` verbatim (no re-transform).
    pub text_transform: TextTransform,
    /// Which render-run-group this run's `InlineFlow` text member persists under:
    /// the run-start entity render reads `InlineFlow` off (its `run[0]`) for the
    /// `emit_inline_run` that paints this group — the IFC parent's first eligible
    /// child (top-level) or a `position:relative`/`sticky` inline's first eligible
    /// child (its Layer-6 sub-flow). `None` = not recorded into any flow (e.g. a
    /// positioned subtree with a direct block child → anonymous-block-in-inline,
    /// left to render's legacy path; CSS 2 §9.2.1.1).
    pub group_key: Option<Entity>,
}

impl StyledRun {
    /// Create a run from text content and a computed style.
    pub(super) fn from_style(
        entity: Entity,
        text: String,
        style: &ComputedStyle,
        group_key: Option<Entity>,
    ) -> Self {
        Self {
            entity,
            text,
            families: style.font_family.clone(),
            font_size: style.font_size,
            font_weight: style.font_weight,
            font_style: to_fontdb_style(style.font_style),
            letter_spacing: style.letter_spacing.unwrap_or(0.0),
            word_spacing: style.word_spacing.unwrap_or(0.0),
            line_height: style.line_height.resolve_px(style.font_size),
            white_space: style.white_space,
            text_transform: style.text_transform,
            group_key,
        }
    }

    /// Build `TextMeasureParams` borrowing from the given families slice.
    pub(crate) fn measure_params<'a>(&self, families: &'a [&'a str]) -> TextMeasureParams<'a> {
        TextMeasureParams {
            families,
            font_size: self.font_size,
            weight: self.font_weight,
            style: self.font_style,
            letter_spacing: self.letter_spacing,
            word_spacing: self.word_spacing,
        }
    }

    /// Collect family name references for use with `measure_params`.
    pub(crate) fn family_refs(&self) -> Vec<&str> {
        self.families.iter().map(String::as_str).collect()
    }
}
