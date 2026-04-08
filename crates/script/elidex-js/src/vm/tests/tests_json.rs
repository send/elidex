use super::{eval, eval_bool, eval_number, eval_string, eval_throws};
use crate::vm::value::JsValue;

// ============================================================================
// JSON.stringify — primitives
// ============================================================================

#[test]
fn stringify_null() {
    assert_eq!(eval_string("JSON.stringify(null)"), "null");
}

#[test]
fn stringify_true() {
    assert_eq!(eval_string("JSON.stringify(true)"), "true");
}

#[test]
fn stringify_false() {
    assert_eq!(eval_string("JSON.stringify(false)"), "false");
}

#[test]
fn stringify_number() {
    assert_eq!(eval_string("JSON.stringify(42)"), "42");
    assert_eq!(eval_string("JSON.stringify(3.14)"), "3.14");
    assert_eq!(eval_string("JSON.stringify(-0)"), "0");
}

#[test]
fn stringify_string() {
    assert_eq!(eval_string(r#"JSON.stringify("hello")"#), r#""hello""#);
}

#[test]
fn stringify_nan_infinity() {
    assert_eq!(eval_string("JSON.stringify(NaN)"), "null");
    assert_eq!(eval_string("JSON.stringify(Infinity)"), "null");
    assert_eq!(eval_string("JSON.stringify(-Infinity)"), "null");
}

#[test]
fn stringify_undefined_returns_undefined() {
    assert!(matches!(
        eval("JSON.stringify(undefined)"),
        Ok(JsValue::Undefined)
    ));
}

#[test]
fn stringify_function_returns_undefined() {
    assert!(matches!(
        eval("JSON.stringify(function(){})"),
        Ok(JsValue::Undefined)
    ));
}

// ============================================================================
// JSON.stringify — objects and arrays
// ============================================================================

#[test]
fn stringify_empty_object() {
    assert_eq!(eval_string("JSON.stringify({})"), "{}");
}

#[test]
fn stringify_empty_array() {
    assert_eq!(eval_string("JSON.stringify([])"), "[]");
}

#[test]
fn stringify_object() {
    assert_eq!(
        eval_string(r#"JSON.stringify({a: 1, b: "two"})"#),
        r#"{"a":1,"b":"two"}"#
    );
}

#[test]
fn stringify_array() {
    assert_eq!(eval_string("JSON.stringify([1, 2, 3])"), "[1,2,3]");
}

#[test]
fn stringify_nested() {
    assert_eq!(
        eval_string(r"JSON.stringify({a: [1, {b: 2}]})"),
        r#"{"a":[1,{"b":2}]}"#
    );
}

#[test]
fn stringify_array_with_undefined() {
    // undefined in arrays becomes "null"
    assert_eq!(
        eval_string("JSON.stringify([1, undefined, 3])"),
        "[1,null,3]"
    );
}

#[test]
fn stringify_object_skip_undefined() {
    // undefined values in objects are skipped
    assert_eq!(
        eval_string("JSON.stringify({a: 1, b: undefined, c: 3})"),
        r#"{"a":1,"c":3}"#
    );
}

#[test]
fn stringify_object_skip_function() {
    assert_eq!(
        eval_string("JSON.stringify({a: 1, b: function(){}, c: 3})"),
        r#"{"a":1,"c":3}"#
    );
}

// ============================================================================
// JSON.stringify — string escaping
// ============================================================================

#[test]
fn stringify_escape_quote() {
    assert_eq!(eval_string(r#"JSON.stringify('a"b')"#), r#""a\"b""#);
}

#[test]
fn stringify_escape_backslash() {
    assert_eq!(eval_string(r#"JSON.stringify("a\\b")"#), r#""a\\b""#);
}

#[test]
fn stringify_escape_newline() {
    assert_eq!(eval_string(r#"JSON.stringify("a\nb")"#), r#""a\nb""#);
}

#[test]
fn stringify_escape_tab() {
    assert_eq!(eval_string(r#"JSON.stringify("a\tb")"#), r#""a\tb""#);
}

// ============================================================================
// JSON.stringify — space argument
// ============================================================================

#[test]
fn stringify_space_number() {
    assert_eq!(
        eval_string("JSON.stringify({a: 1}, null, 2)"),
        "{\n  \"a\": 1\n}"
    );
}

#[test]
fn stringify_space_string() {
    assert_eq!(
        eval_string(r#"JSON.stringify({a: 1}, null, "\t")"#),
        "{\n\t\"a\": 1\n}"
    );
}

#[test]
fn stringify_space_array() {
    assert_eq!(
        eval_string("JSON.stringify([1, 2], null, 2)"),
        "[\n  1,\n  2\n]"
    );
}

#[test]
fn stringify_space_nested() {
    assert_eq!(
        eval_string("JSON.stringify({a: {b: 1}}, null, 2)"),
        "{\n  \"a\": {\n    \"b\": 1\n  }\n}"
    );
}

// ============================================================================
// JSON.stringify — replacer
// ============================================================================

#[test]
fn stringify_replacer_function() {
    assert_eq!(
        eval_string(
            r#"JSON.stringify({a: 1, b: 2}, function(key, value) {
                if (key === "b") return undefined;
                return value;
            })"#
        ),
        r#"{"a":1}"#
    );
}

#[test]
fn stringify_replacer_array() {
    assert_eq!(
        eval_string(r#"JSON.stringify({a: 1, b: 2, c: 3}, ["a", "c"])"#),
        r#"{"a":1,"c":3}"#
    );
}

// ============================================================================
// JSON.stringify — toJSON
// ============================================================================

#[test]
fn stringify_to_json() {
    assert_eq!(
        eval_string(
            r#"var obj = { toJSON: function() { return "custom"; } };
            JSON.stringify(obj)"#
        ),
        r#""custom""#
    );
}

#[test]
fn stringify_to_json_with_key() {
    assert_eq!(
        eval_string(
            r#"var obj = { a: { toJSON: function(key) { return key + "!"; } } };
            JSON.stringify(obj)"#
        ),
        r#"{"a":"a!"}"#
    );
}

#[test]
fn stringify_to_json_non_callable_ignored() {
    // Non-callable toJSON is ignored per spec (IsCallable check).
    assert_eq!(
        eval_string(r"JSON.stringify({ toJSON: {} })"),
        r#"{"toJSON":{}}"#
    );
}

// ============================================================================
// JSON.stringify — Symbol handling
// ============================================================================

#[test]
fn stringify_symbol_value_skip() {
    // Symbol values in objects are skipped.
    assert_eq!(
        eval_string("JSON.stringify({a: 1, b: Symbol('x'), c: 3})"),
        r#"{"a":1,"c":3}"#
    );
    // Symbol values in arrays become "null".
    assert_eq!(
        eval_string("JSON.stringify([1, Symbol('x'), 3])"),
        "[1,null,3]"
    );
}

#[test]
fn stringify_symbol_key_skip() {
    // Symbol-keyed properties are not enumerated.
    assert_eq!(
        eval_string(
            r"var obj = {};
            obj.a = 1;
            obj[Symbol('hidden')] = 2;
            obj.b = 3;
            JSON.stringify(obj)"
        ),
        r#"{"a":1,"b":3}"#
    );
}

// ============================================================================
// JSON.stringify — accessor property (getter)
// ============================================================================

#[test]
fn stringify_getter() {
    assert_eq!(
        eval_string(
            r"var obj = {};
            Object.defineProperty(obj, 'x', { get: function() { return 42; }, enumerable: true });
            obj.y = 10;
            JSON.stringify(obj)"
        ),
        r#"{"x":42,"y":10}"#
    );
}

// ============================================================================
// JSON.stringify — BigInt TypeError
// ============================================================================

#[test]
fn stringify_bigint_throws() {
    eval_throws("JSON.stringify(1n)");
    eval_throws("JSON.stringify({a: 1n})");
    eval_throws("JSON.stringify([1n])");
}

// ============================================================================
// JSON.stringify — circular reference
// ============================================================================

#[test]
fn stringify_circular_throws() {
    eval_throws("var a = {}; a.self = a; JSON.stringify(a)");
    eval_throws("var a = []; a.push(a); JSON.stringify(a)");
}

// Wrapper object tests (new Number/String/Boolean) omitted:
// Number/String/Boolean constructors are not yet constructable in elidex-js VM.
// The wrapper unwrap logic in stringify is tested implicitly via toJSON + replacer.

// ============================================================================
// JSON.parse — primitives
// ============================================================================

#[test]
fn parse_null() {
    assert!(matches!(eval("JSON.parse('null')"), Ok(JsValue::Null)));
}

#[test]
fn parse_true() {
    assert!(eval_bool("JSON.parse('true')"));
}

#[test]
fn parse_false() {
    assert!(!eval_bool("JSON.parse('false')"));
}

#[test]
fn parse_number() {
    assert_eq!(eval_number("JSON.parse('42')"), 42.0);
    assert_eq!(eval_number("JSON.parse('3.125')"), 3.125);
    assert_eq!(eval_number("JSON.parse('-1')"), -1.0);
    assert_eq!(eval_number("JSON.parse('1e2')"), 100.0);
}

#[test]
fn parse_string() {
    assert_eq!(eval_string(r#"JSON.parse('"hello"')"#), "hello");
}

#[test]
fn parse_string_escapes() {
    assert_eq!(eval_string(r#"JSON.parse('"a\\nb"')"#), "a\nb");
    assert_eq!(eval_string(r#"JSON.parse('"a\\tb"')"#), "a\tb");
    assert_eq!(eval_string(r#"JSON.parse('"a\\"b"')"#), "a\"b");
}

#[test]
fn parse_string_unicode_escape() {
    assert_eq!(eval_string(r#"JSON.parse('"\\u0041"')"#), "A");
}

#[test]
fn parse_string_surrogate_pair() {
    // Surrogate pair \uD83D\uDE00 → 😀
    assert_eq!(
        eval_string(r#"JSON.parse('"\\uD83D\\uDE00"')"#),
        "\u{1F600}"
    );
}

#[test]
fn parse_lone_surrogate_round_trip() {
    // Lone surrogate preserved through parse → stringify round-trip.
    assert_eq!(
        eval_string(r#"JSON.stringify(JSON.parse('"\\uD800"'))"#),
        r#""\ud800""#
    );
}

// ============================================================================
// JSON.parse — objects and arrays
// ============================================================================

#[test]
fn parse_empty_object() {
    assert!(matches!(eval("JSON.parse('{}')"), Ok(JsValue::Object(_))));
}

#[test]
fn parse_empty_array() {
    assert!(matches!(eval("JSON.parse('[]')"), Ok(JsValue::Object(_))));
}

#[test]
fn parse_object_property_access() {
    assert_eq!(eval_number(r#"JSON.parse('{"a": 1}').a"#), 1.0);
    assert_eq!(eval_string(r#"JSON.parse('{"a": "hello"}').a"#), "hello");
}

#[test]
fn parse_array_element_access() {
    assert_eq!(eval_number("JSON.parse('[1, 2, 3]')[1]"), 2.0);
}

#[test]
fn parse_nested() {
    assert_eq!(eval_number(r#"JSON.parse('{"a": {"b": 42}}').a.b"#), 42.0);
    assert_eq!(eval_number("JSON.parse('[[1, 2], [3]]')[0][1]"), 2.0);
}

#[test]
fn parse_duplicate_keys_last_wins() {
    // Duplicate keys: last value wins per spec + web compat.
    assert_eq!(eval_number(r#"JSON.parse('{"a": 1, "a": 2}').a"#), 2.0);
}

// ============================================================================
// JSON.parse — error cases
// ============================================================================

#[test]
fn parse_syntax_error_empty() {
    eval_throws("JSON.parse('')");
}

#[test]
fn parse_syntax_error_trailing() {
    eval_throws("JSON.parse('42 foo')");
}

#[test]
fn parse_syntax_error_invalid() {
    eval_throws("JSON.parse('undefined')");
    eval_throws("JSON.parse('{a: 1}')"); // unquoted key
}

// ============================================================================
// JSON.parse — reviver
// ============================================================================

#[test]
fn parse_reviver_transform() {
    assert_eq!(
        eval_number(
            r#"JSON.parse('{"a": 1, "b": 2}', function(key, value) {
                if (typeof value === "number") return value * 10;
                return value;
            }).a"#
        ),
        10.0
    );
}

#[test]
fn parse_reviver_delete() {
    // Returning undefined from reviver deletes the property.
    assert_eq!(
        eval_string(
            r#"var obj = JSON.parse('{"a": 1, "b": 2}', function(key, value) {
                if (key === "b") return undefined;
                return value;
            });
            JSON.stringify(obj)"#
        ),
        r#"{"a":1}"#
    );
}

#[test]
fn parse_reviver_array() {
    assert_eq!(
        eval_number(
            r#"JSON.parse('[1, 2, 3]', function(key, value) {
                if (typeof value === "number") return value + 1;
                return value;
            })[0]"#
        ),
        2.0
    );
}

// ============================================================================
// JSON round-trip
// ============================================================================

#[test]
fn json_roundtrip() {
    assert_eq!(
        eval_string(r#"JSON.stringify(JSON.parse('{"a": [1, true, null, "x"]}'))"#),
        r#"{"a":[1,true,null,"x"]}"#
    );
}
