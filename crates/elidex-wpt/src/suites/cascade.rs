//! Built-in CSS cascade test cases.

use crate::WptTestCase;
use std::collections::HashMap;

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

/// Return the built-in cascade test suite.
#[must_use]
pub fn cascade_suite() -> Vec<WptTestCase> {
    vec![
        case(
            "specificity-id-beats-class",
            "<div id=\"t\" class=\"c\">text</div>",
            "#t { color: red; } .c { color: blue; }",
            &[("#t", "color", "rgb(255, 0, 0)")],
        ),
        case(
            "specificity-class-beats-tag",
            "<div class=\"c\" id=\"t\">text</div>",
            "div { color: blue; } .c { color: red; }",
            &[("#t", "color", "rgb(255, 0, 0)")],
        ),
        case(
            "important-beats-specificity",
            "<div id=\"t\" class=\"c\">text</div>",
            ".c { color: red !important; } #t { color: blue; }",
            &[("#t", "color", "rgb(255, 0, 0)")],
        ),
        case(
            "source-order-last-wins",
            "<div id=\"t\">text</div>",
            "#t { color: red; } #t { color: blue; }",
            &[("#t", "color", "rgb(0, 0, 255)")],
        ),
        case(
            "inheritance-color",
            "<div id=\"p\" style=\"color: green;\"><span id=\"c\">text</span></div>",
            "",
            &[("#c", "color", "rgb(0, 128, 0)")],
        ),
        case(
            "ua-stylesheet-div-display-block",
            "<div id=\"t\">text</div>",
            "",
            &[("#t", "display", "block")],
        ),
        case(
            "display-none",
            "<div id=\"t\">text</div>",
            "#t { display: none; }",
            &[("#t", "display", "none")],
        ),
        case(
            "display-flex",
            "<div id=\"t\">text</div>",
            "#t { display: flex; }",
            &[("#t", "display", "flex")],
        ),
        case(
            "font-size-px",
            "<div id=\"t\">text</div>",
            "#t { font-size: 24px; }",
            &[("#t", "font-size", "24px")],
        ),
        case(
            "multiple-properties",
            "<div id=\"t\">text</div>",
            "#t { color: red; display: flex; font-size: 20px; }",
            &[
                ("#t", "color", "rgb(255, 0, 0)"),
                ("#t", "display", "flex"),
                ("#t", "font-size", "20px"),
            ],
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run_test_suite;

    #[test]
    fn run_cascade_suite_all_pass() {
        let suite = cascade_suite();
        let results = run_test_suite(&suite);
        for result in &results {
            assert!(
                result.passed,
                "FAIL: {} — {:?}",
                result.name, result.failures
            );
        }
        assert_eq!(results.len(), 10);
    }
}
