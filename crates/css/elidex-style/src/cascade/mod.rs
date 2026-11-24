//! CSS cascade algorithm.
//!
//! Implements the CSS cascade for Phase 1: collects matching declarations
//! from stylesheets and inline styles, then determines the winning value
//! for each property based on origin, importance, specificity, and
//! source order.

use elidex_css::media::{evaluate, MediaEnvironment};
use elidex_css::{Declaration, Origin, PseudoElement, Specificity, Stylesheet};
use elidex_ecs::{Attributes, EcsDom, Entity};
use elidex_plugin::CssValue;

/// True if a rule's `@media` condition chain (CSS Conditional §2) is satisfied
/// by `env` — every ancestor `MediaQueryList` must `evaluate` true. An empty
/// chain (unconditional rule, the common case) is always active.
fn media_conditions_match(rule: &elidex_css::CssRule, env: &MediaEnvironment) -> bool {
    rule.media_conditions.iter().all(|q| evaluate(q, env))
}

/// A single declaration entry in the cascade, annotated with priority metadata.
#[derive(Clone, Copy)]
pub(crate) struct CascadeEntry<'a> {
    pub(crate) property: &'a str,
    pub(crate) value: &'a CssValue,
    pub(crate) registry: Option<&'a elidex_plugin::CssPropertyRegistry>,
    block: usize,
    position: usize,
    important: bool,
    priority: CascadePriority,
}

/// CSS Cascade 5 §6.1. Ordered from inner to outer for normal declarations.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum TreeContext {
    HostShadow,
    Slot,
    Element,
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
    context: TreeContext,
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
                    // Important declarations favor inner contexts.
                    other.context.cmp(&self.context)
                } else {
                    // Normal declarations favor outer contexts.
                    self.context.cmp(&other.context)
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
/// a rule, reducing the argument count of [`push_block`].
struct CascadeSource {
    origin: Origin,
    context: TreeContext,
    is_inline: bool,
    specificity: Specificity,
    stylesheet_index: u32,
    source_order: u32,
}

/// Push a cascade entry for a declaration with the given source metadata.
fn push_block<'a>(
    entries: &mut Vec<CascadeEntry<'a>>,
    declarations: &'a [Declaration],
    registry: Option<&'a elidex_plugin::CssPropertyRegistry>,
    source: &CascadeSource,
) {
    let block = entries.len();
    for (position, decl) in declarations.iter().enumerate() {
        entries.push(CascadeEntry {
            block,
            position,
            registry,
            important: decl.important,
            property: &decl.property,
            value: &decl.value,
            priority: CascadePriority {
                importance_layer: importance_layer(source.origin, decl.important),
                context: source.context,
                is_inline: source.is_inline,
                specificity: source.specificity,
                stylesheet_index: source.stylesheet_index,
                source_order: source.source_order,
            },
        });
    }
}

pub(crate) fn push_page_block<'a>(
    entries: &mut Vec<CascadeEntry<'a>>,
    declarations: &'a [Declaration],
    sheet: &'a Stylesheet,
    sheet_index: u32,
    rule_order: u32,
    specificity: Specificity,
) {
    push_block(
        entries,
        declarations,
        sheet.grammar_registry.as_deref(),
        &CascadeSource {
            origin: sheet.origin,
            context: TreeContext::Element,
            is_inline: false,
            specificity,
            stylesheet_index: sheet_index,
            source_order: rule_order,
        },
    );
}

impl CascadeEntry<'_> {
    pub(crate) fn wins(self, other: Self) -> bool {
        if self.block == other.block {
            elidex_plugin::declaration_wins(
                self.important,
                self.position,
                other.important,
                other.position,
            )
        } else {
            self.priority >= other.priority
        }
    }
}

/// Shadow cascade context for `:host` and `::slotted()` rule participation.
///
/// Determines how shadow-internal stylesheets participate in the cascade
/// for a given element (CSS Scoping Module §3).
#[derive(Default)]
pub(crate) struct ShadowCascade<'a> {
    pub(crate) slot: Option<&'a Stylesheet>,
    pub(crate) host: Option<&'a Stylesheet>,
}

/// Collect matching declarations with their cascade priority and block position.
///
/// Keep every candidate until pending shorthands expand in the producer’s grammar.
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
    env: &MediaEnvironment,
) -> Vec<CascadeEntry<'a>> {
    let mut entries: Vec<CascadeEntry<'a>> = Vec::new();

    // Collect presentational hints first — author origin, lowest specificity and
    // source_order=0 so any author stylesheet rule overrides them.
    let hint_source = CascadeSource {
        origin: Origin::Author,
        context: TreeContext::Element,
        is_inline: false,
        specificity: Specificity::default(),
        stylesheet_index: 0,
        source_order: 0,
    };
    push_block(&mut entries, extra_declarations, None, &hint_source);

    // Collect from stylesheets (only selectors without pseudo-elements).
    collect_matching_rules(
        &mut entries,
        entity,
        dom,
        stylesheets,
        None,
        TreeContext::Element,
        env,
    );

    if let Some(sheet) = shadow_cascade.slot {
        collect_shadow_rules(
            &mut entries,
            entity,
            dom,
            sheet,
            TreeContext::Slot,
            elidex_css::Selector::has_slotted,
            env,
        );
    }
    if let Some(sheet) = shadow_cascade.host {
        collect_shadow_rules(
            &mut entries,
            entity,
            dom,
            sheet,
            TreeContext::HostShadow,
            elidex_css::Selector::has_host,
            env,
        );
    }

    // Collect inline styles (highest specificity, treated as author origin).
    // Inline styles use a synthetic source_order of u32::MAX to ensure they
    // come after any stylesheet declarations at the same priority.
    let inline_source = CascadeSource {
        origin: Origin::Author,
        context: TreeContext::Element,
        is_inline: true,
        specificity: Specificity::default(),
        stylesheet_index: u32::MAX,
        source_order: u32::MAX,
    };
    push_block(
        &mut entries,
        inline_declarations,
        Some(crate::default_css_property_registry()),
        &inline_source,
    );

    entries
}

