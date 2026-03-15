use crate::{parse_module, parse_script};

// ── A5: for-in/of multiple declarators ──

#[test]
fn for_in_multiple_declarators_error() {
    let out = parse_script("for (let x, y in obj) {}");
    assert!(
        out.errors.iter().any(|e| e.message.contains("exactly one")),
        "Expected for-in declarator error: {:?}",
        out.errors
    );
}

#[test]
fn for_of_multiple_declarators_error() {
    let out = parse_script("for (const a, b of arr) {}");
    assert!(
        out.errors.iter().any(|e| e.message.contains("exactly one")),
        "Expected for-of declarator error: {:?}",
        out.errors
    );
}

// ── A6: switch multiple default ──

#[test]
fn switch_multiple_default_error() {
    let out = parse_script("switch (x) { default: break; default: break; }");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("Multiple 'default'")),
        "Expected multiple default error: {:?}",
        out.errors
    );
}

#[test]
fn switch_single_default_ok() {
    let out = parse_script("switch (x) { case 1: break; default: break; }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

// ── B19: delete identifier in strict mode ──

#[test]
fn delete_identifier_strict_error() {
    let out = parse_script("\"use strict\"; delete x;");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("delete") && e.message.contains("identifier")),
        "Expected delete identifier error: {:?}",
        out.errors
    );
}

#[test]
fn delete_member_ok() {
    let out = parse_script("delete obj.prop;");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

// ── A10: invalid assignment target ──

#[test]
fn invalid_assignment_target_literal() {
    let out = parse_script("1 = 2;");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("Invalid left-hand side")),
        "Expected invalid assignment target error: {:?}",
        out.errors
    );
}

#[test]
fn invalid_assignment_target_call() {
    let out = parse_script("f() = x;");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("Invalid left-hand side")),
        "Expected invalid assignment target error: {:?}",
        out.errors
    );
}

#[test]
fn valid_assignment_destructuring() {
    // Object destructuring cover grammar
    let out = parse_script("({x} = obj);");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

#[test]
fn valid_assignment_paren() {
    let out = parse_script("(x) = 1;");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

// ── A11: tagged template after optional chain ──

#[test]
fn tagged_template_after_optional_chain_error() {
    let out = parse_script("a?.b`template`;");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("Tagged template")),
        "Expected tagged template error: {:?}",
        out.errors
    );
}

// ── A7: super() only in constructors ──

