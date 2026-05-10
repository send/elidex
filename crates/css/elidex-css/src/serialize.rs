//! Stylesheet → CSS-text serialisation for CSSOM `CSSStyleSheet` mutators.
//!
//! When `CSSStyleSheet.insertRule` / `deleteRule` mutate the rule list, the
//! result must be visible to the cascade — which re-parses `<style>.textContent`
//! on every walk. This module converts a [`Stylesheet`] back to a CSS text
//! string suitable for writing through `node.textContent` (the canonical
//! `EcsDom`-versioned write path).
//!
//! The serialisation is **source-text concatenation**, not AST round-trip:
//! each [`CssRule`] carries a `source_text` captured at parse time so we
//! avoid implementing selector / declaration serialisation. The captured
//! text is the post-parse normalised form (the parser builds
//! `format!("{selector_text} {{ {body_text} }}")` over the trimmed
//! prelude + body slices), so a `<style>{div{color:red}}` round-trip
//! through `insertRule` becomes `div { color:red }` with whitespace
//! around the braces. Author-visible CSS semantics are preserved; only
//! exact byte-equivalence with the original textContent is not.

use crate::parser::Stylesheet;

/// Serialise a [`Stylesheet`] to CSS text for writing back to
/// `<style>.textContent`. Rules are emitted in source order, separated by
/// newlines for readability.
///
/// **Destructive round-trip**: at-rules dropped by
/// [`crate::parse_stylesheet`] (`@media`, `@import`, `@supports`,
/// `@font-face`, `@namespace`, etc.)
/// are NOT preserved. `@keyframes` and `@page` live in dedicated
/// `Stylesheet` fields and are also omitted from this output (they have
/// no entry in `Stylesheet::rules`). The first CSSOM `insertRule` /
/// `deleteRule` call therefore **erases every dropped rule** from the
/// `<style>` element's text content.
///
/// This is acceptable for PR-B because the only currently-CSSOM-visible
/// rule type is `CSSStyleRule` (slot `#11-css-media-rule` extends to
/// `CSSMediaRule`; the other at-rules each have their own deferred
/// slot). Callers that need full round-trip preservation should write
/// `<style>.textContent` directly until those slots land.
#[must_use]
pub fn serialize_stylesheet(stylesheet: &Stylesheet) -> String {
    let cap: usize = stylesheet
        .rules
        .iter()
        .map(|r| r.source_text.len() + 1)
        .sum::<usize>()
        .saturating_sub(1);
    let mut out = String::with_capacity(cap);
    for (i, rule) in stylesheet.rules.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(&rule.source_text);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_stylesheet;
    use crate::Origin;

    #[test]
    fn roundtrip_single_rule() {
        let ss = parse_stylesheet("div { color: red; }", Origin::Author);
        let text = serialize_stylesheet(&ss);
        // Re-parse and ensure structure preserved.
        let again = parse_stylesheet(&text, Origin::Author);
        assert_eq!(again.rules.len(), 1);
        assert_eq!(again.rules[0].declarations[0].property, "color");
    }

    #[test]
    fn roundtrip_multi_rule_preserves_order() {
        let ss = parse_stylesheet(
            "div { color: red; } p { color: blue; } span { display: none; }",
            Origin::Author,
        );
        let text = serialize_stylesheet(&ss);
        let again = parse_stylesheet(&text, Origin::Author);
        assert_eq!(again.rules.len(), 3);
        assert_eq!(again.rules[0].declarations[0].property, "color");
        assert_eq!(again.rules[2].declarations[0].property, "display");
    }

    #[test]
    fn empty_sheet_produces_empty_text() {
        let ss = parse_stylesheet("", Origin::Author);
        assert_eq!(serialize_stylesheet(&ss), "");
    }
}
