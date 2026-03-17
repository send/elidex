//! Tests for CSS Backgrounds Level 3 property handler.

use elidex_plugin::{
    background::*, ComputedStyle, CssColor, CssPropertyHandler, CssValue, LengthUnit,
    PropertyDeclaration, ResolveContext,
};

use crate::BackgroundHandler;

fn parse(name: &str, css: &str) -> Vec<PropertyDeclaration> {
    let mut input = cssparser::ParserInput::new(css);
    let mut parser = cssparser::Parser::new(&mut input);
    BackgroundHandler.parse(name, &mut parser).unwrap()
}

fn parse_value(name: &str, css: &str) -> CssValue {
    let decls = parse(name, css);
    assert_eq!(decls.len(), 1, "expected 1 declaration for {name}: {css}");
    decls.into_iter().next().unwrap().value
}

fn parse_fails(name: &str, css: &str) {
    let mut input = cssparser::ParserInput::new(css);
    let mut parser = cssparser::Parser::new(&mut input);
    assert!(
        BackgroundHandler.parse(name, &mut parser).is_err(),
        "expected parse failure for {name}: {css}"
    );
}

// ---------------------------------------------------------------------------
// background-color
// ---------------------------------------------------------------------------

#[test]
fn parse_bg_color_named() {
    let v = parse_value("background-color", "red");
    assert_eq!(v, CssValue::Color(CssColor::RED));
}

#[test]
fn parse_bg_color_transparent() {
    let v = parse_value("background-color", "transparent");
    assert_eq!(v, CssValue::Color(CssColor::TRANSPARENT));
}

#[test]
fn parse_bg_color_hex() {
    let v = parse_value("background-color", "#00ff00");
    assert_eq!(v, CssValue::Color(CssColor::rgb(0, 255, 0)));
}

// ---------------------------------------------------------------------------
// background-image
// ---------------------------------------------------------------------------

#[test]
fn parse_bg_image_none() {
    let v = parse_value("background-image", "none");
    assert_eq!(v, CssValue::Keyword("none".to_string()));
}

#[test]
fn parse_bg_image_url() {
    let v = parse_value("background-image", "url(bg.png)");
    assert_eq!(v, CssValue::Url("bg.png".to_string()));
}

#[test]
fn parse_bg_image_url_quoted() {
    let v = parse_value("background-image", "url(\"images/bg.jpg\")");
    assert_eq!(v, CssValue::Url("images/bg.jpg".to_string()));
}

// ---------------------------------------------------------------------------
// background-repeat
// ---------------------------------------------------------------------------

#[test]
fn parse_bg_repeat_single() {
    let v = parse_value("background-repeat", "no-repeat");
    assert_eq!(v, CssValue::Keyword("no-repeat".to_string()));
}

#[test]
fn parse_bg_repeat_two_values() {
    let v = parse_value("background-repeat", "repeat no-repeat");
    assert_eq!(
        v,
        CssValue::List(vec![
            CssValue::Keyword("repeat".into()),
            CssValue::Keyword("no-repeat".into()),
        ])
    );
}

#[test]
fn parse_bg_repeat_x() {
    let v = parse_value("background-repeat", "repeat-x");
    assert_eq!(
        v,
        CssValue::List(vec![
            CssValue::Keyword("repeat".into()),
            CssValue::Keyword("no-repeat".into()),
        ])
    );
}

#[test]
fn parse_bg_repeat_y() {
    let v = parse_value("background-repeat", "repeat-y");
    assert_eq!(
        v,
        CssValue::List(vec![
            CssValue::Keyword("no-repeat".into()),
            CssValue::Keyword("repeat".into()),
        ])
    );
}

#[test]
fn parse_bg_repeat_space_round() {
    let v = parse_value("background-repeat", "space round");
    assert_eq!(
        v,
        CssValue::List(vec![
            CssValue::Keyword("space".into()),
            CssValue::Keyword("round".into()),
        ])
    );
}

// ---------------------------------------------------------------------------
// background-origin / background-clip
// ---------------------------------------------------------------------------

#[test]
fn parse_bg_origin() {
    let v = parse_value("background-origin", "content-box");
    assert_eq!(v, CssValue::Keyword("content-box".to_string()));
}

#[test]
fn parse_bg_clip() {
    let v = parse_value("background-clip", "padding-box");
    assert_eq!(v, CssValue::Keyword("padding-box".to_string()));
}

// ---------------------------------------------------------------------------
// background-attachment
// ---------------------------------------------------------------------------

