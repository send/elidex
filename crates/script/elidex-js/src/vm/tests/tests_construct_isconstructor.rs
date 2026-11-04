//! `#11-vm-construct-isconstructor-check` — ECMA-262 IsConstructor at
//! both `[[Construct]]` entries (`do_new` for `new`, and
//! `construct_synchronous` for `super(...)`, written or in the default
//! derived constructor), and the predicate they share with the class
//! heritage check and `customElements.define`.
//!
//! Plan memo: `docs/plans/2026-11-vm-isconstructor-check.md` §4.1 (the
//! cells) and §4.2 (the mutants each cell kills).  Spec, by AO name:
//! IsConstructor; EvaluateNew step 5; SuperCall Evaluation step 5;
//! ClassDefinitionEvaluation steps 8.g and 14.a.iv.3; MakeConstructor;
//! BoundFunctionCreate step 6.
//!
//! Every script runs in a fresh [`Vm`] under `catch_unwind`, so one
//! kind's failure (a debug-build frame-balance panic included) is
//! reported beside the others instead of masking them.

use super::super::value::JsValue;
use super::super::Vm;

/// One callee kind: its label, the expression that evaluates to it, and
/// the tags the cells filter on.
struct Kind {
    name: &'static str,
    src: &'static str,
    /// Its call returns without pushing a frame (§0.3 P-2): the ones
    /// whose `super(...)` over-popped the caller's frame.
    frameless: bool,
    /// A method, getter or setter: the predicate itself was wrong for
    /// these (§1.2), so `extends` accepted them.
    method_like: bool,
    /// A bound function: `class B extends X {}` refuses it at
    /// ClassDefinitionEvaluation step 8.h.ii (a bound function has no
    /// `prototype`), which is spec-correct and not the `extends` cell's.
    bound: bool,
}

const fn kind(name: &'static str, src: &'static str) -> Kind {
    Kind {
        name,
        src,
        frameless: false,
        method_like: false,
        bound: false,
    }
}

impl Kind {
    const fn frameless(mut self) -> Self {
        self.frameless = true;
        self
    }
    const fn method_like(mut self) -> Self {
        self.method_like = true;
        self
    }
    const fn bound(mut self) -> Self {
        self.bound = true;
        self
    }
}

const GENERATOR: &str = "function*(){}";
const FUNCTION_DECLARATION: &str = "(function(){ function F(){} return F; })()";

/// The 15 callee kinds that have no `[[Construct]]` (§4.1's table).
const NON_CONSTRUCTORS: &[Kind] = &[
    kind("generator", GENERATOR).frameless(),
    kind("async", "async function(){}").frameless(),
    kind("async generator", "async function*(){}").frameless(),
    kind("async arrow", "async () => {}").frameless(),
    kind("arrow", "() => {}"),
    kind("object-literal method", "({m(){}}).m").method_like(),
    kind("generator method", "({*m(){}}).m").frameless(),
    kind("async method", "({async m(){}}).m").frameless(),
    kind("class method", "(class { m(){} }).prototype.m").method_like(),
    kind("static method", "(class { static m(){} }).m").method_like(),
    kind(
        "object getter",
        "Object.getOwnPropertyDescriptor({get g(){ return 1; }}, 'g').get",
    )
    .method_like(),
    kind(
        "object setter",
        "Object.getOwnPropertyDescriptor({set s(v){}}, 's').set",
    )
    .method_like(),
    kind(
        "class getter",
        "Object.getOwnPropertyDescriptor((class { get g(){ return 1; } }).prototype, 'g').get",
    )
    .method_like(),
    kind("bound generator", "(function*(){}).bind(null)").frameless(),
    kind("bound method", "({m(){}}).m.bind(null)"),
];

/// Constructors (§4.1's controls).
const CONTROLS: &[Kind] = &[
    kind("function declaration", FUNCTION_DECLARATION),
    kind("function expression", "function(){}"),
    kind("property-value function", "({ f: function(){} }).f"),
    kind("class without constructor", "class {}"),
    kind(
        "class with written constructor",
        "class { constructor(){ this.k = 1; } }",
    ),
    kind("bound function", "(function(){}).bind(null)").bound(),
    kind("bound class", "(class {}).bind(null)").bound(),
    kind("Object", "Object"),
    kind("Error", "Error"),
];

