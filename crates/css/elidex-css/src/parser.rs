//! CSS stylesheet parser.
//!
//! Uses the `cssparser` crate for tokenization and rule-level parsing,
//! delegating property value parsing to [`crate::declaration`].

use cssparser::{
    AtRuleParser, CowRcStr, DeclarationParser, ParseError, Parser, ParserInput, ParserState,
    QualifiedRuleParser, RuleBodyItemParser, RuleBodyParser, StyleSheetParser,
};

use elidex_plugin::{CssPropertyRegistry, PageRule};

use crate::declaration::{parse_property_value, Declaration, Origin};
use crate::page::parse_page_rules;
use crate::selector::{parse_selector_list, Selector};

/// A parsed CSS stylesheet.
#[derive(Clone, Debug, Default)]
pub struct Stylesheet {
    /// The cascade origin of this stylesheet.
    pub origin: Origin,
    /// Rules in source order.
    pub rules: Vec<CssRule>,
    /// Raw `@keyframes` blocks: `(name, body_text)`.
    ///
    /// Extracted during parsing. The body text is the content between
    /// the outer `{ }` braces and must be parsed by the animation handler
    /// (e.g. `elidex_css_anim::parse::parse_keyframes`).
    pub keyframes_raw: Vec<(String, String)>,
    /// Parsed `@page` rules (CSS Paged Media Level 3).
    pub page_rules: Vec<PageRule>,
    /// Next [`CssRule::rule_id`] to issue when CSSOM `insertRule` extends
    /// this stylesheet at run time (CSSOM §6.4 / §6.5). Set to one past
    /// the highest `rule_id` issued during parse so that `insertRule`-
    /// produced rules never collide with parse-time rules.
    pub next_rule_id: u64,
}

/// A single CSS rule (selector list + declarations).
#[derive(Clone, Debug)]
pub struct CssRule {
    /// Selectors for this rule.
    pub selectors: Vec<Selector>,
    /// Declarations (all longhand, shorthands already expanded).
    pub declarations: Vec<Declaration>,
    /// Position in the source stylesheet (0-based).
    pub source_order: u32,
    /// Stable opaque identity for CSSOM rule wrappers (`CSSStyleRule`,
    /// `CSSMediaRule`, …). Issued sequentially at parse time and at
    /// `insertRule` time from [`Stylesheet::next_rule_id`]; survives
    /// `deleteRule` reordering so a JS reference like
    /// `let r = sheet.cssRules[1]; sheet.deleteRule(0); r.cssText`
    /// continues to address the right rule.
    pub rule_id: u64,
    /// Raw source text of this rule (selector list + declaration block,
    /// trimmed). Captured at parse time and used to back CSSOM
    /// `CSSStyleRule.cssText` (read) and `CSSStyleSheet` re-serialisation
    /// when `insertRule` / `deleteRule` writes back to `<style>.textContent`.
    /// Selector and declaration serialisation are not implemented; storing
    /// the source text avoids the round-trip.
    pub source_text: String,
    /// Raw selector portion of the rule's source text (everything
    /// before the opening `{`, trimmed). Captured separately from
    /// [`Self::source_text`] so CSSOM `CSSStyleRule.selectorText`
    /// returns the spec-correct slice without a `split_once('{')`
    /// heuristic — the heuristic mis-slices selectors that contain
    /// `{` inside an attribute value (e.g. `[data-x="{"]`).
    pub selector_text: String,
}

/// Parse a CSS string into a [`Stylesheet`].
///
/// Invalid rules are silently skipped per CSS error recovery rules.
/// `@keyframes` rules are extracted into [`Stylesheet::keyframes_raw`].
#[must_use]
pub fn parse_stylesheet(css: &str, origin: Origin) -> Stylesheet {
    parse_stylesheet_with_registry(css, origin, None)
}

