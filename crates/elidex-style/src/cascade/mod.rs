//! CSS cascade algorithm.
//!
//! Implements the CSS cascade for Phase 1: collects matching declarations
//! from stylesheets and inline styles, then determines the winning value
//! for each property based on origin, importance, specificity, and
//! source order.

use std::collections::HashMap;

use elidex_css::{Declaration, Origin, PseudoElement, Specificity, Stylesheet};
use elidex_ecs::{Attributes, EcsDom, Entity};
use elidex_plugin::CssValue;

/// A single declaration entry in the cascade, annotated with priority metadata.
struct CascadeEntry<'a> {
    property: &'a str,
    value: &'a CssValue,
    priority: CascadePriority,
}

/// Cascade priority for comparing declarations.
///
/// Comparison priority: `importance_layer` > `encapsulation_context` > `is_inline`
/// > `specificity` > `stylesheet_index` > `source_order`.
///
/// Per CSS Cascading L4 §6.1 + CSS Scoping §3.1:
/// - Normal declarations: outer context wins over inner (`:host`/`::slotted()`)
/// - `!important` declarations: inner context wins over outer (reversed)
///
/// Manual `Ord` implementation handles the `!important` reversal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CascadePriority {
    /// 0 = UA normal, 1 = Author normal, 2 = Author !important, 3 = UA !important.
    importance_layer: u8,
    /// `true` for outer (light DOM) context rules, `false` for shadow-internal
    /// (`:host` / `::slotted()`) rules.
    is_outer_context: bool,
    /// Inline styles beat selector-based styles at the same layer.
    is_inline: bool,
    /// Selector specificity.
    specificity: Specificity,
    /// Index of the stylesheet in the list (later stylesheet = higher index = wins).
    stylesheet_index: u32,
    /// Position in source order within a stylesheet (higher = later = wins ties).
    source_order: u32,
}

impl Ord for CascadePriority {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.importance_layer
            .cmp(&other.importance_layer)
            .then_with(|| {
                // CSS Cascading L4 §6.1: For !important (importance_layer >= 2),
                // inner context wins over outer (reversed from normal).
                if self.importance_layer >= 2 {
                    // Reversed: false (inner) > true (outer).
                    other.is_outer_context.cmp(&self.is_outer_context)
                } else {
                    // Normal: true (outer) > false (inner).
                    self.is_outer_context.cmp(&other.is_outer_context)
                }
            })
            .then_with(|| self.is_inline.cmp(&other.is_inline))
            .then_with(|| self.specificity.cmp(&other.specificity))
            .then_with(|| self.stylesheet_index.cmp(&other.stylesheet_index))
            .then_with(|| self.source_order.cmp(&other.source_order))
    }
}

impl PartialOrd for CascadePriority {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

fn importance_layer(origin: Origin, important: bool) -> u8 {
    match (origin, important) {
        (Origin::UserAgent, false) => 0,
        (Origin::UserAgent, true) => 3,
        // Author + any future origins default to author-level.
        (_, false) => 1,
        (_, true) => 2,
    }
}

/// Source location metadata for cascade ordering.
///
/// Groups the cascade source fields that are invariant per-declaration within
/// a rule, reducing the argument count of [`push_cascade_entry`].
struct CascadeSource {
    origin: Origin,
    is_outer_context: bool,
    is_inline: bool,
    specificity: Specificity,
    stylesheet_index: u32,
    source_order: u32,
}

/// Push a cascade entry for a declaration with the given source metadata.
fn push_cascade_entry<'a>(
    entries: &mut Vec<CascadeEntry<'a>>,
    decl: &'a Declaration,
    source: &CascadeSource,
) {
    entries.push(CascadeEntry {
        property: &decl.property,
        value: &decl.value,
        priority: CascadePriority {
            importance_layer: importance_layer(source.origin, decl.important),
            is_outer_context: source.is_outer_context,
            is_inline: source.is_inline,
            specificity: source.specificity,
            stylesheet_index: source.stylesheet_index,
            source_order: source.source_order,
        },
    });
}

