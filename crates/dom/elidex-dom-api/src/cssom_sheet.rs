//! `CSSStyleSheet` / `CSSStyleRule` / `CSSRuleList` / `StyleSheetList` DOM API
//! handlers (CSSOM §6.2 / §6.4 / §6.6 / §6.8).
//!
//! ## Storage model
//!
//! Per stylesheet-owning element the parsed `Stylesheet` lives in
//! `SessionCore::cssom_sheets`, keyed by the owner `Entity`.  The cache is
//! populated lazily on first CSSOM access; on every entry the handler
//! checks the owner's source version against the cached snapshot version,
//! and divergence triggers a re-walk + re-parse (reassigning rule_ids,
//! leaving any held `CSSStyleRule` / `CSSStyleDeclaration` Rule wrapper
//! stale by design).  The source + version differ by owner kind
//! (`sync_and_get_state` source branch / `sheet_version`):
//!
//! - `<style>` — source is the element's text content; version is the
//!   subtree `EcsDom::inclusive_descendants_version` (a direct
//!   `<style>.textContent` write diverges it).
//! - `<link rel="stylesheet">` — source is the `LinkStylesheet` component
//!   (HTML §4.6.7 associated CSS style sheet, fetched by the loader);
//!   version is the component's own monotonic counter, since the void
//!   `<link>` has no child-text mutation signal.
//!
//! Using a version counter rather than a string snapshot keeps the
//! divergence check O(1) per CSSOM access.
//!
//! ## Mutator round-trip (CRIT-3 Option II)
//!
//! `insertRule` / `deleteRule` mutate the cache, then re-serialise the
//! `Stylesheet` via [`elidex_css::serialize_stylesheet`] and write the
//! result back to the owner's source through `flush_sheet_mutation`:
//! for `<style>` the text is replaced via the `apply_replace_all` primitive
//! with its `MutationRecord`s **discarded** (so the cascade picks up the change
//! and `EcsDom::rev_version` fires for `LiveCollection` invalidation, but a
//! `MutationObserver` does NOT observe the engine-internal serialization as a
//! childList mutation — CSSOM edits are invisible to `MutationObserver`); for `<link>`
//! into the `LinkStylesheet` component (the void element has no text node).
//!
//! ## CSSMediaRule deferral
//!
//! `@media` blocks are now **retained** by [`elidex_css::parse_stylesheet`] —
//! their inner rules are flattened into `Stylesheet::rules`, each tagged with a
//! non-empty `media_conditions` chain, so the cascade can gate them (CSS
//! Conditional §2). The CSSOM grouping tree (`CSSMediaRule` + its nested
//! `CSSRuleList`) is still **deferred** (`#11-css-media-rule`): until it lands,
//! `cssRules` exposes only **unconditional** `CSSStyleRule`s and filters out the
//! flattened `@media` rules (`cssom_visible_count` / `cssom_actual_index`).
//! This keeps `cssRules` exactly as it was before `@media` retention (no
//! web-observable regression — the flattened rules must NOT leak in as bogus
//! top-level rules without their `@media` wrapper). The serialize round-trip
//! ([`elidex_css::serialize_stylesheet`]) still **preserves** the `@media`
//! wrappers, so an `insertRule`/`deleteRule` mutation does not drop them.

use elidex_css::{parse_single_rule, parse_stylesheet, serialize_stylesheet, Origin};
use elidex_ecs::{EcsDom, Entity, LinkStylesheet};
use elidex_plugin::JsValue;
use elidex_script_session::{
    apply_replace_all, CssomSheetState, DomApiError, DomApiErrorKind, DomApiHandler, SessionCore,
};

use crate::element::collect_text_content;
use crate::util::require_string_arg;

// ---------------------------------------------------------------------------
// Cache plumbing
// ---------------------------------------------------------------------------