#[test]
fn super_call_in_constructor_ok() {
    let out = parse_script("class A extends B { constructor() { super(); } }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

#[test]
fn super_call_in_method_error() {
    let out = parse_script("class A extends B { method() { super(); } }");
    assert!(
        out.errors.iter().any(|e| e.message.contains("super()")),
        "Expected super() error in method: {:?}",
        out.errors
    );
}

#[test]
fn super_member_in_method_ok() {
    // super.prop is allowed in any method
    let out = parse_script("class A extends B { method() { super.foo(); } }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

// ── E10: for-in/of invalid LHS ──

#[test]
fn for_in_literal_lhs_error() {
    let out = parse_script("for (1 in obj) {}");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("assignment target") || e.message.contains("left-hand")),
        "Expected invalid LHS error: {:?}",
        out.errors
    );
}

#[test]
fn for_of_call_lhs_error() {
    let out = parse_script("for (f() of arr) {}");
    assert!(
        !out.errors.is_empty(),
        "Expected error for call expression as for-of LHS"
    );
}

#[test]
fn for_in_identifier_ok() {
    let out = parse_script("for (x in obj) {}");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

// ── E11: tagged template allow_call guard removed ──

#[test]
fn tagged_template_in_new_ok() {
    // Tagged templates are part of MemberExpression, should work after new
    let out = parse_script("new foo`template`;");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

// ── P3: super in static block ──

#[test]
fn super_call_in_static_block_error() {
    // super() (SuperCall) is forbidden in static blocks — only valid in constructors
    let out = parse_script("class C extends B { static { super(); } }");
    assert!(
        out.errors.iter().any(|e| e.message.contains("super")),
        "Expected super() error in static block: {:?}",
        out.errors
    );
}

#[test]
fn super_property_in_static_block_ok() {
    // super.x (SuperProperty) is allowed in static blocks — refers to parent static members
    let out = parse_script("class C extends B { static { super.x; } }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

#[test]
fn super_in_method_ok() {
    let out = parse_script("class C extends B { method() { super.x; } }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

// ── P4: string import name requires 'as' ──

#[test]
fn string_import_name_requires_as() {
    let out = parse_module("import { 'foo' } from 'mod';");
    assert!(
        !out.errors.is_empty(),
        "String import name without 'as' should error"
    );
}

#[test]
fn string_import_name_with_as_ok() {
    let out = parse_module("import { 'foo' as bar } from 'mod';");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

// ── P5: export string local name requires 'from' ──

#[test]
fn export_string_local_requires_from() {
    let out = parse_module("export { 'foo' };");
    assert!(
        !out.errors.is_empty(),
        "Export with string local name without 'from' should error"
    );
}

#[test]
fn export_string_local_with_from_ok() {
    let out = parse_module("export { 'foo' } from 'mod';");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

// ── P6: duplicate __proto__ ──

#[test]
fn duplicate_proto_error() {
    let out = parse_script("({ __proto__: {}, __proto__: {} });");
    assert!(
        out.errors.iter().any(|e| e.message.contains("__proto__")),
        "Expected duplicate __proto__ error: {:?}",
        out.errors
    );
}

#[test]
fn single_proto_ok() {
    let out = parse_script("({ __proto__: {} });");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

#[test]
fn proto_string_key_duplicate_error() {
    let out = parse_script("({ '__proto__': {}, __proto__: {} });");
    assert!(
        out.errors.iter().any(|e| e.message.contains("__proto__")),
        "Expected duplicate __proto__ error with string key: {:?}",
        out.errors
    );
}

#[test]
fn proto_computed_no_error() {
    // Computed __proto__ doesn't count
    let out = parse_script("({ ['__proto__']: {}, __proto__: {} });");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

// ── T2: CoverInitializedName validation ──

#[test]
fn cover_init_name_in_destructuring_ok() {
    // `{x = 1}` as destructuring assignment target is valid
    let out = parse_script("({x = 1} = obj);");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

#[test]
fn cover_init_name_in_expression_error() {
    // `{x = 1}` as an object literal expression is invalid
    let out = parse_script("({x = 1});");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("Shorthand property initializer")),
        "Expected CoverInitializedName error: {:?}",
        out.errors
    );
}

#[test]
fn cover_init_name_in_call_error() {
    // `{x = 1}` passed as argument is invalid
    let out = parse_script("f({x = 1});");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("Shorthand property initializer")),
        "Expected CoverInitializedName error in call: {:?}",
        out.errors
    );
}

#[test]
fn shorthand_without_init_ok() {
    // `{x}` without initializer is a valid object literal
    let out = parse_script("({x});");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

// ── V1: Update expression target validation ──

#[test]
fn postfix_increment_literal_error() {
    let out = parse_script("5++;");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("left-hand side")),
        "Expected invalid LHS error: {:?}",
        out.errors
    );
}

#[test]
fn prefix_decrement_call_error() {
    let out = parse_script("--f();");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("left-hand side")),
        "Expected invalid LHS error: {:?}",
        out.errors
    );
}

#[test]
fn postfix_increment_identifier_ok() {
    let out = parse_script("x++;");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

#[test]
fn prefix_increment_member_ok() {
    let out = parse_script("++obj.x;");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

// ── V9: eval/arguments as assignment targets ──

#[test]
fn eval_assignment_error() {
    let out = parse_script("eval = 1;");
    assert!(
        out.errors.iter().any(|e| e.message.contains("eval")),
        "Expected strict mode eval error: {:?}",
        out.errors
    );
}

#[test]
fn arguments_increment_error() {
    let out = parse_script("arguments++;");
    assert!(
        out.errors.iter().any(|e| e.message.contains("arguments")),
        "Expected strict mode arguments error: {:?}",
        out.errors
    );
}

#[test]
fn eval_compound_assign_error() {
    let out = parse_script("eval += 1;");
    assert!(
        out.errors.iter().any(|e| e.message.contains("eval")),
        "Expected strict mode eval error: {:?}",
        out.errors
    );
}

// ── V12: Arrow function inherits super context ──

#[test]
fn arrow_in_method_super_ok() {
    // super.prop inside an arrow inside a method should be valid
    let out = parse_script("class C { m() { const f = () => super.x; } }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

#[test]
fn arrow_in_constructor_super_call_ok() {
    // super() inside an arrow inside a derived constructor
    let out = parse_script("class C extends B { constructor() { const f = () => super(); } }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

// ── V11: super() in non-derived constructor ──

#[test]
fn super_call_non_derived_error() {
    let out = parse_script("class C { constructor() { super(); } }");
    assert!(
        out.errors.iter().any(|e| e.message.contains("derived")),
        "Expected derived class error: {:?}",
        out.errors
    );
}

#[test]
fn super_call_derived_ok() {
    let out = parse_script("class C extends B { constructor() { super(); } }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

// ── V18b: Array rest trailing elements ──

#[test]
fn array_rest_trailing_arrow_error() {
    // Arrow param reinterpretation: expr→pattern detects rest-not-last
    let out = parse_script("([...a, b]) => {};");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("Rest") || e.message.contains("rest")),
        "Expected rest element error: {:?}",
        out.errors
    );
}

// ── V10: Parenthesized destructuring in arrow params ──

#[test]
fn parenthesized_destructuring_arrow_error() {
    let out = parse_script("(([a])) => {};");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("Parenthesized")),
        "Expected parenthesized destructuring error: {:?}",
        out.errors
    );
}

// ── V8: Break/continue label display ──

#[test]
fn continue_non_iteration_label_error_display() {
    // Label error should display the label name, not Atom(N)
    let out = parse_script("myLabel: if (true) { continue myLabel; }");
    let err = out
        .errors
        .iter()
        .find(|e| e.message.contains("continue"))
        .expect("Expected continue error");
    assert!(
        err.message.contains("myLabel"),
        "Error should contain label name 'myLabel', got: {}",
        err.message
    );
    assert!(
        !err.message.contains("Atom("),
        "Error should not contain raw Atom display: {}",
        err.message
    );
}

#[test]
fn undefined_label_error_display() {
    let out = parse_script("break foo;");
    let err = out
        .errors
        .iter()
        .find(|e| e.message.contains("Undefined"))
        .expect("Expected undefined label error");
    assert!(
        err.message.contains("foo"),
        "Error should contain label name 'foo', got: {}",
        err.message
    );
}
