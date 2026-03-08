use super::*;

#[test]
fn simple_literal() {
    let node = parse_pattern("abc").unwrap();
    assert!(matches!(node, RegExpNode::Alternative(_)));
}

#[test]
fn dot_and_anchors() {
    let node = parse_pattern("^.$").unwrap();
    if let RegExpNode::Alternative(parts) = &node {
        assert_eq!(parts.len(), 3);
        assert!(matches!(
            parts[0],
            RegExpNode::Assertion(AssertionKind::Start)
        ));
        assert!(matches!(parts[1], RegExpNode::Dot));
        assert!(matches!(
            parts[2],
            RegExpNode::Assertion(AssertionKind::End)
        ));
    } else {
        panic!("Expected Alternative");
    }
}

#[test]
fn character_class() {
    let node = parse_pattern("[a-z]").unwrap();
    if let RegExpNode::CharClass { negated, ranges } = &node {
        assert!(!negated);
        assert_eq!(ranges.len(), 1);
        assert!(matches!(
            &ranges[0],
            CharRange::Range(CharClassAtom::Literal('a'), CharClassAtom::Literal('z'))
        ));
    } else {
        panic!("Expected CharClass");
    }
}

#[test]
fn negated_char_class() {
    let node = parse_pattern("[^0-9]").unwrap();
    if let RegExpNode::CharClass { negated, .. } = &node {
        assert!(negated);
    } else {
        panic!("Expected CharClass");
    }
}

#[test]
fn quantifiers() {
    let node = parse_pattern("a+").unwrap();
    assert!(matches!(
        node,
        RegExpNode::Quantifier {
            min: 1,
            max: None,
            greedy: true,
            ..
        }
    ));

    let node = parse_pattern("b*?").unwrap();
    assert!(matches!(
        node,
        RegExpNode::Quantifier {
            min: 0,
            max: None,
            greedy: false,
            ..
        }
    ));

    let node = parse_pattern("c{2,5}").unwrap();
    assert!(matches!(
        node,
        RegExpNode::Quantifier {
            min: 2,
            max: Some(5),
            ..
        }
    ));
}

#[test]
fn groups() {
    let node = parse_pattern("(a)").unwrap();
    assert!(matches!(
        node,
        RegExpNode::Group {
            kind: GroupKind::Capturing,
            ..
        }
    ));

    let node = parse_pattern("(?:b)").unwrap();
    assert!(matches!(
        node,
        RegExpNode::Group {
            kind: GroupKind::NonCapturing,
            ..
        }
    ));

    let node = parse_pattern("(?<name>c)").unwrap();
    if let RegExpNode::Group {
        kind: GroupKind::Named(n),
        ..
    } = &node
    {
        assert_eq!(n, "name");
    } else {
        panic!("Expected Named group");
    }
}

#[test]
fn alternation() {
    let node = parse_pattern("a|b|c").unwrap();
    if let RegExpNode::Disjunction(alts) = &node {
        assert_eq!(alts.len(), 3);
    } else {
        panic!("Expected Disjunction");
    }
}

#[test]
fn lookahead_lookbehind() {
    let node = parse_pattern("(?=x)").unwrap();
    assert!(matches!(
        node,
        RegExpNode::Assertion(AssertionKind::Lookahead(_))
    ));

    let node = parse_pattern("(?!x)").unwrap();
    assert!(matches!(
        node,
        RegExpNode::Assertion(AssertionKind::NegativeLookahead(_))
    ));

    let node = parse_pattern("(?<=x)").unwrap();
    assert!(matches!(
        node,
        RegExpNode::Assertion(AssertionKind::Lookbehind(_))
    ));

    let node = parse_pattern("(?<!x)").unwrap();
    assert!(matches!(
        node,
        RegExpNode::Assertion(AssertionKind::NegativeLookbehind(_))
    ));
}

#[test]
fn escapes() {
    let node = parse_pattern(r"\d").unwrap();
    assert!(matches!(node, RegExpNode::Escape(EscapeKind::Digit)));

    let node = parse_pattern(r"\b").unwrap();
    assert!(matches!(
        node,
        RegExpNode::Assertion(AssertionKind::WordBoundary)
    ));
}