/// Evaluate `src` in a fresh `Vm`; a string completion is returned as
/// is, anything else is described, and a panic reads `PANIC`.
fn eval_fresh(src: &str) -> String {
    std::panic::catch_unwind(|| {
        let mut vm = Vm::new();
        match vm.eval(src) {
            Ok(JsValue::String(id)) => vm.get_string(id),
            Ok(other) => format!("non-string completion {other:?}"),
            Err(e) => format!("uncaught {e:?}"),
        }
    })
    .unwrap_or_else(|_| "PANIC".to_string())
}

/// Run every `(label, src, expected)` row and fail once, listing every
/// row whose value differs.
fn assert_rows(cell: &str, rows: &[(String, String, String)]) {
    let failures: Vec<String> = rows
        .iter()
        .filter_map(|(label, src, expected)| {
            let got = eval_fresh(src);
            (got != *expected).then(|| format!("  {label}: got {got:?}, want {expected:?}"))
        })
        .collect();
    assert!(
        failures.is_empty(),
        "{cell}: {} of {} rows differ\n{}",
        failures.len(),
        rows.len(),
        failures.join("\n")
    );
}

/// `try { <stmts>; r = 'no throw' } catch (e) { r = <label> }`, then
/// `then` (an expression over `r`, the completion value).  `class` is the
/// expected error class: a match reads `TE` / `RE`, anything else
/// `other:` + the error; `with_message` appends `e.message` instead
/// (`TE:<message>`).
fn catch_as(class: &str, with_message: bool, stmts: &str, then: &str) -> String {
    let tag = &class[..1];
    let label = if with_message {
        format!("(e instanceof {class} ? '{tag}E:' : 'other:') + e.message")
    } else {
        format!("e instanceof {class} ? '{tag}E' : 'other:' + e")
    };
    format!("let r; try {{ {stmts}; r = 'no throw'; }} catch (e) {{ r = {label}; }} {then}")
}

/// The shared prelude of the super cells: `B extends A` with `ctor_body`
/// as its class body, its `[[Prototype]]` set to `x`.
fn setup(x: &str, ctor_body: &str) -> String {
    format!(
        "const X = {x}; class A {{}} class B extends A {{ {ctor_body} }} \
         Object.setPrototypeOf(B, X); "
    )
}

fn super_default(x: &str) -> String {
    setup(x, "")
}

fn super_written(x: &str) -> String {
    setup(x, "constructor(){ super(); }")
}

/// One super cell: `new B()` over every non-constructor kind must throw
/// a TypeError.
fn super_cell(cell: &str, prelude: fn(&str) -> String) {
    let rows: Vec<_> = NON_CONSTRUCTORS
        .iter()
        .map(|k| {
            row(
                k.name,
                catch_as(
                    "TypeError",
                    false,
                    &format!("{}new B()", prelude(k.src)),
                    "r",
                ),
                "TE",
            )
        })
        .collect();
    assert_rows(cell, &rows);
}

/// The T-POP body (§4.1), written once and shared with T-POP-CTL.
fn pop_body(x: &str) -> String {
    format!(
        "{}function inner() {{ let a = 10, b = 20; let r; \
           try {{ r = typeof new B(); }} catch (e) {{ r = e instanceof TypeError ? 'TE' : 'other'; }} \
           return [a, b, r].join(','); }} \
         function outer() {{ let x = 'x'; const s = inner(); return x + '|' + s + '|' + (1 + 1); }} \
         outer()",
        setup(x, "")
    )
}

fn row(label: impl Into<String>, src: String, expected: &str) -> (String, String, String) {
    (label.into(), src, expected.to_string())
}

/// T-NEW: `new X()` refuses every non-constructor kind with
/// `not a constructor`; the spread spelling reaches the same entry; a
/// `CallShape::CallableOnly` native is refused on the `new` path.
#[test]
fn t_new() {
    let mut rows: Vec<_> = NON_CONSTRUCTORS
        .iter()
        .map(|k| {
            row(
                k.name,
                catch_as(
                    "TypeError",
                    true,
                    &format!("const X = {}; new X()", k.src),
                    "r",
                ),
                "TE:not a constructor",
            )
        })
        .collect();
    rows.push(row(
        "spread, generator",
        catch_as(
            "TypeError",
            false,
            &format!("const X = {GENERATOR}; new X(...[1])"),
            "r",
        ),
        "TE",
    ));
    rows.push(row(
        "spread, function declaration (control)",
        format!("const F = {FUNCTION_DECLARATION}; typeof new F(...[1])"),
        "object",
    ));
    rows.push(row(
        "Math.max",
        catch_as("TypeError", true, "new Math.max()", "r"),
        "TE:max is not a constructor",
    ));
    assert_rows("T-NEW", &rows);
}