/// Divergence-version of a stylesheet owner's source. `<link>` (with a
/// `LinkStylesheet` component) uses the component's monotonic counter;
/// `<style>` uses its subtree `EcsDom::inclusive_descendants_version`.
/// Cheap (no string materialisation), so it runs on every CSSOM access.
fn sheet_version(sheet_entity: Entity, dom: &EcsDom) -> u64 {
    dom.world()
        .get::<&LinkStylesheet>(sheet_entity)
        .map_or_else(
            |_| dom.inclusive_descendants_version(sheet_entity),
            |link| link.version,
        )
}

/// Sync the cached `Stylesheet` against the current owner source and
/// return a mutable reference. Re-parses (and reassigns rule_ids) when
/// [`sheet_version`] indicates the source has changed since the last
/// cache fill — far cheaper than stringifying + comparing the source on
/// every CSSOM access (PR-B review F1).
fn sync_and_get_state<'a>(
    sheet_entity: Entity,
    session: &'a mut SessionCore,
    dom: &EcsDom,
) -> &'a mut CssomSheetState {
    let version = sheet_version(sheet_entity, dom);
    let needs_reparse = session
        .cssom_sheets
        .get(&sheet_entity)
        .is_none_or(|s| s.snapshot_version != version);
    if needs_reparse {
        // Parse from the owner source. `<link>` parses its `LinkStylesheet`
        // component text in place (no clone — the component can be a large
        // external sheet); `<style>` collects its child text content
        // (HTML §4.6.7 associated style sheet vs `<style>` text node).
        let parsed = match dom.world().get::<&LinkStylesheet>(sheet_entity) {
            Ok(link) => parse_stylesheet(&link.source, Origin::Author),
            Err(_) => parse_stylesheet(&collect_text_content(sheet_entity, dom), Origin::Author),
        };
        session.cssom_sheets.insert(
            sheet_entity,
            CssomSheetState {
                parsed,
                snapshot_version: version,
            },
        );
    }
    session
        .cssom_sheets
        .get_mut(&sheet_entity)
        .expect("cssom_sheets entry just inserted")
}

/// Re-serialise the cached parsed stylesheet and write it back to the
/// owner's source, then synchronise [`CssomSheetState::snapshot_version`]
/// so the next `sync_and_get_state` skips a redundant re-parse.
///
/// - `<style>`: the serialised CSS replaces the element's text via
///   `apply_replace_all` (the §4.2.3 replace-all primitive), bumping
///   `EcsDom::rev_version` for cascade / `LiveCollection` invalidation — but its
///   returned `MutationRecord`s are **discarded**, NOT pushed to the session.
///   This re-serialization is engine-internal CSSOM↔source sync, not a
///   script-observable DOM mutation: a `MutationObserver` on the `<style>`'s
///   children must NOT see a childList record for an `insertRule`/`deleteRule`
///   (CSSOM edits are invisible to `MutationObserver`). This is the deliberate
///   counterpart to the public `Node.textContent` setter, which routes through the
///   same primitive but *delivers* the records.
/// - `<link rel="stylesheet">`: the void element has no text node, so the
///   serialised CSS is written into the `LinkStylesheet` component and its
///   monotonic `version` is bumped (CSSOM §6.4 / §6.5 round-trip).
fn flush_sheet_mutation(sheet_entity: Entity, session: &mut SessionCore, dom: &mut EcsDom) {
    let serialized = {
        let state = session
            .cssom_sheets
            .get(&sheet_entity)
            .expect("flush_sheet_mutation called before sync_and_get_state");
        serialize_stylesheet(&state.parsed)
    };
    // Probe link-ness immutably first: the `<style>` arm re-borrows `dom`
    // mutably through `apply_replace_all`, so an `if let Ok(mut link)`
    // scrutinee borrow would extend into the else arm and conflict.
    let is_link = dom.world().get::<&LinkStylesheet>(sheet_entity).is_ok();
    if is_link {
        let new_version = {
            let mut link = dom
                .world_mut()
                .get::<&mut LinkStylesheet>(sheet_entity)
                .expect("LinkStylesheet present (checked above)");
            link.source = serialized;
            link.version = link.version.saturating_add(1);
            link.version
        };
        if let Some(state) = session.cssom_sheets.get_mut(&sheet_entity) {
            state.snapshot_version = new_version;
        }
    } else {
        // Engine-internal CSSOM↔source sync: replace the <style>'s text (bumping
        // rev_version for cascade / LiveCollection) but DISCARD the records so a
        // MutationObserver on the <style>'s children does not observe an
        // insertRule/deleteRule as a childList mutation (CSSOM edits are invisible
        // to MutationObserver). `apply_replace_all` returns the records; not
        // pushing them is the record-free path (the public textContent setter
        // pushes the same records — that is the only difference).
        let node = if serialized.is_empty() {
            None
        } else {
            let owner = dom.owner_document(sheet_entity);
            Some(dom.create_text_with_owner(serialized, owner))
        };
        let _ = apply_replace_all(dom, sheet_entity, node);
        if let Some(state) = session.cssom_sheets.get_mut(&sheet_entity) {
            state.snapshot_version = dom.inclusive_descendants_version(sheet_entity);
        }
    }
}

