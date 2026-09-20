//! GC-window pins for the member-assignment slice (Slice 0a).
//!
//! Split out of `tests_member_compound_assign` at the touch-time threshold. The
//! seam: each test here places a collection with the `force_gc_before_next_alloc`
//! one-shot and asserts what survives it — a different kind of assertion from the
//! behavioural cases next door.
//!
//! `get_elem_ref_keeps_temporary_base_rooted` pins a property the slice **owns**;
//! the two `*_known_divergence` tests pin defects it **carries** under
//! `#11-vm-operand-rooting-by-construction`, and flip to the spec answer when
//! that slot lands.

use super::eval_number;
use crate::vm::{JsValue, Vm};

/// `Op::GetElemRef` is rooted **by construction**: it reads the `[object key]`
/// pair in place instead of popping it, so the base stays on the GC-rooted VM
/// stack (`gc/roots.rs` walks the stack but not Rust locals) while the key
/// conversion runs user `toString`.  It is the only arm whose **rooting** this
/// slice owns — `Op::IncElem`/`Op::DecElem` and the rest of the dispatch loop
/// keep merge-base behaviour under `#11-vm-operand-rooting-by-construction`.
///
/// The stack effect alone would not force the property (a pop-then-repush
/// produces the same `[object key -- object key' value]`), so it needs a pin:
/// this test fails on a popping implementation, because the base collected
/// mid-conversion is then stored through by the following `Op::SetElem`.
///
/// The collection is *placed*, not provoked: `setup` performs every allocation
/// the case needs, then the one-shot is armed, so the first `alloc_object` of
/// `expr` takes it — and that is the `var t = {}` inside the key's `toString`,
/// i.e. inside the user-code window.
///
/// ⚠ The trailing assertion is a weaker witness than the placement argument:
/// the one-shot clears on **any** allocation inside `vm.eval(expr)`, so it
/// proves only that something allocated, not that it was the one in the window.
/// It still catches the mode that matters (a case that stops allocating at all,
/// which would pass vacuously); the placement itself rests on `setup`
/// front-loading every other allocation, and would need re-checking if the VM
/// ever allocated eagerly per-eval. `#11-vm-operand-rooting-by-construction`
/// carries the affordance that arms from *inside* the window (`__armGc`), which
/// witnesses placement directly.
///
/// (Pinning `gc_threshold` low does NOT work: `collect_garbage` ends by
/// resetting it to `(live_count * 128).max(32768)`, so a low threshold buys
/// exactly one collection, at the script's *first* allocation.)
#[test]
fn get_elem_ref_keeps_temporary_base_rooted() {
    // `mk()` hands out the base while dropping the pool's reference, so the
    // operand slot is the only thing holding it.
    let setup = "globalThis.pool = []; globalThis.mk = function () { return pool.pop() }; \
                 pool.push({p: 1}); \
                 globalThis.k = {toString(){ var t = {}; return 'p' }};";
    let gc_in_window = |expr: &str| {
        let mut vm = Vm::new();
        vm.eval(setup).expect("probe setup must succeed");
        vm.inner.force_gc_before_next_alloc = true;
        let result = vm.eval(expr);
        assert!(
            !vm.inner.force_gc_before_next_alloc,
            "no collection was placed in the window — `{expr}` never allocated, \
             so this case proves nothing about rooting"
        );
        result
    };
    for expr in ["mk()[k] += 2", "mk()[k] ??= 9", "mk()[k] ||= 9"] {
        match gc_in_window(expr) {
            Ok(JsValue::Number(n)) => {
                assert_eq!(n, if expr.contains("+=") { 3.0 } else { 1.0 }, "{expr}");
            }
            other => panic!("expected a number from `{expr}`, got {other:?}"),
        }
    }
}

/// True when `result` is anything other than the spec's `expected` — a freed or
/// recycled operand shows up as a `TypeError`, a `NaN`, or a wrong number
/// depending on allocator arithmetic, and the divergence is that it is not the
/// spec answer, not which of those it happens to be.
fn diverged_from(result: &Result<JsValue, crate::vm::VmError>, expected: f64) -> bool {
    !matches!(result, Ok(JsValue::Number(n)) if *n == expected)
}

