use super::*;
use elidex_plugin::{CssColor, Dimension, EdgeSizes};

fn handler() -> BoxHandler {
    BoxHandler
}

fn parse(name: &str, css: &str) -> Vec<PropertyDeclaration> {
    let h = handler();
    let mut pi = cssparser::ParserInput::new(css);
    let mut parser = cssparser::Parser::new(&mut pi);
    h.parse(name, &mut parser).unwrap()
}

fn default_ctx() -> ResolveContext {
    ResolveContext {
        viewport_width: 1920.0,
        viewport_height: 1080.0,
        em_base: 16.0,
        root_font_size: 16.0,
    }
}

// --- Parse tests ---

#[test]
fn parse_display_keywords() {
    for kw in DISPLAY_KEYWORDS {
        let decls = parse("display", kw);
        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].value, CssValue::Keyword(kw.to_string()));
    }
}

#[test]
fn parse_position_keywords() {
    for kw in POSITION_KEYWORDS {
        let decls = parse("position", kw);
        assert_eq!(decls[0].value, CssValue::Keyword(kw.to_string()));
    }
}

#[test]
fn parse_width_auto() {
    let decls = parse("width", "auto");
    assert_eq!(decls[0].value, CssValue::Auto);
}

#[test]
fn parse_width_length() {
    let decls = parse("width", "100px");
    assert_eq!(decls[0].value, CssValue::Length(100.0, LengthUnit::Px));
}

#[test]
fn parse_max_width_none() {
    let decls = parse("max-width", "none");
    assert_eq!(decls[0].value, CssValue::Auto);
}

#[test]
fn parse_margin_percentage() {
    let decls = parse("margin-top", "50%");
    assert_eq!(decls[0].value, CssValue::Percentage(50.0));
}

#[test]
fn parse_padding_rejects_negative() {
    let h = handler();
    let mut pi = cssparser::ParserInput::new("-10px");
    let mut parser = cssparser::Parser::new(&mut pi);
    assert!(h.parse("padding-top", &mut parser).is_err());
}

#[test]
fn parse_border_width_keywords() {
    let decls = parse("border-top-width", "thin");
    assert_eq!(decls[0].value, CssValue::Length(1.0, LengthUnit::Px));

    let decls = parse("border-top-width", "medium");
    assert_eq!(decls[0].value, CssValue::Length(3.0, LengthUnit::Px));

    let decls = parse("border-top-width", "thick");
    assert_eq!(decls[0].value, CssValue::Length(5.0, LengthUnit::Px));
}

#[test]
fn parse_border_style() {
    let decls = parse("border-left-style", "dashed");
    assert_eq!(decls[0].value, CssValue::Keyword("dashed".to_string()));
}

#[test]
fn parse_border_color_currentcolor() {
    let decls = parse("border-top-color", "currentcolor");
    assert_eq!(
        decls[0].value,
        CssValue::Keyword("currentcolor".to_string())
    );
}

#[test]
fn parse_border_color_hex() {
    let decls = parse("border-top-color", "#ff0000");
    assert_eq!(
        decls[0].value,
        CssValue::Color(CssColor {
            r: 255,
            g: 0,
            b: 0,
            a: 255
        })
    );
}

#[test]
fn parse_opacity_clamp() {
    let decls = parse("opacity", "1.5");
    assert_eq!(decls[0].value, CssValue::Number(1.0));

    let decls = parse("opacity", "-0.5");
    assert_eq!(decls[0].value, CssValue::Number(0.0));

    let decls = parse("opacity", "0.5");
    assert_eq!(decls[0].value, CssValue::Number(0.5));
}

#[test]
fn parse_content_string() {
    let decls = parse("content", "\"hello\"");
    assert_eq!(decls[0].value, CssValue::String("hello".to_string()));
}

#[test]
fn parse_content_normal_none() {
    let decls = parse("content", "normal");
    assert_eq!(decls[0].value, CssValue::Keyword("normal".to_string()));

    let decls = parse("content", "none");
    assert_eq!(decls[0].value, CssValue::Keyword("none".to_string()));
}

#[test]
fn parse_content_attr() {
    let decls = parse("content", "attr(title)");
    assert_eq!(decls[0].value, CssValue::Keyword("attr(title)".to_string()));
}

#[test]
fn parse_box_sizing() {
    let decls = parse("box-sizing", "border-box");
    assert_eq!(decls[0].value, CssValue::Keyword("border-box".to_string()));
}

#[test]
fn parse_overflow_keywords() {
    for kw in OVERFLOW_KEYWORDS {
        let decls = parse("overflow-x", kw);
        assert_eq!(decls[0].value, CssValue::Keyword(kw.to_string()));
    }
}

#[test]
fn parse_row_gap() {
    let decls = parse("row-gap", "10px");
    assert_eq!(decls[0].value, CssValue::Length(10.0, LengthUnit::Px));
}

// --- Resolve tests ---

#[test]
fn resolve_display() {
    let h = handler();
    let ctx = default_ctx();
    let mut style = ComputedStyle::default();
    h.resolve(
        "display",
        &CssValue::Keyword("flex".into()),
        &ctx,
        &mut style,
    );
    assert_eq!(style.display, Display::Flex);
}

