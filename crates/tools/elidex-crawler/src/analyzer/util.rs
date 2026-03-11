//! Shared utilities for HTML content extraction and source-code comment stripping.

use super::MAX_EXTRACT_ITERATIONS;

/// Extract content between matching HTML tag pairs (e.g. `<style>…</style>`).
///
/// When `skip_src` is true, tags containing a `src=` attribute are skipped
/// (used for `<script src="…">` external scripts).
///
/// Uses `to_ascii_lowercase()` for case-insensitive matching while preserving
/// original byte positions for slicing the source HTML.
pub(crate) fn extract_tag_blocks(html: &str, tag: &str, skip_src: bool) -> Vec<String> {
    let mut blocks = Vec::new();
    let lower = html.to_ascii_lowercase();
    let open_tag = format!("<{tag}");
    let close_tag = format!("</{tag}");
    let mut search_from = 0;
    let mut iterations = 0;

    while let Some(start) = lower[search_from..].find(&*open_tag) {
        iterations += 1;
        if iterations > MAX_EXTRACT_ITERATIONS {
            break;
        }
        let abs_start = search_from + start;
        let Some(tag_end) = lower[abs_start..].find('>') else {
            break;
        };

        if skip_src {
            let tag_content = &lower[abs_start..abs_start + tag_end];
            if tag_content.contains("src=") {
                search_from = abs_start + tag_end + 1;
                continue;
            }
        }

        let content_start = abs_start + tag_end + 1;
        let Some(end) = lower[content_start..].find(&*close_tag) else {
            break;
        };
        let content_end = content_start + end;

        blocks.push(html[content_start..content_end].to_string());
        search_from = content_end;
    }

    blocks
}

/// Strip comments from source code while preserving string literal contents.
///
/// - Block comments (`/* … */`) are always stripped.
/// - When `single_line` is true, single-line comments (`// …`) are also stripped.
/// - When `backtick_strings` is true, backtick-delimited template literals
///   (`` ` … ` ``) are preserved in addition to `"` and `'` strings.
///
/// Operates on `char` boundaries to correctly handle multi-byte UTF-8.
pub(crate) fn strip_comments(source: &str, single_line: bool, backtick_strings: bool) -> String {
    let mut result = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();

    while let Some(&c) = chars.peek() {
        // String literals: skip through to preserve contents.
        if c == '"' || c == '\'' || (backtick_strings && c == '`') {
            let quote = c;
            result.push(chars.next().unwrap());
            while let Some(&sc) = chars.peek() {
                if sc == '\\' {
                    result.push(chars.next().unwrap());
                    if chars.peek().is_some() {
                        result.push(chars.next().unwrap());
                    }
                } else if sc == quote {
                    result.push(chars.next().unwrap());
                    break;
                } else {
                    result.push(chars.next().unwrap());
                }
            }
        }
        // Comments
        else if c == '/' {
            chars.next();
            match chars.peek() {
                // Block comment: /* … */
                Some(&'*') => {
                    chars.next();
                    let mut prev = '\0';
                    for ch in chars.by_ref() {
                        if prev == '*' && ch == '/' {
                            break;
                        }
                        prev = ch;
                    }
                }
                // Single-line comment: // …
                Some(&'/') if single_line => {
                    chars.next();
                    for ch in chars.by_ref() {
                        if ch == '\n' {
                            result.push('\n');
                            break;
                        }
                    }
                }
                _ => result.push('/'),
            }
        }
        // Normal character
        else {
            result.push(chars.next().unwrap());
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- extract_tag_blocks ---

    #[test]
    fn extract_style_blocks() {
        let html = "<html><head><STYLE>.a{}</STYLE><style>.b{}</style></head></html>";
        let blocks = extract_tag_blocks(html, "style", false);
        assert_eq!(blocks, vec![".a{}", ".b{}"]);
    }

    #[test]
    fn extract_script_blocks_skips_src() {
        let html = r#"<script src="a.js">ignored</script><script>kept</script>"#;
        let blocks = extract_tag_blocks(html, "script", true);
        assert_eq!(blocks, vec!["kept"]);
    }

    #[test]
    fn extract_empty_tag() {
        let html = "<style></style>";
        let blocks = extract_tag_blocks(html, "style", false);
        assert_eq!(blocks, vec![""]);
    }

    #[test]
    fn extract_unclosed_tag() {
        let html = "<style>.a {}";
        let blocks = extract_tag_blocks(html, "style", false);
        assert!(blocks.is_empty());
    }

    // --- strip_comments ---

    #[test]
    fn strip_css_block_comments() {
        let css = ".a { /* old */ color: red; } /* end */";
        let stripped = strip_comments(css, false, false);
        assert_eq!(stripped, ".a {  color: red; } ");
    }

    #[test]
    fn strip_css_preserves_strings() {
        let css = r#".a { background: url("/*not*/img.png"); }"#;
        let stripped = strip_comments(css, false, false);
        assert!(stripped.contains("/*not*/"));
    }

    #[test]
    fn strip_js_both_comment_types() {
        let js = "a(); // comment\nb(); /* block */ c();";
        let stripped = strip_comments(js, true, true);
        assert_eq!(stripped, "a(); \nb();  c();");
    }

    #[test]
    fn strip_js_preserves_strings() {
        let js = r#"var s = "// not a comment"; a();"#;
        let stripped = strip_comments(js, true, true);
        assert!(stripped.contains("// not a comment"));
    }

    #[test]
    fn strip_preserves_backtick_strings() {
        let js = "let s = `/* not a comment */`; a();";
        let stripped = strip_comments(js, true, true);
        assert!(stripped.contains("/* not a comment */"));
    }

    #[test]
    fn strip_without_backtick_support() {
        // CSS mode: backtick is not a string delimiter
        let css = "`.a { color: red; }`";
        let stripped = strip_comments(css, false, false);
        assert_eq!(stripped, css); // unchanged, backtick is normal char
    }
}
