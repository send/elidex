//! CSS vendor prefix removal.
//!
//! Strips `-webkit-`, `-moz-`, `-ms-`, `-o-` prefixes from CSS property names
//! at the text level, before parsing.
//!
//! # Phase 4 TODO
//!
//! - `-webkit-box-*` old flexbox → new flexbox mapping (non-trivial)
//! - `-webkit-text-size-adjust`, `-webkit-font-smoothing` (non-standard, drop)

use elidex_css::{Origin, Stylesheet};

/// Known vendor prefixes to strip.
const VENDOR_PREFIXES: &[&str] = &["-webkit-", "-moz-", "-ms-", "-o-"];

/// Strip vendor prefixes from CSS property names.
///
/// Operates at the text level: finds property name positions (after `{` or `;`
/// or start of line) and removes any leading vendor prefix.
///
/// Uses an explicit state machine: `PropStart` (expecting a property name),
/// `Normal` (within a value or selector), and `InString` (inside a quoted
/// string literal). CSS comments (`/* ... */`) are consumed eagerly in both
/// `PropStart` and `Normal` states.
///
/// # Example
///
/// ```
/// use elidex_dom_compat::strip_vendor_prefixes;
/// let input = "div { -webkit-border-radius: 5px; -moz-transform: none; }";
/// let output = strip_vendor_prefixes(input);
/// assert!(output.contains("border-radius: 5px"));
/// assert!(output.contains("transform: none"));
/// ```
#[must_use]
pub fn strip_vendor_prefixes(css: &str) -> String {
    /// Parser state for the vendor prefix stripper.
    #[derive(Clone, Copy)]
    enum State {
        /// Normal CSS content — not at a property name position.
        Normal,
        /// At a property-name position (after `{`, `;`, `}`, or at start).
        PropStart,
        /// Inside a string literal (stores the opening quote character).
        InString(char),
        /// Inside a `url(...)` function — skip until closing `)`.
        InUrl,
    }

    let mut result = String::with_capacity(css.len());
    let mut chars = css.char_indices().peekable();
    let mut state = State::PropStart;

    while let Some(&(i, ch)) = chars.peek() {
        match state {
            State::InUrl => {
                result.push(ch);
                chars.next();
                if ch == ')' {
                    state = State::Normal;
                } else if ch == '"' || ch == '\'' {
                    // Quoted string inside url(): skip until closing quote.
                    // e.g. url("path-with-)-in-it.png")
                    let quote = ch;
                    while let Some(&(_, c)) = chars.peek() {
                        result.push(c);
                        chars.next();
                        if c == '\\' {
                            if let Some(&(_, esc)) = chars.peek() {
                                result.push(esc);
                                chars.next();
                            }
                        } else if c == quote {
                            break;
                        }
                    }
                } else if ch == '\\' {
                    // Backslash escape inside url(): push both chars.
                    if let Some(&(_, esc)) = chars.peek() {
                        result.push(esc);
                        chars.next();
                    }
                }
            }

            State::InString(quote) => {
                if ch == '\\' {
                    // Backslash escape: push both chars verbatim.
                    result.push(ch);
                    chars.next();
                    if let Some(&(_, esc)) = chars.peek() {
                        result.push(esc);
                        chars.next();
                    }
                } else {
                    result.push(ch);
                    chars.next();
                    if ch == quote {
                        state = State::Normal;
                    }
                }
            }

            State::PropStart | State::Normal => {
                // CSS comment: consume eagerly and preserve verbatim.
                if ch == '/' && css[i..].starts_with("/*") {
                    skip_comment(&mut result, &mut chars);
                    continue;
                }

                // String literal start.
                if ch == '"' || ch == '\'' {
                    state = State::InString(ch);
                    result.push(ch);
                    chars.next();
                    continue;
                }

                // url() function: skip contents verbatim to avoid corrupting
                // paths like `url(-webkit-something.png)`.
                if matches!(state, State::Normal) && css[i..].starts_with("url(") {
                    for _ in 0..4 {
                        if let Some(&(_, c)) = chars.peek() {
                            result.push(c);
                            chars.next();
                        }
                    }
                    state = State::InUrl;
                    continue;
                }

                if matches!(state, State::PropStart) {
                    // Whitespace before property name: stay in PropStart.
                    if ch.is_ascii_whitespace() {
                        result.push(ch);
                        chars.next();
                        continue;
                    }
                    // Potential vendor prefix at property position.
                    if ch == '-' {
                        let remaining = &css[i..];
                        let mut stripped = false;
                        for prefix in VENDOR_PREFIXES {
                            if remaining.starts_with(prefix) {
                                for _ in 0..prefix.len() {
                                    chars.next();
                                }
                                stripped = true;
                                break;
                            }
                        }
                        if !stripped {
                            // Not a vendor prefix (e.g., custom property `--foo`).
                            result.push(ch);
                            chars.next();
                        }
                        state = State::Normal;
                        continue;
                    }
                    // Any other char: transition to Normal and fall through.
                    state = State::Normal;
                }

                // Normal: check for delimiters that mark the next property.
                match ch {
                    '{' | ';' | '}' => {
                        state = State::PropStart;
                        result.push(ch);
                        chars.next();
                    }
                    _ => {
                        result.push(ch);
                        chars.next();
                    }
                }
            }
        }
    }

    result
}

/// Consume a CSS comment (`/* ... */`), pushing it verbatim to `result`.
fn skip_comment(result: &mut String, chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>) {
    // Push "/*"
    result.push('/');
    chars.next();
    if let Some(&(_, '*')) = chars.peek() {
        result.push('*');
        chars.next();
    }
    // Scan for closing "*/".
    while let Some(&(_, c)) = chars.peek() {
        result.push(c);
        chars.next();
        if c == '*' {
            if let Some(&(_, '/')) = chars.peek() {
                result.push('/');
                chars.next();
                return;
            }
        }
    }
}

/// Parse a stylesheet with vendor prefix stripping.
///
/// Convenience function that strips vendor prefixes from the CSS text
/// before passing it to `elidex_css::parse_stylesheet()`.
#[must_use]
pub fn parse_compat_stylesheet(css: &str, origin: Origin) -> Stylesheet {
    let stripped = strip_vendor_prefixes(css);
    elidex_css::parse_stylesheet(&stripped, origin)
}
