//! Strict HTML parsing mode.
//!
//! Rejects documents that produce any html5ever parse errors, useful for
//! development-time markup validation.

use std::fmt;

use crate::ParseResult;

/// Errors collected during strict-mode parsing.
#[derive(Debug, Clone)]
pub struct StrictParseError {
    /// html5ever parse error messages.
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

/// Parse HTML in strict mode — any html5ever error results in `Err`.
///
/// Useful for development-time validation of markup. The underlying
/// parse still uses html5ever's error-recovering tree builder, but the
/// caller can treat errors as fatal.
pub fn parse_strict(html: &str) -> Result<ParseResult, StrictParseError> {
    let result = crate::parse_html(html);
    if result.errors.is_empty() {
        Ok(result)
    } else {
        Err(StrictParseError {
            errors: result.errors,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_valid_html() {
        let html = "<!DOCTYPE html><html><head></head><body><p>Hello</p></body></html>";
        let result = parse_strict(html);
        assert!(result.is_ok());
    }

    #[test]
    fn strict_invalid_html() {
        // Mismatched tags produce parse errors.
        let html = "<div><span></div>";
        let result = parse_strict(html);
        assert!(result.is_err());
    }

    #[test]
    fn strict_error_messages() {
        let html = "<div><span></div>";
        let err = parse_strict(html).unwrap_err();
        assert!(!err.errors.is_empty());
        // Each error should be a non-empty string.
        for msg in &err.errors {
            assert!(!msg.is_empty());
        }
    }

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
}
