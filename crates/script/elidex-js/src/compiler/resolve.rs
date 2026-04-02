//! Variable resolution: maps identifier references to storage locations.
//!
//! Uses the scope analysis output to determine whether each identifier
//! is a local, upvalue (closure capture), global, or module variable.

use std::collections::HashMap;

use crate::atom::Atom;
use crate::scope::{BindingKind, ScopeAnalysis, ScopeKind};

/// Where a variable lives at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarLocation {
    /// Local stack slot in the current frame.
    Local(u16),
    /// Captured from an enclosing scope via upvalue chain.
    Upvalue(u16),
    /// Global variable (looked up by name at runtime).
    Global,
    /// Module-level binding (import or top-level module declaration).
    Module(u16),
}

/// Tracks local slot assignments and upvalue chains for a single function.
#[derive(Debug)]
pub struct FunctionScope {
    /// Maps (scope_idx, binding name) → local slot info.
    /// Using scope_idx as part of the key allows block-scoped shadowing
    /// (e.g. `let x` in an inner block gets a separate slot from an
    /// outer `let x`).
    pub locals: HashMap<(usize, Atom), LocalInfo>,
    /// Upvalue descriptors for this function.
    pub upvalues: Vec<UpvalueInfo>,
    /// Maps (parent scope index, binding name) → upvalue index (for dedup).
    upvalue_map: HashMap<(usize, Atom), u16>,
    /// Next available local slot.
    pub next_local: u16,
    /// Scope indices (in ScopeAnalysis.scopes) that belong to this function.
    /// The first is the function's own scope.
    pub scope_indices: Vec<usize>,
    /// Whether this function is strict mode.
    pub is_strict: bool,
}

/// Information about a local variable.
#[derive(Debug, Clone, Copy)]
pub struct LocalInfo {
    /// The local slot index.
    pub slot: u16,
    /// The binding kind (var, let, const, param, etc.).
    pub kind: BindingKind,
    /// Whether this binding requires TDZ checks.
    pub needs_tdz: bool,
}

/// Information about an upvalue capture.
#[derive(Debug, Clone, Copy)]
pub struct UpvalueInfo {
    /// If true, captures from the immediately enclosing function's locals.
    /// If false, captures from the enclosing function's upvalues (transitive).
    pub is_local: bool,
    /// Index into parent's locals (if is_local) or parent's upvalues.
    pub index: u16,
    /// The binding kind being captured.
    pub kind: BindingKind,
}

impl FunctionScope {
    /// Create a new function scope.
    pub fn new(is_strict: bool) -> Self {
        Self {
            locals: HashMap::new(),
            upvalues: Vec::new(),
            upvalue_map: HashMap::new(),
            next_local: 0,
            scope_indices: Vec::new(),
            is_strict,
        }
    }

    /// Allocate a local slot for a binding in a specific scope.
    pub fn add_local(&mut self, scope_idx: usize, name: Atom, kind: BindingKind) -> u16 {
        let slot = self.next_local;
        self.next_local += 1;
        let needs_tdz = matches!(
            kind,
            BindingKind::Let | BindingKind::Const | BindingKind::Class
        );
        self.locals.insert(
            (scope_idx, name),
            LocalInfo {
                slot,
                kind,
                needs_tdz,
            },
        );
        slot
    }

    /// Look up a local by name, searching from the innermost scope
    /// outward within this function's scope indices.
    #[must_use]
    pub fn get_local(&self, name: Atom) -> Option<&LocalInfo> {
        // Search from innermost to outermost scope within this function.
        for &scope_idx in self.scope_indices.iter().rev() {
            if let Some(info) = self.locals.get(&(scope_idx, name)) {
                return Some(info);
            }
        }
        None
    }

    /// Look up a local by name, searching from a specific scope outward
    /// through parent scopes within this function.
    #[must_use]
    pub fn get_local_from_scope(
        &self,
        name: Atom,
        scope_idx: usize,
        analysis: &ScopeAnalysis,
    ) -> Option<&LocalInfo> {
        let mut current = Some(scope_idx);
        while let Some(idx) = current {
            if !self.scope_indices.contains(&idx) {
                break;
            }
            if let Some(info) = self.locals.get(&(idx, name)) {
                return Some(info);
            }
            current = analysis.scopes[idx].parent;
        }
        None
    }

    /// Add an upvalue capture, returning its index. Deduplicates.
    pub fn add_upvalue(&mut self, scope_idx: usize, name: Atom, info: UpvalueInfo) -> u16 {
        let key = (scope_idx, name);
        if let Some(&idx) = self.upvalue_map.get(&key) {
            return idx;
        }
        let idx = self.upvalues.len() as u16;
        self.upvalues.push(info);
        self.upvalue_map.insert(key, idx);
        idx
    }
}

