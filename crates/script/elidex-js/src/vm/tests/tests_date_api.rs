//! ECMA-262 §21.4 Date builtin — end-to-end JS-level tests.
//!
//! Exercises the constructor forms, statics, getters/setters, string
//! conversions, `Symbol.toPrimitive` coercion, and `structuredClone` against
//! the **UTC-baseline** (`getTimezoneOffset()` is `+0`; local components equal
//! UTC — see `vm::natives_date::algorithms`).

#![cfg(feature = "engine")]

use super::super::Vm;
use super::helpers::{assert_eval_number, eval_bool, eval_number, eval_string, eval_throws};

#[test]
fn constructor_and_get_time() {
    assert_eq!(eval_number("new Date(0).getTime()"), 0.0);
    assert_eq!(
        eval_number("new Date(1577836800000).getTime()"),
        1_577_836_800_000.0
    );
    assert_eq!(eval_number("new Date(0).valueOf()"), 0.0);
    // A Date argument copies the source time value.
    assert_eq!(eval_number("new Date(new Date(123)).getTime()"), 123.0);
}

#[test]
fn utc_component_getters() {
    assert_eq!(eval_number("new Date(0).getUTCFullYear()"), 1970.0);
    assert_eq!(eval_number("new Date(0).getUTCMonth()"), 0.0);
    assert_eq!(eval_number("new Date(0).getUTCDate()"), 1.0);
    assert_eq!(eval_number("new Date(0).getUTCDay()"), 4.0); // 1970-01-01 = Thursday
    assert_eq!(eval_number("new Date(0).getUTCHours()"), 0.0);
    assert_eq!(eval_number("new Date(1000).getUTCSeconds()"), 1.0);
    assert_eq!(eval_number("new Date(500).getUTCMilliseconds()"), 500.0);
    // UTC-baseline: local getters coincide with UTC, offset is zero.
    assert_eq!(eval_number("new Date(0).getFullYear()"), 1970.0);
    assert_eq!(eval_number("new Date(0).getHours()"), 0.0);
    assert_eq!(eval_number("new Date(0).getTimezoneOffset()"), 0.0);
}

#[test]
fn component_constructor() {
    // Local == UTC under the baseline.
    assert_eq!(
        eval_number("new Date(2020, 0, 1).getTime()"),
        1_577_836_800_000.0
    );
    assert_eq!(eval_number("new Date(2020, 0, 1).getFullYear()"), 2020.0);
    assert_eq!(eval_number("new Date(2020, 5, 15).getMonth()"), 5.0);
    assert_eq!(eval_number("new Date(2020, 0, 1, 13, 30).getHours()"), 13.0);
    // Month overflow rolls into the next year.
    assert_eq!(eval_number("new Date(2020, 12, 1).getFullYear()"), 2021.0);
    // Two-digit year maps into the 1900s (§21.4.2.1).
    assert_eq!(eval_number("new Date(99, 0, 1).getFullYear()"), 1999.0);
    assert_eq!(eval_number("new Date(0, 0, 1).getFullYear()"), 1900.0);
}

#[test]
fn string_constructor_and_parse() {
    assert_eq!(
        eval_number("new Date('2020-01-01T00:00:00.000Z').getTime()"),
        1_577_836_800_000.0
    );
    assert_eq!(
        eval_number("Date.parse('2020-01-01T00:00:00Z')"),
        1_577_836_800_000.0
    );
    assert_eq!(eval_number("Date.parse('1970-01-01')"), 0.0);
    // Timezone offset is honoured.
    assert_eq!(
        eval_number("Date.parse('2020-01-01T09:00:00+09:00')"),
        1_577_836_800_000.0
    );
    assert!(eval_bool("isNaN(new Date('not a date').getTime())"));
    assert!(eval_bool("isNaN(Date.parse('2021-02-30'))")); // Feb 30 doesn't exist
                                                           // Round-trip through the engine's own toString.
    assert_eq!(
        eval_number("Date.parse(new Date(1577836800000).toString())"),
        1_577_836_800_000.0
    );
}

#[test]
fn date_utc_static() {
    assert_eq!(eval_number("Date.UTC(2020, 0, 1)"), 1_577_836_800_000.0);
    assert_eq!(eval_number("Date.UTC(1970, 0, 1)"), 0.0);
    assert_eq!(eval_number("Date.UTC(1970, 0)"), 0.0); // month-only → date defaults to 1
    assert!(eval_bool("isNaN(Date.UTC())"));
}

#[test]
fn date_now_static() {
    assert!(eval_bool("typeof Date.now() === 'number'"));
    assert!(eval_bool("Date.now() > 1577836800000")); // after 2020-01-01
    assert!(eval_bool("Number.isInteger(Date.now())"));
}