/// T-SUPER-DEF: the default derived constructor's `super(...args)`
/// (ClassDefinitionEvaluation step 14.a.iv.3) refuses every
/// non-constructor super constructor.
#[test]
fn t_super_default() {
    super_cell("T-SUPER-DEF", super_default);
}

/// T-SUPER-EXP: a written `super()` (SuperCall Evaluation step 5)
/// refuses every non-constructor super constructor.
#[test]
fn t_super_written() {
    super_cell("T-SUPER-EXP", super_written);
}

/// T-POP: a frameless super constructor is refused inside `inner`, and
/// the callers' frames are intact (release-observable by value: an
/// over-pop returns early from `outer` with the constructed object).
#[test]
fn t_pop() {
    let rows: Vec<_> = NON_CONSTRUCTORS
        .iter()
        .filter(|k| k.frameless)
        .map(|k| row(k.name, pop_body(k.src), "x|10,20,TE|2"))
        .collect();
    assert_rows("T-POP", &rows);
}

/// T-POP-CTL: the T-POP body with each constructor control.
#[test]
fn t_pop_control() {
    let rows: Vec<_> = CONTROLS
        .iter()
        .map(|k| row(k.name, pop_body(k.src), "x|10,20,object|2"))
        .collect();
    assert_rows("T-POP-CTL", &rows);
}

/// T-EXTENDS: ClassDefinitionEvaluation step 8.g refuses the method,
/// getter and setter kinds (the predicate's own classification).
#[test]
fn t_extends() {
    let rows: Vec<_> = NON_CONSTRUCTORS
        .iter()
        .filter(|k| k.method_like)
        .map(|k| {
            row(
                k.name,
                catch_as(
                    "TypeError",
                    false,
                    &format!("const X = {}; class B extends X {{}}", k.src),
                    "r",
                ),
                "TE",
            )
        })
        .collect();
    assert_rows("T-EXTENDS", &rows);
}

/// T-CTL: every control constructs through `new` and through both
/// super forms; `extends` accepts the non-bound controls.
#[test]
fn t_controls() {
    let mut rows = Vec::new();
    for Kind { name, src: x, .. } in CONTROLS {
        rows.push(row(
            format!("{name}, new"),
            format!("const X = {x}; typeof new X()"),
            "object",
        ));
        rows.push(row(
            format!("{name}, default super"),
            format!("{}typeof new B()", super_default(x)),
            "object",
        ));
        rows.push(row(
            format!("{name}, written super"),
            format!("{}typeof new B()", super_written(x)),
            "object",
        ));
    }
    for k in CONTROLS.iter().filter(|k| !k.bound) {
        rows.push(row(
            format!("{}, extends", k.name),
            catch_as(
                "TypeError",
                false,
                &format!("const X = {}; class B extends X {{}}", k.src),
                "r",
            ),
            "no throw",
        ));
    }
    assert_rows("T-CTL", &rows);
}

/// T-ORD: the refusal follows the argument evaluation and precedes any
/// callee effect.  An async callee, because its push runs its body
/// (a generator's push runs none and would not separate the orders).
/// Each half runs in its own fresh `Vm`, so each has its own `log`.
#[test]
fn t_order() {
    let log = "r + '|' + log.join()";
    let rows = vec![
        row(
            "new half",
            format!(
                "const log = []; async function X(){{ log.push(2) }} {}",
                catch_as("TypeError", false, "new X(log.push(1))", log)
            ),
            "TE|1",
        ),
        row(
            "super half",
            format!(
                "const log = []; async function X(){{ log.push(2) }} \
                 class A {{}} class B extends A {{ constructor(){{ super(log.push(1)); }} }} \
                 Object.setPrototypeOf(B, X); {}",
                catch_as("TypeError", false, "new B()", log)
            ),
            "TE|1",
        ),
        row(
            "null half",
            format!(
                "const log = []; \
                 class A {{}} class B extends A {{ constructor(){{ super(log.push(1)); }} }} \
                 Object.setPrototypeOf(B, null); {}",
                catch_as("TypeError", true, "new B()", log)
            ),
            "TE:Super constructor null is not a constructor|1",
        ),
    ];
    assert_rows("T-ORD", &rows);
}

