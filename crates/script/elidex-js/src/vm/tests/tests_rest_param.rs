//! Tests for function rest-parameter packing (`function f(...args)`)
//! at frame-push time.  Stage 0 prereq for D-17b CE-minimal class
//! default-derived-ctor synthesis (`(...args) => super(...args)`).

#[test]
fn rest_param_collects_all_args() {
    assert_eq!(
        super::eval_number("function f(...x) { return x.length; } f(1, 2, 3);"),
        3.0
    );
}

#[test]
fn rest_param_is_real_array() {
    assert!(super::eval_bool(
        "function f(...x) { return Array.isArray(x); } f(1);"
    ));
}

#[test]
fn rest_param_no_args_is_empty_array() {
    assert_eq!(
        super::eval_number("function f(...x) { return x.length; } f();"),
        0.0
    );
}

#[test]
fn rest_param_after_fixed_params() {
    assert_eq!(
        super::eval_number(
            "function f(a, b, ...rest) { return rest[0] + rest[1]; } f(1, 2, 10, 20);"
        ),
        30.0
    );
}

#[test]
fn rest_param_after_fixed_params_underflow() {
    // Caller passes only the fixed params — rest collects zero args.
    assert_eq!(
        super::eval_number("function f(a, b, ...rest) { return rest.length; } f(1, 2);"),
        0.0
    );
}

#[test]
fn rest_param_arrow_function() {
    assert_eq!(
        super::eval_number("((...x) => x.length)(1, 2, 3, 4, 5);"),
        5.0
    );
}

#[test]
fn rest_param_arrow_after_fixed() {
    assert_eq!(
        super::eval_number("((a, ...rest) => rest.length)(10, 20, 30);"),
        2.0
    );
}

#[test]
fn rest_param_preserves_argument_values() {
    assert_eq!(
        super::eval_number("function f(...x) { return x[0] * 10 + x[1]; } f(1, 2);"),
        12.0
    );
}
