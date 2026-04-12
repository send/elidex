use super::*;
use crate::{analyze_scopes, parse_module, parse_script};

/// Resolve an Atom to String using the program's interner.
fn r(program: &Program, atom: Atom) -> String {
    program.interner.get_utf8(atom)
}

#[test]
fn global_scope_var() {
    let out = parse_script("var x = 1;");
    let sa = analyze_scopes(&out.program);
    assert_eq!(sa.scopes.len(), 1);
    assert_eq!(sa.scopes[0].kind, ScopeKind::Global);
    assert_eq!(sa.scopes[0].bindings.len(), 1);
    assert_eq!(r(&out.program, sa.scopes[0].bindings[0].name), "x");
    assert_eq!(sa.scopes[0].bindings[0].kind, BindingKind::Var);
    assert!(sa.scopes[0].bindings[0].is_hoisted);
}

#[test]
fn block_scoped_let() {
    let out = parse_script("{ let x = 1; }");
    let sa = analyze_scopes(&out.program);
    assert_eq!(sa.scopes.len(), 2); // global + block
    assert_eq!(sa.scopes[1].kind, ScopeKind::Block);
    assert_eq!(r(&out.program, sa.scopes[1].bindings[0].name), "x");
    assert_eq!(sa.scopes[1].bindings[0].kind, BindingKind::Let);
}

#[test]
fn var_hoisting() {
    let out = parse_script("function f() { { var x = 1; } }");
    let sa = analyze_scopes(&out.program);
    // x should be in the function scope, not the block scope
    let func_scope = sa
        .scopes
        .iter()
        .find(|s| s.kind == ScopeKind::Function)
        .unwrap();
    assert!(func_scope
        .bindings
        .iter()
        .any(|b| r(&out.program, b.name) == "x" && b.kind == BindingKind::Var));
}

#[test]
fn duplicate_let_error() {
    let out = parse_script("{ let x = 1; let x = 2; }");
    let sa = analyze_scopes(&out.program);
    assert!(!sa.errors.is_empty());
    assert!(matches!(
        sa.errors[0].kind,
        JsParseErrorKind::DuplicateBinding
    ));
}

#[test]
fn function_creates_scope() {
    let out = parse_script("function f(a, b) { let c = 1; }");
    let sa = analyze_scopes(&out.program);
    let func_scope = sa
        .scopes
        .iter()
        .find(|s| s.kind == ScopeKind::Function)
        .unwrap();
    assert!(func_scope
        .bindings
        .iter()
        .any(|b| r(&out.program, b.name) == "a" && b.kind == BindingKind::Param));
    assert!(func_scope
        .bindings
        .iter()
        .any(|b| r(&out.program, b.name) == "b" && b.kind == BindingKind::Param));
    assert!(func_scope
        .bindings
        .iter()
        .any(|b| r(&out.program, b.name) == "c" && b.kind == BindingKind::Let));
}

#[test]
fn module_is_strict() {
    let out = parse_module("let x = 1;");
    let sa = analyze_scopes(&out.program);
    assert_eq!(sa.scopes[0].kind, ScopeKind::Module);
    assert!(sa.scopes[0].is_strict);
}

#[test]
fn catch_scope() {
    let out = parse_script("try {} catch (e) { let x = 1; }");
    let sa = analyze_scopes(&out.program);
    let catch_scope = sa
        .scopes
        .iter()
        .find(|s| s.kind == ScopeKind::Catch)
        .unwrap();
    assert!(catch_scope
        .bindings
        .iter()
        .any(|b| r(&out.program, b.name) == "e" && b.kind == BindingKind::CatchParam));
    assert!(catch_scope
        .bindings
        .iter()
        .any(|b| r(&out.program, b.name) == "x"));
}

#[test]
fn import_bindings() {
    let out = parse_module("import { a, b as c } from 'mod';");
    let sa = analyze_scopes(&out.program);
    assert!(sa.scopes[0]
        .bindings
        .iter()
        .any(|b| r(&out.program, b.name) == "a" && b.kind == BindingKind::Import));
    assert!(sa.scopes[0]
        .bindings
        .iter()
        .any(|b| r(&out.program, b.name) == "c" && b.kind == BindingKind::Import));
}

#[test]
fn arrow_function_scope() {
    let out = parse_script("const f = (x) => x + 1;");
    let sa = analyze_scopes(&out.program);
    let arrow_scope = sa
        .scopes
        .iter()
        .find(|s| s.kind == ScopeKind::Function)
        .unwrap();
    assert!(arrow_scope
        .bindings
        .iter()
        .any(|b| r(&out.program, b.name) == "x" && b.kind == BindingKind::Param));
}

