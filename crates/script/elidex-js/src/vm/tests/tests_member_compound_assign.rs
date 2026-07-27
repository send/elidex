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

/// The rejection must be **scoped** (umbrella I-1; §9 decision 5 picks a runtime
/// throw for unimplemented expressions).  A `CompileError` is loud but not
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

// ── Carved divergences, pinned so they cannot widen unnoticed ─────────
//
// Both are **pre-existing** defects of the element-access family that this slice
// inherits rather than introduces — each is verified below to misbehave on a
// path that predates this slice entirely. Each gets its own plan-reviewed PR;
// these tests assert the CURRENT (wrong) behaviour so the blast radius is
// fenced, and flip when the slots land.

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

/// `Op::GetElemRef` is rooted **by construction** — it reads the `[object key]`
/// pair in place rather than popping it, so the base stays on the GC-rooted VM
/// stack while the key conversion runs user code.
///
/// This matters beyond a wrong read: `GetElemRef` hands its base to the
/// following `SetElem`, so a base collected mid-conversion would be a **store
/// through a dangling `ObjectId`** — a write into whatever object recycled the
/// slot, or `get_object_mut`'s "object already freed" panic. The read-side
/// members of the family only return a wrong value; this is the one path that
/// writes, which is why it is fixed here rather than carried to the slot.
///
/// The allocation count below has to actually provoke a collection for this to
/// be a real guard; if a GC-threshold change ever makes it stop provoking one,
/// this test passes vacuously rather than failing loudly.
#[test]
fn computed_compound_keeps_temporary_base_rooted() {
    let key = "{toString(){var a=[]; for(var i=0;i<2000;i++) a.push({x:i}); return 'p'}}";
    let temp_base = "(function(){return {p:1}})()";

    assert_eq!(eval_number(&format!("{temp_base}[{key}] += 2")), 3.0);
    assert_eq!(eval_number(&format!("{temp_base}[{key}] ??= 9")), 1.0);
    assert_eq!(eval_number(&format!("{temp_base}[{key}] ||= 9")), 1.0);
}

/// The whole element-access family is rooted **by construction**: every arm
/// reads its operands in place by index instead of popping them into Rust
/// locals, so the base stays on the GC-rooted VM stack while the key conversion
/// — and, for `++`, the `ToNumber` of the old value — runs user code. The three
/// mutating members are why it matters: a base collected mid-conversion would
/// make their store or delete go through a dangling `ObjectId`, into whatever
/// object recycled the slot.
///
/// A store or delete through a temporary base leaves nothing to read back, so
/// each mutating case carries its own witness: an accessor whose setter is
/// reachable from the base alone (it cannot run if the base was collected), and
/// a non-configurable property whose deletion must throw (a recycled object
/// would report success instead).
///
/// The allocation count below has to actually provoke a collection for this to
/// be a real guard; if a GC-threshold change ever makes it stop provoking one,
/// this test passes vacuously rather than failing loudly.
#[test]
fn element_access_keeps_temporary_base_rooted() {
    let key = "{toString(){var a=[]; for(var i=0;i<2000;i++) a.push({x:i}); return 'p'}}";
    let temp_base = "(function(){return {p:1}})()";

    // `Op::GetElem` — read-only; a collected base read as `undefined`.
    assert_eq!(eval_number(&format!("{temp_base}[{key}]")), 1.0);
    // `Op::IncElem` — read-modify-write; postfix yields the old value, so a
    // collected base surfaced as `NaN`.
    assert_eq!(eval_number(&format!("{temp_base}[{key}]++")), 1.0);
    // `Op::DecElem` — same arm, opposite direction.
    assert_eq!(eval_number(&format!("--{temp_base}[{key}]")), 0.0);
    // `Op::SetElem` — the store has to land on the *intended* object.
    assert_eq!(
        eval_number(&format!(
            "var seen=0; (function(){{return {{set p(v){{seen=v}}}}}})()[{key}] = 7; seen"
        )),
        7.0
    );
    assert_eq!(eval_number(&format!("{temp_base}[{key}] = 7")), 7.0);
    // `Op::DeleteElem` — mutating; the frozen base is the sharp witness.
    assert!(eval_bool(&format!("delete {temp_base}[{key}]")));
    eval_throws(&format!(
        "delete (function(){{return Object.freeze({{p:1}})}})()[{key}]"
    ));

    // Control: a base bound to a variable lives in the frame's stack slots, so
    // it was rooted even before the fix.  These pin that reading the operands in
    // place left the ordinary path — including `DeleteElem`'s array-index fast
    // path — alone.
    assert_eq!(eval_number(&format!("var o={{p:1}}; o[{key}]")), 1.0);
    assert_eq!(eval_number(&format!("var o={{p:1}}; o[{key}]++")), 1.0);
    assert_eq!(
        eval_number(&format!("var o={{p:1}}; o[{key}] = 7; o.p")),
        7.0
    );
    assert!(matches!(
        eval(&format!("var o={{p:1}}; delete o[{key}]; o.p")).unwrap(),
        JsValue::Undefined
    ));
    // `DeleteElem`'s array-index fast path: it keys off the *raw* operand, so a
    // Number or already-string key reaches it and punches a hole in the dense
    // storage.  (An object key that stringifies to an index does not — it takes
    // the generic path, which does not see dense elements at all.  Pre-existing
    // and unchanged here: `delete a[{toString(){return '0'}}]` reports success
    // while leaving `a[0]` intact, on both sides of this fix.)
    assert!(matches!(
        eval("var a=[1,2,3]; delete a['0']; a[0]").unwrap(),
        JsValue::Undefined
    ));
    assert!(matches!(
        eval("var a=[1,2,3]; delete a[0]; a[0]").unwrap(),
        JsValue::Undefined
    ));
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
