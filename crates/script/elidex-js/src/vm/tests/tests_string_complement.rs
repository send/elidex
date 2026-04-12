//! Tests for String.prototype P2 additions.

use super::{eval_bool, eval_number, eval_string, eval_throws};

// -- Generic this coercion (P2.5) ---------------------------------------------

#[test]
fn string_method_on_number_this() {
    // String.prototype.indexOf.call(42, '2') → ToString(42)="42", indexOf('2')=1
    assert_eq!(eval_number("String.prototype.indexOf.call(42, '2');"), 1.0);
}

#[test]
fn string_method_on_null_this_throws() {
    eval_throws("String.prototype.indexOf.call(null, 'x');");
}

#[test]
fn string_method_on_undefined_this_throws() {
    eval_throws("String.prototype.trim.call(undefined);");
}

#[test]
fn string_method_on_boolean_this() {
    // String.prototype.slice.call(true, 0, 2) → ToString(true)="true", slice(0,2)="tr"
    assert_eq!(
        eval_string("String.prototype.slice.call(true, 0, 2);"),
        "tr"
    );
}

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

// -- StringWrapper (new String()) -----------------------------------------------

#[test]
fn new_string_typeof() {
    assert_eq!(eval_string("typeof new String('hello');"), "object");
}

#[test]
fn new_string_valueof() {
    assert_eq!(eval_string("new String('hello').valueOf();"), "hello");
}

#[test]
fn new_string_tostring() {
    assert_eq!(eval_string("new String('abc').toString();"), "abc");
}

#[test]
fn new_string_length() {
    assert_eq!(eval_number("new String('hello').length;"), 5.0);
}

#[test]
fn new_string_index_access() {
    assert_eq!(eval_string("new String('abc')[1];"), "b");
}

#[test]
fn new_string_method() {
    assert_eq!(eval_string("new String('Hello').toUpperCase();"), "HELLO");
}

#[test]
fn string_call_returns_primitive() {
    assert_eq!(eval_string("typeof String(42);"), "string");
    assert_eq!(eval_string("String(42);"), "42");
}

#[test]
fn string_call_no_args() {
    assert_eq!(eval_string("String();"), "");
}

#[test]
fn new_string_constructor_property() {
    assert_eq!(
        eval_string("typeof new String('x').constructor;"),
        "function"
    );
}

#[test]
fn string_proto_to_string_and_value_of_have_distinct_names() {
    // §21.1.3.25: toString and valueOf share implementation, but `.name`
    // must reflect the registered method name (§19.2.4.2).
    assert_eq!(eval_string("String.prototype.toString.name;"), "toString");
    assert_eq!(eval_string("String.prototype.valueOf.name;"), "valueOf");
}

#[test]
fn native_function_name_property() {
    // §19.2.4.2: built-in functions have a `.name` data property.
    assert_eq!(eval_string("Object.keys.name;"), "keys");
    assert_eq!(eval_string("Array.isArray.name;"), "isArray");
    assert_eq!(eval_string("String.fromCharCode.name;"), "fromCharCode");
}

#[test]
fn native_function_name_descriptor() {
    // §19.2.4.2: {writable: false, enumerable: false, configurable: true}.
    assert_eq!(
        eval_string(
            "var d = Object.getOwnPropertyDescriptor(Object.keys, 'name');
             d.writable + ',' + d.enumerable + ',' + d.configurable"
        ),
        "false,false,true"
    );
}

// -- String constructor NewTarget detection ---------------------------------

#[test]
fn string_call_with_object_this_returns_primitive() {
    // §21.1.1.1: `String.call(obj, "x")` → `this = obj`, but NewTarget is
    // undefined, so return primitive "x".  Previously regressed: was returning
    // a wrapper.
    assert_eq!(eval_string("typeof String.call({}, 'x');"), "string");
    assert_eq!(eval_string("String.call({}, 'x');"), "x");
}

#[test]
fn string_call_with_symbol_returns_descriptive_string() {
    // §21.1.1.1 step 2: `String(Symbol("foo"))` → "Symbol(foo)" (no throw).
    assert_eq!(eval_string("String(Symbol('foo'));"), "Symbol(foo)");
    assert_eq!(eval_string("String(Symbol());"), "Symbol()");
}

#[test]
fn new_string_with_symbol_throws() {
    // `new String(Symbol())` → ToString throws on Symbol.
    eval_throws("new String(Symbol('foo'));");
}

