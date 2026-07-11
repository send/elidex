//! S1d (boa→VM cutover): the `HostDriver` contract — the shell↔engine
//! host-drive surface ([`ElidexJsEngine`] as `HostDriver`). Covers the two NEW
//! accessors (`next_timer_deadline` lazy-cancel filtering, `sw_controller_scope`
//! round-trip), a real `install_network_handle` + `tick_network` fetch settle,
//! and a generic-dispatch driver that both **locks** the §2.2 decision (the
//! shell pipeline is generic `E: ScriptEngine + HostDriver`, never
//! `dyn HostDriver`) and exercises the per-turn deliver/drain forwards against a
//! bound VM.
//!
//! Like S1a/S1b/S1c these drive the VM through the engine's own batch bracket
//! while boa stays live (the S5 flip wires the shell to call these instead).
//! See `memory/boa-vm-cutover-s1d-plan.md` §5/§7.

#![cfg(feature = "engine")]

use std::rc::Rc;
use std::time::{Duration, Instant};

use elidex_ecs::{EcsDom, Entity};
use elidex_net::broker::NetworkHandle;
use elidex_net::{HttpVersion, Response as NetResponse};
use elidex_script_session::{HostDriver, ScriptContext, ScriptEngine, SessionCore};
use url::Url;

use crate::engine::ElidexJsEngine;
use crate::vm::value::JsValue;

/// Construct an unbound engine + session + dom with a fresh `document_root`
/// (mirrors `tests_engine_s1c::fresh_unbound`).
fn fresh_unbound() -> (ElidexJsEngine, SessionCore, EcsDom, Entity) {
    let mut engine = ElidexJsEngine::new();
    let session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    (engine, session, dom, doc)
}

/// Open the engine's batch bracket (see `tests_engine_s1a::bind_engine`).
#[allow(unsafe_code)]
fn bind_engine(engine: &mut ElidexJsEngine, ctx: &mut ScriptContext<'_>) {
    // SAFETY: the bracket stays open until the paired `unbind`, and no test body
    // aliases `ctx.session`/`ctx.dom` while bound.
    unsafe { engine.bind(ctx) }
}

fn url(s: &str) -> Url {
    Url::parse(s).expect("valid test URL")
}

fn global_true(engine: &mut ElidexJsEngine, name: &str) -> bool {
    matches!(engine.vm().get_global(name), Some(JsValue::Boolean(true)))
}

fn global_string(engine: &mut ElidexJsEngine, name: &str) -> String {
    match engine.vm().get_global(name) {
        Some(JsValue::String(id)) => engine.vm().get_string(id),
        other => panic!("expected string global `{name}`, got {other:?}"),
    }
}

/// A 200 `text/plain` mock response (mirrors `tests_fetch::ok_response`).
fn ok_response(target: &str, status: u16, body: &'static str) -> NetResponse {
    let parsed = url(target);
    NetResponse {
        status,
        headers: vec![("content-type".to_string(), "text/plain".to_string())],
        body: bytes::Bytes::from_static(body.as_bytes()),
        url: parsed.clone(),
        version: HttpVersion::H1,
        url_list: vec![parsed],
        is_redirect_tainted: false,
        credentialed_network: false,
    }
}

// ---------------------------------------------------------------------------
// Generic dispatch lock + per-turn forward smoke
// ---------------------------------------------------------------------------

/// The shell pipeline drives the engine generically over
/// `E: ScriptEngine + HostDriver` (the §2.2 decision — the generic `with_bound`
/// makes `HostDriver` non-object-safe, so the shell monomorphises rather than
/// using `dyn HostDriver`). Compiling this generic driver is the contract lock;
/// running it also asserts the per-turn deliver/drain forwards reach the VM
/// without panicking against a bound engine.
#[allow(unsafe_code)]
fn drive<E: ScriptEngine + HostDriver>(engine: &mut E, ctx: &mut ScriptContext<'_>) {
    // Read-side forwards are valid before/outside a bracket.
    assert_eq!(engine.history_length(), 1);
    assert_eq!(engine.current_url(), Some(url("about:blank")));
    assert!(engine.next_timer_deadline().is_none());
    assert!(engine.sw_controller_scope().is_none());
    assert!(engine.forms_allowed());
    assert!(engine.popups_allowed());
    assert_eq!(engine.iframe_depth(), 0);
    let _ = engine.origin();
    let _ = engine.sandbox_flags();

    // Bound per-turn forwards. SAFETY: `ctx` outlives the bracket and the
    // closure body drives the VM only through the bound engine `e` — it never
    // touches `c.session`/`c.dom` directly (`with_bound` contract).
    unsafe {
        engine.with_bound(ctx, |e, c| {
            assert!(ScriptEngine::eval(e, "globalThis.driven = true;", c).success);
            e.deliver_mutation_records(&[]);
            e.deliver_resize_observations();
            e.deliver_intersection_observations();
            e.tick_network();
            e.sync_dirty_canvases();
            e.drain_worker_messages();
            let _ = e.drain_sw_client_requests();
            e.set_navigation_referrer(Some(url("https://ref.example/p")));
        });
    }
}

