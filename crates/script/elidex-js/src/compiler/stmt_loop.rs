//! Iteration statements: `for-in` / `for-of` / `while` / `do-while` / `for`.
//!
//! ECMA-262 §14.7 Iteration Statements, carved out of `stmt.rs` at the seam the
//! touch-time 1000-line rule points at: these five arms and
//! `compile_forin_left_binding` form one family (the helper has no caller
//! outside `for-in`/`for-of`), while the helpers that stayed behind —
//! `find_child_block_scope`, `emit_iter_close_range`,
//! `emit_pending_finally_bodies` — are shared with `Block`, `Try`, `Return`,
//! `Break` and `Continue` and therefore belong to neither family.
//!
//! Each function takes the arm's destructured fields by reference so the bodies
//! are unchanged from their inline form; `compile_stmt` is the only caller.

use crate::arena::NodeId;
#[allow(clippy::wildcard_imports)]
use crate::ast::*;
use crate::bytecode::opcode::Op;
use crate::scope::ScopeAnalysis;
use crate::span::Span;

use super::expr::compile_expr;
use super::expr_assign::{emit_unsupported, unsupported_member_target};
use super::function::FunctionCompiler;
use super::resolve::FunctionScope;
use super::stmt::{compile_stmt, find_child_block_scope};
use super::stmt_destructure::{compile_destructure_pattern, compile_pattern_store};
use super::CompileError;

/// `for (left in right) body` — ECMA-262 §14.7.5.
#[allow(clippy::too_many_arguments)] // (fc, prog, analysis, func_scopes) is the compiler-wide convention
pub(super) fn compile_for_in(
    fc: &mut FunctionCompiler,
    prog: &Program,
    analysis: &ScopeAnalysis,
    func_scopes: &mut [FunctionScope],
    left: &ForInOfLeft,
    right: NodeId<Expr>,
    body: NodeId<Stmt>,
    span: Span,
) -> Result<(), CompileError> {
    let saved_scope = fc.current_scope_idx;
    if let Some(child_scope) = find_child_block_scope(analysis, fc.current_scope_idx, span) {
        fc.current_scope_idx = child_scope;
    }
    compile_expr(fc, prog, analysis, func_scopes, right)?;
    fc.emit(Op::ForInIterator);
    let loop_start = fc.pc();
    fc.push_loop(loop_start);
    fc.emit(Op::ForInNext); // [iterator key done]
    let exit_patch = fc.emit_jump(Op::JumpIfTrue); // if done, exit
                                                   // Bind `left` to key (key is on stack).
    compile_forin_left_binding(fc, prog, analysis, func_scopes, left)?;
    compile_stmt(fc, prog, analysis, func_scopes, body)?;
    // Patch continue jumps to loop_start.
    fc.patch_continue_jumps_to(loop_start);
    fc.emit_jump_to(Op::Jump, loop_start);
    fc.patch_jump(exit_patch);
    fc.emit(Op::Pop); // pop leftover key from done path
    fc.emit(Op::Pop); // pop iterator
    fc.pop_loop();
    fc.current_scope_idx = saved_scope;
    Ok(())
}