/// Parse a single CSS rule for CSSOM `CSSStyleSheet.insertRule(text)` (CSSOM
/// §6.4 step 2). Returns `Some(rule)` when `text` parses as exactly one
/// qualified rule; returns `None` for empty / invalid / multi-rule input
/// (the caller is expected to throw `SyntaxError` per CSSOM §6.4).
///
/// The returned rule has `rule_id = 0` and `source_order = 0`. The
/// caller (`CSSStyleSheet.insertRule`) overwrites `rule_id` with the
/// next id from [`Stylesheet::next_rule_id`]; `source_order` is left
/// alone because `CSSStyleSheet`'s mutator round-trip writes the
/// updated stylesheet back to `<style>.textContent` and the next
/// cascade walk re-parses it, assigning fresh sequential
/// `source_order` values across all rules.
#[must_use]
pub fn parse_single_rule(css: &str) -> Option<CssRule> {
    let trimmed = css.trim();
    if trimmed.is_empty() {
        return None;
    }
    let mut pi = ParserInput::new(trimmed);
    let mut input = Parser::new(&mut pi);
    let mut source_order: u32 = 0;
    let mut next_rule_id: u64 = 0;
    let mut keyframes_raw = Vec::new();
    let mut page_rules = Vec::new();
    let mut rule_parser = RuleListParser {
        source_order: &mut source_order,
        next_rule_id: &mut next_rule_id,
        keyframes_raw: &mut keyframes_raw,
        page_rules: &mut page_rules,
        registry: None,
    };
    // Strict: must yield exactly one `Ok(rule)` and then end-of-stream.
    // - Any `Err` (parse error / unrecognized at-rule) = skipped content
    //   → `insertRule` must reject per CSSOM §6.4 SyntaxError, despite
    //   the broader `parse_stylesheet` path's CSS error-recovery.
    // - `@keyframes` / `@page` end up in the side-channel vecs even when
    //   their parse_block returns Err; reject if either fired.
    let (first, second) = {
        let mut iter = StyleSheetParser::new(&mut input, &mut rule_parser);
        (iter.next(), iter.next())
    };
    if !keyframes_raw.is_empty() || !page_rules.is_empty() {
        return None;
    }
    match (first, second) {
        (Some(Ok(rule)), None) => Some(rule),
        _ => None,
    }
}

/// Parse a CSS string into a [`Stylesheet`], with optional handler registry.
///
/// When a `registry` is provided, properties not handled by the built-in parser
/// (e.g. `transition-*`, `animation-*`) are dispatched to the matching
/// [`CssPropertyHandler`](elidex_plugin::CssPropertyHandler) for parsing.
///
/// `@keyframes` rules are always extracted into [`Stylesheet::keyframes_raw`].
#[must_use]
pub fn parse_stylesheet_with_registry(
    css: &str,
    origin: Origin,
    registry: Option<&CssPropertyRegistry>,
) -> Stylesheet {
    let mut pi = ParserInput::new(css);
    let mut input = Parser::new(&mut pi);
    let mut rules = Vec::new();
    let mut source_order: u32 = 0;
    let mut next_rule_id: u64 = 0;
    let mut keyframes_raw = Vec::new();
    let mut page_rules = Vec::new();

    let mut rule_parser = RuleListParser {
        source_order: &mut source_order,
        next_rule_id: &mut next_rule_id,
        keyframes_raw: &mut keyframes_raw,
        page_rules: &mut page_rules,
        registry,
    };

    for rule in StyleSheetParser::new(&mut input, &mut rule_parser).flatten() {
        rules.push(rule);
    }

    Stylesheet {
        origin,
        rules,
        keyframes_raw,
        page_rules,
        next_rule_id,
    }
}

// --- cssparser trait implementations ---

struct RuleListParser<'a> {
    source_order: &'a mut u32,
    next_rule_id: &'a mut u64,
    keyframes_raw: &'a mut Vec<(String, String)>,
    page_rules: &'a mut Vec<PageRule>,
    registry: Option<&'a CssPropertyRegistry>,
}

/// At-rule kind tag used as the prelude type.
enum AtRuleKind {
    /// `@keyframes <name>` — name is the keyframes identifier.
    Keyframes(String),
    /// `@page <selectors>` — prelude text for page pseudo-classes.
    Page(String),
}

