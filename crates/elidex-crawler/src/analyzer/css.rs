//! CSS legacy feature detection.
//!
//! Extracts `<style>` blocks and inline `style` attributes from HTML,
//! then scans for vendor prefixes, non-standard properties, and aliased properties.
//! CSS comments (`/* ... */`) are stripped before scanning to reduce false positives.

use super::{util, FeatureCount, MAX_EXTRACT_ITERATIONS};
use serde::{Deserialize, Serialize};

/// Vendor prefixes to detect.
const VENDOR_PREFIXES: &[&str] = &["-webkit-", "-moz-", "-ms-", "-o-"];

/// Non-standard CSS properties.
const NON_STANDARD_PROPERTIES: &[&str] = &[
    "zoom",
    "-webkit-appearance",
    "-moz-appearance",
    "-webkit-text-size-adjust",
    "-moz-text-size-adjust",
    "-ms-text-size-adjust",
    "-webkit-font-smoothing",
    "-moz-osx-font-smoothing",
    "-webkit-tap-highlight-color",
    "-webkit-overflow-scrolling",
    "-ms-overflow-style",
];

/// Aliased properties (old name -> standard name).
const ALIASED_PROPERTIES: &[(&str, &str)] = &[
    ("word-wrap", "overflow-wrap"),
    ("-webkit-box-orient", "flex-direction"),
    ("-webkit-box-pack", "justify-content"),
    ("-webkit-box-align", "align-items"),
];

/// Aggregated CSS legacy feature counts.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CssFeatures {
    /// Vendor prefix usage counts by prefix.
    pub vendor_prefixes: FeatureCount,
    /// Non-standard property usage counts.
    pub non_standard_properties: FeatureCount,
    /// Aliased property usage counts.
    pub aliased_properties: FeatureCount,
}

/// Analyze HTML body for CSS legacy features.
///
/// Extracts CSS from `<style>` blocks and inline `style` attributes,
/// then scans for legacy patterns.
pub fn analyze(html: &str) -> CssFeatures {
    let mut features = CssFeatures::default();

    for css in util::extract_tag_blocks(html, "style", false) {
        let cleaned = util::strip_comments(&css, false, false);
        scan_css(&cleaned, &mut features);
    }

    for css in extract_inline_styles(html) {
        scan_css(&css, &mut features);
    }

    features
}

fn extract_inline_styles(html: &str) -> Vec<String> {
    let mut styles = Vec::new();
    let lower = html.to_ascii_lowercase();
    let mut search_from = 0;
    let mut iterations = 0;

    // Match style="...", style='...', and also style = "..." (with whitespace).
    while search_from < lower.len() {
        iterations += 1;
        if iterations > MAX_EXTRACT_ITERATIONS {
            break;
        }
        let Some(pos) = lower[search_from..].find("style") else {
            break;
        };
        let abs_pos = search_from + pos;
        let after_style = abs_pos + 5; // skip "style"

        // Skip whitespace between "style" and "=".
        let rest = &lower[after_style..];
        let trimmed = rest.trim_start();
        let ws_len = rest.len() - trimmed.len();

        if !trimmed.starts_with('=') {
            search_from = after_style;
            continue;
        }
        let after_eq = after_style + ws_len + 1; // skip "="

        // Skip whitespace between "=" and the opening quote.
        let rest2 = &lower[after_eq..];
        let trimmed2 = rest2.trim_start();
        let ws_len2 = rest2.len() - trimmed2.len();

        let quote = match trimmed2.as_bytes().first() {
            Some(b'"') => '"',
            Some(b'\'') => '\'',
            _ => {
                search_from = after_eq;
                continue;
            }
        };
        let value_start = after_eq + ws_len2 + 1;

        let Some(end) = html[value_start..].find(quote) else {
            break;
        };
        styles.push(html[value_start..value_start + end].to_string());
        search_from = value_start + end + 1;
    }

    styles
}

fn scan_css(css: &str, features: &mut CssFeatures) {
    for line in css.lines() {
        let trimmed = line.trim();

        for prefix in VENDOR_PREFIXES {
            if trimmed.contains(prefix) {
                *features
                    .vendor_prefixes
                    .entry((*prefix).to_string())
                    .or_default() += 1;
            }
        }

        for prop in NON_STANDARD_PROPERTIES {
            if contains_property(trimmed, prop) {
                *features
                    .non_standard_properties
                    .entry((*prop).to_string())
                    .or_default() += 1;
            }
        }

        for (alias, _standard) in ALIASED_PROPERTIES {
            if contains_property(trimmed, alias) {
                *features
                    .aliased_properties
                    .entry((*alias).to_string())
                    .or_default() += 1;
            }
        }
    }
}

