//! Post-parse scope analysis.
//!
//! Walks the AST to build a scope tree with bindings. Detects:
//! - `var` hoisting to function/global scope
//! - `let`/`const`/`class` block scoping
//! - Function declaration hoisting
//! - Strict mode detection (`"use strict"` directive, modules always strict)
//! - Duplicate `let`/`const` declarations

mod visitor;

use std::collections::HashMap;

use crate::arena::NodeId;
#[allow(clippy::wildcard_imports)]
// AST module exports 50+ node types used pervasively in scope analysis.
use crate::ast::*;
use crate::atom::Atom;
use crate::error::{JsParseError, JsParseErrorKind, MAX_ERRORS, MAX_NESTING_DEPTH};
use crate::span::Span;

/// Result of scope analysis.
#[derive(Debug)]
pub struct ScopeAnalysis {
    pub scopes: Vec<Scope>,
    pub errors: Vec<JsParseError>,
}

/// A single scope in the scope tree.
#[derive(Debug)]
pub struct Scope {
    pub kind: ScopeKind,
    pub bindings: Vec<Binding>,
    /// O(1) lookup: binding name → index of first binding in `bindings` Vec.
    pub binding_index: HashMap<Atom, usize>,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub is_strict: bool,
    pub span: Span,
    /// Whether this is an arrow function scope (arrows don't have own `arguments`).
    pub is_arrow: bool,
    /// Whether the `arguments` identifier is referenced in this function scope.
    pub uses_arguments: bool,
}

/// Kind of scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeKind {
    Global,
    Function,
    Block,
    Catch,
    Module,
    /// ES2022 static initialization block — acts as var hoisting boundary.
    StaticBlock,
}

impl ScopeKind {
    /// Whether this scope kind is a var hoisting boundary (function, global, module, static block).
    #[must_use]
    pub fn is_var_boundary(self) -> bool {
        matches!(
            self,
            Self::Function | Self::Global | Self::Module | Self::StaticBlock
        )
    }
}

/// A binding declaration.
#[derive(Debug, Clone)]
pub struct Binding {
    pub name: Atom,
    pub kind: BindingKind,
    pub span: Span,
    pub is_hoisted: bool,
}

/// What kind of declaration created this binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingKind {
    Var,
    Let,
    Const,
    Function,
    Param,
    CatchParam,
    Class,
    Import,
    /// A1: Implicit binding (e.g., `arguments` in non-arrow functions).
    /// Not subject to strict-mode `eval`/`arguments` restriction.
    Implicit,
}

impl From<VarKind> for BindingKind {
    fn from(kind: VarKind) -> Self {
        match kind {
            VarKind::Var => Self::Var,
            VarKind::Let => Self::Let,
            VarKind::Const => Self::Const,
        }
    }
}

/// Analyze scopes of a parsed program.
pub(crate) fn analyze(program: &Program) -> ScopeAnalysis {
    let mut state = ScopeState::new(program);

    let root_kind = match program.kind {
        ProgramKind::Module => ScopeKind::Module,
        ProgramKind::Script => ScopeKind::Global,
    };
    // All top-level code is strict by default (M4-12 PR1.5).  Nested functions
    // inherit strictness from their enclosing scope per §10.2.1, so the VM no
    // longer needs sloppy-mode code paths (`ThisMode::Global`,
    // `bind_this_global`, silent property-write failures, silent global
    // creation).  The `has_use_strict` helper remains in use for nested
    // function bodies — function-level "use strict" is now always a no-op but
    // must still parse cleanly per §14.1.2.
    state.push_scope(root_kind, true, Span::new(0, u32::MAX));

    for &stmt_id in &program.body {
        visitor::visit_stmt(program, &mut state, stmt_id);
    }

    state.pop_scope();

    ScopeAnalysis {
        scopes: state.scopes,
        errors: state.errors,
    }
}

