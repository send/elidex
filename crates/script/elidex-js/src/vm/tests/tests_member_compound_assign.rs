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

use super::{eval, eval_bool, eval_number, eval_string, eval_throws};
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
/// must raise a loud error — never a panic, never a silently-lost write.
///
/// The two original failure modes are different, which is why the guard is not
/// per-form: the *logical* ops reached `compound_op_to_opcode`'s `unreachable!`
/// and aborted the process, while the *plain* and *compound* ops emitted a store
/// that fell to an `Op::Pop` tail — discarding the write and leaving the
/// **object** as the expression's value (`return A.#x += 5` evaluated to `A`).
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

/// The rejection must be **scoped** (umbrella I-1; the umbrella's decision 5
/// picks a runtime throw for unimplemented expressions).  A `CompileError` is loud but not
/// scoped: it yields no bytecode for the whole script, so one unsupported store
/// anywhere would take every unrelated statement in the file down with it —
/// strictly worse than the pre-slice behaviour for `=` and `+=`, which at least
/// let the rest of the script run.  `Op::ThrowUnsupported` keeps it local.
#[test]
fn private_name_assign_throws_at_runtime_not_at_compile_time() {
    // Unrelated statements still run, and the script's value is unaffected — the
    // unsupported store only matters if control actually reaches it.
    assert_eq!(
        eval_number("class C{#x=0; m(){ this.#x = 1 }}; 42"),
        42.0,
        "an uncalled private store must not fail the whole script"
    );
    // It is an ordinary catchable TypeError, not a compile abort.
    assert_eq!(
        eval_string(
            "class C{#x=0; m(){ this.#x += 1 }}; \
             var r='no throw'; try { new C().m() } catch (e) { r = e.name } r"
        ),
        "TypeError"
    );
    // And the statements after the catch still execute.
    assert_eq!(
        eval_number(
            "class C{#x=0; m(){ this.#x ??= 1 }}; \
             var n=0; try { new C().m() } catch (e) { n=1 } n+41"
        ),
        42.0
    );
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
    // `o[k]++` reuses one reference too — §13.4.2.1 (postfix) evaluates it at
    // step 1 and reuses it for `GetValue` (step 3) and `PutValue` (step 6).
    // Same root, fixed with it rather than left as a second answer to one
    // question.
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
    // place — §14.2.2 (`StatementList : StatementList StatementListItem`) step 3
    // `UpdateEmpty` (AO §6.2.4.4).  Its Note 2 gives this exact program shape:
    // `eval("1;var a;")` is 1.
    assert_eq!(eval_number("42; var x = 1;"), 42.0);
}

