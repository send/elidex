//! `yield*` (delegating yield) bytecode expansion (ES2020 §14.4.14).
//!
//! Split from `expr.rs` to keep that file under the project's 1000-line
//! convention; the emitted layout is large (GetIterator + loop +
//! exception / finally stubs) and self-contained.

use crate::arena::NodeId;
use crate::ast::{Expr, Program};
use crate::bytecode::opcode::Op;
use crate::scope::ScopeAnalysis;

use super::expr::compile_expr;
use super::function::FunctionCompiler;
use super::resolve::FunctionScope;
use super::CompileError;

/// Compile `yield* expr` as an in-line iteration loop.
///
/// Leaves the inner iterator's final `return` value on the stack (the
/// expression value of `yield*`).  Each inner step's `value` is re-yielded
/// to the outer generator; the outer's resume arg is forwarded to the
/// inner's `.next(arg)` on the following step.
///
/// Abrupt completion forwarding (simplified):
/// - Outer `.throw(e)` at a yield* suspend point: the catch handler calls
///   `IteratorClose` on the inner iterator, then re-throws `e`.
///   Note: spec §14.4.14 step 8.b has a more elaborate "try `iter.throw`
///   method first" path; the current implementation treats a missing
///   throw method and a throwing throw method identically (close + throw).
///   Proper `iter.throw` method forwarding is a future spec-alignment
///   task (likely landed with the Test262 alignment PR).
/// - Outer `.return(v)` at a yield* suspend point: the finally handler
///   calls `IteratorClose` then resumes the pending `Return(v)`
///   completion via `Op::EndFinally`, which walks further outer
///   finallies or performs the return.  Spec §14.4.14 step 8.c's
///   "try `iter.return` method with the value" is reduced to a plain
///   `IteratorClose` here (same caveat as above).
pub(super) fn compile_yield_star(
    fc: &mut FunctionCompiler,
    prog: &Program,
    analysis: &ScopeAnalysis,
    func_scopes: &mut [FunctionScope],
    arg_id: NodeId<Expr>,
) -> Result<(), CompileError> {
    // Evaluate the iterable and obtain its iterator.
    compile_expr(fc, prog, analysis, func_scopes, arg_id)?;
    fc.emit(Op::GetIterator);

    // Temp locals: iter + received (resume value passed to iter.next).
    let iter_slot = func_scopes[fc.func_scope_idx].next_local;
    func_scopes[fc.func_scope_idx].next_local += 1;
    let received_slot = func_scopes[fc.func_scope_idx].next_local;
    func_scopes[fc.func_scope_idx].next_local += 1;

    fc.emit_u16(Op::SetLocal, iter_slot);
    fc.emit(Op::Pop);

    // Initial received value = undefined (first iter.next(undefined)).
    fc.emit(Op::PushUndefined);
    fc.emit_u16(Op::SetLocal, received_slot);
    fc.emit(Op::Pop);

    // Exception handler: catch_ip = throw-forwarding stub that
    //                    IteratorCloses + rethrows; finally_ip = finally
    //                    stub that IteratorCloses + EndFinallys so outer
    //                    `.return(v)` can propagate through.
    let handler_patch_pos = fc.pc() + 1; // skip opcode byte
    fc.emit_u16_u16(Op::PushExceptionHandler, 0, 0);

    // Property-name constants (reused across the IC slots).
    let next_name = fc.add_name("next");
    let done_name = fc.add_name("done");
    let value_name = fc.add_name("value");

    let loop_start = fc.pc();
    // Call iter.next(received).
    //   [iter iter.next received] → CallMethod(1) → [result]
    fc.emit_u16(Op::GetLocal, iter_slot);
    fc.emit(Op::Dup);
    let next_ic = fc.alloc_ic_slot();
    fc.emit_u16_u16(Op::GetProp, next_name, next_ic);
    fc.emit_u16(Op::GetLocal, received_slot);
    let call_ic = fc.alloc_call_ic_slot();
    fc.emit_u8_u16(Op::CallMethod, 1, call_ic);

    // Check result.done.  If true, exit; result stays on stack for
    // result.value retrieval as the expression result.
    fc.emit(Op::Dup);
    let done_ic = fc.alloc_ic_slot();
    fc.emit_u16_u16(Op::GetProp, done_name, done_ic);
    let exit_patch = fc.emit_jump(Op::JumpIfTrue);

    // Not done: yield result.value; on resume, save the arg.
    let value_ic = fc.alloc_ic_slot();
    fc.emit_u16_u16(Op::GetProp, value_name, value_ic);
    fc.emit(Op::Yield);
    fc.emit_u16(Op::SetLocal, received_slot);
    fc.emit(Op::Pop);
    fc.emit_jump_to(Op::Jump, loop_start);

    // ── Done: result on stack; result.value is the yield* expr value ──
    fc.patch_jump(exit_patch);
    let final_value_ic = fc.alloc_ic_slot();
    fc.emit_u16_u16(Op::GetProp, value_name, final_value_ic);
    fc.emit(Op::PopExceptionHandler);
    let end_jump = fc.emit_jump(Op::Jump);

    // ── Throw handler: close inner, then rethrow ──────────────────────
    // Entered via handle_exception when an outer `.throw(e)` or any
    // uncaught throw from iter.next reaches this frame.
    let throw_handler = fc.pc();
    fc.emit(Op::PushException);
    fc.emit_u16(Op::GetLocal, iter_slot);
    fc.emit(Op::IteratorClose);
    fc.emit(Op::Throw);

    // ── Finally stub: close inner, then propagate pending completion ──
    // Entered only via the finally_ip route — the current caller for
    // that is `route_to_next_finally` (invoked from `resume_generator`
    // on `.return(v)` injection or from a prior `Op::EndFinally` still
    // propagating Return).
    let finally_pc = fc.pc();
    fc.emit_u16(Op::GetLocal, iter_slot);
    fc.emit(Op::IteratorClose);
    fc.emit(Op::EndFinally);

    fc.patch_jump(end_jump);

    // Patch the exception handler's catch_ip + finally_ip.
    fc.patch_exception_handler(handler_patch_pos, Some(throw_handler), Some(finally_pc));

    Ok(())
}