/// `@keyframes` and `@page` rules are parsed and stored in their
/// respective fields. All other at-rules are silently dropped.
impl<'i> AtRuleParser<'i> for RuleListParser<'_> {
    type Prelude = AtRuleKind;
    type AtRule = CssRule;
    type Error = ();

    fn parse_prelude<'t>(
        &mut self,
        name: CowRcStr<'i>,
        input: &mut Parser<'i, 't>,
    ) -> Result<Self::Prelude, ParseError<'i, ()>> {
        if name.eq_ignore_ascii_case("keyframes") || name.eq_ignore_ascii_case("-webkit-keyframes")
        {
            // CSS Animations Level 1 §3: <keyframes-name> = <custom-ident> | <string>
            let keyframes_name = if let Ok(ident) =
                input.try_parse(|i| i.expect_ident().map(|s| s.as_ref().to_string()))
            {
                ident
            } else {
                // Fallback: accept quoted string names (e.g. @keyframes "my-anim" {})
                input
                    .expect_string()
                    .map_err(ParseError::from)?
                    .as_ref()
                    .to_string()
            };
            // CSS Animations Level 1 §3: CSS-wide keywords and `none` are
            // invalid as @keyframes names.
            let lower = keyframes_name.to_ascii_lowercase();
            if matches!(
                lower.as_str(),
                "initial" | "inherit" | "unset" | "revert" | "revert-layer" | "none"
            ) {
                return Err(input.new_custom_error(()));
            }
            Ok(AtRuleKind::Keyframes(keyframes_name))
        } else if name.eq_ignore_ascii_case("page") {
            // CSS Paged Media L3 §4: @page <page-selector-list>? { ... }
            // Consume the rest of the prelude as raw text for selector parsing.
            let start_pos = input.position();
            while input.next_including_whitespace_and_comments().is_ok() {}
            let prelude_text = input.slice_from(start_pos).trim().to_string();
            Ok(AtRuleKind::Page(prelude_text))
        } else {
            Err(input.new_custom_error(()))
        }
    }

    fn parse_block<'t>(
        &mut self,
        prelude: Self::Prelude,
        _start: &ParserState,
        input: &mut Parser<'i, 't>,
    ) -> Result<Self::AtRule, ParseError<'i, ()>> {
        // Collect the raw block text by consuming all tokens (including
        // nested curly-bracket blocks) and using slice_from.
        let start_pos = input.position();
        while input.next_including_whitespace_and_comments().is_ok() {}
        let body = input.slice_from(start_pos).to_string();

        match prelude {
            AtRuleKind::Keyframes(name) => {
                self.keyframes_raw.push((name, body));
            }
            AtRuleKind::Page(prelude_text) => {
                let rules = parse_page_rules(&prelude_text, &body);
                self.page_rules.extend(rules);
            }
        }

        // Return Err so no CssRule is produced — @keyframes/@page are stored
        // separately in their respective Stylesheet fields.
        Err(input.new_custom_error(()))
    }
}

impl<'i> QualifiedRuleParser<'i> for RuleListParser<'_> {
    /// `(selectors, selector_text)` — `selector_text` is captured raw so
    /// CSSOM `CSSStyleRule.cssText` / `selectorText` can return the source
    /// text without re-implementing selector serialisation.
    type Prelude = (Vec<Selector>, String);
    type QualifiedRule = CssRule;
    type Error = ();

    fn parse_prelude<'t>(
        &mut self,
        input: &mut Parser<'i, 't>,
    ) -> Result<Self::Prelude, ParseError<'i, ()>> {
        let start_pos = input.position();
        let selectors = parse_selector_list(input).map_err(|()| input.new_custom_error(()))?;
        let selector_text = input.slice_from(start_pos).trim().to_string();
        Ok((selectors, selector_text))
    }

    fn parse_block<'t>(
        &mut self,
        prelude: Self::Prelude,
        _location: &ParserState,
        input: &mut Parser<'i, 't>,
    ) -> Result<Self::QualifiedRule, ParseError<'i, ()>> {
        let (selectors, selector_text) = prelude;
        let body_start = input.position();

        let mut decl_parser = DeclarationListParser {
            registry: self.registry,
        };
        let mut declarations = Vec::new();

        for decls in RuleBodyParser::new(input, &mut decl_parser).flatten() {
            declarations.extend(decls);
        }

        let body_text = input.slice_from(body_start).trim().to_string();

        let order = *self.source_order;
        *self.source_order = self.source_order.saturating_add(1);
        let rule_id = *self.next_rule_id;
        *self.next_rule_id = self.next_rule_id.saturating_add(1);

        let source_text = if body_text.is_empty() {
            format!("{selector_text} {{ }}")
        } else {
            format!("{selector_text} {{ {body_text} }}")
        };

        Ok(CssRule {
            selectors,
            declarations,
            source_order: order,
            rule_id,
            source_text,
            selector_text,
        })
    }
}

