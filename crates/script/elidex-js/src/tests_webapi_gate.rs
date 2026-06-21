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
//! - A1 classified every real API `Modern`/`Living` (no API moves). **A2 has
//!   since demoted the Web Storage family to `Legacy`** (HTML §12.2), so the
//!   `Storage`/`StorageEvent` globals + the `localStorage`/`sessionStorage`/
//!   `onstorage` Window seams now install only under `BrowserCompat` + the
//!   `compat-webapi` feature, and are `[Exposed=Window]` (absent in worker realms)
//!   — see `storage_*_legacy_gated` / `storage_globals_absent_in_worker_realm_*`.
//!   **A3 has since demoted `document.cookie` to `Legacy`** (HTML §3.1.4) — see
//!   `document_cookie_legacy_gated`. The live-collection getters (B) remain
//!   `Living` until their PR flips that source.
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
use crate::vm::test_helpers::bind_vm;
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

fn storage_event_is_function(
    engine: &mut ElidexJsEngine,
    session: &mut SessionCore,
    dom: &mut EcsDom,
    doc: Entity,
) -> bool {
    let mut ctx = ScriptContext::new(session, dom, doc);
    let r = ScriptEngine::eval(
        engine,
        "globalThis.ok = (typeof StorageEvent === 'function');",
        &mut ctx,
    );
    assert!(r.success, "eval failed");
    global_true(engine, "ok")
}

