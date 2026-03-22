//! CSS Paged Media Level 3 `@page` rule parsing.
//!
//! Parses `@page` selectors, `size` property, and margin box at-rules.

use cssparser::{Parser, ParserInput};
use elidex_plugin::{
    ContentItem, ContentValue, CssValue, ListStyleType, MarginBoxContent, NamedPageSize,
    PageMargins, PageRule, PageSelector, PageSize, PropertyDeclaration,
};

use crate::declaration::parse_declaration_block;

/// Parse page pseudo-class selectors from the `@page` prelude.
///
/// Accepts combined pseudo-classes (`:first:right` → both in one Vec, AND
/// semantics) and comma-separated groups (`:left, :right` → separate Vecs).
///
/// Returns a `Vec` of selector groups: each inner `Vec<PageSelector>` is one
/// comma-separated group whose selectors are combined with AND logic.
/// An empty outer Vec means no selectors (matches all pages).
#[must_use]
pub fn parse_page_selectors(input: &str) -> Vec<Vec<PageSelector>> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    let mut groups = Vec::new();
    for part in trimmed.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        // Split combined pseudo-classes like `:first:right` on `:` boundaries.
        let mut selectors = Vec::new();
        for segment in part.split(':') {
            let segment = segment.trim();
            if segment.is_empty() {
                continue;
            }
            if let Some(sel) = PageSelector::from_keyword(segment) {
                selectors.push(sel);
            }
        }
        if !selectors.is_empty() {
            groups.push(selectors);
        }
    }
    groups
}

/// Parse a `size` property value from a cssparser `Parser`.
///
/// Supported forms:
/// - `auto`
/// - `<named>` (e.g. `A4`, `letter`)
/// - `<named> landscape`/`portrait`
/// - `<length> <length>` (explicit dimensions)
/// - `landscape <length> <length>`
/// - `portrait <length> <length>`
pub fn parse_page_size(input: &mut Parser) -> Option<PageSize> {
    // Try `auto`.
    if input.try_parse(|i| i.expect_ident_matching("auto")).is_ok() {
        return Some(PageSize::Auto);
    }

    // Try orientation keyword first, then named/length.
    let orientation = input
        .try_parse(|i| {
            let ident = i.expect_ident().map_err(|_| ())?.to_string();
            match ident.to_ascii_lowercase().as_str() {
                "landscape" => Ok("landscape"),
                "portrait" => Ok("portrait"),
                _ => Err(()),
            }
        })
        .ok();

    // Try named page size.
    if let Ok(named) = input.try_parse(|i| {
        let ident = i.expect_ident().map_err(|_| ())?.to_string();
        NamedPageSize::from_keyword(&ident).ok_or(())
    }) {
        // Check for trailing orientation keyword if we didn't get one already.
        let orientation = orientation.or_else(|| {
            input
                .try_parse(|i| {
                    let ident = i.expect_ident().map_err(|_| ())?.to_string();
                    match ident.to_ascii_lowercase().as_str() {
                        "landscape" => Ok("landscape"),
                        "portrait" => Ok("portrait"),
                        _ => Err(()),
                    }
                })
                .ok()
        });

        return match orientation {
            Some("landscape") => Some(PageSize::LandscapeNamed(named)),
            Some("portrait") => Some(PageSize::PortraitNamed(named)),
            _ => Some(PageSize::Named(named)),
        };
    }

    // Try explicit dimensions: <length> <length>.
    if let Some(w) = try_parse_length_px(input) {
        let h = try_parse_length_px(input).unwrap_or(w);
        return match orientation {
            Some("landscape") => Some(PageSize::LandscapeExplicit(w, h)),
            Some("portrait") => Some(PageSize::PortraitExplicit(w, h)),
            _ => Some(PageSize::Explicit(w, h)),
        };
    }

    None
}

