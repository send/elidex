//! Compound and logical assignment to member targets (Slice 0a).
//!
//! Before this slice all three forms **panicked the process** on valid JS:
//! `obj[key] op= v` hit an `assert!` in `compiler/expr_assign.rs`, and the
//! logical forms (`||=` / `&&=` / `??=`) reached `compound_op_to_opcode`'s
//! `unreachable!()` because short-circuiting was implemented only for the
//! identifier target.
//!
//! Spec: ECMA-262 §13.15.2 Runtime Semantics: Evaluation
//! (`AssignmentExpression : LeftHandSideExpression AssignmentOperator
//! AssignmentExpression`) — the LeftHandSideExpression is evaluated **once**
//! (step 1) and that same reference is reused by `GetValue` (step 3) and
//! `PutValue` (step 9).  The logical forms evaluate the RHS only when the
//! short-circuit test fails.

use super::{eval_number, eval_string, eval_throws};

// ── Computed compound: `obj[key] op= v` ──────────────────────────────

#[test]
fn computed_compound_arithmetic() {
    assert_eq!(eval_number("var o={n:1}; var k='n'; o[k]+=2; o.n"), 3.0);
    assert_eq!(eval_number("var a=[10]; a[0]-=3; a[0]"), 7.0);
    assert_eq!(eval_number("var a=[8]; a[0]>>=2; a[0]"), 2.0);
    assert_eq!(eval_number("var a=[2]; a[0]**=3; a[0]"), 8.0);
}

#[test]
fn computed_compound_string_concat() {
    assert_eq!(eval_string("var a=['a']; a[0]+='b'; a[0]"), "ab");
}

#[test]
fn computed_compound_yields_new_value() {
    assert_eq!(eval_number("var a=[1]; var r=(a[0]+=2); r"), 3.0);
}

/// §13.15.2 step 1 — the LHS reference is evaluated once, so a side-effecting
/// key expression must not run twice.
#[test]
fn computed_compound_evaluates_key_once() {
    assert_eq!(
        eval_number("var n=0; function k(){n++; return 0} var a=[1]; a[k()]+=2; n"),
        1.0
    );
    assert_eq!(
        eval_number("var n=0; var o={get g(){n++; return {v:1}}}; o.g.v+=1; n"),
        1.0
    );
}

// ── Named-member logical: `o.p ||= v` ────────────────────────────────

#[test]
fn named_logical_or_assign() {
    assert_eq!(eval_number("var o={p:1}; o.p ||= 9; o.p"), 1.0);
    assert_eq!(eval_number("var o={p:0}; o.p ||= 9; o.p"), 9.0);
}

#[test]
fn named_logical_and_assign() {
    assert_eq!(eval_number("var o={p:1}; o.p &&= 9; o.p"), 9.0);
    assert_eq!(eval_number("var o={p:0}; o.p &&= 9; o.p"), 0.0);
}

#[test]
fn named_logical_nullish_assign() {
    assert_eq!(eval_number("var o={p:null}; o.p ??= 9; o.p"), 9.0);
    assert_eq!(eval_number("var o={p:5}; o.p ??= 9; o.p"), 5.0);
    // `0` is falsy but NOT nullish — `??=` must not overwrite it.
    assert_eq!(eval_number("var o={p:0}; o.p ??= 9; o.p"), 0.0);
}

#[test]
fn named_logical_yields_correct_value_on_both_paths() {
    assert_eq!(eval_number("var o={p:0}; var r=(o.p ||= 9); r"), 9.0);
    assert_eq!(eval_number("var o={p:1}; var r=(o.p ||= 9); r"), 1.0);
}

// ── Computed logical: `o[k] ||= v` ───────────────────────────────────

#[test]
fn computed_logical_assign() {
    assert_eq!(eval_number("var o={p:0}; o['p'] ||= 9; o.p"), 9.0);
    assert_eq!(eval_number("var o={p:1}; o['p'] ||= 9; o.p"), 1.0);
    assert_eq!(eval_number("var a=[null]; a[0] ??= 9; a[0]"), 9.0);
    assert_eq!(eval_number("var a=[5]; a[0] ??= 9; a[0]"), 5.0);
    assert_eq!(eval_number("var a=[1]; var r=(a[0] &&= 7); r"), 7.0);
}

// ── Short-circuit observable behaviour ───────────────────────────────

