//! Upvalue identity: one binding = one cell (ECMA-262 §9.1 environment
//! record identity, observed through closure capture).
//!
//! Closures created in the same frame over the same parent local must
//! share a single upvalue cell, both while the frame is live and after it
//! exits (or a generator finishes).  These cells assert observable JS
//! behaviour only; they do not depend on how the VM represents captures.
//!
//! Not covered here: the per-iteration loop case (`for (let i...)` pushing
//! closures, probe P28 expecting `[1,1]`) is `#11-vm-per-iteration-loop-environment`
//! (Slice E)'s acceptance and needs that slice as well; dedup alone does
//! not turn it green.

use super::eval_number;

#[test]
fn share_after_frame_exit() {
    // P10 shape.
    assert_eq!(
        eval_number(
            "const mk = function () { let x = 0; return [() => ++x, () => x]; }; \
             const p = mk(); p[0](); p[1]();"
        ),
        1.0
    );
}

#[test]
fn share_while_frame_live() {
    assert_eq!(
        eval_number(
            "const f = function () { let x = 0; \
               const a = () => ++x; const b = () => x; \
               a(); a(); return b(); }; f();"
        ),
        2.0
    );
}

#[test]
fn share_three_closures_after_exit() {
    assert_eq!(
        eval_number(
            "const mk = function () { let x = 0; \
               return [() => ++x, () => x, () => (x += 10)]; }; \
             const p = mk(); p[0](); p[2](); p[1]();"
        ),
        11.0
    );
}

#[test]
fn share_nested_transitive_capture() {
    // The inner function captures `x` through the middle function's
    // upvalue; all three observers must see one cell after exit.
    assert_eq!(
        eval_number(
            "const mk = function () { let x = 0; \
               const mid = () => () => ++x; \
               return [mid(), () => x, mid()]; }; \
             const p = mk(); p[0](); p[2](); p[1]();"
        ),
        2.0
    );
}

#[test]
fn share_across_generator_yield() {
    assert_eq!(
        eval_number(
            "function* g() { let x = 0; \
               const a = () => ++x; const b = () => x; \
               yield [a, b]; a(); yield 0; a(); } \
             const it = g(); const p = it.next().value; \
             it.next(); it.next(); p[1]();"
        ),
        2.0
    );
}

#[test]
fn share_generator_closure_created_after_resume() {
    // Second closure is created after a yield/resume cycle: it must reuse
    // the cell the first closure created before the yield.
    assert_eq!(
        eval_number(
            "function* g() { let x = 0; \
               const a = () => ++x; yield a; \
               const b = () => x; yield b; } \
             const it = g(); const a = it.next().value; const b = it.next().value; \
             a(); it.next(); b();"
        ),
        1.0
    );
}

#[test]
fn independent_calls_do_not_share() {
    // Control: separate frames, separate cells.
    assert_eq!(
        eval_number(
            "const mk = function () { let x = 0; return [() => ++x, () => x]; }; \
             const p1 = mk(); const p2 = mk(); p1[0](); p2[1]();"
        ),
        0.0
    );
}

#[test]
fn distinct_locals_do_not_share() {
    // Control: two different slots in one frame stay independent.
    assert_eq!(
        eval_number(
            "const mk = function () { let x = 0; let y = 100; \
               return [() => ++x, () => y]; }; \
             const p = mk(); p[0](); p[1]();"
        ),
        100.0
    );
}

#[test]
fn live_caller_frame_cell_is_not_reused_by_callee() {
    // Control: while the outer frame's cell for slot N is still Open, a
    // callee capturing *its own* slot N must get a distinct cell.
    assert_eq!(
        eval_number(
            "const outer = function () { let x = 1; const ox = () => x; \
               const inner = function () { let y = 2; const iy = () => y; return iy(); }; \
               return inner() * 10 + ox(); }; outer();"
        ),
        21.0
    );
}