#[test]
fn storage_event_global_legacy_gated() {
    // A2 demoted the Web Storage family to `Legacy` (HTML §12.2.4). The
    // `StorageEvent` constructor therefore installs ONLY under `BrowserCompat`
    // with the compat shims compiled in; `BrowserCore` / `App` (and any
    // `compat-webapi`-off build, via the construction-time hard ceiling) omit it —
    // gated through the same family source as the `Storage` global + the accessors.
    let (mut engine, mut session, mut dom, doc) = fresh(EngineMode::BrowserCompat);
    let present = storage_event_is_function(&mut engine, &mut session, &mut dom, doc);
    #[cfg(feature = "compat-webapi")]
    assert!(
        present,
        "BrowserCompat (compat-webapi on) must expose StorageEvent"
    );
    #[cfg(not(feature = "compat-webapi"))]
    assert!(
        !present,
        "compat-webapi off: hard ceiling must hide StorageEvent"
    );

    // Core modes never expose it, on either profile.
    for mode in [EngineMode::BrowserCore, EngineMode::App] {
        let (mut engine, mut session, mut dom, doc) = fresh(mode);
        assert!(
            !storage_event_is_function(&mut engine, &mut session, &mut dom, doc),
            "{mode:?}: StorageEvent must be absent (Legacy, demoted in A2)"
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

fn storage_window_seams_present(
    engine: &mut ElidexJsEngine,
    session: &mut SessionCore,
    dom: &mut EcsDom,
    doc: Entity,
) -> bool {
    let mut ctx = ScriptContext::new(session, dom, doc);
    let r = ScriptEngine::eval(
        engine,
        "globalThis.ok = (('localStorage' in globalThis) && ('sessionStorage' in globalThis) \
            && ('onstorage' in globalThis));",
        &mut ctx,
    );
    assert!(r.success, "eval failed");
    global_true(engine, "ok")
}

#[test]
fn storage_window_seams_legacy_gated() {
    // The Web Storage Window seams — seam-1a (`localStorage`/`sessionStorage`
    // accessors) and seam-3 (`onstorage` handler attr) — read the same family
    // source A2 demoted to `Legacy`. So all three are present together ONLY under
    // `BrowserCompat` + compat-webapi, and absent together otherwise (no split
    // surface: accessors-without-onstorage etc.).
    let (mut engine, mut session, mut dom, doc) = fresh(EngineMode::BrowserCompat);
    let present = storage_window_seams_present(&mut engine, &mut session, &mut dom, doc);
    #[cfg(feature = "compat-webapi")]
    assert!(
        present,
        "BrowserCompat (compat-webapi on) must expose the storage Window seams"
    );
    #[cfg(not(feature = "compat-webapi"))]
    assert!(
        !present,
        "compat-webapi off: hard ceiling must hide the storage Window seams"
    );

    for mode in [EngineMode::BrowserCore, EngineMode::App] {
        let (mut engine, mut session, mut dom, doc) = fresh(mode);
        assert!(
            !storage_window_seams_present(&mut engine, &mut session, &mut dom, doc),
            "{mode:?}: localStorage/sessionStorage/onstorage must be absent (Legacy, A2)"
        );
    }
    // The `document.cookie` seam (1b) is demoted to `Legacy` by A3 — its gating is
    // covered by `document_cookie_legacy_gated` below. The live-collection getters
    // (1c) are still `Living` (B0/B1 demote them); their enumeration order is pinned
    // by `live_collection_methods_keep_original_property_order`.
}

fn document_cookie_present(mode: EngineMode) -> bool {
    // `document.cookie` installs on the Document *wrapper*, so the document must be
    // bound (the proven path: `bind_vm` + `vm.eval`, as in
    // `live_collection_methods_keep_original_property_order`) — the engine/
    // ScriptContext path leaves `document` unbound.
    let mut vm = Vm::new_with_mode(mode);
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let present = matches!(
        vm.eval("'cookie' in document;").unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
    present
}

#[test]
fn document_cookie_legacy_gated() {
    // A3 demoted `document.cookie` to `Legacy` (HTML §3.1.4) via its single source
    // `document_cookie_spec_level()` (seam-1b). So the accessor is present ONLY
    // under `BrowserCompat` + compat-webapi, and absent otherwise — the `CookieJar`
    // (HTTP cookies + `navigator.cookieEnabled`) stays in every mode regardless.
    let present = document_cookie_present(EngineMode::BrowserCompat);
    #[cfg(feature = "compat-webapi")]
    assert!(
        present,
        "BrowserCompat (compat-webapi on) must expose document.cookie"
    );
    #[cfg(not(feature = "compat-webapi"))]
    assert!(
        !present,
        "compat-webapi off: hard ceiling must hide document.cookie"
    );

    for mode in [EngineMode::BrowserCore, EngineMode::App] {
        assert!(
            !document_cookie_present(mode),
            "{mode:?}: document.cookie must be absent (Legacy, demoted in A3)"
        );
    }
}

#[test]
fn live_collection_methods_keep_original_property_order() {
    // Codex R9 regression: A1 extracts the `Document` live-collection getters into
    // a gated sub-table (seam-1c), but elidex installs document methods as the
    // wrapper's OWN properties (no shared `Document.prototype`), so install order
    // IS `Object.getOwnPropertyNames(document)` order. The gated install must land
    // at its ORIGINAL ordinal position (between `querySelectorAll` and
    // `createElement`) — a trailing install would reorder the names after
    // `getSelection`, an observable change A1's no-behavior-change contract forbids.
    let mut vm = Vm::new(); // BrowserCompat (production default)
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let order_ok = vm
        .eval(
            "const n = Object.getOwnPropertyNames(document); \
             n.indexOf('querySelectorAll') >= 0 && \
             n.indexOf('createElement') >= 0 && \
             n.indexOf('querySelectorAll') < n.indexOf('getElementsByTagName') && \
             n.indexOf('getElementsByTagName') < n.indexOf('getElementsByClassName') && \
             n.indexOf('getElementsByClassName') < n.indexOf('getElementsByName') && \
             n.indexOf('getElementsByName') < n.indexOf('createElement');",
        )
        .unwrap();
    assert!(
        matches!(order_ok, JsValue::Boolean(true)),
        "live-collection getters must enumerate between querySelectorAll and \
         createElement (original DOCUMENT_METHODS order); got {order_ok:?}"
    );
    vm.unbind();
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
fn legacy_excluded_via_table_and_handler_attr_seams() {
    // A0 acceptance row (Codex R11): Legacy exclusion must be proven through **all
    // four** install seams. `legacy_probe_withheld_in_core_modes` covers seam-2
    // (direct `register_*_global`); the `DomApiHandler` registry covers seam-4
    // (elidex-dom-api). This closes the remaining two:
    //   • seam-1 = method/accessor **table** install (`install_ro_accessors`):
    //     `__a1LegacyAccessorProbe` on `globalThis`.
    //   • seam-3 = event-handler-attr install (`install_bound_accessor_pair` in the
    //     `install_handler_attr_family` loop): `__a1LegacyHandlerProbe` on `document`.
    // bind_vm exposes both `globalThis` and `document` (the accessor probe installs
    // at construction; the handler probe at the document bind).
    fn probes_present(mode: EngineMode) -> (bool, bool) {
        let mut vm = Vm::new_with_mode(mode);
        let mut session = SessionCore::new();
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        #[allow(unsafe_code)]
        unsafe {
            bind_vm(&mut vm, &mut session, &mut dom, doc);
        }
        let accessor = matches!(
            vm.eval("'__a1LegacyAccessorProbe' in globalThis;").unwrap(),
            JsValue::Boolean(true)
        );
        let handler = matches!(
            vm.eval("'__a1LegacyHandlerProbe' in document;").unwrap(),
            JsValue::Boolean(true)
        );
        vm.unbind();
        (accessor, handler)
    }

    // BrowserCore / App withhold the Legacy probes at both seams, both profiles.
    for mode in [EngineMode::BrowserCore, EngineMode::App] {
        let (accessor, handler) = probes_present(mode);
        assert!(
            !accessor,
            "{mode:?}: seam-1 (accessor table) Legacy probe must be withheld"
        );
        assert!(
            !handler,
            "{mode:?}: seam-3 (handler attr) Legacy probe must be withheld"
        );
    }

    // BrowserCompat installs them only when the compat shims are compiled in.
    let (accessor, handler) = probes_present(EngineMode::BrowserCompat);
    #[cfg(feature = "compat-webapi")]
    {
        assert!(
            accessor,
            "BrowserCompat (+compat-webapi): seam-1 accessor-table probe must install"
        );
        assert!(
            handler,
            "BrowserCompat (+compat-webapi): seam-3 handler-attr probe must install"
        );
    }
    #[cfg(not(feature = "compat-webapi"))]
    {
        assert!(
            !accessor,
            "compat-webapi-off ceiling must withhold the seam-1 probe under BrowserCompat"
        );
        assert!(
            !handler,
            "compat-webapi-off ceiling must withhold the seam-3 probe under BrowserCompat"
        );
    }
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

#[cfg(feature = "compat-webapi")]
#[test]
fn storage_globals_absent_in_worker_realm_under_compat() {
    // A2 realm-scope correction: `Storage` / `StorageEvent` are `[Exposed=Window]`
    // (HTML §12.2.1 / §12.2.4), so even under `BrowserCompat` — where `Legacy`
    // installs — a dedicated-worker / service-worker realm must NOT expose them.
    // This is orthogonal to the mode gate (the Window VM below is the positive
    // control proving they DO install when the realm is right).
    let win = Vm::new(); // Window, BrowserCompat
    assert!(
        win.get_global("Storage").is_some(),
        "Window must expose Storage under BrowserCompat (positive control)"
    );
    assert!(
        win.get_global("StorageEvent").is_some(),
        "Window must expose StorageEvent under BrowserCompat (positive control)"
    );

    let worker = Vm::new_worker(
        "w".to_string(),
        url::Url::parse("https://example.com/w.js").unwrap(),
        true,
        elidex_net::CredentialsMode::SameOrigin,
        EngineMode::BrowserCompat,
    );
    assert!(
        worker.get_global("Storage").is_none(),
        "dedicated worker must not expose Storage ([Exposed=Window])"
    );
    assert!(
        worker.get_global("StorageEvent").is_none(),
        "dedicated worker must not expose StorageEvent ([Exposed=Window])"
    );

    let sw = Vm::new_service_worker(
        url::Url::parse("https://example.com/").unwrap(),
        url::Url::parse("https://example.com/sw.js").unwrap(),
        true,
        elidex_net::CredentialsMode::SameOrigin,
        EngineMode::BrowserCompat,
    );
    assert!(
        sw.get_global("Storage").is_none(),
        "service worker must not expose Storage ([Exposed=Window])"
    );
    assert!(
        sw.get_global("StorageEvent").is_none(),
        "service worker must not expose StorageEvent ([Exposed=Window])"
    );
}
