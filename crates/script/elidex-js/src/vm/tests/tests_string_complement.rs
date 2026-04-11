//! Tests for String.prototype P2 additions.

use super::{eval_bool, eval_number, eval_string, eval_throws};

// -- String.prototype.repeat --------------------------------------------------

#[test]
fn string_repeat_basic() {
    assert_eq!(eval_string("'abc'.repeat(3);"), "abcabcabc");
}

#[test]
fn string_repeat_zero() {
    assert_eq!(eval_string("'abc'.repeat(0);"), "");
}

#[test]
fn string_repeat_one() {
    assert_eq!(eval_string("'abc'.repeat(1);"), "abc");
}

#[test]
fn string_repeat_negative_throws() {
    eval_throws("'a'.repeat(-1);");
}

#[test]
fn string_repeat_infinity_throws() {
    eval_throws("'a'.repeat(Infinity);");
}

#[test]
fn string_repeat_nan_is_zero() {
    assert_eq!(eval_string("'abc'.repeat(NaN);"), "");
}

// -- String.prototype.padStart ------------------------------------------------

#[test]
fn string_pad_start_basic() {
    assert_eq!(eval_string("'abc'.padStart(6);"), "   abc");
}

#[test]
fn string_pad_start_fill() {
    assert_eq!(eval_string("'abc'.padStart(6, '0');"), "000abc");
}

#[test]
fn string_pad_start_truncates_fill() {
    assert_eq!(eval_string("'abc'.padStart(5, '12');"), "12abc");
}

#[test]
fn string_pad_start_no_pad_needed() {
    assert_eq!(eval_string("'abc'.padStart(2);"), "abc");
}

// -- String.prototype.padEnd --------------------------------------------------

#[test]
fn string_pad_end_basic() {
    assert_eq!(eval_string("'abc'.padEnd(6);"), "abc   ");
}

#[test]
fn string_pad_end_fill() {
    assert_eq!(eval_string("'abc'.padEnd(6, '0');"), "abc000");
}

#[test]
fn string_pad_end_truncates_fill() {
    assert_eq!(eval_string("'abc'.padEnd(5, '12');"), "abc12");
}

// -- String.prototype.trimStart -----------------------------------------------

#[test]
fn string_trim_start_basic() {
    assert_eq!(eval_string("'  hello  '.trimStart();"), "hello  ");
}

#[test]
fn string_trim_start_no_whitespace() {
    assert_eq!(eval_string("'hello'.trimStart();"), "hello");
}

#[test]
fn string_trim_left_alias() {
    assert_eq!(eval_string("'  hello'.trimLeft();"), "hello");
}

// -- String.prototype.trimEnd -------------------------------------------------

#[test]
fn string_trim_end_basic() {
    assert_eq!(eval_string("'  hello  '.trimEnd();"), "  hello");
}

#[test]
fn string_trim_right_alias() {
    assert_eq!(eval_string("'hello  '.trimRight();"), "hello");
}

#[test]
fn string_trim_end_no_whitespace() {
    assert_eq!(eval_string("'hello'.trimEnd();"), "hello");
}

// -- String.prototype.lastIndexOf ---------------------------------------------

#[test]
fn string_last_index_of_basic() {
    assert_eq!(eval_number("'abcabc'.lastIndexOf('abc');"), 3.0);
}

#[test]
fn string_last_index_of_not_found() {
    assert_eq!(eval_number("'abc'.lastIndexOf('xyz');"), -1.0);
}

#[test]
fn string_last_index_of_with_pos() {
    assert_eq!(eval_number("'abcabc'.lastIndexOf('abc', 2);"), 0.0);
}

#[test]
fn string_last_index_of_empty_search() {
    assert_eq!(eval_number("'abc'.lastIndexOf('');"), 3.0);
}

#[test]
fn string_last_index_of_nan_pos() {
    // NaN position → search entire string
    assert_eq!(eval_number("'abcabc'.lastIndexOf('abc', NaN);"), 3.0);
}

// -- String.prototype.codePointAt ---------------------------------------------

#[test]
fn string_code_point_at_basic() {
    assert_eq!(eval_number("'ABC'.codePointAt(0);"), 65.0);
}

#[test]
fn string_code_point_at_out_of_range() {
    assert!(eval_bool("'abc'.codePointAt(5) === undefined;"));
}

#[test]
fn string_code_point_at_surrogate_pair() {
    // U+1F600 (😀) is a surrogate pair
    assert_eq!(eval_number("'\\uD83D\\uDE00'.codePointAt(0);"), 128_512.0);
}

#[test]
fn string_code_point_at_lone_surrogate() {
    // Lone high surrogate returns the surrogate code unit
    assert_eq!(
        eval_number("'\\uD83D'.codePointAt(0);"),
        f64::from(0xD83Di32)
    );
}

