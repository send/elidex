//! Nested function and arrow function compilation.

#[allow(clippy::wildcard_imports)]
use crate::ast::*;
use crate::bytecode::opcode::Op;

use super::function::FunctionCompiler;
use super::resolve::FunctionScope;
use super::CompileError;
use crate::scope::{BindingKind, ScopeAnalysis, ScopeKind};
use crate::span::Span;

use super::expr::compile_expr;

// ── Nested function compilation ────────────────────────────────────

/// Find the `func_scopes` index for a function/arrow with the given span.
///
/// Searches `analysis.scopes` for a `Function` scope whose span matches,
/// then maps that scope index back to its owning `func_scopes` entry.
fn find_func_scope_for_span(
    analysis: &ScopeAnalysis,
    func_scopes: &[FunctionScope],
    span: Span,
) -> Option<usize> {
    for (scope_idx, scope) in analysis.scopes.iter().enumerate() {
        if scope.kind == ScopeKind::Function && scope.span == span {
            for (fi, fs) in func_scopes.iter().enumerate() {
                if fs.scope_indices.first() == Some(&scope_idx) {
                    return Some(fi);
                }
            }
        }
    }
    None
}

/// Compile a nested `Function` (declaration or expression) into a `CompiledFunction`.
///
/// This is `pub(super)` so that `stmt.rs` can call it for `FunctionDeclaration`.
#[allow(clippy::too_many_lines)]
pub(super) fn compile_nested_function(
    _parent_fc: &mut FunctionCompiler,
    prog: &Program,
    analysis: &ScopeAnalysis,
    func_scopes: &mut [FunctionScope],
    func: &Function,
    _is_declaration: bool,
) -> Result<crate::bytecode::compiled::CompiledFunction, CompileError> {
    let child_func_idx =
        find_func_scope_for_span(analysis, func_scopes, func.span).ok_or_else(|| CompileError {
            message: format!("no function scope found for span {:?}", func.span),
        })?;

    let root_scope_idx = func_scopes[child_func_idx].scope_indices[0];
    let is_strict = analysis.scopes[root_scope_idx].is_strict;

    let mut child_fc = FunctionCompiler::new(child_func_idx, root_scope_idx, is_strict);
    child_fc.name = func.name.map(|a| prog.interner.get_utf8(a));
    child_fc.is_async = func.is_async;
    child_fc.is_generator = func.is_generator;

    // Initialize var-declared locals to undefined (same pattern as top-level in mod.rs).
    let mut var_slots: Vec<u16> = func_scopes[child_func_idx]
        .locals
        .values()
        .filter(|info| matches!(info.kind, BindingKind::Var | BindingKind::Function))
        .map(|info| info.slot)
        .collect();
    var_slots.sort_unstable();
    // Skip param slots (they are filled by the caller).
    let param_count = func.params.len() as u16;
    for slot in var_slots {
        if slot < param_count {
            continue;
        }
        child_fc.emit(Op::PushUndefined);
        child_fc.emit_u16(Op::SetLocal, slot);
        child_fc.emit(Op::Pop);
    }

    // Compile default parameter values.
    for (i, param) in func.params.iter().enumerate() {
        if let Some(default_expr) = param.default {
            let slot = i as u16;
            child_fc.emit_u16(Op::GetLocal, slot);
            child_fc.emit(Op::PushUndefined);
            child_fc.emit(Op::StrictEq);
            let skip = child_fc.emit_jump(Op::JumpIfFalse);
            compile_expr(&mut child_fc, prog, analysis, func_scopes, default_expr)?;
            child_fc.emit_u16(Op::SetLocal, slot);
            child_fc.emit(Op::Pop);
            child_fc.patch_jump(skip);
        }
    }

    // Populate the `arguments` local only when actually referenced.
    if analysis.scopes[root_scope_idx].uses_arguments {
        let args_atom = prog.atoms.arguments;
        if let Some(info) = func_scopes[child_func_idx]
            .locals
            .get(&(root_scope_idx, args_atom))
        {
            if matches!(info.kind, BindingKind::Implicit) {
                // Emit: CreateArguments → SetLocal → Pop → PushUndefined → Pop
                // The extra PushUndefined → Pop resets the completion_value
                // to Undefined, preventing the arguments object from being
                // returned as the constructor's result (§9.2.5).
                child_fc.needs_arguments = true;
                child_fc.emit(Op::CreateArguments);
                child_fc.emit_u16(Op::SetLocal, info.slot);
                child_fc.emit_u16(Op::InitLocal, info.slot);
                child_fc.emit(Op::Pop);
                child_fc.emit(Op::PushUndefined);
                child_fc.emit(Op::Pop);
            }
        }
    }

    // Hoist function declarations: compile and store before executing body statements.
    for &stmt_id in &func.body {
        let stmt = prog.stmts.get(stmt_id);
        if let StmtKind::FunctionDeclaration(inner_func) = &stmt.kind {
            if let Some(name) = &inner_func.name {
                let hoisted = compile_nested_function(
                    &mut child_fc,
                    prog,
                    analysis,
                    func_scopes,
                    inner_func,
                    true,
                )?;
                let idx = child_fc.add_constant(crate::bytecode::compiled::Constant::Function(
                    Box::new(hoisted),
                ));
                child_fc.emit_u16(Op::Closure, idx);
                let loc = super::resolve::resolve_identifier(
                    *name,
                    child_fc.func_scope_idx,
                    child_fc.current_scope_idx,
                    func_scopes,
                    analysis,
                );
                match loc {
                    super::resolve::VarLocation::Local(slot) => {
                        child_fc.emit_u16(Op::SetLocal, slot);
                        child_fc.emit(Op::Pop);
                    }
                    super::resolve::VarLocation::Global => {
                        let name_u16 = prog.interner.get(*name);
                        let name_idx = child_fc.add_name_u16(name_u16);
                        child_fc.emit_u16(Op::SetGlobal, name_idx);
                        child_fc.emit(Op::Pop);
                    }
                    _ => {
                        child_fc.emit(Op::Pop);
                    }
                }
            }
        }
    }

    // Compile body statements.
    for &stmt_id in &func.body {
        super::stmt::compile_stmt(&mut child_fc, prog, analysis, func_scopes, stmt_id)?;
    }

    // Ensure the function ends with a return.
    if child_fc.bytecode.last() != Some(&(Op::Return as u8))
        && child_fc.bytecode.last() != Some(&(Op::ReturnUndefined as u8))
    {
        child_fc.emit(Op::ReturnUndefined);
    }

    let mut compiled = child_fc.finish(&func_scopes[child_func_idx]);
    compiled.param_count = func.params.len() as u16;
    Ok(compiled)
}