/// Collect matching rules from stylesheets, filtered by pseudo-element.
///
/// When `pseudo` is `None`, only selectors without pseudo-elements are matched.
/// When `pseudo` is `Some(pe)`, only selectors with that specific pseudo-element
/// are matched (against the originating element).
fn collect_matching_rules<'a>(
    entries: &mut Vec<CascadeEntry<'a>>,
    entity: Entity,
    dom: &EcsDom,
    stylesheets: &'a [&'a Stylesheet],
    pseudo: Option<&PseudoElement>,
    context: TreeContext,
    env: &MediaEnvironment,
) {
    for (sheet_idx, stylesheet) in stylesheets.iter().enumerate() {
        #[allow(clippy::cast_possible_truncation)] // Stylesheet count won't exceed u32::MAX.
        let sheet_index = sheet_idx as u32;
        for rule in &stylesheet.rules {
            // CSS Conditional §2: skip a rule whose `@media` condition chain is
            // not satisfied by the current environment (unconditional rules pass).
            if !media_conditions_match(rule, env) {
                continue;
            }
            // Single-pass: find max specificity among matching selectors
            // filtered by pseudo-element.
            // B2: Skip :host/:host()/ ::slotted() selectors in outer context —
            // they must only participate via ShadowCascade.
            let max_specificity = rule
                .selectors
                .iter()
                .filter(|sel| {
                    sel.pseudo_element.as_ref() == pseudo
                        && !(context == TreeContext::Element
                            && (sel.has_host() || sel.has_slotted()))
                        && sel.matches(entity, dom)
                })
                .map(|sel| sel.specificity)
                .max();
            let Some(max_specificity) = max_specificity else {
                continue;
            };

            let source = CascadeSource {
                origin: stylesheet.origin,
                context,
                is_inline: false,
                specificity: max_specificity,
                stylesheet_index: sheet_index,
                source_order: rule.source_order,
            };
            push_block(
                entries,
                &rule.declarations,
                stylesheet.grammar_registry.as_deref(),
                &source,
            );
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
    env: &MediaEnvironment,
) -> Vec<CascadeEntry<'a>> {
    let mut entries: Vec<CascadeEntry<'a>> = Vec::new();

    collect_matching_rules(
        &mut entries,
        entity,
        dom,
        stylesheets,
        Some(&pseudo),
        TreeContext::Element,
        env,
    );

    entries
}

/// Collect shadow-internal rules matching a selector filter.
fn collect_shadow_rules<'a>(
    entries: &mut Vec<CascadeEntry<'a>>,
    entity: Entity,
    dom: &EcsDom,
    shadow_sheet: &'a Stylesheet,
    context: TreeContext,
    selector_filter: fn(&elidex_css::Selector) -> bool,
    env: &MediaEnvironment,
) {
    for rule in &shadow_sheet.rules {
        // CSS Conditional §2: `@media`-gated shadow rules obey the same gate.
        if !media_conditions_match(rule, env) {
            continue;
        }
        let max_specificity = rule
            .selectors
            .iter()
            .filter(|sel| {
                sel.pseudo_element.is_none() && selector_filter(sel) && sel.matches(entity, dom)
            })
            .map(|sel| sel.specificity)
            .max();
        let Some(max_specificity) = max_specificity else {
            continue;
        };

        let source = CascadeSource {
            origin: shadow_sheet.origin,
            context,
            is_inline: false,
            specificity: max_specificity,
            stylesheet_index: 0,
            source_order: rule.source_order,
        };
        push_block(
            entries,
            &rule.declarations,
            shadow_sheet.grammar_registry.as_deref(),
            &source,
        );
    }
}

/// Retrieve inline style declarations from an element's `style` attribute.
///
/// Registry-aware: registry-backed properties (`transform`, `transition`,
/// …) parse through the default registry so an inline
/// `style="transform: rotate(45deg)"` reaches the cascade — matching the
/// CSSOM `InlineStyle` materialization (`elidex_dom_api`), which uses the
/// same default registry. Without this the cascade and `InlineStyle`
/// would diverge on registry-backed inline declarations.
pub(crate) fn get_inline_declarations(entity: Entity, dom: &EcsDom) -> Vec<Declaration> {
    let Ok(attrs) = dom.world().get::<&Attributes>(entity) else {
        return Vec::new();
    };
    let Some(style_str) = attrs.get("style") else {
        return Vec::new();
    };
    elidex_css::parse_declaration_block_with_registry(
        style_str,
        Some(crate::default_css_property_registry()),
    )
}

#[cfg(test)]
mod tests;
