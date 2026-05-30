//! Strict HTML parsing error type.
//!
//! Per WHATWG HTML §13.2.2 "Parse errors", strict mode reports the first
//! parse error encountered and aborts parsing (no error recovery). This
//! contrasts with tolerant mode which collects errors and continues.

use std::fmt;

/// Errors collected during strict-mode parsing.
///
/// Per the Phase A plan (`m4-12-pr-html-parser-strict-phase-a-plan.md`),
/// this type is the SoT contract for strict parser errors. The companion
/// compat crate (`elidex-html-parser`) re-exports this from A4 onward via
/// `pub use elidex_html_parser_strict::StrictParseError`, preserving the
/// existing caller import path `use elidex_html_parser::StrictParseError`.
#[derive(Debug, Clone)]
pub struct StrictParseError {
    /// Parse error messages encountered during strict parsing.
    ///
    /// In Phase A1 skeleton stage, this carries a single
    /// `"unimplemented: tokenizer pending A2 / tree builder pending A3"`
    /// entry from the stub `parse_strict`. Once A2-A4 land, populated with
    /// WHATWG HTML §13.2.2 parse-error names (e.g. `"missing-attribute-value"`,
    /// `"unexpected-character-in-attribute-name"`).
    pub errors: Vec<String>,
}

impl fmt::Display for StrictParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "strict parse failed with {} error(s):",
            self.errors.len()
        )?;
        for err in &self.errors {
            write!(f, "\n  - {err}")?;
        }
        Ok(())
    }
}

impl std::error::Error for StrictParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_parse_error_display() {
        let err = StrictParseError {
            errors: vec!["unexpected tag".to_string(), "missing close".to_string()],
        };
        let display = err.to_string();
        assert!(display.contains("2 error(s)"));
        assert!(display.contains("unexpected tag"));
        assert!(display.contains("missing close"));
    }

    #[test]
    fn strict_parse_error_empty_errors() {
        let err = StrictParseError { errors: vec![] };
        let display = err.to_string();
        assert!(display.contains("0 error(s)"));
    }
}