struct DeclarationListParser<'a> {
    registry: Option<&'a CssPropertyRegistry>,
}

impl<'i> DeclarationParser<'i> for DeclarationListParser<'_> {
    type Declaration = Vec<Declaration>;
    type Error = ();

    fn parse_value<'t>(
        &mut self,
        name: CowRcStr<'i>,
        input: &mut Parser<'i, 't>,
        _start: &cssparser::ParserState,
    ) -> Result<Self::Declaration, ParseError<'i, ()>> {
        // CSS Variables Level 1 §2: custom properties (`--*`) are
        // case-sensitive — must preserve `--MyVar` vs `--myvar` so
        // `getPropertyValue('--MyVar')` against a stylesheet rule
        // doesn't miss.  `parse_declaration_block` (declaration.rs)
        // already does this; the stylesheet path was unconditionally
        // lowercasing and producing the asymmetry Copilot flagged.
        let property_name = if name.starts_with("--") {
            name.as_ref().to_string()
        } else {
            name.to_ascii_lowercase()
        };
        let decls = parse_property_value(&property_name, input, self.registry);
        if decls.is_empty() {
            Err(input.new_custom_error(()))
        } else {
            // Check for !important after successfully parsing the value.
            let important = input.try_parse(cssparser::parse_important).is_ok();
            if important {
                Ok(decls
                    .into_iter()
                    .map(|mut d| {
                        d.important = true;
                        d
                    })
                    .collect())
            } else {
                Ok(decls)
            }
        }
    }
}

impl AtRuleParser<'_> for DeclarationListParser<'_> {
    type Prelude = ();
    type AtRule = Vec<Declaration>;
    type Error = ();
}

impl QualifiedRuleParser<'_> for DeclarationListParser<'_> {
    type Prelude = ();
    type QualifiedRule = Vec<Declaration>;
    type Error = ();
}

impl RuleBodyItemParser<'_, Vec<Declaration>, ()> for DeclarationListParser<'_> {
    fn parse_qualified(&self) -> bool {
        false
    }

