//! Tests for Number built-in methods and constants (P2 additions).

use super::{eval_bool, eval_number, eval_string};

// -- Number.isFinite ----------------------------------------------------------

#[test]
fn number_is_finite_true() {
    assert!(eval_bool("Number.isFinite(42);"));
    assert!(eval_bool("Number.isFinite(0);"));
    assert!(eval_bool("Number.isFinite(-1.5);"));
}

#[test]
fn number_is_finite_false() {
    assert!(!eval_bool("Number.isFinite(Infinity);"));
    assert!(!eval_bool("Number.isFinite(-Infinity);"));
    assert!(!eval_bool("Number.isFinite(NaN);"));
}

#[test]
fn number_is_finite_non_number() {
    // Unlike global isFinite, Number.isFinite does NOT coerce
    assert!(!eval_bool("Number.isFinite('42');"));
    assert!(!eval_bool("Number.isFinite(null);"));
    assert!(!eval_bool("Number.isFinite(undefined);"));
}

// -- Number.isInteger ---------------------------------------------------------

#[test]
fn number_is_integer_true() {
    assert!(eval_bool("Number.isInteger(42);"));
    assert!(eval_bool("Number.isInteger(0);"));
    assert!(eval_bool("Number.isInteger(-100);"));
    assert!(eval_bool("Number.isInteger(5.0);"));
}

#[test]
fn number_is_integer_false() {
    assert!(!eval_bool("Number.isInteger(3.5);"));
    assert!(!eval_bool("Number.isInteger(NaN);"));
    assert!(!eval_bool("Number.isInteger(Infinity);"));
    assert!(!eval_bool("Number.isInteger('42');"));
}

// -- Number.isNaN -------------------------------------------------------------

#[test]
fn number_is_nan_true() {
    assert!(eval_bool("Number.isNaN(NaN);"));
}

#[test]
fn number_is_nan_false() {
    assert!(!eval_bool("Number.isNaN(42);"));
    assert!(!eval_bool("Number.isNaN(undefined);"));
    assert!(!eval_bool("Number.isNaN('NaN');"));
}

// -- Number.isSafeInteger -----------------------------------------------------

#[test]
fn number_is_safe_integer_true() {
    assert!(eval_bool("Number.isSafeInteger(42);"));
    assert!(eval_bool("Number.isSafeInteger(9007199254740991);"));
    assert!(eval_bool("Number.isSafeInteger(-9007199254740991);"));
}

#[test]
fn number_is_safe_integer_false() {
    assert!(!eval_bool("Number.isSafeInteger(9007199254740992);"));
    assert!(!eval_bool("Number.isSafeInteger(3.5);"));
    assert!(!eval_bool("Number.isSafeInteger(Infinity);"));
}

// -- Number constants ---------------------------------------------------------

#[test]
fn number_positive_infinity() {
    assert_eq!(eval_number("Number.POSITIVE_INFINITY;"), f64::INFINITY);
}

#[test]
fn number_negative_infinity() {
    assert_eq!(eval_number("Number.NEGATIVE_INFINITY;"), f64::NEG_INFINITY);
}

#[test]
fn number_max_safe_integer() {
    assert_eq!(
        eval_number("Number.MAX_SAFE_INTEGER;"),
        9_007_199_254_740_991.0
    );
}

#[test]
fn number_min_safe_integer() {
    assert_eq!(
        eval_number("Number.MIN_SAFE_INTEGER;"),
        -9_007_199_254_740_991.0
    );
}

#[test]
fn number_epsilon() {
    assert_eq!(eval_number("Number.EPSILON;"), f64::EPSILON);
}

#[test]
fn number_max_value() {
    assert_eq!(eval_number("Number.MAX_VALUE;"), f64::MAX);
}

#[test]
fn number_min_value() {
    assert_eq!(eval_number("Number.MIN_VALUE;"), f64::MIN_POSITIVE);
}

#[test]
fn number_nan_constant() {
    assert!(eval_bool("Number.isNaN(Number.NaN);"));
}

// -- Number.prototype.toFixed -------------------------------------------------

#[test]
fn number_to_fixed_negative_zero() {
    // §20.1.3.3 step 7: -0 formats without minus sign
    assert_eq!(eval_string("(-0).toFixed(2);"), "0.00");
}

// -- Number.prototype.toExponential -------------------------------------------

#[test]
fn number_to_exponential_basic() {
    let s = eval_string("(123456).toExponential(2);");
    assert_eq!(s, "1.23e+5");
}

#[test]
fn number_to_exponential_zero_digits() {
    let s = eval_string("(1.5).toExponential(0);");
    assert_eq!(s, "2e+0");
}

#[test]
fn number_to_exponential_nan() {
    assert_eq!(eval_string("(NaN).toExponential();"), "NaN");
}

// -- Number.prototype.toPrecision ---------------------------------------------

#[test]
fn number_to_precision_basic() {
    let s = eval_string("(123.456).toPrecision(5);");
    assert_eq!(s, "123.46");
}

#[test]
fn number_to_precision_one() {
    let s = eval_string("(5.5).toPrecision(1);");
    assert_eq!(s, "6");
}

#[test]
fn number_to_precision_nan() {
    assert_eq!(eval_string("(NaN).toPrecision(3);"), "NaN");
}

#[test]
fn number_to_precision_undefined_arg() {
    // Should behave like toString
    assert_eq!(eval_string("(42).toPrecision();"), "42");
}

#[test]
fn number_to_precision_large_magnitude() {
    // e >= p: uses exponential notation per spec
    assert_eq!(eval_string("(123456).toPrecision(4);"), "1.235e+5");
}

#[test]
fn number_to_precision_exponential() {
    // e >= p: must use exponential notation
    assert_eq!(eval_string("(123456789).toPrecision(3);"), "1.23e+8");
}

#[test]
fn number_to_precision_small_exponential() {
    // Very small number: e < -6
    assert_eq!(eval_string("(0.0000001).toPrecision(2);"), "1.0e-7");
}

#[test]
fn number_to_exponential_negative_exp() {
    assert_eq!(eval_string("(0.005).toExponential(1);"), "5.0e-3");
}

#[test]
fn number_to_exponential_omitted_digits_finite() {
    // ES spec: as many significant digits as needed
    assert_eq!(eval_string("(123456).toExponential();"), "1.23456e+5");
}

#[test]
fn number_to_exponential_nan_digits_coerces_to_zero() {
    assert_eq!(eval_string("(1.5).toExponential(NaN);"), "2e+0");
}

#[test]
fn number_to_exponential_negative_zero() {
    // -0 should format without minus sign in exponential
    assert_eq!(eval_string("(-0).toExponential(0);"), "0e+0");
}

#[test]
fn number_to_precision_range_error_before_non_finite() {
    // §20.1.3.5: RangeError for invalid precision even when this is Infinity
    use super::eval_throws;
    eval_throws("(Infinity).toPrecision(0);");
}

#[test]
fn number_to_exponential_range_error_before_non_finite() {
    // §20.1.3.2: RangeError for invalid digits even when this is NaN
    use super::eval_throws;
    eval_throws("(NaN).toExponential(101);");
}
