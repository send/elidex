//! `Element.style` (CSSStyleDeclaration §6.6.1) DOM API handlers — backing the
//! mutable inline-style declaration block.
//!
//! Each mutator (`setProperty` / `removeProperty` / `cssText.set`) syncs the
//! resulting `InlineStyle` back into `attrs("style")` so the cascade — which
//! reads inline declarations from the `style` attribute via
//! [`elidex_css::parse_declaration_block`] in
//! `elidex_style::cascade::get_inline_declarations` — observes the change on
//! the next walk.  Without this round-trip, `el.style.color = "red"` would be
//! invisible to layout because the cascade never sees `InlineStyle`-only
//! writes.
//!
//! Property-name normalisation (CSSOM §6.6.1): non-custom property names are
//! ASCII-lowercased before lookup / write so `style.getPropertyValue("Color")`
//! returns the value of `color`.  Custom properties (`--*`) are case-sensitive
//! per CSS Variables Level 1 §2 and are NOT lowercased.

use elidex_ecs::{Attributes, EcsDom, Entity, InlineStyle};
use elidex_plugin::JsValue;
use elidex_script_session::{DomApiError, DomApiHandler, SessionCore};

use crate::util::{
    normalize_property_name, normalize_property_name_owned, not_found_error, require_string_arg,
};

/// The default CSS property registry, so the inline-style parse resolves
/// registry-backed properties (`transform`, `transition`, …) — without
/// it, `el.style.transform = 'rotate(45deg)'` would silently no-op.
/// Same registry the cascade's `get_inline_declarations` uses, so the
/// CSSOM `InlineStyle` and the cascade agree on registry-backed inline
/// declarations.
fn inline_style_registry() -> &'static elidex_plugin::CssPropertyRegistry {
    elidex_style::default_css_property_registry()
}

/// Ensure an `InlineStyle` component exists on the entity, hydrating it
/// from `attrs("style")` when absent (the canonical, registry-aware
/// derivation). This is the single InlineStyle materialization point:
/// the parser does NOT attach `InlineStyle` at element creation —
/// the cascade reads `attrs("style")` directly, so the component is
/// needed only for the CSSOM surface and is built lazily here on first
/// access (read or write). Idempotent: a present component is left
/// untouched (its declarations may already diverge from the attribute
/// via prior CSSOM mutation — the write path keeps them synced).
fn ensure_inline_style(entity: Entity, dom: &mut EcsDom) {
    if dom.world_mut().get::<&InlineStyle>(entity).is_ok() {
        return;
    }
    // Snapshot the attribute before re-borrowing world_mut for insert_one.
    let attr_value: Option<String> = dom
        .world()
        .get::<&Attributes>(entity)
        .ok()
        .and_then(|a| a.get("style").map(str::to_owned));
    let hydrated = attr_value
        .map(|css| elidex_css::parse_inline_style(&css, Some(inline_style_registry())))
        .unwrap_or_default();
    let _ = dom.world_mut().insert_one(entity, hydrated);
}

/// Remove `property`'s declaration(s) from the block — shorthand-aware
/// per CSSOM §6.6.1 `removeProperty`: a shorthand removes each longhand
/// it maps to (the component's canonical key-space is longhand-expanded,
/// so the shorthand key itself never exists).
///
/// Returns `(old_value, removed)`. `old_value` is the removed value for
/// a longhand; for a shorthand it is `None` — §6.6.1 returns
/// `getPropertyValue(property)`, and shorthand *read-side* serialization
/// from longhands is deferred (slot `#11-style-shorthand-expand`), so
/// the getter and this return value agree on the empty string.
/// `removed` mirrors the spec's removed flag: callers run the
/// style-attribute write-back ONLY when it is true, so removing an
/// absent (or unsupported) property is observably a no-op.
fn remove_declarations(style: &mut InlineStyle, property: &str) -> (Option<String>, bool) {
    let longhands = elidex_css::shorthand_longhands(property);
    if longhands.is_empty() {
        let old = style.remove(property);
        let removed = old.is_some();
        return (old, removed);
    }
    let mut removed = false;
    for longhand in &longhands {
        removed |= style.remove(longhand).is_some();
    }
    (None, removed)
}