#[test]
fn parse_bg_attachment_fixed() {
    let v = parse_value("background-attachment", "fixed");
    assert_eq!(v, CssValue::Keyword("fixed".to_string()));
}

#[test]
fn parse_bg_attachment_local() {
    let v = parse_value("background-attachment", "local");
    assert_eq!(v, CssValue::Keyword("local".to_string()));
}

// ---------------------------------------------------------------------------
// background-position
// ---------------------------------------------------------------------------

#[test]
fn parse_bg_position_keyword() {
    let decls = parse("background-position", "center");
    assert_eq!(decls.len(), 1);
    let v = &decls[0].value;
    assert_eq!(
        v,
        &CssValue::List(vec![CssValue::Percentage(50.0), CssValue::Percentage(50.0)])
    );
}

#[test]
fn parse_bg_position_two_values() {
    let decls = parse("background-position", "left top");
    let v = &decls[0].value;
    assert_eq!(
        v,
        &CssValue::List(vec![CssValue::Percentage(0.0), CssValue::Percentage(0.0)])
    );
}

#[test]
fn parse_bg_position_length() {
    let decls = parse("background-position", "10px 20px");
    let v = &decls[0].value;
    assert_eq!(
        v,
        &CssValue::List(vec![
            CssValue::Length(10.0, LengthUnit::Px),
            CssValue::Length(20.0, LengthUnit::Px),
        ])
    );
}

// ---------------------------------------------------------------------------
// background-size
// ---------------------------------------------------------------------------

#[test]
fn parse_bg_size_cover() {
    let v = parse_value("background-size", "cover");
    assert_eq!(v, CssValue::Keyword("cover".to_string()));
}

#[test]
fn parse_bg_size_contain() {
    let v = parse_value("background-size", "contain");
    assert_eq!(v, CssValue::Keyword("contain".to_string()));
}

#[test]
fn parse_bg_size_auto() {
    let v = parse_value("background-size", "auto");
    assert_eq!(v, CssValue::Auto);
}

#[test]
fn parse_bg_size_length() {
    let v = parse_value("background-size", "100px 50px");
    assert_eq!(
        v,
        CssValue::List(vec![
            CssValue::Length(100.0, LengthUnit::Px),
            CssValue::Length(50.0, LengthUnit::Px),
        ])
    );
}

// ---------------------------------------------------------------------------
// resolve
// ---------------------------------------------------------------------------

#[test]
fn resolve_bg_color() {
    let mut style = ComputedStyle::default();
    let ctx = ResolveContext {
        em_base: 16.0,
        root_font_size: 16.0,
        viewport_width: 800.0,
        viewport_height: 600.0,
    };
    BackgroundHandler.resolve(
        "background-color",
        &CssValue::Color(CssColor::RED),
        &ctx,
        &mut style,
    );
    assert_eq!(style.background_color, CssColor::RED);
}

// ---------------------------------------------------------------------------
// initial_value
// ---------------------------------------------------------------------------

#[test]
fn initial_values() {
    let h = BackgroundHandler;
    assert_eq!(
        h.initial_value("background-color"),
        CssValue::Color(CssColor::TRANSPARENT)
    );
    assert_eq!(
        h.initial_value("background-image"),
        CssValue::Keyword("none".into())
    );
    assert_eq!(
        h.initial_value("background-repeat"),
        CssValue::Keyword("repeat".into())
    );
    assert_eq!(
        h.initial_value("background-origin"),
        CssValue::Keyword("padding-box".into())
    );
    assert_eq!(
        h.initial_value("background-clip"),
        CssValue::Keyword("border-box".into())
    );
    assert_eq!(
        h.initial_value("background-attachment"),
        CssValue::Keyword("scroll".into())
    );
}

#[test]
fn is_not_inherited() {
    assert!(!BackgroundHandler.is_inherited("background-color"));
    assert!(!BackgroundHandler.is_inherited("background-image"));
}

#[test]
fn does_not_affect_layout() {
    assert!(!BackgroundHandler.affects_layout("background-color"));
    assert!(!BackgroundHandler.affects_layout("background-image"));
}

// ---------------------------------------------------------------------------
// resolve helpers (CssValue → typed)
// ---------------------------------------------------------------------------

#[test]
fn resolve_bg_image_none() {
    let img = crate::resolve_bg_image(&CssValue::Keyword("none".into()));
    assert_eq!(img, BackgroundImage::None);
}

#[test]
fn resolve_bg_image_url() {
    let img = crate::resolve_bg_image(&CssValue::Url("bg.png".into()));
    assert_eq!(img, BackgroundImage::Url("bg.png".to_string()));
}

