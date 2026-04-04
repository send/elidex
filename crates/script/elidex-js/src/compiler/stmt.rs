//! Statement compilation: StmtKind → bytecode.

use crate::arena::NodeId;
#[allow(clippy::wildcard_imports)]
use crate::ast::*;
use crate::atom::Atom;
use crate::bytecode::opcode::Op;
use crate::scope::{ScopeAnalysis, ScopeKind};
use crate::span::Span;

use super::expr::{compile_class, compile_expr};
use super::function::FunctionCompiler;
use super::resolve::FunctionScope;
use super::CompileError;

/// Compile a statement.
#[allow(clippy::too_many_lines)]
pub fn compile_stmt(
    fc: &mut FunctionCompiler,
    prog: &Program,
    analysis: &ScopeAnalysis,
    func_scopes: &mut [FunctionScope],
    stmt_id: NodeId<Stmt>,
) -> Result<(), CompileError> {
    let stmt = prog.stmts.get(stmt_id);
    let span = stmt.span;
    fc.source_map.add(fc.pc(), span);

    match &stmt.kind {
        // No-ops: empty, parse error, debugger, stubs, hoisted function declarations.
        StmtKind::Empty
        | StmtKind::Error
        | StmtKind::Debugger
        | StmtKind::ImportDeclaration(_)
        | StmtKind::ExportDeclaration(_)
        | StmtKind::FunctionDeclaration(_) => {
            // FunctionDeclaration: hoisted at function/script level
            // (mod.rs compile() or compile_nested_function). No-op at declaration site.
        }

        StmtKind::ClassDeclaration(class) => {
            compile_class(fc, prog, analysis, func_scopes, class)?;
            // Store the class (constructor) in its binding.
            if let Some(name) = &class.name {
                let loc = super::resolve::resolve_identifier(
                    *name,
                    fc.func_scope_idx,
                    fc.current_scope_idx,
                    func_scopes,
                    analysis,
                );
                match loc {
                    super::resolve::VarLocation::Local(slot) => {
                        // Clear TDZ for the outer class declaration binding.
                        fc.emit_u16(Op::InitLocal, slot);
                        fc.emit_u16(Op::SetLocal, slot);
                        fc.emit(Op::Pop);
                    }
                    super::resolve::VarLocation::Global => {
                        let name_str = prog.interner.get(*name);
                        let name_idx = fc.add_name(name_str);
                        fc.emit_u16(Op::SetGlobal, name_idx);
                        fc.emit(Op::Pop);
                    }
                    _ => {
                        fc.emit(Op::Pop);
                    }
                }
            } else {
                fc.emit(Op::Pop);
            }
        }

        StmtKind::ForIn { left, right, body } => {
            let saved_scope = fc.current_scope_idx;
            if let Some(child_scope) = find_child_block_scope(analysis, fc.current_scope_idx, span)
            {
                fc.current_scope_idx = child_scope;
            }
            compile_expr(fc, prog, analysis, func_scopes, *right)?;
            fc.emit(Op::ForInIterator);
            let loop_start = fc.pc();
            fc.push_loop(loop_start);
            fc.emit(Op::ForInNext); // [iterator key done]
            let exit_patch = fc.emit_jump(Op::JumpIfTrue); // if done, exit
                                                           // Bind `left` to key (key is on stack).
            compile_forin_left_binding(fc, prog, analysis, func_scopes, left)?;
            compile_stmt(fc, prog, analysis, func_scopes, *body)?;
            // Patch continue jumps to loop_start.
            fc.patch_continue_jumps_to(loop_start);
            fc.emit_jump_to(Op::Jump, loop_start);
            fc.patch_jump(exit_patch);
            fc.emit(Op::Pop); // pop leftover key from done path
            fc.emit(Op::Pop); // pop iterator
            fc.pop_loop();
            fc.current_scope_idx = saved_scope;
        }

        StmtKind::ForOf {
            left,
            right,
            body,
            is_await: _,
        } => {
            let saved_scope = fc.current_scope_idx;
            if let Some(child_scope) = find_child_block_scope(analysis, fc.current_scope_idx, span)
            {
                fc.current_scope_idx = child_scope;
            }
            compile_expr(fc, prog, analysis, func_scopes, *right)?;
            fc.emit(Op::GetIterator);
            let loop_start = fc.pc();
            fc.push_loop(loop_start);
            fc.emit(Op::IteratorNext); // [iterator value done]
            let exit_patch = fc.emit_jump(Op::JumpIfTrue); // if done, exit
                                                           // Bind `left` to value (value is on stack).
            compile_forin_left_binding(fc, prog, analysis, func_scopes, left)?;
            compile_stmt(fc, prog, analysis, func_scopes, *body)?;
            // Patch continue jumps to loop_start.
            fc.patch_continue_jumps_to(loop_start);
            fc.emit_jump_to(Op::Jump, loop_start);
            fc.patch_jump(exit_patch);
            fc.emit(Op::Pop); // pop leftover value from done path
            fc.emit(Op::Pop); // pop iterator
            fc.pop_loop();
            fc.current_scope_idx = saved_scope;
        }

        StmtKind::With { .. } => {
            return Err(CompileError {
                message: "with statement is not supported (strict mode / ADR #2)".into(),
            });
        }

        StmtKind::Expression(expr_id) => {
            compile_expr(fc, prog, analysis, func_scopes, *expr_id)?;
            fc.emit(Op::Pop); // statement expressions discard their value
        }

        StmtKind::VariableDeclaration { kind, declarators } => {
            for decl in declarators {
                if let Some(init) = decl.init {
                    compile_expr(fc, prog, analysis, func_scopes, init)?;
                } else if *kind == VarKind::Var {
                    // var without init: already initialized to undefined at function entry.
                    continue;
                } else {
                    // let/const without init: push undefined (will be assigned).
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

        StmtKind::Block(body) => {
            let saved_scope = fc.current_scope_idx;
            if let Some(child_scope) = find_child_block_scope(analysis, fc.current_scope_idx, span)
            {
                fc.current_scope_idx = child_scope;
            }
            for &s in body {
                compile_stmt(fc, prog, analysis, func_scopes, s)?;
            }
            fc.current_scope_idx = saved_scope;
        }

        StmtKind::If {
            test,
            consequent,
            alternate,
        } => {
            compile_expr(fc, prog, analysis, func_scopes, *test)?;
            let else_patch = fc.emit_jump(Op::JumpIfFalse);

            compile_stmt(fc, prog, analysis, func_scopes, *consequent)?;

            if let Some(alt) = alternate {
                let end_patch = fc.emit_jump(Op::Jump);
                fc.patch_jump(else_patch);
                compile_stmt(fc, prog, analysis, func_scopes, *alt)?;
                fc.patch_jump(end_patch);
            } else {
                fc.patch_jump(else_patch);
            }
        }

        StmtKind::While { test, body } => {
            let loop_start = fc.pc();
            fc.push_loop(loop_start);

            compile_expr(fc, prog, analysis, func_scopes, *test)?;
            let exit_patch = fc.emit_jump(Op::JumpIfFalse);

            compile_stmt(fc, prog, analysis, func_scopes, *body)?;

            // Patch continue jumps to loop_start (test re-evaluation).
            fc.patch_continue_jumps_to(loop_start);

            fc.emit_jump_to(Op::Jump, loop_start);
            fc.patch_jump(exit_patch);
            fc.pop_loop();
        }

        StmtKind::DoWhile { body, test } => {
            let loop_start = fc.pc();
            // continue_target is a placeholder; actual continue jumps are
            // collected via continue_patches and patched to the test PC.
            fc.push_loop(loop_start);

            compile_stmt(fc, prog, analysis, func_scopes, *body)?;

            // Patch continue jumps to here (the test evaluation).
            fc.patch_continue_jumps();

            compile_expr(fc, prog, analysis, func_scopes, *test)?;
            fc.emit_jump_to(Op::JumpIfTrue, loop_start);
            fc.pop_loop();
        }

        StmtKind::For {
            init,
            test,
            update,
            body,
        } => {
            let saved_scope = fc.current_scope_idx;
            if let Some(child_scope) = find_child_block_scope(analysis, fc.current_scope_idx, span)
            {
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
                                    compile_pattern_store(
                                        fc,
                                        prog,
                                        analysis,
                                        func_scopes,
                                        *atom,
                                        *kind,
                                    )?;
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
                compile_expr(fc, prog, analysis, func_scopes, *test_expr)?;
                Some(fc.emit_jump(Op::JumpIfFalse))
            } else {
                None
            };

            // Body.
            compile_stmt(fc, prog, analysis, func_scopes, *body)?;

            // Patch continue jumps to here (before update expression).
            fc.patch_continue_jumps();

            // Update.
            if let Some(update_expr) = update {
                compile_expr(fc, prog, analysis, func_scopes, *update_expr)?;
                fc.emit(Op::Pop);
            }

            fc.emit_jump_to(Op::Jump, loop_start);
            if let Some(patch) = exit_patch {
                fc.patch_jump(patch);
            }
            fc.pop_loop();
            fc.current_scope_idx = saved_scope;
        }

        StmtKind::Return(arg) => {
            if let Some(expr_id) = arg {
                compile_expr(fc, prog, analysis, func_scopes, *expr_id)?;
            } else {
                fc.emit(Op::PushUndefined);
            }
            // If inside try/finally, emit finally bodies before returning.
            // The return value is on the stack; finally bodies must not consume it.
            emit_pending_finally_bodies(fc, prog, analysis, func_scopes)?;
            fc.emit(Op::Return);
        }

        StmtKind::Throw(expr_id) => {
            compile_expr(fc, prog, analysis, func_scopes, *expr_id)?;
            fc.emit(Op::Throw);
        }

        StmtKind::Break(label) => {
            emit_pending_finally_bodies(fc, prog, analysis, func_scopes)?;
            let patch = fc.emit_jump(Op::Jump);
            if let Some(label_atom) = label {
                let label_name = prog.interner.get(*label_atom);
                if let Some(&loop_idx) = fc.label_map.get(label_atom) {
                    if loop_idx >= fc.loop_stack.len() {
                        return Err(CompileError {
                            message: format!(
                                "label '{label_name}' is not associated with a loop or switch"
                            ),
                        });
                    }
                    fc.loop_stack[loop_idx].break_patches.push(patch);
                } else {
                    return Err(CompileError {
                        message: format!("undefined label '{label_name}'"),
                    });
                }
            } else {
                fc.add_break_patch(patch);
            }
        }

        StmtKind::Continue(label) => {
            emit_pending_finally_bodies(fc, prog, analysis, func_scopes)?;
            let patch = fc.emit_jump(Op::Jump);
            if let Some(label_atom) = label {
                let label_name = prog.interner.get(*label_atom);
                if let Some(&loop_idx) = fc.label_map.get(label_atom) {
                    if loop_idx >= fc.loop_stack.len() {
                        return Err(CompileError {
                            message: format!("label '{label_name}' is not associated with a loop"),
                        });
                    }
                    fc.loop_stack[loop_idx].continue_patches.push(patch);
                } else {
                    return Err(CompileError {
                        message: format!("undefined label '{label_name}'"),
                    });
                }
            } else {
                fc.add_continue_patch(patch);
            }
        }

        StmtKind::Labeled { label, body } => {
            // Map label to the current loop stack depth so that labeled
            // break/continue can target the loop that follows.
            let loop_depth = fc.loop_stack.len();
            fc.label_map.insert(*label, loop_depth);
            compile_stmt(fc, prog, analysis, func_scopes, *body)?;
            fc.label_map.remove(label);
        }

        StmtKind::Try {
            block,
            handler,
            finalizer,
        } => {
            let catch_placeholder = fc.pc();
            // PushExceptionHandler with placeholder offsets.
            fc.emit_u16_u16(Op::PushExceptionHandler, 0, 0);
            let handler_patch_pos = catch_placeholder + 1; // offset of catch u16

            // Push finally body onto the stack so that return/break/continue
            // inside the try block will emit it before jumping.
            if let Some(fin_block) = finalizer {
                fc.finally_stack.push(fin_block.clone());
            }

            // Try block (has its own Block scope in the scope tree).
            let saved_try_scope = fc.current_scope_idx;
            if let Some(try_block_scope) =
                find_child_block_scope(analysis, fc.current_scope_idx, span)
            {
                fc.current_scope_idx = try_block_scope;
            }
            for &s in block {
                compile_stmt(fc, prog, analysis, func_scopes, s)?;
            }
            fc.current_scope_idx = saved_try_scope;
            fc.emit(Op::PopExceptionHandler);

            // ── try/finally without catch: two-copy layout ──────────
            //
            //   PushExceptionHandler(catch=rethrow_entry, finally=0xFFFF)
            //   <try body>
            //   PopExceptionHandler
            //   <finally body>          ← normal path
            //   Jump(end)
            //   rethrow_entry:          ← exception path
            //   PushException
            //   <finally body>          (duplicate)
            //   Throw                   ← re-throw the saved exception
            //   end:
            //
            // When a catch block is present the original layout is kept:
            // exception → catch → finally (shared).

            if handler.is_none() && finalizer.is_some() {
                // Normal path: run finally body, then jump over rethrow.
                let fin_block = finalizer.as_ref().unwrap();
                for &s in fin_block {
                    compile_stmt(fc, prog, analysis, func_scopes, s)?;
                }
                let end_jump = fc.emit_jump(Op::Jump);

                // Exception path: PushException → finally body → Throw.
                let rethrow_entry = fc.pc();
                fc.emit(Op::PushException);
                for &s in fin_block {
                    compile_stmt(fc, prog, analysis, func_scopes, s)?;
                }
                fc.emit(Op::Throw);

                fc.patch_jump(end_jump);

                // Patch handler: catch_ip = rethrow_entry, finally_ip = 0xFFFF.
                assert!(
                    u16::try_from(rethrow_entry).is_ok(),
                    "rethrow entry {rethrow_entry} exceeds u16 range"
                );
                let catch_bytes = (rethrow_entry as u16).to_le_bytes();
                fc.bytecode[handler_patch_pos as usize] = catch_bytes[0];
                fc.bytecode[(handler_patch_pos + 1) as usize] = catch_bytes[1];
                let finally_bytes = 0xFFFFu16.to_le_bytes();
                fc.bytecode[(handler_patch_pos + 2) as usize] = finally_bytes[0];
                fc.bytecode[(handler_patch_pos + 3) as usize] = finally_bytes[1];
            } else {
                // Original layout for try/catch, try/catch/finally.
                let finally_jump = fc.emit_jump(Op::Jump); // jump over catch

                // Catch block.
                let catch_offset = if handler.is_some() {
                    fc.pc()
                } else {
                    u32::MAX // no catch, no finally — shouldn't normally happen
                };
                // If there's a finally block, we need a second exception handler
                // around the catch body so that throws/returns inside catch
                // still execute finally.
                let catch_rethrow_patch = if handler.is_some() && finalizer.is_some() {
                    let placeholder = fc.pc();
                    fc.emit_u16_u16(Op::PushExceptionHandler, 0, 0);
                    Some(placeholder + 1) // offset of catch u16 in this handler
                } else {
                    None
                };

                if let Some(catch) = handler {
                    let saved_scope = fc.current_scope_idx;
                    if let Some(catch_scope) =
                        find_child_catch_scope(analysis, fc.current_scope_idx, catch.span)
                    {
                        fc.current_scope_idx = catch_scope;
                    }
                    // Bind catch parameter.
                    fc.emit(Op::PushException);
                    if let Some(param_id) = catch.param {
                        let pattern = prog.patterns.get(param_id);
                        if let PatternKind::Identifier(atom) = &pattern.kind {
                            let loc = super::resolve::resolve_identifier(
                                *atom,
                                fc.func_scope_idx,
                                fc.current_scope_idx,
                                func_scopes,
                                analysis,
                            );
                            if let super::resolve::VarLocation::Local(slot) = loc {
                                fc.emit_u16(Op::SetLocal, slot);
                            }
                        }
                    }
                    fc.emit(Op::Pop); // pop exception from stack

                    for &s in &catch.body {
                        compile_stmt(fc, prog, analysis, func_scopes, s)?;
                    }
                    fc.current_scope_idx = saved_scope;
                }

                // Pop the catch-body exception handler (if we pushed one).
                if catch_rethrow_patch.is_some() {
                    fc.emit(Op::PopExceptionHandler);
                }

                fc.patch_jump(finally_jump);

                // Finally block (if present).
                let finally_pc = if finalizer.is_some() {
                    Some(fc.pc())
                } else {
                    None
                };
                if let Some(fin_block) = finalizer {
                    for &s in fin_block {
                        compile_stmt(fc, prog, analysis, func_scopes, s)?;
                    }
                }

                // Patch the exception handler offsets.
                let catch_u16 = if catch_offset == u32::MAX {
                    0xFFFFu16
                } else {
                    assert!(
                        u16::try_from(catch_offset).is_ok(),
                        "catch offset {catch_offset} exceeds u16 range"
                    );
                    catch_offset as u16
                };
                let catch_bytes = catch_u16.to_le_bytes();
                fc.bytecode[handler_patch_pos as usize] = catch_bytes[0];
                fc.bytecode[(handler_patch_pos + 1) as usize] = catch_bytes[1];
                let finally_offset = if let Some(fpc) = finally_pc {
                    assert!(
                        u16::try_from(fpc).is_ok(),
                        "finally offset {fpc} exceeds u16 range"
                    );
                    fpc as u16
                } else {
                    0xFFFF
                };
                let finally_bytes = finally_offset.to_le_bytes();
                fc.bytecode[(handler_patch_pos + 2) as usize] = finally_bytes[0];
                fc.bytecode[(handler_patch_pos + 3) as usize] = finally_bytes[1];

                // Patch the catch-body exception handler (if present).
                // This handler routes exceptions thrown inside `catch` to a
                // rethrow stub that runs `finally` body then re-throws.
                if let Some(patch_pos) = catch_rethrow_patch {
                    let end_jump = fc.emit_jump(Op::Jump); // skip rethrow stub on normal path

                    let rethrow_entry = fc.pc();
                    fc.emit(Op::PushException);
                    if let Some(fin_block) = finalizer {
                        for &s in fin_block {
                            compile_stmt(fc, prog, analysis, func_scopes, s)?;
                        }
                    }
                    fc.emit(Op::Throw);

                    fc.patch_jump(end_jump);

                    // Patch: catch_ip = rethrow_entry, finally_ip = 0xFFFF
                    assert!(
                        u16::try_from(rethrow_entry).is_ok(),
                        "catch-body rethrow entry {rethrow_entry} exceeds u16 range"
                    );
                    let rethrow_bytes = (rethrow_entry as u16).to_le_bytes();
                    fc.bytecode[patch_pos as usize] = rethrow_bytes[0];
                    fc.bytecode[(patch_pos + 1) as usize] = rethrow_bytes[1];
                    let no_finally = 0xFFFFu16.to_le_bytes();
                    fc.bytecode[(patch_pos + 2) as usize] = no_finally[0];
                    fc.bytecode[(patch_pos + 3) as usize] = no_finally[1];
                }
            }

            // Pop the finally stack entry (if we pushed one).
            if finalizer.is_some() {
                fc.finally_stack.pop();
            }
        }

        StmtKind::Switch {
            discriminant,
            cases,
        } => {
            compile_expr(fc, prog, analysis, func_scopes, *discriminant)?;

            // First pass: emit tests and conditional jumps.
            //
            // For each case with a test:
            //   Dup discriminant    [disc disc]
            //   PushConst(case_val) [disc disc case_val]
            //   StrictEq            [disc bool]
            //   JumpIfTrue → entry  [disc]       (pops bool; if true, jumps to case entry)
            //                                     (if false, falls through to next test)
            //
            // After all tests, jump to default or end (discriminant on stack).
            let mut case_entry_patches: Vec<u32> = Vec::new();
            let mut has_default = false;
            let mut default_idx: usize = 0;

            for (i, case) in cases.iter().enumerate() {
                if let Some(test) = case.test {
                    fc.emit(Op::Dup); // keep discriminant
                    compile_expr(fc, prog, analysis, func_scopes, test)?;
                    fc.emit(Op::StrictEq);
                    let patch = fc.emit_jump(Op::JumpIfTrue);
                    case_entry_patches.push(patch);
                } else {
                    has_default = true;
                    default_idx = i;
                    case_entry_patches.push(0); // not used for jump
                }
            }

            // No case matched: jump to default or end.
            let no_match_jump = fc.emit_jump(Op::Jump);

            // Second pass: emit case bodies.
            //
            // Each case entry point (jumped to from the test phase) pops
            // the discriminant, then falls through to the body statements.
            // Fall-through from a previous case body goes directly to the
            // body statements (skipping the Pop), which is why we use a
            // jump-over pattern: entry → Pop disc → Jump over skip →
            // [fall-through entry] → body statements.
            //
            // Simplified: each case emits a "Pop + body". The test jump
            // lands at the Pop. Fall-through from prev case jumps over
            // the Pop to the body.
            fc.push_switch(); // for break support (continue skips switch contexts)
            let mut fallthrough_patches: Vec<u32> = Vec::new();
            let mut entry_pcs: Vec<u32> = Vec::new();

            for (i, case) in cases.iter().enumerate() {
                // Record the entry PC (where test-match jumps land).
                let entry_pc = fc.pc();
                entry_pcs.push(entry_pc);

                // Patch the test jump for this case.
                if case.test.is_some() {
                    fc.patch_jump(case_entry_patches[i]);
                }

                // Pop discriminant (only reached from test jump, not fall-through).
                fc.emit(Op::Pop);

                // Patch fall-through from previous case body to skip the Pop.
                for ft_patch in fallthrough_patches.drain(..) {
                    fc.patch_jump(ft_patch);
                }

                // Emit body statements.
                for &s in &case.consequent {
                    compile_stmt(fc, prog, analysis, func_scopes, s)?;
                }

                // If there's a next case, emit a jump placeholder for
                // fall-through (to skip the next case's Pop).
                if i + 1 < cases.len() {
                    let ft = fc.emit_jump(Op::Jump);
                    fallthrough_patches.push(ft);
                }
            }

            // If the last case body falls through (no break) and there's no
            // default, it would reach the trailing Pop intended only for the
            // no-match path. Emit a jump to skip it.
            let end_fallthrough = if has_default {
                None
            } else {
                Some(fc.emit_jump(Op::Jump))
            };

            // Patch the no-match jump.
            if has_default {
                let default_pc = entry_pcs[default_idx];
                // Manually patch to default_pc (not current PC) using assert-checked offset.
                #[allow(clippy::cast_possible_wrap)]
                let offset = (default_pc as i32) - (no_match_jump as i32) - 2;
                assert!(
                    (i32::from(i16::MIN)..=i32::from(i16::MAX)).contains(&offset),
                    "switch default jump offset {offset} out of i16 range"
                );
                let bytes = (offset as i16).to_le_bytes();
                fc.bytecode[no_match_jump as usize] = bytes[0];
                fc.bytecode[(no_match_jump + 1) as usize] = bytes[1];
            } else {
                fc.patch_jump(no_match_jump);
                fc.emit(Op::Pop); // pop discriminant (no case matched)
            }
            // Patch the end-of-switch jump for last-case fall-through.
            if let Some(patch) = end_fallthrough {
                fc.patch_jump(patch);
            }
            fc.pop_loop();
        }
    }

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
                        let idx = fc.add_name(name);
                        fc.emit_u16(Op::SetGlobal, idx);
                        fc.emit(Op::Pop);
                    }
                    super::resolve::VarLocation::Module(_) => {
                        fc.emit(Op::Pop);
                    }
                }
            } else {
                // Complex LHS (e.g. `for (obj.prop in ...)`) — not yet supported.
                fc.emit(Op::Pop);
            }
        }
    }
    Ok(())
}