/// Run `project` against the rule with `rule_id` (extracted from
/// `args[0]`) under the synced cache; return `default` if the rule_id
/// is absent or `args[0]` is non-numeric. Centralises the
/// "stale rule_id ⇒ spec default" invariant for every rule-level
/// read accessor.
fn with_rule<R>(
    sheet: Entity,
    args: &[JsValue],
    session: &mut SessionCore,
    dom: &EcsDom,
    default: R,
    project: impl FnOnce(&elidex_css::CssRule) -> R,
) -> R {
    let Some(rule_id) = rule_id_arg(args) else {
        return default;
    };
    let state = sync_and_get_state(sheet, session, dom);
    state
        .parsed
        .rules
        .iter()
        .find(|r| r.rule_id == rule_id)
        .map_or(default, project)
}

fn syntax_error(message: impl Into<String>) -> DomApiError {
    DomApiError {
        kind: DomApiErrorKind::SyntaxError,
        message: message.into(),
    }
}

fn index_size_error(message: impl Into<String>) -> DomApiError {
    DomApiError {
        kind: DomApiErrorKind::IndexSizeError,
        message: message.into(),
    }
}

// ---------------------------------------------------------------------------
// CSSOM-visible rule view (excludes flattened `@media` rules)
// ---------------------------------------------------------------------------
//
// `Stylesheet::rules` now contains both top-level style rules and the rules
// flattened out of `@media` blocks (the latter carry a non-empty
// `media_conditions` chain). The CSSOM `cssRules` list must expose only the
// former until `CSSMediaRule` lands (`#11-css-media-rule`) — see the module
// doc. These helpers map between the CSSOM index space (visible rules only)
// and the actual `Stylesheet::rules` index space.

/// Number of CSSOM-visible (unconditional) rules.
fn cssom_visible_count(rules: &[elidex_css::CssRule]) -> usize {
    rules
        .iter()
        .filter(|r| r.media_conditions.is_empty())
        .count()
}

/// The actual `Stylesheet::rules` index of the `cssom_index`-th visible rule.
/// `None` when `cssom_index` is past the last visible rule (callers handle the
/// `insertRule` append case — `cssom_index == cssom_visible_count` — separately).
fn cssom_actual_index(rules: &[elidex_css::CssRule], cssom_index: usize) -> Option<usize> {
    rules
        .iter()
        .enumerate()
        .filter(|(_, r)| r.media_conditions.is_empty())
        .nth(cssom_index)
        .map(|(actual, _)| actual)
}

/// The actual `Stylesheet::rules` insert position for CSSOM `insertRule(_, index)`:
/// before the `index`-th visible rule, or at the end of `rules` when `index`
/// equals the visible count (append). Caller validates `index <= visible count`.
fn cssom_insert_position(rules: &[elidex_css::CssRule], cssom_index: usize) -> usize {
    cssom_actual_index(rules, cssom_index).unwrap_or(rules.len())
}

