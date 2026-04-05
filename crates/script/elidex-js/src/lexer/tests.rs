use super::*;
use crate::atom::{Atom, StringInterner};
use crate::token::Keyword;

struct LexResult {
    tokens: Vec<TokenKind>,
    interner: StringInterner,
    errors: Vec<crate::error::JsParseError>,
}

impl LexResult {
    /// Intern a string using the same interner as the lexer, so atoms compare equal.
    fn atom(&mut self, s: &str) -> Atom {
        self.interner.intern(s)
    }

    /// Resolve an atom to its string.
    fn resolve(&self, atom: Atom) -> String {
        self.interner.get_utf8(atom)
    }
}

fn lex(src: &str) -> LexResult {
    let mut lexer = Lexer::new(src);
    let mut tokens = Vec::new();
    loop {
        let (tok, _) = lexer.next_token();
        if matches!(tok.kind, TokenKind::Eof) {
            break;
        }
        tokens.push(tok.kind);
    }
    LexResult {
        tokens,
        errors: lexer.errors,
        interner: lexer.interner,
    }
}

#[test]
fn identifiers_and_keywords() {
    let mut out = lex("let x = 42");
    let x = out.atom("x");
    assert_eq!(
        out.tokens,
        vec![
            TokenKind::Keyword(Keyword::Let),
            TokenKind::Identifier(x),
            TokenKind::Eq,
            TokenKind::NumericLiteral(42.0),
        ]
    );
}

#[test]
fn contextual_keywords_are_identifiers() {
    // S3: `yield` is a reserved keyword in strict mode (always strict in elidex)
    let mut out = lex("async await yield");
    let a = out.atom("async");
    let aw = out.atom("await");
    assert_eq!(
        out.tokens,
        vec![
            TokenKind::Identifier(a),
            TokenKind::Identifier(aw),
            TokenKind::Keyword(Keyword::Yield),
        ]
    );
}

#[test]
fn punctuator_maximal_munch() {
    assert_eq!(lex(">>>").tokens, vec![TokenKind::UShr]);
    assert_eq!(lex(">>>=").tokens, vec![TokenKind::UShrEq]);
    assert_eq!(lex("===").tokens, vec![TokenKind::StrictEq]);
    assert_eq!(lex("!==").tokens, vec![TokenKind::StrictNe]);
    assert_eq!(lex("**=").tokens, vec![TokenKind::ExpEq]);
    assert_eq!(lex("?.").tokens, vec![TokenKind::OptChain]);
    assert_eq!(lex("??=").tokens, vec![TokenKind::NullCoalEq]);
    assert_eq!(lex("=>").tokens, vec![TokenKind::Arrow]);
    assert_eq!(lex("...").tokens, vec![TokenKind::Ellipsis]);
    assert_eq!(lex("&&=").tokens, vec![TokenKind::AndEq]);
    assert_eq!(lex("||=").tokens, vec![TokenKind::OrEq]);
}

