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

/// Codex #335 R6 F19: `item(unsigned long index)` applies WebIDL
/// ToUint32, so a non-finite argument maps to index 0 (not out-of-range).
/// This exercises the engine-independent handler directly — the VM host
/// pre-coerces (`to_uint32`), so only the handler-level coercion catches
/// `NaN`/`Infinity` for callers (boa) that forward the raw number.
#[test]
fn style_item_webidl_coerces_non_finite_index() {
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

    // ToUint32(NaN) == 0 → first property.
    let nan = StyleItem
        .invoke(elem, &[JsValue::Number(f64::NAN)], &mut session, &mut dom)
        .unwrap();
    assert_eq!(nan, JsValue::String("color".into()));

    // ToUint32(+Infinity) == 0 → first property.
    let inf = StyleItem
        .invoke(
            elem,
            &[JsValue::Number(f64::INFINITY)],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(inf, JsValue::String("color".into()));
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
