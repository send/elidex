//! Tests for Math built-in methods and constants (P2 additions).

use super::{eval_bool, eval_number};

// -- Math.trunc ---------------------------------------------------------------

#[test]
fn math_trunc_positive() {
    assert_eq!(eval_number("Math.trunc(3.7);"), 3.0);
}

#[test]
fn math_trunc_negative() {
    assert_eq!(eval_number("Math.trunc(-3.7);"), -3.0);
}

#[test]
fn math_trunc_nan() {
    assert!(eval_bool("isNaN(Math.trunc(NaN));"));
}

// -- Math.sign ----------------------------------------------------------------

#[test]
fn math_sign_positive() {
    assert_eq!(eval_number("Math.sign(42);"), 1.0);
}

#[test]
fn math_sign_negative() {
    assert_eq!(eval_number("Math.sign(-42);"), -1.0);
}

#[test]
fn math_sign_zero() {
    assert_eq!(eval_number("Math.sign(0);"), 0.0);
}

#[test]
fn math_sign_nan() {
    assert!(eval_bool("isNaN(Math.sign(NaN));"));
}

// -- Trigonometric functions ---------------------------------------------------

#[test]
fn math_sin_zero() {
    assert_eq!(eval_number("Math.sin(0);"), 0.0);
}

#[test]
fn math_cos_zero() {
    assert_eq!(eval_number("Math.cos(0);"), 1.0);
}

#[test]
fn math_tan_zero() {
    assert_eq!(eval_number("Math.tan(0);"), 0.0);
}

#[test]
fn math_sin_pi_half() {
    let n = eval_number("Math.sin(Math.PI / 2);");
    assert!((n - 1.0).abs() < 1e-10);
}

#[test]
fn math_cos_pi() {
    let n = eval_number("Math.cos(Math.PI);");
    assert!((n - (-1.0)).abs() < 1e-10);
}

// -- Inverse trigonometric functions ------------------------------------------

#[test]
fn math_asin_zero() {
    assert_eq!(eval_number("Math.asin(0);"), 0.0);
}

#[test]
fn math_acos_one() {
    assert_eq!(eval_number("Math.acos(1);"), 0.0);
}

#[test]
fn math_atan_zero() {
    assert_eq!(eval_number("Math.atan(0);"), 0.0);
}

#[test]
fn math_atan2_basic() {
    let n = eval_number("Math.atan2(1, 1);");
    assert!((n - std::f64::consts::FRAC_PI_4).abs() < 1e-10);
}

#[test]
fn math_atan2_zero_zero() {
    assert_eq!(eval_number("Math.atan2(0, 0);"), 0.0);
}

// -- Logarithmic / exponential functions --------------------------------------

#[test]
fn math_log2_basic() {
    assert_eq!(eval_number("Math.log2(8);"), 3.0);
}

#[test]
fn math_log10_basic() {
    assert_eq!(eval_number("Math.log10(1000);"), 3.0);
}

#[test]
fn math_exp_zero() {
    assert_eq!(eval_number("Math.exp(0);"), 1.0);
}

#[test]
fn math_exp_one() {
    let n = eval_number("Math.exp(1);");
    assert!((n - std::f64::consts::E).abs() < 1e-10);
}

// -- Math.cbrt ----------------------------------------------------------------

#[test]
fn math_cbrt_basic() {
    assert_eq!(eval_number("Math.cbrt(27);"), 3.0);
}

#[test]
fn math_cbrt_negative() {
    assert_eq!(eval_number("Math.cbrt(-8);"), -2.0);
}

// -- Math.hypot ---------------------------------------------------------------

#[test]
fn math_hypot_basic() {
    assert_eq!(eval_number("Math.hypot(3, 4);"), 5.0);
}

#[test]
fn math_hypot_no_args() {
    assert_eq!(eval_number("Math.hypot();"), 0.0);
}

#[test]
fn math_hypot_single() {
    assert_eq!(eval_number("Math.hypot(5);"), 5.0);
}

#[test]
fn math_hypot_infinity() {
    assert_eq!(eval_number("Math.hypot(Infinity, NaN);"), f64::INFINITY);
}

// -- Math.clz32 ---------------------------------------------------------------

#[test]
fn math_clz32_zero() {
    assert_eq!(eval_number("Math.clz32(0);"), 32.0);
}

#[test]
fn math_clz32_one() {
    assert_eq!(eval_number("Math.clz32(1);"), 31.0);
}

