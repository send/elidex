//! `CSS.escape(ident)` — CSSOM §6.7.2 "serialize an identifier" algorithm.
//!
//! Quotes / escapes a CSS identifier per WHATWG CSSOM §6.7.2 so the
//! result, when interpreted as a CSS identifier in a selector, refers to
//! the same string.  Pure character-by-character transformation; allocates
//! a fresh `String` of bounded size.
//!
//! ```text
//! CSS.escape("foo bar")  → "foo\\ bar"
//! CSS.escape("123")      → "\\31 23"
//! CSS.escape("--foo")    → "--foo"
//! ```

/// Serialize an identifier per CSSOM §6.7.2.  Returns a freshly-allocated
/// `String` with the escape transformations applied:
///
/// 1. NULL (U+0000) → U+FFFD REPLACEMENT CHARACTER.
/// 2. Control characters (U+0001-U+001F, U+007F) → `\HH ` (hex code +
///    space).
/// 3. ASCII digits at the start (or after a single leading `-`) →
///    `\HH ` (hex code + space).
/// 4. A single `-` (length-1 string) → `\-`.
/// 5. ASCII alpha-numeric, `_`, `-`, U+0080+ → unchanged.
/// 6. Everything else → `\X` (backslash + character).
#[must_use]
pub fn escape_ident(ident: &str) -> String {
    // Single-pass iteration with a 1-bit lookbehind avoids the
    // `Vec<char>` allocation that an earlier draft used to peek `chars[0]`
    // / `chars[1]`.  `prev_was_lone_dash` records whether the index-0 char
    // was a single `-` so the "second char of `-X` where X is a digit"
    // rule (CSSOM §6.7.2 step 4) can fire without a buffered prefix.
    let mut out = String::with_capacity(ident.len());

    // CSSOM §6.7.2 step 3: lone `-` serializes as `\-`.  Probe just the
    // first two chars without buffering the whole input.
    {
        let mut probe = ident.chars();
        if let (Some('-'), None) = (probe.next(), probe.next()) {
            return "\\-".to_string();
        }
    }

    let mut prev_was_lone_dash = false;
    for (index, c) in ident.chars().enumerate() {
        match c {
            '\0' => out.push('\u{FFFD}'),
            '\u{0001}'..='\u{001F}' | '\u{007F}' => {
                push_hex_escape(&mut out, c);
            }
            '0'..='9' if index == 0 => {
                push_hex_escape(&mut out, c);
            }
            '0'..='9' if index == 1 && prev_was_lone_dash => {
                push_hex_escape(&mut out, c);
            }
            'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-' => out.push(c),
            c if (c as u32) >= 0x0080 => out.push(c),
            _ => {
                out.push('\\');
                out.push(c);
            }
        }
        prev_was_lone_dash = index == 0 && c == '-';
    }
    out
}

fn push_hex_escape(out: &mut String, c: char) {
    out.push('\\');
    let _ = std::fmt::Write::write_fmt(out, format_args!("{:x}", c as u32));
    out.push(' ');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passes_alphanumeric_unchanged() {
        assert_eq!(escape_ident("foo"), "foo");
        assert_eq!(escape_ident("foo-bar"), "foo-bar");
        assert_eq!(escape_ident("_underscore"), "_underscore");
        assert_eq!(escape_ident("--var"), "--var");
    }

    #[test]
    fn escapes_special_chars() {
        assert_eq!(escape_ident("foo bar"), "foo\\ bar");
        assert_eq!(escape_ident("a.b"), "a\\.b");
        assert_eq!(escape_ident("a:b"), "a\\:b");
    }

    #[test]
    fn escapes_leading_digit() {
        assert_eq!(escape_ident("1foo"), "\\31 foo");
        assert_eq!(escape_ident("123"), "\\31 23");
    }

    #[test]
    fn escapes_digit_after_lone_dash() {
        // "-1" → "-\31 ": the "-" passes through, but the digit must be
        // escaped because a leading `-` followed by a digit otherwise
        // forms a number token.
        assert_eq!(escape_ident("-1"), "-\\31 ");
    }

    #[test]
    fn lone_dash() {
        assert_eq!(escape_ident("-"), "\\-");
    }

    #[test]
    fn null_replaced_with_replacement_char() {
        assert_eq!(escape_ident("a\0b"), "a\u{FFFD}b");
    }

    #[test]
    fn control_chars_hex_escaped() {
        // U+0001 → "\1 "
        assert_eq!(escape_ident("a\u{0001}b"), "a\\1 b");
    }

    #[test]
    fn unicode_passes_through() {
        // U+0080+ characters pass unchanged.
        assert_eq!(escape_ident("café"), "café");
    }

    #[test]
    fn empty_string() {
        assert_eq!(escape_ident(""), "");
    }
}
