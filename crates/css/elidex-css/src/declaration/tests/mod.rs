use super::*;
use elidex_plugin::{CssColor, LengthUnit};

mod grid_table;
mod shorthand;
mod values;

fn parse_decls(css: &str) -> Vec<Declaration> {
    parse_declaration_block(css)
}

fn parse_single(property: &str, value: &str) -> Vec<Declaration> {
    parse_decls(&format!("{property}: {value}"))
}

// --- parse_inline_style (canonical attribute→InlineStyle derivation) ---

#[test]
fn inline_style_basic_properties() {
    let style = parse_inline_style("display: block; width: 200px", None);
    assert_eq!(style.get("display"), Some("block"));
    assert_eq!(style.get("width"), Some("200px"));
}

#[test]
fn inline_style_color_keyword_canonicalizes_to_hex() {
    // Values store in post-parse canonical form: `red` → `#ff0000`
    // (same accepted divergence as the CSSOM cssText round-trip).
    let style = parse_inline_style("color: red", None);
    assert_eq!(style.get("color"), Some("#ff0000"));
}

#[test]
fn inline_style_shorthand_expands_to_longhands() {
    let style = parse_inline_style("margin: 10px", None);
    assert_eq!(style.get("margin"), None);
    assert_eq!(style.get("margin-top"), Some("10px"));
    assert_eq!(style.get("margin-bottom"), Some("10px"));
}

#[test]
fn inline_style_value_with_semicolon_inside_function_not_split() {
    // The motivating divergence for the canonical fn: a naive `;`/`:`
    // splitter shreds function values containing `;` or `:` (data URLs).
    // The real parser must not fabricate properties out of the URL body.
    let style = parse_inline_style(
        "background: url(data:image/png;base64,iVBO); color: blue",
        None,
    );
    assert_eq!(style.get("color"), Some("#0000ff"));
    // No garbage keys leaked from inside the url() token.
    assert!(style.get("base64,iVBO)").is_none());
    for i in 0..style.len() {
        let prop = style.property_at(i).unwrap();
        assert!(
            !prop.contains("base64") && !prop.contains("url"),
            "naive-split artifact leaked into InlineStyle: {prop}"
        );
    }
}

#[test]
fn inline_style_unknown_property_dropped() {
    let style = parse_inline_style("not-a-real-property: 12px; display: flex", None);
    assert_eq!(style.get("not-a-real-property"), None);
    assert_eq!(style.get("display"), Some("flex"));
}

#[test]
fn inline_style_calc_round_trips() {
    let style = parse_inline_style("width: calc(100% - 10px)", None);
    assert_eq!(style.get("width"), Some("calc(100% - 10px)"));
    // The serialized form must re-parse to the same value.
    let reparsed = parse_inline_style(&style.css_text(), None);
    assert_eq!(reparsed.get("width"), Some("calc(100% - 10px)"));
}

#[test]
fn inline_style_custom_property_case_preserved() {
    let style = parse_inline_style("--MyVar: 10px", None);
    assert_eq!(style.get("--MyVar"), Some("10px"));
}

#[test]
fn inline_style_empty_and_garbage_input() {
    assert!(parse_inline_style("", None).is_empty());
    assert!(parse_inline_style("garbage", None).is_empty());
    assert!(parse_inline_style(";;;", None).is_empty());
}

#[test]
fn inline_style_important_preserved_and_reemitted() {
    // The cascade reads importance from the style attribute, which
    // `sync_to_attribute` rewrites from `css_text()` — a
    // priority-stripping derivation would silently demote `!important`
    // on the first unrelated style write.
    let style = parse_inline_style("color: red !important; width: 10px", None);
    assert!(style.is_important("color"));
    assert!(!style.is_important("width"));
    assert_eq!(style.get("color"), Some("#ff0000"));

    let text = style.css_text();
    assert_eq!(text, "color: #ff0000 !important; width: 10px");

    // Full round-trip: the re-parse sees the same priority.
    let reparsed = parse_inline_style(&text, None);
    assert!(reparsed.is_important("color"));
    assert!(!reparsed.is_important("width"));
}

