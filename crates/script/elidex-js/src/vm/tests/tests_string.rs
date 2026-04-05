use super::{eval_bool, eval_number, eval_string};

// ── String.prototype methods ────────────────────────────────────

#[test]
fn eval_string_char_at() {
    assert_eq!(eval_string("'hello'.charAt(1);"), "e");
    assert_eq!(eval_string("'hello'.charAt(0);"), "h");
    assert_eq!(eval_string("'hello'.charAt(10);"), "");
}

#[test]
fn eval_string_char_code_at() {
    assert_eq!(eval_number("'A'.charCodeAt(0);"), 65.0);
    assert!(eval_number("'hello'.charCodeAt(10);").is_nan());
}

#[test]
fn eval_string_index_of() {
    assert_eq!(eval_number("'hello'.indexOf('l');"), 2.0);
    assert_eq!(eval_number("'hello'.indexOf('z');"), -1.0);
    assert_eq!(eval_number("'hello'.indexOf('l', 3);"), 3.0);
}

#[test]
fn eval_string_includes() {
    assert!(eval_bool("'hello'.includes('ell');"));
    assert!(!eval_bool("'hello'.includes('xyz');"));
}

#[test]
fn eval_string_slice() {
    assert_eq!(eval_string("'hello'.slice(1, 3);"), "el");
    assert_eq!(eval_string("'hello'.slice(-3);"), "llo");
    assert_eq!(eval_string("'hello'.slice(1);"), "ello");
}

#[test]
fn eval_string_substring() {
    assert_eq!(eval_string("'hello'.substring(1, 3);"), "el");
    assert_eq!(eval_string("'hello'.substring(3, 1);"), "el");
}

#[test]
fn eval_string_to_case() {
    assert_eq!(eval_string("'Hello'.toLowerCase();"), "hello");
    assert_eq!(eval_string("'Hello'.toUpperCase();"), "HELLO");
}

#[test]
fn eval_string_trim() {
    assert_eq!(eval_string("'  hello  '.trim();"), "hello");
}

#[test]
fn eval_string_split() {
    assert_eq!(eval_number("'a,b,c'.split(',').length;"), 3.0);
}

#[test]
fn eval_string_starts_ends_with() {
    assert!(eval_bool("'hello'.startsWith('hel');"));
    assert!(!eval_bool("'hello'.startsWith('llo');"));
    assert!(eval_bool("'hello'.endsWith('llo');"));
    assert!(!eval_bool("'hello'.endsWith('hel');"));
}

#[test]
fn eval_string_replace() {
    assert_eq!(
        eval_string("'hello world'.replace('world', 'rust');"),
        "hello rust"
    );
    // Only replaces first occurrence.
    assert_eq!(eval_string("'aaa'.replace('a', 'b');"), "baa");
}

// ── M4-10.1: String UTF-16 support ─────────────────────────────────

#[test]
fn eval_string_bracket_access() {
    assert_eq!(eval_string("'hello'[1];"), "e");
}

#[test]
fn eval_string_bracket_access_out_of_bounds() {
    assert_eq!(eval_string("typeof 'hi'[5];"), "undefined");
}

#[test]
fn eval_string_char_code_at_utf16() {
    // U+1F600 (😀) encodes as surrogate pair: 0xD83D 0xDE00.
    // charCodeAt(0) should return the high surrogate 0xD83D = 55357.
    assert_eq!(eval_number("'\u{1F600}'.charCodeAt(0);"), 55357.0);
}

#[test]
fn eval_string_char_code_at_low_surrogate() {
    // charCodeAt(1) should return the low surrogate 0xDE00 = 56832.
    assert_eq!(eval_number("'\u{1F600}'.charCodeAt(1);"), 56832.0);
}

#[test]
fn eval_char_at_negative_index() {
    // §21.1.3.1: if pos < 0, return ""
    assert_eq!(eval_string("'abc'.charAt(-1);"), "");
}

#[test]
fn eval_char_code_at_negative_index() {
    // §21.1.3.2: if pos < 0, return NaN
    assert!(eval_number("'abc'.charCodeAt(-1);").is_nan());
}

#[test]
fn eval_string_index_of_utf16() {
    // 'a' + U+1F600 (2 code units) + 'b' → indexOf('b') = 3.
    assert_eq!(eval_number("'a\u{1F600}b'.indexOf('b');"), 3.0);
}

#[test]
fn eval_string_slice_utf16() {
    // slice(1, 3) on 'a' + U+1F600 + 'b' extracts the emoji.
    assert_eq!(eval_string("'a\u{1F600}b'.slice(1, 3);"), "\u{1F600}");
}

#[test]
fn eval_string_substring_utf16() {
    assert_eq!(eval_string("'a\u{1F600}b'.substring(1, 3);"), "\u{1F600}");
}

#[test]
fn eval_string_length_utf16() {
    // Surrogate pair = 2 code units.
    assert_eq!(eval_number("'\u{1F600}'.length;"), 2.0);
}

#[test]
fn eval_string_char_at_bmp() {
    assert_eq!(eval_string("'abc'.charAt(1);"), "b");
}

#[test]
fn eval_string_index_of_bmp() {
    assert_eq!(eval_number("'abcdef'.indexOf('cd');"), 2.0);
}

// ── M4-10.1: String method position arguments ──────────────────────

#[test]
fn eval_string_includes_position() {
    assert_eq!(eval_number("'abcabc'.includes('abc', 4) ? 1 : 0;"), 0.0);
    assert_eq!(eval_number("'abcabc'.includes('abc', 3) ? 1 : 0;"), 1.0);
}

#[test]
fn eval_string_starts_with_position() {
    assert_eq!(eval_number("'foobar'.startsWith('bar', 3) ? 1 : 0;"), 1.0);
    assert_eq!(eval_number("'foobar'.startsWith('foo', 1) ? 1 : 0;"), 0.0);
}

#[test]
fn eval_string_ends_with_end_position() {
    assert_eq!(eval_number("'foobar'.endsWith('foo', 3) ? 1 : 0;"), 1.0);
    assert_eq!(eval_number("'foobar'.endsWith('bar', 3) ? 1 : 0;"), 0.0);
}

#[test]
fn eval_lone_surrogate_length() {
    // \uD800 is a lone high surrogate — length should be 1
    assert_eq!(eval_number("'\\uD800'.length;"), 1.0);
}

#[test]
fn eval_char_at_negative_fraction_truncates_to_zero() {
    // charAt(-0.5) should behave like charAt(0) per ES2020 ToInteger (trunc, not floor).
    assert_eq!(eval_string("'abc'.charAt(-0.5);"), "a");
}

#[test]
fn eval_lone_surrogate_char_code_at() {
    assert_eq!(
        eval_number("'\\uD800'.charCodeAt(0);"),
        f64::from(0xD800_i32)
    );
}

#[test]
fn eval_string_bracket_string_key() {
    // String bracket access with a string key that is a numeric index.
    assert_eq!(eval_string("'abc'['1'];"), "b");
}

#[test]
fn eval_string_bracket_string_key_zero() {
    assert_eq!(eval_string("'abc'['0'];"), "a");
}

#[test]
fn eval_string_bracket_non_numeric_string_key() {
    // Non-numeric string keys return undefined.
    assert!(eval_bool("'abc'['foo'] === undefined;"));
}
