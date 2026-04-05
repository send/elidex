use super::{eval, eval_bool, eval_number, eval_string};

// ── M4-10.1: Computed class member keys ────────────────────────────

#[test]
fn eval_computed_class_method() {
    assert_eq!(
        eval_number("const k = 'greet'; class C { [k]() { return 42; } } new C().greet();"),
        42.0,
    );
}

#[test]
fn eval_computed_class_static_property() {
    assert_eq!(
        eval_number("const k = 'val'; class C { static [k] = 99; } C.val;"),
        99.0,
    );
}

#[test]
fn eval_computed_class_prototype_method_this() {
    assert_eq!(
        eval_number(
            "const k = 'f'; class C { constructor() { this.v = 7; } [k]() { return this.v; } } new C().f();"
        ),
        7.0,
    );
}

#[test]
fn eval_class_method_not_enumerable() {
    // Class methods should not appear in Object.keys (enumerable: false per §14.3.8).
    // Note: constructor back-link uses DefineProperty (enumerable) — accepted for now.
    // We verify the user method 'foo' is NOT in the keys.
    assert_eq!(
        eval_number("class C { foo() {} } var k = Object.keys(C.prototype); var found = false; for (var i = 0; i < k.length; i++) { if (k[i] === 'foo') found = true; } found ? 1 : 0;"),
        0.0,
    );
}

#[test]
fn eval_computed_class_method_not_enumerable() {
    assert_eq!(
        eval_number("const k = 'foo'; class C { [k]() {} } var keys = Object.keys(C.prototype); var found = false; for (var i = 0; i < keys.length; i++) { if (keys[i] === 'foo') found = true; } found ? 1 : 0;"),
        0.0,
    );
}

// ── M4-10.1: Object rest computed key exclusion ────────────────────

#[test]
fn eval_object_rest_computed_key_exclusion() {
    // Computed key should be excluded from the rest object.
    assert_eq!(
        eval_number("const k = 'a'; const { [k]: v, ...rest } = { a: 1, b: 2, c: 3 }; rest.b;"),
        2.0,
    );
}

#[test]
fn eval_object_rest_computed_key_not_in_rest() {
    assert_eq!(
        eval_string("const k = 'a'; const { [k]: v, ...rest } = { a: 1, b: 2 }; typeof rest.a;"),
        "undefined",
    );
}

#[test]
fn eval_symbol_typeof() {
    assert_eq!(eval_string("typeof Symbol();"), "symbol");
}

#[test]
fn eval_symbol_description() {
    assert_eq!(eval_string("Symbol('foo').toString();"), "Symbol(foo)");
}

#[test]
fn eval_symbol_unique() {
    assert_eq!(eval_number("Symbol('a') === Symbol('a') ? 1 : 0;"), 0.0);
}

#[test]
fn eval_symbol_for_registry() {
    assert_eq!(
        eval_number("Symbol.for('x') === Symbol.for('x') ? 1 : 0;"),
        1.0,
    );
}

#[test]
fn eval_symbol_key_for() {
    assert_eq!(
        eval_string("var s = Symbol.for('test'); Symbol.keyFor(s);"),
        "test",
    );
}

#[test]
fn eval_symbol_key_for_non_registered() {
    assert_eq!(
        eval_string("typeof Symbol.keyFor(Symbol('x'));"),
        "undefined"
    );
}

#[test]
fn eval_symbol_as_property_key() {
    assert_eq!(
        eval_number("var s = Symbol('k'); var o = {}; o[s] = 42; o[s];"),
        42.0,
    );
}

#[test]
fn eval_symbol_not_in_object_keys() {
    assert_eq!(
        eval_number("var s = Symbol('k'); var o = {}; o[s] = 1; o.a = 2; Object.keys(o).length;"),
        1.0,
    );
}

#[test]
fn eval_well_known_symbol_iterator() {
    assert_eq!(eval_string("typeof Symbol.iterator;"), "symbol");
}

// ---------------------------------------------------------------------------
// Phase 8: Well-known symbol usage integration
// ---------------------------------------------------------------------------

