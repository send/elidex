//! S1b (boa→VM cutover): the per-VM security context — the
//! `document_origin` store + its single [`VmInner::document_origin`]
//! resolver, the shell-facing sandbox/origin accessors on
//! [`ElidexJsEngine`], and the §5 origin unification (the 4 settings-object
//! origin readers migrate; `location.origin` deliberately does NOT — CRIT F1).
//!
//! See `memory/boa-vm-cutover-s1b-plan.md` §5/§6. Like S1a, these drive the
//! VM through the engine's own batch bracket while boa stays live.

#![cfg(feature = "engine")]

use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::{IframeSandboxFlags, SecurityOrigin};
use elidex_script_session::{HostDriver, ScriptContext, ScriptEngine, SessionCore};
use url::Url;

use crate::engine::ElidexJsEngine;
use crate::vm::value::JsValue;

/// Construct an unbound engine + session + dom with a fresh `document_root`
/// (mirrors `tests_engine_s1a::fresh_unbound`).
fn fresh_unbound() -> (ElidexJsEngine, SessionCore, EcsDom, Entity) {
    let engine = ElidexJsEngine::new();
    let session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    (engine, session, dom, doc)
}

/// Point the bound document at `url` (the shell installs this via the
/// navigation back-channel; tests set the field directly).
fn set_current_url(engine: &mut ElidexJsEngine, url: &str) {
    engine.vm().inner.navigation.current_url = Url::parse(url).expect("valid test URL");
}

/// Open the engine's batch bracket (see `tests_engine_s1a::bind_engine`).
#[allow(unsafe_code)]
fn bind_engine(engine: &mut ElidexJsEngine, ctx: &mut ScriptContext<'_>) {
    // SAFETY: the bracket stays open until the paired `unbind`, and no test
    // body aliases `ctx.session`/`ctx.dom` while bound.
    unsafe { engine.bind(ctx) }
}