#[test]
fn resolve_bg_repeat_keyword() {
    let r = crate::resolve_bg_repeat(&CssValue::Keyword("no-repeat".into()));
    assert_eq!(
        r,
        BgRepeat {
            x: BgRepeatAxis::NoRepeat,
            y: BgRepeatAxis::NoRepeat
        }
    );
}

#[test]
fn resolve_bg_repeat_two_axis() {
    let r = crate::resolve_bg_repeat(&CssValue::List(vec![
        CssValue::Keyword("repeat".into()),
        CssValue::Keyword("no-repeat".into()),
    ]));
    assert_eq!(
        r,
        BgRepeat {
            x: BgRepeatAxis::Repeat,
            y: BgRepeatAxis::NoRepeat
        }
    );
}

#[test]
fn resolve_bg_size_cover() {
    let s = crate::resolve_bg_size(&CssValue::Keyword("cover".into()));
    assert_eq!(s, BgSize::Cover);
}

#[test]
fn resolve_bg_size_auto() {
    let s = crate::resolve_bg_size(&CssValue::Auto);
    assert_eq!(s, BgSize::default());
}

#[test]
fn resolve_box_area_content_box() {
    let a = crate::resolve_box_area_keyword(&CssValue::Keyword("content-box".into()));
    assert_eq!(a, BoxArea::ContentBox);
}

#[test]
fn resolve_bg_attachment_fixed() {
    let a = crate::resolve_bg_attachment(&CssValue::Keyword("fixed".into()));
    assert_eq!(a, BgAttachment::Fixed);
}

// ---------------------------------------------------------------------------
// get_computed
// ---------------------------------------------------------------------------

#[test]
fn get_computed_bg_color() {
    let mut style = ComputedStyle::default();
    style.background_color = CssColor::RED;
    let v = BackgroundHandler.get_computed("background-color", &style);
    assert_eq!(v, CssValue::Color(CssColor::RED));
}

#[test]
fn get_computed_bg_image_none() {
    let style = ComputedStyle::default();
    let v = BackgroundHandler.get_computed("background-image", &style);
    assert_eq!(v, CssValue::Keyword("none".to_string()));
}

#[test]
fn get_computed_bg_image_url() {
    let mut style = ComputedStyle::default();
    style.background_layers = Some(
        vec![BackgroundLayer {
            image: BackgroundImage::Url("bg.png".into()),
            ..BackgroundLayer::default()
        }]
        .into_boxed_slice(),
    );
    let v = BackgroundHandler.get_computed("background-image", &style);
    assert_eq!(v, CssValue::Url("bg.png".to_string()));
}

// ---------------------------------------------------------------------------
// gradient parsing
// ---------------------------------------------------------------------------

#[test]
fn parse_linear_gradient_basic() {
    let v = parse_value("background-image", "linear-gradient(red, blue)");
    assert!(matches!(v, CssValue::Gradient(_)));
}

#[test]
fn parse_linear_gradient_with_angle() {
    let v = parse_value("background-image", "linear-gradient(45deg, red, blue)");
    if let CssValue::Gradient(g) = v {
        match *g {
            elidex_plugin::GradientValue::Linear {
                direction,
                stops,
                repeating,
            } => {
                assert_eq!(direction, elidex_plugin::AngleOrDirection::Angle(45.0));
                assert_eq!(stops.len(), 2);
                assert!(!repeating);
            }
            _ => panic!("expected linear gradient"),
        }
    } else {
        panic!("expected gradient");
    }
}

#[test]
fn parse_linear_gradient_to_right() {
    let v = parse_value("background-image", "linear-gradient(to right, red, blue)");
    if let CssValue::Gradient(g) = v {
        match *g {
            elidex_plugin::GradientValue::Linear { direction, .. } => {
                assert_eq!(
                    direction,
                    elidex_plugin::AngleOrDirection::To(vec!["right".to_string()])
                );
            }
            _ => panic!("expected linear gradient"),
        }
    } else {
        panic!("expected gradient");
    }
}

#[test]
fn parse_repeating_linear_gradient() {
    let v = parse_value(
        "background-image",
        "repeating-linear-gradient(red, blue 50px)",
    );
    if let CssValue::Gradient(g) = v {
        match *g {
            elidex_plugin::GradientValue::Linear { repeating, .. } => {
                assert!(repeating);
            }
            _ => panic!("expected linear gradient"),
        }
    } else {
        panic!("expected gradient");
    }
}

#[test]
fn parse_radial_gradient_basic() {
    let v = parse_value("background-image", "radial-gradient(red, blue)");
    assert!(matches!(v, CssValue::Gradient(_)));
}

