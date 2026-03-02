//! Computed style representation for resolved CSS property values.
//!
//! [`ComputedStyle`] is an ECS component attached to every element after
//! style resolution. It contains fully resolved values for all supported
//! CSS properties.

use std::collections::HashMap;
use std::fmt;

use crate::CssColor;

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
    };
}

keyword_enum! {
    /// The CSS `display` property.
    Display {
        Inline => "inline",
        Block => "block",
        InlineBlock => "inline-block",
        None => "none",
        Flex => "flex",
        InlineFlex => "inline-flex",
        ListItem => "list-item",
        Grid => "grid",
        InlineGrid => "inline-grid",
        Table => "table",
        InlineTable => "inline-table",
        TableCaption => "table-caption",
        TableRow => "table-row",
        TableCell => "table-cell",
        TableRowGroup => "table-row-group",
        TableHeaderGroup => "table-header-group",
        TableFooterGroup => "table-footer-group",
        TableColumn => "table-column",
        TableColumnGroup => "table-column-group",
    }
}

keyword_enum! {
    /// The CSS `border-collapse` property (CSS 2.1 §17.6).
    BorderCollapse {
        Separate => "separate",
        Collapse => "collapse",
    }
}

keyword_enum! {
    /// The CSS `table-layout` property (CSS 2.1 §17.5.2).
    TableLayout {
        Auto => "auto",
        Fixed => "fixed",
    }
}

keyword_enum! {
    /// The CSS `caption-side` property (CSS 2.1 §17.4.1).
    CaptionSide {
        Top => "top",
        Bottom => "bottom",
    }
}

keyword_enum! {
    /// The CSS `grid-auto-flow` property.
    GridAutoFlow {
        Row => "row",
        Column => "column",
        RowDense => "row dense",
        ColumnDense => "column dense",
    }
}

/// A single track sizing function for CSS Grid.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum TrackSize {
    /// A fixed length in pixels.
    Length(f32),
    /// A percentage of the grid container's size.
    Percentage(f32),
    /// A flexible length (`fr` unit).
    Fr(f32),
    /// `auto` — sized by content.
    #[default]
    Auto,
    /// `minmax(min, max)` function.
    MinMax(Box<TrackBreadth>, Box<TrackBreadth>),
}

/// A track breadth value, used inside `minmax()`.
#[derive(Clone, Debug, PartialEq)]
pub enum TrackBreadth {
    /// A fixed length in pixels.
    Length(f32),
    /// A percentage of the grid container's size.
    Percentage(f32),
    /// A flexible length (`fr` unit).
    Fr(f32),
    /// `auto` — sized by content.
    Auto,
    /// `min-content` intrinsic size.
    MinContent,
    /// `max-content` intrinsic size.
    MaxContent,
}

/// A grid line placement value.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum GridLine {
    /// `auto` — automatic placement.
    #[default]
    Auto,
    /// An explicit line number (1-based, can be negative).
    Line(i32),
    /// `span N` — span across N tracks.
    Span(u32),
}

keyword_enum! {
    /// The CSS `flex-direction` property.
    FlexDirection {
        Row => "row",
        RowReverse => "row-reverse",
        Column => "column",
        ColumnReverse => "column-reverse",
    }
}

keyword_enum! {
    /// The CSS `flex-wrap` property.
    FlexWrap {
        Nowrap => "nowrap",
        Wrap => "wrap",
        WrapReverse => "wrap-reverse",
    }
}

keyword_enum! {
    /// The CSS `justify-content` property.
    JustifyContent {
        FlexStart => "flex-start",
        FlexEnd => "flex-end",
        Center => "center",
        SpaceBetween => "space-between",
        SpaceAround => "space-around",
        SpaceEvenly => "space-evenly",
    }
}

keyword_enum! {
    /// The CSS `align-items` property.
    AlignItems {
        Stretch => "stretch",
        FlexStart => "flex-start",
        FlexEnd => "flex-end",
        Center => "center",
        Baseline => "baseline",
    }
}

