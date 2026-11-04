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

/// The 15 callee kinds that have no `[[Construct]]` (§4.1's table).
const NON_CONSTRUCTORS: &[(&str, &str)] = &[
    ("generator", "function*(){}"),
    ("async", "async function(){}"),
    ("async generator", "async function*(){}"),
    ("async arrow", "async () => {}"),
    ("arrow", "() => {}"),
    ("object-literal method", "({m(){}}).m"),
    ("generator method", "({*m(){}}).m"),
    ("async method", "({async m(){}}).m"),
    ("class method", "(class { m(){} }).prototype.m"),
    ("static method", "(class { static m(){} }).m"),
    (
        "object getter",
        "Object.getOwnPropertyDescriptor({get g(){ return 1; }}, 'g').get",
    ),
    (
        "object setter",
        "Object.getOwnPropertyDescriptor({set s(v){}}, 's').set",
    ),
    (
        "class getter",
        "Object.getOwnPropertyDescriptor((class { get g(){ return 1; } }).prototype, 'g').get",
    ),
    ("bound generator", "(function*(){}).bind(null)"),
    ("bound method", "({m(){}}).m.bind(null)"),
];

/// The kinds whose call returns without pushing a frame (§0.3 P-2):
/// the ones whose `super(...)` over-popped the caller's frame.
const FRAMELESS: &[&str] = &[
    "generator",
    "async",
    "async generator",
    "async arrow",
    "generator method",
    "async method",
    "bound generator",
];

/// The kinds that are methods, getters or setters: the predicate
/// itself was wrong for these (§1.2), so `extends` accepted them.
const METHOD_LIKE: &[&str] = &[
    "object-literal method",
    "class method",
    "static method",
    "object getter",
    "object setter",
    "class getter",
];

/// Constructors (§4.1's controls).
const CONTROLS: &[(&str, &str)] = &[
    (
        "function declaration",
        "(function(){ function F(){} return F; })()",
    ),
    ("function expression", "function(){}"),
    ("property-value function", "({ f: function(){} }).f"),
    ("class without constructor", "class {}"),
    (
        "class with written constructor",
        "class { constructor(){ this.k = 1; } }",
    ),
    ("bound function", "(function(){}).bind(null)"),
    ("bound class", "(class {}).bind(null)"),
    ("Object", "Object"),
    ("Error", "Error"),
];

/// Controls that `class B extends X {}` accepts: the bound ones are
/// refused by ClassDefinitionEvaluation step 8.h.ii (a bound function
/// has no `prototype`), which is spec-correct and not this cell's.
const EXTENDS_CONTROLS: &[&str] = &[
    "function declaration",
    "function expression",
    "property-value function",
    "class without constructor",
    "class with written constructor",
    "Object",
    "Error",
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

fn kind(name: &str) -> &'static str {
    NON_CONSTRUCTORS
        .iter()
        .chain(CONTROLS)
        .find(|(n, _)| *n == name)
        .map_or_else(|| panic!("unknown kind {name}"), |(_, src)| *src)
}

/// `try { <expr>; 'no throw' } catch (e) { 'TE:' + message | 'other:' … }`.
fn catch_message(stmts: &str) -> String {
    format!(
        "try {{ {stmts}; 'no throw' }} \
         catch (e) {{ (e instanceof TypeError ? 'TE:' : 'other:') + e.message }}"
    )
}

/// `try { <expr>; 'no throw' } catch (e) { 'TE' | 'other:' … }`.
fn catch_type_error(stmts: &str) -> String {
    format!(
        "try {{ {stmts}; 'no throw' }} \
         catch (e) {{ e instanceof TypeError ? 'TE' : 'other:' + e }}"
    )
}

fn super_default(x: &str) -> String {
    format!("const X = {x}; class A {{}} class B extends A {{}} Object.setPrototypeOf(B, X); ")
}

fn super_written(x: &str) -> String {
    format!(
        "const X = {x}; class A {{}} \
         class B extends A {{ constructor(){{ super(); }} }} \
         Object.setPrototypeOf(B, X); "
    )
}

