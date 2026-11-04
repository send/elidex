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
use super::stmt_destructure::compile_destructure_pattern;

/// Emit the formal-parameter prologue, shared by functions and arrows.
///
/// Each formal parameter occupies one positional slot (`slot == index`),
/// already filled by the caller. For each parameter, in source order:
///
/// 1. If it has a default, replace the slot with the default expression
///    when the incoming argument was `undefined` (ES2020: `undefined`
///    only, not `null`).
/// 2. If its pattern is a destructuring (non-`Identifier`) pattern, read
///    the slot and destructure it into the names the pattern declares
///    (allocated outside the `0..arity` positional region by `resolve`).
///
/// Left-to-right, default-then-destructure ordering mirrors §10.2.11
/// FunctionDeclarationInstantiation, so a later parameter's default may
/// observe an earlier parameter's already-destructured bindings.
fn compile_param_prologue(
    fc: &mut FunctionCompiler,
    prog: &Program,
    analysis: &ScopeAnalysis,
    func_scopes: &mut [FunctionScope],
    params: &[Param],
) -> Result<(), CompileError> {
    for (i, param) in params.iter().enumerate() {
        let slot = i as u16;
        if let Some(default_expr) = param.default {
            fc.emit_u16(Op::GetLocal, slot);
            fc.emit(Op::PushUndefined);
            fc.emit(Op::StrictEq);
            let skip = fc.emit_jump(Op::JumpIfFalse);
            compile_expr(fc, prog, analysis, func_scopes, default_expr)?;
            fc.emit_u16(Op::SetLocal, slot);
            fc.emit(Op::Pop);
            fc.patch_jump(skip);
        }
        // Simple identifier params bind their slot directly; only
        // destructuring patterns need an unpacking prologue. `Var` kind
        // selects no `InitLocal` — param bindings carry no TDZ.
        if !param.is_simple_identifier(&prog.patterns) {
            fc.emit_u16(Op::GetLocal, slot);
            compile_destructure_pattern(
                fc,
                prog,
                analysis,
                func_scopes,
                param.pattern,
                VarKind::Var,
            )?;
        }
    }
    Ok(())
}

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

/// The spec creation path a function object takes, which decides whether
/// MakeConstructor (ECMA-262 §10.2.5) applies to it — the compile-time
/// fact behind [`CompiledFunction::has_construct`].  Every caller of
/// [`compile_nested_function`] names its path; arrows and async arrows
/// (InstantiateArrowFunctionExpression §15.3.4,
/// InstantiateAsyncArrowFunctionExpression §15.9.4) are compiled by
/// [`compile_arrow_function`] and keep the default `false`.
///
/// [`CompiledFunction::has_construct`]: crate::bytecode::compiled::CompiledFunction::has_construct
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FunctionCreation {
    /// InstantiateOrdinaryFunctionObject (§15.2.4) /
    /// InstantiateOrdinaryFunctionExpression (§15.2.5), and their
    /// generator (§15.5.3 / §15.5.4), async generator (§15.6.3 /
    /// §15.6.4) and async (§15.8.2 / §15.8.3) siblings.  Only the
    /// ordinary ones call MakeConstructor, so the bit is set exactly
    /// when the function is neither async nor a generator.
    Ordinary,
    /// DefineMethod (§15.4.4) and MethodDefinitionEvaluation (§15.4.5):
    /// methods, getters, setters, generator / async / async generator
    /// methods — object literal and class alike.  None calls
    /// MakeConstructor.
    Method,
    /// The written class constructor: ClassDefinitionEvaluation
    /// (§15.7.14) creates it by DefineMethod at step 15.a and applies
    /// MakeConstructor at step 17.
    ClassConstructor,
}

impl FunctionCreation {
    /// MakeConstructor applies to the created function object.
    fn has_construct(self, func: &Function) -> bool {
        match self {
            Self::Ordinary => !func.is_async && !func.is_generator,
            Self::Method => false,
            Self::ClassConstructor => true,
        }
    }
}

/// Compile a nested `Function` (declaration or expression) into a `CompiledFunction`.
///
/// This is `pub(super)` so that `stmt.rs` can call it for `FunctionDeclaration`.
/// `creation` names the spec creation path and sets
/// `CompiledFunction::has_construct` (see [`FunctionCreation`]).
#[allow(clippy::too_many_lines)]
pub(super) fn compile_nested_function(
    _parent_fc: &mut FunctionCompiler,
    prog: &Program,
    analysis: &ScopeAnalysis,
    func_scopes: &mut [FunctionScope],
    func: &Function,
    creation: FunctionCreation,
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

    // Apply parameter defaults and unpack destructuring patterns.
    compile_param_prologue(&mut child_fc, prog, analysis, func_scopes, &func.params)?;

    // Populate the `arguments` local only when actually referenced.
    if analysis.scopes[root_scope_idx].uses_arguments {
        let args_atom = prog.atoms.arguments;
        if let Some(info) = func_scopes[child_func_idx]
            .locals
            .get(&(root_scope_idx, args_atom))
        {
            if matches!(info.kind, BindingKind::Implicit) {
                // Emit: CreateArguments → SetLocal → InitLocal → Pop.
                //
                // A trailing `PushUndefined; Pop` used to follow, to "reset
                // completion_value so the arguments object is not returned as
                // the constructor's result".  `Op::Pop` no longer writes
                // `completion_value` at all — only `Op::PopCompletion` does, and
                // only from an `Eval`-kind entry frame — so the pair was two
                // no-op instructions in every function that mentions
                // `arguments`.  Dropped rather than left as dead ballast.
                child_fc.needs_arguments = true;
                child_fc.emit(Op::CreateArguments);
                child_fc.emit_u16(Op::SetLocal, info.slot);
                child_fc.emit_u16(Op::InitLocal, info.slot);
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
                    FunctionCreation::Ordinary,
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
    compiled.has_rest_param = func.params.last().is_some_and(|p| p.rest);
    compiled.has_construct = creation.has_construct(func);
    // User-written class constructor: marks `home_class` threading
    // ([C16] ClassDefinitionEvaluation — class ctor frames get
    // `CallFrame::home_class = closure_obj_id`).  Set here with its
    // sibling `has_construct`, one site for the creation path.
    compiled.is_class_ctor = matches!(creation, FunctionCreation::ClassConstructor);
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

    // Apply parameter defaults and unpack destructuring patterns.
    compile_param_prologue(&mut child_fc, prog, analysis, func_scopes, &arrow.params)?;

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
    compiled.has_rest_param = arrow.params.last().is_some_and(|p| p.rest);
    Ok(compiled)
}
