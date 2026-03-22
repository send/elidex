use super::*;

// --- Custom properties + var() integration tests (M3-0) ---

#[test]
fn root_custom_properties_inherited() {
    // The div should inherit custom properties from html (:root).
    let (_dom, _div, style) = resolve_single(":root { --text-color: #ff0000; }");
    assert_eq!(
        style.custom_properties.get("--text-color"),
        Some(&"#ff0000".to_string())
    );
}

#[test]
fn var_resolves_color_from_root() {
    let (_dom, _div, style) =
        resolve_single(":root { --text-color: #ff0000; } div { color: var(--text-color); }");
    assert_eq!(style.color, CssColor::RED);
}

#[test]
fn var_resolves_background_from_root() {
    let (_dom, _div, style) =
        resolve_single(":root { --bg: #0d1117; } div { background-color: var(--bg); }");
    assert_eq!(style.background_color, CssColor::new(0x0d, 0x11, 0x17, 255));
}

#[test]
fn var_fallback_when_undefined() {
    let (_dom, _div, style) = resolve_single("div { color: var(--undefined, blue); }");
    assert_eq!(style.color, CssColor::BLUE);
}

#[test]
fn var_fallback_length_when_undefined() {
    let (_dom, _div, style) = resolve_single("div { width: var(--undefined, 100px); }");
    assert_eq!(style.width, Dimension::Length(100.0));
}

#[test]
fn var_resolves_font_size_from_root() {
    let (_dom, _div, style) = resolve_single(":root { --fs: 24px; } div { font-size: var(--fs); }");
    assert_eq!(style.font_size, 24.0);
}

// --- M3-1 Font & Text tests ---

#[test]
fn font_weight_resolution() {
    let (_dom, _div, style) = resolve_single("div { font-weight: bold; }");
    assert_eq!(style.font_weight, 700);
}

#[test]
fn font_weight_numeric_resolution() {
    let (_dom, _div, style) = resolve_single("div { font-weight: 300; }");
    assert_eq!(style.font_weight, 300);
}

#[test]
fn font_weight_inheritance() {
    let (_dom, _div, _span, _div_style, span_style) =
        resolve_with_child("div { font-weight: bold; }");
    assert_eq!(span_style.font_weight, 700);
}

#[test]
fn line_height_normal() {
    let (mut dom, _root, _html, body) = build_simple_dom();
    let div = dom.create_element("div", Attributes::default());
    dom.append_child(body, div);

    // No line-height set -> default Normal.
    resolve_styles(&mut dom, &[], Size::new(1920.0, 1080.0));

    let style = get_style(&dom, div);
    assert_eq!(style.line_height, elidex_plugin::LineHeight::Normal);
    // resolve_px gives font_size * 1.2
    let expected = style.font_size * 1.2;
    assert!((style.line_height.resolve_px(style.font_size) - expected).abs() < 0.01);
}

#[test]
fn line_height_px() {
    let (_dom, _div, style) = resolve_single("div { line-height: 24px; }");
    assert_eq!(style.line_height, elidex_plugin::LineHeight::Px(24.0));
}

#[test]
fn line_height_number_multiplier() {
    let (_dom, _div, style) = resolve_single("div { font-size: 20px; line-height: 1.5; }");
    assert_eq!(style.line_height, elidex_plugin::LineHeight::Number(1.5));
    // resolve_px recomputes per element's font-size.
    assert!((style.line_height.resolve_px(20.0) - 30.0).abs() < f32::EPSILON);
}

#[test]
fn line_height_number_inherits_semantically() {
    // CSS spec: unitless <number> is inherited as-is, recomputed per font-size.
    let (_dom, _div, _span, _div_style, span_style) =
        resolve_with_child("div { font-size: 16px; line-height: 1.5; } span { font-size: 32px; }");
    // span inherits line-height: 1.5 as Number, NOT as 24px.
    assert_eq!(
        span_style.line_height,
        elidex_plugin::LineHeight::Number(1.5)
    );
    // resolve_px with span's font-size: 32 * 1.5 = 48
    assert!((span_style.line_height.resolve_px(span_style.font_size) - 48.0).abs() < f32::EPSILON);
}

