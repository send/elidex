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
//!   §3.7 interface-object step 2 + ECMA-262 §27.2.3.1 step 1); `new`
//!   returns a Promise instance.
//! - [`IllegalConstructor`] (Crypto, FileList, …): BOTH bare-call and
//!   `new` throw the canonical `"Failed to construct '{name}': Illegal
//!   constructor"` (WebIDL §3.7 interface-object step 1 — no ctor
//!   operation), gated at `do_new` (Construct) + `call_dispatch` (Call)
//!   with the shared `VmError::illegal_constructor` SoT.  Added by
//!   `#11-vm-native-illegal-constructor-shape`.
//!
//! Plan-memo `m4-12-pr-vm-native-constructor-only-flag-plan.md` §6 +
//! `m4-12-pr-vm-native-illegal-constructor-shape-plan.md` §3.2 / §4
//! Sanity tests (Stage 3c deliverable).

use super::super::value::JsValue;
use super::super::Vm;
use super::{assert_ctor_requires_new, assert_illegal_constructor, eval_bool};

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

// ---------------------------------------------------------------------------
// IllegalConstructor — both-mode throw across all 15 migrated sites
// (#11-vm-native-illegal-constructor-shape §3.2)
// ---------------------------------------------------------------------------

#[test]
fn illegal_constructor_both_modes_all_sites() {
    // Every WebIDL interface object that declares no constructor
    // operation throws `"Failed to construct '{name}': Illegal
    // constructor"` for BOTH `new X()` (gated at `do_new`) and bare
    // `X()` (gated at `call_dispatch`).  `assert_illegal_constructor`
    // exercises both modes per name, so this single test is 30 checks
    // (15 sites × 2 modes) against the shared
    // `VmError::illegal_constructor` SoT — the two-chokepoint sync guard.
    for interface in [
        "Crypto",
        "SubtleCrypto",
        "Storage",
        "Selection",
        "TreeWalker",
        "NodeIterator",
        "CustomElementRegistry",
        "ReadableStreamDefaultController",
        "BeforeUnloadEvent",
        "FileList",
        "TouchList",
        "DataTransferItem",
        "DataTransferItemList",
        "CanvasRenderingContext2D",
        "OffscreenCanvasRenderingContext2D",
    ] {
        assert_illegal_constructor(interface);
    }
}

#[test]
fn illegal_constructor_distinct_from_typedarray_carveout() {
    // %TypedArray% is the deliberate carve-out (ECMA-262 §23.2.1.1
    // abstract-class wording, NOT migrated to IllegalConstructor —
    // plan §3.3 DR-3).  It must still throw, but with its ECMA-flavored
    // message, NOT the WebIDL "Illegal constructor" form — proving the
    // carve-out held.  `%TypedArray%` is not a global binding, so it is
    // reached via `Object.getPrototypeOf(Uint8Array)` (the abstract
    // intrinsic is the [[Prototype]] of the concrete TA ctors).
    let mut vm = Vm::new();
    let msg = match vm
        .eval(
            "try { Object.getPrototypeOf(Uint8Array)(); 'no throw' } \
             catch (e) { e.message }",
        )
        .unwrap()
    {
        JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string error message, got {other:?}"),
    };
    assert!(
        msg.contains("not directly constructable"),
        "%TypedArray% carve-out should keep its ECMA abstract-class message, got: {msg}",
    );
    assert!(
        !msg.contains("Illegal constructor"),
        "%TypedArray% must NOT use the WebIDL IllegalConstructor message (carve-out), got: {msg}",
    );
}
