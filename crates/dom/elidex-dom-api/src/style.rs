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

use std::borrow::Cow;

use elidex_ecs::{Attributes, EcsDom, Entity, InlineStyle};
use elidex_plugin::JsValue;
use elidex_script_session::{DomApiError, DomApiHandler, SessionCore};

use crate::util::{not_found_error, require_string_arg};

/// CSSOM §6.6.1 property-name normalisation: ASCII-lowercase non-custom
/// names; preserve case for custom properties (`--*`).  Returns a borrowed
/// `&str` when no allocation is needed (already lowercase / starts with `--`).
fn normalize_property_name(name: &str) -> Cow<'_, str> {
    if name.starts_with("--") {
        Cow::Borrowed(name)
    } else if name.bytes().any(|b| b.is_ascii_uppercase()) {
        Cow::Owned(name.to_ascii_lowercase())
    } else {
        Cow::Borrowed(name)
    }
}

/// In-place ASCII-lowercase variant for the common path where the caller
/// already owns a `String` from arg coercion.  Avoids the [`Cow`]
/// `into_owned` round-trip for the most-frequent shape (no uppercase).
/// Custom properties (`--*`) are passed through unchanged per CSS
/// Variables L1 §2.
fn normalize_property_name_owned(mut name: String) -> String {
    if !name.starts_with("--") && name.bytes().any(|b| b.is_ascii_uppercase()) {
        name.make_ascii_lowercase();
    }
    name
}

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
    let mut hydrated = InlineStyle::default();
    if let Some(css) = attr_value {
        for decl in elidex_css::parse_declaration_block(&css) {
            hydrated.set(
                decl.property,
                crate::computed_style::css_value_to_string(&decl.value),
            );
        }
    }
    let _ = dom.world_mut().insert_one(entity, hydrated);
}

/// Round-trip the current `InlineStyle` declarations into `attrs("style")`
/// so the cascade picks them up on the next walk.  CRIT-1 mitigation:
/// `elidex_style::cascade::get_inline_declarations` reads from
/// `attrs("style")` (not `InlineStyle`), so without this sync any mutation
/// through `el.style.*` would be invisible to layout.
///
/// Empty inline-style produces an empty attribute (preserves `style=""`
/// rather than removing it) — matches Chrome's behaviour for an
/// inline-style block emptied via `removeProperty`.
fn sync_to_attribute(entity: Entity, dom: &mut EcsDom) {
    let css_text = match dom.world().get::<&InlineStyle>(entity) {
        Ok(style) => style.css_text(),
        Err(_) => return,
    };
    if dom.world_mut().get::<&Attributes>(entity).is_err() {
        let _ = dom.world_mut().insert_one(entity, Attributes::default());
    }
    if let Ok(mut attrs) = dom.world_mut().get::<&mut Attributes>(entity) {
        attrs.set("style", css_text);
    }
}

// ---------------------------------------------------------------------------
// style.setProperty
// ---------------------------------------------------------------------------

/// `element.style.setProperty(property, value)` — sets an inline style.
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

        ensure_inline_style(this, dom);

        {
            let mut style = dom
                .world_mut()
                .get::<&mut InlineStyle>(this)
                .map_err(|_| not_found_error("element not found"))?;
            style.set(property, value);
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
/// re-parsing `value` through [`elidex_css::parse_declaration_block`]
/// (which performs shorthand expansion).  All-or-nothing replacement:
/// `parse_declaration_block` returns an empty `Vec` for empty / unparseable
/// input, which clears the inline-style block.  This matches the spec
/// "parse a CSS declaration block" algorithm and Chrome's behaviour for
/// whole-block-invalid input (accepted divergence: Firefox preserves
/// existing declarations on whole-block parse failure).
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
        let declarations = elidex_css::parse_declaration_block(&css);

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
        let mut new_style = InlineStyle::default();
        for decl in declarations {
            new_style.set(
                decl.property,
                crate::computed_style::css_value_to_string(&decl.value),
            );
        }
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
        assert_eq!(result, JsValue::String("red".into()));
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
        assert_eq!(result, JsValue::String("blue".into()));

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
        assert_eq!(attrs.get("style").unwrap(), "color: red");
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

        // Stored under "color" (lowercase).
        let result = StyleGetPropertyValue
            .invoke(
                elem,
                &[JsValue::String("color".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::String("red".into()));

        // Mixed-case lookup also lowercases.
        let result = StyleGetPropertyValue
            .invoke(
                elem,
                &[JsValue::String("CoLoR".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::String("red".into()));
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

        // `parse_declaration_block` parses `color: red` into
        // `CssValue::Color(...)` which `css_value_to_string` then
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
