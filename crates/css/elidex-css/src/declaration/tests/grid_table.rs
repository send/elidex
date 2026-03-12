use super::*;

// --- Grid ---

#[test]
fn parse_grid_template_columns_px_fr() {
    let decls = parse_single("grid-template-columns", "100px 200px 1fr");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "grid-template-columns");
    assert_eq!(
        decls[0].value,
        CssValue::List(vec![
            CssValue::Length(100.0, LengthUnit::Px),
            CssValue::Length(200.0, LengthUnit::Px),
            CssValue::Length(1.0, LengthUnit::Fr),
        ])
    );
}

#[test]
fn parse_grid_template_rows_minmax_auto() {
    let decls = parse_single("grid-template-rows", "minmax(100px, 1fr) auto");
    assert_eq!(decls.len(), 1);
    assert_eq!(
        decls[0].value,
        CssValue::List(vec![
            CssValue::List(vec![
                CssValue::Keyword("minmax".into()),
                CssValue::Length(100.0, LengthUnit::Px),
                CssValue::Length(1.0, LengthUnit::Fr),
            ]),
            CssValue::Auto,
        ])
    );
}

#[test]
fn parse_grid_template_columns_repeat() {
    let decls = parse_single("grid-template-columns", "repeat(3, 1fr)");
    assert_eq!(decls.len(), 1);
    assert_eq!(
        decls[0].value,
        CssValue::List(vec![
            CssValue::Length(1.0, LengthUnit::Fr),
            CssValue::Length(1.0, LengthUnit::Fr),
            CssValue::Length(1.0, LengthUnit::Fr),
        ])
    );
}

#[test]
fn parse_grid_template_columns_fr_units() {
    let decls = parse_single("grid-template-columns", "1fr 2fr");
    assert_eq!(decls.len(), 1);
    assert_eq!(
        decls[0].value,
        CssValue::List(vec![
            CssValue::Length(1.0, LengthUnit::Fr),
            CssValue::Length(2.0, LengthUnit::Fr),
        ])
    );
}

#[test]
fn parse_grid_column_start_span() {
    let decls = parse_single("grid-column-start", "span 2");
    assert_eq!(decls.len(), 1);
    assert_eq!(
        decls[0].value,
        CssValue::List(vec![
            CssValue::Keyword("span".into()),
            CssValue::Number(2.0),
        ])
    );
}

#[test]
fn parse_grid_column_shorthand() {
    let decls = parse_single("grid-column", "1 / 3");
    assert_eq!(decls.len(), 2);
    assert_eq!(decls[0].property, "grid-column-start");
    assert_eq!(decls[0].value, CssValue::Number(1.0));
    assert_eq!(decls[1].property, "grid-column-end");
    assert_eq!(decls[1].value, CssValue::Number(3.0));
}

#[test]
fn parse_grid_area_shorthand() {
    let decls = parse_single("grid-area", "1 / 2 / 3 / 4");
    assert_eq!(decls.len(), 4);
    assert_eq!(decls[0].property, "grid-row-start");
    assert_eq!(decls[0].value, CssValue::Number(1.0));
    assert_eq!(decls[1].property, "grid-column-start");
    assert_eq!(decls[1].value, CssValue::Number(2.0));
    assert_eq!(decls[2].property, "grid-row-end");
    assert_eq!(decls[2].value, CssValue::Number(3.0));
    assert_eq!(decls[3].property, "grid-column-end");
    assert_eq!(decls[3].value, CssValue::Number(4.0));
}

#[test]
fn parse_grid_column_inherit() {
    let decls = parse_single("grid-column", "inherit");
    assert_eq!(decls.len(), 2);
    assert_eq!(decls[0].property, "grid-column-start");
    assert_eq!(decls[0].value, CssValue::Inherit);
    assert_eq!(decls[1].property, "grid-column-end");
    assert_eq!(decls[1].value, CssValue::Inherit);
}

// --- Table: display variants ---

#[test]
fn parse_display_table_variants() {
    for kw in [
        "table",
        "inline-table",
        "table-caption",
        "table-row",
        "table-cell",
        "table-row-group",
        "table-header-group",
        "table-footer-group",
        "table-column",
        "table-column-group",
    ] {
        let decls = parse_single("display", kw);
        assert_eq!(decls.len(), 1, "display: {kw}");
        assert_eq!(
            decls[0].value,
            CssValue::Keyword(kw.into()),
            "display: {kw}"
        );
    }
}

// --- Table: border-spacing ---

#[test]
fn parse_border_spacing_one_value() {
    let decls = parse_single("border-spacing", "2px");
    assert_eq!(decls.len(), 2);
    assert_eq!(decls[0].property, "border-spacing-h");
    assert_eq!(decls[0].value, CssValue::Length(2.0, LengthUnit::Px));
    assert_eq!(decls[1].property, "border-spacing-v");
    assert_eq!(decls[1].value, CssValue::Length(2.0, LengthUnit::Px));
}

#[test]
fn parse_border_spacing_two_values() {
    let decls = parse_single("border-spacing", "2px 4px");
    assert_eq!(decls.len(), 2);
    assert_eq!(decls[0].property, "border-spacing-h");
    assert_eq!(decls[0].value, CssValue::Length(2.0, LengthUnit::Px));
    assert_eq!(decls[1].property, "border-spacing-v");
    assert_eq!(decls[1].value, CssValue::Length(4.0, LengthUnit::Px));
}

#[test]
fn parse_border_spacing_inherit() {
    let decls = parse_single("border-spacing", "inherit");
    assert_eq!(decls.len(), 2);
    assert_eq!(decls[0].property, "border-spacing-h");
    assert_eq!(decls[0].value, CssValue::Inherit);
    assert_eq!(decls[1].property, "border-spacing-v");
    assert_eq!(decls[1].value, CssValue::Inherit);
}
