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

/// Ensure an `InlineStyle` component exists on the entity.  When the
/// component is absent but `attrs("style")` already has content (from a
/// prior `setAttribute("style", "...")`), parse the attribute string
/// into declarations and seed the new `InlineStyle` so the next mutation
/// via `style.setProperty` / `removeProperty` / `cssText` doesn't
/// silently drop those declarations during the post-mutation
/// `sync_to_attribute` round-trip.
///
/// This is the inverse of `sync_to_attribute`: the cascade reads
/// declarations from `attrs("style")` so writes go style → attrs; this
/// helper handles the symmetric attrs → style hydration on first
/// mutation, closing the data-loss gap that arose when
/// `setAttribute("style", "color: red"); el.style.setProperty("foo",
/// "bar");` would land `attrs("style") = "foo: bar"` (color: red lost).
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
        .map(|css| elidex_css::parse_inline_style(&css))
        .unwrap_or_default();
    let _ = dom.world_mut().insert_one(entity, hydrated);
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
    let css_text = match dom.world().get::<&InlineStyle>(entity) {
        Ok(style) => style.css_text(),
        Err(_) => return,
    };
    let _ = dom.set_attribute(entity, "style", &css_text);
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
        // Optional third argument (CSSOM §6.6.1 `setProperty` step 4):
        // empty / absent ⇒ normal priority, ASCII-case-insensitive
        // "important" ⇒ important, anything else ⇒ return without effect.
        let important = match args.get(2) {
            None | Some(JsValue::Undefined) => false,
            Some(JsValue::String(p)) if p.is_empty() => false,
            Some(JsValue::String(p)) if p.eq_ignore_ascii_case("important") => true,
            Some(_) => return Ok(JsValue::Undefined),
        };

        // §6.6.1 step 3: an empty value means removeProperty.
        if value.is_empty() {
            ensure_inline_style(this, dom);
            if let Ok(mut style) = dom.world_mut().get::<&mut InlineStyle>(this) {
                style.remove(property.as_str());
            }
            sync_to_attribute(this, dom);
            return Ok(JsValue::Undefined);
        }

        // §6.6.1 steps 2.2 / 5–6: parse the value for the property and
        // store the canonical parsed form (longhand-expanded). An
        // unsupported property, an unparseable value, or trailing input
        // (including a smuggled `!important` or `; other: decl`) returns
        // without effect — storing the raw string verbatim would let the
        // cascade's re-parse of the written-back `style` attribute
        // fabricate declarations / priority out of the value text.
        let Some(decls) = elidex_css::parse_value_for_property(&property, &value) else {
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
/// empty string otherwise (CSSOM §6.6.1).
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
        let important = dom
            .world()
            .get::<&InlineStyle>(this)
            .is_ok_and(|style| style.is_important(property.as_ref()));
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
        let old_value = match dom.world_mut().get::<&mut InlineStyle>(this) {
            Ok(mut style) => style.remove(property.as_ref()).unwrap_or_default(),
            Err(_) => return Ok(JsValue::String(String::new())),
        };
        sync_to_attribute(this, dom);
        Ok(JsValue::String(old_value))
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
        let new_style = elidex_css::parse_inline_style(&css);
        dom.world_mut()
            .insert_one(this, new_style)
            .map_err(|_| not_found_error("element not found"))?;
        sync_to_attribute(this, dom);
        Ok(JsValue::Undefined)
    }
}

#[cfg(test)]
#[allow(unused_must_use)] // Test setup calls dom.append_child() etc. without checking return values
mod tests {
    use super::*;
    use elidex_ecs::Attributes;

    fn setup() -> (EcsDom, Entity, SessionCore) {
        let mut dom = EcsDom::new();
        let elem = dom.create_element("div", Attributes::default());
        let session = SessionCore::new();
        (dom, elem, session)
    }