#[test]
fn math_clz32_max() {
    assert_eq!(eval_number("Math.clz32(0xFFFFFFFF);"), 0.0);
}

#[test]
fn math_clz32_nan() {
    assert_eq!(eval_number("Math.clz32(NaN);"), 32.0);
}

// -- Math.imul ----------------------------------------------------------------

#[test]
fn math_imul_basic() {
    assert_eq!(eval_number("Math.imul(2, 3);"), 6.0);
}

#[test]
fn math_imul_overflow() {
    assert_eq!(eval_number("Math.imul(0xFFFFFFFF, 5);"), -5.0);
}

#[test]
fn math_imul_large() {
    // 0x7FFFFFFF * 0x7FFFFFFF in i32 wrapping = 1
    assert_eq!(eval_number("Math.imul(0x7FFFFFFF, 0x7FFFFFFF);"), 1.0);
}

// -- Math.fround --------------------------------------------------------------

#[test]
fn math_fround_basic() {
    assert_eq!(eval_number("Math.fround(1.5);"), 1.5);
}

#[test]
fn math_fround_precision_loss() {
    // 1.337 as f32 is not exactly 1.337
    let n = eval_number("Math.fround(1.337);");
    assert!((n - 1.337_f32 as f64).abs() < 1e-15);
}

#[test]
fn math_fround_nan() {
    assert!(eval_bool("isNaN(Math.fround(NaN));"));
}

// -- New constants ------------------------------------------------------------

#[test]
fn math_constant_ln2() {
    let n = eval_number("Math.LN2;");
    assert!((n - std::f64::consts::LN_2).abs() < 1e-15);
}

#[test]
fn math_constant_ln10() {
    let n = eval_number("Math.LN10;");
    assert!((n - std::f64::consts::LN_10).abs() < 1e-15);
}

#[test]
fn math_constant_log2e() {
    let n = eval_number("Math.LOG2E;");
    assert!((n - std::f64::consts::LOG2_E).abs() < 1e-15);
}

#[test]
fn math_constant_log10e() {
    let n = eval_number("Math.LOG10E;");
    assert!((n - std::f64::consts::LOG10_E).abs() < 1e-15);
}

#[test]
fn math_constant_sqrt2() {
    let n = eval_number("Math.SQRT2;");
    assert!((n - std::f64::consts::SQRT_2).abs() < 1e-15);
}

#[test]
fn math_constant_sqrt1_2() {
    let n = eval_number("Math.SQRT1_2;");
    assert!((n - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-15);
}

// -- Edge cases (spec compliance) ---------------------------------------------

#[test]
fn math_max_negative_zero() {
    // §20.2.2.24: Math.max(-0, +0) should be +0
    assert!(eval_bool("1 / Math.max(-0, 0) === Infinity;"));
    assert!(eval_bool("1 / Math.max(0, -0) === Infinity;"));
}

#[test]
fn math_min_negative_zero() {
    // §20.2.2.25: Math.min(+0, -0) should be -0
    assert!(eval_bool("1 / Math.min(0, -0) === -Infinity;"));
    assert!(eval_bool("1 / Math.min(-0, 0) === -Infinity;"));
}

#[test]
fn math_clz32_large_value() {
    // 1e20 mod 2^32 = 1661992960, leading zeros = 1
    assert_eq!(eval_number("Math.clz32(1e20);"), 1.0);
}

#[test]
fn math_hypot_large_finite() {
    // Should not overflow to Infinity
    let n = eval_number("Math.hypot(1e200, 1e200);");
    assert!(n.is_finite());
    assert!((n - 1e200 * 2.0_f64.sqrt()).abs() / n < 1e-10);
}

#[test]
fn math_hypot_nan_and_infinity() {
    // Infinity takes precedence over NaN
    assert_eq!(eval_number("Math.hypot(Infinity, NaN);"), f64::INFINITY);
    assert!(eval_bool("isNaN(Math.hypot(NaN, 1));"));
}

#[test]
fn math_pow_one_infinity() {
    // §20.2.2.26: ES2020 diverges from IEEE 754 — pow(1, ±Infinity) = NaN
    assert!(eval_bool("isNaN(Math.pow(1, Infinity));"));
    assert!(eval_bool("isNaN(Math.pow(1, -Infinity));"));
    assert!(eval_bool("isNaN(Math.pow(-1, Infinity));"));
    assert!(eval_bool("isNaN(Math.pow(-1, -Infinity));"));
}
