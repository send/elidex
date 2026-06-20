//! A1 Web-API core/compat gate — VM-construction threading + seam behavior.
//!
//! Proves the gate mechanism A1 lands:
//! - the embedder-supplied [`EngineMode`] is threaded into VM construction and
//!   the derived [`SpecLevelPolicy`] is stored where every install seam reads it
//!   (`Vm::inner.spec_level_policy`);
//! - `BrowserCompat` (the default) installs `Legacy`, `BrowserCore` / `App`
//!   exclude it — the predicate the seams consult;
//! - **no behavior change**: A1 marks the Web Storage family `Modern` (no API
//!   moves), so the `StorageEvent` global installs in *every* mode, including
//!   `BrowserCore`. A2 demotes the family to `Legacy`, after which `BrowserCore`
//!   omits it — that exclusion is A2's test, not A1's (A1 has nothing `Legacy`).
//!
//! End-to-end exclusion of a `Legacy` API *at a seam* is proven concretely for
//! seam-4 in `elidex-dom-api::registry` (a mock `Legacy` handler is withheld
//! under `BrowserCore`); for the VM install seams it is latent until A2 marks a
//! real API `Legacy`, and is covered here by the policy-threading assertions.

#![cfg(feature = "engine")]

use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::{EngineMode, WebApiSpecLevel};
use elidex_script_session::{ScriptContext, ScriptEngine, SessionCore};

use crate::engine::ElidexJsEngine;
use crate::vm::host_data::HostData;
use crate::vm::value::JsValue;
use crate::vm::Vm;

#[test]
fn vm_stores_mode_derived_policy() {
    // Default (BrowserCompat) installs Legacy.
    assert!(Vm::new()
        .inner
        .spec_level_policy
        .installs(WebApiSpecLevel::Legacy));

    // BrowserCore / App exclude Legacy but keep Modern.
    for mode in [EngineMode::BrowserCore, EngineMode::App] {
        let vm = Vm::new_with_mode(mode);
        assert!(
            !vm.inner.spec_level_policy.installs(WebApiSpecLevel::Legacy),
            "{mode:?}: VM policy must exclude Legacy"
        );
        assert!(
            vm.inner.spec_level_policy.installs(WebApiSpecLevel::Modern),
            "{mode:?}: VM policy must keep Modern"
        );
    }
}

fn fresh(engine_mode: EngineMode) -> (ElidexJsEngine, SessionCore, EcsDom, Entity) {
    let mut engine = ElidexJsEngine::new_with_mode(engine_mode);
    engine.vm().install_host_data(HostData::new());
    let session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    (engine, session, dom, doc)
}

fn global_true(engine: &mut ElidexJsEngine, name: &str) -> bool {
    matches!(engine.vm().get_global(name), Some(JsValue::Boolean(true)))
}

#[test]
fn storage_event_global_present_in_all_modes() {
    // A1 keeps the Web Storage family `Modern` (no API moves), so the
    // `StorageEvent` constructor installs in every mode — the no-behavior-change
    // guarantee. (After A2 demotes it to `Legacy`, BrowserCore/App will omit it.)
    for mode in [
        EngineMode::BrowserCompat,
        EngineMode::BrowserCore,
        EngineMode::App,
    ] {
        let (mut engine, mut session, mut dom, doc) = fresh(mode);
        let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
        let r = ScriptEngine::eval(
            &mut engine,
            "globalThis.ok = (typeof StorageEvent === 'function');",
            &mut ctx,
        );
        assert!(r.success, "{mode:?}: eval failed");
        assert!(
            global_true(&mut engine, "ok"),
            "{mode:?}: StorageEvent global must be present (Modern in A1)"
        );
    }
}

#[test]
fn new_with_mode_constructs_every_mode() {
    // Smoke: construction + global registration succeed for all three modes
    // (the install seams run under each derived policy without panicking).
    for mode in [
        EngineMode::BrowserCompat,
        EngineMode::BrowserCore,
        EngineMode::App,
    ] {
        let (mut engine, mut session, mut dom, doc) = fresh(mode);
        let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
        let r = ScriptEngine::eval(&mut engine, "globalThis.ran = true;", &mut ctx);
        assert!(r.success, "{mode:?}: basic eval failed");
        assert!(
            global_true(&mut engine, "ran"),
            "{mode:?}: eval side effect lost"
        );
    }
}