    #[test]
    fn set_and_get_property() {
        let (mut dom, elem, mut session) = setup();
        StyleSetProperty
            .invoke(
                elem,
                &[
                    JsValue::String("color".into()),
                    JsValue::String("red".into()),
                ],
                &mut session,
                &mut dom,
            )
            .unwrap();

        let result = StyleGetPropertyValue
            .invoke(
                elem,
                &[JsValue::String("color".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        // Canonical §6.6.1 parse: the `red` keyword stores in hex form.
        assert_eq!(result, JsValue::String("#ff0000".into()));
    }

    #[test]
    fn set_property_with_important_priority_round_trips_to_attribute() {
        // CSSOM §6.6.1 setProperty third argument + the cascade-visible
        // write-back: `sync_to_attribute` must re-emit `!important` so
        // the cascade's attribute re-parse keeps the priority.
        let (mut dom, elem, mut session) = setup();
        StyleSetProperty
            .invoke(
                elem,
                &[
                    JsValue::String("color".into()),
                    JsValue::String("red".into()),
                    JsValue::String("important".into()),
                ],
                &mut session,
                &mut dom,
            )
            .unwrap();

        let priority = StyleGetPropertyPriority
            .invoke(
                elem,
                &[JsValue::String("color".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(priority, JsValue::String("important".into()));

        // getPropertyValue returns the value only (no priority text).
        let value = StyleGetPropertyValue
            .invoke(
                elem,
                &[JsValue::String("color".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(value, JsValue::String("#ff0000".into()));

        let attrs = dom.world().get::<&Attributes>(elem).unwrap();
        assert_eq!(attrs.get("style"), Some("color: #ff0000 !important"));
    }

    #[test]
    fn set_property_invalid_priority_is_no_op() {
        // CSSOM §6.6.1 setProperty step 4: a priority that is neither
        // empty nor "important" returns without effect.
        let (mut dom, elem, mut session) = setup();
        StyleSetProperty
            .invoke(
                elem,
                &[
                    JsValue::String("color".into()),
                    JsValue::String("red".into()),
                    JsValue::String("very-important".into()),
                ],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let value = StyleGetPropertyValue
            .invoke(
                elem,
                &[JsValue::String("color".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(value, JsValue::String(String::new()));
    }

    #[test]
    fn set_property_clears_prior_importance() {
        // setProperty with empty priority resets the flag (§6.6.1).
        let (mut dom, elem, mut session) = setup();
        StyleSetProperty
            .invoke(
                elem,
                &[
                    JsValue::String("color".into()),
                    JsValue::String("red".into()),
                    JsValue::String("IMPORTANT".into()), // ASCII-case-insensitive
                ],
                &mut session,
                &mut dom,
            )
            .unwrap();
        StyleSetProperty
            .invoke(
                elem,
                &[
                    JsValue::String("color".into()),
                    JsValue::String("blue".into()),
                ],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let priority = StyleGetPropertyPriority
            .invoke(
                elem,
                &[JsValue::String("color".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(priority, JsValue::String(String::new()));
    }

    #[test]
    fn parser_important_survives_unrelated_mutation() {
        // The regression this PR closes: a parser-derived (hydrated)
        // `!important` declaration must survive the attribute rewrite
        // triggered by an unrelated `el.style.*` write.
        let (mut dom, elem, mut session) = setup();
        let _ = dom.set_attribute(elem, "style", "color: red !important");
        StyleSetProperty
            .invoke(
                elem,
                &[
                    JsValue::String("width".into()),
                    JsValue::String("10px".into()),
                ],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let attrs = dom.world().get::<&Attributes>(elem).unwrap();
        assert_eq!(
            attrs.get("style"),
            Some("color: #ff0000 !important; width: 10px")
        );
    }

    #[test]
    fn get_nonexistent_property() {
        let (mut dom, elem, mut session) = setup();
        let result = StyleGetPropertyValue
            .invoke(
                elem,
                &[JsValue::String("color".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::String(String::new()));
    }

    #[test]
    fn remove_property() {
        let (mut dom, elem, mut session) = setup();
        StyleSetProperty
            .invoke(
                elem,
                &[
                    JsValue::String("color".into()),
                    JsValue::String("blue".into()),
                ],
                &mut session,
                &mut dom,
            )
            .unwrap();

        let result = StyleRemoveProperty
            .invoke(
                elem,
                &[JsValue::String("color".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::String("#0000ff".into()));

        // Verify it's gone.
        let result = StyleGetPropertyValue
            .invoke(
                elem,
                &[JsValue::String("color".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::String(String::new()));
    }

    #[test]
    fn auto_creates_inline_style_component() {
        let (mut dom, elem, mut session) = setup();
        // No InlineStyle component initially.
        assert!(dom.world().get::<&InlineStyle>(elem).is_err());

        StyleSetProperty
            .invoke(
                elem,
                &[
                    JsValue::String("display".into()),
                    JsValue::String("none".into()),
                ],
                &mut session,
                &mut dom,
            )
            .unwrap();

        // InlineStyle component should now exist.
        assert!(dom.world().get::<&InlineStyle>(elem).is_ok());
    }

    /// Copilot R7 regression: `sync_to_attribute` must route through
    /// `EcsDom::set_attribute` so `rev_version` fires.  Without the
    /// bump, LiveCollection / layout / mutation-observer caches keyed
    /// on `inclusive_descendants_version` stay stale across
    /// `el.style.*` mutations.
    #[test]
    fn set_property_bumps_subtree_version() {
        let (mut dom, elem, mut session) = setup();
        let before = dom.inclusive_descendants_version(elem);
        StyleSetProperty
            .invoke(
                elem,
                &[
                    JsValue::String("color".into()),
                    JsValue::String("red".into()),
                ],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let after = dom.inclusive_descendants_version(elem);
        assert!(
            after > before,
            "subtree version must bump on style write (before={before}, after={after})"
        );
    }

    /// CRIT-1 regression: setProperty must round-trip through
    /// `attrs("style")` so the cascade observes the change.
    #[test]
    fn set_property_syncs_attrs_style() {
        let (mut dom, elem, mut session) = setup();
        StyleSetProperty
            .invoke(
                elem,
                &[
                    JsValue::String("color".into()),
                    JsValue::String("red".into()),
                ],
                &mut session,
                &mut dom,
            )
            .unwrap();

        let attrs = dom.world().get::<&Attributes>(elem).unwrap();
        assert_eq!(attrs.get("style").unwrap(), "color: #ff0000");
    }

    /// CRIT-1 regression: removeProperty must update `attrs("style")`.
    #[test]
    fn remove_property_syncs_attrs_style() {
        let (mut dom, elem, mut session) = setup();
        StyleSetProperty
            .invoke(
                elem,
                &[
                    JsValue::String("color".into()),
                    JsValue::String("red".into()),
                ],
                &mut session,
                &mut dom,
            )
            .unwrap();
        StyleSetProperty
            .invoke(
                elem,
                &[
                    JsValue::String("display".into()),
                    JsValue::String("block".into()),
                ],
                &mut session,
                &mut dom,
            )
            .unwrap();
        StyleRemoveProperty
            .invoke(
                elem,
                &[JsValue::String("color".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();

        let attrs = dom.world().get::<&Attributes>(elem).unwrap();
        assert_eq!(attrs.get("style").unwrap(), "display: block");
    }

    /// IMP-3: ASCII-lowercase normalisation for non-custom property names.
    #[test]
    fn property_name_lowercased() {
        let (mut dom, elem, mut session) = setup();
        StyleSetProperty
            .invoke(
                elem,
                &[
                    JsValue::String("Color".into()),
                    JsValue::String("red".into()),
                ],
                &mut session,
                &mut dom,
            )
            .unwrap();

        // Stored under "color" (lowercase), canonical hex form.
        let result = StyleGetPropertyValue
            .invoke(
                elem,
                &[JsValue::String("color".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::String("#ff0000".into()));

        // Mixed-case lookup also lowercases.
        let result = StyleGetPropertyValue
            .invoke(
                elem,
                &[JsValue::String("CoLoR".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::String("#ff0000".into()));
    }

    /// IMP-3: custom properties (`--*`) preserve case (CSS Variables L1 §2).
    #[test]
    fn custom_property_case_preserved() {
        let (mut dom, elem, mut session) = setup();
        StyleSetProperty
            .invoke(
                elem,
                &[
                    JsValue::String("--MyVar".into()),
                    JsValue::String("42".into()),
                ],
                &mut session,
                &mut dom,
            )
            .unwrap();

        let result = StyleGetPropertyValue
            .invoke(
                elem,
                &[JsValue::String("--MyVar".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::String("42".into()));

        // Different case = different property.
        let result = StyleGetPropertyValue
            .invoke(
                elem,
                &[JsValue::String("--myvar".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::String(String::new()));
    }

    #[test]
    fn length_and_item() {
        let (mut dom, elem, mut session) = setup();
        StyleSetProperty
            .invoke(
                elem,
                &[
                    JsValue::String("color".into()),
                    JsValue::String("red".into()),
                ],
                &mut session,
                &mut dom,
            )
            .unwrap();
        StyleSetProperty
            .invoke(
                elem,
                &[
                    JsValue::String("display".into()),
                    JsValue::String("block".into()),
                ],
                &mut session,
                &mut dom,
            )
            .unwrap();

        let len = StyleLength
            .invoke(elem, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(len, JsValue::Number(2.0));

        let item0 = StyleItem
            .invoke(elem, &[JsValue::Number(0.0)], &mut session, &mut dom)
            .unwrap();
        assert_eq!(item0, JsValue::String("color".into()));

        let item_oob = StyleItem
            .invoke(elem, &[JsValue::Number(99.0)], &mut session, &mut dom)
            .unwrap();
        assert_eq!(item_oob, JsValue::String(String::new()));
    }

    #[test]
    fn css_text_round_trip() {
        let (mut dom, elem, mut session) = setup();
        StyleCssTextSet
            .invoke(
                elem,
                &[JsValue::String("color: red; display: block".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();

        // `parse_inline_style` parses `color: red` into
        // `CssValue::Color(...)` which `CssValue::to_css_string` then
        // serializes via the color's `Display` impl (hex form).  The
        // round-trip therefore produces `#ff0000` rather than the input
        // `red` keyword — accepted divergence for PR-A; lossless
        // colour-keyword round-trip is paired with the CSSOM serializer
        // work in PR-B.
        let result = StyleGetPropertyValue
            .invoke(
                elem,
                &[JsValue::String("color".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let JsValue::String(s) = result else {
            panic!("expected string")
        };
        assert!(
            !s.is_empty(),
            "color round-trip should produce a non-empty value"
        );

        // `display: block` is a keyword — exact round-trip.
        let display = StyleGetPropertyValue
            .invoke(
                elem,
                &[JsValue::String("display".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(display, JsValue::String("block".into()));

        // cssText getter serializes back.
        let text = StyleCssTextGet
            .invoke(elem, &[], &mut session, &mut dom)
            .unwrap();
        let JsValue::String(text_s) = text else {
            panic!("expected string")
        };
        assert!(text_s.contains("display: block"));
    }

    /// IMP-8: cssText="garbage" clears the block (all-or-nothing semantics).
    #[test]
    fn css_text_invalid_clears() {
        let (mut dom, elem, mut session) = setup();
        StyleSetProperty
            .invoke(
                elem,
                &[
                    JsValue::String("color".into()),
                    JsValue::String("red".into()),
                ],
                &mut session,
                &mut dom,
            )
            .unwrap();
        StyleCssTextSet
            .invoke(
                elem,
                &[JsValue::String("garbage }}}".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();

        let len = StyleLength
            .invoke(elem, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(len, JsValue::Number(0.0));
    }
}
