//! Tests for the D-17b CE-minimal class machinery: `class extends`
//! super-class chain, `Op::SuperCall` / `Op::SuperCallSpread` /
//! `Op::NewTarget`, default-derived constructor synthesis.
//!
//! Spec citations: [C13] ECMA-262 §13.3.7.1 SuperCall + [C16]
//! §15.7.14 ClassDefinitionEvaluation + [C11] §10.2.2 [[Construct]].
//! Covers the JS-side class layer in isolation; HTMLElement
//! integration tests live in `tests_custom_elements.rs`.

#[test]
fn extends_chains_static_to_super() {
    // [C16] constructorParent: `B.__proto__ === A` (so static
    // methods inherit from the super class).
    assert!(super::eval_bool(
        "class A {} class B extends A {} Object.getPrototypeOf(B) === A;"
    ));
}

#[test]
fn extends_chains_prototype_to_super_prototype() {
    // [C16] protoParent: `B.prototype.__proto__ === A.prototype` (so
    // instance methods inherit).
    assert!(super::eval_bool(
        "class A {} class B extends A {} \
         Object.getPrototypeOf(B.prototype) === A.prototype;"
    ));
}

#[test]
fn extends_default_derived_ctor_propagates_args_to_super() {
    // [C16] default derived ctor = `constructor(...args) { super(...args); }`.
    // Rest-param packing (Stage 0) + SuperCallSpread (Stage 3)
    // must propagate the call arguments to the super constructor.
    assert_eq!(
        super::eval_number(
            "class A { constructor(x) { this.x = x; } } \
             class B extends A {} \
             let b = new B(42); b.x;"
        ),
        42.0
    );
}

#[test]
fn extends_user_ctor_with_explicit_super_call() {
    // User-written ctor: `super(...)` must reach the super class.
    assert_eq!(
        super::eval_number(
            "class A { constructor(n) { this.value = n * 2; } } \
             class B extends A { constructor() { super(5); } } \
             let b = new B(); b.value;"
        ),
        10.0
    );
}

#[test]
fn instance_method_on_subclass_calls_super_method() {
    // Instance method on B reaches A.prototype.method via
    // B.prototype.__proto__ → A.prototype lookup.
    assert_eq!(
        super::eval_number(
            "class A { foo() { return 7; } } \
             class B extends A {} \
             let b = new B(); b.foo();"
        ),
        7.0
    );
}

#[test]
fn instance_of_super_class() {
    // `instanceof` walks the [[Prototype]] chain — derived
    // instances satisfy both `MyEl instanceof B` and
    // `instanceof A` ([C16] ClassDefinitionEvaluation chains both
    // sides).
    assert!(super::eval_bool(
        "class A {} class B extends A {} \
         let b = new B(); b instanceof A && b instanceof B;"
    ));
}

#[test]
fn new_target_inside_ctor_is_callee() {
    // [C11] step 4 — `new.target` is the constructor invoked at
    // the top of the `new` chain.
    assert!(super::eval_bool(
        "class A { constructor() { globalThis.__nt = new.target; } } \
         new A(); globalThis.__nt === A;"
    ));
}

#[test]
fn new_target_inside_super_is_outer_class() {
    // [C13] propagation — `super()` carries the outer
    // class's NewTarget unchanged, so the inner ctor body sees
    // the outermost-invoked class.
    assert!(super::eval_bool(
        "class A { constructor() { globalThis.__nt = new.target; } } \
         class B extends A {} \
         new B(); globalThis.__nt === B;"
    ));
}

#[test]
fn new_target_outside_new_is_undefined() {
    // Plain `[[Call]]` → no construct context → `new.target`
    // returns Undefined.
    assert!(super::eval_bool(
        "function f() { return new.target === undefined; } f();"
    ));
}

#[test]
fn super_outside_class_ctor_throws_syntax_error() {
    // `super(...)` only valid in a derived class constructor
    // body — anywhere else throws (the parser may catch some
    // cases at parse time; this exercises the runtime
    // home_class=None branch).
    super::eval_throws("function f() { super(); } f();");
}