/// ⚠ CARVED: `#11-vm-operand-rooting-by-construction`.
///
/// `Op::IncElem` / `Op::DecElem` pop the base into a Rust local and then cross
/// **two** user-code windows before storing through it — `get_element_keeping_key`
/// runs the key's `toString`, `to_number` can run the old value's `valueOf` — so
/// a collection in either window leaves `set_element` writing through a collected
/// `ObjectId`.
///
/// This arm is the one the slice *did* touch (moved into `op_inc_dec_elem` and
/// routed through `get_element_keeping_key` for the single key conversion), so
/// leaving its rooting to the slot needs a pin of its own: the compound-operator
/// pin below never emits these opcodes. The exposure itself is merge-base
/// behaviour — the arm popped its operands before this slice and still does.
#[test]
fn inc_elem_base_lost_to_gc_known_divergence() {
    let setup = "globalThis.pool=[]; globalThis.mk=function(){return pool.pop()}; \
                 pool.push({p:1}); globalThis.k={toString(){var t={}; return 'p'}};";
    let probe = |expr: &str| {
        let mut vm = Vm::new();
        vm.eval(setup).expect("probe setup must succeed");
        vm.inner.force_gc_before_next_alloc = true;
        let result = vm.eval(expr);
        assert!(
            !vm.inner.force_gc_before_next_alloc,
            "no collection was placed in the window — `{expr}` proves nothing"
        );
        result
    };
    // ECMA-262 §13.4.2.1 (postfix `++`) yields the old value, §13.4.5.1 (prefix
    // `--`) the new one.
    for (expr, expected) in [("mk()[k]++", 1.0), ("--mk()[k]", 0.0)] {
        assert!(
            diverged_from(&probe(expr), expected),
            "`{expr}` produced the spec answer with a collection in the window — \
             either `#11-vm-operand-rooting-by-construction` has landed (flip \
             this pin to assert {expected}), or the collection stopped reaching \
             the base"
        );
    }
    // Without a collection in the window both are correct.
    assert_eq!(eval_number(&format!("{setup} mk()[k]++")), 1.0);
    assert_eq!(eval_number(&format!("{setup} --mk()[k]")), 0.0);
}

/// ⚠ CARVED: `#11-vm-operand-rooting-by-construction`.
///
/// The compound operators pop both operands before coercing either, so the one
/// that has not been consumed yet spends the other's user code in a Rust local —
/// which `gc/roots.rs` does not walk.  ECMA-262 §13.15.3
/// `ApplyStringOrNumericBinaryOperator` steps 3-4 coerce the **left** operand
/// first, so it is the right operand that is exposed.
///
/// This is **pre-existing, not introduced or widened by this slice**, and the
/// second case is the proof rather than an assertion: `z -= mk()` reaches
/// `binary_numeric` through the identifier lowering that predates the slice
/// entirely, and it diverges too. The slice adds a *spelling* that
/// reaches an already-reachable defect — `o[k] -= v` used to abort the process
/// in the compiler, so it never got here at all.
///
/// The assertion is "not the spec answer", not a specific error: whether the
/// freed slot stays empty (→ `ToPrimitive` finds no `valueOf` and throws) or is
/// recycled by the next allocation (`free_objects.pop()` → a plain object whose
/// inherited `toString` succeeds → `NaN`) is arithmetic over the globals-init
/// object count, and one new built-in global can flip it. Pinning the message
/// would fence that artifact rather than the divergence.
#[test]
fn compound_assign_rhs_lost_to_gc_known_divergence() {
    // `mk()` hands out the RHS while dropping the pool's reference, so the
    // operand is the only thing holding it.  The LHS `valueOf` allocates, and
    // the one-shot places the collection there — inside the window where the
    // popped RHS is unrooted.
    let probe = |setup: &str, expr: &str| {
        let mut vm = Vm::new();
        vm.eval(setup).expect("probe setup must succeed");
        vm.inner.force_gc_before_next_alloc = true;
        let result = vm.eval(expr);
        assert!(
            !vm.inner.force_gc_before_next_alloc,
            "no collection was placed in the window — `{expr}` proves nothing"
        );
        result
    };
    let pool = "globalThis.pool=[]; globalThis.mk=function(){return pool.pop()}; \
                pool.push({valueOf(){return 2}}); ";
    let computed = format!(
        "{pool} globalThis.o={{p:{{valueOf(){{var t={{}}; return 1}}}}}}; globalThis.k='p';"
    );
    let identifier = format!("{pool} globalThis.z={{valueOf(){{var t={{}}; return 1}}}};");

    // Spec: both evaluate to `-1`.  Current: the RHS is collected mid-coercion,
    // so the operand is read out of a freed or recycled slot.
    for (setup, expr) in [(&computed, "o[k] -= mk()"), (&identifier, "z -= mk()")] {
        assert!(
            diverged_from(&probe(setup, expr), -1.0),
            "`{expr}` produced the spec answer with a collection in the window — \
             either `#11-vm-operand-rooting-by-construction` has landed (flip \
             this pin to assert -1), or the collection stopped reaching the RHS"
        );
    }
    // Without a collection in the window both are correct, which is what
    // identifies the defect as rooting rather than arithmetic.
    assert_eq!(eval_number(&format!("{computed} o[k] -= mk()")), -1.0);
    assert_eq!(eval_number(&format!("{identifier} z -= mk()")), -1.0);
}
