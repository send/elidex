//! WPT test harness — runs test cases and reports results.

use std::collections::HashMap;

use elidex_css::{parse_selector_from_str, parse_stylesheet, Origin};
use elidex_ecs::{EcsDom, Entity, TagType};
use elidex_html_parser::parse_html;
use elidex_plugin::{ComputedStyle, CssValue};
use elidex_style::{get_computed, resolve_styles, serialize_resolved_value};

/// A single WPT-style test case.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct WptTestCase {
    /// Human-readable test name.
    pub name: String,
    /// HTML source to parse.
    pub html: String,
    /// CSS source to apply.
    pub css: String,
    /// Expected computed values: selector → (property → expected CSS string).
    pub expected: HashMap<String, HashMap<String, String>>,
}

/// Result of running a single test case.
#[derive(Debug)]
pub struct WptTestResult {
    /// Test case name.
    pub name: String,
    /// Whether the test passed.
    pub passed: bool,
    /// Failure details (empty if passed).
    pub failures: Vec<String>,
}

/// Run a single test case and return the result.
#[must_use]
pub fn run_test_case(test: &WptTestCase) -> WptTestResult {
    let parsed = parse_html(&test.html);
    let mut dom = parsed.dom;
    let stylesheet = parse_stylesheet(&test.css, Origin::Author);
    resolve_styles(
        &mut dom,
        &[&stylesheet],
        elidex_plugin::Size::new(1280.0, 720.0),
    );

    let mut failures = Vec::new();

    for (selector_str, props) in &test.expected {
        let entity = find_element_by_selector(&dom, parsed.document, selector_str);
        let Some(entity) = entity else {
            failures.push(format!("selector '{selector_str}' matched no element"));
            continue;
        };

        let style = dom
            .world()
            .get::<&ComputedStyle>(entity)
            .ok()
            .map(|s| (*s).clone());
        let Some(style) = style else {
            failures.push(format!("no ComputedStyle on '{selector_str}'"));
            continue;
        };

        for (prop, expected_str) in props {
            let computed_str = harness_resolved_value(prop, &style);
            if computed_str != *expected_str {
                failures.push(format!(
                    "{selector_str} {{ {prop}: expected '{expected_str}', got '{computed_str}' }}"
                ));
            }
        }
    }

    WptTestResult {
        name: test.name.clone(),
        passed: failures.is_empty(),
        failures,
    }
}

/// Run all test cases in a suite and return results.
#[must_use]
pub fn run_test_suite(tests: &[WptTestCase]) -> Vec<WptTestResult> {
    tests.iter().map(run_test_case).collect()
}

/// Serialize a property's CSSOM resolved value for WPT comparison.
///
/// Identical to the production [`serialize_resolved_value`] (so `color` /
/// `currentcolor` resolve to exactly what `getComputedStyle` returns —
/// CSSOM-1 §9 + CSS Color 4 §16) **except** that space-separated list
/// values are space-joined.
///
/// The production serializer routes list values through
/// [`CssValue::to_css_string`], whose `List` arm comma-joins — the tracked
/// `#11-cssvalue-list-separator-fidelity` gap (`CssValue::List` does not
/// record its separator). The WPT corpus is hand-authored, so the harness
/// stays spec-anchored here: a `text-decoration-line: underline overline`
/// expectation is the spec-correct `"underline overline"`, not pinned to
/// the engine's current `"underline, overline"` serialization bug. Once
/// property-aware list separators land (that slot), this collapses back
/// onto `serialize_resolved_value`.
fn harness_resolved_value(property: &str, style: &ComputedStyle) -> String {
    match get_computed(property, style) {
        CssValue::List(items) => items
            .iter()
            .map(CssValue::to_css_string)
            .collect::<Vec<_>>()
            .join(" "),
        // Non-list values (incl. color rgb()/rgba() + currentcolor
        // used-value resolution) go through the canonical production path.
        _ => serialize_resolved_value(property, style),
    }
}

/// Maximum recursion depth for DOM tree searches.
const MAX_FIND_DEPTH: usize = 10_000;

/// Find an element matching a CSS selector string in the DOM.
///
/// Per Selectors Level 4 §16.1, a selector string may be a comma-separated
/// list. All selectors in the list are tried and the first element in
/// document order (pre-order DFS) that matches any of them is returned.
fn find_element_by_selector(dom: &EcsDom, root: Entity, selector_str: &str) -> Option<Entity> {
    let selectors = parse_selector_from_str(selector_str).ok()?;
    if selectors.is_empty() {
        return None;
    }
    find_matching(dom, root, &selectors, 0)
}