#[test]
fn for_loop_scope() {
    let out = parse_script("for (let i = 0; i < 10; i++) {}");
    let sa = analyze_scopes(&out.program);
    // `i` should be in the for-block scope
    let block_scope = sa
        .scopes
        .iter()
        .find(|s| s.kind == ScopeKind::Block)
        .unwrap();
    assert!(block_scope
        .bindings
        .iter()
        .any(|b| r(&out.program, b.name) == "i" && b.kind == BindingKind::Let));
}

#[test]
fn use_strict_detection() {
    let out = parse_script("\"use strict\"; let x = 1;");
    let sa = analyze_scopes(&out.program);
    assert!(sa.scopes[0].is_strict);
}

#[test]
fn class_binding() {
    let out = parse_script("class Foo {}");
    let sa = analyze_scopes(&out.program);
    assert!(sa.scopes[0]
        .bindings
        .iter()
        .any(|b| r(&out.program, b.name) == "Foo" && b.kind == BindingKind::Class));
}

#[test]
fn nested_scopes() {
    let out = parse_script("{ { let x = 1; } let y = 2; }");
    let sa = analyze_scopes(&out.program);
    // Should have: global > block > block (for x)
    assert!(sa.scopes.len() >= 3);
}

#[test]
fn error_nodes_skipped() {
    let out = parse_script("let = ;");
    // Should not panic even with error nodes
    let _sa = analyze_scopes(&out.program);
}

// ── M12: let/const vs var collision ──

#[test]
fn var_after_let_error() {
    // var x after let x in same scope should error
    let out = parse_script("function f() { let x = 1; var x = 2; }");
    let sa = analyze_scopes(&out.program);
    assert!(
        sa.errors
            .iter()
            .any(|e| matches!(e.kind, JsParseErrorKind::DuplicateBinding)),
        "Expected duplicate binding error"
    );
}

// ── L16: static block scope ──

#[test]
fn static_block_scope_kind() {
    let out = parse_script("class C { static { let x = 1; } }");
    let sa = analyze_scopes(&out.program);
    assert!(
        sa.scopes.iter().any(|s| s.kind == ScopeKind::StaticBlock),
        "Expected StaticBlock scope"
    );
}

#[test]
fn static_block_var_hoisting_boundary() {
    // var in static block should not hoist to class scope
    let out = parse_script("class C { static { var x = 1; } }");
    let sa = analyze_scopes(&out.program);
    let static_scope = sa
        .scopes
        .iter()
        .find(|s| s.kind == ScopeKind::StaticBlock)
        .unwrap();
    assert!(
        static_scope
            .bindings
            .iter()
            .any(|b| r(&out.program, b.name) == "x" && b.kind == BindingKind::Var),
        "var should be hoisted to static block scope"
    );
}

// ── Step 3: B17 — eval/arguments cannot be let/const/class bound in strict ──

#[test]
fn eval_let_binding_error_strict() {
    let out = parse_script("\"use strict\"; let eval = 1;");
    let sa = analyze_scopes(&out.program);
    assert!(
        sa.errors
            .iter()
            .any(|e| e.message.contains("eval") && e.message.contains("strict")),
        "Expected eval binding error: {:?}",
        sa.errors
    );
}

#[test]
fn arguments_const_binding_error_strict() {
    let out = parse_script("\"use strict\"; const arguments = 1;");
    let sa = analyze_scopes(&out.program);
    assert!(
        sa.errors
            .iter()
            .any(|e| e.message.contains("arguments") && e.message.contains("strict")),
        "Expected arguments binding error: {:?}",
        sa.errors
    );
}

#[test]
fn eval_binding_error_in_module() {
    // Modules are always strict
    let out = parse_module("let eval = 1;");
    let sa = analyze_scopes(&out.program);
    assert!(
        sa.errors.iter().any(|e| e.message.contains("eval")),
        "Expected eval binding error in module: {:?}",
        sa.errors
    );
}

// ── Step 6: A9 — function declarations are block-scoped in strict ──

#[test]
fn function_block_scoped_strict() {
    let out = parse_script("{ function f() {} }");
    let sa = analyze_scopes(&out.program);
    // f should be in block scope, not global
    let block_scope = sa
        .scopes
        .iter()
        .find(|s| s.kind == ScopeKind::Block)
        .unwrap();
    assert!(
        block_scope
            .bindings
            .iter()
            .any(|b| r(&out.program, b.name) == "f" && b.kind == BindingKind::Function),
        "Function should be block-scoped"
    );
    assert!(
        !sa.scopes[0]
            .bindings
            .iter()
            .any(|b| r(&out.program, b.name) == "f"),
        "Function should NOT be in global scope"
    );
}