#[test]
fn string_literals() {
    let mut out = lex(r#""hello""#);
    let hello = out.atom("hello");
    assert_eq!(out.tokens, vec![TokenKind::StringLiteral(hello)]);
    let mut out = lex(r"'world'");
    let world = out.atom("world");
    assert_eq!(out.tokens, vec![TokenKind::StringLiteral(world)]);
}

#[test]
fn string_escape_sequences() {
    let mut out = lex(r#""\n\t\\""#);
    let s = out.atom("\n\t\\");
    assert_eq!(out.tokens, vec![TokenKind::StringLiteral(s)]);
    let mut out = lex(r#""\x41""#);
    let a = out.atom("A");
    assert_eq!(out.tokens, vec![TokenKind::StringLiteral(a)]);
    let mut out = lex(r#""\u0041""#);
    let a = out.atom("A");
    assert_eq!(out.tokens, vec![TokenKind::StringLiteral(a)]);
    let mut out = lex(r#""\u{1F600}""#);
    let emoji = out.atom("\u{1F600}");
    assert_eq!(out.tokens, vec![TokenKind::StringLiteral(emoji)]);
}

#[test]
fn numeric_literals() {
    assert_eq!(lex("42").tokens, vec![TokenKind::NumericLiteral(42.0)]);
    assert_eq!(lex("2.75").tokens, vec![TokenKind::NumericLiteral(2.75)]);
    assert_eq!(lex("1e3").tokens, vec![TokenKind::NumericLiteral(1000.0)]);
    assert_eq!(lex("0xFF").tokens, vec![TokenKind::NumericLiteral(255.0)]);
    assert_eq!(lex("0b1010").tokens, vec![TokenKind::NumericLiteral(10.0)]);
    assert_eq!(lex("0o17").tokens, vec![TokenKind::NumericLiteral(15.0)]);
}

#[test]
fn numeric_separators() {
    assert_eq!(lex("1_000").tokens, vec![TokenKind::NumericLiteral(1000.0)]);
    assert_eq!(
        lex("0xFF_FF").tokens,
        vec![TokenKind::NumericLiteral(65535.0)]
    );
}

#[test]
fn bigint_literals() {
    let mut out = lex("42n");
    let s = out.atom("42");
    assert_eq!(out.tokens, vec![TokenKind::BigIntLiteral(s)]);
    let mut out = lex("0xFFn");
    let s = out.atom("0xFF");
    assert_eq!(out.tokens, vec![TokenKind::BigIntLiteral(s)]);
}

#[test]
fn template_no_sub() {
    let mut out = lex("`hello`");
    let hello = out.atom("hello");
    assert_eq!(
        out.tokens,
        vec![TokenKind::TemplateNoSub {
            cooked: Some(hello),
            raw: hello,
        }]
    );
}

#[test]
fn regexp_literal() {
    let mut out = lex("/abc/gi");
    let pat = out.atom("abc");
    let flags = out.atom("gi");
    assert_eq!(
        out.tokens,
        vec![TokenKind::RegExpLiteral {
            pattern: pat,
            flags,
        }]
    );
}

#[test]
fn division_after_expression() {
    let mut out = lex("x / 2");
    let x = out.atom("x");
    assert_eq!(
        out.tokens,
        vec![
            TokenKind::Identifier(x),
            TokenKind::Slash,
            TokenKind::NumericLiteral(2.0),
        ]
    );
}

#[test]
fn line_comment_and_block_comment() {
    let mut out = lex("a // comment\nb");
    let a = out.atom("a");
    let b = out.atom("b");
    assert_eq!(
        out.tokens,
        vec![TokenKind::Identifier(a), TokenKind::Identifier(b),]
    );
    let mut out = lex("a /* block */ b");
    let a = out.atom("a");
    let b = out.atom("b");
    assert_eq!(
        out.tokens,
        vec![TokenKind::Identifier(a), TokenKind::Identifier(b),]
    );
}

#[test]
fn newline_tracking() {
    let mut lex = Lexer::new("a\nb\r\nc");
    let (_, nl1) = lex.next_token(); // a
    assert!(!nl1);
    let (_, nl2) = lex.next_token(); // b
    assert!(nl2);
    let (_, nl3) = lex.next_token(); // c
    assert!(nl3);
}

#[test]
fn private_identifier() {
    let mut out = lex("#field");
    let field = out.atom("field");
    assert_eq!(out.tokens, vec![TokenKind::PrivateIdentifier(field)]);
}

#[test]
fn dot_number() {
    assert_eq!(lex(".5").tokens, vec![TokenKind::NumericLiteral(0.5)]);
}

#[test]
fn spans_are_correct() {
    let mut lex = Lexer::new("let x");
    let (t1, _) = lex.next_token();
    assert_eq!(t1.span, Span::new(0, 3));
    let (t2, _) = lex.next_token();
    assert_eq!(t2.span, Span::new(4, 5));
}

#[test]
fn template_with_expression() {
    let mut lex = Lexer::new("`a${b}c`");
    let (t1, _) = lex.next_token();
    let a = lex.interner.intern("a");
    assert_eq!(
        t1.kind,
        TokenKind::TemplateHead {
            cooked: Some(a),
            raw: a,
        }
    );
    let (t2, _) = lex.next_token();
    let b = lex.interner.intern("b");
    assert_eq!(t2.kind, TokenKind::Identifier(b));
    let (t_rb, _) = lex.next_token();
    assert_eq!(t_rb.kind, TokenKind::RBrace);
    let t3 = lex.lex_template_part();
    let c = lex.interner.intern("c");
    assert_eq!(
        t3.kind,
        TokenKind::TemplateTail {
            cooked: Some(c),
            raw: c,
        }
    );
}

#[test]
fn scientific_notation() {
    assert_eq!(
        lex("1.5e+3").tokens,
        vec![TokenKind::NumericLiteral(1500.0)]
    );
    assert_eq!(lex("2e-1").tokens, vec![TokenKind::NumericLiteral(0.2)]);
}

#[test]
fn question_dot_vs_optional_chain() {
    let mut out = lex("x?.5");
    let x = out.atom("x");
    assert_eq!(out.tokens[0], TokenKind::Identifier(x));
    assert_eq!(out.tokens[1], TokenKind::Question);
    assert_eq!(out.tokens[2], TokenKind::NumericLiteral(0.5));
}

// ── H6: \0 followed by digit (octal escape) ──

#[test]
fn octal_escape_after_null() {
    let mut lex = Lexer::new(r#""\01""#);
    let (tok, _) = lex.next_token();
    assert!(matches!(tok.kind, TokenKind::StringLiteral(_)));
    assert!(!lex.errors.is_empty(), "Expected error for octal escape");
}

// ── H7: legacy octal escapes \1-\9 ──

#[test]
fn legacy_octal_escape_error() {
    let mut lex = Lexer::new(r#""\8""#);
    let (tok, _) = lex.next_token();
    assert!(matches!(tok.kind, TokenKind::StringLiteral(_)));
    assert!(!lex.errors.is_empty(), "Expected error for \\8 escape");
}

// ── L5: BigInt with decimal/exponent ──

#[test]
fn bigint_decimal_error() {
    let mut lex = Lexer::new("1.5n");
    let (_tok, _) = lex.next_token();
    assert!(!lex.errors.is_empty(), "BigInt cannot have decimal point");
}

#[test]
fn bigint_exponent_error() {
    let mut lex = Lexer::new("1e5n");
    let (_tok, _) = lex.next_token();
    assert!(!lex.errors.is_empty(), "BigInt cannot have exponent");
}

// ── B2/B3: template raw/cooked ──

#[test]
fn template_raw_escape() {
    let out = lex(r"`\n`");
    match &out.tokens[0] {
        TokenKind::TemplateNoSub { cooked, raw } => {
            assert_eq!(cooked.map(|a| out.resolve(a)), Some("\n".to_string()));
            assert_eq!(out.resolve(*raw), "\\n");
        }
        other => panic!("Expected TemplateNoSub, got {other:?}"),
    }
}

#[test]
fn template_invalid_escape_cooked_none() {
    let mut lex = Lexer::new(r"`\8`");
    let (tok, _) = lex.next_token();
    match tok.kind {
        TokenKind::TemplateNoSub { cooked, raw } => {
            assert!(cooked.is_none(), "cooked should be None for invalid escape");
            assert_eq!(lex.interner.get_utf8(raw), "\\8");
        }
        other => panic!("Expected TemplateNoSub, got {other:?}"),
    }
}

#[test]
fn template_head_tail_raw() {
    let mut lex = Lexer::new(r"`a\n${x}b\t`");
    let (t1, _) = lex.next_token();
    match t1.kind {
        TokenKind::TemplateHead { cooked, raw } => {
            assert_eq!(
                cooked.map(|a| lex.interner.get_utf8(a)),
                Some("a\n".to_string())
            );
            assert_eq!(lex.interner.get_utf8(raw), "a\\n");
        }
        other => panic!("Expected TemplateHead, got {other:?}"),
    }
    lex.next_token();
    lex.next_token();
    let t2 = lex.lex_template_part();
    match t2.kind {
        TokenKind::TemplateTail { cooked, raw } => {
            assert_eq!(
                cooked.map(|a| lex.interner.get_utf8(a)),
                Some("b\t".to_string())
            );
            assert_eq!(lex.interner.get_utf8(raw), "b\\t");
        }
        other => panic!("Expected TemplateTail, got {other:?}"),
    }
}

// ── B4: identifier Unicode escape ──

#[test]
fn identifier_unicode_escape() {
    let mut out = lex(r"\u0061");
    let a = out.atom("a");
    assert_eq!(out.tokens, vec![TokenKind::Identifier(a)]);
    let mut out = lex(r"a\u0062c");
    let abc = out.atom("abc");
    assert_eq!(out.tokens, vec![TokenKind::Identifier(abc)]);
}

#[test]
fn identifier_unicode_brace_escape() {
    let mut out = lex(r"\u{61}");
    let a = out.atom("a");
    assert_eq!(out.tokens, vec![TokenKind::Identifier(a)]);
}

#[test]
fn escaped_keyword_is_identifier_with_error() {
    let mut lex = Lexer::new(r"\u006Cet");
    let (tok, _) = lex.next_token();
    let let_atom = lex.interner.intern("let");
    assert_eq!(tok.kind, TokenKind::Identifier(let_atom));
    assert!(
        !lex.errors.is_empty(),
        "Escaped keyword should produce error"
    );
}

// ── M3: Unicode escape ID_Start/ID_Continue validation ──

#[test]
fn unicode_escape_digit_as_id_start_error() {
    let mut lex = Lexer::new(r"\u0030");
    let (tok, _) = lex.next_token();
    assert!(matches!(tok.kind, TokenKind::Identifier(_)));
    assert!(
        lex.errors
            .iter()
            .any(|e| e.message.contains("identifier start")),
        "Digit escape as identifier start should produce error: {:?}",
        lex.errors
    );
}

#[test]
fn unicode_escape_valid_id_continue() {
    let mut lex = Lexer::new(r"x\u0041");
    let (tok, _) = lex.next_token();
    let xa = lex.interner.intern("xA");
    assert_eq!(tok.kind, TokenKind::Identifier(xa));
    assert!(lex.errors.is_empty(), "Valid escape: {:?}", lex.errors);
}

// ── B5: Unicode Zs whitespace ──

#[test]
fn unicode_zs_whitespace_separates_tokens() {
    let mut out = lex("let\u{3000}x");
    let x = out.atom("x");
    assert_eq!(
        out.tokens,
        vec![TokenKind::Keyword(Keyword::Let), TokenKind::Identifier(x),]
    );
}

#[test]
fn nbsp_separates_tokens() {
    let mut out = lex("let\u{00A0}x");
    let x = out.atom("x");
    assert_eq!(
        out.tokens,
        vec![TokenKind::Keyword(Keyword::Let), TokenKind::Identifier(x),]
    );
}

// ── B6: unicode-ident combining mark ──

#[test]
fn combining_mark_in_identifier() {
    let mut out = lex("e\u{0301}");
    let id = out.atom("e\u{0301}");
    assert_eq!(out.tokens, vec![TokenKind::Identifier(id)]);
}

// ── A1: Legacy octal rejection ──

#[test]
fn legacy_octal_077_error() {
    let out = lex("077");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("Legacy octal")),
        "Expected legacy octal error: {:?}",
        out.errors
    );
}

#[test]
fn leading_zero_09_error() {
    let out = lex("09");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("leading zeros")),
        "Expected leading zeros error for 09: {:?}",
        out.errors
    );
}

#[test]
fn leading_zero_allowed() {
    for src in &["0", "0.5", "0e5", "0n", "0x1"] {
        let out = lex(src);
        assert!(
            out.errors.is_empty(),
            "Unexpected error for '{src}': {:?}",
            out.errors
        );
    }
}

// ── A2: Unicode escape out of range ──

#[test]
fn unicode_escape_out_of_range_error() {
    let mut lex = Lexer::new(r#""\u{110000}""#);
    let (tok, _) = lex.next_token();
    assert!(matches!(tok.kind, TokenKind::StringLiteral(_)));
    assert!(
        lex.errors
            .iter()
            .any(|e| e.message.contains("out of range")),
        "Expected out-of-range error: {:?}",
        lex.errors
    );
}

#[test]
fn unicode_escape_surrogate_preserved() {
    // Lone surrogates are valid in ES string literals; now preserved as WTF-16
    let mut lex = Lexer::new(r#""\uD800""#);
    let (tok, _) = lex.next_token();
    assert!(matches!(tok.kind, TokenKind::StringLiteral(_)));
    assert!(
        lex.errors.is_empty(),
        "Surrogates should not produce errors: {:?}",
        lex.errors
    );
    if let TokenKind::StringLiteral(atom) = tok.kind {
        let units = lex.interner.get(atom);
        assert_eq!(
            units,
            &[0xD800u16],
            "Surrogate should be preserved as WTF-16"
        );
    }
}

// ── A3: Empty exponent ──

#[test]
fn empty_exponent_error() {
    let out = lex("1e;");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("Exponent requires")),
        "Expected empty exponent error: {:?}",
        out.errors
    );
}

// ── A4: Empty digits after prefix ──

#[test]
fn empty_hex_digits_error() {
    let out = lex("0x;");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("Expected digits")),
        "Expected empty hex digits error: {:?}",
        out.errors
    );
}

#[test]
fn empty_binary_digits_error() {
    let out = lex("0b;");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("Expected digits")),
        "Expected empty binary digits error: {:?}",
        out.errors
    );
}