/// Compile a destructuring pattern.
///
/// Assumes the value to destructure is on top of the stack.
/// After compilation, the value is consumed (popped).
#[allow(clippy::too_many_lines)]
fn compile_destructure_pattern(
    fc: &mut FunctionCompiler,
    prog: &Program,
    analysis: &ScopeAnalysis,
    func_scopes: &mut [FunctionScope],
    pattern_id: NodeId<Pattern>,
    var_kind: VarKind,
) -> Result<(), CompileError> {
    let pattern = prog.patterns.get(pattern_id);
    match &pattern.kind {
        PatternKind::Identifier(atom) => {
            compile_pattern_store(fc, prog, analysis, func_scopes, *atom, var_kind)?;
        }
        PatternKind::Array { elements, rest } => {
            // Stack: [value]
            fc.emit(Op::GetIterator); // [iterator]

            for elem in elements {
                if let Some(ArrayPatternElement {
                    pattern: p,
                    default: def,
                }) = elem
                {
                    fc.emit(Op::IteratorNext); // [iterator value done]
                    fc.emit(Op::Pop); // [iterator value]

                    if let Some(default_expr) = def {
                        // ES2020: default triggers only on undefined, not null.
                        fc.emit(Op::Dup); // [iterator value value]
                        fc.emit(Op::PushUndefined); // [iterator value value undefined]
                        fc.emit(Op::StrictNotEq); // [iterator value bool]
                        let skip = fc.emit_jump(Op::JumpIfTrue); // [iterator value]
                        fc.emit(Op::Pop); // [iterator]
                        compile_expr(fc, prog, analysis, func_scopes, *default_expr)?;
                        fc.patch_jump(skip); // [iterator value_or_default]
                    }

                    compile_destructure_pattern(fc, prog, analysis, func_scopes, *p, var_kind)?;
                } else {
                    // Elision / hole: skip one iterator value.
                    fc.emit(Op::IteratorNext); // [iterator value done]
                    fc.emit(Op::Pop); // [iterator value]
                    fc.emit(Op::Pop); // [iterator]
                }
            }

            if let Some(rest_pattern) = rest {
                fc.emit(Op::IteratorRest); // [rest_array]
                compile_destructure_pattern(
                    fc,
                    prog,
                    analysis,
                    func_scopes,
                    *rest_pattern,
                    var_kind,
                )?;
            } else {
                fc.emit(Op::Pop); // pop iterator
            }
        }
        PatternKind::Object { properties, rest } => {
            // Stack: [value]
            for prop in properties {
                fc.emit(Op::Dup); // [value value]
                match &prop.key {
                    PropertyKey::Identifier(atom) => {
                        let name = prog.interner.get(*atom);
                        let idx = fc.add_name(name);
                        fc.emit_u16(Op::GetProp, idx); // [value prop_value]
                    }
                    PropertyKey::Literal(Literal::String(atom)) => {
                        let name = prog.interner.get(*atom);
                        let idx = fc.add_name(name);
                        fc.emit_u16(Op::GetProp, idx);
                    }
                    PropertyKey::Computed(expr) => {
                        compile_expr(fc, prog, analysis, func_scopes, *expr)?;
                        fc.emit(Op::GetElem);
                    }
                    _ => {
                        fc.emit(Op::PushUndefined);
                    }
                }
                // Stack: [value prop_value]
                compile_destructure_pattern(fc, prog, analysis, func_scopes, prop.value, var_kind)?;
            }

            if let Some(rest_pat) = rest {
                // Create a new object with all enumerable own properties EXCEPT
                // the keys already destructured above.
                fc.emit(Op::Dup); // [value value]
                fc.emit(Op::CreateObject); // [value value rest_obj]
                fc.emit(Op::Swap); // [value rest_obj value]
                fc.emit(Op::SpreadObject); // [value rest_obj] (copies all props)
                                           // Delete the already-destructured keys from rest_obj.
                for prop in properties {
                    match &prop.key {
                        PropertyKey::Identifier(atom) => {
                            let name = prog.interner.get(*atom);
                            let idx = fc.add_name(name);
                            fc.emit(Op::Dup); // [value rest_obj rest_obj]
                            fc.emit_u16(Op::DeleteProp, idx); // [value rest_obj bool]
                            fc.emit(Op::Pop); // [value rest_obj]
                        }
                        PropertyKey::Literal(Literal::String(atom)) => {
                            let name = prog.interner.get(*atom);
                            let idx = fc.add_name(name);
                            fc.emit(Op::Dup);
                            fc.emit_u16(Op::DeleteProp, idx);
                            fc.emit(Op::Pop);
                        }
                        // Computed/other keys: can't statically delete, skip
                        _ => {}
                    }
                }
                // Stack: [value rest_obj]
                compile_destructure_pattern(fc, prog, analysis, func_scopes, *rest_pat, var_kind)?;
            }

            fc.emit(Op::Pop); // pop original object
        }
        PatternKind::Assign { left, right } => {
            // Stack: [value]
            // ES2020: default triggers only on undefined, not null.
            fc.emit(Op::Dup); // [value value]
            fc.emit(Op::PushUndefined); // [value value undefined]
            fc.emit(Op::StrictNotEq); // [value bool]
            let skip = fc.emit_jump(Op::JumpIfTrue); // [value]
            fc.emit(Op::Pop); // discard undefined
            compile_expr(fc, prog, analysis, func_scopes, *right)?;
            fc.patch_jump(skip);
            // Stack: [actual_value_or_default]
            compile_destructure_pattern(fc, prog, analysis, func_scopes, *left, var_kind)?;
        }
        PatternKind::Expression(_) | PatternKind::Error => {
            fc.emit(Op::Pop);
        }
    }
    Ok(())
}

