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

/// The **value-kind** of a shorthand component longhand, classified on its
/// *serialized string* for the CSSOM §6.7.2 step 1.2 gate below.
///
/// The gate serves BOTH the inline `el.style` path and the rule path, and
/// `InlineStyle` is a string-backed CSSOM store (`InlineDeclaration { value:
/// String }`) — the inline path can only surface serialized strings, never a
/// `CssValue`. So the single property-agnostic gate must classify the common
/// denominator (strings). That is robust here: CSS-wide keywords are globally
/// reserved exact tokens and `var(` is the only CSS function spelled that way,
/// so a standard serialized value never contains the literal substring `var(`
/// except an actual `var()`.
enum Kind {
    /// A concrete component value (`5px`, `hidden`, `red`).
    Physical,
    /// A CSS-wide keyword — css-cascade-4 §7.3 *Explicit Defaulting*
    /// (`initial`/`inherit`/`unset`/`revert`; `revert-layer` is css-cascade-5
    /// §7.3.5 *Rolling Back Cascade Layers*).
    CssWide,
    /// An unsubstituted `var()` — css-variables-1 §3 *the var() notation* / §2.2
    /// *Guaranteed-Invalid Values*: a var-carrying value is not substituted
    /// until computed-value time, so its specified-value serialization is not a
    /// concrete component.
    Var,
}

/// Classify a serialized component value string into its [`Kind`].
///
/// Substring-matches `var(` (the only such-spelled CSS function), else
/// exact-matches the CSS-wide keyword set with `eq_ignore_ascii_case` (no
/// per-component allocation — the common all-physical read falls through here).
fn value_kind(v: &str) -> Kind {
    const CSS_WIDE: [&str; 5] = ["initial", "inherit", "unset", "revert", "revert-layer"];
    if v.contains("var(") {
        Kind::Var // CssValue::Var or a RawTokens("…var(…")
    } else if CSS_WIDE.iter().any(|kw| v.eq_ignore_ascii_case(kw)) {
        Kind::CssWide // css-cascade-4 §7.3 (revert-layer: css-cascade-5 §7.3.5)
    } else {
        Kind::Physical
    }
}

/// CSSOM §6.7.2 step 1.2 ("if the shorthand cannot exactly represent the values
/// of all the properties in list, return the empty string") applied by
/// component value-kind:
/// - any unresolved var()        → Some("")   (not a concrete component pre-substitution)
/// - all the same CSS-wide kw     → Some(kw)   (the shorthand IS that keyword)
/// - mixed different CSS-wide kw   → Some("")   (cannot exactly represent)
/// - a CSS-wide kw mixed with a physical value → Some("")   (cannot exactly represent)
/// - all physical                 → None       (defer to the family collapse)
fn value_kind_gate(pairs: &[(&str, &str)]) -> Option<String> {
    let mut csswide = 0usize;
    for (_, v) in pairs {
        match value_kind(v) {
            Kind::Var => return Some(String::new()), // any var ⇒ "" regardless of siblings
            Kind::CssWide => csswide += 1,
            Kind::Physical => {}
        }
    }
    if csswide == 0 {
        return None; // all physical → the family collapse runs below
    }
    // ≥1 CSS-wide, no var: the shorthand IS the keyword only when every component
    // is that same keyword; a physical sibling or a differing keyword ⇒ "".
    let first = pairs[0].1;
    if csswide == pairs.len() && pairs.iter().all(|(_, v)| v.eq_ignore_ascii_case(first)) {
        return Some(first.to_ascii_lowercase()); // §6.7.2 keyword serialization
    }
    Some(String::new()) // mixed CSS-wide, or CSS-wide + physical → cannot represent
}