#[test]
fn super_call_spread_propagates_argv() {
    // [C19] ArgumentListEvaluation spread variant — Op::SuperCallSpread.
    assert_eq!(
        super::eval_number(
            "class A { constructor(a, b, c) { this.sum = a + b + c; } } \
             class B extends A { constructor() { let xs = [1, 2, 3]; super(...xs); } } \
             let b = new B(); b.sum;"
        ),
        6.0
    );
}

#[test]
fn class_without_extends_remains_base() {
    // Sanity: classes without `extends` get the default base
    // ctor (existing path, unchanged) — the `__proto__` of the
    // prototype is Object's prototype (the empty class lives on
    // the ordinary Object chain). Walk `__proto__` twice and
    // confirm we reach `null` (proto-of-Object.prototype).
    assert!(super::eval_bool(
        "class A {} \
         let p = Object.getPrototypeOf(A.prototype); \
         Object.getPrototypeOf(p) === null;"
    ));
}

// ---------------------------------------------------------------------------
// construct_synchronous error-path coverage (D-17b Risk #4 from
// /review: a user ctor body that throws inside a nested catch must
// not leave construct_synchronous's frame state inconsistent. The
// RAII guard in invoke_upgrade closes the CE side; this test
// exercises the JS-construct-path branch via direct `new`.)
// ---------------------------------------------------------------------------

#[test]
fn class_ctor_throws_inside_try_caught_outer() {
    // class B extends A { constructor() { super(); try { throw 1; } catch (e) {} } }
    // The ctor body's try/catch swallows the throw — construct
    // returns normally with `this` substituted.
    assert!(super::eval_bool(
        "class A { constructor() { this.x = 1; } } \
         class B extends A { constructor() { super(); try { throw new Error('inner'); } catch (e) {} } } \
         let b = new B(); b.x === 1;"
    ));
}

#[test]
fn class_ctor_uncaught_throw_propagates_out_of_new() {
    // Uncaught throw inside the ctor body propagates through
    // construct_synchronous → do_new → Op::New → outer try/catch.
    // Tests that the construct_synchronous error path correctly
    // unwinds the JS frame + pops native_construct_stack.
    assert!(super::eval_bool(
        "let caught = false; \
         class B extends Error { constructor() { super(); throw new RangeError('boom'); } } \
         try { new B(); } catch (e) { caught = (e instanceof RangeError); } \
         caught;"
    ));
}

#[test]
fn nested_class_new_after_outer_throw_recovers() {
    // After an outer construct throws, a subsequent construct of a
    // sibling class should succeed cleanly — verifies the global
    // native_construct_stack / completion_value / GC state was
    // properly restored on the failure path.
    assert!(super::eval_bool(
        "class A {} class B extends A { constructor() { throw 1; } } \
         try { new B(); } catch (e) {} \
         class C extends A {} \
         let c = new C(); c instanceof A;"
    ));
}

// ---------------------------------------------------------------------------
// D-17b R4 G4-2 / G4-3: `extends null` (ECMA-262 §15.7.14 step 6.f).
// `protoParent = null`, `constructorParent = %Function.prototype%`.
// Default ctor is BASE (no super call); user-written super() resolves
// to %Function.prototype% and throws TypeError on Construct.
// ---------------------------------------------------------------------------

#[test]
fn extends_null_default_ctor_is_base() {
    // `class X extends null {}` with no user ctor must construct
    // cleanly via a BASE default ctor (no super() call). Previously
    // synthesized a DERIVED ctor that called super(...args) on null
    // → TypeError at runtime.
    assert!(super::eval_bool(
        "class X extends null {} let x = new X(); x instanceof X;"
    ));
}

#[test]
fn extends_null_prototype_proto_is_null() {
    // [C16] step 6.f.iii: protoParent = null →
    // `Object.getPrototypeOf(X.prototype) === null`.
    assert!(super::eval_bool(
        "class X extends null {} Object.getPrototypeOf(X.prototype) === null;"
    ));
}

#[test]
fn extends_null_ctor_proto_is_function_prototype() {
    // [C16] step 6.f.ii: constructorParent = %Function.prototype% →
    // `Object.getPrototypeOf(X) === Function.prototype` (NOT null).
    // The VM doesn't install a `Function` global, so probe
    // %Function.prototype% via a function literal's [[Prototype]].
    assert!(super::eval_bool(
        "class X extends null {} \
         Object.getPrototypeOf(X) === Object.getPrototypeOf(function(){});"
    ));
}