// ── B3: LS/PS in regexp ──

#[test]
fn regexp_ls_ps_unterminated() {
    let src = format!("/abc{}def/", '\u{2028}');
    let mut lex = Lexer::new(&src);
    lex.prev_allows_regexp = true;
    let (tok, _) = lex.next_token();
    assert!(
        matches!(tok.kind, TokenKind::RegExpLiteral { .. }),
        "Expected RegExpLiteral, got {:?}",
        tok.kind
    );
    assert!(
        lex.errors
            .iter()
            .any(|e| e.message.contains("Unterminated")),
        "Expected unterminated regexp error: {:?}",
        lex.errors
    );
}

// ── H2: \u{...} termination check ──

#[test]
fn unicode_brace_escape_unterminated() {
    let mut lex = Lexer::new("\"\\u{41G}\"");
    let _ = lex.next_token();
    assert!(!lex.errors.is_empty(), "\\u{{41G}} should produce an error");
}

#[test]
fn unicode_brace_escape_missing_close() {
    let mut lex = Lexer::new("\"\\u{41\"");
    let _ = lex.next_token();
    assert!(
        !lex.errors.is_empty(),
        "\\u{{41 (no close brace) should produce an error"
    );
}

#[test]
fn unicode_brace_escape_valid() {
    let mut lex = Lexer::new("\"\\u{41}\"");
    let (tok, _) = lex.next_token();
    let a = lex.interner.intern("A");
    assert_eq!(tok.kind, TokenKind::StringLiteral(a));
    assert!(lex.errors.is_empty(), "{:?}", lex.errors);
}

