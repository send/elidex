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

                // Simple identifier pattern.
                let pattern = prog.patterns.get(decl.pattern);
                if let PatternKind::Identifier(atom) = &pattern.kind {
                    compile_pattern_store(fc, prog, analysis, func_scopes, *atom, *kind);
                }
                // TODO: destructuring patterns
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

            fc.emit_jump_to(Op::Jump, loop_start);
            fc.patch_jump(exit_patch);
            fc.pop_loop();
        }

        StmtKind::DoWhile { body, test } => {
            let loop_start = fc.pc();
            let continue_target = fc.pc(); // continue jumps to test
            fc.push_loop(continue_target);

            compile_stmt(fc, prog, analysis, func_scopes, *body);

            // Continue target: evaluate test.
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
                                let pattern = prog.patterns.get(decl.pattern);
                                if let PatternKind::Identifier(atom) = &pattern.kind {
                                    compile_pattern_store(
                                        fc,
                                        prog,
                                        analysis,
                                        func_scopes,
                                        *atom,
                                        *kind,
                                    );
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
            // Continue target is the update expression (or loop start if no update).
            let continue_target = fc.pc(); // will be updated below
            fc.push_loop(continue_target);

            // Test.
            let exit_patch = if let Some(test_expr) = test {
                compile_expr(fc, prog, analysis, func_scopes, *test_expr);
                Some(fc.emit_jump(Op::JumpIfFalse))
            } else {
                None
            };

            // Body.
            compile_stmt(fc, prog, analysis, func_scopes, *body);

            // Update the continue target to point here (before update expression).
            let current_pc = fc.pc();
            if let Some(ctx) = fc.loop_stack.last_mut() {
                ctx.continue_target = current_pc;
            }

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
            if let Some(ctx) = fc.loop_stack.last() {
                let target = ctx.continue_target;
                fc.emit_jump_to(Op::Jump, target);
            }
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
            if let Some(fin_block) = finalizer {
                for &s in fin_block {
                    compile_stmt(fc, prog, analysis, func_scopes, s);
                }
            }

            // Patch the exception handler offsets.
            let catch_bytes = (catch_offset as u16).to_le_bytes();
            fc.bytecode[handler_patch_pos as usize] = catch_bytes[0];
            fc.bytecode[(handler_patch_pos + 1) as usize] = catch_bytes[1];
            // Finally offset = 0xFFFF (no finally offset in simple model).
            fc.bytecode[(handler_patch_pos + 2) as usize] = 0xFF;
            fc.bytecode[(handler_patch_pos + 3) as usize] = 0xFF;
        }

        StmtKind::Switch {
            discriminant,
            cases,
        } => {
            compile_expr(fc, prog, analysis, func_scopes, *discriminant);

            let mut case_patches: Vec<u32> = Vec::new();
            let mut default_patch: Option<u32> = None;

            // First pass: emit tests and conditional jumps.
            for case in cases {
                if let Some(test) = case.test {
                    fc.emit(Op::Dup); // keep discriminant
                    compile_expr(fc, prog, analysis, func_scopes, test);
                    fc.emit(Op::StrictEq);
                    let patch = fc.emit_jump(Op::JumpIfTrue);
                    case_patches.push(patch);
                } else {
                    // default case
                    default_patch = Some(0); // placeholder
                    case_patches.push(0); // placeholder
                }
            }

            // Jump to default or end.
            let end_or_default = fc.emit_jump(Op::Jump);
            fc.emit(Op::Pop); // pop discriminant

            // Second pass: emit case bodies.
            fc.push_loop(0); // for break support
            for (i, case) in cases.iter().enumerate() {
                if case.test.is_some() {
                    fc.patch_jump(case_patches[i]);
                    fc.emit(Op::Pop); // pop true from StrictEq
                } else if default_patch.is_some() {
                    fc.patch_jump(end_or_default);
                }
                fc.emit(Op::Pop); // pop discriminant copy
                for &s in &case.consequent {
                    compile_stmt(fc, prog, analysis, func_scopes, s);
                }
            }

            if default_patch.is_none() {
                fc.patch_jump(end_or_default);
                fc.emit(Op::Pop); // pop discriminant
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
