use super::*;
use cssparser::ParserInput;

// ---------------------------------------------------------------------------
// Grid-line named value tests
// ---------------------------------------------------------------------------

fn parse_line(css: &str) -> Vec<Declaration> {
    let mut input = ParserInput::new(css);
    let mut parser = Parser::new(&mut input);
    parse_grid_line(&mut parser, "grid-column-start")
}

#[test]
fn grid_line_named_ident() {
    let decls = parse_line("header");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("header".into()));
}

#[test]
fn grid_line_integer_ident() {
    // "2 header" → List([Number(2), Keyword("header")])
    let decls = parse_line("2 header");
    assert_eq!(decls.len(), 1);
    if let CssValue::List(items) = &decls[0].value {
        assert_eq!(items.len(), 2);
        assert_eq!(items[0], CssValue::Number(2.0));
        assert_eq!(items[1], CssValue::Keyword("header".into()));
    } else {
        panic!("expected List, got {:?}", decls[0].value);
    }
}

#[test]
fn grid_line_ident_integer() {
    // "header 2" → List([Keyword("header"), Number(2)])
    let decls = parse_line("header 2");
    assert_eq!(decls.len(), 1);
    if let CssValue::List(items) = &decls[0].value {
        assert_eq!(items.len(), 2);
        assert_eq!(items[0], CssValue::Keyword("header".into()));
        assert_eq!(items[1], CssValue::Number(2.0));
    } else {
        panic!("expected List, got {:?}", decls[0].value);
    }
}

#[test]
fn grid_line_span_named() {
    // "span header" → List([Keyword("span-named"), Number(1), Keyword("header")])
    let decls = parse_line("span header");
    assert_eq!(decls.len(), 1);
    if let CssValue::List(items) = &decls[0].value {
        assert_eq!(items.len(), 3);
        assert_eq!(items[0], CssValue::Keyword("span-named".into()));
        assert_eq!(items[1], CssValue::Number(1.0));
        assert_eq!(items[2], CssValue::Keyword("header".into()));
    } else {
        panic!("expected List, got {:?}", decls[0].value);
    }
}

#[test]
fn grid_line_span_integer_named() {
    // "span 2 header" → List([Keyword("span-named"), Number(2), Keyword("header")])
    let decls = parse_line("span 2 header");
    assert_eq!(decls.len(), 1);
    if let CssValue::List(items) = &decls[0].value {
        assert_eq!(items.len(), 3);
        assert_eq!(items[0], CssValue::Keyword("span-named".into()));
        assert_eq!(items[1], CssValue::Number(2.0));
        assert_eq!(items[2], CssValue::Keyword("header".into()));
    } else {
        panic!("expected List, got {:?}", decls[0].value);
    }
}

#[test]
fn grid_line_forbidden_ident_rejected() {
    // "auto" → CssValue::Auto (not Named("auto"))
    let decls = parse_line("auto");
    assert_eq!(decls.len(), 1);
    assert_eq!(
        decls[0].value,
        CssValue::Auto,
        "auto keyword should yield Auto, not Named"
    );

    // "span" alone (no integer or ident) → empty (rejected)
    let decls = parse_line("span");
    assert!(
        decls.is_empty(),
        "bare span with no target should be rejected, got {decls:?}"
    );
}

// ---------------------------------------------------------------------------
// Grid-template-areas tests
// ---------------------------------------------------------------------------

fn parse_areas(css: &str) -> Vec<Declaration> {
    let mut input = ParserInput::new(css);
    let mut parser = Parser::new(&mut input);
    parse_grid_template_areas(&mut parser)
}