/// Round-trip the current `InlineStyle` declarations into `attrs("style")`
/// so the cascade picks them up on the next walk.  CRIT-1 mitigation:
/// `elidex_style::cascade::get_inline_declarations` reads from
/// `attrs("style")` (not `InlineStyle`), so without this sync any mutation
/// through `el.style.*` would be invisible to layout.
///
/// Routes through [`EcsDom::set_attribute`] (rather than mutating the
/// `Attributes` component directly) so the canonical
/// [`EcsDom::rev_version`] bump fires alongside the write.  Without that
/// version-bump path, LiveCollection / layout / mutation-observer caches
/// keyed on `inclusive_descendants_version` would stay stale across
/// `el.style.*` mutations even though the attribute string changed.
///
/// Empty inline-style produces an empty attribute (preserves `style=""`
/// rather than removing it) — matches Chrome's behaviour for an
/// inline-style block emptied via `removeProperty`.
fn sync_to_attribute(entity: Entity, dom: &mut EcsDom) {
    // Clone the component before the write: `EcsDom::set_attribute("style",
    // …)` drops the cached `InlineStyle` (it treats the component as a
    // memoized parse of the attribute, invalidated when the attribute
    // changes externally). Here the write is derived FROM the component,
    // so the cache is already consistent — re-insert it afterward to keep
    // the CSSOM mutation path warm (no re-parse on the next read).
    let style: InlineStyle = {
        match dom.world().get::<&InlineStyle>(entity) {
            Ok(s) => (*s).clone(),
            Err(_) => return,
        }
    };
    let css_text = style.css_text();
    let _ = dom.set_attribute(entity, "style", &css_text);
    let _ = dom.world_mut().insert_one(entity, style);
}

// ---------------------------------------------------------------------------
// style.setProperty
// ---------------------------------------------------------------------------

/// `element.style.setProperty(property, value, priority?)` — sets an
/// inline style declaration per the CSSOM §6.6.1 algorithm (empty value
/// ⇒ remove; unsupported property / unparseable value / invalid
/// priority ⇒ no-op; value stored in canonical longhand-expanded form).
pub struct StyleSetProperty;

impl DomApiHandler for StyleSetProperty {
    fn method_name(&self) -> &str {
        "style.setProperty"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let property_raw = require_string_arg(args, 0)?;
        let property = normalize_property_name_owned(property_raw);
        let value = require_string_arg(args, 1)?;

        // Stale-wrapper guard: same contract as the sibling mutators
        // (`removeProperty` / `cssText.set`), before any early return.
        if !dom.world().contains(this) {
            return Err(not_found_error("element not found"));
        }

        // §6.6.1 step 3: an empty value means removeProperty
        // (shorthand-aware, like the real handler). Runs BEFORE the
        // step-4 priority check per the spec's step order — an invalid
        // priority does not rescue the declaration from removal.
        if value.is_empty() {
            ensure_inline_style(this, dom);
            let removed = match dom.world_mut().get::<&mut InlineStyle>(this) {
                Ok(mut style) => remove_declarations(&mut style, property.as_str()).1,
                Err(_) => false,
            };
            // §6.6.1 removeProperty step 6: update the style attribute
            // only "if removed is true" — an unsupported or absent
            // property must not dirty `attrs("style")`.
            if removed {
                sync_to_attribute(this, dom);
            }
            return Ok(JsValue::Undefined);
        }

        // §6.6.1 step 4: empty / absent priority ⇒ normal,
        // ASCII-case-insensitive "important" ⇒ important, anything else
        // ⇒ return without effect.
        let important = match args.get(2) {
            None | Some(JsValue::Undefined) => false,
            Some(JsValue::String(p)) if p.is_empty() => false,
            Some(JsValue::String(p)) if p.eq_ignore_ascii_case("important") => true,
            Some(_) => return Ok(JsValue::Undefined),
        };

        // §6.6.1 steps 2.2 / 5–6: parse the value for the property and
        // store the canonical parsed form (longhand-expanded). An
        // unsupported property, an unparseable value, or trailing input
        // (including a smuggled `!important` or `; other: decl`) returns
        // without effect — storing the raw string verbatim would let the
        // cascade's re-parse of the written-back `style` attribute
        // fabricate declarations / priority out of the value text.
        let Some(decls) =
            elidex_css::parse_value_for_property(&property, &value, Some(inline_style_registry()))
        else {
            return Ok(JsValue::Undefined);
        };

        ensure_inline_style(this, dom);

        {
            let mut style = dom
                .world_mut()
                .get::<&mut InlineStyle>(this)
                .map_err(|_| not_found_error("element not found"))?;
            for decl in decls {
                style.set_with_priority(decl.property, decl.value.to_css_string(), important);
            }
        }
        sync_to_attribute(this, dom);
        Ok(JsValue::Undefined)
    }
}