// ---------------------------------------------------------------------------
// Sheet-level handlers
// ---------------------------------------------------------------------------

/// `CSSStyleSheet.cssRules.length` — number of rules in the sheet.
pub struct CssRulesLength;

impl DomApiHandler for CssRulesLength {
    fn method_name(&self) -> &str {
        "cssRules.length"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let state = sync_and_get_state(this, session, dom);
        #[allow(clippy::cast_precision_loss)]
        Ok(JsValue::Number(
            cssom_visible_count(&state.parsed.rules) as f64
        ))
    }
}

/// `CSSStyleSheet.cssRules.item(index)` — returns the stable `rule_id` of the
/// rule at `index`, or `-1` when out-of-range.  The host wraps this id into a
/// `CSSStyleRule` JS object; encoding the id as a number lets the dom-api
/// layer stay JS-engine-agnostic (the alternative would require returning an
/// engine-specific wrapper handle).
pub struct CssRulesItemId;

impl DomApiHandler for CssRulesItemId {
    fn method_name(&self) -> &str {
        "cssRules.itemId"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let JsValue::Number(idx_f) = args.first().cloned().unwrap_or(JsValue::Undefined) else {
            return Ok(JsValue::Number(-1.0));
        };
        if !idx_f.is_finite() || idx_f < 0.0 {
            return Ok(JsValue::Number(-1.0));
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let idx = idx_f as usize;
        let state = sync_and_get_state(this, session, dom);
        // CSSOM-visible (unconditional) rules only — flattened `@media` rules
        // are not surfaced (`#11-css-media-rule`).
        let id = cssom_actual_index(&state.parsed.rules, idx)
            .and_then(|actual| state.parsed.rules.get(actual))
            .map(|r| r.rule_id)
            .map_or(-1.0, |id| {
                #[allow(clippy::cast_precision_loss)]
                {
                    id as f64
                }
            });
        Ok(JsValue::Number(id))
    }
}

/// `CSSStyleSheet.insertRule(rule, index)` (CSSOM §6.4) — parse the rule
/// text, assign the next stable `rule_id`, splice into the rule list, then
/// re-serialise.  Returns the new rule's index.  Per spec:
/// - omitted `index` defaults to `0`
/// - `index > rules.length` throws `IndexSizeError`
/// - unparseable / multi-rule input throws `SyntaxError`
pub struct InsertRule;

impl DomApiHandler for InsertRule {
    fn method_name(&self) -> &str {
        "stylesheet.insertRule"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let rule_text = require_string_arg(args, 0)?;
        // Host-side `native_sheet_insert_rule` applies the WebIDL
        // `optional unsigned long index = 0` coercion: `args[1]` is
        // guaranteed to be a non-negative `JsValue::Number` (or 0.0
        // for the absent case). Anything else is a binding-layer bug.
        let index_f = match args.get(1).cloned() {
            Some(JsValue::Number(n)) if n.is_finite() && n >= 0.0 => n,
            _ => 0.0,
        };
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let index = index_f as usize;

        let state = sync_and_get_state(this, session, dom);
        // Bounds + insert position are in the CSSOM-visible index space
        // (excludes flattened `@media` rules — `#11-css-media-rule`).
        let visible = cssom_visible_count(&state.parsed.rules);
        if index > visible {
            return Err(index_size_error(format!(
                "Failed to execute 'insertRule' on 'CSSStyleSheet': index {index} is larger than rule count {visible}"
            )));
        }
        let mut new_rule = parse_single_rule(&rule_text).ok_or_else(|| {
            syntax_error(
                "Failed to execute 'insertRule' on 'CSSStyleSheet': the rule could not be parsed",
            )
        })?;
        new_rule.rule_id = state.parsed.next_rule_id;
        state.parsed.next_rule_id = state.parsed.next_rule_id.saturating_add(1);
        let actual = cssom_insert_position(&state.parsed.rules, index);
        state.parsed.rules.insert(actual, new_rule);
        // No need to renumber `source_order` here: `flush_sheet_mutation`
        // re-serialises the sheet and writes back to the owner's source
        // (`<style>.textContent` or the `<link>` `LinkStylesheet` component),
        // so the next `sync_and_get_state` re-parses and assigns fresh
        // sequential `source_order` values from scratch.  Any in-memory
        // mutation here would be overwritten on the next walk.
        flush_sheet_mutation(this, session, dom);
        #[allow(clippy::cast_precision_loss)]
        Ok(JsValue::Number(index_f))
    }
}

/// `CSSStyleSheet.deleteRule(index)` (CSSOM §6.5).  Per spec:
/// - `index >= rules.length` throws `IndexSizeError`
pub struct DeleteRule;

impl DomApiHandler for DeleteRule {
    fn method_name(&self) -> &str {
        "stylesheet.deleteRule"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        // Host-side `native_sheet_delete_rule` already applied the
        // WebIDL `unsigned long` → ToUint32 coercion (and threw TypeError
        // on missing arg), so we only need to handle the `JsValue::Number`
        // payload. A non-Number reaching here is a binding-layer bug,
        // not a script-level error — surface as `Other`.
        let JsValue::Number(idx_f) = args.first().cloned().unwrap_or(JsValue::Undefined) else {
            return Err(DomApiError {
                kind: DomApiErrorKind::Other,
                message: "stylesheet.deleteRule: numeric index expected from binding layer".into(),
            });
        };
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let index = idx_f as usize;

        let state = sync_and_get_state(this, session, dom);
        // Bounds + removal target are in the CSSOM-visible index space
        // (excludes flattened `@media` rules — `#11-css-media-rule`).
        let Some(actual) = cssom_actual_index(&state.parsed.rules, index) else {
            return Err(index_size_error(format!(
                "Failed to execute 'deleteRule' on 'CSSStyleSheet': index {index} is out of range",
            )));
        };
        state.parsed.rules.remove(actual);
        // No `source_order` renumbering here — the next `sync_and_get_state`
        // re-parses from the post-flush source and assigns fresh sequential
        // values (mirrors `InsertRule` above).
        flush_sheet_mutation(this, session, dom);
        Ok(JsValue::Undefined)
    }
}

// ---------------------------------------------------------------------------
// Rule-level handlers — keyed by `(sheet entity, rule_id)` where the rule_id
// arrives as `args[0]` (Number).  Returns spec defaults (empty string / null)
// when the rule_id is no longer present so stale wrapper reads do not throw.
// ---------------------------------------------------------------------------

fn rule_id_arg(args: &[JsValue]) -> Option<u64> {
    match args.first().cloned() {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        Some(JsValue::Number(n)) if n.is_finite() && n >= 0.0 => Some(n as u64),
        _ => None,
    }
}

/// Append the serialisation of one [`Declaration`] to `out` (`property:
/// value[ !important]`). Used by [`RuleStyleCssText`] to build the
/// declaration-block text in a single allocation rather than collecting
/// per-declaration `String`s into a `Vec` and joining (PR-B review F4).
fn push_declaration(out: &mut String, decl: &elidex_css::Declaration) {
    out.push_str(&decl.property);
    out.push_str(": ");
    out.push_str(&decl.value.to_css_string());
    if decl.important {
        out.push_str(" !important");
    }
}

/// CSSOM §6.6.1 supported-property-name list: one [`Declaration`] per
/// distinct property name, ordered by first-occurrence index, with the
/// value taken from the last occurrence (cascade-wins semantics —
/// matches Chrome's `style.cssText` for `div{a:1; a:2}` returning
/// `a: 2;` and `style.length === 1`).  Backs `length` / `item` /
/// `cssText` for both Inline and Rule sources.
///
/// O(n) via a `HashMap<&str, slot>` mapping each property name to its
/// position in the result vector. Naive `position`-search would be
/// O(n²) — deep shorthand expansion (`border` / `font` / etc.) inflates
/// declaration counts past the point where that matters.
fn unique_properties_last_wins(decls: &[elidex_css::Declaration]) -> Vec<&elidex_css::Declaration> {
    let mut name_to_slot: std::collections::HashMap<&str, usize> =
        std::collections::HashMap::with_capacity(decls.len());
    let mut last_index: Vec<usize> = Vec::with_capacity(decls.len());
    for (i, d) in decls.iter().enumerate() {
        let name: &str = &d.property;
        if let Some(&slot) = name_to_slot.get(name) {
            last_index[slot] = i;
        } else {
            name_to_slot.insert(name, last_index.len());
            last_index.push(i);
        }
    }
    last_index.into_iter().map(|i| &decls[i]).collect()
}

/// `CSSStyleRule.cssText` (read) — full source text of the rule.
pub struct RuleCssText;

impl DomApiHandler for RuleCssText {
    fn method_name(&self) -> &str {
        "rule.cssText.get"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let text = with_rule(this, args, session, dom, String::new(), |r| {
            r.source_text.clone()
        });
        Ok(JsValue::String(text))
    }
}

/// `CSSStyleRule.selectorText` (read) — selector portion of the rule,
/// returned from the parser-captured `CssRule::selector_text` field
/// (the trimmed slice from `parse_prelude`'s `slice_from`). Avoids the
/// `split_once('{')` heuristic that would mis-slice selectors with
/// `{` inside an attribute value (e.g. `[data-x="{"]`). Setter is
/// deferred to slot `#11-css-rule-selector-text-set`.
pub struct RuleSelectorText;

impl DomApiHandler for RuleSelectorText {
    fn method_name(&self) -> &str {
        "rule.selectorText.get"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let text = with_rule(this, args, session, dom, String::new(), |r| {
            r.selector_text.clone()
        });
        Ok(JsValue::String(text))
    }
}

/// `CSSStyleRule.style.getPropertyValue(name)` — read declaration for
/// the named property from the rule's parsed declarations. Returns the
/// empty string when the rule_id is stale or the property is absent.
pub struct RuleStyleGetPropertyValue;

impl DomApiHandler for RuleStyleGetPropertyValue {
    fn method_name(&self) -> &str {
        "rule.style.getPropertyValue"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let property = require_string_arg(args, 1)?;
        let normalized = crate::util::normalize_property_name(&property);
        let value = with_rule(this, args, session, dom, String::new(), |r| {
            // Last declaration wins (mirrors the cascade). A shorthand
            // reconstructs from its longhands via the same canonical
            // `elidex_css::serialize_shorthand_value` the inline path
            // uses — rules are always parser-expanded, so a shorthand key
            // never appears in `declarations` directly.
            let last = |name: &str| {
                r.declarations
                    .iter()
                    .rev()
                    .find(|d| d.property == name)
                    .map(|d| (d.value.to_css_string(), d.important))
            };
            elidex_css::serialize_shorthand_value(&normalized, |lh| last(lh))
                .or_else(|| last(&normalized).map(|(value, _)| value))
                .unwrap_or_default()
        });
        Ok(JsValue::String(value))
    }
}

/// `CSSStyleRule.style.getPropertyPriority(name)` — returns `"important"`
/// when the rule's declaration for the named property carries the
/// `!important` flag, the empty string otherwise (CSSOM §6.6.1). The
/// last matching declaration wins, mirroring `getPropertyValue`.
pub struct RuleStyleGetPropertyPriority;

impl DomApiHandler for RuleStyleGetPropertyPriority {
    fn method_name(&self) -> &str {
        "rule.style.getPropertyPriority"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let property = require_string_arg(args, 1)?;
        let normalized = crate::util::normalize_property_name(&property);
        let important = with_rule(this, args, session, dom, false, |r| {
            // §6.6.1 getPropertyPriority step 1.2: a shorthand reads
            // "important" iff every mapped longhand does (the parser
            // stores rules longhand-expanded, so the shorthand key
            // itself never appears in `declarations`).
            let last_is_important = |name: &str| {
                r.declarations
                    .iter()
                    .rev()
                    .find(|d| d.property == name)
                    .is_some_and(|d| d.important)
            };
            let longhands = elidex_css::shorthand_longhands(&normalized);
            if longhands.is_empty() {
                last_is_important(&normalized)
            } else {
                longhands.iter().all(|lh| last_is_important(lh))
            }
        });
        Ok(JsValue::String(
            if important { "important" } else { "" }.to_string(),
        ))
    }
}

/// `CSSStyleRule.style.length` — number of declared properties (after
/// shorthand expansion) for the rule.
pub struct RuleStyleLength;

impl DomApiHandler for RuleStyleLength {
    fn method_name(&self) -> &str {
        "rule.style.length"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        #[allow(clippy::cast_precision_loss)]
        let len = with_rule(this, args, session, dom, 0.0, |r| {
            unique_properties_last_wins(&r.declarations).len() as f64
        });
        Ok(JsValue::Number(len))
    }
}

/// `CSSStyleRule.style[i]` — declared property name at index `i`.
pub struct RuleStyleItem;

impl DomApiHandler for RuleStyleItem {
    fn method_name(&self) -> &str {
        "rule.style.item"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        // CSSOM §6.6.1 `item(unsigned long index)` — WebIDL ToUint32
        // coercion (NaN → 0), matching the inline `StyleItem` path.
        let idx_f = match args.get(1) {
            Some(JsValue::Number(n)) => *n,
            _ => 0.0,
        };
        let idx = crate::util::webidl_unsigned_long(idx_f);
        let name = with_rule(this, args, session, dom, String::new(), |r| {
            unique_properties_last_wins(&r.declarations)
                .get(idx)
                .map_or_else(String::new, |d| d.property.clone())
        });
        Ok(JsValue::String(name))
    }
}

/// `CSSStyleRule.style.cssText` (read) — concatenated declaration block
/// as CSS text (`property: value; …`).  Per CSSOM §6.6.1.4 "serialize
/// a CSS declaration block": each property name appears at most once;
/// duplicates collapse to the cascade-winning (last) declaration.
pub struct RuleStyleCssText;

impl DomApiHandler for RuleStyleCssText {
    fn method_name(&self) -> &str {
        "rule.style.cssText.get"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let text = with_rule(this, args, session, dom, String::new(), |r| {
            let mut out = String::new();
            for (i, d) in unique_properties_last_wins(&r.declarations)
                .iter()
                .enumerate()
            {
                if i > 0 {
                    out.push_str("; ");
                }
                push_declaration(&mut out, d);
            }
            out
        });
        Ok(JsValue::String(text))
    }
}

// ---------------------------------------------------------------------------
// document.styleSheets walker
// ---------------------------------------------------------------------------

/// Collect all stylesheet-owning element entities (`<style>` and loaded
/// `<link rel="stylesheet">`) in document (tree) order. Backs the
/// host-side `document.styleSheets` indexed-property exotic and `item`
/// (CSSOM §6.8 — the sheets are returned in tree order).
#[must_use]
pub fn collect_stylesheet_owners(document: Entity, dom: &EcsDom) -> Vec<Entity> {
    let mut out = Vec::new();
    walk_styles(document, dom, |e| out.push(e));
    out
}

/// Count stylesheet-owning entities under `document` without
/// materialising the entity list. Backs `document.styleSheets.length` so
/// the common length-only read path avoids a per-access `Vec` allocation.
#[must_use]
pub fn count_stylesheet_owners(document: Entity, dom: &EcsDom) -> usize {
    let mut n: usize = 0;
    walk_styles(document, dom, |_| n += 1);
    n
}

/// `true` if `entity` is a `<link>` with an associated CSS style sheet —
/// non-null only after a successful load (HTML §4.6.7), i.e. a
/// `LinkStylesheet` component is attached. Backs the host `link.sheet`
/// getter (CSSOM §6.2 `LinkStyle.sheet`).
#[must_use]
pub fn link_has_loaded_sheet(entity: Entity, dom: &EcsDom) -> bool {
    dom.world().get::<&LinkStylesheet>(entity).is_ok()
}

/// Resolved absolute URL of a `<link>`-loaded sheet (CSSOM §6.2
/// `StyleSheet.href`); `None` for a `<style>` sheet (no href). Backs the
/// host `sheet.href` getter.
#[must_use]
pub fn link_sheet_href(entity: Entity, dom: &EcsDom) -> Option<String> {
    dom.world()
        .get::<&LinkStylesheet>(entity)
        .ok()
        .map(|link| link.href.clone())
}

fn walk_styles(entity: Entity, dom: &EcsDom, mut visit: impl FnMut(Entity)) {
    walk_styles_inner(entity, dom, &mut visit);
}

fn walk_styles_inner(entity: Entity, dom: &EcsDom, visit: &mut impl FnMut(Entity)) {
    // ASCII case-insensitive tag match per WHATWG DOM §4.2.6.2 — `<STYLE>`
    // (mixed-case via raw `create_element`) is a valid HTML style element
    // and must surface in `document.styleSheets`.
    let is_style = dom.with_tag_name(entity, |t| {
        t.is_some_and(|t| t.eq_ignore_ascii_case("style"))
    });
    // CSSOM §6.8: `document.styleSheets` enumerates every element with an
    // associated CSS style sheet, in tree order. A `<link rel="stylesheet">`
    // gains one once its resource loads (HTML §4.6.7) — signalled by the
    // `LinkStylesheet` component attached by the resource loader.
    let is_loaded_link = dom.world().get::<&LinkStylesheet>(entity).is_ok();
    if is_style || is_loaded_link {
        visit(entity);
    }
    for child in dom.children_iter(entity) {
        walk_styles_inner(child, dom, visit);
    }
}

#[cfg(test)]
mod tests {
    use super::{cssom_actual_index, cssom_insert_position, cssom_visible_count};
    use elidex_css::{parse_stylesheet, Origin};

