//! Custom element name validation (WHATWG HTML §4.13.2).

/// Reserved element names that cannot be used as custom element names.
const RESERVED_NAMES: &[&str] = &[
    "annotation-xml",
    "color-profile",
    "font-face",
    "font-face-src",
    "font-face-uri",
    "font-face-format",
    "font-face-name",
    "missing-glyph",
];

/// Check whether a name is a valid custom element name per WHATWG HTML §4.13.2.
///
/// Valid custom element names must:
/// - Start with a lowercase ASCII letter (`a-z`)
/// - Contain at least one hyphen (`-`)
/// - Not be in the reserved names list
/// - Only contain ASCII lowercase, digits, `-`, `_`, `.`, and non-ASCII characters
#[must_use]
pub fn is_valid_custom_element_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    // Must start with lowercase ASCII letter.
    let first = name.as_bytes()[0];
    if !first.is_ascii_lowercase() {
        return false;
    }
    // Must contain a hyphen.
    if !name.contains('-') {
        return false;
    }
    // Must not be a reserved name.
    if RESERVED_NAMES.contains(&name) {
        return false;
    }
    // Names starting with "xml" are reserved per XML namespace restrictions
    // (Namespaces in XML §3). Reject to avoid conflicts with XML-prefixed names.
    if name.starts_with("xml") {
        return false;
    }
    // All characters must be valid: [a-z0-9._-] or non-ASCII.
    name.chars().all(|c| {
        c.is_ascii_lowercase()
            || c.is_ascii_digit()
            || c == '-'
            || c == '_'
            || c == '.'
            || !c.is_ascii()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_names() {
        assert!(is_valid_custom_element_name("my-element"));
        assert!(is_valid_custom_element_name("x-foo"));
        assert!(is_valid_custom_element_name("app-header"));
        assert!(is_valid_custom_element_name("my-component.v2"));
        assert!(is_valid_custom_element_name("my_custom-element"));
    }

    #[test]
    fn invalid_names() {
        assert!(!is_valid_custom_element_name("")); // empty
        assert!(!is_valid_custom_element_name("div")); // no hyphen
        assert!(!is_valid_custom_element_name("MyElement")); // uppercase start
        assert!(!is_valid_custom_element_name("1-element")); // digit start
        assert!(!is_valid_custom_element_name("-element")); // hyphen start
        assert!(!is_valid_custom_element_name("font-face")); // reserved
        assert!(!is_valid_custom_element_name("annotation-xml")); // reserved
        assert!(!is_valid_custom_element_name("xml-button")); // xml prefix reserved
        assert!(!is_valid_custom_element_name("xmlns-foo")); // xml prefix reserved
    }
}