/// Build function scopes from scope analysis.
///
/// Walks the scope tree and groups scopes into function boundaries.
/// Each function scope gets local slot assignments for all bindings
/// declared within it.
///
/// Returns `(func_scopes, scope_to_func)`. Index 0 is the top-level
/// (script/module body). Nested functions are added in order of
/// appearance. `scope_to_func` maps each scope index to its owning
/// function index for O(1) lookup.
pub fn build_function_scopes(analysis: &ScopeAnalysis) -> (Vec<FunctionScope>, Vec<usize>) {
    let scopes = &analysis.scopes;
    if scopes.is_empty() {
        return (vec![FunctionScope::new(false)], vec![]);
    }

    // Map each scope index to its owning function index.
    let mut scope_to_func: Vec<usize> = vec![0; scopes.len()];
    let mut func_scopes: Vec<FunctionScope> = Vec::new();

    // First pass: identify function boundaries.
    // The root scope (index 0) always belongs to function 0.
    func_scopes.push(FunctionScope::new(scopes[0].is_strict));
    func_scopes[0].scope_indices.push(0);

    for (i, scope) in scopes.iter().enumerate().skip(1) {
        if is_function_boundary(scope.kind) {
            // This scope starts a new function.
            let func_idx = func_scopes.len();
            func_scopes.push(FunctionScope::new(scope.is_strict));
            func_scopes[func_idx].scope_indices.push(i);
            scope_to_func[i] = func_idx;
        } else {
            // Inherit from parent.
            let parent_func = scope.parent.map_or(0, |p| scope_to_func[p]);
            scope_to_func[i] = parent_func;
            func_scopes[parent_func].scope_indices.push(i);
        }
    }

    // Second pass: allocate local slots for bindings in each function.
    for (scope_idx, scope) in scopes.iter().enumerate() {
        let func_idx = scope_to_func[scope_idx];
        let func = &mut func_scopes[func_idx];

        for binding in &scope.bindings {
            // Skip if already allocated in this exact scope (duplicate binding).
            if func.locals.contains_key(&(scope_idx, binding.name)) {
                continue;
            }
            // For `var` declarations, skip if already allocated in any scope
            // within this function (var is function-scoped, not block-scoped).
            if binding.kind == BindingKind::Var
                && func
                    .scope_indices
                    .iter()
                    .any(|&si| func.locals.contains_key(&(si, binding.name)))
            {
                continue;
            }
            func.add_local(scope_idx, binding.name, binding.kind);
        }
    }

    (func_scopes, scope_to_func)
}

/// Resolve an identifier reference to its storage location.
///
/// `current_func_idx` is the index into the `func_scopes` array.
/// Walks outward through enclosing functions to find the binding.
pub fn resolve_identifier(
    name: Atom,
    current_func_idx: usize,
    func_scopes: &mut [FunctionScope],
    analysis: &ScopeAnalysis,
) -> VarLocation {
    // 1. Check current function's locals.
    if let Some(info) = func_scopes[current_func_idx].get_local(name) {
        return VarLocation::Local(info.slot);
    }

    // 2. Walk enclosing functions outward.
    // Find the parent function by walking scope parents.
    let enclosing = find_enclosing_functions(current_func_idx, func_scopes, analysis);

    for &(enc_func_idx, enc_scope_idx) in &enclosing {
        if let Some(info) = func_scopes[enc_func_idx].get_local(name) {
            // Found in an enclosing function — create upvalue chain.
            return create_upvalue_chain(
                name,
                enc_func_idx,
                enc_scope_idx,
                info.slot,
                info.kind,
                current_func_idx,
                func_scopes,
                analysis,
            );
        }
    }

    // 3. Check for module scope.
    if let Some(root_scope) = analysis.scopes.first() {
        if root_scope.kind == ScopeKind::Module {
            if let Some(&binding_idx) = root_scope.binding_index.get(&name) {
                let binding = &root_scope.bindings[binding_idx];
                if binding.kind == BindingKind::Import {
                    return VarLocation::Module(binding_idx as u16);
                }
            }
        }
    }

    // 4. Not found — it's a global.
    VarLocation::Global
}

/// Create upvalue chain from the function where the binding lives
/// to the function that references it.
#[allow(clippy::too_many_arguments)]
fn create_upvalue_chain(
    name: Atom,
    source_func_idx: usize,
    source_scope_idx: usize,
    source_slot: u16,
    source_kind: BindingKind,
    target_func_idx: usize,
    func_scopes: &mut [FunctionScope],
    analysis: &ScopeAnalysis,
) -> VarLocation {
    // Build the chain of function indices from source to target.
    let chain = build_func_chain(source_func_idx, target_func_idx, func_scopes, analysis);

    if chain.len() < 2 {
        // Same function — shouldn't happen (would have been found as local).
        return VarLocation::Local(source_slot);
    }

    // First link: capture from source function's local.
    let mut prev_index = source_slot;
    let mut is_local = true;

    for &func_idx in &chain[1..] {
        let uv_info = UpvalueInfo {
            is_local,
            index: prev_index,
            kind: source_kind,
        };
        let uv_idx = func_scopes[func_idx].add_upvalue(source_scope_idx, name, uv_info);
        prev_index = uv_idx;
        is_local = false; // subsequent captures are from upvalues, not locals
    }

    VarLocation::Upvalue(prev_index)
}

