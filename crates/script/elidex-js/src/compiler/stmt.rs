//! Statement compilation: StmtKind → bytecode.

use crate::arena::NodeId;
#[allow(clippy::wildcard_imports)]
use crate::ast::*;
use crate::atom::Atom;
use crate::bytecode::opcode::Op;
use crate::scope::ScopeAnalysis;

use super::expr::compile_expr;
use super::function::FunctionCompiler;
use super::resolve::FunctionScope;

/// Compile a statement.
#[allow(clippy::too_many_lines)]
pub fn compile_stmt(
    fc: &mut FunctionCompiler,
    prog: &Program,
    analysis: &ScopeAnalysis,
    func_scopes: &mut [FunctionScope],
    stmt_id: NodeId<Stmt>,
) {
    let stmt = prog.stmts.get(stmt_id);
    let span = stmt.span;
    fc.source_map.add(fc.pc(), span);

    match &stmt.kind {
        // No-ops: empty, parse error, debugger, hoisted declarations, stubs.
        StmtKind::Empty
        | StmtKind::Error
        | StmtKind::Debugger
        | StmtKind::FunctionDeclaration(_)
        | StmtKind::ClassDeclaration(_)
        | StmtKind::With { .. }
        | StmtKind::ForIn { .. }
        | StmtKind::ForOf { .. }
        | StmtKind::ImportDeclaration(_)
        | StmtKind::ExportDeclaration(_) => {
            // TODO: FunctionDeclaration, ClassDeclaration, ForIn/ForOf, With
        }

        StmtKind::Expression(expr_id) => {
            compile_expr(fc, prog, analysis, func_scopes, *expr_id);
            fc.emit(Op::Pop); // statement expressions discard their value
        }

        StmtKind::VariableDeclaration { kind, declarators } => {
            for decl in declarators {
                if let Some(init) = decl.init {
                    compile_expr(fc, prog, analysis, func_scopes, init);
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
                        compile_pattern_store(fc, prog, analysis, func_scopes, *atom, *kind);
                    }
                    // Destructuring patterns not yet supported — pop value to keep stack balanced.
                    _ => {
                        fc.emit(Op::Pop);
                    }
                }
            }
        }

        StmtKind::Block(body) => {
            for &s in body {
                compile_stmt(fc, prog, analysis, func_scopes, s);
            }
        }

        StmtKind::If {
            test,
            consequent,
            alternate,
        } => {
            compile_expr(fc, prog, analysis, func_scopes, *test);
            let else_patch = fc.emit_jump(Op::JumpIfFalse);

            compile_stmt(fc, prog, analysis, func_scopes, *consequent);

            if let Some(alt) = alternate {
                let end_patch = fc.emit_jump(Op::Jump);
                fc.patch_jump(else_patch);
                compile_stmt(fc, prog, analysis, func_scopes, *alt);
                fc.patch_jump(end_patch);
            } else {
                fc.patch_jump(else_patch);
            }
        }

        StmtKind::While { test, body } => {
            let loop_start = fc.pc();
            fc.push_loop(loop_start);

            compile_expr(fc, prog, analysis, func_scopes, *test);
            let exit_patch = fc.emit_jump(Op::JumpIfFalse);

            compile_stmt(fc, prog, analysis, func_scopes, *body);

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

            compile_stmt(fc, prog, analysis, func_scopes, *body);

            // Patch continue jumps to here (the test evaluation).
            fc.patch_continue_jumps();

            compile_expr(fc, prog, analysis, func_scopes, *test);
            fc.emit_jump_to(Op::JumpIfTrue, loop_start);
            fc.pop_loop();
        }

        StmtKind::For {
            init,
            test,
            update,
            body,
        } => {
            // Init.
            if let Some(for_init) = init {
                match for_init {
                    ForInit::Declaration { kind, declarators } => {
                        for decl in declarators {
                            if let Some(init_expr) = decl.init {
                                compile_expr(fc, prog, analysis, func_scopes, init_expr);
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
                                    );
                                }
                                _ => {
                                    fc.emit(Op::Pop);
                                }
                            }
                        }
                    }
                    ForInit::Expression(e) => {
                        compile_expr(fc, prog, analysis, func_scopes, *e);
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
                compile_expr(fc, prog, analysis, func_scopes, *test_expr);
                Some(fc.emit_jump(Op::JumpIfFalse))
            } else {
                None
            };

            // Body.
            compile_stmt(fc, prog, analysis, func_scopes, *body);

            // Patch continue jumps to here (before update expression).
            fc.patch_continue_jumps();

            // Update.
            if let Some(update_expr) = update {
                compile_expr(fc, prog, analysis, func_scopes, *update_expr);
                fc.emit(Op::Pop);
            }

            fc.emit_jump_to(Op::Jump, loop_start);
            if let Some(patch) = exit_patch {
                fc.patch_jump(patch);
            }
            fc.pop_loop();
        }

        StmtKind::Return(arg) => {
            if let Some(expr_id) = arg {
                compile_expr(fc, prog, analysis, func_scopes, *expr_id);
                fc.emit(Op::Return);
            } else {
                fc.emit(Op::ReturnUndefined);
            }
        }

        StmtKind::Throw(expr_id) => {
            compile_expr(fc, prog, analysis, func_scopes, *expr_id);
            fc.emit(Op::Throw);
        }

        StmtKind::Break(label) => {
            if label.is_some() {
                // TODO: labeled break
            }
            let patch = fc.emit_jump(Op::Jump);
            fc.add_break_patch(patch);
        }

        StmtKind::Continue(label) => {
            if label.is_some() {
                // TODO: labeled continue
            }
            // Emit a placeholder jump; it will be patched to the correct
            // continue target (test for do-while, update for for-loops)
            // when patch_continue_jumps() is called.
            let patch = fc.emit_jump(Op::Jump);
            fc.add_continue_patch(patch);
        }

        StmtKind::Labeled { label: _, body } => {
            // TODO: proper labeled statement support
            compile_stmt(fc, prog, analysis, func_scopes, *body);
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

            // Try block.
            for &s in block {
                compile_stmt(fc, prog, analysis, func_scopes, s);
            }
            fc.emit(Op::PopExceptionHandler);
            let finally_jump = fc.emit_jump(Op::Jump); // jump over catch

            // Catch block.
            let catch_offset = fc.pc();
            if let Some(catch) = handler {
                // Bind catch parameter.
                fc.emit(Op::PushException);
                if let Some(param_id) = catch.param {
                    let pattern = prog.patterns.get(param_id);
                    if let PatternKind::Identifier(atom) = &pattern.kind {
                        let loc = super::resolve::resolve_identifier(
                            *atom,
                            fc.func_scope_idx,
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
                    compile_stmt(fc, prog, analysis, func_scopes, s);
                }
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
                    compile_stmt(fc, prog, analysis, func_scopes, s);
                }
            }

            // Patch the exception handler offsets.
            assert!(
                u16::try_from(catch_offset).is_ok(),
                "catch offset {catch_offset} exceeds u16 range"
            );
            let catch_bytes = (catch_offset as u16).to_le_bytes();
            fc.bytecode[handler_patch_pos as usize] = catch_bytes[0];
            fc.bytecode[(handler_patch_pos + 1) as usize] = catch_bytes[1];
            // Patch finally offset: actual PC if present, 0xFFFF if absent.
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
        }

        StmtKind::Switch {
            discriminant,
            cases,
        } => {
            compile_expr(fc, prog, analysis, func_scopes, *discriminant);

            // First pass: emit tests and conditional jumps.
            //
            // For each case with a test:
            //   Dup discriminant    [disc disc]
            //   PushConst(case_val) [disc disc case_val]
            //   StrictEq            [disc bool]
            //   JumpIfTrue → entry  [disc]       (pops true)
            //   Pop                 [disc]       (pops false, next test)
            //
            // After all tests, jump to default or end (discriminant on stack).
            let mut case_entry_patches: Vec<u32> = Vec::new();
            let mut has_default = false;
            let mut default_idx: usize = 0;

            for (i, case) in cases.iter().enumerate() {
                if let Some(test) = case.test {
                    fc.emit(Op::Dup); // keep discriminant
                    compile_expr(fc, prog, analysis, func_scopes, test);
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
                    compile_stmt(fc, prog, analysis, func_scopes, s);
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
            let end_fallthrough = if !has_default {
                Some(fc.emit_jump(Op::Jump))
            } else {
                None
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
}

/// Store a value (on top of stack) to a variable binding.
fn compile_pattern_store(
    fc: &mut FunctionCompiler,
    prog: &Program,
    analysis: &ScopeAnalysis,
    func_scopes: &mut [FunctionScope],
    atom: Atom,
    kind: VarKind,
) {
    let loc = super::resolve::resolve_identifier(atom, fc.func_scope_idx, func_scopes, analysis);
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
}