// ── Coverage: unterminated string literal ──

#[test]
fn unterminated_string() {
    let mut lex = Lexer::new("\"hello");
    let _ = lex.next_token();
    assert!(
        lex.errors.iter().any(|e| e.message.contains("nterminated")),
        "Expected unterminated string error: {:?}",
        lex.errors
    );
}

// ── Coverage: unterminated block comment ──

#[test]
fn unterminated_block_comment() {
    let mut lex = Lexer::new("/* unclosed");
    let _ = lex.next_token();
    assert!(
        lex.errors
            .iter()
            .any(|e| e.message.contains("nterminated") || e.message.contains("comment")),
        "Expected unterminated comment error: {:?}",
        lex.errors
    );
}

// ── M3: braced unicode escape digit limit ──

#[test]
fn braced_unicode_overlong_hex() {
    let out = lex(r"'\u{0041}'");
    match &out.tokens[0] {
        TokenKind::StringLiteral(s) => assert_eq!(out.resolve(*s), "A"),
        other => panic!("Expected StringLiteral, got {other:?}"),
    }
}

#[test]
fn braced_unicode_overflow_error() {
    let out = lex(r"'\u{FFFFFF1}'");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("out of range") || e.message.contains("codepoint")),
        "Expected out-of-range unicode error: {:?}",
        out.errors
    );
}

