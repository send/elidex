//! Error types for the plugin system.

use std::fmt;

/// Error returned when CSS value parsing fails.
///
/// Contains the property name, the invalid input, and a human-readable message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    /// The CSS property that failed to parse (e.g. `"color"`).
    pub property: String,
    /// The invalid input string.
    pub input: String,
    /// A human-readable description of the error.
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CSS parse error for '{}': {} (input: '{}')",
            self.property, self.message, self.input
        )
    }
}

impl std::error::Error for ParseError {}

impl ParseError {
    /// Construct a `ParseError` with only a message, leaving `property` and
    /// `input` empty.
    #[must_use]
    pub fn simple(message: impl Into<String>) -> Self {
        Self {
            property: String::new(),
            input: String::new(),
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_error_display() {
        let e = ParseError {
            property: "color".into(),
            input: "abc".into(),
            message: "invalid color value".into(),
        };
        let s = e.to_string();
        assert!(s.contains("color"));
        assert!(s.contains("abc"));
        assert!(s.contains("invalid color value"));
    }

    #[test]
    fn parse_error_is_error() {
        let e = ParseError {
            property: "width".into(),
            input: "foo".into(),
            message: "expected length".into(),
        };
        let _: &dyn std::error::Error = &e;
    }
}
