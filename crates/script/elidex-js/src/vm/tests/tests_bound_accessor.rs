//! Bound-key native accessor mechanism (`#11-bound-key-native-accessor`).
//!
//! Verifies that a *single* backend getter fn, installed twice via
//! [`super::super::VmInner::install_bound_accessor_pair`] with distinct bound
//! keys, resolves a distinct value per accessor — i.e. `NativeContext::bound_key`
//! recovers which property the shared fn serves. WebIDL §3.7.6.

#![cfg(feature = "engine")]

use super::super::shape::PropertyAttrs;
use super::super::value::{JsValue, NativeContext, PropertyKey, PropertyValue, VmError};
use super::super::Vm;

/// Backend getter shared by both accessors: returns its bound key as a string.
fn echo_bound_key(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let key = ctx.bound_key().expect("accessor missing bound_key");
    Ok(JsValue::String(key))
}

#[test]
fn distinct_bound_keys_yield_distinct_values() {
    let mut vm = Vm::new();
    let global = vm.inner.global_object;

    let alpha = vm.inner.strings.intern("alpha");
    let beta = vm.inner.strings.intern("beta");

    // Two read-only accessors over ONE backend fn, distinguished only by bound key.
    vm.inner.install_bound_accessor_pair(
        global,
        alpha,
        echo_bound_key,
        None,
        alpha,
        PropertyAttrs::WEBIDL_RO_ACCESSOR,
    );
    vm.inner.install_bound_accessor_pair(
        global,
        beta,
        echo_bound_key,
        None,
        beta,
        PropertyAttrs::WEBIDL_RO_ACCESSOR,
    );

    let got_alpha = match vm.eval("globalThis.alpha;").unwrap() {
        JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string, got {other:?}"),
    };
    let got_beta = match vm.eval("globalThis.beta;").unwrap() {
        JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string, got {other:?}"),
    };

    assert_eq!(got_alpha, "alpha");
    assert_eq!(got_beta, "beta");
    assert_ne!(got_alpha, got_beta);
}

/// A `None` setter installs a genuinely read-only accessor — no no-op setter
/// that would suppress strict-mode assignment semantics (the `Option<setter>`
/// fix vs. the original mandatory-setter API).
#[test]
fn none_setter_installs_readonly_accessor() {
    let mut vm = Vm::new();
    let global = vm.inner.global_object;
    let alpha = vm.inner.strings.intern("alpha");
    vm.inner.install_bound_accessor_pair(
        global,
        alpha,
        echo_bound_key,
        None,
        alpha,
        PropertyAttrs::WEBIDL_RO_ACCESSOR,
    );

    let no_setter = match vm
        .eval("Object.getOwnPropertyDescriptor(globalThis, 'alpha').set === undefined;")
        .unwrap()
    {
        JsValue::Boolean(b) => b,
        other => panic!("expected bool, got {other:?}"),
    };
    assert!(no_setter, "None setter must not install a setter fn");
}

/// The bound key is call-scoped: after a bound-accessor call returns, the
/// staged key is restored to `None` (no leak into subsequent ordinary natives).
#[test]
fn bound_key_does_not_leak_after_call() {
    let mut vm = Vm::new();
    let global = vm.inner.global_object;
    let alpha = vm.inner.strings.intern("alpha");
    vm.inner.install_bound_accessor_pair(
        global,
        alpha,
        echo_bound_key,
        None,
        alpha,
        PropertyAttrs::WEBIDL_RO_ACCESSOR,
    );

    // Read the bound accessor, then confirm the VM's active key is cleared.
    let _ = vm.eval("globalThis.alpha;").unwrap();
    assert_eq!(vm.inner.active_bound_key, None);

    // A plain data property installed afterwards sees no stale key.
    let plain = vm.inner.strings.intern("plain");
    vm.inner.define_shaped_property(
        global,
        PropertyKey::String(plain),
        PropertyValue::Data(JsValue::Number(7.0)),
        PropertyAttrs::BUILTIN,
    );
    assert!(matches!(
        vm.eval("globalThis.plain;").unwrap(),
        JsValue::Number(n) if (n - 7.0).abs() < f64::EPSILON
    ));
}
