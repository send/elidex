//! Computed style representation for resolved CSS property values.
//!
//! [`ComputedStyle`] is an ECS component attached to every element after
//! style resolution. It contains fully resolved values for all supported
//! CSS properties.

use std::collections::HashMap;

use crate::background::BackgroundLayer;
use crate::{BackfaceVisibility, CssColor, EdgeSizes, TransformFunction, TransformStyle};

/// Define a CSS keyword enum with `Default` (first variant), `AsRef<str>`, and
/// `fmt::Display` implementations.
///
/// # Syntax
///
/// ```ignore
/// keyword_enum! {
///     /// Doc comment for the enum.
///     EnumName {
///         VariantA => "variant-a",
///         VariantB => "variant-b",
///     }
/// }
/// ```
///
/// The **first** variant automatically receives `#[default]`.
#[macro_export]
macro_rules! keyword_enum {
    (
        $( #[doc = $doc:expr] )*
        $name:ident {
            $first_variant:ident => $first_str:expr,
            $( $variant:ident => $str:expr, )*
        }
    ) => {
        $( #[doc = $doc] )*
        #[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
        pub enum $name {
            #[default]
            $first_variant,
            $( $variant, )*
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                match self {
                    Self::$first_variant => $first_str,
                    $( Self::$variant => $str, )*
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_ref())
            }
        }

        impl $name {
            /// Parse a CSS keyword string into this enum variant
            /// (case-insensitive).
            #[must_use]
            pub fn from_keyword(s: &str) -> Option<Self> {
                if s.eq_ignore_ascii_case($first_str) {
                    return Some(Self::$first_variant);
                }
                $( if s.eq_ignore_ascii_case($str) {
                    return Some(Self::$variant);
                } )*
                None
            }
        }
    };
}

mod box_model;
mod columns;
mod display;
mod flex;
mod float_visibility;
mod fragmentation;
mod grid;
mod table;
mod text;
mod writing_mode;

pub use box_model::{BorderSide, BorderStyle, BoxSizing, ContentItem, ContentValue, Dimension};
pub use columns::{ColumnFill, ColumnSpan};
pub use display::{Display, Overflow, Position, ViewportOverflow};
pub use flex::{
    AlignContent, AlignItems, AlignSelf, AlignmentSafety, FlexBasis, FlexDirection, FlexWrap,
    JustifyContent,
};
pub use float_visibility::{Clear, Float, VerticalAlign, Visibility};
pub use fragmentation::{BoxDecorationBreak, BreakInsideValue, BreakValue};
pub use grid::{
    validate_area_rectangles, AutoRepeatMode, GridAutoFlow, GridLine, GridTemplateAreas,
    GridTrackList, JustifyItems, JustifySelf, TrackBreadth, TrackSection, TrackSize,
};
pub use table::{BorderCollapse, CaptionSide, EmptyCells, TableLayout};
pub use text::{
    FontStyle, LineHeight, ListStyleType, TextAlign, TextDecorationLine, TextDecorationStyle,
    TextTransform, WhiteSpace,
};
pub use writing_mode::{Direction, TextOrientation, UnicodeBidi, WritingMode};

#[cfg(test)]
mod tests;

/// Fully resolved CSS property values for an element.
///
/// Attached as an ECS component by `elidex_style::resolve_styles()`.
/// All relative units have been resolved to absolute pixel values.
#[derive(Clone, Debug, PartialEq)]
#[allow(clippy::struct_excessive_bools)] // Stacking context flags are independent CSS conditions.
pub struct ComputedStyle {
    // --- Inherited properties ---
    /// Foreground color. Initial: black.
    pub color: CssColor,
    /// Font size in pixels. Initial: 16.0.
    pub font_size: f32,
    /// Font weight (100-900). Initial: 400 (normal).
    pub font_weight: u16,
    /// Font style. Initial: `Normal`.
    pub font_style: FontStyle,
    /// Font family list. Initial: `["serif"]`.
    pub font_family: Vec<String>,
    /// Line height. Initial: `Normal` (1.2 × font-size).
    pub line_height: LineHeight,
    /// Text transform. Initial: `None`.
    pub text_transform: TextTransform,
    /// Text alignment. Initial: `Left`.
    pub text_align: TextAlign,
    /// White-space handling. Initial: `Normal`.
    pub white_space: WhiteSpace,
    /// List style type. Initial: `Disc`.
    pub list_style_type: ListStyleType,
    /// Writing mode. Initial: `HorizontalTb`. **Inherited.**
    pub writing_mode: WritingMode,
    /// Text orientation. Initial: `Mixed`. **Inherited.**
    pub text_orientation: TextOrientation,
    /// Inline base direction. Initial: `Ltr`. **Inherited.**
    pub direction: Direction,
    /// Visibility. Initial: `Visible`. **Inherited.**
    pub visibility: Visibility,

    // --- Non-inherited properties ---
    /// Display type. Initial: Inline.
    pub display: Display,
    /// Positioning scheme. Initial: Static.
    pub position: Position,
    /// Bidi embedding control. Initial: `Normal`.
    pub unicode_bidi: UnicodeBidi,
    /// Background color. Initial: transparent.
    pub background_color: CssColor,
    /// Background image layers (CSS Backgrounds Level 3).
    /// `None` = no background images (zero-cost default).
    pub background_layers: Option<Box<[BackgroundLayer]>>,

    /// Overflow behavior on the x-axis. Initial: Visible.
    pub overflow_x: Overflow,
    /// Overflow behavior on the y-axis. Initial: Visible.
    pub overflow_y: Overflow,

    /// Content width. Initial: Auto.
    pub width: Dimension,
    /// Content height. Initial: Auto.
    pub height: Dimension,
    /// Minimum width. Initial: Length(0.0).
    pub min_width: Dimension,
    /// Maximum width. Initial: Auto (= none/unconstrained).
    pub max_width: Dimension,
    /// Minimum height. Initial: Length(0.0).
    pub min_height: Dimension,
    /// Maximum height. Initial: Auto (= none/unconstrained).
    pub max_height: Dimension,

    /// Margin top. Initial: Length(0.0).
    pub margin_top: Dimension,
    /// Margin right. Initial: Length(0.0).
    pub margin_right: Dimension,
    /// Margin bottom. Initial: Length(0.0).
    pub margin_bottom: Dimension,
    /// Margin left. Initial: Length(0.0).
    pub margin_left: Dimension,

    /// Padding edges (computed value — may contain percentages). Initial: all 0.
    pub padding: EdgeSizes<Dimension>,

    /// Top border. Computed initial: width 0.0 (medium=3px, but 0 when style=none).
    pub border_top: BorderSide,
    /// Right border.
    pub border_right: BorderSide,
    /// Bottom border.
    pub border_bottom: BorderSide,
    /// Left border.
    pub border_left: BorderSide,

    // --- Inherited text spacing ---
    /// Letter spacing in pixels. `None` = `normal` (CSS initial value).
    /// `Some(0.0)` = explicit `0px`. **Inherited.**
    pub letter_spacing: Option<f32>,
    /// Word spacing in pixels. `None` = `normal` (CSS initial value).
    /// `Some(0.0)` = explicit `0px`. **Inherited.**
    pub word_spacing: Option<f32>,

    // --- Text decoration (non-inherited) ---
    /// Text decoration line. Initial: none.
    pub text_decoration_line: TextDecorationLine,
    /// Text decoration style. Initial: solid.
    pub text_decoration_style: TextDecorationStyle,
    /// Text decoration color. Initial: None (= currentcolor).
    pub text_decoration_color: Option<CssColor>,

    // --- Box model (non-inherited) ---
    /// Box sizing model. Initial: content-box.
    pub box_sizing: BoxSizing,
    /// Per-corner border radii in pixels `[top-left, top-right, bottom-right, bottom-left]`.
    /// Initial: `[0.0; 4]`.
    pub border_radii: [f32; 4],
    /// Opacity (0.0–1.0). Initial: 1.0.
    pub opacity: f32,

    // --- Flex gap properties (non-inherited) ---
    /// Row gap (computed value — may contain percentages). Initial: 0.
    pub row_gap: Dimension,
    /// Column gap (computed value — may contain percentages). Initial: 0.
    pub column_gap: Dimension,

    // --- Alignment safety (non-inherited) ---
    /// Justify-content alignment safety. Initial: `Unsafe`.
    pub justify_content_safety: AlignmentSafety,
    /// Align-content alignment safety. Initial: `Unsafe`.
    pub align_content_safety: AlignmentSafety,

    // --- Flex container properties (non-inherited) ---
    /// Flex direction. Initial: `Row`.
    pub flex_direction: FlexDirection,
    /// Flex wrap. Initial: `Nowrap`.
    pub flex_wrap: FlexWrap,
    /// Justify content. Initial: `FlexStart`.
    pub justify_content: JustifyContent,
    /// Align items. Initial: `Stretch`.
    pub align_items: AlignItems,
    /// Align content. Initial: `Stretch`.
    pub align_content: AlignContent,

    // --- Flex item properties (non-inherited) ---
    /// Flex grow factor. Initial: `0.0`.
    pub flex_grow: f32,
    /// Flex shrink factor. Initial: `1.0`.
    pub flex_shrink: f32,
    /// Flex basis. Initial: `Auto`.
    pub flex_basis: FlexBasis,
    /// Order. Initial: `0`.
    pub order: i32,
    /// Align self. Initial: `Auto`.
    pub align_self: AlignSelf,

    // --- Grid container properties (non-inherited) ---
    /// Grid template column track sizes. Initial: empty (= `none`).
    pub grid_template_columns: GridTrackList,
    /// Grid template row track sizes. Initial: empty (= `none`).
    pub grid_template_rows: GridTrackList,
    /// Grid auto-flow direction. Initial: `Row`.
    pub grid_auto_flow: GridAutoFlow,
    /// Implicit column track sizes (cycled for implicit tracks). Initial: `[Auto]`.
    pub grid_auto_columns: Vec<TrackSize>,
    /// Implicit row track sizes (cycled for implicit tracks). Initial: `[Auto]`.
    pub grid_auto_rows: Vec<TrackSize>,
    /// Grid template areas (CSS Grid §8.2). Initial: `none` (empty).
    pub grid_template_areas: GridTemplateAreas,
    /// Justify items for grid children. Initial: `Stretch`.
    pub justify_items: JustifyItems,
    /// Justify self for grid items. Initial: `Auto`.
    pub justify_self: JustifySelf,

    // --- Grid item properties (non-inherited) ---
    /// Grid column start line. Initial: `Auto`.
    pub grid_column_start: GridLine,
    /// Grid column end line. Initial: `Auto`.
    pub grid_column_end: GridLine,
    /// Grid row start line. Initial: `Auto`.
    pub grid_row_start: GridLine,
    /// Grid row end line. Initial: `Auto`.
    pub grid_row_end: GridLine,

    // --- Table properties ---
    /// Empty cells visibility. Initial: `Show`. **Inherited.**
    pub empty_cells: EmptyCells,
    /// Border collapse model. Initial: `Separate`. **Inherited.**
    pub border_collapse: BorderCollapse,
    /// Horizontal border spacing in pixels (separate model only). Initial: 0.0. **Inherited.**
    pub border_spacing_h: f32,
    /// Vertical border spacing in pixels (separate model only). Initial: 0.0. **Inherited.**
    pub border_spacing_v: f32,
    /// Table layout algorithm. Initial: `Auto`.
    pub table_layout: TableLayout,
    /// Caption placement. Initial: `Top`. **Inherited.**
    pub caption_side: CaptionSide,

    // --- Position offsets (non-inherited) ---
    /// Top offset. Initial: Auto.
    pub top: Dimension,
    /// Right offset. Initial: Auto.
    pub right: Dimension,
    /// Bottom offset. Initial: Auto.
    pub bottom: Dimension,
    /// Left offset. Initial: Auto.
    pub left: Dimension,
    /// Stacking order. Initial: `None` (auto).
    pub z_index: Option<i32>,

    // --- Float/clear properties (non-inherited) ---
    /// Float positioning. Initial: `None`.
    pub float: Float,
    /// Clear floating. Initial: `None`.
    pub clear: Clear,
    /// Vertical alignment for inline/table-cell. Initial: `Baseline`.
    pub vertical_align: VerticalAlign,

    // --- Fragmentation properties ---
    /// Break before the element. Initial: `Auto`.
    pub break_before: BreakValue,
    /// Break after the element. Initial: `Auto`.
    pub break_after: BreakValue,
    /// Break inside the element. Initial: `Auto`.
    pub break_inside: BreakInsideValue,
    /// Box decoration break. Initial: `Slice`.
    pub box_decoration_break: BoxDecorationBreak,
    /// Minimum lines at bottom of page/column. Initial: `2`. **Inherited.**
    pub orphans: u32,
    /// Minimum lines at top of page/column. Initial: `2`. **Inherited.**
    pub widows: u32,

    // --- Multi-column properties (non-inherited) ---
    /// Column count. Initial: `None` (= `auto`).
    pub column_count: Option<u32>,
    /// Column width. Initial: `Auto`.
    pub column_width: Dimension,
    /// Column fill. Initial: `Balance`.
    pub column_fill: ColumnFill,
    /// Column span. Initial: `None`.
    pub column_span: ColumnSpan,
    /// Column rule width in pixels. Initial: `3.0` (medium).
    pub column_rule_width: f32,
    /// Column rule style. Initial: `None`.
    pub column_rule_style: BorderStyle,
    /// Column rule color. Initial: `currentColor`.
    pub column_rule_color: CssColor,

    // --- Generated content (non-inherited) ---
    /// The `content` property. Initial: `Normal`.
    pub content: ContentValue,

    // --- Stacking context flags (non-inherited) ---
    // Set by CSS property handlers when resolved; default = initial value (no stacking context).
    // Parsing/resolution of these CSS properties is deferred to future milestones.
    /// `true` when `transform` is not `none` (CSS Transforms L1 §2).
    pub has_transform: bool,
    /// `true` when `filter` is not `none` (CSS Filter Effects L1 §2).
    pub has_filter: bool,
    /// `true` when `backdrop-filter` is not `none` (CSS Filter Effects L2).
    pub has_backdrop_filter: bool,
    /// `true` when `clip-path` is not `none` (CSS Masking L1 §3.1).
    pub has_clip_path: bool,
    /// `true` when `mask`/`mask-image` is not `none` (CSS Masking L1 §3.1).
    pub has_mask: bool,
    /// `true` when `perspective` is not `none` (CSS Transforms L2 §3.1).
    pub has_perspective: bool,
    /// `true` when `will-change` specifies a stacking-context-creating property (CSS Will Change L1 §2.2).
    pub will_change_stacking: bool,
    /// `true` when `isolation` is `isolate` (CSS Compositing L1 §3).
    pub isolation_isolate: bool,
    /// `true` when `mix-blend-mode` is not `normal` (CSS Compositing L1 §3).
    pub has_mix_blend: bool,
    /// `true` when `contain` includes `paint`, `layout`, `strict`, or `content` (CSS Containment L2 §3).
    pub contain_stacking: bool,

    // --- Transform properties (non-inherited, CSS Transforms L1/L2) ---
    /// Parsed transform function list. Empty vec = `none`.
    pub transform: Vec<TransformFunction>,
    /// Transform origin (x, y, z). Z is always in px. Default: (50%, 50%, 0).
    pub transform_origin: (Dimension, Dimension, f32),
    /// CSS `perspective` property value in px. `None` = `none`.
    pub perspective: Option<f32>,
    /// Perspective origin (x, y). Default: (50%, 50%).
    pub perspective_origin: (Dimension, Dimension),
    /// CSS `transform-style`. Default: `flat`.
    pub transform_style: TransformStyle,
    /// CSS `backface-visibility`. Default: `visible`.
    pub backface_visibility: BackfaceVisibility,
    /// CSS `will-change` property values. Empty vec = `auto`.
    pub will_change: Vec<String>,

    // --- Custom properties (CSS Variables) ---
    /// Custom property values (e.g. `--bg: #0d1117`).
    ///
    /// Keys include the `--` prefix. Values are raw token strings.
    /// Custom properties are inherited by default (CSS Variables Level 1).
    pub custom_properties: HashMap<String, String>,
}

impl Default for ComputedStyle {
    #[allow(clippy::too_many_lines)] // All CSS initial values need explicit initialization.
    fn default() -> Self {
        let color = CssColor::BLACK;
        Self {
            // Inherited
            color,
            font_size: 16.0,
            font_weight: 400,
            font_style: FontStyle::default(),
            font_family: vec!["serif".to_string()],
            line_height: LineHeight::Normal,
            text_transform: TextTransform::default(),
            text_align: TextAlign::default(),
            white_space: WhiteSpace::default(),
            list_style_type: ListStyleType::default(),
            writing_mode: WritingMode::default(),
            text_orientation: TextOrientation::default(),
            direction: Direction::default(),
            visibility: Visibility::default(),

            // Non-inherited
            display: Display::default(),
            position: Position::default(),
            unicode_bidi: UnicodeBidi::default(),
            background_color: CssColor::TRANSPARENT,
            background_layers: None,
            overflow_x: Overflow::default(),
            overflow_y: Overflow::default(),

            width: Dimension::Auto,
            height: Dimension::Auto,
            min_width: Dimension::ZERO,
            max_width: Dimension::Auto,
            min_height: Dimension::ZERO,
            max_height: Dimension::Auto,

            margin_top: Dimension::ZERO,
            margin_right: Dimension::ZERO,
            margin_bottom: Dimension::ZERO,
            margin_left: Dimension::ZERO,

            padding: EdgeSizes::<Dimension>::default(),

            // CSS initial value is `medium` (3px), but computed value is 0
            // when border-style is `none` (the default).
            // currentcolor → resolved to `color` field value.
            border_top: BorderSide {
                color,
                ..BorderSide::NONE
            },
            border_right: BorderSide {
                color,
                ..BorderSide::NONE
            },
            border_bottom: BorderSide {
                color,
                ..BorderSide::NONE
            },
            border_left: BorderSide {
                color,
                ..BorderSide::NONE
            },

            // Inherited text spacing
            letter_spacing: None,
            word_spacing: None,

            // Text decoration (non-inherited)
            text_decoration_line: TextDecorationLine::default(),
            text_decoration_style: TextDecorationStyle::default(),
            text_decoration_color: None,

            // Box model
            box_sizing: BoxSizing::default(),
            border_radii: [0.0; 4],
            opacity: 1.0,

            // Flex gap
            row_gap: Dimension::ZERO,
            column_gap: Dimension::ZERO,

            // Flex container
            flex_direction: FlexDirection::default(),
            flex_wrap: FlexWrap::default(),
            justify_content: JustifyContent::default(),
            align_items: AlignItems::default(),
            align_content: AlignContent::default(),

            // Flex item
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: FlexBasis::Auto,
            order: 0,
            align_self: AlignSelf::default(),

            // Alignment safety
            justify_content_safety: AlignmentSafety::default(),
            align_content_safety: AlignmentSafety::default(),

            // Grid container
            grid_template_columns: GridTrackList::default(),
            grid_template_rows: GridTrackList::default(),
            grid_auto_flow: GridAutoFlow::default(),
            grid_auto_columns: vec![TrackSize::Auto],
            grid_auto_rows: vec![TrackSize::Auto],
            grid_template_areas: GridTemplateAreas::default(),
            justify_items: JustifyItems::default(),
            justify_self: JustifySelf::default(),

            // Grid item
            grid_column_start: GridLine::Auto,
            grid_column_end: GridLine::Auto,
            grid_row_start: GridLine::Auto,
            grid_row_end: GridLine::Auto,

            // Table
            empty_cells: EmptyCells::default(),
            border_collapse: BorderCollapse::default(),
            border_spacing_h: 0.0,
            border_spacing_v: 0.0,
            table_layout: TableLayout::default(),
            caption_side: CaptionSide::default(),

            // Position offsets
            top: Dimension::Auto,
            right: Dimension::Auto,
            bottom: Dimension::Auto,
            left: Dimension::Auto,
            z_index: None,

            // Float/clear
            float: Float::default(),
            clear: Clear::default(),
            vertical_align: VerticalAlign::default(),

            // Fragmentation
            break_before: BreakValue::default(),
            break_after: BreakValue::default(),
            break_inside: BreakInsideValue::default(),
            box_decoration_break: BoxDecorationBreak::default(),
            orphans: 2,
            widows: 2,

            // Multi-column
            column_count: None,
            column_width: Dimension::Auto,
            column_fill: ColumnFill::default(),
            column_span: ColumnSpan::default(),
            column_rule_width: 3.0,
            column_rule_style: BorderStyle::None,
            column_rule_color: color,

            // Generated content
            content: ContentValue::Normal,

            // Stacking context flags
            has_transform: false,
            has_filter: false,
            has_backdrop_filter: false,
            has_clip_path: false,
            has_mask: false,
            has_perspective: false,
            will_change_stacking: false,
            isolation_isolate: false,
            has_mix_blend: false,
            contain_stacking: false,

            // Transform properties
            transform: Vec::new(),
            transform_origin: (
                Dimension::Percentage(50.0),
                Dimension::Percentage(50.0),
                0.0,
            ),
            perspective: None,
            perspective_origin: (Dimension::Percentage(50.0), Dimension::Percentage(50.0)),
            transform_style: TransformStyle::default(),
            backface_visibility: BackfaceVisibility::default(),
            will_change: Vec::new(),

            // Custom properties
            custom_properties: HashMap::new(),
        }
    }
}

impl ComputedStyle {
    /// Returns `true` if this element is a scroll container on either axis.
    #[must_use]
    pub fn is_scroll_container(&self) -> bool {
        self.overflow_x.is_scroll_container() || self.overflow_y.is_scroll_container()
    }

    /// Returns `true` if overflow is clipped on either axis.
    #[must_use]
    pub fn clips_overflow(&self) -> bool {
        self.overflow_x.clips() || self.overflow_y.clips()
    }

    /// Returns `true` if this element creates a stacking context.
    ///
    /// Full condition list per current CSS specifications:
    /// - CSS Positioned Layout L3 §3: position absolute/fixed (any z-index)
    /// - CSS 2.1 §9.9.1: position relative/sticky with z-index != auto
    /// - CSS Color L4: opacity < 1.0
    /// - CSS Overflow L3 §3: overflow != visible on either axis
    /// - CSS Transforms L1 §2: transform != none
    /// - CSS Filter Effects L1 §2: filter != none
    /// - CSS Filter Effects L2: backdrop-filter != none
    /// - CSS Transforms L2 §3.1: perspective != none
    /// - CSS Will Change L1 §2.2: will-change creates stacking context
    /// - CSS Compositing L1 §3: isolation: isolate
    /// - CSS Compositing L1 §3: mix-blend-mode != normal
    /// - CSS Masking L1 §3.1: clip-path != none
    /// - CSS Masking L1 §3.1: mask/mask-image != none
    /// - CSS Containment L2 §3: contain includes paint/layout/strict/content
    #[must_use]
    pub fn creates_stacking_context(&self) -> bool {
        // CSS 2.1 §9.9.1: positioned + z-index: <integer> → stacking context.
        // positioned + z-index: auto → NOT a stacking context (children bubble up).
        if self.position != Position::Static && self.z_index.is_some() {
            return true;
        }
        // Visual effects
        if self.opacity < 1.0 {
            return true;
        }
        // Overflow
        if self.overflow_x != Overflow::Visible || self.overflow_y != Overflow::Visible {
            return true;
        }
        // Transform, filter, masking, compositing, containment
        self.has_transform
            || self.has_filter
            || self.has_backdrop_filter
            || self.has_clip_path
            || self.has_mask
            || self.has_perspective
            || self.will_change_stacking
            || self.isolation_isolate
            || self.has_mix_blend
            || self.contain_stacking
    }
}
