//! `CallShape` enum end-to-end sanity tests for the dispatch-side gate
//! introduced by `#11-vm-native-constructor-only-flag` (Stage 2).
//!
//! Three shape variants × two call modes = 6 sanity points, plus the
//! Promise canonical-message regression and the Promise `new` happy-path
//! continuity check:
//!
//! - [`CallableOnly`] (Symbol): bare-call returns a Symbol; `new` throws
//!   the existing "not a constructor" path (ECMA-262 §10.3 + §7.2.4
//!   IsConstructor precondition).
//! - [`Ordinary`] (Object): both bare-call and `new` succeed.
//! - [`ConstructorOnly`] (Promise): bare-call throws the canonical
//!   TypeError emitted at `vm/interpreter.rs::call_dispatch` (WebIDL
//!   §3.7.1 step 1.2 + ECMA-262 §27.2.3.1 step 1); `new` returns a
//!   Promise instance.
//!
//! Plan-memo `m4-12-pr-vm-native-constructor-only-flag-plan.md` §6
//! Sanity tests bullet list (Stage 3c deliverable).

use super::super::value::JsValue;
use super::super::Vm;
use super::{assert_ctor_requires_new, eval_bool};

// ---------------------------------------------------------------------------
// CallableOnly — Symbol (ECMA-262 §10.3 + §20.4.1.1)
// ---------------------------------------------------------------------------

#[test]
fn symbol_call_returns_symbol() {
    // Bare-call Symbol is the public construction path (§20.4.1.1).
    // Confirms Stage 1 + Stage 2 didn't promote Symbol's call-mode to
    // throw — only `new` should throw.
    assert!(eval_bool("typeof Symbol('s') === 'symbol'"));
}

#[test]
fn symbol_construct_throws_not_a_constructor() {
    // CallableOnly disallows `new` via the existing "not a constructor"
    // path (ECMA-262 §10.3 absence of [[Construct]] + §7.2.4
    // IsConstructor precondition).  NOT the canonical
    // `ConstructorOnly` message — the two error families are distinct.
    assert!(eval_bool(
        "var threw = false; \
         try { new Symbol('s'); } \
         catch (e) { threw = e instanceof TypeError; } threw;",
    ));
}

// ---------------------------------------------------------------------------
// Ordinary — Array (ECMA-262 §23.1)
// ---------------------------------------------------------------------------

#[test]
fn array_both_modes_work() {
    // Ordinary accepts both [[Call]] and [[Construct]].  Confirms the
    // default `CallShape::Ordinary` is unchanged by Stage 2.  (Elidex
    // does not surface `Object` as a callable constructor — see
    // `vm/globals.rs::register_object_global` — so `Array` stands as
    // the Ordinary witness.)
    assert!(eval_bool(
        "Array().length === 0 && new Array().length === 0",
    ));
}

// ---------------------------------------------------------------------------
// ConstructorOnly — Promise canonical-message + happy-path continuity
// (F23 IMP 2026-05-30 — plan-memo §0.5 + §3 + §6)
// ---------------------------------------------------------------------------

#[test]
fn promise_call_throws_canonical() {
    // ECMA-262 §27.2.3.1 step 1 — Promise ctor mandate routed through
    // the single dispatch-side gate.  Pre-Stage 3 this threw
    // `"Promise constructor cannot be invoked without 'new'"`; post-
    // Stage 3 it canonicalises to the WebIDL §3.7.1-shape form for
    // all 66 ctor sites uniformly.
    assert_ctor_requires_new("Promise(function(){})", "Promise");
}

#[test]
fn promise_construct_resolves() {
    // Regression — `ConstructorOnly` does NOT block `new`.  The
    // happy-path Promise ctor still returns a Promise instance whose
    // `instanceof Promise` brand check holds (Stage 1 + Stage 2
    // continuity guard).
    let mut vm = Vm::new();
    let result = vm
        .eval("new Promise(function(res, rej){res(42)}) instanceof Promise")
        .unwrap();
    assert_eq!(result, JsValue::Boolean(true));
}