#[test]
fn text_transform_resolution() {
    let (_dom, _div, style) = resolve_single("div { text-transform: uppercase; }");
    assert_eq!(
        style.text_transform,
        elidex_plugin::TextTransform::Uppercase
    );
}

#[test]
fn text_transform_inheritance() {
    let (_dom, _div, _span, _div_style, span_style) =
        resolve_with_child("div { text-transform: capitalize; }");
    assert_eq!(
        span_style.text_transform,
        elidex_plugin::TextTransform::Capitalize
    );
}

#[test]
fn text_decoration_line_resolution() {
    let (_dom, _div, style) = resolve_single("div { text-decoration: underline; }");
    assert!(style.text_decoration_line.underline);
    assert!(!style.text_decoration_line.line_through);
}

#[test]
fn text_decoration_not_inherited() {
    let (_dom, _div, _span, div_style, span_style) =
        resolve_with_child("div { text-decoration: underline; }");
    assert!(div_style.text_decoration_line.underline);
    // text-decoration-line is NOT inherited
    assert!(!span_style.text_decoration_line.underline);
}

#[test]
fn text_decoration_multiple_values() {
    let (_dom, _div, style) = resolve_single("div { text-decoration: underline line-through; }");
    assert!(style.text_decoration_line.underline);
    assert!(style.text_decoration_line.line_through);
}

#[test]
fn sendsh_style_integration() {
    // Simulates send.sh's CSS pattern: :root defines theme variables,
    // body uses them via var().
    let (mut dom, _root, _html, body) = build_simple_dom();
    let div = dom.create_element("div", Attributes::default());
    dom.append_child(body, div);

    let css = r"
        :root {
            --color-canvas-default: #0d1117;
            --color-fg-default: #e6edf3;
        }
        body {
            background-color: var(--color-canvas-default);
            color: var(--color-fg-default);
        }
    ";
    let ss = parse_stylesheet(css, Origin::Author);
    resolve_styles(&mut dom, &[&ss], Size::new(1920.0, 1080.0));

    let body_style = get_style(&dom, body);
    assert_eq!(
        body_style.background_color,
        CssColor::new(0x0d, 0x11, 0x17, 255)
    );
    assert_eq!(body_style.color, CssColor::new(0xe6, 0xed, 0xf3, 255));

    // div inherits color from body.
    let div_style = get_style(&dom, div);
    assert_eq!(div_style.color, CssColor::new(0xe6, 0xed, 0xf3, 255));
}

// --- M3-2: Box model integration tests ---

#[test]
fn box_sizing_border_box_integration() {
    let (_dom, _div, style) = resolve_single(
        "div { box-sizing: border-box; width: 200px; padding: 10px; border: 2px solid black; }",
    );
    assert_eq!(style.box_sizing, elidex_plugin::BoxSizing::BorderBox);
    assert_eq!(style.border_top.width, 2.0);
    assert_eq!(style.border_top.style, BorderStyle::Solid);
}

#[test]
fn opacity_integration() {
    let (_dom, _div, style) = resolve_single("div { opacity: 0.5; }");
    assert!((style.opacity - 0.5).abs() < f32::EPSILON);
}

#[test]
fn border_radius_integration() {
    let (_dom, _div, style) = resolve_single("div { border-radius: 8px; }");
    assert_eq!(style.border_radii, [8.0; 4]);
}

#[test]
fn box_sizing_not_inherited_integration() {
    let (_dom, _parent, _child, parent_style, child_style) =
        resolve_with_child("div { box-sizing: border-box; }");
    assert_eq!(parent_style.box_sizing, elidex_plugin::BoxSizing::BorderBox);
    // Non-inherited: child should have content-box.
    assert_eq!(child_style.box_sizing, elidex_plugin::BoxSizing::ContentBox);
}

#[test]
fn opacity_border_radius_combined() {
    let (_dom, _div, style) =
        resolve_single("div { opacity: 0.8; border-radius: 12px; background-color: red; }");
    assert!((style.opacity - 0.8).abs() < f32::EPSILON);
    assert!((style.border_radii[0] - 12.0).abs() < f32::EPSILON);
    assert_eq!(style.background_color, CssColor::RED);
}