#[test]
fn eval_symbol_has_instance() {
    assert_eq!(
        eval_number("function Foo() {} var f = new Foo(); f instanceof Foo ? 1 : 0;"),
        1.0,
    );
}

#[test]
fn eval_symbol_has_instance_custom() {
    assert_eq!(
        eval_number(
            "var Even = { [Symbol.hasInstance](x) { return x % 2 === 0; } }; 4 instanceof Even ? 1 : 0;",
        ),
        1.0,
    );
}

#[test]
fn eval_symbol_has_instance_custom_false() {
    assert_eq!(
        eval_number(
            "var Even = { [Symbol.hasInstance](x) { return x % 2 === 0; } }; 3 instanceof Even ? 1 : 0;",
        ),
        0.0,
    );
}

// ---------------------------------------------------------------------------
// Iterator protocol (Symbol.iterator)
// ---------------------------------------------------------------------------

#[test]
fn eval_custom_iterable_for_of() {
    assert_eq!(
        eval_number(
            "var obj = { [Symbol.iterator]() { var i = 0; return { next() { i++; return { value: i, done: i > 3 }; } }; } }; var sum = 0; for (var x of obj) { sum += x; } sum;",
        ),
        6.0,
    );
}

#[test]
fn eval_array_destructuring_via_iterator() {
    assert_eq!(eval_number("var [a, b, c] = [10, 20, 30]; b;"), 20.0);
}

#[test]
fn eval_array_destructuring_rest_via_iterator() {
    assert_eq!(
        eval_number("var [a, ...rest] = [1, 2, 3, 4]; rest.length;"),
        3.0,
    );
}

#[test]
fn eval_custom_iterable_spread() {
    assert_eq!(
        eval_number(
            "var obj = { [Symbol.iterator]() { var i = 0; return { next() { i++; return { value: i * 10, done: i > 2 }; } }; } }; var arr = [...obj]; arr[0] + arr[1];",
        ),
        30.0,
    );
}

// -- Symbol.toPrimitive (§7.1.1) -------------------------------------------

#[test]
fn eval_symbol_to_primitive_add() {
    assert_eq!(
        eval_number("var obj = { [Symbol.toPrimitive](hint) { return 42; } }; obj + 0;",),
        42.0,
    );
}

#[test]
fn eval_symbol_to_primitive_hint_default() {
    assert_eq!(
        eval_string("var obj = { [Symbol.toPrimitive](hint) { return hint; } }; '' + obj;",),
        "default",
    );
}

#[test]
fn eval_symbol_to_primitive_returns_object_throws() {
    let result = eval("var obj = { [Symbol.toPrimitive](hint) { return {}; } }; obj + 1;");
    assert!(result.is_err());
}

#[test]
fn eval_symbol_to_primitive_string_concat() {
    assert_eq!(
        eval_string(
            "var obj = { [Symbol.toPrimitive](hint) { return 'hello'; } }; obj + ' world';",
        ),
        "hello world",
    );
}

// -- Symbol.toStringTag (§19.1.3.6) ----------------------------------------

#[test]
fn eval_object_prototype_to_string_default() {
    assert_eq!(eval_string("({}).toString();"), "[object Object]",);
}

#[test]
fn eval_object_prototype_to_string_custom_tag() {
    assert_eq!(
        eval_string("var obj = { [Symbol.toStringTag]: 'MyTag' }; obj.toString();",),
        "[object MyTag]",
    );
}

#[test]
fn eval_object_prototype_to_string_inherits() {
    // Objects without a custom @@toStringTag get "[object Object]".
    assert_eq!(
        eval_string("var obj = {}; obj.toString();"),
        "[object Object]",
    );
}

// -- IteratorClose on break (for-of) ---------------------------------------

#[test]
fn eval_for_of_break_closes_iterator() {
    // The iterator's .return() should be called on break.
    assert_eq!(
        eval_number(
            "var closed = 0; var obj = { [Symbol.iterator]() { return { next() { return { value: 1, done: false }; }, return() { closed = 1; return { done: true }; } }; } }; for (var x of obj) { break; } closed;",
        ),
        1.0,
    );
}

