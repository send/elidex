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

/// Error returned during HTML element creation or insertion.
///
/// Contains the tag name, error kind, and a human-readable message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HtmlParseError {
    /// The HTML tag that caused the error (e.g. `"div"`).
    pub tag: String,
    /// The kind of HTML parsing error.
    pub kind: HtmlErrorKind,
    /// A human-readable description of the error.
    pub message: String,
}

impl fmt::Display for HtmlParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "HTML parse error for <{}>: {}: {}",
            self.tag, self.kind, self.message
        )
    }
}

impl std::error::Error for HtmlParseError {}

/// The kind of HTML parsing error.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum HtmlErrorKind {
    /// The tag name is invalid.
    InvalidTag,
    /// An attribute is invalid.
    InvalidAttribute,
    /// The element nesting is invalid.
    InvalidNesting,
    /// An unclassified error.
    Other,
}

impl fmt::Display for HtmlErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTag => f.write_str("invalid tag"),
            Self::InvalidAttribute => f.write_str("invalid attribute"),
            Self::InvalidNesting => f.write_str("invalid nesting"),
            Self::Other => f.write_str("other error"),
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

    #[test]
    fn html_parse_error_display() {
        let e = HtmlParseError {
            tag: "script".into(),
            kind: HtmlErrorKind::InvalidNesting,
            message: "cannot nest script elements".into(),
        };
        let s = e.to_string();
        assert!(s.contains("<script>"));
        assert!(s.contains("invalid nesting"));
        assert!(s.contains("cannot nest script elements"));
    }

    #[test]
    fn html_parse_error_is_error() {
        let e = HtmlParseError {
            tag: "div".into(),
            kind: HtmlErrorKind::Other,
            message: "unknown error".into(),
        };
        let _: &dyn std::error::Error = &e;
    }

    #[test]
    fn html_error_kind_eq() {
        assert_eq!(HtmlErrorKind::InvalidTag, HtmlErrorKind::InvalidTag);
        assert_ne!(HtmlErrorKind::InvalidTag, HtmlErrorKind::InvalidAttribute);
    }
}
