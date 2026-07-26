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

use super::{eval, eval_number, eval_string, eval_throws};
use crate::vm::JsValue;

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

/// No store to a private name is compilable until Slice 5
/// (`#11-vm-class-private-fields`) gives `#x` a store opcode, so **every** form
/// must be a loud `CompileError` — never a panic, never a silently-lost write.
///
/// The two failure modes are different, which is why the guard is not per-form:
/// the *logical* ops reached `compound_op_to_opcode`'s `unreachable!` and aborted
/// the process, while the *plain* and *compound* ops emitted a store that fell to
/// an `Op::Pop` tail — discarding the write and leaving the **object** as the
/// expression's value (`return A.#x += 5` evaluated to `A`).
#[test]
fn private_name_assign_is_rejected_not_a_panic_and_not_silent() {
    for op in [
        "=", "+=", "-=", "*=", "/=", "%=", "**=", "&=", "|=", "^=", "<<=", ">>=", ">>>=", "||=",
        "&&=", "??=",
    ] {
        eval_throws(&format!(
            "class C{{#x=0; m(){{ this.#x {op} 1 }}}}; new C().m()"
        ));
        // A static receiver (`A.#x`) reaches the same store path as `this.#x`.
        eval_throws(&format!(
            "class A{{ static #x=0; static m(){{ A.#x {op} 1 }} }} A.m()"
        ));
        // The silent loss was observable without reading the field back, because
        // the expression's value was the object rather than the stored value.
        eval_throws(&format!(
            "class A{{ static #x=0; static m(){{ return A.#x {op} 1 }} }} A.m()"
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

/// ECMA-262 §6.2.5.5 GetValue step 3.c.i sets `[[ReferencedName]]` to
/// `ToPropertyKey`'s result **on the Reference Record**, so §13.15.2's single
/// LHS evaluation converts a stateful key exactly once and the read and the
/// write hit the same property.  `Op::GetElemRef` carries the converted key
/// forward to the store.
#[test]
fn computed_compound_converts_key_once() {
    assert_eq!(
        eval_number("var n=0; var k={toString(){n++;return 'p'}}; var o={p:1}; o[k]+=2; n"),
        1.0
    );
    // A key whose conversion is *observable* must not read one property and
    // write another.
    assert_eq!(
        eval_string(
            "var n=0; var k={toString(){n++;return 'p'+n}}; var o={p1:1,p2:0}; \
             o[k]+=5; JSON.stringify(o)"
        ),
        r#"{"p1":6,"p2":0}"#
    );
    // Plain assignment evaluates the reference once and stores through it, so
    // `SetElem`'s own conversion is already the single conversion.
    assert_eq!(
        eval_number("var n=0; var k={toString(){n++;return 'p'}}; var o={p:1}; o[k]=2; n"),
        1.0
    );
    // The logical forms share the same reference.
    assert_eq!(
        eval_number("var n=0; var k={toString(){n++;return 'p'}}; var o={p:0}; o[k]||=2; n"),
        1.0
    );
    // `o[k]++` (§13.4.4.1 steps 1/5) reuses one reference too — the same root,
    // fixed with it rather than left as a second answer to one question.
    assert_eq!(
        eval_number("var n=0; var k={toString(){n++;return 'p'}}; var o={p:1}; o[k]++; n"),
        1.0
    );
    assert_eq!(
        eval_string(
            "var n=0; var k={toString(){n++;return 'p'+n}}; var o={p1:1,p2:0}; \
             o[k]++; JSON.stringify(o)"
        ),
        r#"{"p1":2,"p2":0}"#
    );
}

/// §6.2.5.5 step 3.a (base coercion) precedes step 3.c (key conversion), so a
/// throwing key `toString` must not pre-empt the base's `TypeError`.  Emitting
/// the conversion inside `Op::GetElemRef` rather than at a separate site is what
/// preserves the order.
#[test]
fn computed_compound_base_coercion_precedes_key_conversion() {
    assert_eq!(
        eval_string(
            "var log=''; var k={toString(){log+='k'; return 'p'}}; \
             try { null[k] += 1 } catch(e) { log += (e instanceof TypeError) ? 'T' : 'X' } log"
        ),
        "T"
    );
}

// ── Completion value ownership ───────────────────────────────────────

/// `Op::Pop` used to record **every** value it discarded as the script's
/// completion value, so internal stack housekeeping overwrote it.  Only an
/// ExpressionStatement's value belongs there (ECMA-262 §14.5.1), which is now
/// `Op::PopCompletion`'s sole job.
#[test]
fn completion_value_is_not_written_by_internal_housekeeping() {
    // Reference cleanup on a short-circuiting member assignment leaked the base
    // object out as the completion value.
    assert!(
        matches!(
            eval("if (globalThis.Math ||= 2) {}").unwrap(),
            JsValue::Undefined
        ),
        "reference cleanup must not write completion_value"
    );
    // A VariableStatement's evaluation is EMPTY (§14.3.2.1), not its initializer.
    assert!(
        matches!(eval("var x = 5;").unwrap(), JsValue::Undefined),
        "a declaration must not write completion_value"
    );
    // ...and an EMPTY completion leaves the preceding statement's value in
    // place (§14.5.1 `UpdateEmpty`).
    assert_eq!(eval_number("42; var x = 1;"), 42.0);
}

/// The ExpressionStatement path still records — the split must not cost the
/// completion values that already worked.
#[test]
fn completion_value_still_records_expression_statements() {
    assert_eq!(eval_number("42;"), 42.0);
    assert_eq!(eval_number("if (true) { 1 }"), 1.0);
    assert_eq!(eval_number("switch(1){case 1: 42}"), 42.0);
    assert_eq!(eval_number("for(var i=0;i<3;i++){ i }"), 2.0);
    assert_eq!(eval_number("try { 7 } finally { }"), 7.0);
    assert_eq!(eval_number("var o={p:0}; o.p ||= 9;"), 9.0);
    assert_eq!(eval_number("var a=[1]; a[0] += 2;"), 3.0);
}

// ── One lowering for every logical-assignment target ─────────────────

/// The identifier form routes through `emit_logical_assign_tail` like the member
/// forms; these pin the semantics across the re-routing.
#[test]
fn identifier_logical_assign_semantics() {
    assert_eq!(eval_number("var x=1; x ||= 9; x"), 1.0);
    assert_eq!(eval_number("var x=0; x ||= 9; x"), 9.0);
    assert_eq!(eval_number("var x=1; x &&= 9; x"), 9.0);
    assert_eq!(eval_number("var x=0; x &&= 9; x"), 0.0);
    assert_eq!(eval_number("var x=null; x ??= 9; x"), 9.0);
    // `0` is falsy but not nullish.
    assert_eq!(eval_number("var x=0; x ??= 9; x"), 0.0);
    // The RHS is not evaluated when the assignment short-circuits.
    assert_eq!(
        eval_number("var n=0; function r(){n++; return 9} var x=1; x ||= r(); n"),
        0.0
    );
    // The expression's value on both paths.
    assert_eq!(eval_number("var x=0; var r=(x ||= 9); r"), 9.0);
    assert_eq!(eval_number("var x=1; var r=(x ||= 9); r"), 1.0);
}