#[test]
fn backreference() {
    let node = parse_pattern(r"(a)\1").unwrap();
    if let RegExpNode::Alternative(parts) = &node {
        assert!(matches!(&parts[1], RegExpNode::Backreference(1)));
    } else {
        panic!("Expected Alternative");
    }
}

#[test]
fn named_backreference() {
    let node = parse_pattern(r"(?<foo>a)\k<foo>").unwrap();
    if let RegExpNode::Alternative(parts) = &node {
        assert!(matches!(&parts[1], RegExpNode::NamedBackreference(name) if name == "foo"));
    } else {
        panic!("Expected Alternative");
    }
}

#[test]
fn unicode_property() {
    let u_flags = RegExpFlags {
        unicode: true,
        ..Default::default()
    };
    let node = parse_pattern_with_flags(r"\p{Letter}", &u_flags).unwrap();
    assert!(matches!(
        node,
        RegExpNode::UnicodeProperty { negated: false, .. }
    ));

    let node = parse_pattern_with_flags(r"\P{Script=Latin}", &u_flags).unwrap();
    if let RegExpNode::UnicodeProperty {
        name,
        value,
        negated,
    } = &node
    {
        assert!(negated);
        assert_eq!(name, "Script");
        assert_eq!(value.as_deref(), Some("Latin"));
    } else {
        panic!("Expected UnicodeProperty");
    }

    // A9: without unicode flag, \p is identity escape
    let node = parse_pattern(r"\p{Letter}").unwrap();
    assert!(!matches!(node, RegExpNode::UnicodeProperty { .. }));
}

#[test]
fn flags_validation() {
    assert!(parse_flags("gi").is_ok());
    assert!(parse_flags("gimsuy").is_ok());
    assert!(parse_flags("d").is_ok());

    // Duplicates
    assert!(parse_flags("gg").is_err());

    // Invalid
    assert!(parse_flags("z").is_err());

    // u and v mutually exclusive
    assert!(parse_flags("uv").is_err());
}

#[test]
fn hex_escape_in_pattern() {
    let node = parse_pattern(r"\x41").unwrap();
    assert!(matches!(node, RegExpNode::Escape(EscapeKind::Hex('A'))));
}

#[test]
fn unicode_escape_in_pattern() {
    let node = parse_pattern(r"\u0041").unwrap();
    assert!(matches!(node, RegExpNode::Escape(EscapeKind::Unicode('A'))));

    // B6: \u{...} requires unicode flag
    let flags = parse_flags("u").unwrap();
    let node = parse_pattern_with_flags(r"\u{1F600}", &flags).unwrap();
    assert!(matches!(
        node,
        RegExpNode::Escape(EscapeKind::Unicode('\u{1F600}'))
    ));
}

// ── M9: unterminated \u{...} ──

#[test]
fn unterminated_unicode_brace_escape() {
    // B6: \u{...} requires unicode flag; test with unicode mode
    let flags = parse_flags("u").unwrap();
    let result = parse_pattern_with_flags(r"\u{41", &flags);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.message.contains("Unterminated"),
        "Expected unterminated error, got: {}",
        err.message
    );
}

// ── L7: backreference overflow ──

#[test]
fn backreference_large_number() {
    // R7: invalid backreferences are always errors (no Annex B)
    let result = parse_pattern(r"\99999999999");
    assert!(result.is_err(), "Large backreference should error");
}

// ── Step 7: A12 — character class range validation ──

#[test]
fn char_class_range_out_of_order() {
    let result = parse_pattern("[z-a]");
    assert!(result.is_err());
    assert!(
        result.unwrap_err().message.contains("out of order"),
        "Expected range out of order error"
    );
}

#[test]
fn char_class_escape_as_range_endpoint() {
    let result = parse_pattern(r"[\d-z]");
    assert!(result.is_err());
    assert!(
        result.unwrap_err().message.contains("Invalid range"),
        "Expected invalid range error"
    );
}

// ── Step 7: A14 — named group validation ──

#[test]
fn group_name_digit_start_error() {
    let result = parse_pattern("(?<1abc>x)");
    assert!(result.is_err());
    assert!(
        result.unwrap_err().message.contains("must start with"),
        "Expected invalid-start error"
    );
}