#[test]
fn inline_style_important_on_shorthand_expands_to_longhands() {
    let style = parse_inline_style("margin: 10px !important", None);
    assert!(style.is_important("margin-top"));
    assert!(style.is_important("margin-left"));
}

#[test]
fn inline_style_important_wins_over_later_normal_duplicate() {
    // Codex #335 R8 F24 / CSS Cascade 4 §6.3: collapsing duplicate
    // declarations to a single InlineStyle entry must keep the
    // cascade-winning important value — a later *normal* duplicate must
    // not overwrite an earlier *important* one.
    let style = parse_inline_style("color: red !important; color: blue", None);
    assert_eq!(style.get("color"), Some("#ff0000"));
    assert!(style.is_important("color"));

    // A later *important* duplicate does win (last-important-wins).
    let style2 = parse_inline_style("color: red !important; color: blue !important", None);
    assert_eq!(style2.get("color"), Some("#0000ff"));
    assert!(style2.is_important("color"));

    // normal-then-important: important wins and is flagged.
    let style3 = parse_inline_style("color: red; color: blue !important", None);
    assert_eq!(style3.get("color"), Some("#0000ff"));
    assert!(style3.is_important("color"));

    // normal-then-normal: ordinary last-wins.
    let style4 = parse_inline_style("color: red; color: blue", None);
    assert_eq!(style4.get("color"), Some("#0000ff"));
    assert!(!style4.is_important("color"));
}

// --- parse_value_for_property (CSSOM §6.6.1 setProperty value parse) ---

#[test]
fn value_for_property_basic_and_shorthand() {
    let decls = parse_value_for_property("color", "red", None).expect("valid value");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "color");

    let decls = parse_value_for_property("margin", "10px", None).expect("shorthand value");
    assert_eq!(decls.len(), 4);
    assert!(decls.iter().any(|d| d.property == "margin-top"));
}

#[test]
fn value_for_property_rejects_unsupported_and_unparseable() {
    assert!(parse_value_for_property("not-a-property", "12px", None).is_none());
    assert!(parse_value_for_property("color", "12px", None).is_none());
}

#[test]
fn value_for_property_rejects_trailing_input() {
    // Declaration injection: a `;` would fabricate a second declaration
    // when the serialized block is re-parsed by the cascade.
    assert!(parse_value_for_property("color", "red; background: url(//evil)", None).is_none());
    // §6.6.1 note: value cannot include `!important` — priority is a
    // separate argument.
    assert!(parse_value_for_property("color", "red !important", None).is_none());
}

// --- serialize_declaration_value_for_storage (Codex #335 R6 F18) ---

#[test]
fn storage_serialization_roundtrips_space_separated_list() {
    // `text-decoration-line: underline overline` parses to a
    // `CssValue::List`; `to_css_string` comma-joins it, and the comma form
    // does NOT re-parse (the declaration would vanish from the cascade).
    // The storage serializer must fall back to the space-joined,
    // round-trip-safe form.
    let decls =
        parse_value_for_property("text-decoration-line", "underline overline", None).unwrap();
    assert_eq!(decls.len(), 1);
    let stored =
        serialize_declaration_value_for_storage("text-decoration-line", &decls[0].value, None);
    assert_eq!(stored, "underline overline");
    // Round-trips: re-parsing the stored string yields the same value.
    let reparsed = parse_value_for_property("text-decoration-line", &stored, None).unwrap();
    assert_eq!(reparsed[0].value, decls[0].value);
}

#[test]
fn storage_serialization_keeps_canonical_for_roundtrippable_value() {
    // A non-list value keeps its canonical serialization (colour keyword
    // → hex) — the space-fallback only triggers for a lossy list.
    let decls = parse_value_for_property("color", "red", None).unwrap();
    let stored = serialize_declaration_value_for_storage("color", &decls[0].value, None);
    assert_eq!(stored, "#ff0000");
}

