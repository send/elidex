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
pub mod generated_content;
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

use elidex_css::media::{MediaEnvironment, Medium};
use elidex_css::{Declaration, Stylesheet};
use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::{ComputedStyle, CssPropertyRegistry, CssValue, Size};

pub use elidex_plugin::ViewportOverflow;
pub use generated_content::resolve_generated_content;
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

/// Serialize a property's CSSOM **resolved value** (CSSOM-1 §9 +
/// §6.7.2 "serialize a CSS value") — the string `getComputedStyle`
/// returns.
///
/// This differs from `get_computed(..).to_css_string()` (the *declared*
/// value serialization) for **color** properties only:
///
/// - A concrete [`CssValue::Color`] serializes in the CSSOM resolved/used
///   form `rgb()` / `rgba()` (CSS Color 4 §16.2.2) via
///   [`CssColor::to_resolved_value_string`][elidex_plugin::CssColor::to_resolved_value_string],
///   not the declared `#rrggbb` form.
/// - A residual `currentcolor` keyword (CSSOM-1 §9: a color longhand's
///   resolved value is its *used* value) resolves to the element's own
///   `color`. This is reachable today only via `text-decoration-color`
///   whose `None` field surfaces `currentcolor` — every other color
///   longhand is already concretized to a `CssColor` during the cascade.
///   Any future color longhand whose used value is the element color
///   inherits this resolution for free.
///
/// Every non-color value delegates to the unchanged declared-value
/// serializer [`CssValue::to_css_string`].
///
/// KNOWN GAP — list separators (slot `#11-cssvalue-list-separator-fidelity`):
/// list-valued resolved values are delegated to `to_css_string`, whose
/// `List` arm comma-joins regardless of property. That is wrong for
/// space-separated list properties (`text-decoration-line`, grid track
/// lists, …) — the correct separator is property-specific. Two ways to
/// close it:
/// - **(D)** give `CssValue::List` separator semantics (type-level
///   redesign, ~30 sites / 6 crates — the slot's primary scope; also fixes
///   the *declared*-value path / `cssText`).
/// - **(G)** since this fn already receives `property`, a property→separator
///   classification could be applied to the `List` case *here*, fixing the
///   resolved-value (getComputedStyle) path without a type change — lighter,
///   but resolved-value-only (declared-value path stays wrong) and needs a
///   CSS-wide property classification (edge-dense → its own plan-review).
///
/// Either way the fix lands in the engine, NOT in consumers: the WPT harness
/// (`elidex-wpt`) mirrors this serializer verbatim, so fixing it here fixes
/// the harness in lockstep (a harness-local list serializer was tried and
/// removed as an incomplete-by-construction generator layer — PR #385).
#[must_use]
pub fn serialize_resolved_value(property: &str, style: &ComputedStyle) -> String {
    match get_computed(property, style) {
        CssValue::Color(c) => c.to_resolved_value_string(),
        CssValue::Keyword(ref k) if k.eq_ignore_ascii_case("currentcolor") => {
            style.color.to_resolved_value_string()
        }
        other => other.to_css_string(),
    }
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
    // Screen medium — the continuous-output default. Paged/print output resolves
    // via `resolve_styles_with_compat(.., Medium::Print, ..)` so `@media print`
    // applies (mediaqueries-5 §2.3 / CSS Conditional §2).
    resolve_styles_with_compat(
        dom,
        author_stylesheets,
        &[],
        &no_hints,
        viewport,
        Medium::Screen,
        None,
    )
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
    medium: Medium,
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

    // The `@media` cascade environment (CSS Conditional §2 / mediaqueries-5).
    // Derived from the same `viewport` device-fact `ctx` uses — a derived view,
    // not a competing SoT. `medium` is caller-supplied: the screen pipeline
    // passes `Medium::Screen`, the paged/print pipeline passes `Medium::Print`
    // so `@media print` applies in paged output (mediaqueries-5 §2.3). The
    // non-viewport device facts (dppx / prefers-* / color) take
    // `MediaEnvironment::default()` until shell producers light them up (carved
    // `#11-media-prefers-features` / `#11-media-css-values-fidelity`).
    let media_env = MediaEnvironment {
        medium,
        viewport_width: f64::from(viewport.width),
        viewport_height: f64::from(viewport.height),
        root_font_size_px: 16.0,
        ..MediaEnvironment::default()
    };

    // Find the document root (entity with children but no parent and no TagType).
    // Fallback: walk all entities with TagType that have no parent.
    let roots = find_roots(dom);

    let default_parent = ComputedStyle::default();

    let mut total_shadow_css = 0;
    let mut state = WalkState {
        ctx,
        media_env,
        hint_generator,
        depth: 0,
        total_shadow_css: &mut total_shadow_css,
    };
    for root in roots {
        state.depth = 0;
        walk_tree(dom, root, &all_sheets, &default_parent, &mut state);
    }

    // Final phase: resolve CSS counters + generated content (pseudo `content`,
    // list-item markers) in document order now that the cascade has spawned
    // pseudo entities and attached every `ComputedStyle`. Writes resolved text
    // to ECS components layout + render read (single source — One-issue-one-way).
    generated_content::resolve_generated_content(dom);

    // Propagate root overflow to viewport (CSS Overflow L3 §3.1).
    walk::propagate_root_overflow(dom)
}
