//! Style resolution engine (cascade, inheritance, computed values) for elidex.
//!
//! Combines parsed stylesheets with the ECS-based DOM tree to produce
//! computed style values for each element.
//!
//! # Usage
//!
//! ```ignore
//! use elidex_style::resolve_styles;
//!
//! resolve_styles(&mut dom, &[&author_stylesheet], 1920.0, 1080.0);
//! ```

pub mod cascade;
pub mod inherit;
pub mod resolve;
pub mod ua;

use elidex_css::Stylesheet;
use elidex_ecs::{EcsDom, Entity, TagType};
use elidex_plugin::ComputedStyle;

pub use resolve::{dimension_to_css_value, get_computed_as_css_value};

use cascade::{collect_and_cascade, get_inline_declarations};
use resolve::{build_computed_style, ResolveContext};

/// Resolve styles for all elements in the DOM tree.
///
/// Walks the DOM in pre-order, applying the CSS cascade and value resolution
/// to produce a [`ComputedStyle`] ECS component on each element.
///
/// The UA stylesheet is automatically prepended to the stylesheet list.
pub fn resolve_styles(
    dom: &mut EcsDom,
    author_stylesheets: &[&Stylesheet],
    viewport_width: f32,
    viewport_height: f32,
) {
    let ua = ua::ua_stylesheet();

    // Build the full stylesheet list: UA first, then author.
    let mut all_sheets: Vec<&Stylesheet> = Vec::with_capacity(1 + author_stylesheets.len());
    all_sheets.push(ua);
    all_sheets.extend_from_slice(author_stylesheets);

    let ctx = ResolveContext {
        viewport_width,
        viewport_height,
        em_base: 16.0,
        root_font_size: 16.0,
    };

    // Find the document root (entity with children but no parent and no TagType).
    // Fallback: walk all entities with TagType that have no parent.
    let roots = find_roots(dom);

    let default_parent = ComputedStyle::default();

    for root in roots {
        walk_tree(dom, root, &all_sheets, &default_parent, &ctx);
    }
}

/// Find root entities to start the tree walk.
///
/// Currently scans all entities.
/// TODO: track the document root entity directly in `EcsDom`.
fn find_roots(dom: &EcsDom) -> Vec<Entity> {
    // Collect all entities that have no parent.
    let mut roots = Vec::new();
    for (entity, ()) in &mut dom.world().query::<()>() {
        if dom.get_parent(entity).is_none() {
            roots.push(entity);
        }
    }
    roots
}

/// Pre-order tree walk: resolve styles for `entity` then recurse into children.
fn walk_tree(
    dom: &mut EcsDom,
    entity: Entity,
    stylesheets: &[&Stylesheet],
    parent_style: &ComputedStyle,
    ctx: &ResolveContext,
) {
    // Collect children first (releases immutable borrow on dom).
    let children = dom.children(entity);

    // Only resolve styles for element nodes (those with TagType).
    let is_element = dom.world().get::<&TagType>(entity).is_ok();

    let entity_style = if is_element {
        // Collect inline style declarations.
        let inline_decls = get_inline_declarations(entity, dom);

        // Cascade: collect matching declarations and determine winners.
        let winners = collect_and_cascade(entity, dom, stylesheets, &inline_decls);

        // Build resolve context with parent's font-size.
        let element_ctx = ctx.with_em_base(parent_style.font_size);

        // Resolve values → ComputedStyle.
        let style = build_computed_style(&winners, parent_style, &element_ctx);

        // Attach ComputedStyle to the entity.
        let _ = dom.world_mut().insert_one(entity, style.clone());

        style
    } else {
        // Non-element nodes (text, document root) inherit parent style.
        parent_style.clone()
    };

    // Update root_font_size for children: if this is the root element (html),
    // its font-size becomes the root font-size for rem resolution.
    let root_fs = if is_root_element(dom, entity) {
        entity_style.font_size
    } else {
        ctx.root_font_size
    };
    let child_ctx = ctx.with_em_and_root(entity_style.font_size, root_fs);

    // Recurse into children.
    for child in children {
        walk_tree(dom, child, stylesheets, &entity_style, &child_ctx);
    }
}