#[test]
fn duplicate_group_name_error() {
    let result = parse_pattern("(?<foo>a)(?<foo>b)");
    assert!(result.is_err());
    assert!(
        result.unwrap_err().message.contains("Duplicate"),
        "Expected duplicate group name error"
    );
}

// ── Step 7: B25 — quantifier range out of order ──

#[test]
fn quantifier_range_out_of_order() {
    let result = parse_pattern("a{5,2}");
    assert!(result.is_err());
    assert!(
        result.unwrap_err().message.contains("out of order"),
        "Expected quantifier range error"
    );
}

// ── Step 7: B26 — unicode escape overflow ──

#[test]
fn unicode_escape_overflow() {
    // B6: \u{...} requires unicode flag
    let flags = parse_flags("u").unwrap();
    let result = parse_pattern_with_flags(r"\u{110000}", &flags);
    assert!(result.is_err());
    assert!(
        result.unwrap_err().message.contains("exceeds"),
        "Expected unicode overflow error"
    );
}

#[test]
fn unicode_escape_max_valid() {
    let flags = parse_flags("u").unwrap();
    let result = parse_pattern_with_flags(r"\u{10FFFF}", &flags);
    assert!(result.is_ok());
}

// ── Step 7: A13 — \0 + digit in unicode mode ──

#[test]
fn null_escape_digit_unicode_error() {
    let flags = RegExpFlags {
        unicode: true,
        ..Default::default()
    };
    let result = parse_pattern_with_flags(r"\01", &flags);
    assert!(result.is_err());
    assert!(
        result.unwrap_err().message.contains("\\0"),
        "Expected \\0+digit error in unicode mode"
    );
}

#[test]
fn null_escape_no_digit_ok() {
    let flags = RegExpFlags {
        unicode: true,
        ..Default::default()
    };
    let result = parse_pattern_with_flags(r"\0a", &flags);
    assert!(result.is_ok());
}

// ── A20: unicode mode identity escape ──

#[test]
fn identity_escape_unicode_mode_error() {
    let flags = RegExpFlags {
        unicode: true,
        ..Default::default()
    };
    let result = parse_pattern_with_flags(r"\a", &flags);
    assert!(result.is_err());
    assert!(
        result.unwrap_err().message.contains("identity escape"),
        "Expected identity escape error in unicode mode"
    );
}

#[test]
fn syntax_char_escape_unicode_mode_ok() {
    let flags = RegExpFlags {
        unicode: true,
        ..Default::default()
    };
    assert!(parse_pattern_with_flags(r"\$", &flags).is_ok());
    assert!(parse_pattern_with_flags(r"\.", &flags).is_ok());
    assert!(parse_pattern_with_flags(r"\*", &flags).is_ok());
}

#[test]
fn identity_escape_non_unicode_ok() {
    let result = parse_pattern(r"\a");
    assert!(result.is_ok());
}

// ── B30: ES2025 duplicate named groups in different alternatives ──

#[test]
fn duplicate_named_group_different_alts_ok() {
    let result = parse_pattern("(?<x>a)|(?<x>b)");
    assert!(
        result.is_ok(),
        "Should allow duplicate names in different alts"
    );
}

#[test]
fn duplicate_named_group_same_alt_error() {
    let result = parse_pattern("(?<x>a)(?<x>b)");
    assert!(result.is_err());
    assert!(
        result.unwrap_err().message.contains("Duplicate"),
        "Expected duplicate group name error within same alt"
    );
}

#[test]
fn duplicate_named_group_nested_ok() {
    let result = parse_pattern("((?<x>a)|(?<x>b))|((?<x>c)|(?<x>d))");
    assert!(
        result.is_ok(),
        "Should allow duplicate names in nested alts"
    );
}

#[test]
fn duplicate_named_group_cross_group_error() {
    // After group ((?<x>a)|(?<y>b)), name "x" is merged.
    // Then (?<x>c) is in the same alternative as the group, so "x" is duplicate.
    let result = parse_pattern("((?<x>a)|(?<y>b))(?<x>c)");
    assert!(result.is_err());
    assert!(
        result.unwrap_err().message.contains("Duplicate"),
        "Expected duplicate group name error across groups"
    );
}

