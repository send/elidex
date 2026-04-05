use crate::ast::*;
use crate::atom::Atom;
use crate::parse_script;

fn r(prog: &Program, atom: Atom) -> String {
    prog.interner.get_utf8(atom)
}

#[test]
fn function_declaration() {
    let out = parse_script("function foo(a, b) { return a + b; }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    if let StmtKind::FunctionDeclaration(f) = &out.program.stmts.get(out.program.body[0]).kind {
        assert_eq!(f.name.map(|a| r(&out.program, a)), Some("foo".to_string()));
        assert_eq!(f.params.len(), 2);
        assert!(!f.is_async);
        assert!(!f.is_generator);
    } else {
        panic!("Expected function declaration");
    }
}

#[test]
fn generator_function() {
    let out = parse_script("function* gen() { }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    if let StmtKind::FunctionDeclaration(f) = &out.program.stmts.get(out.program.body[0]).kind {
        assert!(f.is_generator);
    } else {
        panic!("Expected function declaration");
    }
}

#[test]
fn async_function() {
    let out = parse_script("async function fetchData() { }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    if let StmtKind::FunctionDeclaration(f) = &out.program.stmts.get(out.program.body[0]).kind {
        assert!(f.is_async);
        assert_eq!(
            f.name.map(|a| r(&out.program, a)),
            Some("fetchData".to_string())
        );
    } else {
        panic!("Expected async function declaration");
    }
}

#[test]
fn class_declaration() {
    let out = parse_script("class Foo extends Bar { constructor() {} method() {} }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    if let StmtKind::ClassDeclaration(c) = &out.program.stmts.get(out.program.body[0]).kind {
        assert_eq!(c.name.map(|a| r(&out.program, a)), Some("Foo".to_string()));
        assert!(c.super_class.is_some());
        assert_eq!(c.body.len(), 2);
    } else {
        panic!("Expected class declaration");
    }
}

#[test]
fn class_private_fields() {
    let out = parse_script("class C { #x = 1; #method() { return this.#x; } }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    if let StmtKind::ClassDeclaration(c) = &out.program.stmts.get(out.program.body[0]).kind {
        assert!(matches!(
            c.body[0].kind,
            ClassMemberKind::PrivateField { .. }
        ));
        assert!(matches!(
            c.body[1].kind,
            ClassMemberKind::PrivateMethod { .. }
        ));
    } else {
        panic!("Expected class");
    }
}

#[test]
fn class_static_block() {
    let out = parse_script("class C { static { let x = 1; } }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    if let StmtKind::ClassDeclaration(c) = &out.program.stmts.get(out.program.body[0]).kind {
        assert!(matches!(c.body[0].kind, ClassMemberKind::StaticBlock(_)));
    } else {
        panic!("Expected class with static block");
    }
}

#[test]
fn function_default_params() {
    let out = parse_script("function f(a, b = 1, ...rest) {}");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    if let StmtKind::FunctionDeclaration(f) = &out.program.stmts.get(out.program.body[0]).kind {
        assert_eq!(f.params.len(), 3);
        assert!(f.params[1].default.is_some());
        assert!(f.params[2].rest);
    } else {
        panic!("Expected function");
    }
}

#[test]
fn class_getter_setter() {
    let out = parse_script("class C { get x() { return 1; } set x(v) {} }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    if let StmtKind::ClassDeclaration(c) = &out.program.stmts.get(out.program.body[0]).kind {
        if let ClassMemberKind::Method { kind, .. } = &c.body[0].kind {
            assert_eq!(*kind, MethodKind::Get);
        }
        if let ClassMemberKind::Method { kind, .. } = &c.body[1].kind {
            assert_eq!(*kind, MethodKind::Set);
        }
    } else {
        panic!("Expected class");
    }
}

#[test]
fn class_computed_key() {
    let out = parse_script("class C { [Symbol.iterator]() {} }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    if let StmtKind::ClassDeclaration(c) = &out.program.stmts.get(out.program.body[0]).kind {
        if let ClassMemberKind::Method { computed, .. } = &c.body[0].kind {
            assert!(computed);
        }
    }
}