/// Check if entity is the `<html>` root element (tag name only).
///
/// Simplified check for the style tree walk — only needs the tag name since
/// the tree walk already processes elements in document order.
/// See also `elidex_css::selector::is_root_element` which additionally
/// verifies the parent is a document root (for selector matching).
fn is_root_element(dom: &EcsDom, entity: Entity) -> bool {
    dom.world()
        .get::<&TagType>(entity)
        .ok()
        .is_some_and(|t| t.0 == "html")
}

#[cfg(test)]
#[allow(unused_must_use)]
mod tests {
    use super::*;
    use elidex_css::{parse_stylesheet, Origin};
    use elidex_ecs::Attributes;
    use elidex_plugin::{BorderStyle, CssColor, Dimension, Display, Position};

    fn build_simple_dom() -> (EcsDom, Entity, Entity, Entity) {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let html = dom.create_element("html", Attributes::default());
        let body = dom.create_element("body", Attributes::default());
        dom.append_child(root, html);
        dom.append_child(html, body);
        (dom, root, html, body)
    }

    fn get_style(dom: &EcsDom, entity: Entity) -> ComputedStyle {
        let r = dom
            .world()
            .get::<&ComputedStyle>(entity)
            .expect("ComputedStyle not found");
        (*r).clone()
    }

    #[test]
    fn basic_style_resolution() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(body, div);