#[test]
fn parse_inline_style_preserves_space_separated_list() {
    // End-to-end: hydrating from a `style` attribute must not corrupt a
    // space-separated list property on the round-trip into `InlineStyle`.
    let style = parse_inline_style("text-decoration-line: underline overline", None);
    assert_eq!(
        style.get("text-decoration-line"),
        Some("underline overline")
    );
}

#[test]
fn value_for_property_rejects_invalid_custom_property_name() {
    // Codex #335 R7 F23: a name that merely starts with `--` but carries
    // CSS delimiters is not a valid custom-property name (CSS Syntax 3 —
    // a `--` dashed-ident is a single ident token). Accepting it would let
    // `css_text()` write `--x;color: red`, which the cascade re-parse
    // splits to inject `color: red`.
    assert!(parse_value_for_property("--x;color", "red", None).is_none());
    assert!(parse_value_for_property("--a:b", "red", None).is_none());
    assert!(parse_value_for_property("--a b", "red", None).is_none());
    // A plain dashed-ident is valid.
    assert!(parse_value_for_property("--valid", "red", None).is_some());
    // An escaped delimiter is a single ident token → valid (round-trips
    // with the escape intact).
    assert!(parse_value_for_property("--a\\;b", "red", None).is_some());
}

#[test]
fn value_for_property_custom_property() {
    let decls = parse_value_for_property("--x", "calc(1px + 2px)", None).expect("raw tokens");
    assert_eq!(decls[0].property, "--x");
    // Top-level `;` is not a <declaration-value> (CSS Syntax) — reject;
    // `;` inside a nested block is fine.
    assert!(parse_value_for_property("--x", "a; b", None).is_none());
    assert!(parse_value_for_property("--x", "var(--y, a)", None).is_some());
    assert!(parse_value_for_property("--x", "", None).is_none());
}

#[test]
fn value_for_property_custom_property_declaration_value_exclusions() {
    // <declaration-value> also excludes top-level `!` (priority
    // fabrication), unmatched close brackets, and bad-string/bad-url.
    assert!(parse_value_for_property("--x", "red !important", None).is_none());
    assert!(parse_value_for_property("--x", "a ) b", None).is_none());
    assert!(parse_value_for_property("--x", "a ] b", None).is_none());
    assert!(parse_value_for_property("--x", "a } b", None).is_none());
    // The same characters nested inside a function are legal.
    assert!(parse_value_for_property("--x", "url(\"a;b\")", None).is_some());
}

#[test]
fn shorthand_longhands_mapping() {
    assert_eq!(
        shorthand_longhands("margin"),
        vec!["margin-top", "margin-right", "margin-bottom", "margin-left"]
    );
    assert!(shorthand_longhands("color").is_empty());
    assert!(shorthand_longhands("--x").is_empty());
}

#[test]
fn value_for_property_custom_property_nested_bad_tokens_rejected() {
    // <declaration-value> excludes bad-string / bad-url / unmatched
    // closers at ANY nesting level (only `;` / `!` are top-level-only).
    assert!(parse_value_for_property("--x", "f(\"a\nb\")", None).is_none());
    assert!(parse_value_for_property("--x", "f( ] )", None).is_none());
    // Nested `;` / `!` remain legitimate.
    assert!(parse_value_for_property("--x", "f(a;b)", None).is_some());
    assert!(parse_value_for_property("--x", "f(a!b)", None).is_some());
}