#[test]
fn string_conversions() {
    assert_eq!(
        eval_string("new Date(0).toISOString()"),
        "1970-01-01T00:00:00.000Z"
    );
    assert_eq!(
        eval_string("new Date(0).toUTCString()"),
        "Thu, 01 Jan 1970 00:00:00 GMT"
    );
    assert_eq!(
        eval_string("new Date(0).toJSON()"),
        "1970-01-01T00:00:00.000Z"
    );
    assert_eq!(eval_string("new Date(0).toDateString()"), "Thu Jan 01 1970");
    assert_eq!(
        eval_string("new Date(0).toTimeString()"),
        "00:00:00 GMT+0000 (Coordinated Universal Time)"
    );
    assert_eq!(
        eval_string("new Date(0).toString()"),
        "Thu Jan 01 1970 00:00:00 GMT+0000 (Coordinated Universal Time)"
    );
    // toJSON drives JSON.stringify (§25.5.4.2 SerializeJSONProperty).
    assert_eq!(
        eval_string("JSON.stringify(new Date(0))"),
        "\"1970-01-01T00:00:00.000Z\""
    );
}

#[test]
fn setters_mutate_in_place() {
    let mut vm = Vm::new();
    vm.eval("globalThis.d = new Date(0)").unwrap();
    vm.eval("d.setUTCFullYear(2000)").unwrap();
    assert_eval_number(&mut vm, "d.getUTCFullYear()", 2000.0);
    vm.eval("d.setUTCMonth(5)").unwrap();
    assert_eval_number(&mut vm, "d.getUTCMonth()", 5.0);
    vm.eval("d.setUTCDate(15)").unwrap();
    assert_eval_number(&mut vm, "d.getUTCDate()", 15.0);
    vm.eval("d.setUTCHours(12)").unwrap();
    assert_eval_number(&mut vm, "d.getUTCHours()", 12.0);
    vm.eval("d.setTime(1000)").unwrap();
    assert_eval_number(&mut vm, "d.getTime()", 1000.0);
    // setFullYear is the one setter that revives an Invalid Date (§21.4.4.21).
    vm.eval("globalThis.e = new Date(NaN); e.setFullYear(2021)")
        .unwrap();
    assert_eval_number(&mut vm, "e.getUTCFullYear()", 2021.0);
}

#[test]
fn invalid_date() {
    assert!(eval_bool("isNaN(new Date(NaN).getTime())"));
    assert_eq!(eval_string("new Date(NaN).toString()"), "Invalid Date");
    assert_eq!(eval_string("new Date(NaN).toDateString()"), "Invalid Date");
    // toJSON returns null for a non-finite time value (§21.4.4.37).
    assert!(eval_bool("new Date(NaN).toJSON() === null"));
    // TimeClip boundary (§21.4.1.31): ±8.64e15 valid, beyond → NaN.
    assert_eq!(eval_number("new Date(8.64e15).getTime()"), 8.64e15);
    assert!(eval_bool("isNaN(new Date(8.64e15 + 1).getTime())"));
    // toISOString throws RangeError on an invalid time value (§21.4.4.36).
    eval_throws("new Date(NaN).toISOString()");
}

#[test]
fn primitive_coercion() {
    // Symbol.toPrimitive number hint → the time value.
    assert_eq!(eval_number("Number(new Date(5))"), 5.0);
    assert_eq!(eval_number("+new Date(42)"), 42.0);
    // Subtraction coerces via the number hint.
    assert_eq!(eval_number("new Date(5000) - new Date(2000)"), 3000.0);
    // default / string hint → toString.
    assert!(eval_bool(
        "('' + new Date(0)).startsWith('Thu Jan 01 1970')"
    ));
    assert!(eval_bool("(`${new Date(0)}`).includes('GMT+0000')"));
    // Explicit hints via the well-known symbol.
    assert_eq!(
        eval_number("new Date(7)[Symbol.toPrimitive]('number')"),
        7.0
    );
    assert!(eval_bool(
        "new Date(0)[Symbol.toPrimitive]('string') === new Date(0).toString()"
    ));
}

#[test]
fn brand_and_identity() {
    assert!(eval_bool("typeof Date === 'function'"));
    assert!(eval_bool("new Date() instanceof Date"));
    assert!(eval_bool("Date.prototype.constructor === Date"));
    assert!(eval_bool("typeof new Date().getTime === 'function'"));
    // getUTCFullYear and getFullYear are the same fn under the UTC-baseline,
    // but both are installed and callable.
    assert!(eval_bool(
        "typeof Date.prototype.getUTCFullYear === 'function'"
    ));
    // Wrong-receiver brand check throws TypeError.
    assert!(eval_bool(
        "(() => { try { Date.prototype.getTime.call({}); return false; } \
         catch (e) { return e instanceof TypeError; } })()"
    ));
}

#[test]
fn called_as_function_returns_string() {
    // `Date()` (no `new`) returns the current time as a String, ignoring args.
    assert!(eval_bool("typeof Date() === 'string'"));
    assert!(eval_bool("Date().includes('GMT')"));
    assert!(eval_bool("typeof Date(9999) === 'string'"));
}