/// Compile an arrow function expression into a `CompiledFunction`.
pub(super) fn compile_arrow_function(
    _parent_fc: &mut FunctionCompiler,
    prog: &Program,
    analysis: &ScopeAnalysis,
    func_scopes: &mut [FunctionScope],
    arrow: &ArrowFunction,
) -> Result<crate::bytecode::compiled::CompiledFunction, CompileError> {
    let child_func_idx =
        find_func_scope_for_span(analysis, func_scopes, arrow.span).ok_or_else(|| {
            CompileError {
                message: format!("no function scope found for arrow span {:?}", arrow.span),
            }
        })?;

    let root_scope_idx = func_scopes[child_func_idx].scope_indices[0];
    let is_strict = analysis.scopes[root_scope_idx].is_strict;

    let mut child_fc = FunctionCompiler::new(child_func_idx, root_scope_idx, is_strict);
    child_fc.is_arrow = true;
    child_fc.is_async = arrow.is_async;

    // Initialize var-declared locals to undefined, skipping params.
    let mut var_slots: Vec<u16> = func_scopes[child_func_idx]
        .locals
        .values()
        .filter(|info| matches!(info.kind, BindingKind::Var | BindingKind::Function))
        .map(|info| info.slot)
        .collect();
    var_slots.sort_unstable();
    let param_count = arrow.params.len() as u16;
    for slot in var_slots {
        if slot < param_count {
            continue;
        }
        child_fc.emit(Op::PushUndefined);
        child_fc.emit_u16(Op::SetLocal, slot);
        child_fc.emit(Op::Pop);
    }

    // Compile default parameter values.
    for (i, param) in arrow.params.iter().enumerate() {
        if let Some(default_expr) = param.default {
            let slot = i as u16;
            child_fc.emit_u16(Op::GetLocal, slot);
            child_fc.emit(Op::PushUndefined);
            child_fc.emit(Op::StrictEq);
            let skip = child_fc.emit_jump(Op::JumpIfFalse);
            compile_expr(&mut child_fc, prog, analysis, func_scopes, default_expr)?;
            child_fc.emit_u16(Op::SetLocal, slot);
            child_fc.emit(Op::Pop);
            child_fc.patch_jump(skip);
        }
    }

    match &arrow.body {
        ArrowBody::Expression(expr_id) => {
            compile_expr(&mut child_fc, prog, analysis, func_scopes, *expr_id)?;
            child_fc.emit(Op::Return); // implicit return of expression value
        }
        ArrowBody::Block(stmts) => {
            for &stmt_id in stmts {
                super::stmt::compile_stmt(&mut child_fc, prog, analysis, func_scopes, stmt_id)?;
            }
            // Ensure block-body arrow ends with return.
            if child_fc.bytecode.last() != Some(&(Op::Return as u8))
                && child_fc.bytecode.last() != Some(&(Op::ReturnUndefined as u8))
            {
                child_fc.emit(Op::ReturnUndefined);
            }
        }
    }

    let mut compiled = child_fc.finish(&func_scopes[child_func_idx]);
    compiled.param_count = arrow.params.len() as u16;
    Ok(compiled)
}