// ── Step 6: A11 — export default function/class binding ──

#[test]
fn export_default_function_binding() {
    let out = parse_module("export default function foo() {}");
    let sa = analyze_scopes(&out.program);
    assert!(
        sa.scopes[0]
            .bindings
            .iter()
            .any(|b| r(&out.program, b.name) == "foo" && b.kind == BindingKind::Function),
        "Named export default function should create binding: {:?}",
        sa.scopes[0].bindings
    );
}

#[test]
fn export_default_class_binding() {
    let out = parse_module("export default class Foo {}");
    let sa = analyze_scopes(&out.program);
    assert!(
        sa.scopes[0]
            .bindings
            .iter()
            .any(|b| r(&out.program, b.name) == "Foo" && b.kind == BindingKind::Class),
        "Named export default class should create binding: {:?}",
        sa.scopes[0].bindings
    );
}

// ── Step 6: B23 — import binding conflicts ──

#[test]
fn import_let_conflict_error() {
    let out = parse_module("import { x } from 'mod'; let x = 1;");
    let sa = analyze_scopes(&out.program);
    assert!(
        sa.errors
            .iter()
            .any(|e| matches!(e.kind, JsParseErrorKind::DuplicateBinding)),
        "Expected import/let conflict error: {:?}",
        sa.errors
    );
}

#[test]
fn duplicate_import_error() {
    let out = parse_module("import { x } from 'a'; import { x } from 'b';");
    let sa = analyze_scopes(&out.program);
    assert!(
        sa.errors
            .iter()
            .any(|e| matches!(e.kind, JsParseErrorKind::DuplicateBinding)),
        "Expected duplicate import error: {:?}",
        sa.errors
    );
}

// ── M2: strict mode duplicate parameters ──

#[test]
fn strict_duplicate_params_error() {
    let out = parse_script("\"use strict\"; function f(a, a) {}");
    let sa = analyze_scopes(&out.program);
    assert!(
        sa.errors
            .iter()
            .any(|e| matches!(e.kind, JsParseErrorKind::DuplicateBinding)
                && e.message.contains("'a'")),
        "Expected duplicate param error in strict mode: {:?}",
        sa.errors
    );
}

#[test]
fn strict_duplicate_params_module() {
    // Modules are always strict
    let out = parse_module("function f(a, a) {}");
    let sa = analyze_scopes(&out.program);
    assert!(
        sa.errors
            .iter()
            .any(|e| matches!(e.kind, JsParseErrorKind::DuplicateBinding)),
        "Expected duplicate param error in module: {:?}",
        sa.errors
    );
}

// ── B26: class expression name inner scope ──

// ── B2: eval/arguments as parameter names ──

#[test]
fn eval_param_error_strict() {
    let out = parse_script("\"use strict\"; function f(eval) {}");
    let sa = analyze_scopes(&out.program);
    assert!(
        sa.errors
            .iter()
            .any(|e| e.message.contains("eval") && e.message.contains("strict")),
        "Expected eval param error in strict mode: {:?}",
        sa.errors
    );
}

#[test]
fn arguments_param_error_module() {
    // Modules are always strict
    let out = parse_module("function f(arguments) {}");
    let sa = analyze_scopes(&out.program);
    assert!(
        sa.errors
            .iter()
            .any(|e| e.message.contains("arguments") && e.message.contains("strict")),
        "Expected arguments param error in module: {:?}",
        sa.errors
    );
}

#[test]
fn class_expr_name_inner_scope() {
    // Class expression name should be in inner scope (not outer)
    let out = parse_script("const C = class MyClass { method() { return MyClass; } };");
    let sa = analyze_scopes(&out.program);
    // MyClass should NOT be in global scope
    assert!(
        !sa.scopes[0]
            .bindings
            .iter()
            .any(|b| r(&out.program, b.name) == "MyClass"),
        "MyClass should not be in global scope: {:?}",
        sa.scopes[0].bindings
    );
    // MyClass should be in the class inner scope (Block scope)
    let inner = sa.scopes.iter().find(|s| {
        s.kind == ScopeKind::Block
            && s.bindings
                .iter()
                .any(|b| r(&out.program, b.name) == "MyClass" && b.kind == BindingKind::Class)
    });
    assert!(
        inner.is_some(),
        "Expected inner scope with MyClass binding: {:?}",
        sa.scopes
    );
}