/// `for (left of right) body` — ECMA-262 §14.7.5.
#[allow(clippy::too_many_arguments)] // (fc, prog, analysis, func_scopes) is the compiler-wide convention
pub(super) fn compile_for_of(
    fc: &mut FunctionCompiler,
    prog: &Program,
    analysis: &ScopeAnalysis,
    func_scopes: &mut [FunctionScope],
    left: &ForInOfLeft,
    right: NodeId<Expr>,
    body: NodeId<Stmt>,
    span: Span,
) -> Result<(), CompileError> {
    let saved_scope = fc.current_scope_idx;
    if let Some(child_scope) = find_child_block_scope(analysis, fc.current_scope_idx, span) {
        fc.current_scope_idx = child_scope;
    }
    compile_expr(fc, prog, analysis, func_scopes, right)?;
    fc.emit(Op::GetIterator);
    // Save iterator to a temp local so return/throw can close it.
    let iter_slot = func_scopes[fc.func_scope_idx].next_local;
    func_scopes[fc.func_scope_idx].next_local += 1;
    fc.emit(Op::Dup);
    fc.emit_u16(Op::SetLocal, iter_slot);
    fc.emit(Op::Pop);

    // §7.4.11 + §14.7.5.7: a throw from `IteratorNext` itself
    // (i.e. the iterator's own `.next()` threw) does NOT trigger
    // IteratorClose — the iterator is already considered closed.
    // Only abrupt completions *after* a successful step (e.g.
    // throw from the loop body) close the iterator.  Gate the
    // catch handler's IteratorClose on `close_flag`, set true
    // after each successful IteratorNext and reset to false
    // before the next step.
    let close_flag_slot = func_scopes[fc.func_scope_idx].next_local;
    func_scopes[fc.func_scope_idx].next_local += 1;
    fc.emit(Op::PushFalse);
    fc.emit_u16(Op::SetLocal, close_flag_slot);
    fc.emit(Op::Pop);

    // Wrap the loop body in an implicit exception handler so that
    // an uncaught throw from the body closes the iterator.
    let handler_pos = fc.pc();
    fc.emit_u16_u16(Op::PushExceptionHandler, 0, 0);
    let handler_patch_pos = handler_pos + 1; // offset of catch u16

    let loop_start = fc.pc();
    fc.push_for_of_loop(loop_start, iter_slot);
    // Reset close_flag = false before IteratorNext so a throw
    // from .next() itself skips the catch handler's close.
    fc.emit(Op::PushFalse);
    fc.emit_u16(Op::SetLocal, close_flag_slot);
    fc.emit(Op::Pop);
    fc.emit(Op::IteratorNext); // [iterator value done]
    let exit_patch = fc.emit_jump(Op::JumpIfTrue); // if done, exit
                                                   // IteratorNext succeeded → arm close_flag for any
                                                   // subsequent abrupt (e.g. body throw).
    fc.emit(Op::PushTrue);
    fc.emit_u16(Op::SetLocal, close_flag_slot);
    fc.emit(Op::Pop);
    // Bind `left` to value (value is on stack).
    compile_forin_left_binding(fc, prog, analysis, func_scopes, left)?;
    compile_stmt(fc, prog, analysis, func_scopes, body)?;
    // Patch continue jumps to loop_start.
    fc.patch_continue_jumps_to(loop_start);
    fc.emit_jump_to(Op::Jump, loop_start);
    fc.patch_jump(exit_patch);
    fc.emit(Op::PopExceptionHandler);
    fc.emit(Op::Pop); // pop leftover value from done path
    fc.emit(Op::Pop); // normal exhaustion: discard iterator without calling .return()
    let end_patch = fc.emit_jump(Op::Jump); // jump over catch handler

    // Catch handler: gate IteratorClose on close_flag, then
    // re-throw the original exception.  If IteratorClose
    // itself throws, that new error correctly takes
    // precedence over the original abrupt completion per
    // §7.4.11 (a throw from Op::IteratorClose skips the
    // re-throw below).
    let catch_ip = fc.pc();
    fc.emit_u16(Op::GetLocal, close_flag_slot);
    let skip_close = fc.emit_jump(Op::JumpIfFalse);
    fc.emit_u16(Op::GetLocal, iter_slot);
    fc.emit(Op::IteratorClose);
    fc.patch_jump(skip_close);
    fc.emit(Op::PushException);
    fc.emit(Op::Throw);

    fc.patch_jump(end_patch);

    // Patch the exception handler: catch_ip = catch handler,
    // no finally_ip.
    fc.patch_exception_handler(handler_patch_pos, Some(catch_ip), None);

    fc.pop_loop();
    fc.current_scope_idx = saved_scope;
    Ok(())
}

