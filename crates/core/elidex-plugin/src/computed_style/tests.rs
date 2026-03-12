//! Tests for computed style types.

use super::*;

#[test]
fn default_initial_values() {
    let s = ComputedStyle::default();

    // --- Inherited properties ---
    assert_eq!(s.color, CssColor::BLACK);
    assert_eq!(s.font_size, 16.0);
    assert_eq!(s.font_weight, 400);
    assert_eq!(s.font_style, FontStyle::Normal);
    assert_eq!(s.font_family, vec!["serif".to_string()]);
    assert_eq!(s.line_height, LineHeight::Normal);
    assert_eq!(s.text_transform, TextTransform::None);
    assert_eq!(s.text_align, TextAlign::Start);
    assert_eq!(s.white_space, WhiteSpace::Normal);
    assert_eq!(s.list_style_type, ListStyleType::Disc);
    assert_eq!(s.writing_mode, WritingMode::HorizontalTb);
    assert_eq!(s.text_orientation, TextOrientation::Mixed);
    assert_eq!(s.direction, Direction::Ltr);

    // --- Non-inherited: display, position, background ---
    assert_eq!(s.display, Display::Inline);
    assert_eq!(s.position, Position::Static);
    assert_eq!(s.unicode_bidi, UnicodeBidi::Normal);
    assert_eq!(s.background_color, CssColor::TRANSPARENT);
    assert_eq!(s.overflow, Overflow::Visible);

    // --- Dimensions ---
    assert_eq!(s.width, Dimension::Auto);
    assert_eq!(s.height, Dimension::Auto);
    assert_eq!(s.min_width, Dimension::ZERO);
    assert_eq!(s.max_width, Dimension::Auto);
    assert_eq!(s.min_height, Dimension::ZERO);
    assert_eq!(s.max_height, Dimension::Auto);

    // --- Margins ---
    assert_eq!(s.margin_top, Dimension::ZERO);
    assert_eq!(s.margin_right, Dimension::ZERO);
    assert_eq!(s.margin_bottom, Dimension::ZERO);
    assert_eq!(s.margin_left, Dimension::ZERO);

    // --- Padding ---
    assert_eq!(s.padding_top, 0.0);
    assert_eq!(s.padding_right, 0.0);
    assert_eq!(s.padding_bottom, 0.0);
    assert_eq!(s.padding_left, 0.0);

    // --- Borders ---
    assert_eq!(s.border_top_width, 0.0);
    assert_eq!(s.border_right_width, 0.0);
    assert_eq!(s.border_bottom_width, 0.0);
    assert_eq!(s.border_left_width, 0.0);
    assert_eq!(s.border_top_style, BorderStyle::None);
    assert_eq!(s.border_right_style, BorderStyle::None);
    assert_eq!(s.border_bottom_style, BorderStyle::None);
    assert_eq!(s.border_left_style, BorderStyle::None);
    // currentcolor -> resolved to color (BLACK)
    assert_eq!(s.border_top_color, CssColor::BLACK);
    assert_eq!(s.border_right_color, CssColor::BLACK);
    assert_eq!(s.border_bottom_color, CssColor::BLACK);
    assert_eq!(s.border_left_color, CssColor::BLACK);

    // --- Text decoration ---
    assert_eq!(s.text_decoration_line, TextDecorationLine::default());

    // --- Box model ---
    assert_eq!(s.box_sizing, BoxSizing::ContentBox);
    assert_eq!(s.border_radius, 0.0);
    assert_eq!(s.opacity, 1.0);

    // --- Flex gap ---
    assert_eq!(s.row_gap, 0.0);
    assert_eq!(s.column_gap, 0.0);

    // --- Flex container ---
    assert_eq!(s.flex_direction, FlexDirection::Row);
    assert_eq!(s.flex_wrap, FlexWrap::Nowrap);
    assert_eq!(s.justify_content, JustifyContent::FlexStart);
    assert_eq!(s.align_items, AlignItems::Stretch);
    assert_eq!(s.align_content, AlignContent::Stretch);

    // --- Flex item ---
    assert_eq!(s.flex_grow, 0.0);
    assert_eq!(s.flex_shrink, 1.0);
    assert_eq!(s.flex_basis, Dimension::Auto);
    assert_eq!(s.order, 0);
    assert_eq!(s.align_self, AlignSelf::Auto);

    // --- Grid container ---
    assert!(s.grid_template_columns.is_empty());
    assert!(s.grid_template_rows.is_empty());
    assert_eq!(s.grid_auto_flow, GridAutoFlow::Row);
    assert_eq!(s.grid_auto_columns, TrackSize::Auto);
    assert_eq!(s.grid_auto_rows, TrackSize::Auto);

    // --- Grid item ---
    assert_eq!(s.grid_column_start, GridLine::Auto);
    assert_eq!(s.grid_column_end, GridLine::Auto);
    assert_eq!(s.grid_row_start, GridLine::Auto);
    assert_eq!(s.grid_row_end, GridLine::Auto);

    // --- Table ---
    assert_eq!(s.border_collapse, BorderCollapse::Separate);
    assert!((s.border_spacing_h - 0.0).abs() < f32::EPSILON);
    assert!((s.border_spacing_v - 0.0).abs() < f32::EPSILON);
    assert_eq!(s.table_layout, TableLayout::Auto);
    assert_eq!(s.caption_side, CaptionSide::Top);

    // --- Generated content ---
    assert_eq!(s.content, ContentValue::Normal);

    // --- Float/clear/visibility ---
    assert_eq!(s.visibility, Visibility::Visible);
    assert_eq!(s.float, Float::None);
    assert_eq!(s.clear, Clear::None);
    assert_eq!(s.vertical_align, VerticalAlign::Baseline);

    // --- Custom properties ---
    assert!(s.custom_properties.is_empty());
}