#[test]
fn parse_radial_gradient_circle() {
    let v = parse_value("background-image", "radial-gradient(circle, red, blue)");
    if let CssValue::Gradient(g) = v {
        match *g {
            elidex_plugin::GradientValue::Radial { shape, .. } => {
                assert_eq!(shape, Some("circle".to_string()));
            }
            _ => panic!("expected radial gradient"),
        }
    } else {
        panic!("expected gradient");
    }
}

#[test]
fn parse_conic_gradient_basic() {
    let v = parse_value("background-image", "conic-gradient(red, blue)");
    assert!(matches!(v, CssValue::Gradient(_)));
}

#[test]
fn parse_conic_gradient_from_angle() {
    let v = parse_value("background-image", "conic-gradient(from 45deg, red, blue)");
    if let CssValue::Gradient(g) = v {
        match *g {
            elidex_plugin::GradientValue::Conic { from_angle, .. } => {
                assert_eq!(from_angle, Some(45.0));
            }
            _ => panic!("expected conic gradient"),
        }
    } else {
        panic!("expected gradient");
    }
}

#[test]
fn parse_gradient_with_stops() {
    let v = parse_value(
        "background-image",
        "linear-gradient(red 0%, green 50%, blue 100%)",
    );
    if let CssValue::Gradient(g) = v {
        match *g {
            elidex_plugin::GradientValue::Linear { stops, .. } => {
                assert_eq!(stops.len(), 3);
                assert_eq!(stops[0].position, Some(CssValue::Percentage(0.0)));
                assert_eq!(stops[1].position, Some(CssValue::Percentage(50.0)));
                assert_eq!(stops[2].position, Some(CssValue::Percentage(100.0)));
            }
            _ => panic!("expected linear gradient"),
        }
    } else {
        panic!("expected gradient");
    }
}

// ---------------------------------------------------------------------------
// resolve gradient
// ---------------------------------------------------------------------------

#[test]
fn resolve_linear_gradient() {
    let gradient = elidex_plugin::GradientValue::Linear {
        direction: elidex_plugin::AngleOrDirection::Angle(90.0),
        stops: vec![
            elidex_plugin::CssColorStop {
                color: CssValue::Color(CssColor::RED),
                position: None,
            },
            elidex_plugin::CssColorStop {
                color: CssValue::Color(CssColor::BLUE),
                position: None,
            },
        ],
        repeating: false,
    };
    let img = crate::resolve_bg_image(&CssValue::Gradient(Box::new(gradient)));
    match img {
        BackgroundImage::LinearGradient(lg) => {
            assert_eq!(lg.angle, 90.0);
            assert_eq!(lg.stops.len(), 2);
            assert_eq!(lg.stops[0].position, 0.0);
            assert_eq!(lg.stops[1].position, 1.0);
            assert!(!lg.repeating);
        }
        _ => panic!("expected linear gradient"),
    }
}

// ---------------------------------------------------------------------------
// background shorthand
// ---------------------------------------------------------------------------

#[test]
fn parse_background_shorthand_color() {
    let decls = parse("background", "red");
    assert!(decls.len() >= 2); // At least bg-color + bg-image
    let color_decl = decls.iter().find(|d| d.property == "background-color");
    assert!(color_decl.is_some());
    assert_eq!(color_decl.unwrap().value, CssValue::Color(CssColor::RED));
}

#[test]
fn parse_background_shorthand_url() {
    let decls = parse("background", "url(bg.png)");
    let img_decl = decls.iter().find(|d| d.property == "background-image");
    assert!(img_decl.is_some());
    assert_eq!(img_decl.unwrap().value, CssValue::Url("bg.png".to_string()));
}

#[test]
fn parse_background_shorthand_color_and_url() {
    let decls = parse("background", "url(bg.png) red");
    let color_decl = decls.iter().find(|d| d.property == "background-color");
    assert_eq!(color_decl.unwrap().value, CssValue::Color(CssColor::RED));
    let img_decl = decls.iter().find(|d| d.property == "background-image");
    assert_eq!(img_decl.unwrap().value, CssValue::Url("bg.png".to_string()));
}

#[test]
fn parse_background_shorthand_no_repeat() {
    let decls = parse("background", "url(bg.png) no-repeat");
    let repeat_decl = decls.iter().find(|d| d.property == "background-repeat");
    assert!(repeat_decl.is_some());
    assert_eq!(
        repeat_decl.unwrap().value,
        CssValue::Keyword("no-repeat".to_string())
    );
}
