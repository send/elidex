//! Computed style representation for resolved CSS property values.
//!
//! [`ComputedStyle`] is an ECS component attached to every element after
//! style resolution. It contains fully resolved values for all supported
//! CSS properties.

use std::collections::HashMap;

use crate::{CssColor, EdgeSizes};

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
mod display;
mod flex;
mod float_visibility;
mod grid;
mod table;
mod text;
mod writing_mode;

pub use box_model::{BorderSide, BorderStyle, BoxSizing, ContentItem, ContentValue, Dimension};
pub use display::{Display, Overflow, Position};
pub use flex::{AlignContent, AlignItems, AlignSelf, FlexDirection, FlexWrap, JustifyContent};
pub use float_visibility::{Clear, Float, VerticalAlign, Visibility};
pub use grid::{GridAutoFlow, GridLine, TrackBreadth, TrackSize};
pub use table::{BorderCollapse, CaptionSide, TableLayout};
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

    /// Overflow behavior. Initial: Visible.
    pub overflow: Overflow,

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

    /// Padding edges in pixels. Initial: all 0.0.
    pub padding: EdgeSizes,

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
    /// Border radius (uniform, all corners) in pixels. Initial: 0.0.
    pub border_radius: f32,
    /// Opacity (0.0–1.0). Initial: 1.0.
    pub opacity: f32,

    // --- Flex gap properties (non-inherited) ---
    /// Row gap in pixels. Initial: 0.0.
    pub row_gap: f32,
    /// Column gap in pixels. Initial: 0.0.
    pub column_gap: f32,

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
    pub flex_basis: Dimension,
    /// Order. Initial: `0`.
    pub order: i32,
    /// Align self. Initial: `Auto`.
    pub align_self: AlignSelf,

    // --- Grid container properties (non-inherited) ---
    /// Grid template column track sizes. Initial: empty (= `none`).
    pub grid_template_columns: Vec<TrackSize>,
    /// Grid template row track sizes. Initial: empty (= `none`).
    pub grid_template_rows: Vec<TrackSize>,
    /// Grid auto-flow direction. Initial: `Row`.
    pub grid_auto_flow: GridAutoFlow,
    /// Implicit column track size. Initial: `Auto`.
    pub grid_auto_columns: TrackSize,
    /// Implicit row track size. Initial: `Auto`.
    pub grid_auto_rows: TrackSize,

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

    // --- Float/clear properties (non-inherited) ---
    /// Float positioning. Initial: `None`.
    pub float: Float,
    /// Clear floating. Initial: `None`.
    pub clear: Clear,
    /// Vertical alignment for inline/table-cell. Initial: `Baseline`.
    pub vertical_align: VerticalAlign,

    // --- Generated content (non-inherited) ---
    /// The `content` property. Initial: `Normal`.
    pub content: ContentValue,

    // --- Custom properties (CSS Variables) ---
    /// Custom property values (e.g. `--bg: #0d1117`).
    ///
    /// Keys include the `--` prefix. Values are raw token strings.
    /// Custom properties are inherited by default (CSS Variables Level 1).
    pub custom_properties: HashMap<String, String>,
}

impl Default for ComputedStyle {
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
            overflow: Overflow::default(),

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

            padding: EdgeSizes::default(),

            // CSS initial value is `medium` (3px), but computed value is 0
            // when border-style is `none` (the default).
            // currentcolor → resolved to `color` field value.
            border_top: BorderSide { color, ..BorderSide::NONE },
            border_right: BorderSide { color, ..BorderSide::NONE },
            border_bottom: BorderSide { color, ..BorderSide::NONE },
            border_left: BorderSide { color, ..BorderSide::NONE },

            // Inherited text spacing
            letter_spacing: None,
            word_spacing: None,

            // Text decoration (non-inherited)
            text_decoration_line: TextDecorationLine::default(),
            text_decoration_style: TextDecorationStyle::default(),
            text_decoration_color: None,

            // Box model
            box_sizing: BoxSizing::default(),
            border_radius: 0.0,
            opacity: 1.0,

            // Flex gap
            row_gap: 0.0,
            column_gap: 0.0,

            // Flex container
            flex_direction: FlexDirection::default(),
            flex_wrap: FlexWrap::default(),
            justify_content: JustifyContent::default(),
            align_items: AlignItems::default(),
            align_content: AlignContent::default(),

            // Flex item
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: Dimension::Auto,
            order: 0,
            align_self: AlignSelf::default(),

            // Grid container
            grid_template_columns: Vec::new(),
            grid_template_rows: Vec::new(),
            grid_auto_flow: GridAutoFlow::default(),
            grid_auto_columns: TrackSize::Auto,
            grid_auto_rows: TrackSize::Auto,

            // Grid item
            grid_column_start: GridLine::Auto,
            grid_column_end: GridLine::Auto,
            grid_row_start: GridLine::Auto,
            grid_row_end: GridLine::Auto,

            // Table
            border_collapse: BorderCollapse::default(),
            border_spacing_h: 0.0,
            border_spacing_v: 0.0,
            table_layout: TableLayout::default(),
            caption_side: CaptionSide::default(),

            // Float/clear
            float: Float::default(),
            clear: Clear::default(),
            vertical_align: VerticalAlign::default(),

            // Generated content
            content: ContentValue::Normal,

            // Custom properties
            custom_properties: HashMap::new(),
        }
    }
}