/// ⚠ CARVED: `#11-vm-statement-completion-updateempty`.
///
/// The VM has no `UpdateEmpty` (AO §6.2.4.4) equivalent — `completion_value` is
/// a sticky register that only an ExpressionStatement writes.  But several
/// statements produce a **non-empty `undefined`** that must *overwrite* the
/// accumulated value rather than leave it: §14.6.2 step 3 (`if` with a false
/// test and no else) and its step 5 `UpdateEmpty(stmtCompletion, undefined)`,
/// and the same shape in iteration, `switch`, labelled and `try` statements.
///
/// So the completion split closed the `Op::Pop` half of the ownership bug and
/// left this half open. Recording current behaviour; flip when the slot lands.
#[test]
fn statement_completion_is_sticky_known_divergence() {
    // Spec: `undefined` for each.  Current: the preceding `42` survives.
    assert_eq!(eval_number("42; if (false) {}"), 42.0);
    assert_eq!(eval_number("42; if (true) {}"), 42.0);
    assert_eq!(eval_number("42; while (false) {}"), 42.0);
    assert_eq!(eval_number("42; switch (99) { case 1: 7 }"), 42.0);
    // A neighbour that DOES produce a value is unaffected — this is a missing
    // overwrite, not a missing record.
    assert_eq!(eval_number("42; if (true) { 7 }"), 7.0);
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

/// The for-in/of head is the **third** lowering of an assignment target, and it
/// was the gate's un-swept sibling: `compile_forin_left_binding` decided
/// admissibility inside its own branch, so every non-identifier head discarded
/// the iteration value through `Op::Pop` and ran the loop body with the target
/// never written — the silent no-op shape the gate exists to ban, and the fourth
/// instance of "admissibility decided inside a lowering" in this one slice.
///
/// ECMA-262 §14.7.5.7 ForIn/OfBodyEvaluation routes a **non-destructuring** head
/// through step 8.g.ii.4.a `Completion(PutValue(lhsRef.[[Value]], nextValue))`
/// and a **destructuring** one through step 8.g.i.1.a
/// `DestructuringAssignmentEvaluation` — the analogues of §13.15.2's
/// `LeftHandSideExpression = AssignmentExpression` step 1.e and its
/// destructuring branch at step 5. Both cases appear in the sweep below,
/// and neither has a store lowering, which is why one gate covers them.
///
/// Step 8.g runs **per iteration**, which is why the rejection is emitted at the
/// assignment site: an empty iterable performs no assignment and must not throw.
#[test]
fn forin_of_heads_share_the_assignment_admissibility_gate() {
    for src in [
        "var o={}; for (o.p in {a:1}) {}",
        "var o={}; for (o.p of [1]) {}",
        "var o={}, k='p'; for (o[k] of [1]) {}",
        "var a=[],b=[]; for ([a,b] of [[1,2]]) {}",
        "class C{#x=0; m(){ for (this.#x of [1]) {} }}; new C().m()",
        "class B{}; class D extends B{ m(){ for (super.x of [1]) {} } }; new D().m()",
    ] {
        eval_throws(src);
    }
    // Per-iteration, not per-loop: an empty iterable never assigns, so it must
    // not throw — and the loop still completes normally.
    assert_eq!(eval_number("var o={}; for (o.p of []) {} 42"), 42.0);
    assert_eq!(eval_number("var o={}; for (o.p in {}) {} 42"), 42.0);
    // Scoped: an unlowerable head elsewhere does not stop unrelated statements.
    assert_eq!(
        eval_number("function f(){ var o={}; for (o.p of [1]) {} } 42"),
        42.0
    );
    // The diagnosis matches the assignment forms for the targets the gate names.
    assert_eq!(
        eval_string(
            "class C{#x=0; m(){ for (this.#x of [1]) {} }}; \
             var r=''; try { new C().m() } catch (e) { r = e.message } r"
        ),
        "assignment to a private name is not yet supported"
    );
    // An identifier head still works — the gate did not widen.
    assert_eq!(eval_number("var x=0; for (x of [7]) {} x"), 7.0);
    assert_eq!(eval_string("var x=''; for (x in {q:1}) {} x"), "q");
}

// ── Carved divergences, pinned so they cannot widen unnoticed ─────────
//
// Pre-existing defects this slice inherits rather than introduces. Each gets its
// own plan-reviewed PR; the pins assert the CURRENT (wrong) behaviour so the
// blast radius is fenced, and flip when the slots land. The GC-window pins live
// in `tests_member_compound_assign_gc`; the rest are below.
//
// ⚠ CARVED: `#11-vm-operand-rooting-by-construction`.  Roughly twenty dispatch
// arms pop an operand into a Rust local and then run user JS before reading or
// storing through it; `gc/roots.rs` walks the VM stack but not Rust locals, so a
// collection in that window yields a value read from a recycled object, a store
// through a dangling `ObjectId`, or a `get_object`/`get_object_mut` panic.  This
// slice roots only `Op::GetElemRef`, the one arm it introduces
// (`get_elem_ref_keeps_temporary_base_rooted`); `Op::IncElem`/`Op::DecElem` and
// the rest keep merge-base behaviour, pinned by
// `compound_assign_rhs_lost_to_gc_known_divergence` (the compound operators, via
// `binary_numeric`) and `inc_elem_base_lost_to_gc_known_divergence` (the
// `Op::IncElem`/`Op::DecElem` base hold — the arm this slice moved into a helper
// and routed through `get_element_keeping_key`).  The slot's deliverable is an invariant
// that makes an unrooted hold unrepresentable, not another per-site sweep —
// five successive sweeps each declared a different boundary complete and each
// was falsified by the next round.  `#11-vm-element-access-base-rooting`, which
// an earlier round of this PR recorded as closed, is subsumed by it.

/// ⚠ CARVED: `#11-vm-topropertykey-symbol-from-toprimitive`.
///
/// ECMA-262 §7.1.20 ToPropertyKey step 1 does `ToPrimitive(arg, string)` and
/// step 2 returns the **result** when it is a Symbol. Every implementation in
/// the crate tests the *argument* instead and sends everything else to
/// `to_string`, which rejects a Symbol — so a key object whose `@@toPrimitive`
/// yields a Symbol throws where the spec keys on that Symbol.
///
/// "Every implementation" is literal, and is why this is not a one-helper fix:
/// §7.1.20 is open-coded **8 times** — two named helpers (`VmInner::make_property_key`,
/// `natives_object::to_property_key`) and six inline copies (`get_element`'s
/// symbol fast path, its two primitive-receiver paths, `set_element`'s symbol
/// fast path and its primitive-wrapper path, and `Object.defineProperty`).
/// `make_property_key` has 6 callers, but `get_element` and `set_element` — the
/// plain `o[k]` read and write — are **not** among them. So the unit of work is
/// "collapse the 8 onto one canonical implementation, then fix it", not "patch
/// the helper", and certainly not "special-case this slice's opcode".
#[test]
fn symbol_from_toprimitive_key_throws_known_divergence() {
    let src = |expr: &str| {
        format!(
            "var s=Symbol(); var k={{[Symbol.toPrimitive](){{return s}}}}; \
             var o={{}}; o[s]=1; {expr}"
        )
    };
    // Spec: updates `o[s]` to 3.  Current: TypeError from `to_string`.
    eval_throws(&src("o[k]+=2"));
    // The plain read diverges identically — this is not specific to compound
    // assignment, which is why it is carved rather than patched here.
    eval_throws(&src("o[k]"));
}

/// Sibling targets in the same `match` the private-name guard hardened. Each
/// used to be a *silent* no-op — the umbrella's I-1 bans that shape wherever it
/// appears, not only in the arm a review happened to point at.
///
/// - `(x) += 1` / `(x)++` — `ExprKind::Paren` is never unwrapped, so the arm
///   compiled the RHS and emitted no store: the assignment did nothing and
///   evaluated to the RHS.
/// - `[a,b] = [b,a]` — the destructuring arm compiled the RHS and `Op::Pop`ed
///   it, under a comment claiming it would "fail explicitly". It left ZERO
///   values where every other expression leaves one, so in statement position
///   it underflowed the following discard.
/// - `super.x = v` — lowers its base with `compile_expr`, which turns
///   `ExprKind::Super` into `Op::PushUndefined`; there is no `SetSuperProp` emit
///   path, so the store went to `undefined` and surfaced as a misleading
///   "cannot read property of undefined".
#[test]
fn unlowerable_assignment_targets_throw_rather_than_silently_doing_nothing() {
    for src in [
        "var x = 1; (x) += 1",
        "var x = 1; (x) = 2",
        "var a = [1]; (a[0]) += 1",
        "var a=1, b=2; [a,b] = [b,a]",
        "var o = {}; ({x:o.p} = {x:1})",
        "class B{}; class D extends B{ m(){ super.x = 1 } }; new D().m()",
        "class B{}; class D extends B{ m(){ super.x += 1 } }; new D().m()",
    ] {
        eval_throws(src);
    }
    // ...and the throw is scoped: an unlowerable target elsewhere in the script
    // does not stop unrelated statements from running.
    assert_eq!(eval_number("function f(){ var x=1; (x) += 1 } 42"), 42.0);
}

/// The admissibility gate is one decision site evaluated **before** the
/// computed/non-computed split, so a target's rejection cannot depend on which
/// lowering it would have taken.
///
/// Every guard this slice added was first written inside one branch and was
/// wrong for its sibling: the private-name check started as
/// `MemberProp::Identifier`-only (so `this.#x ??= 1` still aborted the process),
/// its replacement sat below the computed branch, and the `super` guard sat
/// below it too — so `super.x = v` threw the scoped error while `super[k] = v`
/// fell through to `Op::PushUndefined` and a misleading "cannot read property of
/// undefined", *after* evaluating both operands. The `eval_throws` loop is a
/// shape sweep — several of its cases threw before the fix too, for the wrong
/// reason — so the sharp assertions are the side-effect-log block below, which
/// fails on the pre-fix compiler: it pins that the same cause is reported for
/// `super.x` and `super[k]`, and that the throw precedes operand side effects.
#[test]
fn unsupported_member_targets_are_rejected_by_shape_not_by_lowering() {
    let in_class =
        |body: &str| format!("class B{{}}; class D extends B{{ m(){{ {body} }} }}; new D().m()");

    for op in ["=", "+=", "??=", "||=", "&&="] {
        // Named and computed super targets must behave identically.
        eval_throws(&in_class(&format!("super.x {op} 1")));
        eval_throws(&in_class(&format!("super['x'] {op} 1")));
        eval_throws(&in_class(&format!("super[k()] {op} 1")));
        // Private names.
        eval_throws(&format!(
            "class C{{#x=0; m(){{ this.#x {op} 1 }}}}; new C().m()"
        ));
    }

    // The throw precedes operand evaluation: neither the key expression nor the
    // RHS runs. `super[k()] = r()` previously evaluated BOTH before failing.
    for src in [
        "super[k()] = r()",
        "super[k()] += r()",
        "super.x = r()",
        "super[k()] ??= r()",
    ] {
        let program = format!(
            "var log=''; function k(){{log+='k'; return 'x'}} function r(){{log+='r'; return 1}} \
             class B{{}}; class D extends B{{ m(){{ {src} }} }} \
             try {{ new D().m() }} catch (e) {{ log += '!' }} log"
        );
        assert_eq!(
            eval_string(&program),
            "!",
            "side effects ran before the throw in: {src}"
        );
    }
}

/// Update expressions are a second *lowering* of the same targets, not a second
/// set of targets, so they share `unsupported_member_target` rather than
/// re-deciding admissibility.
///
/// Before they did: `this.#x++` emitted only the base, so the update was
/// silently lost and the expression evaluated to the **object** — the exact mode
/// the gate exists to ban, still live one file away after the assignment forms
/// were fixed. `super.x++` lowered its base to `Op::PushUndefined` and gave the
/// misleading "cannot read property of undefined" that `super[k] = v` had.
#[test]
fn update_expressions_share_the_assignment_admissibility_gate() {
    for form in ["{t}++", "++{t}", "{t}--", "--{t}"] {
        for target in ["this.#x", "super.x", "super['x']", "super[k()]"] {
            let expr = form.replace("{t}", target);
            eval_throws(&format!(
                "function k(){{return 'x'}} class B{{}} \
                 class D extends B{{ #x=0; m(){{ {expr} }} }}; new D().m()"
            ));
        }
    }
    // Parenthesized and call targets took the outer `else`, which compiled the
    // operand for side effects and emitted no update at all.
    eval_throws("var x = 1; (x)++");
    eval_throws("var a = [1]; (a[0])++");
    // The rejection stays scoped — an unreachable one does not fail the script.
    assert_eq!(eval_number("class C{#x=0; m(){ this.#x++ }}; 42"), 42.0);
}

/// ⚠ CARVED: `#11-vm-delete-elem-raw-key-array-fast-path`.
///
/// `Op::DeleteElem` derives its array-index fast path from the **raw** operand,
/// so an object key that stringifies to an index skips it — and the generic
/// `try_delete_property` path never consults `ObjectKind::Array { elements }`,
/// so it reports success without clearing the dense element.
///
/// Same root as `#11-vm-topropertykey-symbol-from-toprimitive`: a fast path keyed
/// on the raw value rather than the `ToPropertyKey` *result*. Both are discharged
/// by the canonical computed-member-reference primitive. Pre-existing — this
/// slice does not touch `Op::DeleteElem`, and the assertions below hold
/// byte-identically on the pre-slice tree.
#[test]
fn delete_elem_object_key_misses_dense_element_known_divergence() {
    // Spec: deletes the element, so `a[0]` is `undefined`.  Current: reports
    // `true` and leaves it in place.
    assert!(eval_bool(
        "var a=[1,2,3]; delete a[{toString(){return '0'}}]"
    ));
    assert_eq!(
        eval_number("var a=[1,2,3]; delete a[{toString(){return '0'}}]; a[0]"),
        1.0
    );
    // A raw string or number key takes the fast path and IS correct — which is
    // what identifies the defect as the raw-vs-converted key, not `delete`.
    assert_eq!(
        eval_string("var a=[1,2,3]; delete a['0']; String(a[0])"),
        "undefined"
    );
    assert_eq!(
        eval_string("var a=[1,2,3]; delete a[0]; String(a[0])"),
        "undefined"
    );
}