// ── B3: backreference count validation (unicode mode) ──

#[test]
fn backreference_valid_unicode() {
    let u_flags = RegExpFlags {
        unicode: true,
        ..Default::default()
    };
    let result = parse_pattern_with_flags(r"(a)\1", &u_flags);
    assert!(result.is_ok(), "\\1 with 1 group should be ok: {result:?}");
}

#[test]
fn backreference_invalid_unicode_error() {
    let u_flags = RegExpFlags {
        unicode: true,
        ..Default::default()
    };
    let result = parse_pattern_with_flags(r"(a)\2", &u_flags);
    assert!(
        result.is_err(),
        "\\2 with only 1 group should error in unicode mode"
    );
}

#[test]
fn backreference_invalid_non_unicode_error() {
    // R7: invalid backreferences are always errors (no Annex B)
    let result = parse_pattern(r"\2");
    assert!(
        result.is_err(),
        "\\2 without groups should error (no Annex B)"
    );
}

// ── B4: named backreference validation ──

#[test]
fn named_backreference_valid() {
    let result = parse_pattern(r"(?<foo>a)\k<foo>");
    assert!(
        result.is_ok(),
        "\\k<foo> with named group should be ok: {result:?}"
    );
}

#[test]
fn named_backreference_missing_group_error() {
    let result = parse_pattern(r"\k<missing>");
    assert!(
        result.is_err(),
        "\\k<missing> without a named group should error"
    );
}

// ── B6: \cX control escape ──

#[test]
fn control_escape_uppercase() {
    let result = parse_pattern(r"\cA");
    assert!(result.is_ok(), "\\cA should be ok: {result:?}");
}

#[test]
fn control_escape_lowercase() {
    let result = parse_pattern(r"\ca");
    assert!(result.is_ok(), "\\ca should be ok: {result:?}");
}

#[test]
fn control_escape_invalid_unicode_error() {
    let u_flags = RegExpFlags {
        unicode: true,
        ..Default::default()
    };
    let result = parse_pattern_with_flags(r"\c1", &u_flags);
    assert!(result.is_err(), "\\c1 in unicode mode should error");
}

#[test]
fn control_escape_invalid_non_unicode_error() {
    // R3: \c followed by non-letter is always an error (no Annex B)
    let result = parse_pattern(r"\c1");
    assert!(result.is_err(), "\\c1 should error without Annex B");
}

#[cfg(test)]
mod edge_case_tests {
    use crate::regexp::parse_pattern;

    #[test]
    fn quantified_assertion_lookahead() {
        // E5/§22.2.1: Lookahead is QuantifiableAssertion — quantifiers are allowed
        let result = parse_pattern("(?=a)*");
        assert!(
            result.is_ok(),
            "(?=a)* should be valid — lookahead is quantifiable"
        );

        let result = parse_pattern("(?!a)+");
        assert!(
            result.is_ok(),
            "(?!a)+ should be valid — negative lookahead is quantifiable"
        );

        // Lookbehind is NOT quantifiable
        let result = parse_pattern("(?<=a)*");
        assert!(
            result.is_err(),
            "(?<=a)* should error — lookbehind is not quantifiable"
        );
    }

    #[test]
    fn quantified_assertion_word_boundary() {
        let result = parse_pattern(r"\B+");
        assert!(
            result.is_err(),
            r"\B+ should error - assertion cannot be quantified"
        );
    }

    #[test]
    fn quantified_assertion_at_start() {
        let result = parse_pattern("^+");
        assert!(
            result.is_err(),
            "^+ should error - assertion cannot be quantified"
        );
    }

    #[test]
    fn empty_char_class() {
        let result = parse_pattern("[]");
        // Empty char class [] should be valid per ES spec (matches nothing)
        assert!(result.is_ok(), "[] should be valid (empty character class)");
    }

    #[test]
    fn nothing_to_repeat_star() {
        let result = parse_pattern("*");
        assert!(result.is_err(), "* at start should error");
    }

    #[test]
    fn nothing_to_repeat_plus() {
        let result = parse_pattern("+");
        assert!(result.is_err(), "+ at start should error");
    }