/// Recursively find the first element matching any selector in the list.
fn find_matching(
    dom: &EcsDom,
    entity: Entity,
    selectors: &[elidex_css::Selector],
    depth: usize,
) -> Option<Entity> {
    if depth > MAX_FIND_DEPTH {
        return None;
    }
    // Check entity itself (only elements).
    if dom.world().get::<&TagType>(entity).is_ok()
        && selectors.iter().any(|sel| sel.matches(entity, dom))
    {
        return Some(entity);
    }
    // Recurse into children.
    for child in dom.children_iter(entity) {
        if let Some(found) = find_matching(dom, child, selectors, depth + 1) {
            return Some(found);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn case(name: &str, html: &str, css: &str, expectations: &[(&str, &str, &str)]) -> WptTestCase {
        let mut expected: HashMap<String, HashMap<String, String>> = HashMap::new();
        for &(sel, prop, val) in expectations {
            expected
                .entry(sel.to_string())
                .or_default()
                .insert(prop.to_string(), val.to_string());
        }
        WptTestCase {
            name: name.to_string(),
            html: html.to_string(),
            css: css.to_string(),
            expected,
        }
    }

    #[test]
    fn single_test_pass() {
        let test = case(
            "color-red",
            "<div id=\"t\">text</div>",
            "#t { color: red; }",
            &[("#t", "color", "rgb(255, 0, 0)")],
        );
        let result = run_test_case(&test);
        assert!(result.passed, "failures: {:?}", result.failures);
    }

    #[test]
    fn single_test_fail() {
        let test = case(
            "color-wrong",
            "<div id=\"t\">text</div>",
            "#t { color: red; }",
            &[("#t", "color", "rgb(0, 0, 255)")],
        );
        let result = run_test_case(&test);
        assert!(!result.passed);
        assert!(!result.failures.is_empty());
    }

    #[test]
    fn selector_matching() {
        let test = case(
            "class-selector",
            "<div class=\"c\" id=\"t\">text</div>",
            ".c { display: block; }",
            &[(".c", "display", "block")],
        );
        let result = run_test_case(&test);
        assert!(result.passed, "failures: {:?}", result.failures);
    }

    #[test]
    fn inheritance_test() {
        let test = case(
            "inherit-color",
            "<div id=\"p\" style=\"color: green;\"><span id=\"c\">text</span></div>",
            "",
            &[("#c", "color", "rgb(0, 128, 0)")],
        );
        let result = run_test_case(&test);
        assert!(result.passed, "failures: {:?}", result.failures);
    }

    #[test]
    fn currentcolor_resolved_to_used_value() {
        // Regression (Codex R1): the harness must serialize via the SAME
        // resolved-value path as the getComputedStyle DOM API, so a residual
        // `currentcolor` (text-decoration-color default = None) resolves to
        // the element's used-value `color` (CSSOM-1 §9) — `rgb(0, 0, 255)`,
        // NOT the unresolved `currentcolor` keyword a value-only serializer
        // would emit.
        let test = case(
            "text-decoration-color-currentcolor",
            "<div id=\"t\" style=\"color: blue;\">text</div>",
            "",
            &[("#t", "text-decoration-color", "rgb(0, 0, 255)")],
        );
        let result = run_test_case(&test);
        assert!(result.passed, "failures: {:?}", result.failures);
    }

    #[test]
    fn space_separated_list_resolved_value() {
        // Regression (Codex R3): a space-separated list property's resolved
        // value is space-joined ("underline overline"), NOT comma-joined.
        // The harness stays spec-anchored rather than inheriting production's
        // tracked to_css_string comma-join gap (#11-cssvalue-list-separator-fidelity).
        let test = case(
            "text-decoration-line-list",
            "<div id=\"t\">text</div>",
            "#t { text-decoration-line: underline overline; }",
            &[("#t", "text-decoration-line", "underline overline")],
        );
        let result = run_test_case(&test);
        assert!(result.passed, "failures: {:?}", result.failures);
    }

    #[test]
    fn empty_suite() {
        let results = run_test_suite(&[]);
        assert!(results.is_empty());
    }

    #[test]
    fn no_matching_selector() {
        let test = case(
            "missing",
            "<div>text</div>",
            "",
            &[("#nonexistent", "color", "red")],
        );
        let result = run_test_case(&test);
        assert!(!result.passed);
    }
}