/// Sort cascade entries by priority and return the last-wins per-property map.
fn compute_winners<'a>(entries: &mut [CascadeEntry<'a>]) -> HashMap<&'a str, &'a CssValue> {
    entries.sort_by(|a, b| a.priority.cmp(&b.priority));
    let mut winners: HashMap<&str, &CssValue> = HashMap::with_capacity(entries.len());
    for entry in entries.iter() {
        winners.insert(entry.property, entry.value);
    }
    winners
}

/// Shadow cascade context for `:host` and `::slotted()` rule participation.
///
/// Determines how shadow-internal stylesheets participate in the cascade
/// for a given element (CSS Scoping Module §3).
pub(crate) enum ShadowCascade<'a> {
    /// Normal cascade — no shadow rules participate (outer context).
    Outer,
    /// Shadow host cascade — `:host` rules from the shadow stylesheet participate
    /// at `is_outer_context = false` (lower priority than outer rules).
    Host(&'a Stylesheet),
    /// Slotted node cascade — `::slotted()` rules from the shadow stylesheet
    /// participate at `is_outer_context = false`.
    Slotted(&'a Stylesheet),
}

/// Collect matching declarations and cascade to determine winners.
///
/// Returns a map from property name to the winning `CssValue` reference.
///
/// `extra_declarations` are presentational hints (e.g. HTML attributes mapped to CSS).
/// They participate at author-origin, specificity (0,0,0), ordered before all author
/// rules so that any selector-based rule or inline style overrides them.
///
/// `shadow_cascade` controls how shadow-internal rules participate for the entity
/// being styled (CSS Scoping Module §3).
pub(crate) fn collect_and_cascade<'a>(
    entity: Entity,
    dom: &EcsDom,
    stylesheets: &'a [&'a Stylesheet],
    inline_declarations: &'a [Declaration],
    extra_declarations: &'a [Declaration],
    shadow_cascade: &ShadowCascade<'a>,
) -> HashMap<&'a str, &'a CssValue> {
    let mut entries: Vec<CascadeEntry<'a>> = Vec::new();

    // Collect presentational hints first — author origin, lowest specificity and
    // source_order=0 so any author stylesheet rule overrides them.
    let hint_source = CascadeSource {
        origin: Origin::Author,
        is_outer_context: true,
        is_inline: false,
        specificity: Specificity::default(),
        stylesheet_index: 0,
        source_order: 0,
    };
    for decl in extra_declarations {
        push_cascade_entry(&mut entries, decl, &hint_source);
    }

    // Collect from stylesheets (only selectors without pseudo-elements).
    collect_matching_rules(&mut entries, entity, dom, stylesheets, None, true);

    // Collect shadow-internal rules at lower priority (is_outer_context = false).
    match shadow_cascade {
        ShadowCascade::Host(shadow_sheet) => {
            collect_shadow_rules(
                &mut entries,
                entity,
                dom,
                shadow_sheet,
                elidex_css::Selector::has_host,
                None,
            );
        }
        ShadowCascade::Slotted(shadow_sheet) => {
            collect_shadow_rules(
                &mut entries,
                entity,
                dom,
                shadow_sheet,
                elidex_css::Selector::has_slotted,
                None,
            );
        }
        ShadowCascade::Outer => {}
    }

    // Collect inline styles (highest specificity, treated as author origin).
    // Inline styles use a synthetic source_order of u32::MAX to ensure they
    // come after any stylesheet declarations at the same priority.
    let inline_source = CascadeSource {
        origin: Origin::Author,
        is_outer_context: true,
        is_inline: true,
        specificity: Specificity::default(),
        stylesheet_index: u32::MAX,
        source_order: u32::MAX,
    };
    for decl in inline_declarations {
        push_cascade_entry(&mut entries, decl, &inline_source);
    }

    compute_winners(&mut entries)
}