    // H1: \b in character class means backspace (U+0008)
    #[test]
    fn backslash_b_in_char_class_is_backspace() {
        let result = parse_pattern(r"[\b]");
        assert!(result.is_ok(), r"[\b] should be valid (backspace)");
    }

    #[test]
    fn backslash_b_in_char_class_unicode_mode() {
        use crate::regexp::{parse_pattern_with_flags, RegExpFlags};
        let flags = RegExpFlags {
            unicode: true,
            ..Default::default()
        };
        let result = parse_pattern_with_flags(r"[\b]", &flags);
        assert!(result.is_ok(), r"[\b] should be valid in unicode mode too");
    }

    // M1: \B in character class is always an error
    #[test]
    fn backslash_big_b_in_char_class_error() {
        let result = parse_pattern(r"[\B]");
        assert!(result.is_err(), r"[\B] should error inside character class");
    }

    // M5: surrogate code points in unicode mode
    #[test]
    fn surrogate_escape_unicode_mode_error() {
        use crate::regexp::{parse_pattern_with_flags, RegExpFlags};
        let flags = RegExpFlags {
            unicode: true,
            ..Default::default()
        };
        let result = parse_pattern_with_flags(r"\u{D800}", &flags);
        assert!(
            result.is_err(),
            "Surrogate U+D800 should error in unicode mode"
        );
        let msg = result.unwrap_err().message;
        assert!(
            msg.contains("Surrogate") || msg.contains("surrogate"),
            "{msg}"
        );
    }

    #[test]
    fn surrogate_escape_non_unicode_ok() {
        // B6: \u{...} is not valid without unicode flag, so test with \uD800 (4-digit form)
        // In non-unicode mode, \uD800 is treated as literal (becomes FFFD silently)
        let result = parse_pattern(r"\uD800");
        assert!(
            result.is_ok(),
            "Surrogate should be accepted in non-unicode mode"
        );
    }

    // S2: quantifier overflow must be an error (not silently saturated)
    #[test]
    fn quantifier_overflow_large_number() {
        let result = parse_pattern(r"a{99999999999}");
        assert!(result.is_err(), "Overflowed quantifier should be an error");
        assert!(
            result.unwrap_err().message.contains("too large"),
            "Error message should mention overflow"
        );
    }

    // A10: braced quantifier position restore on parse_digits failure
    #[test]
    fn lone_brace_non_unicode_literal() {
        // `{` not forming a valid quantifier should be treated as literal in non-unicode mode
        let result = parse_pattern(r"a{b}");
        assert!(
            result.is_ok(),
            "Lone {{ in non-unicode should be literal: {result:?}"
        );
    }

    // B8: lone `{` in unicode mode is an error
    #[test]
    fn lone_brace_unicode_error() {
        use crate::regexp::{parse_flags, parse_pattern_with_flags};
        let flags = parse_flags("u").unwrap();
        let result = parse_pattern_with_flags(r"a{b}", &flags);
        assert!(
            result.is_err(),
            "Lone {{ in unicode mode should be an error"
        );
    }

    #[test]
    fn identity_escape_brace_non_unicode() {
        // `\p{Letter}` without unicode flag — \p is identity escape, {Letter} is literal chars
        let result = parse_pattern(r"\p{Letter}");
        assert!(
            result.is_ok(),
            "\\p{{Letter}} in non-unicode should succeed: {result:?}"
        );
    }

    // B6: \u{...} without unicode flag
    #[test]
    fn braced_unicode_escape_requires_unicode_flag() {
        let result = parse_pattern(r"\u{41}");
        assert!(result.is_err(), "\\u{{}} without unicode flag should error");
        assert!(result.unwrap_err().message.contains("unicode flag"));
    }

    // B7: \uHHHH surrogate in unicode mode
    #[test]
    fn surrogate_4digit_unicode_error() {
        use crate::regexp::{parse_flags, parse_pattern_with_flags};
        let flags = parse_flags("u").unwrap();
        let result = parse_pattern_with_flags(r"\uD800", &flags);
        assert!(
            result.is_err(),
            "Surrogate \\uD800 in unicode mode should error"
        );
        assert!(result.unwrap_err().message.contains("Surrogate"));
    }

