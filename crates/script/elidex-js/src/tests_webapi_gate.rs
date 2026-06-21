//! A1 Web-API core/compat gate — VM-construction threading + general-predicate
//! seam behavior.
//!
//! Proves the gate mechanism A1 lands (option-A general gate):
//! - the embedder-supplied [`EngineMode`] is threaded into VM construction and
//!   the derived `SpecLevelPolicy` is stored where every install seam reads it
//!   (`Vm::inner.spec_level_policy`), via the **family-neutral** `installs(level)`
//!   / `installs_dom(level)` predicate (no storage-specific helper);
//! - `BrowserCompat` (the default) installs `Legacy`, `BrowserCore` / `App`
//!   exclude it — the one predicate every seam consults;
//! - **no behavior change**: A1 classifies every real API `Modern`/`Living` (no
//!   API moves), so the `StorageEvent` global, the rewired Window accessors
//!   (`localStorage` / `onstorage`), `document.cookie`, and the live-collection
//!   getters all install in *every* mode. A2/A3/B demote a family by flipping
//!   one level literal at its site — that exclusion is their test, not A1's.
//!
//! End-to-end exclusion of a `Legacy` API *at a VM seam* is proven concretely
//! two ways: (i) seam-4 in `elidex-dom-api::registry` (a mock `Legacy`
//! `DomApiHandler` withheld under `BrowserCore`), and (ii) a test-only
//! `Legacy`-classified **direct-global** probe routed through the same general
//! predicate (`legacy_probe_withheld_in_core_modes`) — closing F9's "direct
//! table/global installs" doubt that the storage-specific first A1 left open.

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
    // Default (BrowserCompat) installs Legacy — but only when the compat shims
    // are compiled in. Under the app profile (`engine` without `compat-webapi`)
    // the construction-time hard ceiling (`with_legacy_excluded`) excludes Legacy
    // regardless of the runtime mode, so `Vm::new()` (BrowserCompat) must report
    // Legacy excluded there. This assertion is therefore cfg-split so it holds on
    // BOTH supported build profiles.
    #[cfg(feature = "compat-webapi")]
    assert!(Vm::new()
        .inner
        .spec_level_policy
        .installs(WebApiSpecLevel::Legacy));
    #[cfg(not(feature = "compat-webapi"))]
    assert!(!Vm::new()
        .inner
        .spec_level_policy
        .installs(WebApiSpecLevel::Legacy));

    // BrowserCore / App exclude Legacy but keep Modern (holds in both profiles —
    // the ceiling only tightens, never loosens).
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

#[test]
fn rewired_window_seams_present_in_all_modes() {
    // The redesign re-expresses seam-1a (`localStorage` accessor) through the
    // general `installs(level)` predicate and routes seam-3 (`onstorage`) through
    // the per-attr handler-attr gate — both at `Modern`. A1 classifies nothing
    // `Legacy`, so both Window accessors must remain present in EVERY mode
    // (behavior-preserving rewiring; A2 demotes them later).
    for mode in [
        EngineMode::BrowserCompat,
        EngineMode::BrowserCore,
        EngineMode::App,
    ] {
        let (mut engine, mut session, mut dom, doc) = fresh(mode);
        let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
        let r = ScriptEngine::eval(
            &mut engine,
            "globalThis.ok = (('localStorage' in globalThis) && ('onstorage' in globalThis));",
            &mut ctx,
        );
        assert!(r.success, "{mode:?}: eval failed");
        assert!(
            global_true(&mut engine, "ok"),
            "{mode:?}: localStorage + onstorage accessors must stay present (Modern in A1)"
        );
    }
    // The document-side rewired seams (1b `document.cookie`, 1c live-collection
    // getters) are extracted into their own gated sub-tables at `Modern`/`Living`;
    // their presence + behavior is covered by the broader elidex-js DOM suite
    // (the 5.9k-test run would regress if the extraction dropped a property).
}

#[test]
fn legacy_probe_withheld_in_core_modes() {
    // F9 end-to-end at a VM **direct-global** install seam (not only the seam-4
    // registry): the test-only `Legacy` probe global (`register_globals`) routes
    // through the same general `installs(level)` predicate the real seams use.
    // BrowserCore / App withhold it in BOTH build profiles (the construction-time
    // ceiling can only tighten). BrowserCompat installs it only when the compat
    // shims are compiled in (`compat-webapi`).
    for mode in [EngineMode::BrowserCore, EngineMode::App] {
        let mut engine = ElidexJsEngine::new_with_mode(mode);
        assert!(
            engine.vm().get_global("__a1LegacyProbe").is_none(),
            "{mode:?}: a Legacy-classified VM global must be withheld"
        );
    }
    let mut compat = ElidexJsEngine::new();
    let probe = compat.vm().get_global("__a1LegacyProbe");
    #[cfg(feature = "compat-webapi")]
    assert!(
        matches!(probe, Some(JsValue::Boolean(true))),
        "BrowserCompat (+compat-webapi) must install the Legacy-classified probe \
         through the general direct-global seam"
    );
    #[cfg(not(feature = "compat-webapi"))]
    assert!(
        probe.is_none(),
        "compat-webapi-off ceiling must withhold Legacy even under BrowserCompat"
    );
}

#[test]
fn worker_realms_inherit_engine_mode() {
    // Codex R1 regression: the dedicated-worker and service-worker constructors
    // must honor the supplied engine mode, not reset it to `BrowserCompat` — a
    // `BrowserCore`/`App` document's worker realms install the same policy-gated
    // surface (the DOM-handler registry + the currently over-exposed storage
    // globals A2 demotes), so resetting would re-expose the compat surface in a
    // core/app worker. (Under `compat-webapi`-off the ceiling already excludes
    // Legacy for every mode, so this catches the reset on the `--all-features`
    // profile, where BrowserCompat would otherwise install Legacy.)
    let worker = Vm::new_worker(
        "w".to_string(),
        url::Url::parse("https://example.com/w.js").unwrap(),
        true,
        elidex_net::CredentialsMode::SameOrigin,
        EngineMode::BrowserCore,
    );
    assert!(
        !worker
            .inner
            .spec_level_policy
            .installs(WebApiSpecLevel::Legacy),
        "dedicated worker must inherit BrowserCore (Legacy excluded), not reset to BrowserCompat"
    );

    let sw = Vm::new_service_worker(
        url::Url::parse("https://example.com/").unwrap(),
        url::Url::parse("https://example.com/sw.js").unwrap(),
        true,
        elidex_net::CredentialsMode::SameOrigin,
        EngineMode::App,
    );
    assert!(
        !sw.inner.spec_level_policy.installs(WebApiSpecLevel::Legacy),
        "service worker must inherit App (Legacy excluded), not reset to BrowserCompat"
    );
}