/// The setter runs only when the short-circuit test fails.
#[test]
fn logical_assign_accessor_call_order() {
    let mk = |init: &str| {
        format!(
            "var log=''; var o={{}}; Object.defineProperty(o,'p',\
             {{get(){{log+='g';return {init}}},set(v){{log+='s'}}}}); o.p ||= 9; log"
        )
    };
    assert_eq!(eval_string(&mk("0")), "gs");
    assert_eq!(eval_string(&mk("1")), "g");
}

/// The RHS is not evaluated when the assignment short-circuits.
#[test]
fn logical_assign_rhs_not_evaluated_on_short_circuit() {
    let src = "var n=0; function r(){n++; return 9} var o={p:PLACEHOLDER}; o.p ||= r(); n";
    assert_eq!(eval_number(&src.replace("PLACEHOLDER", "1")), 0.0);
    assert_eq!(eval_number(&src.replace("PLACEHOLDER", "0")), 1.0);
}

/// Every form must leave exactly one value on the stack.
#[test]
fn assignment_forms_are_stack_balanced() {
    for src in [
        "var a=[1]; a[0]+=1; var z=42; z",
        "var a=[1]; a[0] ||= 2; var z=42; z",
        "var o={p:1}; o.p ??= 2; var z=42; z",
        "var o={p:null}; o.p ??= 2; var z=42; z",
        "var o={p:0}; o.p &&= 2; var z=42; z",
    ] {
        assert_eq!(eval_number(src), 42.0, "stack imbalance in: {src}");
    }
}

// ── Regressions found by the pre-push review ─────────────────────────

/// A logical operator on a **private** name used to reach
/// `compound_op_to_opcode`'s `unreachable!` and abort the process — the guard
/// matched only `MemberProp::Identifier`, not `PrivateIdentifier`.  Until Slice 5
/// gives `#x` a store path it must be a loud `CompileError`, never a panic and
/// never a silently-lost write.
#[test]
fn private_name_logical_assign_is_rejected_not_a_panic() {
    for op in ["||=", "&&=", "??="] {
        eval_throws(&format!(
            "class C{{#x=0; m(){{ this.#x {op} 1 }}}}; new C().m()"
        ));
    }
}

/// `??=` must not leave a stray beneath its result.  `JumpIfNotNullish` peeks
/// where `JumpIfFalse`/`JumpIfTrue` pop, so an unconditional `Dup` leaked one
/// slot per evaluation — observable because the stray was then read as a
/// callee.
#[test]
fn identifier_nullish_assign_is_stack_balanced() {
    assert_eq!(
        eval_number("var x=null; function f(a){return a} f(x ??= 1)"),
        1.0
    );
    assert_eq!(
        eval_number("var x=7; function f(a){return a} f(x ??= 1)"),
        7.0
    );
    // The member forms share the rule via `emit_logical_assign_tail`.
    assert_eq!(
        eval_number("var o={p:null}; function f(a){return a} f(o.p ??= 1)"),
        1.0
    );
    assert_eq!(
        eval_number("var a=[null]; function f(v){return v} f(a[0] ??= 1)"),
        1.0
    );
}

/// ⚠ KNOWN SPEC-DIVERGENCE (`#11-vm-element-ref-single-key-conversion`).
///
/// ECMA-262 §6.2.5.5 GetValue step 3.c memoizes `ToPropertyKey` into the
/// Reference Record, so a stateful key's `toString` runs **once** and the read
/// and write hit the same property.  `Op::Dup2` copies the raw key, so
/// `GetElem` and `SetElem` convert independently.  These assertions pin the
/// *current* behaviour so the divergence cannot widen unnoticed; flip them when
/// the slot lands.
#[test]
fn computed_compound_converts_key_twice_known_divergence() {
    // Spec: 1.  Current: 2.
    assert_eq!(
        eval_number("var n=0; var k={toString(){n++;return 'p'}}; var o={p:1}; o[k]+=2; n"),
        2.0
    );
    // Spec: reads and writes `p1` → {"p1":6,"p2":0}.  Current: reads `p1`, writes `p2`.
    assert_eq!(
        eval_string(
            "var n=0; var k={toString(){n++;return 'p'+n}}; var o={p1:1,p2:0}; \
             o[k]+=5; JSON.stringify(o)"
        ),
        r#"{"p1":1,"p2":6}"#
    );
    // Plain assignment is unaffected — it converts once.
    assert_eq!(
        eval_number("var n=0; var k={toString(){n++;return 'p'}}; var o={p:1}; o[k]=2; n"),
        1.0
    );
}