/// `while (test) body` — ECMA-262 §14.7.3.
pub(super) fn compile_while(
    fc: &mut FunctionCompiler,
    prog: &Program,
    analysis: &ScopeAnalysis,
    func_scopes: &mut [FunctionScope],
    test: NodeId<Expr>,
    body: NodeId<Stmt>,
) -> Result<(), CompileError> {
    let loop_start = fc.pc();
    fc.push_loop(loop_start);

    compile_expr(fc, prog, analysis, func_scopes, test)?;
    let exit_patch = fc.emit_jump(Op::JumpIfFalse);

    compile_stmt(fc, prog, analysis, func_scopes, body)?;

    // Patch continue jumps to loop_start (test re-evaluation).
    fc.patch_continue_jumps_to(loop_start);

    fc.emit_jump_to(Op::Jump, loop_start);
    fc.patch_jump(exit_patch);
    fc.pop_loop();
    Ok(())
}

/// `do body while (test)` — ECMA-262 §14.7.2.
pub(super) fn compile_do_while(
    fc: &mut FunctionCompiler,
    prog: &Program,
    analysis: &ScopeAnalysis,
    func_scopes: &mut [FunctionScope],
    body: NodeId<Stmt>,
    test: NodeId<Expr>,
) -> Result<(), CompileError> {
    let loop_start = fc.pc();
    // continue_target is a placeholder; actual continue jumps are
    // collected via continue_patches and patched to the test PC.
    fc.push_loop(loop_start);

    compile_stmt(fc, prog, analysis, func_scopes, body)?;

    // Patch continue jumps to here (the test evaluation).
    fc.patch_continue_jumps();

    compile_expr(fc, prog, analysis, func_scopes, test)?;
    fc.emit_jump_to(Op::JumpIfTrue, loop_start);
    fc.pop_loop();
    Ok(())
}

/// `for (init; test; update) body` — ECMA-262 §14.7.4.
#[allow(clippy::too_many_arguments)]
pub(super) fn compile_for(
    fc: &mut FunctionCompiler,
    prog: &Program,
    analysis: &ScopeAnalysis,
    func_scopes: &mut [FunctionScope],
    init: Option<&ForInit>,
    test: Option<NodeId<Expr>>,
    update: Option<NodeId<Expr>>,
    body: NodeId<Stmt>,
    span: Span,
) -> Result<(), CompileError> {
    let saved_scope = fc.current_scope_idx;
    if let Some(child_scope) = find_child_block_scope(analysis, fc.current_scope_idx, span) {
        fc.current_scope_idx = child_scope;
    }
    // Init.
    if let Some(for_init) = init {
        match for_init {
            ForInit::Declaration { kind, declarators } => {
                for decl in declarators {
                    if let Some(init_expr) = decl.init {
                        compile_expr(fc, prog, analysis, func_scopes, init_expr)?;
                    } else if *kind == VarKind::Var {
                        // var without init: already undefined at function entry.
                        continue;
                    } else {
                        // let/const without init: push undefined to exit TDZ.
                        fc.emit(Op::PushUndefined);
                    }
                    let pattern = prog.patterns.get(decl.pattern);
                    match &pattern.kind {
                        PatternKind::Identifier(atom) => {
                            compile_pattern_store(fc, prog, analysis, func_scopes, *atom, *kind)?;
                        }
                        _ => {
                            compile_destructure_pattern(
                                fc,
                                prog,
                                analysis,
                                func_scopes,
                                decl.pattern,
                                *kind,
                            )?;
                        }
                    }
                }
            }
            ForInit::Expression(e) => {
                compile_expr(fc, prog, analysis, func_scopes, *e)?;
                fc.emit(Op::Pop);
            }
        }
    }

    let loop_start = fc.pc();
    // continue_target is a placeholder; actual continue jumps are
    // collected via continue_patches and patched before the update.
    fc.push_loop(loop_start);

    // Test.
    let exit_patch = if let Some(test_expr) = test {
        compile_expr(fc, prog, analysis, func_scopes, test_expr)?;
        Some(fc.emit_jump(Op::JumpIfFalse))
    } else {
        None
    };

    // Body.
    compile_stmt(fc, prog, analysis, func_scopes, body)?;

    // Patch continue jumps to here (before update expression).
    fc.patch_continue_jumps();

    // Update.
    if let Some(update_expr) = update {
        compile_expr(fc, prog, analysis, func_scopes, update_expr)?;
        fc.emit(Op::Pop);
    }

    fc.emit_jump_to(Op::Jump, loop_start);
    if let Some(patch) = exit_patch {
        fc.patch_jump(patch);
    }
    fc.pop_loop();
    fc.current_scope_idx = saved_scope;
    Ok(())
}

