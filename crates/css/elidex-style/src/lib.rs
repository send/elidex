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
//! use elidex_plugin::Size;
//! resolve_styles(&mut dom, &[&author_stylesheet], Size::new(1920.0, 1080.0));
//! ```

pub mod cascade;
pub mod counter;
pub mod inherit;
#[cfg(feature = "parallel")]
mod parallel;
mod pseudo;
pub mod resolve;
pub mod slot;
pub mod ua;
mod walk;

#[cfg(test)]
mod tests;

use std::sync::OnceLock;

use elidex_css::{Declaration, Stylesheet};
use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::{ComputedStyle, CssPropertyRegistry, CssValue, Size};

pub use elidex_plugin::ViewportOverflow;
pub use resolve::{dimension_to_css_value, get_computed_with_registry};

/// Build the CSS property registry with all standard property handlers.
#[must_use]
pub fn create_css_property_registry() -> CssPropertyRegistry {
    let mut registry = CssPropertyRegistry::new();
    elidex_css_box::BoxHandler::register(&mut registry);
    elidex_css_text::TextHandler::register(&mut registry);
    elidex_css_flex::FlexHandler::register(&mut registry);
    elidex_css_grid::GridHandler::register(&mut registry);
    elidex_css_table::TableHandler::register(&mut registry);
    elidex_css_float::FloatHandler::register(&mut registry);
    elidex_css_anim::AnimHandler::register(&mut registry);
    elidex_css_background::BackgroundHandler::register(&mut registry);
    elidex_css_transform::TransformHandler::register(&mut registry);
    elidex_css_multicol::MulticolHandler::register(&mut registry);
    registry
}

/// Default CSS property registry, lazily initialized.
static DEFAULT_REGISTRY: OnceLock<CssPropertyRegistry> = OnceLock::new();

/// Returns the default CSS property registry.
#[must_use]
pub fn default_css_property_registry() -> &'static CssPropertyRegistry {
    DEFAULT_REGISTRY.get_or_init(create_css_property_registry)
}

/// Extract a property's computed value into a [`CssValue`].
///
/// Convenience wrapper over [`get_computed_with_registry`] using the default registry.
#[must_use]
pub fn get_computed(property: &str, style: &ComputedStyle) -> CssValue {
    resolve::get_computed_with_registry(property, style, default_css_property_registry())
}

use resolve::ResolveContext;
use walk::{find_roots, walk_tree, WalkState};

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
    viewport: Size,
) -> ViewportOverflow {
    resolve_styles_with_compat(dom, author_stylesheets, &[], &no_hints, viewport, None)
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
    viewport: Size,
    _registry: Option<&CssPropertyRegistry>,
) -> ViewportOverflow {
    let ua = ua::ua_stylesheet();

    // Build the full stylesheet list: UA first, then extra UA, then author.
    let mut all_sheets: Vec<&Stylesheet> =
        Vec::with_capacity(1 + extra_ua_sheets.len() + author_stylesheets.len());
    all_sheets.push(ua);
    all_sheets.extend_from_slice(extra_ua_sheets);
    all_sheets.extend_from_slice(author_stylesheets);

    let ctx = ResolveContext {
        viewport,
        em_base: 16.0,
        root_font_size: 16.0,
    };

    // Find the document root (entity with children but no parent and no TagType).
    // Fallback: walk all entities with TagType that have no parent.
    let roots = find_roots(dom);

    let default_parent = ComputedStyle::default();

    let mut total_shadow_css = 0;
    let mut state = WalkState {
        ctx,
        hint_generator,
        depth: 0,
        total_shadow_css: &mut total_shadow_css,
    };
    for root in roots {
        state.depth = 0;
        walk_tree(dom, root, &all_sheets, &default_parent, &mut state);
    }

    // Propagate root overflow to viewport (CSS Overflow L3 §3.1).
    walk::propagate_root_overflow(dom)
}