#[test]
fn areas_basic_2x2() {
    let decls = parse_areas(r#""header header" "sidebar main""#);
    assert_eq!(decls.len(), 1);
    if let CssValue::List(rows) = &decls[0].value {
        assert_eq!(rows.len(), 2, "expected 2 rows");
        if let CssValue::List(row0) = &rows[0] {
            assert_eq!(row0.len(), 2);
            assert_eq!(row0[0], CssValue::Keyword("header".into()));
            assert_eq!(row0[1], CssValue::Keyword("header".into()));
        } else {
            panic!("row 0 should be List");
        }
        if let CssValue::List(row1) = &rows[1] {
            assert_eq!(row1.len(), 2);
            assert_eq!(row1[0], CssValue::Keyword("sidebar".into()));
            assert_eq!(row1[1], CssValue::Keyword("main".into()));
        } else {
            panic!("row 1 should be List");
        }
    } else {
        panic!("expected List of rows, got {:?}", decls[0].value);
    }
}

#[test]
fn areas_null_cell() {
    let decls = parse_areas(r#""a ." ". b""#);
    assert_eq!(decls.len(), 1);
    if let CssValue::List(rows) = &decls[0].value {
        assert_eq!(rows.len(), 2);
        if let CssValue::List(row0) = &rows[0] {
            assert_eq!(
                row0[1],
                CssValue::Keyword(".".into()),
                "null cell should be '.'"
            );
        } else {
            panic!("row 0 should be List");
        }
        if let CssValue::List(row1) = &rows[1] {
            assert_eq!(
                row1[0],
                CssValue::Keyword(".".into()),
                "null cell should be '.'"
            );
        } else {
            panic!("row 1 should be List");
        }
    } else {
        panic!("expected List, got {:?}", decls[0].value);
    }
}

#[test]
fn areas_non_rectangular_rejected() {
    // "a" appears in non-rectangular positions — invalid
    let decls = parse_areas(r#""a a" "a b""#);
    assert!(
        decls.is_empty(),
        "non-rectangular area should be rejected, got {decls:?}"
    );
}

#[test]
fn areas_none_keyword() {
    let decls = parse_areas("none");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("none".into()));
}

#[test]
fn areas_unequal_columns_rejected() {
    // "a b" has 2 cols, "c" has 1 col — mismatch → rejected
    let decls = parse_areas(r#""a b" "c""#);
    assert!(
        decls.is_empty(),
        "unequal column counts should be rejected, got {decls:?}"
    );
}

// ---------------------------------------------------------------------------
// Grid-area shorthand named ident tests
// ---------------------------------------------------------------------------

fn parse_area_shorthand(css: &str) -> Vec<Declaration> {
    let mut input = ParserInput::new(css);
    let mut parser = Parser::new(&mut input);
    parse_grid_area(&mut parser)
}

#[test]
fn grid_area_named_single() {
    // "header" → all 4 values should be Keyword("header")
    let decls = parse_area_shorthand("header");
    assert_eq!(decls.len(), 4);
    for decl in &decls {
        assert_eq!(
            decl.value,
            CssValue::Keyword("header".into()),
            "all slots should be 'header', got {:?}",
            decl.value
        );
    }
}

#[test]
fn grid_area_named_two() {
    // "header / sidebar" → row-start: header, col-start: sidebar, row-end: header, col-end: sidebar
    let decls = parse_area_shorthand("header / sidebar");
    assert_eq!(decls.len(), 4);
    // grid-row-start
    assert_eq!(decls[0].property, "grid-row-start");
    assert_eq!(decls[0].value, CssValue::Keyword("header".into()));
    // grid-column-start
    assert_eq!(decls[1].property, "grid-column-start");
    assert_eq!(decls[1].value, CssValue::Keyword("sidebar".into()));
    // grid-row-end: copies row-start (header)
    assert_eq!(decls[2].property, "grid-row-end");
    assert_eq!(decls[2].value, CssValue::Keyword("header".into()));
    // grid-column-end: copies col-start (sidebar)
    assert_eq!(decls[3].property, "grid-column-end");
    assert_eq!(decls[3].value, CssValue::Keyword("sidebar".into()));
}

#[test]
fn grid_area_numeric_defaults_auto() {
    // "1" → row-start: Number(1), col-start: Auto, row-end: Auto, col-end: Auto
    let decls = parse_area_shorthand("1");
    assert_eq!(decls.len(), 4);
    assert_eq!(
        decls[0].value,
        CssValue::Number(1.0),
        "row-start should be Number(1)"
    );
    assert_eq!(decls[1].value, CssValue::Auto, "col-start should be Auto");
    assert_eq!(decls[2].value, CssValue::Auto, "row-end should be Auto");
    assert_eq!(decls[3].value, CssValue::Auto, "col-end should be Auto");
}

#[test]
fn grid_area_numeric_two() {
    // "1 / 2" → row-start: Number(1), col-start: Number(2), row-end: Auto, col-end: Auto
    let decls = parse_area_shorthand("1 / 2");
    assert_eq!(decls.len(), 4);
    assert_eq!(
        decls[0].value,
        CssValue::Number(1.0),
        "row-start should be Number(1)"
    );
    assert_eq!(
        decls[1].value,
        CssValue::Number(2.0),
        "col-start should be Number(2)"
    );
    assert_eq!(decls[2].value, CssValue::Auto, "row-end should be Auto");
    assert_eq!(decls[3].value, CssValue::Auto, "col-end should be Auto");
}

// ---------------------------------------------------------------------------
// grid-template shorthand tests
// ---------------------------------------------------------------------------

fn parse_template_shorthand(css: &str) -> Vec<Declaration> {
    let mut input = ParserInput::new(css);
    let mut parser = Parser::new(&mut input);
    parse_grid_template_shorthand(&mut parser)
}

#[test]
fn grid_template_none() {
    // "none" → all 3 longhands Keyword("none")
    let decls = parse_template_shorthand("none");
    assert_eq!(decls.len(), 3);
    for decl in &decls {
        assert_eq!(
            decl.value,
            CssValue::Keyword("none".into()),
            "all template longhands should be 'none', got {:?} for {}",
            decl.value,
            decl.property
        );
    }
    let names: Vec<&str> = decls.iter().map(|d| d.property.as_str()).collect();
    assert!(names.contains(&"grid-template-rows"));
    assert!(names.contains(&"grid-template-columns"));
    assert!(names.contains(&"grid-template-areas"));
}

#[test]
fn grid_template_rows_cols() {
    // "100px / 200px 1fr" → rows has 100px, cols has 200px 1fr, areas = none
    let decls = parse_template_shorthand("100px / 200px 1fr");
    assert!(
        !decls.is_empty(),
        "100px / 200px 1fr should produce declarations"
    );
    let areas = decls.iter().find(|d| d.property == "grid-template-areas");
    assert!(areas.is_some(), "should have grid-template-areas");
    assert_eq!(
        areas.unwrap().value,
        CssValue::Keyword("none".into()),
        "areas should be none"
    );
    let rows = decls.iter().find(|d| d.property == "grid-template-rows");
    assert!(rows.is_some(), "should have grid-template-rows");
    let cols = decls.iter().find(|d| d.property == "grid-template-columns");
    assert!(cols.is_some(), "should have grid-template-columns");
    // cols should have 2 tracks
    if let CssValue::List(col_tracks) = &cols.unwrap().value {
        assert_eq!(col_tracks.len(), 2, "columns should have 2 tracks");
    }
}

#[test]
fn grid_template_with_areas() {
    // Areas pattern: "a a" 60px / 200px 1fr
    let decls = parse_template_shorthand(r#""a a" 60px / 200px 1fr"#);
    assert!(
        !decls.is_empty(),
        r#""a a" 60px / 200px 1fr should produce declarations"#
    );
    let names: Vec<&str> = decls.iter().map(|d| d.property.as_str()).collect();
    assert!(
        names.contains(&"grid-template-rows"),
        "should have grid-template-rows"
    );
    assert!(
        names.contains(&"grid-template-columns"),
        "should have grid-template-columns"
    );
    assert!(
        names.contains(&"grid-template-areas"),
        "should have grid-template-areas"
    );
    // Areas should not be "none"
    let areas = decls
        .iter()
        .find(|d| d.property == "grid-template-areas")
        .unwrap();
    assert_ne!(
        areas.value,
        CssValue::Keyword("none".into()),
        "areas should contain actual area data"
    );
}

// ---------------------------------------------------------------------------
// grid shorthand tests
// ---------------------------------------------------------------------------

fn parse_grid_sh(css: &str) -> Vec<Declaration> {
    let mut input = ParserInput::new(css);
    let mut parser = Parser::new(&mut input);
    parse_grid_shorthand(&mut parser)
}

#[test]
fn grid_shorthand_as_template() {
    // "100px / 200px" → includes grid-template-rows, grid-template-columns, grid-template-areas, and auto-* resets
    let decls = parse_grid_sh("100px / 200px");
    assert!(
        !decls.is_empty(),
        "100px / 200px should produce declarations"
    );
    let names: Vec<&str> = decls.iter().map(|d| d.property.as_str()).collect();
    assert!(
        names.contains(&"grid-template-rows"),
        "should have grid-template-rows"
    );
    assert!(
        names.contains(&"grid-template-columns"),
        "should have grid-template-columns"
    );
    assert!(
        names.contains(&"grid-template-areas"),
        "should have grid-template-areas"
    );
    // auto-* resets
    assert!(
        names.contains(&"grid-auto-flow"),
        "should have grid-auto-flow"
    );
    assert!(
        names.contains(&"grid-auto-rows"),
        "should have grid-auto-rows"
    );
    assert!(
        names.contains(&"grid-auto-columns"),
        "should have grid-auto-columns"
    );
}

#[test]
fn grid_shorthand_auto_flow_rows() {
    // "auto-flow / 200px 1fr" → grid-auto-flow = "row" (CSS Grid §7.4)
    let decls = parse_grid_sh("auto-flow / 200px 1fr");
    assert!(
        !decls.is_empty(),
        "auto-flow / 200px 1fr should produce declarations"
    );
    let auto_flow = decls.iter().find(|d| d.property == "grid-auto-flow");
    assert!(auto_flow.is_some(), "should have grid-auto-flow");
    assert_eq!(
        auto_flow.unwrap().value,
        CssValue::Keyword("row".into()),
        "auto-flow / cols should set flow to row (CSS Grid §7.4)"
    );
    let rows = decls.iter().find(|d| d.property == "grid-template-rows");
    assert!(rows.is_some(), "should have grid-template-rows");
    assert_eq!(
        rows.unwrap().value,
        CssValue::Keyword("none".into()),
        "grid-template-rows should be none"
    );
}

#[test]
fn grid_shorthand_auto_flow_dense() {
    // "auto-flow dense / 200px" → grid-auto-flow = "row dense" (CSS Grid §7.4)
    let decls = parse_grid_sh("auto-flow dense / 200px");
    assert!(
        !decls.is_empty(),
        "auto-flow dense / 200px should produce declarations"
    );
    let auto_flow = decls.iter().find(|d| d.property == "grid-auto-flow");
    assert!(auto_flow.is_some(), "should have grid-auto-flow");
    assert_eq!(
        auto_flow.unwrap().value,
        CssValue::Keyword("row dense".into()),
        "auto-flow dense / cols should set flow to row dense (CSS Grid §7.4)"
    );
}

#[test]
fn grid_shorthand_rows_auto_flow() {
    // "100px / auto-flow 200px" → grid-auto-flow = "column" (CSS Grid §7.4)
    let decls = parse_grid_sh("100px / auto-flow 200px");
    assert!(
        !decls.is_empty(),
        "100px / auto-flow 200px should produce declarations"
    );
    let auto_flow = decls.iter().find(|d| d.property == "grid-auto-flow");
    assert!(auto_flow.is_some(), "should have grid-auto-flow");
    assert_eq!(
        auto_flow.unwrap().value,
        CssValue::Keyword("column".into()),
        "rows / auto-flow should set flow to column (CSS Grid §7.4)"
    );
    let cols = decls.iter().find(|d| d.property == "grid-template-columns");
    assert!(cols.is_some(), "should have grid-template-columns");
    assert_eq!(
        cols.unwrap().value,
        CssValue::Keyword("none".into()),
        "grid-template-columns should be none"
    );
    let auto_cols = decls.iter().find(|d| d.property == "grid-auto-columns");
    assert!(auto_cols.is_some(), "should have grid-auto-columns");
}