// ---------------------------------------------------------------------------
// style.getPropertyValue
// ---------------------------------------------------------------------------

/// `element.style.getPropertyValue(property)` — gets an inline style value.
pub struct StyleGetPropertyValue;

impl DomApiHandler for StyleGetPropertyValue {
    fn method_name(&self) -> &str {
        "style.getPropertyValue"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let property_raw = require_string_arg(args, 0)?;
        let property = normalize_property_name(&property_raw);
        // Distinguish stale-wrapper (entity not in world → NotFoundError,
        // matches the mutator handlers' shape) from "InlineStyle component
        // absent" (freshly-created element with no inline declarations →
        // empty string, which is the spec-correct CSSOM §6.6.1 named-getter
        // result for an empty declaration block).
        if !dom.world().contains(this) {
            return Err(not_found_error("element not found"));
        }
        ensure_inline_style(this, dom);
        match dom.world().get::<&InlineStyle>(this) {
            Ok(style) => match style.get(property.as_ref()) {
                Some(val) => Ok(JsValue::String(val.to_string())),
                None => Ok(JsValue::String(String::new())),
            },
            Err(_) => Ok(JsValue::String(String::new())),
        }
    }
}

// ---------------------------------------------------------------------------
// style.getPropertyPriority
// ---------------------------------------------------------------------------

/// `element.style.getPropertyPriority(property)` — returns `"important"`
/// when the named inline declaration carries the `!important` flag, the
/// empty string otherwise (CSSOM §6.6.1). Hydrates `InlineStyle` from
/// `attrs("style")` on read like its sibling handlers.
pub struct StyleGetPropertyPriority;

impl DomApiHandler for StyleGetPropertyPriority {
    fn method_name(&self) -> &str {
        "style.getPropertyPriority"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let property_raw = require_string_arg(args, 0)?;
        let property = normalize_property_name(&property_raw);
        // Stale-wrapper guard mirrors `StyleGetPropertyValue`.
        if !dom.world().contains(this) {
            return Err(not_found_error("element not found"));
        }
        ensure_inline_style(this, dom);
        // CSSOM §6.6.1 getPropertyPriority step 1.2: for a shorthand,
        // return "important" iff every mapped longhand's priority is
        // "important" (an absent longhand reads as "", failing the all).
        let important = dom.world().get::<&InlineStyle>(this).is_ok_and(|style| {
            let longhands = elidex_css::shorthand_longhands(property.as_ref());
            if longhands.is_empty() {
                style.is_important(property.as_ref())
            } else {
                longhands.iter().all(|lh| style.is_important(lh))
            }
        });
        Ok(JsValue::String(
            if important { "important" } else { "" }.to_string(),
        ))
    }
}

// ---------------------------------------------------------------------------
// style.removeProperty
// ---------------------------------------------------------------------------

/// `element.style.removeProperty(property)` — removes an inline style.
pub struct StyleRemoveProperty;

impl DomApiHandler for StyleRemoveProperty {
    fn method_name(&self) -> &str {
        "style.removeProperty"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let property_raw = require_string_arg(args, 0)?;
        let property = normalize_property_name(&property_raw);
        // Stale-wrapper guard mirrors `StyleGetPropertyValue`.
        if !dom.world().contains(this) {
            return Err(not_found_error("element not found"));
        }
        // Hydrate `InlineStyle` from `attrs("style")` if absent so
        // `setAttribute("style","color:red"); el.style.removeProperty(
        // "color")` actually removes the declaration from the cascade-
        // visible attribute (instead of silently no-op'ing because the
        // ECS component happened not to exist yet).  Symmetric with
        // `StyleSetProperty`'s seed-on-first-mutation policy in
        // `ensure_inline_style`.
        ensure_inline_style(this, dom);
        let (old_value, removed) = match dom.world_mut().get::<&mut InlineStyle>(this) {
            Ok(mut style) => remove_declarations(&mut style, property.as_ref()),
            Err(_) => return Ok(JsValue::String(String::new())),
        };
        // §6.6.1 removeProperty step 6: write back only "if removed is
        // true" — removing an absent property is observably a no-op.
        if removed {
            sync_to_attribute(this, dom);
        }
        Ok(JsValue::String(old_value.unwrap_or_default()))
    }
}

// ---------------------------------------------------------------------------
// style.length (RO accessor)
// ---------------------------------------------------------------------------