#[test]
fn host_driver_is_usable_via_generic_dispatch() {
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    drive(&mut engine, &mut ctx);
    assert!(
        global_true(&mut engine, "driven"),
        "the generic driver bound the VM and ran a script through it"
    );
}

// ---------------------------------------------------------------------------
// next_timer_deadline — the NEW accessor (min over live entries, skip cancelled)
// ---------------------------------------------------------------------------

#[test]
fn next_timer_deadline_returns_min_and_skips_cancelled() {
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    bind_engine(&mut engine, &mut ctx);

    // No timers scheduled → no wake hint.
    assert!(
        engine.next_timer_deadline().is_none(),
        "no deadline before any timer is scheduled"
    );

    // Schedule a near (1s) and a far (100s) timer; neither fires during the test.
    let t0 = Instant::now();
    assert!(
        ScriptEngine::eval(
            &mut engine,
            "globalThis.near = setTimeout(() => {}, 1000);
         setTimeout(() => {}, 100000);",
            &mut ctx,
        )
        .success
    );

    // The accessor returns the MIN deadline — the 1s timer, not the heap-head
    // 100s one.
    let near = engine.next_timer_deadline().expect("a timer is scheduled");
    assert!(
        near < t0 + Duration::from_secs(50),
        "returns the earliest (1s) deadline, not the 100s timer"
    );

    // Cancel the near timer: `clearTimeout` only marks it (lazy-cancel), leaving
    // its entry in the heap until a drain. The accessor MUST skip the cancelled
    // entry and report the still-live 100s timer — not the cancelled head.
    assert!(ScriptEngine::eval(&mut engine, "clearTimeout(globalThis.near);", &mut ctx).success);
    let far = engine
        .next_timer_deadline()
        .expect("the 100s timer is still live");
    assert!(
        far > t0 + Duration::from_secs(50),
        "skips the lazily-cancelled 1s entry and returns the 100s timer"
    );

    engine.unbind();
}

// ---------------------------------------------------------------------------
// seed_sw_client + sw_controller_scope — the NEW read accessor round-trip
// ---------------------------------------------------------------------------

#[test]
fn seed_sw_client_round_trips_through_sw_controller_scope() {
    let mut engine = ElidexJsEngine::new();

    // Uncontrolled page → no controller scope.
    assert!(
        engine.sw_controller_scope().is_none(),
        "a fresh page is uncontrolled"
    );

    // Seed a controller scope (no registrations needed for the controller field).
    let scope = url("https://example.com/app/");
    engine.seed_sw_client(Some(scope.clone()), &[]);
    assert_eq!(
        engine.sw_controller_scope(),
        Some(scope),
        "the seeded controller scope reads back as a parsed Url"
    );

    // Re-seeding `None` returns the page to uncontrolled.
    engine.seed_sw_client(None, &[]);
    assert!(
        engine.sw_controller_scope().is_none(),
        "seeding None clears the controller"
    );
}

// ---------------------------------------------------------------------------
// install_network_handle + tick_network — real fetch settlement
// ---------------------------------------------------------------------------

#[test]
fn install_network_handle_then_tick_network_settles_fetch() {
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    // Same-origin http context so the fetch classifies as Basic (not opaque-cors).
    engine.set_current_url(Some(url("http://example.com/page")));
    engine.install_network_handle(Rc::new(NetworkHandle::mock_with_responses(vec![(
        url("http://example.com/data"),
        Ok(ok_response("http://example.com/data", 200, "hello-s1d")),
    )])));

    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    bind_engine(&mut engine, &mut ctx);
    assert!(
        ScriptEngine::eval(
            &mut engine,
            "globalThis.body = 'unset';
         fetch('http://example.com/data').then(r => r.text()).then(t => { globalThis.body = t; });",
            &mut ctx,
        )
        .success
    );

    // `HostDriver::tick_network` fuses fetch settlement + the microtask
    // checkpoint; loop until the pending fetch settles (mirrors the VM
    // fetch-test harness `drain_fetch_replies`).
    for _ in 0..16 {
        engine.tick_network();
    }
    engine.unbind();

    assert_eq!(
        global_string(&mut engine, "body"),
        "hello-s1d",
        "tick_network settled the installed mock handle's fetch reply"
    );
}