// -- String.prototype.replaceAll ----------------------------------------------

#[test]
fn string_replace_all_basic() {
    assert_eq!(eval_string("'aabbcc'.replaceAll('b', 'x');"), "aaxxcc");
}

#[test]
fn string_replace_all_no_match() {
    assert_eq!(eval_string("'abc'.replaceAll('x', 'y');"), "abc");
}

#[test]
fn string_replace_all_empty_search() {
    assert_eq!(eval_string("'ab'.replaceAll('', '-');"), "-a-b-");
}

// -- String.prototype.concat --------------------------------------------------

#[test]
fn string_concat_basic() {
    assert_eq!(eval_string("'hello'.concat(' ', 'world');"), "hello world");
}

#[test]
fn string_concat_no_args() {
    assert_eq!(eval_string("'hello'.concat();"), "hello");
}

// -- String.fromCharCode ------------------------------------------------------

#[test]
fn string_from_char_code_basic() {
    assert_eq!(
        eval_string("String.fromCharCode(72, 101, 108, 108, 111);"),
        "Hello"
    );
}

#[test]
fn string_from_char_code_unicode() {
    assert_eq!(eval_string("String.fromCharCode(0x2764);"), "\u{2764}");
}

#[test]
fn string_from_char_code_negative() {
    // ToUint16(-1) = 65535 = 0xFFFF
    assert_eq!(
        eval_number("String.fromCharCode(-1).charCodeAt(0);"),
        65535.0
    );
}

// -- String.fromCodePoint -----------------------------------------------------

#[test]
fn string_from_code_point_basic() {
    assert_eq!(eval_string("String.fromCodePoint(65, 66, 67);"), "ABC");
}

#[test]
fn string_from_code_point_supplementary() {
    // U+1F600 (😀) requires surrogate pair encoding
    assert_eq!(eval_string("String.fromCodePoint(0x1F600);"), "\u{1F600}");
}

#[test]
fn string_from_code_point_invalid_throws() {
    eval_throws("String.fromCodePoint(-1);");
    eval_throws("String.fromCodePoint(0x110000);");
    eval_throws("String.fromCodePoint(3.14);");
}

// -- StringWrapper own property -----------------------------------------------
// NOTE: StringWrapper index/length tests deferred — requires `new String()`
// constructor (not yet implemented). The property hook is in place in
// ops_property.rs but cannot be exercised from JS yet.

// -- Global URI encoding/decoding ---------------------------------------------

#[test]
fn encode_uri_basic() {
    assert_eq!(
        eval_string("encodeURI('https://example.com/path?q=hello world');"),
        "https://example.com/path?q=hello%20world"
    );
}

#[test]
fn encode_uri_preserves_reserved() {
    // encodeURI does NOT encode reserved characters like : / ? # & = +
    assert_eq!(eval_string("encodeURI(':/?#&=+');"), ":/?#&=+");
}

#[test]
fn encode_uri_component_basic() {
    assert_eq!(
        eval_string("encodeURIComponent('hello world');"),
        "hello%20world"
    );
}

#[test]
fn encode_uri_component_encodes_reserved() {
    // encodeURIComponent DOES encode reserved characters
    assert_eq!(
        eval_string("encodeURIComponent('a=b&c=d');"),
        "a%3Db%26c%3Dd"
    );
}

#[test]
fn decode_uri_basic() {
    assert_eq!(eval_string("decodeURI('hello%20world');"), "hello world");
}

#[test]
fn decode_uri_preserves_reserved_encoding() {
    // decodeURI does NOT decode reserved characters
    assert_eq!(eval_string("decodeURI('%23');"), "%23"); // # is reserved
}

#[test]
fn decode_uri_component_basic() {
    assert_eq!(
        eval_string("decodeURIComponent('hello%20world');"),
        "hello world"
    );
}

#[test]
fn decode_uri_component_decodes_reserved() {
    assert_eq!(eval_string("decodeURIComponent('%23');"), "#");
}

#[test]
fn decode_uri_malformed_throws() {
    eval_throws("decodeURI('%');");
    eval_throws("decodeURI('%ZZ');");
}

// -- globalThis ---------------------------------------------------------------

#[test]
fn global_this_exists() {
    assert_eq!(eval_string("typeof globalThis;"), "object");
}

#[test]
fn global_this_has_math() {
    assert_eq!(eval_string("typeof globalThis.Math;"), "object");
}

// -- SyntaxError / URIError constructors --------------------------------------

#[test]
fn syntax_error_constructor() {
    assert_eq!(
        eval_string("var e = new SyntaxError('bad'); e.message;"),
        "bad"
    );
}

#[test]
fn uri_error_constructor() {
    assert_eq!(
        eval_string("var e = new URIError('malformed'); e.message;"),
        "malformed"
    );
}