keyword_enum! {
    /// The CSS `align-self` property.
    AlignSelf {
        Auto => "auto",
        Stretch => "stretch",
        FlexStart => "flex-start",
        FlexEnd => "flex-end",
        Center => "center",
        Baseline => "baseline",
    }
}

keyword_enum! {
    /// The CSS `align-content` property.
    AlignContent {
        Stretch => "stretch",
        FlexStart => "flex-start",
        FlexEnd => "flex-end",
        Center => "center",
        SpaceBetween => "space-between",
        SpaceAround => "space-around",
    }
}

keyword_enum! {
    /// The CSS `position` property.
    Position {
        Static => "static",
        Relative => "relative",
        Absolute => "absolute",
        Fixed => "fixed",
    }
}

keyword_enum! {
    /// The CSS `text-align` property.
    TextAlign {
        Left => "left",
        Center => "center",
        Right => "right",
    }
}

keyword_enum! {
    /// The CSS `text-transform` property.
    TextTransform {
        None => "none",
        Uppercase => "uppercase",
        Lowercase => "lowercase",
        Capitalize => "capitalize",
    }
}

/// The CSS `text-decoration-line` property.
///
/// Not inherited. Multiple values possible (e.g. `underline line-through`).
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct TextDecorationLine {
    /// Whether `underline` is set.
    pub underline: bool,
    /// Whether `line-through` is set.
    pub line_through: bool,
}

impl fmt::Display for TextDecorationLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.underline, self.line_through) {
            (false, false) => f.write_str("none"),
            (true, false) => f.write_str("underline"),
            (false, true) => f.write_str("line-through"),
            (true, true) => f.write_str("underline line-through"),
        }
    }
}

/// The CSS `line-height` property, preserving keyword/number semantics.
///
/// CSS Variables Level 1 requires `normal` and unitless `<number>` to be
/// inherited as-is and recomputed relative to each element's `font-size`.
/// Storing the resolved px value at computed time would lose this semantic.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum LineHeight {
    /// `line-height: normal` — typically 1.2 × font-size.
    #[default]
    Normal,
    /// Unitless number multiplier (e.g. `line-height: 1.5`).
    Number(f32),
    /// Absolute length in pixels (e.g. `line-height: 24px` or resolved from `%`).
    Px(f32),
}

impl LineHeight {
    /// Resolve to an absolute pixel value given the element's font size.
    #[must_use]
    pub fn resolve_px(self, font_size: f32) -> f32 {
        match self {
            Self::Normal => font_size * 1.2,
            Self::Number(n) => font_size * n,
            Self::Px(px) => px,
        }
    }
}

impl fmt::Display for LineHeight {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Normal => f.write_str("normal"),
            Self::Number(n) => write!(f, "{n}"),
            Self::Px(px) => write!(f, "{px}px"),
        }
    }
}

keyword_enum! {
    /// The CSS `white-space` property.
    WhiteSpace {
        Normal => "normal",
        Pre => "pre",
        NoWrap => "nowrap",
        PreWrap => "pre-wrap",
        PreLine => "pre-line",
    }
}

keyword_enum! {
    /// The CSS `overflow` property.
    ///
    /// CSS `scroll` and `auto` are mapped to `Hidden` during parsing
    /// (scrollbar rendering is deferred to Phase 4).
    Overflow {
        Visible => "visible",
        Hidden => "hidden",
    }
}

keyword_enum! {
    /// The CSS `list-style-type` property.
    ListStyleType {
        Disc => "disc",
        Circle => "circle",
        Square => "square",
        Decimal => "decimal",
        None => "none",
    }
}

keyword_enum! {
    /// The CSS `box-sizing` property.
    BoxSizing {
        ContentBox => "content-box",
        BorderBox => "border-box",
    }
}

keyword_enum! {
    /// The CSS `border-*-style` property.
    BorderStyle {
        None => "none",
        Solid => "solid",
        Dashed => "dashed",
        Dotted => "dotted",
    }
}

/// A single item in a `content` property value.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ContentItem {
    /// A literal string (e.g. `content: ">>"`).
    String(String),
    /// An `attr()` function reference (e.g. `content: attr(title)`).
    Attr(String),
}

