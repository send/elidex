//! JSON test case format parsing.

use crate::WptTestCase;

/// Parse a JSON string containing an array of test cases.
///
/// # Errors
///
/// Returns an error if the JSON is malformed or does not match the expected schema.
pub fn parse_test_cases(json: &str) -> Result<Vec<WptTestCase>, String> {
    serde_json::from_str(json).map_err(|e| format!("JSON parse error: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_test_cases_basic() {
        let json = r##"[{
            "name": "test1",
            "html": "<div id=\"t\">hi</div>",
            "css": "#t { color: red; }",
            "expected": { "#t": { "color": "red" } }
        }]"##;
        let cases = parse_test_cases(json).unwrap();
        assert_eq!(cases.len(), 1);
        assert_eq!(cases[0].name, "test1");
        assert_eq!(cases[0].expected["#t"]["color"], "red");
    }

    #[test]
    fn malformed_json_error() {
        let result = parse_test_cases("not json");
        assert!(result.is_err());
    }
}
