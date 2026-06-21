//! Unit tests for the DOMTokenList handlers in `class_list.rs`
//! (`classList` / `relList` / `linkSizes` families). Split out to keep the
//! algorithm + handler-registry file under the 1000-line review convention.
#![allow(unused_must_use)] // Test setup calls dom.append_child() etc. without checking return values

use super::*;

fn setup() -> (EcsDom, Entity, SessionCore) {
    let mut dom = EcsDom::new();
    let mut attrs = Attributes::default();
    attrs.set("class", "foo bar");
    let elem = dom.create_element("div", attrs);
    let session = SessionCore::new();
    (dom, elem, session)
}

#[test]
fn validate_token_rejects_ascii_whitespace() {
    // Spec scope: each of the 5 ASCII whitespace bytes must error.
    for ws in ["\t", "\n", "\x0c", "\r", " "] {
        let token = format!("foo{ws}bar");
        let err = validate_token(&token).unwrap_err();
        assert_eq!(err.kind, DomApiErrorKind::InvalidCharacterError);
    }
}

#[test]
fn validate_token_accepts_non_ascii_whitespace() {
    // PR178 R4 IMP regression — `char::is_whitespace` previously
    // rejected non-ASCII whitespace such as U+00A0 (NBSP), which
    // the spec considers a valid token character.
    for ch in ["\u{00A0}", "\u{2003}", "\u{3000}"] {
        let token = format!("foo{ch}bar");
        assert!(
            validate_token(&token).is_ok(),
            "token containing {ch:?} should be accepted (non-ASCII whitespace)"
        );
    }
}