// ── A8: template literal LS/PS cooked value ──

#[test]
fn template_ls_ps_preserves_actual_char() {
    let src = format!("`a{}b`", '\u{2028}');
    let out = lex(&src);
    match &out.tokens[0] {
        TokenKind::TemplateNoSub {
            cooked: Some(cooked),
            ..
        } => {
            let s = out.resolve(*cooked);
            assert!(
                s.contains('\u{2028}'),
                "Template cooked value should contain LS (U+2028), got: {s:?}",
            );
            assert!(
                !s.contains('\n'),
                "Template cooked value should NOT normalize LS to \\n"
            );
        }
        other => panic!("Expected TemplateNoSub with cooked, got {other:?}"),
    }
}

#[test]
fn template_ps_preserves_actual_char() {
    let src = format!("`a{}b`", '\u{2029}');
    let out = lex(&src);
    match &out.tokens[0] {
        TokenKind::TemplateNoSub {
            cooked: Some(cooked),
            ..
        } => {
            let s = out.resolve(*cooked);
            assert!(
                s.contains('\u{2029}'),
                "Template cooked value should contain PS (U+2029), got: {s:?}",
            );
        }
        other => panic!("Expected TemplateNoSub with cooked, got {other:?}"),
    }
}

#[test]
fn template_cr_lf_still_normalizes() {
    let out = lex("`a\rb`");
    match &out.tokens[0] {
        TokenKind::TemplateNoSub {
            cooked: Some(cooked),
            ..
        } => {
            let s = out.resolve(*cooked);
            assert!(s.contains('\n'), "CR should normalize to LF in template");
            assert!(
                !s.contains('\r'),
                "CR should not be preserved in template cooked value"
            );
        }
        other => panic!("Expected TemplateNoSub with cooked, got {other:?}"),
    }
}