/// Check if a CSS line contains a specific property declaration.
///
/// Uses word-boundary checking to avoid false positives where `property` is a
/// substring of a longer property name (e.g. `my-zoom` should not match `zoom`).
fn contains_property(line: &str, property: &str) -> bool {
    let mut search_from = 0;
    while let Some(rel_pos) = line[search_from..].find(property) {
        let pos = search_from + rel_pos;

        // Check that the character before the match is a valid property boundary.
        let boundary_before = if pos == 0 {
            true
        } else {
            let before = line.as_bytes()[pos - 1];
            matches!(before, b' ' | b'\t' | b'{' | b';' | b'\n' | b'\r')
        };

        if boundary_before {
            let after = &line[pos + property.len()..];
            let after_trimmed = after.trim_start();
            if after_trimmed.starts_with(':') {
                return true;
            }
        }

        search_from = pos + 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_vendor_prefixes() {
        let html = r"<html><head><style>
            .foo { -webkit-transform: rotate(45deg); }
            .bar { -moz-border-radius: 5px; }
        </style></head><body></body></html>";

        let features = analyze(html);
        assert_eq!(features.vendor_prefixes.get("-webkit-"), Some(&1));
        assert_eq!(features.vendor_prefixes.get("-moz-"), Some(&1));
    }

    #[test]
    fn detect_non_standard_properties() {
        let html = r"<html><head><style>
            .foo { zoom: 1; }
            .bar { -webkit-appearance: none; }
        </style></head><body></body></html>";

        let features = analyze(html);
        assert_eq!(features.non_standard_properties.get("zoom"), Some(&1));
        assert_eq!(
            features.non_standard_properties.get("-webkit-appearance"),
            Some(&1)
        );
    }

    #[test]
    fn detect_aliased_properties() {
        let html = r"<html><head><style>
            .foo { word-wrap: break-word; }
        </style></head><body></body></html>";

        let features = analyze(html);
        assert_eq!(features.aliased_properties.get("word-wrap"), Some(&1));
    }

    #[test]
    fn detect_inline_styles() {
        let html = r#"<html><body><div style="-webkit-transform: scale(1)">hi</div></body></html>"#;

        let features = analyze(html);
        assert_eq!(features.vendor_prefixes.get("-webkit-"), Some(&1));
    }

    #[test]
    fn detect_single_quoted_inline_styles() {
        let html = "<html><body><div style='-webkit-transform: scale(1)'>hi</div></body></html>";

        let features = analyze(html);
        assert_eq!(features.vendor_prefixes.get("-webkit-"), Some(&1));
    }

    #[test]
    fn no_false_positives_on_clean_css() {
        let html = r"<html><head><style>
            .foo { color: red; margin: 0; }
        </style></head><body></body></html>";

        let features = analyze(html);
        assert!(features.vendor_prefixes.is_empty());
        assert!(features.non_standard_properties.is_empty());
        assert!(features.aliased_properties.is_empty());
    }

    #[test]
    fn css_comments_ignored() {
        let html = r"<html><head><style>
            /* -webkit-transform: rotate(45deg); */
            .foo { color: red; }
        </style></head><body></body></html>";

        let features = analyze(html);
        assert!(features.vendor_prefixes.is_empty());
    }

    #[test]
    fn no_false_positive_on_substring_property() {
        let html = r"<html><head><style>
            .foo { my-zoom: 2; }
        </style></head><body></body></html>";
        let features = analyze(html);
        assert!(
            !features.non_standard_properties.contains_key("zoom"),
            "my-zoom should not match zoom"
        );
    }

    #[test]
    fn property_at_start_of_line() {
        let html = r"<html><head><style>
zoom: 1;
        </style></head><body></body></html>";
        let features = analyze(html);
        assert_eq!(features.non_standard_properties.get("zoom"), Some(&1));
    }

    #[test]
    fn non_ascii_html_does_not_panic() {
        let html = r"<html><head><style>
            .emoji { -webkit-transform: rotate(45deg); }
        </style></head><body>😀日本語テスト</body></html>";
        let features = analyze(html);
        assert_eq!(features.vendor_prefixes.get("-webkit-"), Some(&1));
    }
}
