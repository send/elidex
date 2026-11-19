//! CSS vendor prefix removal and legacy property mapping.
//!
//! Strips `-webkit-`, `-moz-`, `-ms-`, `-o-` prefixes from CSS property names
//! at the text level, before parsing.
//!
//! Also maps legacy `-webkit-box-*` (2009 flexbox draft) properties to their
//! modern equivalents: `flex-direction`, `justify-content`, `align-items`, etc.

use elidex_css::{Origin, Stylesheet};

/// Known vendor prefixes to strip.
const VENDOR_PREFIXES: &[&str] = &["-webkit-", "-moz-", "-ms-", "-o-"];

/// Non-standard vendor properties to drop entirely (property + value).
///
/// These properties have no standard equivalent — stripping just the prefix
/// would leave a meaningless declaration. Instead, the entire `property: value;`
/// is removed from the output.
const DROP_PROPERTIES: &[&str] = &[
    "-webkit-text-size-adjust",
    "-moz-text-size-adjust",
    "-ms-text-size-adjust",
    "-webkit-font-smoothing",
    "-moz-osx-font-smoothing",
    "-webkit-tap-highlight-color",
];

/// Legacy `-webkit-box-*` properties (2009 flexbox draft) that need property+value
/// mapping to modern flexbox. These are intercepted before generic prefix stripping.
const WEBKIT_BOX_PROPERTIES: &[&str] = &[
    "-webkit-box-orient",
    "-webkit-box-direction",
    "-webkit-box-pack",
    "-webkit-box-align",
    "-webkit-box-flex",
    "-webkit-box-ordinal-group",
    "-webkit-box-lines",
];

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

                        // Check for `-webkit-box-*` legacy flexbox properties.
                        if let Some(prop_len) = WEBKIT_BOX_PROPERTIES
                            .iter()
                            .find_map(|prop| remaining.starts_with(prop).then_some(prop.len()))
                        {
                            let prop_name = &remaining[..prop_len];
                            // Skip the property name.
                            for _ in 0..prop_len {
                                chars.next();
                            }
                            // Extract the declaration value (between `:` and `;`/`}`).
                            let value = extract_declaration_value(&mut chars);
                            let value = value.trim();
                            // Map property+value to modern flexbox equivalent.
                            if let Some((new_prop, new_val)) = map_webkit_box(prop_name, value) {
                                result.push_str(new_prop);
                                result.push_str(": ");
                                result.push_str(&new_val);
                                result.push(';');
                            }
                            state = State::PropStart;
                            continue;
                        }

                        // Check for non-standard properties to drop entirely.
                        if let Some(drop_len) = DROP_PROPERTIES
                            .iter()
                            .find_map(|prop| remaining.starts_with(prop).then_some(prop.len()))
                        {
                            // Skip the property name.
                            for _ in 0..drop_len {
                                chars.next();
                            }
                            // Skip everything up to `;` or `}` (end of declaration).
                            // Also trim any leading whitespace that was already pushed
                            // (between the previous `;`/`{` and this property name).
                            trim_trailing_whitespace(&mut result);
                            skip_declaration_value(&mut chars);
                            state = State::PropStart;
                            continue;
                        }

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

    // Post-process: map display value keywords for legacy flexbox.
    replace_display_webkit_box(&mut result);

    result
}

/// Replace `display: -webkit-box` / `-webkit-inline-box` with modern equivalents.
fn replace_display_webkit_box(css: &mut String) {
    // Simple targeted replacements — these are value-level so can't be caught
    // by the property-level prefix stripper.
    for (old, new) in [
        ("-webkit-inline-box", "inline-flex"), // must check longer pattern first
        ("-webkit-box", "flex"),
    ] {
        while let Some(pos) = css.find(old) {
            css.replace_range(pos..pos + old.len(), new);
        }
    }
}

/// Extract declaration value text (after `:`, up to `;` or `}`).
///
/// Returns the value string. Consumes through the terminating `;` if present.
/// Stops before `}` without consuming it.
fn extract_declaration_value(chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>) -> String {
    let mut value = String::new();
    // Skip colon and leading whitespace.
    let mut found_colon = false;
    while let Some(&(_, c)) = chars.peek() {
        if c == ':' && !found_colon {
            chars.next();
            found_colon = true;
            continue;
        }
        if !found_colon {
            // Skip whitespace before colon.
            if c.is_ascii_whitespace() {
                chars.next();
                continue;
            }
            // No colon found but hit non-whitespace — malformed, bail.
            break;
        }
        if c == ';' {
            chars.next();
            break;
        }
        if c == '}' {
            // Don't consume — let the main loop handle it.
            break;
        }
        value.push(c);
        chars.next();
    }
    value
}