#[test]
fn trailing_comma_params() {
    let out = parse_script("function f(a, b,) {}");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

// ── H8-H9: class member get/set/async as method names ──

#[test]
fn class_method_named_get() {
    // `get()` should be a normal method, not a getter prefix
    let out = parse_script("class C { get() {} }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    if let StmtKind::ClassDeclaration(c) = &out.program.stmts.get(out.program.body[0]).kind {
        if let ClassMemberKind::Method { kind, key, .. } = &c.body[0].kind {
            assert_eq!(*kind, MethodKind::Method);
            assert!(
                matches!(key, PropertyKey::Identifier(name) if r(&out.program, *name) == "get")
            );
        } else {
            panic!("Expected method");
        }
    } else {
        panic!("Expected class");
    }
}

#[test]
fn class_method_named_async() {
    // `async()` should be a normal method, not an async prefix
    let out = parse_script("class C { async() {} }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    if let StmtKind::ClassDeclaration(c) = &out.program.stmts.get(out.program.body[0]).kind {
        if let ClassMemberKind::Method { key, .. } = &c.body[0].kind {
            assert!(
                matches!(key, PropertyKey::Identifier(name) if r(&out.program, *name) == "async")
            );
        } else {
            panic!("Expected method");
        }
    } else {
        panic!("Expected class");
    }
}

#[test]
fn class_get_field() {
    // `get = 1;` should be a field, not a getter
    let out = parse_script("class C { get = 1; }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    if let StmtKind::ClassDeclaration(c) = &out.program.stmts.get(out.program.body[0]).kind {
        assert!(matches!(c.body[0].kind, ClassMemberKind::Property { .. }));
    } else {
        panic!("Expected class");
    }
}

// ── Step 4: B7 — const without initializer ──

#[test]
fn const_without_init_error() {
    let out = parse_script("const x;");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("initializer") && e.message.contains("const")),
        "Expected const initializer error: {:?}",
        out.errors
    );
}

// ── Step 4: B10 — duplicate parameter names ──

#[test]
fn duplicate_param_error() {
    let out = parse_script("function f(a, a) {}");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("Duplicate parameter")),
        "Expected duplicate param error: {:?}",
        out.errors
    );
}

// ── Step 4: B12 — rest parameter with default ──

#[test]
fn rest_param_default_error() {
    let out = parse_script("function f(...rest = []) {}");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("Rest parameter") && e.message.contains("default")),
        "Expected rest default error: {:?}",
        out.errors
    );
}

// ── Step 4: B13 — duplicate constructor ──

#[test]
fn duplicate_constructor_error() {
    let out = parse_script("class C { constructor() {} constructor() {} }");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("one constructor")),
        "Expected duplicate constructor error: {:?}",
        out.errors
    );
}

// ── Step 4: B14 — constructor + generator/async ──

#[test]
fn constructor_generator_error() {
    let out = parse_script("class C { *constructor() {} }");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("Constructor") && e.message.contains("generator")),
        "Expected constructor generator error: {:?}",
        out.errors
    );
}

#[test]
fn constructor_async_error() {
    let out = parse_script("class C { async constructor() {} }");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("Constructor") && e.message.contains("async")),
        "Expected constructor async error: {:?}",
        out.errors
    );
}

// ── Step 4: B15 — #constructor ──

#[test]
fn private_constructor_error() {
    let out = parse_script("class C { #constructor() {} }");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("#constructor")),
        "Expected #constructor error: {:?}",
        out.errors
    );
}

// ── Step 4: B16 — static prototype method ──

#[test]
fn static_prototype_error() {
    let out = parse_script("class C { static prototype() {} }");
    assert!(
        out.errors.iter().any(|e| e.message.contains("prototype")),
        "Expected static prototype error: {:?}",
        out.errors
    );
}

// ── Step 4: A7 — static as field name ──

#[test]
fn static_as_field_name() {
    let out = parse_script("class C { static = 1; }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    if let StmtKind::ClassDeclaration(c) = &out.program.stmts.get(out.program.body[0]).kind {
        if let ClassMemberKind::Property { key, is_static, .. } = &c.body[0].kind {
            assert!(!is_static, "static should not be a modifier here");
            assert!(
                matches!(key, PropertyKey::Identifier(n) if r(&out.program, *n) == "static"),
                "key should be 'static'"
            );
        } else {
            panic!("Expected property");
        }
    } else {
        panic!("Expected class");
    }
}

// ── A17: export async NLT ──

#[test]
fn export_async_function_ok() {
    let out = crate::parse_module("export async function f() {}");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

#[test]
fn export_async_newline_function_error() {
    let out = crate::parse_module("export async\nfunction f() {}");
    assert!(
        !out.errors.is_empty(),
        "Expected error for async\\nfunction export"
    );
}

// ── A18: duplicate destructured params ──

#[test]
fn duplicate_destructured_param_error() {
    let out = parse_script("function f({x}, {x}) {}");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("Duplicate parameter")),
        "Expected duplicate destructured param error: {:?}",
        out.errors
    );
}

#[test]
fn duplicate_nested_param_error() {
    let out = parse_script("function f([a, {a}]) {}");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("Duplicate parameter")),
        "Expected duplicate nested param error: {:?}",
        out.errors
    );
}

#[test]
fn unique_destructured_params_ok() {
    let out = parse_script("function f({a}, {b}) {}");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

// ── Step 4: B9 — super outside function ──

#[test]
fn super_outside_function_error() {
    let out = parse_script("super.method();");
    assert!(
        out.errors.iter().any(|e| e.message.contains("super")),
        "Expected super error: {:?}",
        out.errors
    );
}

// ── B22: static block return prohibition ──

#[test]
fn static_block_return_error() {
    let out = parse_script("class C { static { return; } }");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("return") || e.message.contains("Return")),
        "Expected return in static block error: {:?}",
        out.errors
    );
}

// ── B-new1: getter/setter parameter count ──