/// Mutable state for scope analysis, separated from the read-only `Program`
/// so that visitor functions can hold shared borrows on the AST while mutating state.
pub(super) struct ScopeState {
    pub(super) scopes: Vec<Scope>,
    pub(super) scope_stack: Vec<usize>,
    pub(super) errors: Vec<JsParseError>,
    pub(super) depth: u32,
    /// B8: track exported names for duplicate detection across export statements.
    pub(super) exported_names: std::collections::HashSet<Atom>,
    /// Pre-looked-up atom for the "default" export name.
    pub(super) default_atom: Atom,
}

impl ScopeState {
    fn new(program: &Program) -> Self {
        Self {
            scopes: Vec::new(),
            scope_stack: Vec::new(),
            errors: Vec::new(),
            depth: 0,
            exported_names: std::collections::HashSet::new(),
            default_atom: program.atoms.default,
        }
    }

    /// M7: Check if error limit has been reached.
    pub(super) fn at_error_limit(&self) -> bool {
        self.errors.len() >= MAX_ERRORS
    }

    pub(super) fn current_scope(&self) -> usize {
        // S3: fallback to global scope (index 0) if stack is empty due to error recovery
        self.scope_stack.last().copied().unwrap_or(0)
    }

    pub(super) fn push_scope(&mut self, kind: ScopeKind, is_strict: bool, span: Span) -> usize {
        let idx = self.scopes.len();
        let parent = self.scope_stack.last().copied();
        self.scopes.push(Scope {
            kind,
            bindings: Vec::new(),
            binding_index: HashMap::new(),
            parent,
            children: Vec::new(),
            is_strict,
            span,
            is_arrow: false,
            uses_arguments: false,
        });
        if let Some(parent_idx) = parent {
            self.scopes[parent_idx].children.push(idx);
        }
        self.scope_stack.push(idx);
        idx
    }

    pub(super) fn pop_scope(&mut self) {
        // S3: guard against empty stack from error recovery ASTs
        if !self.scope_stack.is_empty() {
            self.scope_stack.pop();
        }
    }

    /// Enter a recursive visit level. Returns false if max depth exceeded.
    pub(super) fn enter_recursion(&mut self) -> bool {
        self.depth += 1;
        self.depth <= MAX_NESTING_DEPTH
    }