#[test]
fn resolve_width_and_margin() {
    let h = handler();
    let ctx = default_ctx();
    let mut style = ComputedStyle::default();
    h.resolve(
        "width",
        &CssValue::Length(200.0, LengthUnit::Px),
        &ctx,
        &mut style,
    );
    assert_eq!(style.width, Dimension::Length(200.0));

    h.resolve("margin-left", &CssValue::Auto, &ctx, &mut style);
    assert_eq!(style.margin_left, Dimension::Auto);
}

#[test]
fn resolve_padding_non_negative() {
    let h = handler();
    let ctx = default_ctx();
    let mut style = ComputedStyle::default();
    h.resolve(
        "padding-top",
        &CssValue::Length(10.0, LengthUnit::Px),
        &ctx,
        &mut style,
    );
    assert_eq!(style.padding.top, Dimension::Length(10.0));
}

#[test]
fn resolve_border_color_currentcolor() {
    let h = handler();
    let ctx = default_ctx();
    let mut style = ComputedStyle {
        color: CssColor {
            r: 0,
            g: 128,
            b: 255,
            a: 255,
        },
        ..ComputedStyle::default()
    };
    h.resolve(
        "border-top-color",
        &CssValue::Keyword("currentcolor".into()),
        &ctx,
        &mut style,
    );
    assert_eq!(
        style.border_top.color,
        CssColor {
            r: 0,
            g: 128,
            b: 255,
            a: 255
        }
    );
}

#[test]
fn resolve_opacity() {
    let h = handler();
    let ctx = default_ctx();
    let mut style = ComputedStyle::default();
    h.resolve("opacity", &CssValue::Number(0.75), &ctx, &mut style);
    assert_eq!(style.opacity, 0.75);
}

#[test]
fn resolve_content_string() {
    let h = handler();
    let ctx = default_ctx();
    let mut style = ComputedStyle::default();
    h.resolve("content", &CssValue::String(">>".into()), &ctx, &mut style);
    assert_eq!(
        style.content,
        ContentValue::Items(vec![ContentItem::String(">>".into())])
    );
}

// --- Initial value tests ---

#[test]
fn initial_values() {
    let h = handler();
    assert_eq!(
        h.initial_value("display"),
        CssValue::Keyword("inline".to_string())
    );
    assert_eq!(h.initial_value("width"), CssValue::Auto);
    assert_eq!(
        h.initial_value("padding-top"),
        CssValue::Length(0.0, LengthUnit::Px)
    );
    assert_eq!(
        h.initial_value("border-top-width"),
        CssValue::Length(3.0, LengthUnit::Px)
    );
    assert_eq!(h.initial_value("opacity"), CssValue::Number(1.0));
}

// --- Inheritance ---

#[test]
fn no_properties_inherited() {
    let h = handler();
    for name in BOX_PROPERTIES {
        assert!(!h.is_inherited(name), "{name} should not be inherited");
    }
}

// --- get_computed ---

#[test]
fn get_computed_display() {
    let h = handler();
    let style = ComputedStyle {
        display: Display::Flex,
        ..ComputedStyle::default()
    };
    assert_eq!(
        h.get_computed("display", &style),
        CssValue::Keyword("flex".to_string())
    );
}

#[test]
fn get_computed_max_width_none() {
    let h = handler();
    let style = ComputedStyle {
        max_width: Dimension::Auto,
        ..ComputedStyle::default()
    };
    assert_eq!(
        h.get_computed("max-width", &style),
        CssValue::Keyword("none".to_string())
    );
}

#[test]
fn get_computed_padding() {
    let h = handler();
    let style = ComputedStyle {
        padding: EdgeSizes {
            top: Dimension::ZERO,
            right: Dimension::ZERO,
            bottom: Dimension::ZERO,
            left: Dimension::Length(20.0),
        },
        ..ComputedStyle::default()
    };
    assert_eq!(
        h.get_computed("padding-left", &style),
        CssValue::Length(20.0, LengthUnit::Px)
    );
}

#[test]
fn get_computed_content_items() {
    let h = handler();
    let style = ComputedStyle {
        content: ContentValue::Items(vec![
            ContentItem::String("a".into()),
            ContentItem::Attr("title".into()),
        ]),
        ..ComputedStyle::default()
    };
    assert_eq!(
        h.get_computed("content", &style),
        CssValue::List(vec![
            CssValue::String("a".into()),
            CssValue::Keyword("attr:title".into()),
        ])
    );
}

#[test]
fn resolve_overflow_scroll() {
    let h = handler();
    let ctx = default_ctx();
    let mut style = ComputedStyle::default();
    h.resolve(
        "overflow-x",
        &CssValue::Keyword("scroll".into()),
        &ctx,
        &mut style,
    );
    assert_eq!(style.overflow_x, Overflow::Scroll);
}

#[test]
fn resolve_em_units() {
    let h = handler();
    let ctx = ResolveContext {
        em_base: 20.0,
        ..default_ctx()
    };
    let mut style = ComputedStyle::default();
    h.resolve(
        "width",
        &CssValue::Length(2.0, LengthUnit::Em),
        &ctx,
        &mut style,
    );
    assert_eq!(style.width, Dimension::Length(40.0));
}
