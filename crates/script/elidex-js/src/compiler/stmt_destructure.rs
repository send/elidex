//! Destructuring pattern compilation: PatternKind → bytecode.
//!
//! Extracted from `stmt.rs` for file-size management.

use crate::arena::NodeId;
#[allow(clippy::wildcard_imports)]
use crate::ast::*;
use crate::atom::Atom;
use crate::bytecode::opcode::Op;
use crate::scope::ScopeAnalysis;

use super::expr::compile_expr;
use super::function::FunctionCompiler;
use super::resolve::FunctionScope;
use super::CompileError;

/// Compile a destructuring pattern.
///
/// Assumes the value to destructure is on top of the stack.
/// After compilation, the value is consumed (popped).
#[allow(clippy::too_many_lines)]
pub(super) fn compile_destructure_pattern(
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
            // When rest is present, save computed key values to temp locals
            // so we can exclude them from the rest object later.
            let has_rest = rest.is_some();
            let mut computed_key_slots: Vec<u16> = Vec::new();

            for prop in properties {
                fc.emit(Op::Dup); // [value value]
                match &prop.key {
                    PropertyKey::Identifier(atom) => {
                        let name = prog.interner.get(*atom);
                        let idx = fc.add_name_u16(name);
                        let ic = fc.alloc_ic_slot();
                        fc.emit_u16_u16(Op::GetProp, idx, ic); // [value prop_value]
                    }
                    PropertyKey::Literal(Literal::String(atom)) => {
                        let name = prog.interner.get(*atom);
                        let idx = fc.add_name_u16(name);
                        let ic = fc.alloc_ic_slot();
                        fc.emit_u16_u16(Op::GetProp, idx, ic);
                    }
                    PropertyKey::Computed(expr) => {
                        compile_expr(fc, prog, analysis, func_scopes, *expr)?;
                        if has_rest {
                            // Save computed key to a temp local for rest exclusion.
                            let slot = func_scopes[fc.func_scope_idx].next_local;
                            func_scopes[fc.func_scope_idx].next_local += 1;
                            fc.emit(Op::Dup); // [value key key]
                            fc.emit_u16(Op::SetLocal, slot); // [value key key]
                            fc.emit(Op::Pop); // [value key]
                            computed_key_slots.push(slot);
                        }
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
                            let idx = fc.add_name_u16(name);
                            fc.emit(Op::Dup); // [value rest_obj rest_obj]
                            fc.emit_u16(Op::DeleteProp, idx); // [value rest_obj bool]
                            fc.emit(Op::Pop); // [value rest_obj]
                        }
                        PropertyKey::Literal(Literal::String(atom)) => {
                            let name = prog.interner.get(*atom);
                            let idx = fc.add_name_u16(name);
                            fc.emit(Op::Dup);
                            fc.emit_u16(Op::DeleteProp, idx);
                            fc.emit(Op::Pop);
                        }
                        // Computed/other keys: handled below via temp locals
                        _ => {}
                    }
                }
                // Delete computed keys from rest_obj using saved temp locals.
                for slot in &computed_key_slots {
                    fc.emit(Op::Dup); // [value rest_obj rest_obj]
                    fc.emit_u16(Op::GetLocal, *slot); // [value rest_obj rest_obj key]
                    fc.emit(Op::DeleteElem); // [value rest_obj bool]
                    fc.emit(Op::Pop); // [value rest_obj]
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
pub(super) fn compile_pattern_store(
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
            let idx = fc.add_name_u16(name);
            fc.emit_u16(Op::SetGlobal, idx);
            fc.emit(Op::Pop);
        }
        super::resolve::VarLocation::Module(_) => {
            fc.emit(Op::Pop);
        }
    }

    Ok(())
}