#[test]
fn extends_null_user_super_call_throws() {
    // [C13] GetSuperConstructor resolves home_class.[[Prototype]] =
    // %Function.prototype%; Construct(%Function.prototype%, ...)
    // throws TypeError (Function.prototype is not constructable).
    assert!(super::eval_bool(
        "class X extends null { constructor() { super(); } } \
         let caught = false; \
         try { new X(); } catch (e) { caught = (e instanceof TypeError); } \
         caught;"
    ));
}

// ---------------------------------------------------------------------------
// D-17b R4 G4-4: BoundFunction in [[Construct]] dispatch. `do_new`
// and `construct_synchronous` share `unwrap_bound_function_chain`
// (ECMA-262 §10.4.1.2 Bound Function Exotic Objects [[Construct]]) so the two paths
// can't drift on bound-chain unwrapping. The user-visible surface
// for the divergence today is small — `class B extends Bound`
// throws at class-def time per spec (Bound has no `prototype`
// property → protoParent is undefined → TypeError), matching V8 —
// so the shared helper protects against future construct callers
// (e.g. a `Reflect.construct(Bound, args, NewTarget)` path) seeing
// the inconsistency.
// ---------------------------------------------------------------------------

#[test]
fn extends_non_constructor_throws_at_class_definition() {
    // ECMA-262 §15.7.14 step 6.f ClassDefinitionEvaluation: if the
    // heritage value is callable but lacks [[Construct]] (Symbol,
    // BigInt, arrow fn, etc.), throw TypeError AT class-definition
    // time — not later at super() dispatch. The previous
    // implementation only checked "Object or Null" inside the
    // SetPrototype splice, so `class B extends Symbol {}` defined
    // successfully and only failed at construct time. D-17b R17 G17-1
    // adds `Op::AssertConstructor` emitted by `compile_class` for the
    // ClassHeritage::Expr arm.
    super::eval_throws("class B extends Symbol {}");
}

#[test]
fn extends_arrow_function_throws_at_class_definition() {
    // Same theme: arrow functions are callable but NOT constructable
    // per ECMA-262 §10.2.1 [[Construct]] — extending one must throw
    // at class-definition time.
    super::eval_throws("class B extends (() => {}) {}");
}

#[test]
fn extends_bound_constructor_throws_on_undefined_prototype() {
    // Spec/V8 alignment: `class B extends A.bind(null, 7)` throws
    // TypeError because BoundFunction objects have no `prototype`
    // own property; `Get(Bound, "prototype")` returns undefined,
    // which fails the protoParent Object/Null check (ECMA-262
    // §15.7.14 step 6.f.ii). The SetPrototype dispatcher surfaces
    // a spec-aligned "Class extends value is not a constructor or
    // null" diagnostic.
    super::eval_throws(
        "class A { constructor(x, y) { this.sum = x + y; } } \
         class B extends A.bind(null, 7) { constructor(y) { super(y); } } \
         new B(3);",
    );
}

#[test]
fn extends_heritage_expression_evaluated_once() {
    // ECMA-262 §15.7.14 step 6 ClassDefinitionEvaluation: the
    // ClassHeritage expression must be evaluated exactly once. The
    // compiler previously emitted `compile_expr(super_id)` twice —
    // once for `ctor.prototype.__proto__ = super.prototype` and
    // once for `ctor.__proto__ = super` — duplicating side effects
    // for non-trivial heritage expressions like a call (D-17b R6 G6-1).
    assert_eq!(
        super::eval_number(
            "let count = 0; \
             function H() { count++; return class A {}; } \
             class B extends H() {} \
             count;"
        ),
        1.0
    );
}

#[test]
fn new_bound_constructor_via_do_new_still_works() {
    // `new BoundCtor(...)` itself (no class heritage) continues to
    // unwrap via `do_new` → `unwrap_bound_function_chain` and
    // dispatch to the inner constructor with bound args prepended.
    // Regression for the shared-helper refactor: `do_new`'s
    // existing behavior must survive the extraction.
    assert_eq!(
        super::eval_number(
            "class A { constructor(x, y) { this.sum = x + y; } } \
             let Bound = A.bind(null, 7); \
             let b = new Bound(3); b.sum;"
        ),
        10.0
    );
}