#[test]
fn eval_for_of_break_without_return_method() {
    // break should work even if iterator has no .return() method.
    assert_eq!(
        eval_number(
            "var sum = 0; var obj = { [Symbol.iterator]() { var i = 0; return { next() { i++; return { value: i, done: i > 5 }; } }; } }; for (var x of obj) { sum = x; break; } sum;",
        ),
        1.0,
    );
}

#[test]
fn eval_for_of_normal_completion_does_not_close() {
    // Normal completion (exhausting iterator) should NOT call IteratorClose.
    // Per ECMA-262 §14.7.5.9, .return() is only called for abrupt completions.
    assert_eq!(
        eval_number(
            "var closed = 0; var obj = { [Symbol.iterator]() { var i = 0; return { next() { i++; return { value: i, done: i > 2 }; }, return() { closed = 1; return { done: true }; } }; } }; for (var x of obj) {} closed;",
        ),
        0.0,
    );
}

// -- Fix 1: IteratorClose on return from for-of ----------------------------

#[test]
fn eval_for_of_return_closes_iterator() {
    // return inside for-of should call .return() on the iterator.
    assert_eq!(
        eval_number(
            "var closed = 0; var obj = { [Symbol.iterator]() { return { next() { return { value: 1, done: false }; }, return() { closed = 1; return { done: true }; } }; } }; function f() { for (var x of obj) { return x; } } f(); closed;",
        ),
        1.0,
    );
}

#[test]
fn eval_for_of_return_nested_closes_all() {
    // return inside nested for-of loops should close all active iterators.
    assert_eq!(
        eval_number(
            "var c1 = 0; var c2 = 0; var o1 = { [Symbol.iterator]() { return { next() { return { value: 1, done: false }; }, return() { c1 = 1; return { done: true }; } }; } }; var o2 = { [Symbol.iterator]() { return { next() { return { value: 2, done: false }; }, return() { c2 = 1; return { done: true }; } }; } }; function f() { for (var x of o1) { for (var y of o2) { return x + y; } } } f(); c1 + c2;",
        ),
        2.0,
    );
}

// -- Fix 2: String iteration (for-of over strings) -------------------------

#[test]
fn eval_string_for_of_basic() {
    assert_eq!(
        eval_string("var s = ''; for (var ch of 'abc') { s += ch; } s;"),
        "abc",
    );
}

#[test]
fn eval_string_for_of_empty() {
    assert_eq!(
        eval_string("var s = 'X'; for (var ch of '') { s += ch; } s;"),
        "X",
    );
}

#[test]
fn eval_string_spread() {
    assert_eq!(eval_number("var a = [...'hi']; a.length;"), 2.0,);
}

#[test]
fn eval_string_destructure() {
    assert_eq!(eval_string("var [a, b, c] = 'xyz'; b;"), "y",);
}

// -- Fix 3: new Symbol() TypeError -----------------------------------------

#[test]
fn eval_new_symbol_throws() {
    let result = eval("new Symbol();");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.message.contains("not a constructor"),
        "expected constructor error, got: {err}"
    );
}

#[test]
fn eval_symbol_call_still_works() {
    // Calling Symbol() (not via new) should still work.
    assert_eq!(eval_string("typeof Symbol('test');"), "symbol");
}

// -- Fix 4: Symbol.keyFor reverse map (O(1)) --------------------------------

#[test]
fn eval_symbol_key_for_o1() {
    // Same as existing test, but verifies the reverse map path works.
    assert_eq!(
        eval_string("Symbol.for('alpha'); Symbol.for('beta'); var s = Symbol.for('alpha'); Symbol.keyFor(s);"),
        "alpha",
    );
}

// ── Class tests (moved from mod.rs) ─────────────────────────────

#[test]
fn class_declaration_basic() {
    assert_eq!(
        eval_number(
            "class Foo {
                constructor(x) { this.x = x; }
                getX() { return this.x; }
            }
            var f = new Foo(42);
            f.getX();"
        ),
        42.0
    );
}

#[test]
fn class_static_method() {
    assert_eq!(
        eval_number(
            "class Foo {
                static create(n) { return new Foo(n); }
                constructor(x) { this.x = x; }
            }
            var f = Foo.create(99);
            f.x;"
        ),
        99.0
    );
}