/// T-MSG: one error mapping for both entries — the super path reports
/// a `CallShape::IllegalConstructor` native exactly as `new` does, and
/// a `CallShape::CallableOnly` one by name; `new null()` keeps the
/// `New` entry's non-Object message.
#[test]
fn t_messages() {
    let rows = vec![
        row(
            "super Crypto equals new Crypto",
            "class A {} class B extends A {} Object.setPrototypeOf(B, Crypto); \
             let m1, m2; \
             try { new B(); m1 = 'no throw'; } catch (e) { m1 = e.message; } \
             try { new Crypto(); m2 = 'no throw'; } catch (e) { m2 = e.message; } \
             (m1 === m2 ? 'eq:' : 'ne:' + m1 + ' vs ') + m2"
                .to_string(),
            "eq:Failed to construct 'Crypto': Illegal constructor",
        ),
        row(
            "super Math.max",
            catch_as(
                "TypeError",
                true,
                "class A {} class B extends A {} Object.setPrototypeOf(B, Math.max); new B()",
                "r",
            ),
            "TE:max is not a constructor",
        ),
        row(
            "new null()",
            catch_as("TypeError", true, "new null()", "r"),
            "TE:not a constructor",
        ),
    ];
    assert_rows("T-MSG", &rows);
}

/// T-DEPTH: a bound chain one wrapper past `MAX_BIND_CHAIN_DEPTH` is a
/// RangeError at both entries (the admission unwraps with
/// `unwrap_bound_function_chain`, it does not classify the wrapper).
#[test]
fn t_depth() {
    let deep = format!(
        "let deep = function(){{}}; \
         for (let i = 0; i < {}; i++) deep = deep.bind(null); ",
        crate::vm::MAX_BIND_CHAIN_DEPTH + 1
    );
    let catch_range = |stmts: &str| format!("{deep}{}", catch_as("RangeError", true, stmts, "r"));
    let rows = vec![
        row(
            "new deep()",
            catch_range("new deep()"),
            "RE:Maximum bind chain depth exceeded",
        ),
        row(
            "super over deep",
            catch_range("class A {} class B extends A {} Object.setPrototypeOf(B, deep); new B()"),
            "RE:Maximum bind chain depth exceeded",
        ),
    ];
    assert_rows("T-DEPTH", &rows);
}

/// T-CE: `customElements.define` refuses a method and a getter through
/// the predicate, with `define`'s own message (the brand check
/// `validate_html_element_constructor_chain` throws a different
/// TypeError, so the message is load-bearing), and defines nothing.
#[cfg(feature = "engine")]
#[test]
fn t_custom_elements_define() {
    use super::super::test_helpers::{eval_str, with_page};

    with_page(|vm| {
        let out = eval_str(
            vm,
            "const r = []; \
             for (const [n, f] of [['x-m', ({m(){}}).m], \
                                   ['x-g', Object.getOwnPropertyDescriptor({get g(){ return 1; }}, 'g').get]]) { \
               let m; \
               try { customElements.define(n, f); m = 'no throw'; } \
               catch (e) { m = (e instanceof TypeError ? 'TE:' : 'other:') + e.message; } \
               r.push(n + '=' + m + '|undefined after:' + (customElements.get(n) === undefined)); \
             } \
             customElements.define('x-c', class extends HTMLElement {}); \
             r.push('x-c defined:' + (customElements.get('x-c') !== undefined)); \
             r.join('\\n')",
        );
        let msg = "Failed to execute 'define' on 'CustomElementRegistry': parameter 2 is not a constructor.";
        assert_eq!(
            out,
            format!(
                "x-m=TE:{msg}|undefined after:true\n\
                 x-g=TE:{msg}|undefined after:true\n\
                 x-c defined:true"
            )
        );
    });
}