/// Map a legacy `-webkit-box-*` property + value to its modern flexbox equivalent.
///
/// Returns `Some((property, value))` for recognized mappings, `None` to drop.
///
/// Reference: <https://www.w3.org/TR/2009/WD-css3-flexbox-20090723/>
fn map_webkit_box(property: &str, value: &str) -> Option<(&'static str, String)> {
    match property {
        "-webkit-box-orient" => {
            let new_val = match value {
                "horizontal" | "inline-axis" => "row",
                "vertical" | "block-axis" => "column",
                _ => return None,
            };
            Some(("flex-direction", new_val.into()))
        }
        "-webkit-box-direction" => {
            // `normal` is default (no-op), `reverse` maps to row-reverse/column-reverse.
            // Since we don't know orient here, emit flex-direction with the reverse value.
            // In practice, orient+direction combos are rare — sites typically use just orient.
            // Chrome also treats direction:reverse alone as row-reverse.
            let new_val = match value {
                "normal" => "row",
                "reverse" => "row-reverse",
                _ => return None,
            };
            Some(("flex-direction", new_val.into()))
        }
        "-webkit-box-pack" => {
            let new_val = match value {
                "start" => "flex-start",
                "end" => "flex-end",
                "center" => "center",
                "justify" => "space-between",
                _ => return None,
            };
            Some(("justify-content", new_val.into()))
        }
        "-webkit-box-align" => {
            let new_val = match value {
                "start" => "flex-start",
                "end" => "flex-end",
                "center" => "center",
                "stretch" => "stretch",
                "baseline" => "baseline",
                _ => return None,
            };
            Some(("align-items", new_val.into()))
        }
        "-webkit-box-flex" => {
            // -webkit-box-flex: <number> → flex-grow: <number>
            // Value is passed through as-is (must be a valid number).
            if value.parse::<f32>().is_ok() {
                Some(("flex-grow", value.into()))
            } else {
                None
            }
        }
        "-webkit-box-ordinal-group" => {
            // -webkit-box-ordinal-group: <integer> → order: <integer>
            if value.parse::<i32>().is_ok() {
                Some(("order", value.into()))
            } else {
                None
            }
        }
        "-webkit-box-lines" => {
            let new_val = match value {
                "single" => "nowrap",
                "multiple" => "wrap",
                _ => return None,
            };
            Some(("flex-wrap", new_val.into()))
        }
        _ => None,
    }
}

/// Skip the rest of a declaration (`: value;`) without emitting it.
///
/// Advances `chars` past everything up to and including the next `;`.
/// If `}` is encountered first, stops before it (so the caller's main
/// loop sees the `}` and transitions to `PropStart`).
fn skip_declaration_value(chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>) {
    while let Some(&(_, c)) = chars.peek() {
        if c == '}' {
            // Don't consume — let the main loop handle it.
            return;
        }
        chars.next();
        if c == ';' {
            return;
        }
    }
}

/// Trim trailing ASCII whitespace (spaces, newlines, tabs) from `result`.
fn trim_trailing_whitespace(result: &mut String) {
    let trimmed_len = result.trim_end().len();
    result.truncate(trimmed_len);
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
    parse_compat_stylesheet_with_registry(css, origin, None)
}

/// Parse a stylesheet with vendor prefix stripping and handler registry.
///
/// Like [`parse_compat_stylesheet`], but passes the `registry` to the parser
/// for plugin-handled property dispatch (e.g. `transition-*`, `animation-*`).
#[must_use]
pub fn parse_compat_stylesheet_with_registry(
    css: &str,
    origin: Origin,
    registry: Option<&elidex_plugin::CssPropertyRegistry>,
) -> Stylesheet {
    let stripped = strip_vendor_prefixes(css);
    elidex_css::parse_stylesheet_with_registry(&stripped, origin, registry)
}
