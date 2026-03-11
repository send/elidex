//! JavaScript legacy pattern detection (static analysis).
//!
//! Scans `<script>` block contents for legacy JS patterns.
//! Single-line (`//`) and block (`/* */`) comments are stripped before
//! scanning to reduce false positives.

use super::{util, FeatureCount};
use serde::{Deserialize, Serialize};

/// Legacy JS patterns to detect in script content.
const LEGACY_PATTERNS: &[(&str, &str)] = &[
    ("document.write(", "document.write"),
    ("document.writeln(", "document.writeln"),
    ("document.all", "document.all"),
    ("document.layers", "document.layers"),
    ("arguments.callee", "arguments.callee"),
    ("__proto__", "__proto__"),
    (".attachEvent(", "attachEvent"),
    ("window.showModalDialog", "showModalDialog"),
    ("document.createStyleSheet", "createStyleSheet"),
];

/// Aggregated JS feature usage.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JsFeatures {
    /// Count of each legacy pattern found in script blocks.
    pub patterns: FeatureCount,
}

/// Analyze HTML body for JavaScript legacy patterns.
pub fn analyze(html: &str) -> JsFeatures {
    let mut features = JsFeatures::default();

    for script in util::extract_tag_blocks(html, "script", true) {
        let cleaned = util::strip_comments(&script, true, true);
        scan_js(&cleaned, &mut features);
    }

    features
}

fn scan_js(script: &str, features: &mut JsFeatures) {
    for (needle, key) in LEGACY_PATTERNS {
        let count = script.matches(needle).count();
        if count > 0 {
            *features.patterns.entry((*key).to_string()).or_default() += count;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_document_write_in_script() {
        let html = r#"<html><body><script>document.write("hello"); document.write("world");</script></body></html>"#;
        let features = analyze(html);
        assert_eq!(features.patterns.get("document.write"), Some(&2));
    }

    #[test]
    fn detect_document_all() {
        let html = r"<html><body><script>if (document.all) { }</script></body></html>";
        let features = analyze(html);
        assert_eq!(features.patterns.get("document.all"), Some(&1));
    }

    #[test]
    fn no_false_positives_on_clean_js() {
        let html = r#"<html><body><script>console.log("hello");</script></body></html>"#;
        let features = analyze(html);
        assert!(!features.patterns.contains_key("document.write"));
    }

    #[test]
    fn multiple_script_blocks() {
        let html = r#"<html><body>
            <script>document.write("a");</script>
            <script>document.write("b");</script>
        </body></html>"#;
        let features = analyze(html);
        assert_eq!(features.patterns.get("document.write"), Some(&2));
    }

    #[test]
    fn block_comments_ignored() {
        let html = r"<html><body><script>
            /* document.write('old code'); */
            console.log('clean');
        </script></body></html>";
        let features = analyze(html);
        assert!(!features.patterns.contains_key("document.write"));
    }

    #[test]
    fn single_line_comments_ignored() {
        let html = "<html><body><script>\n// document.write('old');\nconsole.log('ok');\n</script></body></html>";
        let features = analyze(html);
        assert!(!features.patterns.contains_key("document.write"));
    }

    #[test]
    fn patterns_in_strings_preserved() {
        let html = r#"<html><body><script>var x = "document.write(";</script></body></html>"#;
        let features = analyze(html);
        assert!(features.patterns.contains_key("document.write"));
    }

    #[test]
    fn external_scripts_skipped() {
        let html =
            r#"<html><body><script src="app.js">document.write("x");</script></body></html>"#;
        let features = analyze(html);
        assert!(!features.patterns.contains_key("document.write"));
    }

    #[test]
    fn non_ascii_html_does_not_panic() {
        let html = r#"<html><body>😀日本語<script>document.write("x");</script></body></html>"#;
        let features = analyze(html);
        assert_eq!(features.patterns.get("document.write"), Some(&1));
    }
}