/// Store a value (on top of stack) to a variable binding.
#[allow(clippy::unnecessary_wraps)] // Result kept for consistency with other compile_* fns
fn compile_pattern_store(
    fc: &mut FunctionCompiler,
    prog: &Program,
    analysis: &ScopeAnalysis,
    func_scopes: &mut [FunctionScope],
    atom: Atom,
    kind: VarKind,
) -> Result<(), CompileError> {
    let loc = super::resolve::resolve_identifier(
        atom,
        fc.func_scope_idx,
        fc.current_scope_idx,
        func_scopes,
        analysis,
    );
    match loc {
        super::resolve::VarLocation::Local(slot) => {
            // For let/const, mark as initialized.
            if matches!(kind, VarKind::Let | VarKind::Const) {
                fc.emit_u16(Op::InitLocal, slot);
            }
            fc.emit_u16(Op::SetLocal, slot);
            fc.emit(Op::Pop); // discard value (statement, not expression)
        }
        super::resolve::VarLocation::Upvalue(idx) => {
            fc.emit_u16(Op::SetUpvalue, idx);
            fc.emit(Op::Pop);
        }
        super::resolve::VarLocation::Global => {
            let name = prog.interner.get(atom);
            let idx = fc.add_name(name);
            fc.emit_u16(Op::SetGlobal, idx);
            fc.emit(Op::Pop);
        }
        super::resolve::VarLocation::Module(_) => {
            fc.emit(Op::Pop);
        }
    }

    Ok(())
}

