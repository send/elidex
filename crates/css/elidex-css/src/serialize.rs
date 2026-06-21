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
/// **`@media` is preserved**: a rule flattened out of an `@media` block carries
/// its condition chain in `CssRule::media_conditions`, so it is re-emitted
/// wrapped in `@media <serialized-condition> { … }` (nested for a multi-level
/// chain). This keeps a CSSOM `insertRule` / `deleteRule` round-trip from
/// *unwrapping* `@media` blocks into unconditional rules — CSS Conditional
/// Rules §2: a conditional group rule's contents "always remain within the
/// group rule". The serialized condition is the engine-independent
/// [`crate::media::MediaQueryList`] `Display` (canonical, #364). Per-rule
/// wrapping is semantically exact (each rule re-gates identically on re-parse),
/// not byte-identical to the author's grouping.
///
/// **Still destructive for other at-rules**: `@import`/`@supports`/
/// `@font-face`/`@namespace` remain dropped at parse, and `@keyframes`/`@page`
/// live in dedicated `Stylesheet` fields with no entry in `Stylesheet::rules`,
/// so the first CSSOM mutation still erases them. Each has its own deferred slot
/// (CSSOM `CSSMediaRule` surfacing = `#11-css-media-rule`). Callers that need
/// full round-trip preservation should write `<style>.textContent` directly
/// until those slots land.
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
        if rule.media_conditions.is_empty() {
            out.push_str(&rule.source_text);
        } else {
            // Reconstruct the `@media` wrapper(s), outermost first.
            for cond in &rule.media_conditions {
                out.push_str("@media ");
                out.push_str(&cond.to_string());
                out.push_str(" { ");
            }
            out.push_str(&rule.source_text);
            for _ in &rule.media_conditions {
                out.push_str(" }");
            }
        }
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

    #[test]
    fn media_wrapper_survives_roundtrip() {
        // Regression guard: a CSSOM mutation round-trip must NOT unwrap `@media`
        // into an unconditional rule (CSS Conditional §2). After
        // parse → serialize → re-parse, the inner rule keeps its condition.
        let ss = parse_stylesheet(
            "@media screen { div { color: red; } } p { color: blue; }",
            Origin::Author,
        );
        let text = serialize_stylesheet(&ss);
        assert!(text.contains("@media"), "wrapper reconstructed: {text}");
        let again = parse_stylesheet(&text, Origin::Author);
        assert_eq!(again.rules.len(), 2);
        // The `div` rule is still gated; `p` is still unconditional.
        let div = again
            .rules
            .iter()
            .find(|r| r.selector_text == "div")
            .unwrap();
        let p = again.rules.iter().find(|r| r.selector_text == "p").unwrap();
        assert_eq!(div.media_conditions.len(), 1);
        assert!(p.media_conditions.is_empty());
    }

    #[test]
    fn nested_media_wrapper_survives_roundtrip() {
        let ss = parse_stylesheet(
            "@media screen { @media (min-width: 1px) { x { top: 0; } } }",
            Origin::Author,
        );
        let again = parse_stylesheet(&serialize_stylesheet(&ss), Origin::Author);
        assert_eq!(again.rules.len(), 1);
        assert_eq!(again.rules[0].media_conditions.len(), 2);
    }
}