    /// `div{}` (0) / `@media screen{p{}}` (1, flattened+conditioned) / `span{}` (2).
    /// The CSSOM view is `[div, span]` — the `@media`-flattened `p` is hidden.
    fn mixed() -> elidex_css::Stylesheet {
        parse_stylesheet(
            "div { color: red } @media screen { p { color: blue } } span { color: green }",
            Origin::Author,
        )
    }

    #[test]
    fn visible_count_excludes_media_rules() {
        let ss = mixed();
        assert_eq!(ss.rules.len(), 3, "all three rules are in the cascade list");
        assert_eq!(
            cssom_visible_count(&ss.rules),
            2,
            "cssRules hides the flattened @media rule"
        );
    }

    #[test]
    fn actual_index_skips_media_rules() {
        let ss = mixed();
        // CSSOM index 0 → `div` (actual 0); CSSOM index 1 → `span` (actual 2,
        // skipping the @media `p` at actual 1).
        assert_eq!(cssom_actual_index(&ss.rules, 0), Some(0));
        assert_eq!(cssom_actual_index(&ss.rules, 1), Some(2));
        assert_eq!(cssom_actual_index(&ss.rules, 2), None); // past the last visible
    }

    #[test]
    fn insert_position_maps_visible_to_actual() {
        let ss = mixed();
        assert_eq!(cssom_insert_position(&ss.rules, 0), 0); // before `div`
        assert_eq!(cssom_insert_position(&ss.rules, 1), 2); // before `span`
        assert_eq!(cssom_insert_position(&ss.rules, 2), 3); // append (end of rules)
    }
}