    #[test]
    fn surrogate_4digit_non_unicode_ok() {
        // Non-unicode mode allows surrogates
        let result = parse_pattern(r"\uD800");
        assert!(result.is_ok(), "Surrogate in non-unicode mode should be OK");
    }
}

// ── T4: Unicode escapes in group names ──

#[test]
fn group_name_unicode_escape_ok() {
    let result = parse_pattern(r"(?<\u0041>a)");
    assert!(
        result.is_ok(),
        "Unicode escape in group name should work: {result:?}"
    );
}

#[test]
fn group_name_unicode_brace_escape_ok() {
    let result = parse_pattern(r"(?<\u{42}>b)");
    assert!(
        result.is_ok(),
        "Braced unicode escape in group name should work: {result:?}"
    );
}

#[test]
fn group_name_unicode_non_id_start_error() {
    // U+0030 = '0', not a valid ID_Start character
    let result = parse_pattern(r"(?<\u0030>a)");
    assert!(
        result.is_err(),
        "Digit as first char in group name should error"
    );
}

// ── T3: v flag (unicodeSets) character class extensions ──

mod v_flag {
    use crate::regexp::{parse_pattern_with_flags, RegExpFlags, RegExpNode};

    fn parse_v(pattern: &str) -> Result<RegExpNode, crate::regexp::RegExpError> {
        let flags = RegExpFlags {
            unicode_sets: true,
            ..Default::default()
        };
        parse_pattern_with_flags(pattern, &flags)
    }

    #[test]
    fn nested_char_class_ok() {
        let result = parse_v("[[a-z]]");
        assert!(result.is_ok(), "Nested char class should parse: {result:?}");
    }

    #[test]
    fn intersection_ok() {
        let result = parse_v(r"[[a-z]&&[^aeiou]]");
        assert!(result.is_ok(), "Intersection should parse: {result:?}");
    }

    #[test]
    fn subtraction_ok() {
        let result = parse_v(r"[[a-z]--[aeiou]]");
        assert!(result.is_ok(), "Subtraction should parse: {result:?}");
    }

    #[test]
    fn mixed_operators_error() {
        let result = parse_v(r"[a&&b--c]");
        assert!(result.is_err(), "Mixed && and -- should error");
        assert!(result.unwrap_err().message.contains("Cannot mix"));
    }

    #[test]
    fn string_alternative_ok() {
        let result = parse_v(r"[\q{abc|def}]");
        assert!(
            result.is_ok(),
            "String alternative should parse: {result:?}"
        );
    }

    #[test]
    fn simple_class_v_mode_ok() {
        let result = parse_v("[abc]");
        assert!(
            result.is_ok(),
            "Simple class in v mode should parse: {result:?}"
        );
    }

    #[test]
    fn identity_escape_v_mode_restricted() {
        // v flag implies unicode semantics — identity escapes are restricted
        let result = parse_v(r"\a");
        assert!(
            result.is_err(),
            "Identity escape \\a should be rejected in v mode"
        );
    }

    // ── E1: negated sequence property ──

    #[test]
    fn negated_sequence_property_error() {
        // \P{Basic_Emoji} must be a SyntaxError (negation of sequence properties)
        let flags = RegExpFlags {
            unicode_sets: true,
            ..Default::default()
        };
        let result = parse_pattern_with_flags(r"\P{Basic_Emoji}", &flags);
        assert!(result.is_err(), "\\P{{Basic_Emoji}} should be rejected");
    }

    #[test]
    fn non_negated_sequence_property_ok() {
        // \p{Basic_Emoji} is fine
        let result = parse_v(r"\p{Basic_Emoji}");
        assert!(result.is_ok(), "\\p{{Basic_Emoji}} should be accepted: {result:?}");
    }

    // ── E2: Old_Sogdian alias fix ──

    #[test]
    fn old_sogdian_alias_sogo() {
        let flags = RegExpFlags {
            unicode: true,
            ..Default::default()
        };
        let result = parse_pattern_with_flags(r"\p{sc=Sogo}", &flags);
        assert!(result.is_ok(), "Old_Sogdian alias Sogo should be accepted: {result:?}");
    }
}