#[test]
fn getter_no_params_ok() {
    let out = parse_script("class C { get x() { return 1; } }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

#[test]
fn getter_with_params_error() {
    let out = parse_script("class C { get x(v) { return v; } }");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("Getter") && e.message.contains("no parameters")),
        "Expected getter params error: {:?}",
        out.errors
    );
}

#[test]
fn setter_one_param_ok() {
    let out = parse_script("class C { set x(v) {} }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

#[test]
fn setter_zero_params_error() {
    let out = parse_script("class C { set x() {} }");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("Setter") && e.message.contains("exactly one")),
        "Expected setter params error: {:?}",
        out.errors
    );
}

#[test]
fn setter_two_params_error() {
    let out = parse_script("class C { set x(a, b) {} }");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("Setter") && e.message.contains("exactly one")),
        "Expected setter params error: {:?}",
        out.errors
    );
}

// ── B-new1: object literal getter/setter ──

#[test]
fn object_getter_with_params_error() {
    let out = parse_script("var o = { get x(v) { return v; } };");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("Getter") && e.message.contains("no parameters")),
        "Expected object getter params error: {:?}",
        out.errors
    );
}

#[test]
fn object_setter_zero_params_error() {
    let out = parse_script("var o = { set x() {} };");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("Setter") && e.message.contains("exactly one")),
        "Expected object setter params error: {:?}",
        out.errors
    );
}

// ── B-new2: class async NLT ──

#[test]
fn async_newline_class_method_is_property() {
    // `async\n method() {}` — `async` should be treated as a property name
    let out = parse_script("class C { async\n method() {} }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    if let StmtKind::ClassDeclaration(c) = &out.program.stmts.get(out.program.body[0]).kind {
        // First member should be `async` (field/property), not an async method
        assert!(
            c.body.len() >= 2,
            "Expected 2+ members, got {}",
            c.body.len()
        );
    } else {
        panic!("Expected class");
    }
}

// ── B1: static block does not inherit loop/switch context ──

#[test]
fn static_block_break_error() {
    // `break` inside a static block inside a loop should error
    let out = parse_script("while (true) { class C { static { break; } } }");
    assert!(
        out.errors.iter().any(|e| e.message.contains("outside")),
        "Expected break error in static block: {:?}",
        out.errors
    );
}

#[test]
fn static_block_continue_error() {
    // `continue` inside a static block inside a loop should error
    let out = parse_script("while (true) { class C { static { continue; } } }");
    assert!(
        out.errors.iter().any(|e| e.message.contains("outside")),
        "Expected continue error in static block: {:?}",
        out.errors
    );
}

// ── E3: static field with NLT should not be confused ──

#[test]
fn static_field_declaration() {
    let out = parse_script("class C { static x = 1; }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

// ── E7: get/set constructor rejection ──

#[test]
fn getter_constructor_error() {
    let out = parse_script("class C { get constructor() { return 1; } }");
    assert!(
        out.errors.iter().any(|e| e.message.contains("constructor")),
        "Expected get constructor error: {:?}",
        out.errors
    );
}

#[test]
fn setter_constructor_error() {
    let out = parse_script("class C { set constructor(v) {} }");
    assert!(
        out.errors.iter().any(|e| e.message.contains("constructor")),
        "Expected set constructor error: {:?}",
        out.errors
    );
}

#[test]
fn normal_constructor_ok() {
    let out = parse_script("class C { constructor() {} }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

// ── E8: duplicate private names ──

#[test]
fn duplicate_private_field_error() {
    let out = parse_script("class C { #x; #x; }");
    assert!(
        out.errors.iter().any(|e| e.message.contains("Duplicate")),
        "Expected duplicate private name error: {:?}",
        out.errors
    );
}

#[test]
fn private_getter_setter_pair_ok() {
    // get #x and set #x should not conflict
    let out = parse_script("class C { get #x() { return 1; } set #x(v) {} }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

#[test]
fn duplicate_private_getter_error() {
    let out = parse_script("class C { get #x() {} get #x() {} }");
    assert!(
        out.errors.iter().any(|e| e.message.contains("Duplicate")),
        "Expected duplicate private getter error: {:?}",
        out.errors
    );
}

// ── E3: yield in generator parameter defaults ──

#[test]
fn yield_in_generator_param_default_error() {
    // E3: In a nested generator, the outer `in_generator` context leaks into
    // the inner generator's param defaults. `yield` must be rejected there.
    let out = parse_script("function* outer() { function* inner(x = yield 1) {} }");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("yield") && e.message.contains("parameter")),
        "Expected yield-in-params error: {:?}",
        out.errors
    );
}

#[test]
fn yield_in_non_generator_param_ok() {
    // yield is a keyword in strict mode, so using it in non-generator params
    // would be a keyword error, not a param-context error
    let out = parse_script("function f(x = 1) {}");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

#[test]
fn await_in_async_param_default_error() {
    let out = parse_script("async function f(x = await 1) {}");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("await") && e.message.contains("parameter")),
        "Expected await-in-params error: {:?}",
        out.errors
    );
}