#[test]
fn new_string_coerces_non_string_inputs() {
    assert_eq!(eval_string("new String(42).valueOf();"), "42");
    assert_eq!(eval_string("new String(undefined).valueOf();"), "undefined");
    assert_eq!(eval_string("new String(null).valueOf();"), "null");
    assert_eq!(eval_string("new String(true).valueOf();"), "true");
}

#[test]
fn new_string_no_args_is_empty() {
    assert_eq!(eval_string("new String().valueOf();"), "");
    assert_eq!(eval_number("new String().length;"), 0.0);
}

#[test]
fn new_string_length_is_non_writable() {
    // §21.1.5.1: `length` is non-writable.  Strict mode throws on assignment.
    eval_throws(
        "(function() {
           'use strict';
           var w = new String('abc');
           w.length = 10;
         })();",
    );
    // Sloppy mode silently ignores (pre-existing top-level behavior):
    // value is unchanged.
    assert_eq!(
        eval_number("var w = new String('abc'); w.length = 10; w.length;"),
        3.0
    );
}

#[test]
fn new_string_length_descriptor() {
    // §21.1.5.1: {writable: false, enumerable: false, configurable: false}.
    assert_eq!(
        eval_string(
            "var d = Object.getOwnPropertyDescriptor(new String('ab'), 'length');
             d.writable + ',' + d.enumerable + ',' + d.configurable"
        ),
        "false,false,false"
    );
}

// -- BoundFunction [[Construct]] --------------------------------------------

#[test]
fn new_on_bound_function_returns_instance() {
    // §9.4.1.2: `new boundFn()` constructs via target, prepends bound_args.
    assert_eq!(
        eval_string(
            "function F(a, b) { this.sum = a + b; }
             var bf = F.bind(null, 10);
             var r = new bf(5);
             String(r.sum);"
        ),
        "15"
    );
}

#[test]
fn new_on_nested_bound_chain() {
    // Bound chain: all bound_args prepended in outer-to-inner order.
    assert_eq!(
        eval_string(
            "function F(a, b, c) { this.vals = [a,b,c].join(','); }
             var bf = F.bind(null, 1).bind(null, 2);
             new bf(3).vals;"
        ),
        "1,2,3"
    );
}

#[test]
fn bind_honors_defineproperty_name() {
    // §19.2.3.2 step 11: bind reads target's current `.name` property via Get,
    // not the internal slot, so defineProperty overrides propagate.
    assert_eq!(
        eval_string(
            "function foo() {}
             Object.defineProperty(foo, 'name', { value: 'bar', configurable: true });
             foo.bind(null).name;"
        ),
        "bound bar"
    );
}

#[test]
fn bind_honors_defineproperty_length() {
    // §19.2.3.2 steps 4-5: bind reads target's current `.length` property.
    assert_eq!(
        eval_number(
            "function f(a, b, c) {}
             Object.defineProperty(f, 'length', { value: 5, configurable: true });
             f.bind(null, 1).length;"
        ),
        4.0 // 5 - 1 bound arg = 4
    );
}

#[test]
fn new_error_undefined_skips_message_property() {
    // §19.5.1.1 step 4: `new Error(undefined)` must not install an own
    // "message" property (regression: previously set .message = "undefined"
    // by ToString-coercing undefined).
    // Use `in` + Object.getOwnPropertyNames to observe — typing constraints
    // and prototype chain differ from V8 but the own-property absence is
    // the observable behavior we care about.
    assert_eq!(
        eval_string("Object.getOwnPropertyNames(new Error(undefined)).join(',');"),
        "name"
    );
    // Explicit message is still installed.
    assert_eq!(eval_string("new Error('oops').message;"), "oops");
}

#[test]
fn bind_length_zero_for_non_number_property() {
    // §19.2.3.2 step 4-5: non-Number `.length` → 0.
    assert_eq!(
        eval_number(
            "function f(a, b, c) {}
             Object.defineProperty(f, 'length', { value: 'nope', configurable: true });
             f.bind(null).length;"
        ),
        0.0
    );
}

#[test]
fn number_to_string_radix_hex() {
    // §20.1.3.6: `(255).toString(16)` → "ff".  Regression: previously
    // radix was ignored entirely.
    assert_eq!(eval_string("(255).toString(16);"), "ff");
    assert_eq!(eval_string("(10).toString(2);"), "1010");
    assert_eq!(eval_string("(35).toString(36);"), "z");
}

#[test]
fn number_to_string_radix_out_of_range() {
    eval_throws("(10).toString(1);");
    eval_throws("(10).toString(37);");
}