#[test]
fn keyword_enum_defaults_and_as_ref() {
    // Verify defaults
    assert_eq!(Display::default().as_ref(), "inline");
    assert_eq!(Position::default().as_ref(), "static");
    assert_eq!(BorderStyle::default().as_ref(), "none");
    assert_eq!(FlexDirection::default().as_ref(), "row");
    assert_eq!(FlexWrap::default().as_ref(), "nowrap");
    assert_eq!(JustifyContent::default().as_ref(), "flex-start");
    assert_eq!(AlignItems::default().as_ref(), "stretch");
    assert_eq!(AlignSelf::default().as_ref(), "auto");
    assert_eq!(AlignContent::default().as_ref(), "stretch");
    assert_eq!(FontStyle::default().as_ref(), "normal");
    assert_eq!(TextAlign::default().as_ref(), "start");
    assert_eq!(Direction::default().as_ref(), "ltr");
    assert_eq!(UnicodeBidi::default().as_ref(), "normal");
    assert_eq!(WritingMode::default().as_ref(), "horizontal-tb");
    assert_eq!(TextOrientation::default().as_ref(), "mixed");
    assert_eq!(TextTransform::default().as_ref(), "none");
    assert_eq!(BoxSizing::default().as_ref(), "content-box");
    assert_eq!(WhiteSpace::default().as_ref(), "normal");
    assert_eq!(Overflow::default().as_ref(), "visible");
    assert_eq!(ListStyleType::default().as_ref(), "disc");
    assert_eq!(GridAutoFlow::default().as_ref(), "row");
    assert_eq!(BorderCollapse::default().as_ref(), "separate");
    assert_eq!(TableLayout::default().as_ref(), "auto");
    assert_eq!(CaptionSide::default().as_ref(), "top");
    assert_eq!(Dimension::default(), Dimension::Auto);
    assert_eq!(Float::default().as_ref(), "none");
    assert_eq!(Clear::default().as_ref(), "none");
    assert_eq!(Visibility::default().as_ref(), "visible");

    // Spot-check as_ref values via table
    for (variant_ref, expected) in [
        (Display::Block.as_ref(), "block"),
        (Display::InlineBlock.as_ref(), "inline-block"),
        (Display::None.as_ref(), "none"),
        (Display::Flex.as_ref(), "flex"),
        (Display::InlineFlex.as_ref(), "inline-flex"),
        (Display::ListItem.as_ref(), "list-item"),
        (Display::Grid.as_ref(), "grid"),
        (Display::InlineGrid.as_ref(), "inline-grid"),
        (Display::Table.as_ref(), "table"),
        (Display::InlineTable.as_ref(), "inline-table"),
        (Display::TableCaption.as_ref(), "table-caption"),
        (Display::TableRow.as_ref(), "table-row"),
        (Display::TableCell.as_ref(), "table-cell"),
        (Display::TableRowGroup.as_ref(), "table-row-group"),
        (Display::TableHeaderGroup.as_ref(), "table-header-group"),
        (Display::TableFooterGroup.as_ref(), "table-footer-group"),
        (Display::TableColumn.as_ref(), "table-column"),
        (Display::TableColumnGroup.as_ref(), "table-column-group"),
        (FlexDirection::RowReverse.as_ref(), "row-reverse"),
        (FlexWrap::WrapReverse.as_ref(), "wrap-reverse"),
        (JustifyContent::SpaceBetween.as_ref(), "space-between"),
        (AlignItems::Center.as_ref(), "center"),
        (AlignSelf::FlexEnd.as_ref(), "flex-end"),
        (AlignContent::SpaceAround.as_ref(), "space-around"),
        (TextTransform::Uppercase.as_ref(), "uppercase"),
        (TextTransform::Lowercase.as_ref(), "lowercase"),
        (TextTransform::Capitalize.as_ref(), "capitalize"),
        (BoxSizing::BorderBox.as_ref(), "border-box"),
        (WhiteSpace::Pre.as_ref(), "pre"),
        (WhiteSpace::NoWrap.as_ref(), "nowrap"),
        (WhiteSpace::PreWrap.as_ref(), "pre-wrap"),
        (WhiteSpace::PreLine.as_ref(), "pre-line"),
        (Overflow::Hidden.as_ref(), "hidden"),
        (ListStyleType::Circle.as_ref(), "circle"),
        (ListStyleType::Square.as_ref(), "square"),
        (ListStyleType::Decimal.as_ref(), "decimal"),
        (ListStyleType::None.as_ref(), "none"),
        (GridAutoFlow::Column.as_ref(), "column"),
        (GridAutoFlow::RowDense.as_ref(), "row dense"),
        (GridAutoFlow::ColumnDense.as_ref(), "column dense"),
        (BorderCollapse::Collapse.as_ref(), "collapse"),
        (TableLayout::Fixed.as_ref(), "fixed"),
        (CaptionSide::Bottom.as_ref(), "bottom"),
        (FontStyle::Italic.as_ref(), "italic"),
        (FontStyle::Oblique.as_ref(), "oblique"),
        (TextAlign::End.as_ref(), "end"),
        (TextAlign::Left.as_ref(), "left"),
        (TextAlign::Center.as_ref(), "center"),
        (TextAlign::Right.as_ref(), "right"),
        (Direction::Rtl.as_ref(), "rtl"),
        (UnicodeBidi::Embed.as_ref(), "embed"),
        (UnicodeBidi::BidiOverride.as_ref(), "bidi-override"),
        (UnicodeBidi::Isolate.as_ref(), "isolate"),
        (UnicodeBidi::IsolateOverride.as_ref(), "isolate-override"),
        (UnicodeBidi::Plaintext.as_ref(), "plaintext"),
        (WritingMode::VerticalRl.as_ref(), "vertical-rl"),
        (WritingMode::VerticalLr.as_ref(), "vertical-lr"),
        (TextOrientation::Upright.as_ref(), "upright"),
        (TextOrientation::Sideways.as_ref(), "sideways"),
    ] {
        assert_eq!(variant_ref, expected, "as_ref mismatch for {expected}");
    }
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

#[test]
fn text_decoration_line_display() {
    let none = TextDecorationLine::default();
    assert_eq!(none.to_string(), "none");

    let ul = TextDecorationLine {
        underline: true,
        ..TextDecorationLine::default()
    };
    assert_eq!(ul.to_string(), "underline");

    let lt = TextDecorationLine {
        line_through: true,
        ..TextDecorationLine::default()
    };
    assert_eq!(lt.to_string(), "line-through");

    let both = TextDecorationLine {
        underline: true,
        line_through: true,
        ..TextDecorationLine::default()
    };
    assert_eq!(both.to_string(), "underline line-through");

    let overline = TextDecorationLine {
        overline: true,
        ..TextDecorationLine::default()
    };
    assert_eq!(overline.to_string(), "overline");

    let all = TextDecorationLine {
        underline: true,
        overline: true,
        line_through: true,
    };
    assert_eq!(all.to_string(), "underline overline line-through");
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
fn line_height_resolve_px() {
    assert_eq!(LineHeight::Normal.resolve_px(16.0), 19.2);
    assert_eq!(LineHeight::Number(1.5).resolve_px(16.0), 24.0);
    assert_eq!(LineHeight::Px(20.0).resolve_px(16.0), 20.0);
}

#[test]
fn grid_line_default() {
    assert_eq!(GridLine::default(), GridLine::Auto);
    assert_eq!(GridLine::Line(2), GridLine::Line(2));
    assert_eq!(GridLine::Span(3), GridLine::Span(3));
}

// --- keyword_enum! from_keyword() ---

#[test]
fn from_keyword_case_insensitive() {
    assert_eq!(Display::from_keyword("BLOCK"), Some(Display::Block));
    assert_eq!(Display::from_keyword("Inline"), Some(Display::Inline));
    assert_eq!(
        FlexDirection::from_keyword("ROW-REVERSE"),
        Some(FlexDirection::RowReverse)
    );
    assert_eq!(
        WhiteSpace::from_keyword("Pre-Wrap"),
        Some(WhiteSpace::PreWrap)
    );
}

#[test]
fn from_keyword_unknown_returns_none() {
    assert_eq!(Display::from_keyword("unknown"), None);
    assert_eq!(Position::from_keyword(""), None);
    assert_eq!(Overflow::from_keyword("scroll"), None);
}

#[test]
#[allow(clippy::too_many_lines)]
// Test setup with multiple assertions.
fn from_keyword_roundtrip() {
    // Every variant of every keyword_enum should roundtrip through
    // as_ref -> from_keyword. Since the macro is uniform, a typo in
    // the string literal would cause a single-variant failure.
    fn assert_roundtrips<T: Copy + PartialEq + AsRef<str> + std::fmt::Debug>(
        variants: &[T],
        from_kw: fn(&str) -> Option<T>,
    ) {
        for v in variants {
            assert_eq!(
                from_kw(v.as_ref()),
                Some(*v),
                "roundtrip failed for {:?} (\"{}\")",
                v,
                v.as_ref()
            );
        }
    }

    assert_roundtrips(
        &[
            Display::Inline,
            Display::Block,
            Display::InlineBlock,
            Display::None,
            Display::Flex,
            Display::InlineFlex,
            Display::ListItem,
            Display::Grid,
            Display::InlineGrid,
            Display::Table,
            Display::InlineTable,
            Display::TableCaption,
            Display::TableRow,
            Display::TableCell,
            Display::TableRowGroup,
            Display::TableHeaderGroup,
            Display::TableFooterGroup,
            Display::TableColumn,
            Display::TableColumnGroup,
        ],
        Display::from_keyword,
    );
    assert_roundtrips(
        &[
            Position::Static,
            Position::Relative,
            Position::Absolute,
            Position::Fixed,
            Position::Sticky,
        ],
        Position::from_keyword,
    );
    assert_roundtrips(
        &[Overflow::Visible, Overflow::Hidden],
        Overflow::from_keyword,
    );
    assert_roundtrips(
        &[BoxSizing::ContentBox, BoxSizing::BorderBox],
        BoxSizing::from_keyword,
    );
    assert_roundtrips(
        &[
            BorderStyle::None,
            BorderStyle::Hidden,
            BorderStyle::Solid,
            BorderStyle::Dashed,
            BorderStyle::Dotted,
            BorderStyle::Double,
            BorderStyle::Groove,
            BorderStyle::Ridge,
            BorderStyle::Inset,
            BorderStyle::Outset,
        ],
        BorderStyle::from_keyword,
    );
    assert_roundtrips(
        &[BorderCollapse::Separate, BorderCollapse::Collapse],
        BorderCollapse::from_keyword,
    );
    assert_roundtrips(
        &[TableLayout::Auto, TableLayout::Fixed],
        TableLayout::from_keyword,
    );
    assert_roundtrips(
        &[CaptionSide::Top, CaptionSide::Bottom],
        CaptionSide::from_keyword,
    );
    assert_roundtrips(
        &[FontStyle::Normal, FontStyle::Italic, FontStyle::Oblique],
        FontStyle::from_keyword,
    );
    assert_roundtrips(
        &[
            FlexDirection::Row,
            FlexDirection::RowReverse,
            FlexDirection::Column,
            FlexDirection::ColumnReverse,
        ],
        FlexDirection::from_keyword,
    );
    assert_roundtrips(
        &[FlexWrap::Nowrap, FlexWrap::Wrap, FlexWrap::WrapReverse],
        FlexWrap::from_keyword,
    );
    assert_roundtrips(
        &[
            JustifyContent::FlexStart,
            JustifyContent::FlexEnd,
            JustifyContent::Center,
            JustifyContent::SpaceBetween,
            JustifyContent::SpaceAround,
            JustifyContent::SpaceEvenly,
        ],
        JustifyContent::from_keyword,
    );
    assert_roundtrips(
        &[
            AlignItems::Stretch,
            AlignItems::FlexStart,
            AlignItems::FlexEnd,
            AlignItems::Center,
            AlignItems::Baseline,
        ],
        AlignItems::from_keyword,
    );
    assert_roundtrips(
        &[
            AlignContent::Stretch,
            AlignContent::FlexStart,
            AlignContent::FlexEnd,
            AlignContent::Center,
            AlignContent::SpaceBetween,
            AlignContent::SpaceAround,
            AlignContent::SpaceEvenly,
        ],
        AlignContent::from_keyword,
    );
    assert_roundtrips(
        &[
            AlignSelf::Auto,
            AlignSelf::Stretch,
            AlignSelf::FlexStart,
            AlignSelf::FlexEnd,
            AlignSelf::Center,
            AlignSelf::Baseline,
        ],
        AlignSelf::from_keyword,
    );
    assert_roundtrips(
        &[
            TextAlign::Start,
            TextAlign::End,
            TextAlign::Left,
            TextAlign::Center,
            TextAlign::Right,
        ],
        TextAlign::from_keyword,
    );
    assert_roundtrips(&[Direction::Ltr, Direction::Rtl], Direction::from_keyword);
    assert_roundtrips(
        &[
            UnicodeBidi::Normal,
            UnicodeBidi::Embed,
            UnicodeBidi::BidiOverride,
            UnicodeBidi::Isolate,
            UnicodeBidi::IsolateOverride,
            UnicodeBidi::Plaintext,
        ],
        UnicodeBidi::from_keyword,
    );
    assert_roundtrips(
        &[
            WritingMode::HorizontalTb,
            WritingMode::VerticalRl,
            WritingMode::VerticalLr,
        ],
        WritingMode::from_keyword,
    );
    assert_roundtrips(
        &[
            TextOrientation::Mixed,
            TextOrientation::Upright,
            TextOrientation::Sideways,
        ],
        TextOrientation::from_keyword,
    );
    assert_roundtrips(
        &[
            TextTransform::None,
            TextTransform::Uppercase,
            TextTransform::Lowercase,
            TextTransform::Capitalize,
        ],
        TextTransform::from_keyword,
    );
    assert_roundtrips(
        &[
            WhiteSpace::Normal,
            WhiteSpace::Pre,
            WhiteSpace::NoWrap,
            WhiteSpace::PreWrap,
            WhiteSpace::PreLine,
        ],
        WhiteSpace::from_keyword,
    );
    assert_roundtrips(
        &[
            ListStyleType::Disc,
            ListStyleType::Circle,
            ListStyleType::Square,
            ListStyleType::Decimal,
            ListStyleType::None,
        ],
        ListStyleType::from_keyword,
    );
    assert_roundtrips(
        &[
            GridAutoFlow::Row,
            GridAutoFlow::Column,
            GridAutoFlow::RowDense,
            GridAutoFlow::ColumnDense,
        ],
        GridAutoFlow::from_keyword,
    );
    assert_roundtrips(
        &[Float::None, Float::Left, Float::Right],
        Float::from_keyword,
    );
    assert_roundtrips(
        &[Clear::None, Clear::Left, Clear::Right, Clear::Both],
        Clear::from_keyword,
    );
    assert_roundtrips(
        &[
            Visibility::Visible,
            Visibility::Hidden,
            Visibility::Collapse,
        ],
        Visibility::from_keyword,
    );
}

#[test]
fn vertical_align_from_keyword() {
    assert_eq!(
        VerticalAlign::from_keyword("baseline"),
        Some(VerticalAlign::Baseline)
    );
    assert_eq!(
        VerticalAlign::from_keyword("middle"),
        Some(VerticalAlign::Middle)
    );
    assert_eq!(
        VerticalAlign::from_keyword("TEXT-TOP"),
        Some(VerticalAlign::TextTop)
    );
    assert_eq!(VerticalAlign::from_keyword("10px"), None);
}

#[test]
fn vertical_align_display() {
    assert_eq!(VerticalAlign::Baseline.to_string(), "baseline");
    assert_eq!(VerticalAlign::TextBottom.to_string(), "text-bottom");
    assert_eq!(VerticalAlign::Length(5.0).to_string(), "5px");
    assert_eq!(VerticalAlign::Percentage(50.0).to_string(), "50%");
}