/// Try to parse a length value and convert to px.
fn try_parse_length_px(input: &mut Parser) -> Option<f32> {
    input
        .try_parse(|i| {
            let token = i.next().map_err(|_| ())?;
            match *token {
                cssparser::Token::Dimension {
                    value, ref unit, ..
                } if value.is_finite() => {
                    let px = length_to_px(value, unit)?;
                    Ok(px)
                }
                _ => Err(()),
            }
        })
        .ok()
}

/// Convert a CSS length value to pixels.
fn length_to_px(value: f32, unit: &str) -> Result<f32, ()> {
    match unit.to_ascii_lowercase().as_str() {
        "px" => Ok(value),
        "in" => Ok(value * 96.0),
        "cm" => Ok(value * 96.0 / 2.54),
        "mm" => Ok(value * 96.0 / 25.4),
        "pt" => Ok(value * 96.0 / 72.0),
        "pc" => Ok(value * 96.0 / 6.0),
        _ => Err(()),
    }
}

/// Margin box at-rule names (CSS Paged Media L3 §4.2).
const MARGIN_BOX_NAMES: &[&str] = &[
    "top-left-corner",
    "top-left",
    "top-center",
    "top-right",
    "top-right-corner",
    "right-top",
    "right-middle",
    "right-bottom",
    "bottom-right-corner",
    "bottom-right",
    "bottom-center",
    "bottom-left",
    "bottom-left-corner",
    "left-bottom",
    "left-middle",
    "left-top",
];

/// Parse a margin box at-rule inside `@page`.
///
/// Returns the margin box content if the name is a valid margin box type.
#[must_use]
pub fn parse_margin_box(name: &str, block: &str) -> Option<MarginBoxContent> {
    let lower = name.to_ascii_lowercase();
    if !MARGIN_BOX_NAMES.contains(&lower.as_str()) {
        return None;
    }

    let declarations = parse_declaration_block(block);
    let mut content = ContentValue::Normal;
    let mut properties = Vec::new();

    for decl in declarations {
        if decl.property == "content" {
            content = content_value_from_css_value(&decl.value);
        } else {
            properties.push(PropertyDeclaration::new(decl.property, decl.value));
        }
    }

    Some(MarginBoxContent {
        content,
        properties,
    })
}

/// Convert a `CssValue` to a `ContentValue`.
fn content_value_from_css_value(value: &CssValue) -> ContentValue {
    match value {
        CssValue::Keyword(k) if k == "normal" => ContentValue::Normal,
        CssValue::Keyword(k) if k == "none" => ContentValue::None,
        CssValue::String(s) => ContentValue::Items(vec![ContentItem::String(s.clone())]),
        CssValue::Keyword(k) => {
            // The declaration parser encodes counter()/counters() as keyword strings:
            //   "counter:name:style" or "counters:name:separator:style"
            if let Some(item) = parse_counter_keyword(k) {
                ContentValue::Items(vec![item])
            } else {
                ContentValue::Normal
            }
        }
        CssValue::List(items) => {
            let content_items: Vec<ContentItem> =
                items.iter().filter_map(css_value_to_content_item).collect();
            if content_items.is_empty() {
                ContentValue::Normal
            } else {
                ContentValue::Items(content_items)
            }
        }
        _ => ContentValue::Normal,
    }
}

/// Convert a single `CssValue` to a `ContentItem`.
fn css_value_to_content_item(value: &CssValue) -> Option<ContentItem> {
    match value {
        CssValue::String(s) => Some(ContentItem::String(s.clone())),
        CssValue::Keyword(k) => parse_counter_keyword(k),
        _ => None,
    }
}