/// The T-POP body (§4.1), written once and shared with T-POP-CTL.
fn pop_body(x: &str) -> String {
    format!(
        "const X = {x}; class A {{}} class B extends A {{}} Object.setPrototypeOf(B, X); \
         function inner() {{ let a = 10, b = 20; let r; \
           try {{ r = typeof new B(); }} catch (e) {{ r = e instanceof TypeError ? 'TE' : 'other'; }} \
           return [a, b, r].join(','); }} \
         function outer() {{ let x = 'x'; const s = inner(); return x + '|' + s + '|' + (1 + 1); }} \
         outer()"
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
        .map(|(name, x)| {
            row(
                *name,
                catch_message(&format!("const X = {x}; new X()")),
                "TE:not a constructor",
            )
        })
        .collect();
    rows.push(row(
        "spread, generator",
        catch_type_error(&format!("const X = {}; new X(...[1])", kind("generator"))),
        "TE",
    ));
    rows.push(row(
        "spread, function declaration (control)",
        format!(
            "const F = {}; typeof new F(...[1])",
            kind("function declaration")
        ),
        "object",
    ));
    rows.push(row(
        "Math.max",
        catch_message("new Math.max()"),
        "TE:max is not a constructor",
    ));
    assert_rows("T-NEW", &rows);
}

/// T-SUPER-DEF: the default derived constructor's `super(...args)`
/// (ClassDefinitionEvaluation step 14.a.iv.3) refuses every
/// non-constructor super constructor.
#[test]
fn t_super_default() {
    let rows: Vec<_> = NON_CONSTRUCTORS
        .iter()
        .map(|(name, x)| {
            row(
                *name,
                catch_type_error(&format!("{}new B()", super_default(x))),
                "TE",
            )
        })
        .collect();
    assert_rows("T-SUPER-DEF", &rows);
}

/// T-SUPER-EXP: a written `super()` (SuperCall Evaluation step 5)
/// refuses every non-constructor super constructor.
#[test]
fn t_super_written() {
    let rows: Vec<_> = NON_CONSTRUCTORS
        .iter()
        .map(|(name, x)| {
            row(
                *name,
                catch_type_error(&format!("{}new B()", super_written(x))),
                "TE",
            )
        })
        .collect();
    assert_rows("T-SUPER-EXP", &rows);
}

/// T-POP: a frameless super constructor is refused inside `inner`, and
/// the callers' frames are intact (release-observable by value: an
/// over-pop returns early from `outer` with the constructed object).
#[test]
fn t_pop() {
    let rows: Vec<_> = FRAMELESS
        .iter()
        .map(|name| row(*name, pop_body(kind(name)), "x|10,20,TE|2"))
        .collect();
    assert_rows("T-POP", &rows);
}

/// T-POP-CTL: the T-POP body with each constructor control.
#[test]
fn t_pop_control() {
    let rows: Vec<_> = CONTROLS
        .iter()
        .map(|(name, x)| row(*name, pop_body(x), "x|10,20,object|2"))
        .collect();
    assert_rows("T-POP-CTL", &rows);
}

/// T-EXTENDS: ClassDefinitionEvaluation step 8.g refuses the method,
/// getter and setter kinds (the predicate's own classification).
#[test]
fn t_extends() {
    let rows: Vec<_> = METHOD_LIKE
        .iter()
        .map(|name| {
            row(
                *name,
                catch_type_error(&format!("const X = {}; class B extends X {{}}", kind(name))),
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
    for (name, x) in CONTROLS {
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
    for name in EXTENDS_CONTROLS {
        rows.push(row(
            format!("{name}, extends"),
            catch_type_error(&format!("const X = {}; class B extends X {{}}", kind(name))),
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
    let rows = vec![
        row(
            "new half",
            "const log = []; async function X(){ log.push(2) } \
             let r; try { r = typeof new X(log.push(1)); } \
             catch (e) { r = e instanceof TypeError ? 'TE' : 'other'; } \
             r + '|' + log.join()"
                .to_string(),
            "TE|1",
        ),
        row(
            "super half",
            "const log = []; async function X(){ log.push(2) } \
             class A {} class B extends A { constructor(){ super(log.push(1)); } } \
             Object.setPrototypeOf(B, X); \
             let r; try { r = typeof new B(); } \
             catch (e) { r = e instanceof TypeError ? 'TE' : 'other'; } \
             r + '|' + log.join()"
                .to_string(),
            "TE|1",
        ),
        row(
            "null half",
            "const log = []; \
             class A {} class B extends A { constructor(){ super(log.push(1)); } } \
             Object.setPrototypeOf(B, null); \
             let r; try { r = typeof new B(); } \
             catch (e) { r = e instanceof TypeError ? 'TE:' + e.message : 'other'; } \
             r + '|' + log.join()"
                .to_string(),
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
            catch_message(
                "class A {} class B extends A {} Object.setPrototypeOf(B, Math.max); new B()",
            ),
            "TE:max is not a constructor",
        ),
        row(
            "new null()",
            catch_message("new null()"),
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
    let catch_range = |stmts: &str| {
        format!(
            "{deep}try {{ {stmts}; 'no throw' }} \
             catch (e) {{ (e instanceof RangeError ? 'RE:' : 'other:') + e.message }}"
        )
    };
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
    use elidex_ecs::{Attributes, EcsDom};
    use elidex_script_session::SessionCore;

    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    let body = dom.create_element("body", Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, body));
    #[allow(unsafe_code)]
    // SAFETY: `session` / `dom` outlive the binding; `vm.unbind()` runs
    // before either is dropped.
    unsafe {
        super::super::test_helpers::bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let out = vm.eval(
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
    let out = match out {
        Ok(JsValue::String(id)) => vm.get_string(id),
        other => format!("{other:?}"),
    };
    vm.unbind();
    let msg =
        "Failed to execute 'define' on 'CustomElementRegistry': parameter 2 is not a constructor.";
    assert_eq!(
        out,
        format!(
            "x-m=TE:{msg}|undefined after:true\n\
             x-g=TE:{msg}|undefined after:true\n\
             x-c defined:true"
        )
    );
}