/// `element.style.length` — the number of declared inline-style properties.
pub struct StyleLength;

impl DomApiHandler for StyleLength {
    fn method_name(&self) -> &str {
        "style.length"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        // Stale-wrapper distinction: NotFoundError if the entity has been
        // removed; `0` if the entity exists but lacks `InlineStyle` (freshly-
        // created element).  Mirrors the read-handler shape in
        // `StyleGetPropertyValue`.
        if !dom.world().contains(this) {
            return Err(not_found_error("element not found"));
        }
        ensure_inline_style(this, dom);
        let len = dom.world().get::<&InlineStyle>(this).map_or(0, |s| s.len());
        #[allow(clippy::cast_precision_loss)]
        Ok(JsValue::Number(len as f64))
    }
}

// ---------------------------------------------------------------------------
// style.item(index)
// ---------------------------------------------------------------------------

/// `element.style.item(i)` — property name at index `i`, or empty string when
/// out of range (CSSOM §6.6.1 indexed getter).
pub struct StyleItem;

impl DomApiHandler for StyleItem {
    fn method_name(&self) -> &str {
        "style.item"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let idx_f = match args.first() {
            Some(JsValue::Number(n)) => *n,
            _ => 0.0,
        };
        if !idx_f.is_finite() || idx_f < 0.0 {
            return Ok(JsValue::String(String::new()));
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let idx = idx_f as usize;
        // Same stale-wrapper / missing-component split as
        // `StyleGetPropertyValue`.
        if !dom.world().contains(this) {
            return Err(not_found_error("element not found"));
        }
        ensure_inline_style(this, dom);
        match dom.world().get::<&InlineStyle>(this) {
            Ok(style) => Ok(JsValue::String(
                style.property_at(idx).unwrap_or("").to_string(),
            )),
            Err(_) => Ok(JsValue::String(String::new())),
        }
    }
}

// ---------------------------------------------------------------------------
// style.cssText (RW)
// ---------------------------------------------------------------------------

/// `element.style.cssText` getter — serialised inline declarations.
pub struct StyleCssTextGet;

impl DomApiHandler for StyleCssTextGet {
    fn method_name(&self) -> &str {
        "style.cssText.get"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        // Same stale-wrapper / missing-component split as the read-handler
        // family above — NotFoundError on stale entities, empty string on
        // entity-without-InlineStyle (CSSOM §6.6.1 cssText getter on an
        // empty declaration block).
        if !dom.world().contains(this) {
            return Err(not_found_error("element not found"));
        }
        ensure_inline_style(this, dom);
        let text = match dom.world().get::<&InlineStyle>(this) {
            Ok(style) => style.css_text(),
            Err(_) => String::new(),
        };
        Ok(JsValue::String(text))
    }
}

/// `element.style.cssText` setter — replaces the declaration block by
/// re-parsing `value` through the canonical
/// [`elidex_css::parse_inline_style`] (which performs shorthand
/// expansion).  All-or-nothing replacement: it returns an empty
/// `InlineStyle` for empty / unparseable input, which clears the
/// inline-style block.  This matches the spec "parse a CSS declaration
/// block" algorithm and Chrome's behaviour for whole-block-invalid input
/// (accepted divergence: Firefox preserves existing declarations on
/// whole-block parse failure).
pub struct StyleCssTextSet;

impl DomApiHandler for StyleCssTextSet {
    fn method_name(&self) -> &str {
        "style.cssText.set"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let css = require_string_arg(args, 0)?;

        // Stale-wrapper guard: same shape as the read handlers + the
        // mutator handlers (`setProperty` / `removeProperty`).  Without
        // this, `world_mut().insert_one` silently fails on a removed
        // entity and the cssText set becomes a no-op — inconsistent with
        // the rest of the surface.
        if !dom.world().contains(this) {
            return Err(not_found_error("element not found"));
        }

        // All-or-nothing replace: drop any existing component and insert
        // a freshly-built one so insertion order matches the parsed
        // declarations exactly (no leftover keys from prior content).
        let new_style = elidex_css::parse_inline_style(&css, Some(inline_style_registry()));
        dom.world_mut()
            .insert_one(this, new_style)
            .map_err(|_| not_found_error("element not found"))?;
        sync_to_attribute(this, dom);
        Ok(JsValue::Undefined)
    }
}

#[cfg(test)]
#[allow(unused_must_use)] // Test setup calls dom.append_child() etc. without checking return values
mod tests;