    fn parse_declarations(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::declaration::parse_declaration_block;
    use elidex_plugin::{CssColor, CssValue};

    #[test]
    fn parse_single_rule() {
        let ss = parse_stylesheet("div { color: red; }", Origin::Author);
        assert_eq!(ss.rules.len(), 1);
        assert_eq!(ss.rules[0].selectors.len(), 1);
        assert_eq!(ss.rules[0].declarations.len(), 1);
        assert_eq!(ss.rules[0].declarations[0].property, "color");
        assert_eq!(
            ss.rules[0].declarations[0].value,
            CssValue::Color(CssColor::RED)
        );
    }

    #[test]
    fn parse_multiple_rules() {
        let css = "div { color: red; } p { display: block; }";
        let ss = parse_stylesheet(css, Origin::Author);
        assert_eq!(ss.rules.len(), 2);
    }

    #[test]
    fn at_rule_silently_dropped() {
        let css = "@media screen { div { color: red; } } p { display: block; }";
        let ss = parse_stylesheet(css, Origin::Author);
        // @media is skipped; p rule survives.
        assert_eq!(ss.rules.len(), 1);
        assert_eq!(ss.rules[0].declarations[0].property, "display");
    }

    #[test]
    fn keyframes_extracted() {
        let css = r"
            @keyframes fadeIn {
                from { opacity: 0; }
                to { opacity: 1; }
            }
            p { color: red; }
        ";
        let ss = parse_stylesheet(css, Origin::Author);
        assert_eq!(ss.rules.len(), 1, "only the p rule should be in rules");
        assert_eq!(ss.keyframes_raw.len(), 1);
        assert_eq!(ss.keyframes_raw[0].0, "fadeIn");
        assert!(ss.keyframes_raw[0].1.contains("opacity"));
    }

    #[test]
    fn webkit_keyframes_extracted() {
        let css = "@-webkit-keyframes slide { from { left: 0; } to { left: 100px; } }";
        let ss = parse_stylesheet(css, Origin::Author);
        assert_eq!(ss.keyframes_raw.len(), 1);
        assert_eq!(ss.keyframes_raw[0].0, "slide");
    }

    #[test]
    fn multiple_keyframes() {
        let css = r"
            @keyframes a { from { opacity: 0; } to { opacity: 1; } }
            @keyframes b { 0% { width: 0; } 100% { width: 100px; } }
            div { color: red; }
        ";
        let ss = parse_stylesheet(css, Origin::Author);
        assert_eq!(ss.rules.len(), 1);
        assert_eq!(ss.keyframes_raw.len(), 2);
        assert_eq!(ss.keyframes_raw[0].0, "a");
        assert_eq!(ss.keyframes_raw[1].0, "b");
    }

    #[test]
    fn parse_inline_declarations() {
        let decls = parse_declaration_block("color: red; margin: 10px");
        // color: 1 decl + margin: 4 decls = 5
        assert_eq!(decls.len(), 5);
        assert_eq!(decls[0].property, "color");
        assert_eq!(decls[1].property, "margin-top");
    }

    #[test]
    fn source_order_preserved() {
        let css = "a { color: red; } b { color: blue; } c { color: green; }";
        let ss = parse_stylesheet(css, Origin::Author);
        assert_eq!(ss.rules.len(), 3);
        assert_eq!(ss.rules[0].source_order, 0);
        assert_eq!(ss.rules[1].source_order, 1);
        assert_eq!(ss.rules[2].source_order, 2);
    }

    #[test]
    fn selector_list_rule() {
        let ss = parse_stylesheet("h1, h2 { color: blue; }", Origin::Author);
        assert_eq!(ss.rules.len(), 1);
        assert_eq!(ss.rules[0].selectors.len(), 2);
    }

    #[test]
    fn empty_stylesheet() {
        let ss = parse_stylesheet("", Origin::Author);
        assert!(ss.rules.is_empty());
    }

    #[test]
    fn important_declaration() {
        let ss = parse_stylesheet("div { color: red !important; }", Origin::Author);
        assert_eq!(ss.rules.len(), 1);
        assert!(ss.rules[0].declarations[0].important);
    }

    #[test]
    fn keyframes_quoted_name() {
        let css = r#"@keyframes "quoted-name" { from { opacity: 0; } to { opacity: 1; } }"#;
        let ss = parse_stylesheet(css, Origin::Author);
        assert_eq!(ss.keyframes_raw.len(), 1);
        assert_eq!(ss.keyframes_raw[0].0, "quoted-name");
    }

    #[test]
    fn keyframes_none_rejected() {
        let css = "@keyframes none { from { opacity: 0; } to { opacity: 1; } }";
        let ss = parse_stylesheet(css, Origin::Author);
        assert!(
            ss.keyframes_raw.is_empty(),
            "@keyframes none should be rejected"
        );
    }

    #[test]
    fn keyframes_initial_rejected() {
        let css = "@keyframes initial { from { opacity: 0; } to { opacity: 1; } }";
        let ss = parse_stylesheet(css, Origin::Author);
        assert!(
            ss.keyframes_raw.is_empty(),
            "@keyframes initial should be rejected"
        );
    }
}