/// The computed value of the CSS `content` property.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub enum ContentValue {
    /// `content: normal` — no generated content for regular elements.
    #[default]
    Normal,
    /// `content: none` — suppress generated content.
    None,
    /// One or more content items.
    Items(Vec<ContentItem>),
}

/// A resolved dimension value: lengths are always in px, percentages in `0..100` range.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Dimension {
    Length(f32),
    Percentage(f32),
    #[default]
    Auto,
}

impl Dimension {
    /// Zero-length constant (`Length(0.0)`), used as the CSS initial value
    /// for margins, `min-width`, `min-height`, etc.
    pub const ZERO: Self = Self::Length(0.0);
}

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

    // --- Non-inherited properties ---
    /// Display type. Initial: Inline.
    pub display: Display,
    /// Positioning scheme. Initial: Static.
    pub position: Position,
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

    // TODO: replace padding_{top,right,bottom,left} with EdgeSizes
    /// Padding top in pixels. Initial: 0.0.
    pub padding_top: f32,
    /// Padding right in pixels. Initial: 0.0.
    pub padding_right: f32,
    /// Padding bottom in pixels. Initial: 0.0.
    pub padding_bottom: f32,
    /// Padding left in pixels. Initial: 0.0.
    pub padding_left: f32,

    // TODO: replace border_{top,right,bottom,left}_{width,style,color} with BorderSide struct
    /// Border top width in pixels. Computed initial: 0.0 (medium=3px, but 0 when style=none).
    pub border_top_width: f32,
    /// Border right width in pixels. Computed initial: 0.0.
    pub border_right_width: f32,
    /// Border bottom width in pixels. Computed initial: 0.0.
    pub border_bottom_width: f32,
    /// Border left width in pixels. Computed initial: 0.0.
    pub border_left_width: f32,

    /// Border top style. Initial: None.
    pub border_top_style: BorderStyle,
    /// Border right style. Initial: None.
    pub border_right_style: BorderStyle,
    /// Border bottom style. Initial: None.
    pub border_bottom_style: BorderStyle,
    /// Border left style. Initial: None.
    pub border_left_style: BorderStyle,

    /// Border top color. Initial: currentcolor (resolved to `color`).
    pub border_top_color: CssColor,
    /// Border right color. Initial: currentcolor (resolved to `color`).
    pub border_right_color: CssColor,
    /// Border bottom color. Initial: currentcolor (resolved to `color`).
    pub border_bottom_color: CssColor,
    /// Border left color. Initial: currentcolor (resolved to `color`).
    pub border_left_color: CssColor,

    // --- Text decoration (non-inherited) ---
    /// Text decoration line. Initial: none.
    pub text_decoration_line: TextDecorationLine,

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
            font_family: vec!["serif".to_string()],
            line_height: LineHeight::Normal,
            text_transform: TextTransform::default(),
            text_align: TextAlign::default(),
            white_space: WhiteSpace::default(),
            list_style_type: ListStyleType::default(),

            // Non-inherited
            display: Display::default(),
            position: Position::default(),
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

            padding_top: 0.0,
            padding_right: 0.0,
            padding_bottom: 0.0,
            padding_left: 0.0,

            // CSS initial value is `medium` (3px), but computed value is 0
            // when border-style is `none` (the default).
            border_top_width: 0.0,
            border_right_width: 0.0,
            border_bottom_width: 0.0,
            border_left_width: 0.0,

            border_top_style: BorderStyle::default(),
            border_right_style: BorderStyle::default(),
            border_bottom_style: BorderStyle::default(),
            border_left_style: BorderStyle::default(),

            // currentcolor → resolved to `color` field value
            border_top_color: color,
            border_right_color: color,
            border_bottom_color: color,
            border_left_color: color,

            // Text decoration (non-inherited)
            text_decoration_line: TextDecorationLine::default(),

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

            // Generated content
            content: ContentValue::Normal,

            // Custom properties
            custom_properties: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_initial_values() {
        let s = ComputedStyle::default();
        assert_eq!(s.color, CssColor::BLACK);
        assert_eq!(s.font_size, 16.0);
        assert_eq!(s.font_family, vec!["serif".to_string()]);
        assert_eq!(s.display, Display::Inline);
        assert_eq!(s.position, Position::Static);
        assert_eq!(s.background_color, CssColor::TRANSPARENT);
        assert_eq!(s.width, Dimension::Auto);
        assert_eq!(s.height, Dimension::Auto);
        assert_eq!(s.margin_top, Dimension::ZERO);
        assert_eq!(s.padding_top, 0.0);
        assert_eq!(s.border_top_width, 0.0);
        assert_eq!(s.border_top_style, BorderStyle::None);
        // currentcolor → color (BLACK)
        assert_eq!(s.border_top_color, CssColor::BLACK);
    }

    #[test]
    fn enum_defaults() {
        assert_eq!(Display::default(), Display::Inline);
        assert_eq!(Position::default(), Position::Static);
        assert_eq!(BorderStyle::default(), BorderStyle::None);
        assert_eq!(Dimension::default(), Dimension::Auto);
        assert_eq!(FlexDirection::default(), FlexDirection::Row);
        assert_eq!(FlexWrap::default(), FlexWrap::Nowrap);
        assert_eq!(JustifyContent::default(), JustifyContent::FlexStart);
        assert_eq!(AlignItems::default(), AlignItems::Stretch);
        assert_eq!(AlignSelf::default(), AlignSelf::Auto);
        assert_eq!(AlignContent::default(), AlignContent::Stretch);
    }

    #[test]
    fn flex_enum_as_ref() {
        assert_eq!(FlexDirection::RowReverse.as_ref(), "row-reverse");
        assert_eq!(FlexWrap::WrapReverse.as_ref(), "wrap-reverse");
        assert_eq!(JustifyContent::SpaceBetween.as_ref(), "space-between");
        assert_eq!(AlignItems::Center.as_ref(), "center");
        assert_eq!(AlignSelf::FlexEnd.as_ref(), "flex-end");
        assert_eq!(AlignContent::SpaceAround.as_ref(), "space-around");
    }

    #[test]
    fn computed_style_flex_defaults() {
        let s = ComputedStyle::default();
        assert_eq!(s.flex_direction, FlexDirection::Row);
        assert_eq!(s.flex_wrap, FlexWrap::Nowrap);
        assert_eq!(s.justify_content, JustifyContent::FlexStart);
        assert_eq!(s.align_items, AlignItems::Stretch);
        assert_eq!(s.align_content, AlignContent::Stretch);
        assert_eq!(s.flex_grow, 0.0);
        assert_eq!(s.flex_shrink, 1.0);
        assert_eq!(s.flex_basis, Dimension::Auto);
        assert_eq!(s.order, 0);
        assert_eq!(s.align_self, AlignSelf::Auto);
    }

    #[test]
    fn clone_and_partial_eq() {
        let a = ComputedStyle::default();
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn dimension_variants() {
        let l = Dimension::Length(42.0);
        let p = Dimension::Percentage(50.0);
        let a = Dimension::Auto;
        assert_ne!(l, p);
        assert_ne!(p, a);
        assert_ne!(l, a);
    }

    // --- M3-1: Text property types ---

    #[test]
    fn text_transform_defaults_and_as_ref() {
        assert_eq!(TextTransform::default(), TextTransform::None);
        assert_eq!(TextTransform::None.as_ref(), "none");
        assert_eq!(TextTransform::Uppercase.as_ref(), "uppercase");
        assert_eq!(TextTransform::Lowercase.as_ref(), "lowercase");
        assert_eq!(TextTransform::Capitalize.as_ref(), "capitalize");
    }

    #[test]
    fn text_decoration_line_display() {
        let none = TextDecorationLine::default();
        assert_eq!(none.to_string(), "none");

        let ul = TextDecorationLine {
            underline: true,
            line_through: false,
        };
        assert_eq!(ul.to_string(), "underline");

        let lt = TextDecorationLine {
            underline: false,
            line_through: true,
        };
        assert_eq!(lt.to_string(), "line-through");

        let both = TextDecorationLine {
            underline: true,
            line_through: true,
        };
        assert_eq!(both.to_string(), "underline line-through");
    }

    #[test]
    fn computed_style_text_defaults() {
        let s = ComputedStyle::default();
        assert_eq!(s.font_weight, 400);
        assert_eq!(s.line_height, LineHeight::Normal);
        assert_eq!(s.text_transform, TextTransform::None);
        assert_eq!(s.text_decoration_line, TextDecorationLine::default());
    }

    // --- M3-2: Box model types ---

    #[test]
    fn box_sizing_defaults_and_as_ref() {
        assert_eq!(BoxSizing::default(), BoxSizing::ContentBox);
        assert_eq!(BoxSizing::ContentBox.as_ref(), "content-box");
        assert_eq!(BoxSizing::BorderBox.as_ref(), "border-box");
        assert_eq!(BoxSizing::ContentBox.to_string(), "content-box");
        assert_eq!(BoxSizing::BorderBox.to_string(), "border-box");
    }

    #[test]
    fn computed_style_box_model_defaults() {
        let s = ComputedStyle::default();
        assert_eq!(s.box_sizing, BoxSizing::ContentBox);
        assert!((s.border_radius - 0.0).abs() < f32::EPSILON);
        assert!((s.opacity - 1.0).abs() < f32::EPSILON);
    }

    // --- M3-6: WhiteSpace, Overflow, ListStyleType types ---

    #[test]
    fn white_space_defaults_and_as_ref() {
        assert_eq!(WhiteSpace::default(), WhiteSpace::Normal);
        assert_eq!(WhiteSpace::Normal.as_ref(), "normal");
        assert_eq!(WhiteSpace::Pre.as_ref(), "pre");
        assert_eq!(WhiteSpace::NoWrap.as_ref(), "nowrap");
        assert_eq!(WhiteSpace::PreWrap.as_ref(), "pre-wrap");
        assert_eq!(WhiteSpace::PreLine.as_ref(), "pre-line");
    }

    #[test]
    fn overflow_defaults_and_as_ref() {
        assert_eq!(Overflow::default(), Overflow::Visible);
        assert_eq!(Overflow::Visible.as_ref(), "visible");
        assert_eq!(Overflow::Hidden.as_ref(), "hidden");
    }

    #[test]
    fn list_style_type_defaults_and_as_ref() {
        assert_eq!(ListStyleType::default(), ListStyleType::Disc);
        assert_eq!(ListStyleType::Disc.as_ref(), "disc");
        assert_eq!(ListStyleType::Circle.as_ref(), "circle");
        assert_eq!(ListStyleType::Square.as_ref(), "square");
        assert_eq!(ListStyleType::Decimal.as_ref(), "decimal");
        assert_eq!(ListStyleType::None.as_ref(), "none");
    }

    #[test]
    fn computed_style_m3_6_defaults() {
        let s = ComputedStyle::default();
        assert_eq!(s.white_space, WhiteSpace::Normal);
        assert_eq!(s.overflow, Overflow::Visible);
        assert_eq!(s.list_style_type, ListStyleType::Disc);
        assert_eq!(s.min_width, Dimension::ZERO);
        assert_eq!(s.max_width, Dimension::Auto);
        assert_eq!(s.min_height, Dimension::ZERO);
        assert_eq!(s.max_height, Dimension::Auto);
    }

    // --- M3.5-1: Grid types ---

    #[test]
    fn display_grid_as_ref() {
        assert_eq!(Display::Grid.as_ref(), "grid");
        assert_eq!(Display::InlineGrid.as_ref(), "inline-grid");
    }

    #[test]
    fn grid_auto_flow_defaults_and_as_ref() {
        assert_eq!(GridAutoFlow::default(), GridAutoFlow::Row);
        assert_eq!(GridAutoFlow::Row.as_ref(), "row");
        assert_eq!(GridAutoFlow::Column.as_ref(), "column");
        assert_eq!(GridAutoFlow::RowDense.as_ref(), "row dense");
        assert_eq!(GridAutoFlow::ColumnDense.as_ref(), "column dense");
    }

    #[test]
    fn track_size_and_breadth_variants() {
        assert_eq!(TrackSize::default(), TrackSize::Auto);
        let ts = TrackSize::Fr(1.0);
        assert_eq!(ts, TrackSize::Fr(1.0));
        let ts_mm = TrackSize::MinMax(
            Box::new(TrackBreadth::Length(100.0)),
            Box::new(TrackBreadth::Fr(1.0)),
        );
        if let TrackSize::MinMax(min, max) = ts_mm {
            assert_eq!(*min, TrackBreadth::Length(100.0));
            assert_eq!(*max, TrackBreadth::Fr(1.0));
        } else {
            panic!("expected MinMax");
        }
    }

    #[test]
    fn grid_line_default() {
        assert_eq!(GridLine::default(), GridLine::Auto);
        assert_eq!(GridLine::Line(2), GridLine::Line(2));
        assert_eq!(GridLine::Span(3), GridLine::Span(3));
    }

    // --- M3.5-2: Table types ---

    #[test]
    fn display_table_as_ref() {
        assert_eq!(Display::Table.as_ref(), "table");
        assert_eq!(Display::InlineTable.as_ref(), "inline-table");
        assert_eq!(Display::TableCaption.as_ref(), "table-caption");
        assert_eq!(Display::TableRow.as_ref(), "table-row");
        assert_eq!(Display::TableCell.as_ref(), "table-cell");
        assert_eq!(Display::TableRowGroup.as_ref(), "table-row-group");
        assert_eq!(Display::TableHeaderGroup.as_ref(), "table-header-group");
        assert_eq!(Display::TableFooterGroup.as_ref(), "table-footer-group");
        assert_eq!(Display::TableColumn.as_ref(), "table-column");
        assert_eq!(Display::TableColumnGroup.as_ref(), "table-column-group");
    }

    #[test]
    fn border_collapse_defaults_and_as_ref() {
        assert_eq!(BorderCollapse::default(), BorderCollapse::Separate);
        assert_eq!(BorderCollapse::Separate.as_ref(), "separate");
        assert_eq!(BorderCollapse::Collapse.as_ref(), "collapse");
    }

    #[test]
    fn table_layout_defaults_and_as_ref() {
        assert_eq!(TableLayout::default(), TableLayout::Auto);
        assert_eq!(TableLayout::Auto.as_ref(), "auto");
        assert_eq!(TableLayout::Fixed.as_ref(), "fixed");
    }

    #[test]
    fn caption_side_defaults_and_as_ref() {
        assert_eq!(CaptionSide::default(), CaptionSide::Top);
        assert_eq!(CaptionSide::Top.as_ref(), "top");
        assert_eq!(CaptionSide::Bottom.as_ref(), "bottom");
    }

    #[test]
    fn computed_style_table_defaults() {
        let s = ComputedStyle::default();
        assert_eq!(s.border_collapse, BorderCollapse::Separate);
        assert!((s.border_spacing_h - 0.0).abs() < f32::EPSILON);
        assert!((s.border_spacing_v - 0.0).abs() < f32::EPSILON);
        assert_eq!(s.table_layout, TableLayout::Auto);
        assert_eq!(s.caption_side, CaptionSide::Top);
    }

    #[test]
    fn computed_style_grid_defaults() {
        let s = ComputedStyle::default();
        assert!(s.grid_template_columns.is_empty());
        assert!(s.grid_template_rows.is_empty());
        assert_eq!(s.grid_auto_flow, GridAutoFlow::Row);
        assert_eq!(s.grid_auto_columns, TrackSize::Auto);
        assert_eq!(s.grid_auto_rows, TrackSize::Auto);
        assert_eq!(s.grid_column_start, GridLine::Auto);
        assert_eq!(s.grid_column_end, GridLine::Auto);
        assert_eq!(s.grid_row_start, GridLine::Auto);
        assert_eq!(s.grid_row_end, GridLine::Auto);
    }
}