#[test]
fn add_then_contains_token_with_nbsp() {
    // PR178 R5 IMP regression — every tokenisation site (`parse_ordered_set`,
    // `token_set`, the `add`/`remove`/`toggle`/`contains`/`replace`/`length`/
    // `item` ops) was using `split_whitespace` (Unicode-aware),
    // which would break `contains`/`add` for tokens containing NBSP
    // (U+00A0) and other non-ASCII whitespace.  Switched to
    // `split_ascii_whitespace` so the membership check matches the
    // ASCII-whitespace parser used at insertion time.
    let (mut dom, elem, mut session) = setup();
    let nbsp_token = "foo\u{00A0}bar";
    CLASS_LIST_ADD
        .invoke(
            elem,
            &[JsValue::String(nbsp_token.into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    let result = CLASS_LIST_CONTAINS
        .invoke(
            elem,
            &[JsValue::String(nbsp_token.into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(
        result,
        JsValue::Bool(true),
        "contains() must find an NBSP-containing token previously added"
    );
}

#[test]
fn add_new_class() {
    let (mut dom, elem, mut session) = setup();
    CLASS_LIST_ADD
        .invoke(
            elem,
            &[JsValue::String("baz".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    let attrs = dom.world().get::<&Attributes>(elem).unwrap();
    let classes: Vec<&str> = attrs
        .get("class")
        .unwrap()
        .split_ascii_whitespace()
        .collect();
    assert!(classes.contains(&"baz"));
    assert!(classes.contains(&"foo"));
}

#[test]
fn add_existing_class_noop() {
    let (mut dom, elem, mut session) = setup();
    CLASS_LIST_ADD
        .invoke(
            elem,
            &[JsValue::String("foo".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    let attrs = dom.world().get::<&Attributes>(elem).unwrap();
    let count = attrs
        .get("class")
        .unwrap()
        .split_ascii_whitespace()
        .filter(|c| *c == "foo")
        .count();
    assert_eq!(count, 1);
}

#[test]
fn remove_class() {
    let (mut dom, elem, mut session) = setup();
    CLASS_LIST_REMOVE
        .invoke(
            elem,
            &[JsValue::String("foo".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    let attrs = dom.world().get::<&Attributes>(elem).unwrap();
    assert!(!attrs
        .get("class")
        .unwrap()
        .split_ascii_whitespace()
        .any(|c| c == "foo"));
}

#[test]
fn toggle_adds_when_absent() {
    let (mut dom, elem, mut session) = setup();
    let result = CLASS_LIST_TOGGLE
        .invoke(
            elem,
            &[JsValue::String("baz".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(result, JsValue::Bool(true));
}

#[test]
fn toggle_removes_when_present() {
    let (mut dom, elem, mut session) = setup();
    let result = CLASS_LIST_TOGGLE
        .invoke(
            elem,
            &[JsValue::String("foo".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(result, JsValue::Bool(false));
}

#[test]
fn contains_true() {
    let (mut dom, elem, mut session) = setup();
    let result = CLASS_LIST_CONTAINS
        .invoke(
            elem,
            &[JsValue::String("foo".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(result, JsValue::Bool(true));
}

#[test]
fn contains_false() {
    let (mut dom, elem, mut session) = setup();
    let result = CLASS_LIST_CONTAINS
        .invoke(
            elem,
            &[JsValue::String("missing".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(result, JsValue::Bool(false));
}

#[test]
fn add_rejects_empty_token() {
    let (mut dom, elem, mut session) = setup();
    let err = CLASS_LIST_ADD
        .invoke(
            elem,
            &[JsValue::String(String::new())],
            &mut session,
            &mut dom,
        )
        .unwrap_err();
    assert_eq!(err.kind, DomApiErrorKind::SyntaxError);
}

#[test]
fn add_rejects_whitespace_token() {
    let (mut dom, elem, mut session) = setup();
    let err = CLASS_LIST_ADD
        .invoke(
            elem,
            &[JsValue::String("a b".into())],
            &mut session,
            &mut dom,
        )
        .unwrap_err();
    assert_eq!(err.kind, DomApiErrorKind::InvalidCharacterError);
}

#[test]
fn add_normalizes_whitespace() {
    let mut dom = EcsDom::new();
    let mut attrs = Attributes::default();
    attrs.set("class", "  foo  bar  ");
    let elem = dom.create_element("div", attrs);
    let mut session = SessionCore::new();
    CLASS_LIST_ADD
        .invoke(
            elem,
            &[JsValue::String("baz".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    let attrs = dom.world().get::<&Attributes>(elem).unwrap();
    assert_eq!(attrs.get("class").unwrap(), "foo bar baz");
}

// -----------------------------------------------------------------------
// toggle with force parameter
// -----------------------------------------------------------------------

#[test]
fn toggle_force_true_adds() {
    let (mut dom, elem, mut session) = setup();
    let result = CLASS_LIST_TOGGLE
        .invoke(
            elem,
            &[JsValue::String("baz".into()), JsValue::Bool(true)],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(result, JsValue::Bool(true));
    let attrs = dom.world().get::<&Attributes>(elem).unwrap();
    assert!(attrs
        .get("class")
        .unwrap()
        .split_ascii_whitespace()
        .any(|c| c == "baz"));
}

#[test]
fn toggle_force_true_keeps_existing() {
    let (mut dom, elem, mut session) = setup();
    let result = CLASS_LIST_TOGGLE
        .invoke(
            elem,
            &[JsValue::String("foo".into()), JsValue::Bool(true)],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(result, JsValue::Bool(true));
    let attrs = dom.world().get::<&Attributes>(elem).unwrap();
    assert!(attrs
        .get("class")
        .unwrap()
        .split_ascii_whitespace()
        .any(|c| c == "foo"));
}

#[test]
fn toggle_force_false_removes() {
    let (mut dom, elem, mut session) = setup();
    let result = CLASS_LIST_TOGGLE
        .invoke(
            elem,
            &[JsValue::String("foo".into()), JsValue::Bool(false)],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(result, JsValue::Bool(false));
    let attrs = dom.world().get::<&Attributes>(elem).unwrap();
    assert!(!attrs
        .get("class")
        .unwrap()
        .split_ascii_whitespace()
        .any(|c| c == "foo"));
}

#[test]
fn toggle_force_false_noop_when_absent() {
    let (mut dom, elem, mut session) = setup();
    let result = CLASS_LIST_TOGGLE
        .invoke(
            elem,
            &[JsValue::String("baz".into()), JsValue::Bool(false)],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(result, JsValue::Bool(false));
}

// -----------------------------------------------------------------------
// classList.replace
// -----------------------------------------------------------------------

#[test]
fn replace_existing_class() {
    let (mut dom, elem, mut session) = setup();
    let result = CLASS_LIST_REPLACE
        .invoke(
            elem,
            &[JsValue::String("foo".into()), JsValue::String("baz".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(result, JsValue::Bool(true));
    let attrs = dom.world().get::<&Attributes>(elem).unwrap();
    let classes: Vec<&str> = attrs
        .get("class")
        .unwrap()
        .split_ascii_whitespace()
        .collect();
    // "baz" should be in the position of "foo" (first).
    assert_eq!(classes, vec!["baz", "bar"]);
}

#[test]
fn replace_missing_class() {
    let (mut dom, elem, mut session) = setup();
    let result = CLASS_LIST_REPLACE
        .invoke(
            elem,
            &[
                JsValue::String("missing".into()),
                JsValue::String("baz".into()),
            ],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(result, JsValue::Bool(false));
    // Class string unchanged.
    let attrs = dom.world().get::<&Attributes>(elem).unwrap();
    let classes: Vec<&str> = attrs
        .get("class")
        .unwrap()
        .split_ascii_whitespace()
        .collect();
    assert!(classes.contains(&"foo"));
    assert!(classes.contains(&"bar"));
}

#[test]
fn replace_rejects_invalid_token() {
    let (mut dom, elem, mut session) = setup();
    let err = CLASS_LIST_REPLACE
        .invoke(
            elem,
            &[
                JsValue::String(String::new()),
                JsValue::String("baz".into()),
            ],
            &mut session,
            &mut dom,
        )
        .unwrap_err();
    assert_eq!(err.kind, DomApiErrorKind::SyntaxError);
}

// -----------------------------------------------------------------------
// classList.value getter/setter
// -----------------------------------------------------------------------

#[test]
fn value_get() {
    let (mut dom, elem, mut session) = setup();
    let result = CLASS_LIST_VALUE_GET
        .invoke(elem, &[], &mut session, &mut dom)
        .unwrap();
    assert_eq!(result, JsValue::String("foo bar".into()));
}

#[test]
fn value_set() {
    let (mut dom, elem, mut session) = setup();
    CLASS_LIST_VALUE_SET
        .invoke(
            elem,
            &[JsValue::String("a b c".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    let result = CLASS_LIST_VALUE_GET
        .invoke(elem, &[], &mut session, &mut dom)
        .unwrap();
    assert_eq!(result, JsValue::String("a b c".into()));
}

// -----------------------------------------------------------------------
// classList.length
// -----------------------------------------------------------------------

#[test]
fn length() {
    let (mut dom, elem, mut session) = setup();
    let result = CLASS_LIST_LENGTH
        .invoke(elem, &[], &mut session, &mut dom)
        .unwrap();
    assert_eq!(result, JsValue::Number(2.0));
}

#[test]
fn length_empty() {
    let mut dom = EcsDom::new();
    let elem = dom.create_element("div", Attributes::default());
    let mut session = SessionCore::new();
    let result = CLASS_LIST_LENGTH
        .invoke(elem, &[], &mut session, &mut dom)
        .unwrap();
    assert_eq!(result, JsValue::Number(0.0));
}

// -----------------------------------------------------------------------
// classList.item
// -----------------------------------------------------------------------

#[test]
fn item_valid_index() {
    let (mut dom, elem, mut session) = setup();
    let result = CLASS_LIST_ITEM
        .invoke(elem, &[JsValue::Number(0.0)], &mut session, &mut dom)
        .unwrap();
    assert_eq!(result, JsValue::String("foo".into()));

    let result = CLASS_LIST_ITEM
        .invoke(elem, &[JsValue::Number(1.0)], &mut session, &mut dom)
        .unwrap();
    assert_eq!(result, JsValue::String("bar".into()));
}

#[test]
fn item_out_of_bounds() {
    let (mut dom, elem, mut session) = setup();
    let result = CLASS_LIST_ITEM
        .invoke(elem, &[JsValue::Number(5.0)], &mut session, &mut dom)
        .unwrap();
    assert_eq!(result, JsValue::Null);
}

// -----------------------------------------------------------------------
// classList.supports
// -----------------------------------------------------------------------

#[test]
fn supports_throws() {
    let (mut dom, elem, mut session) = setup();
    let err = CLASS_LIST_SUPPORTS
        .invoke(
            elem,
            &[JsValue::String("foo".into())],
            &mut session,
            &mut dom,
        )
        .unwrap_err();
    assert_eq!(err.kind, DomApiErrorKind::TypeError);
}

// -----------------------------------------------------------------------
// relList.supports — DOM §7.1 supported tokens (HTML §4.2.4 / §4.6.2)
// -----------------------------------------------------------------------

fn supports_on(tag: &str, token: &str) -> Result<JsValue, DomApiError> {
    let mut dom = EcsDom::new();
    let mut session = SessionCore::new();
    let elem = dom.create_element(tag, Attributes::default());
    REL_LIST_SUPPORTS.invoke(
        elem,
        &[JsValue::String(token.into())],
        &mut session,
        &mut dom,
    )
}

#[test]
fn rellist_supports_link_implemented_tokens() {
    // `supports()` reflects the UA-*implemented* rel processing models, not the
    // full spec enumeration (HTML §4.2.4: possible keywords ∩ implemented).
    // elidex fully implements only `stylesheet` (CSS load + cascade).
    assert_eq!(
        supports_on("link", "stylesheet").unwrap(),
        JsValue::Bool(true)
    );
    // Possible `<link>` keywords with no end-to-end processing model → false
    // (advertising them would make feature detection lie). `manifest` is
    // discovered but not fetched/parsed/applied, so it is NOT advertised.
    for unimpl in [
        "manifest",
        "preload",
        "modulepreload",
        "preconnect",
        "icon",
        "bogus",
    ] {
        assert_eq!(
            supports_on("link", unimpl).unwrap(),
            JsValue::Bool(false),
            "link.relList.supports({unimpl:?}) must be false (no processing model)"
        );
    }
    // `noopener` is a hyperlink keyword, never a `<link>` one.
    assert_eq!(
        supports_on("link", "noopener").unwrap(),
        JsValue::Bool(false)
    );
}

#[test]
fn rellist_supports_is_ascii_case_insensitive() {
    // DOM §7.1 validation steps compare tokens ASCII case-insensitively.
    assert_eq!(
        supports_on("link", "StyleSheet").unwrap(),
        JsValue::Bool(true)
    );
    assert_eq!(
        supports_on("link", "STYLESHEET").unwrap(),
        JsValue::Bool(true)
    );
}

#[test]
fn rellist_supports_foreign_namespace_throws() {
    // Supported tokens are defined for HTML `link`/`a`/`area` only; a foreign
    // element with the same local name (e.g. an SVG `<a>`) defines none → throws
    // (DOM §7.1), not a tag-name-only match against the HTML set.
    let mut dom = EcsDom::new();
    let mut session = SessionCore::new();
    let svg_a = dom.create_element_ns("a", elidex_ecs::Namespace::Svg, Attributes::default(), None);
    let err = REL_LIST_SUPPORTS
        .invoke(
            svg_a,
            &[JsValue::String("noopener".into())],
            &mut session,
            &mut dom,
        )
        .unwrap_err();
    assert_eq!(err.kind, DomApiErrorKind::TypeError);
}

#[test]
fn rellist_supports_hyperlink_returns_false_not_throw() {
    // `<a>` / `<area>` `rel` *defines* supported tokens (noopener/noreferrer/
    // opener, HTML §4.6.2), so `supports()` does NOT throw — but elidex
    // implements none of those processing models, so the implemented subset is
    // empty and every token returns `false`.
    for tag in ["a", "area"] {
        for token in ["noopener", "noreferrer", "opener", "stylesheet"] {
            assert_eq!(
                supports_on(tag, token).unwrap(),
                JsValue::Bool(false),
                "{tag}.relList.supports({token:?}) must be false (not implemented), not throw"
            );
        }
    }
}

#[test]
fn rellist_supports_unknown_element_throws() {
    // A `rel` token list on an element whose `rel` defines no supported tokens
    // (e.g. a `<div>`) → `supports()` throws (DOM §7.1).
    let err = supports_on("div", "stylesheet").unwrap_err();
    assert_eq!(err.kind, DomApiErrorKind::TypeError);
}

#[test]
fn linksizes_supports_throws() {
    // `<link>.sizes` defines no supported tokens → throws, even on a `<link>`.
    let mut dom = EcsDom::new();
    let mut session = SessionCore::new();
    let elem = dom.create_element("link", Attributes::default());
    let err = LINK_SIZES_SUPPORTS
        .invoke(
            elem,
            &[JsValue::String("any".into())],
            &mut session,
            &mut dom,
        )
        .unwrap_err();
    assert_eq!(err.kind, DomApiErrorKind::TypeError);
}

// -----------------------------------------------------------------------
// Step 3 spec-compliance tests
// -----------------------------------------------------------------------

#[test]
fn validate_token_whitespace_is_invalid_character_error() {
    let (mut dom, elem, mut session) = setup();
    let err = CLASS_LIST_ADD
        .invoke(
            elem,
            &[JsValue::String("a b".into())],
            &mut session,
            &mut dom,
        )
        .unwrap_err();
    assert_eq!(err.kind, DomApiErrorKind::InvalidCharacterError);
}

#[test]
fn validate_token_empty_is_syntax_error() {
    let (mut dom, elem, mut session) = setup();
    let err = CLASS_LIST_ADD
        .invoke(
            elem,
            &[JsValue::String(String::new())],
            &mut session,
            &mut dom,
        )
        .unwrap_err();
    assert_eq!(err.kind, DomApiErrorKind::SyntaxError);
}

#[test]
fn contains_no_validate_empty() {
    let (mut dom, elem, mut session) = setup();
    let result = CLASS_LIST_CONTAINS
        .invoke(
            elem,
            &[JsValue::String(String::new())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(result, JsValue::Bool(false));
}

#[test]
fn contains_no_validate_whitespace() {
    let (mut dom, elem, mut session) = setup();
    let result = CLASS_LIST_CONTAINS
        .invoke(
            elem,
            &[JsValue::String("a b".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(result, JsValue::Bool(false));
}

#[test]
fn length_dedup() {
    let mut dom = EcsDom::new();
    let mut attrs = Attributes::default();
    attrs.set("class", "foo bar foo");
    let elem = dom.create_element("div", attrs);
    let mut session = SessionCore::new();
    let result = CLASS_LIST_LENGTH
        .invoke(elem, &[], &mut session, &mut dom)
        .unwrap();
    assert_eq!(result, JsValue::Number(2.0)); // foo, bar (dedup)
}

#[test]
fn item_dedup() {
    let mut dom = EcsDom::new();
    let mut attrs = Attributes::default();
    attrs.set("class", "foo bar foo");
    let elem = dom.create_element("div", attrs);
    let mut session = SessionCore::new();
    let result = CLASS_LIST_ITEM
        .invoke(elem, &[JsValue::Number(1.0)], &mut session, &mut dom)
        .unwrap();
    assert_eq!(result, JsValue::String("bar".into()));
}

#[test]
fn replace_existing_new_token() {
    let mut dom = EcsDom::new();
    let mut attrs = Attributes::default();
    attrs.set("class", "foo bar baz");
    let elem = dom.create_element("div", attrs);
    let mut session = SessionCore::new();
    // Replace "foo" with "bar" — Infra ordered set "replace":
    // "foo" at index 0 becomes "bar", then existing "bar" at index 1 is removed.
    let result = CLASS_LIST_REPLACE
        .invoke(
            elem,
            &[JsValue::String("foo".into()), JsValue::String("bar".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(result, JsValue::Bool(true));
    let attrs = dom.world().get::<&Attributes>(elem).unwrap();
    let classes: Vec<&str> = attrs
        .get("class")
        .unwrap()
        .split_ascii_whitespace()
        .collect();
    assert_eq!(classes, vec!["bar", "baz"]);
}

#[test]
fn replace_infra_ordered_set_position() {
    let mut dom = EcsDom::new();
    let mut attrs = Attributes::default();
    attrs.set("class", "x foo y bar z");
    let elem = dom.create_element("div", attrs);
    let mut session = SessionCore::new();
    // Infra "replace": foo→bar at position 1, remove existing bar at position 3.
    let result = CLASS_LIST_REPLACE
        .invoke(
            elem,
            &[JsValue::String("foo".into()), JsValue::String("bar".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(result, JsValue::Bool(true));
    let attrs = dom.world().get::<&Attributes>(elem).unwrap();
    let classes: Vec<&str> = attrs
        .get("class")
        .unwrap()
        .split_ascii_whitespace()
        .collect();
    assert_eq!(classes, vec!["x", "bar", "y", "z"]);
}

#[test]
fn replace_infra_position_when_new_precedes_old() {
    // Infra "replace within an ordered set": `new` lands at the first instance
    // of *either* `old` or `new`. When `new` precedes `old`, that position is
    // `new`'s — `replace("foo","bar")` on `« bar x foo »` gives `« bar x »`,
    // NOT `« x bar »` (the latter would be replacing only at `old`'s slot).
    let mut dom = EcsDom::new();
    let mut attrs = Attributes::default();
    attrs.set("class", "bar x foo");
    let elem = dom.create_element("div", attrs);
    let mut session = SessionCore::new();
    let result = CLASS_LIST_REPLACE
        .invoke(
            elem,
            &[JsValue::String("foo".into()), JsValue::String("bar".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(result, JsValue::Bool(true));
    let attrs = dom.world().get::<&Attributes>(elem).unwrap();
    assert_eq!(attrs.get("class"), Some("bar x"));
}

#[test]
fn supports_throws_type_error() {
    let (mut dom, elem, mut session) = setup();
    let err = CLASS_LIST_SUPPORTS
        .invoke(
            elem,
            &[JsValue::String("foo".into())],
            &mut session,
            &mut dom,
        )
        .unwrap_err();
    assert_eq!(err.kind, DomApiErrorKind::TypeError);
}

/// DOMTokenList writes route through the `EcsDom::set_attribute`
/// chokepoint, so `classList.add` / `value=` dispatch
/// `MutationEvent::AttributeChange` (slot
/// `#11-attr-handler-chokepoint-mutationevent`). The prior `set_token_string`
/// wrote `Attributes` directly + bumped `rev_version`, dropping the event.
#[test]
fn classlist_add_and_value_set_dispatch_mutation_event() {
    use crate::test_util::AttrChangeCounter;
    let (mut dom, elem, mut session) = setup();
    let hook = AttrChangeCounter::default();
    let count = hook.count.clone();
    dom.set_mutation_dispatcher(Box::new(hook));

    // Adding a new token writes the `class` attribute → one record.
    CLASS_LIST_ADD
        .invoke(
            elem,
            &[JsValue::String("baz".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    // `classList.value = …` writes the attribute → one record.
    CLASS_LIST_VALUE_SET
        .invoke(
            elem,
            &[JsValue::String("a b".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(
            *count.lock().unwrap(),
            2,
            "classList.add + classList.value= must each route through the chokepoint and dispatch one AttributeChange"
        );
}

/// DOMTokenList update steps (DOM §7.1 `#concept-dtl-update`) step 1
/// (Codex PR341 R1): a method (`remove` is the reachable case) that nets no
/// change on an attribute-less element is a no-op — it must NOT create an
/// empty backing attribute or dispatch `AttributeChange`. Routing through
/// the chokepoint made the spurious record observable, which this guards.
#[test]
fn remove_on_absent_attribute_is_noop() {
    use crate::test_util::AttrChangeCounter;

    let mut dom = EcsDom::new();
    let el = dom.create_element("div", Attributes::default());
    let mut session = SessionCore::new();
    let hook = AttrChangeCounter::default();
    let count = hook.count.clone();
    dom.set_mutation_dispatcher(Box::new(hook));

    CLASS_LIST_REMOVE
        .invoke(el, &[JsValue::String("x".into())], &mut session, &mut dom)
        .unwrap();
    assert!(
        dom.world()
            .get::<&Attributes>(el)
            .unwrap()
            .get("class")
            .is_none(),
        "remove on a class-less element must not create an empty class attribute"
    );
    assert_eq!(
        *count.lock().unwrap(),
        0,
        "remove on an absent attribute must be a no-op (no AttributeChange)"
    );

    // Contrast: removing the last token from a *present* attribute still
    // writes `class=\"\"` (step 1 only returns when the attribute is absent).
    dom.set_attribute(el, "class", "x");
    let baseline = *count.lock().unwrap();
    CLASS_LIST_REMOVE
        .invoke(el, &[JsValue::String("x".into())], &mut session, &mut dom)
        .unwrap();
    assert_eq!(
        dom.world().get::<&Attributes>(el).unwrap().get("class"),
        Some(""),
        "removing the last token from a present attribute leaves class=\"\""
    );
    assert_eq!(
        *count.lock().unwrap(),
        baseline + 1,
        "removing the last token from a present attribute still dispatches one AttributeChange"
    );
}

/// DOM §7.1 `add(tokens…)` / `remove(tokens…)` are variadic and run the
/// update steps ONCE for the whole call (Codex PR341 R2): a multi-token
/// `classList.add("a", "b")` dispatches exactly one `AttributeChange`, not
/// one per token. The VM native forwards all tokens to this handler in a
/// single call; validation of all tokens precedes any mutation (atomic).
#[test]
fn variadic_add_remove_run_update_steps_once() {
    use crate::test_util::AttrChangeCounter;

    let mut dom = EcsDom::new();
    let el = dom.create_element("div", Attributes::default());
    let mut session = SessionCore::new();
    let hook = AttrChangeCounter::default();
    let count = hook.count.clone();
    dom.set_mutation_dispatcher(Box::new(hook));

    CLASS_LIST_ADD
        .invoke(
            el,
            &[JsValue::String("a".into()), JsValue::String("b".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(
        dom.world().get::<&Attributes>(el).unwrap().get("class"),
        Some("a b"),
        "both tokens appended in a single update"
    );
    assert_eq!(
        *count.lock().unwrap(),
        1,
        "variadic add must dispatch exactly one AttributeChange"
    );

    CLASS_LIST_REMOVE
        .invoke(
            el,
            &[JsValue::String("a".into()), JsValue::String("b".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(
        *count.lock().unwrap(),
        2,
        "variadic remove must dispatch exactly one more AttributeChange"
    );

    // Validate-all-before-mutate: an invalid token aborts the whole call
    // (DOM §7.1 step 1 runs over every token first), leaving the attribute
    // untouched — no partial write of the valid prefix.
    let fresh = dom.create_element("div", Attributes::default());
    CLASS_LIST_ADD
        .invoke(
            fresh,
            &[JsValue::String("ok".into()), JsValue::String(String::new())],
            &mut session,
            &mut dom,
        )
        .unwrap_err();
    assert!(
        dom.world()
            .get::<&Attributes>(fresh)
            .unwrap()
            .get("class")
            .is_none(),
        "an invalid token must abort the whole variadic add (no partial write)"
    );
}
