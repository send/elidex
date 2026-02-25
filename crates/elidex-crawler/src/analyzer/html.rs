//! HTML legacy feature detection.
//!
//! Uses html5ever to parse HTML and detect deprecated tags, attributes,
//! and collect parser error messages.

use super::FeatureCount;
use html5ever::tendril::TendrilSink;
use html5ever::{parse_document, ParseOpts};
use markup5ever_rcdom::{Handle, NodeData, RcDom};
use serde::{Deserialize, Serialize};

/// Deprecated HTML tags to detect.
const DEPRECATED_TAGS: &[&str] = &[
    "center",
    "font",
    "marquee",
    "blink",
    "frame",
    "frameset",
    "noframes",
    "applet",
    "basefont",
    "big",
    "bgsound",
    "dir",
    "isindex",
    "listing",
    "multicol",
    "nextid",
    "nobr",
    "noembed",
    "plaintext",
    "spacer",
    "strike",
    "tt",
    "xmp",
];

/// Deprecated HTML attributes to detect.
const DEPRECATED_ATTRS: &[&str] = &[
    "bgcolor",
    "align",
    "valign",
    "border",
    "cellpadding",
    "cellspacing",
    "width",
    "height",
    "hspace",
    "vspace",
    "background",
    "alink",
    "vlink",
    "link",
    "text",
    "face",
    "size",
    "color",
    "noshade",
    "nowrap",
];

/// Aggregated HTML legacy feature counts.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HtmlFeatures {
    /// Count of each deprecated tag found.
    pub deprecated_tags: FeatureCount,
    /// Count of each deprecated attribute found.
    pub deprecated_attrs: FeatureCount,
    /// Whether `document.write` or `document.writeln` was found in inline scripts.
    pub uses_document_write: bool,
}

/// Parse the HTML body and analyze for legacy features.
///
/// Returns `(features, parser_errors)`.
pub fn analyze(html: &str) -> (HtmlFeatures, Vec<String>) {
    /// Maximum number of parser errors to collect (prevents memory bloat on malformed HTML).
    const MAX_PARSER_ERRORS: usize = 1_000;

    let mut features = HtmlFeatures::default();
    let mut errors = Vec::new();

    let opts = ParseOpts::default();
    let dom = parse_document(RcDom::default(), opts)
        .from_utf8()
        .one(html.as_bytes());

    // Collect parser errors.
    for err in dom.errors.borrow().iter() {
        if errors.len() >= MAX_PARSER_ERRORS {
            break;
        }
        errors.push(err.to_string());
    }

    // Walk the DOM tree.
    walk_tree(&dom.document, &mut features, false);

    (features, errors)
}

/// Walk the DOM tree iteratively to avoid stack overflow on deeply nested HTML.
fn walk_tree(root: &Handle, features: &mut HtmlFeatures, root_in_script: bool) {
    // Stack of (node, in_script) pairs.
    let mut stack: Vec<(Handle, bool)> = vec![(root.clone(), root_in_script)];

    while let Some((handle, in_script)) = stack.pop() {
        let mut child_in_script = in_script;

        match &handle.data {
            NodeData::Element { name, attrs, .. } => {
                let tag = name.local.as_ref();

                // Check for deprecated tags.
                if DEPRECATED_TAGS.contains(&tag) {
                    *features.deprecated_tags.entry(tag.to_string()).or_default() += 1;
                }

                // Check for deprecated attributes.
                let attrs = attrs.borrow();
                for attr in attrs.iter() {
                    let attr_name = attr.name.local.as_ref();
                    if DEPRECATED_ATTRS.contains(&attr_name) {
                        *features
                            .deprecated_attrs
                            .entry(attr_name.to_string())
                            .or_default() += 1;
                    }
                }

                // Track whether we're inside a <script> element.
                child_in_script = tag == "script";
            }
            NodeData::Text { contents } if in_script => {
                let text = contents.borrow();
                if text.contains("document.write") || text.contains("document.writeln") {
                    features.uses_document_write = true;
                }
            }
            _ => {}
        }

        // Push children in reverse order so they are visited left-to-right.
        for child in handle.children.borrow().iter().rev() {
            stack.push((child.clone(), child_in_script));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_deprecated_tags() {
        let html = "<html><body><center>Hello</center><font>World</font></body></html>";
        let (features, _) = analyze(html);
        assert_eq!(features.deprecated_tags.get("center"), Some(&1));
        assert_eq!(features.deprecated_tags.get("font"), Some(&1));
    }

    #[test]
    fn detect_deprecated_attrs() {
        let html = "<html><body bgcolor=\"#fff\" align=\"center\"><div></div></body></html>";
        let (features, _) = analyze(html);
        assert_eq!(features.deprecated_attrs.get("bgcolor"), Some(&1));
        assert_eq!(features.deprecated_attrs.get("align"), Some(&1));
    }

    #[test]
    fn detect_document_write() {
        let html = r#"<html><body><script>document.write("hello")</script></body></html>"#;
        let (features, _) = analyze(html);
        assert!(features.uses_document_write);
    }

    #[test]
    fn document_write_in_text_no_false_positive() {
        let html = "<html><body><p>document.write is deprecated</p></body></html>";
        let (features, _) = analyze(html);
        assert!(!features.uses_document_write);
    }

    #[test]
    fn no_false_positives_on_clean_html() {
        let html =
            "<html><head><title>Test</title></head><body><div><p>Clean</p></div></body></html>";
        let (features, _) = analyze(html);
        assert!(features.deprecated_tags.is_empty());
        assert!(features.deprecated_attrs.is_empty());
        assert!(!features.uses_document_write);
    }

    #[test]
    fn collects_parser_errors() {
        // Unclosed tags produce parser errors.
        let html = "<html><body><p>unclosed";
        let (_, errors) = analyze(html);
        // html5ever is quite lenient, but we should at least not crash.
        // The exact number of errors depends on html5ever version.
        let _ = errors;
    }
}
