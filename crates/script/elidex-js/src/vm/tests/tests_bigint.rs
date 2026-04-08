//! BigInt tests (ES2020 §6.1.6.2).

use super::{eval_bool, eval_string, eval_throws};

// ─── Literals ───────────────────────────────────────────────────────────

#[test]
fn bigint_literal_typeof() {
    assert_eq!(eval_string("typeof 42n"), "bigint");
}

#[test]
fn bigint_hex_literal() {
    assert_eq!(eval_string("(0xFFn).toString()"), "255");
}

#[test]
fn bigint_binary_literal() {
    assert_eq!(eval_string("(0b101n).toString()"), "5");
}

#[test]
fn bigint_octal_literal() {
    assert_eq!(eval_string("(0o77n).toString()"), "63");
}

// ─── Arithmetic ─────────────────────────────────────────────────────────

#[test]
fn bigint_add() {
    assert_eq!(eval_string("(1n + 2n).toString()"), "3");
}

#[test]
fn bigint_sub() {
    assert_eq!(eval_string("(10n - 7n).toString()"), "3");
}

#[test]
fn bigint_mul() {
    assert_eq!(eval_string("(6n * 7n).toString()"), "42");
}

#[test]
fn bigint_div_truncates() {
    assert_eq!(eval_string("(7n / 2n).toString()"), "3");
}

#[test]
fn bigint_mod() {
    assert_eq!(eval_string("(10n % 3n).toString()"), "1");
}

#[test]
fn bigint_exp() {
    assert_eq!(eval_string("(2n ** 10n).toString()"), "1024");
}

#[test]
fn bigint_div_by_zero_throws() {
    eval_throws("1n / 0n");
}

#[test]
fn bigint_exp_negative_throws() {
    eval_throws("2n ** -1n");
}

#[test]
fn bigint_mixed_add_throws() {
    eval_throws("1n + 1");
}

#[test]
fn bigint_mixed_sub_throws() {
    eval_throws("1n - 1");
}

// ─── Comparison ─────────────────────────────────────────────────────────

#[test]
fn bigint_strict_eq() {
    assert!(eval_bool("1n === 1n"));
}

#[test]
fn bigint_strict_neq_number() {
    assert!(eval_bool("1n !== 1"));
}

#[test]
fn bigint_abstract_eq_number() {
    assert!(eval_bool("1n == 1"));
}

#[test]
fn bigint_abstract_eq_string() {
    assert!(eval_bool("1n == '1'"));
}

#[test]
fn bigint_lt() {
    assert!(eval_bool("1n < 2n"));
}

#[test]
fn bigint_gt_number() {
    assert!(eval_bool("2n > 1"));
}

// ─── Bitwise ────────────────────────────────────────────────────────────

#[test]
fn bigint_bitand() {
    assert_eq!(eval_string("(0xFn & 0x3n).toString()"), "3");
}

#[test]
fn bigint_bitor() {
    assert_eq!(eval_string("(0x1n | 0x2n).toString()"), "3");
}

#[test]
fn bigint_bitxor() {
    assert_eq!(eval_string("(0x3n ^ 0x1n).toString()"), "2");
}

#[test]
fn bigint_shl() {
    assert_eq!(eval_string("(1n << 10n).toString()"), "1024");
}

#[test]
fn bigint_shr() {
    assert_eq!(eval_string("(1024n >> 5n).toString()"), "32");
}

#[test]
fn bigint_bitnot() {
    assert_eq!(eval_string("(~0n).toString()"), "-1");
}

#[test]
fn bigint_ushr_throws() {
    eval_throws("1n >>> 0n");
}

#[test]
fn bigint_mixed_bitwise_throws() {
    eval_throws("1n & 1");
}

// ─── Unary ──────────────────────────────────────────────────────────────

#[test]
fn bigint_negate() {
    assert_eq!(eval_string("(-42n).toString()"), "-42");
}

#[test]
fn bigint_unary_plus_throws() {
    eval_throws("+1n");
}

// ─── Boolean coercion ───────────────────────────────────────────────────

#[test]
fn bigint_zero_is_falsy() {
    assert!(eval_bool("!0n"));
}

#[test]
fn bigint_nonzero_is_truthy() {
    assert!(eval_bool("!!1n"));
}

// ─── BigInt() function ──────────────────────────────────────────────────

#[test]
fn bigint_from_number() {
    assert_eq!(eval_string("BigInt(42).toString()"), "42");
}

#[test]
fn bigint_from_string() {
    assert_eq!(eval_string("BigInt('123').toString()"), "123");
}

#[test]
fn bigint_from_boolean() {
    assert_eq!(eval_string("BigInt(true).toString()"), "1");
}

#[test]
fn bigint_from_float_throws() {
    eval_throws("BigInt(1.5)");
}

#[test]
fn bigint_from_nan_throws() {
    eval_throws("BigInt(NaN)");
}

// ─── toString with radix ────────────────────────────────────────────────

#[test]
fn bigint_to_string_hex() {
    assert_eq!(eval_string("(255n).toString(16)"), "ff");
}

#[test]
fn bigint_to_string_binary() {
    assert_eq!(eval_string("(10n).toString(2)"), "1010");
}

// ─── Large values ───────────────────────────────────────────────────────

#[test]
fn bigint_large_value() {
    assert_eq!(
        eval_string("(2n ** 64n).toString()"),
        "18446744073709551616"
    );
}

#[test]
fn bigint_string_concat() {
    assert_eq!(eval_string("'' + 42n"), "42");
}

// ─── Copilot review: additional coverage ────────────────────────────────

#[test]
fn bigint_from_hex_string() {
    assert_eq!(eval_string("BigInt('0xFF').toString()"), "255");
}

#[test]
fn bigint_from_binary_string() {
    assert_eq!(eval_string("BigInt('0b1010').toString()"), "10");
}

#[test]
fn bigint_from_octal_string() {
    assert_eq!(eval_string("BigInt('0o77').toString()"), "63");
}

#[test]
fn bigint_to_string_radix_coercion() {
    // Radix is coerced via ToNumber: string '16' → 16.
    assert_eq!(eval_string("(255n).toString('16')"), "ff");
}

#[test]
fn bigint_relational_precision() {
    // 2^53 + 1 cannot be represented exactly as f64 (rounds to 2^53).
    // The comparison must still be correct.
    assert!(eval_bool("9007199254740993n > 9007199254740992"));
}

#[test]
fn bigint_relational_negative_large() {
    assert!(eval_bool("-9007199254740993n < -9007199254740992"));
}

#[test]
fn bigint_as_int_n_coerces_string_arg() {
    // Second arg coerced via ToBigInt: '255' → 255n
    assert_eq!(eval_string("BigInt.asIntN(8, '127').toString()"), "127");
}

#[test]
fn bigint_as_uint_n_coerces_string_arg() {
    assert_eq!(eval_string("BigInt.asUintN(8, '256').toString()"), "0");
}

#[test]
fn bigint_as_int_n_fractional_bits_truncates() {
    // ToIndex truncates: 8.5 → 8 (per §7.1.22 ToIntegerOrInfinity).
    assert_eq!(eval_string("BigInt.asIntN(8.5, 127n).toString()"), "127");
}
