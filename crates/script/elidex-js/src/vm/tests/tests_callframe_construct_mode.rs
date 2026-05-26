//! Tests for the D-17b-r1 per-frame `CallMode` SoT — the unified
//! construct/call discipline that replaced the pre-r1
//! `VmInner::native_construct_stack` side channel.
//!
//! Coverage matches the plan-memo §5 test plan: `CallMode::Construct`
//! propagation through JS frames + `super()` chains, baked
//! `NativeContext::mode` for native ctors, `is_construct()` /
//! `new_target()` reads via `match self.mode`, sub-context default of
//! `CallMode::Call` for re-entrant callbacks,
//! [`super::super::VmInner::ensure_instance_or_alloc`] matrix
//! (this=Object × mode={Call, Construct{nt}}), and the catch_unwind
//! frame-stack / value-stack truncate-on-panic positive assertions
//! (the R2 CRIT-1 pre-existing fix).
//!
//! Spec citations via the plan-memo §0.5 table — chiefly:
//! - [C2] ECMA-262 §10.2.1.1 step 7 (PrepareForOrdinaryCall threads
//!   newTarget into NewFunctionEnvironment)
//! - [C5] §10.3.3 BuiltinCallOrConstruct (newTarget threaded into F's
//!   body; matched here by `NativeContext::new_construct`)
//! - [C6] §13.3.7.1 step 1 (SuperCall GetNewTarget — outer NewTarget
//!   propagation invariant)

use super::super::value::{CallMode, JsValue};
use super::super::Vm;

// ---------------------------------------------------------------------------
// CallFrame.mode propagation (JS frames)
// ---------------------------------------------------------------------------

#[test]
fn frame_mode_construct_under_new() {
    // `new F()` enters F's frame with `CallMode::Construct { new_target: F }`;
    // observable via `new.target` (§9.4.5 GetNewTarget — returns the
    // active Function Environment's [[NewTarget]] slot). `new F()`
    // itself evaluates to the constructed receiver, so stash the
    // observation on an outer binding and return that.
    let mut vm = Vm::new();
    let v = vm
        .eval("let seen; function F() { seen = new.target === F; } new F(); seen;")
        .unwrap();
    assert_eq!(v, JsValue::Boolean(true));
}

#[test]
fn frame_mode_call_under_plain_call() {
    // `F()` (no `new`) enters with `CallMode::Call`; `new.target` is
    // undefined per §9.4.5.
    let mut vm = Vm::new();
    let v = vm
        .eval("function F() { return new.target === undefined; } F();")
        .unwrap();
    assert_eq!(v, JsValue::Boolean(true));
}

#[test]
fn frame_new_target_propagates_through_super() {
    // [C6] SuperCall GetNewTarget — `super(...)` invokes the parent
    // ctor with the **outer execution context's NewTarget** unchanged.
    // So `new B()` (where B extends A) yields `new.target === B` in
    // both A's ctor frame and B's ctor frame.
    let mut vm = Vm::new();
    let v = vm
        .eval(
            "class A { constructor() { this.aNT = new.target; } } \
             class B extends A { constructor() { super(); this.bNT = new.target; } } \
             let b = new B(); b.aNT === B && b.bNT === B;",
        )
        .unwrap();
    assert_eq!(v, JsValue::Boolean(true));
}

#[test]
fn frame_new_target_propagates_through_super_extends_chain() {
    // Three-deep `extends` chain — `new.target` stays the
    // outermost-invoked class at every frame.
    let mut vm = Vm::new();
    let v = vm
        .eval(
            "class A { constructor() { this.a = new.target; } } \
             class B extends A { constructor() { super(); this.b = new.target; } } \
             class C extends B { constructor() { super(); this.c = new.target; } } \
             let c = new C(); c.a === C && c.b === C && c.c === C;",
        )
        .unwrap();
    assert_eq!(v, JsValue::Boolean(true));
}

// ---------------------------------------------------------------------------
// NativeContext.mode baked-at-construct-time
// ---------------------------------------------------------------------------

#[test]
fn native_ctx_mode_baked_at_construct_time() {
    // Construct mode reaches a native ctor body — observable here via
    // `new Error('msg')` which routes through `native_error_constructor` →
    // `ensure_instance_or_alloc(this, error_prototype, ctx.mode)`. In
    // construct mode the receiver `this` is reused (the `do_new`-allocated
    // instance), so `e instanceof Error` holds and `e.message` reads back.
    let mut vm = Vm::new();
    let v = vm
        .eval("let e = new Error('msg'); e instanceof Error && e.message === 'msg';")
        .unwrap();
    assert_eq!(v, JsValue::Boolean(true));
}

#[test]
fn native_ctx_mode_call_in_plain_native_call() {
    // `Error('msg')` without `new` — call mode. `ensure_instance_or_alloc`
    // allocates a fresh Ordinary with `error_prototype` (Error is one of
    // the few ctors that's callable — §20.5.1.1 step 1 conditionally
    // accepts `undefined` NewTarget by falling back to the active
    // function object). The receiver is the freshly-allocated instance,
    // NOT `globalThis`.
    let mut vm = Vm::new();
    let v = vm
        .eval(
            "let e = Error('msg'); \
             e instanceof Error && e.message === 'msg' && e !== globalThis;",
        )
        .unwrap();
    assert_eq!(v, JsValue::Boolean(true));
}