// ── A9: private identifier multi-byte first character ──

#[test]
fn combining_mark_as_first_private_id_char_rejected() {
    let mut lex = Lexer::new("#\u{0301}");
    let (tok, _) = lex.next_token();
    match tok.kind {
        TokenKind::PrivateIdentifier(name) => {
            let s = lex.interner.get_utf8(name);
            assert!(
                s.is_empty() || !s.starts_with('\u{0301}'),
                "Combining mark should not be accepted as first char of private identifier"
            );
        }
        _ => panic!("Expected PrivateIdentifier, got {:?}", tok.kind),
    }
}

#[test]
fn unicode_letter_as_first_private_id_char_ok() {
    let mut out = lex("#\u{00E9}");
    let id = out.atom("\u{00E9}");
    assert_eq!(out.tokens, vec![TokenKind::PrivateIdentifier(id)]);
}

// ── E1: String identity escape errors ──

#[test]
fn string_identity_escape_error() {
    let out = crate::parse_script(r#"let x = "\a";"#);
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("Invalid escape")),
        "Expected identity escape error: {:?}",
        out.errors
    );
}

#[test]
fn string_valid_escapes_ok() {
    let out = crate::parse_script(r#"let x = "\n\r\t\\\"\'\0";"#);
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

// ── E2: Template invalid escape in tagged template ──

#[test]
fn tagged_template_invalid_escape_no_error() {
    let out = crate::parse_script(r"tag`\unicode`;");
    assert!(
        out.errors.is_empty(),
        "Tagged template should not error on invalid escape: {:?}",
        out.errors
    );
}

#[test]
fn untagged_template_invalid_escape_cooked_none() {
    let out = crate::parse_script(r"let x = `\unicode`;");
    if let crate::ast::StmtKind::VariableDeclaration { declarators, .. } =
        &out.program.stmts.get(out.program.body[0]).kind
    {
        let init = declarators[0].init.expect("Expected init");
        if let crate::ast::ExprKind::Template(tl) = &out.program.exprs.get(init).kind {
            assert!(
                tl.quasis[0].cooked.is_none(),
                "Cooked should be None for invalid escape"
            );
        } else {
            panic!("Expected template literal");
        }
    }
}

// ── V18a: NumericLiteral followed by IdentifierStart ──

#[test]
fn number_followed_by_identifier_error() {
    let out = lex("123abc");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("Identifier starts immediately")),
        "Expected identifier-after-number error: {:?}",
        out.errors
    );
}

#[test]
fn hex_followed_by_identifier_error() {
    let out = lex("0xFFg");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("Identifier starts immediately")),
        "Expected identifier-after-number error: {:?}",
        out.errors
    );
}

#[test]
fn number_followed_by_operator_ok() {
    let out = lex("123+x");
    assert!(
        !out.errors
            .iter()
            .any(|e| e.message.contains("Identifier starts immediately")),
        "Should not error for number followed by operator: {:?}",
        out.errors
    );
}

// ── V24: Multi-byte character skip ──

#[test]
fn unexpected_multibyte_no_cascade() {
    // A non-identifier-start multi-byte character should produce exactly one error
    let out = lex("§"); // U+00A7 (2 bytes: C2 A7), not ID_Start
                        // Should be at most 1 error (the unexpected character)
    let unexpected_count = out
        .errors
        .iter()
        .filter(|e| e.message.contains("Unexpected"))
        .count();
    assert!(
        unexpected_count <= 1,
        "Expected at most 1 unexpected error, got {unexpected_count}: {:?}",
        out.errors
    );
}