/// Compile the left-hand-side binding for `for-in` / `for-of`.
///
/// Expects the iteration value (key or element) on top of stack.
/// After this function, the value is consumed (popped).
fn compile_forin_left_binding(
    fc: &mut FunctionCompiler,
    prog: &Program,
    analysis: &ScopeAnalysis,
    func_scopes: &mut [FunctionScope],
    left: &ForInOfLeft,
) -> Result<(), CompileError> {
    match left {
        ForInOfLeft::Declaration { kind, pattern } => {
            let pat = prog.patterns.get(*pattern);
            if let PatternKind::Identifier(atom) = &pat.kind {
                compile_pattern_store(fc, prog, analysis, func_scopes, *atom, *kind)?;
            } else {
                compile_destructure_pattern(fc, prog, analysis, func_scopes, *pattern, *kind)?;
            }
        }
        ForInOfLeft::Pattern(expr_id) => {
            // Assignment target (e.g. `for (x in obj)`).
            let expr = prog.exprs.get(*expr_id);
            if let ExprKind::Identifier(atom) = &expr.kind {
                // Store the value to the identifier.
                let loc = super::resolve::resolve_identifier(
                    *atom,
                    fc.func_scope_idx,
                    fc.current_scope_idx,
                    func_scopes,
                    analysis,
                );
                match loc {
                    super::resolve::VarLocation::Local(slot) => {
                        fc.emit_u16(Op::SetLocal, slot);
                        fc.emit(Op::Pop);
                    }
                    super::resolve::VarLocation::Upvalue(idx) => {
                        fc.emit_u16(Op::SetUpvalue, idx);
                        fc.emit(Op::Pop);
                    }
                    super::resolve::VarLocation::Global => {
                        let name = prog.interner.get(*atom);
                        let idx = fc.add_name_u16(name);
                        fc.emit_u16(Op::SetGlobal, idx);
                        fc.emit(Op::Pop);
                    }
                    super::resolve::VarLocation::Module(_) => {
                        fc.emit(Op::Pop);
                    }
                }
            } else {
                // A for-in/of head is a third *lowering* of an assignment
                // target, not a third set of targets: ECMA-262 §14.7.5.7
                // ForIn/OfBodyEvaluation step 8.g.ii.4.a performs the identical
                // `PutValue(lhsRef, nextValue)` that §13.15.2 step 1.e does.
                // Until it has one, every non-identifier head — `for (o.p in
                // obj)`, `for (this.#x of a)`, `for (super.x of a)`,
                // `for ([a,b] of a)` — is rejected the way the assignment forms
                // reject theirs, rather than discarding the value through
                // `Op::Pop`: that silent no-op is the shape umbrella I-1 bans
                // wherever it appears, and it survived here because
                // admissibility was decided inside each lowering.
                //
                // `unsupported_member_target` supplies the message whenever it
                // recognises the target, so `for (this.#x of a)` reports the
                // same cause as `this.#x = v`.  It returning `None` means only
                // "the *assignment* lowering accepts this shape" — there is no
                // for-in/of member store yet either way.
                //
                // Emitted at the assignment site, not at loop entry: step 8.g
                // runs per iteration, so an empty iterable performs no
                // assignment and must not throw.  The value is consumed first,
                // keeping this function's documented stack effect.
                let message = match &expr.kind {
                    ExprKind::Member {
                        object,
                        property,
                        computed,
                    } => unsupported_member_target(prog, *object, property, *computed).unwrap_or(
                        "assignment to a member target in a for-in/of head is not yet supported",
                    ),
                    _ => "this for-in/of assignment target is not yet supported",
                };
                fc.emit(Op::Pop);
                emit_unsupported(fc, message);
            }
        }
    }
    Ok(())
}
