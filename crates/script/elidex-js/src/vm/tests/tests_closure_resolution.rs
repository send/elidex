//! `#11-vm-closure-resolution-lexical-position` — a closure resolves a
//! free identifier against the binding visible at the closure's lexical
//! position in each enclosing function, not against whichever same-named
//! binding in that function has the highest scope index.
//!
//! Spec, by AO name: ResolveBinding; GetIdentifierReference (the walk
//! follows `[[OuterEnv]]`, i.e. the lexical parent chain).
//!
//! Keying the upvalue chain on the binding's own scope is the right key.
//! The mutant that keys the chain on the closure's position instead of the
//! binding's scope is equivalent: each closure function has its own
//! `upvalue_map` and sits at one fixed position, so the difference is
//! unobservable today.
//!
//! Known deviation, pinned by `known_deviation_block_body_binding_per_entry`:
//! a block-body `let` inside a loop is one binding per function entry, not
//! per iteration, so every closure sees the last value. Owner:
//! `#11-vm-block-declaration-instantiation` (Slice B: a fresh binding per
//! Block evaluation), with `#11-vm-per-iteration-loop-environment` (Slice E)
//! planned to supply the shared mechanism.
//!
//! The cells are observable JS only; they presume nothing about how
//! captured bindings are represented.

use super::{eval_number, eval_string};

/// The sibling-block shadow case: a later sibling block's `let x` must not shadow the outer one.
#[test]
fn sibling_shadow_closure_sees_outer() {
    assert_eq!(
        eval_string(r#"let x = "outer"; { let x = "inner"; } const f = () => x; f();"#),
        "outer"
    );
}

/// The sibling-blocks case: a closure created in one block is not affected by a later
/// sibling block's same-named binding.
#[test]
fn sibling_blocks_closure_sees_own_block() {
    assert_eq!(
        eval_number("const fs = []; { let x = 1; fs.push(() => x); } { let x = 2; } fs[0]();"),
        1.0
    );
}

/// The same shapes inside a function, so the capture is a real upvalue
/// of a non-top-level enclosing function.
#[test]
fn sibling_shadow_inside_function() {
    assert_eq!(
        eval_string(
            r#"function g() { let x = "outer"; { let x = "inner"; } return () => x; } g()();"#
        ),
        "outer"
    );
}

/// Two functions deep: the middle function has a same-named binding in a
/// sibling block; the innermost closure must see the lexically enclosing
/// one.
#[test]
fn nested_fn_sees_lexically_enclosing_binding() {
    assert_eq!(
        eval_number(
            "function outer() {
               let x = 1;
               function middle() {
                 let x = 2;
                 { let x = 3; }
                 return () => x;
               }
               return middle()();
             }
             outer();"
        ),
        2.0
    );
    // Control (no same-name shadowing, so green before F2 too): a depth-2
    // capture through a middle function still resolves.
    assert_eq!(
        eval_number(
            "function outer() {
               { let x = 10; }
               let y = 1;
               function middle() {
                 { let x = 20; }
                 return () => () => y;
               }
               return middle()()();
             }
             outer();"
        ),
        1.0
    );
    // Capture crosses two function boundaries to a binding whose block is
    // followed by a same-named sibling block.
    assert_eq!(
        eval_number(
            "function outer() {
               let fn1;
               { let x = 7; fn1 = () => () => x; }
               { let x = 8; }
               return fn1()();
             }
             outer();"
        ),
        7.0
    );
}

/// One enclosing function, two blocks each declaring `let x` with a
/// closure: after mutating through each, each closure sees its own `x`
/// (guards that distinct bindings do not collapse into one upvalue).
#[test]
fn two_bindings_two_closures_stay_distinct() {
    assert_eq!(
        eval_number(
            "function g() {
               let a, b;
               { let x = 1; a = () => x++; }
               { let x = 100; b = () => x++; }
               a(); a(); b();
               return a() * 1000 + b();
             }
             g();"
        ),
        // a: x was 1, two calls -> 3, third call returns 3; b: 100 -> 101, returns 101.
        3101.0
    );
}

/// Control: ordinary captures keep working.
#[test]
fn control_ordinary_capture_and_var_hoisting() {
    assert_eq!(
        eval_number("function g() { let x = 5; return () => x; } g()();"),
        5.0
    );
    assert_eq!(
        eval_number("function g() { const f = () => v; var v = 9; return f(); } g();"),
        9.0
    );
    assert_eq!(
        eval_number("function g() { { var v = 4; } return () => v; } g()();"),
        4.0
    );
    assert_eq!(
        eval_number("let top = 3; function h() { return top; } h();"),
        3.0
    );
}

/// KNOWN DEVIATION (pinned so Slice B landing is visible). Spec: Block
/// Evaluation step 2 ("Let blockEnv be NewDeclarativeEnvironment(oldEnv)",
/// ECMA-262 §14.2.2) with step 3 BlockDeclarationInstantiation, so every
/// Block evaluation creates a fresh `x` and the closures see "5,6,7". Today
/// the binding is per function entry, so every closure sees the last `x` (7).
/// Owner: `#11-vm-block-declaration-instantiation` (Slice B: a fresh binding
/// per Block evaluation), with `#11-vm-per-iteration-loop-environment`
/// (Slice E) planned to supply the shared mechanism.
#[test]
fn known_deviation_block_body_binding_per_entry() {
    assert_eq!(
        eval_string(
            "const out = [];
             for (let i = 0; i < 3; i++) { let x = i + 5; out.push(() => x); }
             out.map(f => f()).join(',');"
        ),
        "7,7,7"
    );
}

/// KNOWN DEVIATION (pinned). FunctionDeclarationInstantiation step 30.1:
/// "NOTE: A separate Environment Record is needed to ensure that closures
/// created by expressions in the formal parameter list do not have
/// visibility of declarations in the function body." Spec (step 30.2, a
/// separate varEnv): `v` is invisible to the default-parameter closure, so
/// the call throws `ReferenceError: v is not defined`. Today it returns 1.
/// F2 does not change this: the body var shares the parameter scope, so
/// position-aware resolution still finds it. Owner:
/// `#11-vm-parameter-environment-instantiation`.
#[test]
fn known_deviation_default_param_closure_sees_body_var() {
    assert_eq!(
        eval_number("function g(a = () => v) { var v = 1; return a(); } g();"),
        1.0
    );
}