#[test]
fn class_default_constructor() {
    // Class with no explicit constructor should still work with `new`.
    assert_eq!(
        eval_number(
            "class Empty {}
            var e = new Empty();
            e.x = 7;
            e.x;"
        ),
        7.0
    );
}

#[test]
fn class_expression() {
    assert_eq!(
        eval_number(
            "var Foo = class {
                constructor(v) { this.v = v; }
                get() { return this.v; }
            };
            new Foo(10).get();"
        ),
        10.0
    );
}

#[test]
fn class_multiple_methods() {
    assert_eq!(
        eval_number(
            "class Calc {
                constructor(a, b) { this.a = a; this.b = b; }
                sum() { return this.a + this.b; }
                product() { return this.a * this.b; }
            }
            var c = new Calc(3, 4);
            c.sum() + c.product();"
        ),
        19.0 // 7 + 12
    );
}

#[test]
fn class_prototype_shared() {
    // Instances of the same class share methods via the prototype.
    assert_eq!(
        eval_number(
            "class Foo {
                method() { return 1; }
            }
            var a = new Foo();
            var b = new Foo();
            a.method() + b.method();"
        ),
        2.0
    );
}

#[test]
fn class_static_property() {
    assert_eq!(
        eval_number(
            "class Config {
                static defaultValue = 100;
            }
            Config.defaultValue;"
        ),
        100.0
    );
}

// ── Destructuring tests (moved from mod.rs) ─────────────────────

#[test]
fn eval_array_destructuring() {
    assert_eq!(eval_number("var [a, b] = [1, 2]; a + b;"), 3.0);
}

#[test]
fn eval_array_destructuring_skip() {
    assert_eq!(eval_number("var [, b] = [1, 2]; b;"), 2.0);
}

#[test]
fn eval_object_destructuring() {
    assert_eq!(eval_number("var {x, y} = {x: 10, y: 20}; x + y;"), 30.0);
}

#[test]
fn eval_object_destructuring_rename() {
    assert_eq!(
        eval_number("var {x: a, y: b} = {x: 10, y: 20}; a + b;"),
        30.0
    );
}

#[test]
fn eval_destructuring_default() {
    assert_eq!(eval_number("var [a, b = 5] = [1]; a + b;"), 6.0);
}

#[test]
fn eval_nested_destructuring() {
    assert_eq!(eval_number("var {a: {b}} = {a: {b: 42}}; b;"), 42.0);
}

#[test]
fn eval_array_rest() {
    assert_eq!(
        eval_number("var [a, ...rest] = [1, 2, 3]; rest.length;"),
        2.0
    );
}

#[test]
fn eval_object_rest_destructuring() {
    // rest should exclude already-destructured keys
    assert_eq!(
        eval_number("var {a, ...rest} = {a: 1, b: 2, c: 3}; rest.b + rest.c;"),
        5.0
    );
}

#[test]
fn eval_object_rest_no_excluded_key() {
    // rest should not contain 'a'
    assert!(eval_bool(
        "var {a, ...rest} = {a: 1, b: 2}; !('a' in rest);"
    ));
}

// ── Symbol property tests (moved from mod.rs) ───────────────────

#[test]
fn symbol_in_operator() {
    assert_eq!(
        eval_number("var s = Symbol(); var o = {}; o[s] = 1; s in o ? 1 : 0;"),
        1.0
    );
}

#[test]
fn symbol_delete_elem() {
    assert_eq!(
        eval_number("var s = Symbol(); var o = {}; o[s] = 1; delete o[s]; s in o ? 1 : 0;"),
        0.0
    );
}

#[test]
fn symbol_define_property() {
    assert_eq!(
        eval_number(
            "var s = Symbol(); var o = {}; Object.defineProperty(o, s, {value: 42}); o[s];"
        ),
        42.0
    );
}

#[test]
fn eval_symbol_to_string_type_error() {
    assert_eq!(
        eval_string("var r; try { '' + Symbol(); } catch(e) { r = e.message; } r;"),
        "Cannot convert a Symbol value to a string",
    );
}
