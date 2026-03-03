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

use elidex_css::{Declaration, PseudoElement, Stylesheet};
use elidex_ecs::{Attributes, EcsDom, Entity, PseudoElementMarker, TagType};
use elidex_plugin::{ComputedStyle, ContentItem, ContentValue, Display};

pub use resolve::{dimension_to_css_value, get_computed_as_css_value};

use cascade::{collect_and_cascade, collect_and_cascade_pseudo, get_inline_declarations};
use resolve::{build_computed_style, ResolveContext};

/// No-op hint generator: produces no extra declarations.
fn no_hints(_entity: Entity, _dom: &EcsDom) -> Vec<Declaration> {
    Vec::new()
}

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
    resolve_styles_with_compat(
        dom,
        author_stylesheets,
        &[],
        &no_hints,
        viewport_width,
        viewport_height,
    );
}

/// Extended style resolution accepting compat layer data.
///
/// - `extra_ua_sheets`: additional UA-origin stylesheets (e.g. legacy tag rules).
/// - `hint_generator`: per-entity function producing presentational hint declarations
///   (e.g. HTML `bgcolor`, `width` attributes → CSS declarations). These participate
///   in the cascade at author-origin, specificity (0,0,0), ordered before all author
///   rules.
pub fn resolve_styles_with_compat(
    dom: &mut EcsDom,
    author_stylesheets: &[&Stylesheet],
    extra_ua_sheets: &[&Stylesheet],
    hint_generator: &dyn Fn(Entity, &EcsDom) -> Vec<Declaration>,
    viewport_width: f32,
    viewport_height: f32,
) {
    let ua = ua::ua_stylesheet();

    // Build the full stylesheet list: UA first, then extra UA, then author.
    let mut all_sheets: Vec<&Stylesheet> =
        Vec::with_capacity(1 + extra_ua_sheets.len() + author_stylesheets.len());
    all_sheets.push(ua);
    all_sheets.extend_from_slice(extra_ua_sheets);
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
        walk_tree(
            dom,
            root,
            &all_sheets,
            &default_parent,
            &ctx,
            hint_generator,
        );
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
    hint_generator: &dyn Fn(Entity, &EcsDom) -> Vec<Declaration>,
) {
    // Only resolve styles for element nodes (those with TagType).
    let is_element = dom.world().get::<&TagType>(entity).is_ok();

    let entity_style = if is_element {
        // Set LINK state for `<a href>` elements BEFORE cascade,
        // so :link pseudo-class matching can see the state.
        if is_link_element(dom, entity) {
            let mut state = dom
                .world()
                .get::<&ElementState>(entity)
                .ok()
                .map_or(ElementState::default(), |s| *s);
            // :link and :visited are mutually exclusive (Selectors §3.2).
            state.insert(ElementState::LINK);
            state.remove(ElementState::VISITED);
            let _ = dom.world_mut().insert_one(entity, state);
        }

        // Remove stale pseudo-element entities from previous resolution
        // BEFORE cascade, so :empty and other structural pseudo-classes
        // don't see generated content children.
        remove_pseudo_entities(dom, entity);

        // Collect inline style declarations.
        let inline_decls = get_inline_declarations(entity, dom);

        // Generate presentational hints for this entity.
        let extra_decls = hint_generator(entity, dom);

        // Cascade: collect matching declarations and determine winners.
        let winners = collect_and_cascade(entity, dom, stylesheets, &inline_decls, &extra_decls);

        // Build resolve context with parent's font-size.
        let element_ctx = ctx.with_em_base(parent_style.font_size);

        // Resolve values → ComputedStyle.
        let style = build_computed_style(&winners, parent_style, &element_ctx);

        // Attach ComputedStyle to the entity.
        let _ = dom.world_mut().insert_one(entity, style.clone());

        // Only generate pseudo-elements for visible elements.
        if style.display != Display::None {
            generate_pseudo_entity(
                dom,
                entity,
                stylesheets,
                &style,
                &element_ctx,
                PseudoElement::Before,
            );
            generate_pseudo_entity(
                dom,
                entity,
                stylesheets,
                &style,
                &element_ctx,
                PseudoElement::After,
            );
        }

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

    // Recurse into children (re-collect since pseudo entities may have been added).
    let children = dom.children(entity);
    for child in children {
        // Skip pseudo-element entities — they already have their ComputedStyle.
        if dom.world().get::<&PseudoElementMarker>(child).is_ok() {
            continue;
        }
        walk_tree(
            dom,
            child,
            stylesheets,
            &entity_style,
            &child_ctx,
            hint_generator,
        );
    }
}

use elidex_ecs::ElementState;

/// Check if an entity is a link element per Selectors §3.2.
///
/// Matches `<a>`, `<area>`, and `<link>` elements that have an `href` attribute.
fn is_link_element(dom: &EcsDom, entity: Entity) -> bool {
    let is_link_tag = dom
        .world()
        .get::<&TagType>(entity)
        .ok()
        .is_some_and(|t| matches!(t.0.as_str(), "a" | "area" | "link"));
    if !is_link_tag {
        return false;
    }
    dom.world()
        .get::<&Attributes>(entity)
        .ok()
        .is_some_and(|attrs| attrs.get("href").is_some())
}

/// Remove all pseudo-element child entities from a parent.
///
/// Called before generating new pseudo-elements to avoid stale entities
/// from a previous style resolution pass.
fn remove_pseudo_entities(dom: &mut EcsDom, parent: Entity) {
    let pseudo_children: Vec<Entity> = dom
        .children_iter(parent)
        .filter(|&child| dom.world().get::<&PseudoElementMarker>(child).is_ok())
        .collect();
    for pe in pseudo_children {
        let _ = dom.destroy_entity(pe);
    }
}

/// Generate a pseudo-element entity (`::before` or `::after`) if the
/// originating element has matching CSS rules with non-empty content.
fn generate_pseudo_entity(
    dom: &mut EcsDom,
    entity: Entity,
    stylesheets: &[&Stylesheet],
    parent_style: &ComputedStyle,
    ctx: &ResolveContext,
    pseudo: PseudoElement,
) {
    // Cascade pseudo-element rules.
    let winners = collect_and_cascade_pseudo(entity, dom, stylesheets, pseudo);
    if winners.is_empty() {
        return;
    }

    // Build computed style for the pseudo-element (inherits from originating element).
    let pe_style = build_computed_style(&winners, parent_style, ctx);

    // CSS Generated Content §2: on pseudo-elements, `content: normal` computes
    // to `none`.  Both `normal` and `none` suppress generation.
    let text = match &pe_style.content {
        ContentValue::Items(ref items) => resolve_content_text(items, entity, dom),
        // Normal or None → no generation.
        _ => return,
    };

    // Create the pseudo-element entity with inline display.
    let mut style = pe_style;
    // Pseudo-elements default to inline display unless explicitly set.
    if !winners.contains_key("display") {
        style.display = Display::Inline;
    }

    // Use create_text() to ensure the entity has a TreeRelation component,
    // which is required for EcsDom tree operations (append_child, destroy_entity, etc.).
    let pe_entity = dom.create_text(text);
    let _ = dom.world_mut().insert_one(pe_entity, style);
    let _ = dom.world_mut().insert_one(pe_entity, PseudoElementMarker);

    // Insert as first child (::before) or last child (::after).
    match pseudo {
        PseudoElement::Before => {
            let first_child = dom.get_first_child(entity);
            if let Some(fc) = first_child {
                let _ = dom.insert_before(entity, pe_entity, fc);
            } else {
                let _ = dom.append_child(entity, pe_entity);
            }
        }
        PseudoElement::After => {
            let _ = dom.append_child(entity, pe_entity);
        }
    }
}

/// Resolve content items to a text string.
fn resolve_content_text(items: &[ContentItem], entity: Entity, dom: &EcsDom) -> String {
    let mut result = String::new();
    for item in items {
        match item {
            ContentItem::String(s) => result.push_str(s),
            ContentItem::Attr(name) => {
                if let Ok(attrs) = dom.world().get::<&Attributes>(entity) {
                    if let Some(val) = attrs.get(name) {
                        result.push_str(val);
                    }
                }
            }
        }
    }
    result
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
    use elidex_css::{parse_stylesheet, Declaration, Origin};
    use elidex_ecs::{Attributes, TextContent};
    use elidex_plugin::{BorderStyle, CssColor, CssValue, Dimension, Display, Position};

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

    // --- M3.5-0: Pseudo-element tests ---

    #[test]
    fn pseudo_before_generates_entity() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let p = dom.create_element("p", Attributes::default());
        dom.append_child(body, p);

        let css = r#"p::before { content: ">>"; color: red; }"#;
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        // p should have a pseudo-element child.
        let children: Vec<Entity> = dom.children_iter(p).collect();
        let pe = children
            .iter()
            .find(|&&c| dom.world().get::<&PseudoElementMarker>(c).is_ok());
        assert!(pe.is_some(), "pseudo-element entity not found");
        let pe = *pe.unwrap();
        // Check text content.
        let tc = dom.world().get::<&TextContent>(pe).unwrap();
        assert_eq!(tc.0, ">>");
        // Check style.
        let pe_style = get_style(&dom, pe);
        assert_eq!(pe_style.color, CssColor::RED);
    }

    #[test]
    fn pseudo_after_generates_entity() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let p = dom.create_element("p", Attributes::default());
        dom.append_child(body, p);

        let css = r#"p::after { content: "<<"; }"#;
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let children: Vec<Entity> = dom.children_iter(p).collect();
        let last = children.last().unwrap();
        assert!(dom.world().get::<&PseudoElementMarker>(*last).is_ok());
        let tc = dom.world().get::<&TextContent>(*last).unwrap();
        assert_eq!(tc.0, "<<");
    }

    #[test]
    fn pseudo_content_none_no_entity() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let p = dom.create_element("p", Attributes::default());
        dom.append_child(body, p);

        let css = r"p::before { content: none; }";
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let children: Vec<Entity> = dom.children_iter(p).collect();
        let has_pe = children
            .iter()
            .any(|&c| dom.world().get::<&PseudoElementMarker>(c).is_ok());
        assert!(!has_pe);
    }

    #[test]
    fn pseudo_content_attr() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let mut attrs = Attributes::default();
        attrs.set("title", "TitleText");
        let p = dom.create_element("p", attrs);
        dom.append_child(body, p);

        let css = r"p::before { content: attr(title); }";
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let children: Vec<Entity> = dom.children_iter(p).collect();
        let pe = children
            .iter()
            .find(|&&c| dom.world().get::<&PseudoElementMarker>(c).is_ok())
            .unwrap();
        let tc = dom.world().get::<&TextContent>(*pe).unwrap();
        assert_eq!(tc.0, "TitleText");
    }

    #[test]
    fn pseudo_cascade_later_wins() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let mut attrs = Attributes::default();
        attrs.set("class", "x");
        let p = dom.create_element("p", attrs);
        dom.append_child(body, p);

        let css = r#".x::before { content: "A"; } .x::before { content: "B"; }"#;
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let children: Vec<Entity> = dom.children_iter(p).collect();
        let pe = children
            .iter()
            .find(|&&c| dom.world().get::<&PseudoElementMarker>(c).is_ok())
            .unwrap();
        let tc = dom.world().get::<&TextContent>(*pe).unwrap();
        assert_eq!(tc.0, "B");
    }

    #[test]
    fn pseudo_re_resolve_removes_old() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let p = dom.create_element("p", Attributes::default());
        let text = dom.create_text("hello");
        dom.append_child(body, p);
        dom.append_child(p, text);

        let css = r#"p::before { content: ">>"; }"#;
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        // First resolution: one pseudo entity + one text node.
        let pe_count1 = dom
            .children_iter(p)
            .filter(|&c| dom.world().get::<&PseudoElementMarker>(c).is_ok())
            .count();
        assert_eq!(pe_count1, 1);

        // Re-resolve: should still have exactly one PE.
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);
        let pe_count2 = dom
            .children_iter(p)
            .filter(|&c| dom.world().get::<&PseudoElementMarker>(c).is_ok())
            .count();
        assert_eq!(pe_count2, 1);
    }

    #[test]
    fn pseudo_does_not_affect_normal_element_matching() {
        // Ensure pseudo-element selectors don't affect normal element styling.
        let (mut dom, _root, _html, body) = build_simple_dom();
        let p = dom.create_element("p", Attributes::default());
        dom.append_child(body, p);

        let css = r#"p::before { content: ">>"; color: red; } p { color: blue; }"#;
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        // p itself should be blue, not red.
        let p_style = get_style(&dom, p);
        assert_eq!(p_style.color, CssColor::BLUE);
    }

    #[test]
    fn link_element_gets_link_state() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let mut attrs = Attributes::default();
        attrs.set("href", "https://example.com");
        let a = dom.create_element("a", attrs);
        dom.append_child(body, a);

        resolve_styles(&mut dom, &[], 1920.0, 1080.0);

        let state = dom
            .world()
            .get::<&ElementState>(a)
            .ok()
            .map(|s| *s)
            .unwrap_or_default();
        assert!(state.contains(ElementState::LINK));
    }

    #[test]
    fn ua_link_gets_blue_color() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let mut attrs = Attributes::default();
        attrs.set("href", "https://example.com");
        let a = dom.create_element("a", attrs);
        let text = dom.create_text("Link");
        dom.append_child(body, a);
        dom.append_child(a, text);

        resolve_styles(&mut dom, &[], 1920.0, 1080.0);

        let style = get_style(&dom, a);
        // UA a:link color is #0000ee = rgb(0, 0, 238)
        assert_eq!(style.color, CssColor::new(0, 0, 238, 255));
    }

    #[test]
    fn pseudo_before_after_full_pipeline() {
        // Full pipeline: parse CSS → resolve styles → verify pseudo entities.
        let (mut dom, _root, _html, body) = build_simple_dom();
        let p = dom.create_element("p", Attributes::default());
        let text = dom.create_text("Hello");
        dom.append_child(body, p);
        dom.append_child(p, text);

        let css = "p::before { content: \">> \"; color: red; } p::after { content: \" <<\"; color: blue; }";
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let children = dom.children(p);
        // Should have: ::before PE, text node, ::after PE = 3 children
        assert_eq!(
            children.len(),
            3,
            "expected 3 children (::before, text, ::after)"
        );

        // First child: ::before
        let before_pe = children[0];
        assert!(dom.world().get::<&PseudoElementMarker>(before_pe).is_ok());
        let before_tc = dom.world().get::<&TextContent>(before_pe).unwrap();
        assert_eq!(before_tc.0, ">> ");
        let before_style = get_style(&dom, before_pe);
        assert_eq!(before_style.color, CssColor::new(255, 0, 0, 255));

        // Last child: ::after
        let after_pe = children[2];
        assert!(dom.world().get::<&PseudoElementMarker>(after_pe).is_ok());
        let after_tc = dom.world().get::<&TextContent>(after_pe).unwrap();
        assert_eq!(after_tc.0, " <<");
        let after_style = get_style(&dom, after_pe);
        assert_eq!(after_style.color, CssColor::new(0, 0, 255, 255));

        // Middle child: original text node (no PseudoElementMarker)
        let text_node = children[1];
        assert!(dom.world().get::<&PseudoElementMarker>(text_node).is_err());
    }

    #[test]
    fn hover_pseudo_class_applies_style() {
        use elidex_ecs::ElementState as DomState;
        let (mut dom, _root, _html, body) = build_simple_dom();
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(body, div);

        let css = "div { color: black; } div:hover { color: red; }";
        let ss = parse_stylesheet(css, Origin::Author);

        // Without hover: color is black.
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);
        let style_no_hover = get_style(&dom, div);
        assert_eq!(style_no_hover.color, CssColor::new(0, 0, 0, 255));

        // Set hover state and re-resolve.
        let mut state = DomState::default();
        state.insert(DomState::HOVER);
        dom.world_mut().insert_one(div, state);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let style_hover = get_style(&dom, div);
        assert_eq!(style_hover.color, CssColor::new(255, 0, 0, 255));
    }

    #[test]
    fn pseudo_content_var_resolution() {
        let (mut dom, _root, _html, body) = build_simple_dom();
        let p = dom.create_element("p", Attributes::default());
        let text = dom.create_text("Hello");
        dom.append_child(body, p);
        dom.append_child(p, text);

        let css = r#":root { --icon: ">>"; } p::before { content: var(--icon); }"#;
        let ss = parse_stylesheet(css, Origin::Author);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        // The ::before pseudo-element should have content from var(--icon).
        let children = dom.children(p);
        let pe_children: Vec<_> = children
            .iter()
            .filter(|&&c| dom.world().get::<&PseudoElementMarker>(c).is_ok())
            .collect();
        assert_eq!(pe_children.len(), 1, "expected 1 pseudo-element");
        let tc = dom.world().get::<&TextContent>(*pe_children[0]).unwrap();
        assert_eq!(tc.0, ">>");
    }

    #[test]
    fn hover_pseudo_element_combined() {
        use elidex_ecs::ElementState as DomState;
        let (mut dom, _root, _html, body) = build_simple_dom();
        let div = dom.create_element("div", Attributes::default());
        let text = dom.create_text("Hello");
        dom.append_child(body, div);
        dom.append_child(div, text);

        let css = r#"div:hover::before { content: ">>"; color: green; }"#;
        let ss = parse_stylesheet(css, Origin::Author);

        // Without hover: no pseudo-element generated.
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);
        let children = dom.children(div);
        let pe_count = children
            .iter()
            .filter(|&&c| dom.world().get::<&PseudoElementMarker>(c).is_ok())
            .count();
        assert_eq!(pe_count, 0, "no PE without hover");

        // Set hover and re-resolve.
        let mut state = DomState::default();
        state.insert(DomState::HOVER);
        dom.world_mut().insert_one(div, state);
        resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);

        let children = dom.children(div);
        let pe_children: Vec<_> = children
            .iter()
            .filter(|&&c| dom.world().get::<&PseudoElementMarker>(c).is_ok())
            .collect();
        assert_eq!(pe_children.len(), 1, "1 PE with hover");
        let tc = dom.world().get::<&TextContent>(*pe_children[0]).unwrap();
        assert_eq!(tc.0, ">>");
        let pe_style = get_style(&dom, *pe_children[0]);
        assert_eq!(pe_style.color, CssColor::new(0, 128, 0, 255));
    }

    // --- resolve_styles_with_compat integration tests ---

    #[test]
    fn compat_extra_ua_and_hints_combined() {
        // Verify that resolve_styles_with_compat applies both extra UA sheets
        // and presentational hints from the hint_generator.
        let (mut dom, _root, _html, body) = build_simple_dom();

        // Create a <b> element (needs legacy UA for font-weight: bolder)
        let b = dom.create_element("b", Attributes::default());
        dom.append_child(body, b);

        // Create an img with bgcolor (needs hint_generator for background-color)
        let mut attrs = Attributes::default();
        attrs.set("bgcolor", "red");
        let div = dom.create_element("body", attrs);
        dom.append_child(body, div);

        // Extra UA sheet with b { font-weight: bolder; }
        let extra_ua = parse_stylesheet("b { font-weight: bolder; }", Origin::UserAgent);

        // Hint generator: emit background-color for bgcolor attribute
        let hint_gen = |entity: Entity, dom: &EcsDom| -> Vec<Declaration> {
            let Ok(attrs) = dom.world().get::<&Attributes>(entity) else {
                return Vec::new();
            };
            if let Some(val) = attrs.get("bgcolor") {
                if val == "red" {
                    return vec![Declaration::new(
                        "background-color",
                        CssValue::Color(CssColor::RED),
                    )];
                }
            }
            Vec::new()
        };

        resolve_styles_with_compat(&mut dom, &[], &[&extra_ua], &hint_gen, 1920.0, 1080.0);

        // <b> should pick up font-weight: bolder from extra UA sheet.
        let b_style = get_style(&dom, b);
        // bolder from 400 (default) → 700
        assert_eq!(b_style.font_weight, 700);

        // div with bgcolor="red" should have background-color from hint.
        let div_style = get_style(&dom, div);
        assert_eq!(div_style.background_color, CssColor::RED);
    }

    #[test]
    fn compat_hint_loses_to_author_selector() {
        // Hint (author origin, specificity (0,0,0)) should lose to author rule.
        let (mut dom, _root, _html, body) = build_simple_dom();
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(body, div);

        let author = parse_stylesheet("div { background-color: blue; }", Origin::Author);

        let hint_gen = |_entity: Entity, _dom: &EcsDom| -> Vec<Declaration> {
            vec![Declaration::new(
                "background-color",
                CssValue::Color(CssColor::RED),
            )]
        };

        resolve_styles_with_compat(&mut dom, &[&author], &[], &hint_gen, 1920.0, 1080.0);

        let style = get_style(&dom, div);
        assert_eq!(style.background_color, CssColor::BLUE);
    }
}
