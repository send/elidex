//! Unit tests for the strict tokenizer.
//!
//! Lives inside the crate (not `tests/`) because [`Tokenizer`] is
//! `pub(crate)` — internal to the crate until A4 wires it into
//! `parse_strict`, so an external integration test cannot reach it. The
//! html5lib-driven corpus is exercised here via [`super::tests_html5lib`].

use super::states::{State, Tokenizer};
use super::token::{TagToken, Token};
use crate::StrictParseError;

/// Run the tokenizer in the Data state to EOF, collecting every token
/// (including the final [`Token::EndOfFile`]).
fn tokenize(input: &str) -> Result<Vec<Token>, StrictParseError> {
    tokenize_in(input, State::Data, None)
}

/// Run the tokenizer from `state` with an optional last-start-tag name.
fn tokenize_in(
    input: &str,
    state: State,
    last_start_tag: Option<&str>,
) -> Result<Vec<Token>, StrictParseError> {
    let mut t = Tokenizer::new(input);
    t.set_state(state);
    if let Some(name) = last_start_tag {
        t.set_last_start_tag(name);
    }
    let mut out = Vec::new();
    loop {
        let tok = t.next_token()?;
        let done = tok == Token::EndOfFile;
        out.push(tok);
        if done {
            return Ok(out);
        }
    }
}

/// Collect just the character tokens into a `String`.
fn text_of(tokens: &[Token]) -> String {
    tokens
        .iter()
        .filter_map(|t| match t {
            Token::Character(c) => Some(*c),
            _ => None,
        })
        .collect()
}

fn start(name: &str, attrs: &[(&str, &str)]) -> Token {
    Token::StartTag(TagToken {
        name: name.to_string(),
        attrs: attrs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        self_closing: false,
    })
}

#[test]
fn plain_text_emits_characters() {
    let toks = tokenize("hello").unwrap();
    assert_eq!(text_of(&toks), "hello");
    assert_eq!(toks.last(), Some(&Token::EndOfFile));
}

#[test]
fn start_tag_with_attributes() {
    let toks = tokenize("<a href=\"x\" id=y>").unwrap();
    assert_eq!(toks[0], start("a", &[("href", "x"), ("id", "y")]));
}

#[test]
fn tag_name_is_ascii_lowercased() {
    let toks = tokenize("<DIV>").unwrap();
    assert_eq!(toks[0], start("div", &[]));
}

#[test]
fn end_tag() {
    let toks = tokenize("</p>").unwrap();
    assert_eq!(
        toks[0],
        Token::EndTag(TagToken {
            name: "p".to_string(),
            attrs: vec![],
            self_closing: false,
        })
    );
}

#[test]
fn self_closing_tag() {
    let toks = tokenize("<br/>").unwrap();
    match &toks[0] {
        Token::StartTag(t) => {
            assert_eq!(t.name, "br");
            assert!(t.self_closing);
        }
        other => panic!("expected start tag, got {other:?}"),
    }
}

#[test]
fn comment() {
    let toks = tokenize("<!-- hi -->").unwrap();
    assert_eq!(toks[0], Token::Comment(" hi ".to_string()));
}

#[test]
fn doctype_html() {
    let toks = tokenize("<!DOCTYPE html>").unwrap();
    match &toks[0] {
        Token::Doctype(d) => {
            assert_eq!(d.name.as_deref(), Some("html"));
            assert!(!d.force_quirks);
        }
        other => panic!("expected doctype, got {other:?}"),
    }
}

#[test]
fn named_character_reference() {
    let toks = tokenize("a&amp;b").unwrap();
    assert_eq!(text_of(&toks), "a&b");
}

#[test]
fn numeric_decimal_reference() {
    let toks = tokenize("&#65;").unwrap();
    assert_eq!(text_of(&toks), "A");
}

#[test]
fn numeric_hex_reference() {
    let toks = tokenize("&#x41;").unwrap();
    assert_eq!(text_of(&toks), "A");
}

#[test]
fn character_reference_in_attribute_value() {
    let toks = tokenize("<a title=\"&amp;\">").unwrap();
    assert_eq!(toks[0], start("a", &[("title", "&")]));
}

#[test]
fn rcdata_treats_markup_as_text_until_matching_end_tag() {
    // In RCDATA, `<b>` is text but `</title>` closes.
    let toks = tokenize_in("<b>x</title>", State::Rcdata, Some("title")).unwrap();
    assert_eq!(text_of(&toks), "<b>x");
    assert_eq!(
        toks.iter().find(|t| matches!(t, Token::EndTag(_))),
        Some(&Token::EndTag(TagToken {
            name: "title".to_string(),
            attrs: vec![],
            self_closing: false,
        }))
    );
}

#[test]
fn rawtext_emits_markup_as_text() {
    let toks = tokenize_in("a<b>c</style>", State::Rawtext, Some("style")).unwrap();
    assert_eq!(text_of(&toks), "a<b>c");
}

#[test]
fn ambiguous_ampersand_without_semicolon_is_literal() {
    // `&foo ` is not a known entity and has no `;` — emitted literally.
    let toks = tokenize("&foo ").unwrap();
    assert_eq!(text_of(&toks), "&foo ");
}

// ----- strict reject cases (no recovery) -----

#[test]
fn duplicate_attribute_is_rejected() {
    let err = tokenize("<a x=1 x=2>").unwrap_err();
    assert!(err.errors[0].contains("duplicate-attribute"));
}

#[test]
fn eof_in_tag_is_rejected() {
    let err = tokenize("<a ").unwrap_err();
    assert!(err.errors[0].contains("eof-in-tag"));
}

#[test]
fn unexpected_null_is_rejected() {
    let err = tokenize("a\u{0000}b").unwrap_err();
    assert!(err.errors[0].contains("unexpected-null-character"));
}

#[test]
fn missing_semicolon_entity_is_rejected() {
    // `&amp` (legacy, no semicolon) in data is a parse error in strict.
    let err = tokenize("x&ampy").unwrap_err();
    assert!(err.errors[0].contains("missing-semicolon-after-character-reference"));
}

#[test]
fn unknown_named_reference_with_semicolon_is_rejected() {
    let err = tokenize("&foo;").unwrap_err();
    assert!(err.errors[0].contains("unknown-named-character-reference"));
}

#[test]
fn surrogate_numeric_reference_is_rejected() {
    let err = tokenize("&#xD800;").unwrap_err();
    assert!(err.errors[0].contains("surrogate-character-reference"));
}

#[test]
fn out_of_range_numeric_reference_is_rejected() {
    let err = tokenize("&#x110000;").unwrap_err();
    assert!(err.errors[0].contains("character-reference-outside-unicode-range"));
}

#[test]
fn null_numeric_reference_is_rejected() {
    let err = tokenize("&#0;").unwrap_err();
    assert!(err.errors[0].contains("null-character-reference"));
}

#[test]
fn cdata_in_html_content_is_rejected() {
    let err = tokenize("<![CDATA[x]]>").unwrap_err();
    assert!(err.errors[0].contains("cdata-in-html-content"));
}

#[test]
fn bare_ampersand_then_eof_is_literal() {
    let toks = tokenize("&").unwrap();
    assert_eq!(text_of(&toks), "&");
}

#[test]
fn crlf_is_normalized_to_lf() {
    let toks = tokenize("a\r\nb").unwrap();
    assert_eq!(text_of(&toks), "a\nb");
}