/// Parse an encoded counter keyword string into a `ContentItem`.
///
/// The declaration parser encodes `counter(name, style)` as `"counter:name:style"`
/// and `counters(name, sep, style)` as `"counters:name:sep:style"`.
fn parse_counter_keyword(k: &str) -> Option<ContentItem> {
    if let Some(rest) = k.strip_prefix("counter:") {
        // "counter:name:style"
        let mut parts = rest.splitn(2, ':');
        let name = parts.next()?.to_string();
        let style_str = parts.next().unwrap_or("decimal");
        let style = ListStyleType::from_keyword(style_str).unwrap_or_default();
        Some(ContentItem::Counter { name, style })
    } else if let Some(rest) = k.strip_prefix("counters:") {
        // "counters:name:separator:style"
        let mut parts = rest.splitn(3, ':');
        let name = parts.next()?.to_string();
        let separator = parts.next()?.to_string();
        let style_str = parts.next().unwrap_or("decimal");
        let style = ListStyleType::from_keyword(style_str).unwrap_or_default();
        Some(ContentItem::Counters {
            name,
            separator,
            style,
        })
    } else {
        None
    }
}

/// Parse a complete `@page` rule from prelude + block text.
///
/// Comma-separated selectors in the prelude produce separate `PageRule`
/// instances (CSS Paged Media L3 §4.1: commas create independent rules).
/// Combined selectors like `:first:right` stay in a single rule with AND
/// semantics.
///
/// Called from the stylesheet parser after recognizing `@page`.
#[must_use]
pub fn parse_page_rules(prelude: &str, block: &str) -> Vec<PageRule> {
    let selector_groups = parse_page_selectors(prelude);

    // Parse the block content once, then clone for each selector group.
    let template = parse_page_rule_body(block);

    if selector_groups.is_empty() {
        // No selectors → single rule matching all pages.
        return vec![template];
    }

    selector_groups
        .into_iter()
        .map(|selectors| {
            let mut rule = template.clone();
            rule.selectors = selectors;
            rule
        })
        .collect()
}

/// Parse a complete `@page` rule from prelude + block text (legacy API).
///
/// Returns a single `PageRule`. For comma-separated selectors, all selectors
/// are combined into one rule (AND semantics). Prefer `parse_page_rules()`
/// for correct comma-separated handling.
#[must_use]
pub fn parse_page_rule(prelude: &str, block: &str) -> PageRule {
    let mut rules = parse_page_rules(prelude, block);
    if rules.len() == 1 {
        return rules.swap_remove(0);
    }
    // Fallback: merge selectors into one rule (backward compat for tests).
    let mut merged = parse_page_rule_body(block);
    for rule in rules {
        merged.selectors.extend(rule.selectors);
    }
    merged
}

/// Parse the body (block) of an `@page` rule into a `PageRule` with empty selectors.
fn parse_page_rule_body(block: &str) -> PageRule {
    let mut rule = PageRule::default();

    // Parse the block: declarations and nested @margin-box rules.
    // We do a two-pass approach:
    // 1. Extract nested @-rules (margin boxes) by scanning for @<name> { ... }
    // 2. Parse remaining declarations normally.

    let mut remaining = String::new();
    let mut chars = block.chars().peekable();

    while let Some(&ch) = chars.peek() {
        if ch == '@' {
            // Consume '@'
            chars.next();
            // Read at-rule name.
            let mut name = String::new();
            while let Some(&c) = chars.peek() {
                if c == '{' || c.is_whitespace() {
                    break;
                }
                name.push(c);
                chars.next();
            }
            // Skip whitespace.
            while let Some(&c) = chars.peek() {
                if !c.is_whitespace() {
                    break;
                }
                chars.next();
            }
            // Expect '{'.
            if chars.peek() == Some(&'{') {
                chars.next(); // consume '{'
                let mut depth = 1;
                let mut body = String::new();
                for c in chars.by_ref() {
                    if c == '{' {
                        depth += 1;
                        body.push(c);
                    } else if c == '}' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                        body.push(c);
                    } else {
                        body.push(c);
                    }
                }
                // Try to parse as margin box.
                if let Some(margin_box) = parse_margin_box(&name, &body) {
                    set_margin_box(&mut rule.margins, &name.to_ascii_lowercase(), margin_box);
                }
            } else {
                // Not a block, just put it back.
                remaining.push('@');
                remaining.push_str(&name);
            }
        } else {
            remaining.push(ch);
            chars.next();
        }
    }

    // Parse remaining declarations.
    // The `size` property is @page-specific, so we extract it manually
    // before passing the rest to the standard declaration parser.
    // Standard properties (margin, etc.) go through parse_declaration_block.
    rule.size = extract_size_property(&remaining);

    let declarations = parse_declaration_block(&remaining);
    for decl in declarations {
        // Skip `size` if somehow parsed (it won't be, but guard anyway).
        if decl.property == "size" {
            continue;
        }
        rule.properties
            .push(PropertyDeclaration::new(decl.property, decl.value));
    }

    rule
}

