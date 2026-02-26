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
/// Currently scans all entities — acceptable for Phase 1 tree sizes.
/// Phase 2: track the document root entity directly in `EcsDom`.
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
        let element_ctx = ResolveContext {
            viewport_width: ctx.viewport_width,
            viewport_height: ctx.viewport_height,
            em_base: parent_style.font_size,
            root_font_size: ctx.root_font_size,
        };

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
    let child_ctx = if is_root_element(dom, entity) {
        ResolveContext {
            viewport_width: ctx.viewport_width,
            viewport_height: ctx.viewport_height,
            em_base: entity_style.font_size,
            root_font_size: entity_style.font_size,
        }
    } else {
        ResolveContext {
            viewport_width: ctx.viewport_width,
            viewport_height: ctx.viewport_height,
            em_base: entity_style.font_size,
            root_font_size: ctx.root_font_size,
        }
    };

    // Recurse into children.
    for child in children {
        walk_tree(dom, child, stylesheets, &entity_style, &child_ctx);
    }
}

/// Check if entity is the `<html>` root element.
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
}