#[test]
fn structured_clone_preserves_date() {
    // Date is [Serializable]; the clone carries the same [[DateValue]] and is
    // a distinct object with Date.prototype.
    assert_eq!(
        eval_number("structuredClone(new Date(123456)).getTime()"),
        123_456.0
    );
    assert!(eval_bool("structuredClone(new Date(0)) instanceof Date"));
    assert!(eval_bool(
        "(() => { const d = new Date(1); const c = structuredClone(d); \
         return c !== d && c.getTime() === d.getTime(); })()"
    ));
}

#[test]
fn parse_robustness() {
    // code-review CRIT: a non-ASCII token must not panic the VM — the legacy
    // parser's `tok[..3]` slice used to crash on a non-char-boundary offset.
    assert!(eval_bool("isNaN(Date.parse('aa€'))"));
    assert!(eval_bool("isNaN(new Date('日本語').getTime())"));
    // A bare RFC-2822 numeric offset ("-0800"), unmodeled by this bounded
    // legacy parser, rejects to NaN rather than silently parsing year -800.
    assert!(eval_bool(
        "isNaN(Date.parse('Wed, 01 Jan 2020 00:00:00 -0800'))"
    ));
    // The engine's own toUTCString still round-trips.
    assert_eq!(
        eval_number("Date.parse(new Date(1577836800000).toUTCString())"),
        1_577_836_800_000.0
    );
}

#[test]
fn negative_zero_time_value() {
    // §21.4.1.31 TimeClip normalizes -0 → +0 (observable via Object.is).
    assert!(eval_bool("Object.is(new Date(-0.4).getTime(), 0)"));
    assert!(eval_bool("!Object.is(new Date(-0.4).getTime(), -0)"));
}

#[test]
fn coercion_honors_user_overrides() {
    // Codex R1 #1 (§21.4.4.45): Symbol.toPrimitive delegates to
    // OrdinaryToPrimitive, so a user valueOf/toString override is observed.
    assert_eq!(
        eval_number("(() => { const d = new Date(0); d.valueOf = () => 5; return +d; })()"),
        5.0
    );
    assert!(eval_bool(
        "(() => { const d = new Date(0); d.toString = () => 'x'; return ('' + d) === 'x'; })()"
    ));
    // Codex R1 #2 (§21.4.4.37): toJSON invokes toISOString (overridable) and is
    // generic (works on any object with toISOString).
    assert!(eval_bool(
        "(() => { const d = new Date(0); d.toISOString = () => 'y'; \
         return JSON.stringify(d) === '\"y\"'; })()"
    ));
    assert!(eval_bool(
        "Date.prototype.toJSON.call({ toISOString() { return 'z'; } }) === 'z'"
    ));
}

#[test]
fn locale_methods_exist() {
    // Codex R1 #3 (§21.4.4.38-40): toLocale* exist with impl-defined
    // (locale-independent) string fallbacks, not `undefined`.
    assert!(eval_bool(
        "typeof new Date(0).toLocaleString() === 'string'"
    ));
    assert!(eval_bool(
        "typeof new Date(0).toLocaleDateString() === 'string'"
    ));
    assert!(eval_bool(
        "typeof new Date(0).toLocaleTimeString() === 'string'"
    ));
}

// Codex R1 #4 (§20.1.3.6): a Date arm was added to
// `native_object_prototype_to_string`, so the builtin-tag path yields "Date".
// A JS-observable regression test is deferred: invoking
// `Object.prototype.toString` generically on ANY receiver (assigned property,
// `.call`, or `.apply`) currently throws a pre-existing, Date-unrelated
// "Cannot convert undefined or null to object" in the native-fn generic-call
// path (`#11-vm-native-fn-generic-invocation`) — only the interpreter's
// inherited-method fast path reaches the builtin. Once that lands, assert
// `Object.prototype.toString.call(new Date()) === "[object Date]"`.

#[test]
fn tojson_boxes_primitive_receiver() {
    // Codex R2 #8 (§21.4.4.37): toJSON applies ToObject(this), so a primitive
    // receiver is boxed and its (overridden) toISOString is invoked.
    assert_eq!(
        eval_string(
            "(() => { Number.prototype.toISOString = () => 'ok'; \
             return Date.prototype.toJSON.call(5); })()"
        ),
        "ok"
    );
}

#[test]
fn invalid_date_setter_preserves_side_effects() {
    // Codex R2 #9 (§21.4.4.23): a non-reviving setter on an Invalid Date coerces
    // its argument (running side effects) then returns NaN WITHOUT rewriting
    // [[DateValue]] — so a valueOf that revived the date via setTime persists.
    assert!(eval_bool(
        "(() => { const d = new Date(NaN); \
         d.setMilliseconds({ valueOf() { d.setTime(0); return 1; } }); \
         return d.getTime() === 0; })()"
    ));
    // The setter still returns NaN for the Invalid Date.
    assert!(eval_bool("isNaN(new Date(NaN).setSeconds(5))"));
}