/// Reconstruct a CSS **shorthand** value from its longhand declarations —
/// CSSOM §6.6.1 `getPropertyValue`, the single canonical entry used by BOTH the
/// inline `el.style` path and the rule `cssRule.style` path.
///
/// The declaration block stores **longhands** (the parser expands shorthands,
/// defaulting omitted components to their initial value), so a shorthand getter
/// must rebuild the shorthand string. `get` maps a longhand name to its
/// `(serialized value, important flag)`.
///
/// This coordinator owns only the **property-agnostic** CSSOM §6.6.1 checks:
///
/// 1. `property` is a shorthand (else `None` — the caller reads it directly);
/// 2. **every** mapped longhand is present (a missing one makes the shorthand
///    non-serializable);
/// 3. the longhands' `!important` flags are **uniform** (a mixed block yields
///    `""`). Note this is *uniformity*, not "all important" —
///    `getPropertyPriority` checks all-important; the value getter checks
///    all-equal.
///
/// The per-family **collapse** (CSSOM §6.7.2 "serialize a CSS value") is then
/// dispatched to the property's own [`CssPropertyHandler::serialize_shorthand`],
/// which owns that family's grammar and — for omit-initial families — its own
/// `initial_value` (single source of truth; initials are never duplicated).
///
/// Returns `None` when the shorthand is not serializable or the owning handler
/// does not cover it; the caller maps `None` to the empty string (CSSOM-valid).
///
/// [`CssPropertyHandler::serialize_shorthand`]: elidex_plugin::CssPropertyHandler::serialize_shorthand
#[must_use]
pub fn serialize_shorthand_value(
    registry: &CssPropertyRegistry,
    property: &str,
    get: impl Fn(&str) -> Option<(String, bool)>,
) -> Option<String> {
    let longhands = elidex_css::shorthand_longhands(property);
    if longhands.is_empty() {
        return None; // not a shorthand — caller reads the property directly
    }
    // §6.6.1: every mapped longhand must be present.
    let decls: Vec<(String, bool)> = longhands
        .iter()
        .map(|lh| get(lh))
        .collect::<Option<Vec<_>>>()?;

    // §6.6.1: the important flags must be uniform (all-important OR all-normal).
    let first_important = decls[0].1;
    if !decls
        .iter()
        .all(|(_, important)| *important == first_important)
    {
        return None;
    }

    // Build the (longhand, value) component pairs read by BOTH the value-KIND
    // gate and the family collapse.
    let pairs: Vec<(&str, &str)> = longhands
        .iter()
        .map(String::as_str)
        .zip(decls.iter().map(|(value, _)| value.as_str()))
        .collect();

    // Dispatch to the owning handler FIRST — its return distinguishes "this
    // coordinator serializes `property`" (`Some`) from "it does not" (`None`).
    //
    // The registry is keyed by **longhand** name (`CssPropertyHandler::property_names`
    // is longhands-only — shorthand expansion is internal to `parse`), so a
    // shorthand name never resolves. Find the owning handler via the shorthand's
    // first longhand: every longhand of a shorthand is owned by the same handler
    // (`margin-*` → Box, `border-spacing-*` → Table, `flex-*` → Flex, …).
    //
    // A `None` here means the shorthand is **not covered** — no handler
    // serializes it (`background` / `flex` / `text-decoration`, or the
    // omit-initial families still deferred under `#11-style-shorthand-expand`).
    // The coordinator is then NOT authoritative: the caller falls back to a
    // *direct* shorthand declaration stored under the shorthand name — a
    // whole-shorthand `var()` (`background: var(--bg)`) — which, being the later
    // declaration, is the CSSOM §6.6.1 cascade winner (Blink 148:
    // `background: initial; background: var(--bg)` →
    // `getPropertyValue("background")` == `"var(--bg)"`, NOT the expanded-longhand
    // `"initial"`). Running the value-KIND gate *before* this dispatch would emit
    // `Some("initial")` / `Some("")` from those still-present longhands and preempt
    // that fallback — so the gate applies ONLY once coverage is established.
    let collapsed = registry
        .resolve(&longhands[0])?
        .serialize_shorthand(property, &pairs)?;

    // Covered. CSSOM §6.7.2 step 1.2 — value-KIND gate (property-agnostic).
    // Override the pure string-equality collapse when a component's KIND (an
    // unresolved `var()`, or a CSS-wide keyword — uniform, mixed, or with a
    // physical sibling) cannot be represented; `None` = all-physical → keep the
    // collapse. This stops the collapse helpers from emitting an invalid,
    // non-round-tripping shorthand (`"initial 5px"`, `"var(--x) 0px 0px"`).
    Some(value_kind_gate(&pairs).unwrap_or(collapsed))
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