/// Build the chain of function indices from source (outer) to target (inner).
/// Returns [source_func_idx, ..., target_func_idx].
fn build_func_chain(
    source_func_idx: usize,
    target_func_idx: usize,
    func_scopes: &[FunctionScope],
    analysis: &ScopeAnalysis,
) -> Vec<usize> {
    // Simple approach: walk from target up to source via scope parents.
    let mut chain = vec![target_func_idx];
    let mut current = target_func_idx;

    while current != source_func_idx {
        // Find the parent function of `current`.
        let root_scope_idx = func_scopes[current].scope_indices[0];
        let parent_scope_idx = analysis.scopes[root_scope_idx].parent;

        if let Some(parent_idx) = parent_scope_idx {
            // Find which function owns this parent scope.
            let parent_func = find_func_for_scope(parent_idx, func_scopes);
            if parent_func == current {
                // Shouldn't happen, but prevent infinite loop.
                break;
            }
            chain.push(parent_func);
            current = parent_func;
        } else {
            break;
        }
    }

    chain.reverse();
    chain
}

/// Find which function scope owns a given scope index.
fn find_func_for_scope(scope_idx: usize, func_scopes: &[FunctionScope]) -> usize {
    for (i, fs) in func_scopes.iter().enumerate() {
        if fs.scope_indices.contains(&scope_idx) {
            return i;
        }
    }
    0 // fallback to top-level
}

/// Find enclosing functions (from innermost to outermost).
fn find_enclosing_functions(
    func_idx: usize,
    func_scopes: &[FunctionScope],
    analysis: &ScopeAnalysis,
) -> Vec<(usize, usize)> {
    let mut result = Vec::new();
    let mut current = func_idx;

    loop {
        let root_scope_idx = func_scopes[current].scope_indices[0];
        let parent_scope_idx = analysis.scopes[root_scope_idx].parent;

        if let Some(parent_idx) = parent_scope_idx {
            let parent_func = find_func_for_scope(parent_idx, func_scopes);
            if parent_func == current {
                break;
            }
            result.push((parent_func, parent_idx));
            current = parent_func;
        } else {
            break;
        }
    }

    result
}

/// Whether a scope kind starts a new function boundary.
fn is_function_boundary(kind: ScopeKind) -> bool {
    matches!(kind, ScopeKind::Function | ScopeKind::StaticBlock)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{analyze_scopes, parse_script};

    /// Helper: parse JS, analyze scopes, build function scopes.
    fn setup(source: &str) -> (ScopeAnalysis, Vec<FunctionScope>, Vec<usize>) {
        let output = parse_script(source);
        assert!(
            output.errors.is_empty(),
            "parse errors: {:?}",
            output.errors
        );
        let analysis = analyze_scopes(&output.program);
        assert!(
            analysis.errors.is_empty(),
            "scope errors: {:?}",
            analysis.errors
        );
        let (func_scopes, scope_to_func) = build_function_scopes(&analysis);
        (analysis, func_scopes, scope_to_func)
    }

    #[test]
    fn global_reference() {
        let (analysis, _func_scopes, _) = setup("console.log(x);");
        assert!(analysis.scopes[0].bindings.is_empty());
    }

    #[test]
    fn local_var() {
        let (_analysis, func_scopes, _) = setup("var x = 1;");
        assert_eq!(func_scopes.len(), 1);
        assert_eq!(func_scopes[0].locals.len(), 1);
        assert_eq!(func_scopes[0].next_local, 1);
    }

    #[test]
    fn local_let_const() {
        let (_analysis, func_scopes, _) = setup("let x = 1; const y = 2;");
        assert_eq!(func_scopes[0].locals.len(), 2);
        for info in func_scopes[0].locals.values() {
            assert!(info.needs_tdz);
        }
    }

    #[test]
    fn function_creates_new_scope() {
        let (_analysis, func_scopes, _) = setup("function foo(a) { let b = a; }");
        assert!(func_scopes.len() >= 2);
        assert!(func_scopes[1].locals.len() >= 2);
    }

    #[test]
    fn nested_function_upvalue() {
        let source = "function outer() { let x = 1; function inner() { return x; } }";
        let output = parse_script(source);
        assert!(output.errors.is_empty());
        let analysis = analyze_scopes(&output.program);
        assert!(analysis.errors.is_empty());
        let (mut func_scopes, _) = build_function_scopes(&analysis);
        assert!(func_scopes.len() >= 3);

        // Use the program's interner to find the exact Atom for "x".
        let x_atom = output.program.interner.lookup("x");
        assert!(
            func_scopes[1].get_local(x_atom).is_some(),
            "outer should have local 'x'"
        );

        let loc = resolve_identifier(x_atom, 2, &mut func_scopes, &analysis);
        assert!(matches!(loc, VarLocation::Upvalue(_)));
    }

    #[test]
    fn var_not_tdz() {
        let (_, func_scopes, _) = setup("var x = 1;");
        for info in func_scopes[0].locals.values() {
            assert!(!info.needs_tdz);
        }
    }

    #[test]
    fn param_not_tdz() {
        let (_, func_scopes, _) = setup("function f(_a, _b) {}");
        assert!(func_scopes.len() >= 2);
        for info in func_scopes[1].locals.values() {
            assert!(!info.needs_tdz);
        }
    }
}