    /// Leave a recursive visit level.
    pub(super) fn leave_recursion(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    pub(super) fn is_strict(&self) -> bool {
        self.scopes[self.current_scope()].is_strict
    }

    /// Find nearest function or global scope for var hoisting.
    pub(super) fn var_scope(&self) -> usize {
        for &idx in self.scope_stack.iter().rev() {
            if self.scopes[idx].kind.is_var_boundary() {
                return idx;
            }
        }
        self.current_scope()
    }

    /// M3: Check for duplicate exported name and emit error if found.
    pub(super) fn check_duplicate_export(&mut self, prog: &Program, name: Atom, span: Span) {
        if self.at_error_limit() {
            return;
        }
        if !self.exported_names.insert(name) {
            let s = prog.interner.get_utf8(name);
            self.errors.push(JsParseError {
                kind: JsParseErrorKind::DuplicateBinding,
                span,
                message: format!("Duplicate export name '{s}'"),
            });
        }
    }

    pub(super) fn add_binding(
        &mut self,
        prog: &Program,
        scope_idx: usize,
        name: Atom,
        kind: BindingKind,
        span: Span,
        hoisted: bool,
    ) {
        if self.at_error_limit() {
            return;
        }
        // B17/B2: `eval` and `arguments` cannot be bound with let/const/class/param/function in strict mode
        if matches!(
            kind,
            BindingKind::Let
                | BindingKind::Const
                | BindingKind::Class
                | BindingKind::Param
                | BindingKind::Function
        ) && (name == prog.atoms.eval || name == prog.atoms.arguments)
            && self.is_strict()
        {
            let name_str = prog.interner.get_utf8(name);
            self.errors.push(JsParseError {
                kind: JsParseErrorKind::StrictModeViolation,
                span,
                message: format!("Cannot bind '{name_str}' in strict mode"),
            });
            return;
        }

        // O(1) conflict detection via binding index.
        if let Some(&existing_idx) = self.scopes[scope_idx].binding_index.get(&name) {
            let existing = &self.scopes[scope_idx].bindings[existing_idx];
            let conflict = match kind {
                // B23: import conflicts with any existing binding;
                // let/const/class conflict with any existing binding
                BindingKind::Import
                | BindingKind::Let
                | BindingKind::Const
                | BindingKind::Class => true,
                // var conflicts with existing lexical or import
                BindingKind::Var => matches!(
                    existing.kind,
                    BindingKind::Let
                        | BindingKind::Const
                        | BindingKind::Class
                        | BindingKind::Import
                ),
                // E6: function conflicts with lexical/import; in strict mode also with another function
                BindingKind::Function => matches!(
                    existing.kind,
                    BindingKind::Let
                        | BindingKind::Const
                        | BindingKind::Class
                        | BindingKind::Import
                        | BindingKind::Function
                ),
                // Param conflicts with existing import, or with another Param in strict mode
                BindingKind::Param => {
                    existing.kind == BindingKind::Import
                        || (self.scopes[scope_idx].is_strict && existing.kind == BindingKind::Param)
                }
                // CatchParam only conflicts with existing import
                BindingKind::CatchParam => existing.kind == BindingKind::Import,
                // Implicit (e.g. `arguments`) is silently overridden by any explicit binding
                BindingKind::Implicit => false,
            };
            if conflict {
                let s = prog.interner.get_utf8(name);
                self.errors.push(JsParseError {
                    kind: JsParseErrorKind::DuplicateBinding,
                    span,
                    message: format!("Duplicate binding '{s}'"),
                });
                return;
            }
        }

        // Insert into index before pushing (index points to first binding with this name).
        let next_idx = self.scopes[scope_idx].bindings.len();
        self.scopes[scope_idx]
            .binding_index
            .entry(name)
            .or_insert(next_idx);
        self.scopes[scope_idx].bindings.push(Binding {
            name,
            kind,
            span,
            is_hoisted: hoisted,
        });
    }

    /// S7: Check if a `var` declaration being hoisted to `target_scope` conflicts with
    /// any `CatchParam` in intermediate catch scopes on the scope stack.
    pub(super) fn check_var_catch_conflict(
        &mut self,
        prog: &Program,
        name: Atom,
        span: Span,
        target_scope: usize,
    ) {
        if self.at_error_limit() {
            return;
        }
        for &idx in self.scope_stack.iter().rev() {
            if idx == target_scope {
                break;
            }
            if self.scopes[idx].kind == ScopeKind::Catch {
                for b in &self.scopes[idx].bindings {
                    if b.name == name && b.kind == BindingKind::CatchParam {
                        let s = prog.interner.get_utf8(name);
                        self.errors.push(JsParseError {
                            kind: JsParseErrorKind::DuplicateBinding,
                            span,
                            message: format!("Cannot redeclare catch parameter '{s}' with var"),
                        });
                        return;
                    }
                }
            }
        }
    }
}

/// Check for "use strict" directive prologue.
pub(super) fn has_use_strict(prog: &Program, body: &[NodeId<Stmt>]) -> bool {
    // "use strict" is pure ASCII — use a const UTF-16 array to avoid
    // allocating inside the loop.
    const USE_STRICT_U16: &[u16] = &[
        b'u' as u16,
        b's' as u16,
        b'e' as u16,
        b' ' as u16,
        b's' as u16,
        b't' as u16,
        b'r' as u16,
        b'i' as u16,
        b'c' as u16,
        b't' as u16,
    ];
    for &stmt_id in body {
        let stmt = prog.stmts.get(stmt_id);
        if let StmtKind::Expression(expr_id) = &stmt.kind {
            let expr = prog.exprs.get(*expr_id);
            if let ExprKind::Literal(Literal::String(s)) = expr.kind {
                if prog.interner.get(s) == USE_STRICT_U16 {
                    return true;
                }
                continue; // other string directives
            }
        }
        break; // non-directive statement
    }
    false
}

#[cfg(test)]
mod tests;