#[test]
fn class_decl_name_outer_scope() {
    // Class declaration name should be in outer (global) scope
    let out = parse_script("class MyClass {}");
    let sa = analyze_scopes(&out.program);
    assert!(
        sa.scopes[0]
            .bindings
            .iter()
            .any(|b| r(&out.program, b.name) == "MyClass" && b.kind == BindingKind::Class),
        "MyClass should be in global scope: {:?}",
        sa.scopes[0].bindings
    );
}

// ── B8: export name uniqueness ──

#[test]
fn duplicate_export_name_error() {
    let out = parse_module("const a = 1; export { a }; export { a };");
    let sa = analyze_scopes(&out.program);
    assert!(
        sa.errors
            .iter()
            .any(|e| matches!(e.kind, JsParseErrorKind::DuplicateBinding)
                && e.message.contains("'a'")),
        "Expected duplicate export error: {:?}",
        sa.errors
    );
}

#[test]
fn unique_export_names_ok() {
    let out = parse_module("const a = 1, b = 2; export { a, b };");
    let sa = analyze_scopes(&out.program);
    assert!(
        !sa.errors
            .iter()
            .any(|e| matches!(e.kind, JsParseErrorKind::DuplicateBinding)),
        "Unique exports should not error: {:?}",
        sa.errors
    );
}

// ── M3: duplicate default export detection ──

#[test]
fn duplicate_default_export_error() {
    let out = parse_module("export default 1; export default 2;");
    let sa = analyze_scopes(&out.program);
    assert!(
        sa.errors
            .iter()
            .any(|e| e.message.contains("Duplicate export name 'default'")),
        "Expected duplicate default export error: {:?}",
        sa.errors
    );
}

#[test]
fn single_default_export_ok() {
    let out = parse_module("export default function f() {}");
    let sa = analyze_scopes(&out.program);
    assert!(
        !sa.errors.iter().any(|e| e.message.contains("default")),
        "Single default export should not error: {:?}",
        sa.errors
    );
}

// ── A2: export declaration duplicate detection ──

#[test]
fn export_const_duplicate_error() {
    let out = parse_module("export const x = 1; export { x };");
    let sa = analyze_scopes(&out.program);
    assert!(
        sa.errors
            .iter()
            .any(|e| e.message.contains("Duplicate export name 'x'")),
        "Expected duplicate export error: {:?}",
        sa.errors
    );
}

#[test]
fn export_function_duplicate_error() {
    let out = parse_module("export function f() {} export { f };");
    let sa = analyze_scopes(&out.program);
    assert!(
        sa.errors
            .iter()
            .any(|e| e.message.contains("Duplicate export name 'f'")),
        "Expected duplicate export error: {:?}",
        sa.errors
    );
}

#[test]
fn export_const_unique_ok() {
    let out = parse_module("export const x = 1; export const y = 2;");
    let sa = analyze_scopes(&out.program);
    assert!(
        !sa.errors
            .iter()
            .any(|e| e.message.contains("Duplicate export")),
        "Unique export declarations should not error: {:?}",
        sa.errors
    );
}

// ── A3: export * as ns duplicate detection ──

#[test]
fn export_namespace_duplicate_error() {
    let out = parse_module("export * as ns from 'a'; export * as ns from 'b';");
    let sa = analyze_scopes(&out.program);
    assert!(
        sa.errors
            .iter()
            .any(|e| e.message.contains("Duplicate export name 'ns'")),
        "Expected duplicate namespace export error: {:?}",
        sa.errors
    );
}

// ── B9: class body always strict ──

#[test]
fn class_body_always_strict() {
    // In sloppy script, class body should still be strict
    let out = parse_script("class C { method(eval) {} }");
    let sa = analyze_scopes(&out.program);
    assert!(
        sa.errors
            .iter()
            .any(|e| e.message.contains("eval") && e.message.contains("strict")),
        "Class method should be strict even in sloppy script: {:?}",
        sa.errors
    );
}

// ── E6: unnamed class expression gets Block scope ──

#[test]
fn unnamed_class_expression_scope() {
    let out = parse_script("let C = class { method() {} };");
    let sa = analyze_scopes(&out.program);
    // Should have a Block scope for the unnamed class expression body
    assert!(
        sa.scopes.iter().any(|s| s.kind == ScopeKind::Block),
        "Expected Block scope for unnamed class: {:?}",
        sa.scopes.iter().map(|s| &s.kind).collect::<Vec<_>>()
    );
}