/// Collect matching rules from stylesheets, filtered by pseudo-element.
///
/// When `pseudo` is `None`, only selectors without pseudo-elements are matched.
/// When `pseudo` is `Some(pe)`, only selectors with that specific pseudo-element
/// are matched (against the originating element).
///
/// `is_outer_context` marks whether these rules come from the outer (light DOM)
/// context (`true`) or from shadow-internal context (`false`).
fn collect_matching_rules<'a>(
    entries: &mut Vec<CascadeEntry<'a>>,
    entity: Entity,
    dom: &EcsDom,
    stylesheets: &'a [&'a Stylesheet],
    pseudo: Option<&PseudoElement>,
    is_outer_context: bool,
) {
    for (sheet_idx, stylesheet) in stylesheets.iter().enumerate() {
        #[allow(clippy::cast_possible_truncation)] // Stylesheet count won't exceed u32::MAX.
        let sheet_index = sheet_idx as u32;
        for rule in &stylesheet.rules {
            // Single-pass: find max specificity among matching selectors
            // filtered by pseudo-element.
            // B2: Skip :host/:host()/ ::slotted() selectors in outer context —
            // they must only participate via ShadowCascade.
            let max_specificity = rule
                .selectors
                .iter()
                .filter(|sel| {
                    sel.pseudo_element.as_ref() == pseudo
                        && !(is_outer_context && (sel.has_host() || sel.has_slotted()))
                        && sel.matches(entity, dom)
                })
                .map(|sel| sel.specificity)
                .max();
            let Some(max_specificity) = max_specificity else {
                continue;
            };

            let source = CascadeSource {
                origin: stylesheet.origin,
                is_outer_context,
                is_inline: false,
                specificity: max_specificity,
                stylesheet_index: sheet_index,
                source_order: rule.source_order,
            };
            for decl in &rule.declarations {
                push_cascade_entry(entries, decl, &source);
            }
        }
    }
}

/// Collect and cascade declarations for a pseudo-element.
///
/// Similar to [`collect_and_cascade`] but only matches selectors that target
/// the given pseudo-element. Inline styles are not included (pseudo-elements
/// cannot have inline styles).
pub(crate) fn collect_and_cascade_pseudo<'a>(
    entity: Entity,
    dom: &EcsDom,
    stylesheets: &'a [&'a Stylesheet],
    pseudo: PseudoElement,
) -> HashMap<&'a str, &'a CssValue> {
    let mut entries: Vec<CascadeEntry<'a>> = Vec::new();

    collect_matching_rules(&mut entries, entity, dom, stylesheets, Some(&pseudo), true);

    compute_winners(&mut entries)
}

/// Collect shadow-internal rules matching a selector filter.
///
/// Used for both `:host` and `::slotted()` rule collection. Only rules
/// where at least one selector passes `selector_filter` and matches the
/// entity are collected, at `is_outer_context = false` so outer rules
/// always win (CSS Scoping §3.1).
///
/// M4: `pseudo` controls pseudo-element filtering. Pass `None` for
/// non-pseudo elements, or `Some(&pe)` for pseudo-element cascade
/// (e.g. `::slotted(div)::before`).
fn collect_shadow_rules<'a>(
    entries: &mut Vec<CascadeEntry<'a>>,
    entity: Entity,
    dom: &EcsDom,
    shadow_sheet: &'a Stylesheet,
    selector_filter: fn(&elidex_css::Selector) -> bool,
    pseudo: Option<&PseudoElement>,
) {
    for rule in &shadow_sheet.rules {
        let max_specificity = rule
            .selectors
            .iter()
            .filter(|sel| {
                sel.pseudo_element.as_ref() == pseudo
                    && selector_filter(sel)
                    && sel.matches(entity, dom)
            })
            .map(|sel| sel.specificity)
            .max();
        let Some(max_specificity) = max_specificity else {
            continue;
        };

        let source = CascadeSource {
            origin: shadow_sheet.origin,
            is_outer_context: false,
            is_inline: false,
            specificity: max_specificity,
            stylesheet_index: 0,
            source_order: rule.source_order,
        };
        for decl in &rule.declarations {
            push_cascade_entry(entries, decl, &source);
        }
    }
}

/// Retrieve inline style declarations from an element's `style` attribute.
pub(crate) fn get_inline_declarations(entity: Entity, dom: &EcsDom) -> Vec<Declaration> {
    let Ok(attrs) = dom.world().get::<&Attributes>(entity) else {
        return Vec::new();
    };
    let Some(style_str) = attrs.get("style") else {
        return Vec::new();
    };
    elidex_css::parse_declaration_block(style_str)
}

#[cfg(test)]
mod tests;