/// Extract `size: <value>;` from a declaration block text and parse it.
fn extract_size_property(block: &str) -> Option<PageSize> {
    // Simple scan for `size:` declarations.
    for part in block.split(';') {
        let trimmed = part.trim();
        if let Some(value_str) = trimmed
            .strip_prefix("size")
            .and_then(|rest| rest.trim_start().strip_prefix(':'))
        {
            let value_str = value_str.trim();
            if value_str.is_empty() {
                continue;
            }
            let mut pi = ParserInput::new(value_str);
            let mut parser = Parser::new(&mut pi);
            if let Some(size) = parse_page_size(&mut parser) {
                return Some(size);
            }
        }
    }
    None
}

/// Set a margin box on `PageMargins` by name.
fn set_margin_box(margins: &mut PageMargins, name: &str, content: MarginBoxContent) {
    match name {
        "top-left-corner" => margins.top_left_corner = Some(content),
        "top-left" => margins.top_left = Some(content),
        "top-center" => margins.top_center = Some(content),
        "top-right" => margins.top_right = Some(content),
        "top-right-corner" => margins.top_right_corner = Some(content),
        "right-top" => margins.right_top = Some(content),
        "right-middle" => margins.right_middle = Some(content),
        "right-bottom" => margins.right_bottom = Some(content),
        "bottom-right-corner" => margins.bottom_right_corner = Some(content),
        "bottom-right" => margins.bottom_right = Some(content),
        "bottom-center" => margins.bottom_center = Some(content),
        "bottom-left" => margins.bottom_left = Some(content),
        "bottom-left-corner" => margins.bottom_left_corner = Some(content),
        "left-bottom" => margins.left_bottom = Some(content),
        "left-middle" => margins.left_middle = Some(content),
        "left-top" => margins.left_top = Some(content),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_page_no_selectors() {
        let rule = parse_page_rule("", "size: A4;");
        assert!(rule.selectors.is_empty());
        assert_eq!(rule.size, Some(PageSize::Named(NamedPageSize::A4)));
    }

    #[test]
    fn parse_page_first_selector() {
        let rule = parse_page_rule(":first", "size: letter;");
        assert_eq!(rule.selectors, vec![PageSelector::First]);
        assert_eq!(rule.size, Some(PageSize::Named(NamedPageSize::Letter)));
    }

    #[test]
    fn parse_page_size_auto() {
        let rule = parse_page_rule("", "size: auto;");
        assert_eq!(rule.size, Some(PageSize::Auto));
    }

    #[test]
    fn parse_page_size_explicit() {
        let rule = parse_page_rule("", "size: 210mm 297mm;");
        let size = rule.size.unwrap();
        match size {
            PageSize::Explicit(w, h) => {
                // 210mm ≈ 793.7px, 297mm ≈ 1122.5px
                assert!((w - 793.7).abs() < 1.0, "width {w}");
                assert!((h - 1122.5).abs() < 1.0, "height {h}");
            }
            other => panic!("expected Explicit, got {other:?}"),
        }
    }

    #[test]
    fn parse_page_size_landscape() {
        let rule = parse_page_rule("", "size: A4 landscape;");
        assert_eq!(rule.size, Some(PageSize::LandscapeNamed(NamedPageSize::A4)));
    }

    #[test]
    fn parse_page_margin_box() {
        let rule = parse_page_rule("", r#"@top-center { content: "Title"; }"#);
        assert!(rule.margins.top_center.is_some());
        let tc = rule.margins.top_center.unwrap();
        assert_eq!(
            tc.content,
            ContentValue::Items(vec![elidex_plugin::ContentItem::String(
                "Title".to_string()
            )])
        );
    }

    #[test]
    fn parse_page_selectors_multiple() {
        // Comma-separated selectors produce separate groups.
        let groups = parse_page_selectors(":left, :right");
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0], vec![PageSelector::Left]);
        assert_eq!(groups[1], vec![PageSelector::Right]);
    }

    #[test]
    fn parse_page_selectors_combined() {
        // Combined selectors (no comma) produce a single group with AND semantics.
        let groups = parse_page_selectors(":first:right");
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0], vec![PageSelector::First, PageSelector::Right]);
    }

    #[test]
    fn parse_page_rules_comma_separated() {
        // Comma-separated selectors produce separate PageRule instances.
        let rules = parse_page_rules(":left, :right", "size: A4;");
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].selectors, vec![PageSelector::Left]);
        assert_eq!(rules[1].selectors, vec![PageSelector::Right]);
        // Both share the same block content.
        assert_eq!(rules[0].size, Some(PageSize::Named(NamedPageSize::A4)));
        assert_eq!(rules[1].size, Some(PageSize::Named(NamedPageSize::A4)));
    }

    #[test]
    fn parse_page_margin_box_counter() {
        let rule = parse_page_rule("", r#"@bottom-center { content: "Page " counter(page); }"#);
        assert!(rule.margins.bottom_center.is_some());
        let bc = rule.margins.bottom_center.unwrap();
        match &bc.content {
            ContentValue::Items(items) => {
                assert_eq!(items.len(), 2);
                assert_eq!(items[0], ContentItem::String("Page ".to_string()));
                assert_eq!(
                    items[1],
                    ContentItem::Counter {
                        name: "page".to_string(),
                        style: elidex_plugin::ListStyleType::Decimal,
                    }
                );
            }
            other => panic!("expected Items, got {other:?}"),
        }
    }

    #[test]
    fn parse_page_margin_box_counter_with_style() {
        let rule = parse_page_rule(
            "",
            r#"@top-right { content: counter(section, upper-roman); }"#,
        );
        assert!(rule.margins.top_right.is_some());
        let tr = rule.margins.top_right.unwrap();
        match &tr.content {
            ContentValue::Items(items) => {
                assert_eq!(items.len(), 1);
                assert_eq!(
                    items[0],
                    ContentItem::Counter {
                        name: "section".to_string(),
                        style: elidex_plugin::ListStyleType::UpperRoman,
                    }
                );
            }
            other => panic!("expected Items, got {other:?}"),
        }
    }

    #[test]
    fn parse_page_margin_box_counters() {
        let rule = parse_page_rule("", r#"@top-left { content: counters(section, "."); }"#);
        assert!(rule.margins.top_left.is_some());
        let tl = rule.margins.top_left.unwrap();
        match &tl.content {
            ContentValue::Items(items) => {
                assert_eq!(items.len(), 1);
                assert_eq!(
                    items[0],
                    ContentItem::Counters {
                        name: "section".to_string(),
                        separator: ".".to_string(),
                        style: elidex_plugin::ListStyleType::Decimal,
                    }
                );
            }
            other => panic!("expected Items, got {other:?}"),
        }
    }

    #[test]
    fn parse_named_page_sizes_all() {
        let names = ["A5", "A4", "A3", "B5", "B4", "letter", "legal", "ledger"];
        for name in &names {
            let rule = parse_page_rule("", &format!("size: {name};"));
            assert!(rule.size.is_some(), "should parse size for '{name}'");
            match rule.size.unwrap() {
                PageSize::Named(n) => {
                    let (w, h) = n.dimensions();
                    assert!(w > 0.0 && h > 0.0, "{name}: dimensions should be positive");
                }
                other => panic!("expected Named for '{name}', got {other:?}"),
            }
        }
    }
}