// ── E13: function expression name in inner scope ──

#[test]
fn function_expression_name_inner_scope() {
    // Named function expression: name should be in function's own scope, not outer
    let out = parse_script("let f = function myFunc() { myFunc; };");
    let sa = analyze_scopes(&out.program);
    // The function scope should have a binding for "myFunc"
    let func_scope = sa.scopes.iter().find(|s| {
        s.kind == ScopeKind::Function
            && s.bindings
                .iter()
                .any(|b| r(&out.program, b.name) == "myFunc")
    });
    assert!(
        func_scope.is_some(),
        "Function expression name should be bound in function scope: {:?}",
        sa.scopes
            .iter()
            .map(|s| (&s.kind, &s.bindings))
            .collect::<Vec<_>>()
    );
}

#[test]
fn function_declaration_name_outer_scope() {
    // Function declaration: name should be in outer scope
    let out = parse_script("function myFunc() {}");
    let sa = analyze_scopes(&out.program);
    // The global scope should have a binding for "myFunc"
    assert!(
        sa.scopes[0]
            .bindings
            .iter()
            .any(|b| r(&out.program, b.name) == "myFunc"),
        "Function declaration name should be in global scope: {:?}",
        sa.scopes[0].bindings
    );
}

// ── S4: "use strict" + non-simple params ──

#[test]
fn use_strict_non_simple_params_error() {
    let out = parse_script("function f(a = 1) { 'use strict'; }");
    let sa = analyze_scopes(&out.program);
    assert!(
        sa.errors
            .iter()
            .any(|e| e.message.contains("non-simple parameter")),
        "Expected non-simple params + use strict error: {:?}",
        sa.errors
    );
}

#[test]
fn use_strict_simple_params_ok() {
    let out = parse_script("function f(a, b) { 'use strict'; }");
    let sa = analyze_scopes(&out.program);
    assert!(
        !sa.errors
            .iter()
            .any(|e| e.message.contains("non-simple parameter")),
        "Simple params should not trigger error: {:?}",
        sa.errors
    );
}

#[test]
fn use_strict_rest_params_error() {
    let out = parse_script("function f(...args) { 'use strict'; }");
    let sa = analyze_scopes(&out.program);
    assert!(
        sa.errors
            .iter()
            .any(|e| e.message.contains("non-simple parameter")),
        "Expected non-simple params error for rest: {:?}",
        sa.errors
    );
}

#[test]
fn use_strict_destructured_params_error() {
    let out = parse_script("function f({a}) { 'use strict'; }");
    let sa = analyze_scopes(&out.program);
    assert!(
        sa.errors
            .iter()
            .any(|e| e.message.contains("non-simple parameter")),
        "Expected non-simple params error for destructuring: {:?}",
        sa.errors
    );
}

// ── E6: duplicate function declarations in strict mode ──

#[test]
fn duplicate_function_declaration_error() {
    let out = crate::parse_script("function f() {} function f() {}");
    let sa = analyze(&out.program);
    assert!(
        sa.errors.iter().any(|e| e.message.contains("Duplicate")),
        "Expected duplicate function error: {:?}",
        sa.errors
    );
}

#[test]
fn function_declaration_unique_ok() {
    let out = crate::parse_script("function f() {} function g() {}");
    let sa = analyze(&out.program);
    assert!(sa.errors.is_empty(), "{:?}", sa.errors);
}

// ── E8: function eval/arguments in strict mode ──

#[test]
fn function_eval_name_error() {
    // E8: function named `eval` is forbidden in strict mode
    let out = crate::parse_script("\"use strict\"; function eval() {}");
    let sa = analyze(&out.program);
    // Error may come from parser (check_reserved_binding) or scope analysis
    let has_error = sa.errors.iter().any(|e| e.message.contains("eval"))
        || out.errors.iter().any(|e| e.message.contains("eval"));
    assert!(
        has_error,
        "Expected eval binding error: parser={:?}, scope={:?}",
        out.errors, sa.errors
    );
}

#[test]
fn function_arguments_name_error() {
    // E8: function named `arguments` is forbidden in strict mode
    let out = crate::parse_script("\"use strict\"; function arguments() {}");
    let sa = analyze(&out.program);
    let has_error = sa.errors.iter().any(|e| e.message.contains("arguments"))
        || out.errors.iter().any(|e| e.message.contains("arguments"));
    assert!(
        has_error,
        "Expected arguments binding error: parser={:?}, scope={:?}",
        out.errors, sa.errors
    );
}