/// Find a child scope of `parent_idx` that is a `Block` scope matching `span`.
fn find_child_block_scope(
    analysis: &ScopeAnalysis,
    parent_idx: usize,
    span: Span,
) -> Option<usize> {
    let parent = &analysis.scopes[parent_idx];
    for &child_idx in &parent.children {
        let child = &analysis.scopes[child_idx];
        if child.kind == ScopeKind::Block && child.span == span {
            return Some(child_idx);
        }
    }
    None
}

/// Emit all pending finally bodies (innermost first) before a return/break/continue.
///
/// This ensures `try { return x; } finally { cleanup(); }` runs the cleanup
/// before the return takes effect.
fn emit_pending_finally_bodies(
    fc: &mut FunctionCompiler,
    prog: &Program,
    analysis: &ScopeAnalysis,
    func_scopes: &mut [FunctionScope],
) -> Result<(), CompileError> {
    // Clone the stack to avoid borrow conflict (finally_stack is on fc).
    let stacks: Vec<Vec<NodeId<Stmt>>> = fc.finally_stack.clone();
    // Emit innermost (last pushed) first — but for return semantics,
    // we need innermost-to-outermost order, which is reverse iteration.
    for fin_stmts in stacks.iter().rev() {
        for &s in fin_stmts {
            compile_stmt(fc, prog, analysis, func_scopes, s)?;
        }
    }
    Ok(())
}

/// Find a child scope of `parent_idx` that is a `Catch` scope matching `catch_span`.
fn find_child_catch_scope(
    analysis: &ScopeAnalysis,
    parent_idx: usize,
    catch_span: Span,
) -> Option<usize> {
    let parent = &analysis.scopes[parent_idx];
    for &child_idx in &parent.children {
        let child = &analysis.scopes[child_idx];
        if child.kind == ScopeKind::Catch && child.span == catch_span {
            return Some(child_idx);
        }
    }
    None
}