#[test]
fn shorthand_longhands_covers_every_multi_longhand_value_parse() {
    // Invariant: every shorthand the value parser expands must have a
    // `shorthand_longhands` entry covering its expansion — otherwise
    // CSSOM removeProperty leaves longhand residue (re-check finding:
    // `overflow` / `border-radius` were missing).
    let samples = [
        ("margin", "10px"),
        ("padding", "10px"),
        ("border", "1px solid red"),
        ("border-top", "1px solid red"),
        ("background", "red"),
        ("font", "16px serif"),
        ("flex", "1"),
        ("flex-flow", "row wrap"),
        ("gap", "4px"),
        ("text-decoration", "underline"),
        ("overflow", "hidden"),
        ("border-radius", "4px"),
        ("columns", "2"),
        ("column-rule", "1px solid red"),
        ("grid-column", "1 / 2"),
        ("grid-row", "1 / 2"),
    ];
    for (shorthand, value) in samples {
        let decls = parse_value_for_property(shorthand, value, None)
            .unwrap_or_else(|| panic!("{shorthand}: {value} should parse"));
        if decls.len() > 1 || decls.iter().any(|d| d.property != shorthand) {
            let map = shorthand_longhands(shorthand);
            for d in &decls {
                assert!(
                    map.contains(&d.property),
                    "{shorthand} expands to {} but shorthand_longhands lacks it",
                    d.property
                );
            }
        }
    }
}

// --- Codex R1: <declaration-value> scoping in declaration blocks ---

#[test]
fn custom_property_does_not_swallow_following_declarations() {
    // Codex R1-F1: the custom-property raw collection must stop at the
    // top-level `;` — an unscoped collector consumed the rest of the
    // block into `--x` and lost `color`.
    let decls = parse_decls("--x: 1; color: red");
    assert_eq!(decls.len(), 2);
    assert_eq!(decls[0].property, "--x");
    assert_eq!(decls[0].value, CssValue::RawTokens("1".into()));
    assert_eq!(decls[1].property, "color");

    let style = parse_inline_style("--x: 1; color: red", None);
    assert_eq!(style.len(), 2);
    assert_eq!(style.get("color"), Some("#ff0000"));
}

#[test]
fn custom_property_important_flag_parsed() {
    // Stopping before the top-level `!` lets the block parser's
    // `!important` pass see the suffix instead of swallowing it.
    let decls = parse_decls("--x: foo !important");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::RawTokens("foo".into()));
    assert!(decls[0].important);
}

#[test]
fn var_value_does_not_swallow_following_declarations() {
    // Codex R1-F4 (block half): the var()-carrying raw fallback must
    // also stop at the top-level `;` / `!`.
    let decls = parse_decls("width: var(--x); color: red");
    assert_eq!(decls.len(), 2);
    assert_eq!(decls[1].property, "color");

    let decls = parse_decls("width: calc(var(--x) + 1px) !important");
    assert_eq!(decls.len(), 1);
    assert!(decls[0].important);
    assert_eq!(
        decls[0].value,
        CssValue::RawTokens("calc(var(--x) + 1px)".into())
    );
}

#[test]
fn value_for_property_rejects_var_smuggled_injection() {
    // Codex R1-F4 (setProperty half): the raw fallback used to consume
    // the whole input, defeating the trailing-input injection guard.
    assert!(parse_value_for_property("width", "var(--x); color: red", None).is_none());
    assert!(parse_value_for_property("width", "calc(var(--x) + 1px) !important", None).is_none());
    // Plain var() values stay accepted.
    assert!(parse_value_for_property("width", "var(--x)", None).is_some());
    assert!(parse_value_for_property("width", "calc(var(--x) + 1px)", None).is_some());
}

#[test]
fn custom_property_bare_bang_invalidates_declaration() {
    // Codex R2: a top-level `!` that is not `!important` is excluded from
    // `<declaration-value>` — `--x: foo ! bar` is invalid whole and must
    // not leak `--x: foo` into the block; `color: red` still parses.
    let decls = parse_decls("--x: foo ! bar; color: red");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "color");
}

#[test]
fn known_property_trailing_junk_dropped() {
    // CSS Syntax §5.4.4: a declaration with trailing junk after its
    // value (no top-level `;`) is malformed and dropped whole.
    let decls = parse_decls("color: red blue; display: block");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "display");
}