#[test]
fn number_to_string_radix_fractional_truncates() {
    // §20.1.3.6 step 5-8: ToIntegerOrInfinity first, then range check.
    // `36.1` truncates to `36`, which is in [2, 36] → accepted.
    assert_eq!(eval_string("(35).toString(36.1);"), "z");
    assert_eq!(eval_string("(255).toString(16.9);"), "ff");
}

#[test]
fn number_to_string_radix_default_is_decimal() {
    assert_eq!(eval_string("(255).toString();"), "255");
    assert_eq!(eval_string("(255).toString(10);"), "255");
}

#[test]
fn string_split_honors_limit() {
    // §21.1.3.19 step 6: limit truncates the result.
    assert_eq!(eval_string("'a,b,c,d'.split(',', 2).join('|');"), "a|b");
    assert_eq!(eval_string("'a,b,c'.split(',', 0).join('|');"), "");
}

#[test]
fn string_split_limit_coerces_via_to_number() {
    // Numeric string limit: ToNumber → ToUint32.
    assert_eq!(eval_string("'a,b,c'.split(',', '2').join('|');"), "a|b");
}

#[test]
fn string_split_limit_non_finite_is_zero() {
    // §7.1.7 ToUint32: NaN / ±Infinity → 0.  `split(sep, Infinity)` must
    // not throw; it yields an empty array (limit = 0).
    assert_eq!(eval_number("'a,b,c'.split(',', Infinity).length;"), 0.0);
    assert_eq!(eval_number("'a,b,c'.split(',', NaN).length;"), 0.0);
}

#[test]
fn bind_length_propagates_infinity() {
    // §19.2.3.2 step 5 (ToIntegerOrInfinity): +Infinity length propagates.
    // max(0, Infinity - argCount) = Infinity.
    assert_eq!(
        eval_number(
            "function f() {}
             Object.defineProperty(f, 'length', { value: Infinity, configurable: true });
             f.bind(null, 1, 2, 3).length;"
        ),
        f64::INFINITY
    );
}

#[test]
fn array_to_string_honors_join_override() {
    // §22.1.3.30: Array.prototype.toString calls Get(O, "join"); if
    // callable, invokes it.  User-installed .join override must be honored.
    // Note: `'' + arr` would also exercise this via OrdinaryToPrimitive,
    // but that helper is tracked as a separate follow-up (phase4-plan.md).
    assert_eq!(
        eval_string(
            "var arr = [1, 2, 3];
             arr.join = function() { return 'custom'; };
             arr.toString();"
        ),
        "custom"
    );
}

#[test]
fn array_to_string_non_callable_join_falls_back() {
    // §22.1.3.30: If Get(O, "join") is not callable, fall back to
    // Object.prototype.toString → "[object Object]".
    assert_eq!(
        eval_string(
            "var arr = [1, 2, 3];
             arr.join = 'not a function';
             arr.toString();"
        ),
        "[object Object]"
    );
}

#[test]
fn to_primitive_honors_at_to_primitive_on_wrapper() {
    // §7.1.1 step 2.a: @@toPrimitive takes precedence even for primitive
    // wrapper objects.  Regression guard: the fast-path that unwraps
    // NumberWrapper/StringWrapper/BooleanWrapper must not shadow a
    // user-defined Symbol.toPrimitive.
    assert_eq!(
        eval_string(
            "var s = new String('wrapped');
             s[Symbol.toPrimitive] = function() { return 'custom'; };
             String(s + '');"
        ),
        "custom"
    );
}

#[test]
fn bind_name_coerces_non_string_via_to_string() {
    // §19.2.3.2 step 11-13: Get(target, "name") returns arbitrary JsValue;
    // non-Symbol values must be ToString-coerced before the "bound " prefix.
    assert_eq!(
        eval_string(
            "function foo() {}
             Object.defineProperty(foo, 'name', { value: 42, configurable: true });
             foo.bind(null).name;"
        ),
        "bound 42"
    );
}

#[test]
fn bind_name_symbol_falls_back_to_empty() {
    // §19.2.3.2 step 13: non-String `name` → targetName = "".
    // (Symbol would otherwise throw in ToString; spec says treat as empty.)
    assert_eq!(
        eval_string(
            "function foo() {}
             Object.defineProperty(foo, 'name', { value: Symbol('bar'), configurable: true });
             foo.bind(null).name;"
        ),
        "bound "
    );
}

#[test]
fn abstract_eq_propagates_to_primitive_throw() {
    // §7.2.15 steps 10/12: Object == primitive calls ? ToPrimitive; an
    // abrupt completion from @@toPrimitive must propagate, not silently
    // yield `false`.  Regression guard for the swallow fix in abstract_eq.
    eval_throws(
        "var o = {};
         o[Symbol.toPrimitive] = function() { throw new Error('prim-throw'); };
         o == 1;",
    );
}