        let css = "div { color: red; display: block; }";
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let style = get_style(&dom, div);
        assert_eq!(style.color, CssColor::RED);
        assert_eq!(style.display, Display::Block);
    }

    #[test]
    fn color_inheritance() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let div = dom.create_element("div", Attributes::default());
        let span = dom.create_element("span", Attributes::default());
        dom.append_child(body, div);
        dom.append_child(div, span);

        let css = "div { color: red; }";
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let div_style = get_style(&dom, div);
        let span_style = get_style(&dom, span);
        assert_eq!(div_style.color, CssColor::RED);
        // span inherits color from div
        assert_eq!(span_style.color, CssColor::RED);
    }

    #[test]
    fn font_size_inheritance() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let div = dom.create_element("div", Attributes::default());
        let span = dom.create_element("span", Attributes::default());
        dom.append_child(body, div);
        dom.append_child(div, span);

        let css = "div { font-size: 24px; }";
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let span_style = get_style(&dom, span);
        assert_eq!(span_style.font_size, 24.0);
    }

    #[test]
    fn inline_style_overrides_author() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let mut attrs = Attributes::default();
        attrs.set("style", "color: blue");
        let div = dom.create_element("div", attrs);
        dom.append_child(body, div);

        let css = "div { color: red; }";
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let style = get_style(&dom, div);
        assert_eq!(style.color, CssColor::BLUE);
    }

    #[test]
    fn author_important_overrides_inline() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let mut attrs = Attributes::default();
        attrs.set("style", "color: blue");
        let div = dom.create_element("div", attrs);
        dom.append_child(body, div);

        let css = "div { color: red !important; }";
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let style = get_style(&dom, div);
        assert_eq!(style.color, CssColor::RED);
    }

    #[test]
    fn ua_stylesheet_body_margin() {
        let (mut dom, _root, _html, body) = build_simple_dom();

        resolve_styles(&mut dom, &[], 1920.0, 1080.0);

        let style = get_style(&dom, body);
        assert_eq!(style.margin_top, Dimension::Length(8.0));
        assert_eq!(style.margin_right, Dimension::Length(8.0));
        assert_eq!(style.margin_bottom, Dimension::Length(8.0));
        assert_eq!(style.margin_left, Dimension::Length(8.0));
    }

    #[test]
    fn inherit_keyword() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let div = dom.create_element("div", Attributes::default());
        let span = dom.create_element("span", Attributes::default());
        dom.append_child(body, div);
        dom.append_child(div, span);

        // display is non-inherited; inherit keyword forces inheritance
        let css = "div { display: flex; } span { display: inherit; }";
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let span_style = get_style(&dom, span);
        assert_eq!(span_style.display, Display::Flex);
    }

    #[test]
    fn initial_keyword() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(body, div);

        // div has UA display: block, but initial should reset to inline
        let css = "div { display: initial; }";
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let style = get_style(&dom, div);
        assert_eq!(style.display, Display::Inline);
    }

    #[test]
    fn unset_keyword() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let div = dom.create_element("div", Attributes::default());
        let span = dom.create_element("span", Attributes::default());
        dom.append_child(body, div);
        dom.append_child(div, span);

        // color is inherited: unset → inherit from parent
        // display is non-inherited: unset → initial
        let css = "div { color: red; display: flex; } span { color: unset; display: unset; }";
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let span_style = get_style(&dom, span);
        assert_eq!(span_style.color, CssColor::RED); // inherited
        assert_eq!(span_style.display, Display::Inline); // initial
    }

    #[test]
    fn mixed_ua_author_inline() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let mut attrs = Attributes::default();
        attrs.set("style", "margin-left: 20px");
        let div = dom.create_element("div", attrs);
        dom.append_child(body, div);

        let css = "div { margin-top: 10px; }";
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let style = get_style(&dom, div);
        assert_eq!(style.display, Display::Block); // UA
        assert_eq!(style.margin_top, Dimension::Length(10.0)); // Author
        assert_eq!(style.margin_left, Dimension::Length(20.0)); // Inline
    }

    #[test]
    fn empty_element_non_inherited_initial() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let div = dom.create_element("div", Attributes::default());
        let span = dom.create_element("span", Attributes::default());
        dom.append_child(body, div);
        dom.append_child(div, span);

        let css = "div { background-color: red; }";
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let div_style = get_style(&dom, div);
        let span_style = get_style(&dom, span);
        // background-color is non-inherited
        assert_eq!(div_style.background_color, CssColor::RED);
        assert_eq!(span_style.background_color, CssColor::TRANSPARENT); // initial
    }

    #[test]
    fn border_style_width_interaction() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(body, div);

        let css = "div { border-top-style: solid; border-top-width: 5px; }";
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let style = get_style(&dom, div);
        assert_eq!(style.border_top_style, BorderStyle::Solid);
        assert_eq!(style.border_top_width, 5.0);
        // Other sides have style: none → width should be 0
        assert_eq!(style.border_right_width, 0.0);
    }

    #[test]
    fn currentcolor_border() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(body, div);

        let css = "div { color: red; }";
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let style = get_style(&dom, div);
        // border-*-color initial = currentcolor → element's color
        assert_eq!(style.border_top_color, CssColor::RED);
    }

    #[test]
    fn currentcolor_background() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(body, div);

        let css = "div { color: blue; background-color: currentcolor; }";
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let style = get_style(&dom, div);
        assert_eq!(style.background_color, CssColor::BLUE);
    }

    #[test]
    fn head_display_none() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let html = dom.create_element("html", Attributes::default());
        let head = dom.create_element("head", Attributes::default());
        let body = dom.create_element("body", Attributes::default());
        dom.append_child(root, html);
        dom.append_child(html, head);
        dom.append_child(html, body);

        resolve_styles(&mut dom, &[], 1920.0, 1080.0);

        let style = get_style(&dom, head);
        assert_eq!(style.display, Display::None);
    }

    #[test]
    fn position_property() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(body, div);

        let css = "div { position: fixed; }";
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let style = get_style(&dom, div);
        assert_eq!(style.position, Position::Fixed);
    }

    #[test]
    fn rem_uses_html_font_size() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(body, div);

        let css = "html { font-size: 20px; } div { width: 2rem; }";
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let style = get_style(&dom, div);
        assert_eq!(style.width, Dimension::Length(40.0));
    }

    #[test]
    fn multiple_author_stylesheets_ordering() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(body, div);

        // ss1 has extra rules so its source_order for div{color:red} > ss2's source_order.
        // Later stylesheet (ss2) must still win.
        let ss1 = parse_stylesheet("p { display: block; } div { color: red; }", Origin::Author);
        let ss2 = parse_stylesheet("div { color: blue; }", Origin::Author);
        resolve_styles(&mut dom, &[&ss1, &ss2], 1920.0, 1080.0);

        let style = get_style(&dom, div);
        // Later stylesheet wins at same specificity.
        assert_eq!(style.color, CssColor::BLUE);
    }

    // --- Custom properties + var() integration tests (M3-0) ---

    #[test]
    fn root_custom_properties_inherited() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(body, div);

        let css = ":root { --text-color: #ff0000; }";
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        // The div should inherit custom properties from html (:root).
        let style = get_style(&dom, div);
        assert_eq!(
            style.custom_properties.get("--text-color"),
            Some(&"#ff0000".to_string())
        );
    }

    #[test]
    fn var_resolves_color_from_root() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(body, div);

        let css = ":root { --text-color: #ff0000; } div { color: var(--text-color); }";
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let style = get_style(&dom, div);
        assert_eq!(style.color, CssColor::RED);
    }

    #[test]
    fn var_resolves_background_from_root() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(body, div);

        let css = ":root { --bg: #0d1117; } div { background-color: var(--bg); }";
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let style = get_style(&dom, div);
        assert_eq!(style.background_color, CssColor::new(0x0d, 0x11, 0x17, 255));
    }

    #[test]
    fn var_fallback_when_undefined() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(body, div);

        let css = "div { color: var(--undefined, blue); }";
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let style = get_style(&dom, div);
        assert_eq!(style.color, CssColor::BLUE);
    }

    #[test]
    fn var_fallback_length_when_undefined() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(body, div);

        let css = "div { width: var(--undefined, 100px); }";
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let style = get_style(&dom, div);
        assert_eq!(style.width, Dimension::Length(100.0));
    }

    #[test]
    fn var_resolves_font_size_from_root() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(body, div);

        let css = ":root { --fs: 24px; } div { font-size: var(--fs); }";
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let style = get_style(&dom, div);
        assert_eq!(style.font_size, 24.0);
    }

    // --- M3-1 Font & Text tests ---

    #[test]
    fn font_weight_resolution() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(body, div);

        let css = "div { font-weight: bold; }";
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let style = get_style(&dom, div);
        assert_eq!(style.font_weight, 700);
    }

    #[test]
    fn font_weight_numeric_resolution() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(body, div);

        let css = "div { font-weight: 300; }";
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let style = get_style(&dom, div);
        assert_eq!(style.font_weight, 300);
    }

    #[test]
    fn font_weight_inheritance() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let div = dom.create_element("div", Attributes::default());
        let span = dom.create_element("span", Attributes::default());
        dom.append_child(body, div);
        dom.append_child(div, span);

        let css = "div { font-weight: bold; }";
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let span_style = get_style(&dom, span);
        assert_eq!(span_style.font_weight, 700);
    }

    #[test]
    fn line_height_normal() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(body, div);

        // No line-height set → default Normal.
        resolve_styles(&mut dom, &[], 1920.0, 1080.0);

        let style = get_style(&dom, div);
        assert_eq!(style.line_height, elidex_plugin::LineHeight::Normal);
        // resolve_px gives font_size * 1.2
        let expected = style.font_size * 1.2;
        assert!((style.line_height.resolve_px(style.font_size) - expected).abs() < 0.01);
    }

    #[test]
    fn line_height_px() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(body, div);

        let css = "div { line-height: 24px; }";
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let style = get_style(&dom, div);
        assert_eq!(style.line_height, elidex_plugin::LineHeight::Px(24.0));
    }

    #[test]
    fn line_height_number_multiplier() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(body, div);

        let css = "div { font-size: 20px; line-height: 1.5; }";
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let style = get_style(&dom, div);
        assert_eq!(style.line_height, elidex_plugin::LineHeight::Number(1.5));
        // resolve_px recomputes per element's font-size.
        assert!((style.line_height.resolve_px(20.0) - 30.0).abs() < f32::EPSILON);
    }

    #[test]
    fn line_height_number_inherits_semantically() {
        // CSS spec: unitless <number> is inherited as-is, recomputed per font-size.
        let (mut dom, _root, _html, body) = build_simple_dom();
        let div = dom.create_element("div", Attributes::default());
        let span = dom.create_element("span", Attributes::default());
        dom.append_child(body, div);
        dom.append_child(div, span);

        let css = "div { font-size: 16px; line-height: 1.5; } span { font-size: 32px; }";
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let span_style = get_style(&dom, span);
        // span inherits line-height: 1.5 as Number, NOT as 24px.
        assert_eq!(
            span_style.line_height,
            elidex_plugin::LineHeight::Number(1.5)
        );
        // resolve_px with span's font-size: 32 * 1.5 = 48
        assert!(
            (span_style.line_height.resolve_px(span_style.font_size) - 48.0).abs() < f32::EPSILON
        );
    }

    #[test]
    fn text_transform_resolution() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(body, div);

        let css = "div { text-transform: uppercase; }";
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let style = get_style(&dom, div);
        assert_eq!(
            style.text_transform,
            elidex_plugin::TextTransform::Uppercase
        );
    }

    #[test]
    fn text_transform_inheritance() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let div = dom.create_element("div", Attributes::default());
        let span = dom.create_element("span", Attributes::default());
        dom.append_child(body, div);
        dom.append_child(div, span);

        let css = "div { text-transform: capitalize; }";
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let span_style = get_style(&dom, span);
        assert_eq!(
            span_style.text_transform,
            elidex_plugin::TextTransform::Capitalize
        );
    }

    #[test]
    fn text_decoration_line_resolution() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(body, div);

        let css = "div { text-decoration: underline; }";
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let style = get_style(&dom, div);
        assert!(style.text_decoration_line.underline);
        assert!(!style.text_decoration_line.line_through);
    }

    #[test]
    fn text_decoration_not_inherited() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let div = dom.create_element("div", Attributes::default());
        let span = dom.create_element("span", Attributes::default());
        dom.append_child(body, div);
        dom.append_child(div, span);

        let css = "div { text-decoration: underline; }";
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let div_style = get_style(&dom, div);
        assert!(div_style.text_decoration_line.underline);

        let span_style = get_style(&dom, span);
        // text-decoration-line is NOT inherited
        assert!(!span_style.text_decoration_line.underline);
    }

    #[test]
    fn text_decoration_multiple_values() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(body, div);

        let css = "div { text-decoration: underline line-through; }";
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let style = get_style(&dom, div);
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
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

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
        let (mut dom, _root, _html, body) = build_simple_dom();
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(body, div);

        let css =
            "div { box-sizing: border-box; width: 200px; padding: 10px; border: 2px solid black; }";
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let style = get_style(&dom, div);
        assert_eq!(style.box_sizing, elidex_plugin::BoxSizing::BorderBox);
        assert_eq!(style.border_top_width, 2.0);
        assert_eq!(style.border_top_style, BorderStyle::Solid);
    }

    #[test]
    fn opacity_integration() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(body, div);

        let css = "div { opacity: 0.5; }";
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let style = get_style(&dom, div);
        assert!((style.opacity - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn border_radius_integration() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(body, div);

        let css = "div { border-radius: 8px; }";
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let style = get_style(&dom, div);
        assert!((style.border_radius - 8.0).abs() < f32::EPSILON);
    }

    #[test]
    fn box_sizing_not_inherited_integration() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let parent = dom.create_element("div", Attributes::default());
        let child = dom.create_element("span", Attributes::default());
        dom.append_child(body, parent);
        dom.append_child(parent, child);

        let css = "div { box-sizing: border-box; }";
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let parent_style = get_style(&dom, parent);
        let child_style = get_style(&dom, child);
        assert_eq!(parent_style.box_sizing, elidex_plugin::BoxSizing::BorderBox);
        // Non-inherited: child should have content-box.
        assert_eq!(child_style.box_sizing, elidex_plugin::BoxSizing::ContentBox);
    }

    #[test]
    fn opacity_border_radius_combined() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(body, div);

        let css = "div { opacity: 0.8; border-radius: 12px; background-color: red; }";
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let style = get_style(&dom, div);
        assert!((style.opacity - 0.8).abs() < f32::EPSILON);
        assert!((style.border_radius - 12.0).abs() < f32::EPSILON);
        assert_eq!(style.background_color, CssColor::RED);
    }

    // --- M3-3: Selector enhancement integration tests ---

    #[test]
    fn attr_selector_style_integration() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let mut attrs = Attributes::default();
        attrs.set("type", "text");
        let input = dom.create_element("input", attrs);
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(body, input);
        dom.append_child(body, div);

        let css = r#"[type="text"] { color: red; }"#;
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let input_style = get_style(&dom, input);
        assert_eq!(input_style.color, CssColor::RED);
        // div should not be affected.
        let div_style = get_style(&dom, div);
        assert_ne!(div_style.color, CssColor::RED);
    }

    #[test]
    fn adjacent_sibling_style_integration() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let h1 = dom.create_element("h1", Attributes::default());
        let p = dom.create_element("p", Attributes::default());
        dom.append_child(body, h1);
        dom.append_child(body, p);

        let css = "h1 + p { color: blue; }";
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let p_style = get_style(&dom, p);
        assert_eq!(p_style.color, CssColor::BLUE);
    }

    #[test]
    fn first_child_style_integration() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let li1 = dom.create_element("li", Attributes::default());
        let li2 = dom.create_element("li", Attributes::default());
        dom.append_child(body, li1);
        dom.append_child(body, li2);

        // Use background-color (non-inherited) to avoid inheritance leaks.
        let css = "li:first-child { background-color: green; }";
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let li1_style = get_style(&dom, li1);
        assert_eq!(li1_style.background_color, CssColor::new(0, 128, 0, 255));
        let li2_style = get_style(&dom, li2);
        assert_ne!(li2_style.background_color, CssColor::new(0, 128, 0, 255));
    }

    #[test]
    fn not_selector_style_integration() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let mut attrs = Attributes::default();
        attrs.set("class", "hidden");
        let hidden = dom.create_element("div", attrs);
        let visible = dom.create_element("div", Attributes::default());
        dom.append_child(body, hidden);
        dom.append_child(body, visible);

        let css = "div:not(.hidden) { color: red; }";
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let hidden_style = get_style(&dom, hidden);
        assert_ne!(hidden_style.color, CssColor::RED);
        let visible_style = get_style(&dom, visible);
        assert_eq!(visible_style.color, CssColor::RED);
    }

    #[test]
    fn child_first_child_combined_style_integration() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let ul = dom.create_element("ul", Attributes::default());
        let li1 = dom.create_element("li", Attributes::default());
        let li2 = dom.create_element("li", Attributes::default());
        dom.append_child(body, ul);
        dom.append_child(ul, li1);
        dom.append_child(ul, li2);

        let css = "ul > li:first-child { color: red; }";
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let li1_style = get_style(&dom, li1);
        assert_eq!(li1_style.color, CssColor::RED);
        let li2_style = get_style(&dom, li2);
        assert_ne!(li2_style.color, CssColor::RED);
    }
}