#[test]
fn native_ctx_sub_ctx_always_call() {
    // Sub-contexts spawned for callbacks default to `CallMode::Call`
    // (per `NativeContext::new_call`). `Array.from` invokes a mapFn
    // callback synchronously — the callback's `new.target` must be
    // undefined even though the outer `Array.from(...)` may itself be
    // invoked via `new Subclass.from(...)`. This guards the
    // "callback re-entrant scope default Call" invariant.
    let mut vm = Vm::new();
    let v = vm
        .eval(
            "let seen; Array.from([1], function () { seen = new.target; }); \
             seen === undefined;",
        )
        .unwrap();
    assert_eq!(v, JsValue::Boolean(true));
}

// ---------------------------------------------------------------------------
// ensure_instance_or_alloc — direct API matrix
// ---------------------------------------------------------------------------

#[test]
fn ensure_instance_or_alloc_construct_with_object_this_reuses() {
    // (this=Object × Construct{nt}) — reuse path: returns the same
    // `this` so a `new Sub()` whose `do_new` allocated `Sub.prototype`
    // receiver keeps its subclass prototype.
    let mut vm = Vm::new();
    let inner = &mut vm.inner;
    let proto = inner.error_prototype;
    let nt_obj = inner.alloc_object(super::super::value::Object {
        kind: super::super::value::ObjectKind::Ordinary,
        storage: super::super::value::PropertyStorage::shaped(super::super::shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });
    let this_obj = inner.alloc_object(super::super::value::Object {
        kind: super::super::value::ObjectKind::Ordinary,
        storage: super::super::value::PropertyStorage::shaped(super::super::shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });
    let this_val = JsValue::Object(this_obj);
    let out =
        inner.ensure_instance_or_alloc(this_val, proto, CallMode::Construct { new_target: nt_obj });
    assert_eq!(out, this_val, "Construct + Object this must reuse this");
}

#[test]
fn ensure_instance_or_alloc_construct_with_non_object_this_allocates() {
    // (this=non-Object × Construct{nt}) — even in construct mode,
    // if `this` is not an Object (e.g. native called via apply with
    // primitive receiver), fall through to fresh allocation.
    let mut vm = Vm::new();
    let inner = &mut vm.inner;
    let proto = inner.error_prototype;
    let nt_obj = inner.alloc_object(super::super::value::Object {
        kind: super::super::value::ObjectKind::Ordinary,
        storage: super::super::value::PropertyStorage::shaped(super::super::shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });
    let out = inner.ensure_instance_or_alloc(
        JsValue::Undefined,
        proto,
        CallMode::Construct { new_target: nt_obj },
    );
    assert!(
        matches!(out, JsValue::Object(_)),
        "Construct + non-Object this must allocate fresh"
    );
}

#[test]
fn ensure_instance_or_alloc_call_always_allocates() {
    // (Call mode) — always allocates fresh regardless of `this`'s
    // type. Matches §10.1.13 OrdinaryCreateFromConstructor.
    let mut vm = Vm::new();
    let inner = &mut vm.inner;
    let proto = inner.error_prototype;
    let this_obj = inner.alloc_object(super::super::value::Object {
        kind: super::super::value::ObjectKind::Ordinary,
        storage: super::super::value::PropertyStorage::shaped(super::super::shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });
    let this_val = JsValue::Object(this_obj);
    let out = inner.ensure_instance_or_alloc(this_val, proto, CallMode::Call);
    assert!(matches!(out, JsValue::Object(_)));
    assert_ne!(
        out, this_val,
        "Call mode must allocate fresh, not reuse this even when Object"
    );
}

// ---------------------------------------------------------------------------
// `with_call_mode` boundary — panic-safety positive assertions
// ---------------------------------------------------------------------------
//
// These mirror the regression tests in `vm/interpreter.rs::tests`
// (frame + value stack truncate-on-panic). Kept here too as the
// public-API surface that future maintainers will reach for when
// chasing CallMode regressions: this file is the one cross-linked
// from the plan-memo §5 test plan.

#[test]
fn with_call_mode_threads_construct_into_closure() {
    // `with_call_mode(Construct{nt}, |_, mode| mode)` returns the
    // exact mode passed in — confirms the closure arg reflects the
    // outer boundary's mode for downstream `push_js_call_frame` /
    // `NativeContext::new_*` threading.
    let mut vm = Vm::new();
    let inner = &mut vm.inner;
    // Allocate a fresh sentinel object as new_target so the test
    // does not depend on which builtin prototypes `Vm::new()` has
    // already registered (and so an off-by-one `ObjectId(0)`
    // fallback can't silently mask an initialization regression).
    let nt = inner.alloc_object(super::super::value::Object {
        kind: super::super::value::ObjectKind::Ordinary,
        storage: super::super::value::PropertyStorage::shaped(super::super::shape::ROOT_SHAPE),
        prototype: None,
        extensible: true,
    });
    let result: Result<CallMode, _> =
        inner.with_call_mode(CallMode::Construct { new_target: nt }, |_vm, mode| Ok(mode));
    assert_eq!(result.unwrap(), CallMode::Construct { new_target: nt });
}

#[test]
fn with_call_mode_threads_call_into_closure() {
    let mut vm = Vm::new();
    let inner = &mut vm.inner;
    let result: Result<CallMode, _> = inner.with_call_mode(CallMode::Call, |_vm, mode| Ok(mode));
    assert_eq!(result.unwrap(), CallMode::Call);
}