/// Read a string global a script assigned (e.g. `globalThis.lo`).
fn global_string(engine: &mut ElidexJsEngine, name: &str) -> String {
    match engine.vm().get_global(name) {
        Some(JsValue::String(id)) => engine.vm().get_string(id),
        other => panic!("expected string global `{name}`, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// §5 resolver — VmInner::document_origin()
// ---------------------------------------------------------------------------

#[test]
fn document_origin_unset_derives_from_current_url() {
    // No override installed → the resolver derives the origin from
    // `current_url` (the spec default, HTML §7.1.1). `ElidexJsEngine::origin`
    // returns the resolved value (parity with boa `bridge().origin()`).
    let mut engine = fresh_unbound().0;
    set_current_url(&mut engine, "https://example.com/page?q=1#frag");
    assert_eq!(engine.origin().serialize(), "https://example.com");
}

#[test]
fn document_origin_default_port_omitted_and_explicit_port_kept() {
    let mut engine = fresh_unbound().0;
    set_current_url(&mut engine, "https://example.com:443/");
    assert_eq!(engine.origin().serialize(), "https://example.com");
    set_current_url(&mut engine, "http://example.com:8080/");
    assert_eq!(engine.origin().serialize(), "http://example.com:8080");
}

#[test]
fn set_origin_opaque_override_reports_null_even_with_real_url() {
    // The sandboxed-iframe case: `current_url` is a real https URL but the
    // installed document origin is opaque → the resolver reports it opaque,
    // so settings-object-origin surfaces serialize to "null".
    let mut engine = fresh_unbound().0;
    set_current_url(&mut engine, "https://real.example.com/page");
    engine.set_origin(SecurityOrigin::opaque());
    assert_eq!(engine.origin().serialize(), "null");
    assert!(matches!(engine.origin(), SecurityOrigin::Opaque(_)));
}

#[test]
fn set_origin_tuple_override_wins_over_current_url() {
    let mut engine = fresh_unbound().0;
    set_current_url(&mut engine, "https://real.example.com/page");
    engine.set_origin(SecurityOrigin::from_url(
        &Url::parse("https://override.example").unwrap(),
    ));
    assert_eq!(engine.origin().serialize(), "https://override.example");
}

#[test]
fn document_origin_blob_url_reports_inner_origin_not_null() {
    // An unsandboxed `blob:` document (no override installed) must resolve to
    // its inner URL's origin, not opaque — otherwise the migrated
    // settings-origin readers (postMessage / WS+SSE `Origin` / storage) would
    // report "null" where the prior `current_url.origin()` reported the real
    // origin. Guards the §5 resolver's dependency on `SecurityOrigin::from_url`
    // handling blob URLs (URL Standard "origin of a URL", blob steps).
    let mut engine = fresh_unbound().0;
    set_current_url(
        &mut engine,
        "blob:https://example.com/550e8400-e29b-41d4-a716-446655440000",
    );
    assert_eq!(engine.origin().serialize(), "https://example.com");
}

#[test]
fn document_origin_opaque_fallback_is_identity_stable() {
    // No override + an opaque `current_url` (the standalone / about:blank
    // pipeline path, where the shell never calls `set_origin` because
    // `current_url` is `None`) must resolve to ONE stable opaque per VM, not a
    // fresh `Opaque(n)` per read. iframe/lifecycle.rs reads `bridge().origin()`
    // and propagates it parent→child, so a re-minting fallback would hand the
    // child a different parent origin each call (Codex R2). A document's origin
    // is stable document state (HTML §7.1.1).
    let mut engine = fresh_unbound().0;
    set_current_url(&mut engine, "about:blank");
    let first = engine.origin();
    let second = engine.origin();
    assert!(matches!(first, SecurityOrigin::Opaque(_)));
    assert_eq!(
        first, second,
        "opaque fallback must be identity-stable across reads"
    );
    assert_eq!(first.serialize(), "null");
}

#[test]
fn document_origin_resolves_and_reflects_override() {
    // A fresh engine (HostData installed by construction) resolves without
    // panicking — the default about:blank `current_url` yields an opaque "null".
    let mut engine = ElidexJsEngine::new();
    assert_eq!(engine.origin().serialize(), "null");
    // With HostData installed by the engine ctor, `set_origin` stores the
    // override and the resolver returns it. (The pre-flip "no HostData → no-op"
    // path is now unreachable at the engine level: `ElidexJsEngine::new` always
    // installs a default `HostData`; only a bare `Vm::new()` lacks one.)
    engine.set_origin(SecurityOrigin::from_url(
        &Url::parse("https://x.example").unwrap(),
    ));
    assert_eq!(engine.origin().serialize(), "https://x.example");
}

// ---------------------------------------------------------------------------
// §6 shell-facing sandbox accessors
// ---------------------------------------------------------------------------

#[test]
fn iframe_depth_round_trips() {
    let mut engine = fresh_unbound().0;
    assert_eq!(engine.iframe_depth(), 0);
    engine.set_iframe_depth(4);
    assert_eq!(engine.iframe_depth(), 4);
}

#[test]
fn sandbox_capability_accessors_track_flags() {
    let mut engine = fresh_unbound().0;
    // Unsandboxed (no flags) → everything allowed; getter is None.
    assert_eq!(engine.sandbox_flags(), None);
    assert!(engine.forms_allowed());
    assert!(engine.popups_allowed());

    // Sandboxed with no allow-* tokens → forms + popups denied.
    engine.set_sandbox_flags(Some(IframeSandboxFlags::empty()));
    assert_eq!(engine.sandbox_flags(), Some(IframeSandboxFlags::empty()));
    assert!(!engine.forms_allowed());
    assert!(!engine.popups_allowed());

    // Granting allow-forms flips only forms.
    engine.set_sandbox_flags(Some(IframeSandboxFlags::ALLOW_FORMS));
    assert!(engine.forms_allowed());
    assert!(!engine.popups_allowed());

    // Granting allow-popups flips only popups.
    engine.set_sandbox_flags(Some(IframeSandboxFlags::ALLOW_POPUPS));
    assert!(!engine.forms_allowed());
    assert!(engine.popups_allowed());
}

#[test]
fn sandbox_accessors_default_open() {
    // A fresh engine's default `HostData` is unsandboxed, so the sandbox
    // accessors default permissive (the absence of an installed security
    // context never silently denies).
    // (The read accessors are `&self`, so no `mut` binding is needed here.)
    let engine = ElidexJsEngine::new();
    assert!(engine.forms_allowed());
    assert!(engine.popups_allowed());
    assert_eq!(engine.sandbox_flags(), None);
    assert_eq!(engine.iframe_depth(), 0);
}

// ---------------------------------------------------------------------------
// §5 migration — observable via eval
// ---------------------------------------------------------------------------

#[test]
fn location_origin_stays_url_derived_for_opaque_document() {
    // CRIT F1 guard: HTML §7.2.4 — `location.origin` is the serialization of
    // the Location *URL's* origin, NOT the document origin. So even when the
    // document origin is opaque (sandboxed), `location.origin` reports the
    // real URL's origin — it must NOT become "null". This is exactly where
    // `window.origin`/`self.origin` (settings-object, opaque) diverge.
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    set_current_url(&mut engine, "https://example.com/page");
    engine.set_origin(SecurityOrigin::opaque());
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    bind_engine(&mut engine, &mut ctx);
    let r = ScriptEngine::eval(&mut engine, "globalThis.lo = location.origin;", &mut ctx);
    assert!(r.success);
    engine.unbind();
    assert_eq!(global_string(&mut engine, "lo"), "https://example.com");
}

#[test]
fn window_postmessage_origin_uses_document_origin() {
    // window.postMessage's MessageEvent.origin = incumbentSettings's origin
    // (HTML §9.3.3) = the document origin. An opaque document origin →
    // "null"; this is a migrated §5 reader (`compute_own_origin_sid`).
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    set_current_url(&mut engine, "https://example.com/page");
    engine.set_origin(SecurityOrigin::opaque());
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    bind_engine(&mut engine, &mut ctx);
    // Delivery fires during the eval's drain_tasks; read the captured origin.
    let r = ScriptEngine::eval(
        &mut engine,
        "globalThis.got = 'unset';
         window.addEventListener('message', function(e){ globalThis.got = e.origin; });
         window.postMessage(0, '*');",
        &mut ctx,
    );
    assert!(r.success);
    engine.unbind();
    assert_eq!(global_string(&mut engine, "got"), "null");
}

#[test]
fn window_postmessage_origin_is_real_origin_when_unsandboxed() {
    // The positive counterpart: no override → the migrated reader serializes
    // the `current_url`-derived document origin (unchanged for a normal doc).
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    set_current_url(&mut engine, "https://example.com/page");
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    bind_engine(&mut engine, &mut ctx);
    let r = ScriptEngine::eval(
        &mut engine,
        "globalThis.got = 'unset';
         window.addEventListener('message', function(e){ globalThis.got = e.origin; });
         window.postMessage(0, '*');",
        &mut ctx,
    );
    assert!(r.success);
    engine.unbind();
    assert_eq!(global_string(&mut engine, "got"), "https://example.com");
}

// ---------------------------------------------------------------------------
// §5 storage isolation — the resolver pivot
// ---------------------------------------------------------------------------

#[test]
fn storage_origin_discriminant_is_document_origin_not_current_url() {
    // localStorage is partitioned by the document origin (HTML §12.2.3). The
    // S1b change to `storage::current_origin` is solely that it decides
    // tuple-vs-opaque from `document_origin()` rather than
    // `current_url.origin()`; the opaque→`opaque_origin_sentinel` /
    // tuple→serialize mapping is unchanged. The behaviour-defining pivot is
    // therefore that a sandboxed doc — real tuple `current_url`, opaque
    // override — resolves to an *opaque* origin (→ the per-VM sentinel key,
    // isolated from the real origin's storage), while the same `current_url`
    // without the override resolves to the real tuple origin (→ shared key).
    let mut engine = fresh_unbound().0;
    set_current_url(&mut engine, "https://example.com/app");

    // Unsandboxed: tuple origin → real-origin storage key.
    assert!(matches!(engine.origin(), SecurityOrigin::Tuple { .. }));
    assert_eq!(engine.origin().serialize(), "https://example.com");

    // Sandboxed (opaque override) with the SAME real URL: opaque origin →
    // the storage discriminant is opaque, so keying falls to the per-VM
    // sentinel rather than "https://example.com" — the isolation fix.
    engine.set_origin(SecurityOrigin::opaque());
    assert!(matches!(engine.origin(), SecurityOrigin::Opaque(_)));
}