#[test]
fn bind_propagates_name_getter_throw() {
    // §19.2.3.2 step 11: `Get(target, "name")` is an abrupt completion
    // point.  If the getter throws, bind must propagate the exception
    // rather than silently falling back to the internal name.
    eval_throws(
        "function foo() {}
         Object.defineProperty(foo, 'name', {
             get() { throw new Error('name-getter-throws'); },
             configurable: true
         });
         foo.bind(null);",
    );
}

#[test]
fn new_array_reuses_preallocated_instance() {
    // §22.1.1: `new Array(...)` should return an Array-kind object with the
    // given elements. Regression guard: constructor must reuse do_new's
    // pre-allocated instance rather than leak it.
    assert_eq!(eval_number("new Array(1, 2, 3).length;"), 3.0);
    assert_eq!(eval_string("new Array(1, 2, 3).join(',');"), "1,2,3");
    assert_eq!(eval_number("new Array(5).length;"), 5.0);
}

#[test]
fn json_parse_depth_cap() {
    // Deeply-nested JSON array must throw RangeError instead of causing
    // a Rust-stack overflow (process abort).
    let deep = "[".repeat(2000) + &"]".repeat(2000);
    let script = format!("JSON.parse('{deep}');");
    eval_throws(&script);
}

#[test]
fn json_stringify_depth_cap() {
    // Deeply-nested object passed to stringify must throw RangeError,
    // not abort the process via Rust-stack overflow.
    eval_throws(
        "var a = {};
         var top = a;
         for (var i = 0; i < 2000; i++) { top.x = {}; top = top.x; }
         JSON.stringify(a);",
    );
}

#[test]
fn for_in_prototype_chain_cap() {
    // Attacker-built deep prototype chain must not cause unbounded
    // iteration in `for (k in obj)`.
    eval_throws(
        "var p = {};
         for (var i = 0; i < 10_001; i++) { p = Object.create(p); }
         for (var k in p) {}",
    );
}

#[test]
fn function_tostring_chain_depth_cap() {
    // §19.2.3.5: Function.prototype.toString on a deeply-bound chain
    // must enforce the same depth cap as call/construct to prevent
    // unbounded "bound " string growth.
    eval_throws(
        "var f = function(){};
         for (var i = 0; i < 10001; i++) { f = f.bind(null); }
         f.toString();",
    );
}

#[test]
fn bind_chain_depth_cap() {
    // Attacker-built chain beyond MAX_BIND_CHAIN_DEPTH should throw RangeError
    // on call.  Building the chain itself must also be stack-safe
    // (target_function_length_name was previously recursive over the chain).
    eval_throws(
        "var f = function(){};
         for (var i = 0; i < 10001; i++) { f = f.bind(null); }
         f();",
    );
}

// -- Computed getter/setter keys --------------------------------------------

#[test]
fn class_computed_getter() {
    assert_eq!(
        eval_string(
            "const k = 'x';
             class C { get [k]() { return 42; } }
             String(new C().x);"
        ),
        "42"
    );
}

#[test]
fn class_computed_setter() {
    assert_eq!(
        eval_string(
            "const k = 'x';
             class C {
               constructor() { this._x = 0; }
               get [k]() { return this._x; }
               set [k](v) { this._x = v * 2; }
             }
             var c = new C();
             c.x = 5;
             String(c.x);"
        ),
        "10"
    );
}

#[test]
fn class_computed_getter_symbol_key() {
    assert_eq!(
        eval_string(
            "class C { get [Symbol.iterator]() { return 1; } }
             String(new C()[Symbol.iterator]);"
        ),
        "1"
    );
}

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

#[test]
fn encode_uri_lone_surrogate_throws() {
    // Lone high surrogate must throw URIError
    eval_throws("encodeURI('\\uD800');");
    // Lone low surrogate must throw URIError
    eval_throws("encodeURIComponent('\\uDC00');");
}

#[test]
fn encode_uri_surrogate_pair_ok() {
    // Valid surrogate pair (U+1F600) should encode as UTF-8 bytes
    assert_eq!(eval_string("encodeURI('\\uD83D\\uDE00');"), "%F0%9F%98%80");
}

#[test]
fn decode_uri_multibyte() {
    // %C3%A9 = UTF-8 for "é" (U+00E9)
    assert_eq!(eval_string("decodeURIComponent('%C3%A9');"), "\u{00E9}");
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
