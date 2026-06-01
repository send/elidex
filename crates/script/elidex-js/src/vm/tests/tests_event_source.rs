//! D-12 `#11-net-ws-sse` Phase 3 — `EventSource` JS thin binding
//! tests (WHATWG HTML §9.2).
//!
//! Coverage:
//! - Constructor success path + URL relative-resolution against the
//!   document base + `init.withCredentials` echo + brand promotion
//!   + reverse-map population.
//! - `readyState` accessor + CONNECTING-on-construction invariant.
//! - State machine transitions via `dispatch_realtime_event`:
//!   Connecting→Open (Connected), Open→Connecting (transient
//!   Error auto-reconnect, IMP-3), →Closed (FatalError + JS
//!   `close()`).
//! - `close()` idempotency + `EventSourceClose(conn_id)` emit.
//! - `onopen` / `onmessage` / `onerror` handler accessor pairs
//!   (callable-only retention).
//! - `addEventListener` / `removeEventListener` / `dispatchEvent`
//!   inherited from `EventTarget.prototype` and routed through the
//!   shared §2.9 VmObject core (`#11-realtime-event-listeners`):
//!   named-event routing (`onmessage` NOT fired for named events) +
//!   `message`-type both-fire fan-out (§9.2.6) + capture / once /
//!   {signal} support + non-callable listener TypeError + the F10
//!   single-home structural guard (no bespoke own `addEventListener`).
//! - `MessageEvent` slot population (data / origin / lastEventId
//!   sticky / source / ports) including the lastEventId
//!   accumulator semantics across multiple events.

#![cfg(feature = "engine")]

use std::rc::Rc;

use elidex_ecs::{Attributes, EcsDom};
use elidex_net::broker::{NetworkHandle, NetworkToRenderer};
use elidex_net::sse::SseEvent;
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;
use super::{assert_eval_bool, assert_eval_number, assert_eval_string};

fn build_min_doc(dom: &mut EcsDom) -> elidex_ecs::Entity {
    let doc = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    let body = dom.create_element("body", Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, body));
    doc
}

struct UnbindOnDrop<'a>(&'a mut Vm);

impl Drop for UnbindOnDrop<'_> {
    fn drop(&mut self) {
        self.0.unbind();
    }
}

/// Run `f` against a VM bound to a fresh `HostData` session with a
/// mock `NetworkHandle` installed and the navigation URL pointing
/// at `https://example.com/page/` (with a trailing slash so
/// `Url::join("/events")` produces `https://example.com/events`
/// rather than dropping the `page` segment).
fn with_es_vm<F, R>(f: F) -> R
where
    F: FnOnce(&mut Vm) -> R,
{
    let mut vm = Vm::new();
    vm.inner.navigation.current_url =
        url::Url::parse("https://example.com/page/").expect("valid base URL");
    vm.install_network_handle(Rc::new(NetworkHandle::mock_with_responses(vec![])));

    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_min_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let guard = UnbindOnDrop(&mut vm);
    let result = f(guard.0);
    drop(guard);
    drop(session);
    drop(dom);
    result
}

/// Inject an `SseEvent` for `conn_id` and drive a network tick —
/// the dispatch helper inside `tick_network` will route the event
/// to the matching wrapper via the reverse map.
fn inject_sse_event_and_tick(vm: &mut Vm, conn_id: u64, ev: SseEvent) {
    let handle = vm.inner.network_handle.clone().expect("handle installed");
    handle.rebuffer_events(vec![NetworkToRenderer::EventSourceEvent(conn_id, ev)]);
    vm.tick_network();
}

/// Build a `SseEvent::Connected { final_url }` for tests.  The
/// no-redirect case passes the same URL the ctor used; the
/// post-redirect case passes whatever the broker would have
/// settled on after following 3xx hops.  Centralises the
/// construction so a future `Connected` payload addition only
/// touches this helper.
fn connected_event(url_str: &str) -> SseEvent {
    SseEvent::Connected {
        final_url: url::Url::parse(url_str).expect("valid test URL"),
    }
}

// ---------------------------------------------------------------------------
// Constructor: success path + URL relative-resolution + init dict
// ---------------------------------------------------------------------------

#[test]
fn ctor_accepts_absolute_https_url() {
    with_es_vm(|vm| {
        vm.eval("globalThis.s = new EventSource('https://example.com/events');")
            .expect("ctor succeeds");
        assert_eval_number(vm, "s.readyState", 0.0);
        assert_eval_string(vm, "s.url", "https://example.com/events");
        assert_eval_bool(vm, "s.withCredentials", false);
    });
}

#[test]
fn ctor_resolves_relative_url_against_document_base() {
    // WHATWG HTML §9.2.1 — relative-URL resolution against the
    // settings object's API base URL.  Base is
    // `https://example.com/page/` (trailing slash); `/events`
    // resolves to `https://example.com/events`.
    with_es_vm(|vm| {
        vm.eval("globalThis.s = new EventSource('/events');")
            .expect("relative URL resolves");
        assert_eval_string(vm, "s.url", "https://example.com/events");
    });
}

#[test]
fn ctor_resolves_dot_relative_url() {
    // `./feed` relative to base `https://example.com/page/` →
    // `https://example.com/page/feed`.
    with_es_vm(|vm| {
        vm.eval("globalThis.s = new EventSource('./feed');")
            .expect("dot-relative URL resolves");
        assert_eval_string(vm, "s.url", "https://example.com/page/feed");
    });
}

#[test]
fn ctor_rejects_garbage_url() {
    with_es_vm(|vm| {
        let err = vm.eval("new EventSource('http://[invalid');").unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.contains("Syntax"), "expected SyntaxError, got: {msg}");
    });
}

#[test]
fn ctor_rejects_missing_url_arg() {
    with_es_vm(|vm| {
        let err = vm.eval("new EventSource();").unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("argument") || msg.contains("required"),
            "expected arity error, got: {msg}"
        );
    });
}

#[test]
fn ctor_requires_new_operator() {
    super::assert_ctor_requires_new("EventSource('https://example.com/events')", "EventSource");
}

#[test]
fn ctor_with_credentials_true_echoes() {
    with_es_vm(|vm| {
        vm.eval(
            "globalThis.s = new EventSource('https://example.com/events', \
             { withCredentials: true });",
        )
        .expect("ctor with init succeeds");
        assert_eval_bool(vm, "s.withCredentials", true);
    });
}

#[test]
fn ctor_with_credentials_false_default() {
    with_es_vm(|vm| {
        vm.eval("globalThis.s = new EventSource('https://example.com/events', {});")
            .expect("empty init ok");
        assert_eval_bool(vm, "s.withCredentials", false);
    });
}

#[test]
fn ctor_with_undefined_init_treated_as_empty_dict() {
    with_es_vm(|vm| {
        vm.eval("globalThis.s = new EventSource('https://example.com/events', undefined);")
            .expect("undefined init ok");
        assert_eval_bool(vm, "s.withCredentials", false);
    });
}

#[test]
fn ctor_with_null_init_treated_as_empty_dict() {
    with_es_vm(|vm| {
        vm.eval("globalThis.s = new EventSource('https://example.com/events', null);")
            .expect("null init ok");
        assert_eval_bool(vm, "s.withCredentials", false);
    });
}

#[test]
fn ctor_with_primitive_init_throws_type_error() {
    // WebIDL §3.10.6 — non-Object dict argument throws TypeError.
    with_es_vm(|vm| {
        let err = vm
            .eval("new EventSource('https://example.com/events', 42);")
            .unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("Type") && msg.contains("object"),
            "expected TypeError for primitive init, got: {msg}"
        );
    });
}

// ---------------------------------------------------------------------------
// readyState transitions via dispatch_realtime_event
// ---------------------------------------------------------------------------

#[test]
fn ready_state_starts_at_connecting() {
    with_es_vm(|vm| {
        vm.eval("globalThis.s = new EventSource('/events');")
            .unwrap();
        assert_eval_number(vm, "s.readyState", 0.0);
    });
}

#[test]
fn ready_state_transitions_connecting_to_open_via_connected_event() {
    with_es_vm(|vm| {
        vm.eval("globalThis.s = new EventSource('/events');")
            .unwrap();
        inject_sse_event_and_tick(vm, 0, connected_event("https://example.com/events"));
        assert_eval_number(vm, "s.readyState", 1.0);
    });
}

#[test]
fn ready_state_open_to_connecting_via_transient_error_then_back() {
    // IMP-3 regression: SSE transient Error transitions
    // OPEN→CONNECTING during auto-reconnect, NOT stays at OPEN.
    with_es_vm(|vm| {
        vm.eval("globalThis.s = new EventSource('/events');")
            .unwrap();
        inject_sse_event_and_tick(vm, 0, connected_event("https://example.com/events"));
        assert_eval_number(vm, "s.readyState", 1.0);
        inject_sse_event_and_tick(vm, 0, SseEvent::Error("transient".to_string()));
        assert_eval_number(vm, "s.readyState", 0.0);
        // Reconnect succeeds: another Connected snaps back to OPEN.
        inject_sse_event_and_tick(vm, 0, connected_event("https://example.com/events"));
        assert_eval_number(vm, "s.readyState", 1.0);
    });
}

#[test]
fn ready_state_transitions_to_closed_via_fatal_error() {
    with_es_vm(|vm| {
        vm.eval("globalThis.s = new EventSource('/events');")
            .unwrap();
        inject_sse_event_and_tick(vm, 0, connected_event("https://example.com/events"));
        inject_sse_event_and_tick(vm, 0, SseEvent::FatalError("server gone".to_string()));
        assert_eval_number(vm, "s.readyState", 2.0);
    });
}

// ---------------------------------------------------------------------------
// close() — idempotency + state transition
// ---------------------------------------------------------------------------

#[test]
fn close_transitions_to_closed() {
    with_es_vm(|vm| {
        vm.eval("globalThis.s = new EventSource('/events');")
            .unwrap();
        inject_sse_event_and_tick(vm, 0, connected_event("https://example.com/events"));
        vm.eval("s.close();").expect("close ok");
        assert_eval_number(vm, "s.readyState", 2.0);
    });
}

#[test]
fn close_is_idempotent_when_already_closed() {
    with_es_vm(|vm| {
        vm.eval("globalThis.s = new EventSource('/events');")
            .unwrap();
        vm.eval("s.close();").expect("first close ok");
        vm.eval("s.close();").expect("second close idempotent");
        assert_eval_number(vm, "s.readyState", 2.0);
    });
}

#[test]
fn close_from_connecting_state_works() {
    with_es_vm(|vm| {
        vm.eval("globalThis.s = new EventSource('/events');")
            .unwrap();
        assert_eval_number(vm, "s.readyState", 0.0);
        vm.eval("s.close();").expect("close from CONNECTING ok");
        assert_eval_number(vm, "s.readyState", 2.0);
    });
}

// ---------------------------------------------------------------------------
// onopen / onmessage / onerror handler attributes
// ---------------------------------------------------------------------------

#[test]
fn onopen_fires_with_open_event_after_connected_dispatch() {
    with_es_vm(|vm| {
        vm.eval(
            "globalThis._evts = []; \
             globalThis.s = new EventSource('/events'); \
             s.onopen = function(e) { globalThis._evts.push({t: e.type, tg: e.target === s}); };",
        )
        .unwrap();
        inject_sse_event_and_tick(vm, 0, connected_event("https://example.com/events"));
        assert_eval_number(vm, "_evts.length", 1.0);
        assert_eval_string(vm, "_evts[0].t", "open");
        assert_eval_bool(vm, "_evts[0].tg", true);
    });
}

#[test]
fn onmessage_fires_for_default_message_event_type() {
    with_es_vm(|vm| {
        vm.eval(
            "globalThis._evts = []; \
             globalThis.s = new EventSource('/events'); \
             s.onmessage = function(e) { \
               globalThis._evts.push({ \
                 t: e.type, d: e.data, lei: e.lastEventId, src: e.source, \
                 portsLen: e.ports.length, tg: e.target === s \
               }); \
             };",
        )
        .unwrap();
        inject_sse_event_and_tick(
            vm,
            0,
            SseEvent::Event {
                event_type: "message".to_string(),
                data: "hello".to_string(),
                last_event_id: String::new(),
            },
        );
        assert_eval_number(vm, "_evts.length", 1.0);
        assert_eval_string(vm, "_evts[0].t", "message");
        assert_eval_string(vm, "_evts[0].d", "hello");
        assert_eval_string(vm, "_evts[0].lei", "");
        assert_eval_bool(vm, "_evts[0].src === null", true);
        assert_eval_bool(vm, "_evts[0].tg", true);
        assert_eval_number(vm, "_evts[0].portsLen", 0.0);
    });
}

#[test]
fn onmessage_does_not_fire_for_named_events() {
    // §9.2 "Dispatch the event": `event: notification` named
    // events fire only to `addEventListener("notification", ...)`,
    // NOT to `onmessage`.
    with_es_vm(|vm| {
        vm.eval(
            "globalThis._count = 0; \
             globalThis.s = new EventSource('/events'); \
             s.onmessage = function() { globalThis._count++; };",
        )
        .unwrap();
        inject_sse_event_and_tick(
            vm,
            0,
            SseEvent::Event {
                event_type: "notification".to_string(),
                data: "x".to_string(),
                last_event_id: String::new(),
            },
        );
        assert_eval_number(vm, "_count", 0.0);
    });
}

#[test]
fn onerror_fires_plain_event_on_transient_error() {
    with_es_vm(|vm| {
        vm.eval(
            "globalThis._evts = []; \
             globalThis.s = new EventSource('/events'); \
             s.onerror = function(e) { \
               globalThis._evts.push({t: e.type, tg: e.target === s, msg: e.message}); \
             };",
        )
        .unwrap();
        inject_sse_event_and_tick(vm, 0, connected_event("https://example.com/events"));
        inject_sse_event_and_tick(vm, 0, SseEvent::Error("noisy network".to_string()));
        assert_eval_number(vm, "_evts.length", 1.0);
        assert_eval_string(vm, "_evts[0].t", "error");
        assert_eval_bool(vm, "_evts[0].tg", true);
        // Server-internals opacity: plain Event has no `message`.
        assert_eval_bool(vm, "_evts[0].msg === undefined", true);
    });
}

#[test]
fn onerror_fires_plain_event_on_fatal_error() {
    with_es_vm(|vm| {
        vm.eval(
            "globalThis._evts = []; \
             globalThis.s = new EventSource('/events'); \
             s.onerror = function(e) { globalThis._evts.push({t: e.type}); };",
        )
        .unwrap();
        inject_sse_event_and_tick(vm, 0, SseEvent::FatalError("404".to_string()));
        assert_eval_number(vm, "_evts.length", 1.0);
        assert_eval_string(vm, "_evts[0].t", "error");
        assert_eval_number(vm, "s.readyState", 2.0);
    });
}

#[test]
fn onmessage_setter_only_retains_callable_values() {
    with_es_vm(|vm| {
        vm.eval(
            "globalThis.s = new EventSource('/events'); \
             s.onmessage = function() {}; \
             s.onmessage = 42;",
        )
        .unwrap();
        match vm.eval("s.onmessage").unwrap() {
            JsValue::Null => {}
            other => panic!("onmessage should be null after non-callable, got {other:?}"),
        }
    });
}

#[test]
fn onmessage_setter_round_trips_callable() {
    with_es_vm(|vm| {
        vm.eval(
            "globalThis.s = new EventSource('/events'); \
             globalThis.fn = function() {}; \
             s.onmessage = fn;",
        )
        .unwrap();
        assert_eval_bool(vm, "s.onmessage === fn", true);
    });
}

// ---------------------------------------------------------------------------
// MessageEvent.origin + lastEventId sticky semantics
// ---------------------------------------------------------------------------

#[test]
fn message_event_origin_is_event_source_url_origin() {
    // Phase 2 lesson reapplied: origin is cached on side-table
    // at ctor time, computed from the SSE URL (NOT the page URL).
    // Page is at https://example.com but the EventSource targets
    // https://stream.example.org so the two are distinct.
    with_es_vm(|vm| {
        vm.eval(
            "globalThis._origin = null; \
             globalThis.s = new EventSource('https://stream.example.org:8443/feed'); \
             s.onmessage = function(e) { globalThis._origin = e.origin; };",
        )
        .unwrap();
        inject_sse_event_and_tick(
            vm,
            0,
            SseEvent::Event {
                event_type: "message".to_string(),
                data: "x".to_string(),
                last_event_id: String::new(),
            },
        );
        assert_eval_string(vm, "_origin", "https://stream.example.org:8443");
    });
}

#[test]
fn message_event_origin_reflects_final_url_after_redirect() {
    // WHATWG HTML §9.2 "Dispatch the event": `MessageEvent.origin`
    // is the serialization of the FINAL URL's origin (i.e. after
    // all HTTP 3xx redirects).  The broker's `connect_sse_stream`
    // follows redirects internally and surfaces the resolved URL
    // through `SseEvent::Connected { final_url }`; the dispatcher
    // refreshes `state.origin_sid` from it so post-Connected
    // message events observe the redirect-changed origin.
    with_es_vm(|vm| {
        vm.eval(
            "globalThis._origin = null; \
             globalThis.s = new EventSource('https://stream.example.org/feed'); \
             s.onmessage = function(e) { globalThis._origin = e.origin; };",
        )
        .unwrap();
        // Server-side 3xx pointed the broker at stream-cdn.example.net.
        inject_sse_event_and_tick(
            vm,
            0,
            connected_event("https://stream-cdn.example.net/feed"),
        );
        inject_sse_event_and_tick(
            vm,
            0,
            SseEvent::Event {
                event_type: "message".to_string(),
                data: "x".to_string(),
                last_event_id: String::new(),
            },
        );
        // Post-redirect origin (NOT the ctor URL's origin).
        assert_eval_string(vm, "_origin", "https://stream-cdn.example.net");
    });
}

#[test]
fn message_event_origin_uses_ctor_url_until_connected() {
    // Locks in the "defensive ctor default" contract: even in the
    // (unreachable-in-practice) window between ctor return and the
    // broker's first `Connected`, a message dispatched against the
    // instance observes the ctor URL's origin.  Without the ctor
    // seed the JS-visible `e.origin` would be the empty intern
    // (`well_known.empty`) — neither spec-compliant nor browser-
    // parity.
    with_es_vm(|vm| {
        vm.eval(
            "globalThis._origin = null; \
             globalThis.s = new EventSource('https://stream.example.org/feed'); \
             s.onmessage = function(e) { globalThis._origin = e.origin; };",
        )
        .unwrap();
        // NO Connected injected — dispatch_sse_event still fires
        // (it does not gate on ready_state) and uses the seeded
        // ctor origin from EventSourceState::origin_sid.
        inject_sse_event_and_tick(
            vm,
            0,
            SseEvent::Event {
                event_type: "message".to_string(),
                data: "x".to_string(),
                last_event_id: String::new(),
            },
        );
        assert_eval_string(vm, "_origin", "https://stream.example.org");
    });
}

#[test]
fn message_event_origin_refreshes_on_reconnect_with_different_redirect() {
    // Auto-reconnect cycle: the broker may settle on a different
    // final URL across reconnect attempts (e.g. a load-balancer
    // moving the stream endpoint).  Each fresh `Connected` MUST
    // refresh `state.origin_sid` so post-reconnect messages reflect
    // the new origin, not the previous one.  Locks the per-handshake
    // refresh invariant against a regression that caches the origin
    // only on the first Connected.
    with_es_vm(|vm| {
        vm.eval(
            "globalThis._origins = []; \
             globalThis.s = new EventSource('https://stream.example.org/feed'); \
             s.onmessage = function(e) { globalThis._origins.push(e.origin); };",
        )
        .unwrap();
        // First handshake → origin A.
        inject_sse_event_and_tick(vm, 0, connected_event("https://origin-a.example/feed"));
        inject_sse_event_and_tick(
            vm,
            0,
            SseEvent::Event {
                event_type: "message".to_string(),
                data: "first".to_string(),
                last_event_id: String::new(),
            },
        );
        // Transient error → Open→Connecting (no origin change yet).
        inject_sse_event_and_tick(vm, 0, SseEvent::Error("flaky".to_string()));
        // Reconnect handshake → origin B (different host).
        inject_sse_event_and_tick(vm, 0, connected_event("https://origin-b.example/feed"));
        inject_sse_event_and_tick(
            vm,
            0,
            SseEvent::Event {
                event_type: "message".to_string(),
                data: "second".to_string(),
                last_event_id: String::new(),
            },
        );
        assert_eval_number(vm, "_origins.length", 2.0);
        assert_eval_string(vm, "_origins[0]", "https://origin-a.example");
        assert_eval_string(vm, "_origins[1]", "https://origin-b.example");
    });
}

#[test]
fn last_event_id_sticky_across_messages() {
    // §9.2.6 step 11: broker's `take_event` emits the cumulative
    // sticky value per event.  The VM-side state mirrors the
    // broker so a `addEventListener` listener that fires AFTER an
    // event without `id:` still sees the previous sticky value
    // delivered as `e.lastEventId`.
    with_es_vm(|vm| {
        vm.eval(
            "globalThis._ids = []; \
             globalThis.s = new EventSource('/events'); \
             s.onmessage = function(e) { globalThis._ids.push(e.lastEventId); };",
        )
        .unwrap();
        inject_sse_event_and_tick(
            vm,
            0,
            SseEvent::Event {
                event_type: "message".to_string(),
                data: "a".to_string(),
                last_event_id: "evt-1".to_string(),
            },
        );
        // Second event WITHOUT `id:` line — broker emits the
        // cumulative sticky value "evt-1" again, so the VM still
        // sees "evt-1" on the second dispatch.
        inject_sse_event_and_tick(
            vm,
            0,
            SseEvent::Event {
                event_type: "message".to_string(),
                data: "b".to_string(),
                last_event_id: "evt-1".to_string(),
            },
        );
        assert_eval_number(vm, "_ids.length", 2.0);
        assert_eval_string(vm, "_ids[0]", "evt-1");
        assert_eval_string(vm, "_ids[1]", "evt-1");
    });
}

// ---------------------------------------------------------------------------
// addEventListener / removeEventListener
// ---------------------------------------------------------------------------

#[test]
fn add_event_listener_fires_named_event() {
    with_es_vm(|vm| {
        vm.eval(
            "globalThis._evts = []; \
             globalThis.s = new EventSource('/events'); \
             s.addEventListener('notification', function(e) { \
               globalThis._evts.push({t: e.type, d: e.data, tg: e.target === s}); \
             });",
        )
        .unwrap();
        inject_sse_event_and_tick(
            vm,
            0,
            SseEvent::Event {
                event_type: "notification".to_string(),
                data: "hi".to_string(),
                last_event_id: String::new(),
            },
        );
        assert_eval_number(vm, "_evts.length", 1.0);
        assert_eval_string(vm, "_evts[0].t", "notification");
        assert_eval_string(vm, "_evts[0].d", "hi");
        assert_eval_bool(vm, "_evts[0].tg", true);
    });
}

#[test]
fn add_event_listener_for_message_fires_alongside_onmessage() {
    // §9.2 "Dispatch the event": message events fire BOTH onmessage AND
    // addEventListener("message", ...) listeners (the on* handler
    // is the implicit `addEventListener("message", ...)` per
    // EventHandler IDL §8.1.8.1).
    with_es_vm(|vm| {
        vm.eval(
            "globalThis._who = []; \
             globalThis.s = new EventSource('/events'); \
             s.onmessage = function() { globalThis._who.push('onmessage'); }; \
             s.addEventListener('message', function() { globalThis._who.push('listener'); });",
        )
        .unwrap();
        inject_sse_event_and_tick(
            vm,
            0,
            SseEvent::Event {
                event_type: "message".to_string(),
                data: "x".to_string(),
                last_event_id: String::new(),
            },
        );
        assert_eval_number(vm, "_who.length", 2.0);
        assert_eval_string(vm, "_who[0]", "onmessage");
        assert_eval_string(vm, "_who[1]", "listener");
    });
}

#[test]
fn add_event_listener_multiple_listeners_fire_in_insertion_order() {
    with_es_vm(|vm| {
        vm.eval(
            "globalThis._order = []; \
             globalThis.s = new EventSource('/events'); \
             s.addEventListener('ping', function() { globalThis._order.push(1); }); \
             s.addEventListener('ping', function() { globalThis._order.push(2); }); \
             s.addEventListener('ping', function() { globalThis._order.push(3); });",
        )
        .unwrap();
        inject_sse_event_and_tick(
            vm,
            0,
            SseEvent::Event {
                event_type: "ping".to_string(),
                data: "x".to_string(),
                last_event_id: String::new(),
            },
        );
        assert_eval_number(vm, "_order.length", 3.0);
        assert_eval_number(vm, "_order[0]", 1.0);
        assert_eval_number(vm, "_order[1]", 2.0);
        assert_eval_number(vm, "_order[2]", 3.0);
    });
}

#[test]
fn add_event_listener_dedups_same_callback_per_type() {
    // WHATWG DOM §2.7.2 step 5: same `(type, callback, capture)`
    // triple is deduped on registration.  The minimal shim
    // collapses capture to false, so `(type, callback)` is
    // sufficient.
    with_es_vm(|vm| {
        vm.eval(
            "globalThis._count = 0; \
             globalThis.s = new EventSource('/events'); \
             globalThis.cb = function() { globalThis._count++; }; \
             s.addEventListener('ping', cb); \
             s.addEventListener('ping', cb); \
             s.addEventListener('ping', cb);",
        )
        .unwrap();
        inject_sse_event_and_tick(
            vm,
            0,
            SseEvent::Event {
                event_type: "ping".to_string(),
                data: "x".to_string(),
                last_event_id: String::new(),
            },
        );
        assert_eval_number(vm, "_count", 1.0);
    });
}

#[test]
fn remove_event_listener_removes_specific_listener() {
    with_es_vm(|vm| {
        vm.eval(
            "globalThis._who = []; \
             globalThis.s = new EventSource('/events'); \
             globalThis.a = function() { globalThis._who.push('a'); }; \
             globalThis.b = function() { globalThis._who.push('b'); }; \
             s.addEventListener('ping', a); \
             s.addEventListener('ping', b); \
             s.removeEventListener('ping', a);",
        )
        .unwrap();
        inject_sse_event_and_tick(
            vm,
            0,
            SseEvent::Event {
                event_type: "ping".to_string(),
                data: "x".to_string(),
                last_event_id: String::new(),
            },
        );
        assert_eval_number(vm, "_who.length", 1.0);
        assert_eval_string(vm, "_who[0]", "b");
    });
}

#[test]
fn remove_event_listener_no_op_for_unknown_pair() {
    with_es_vm(|vm| {
        vm.eval(
            "globalThis.s = new EventSource('/events'); \
             s.removeEventListener('never', function() {});",
        )
        .expect("no-op must not throw");
    });
}

#[test]
fn remove_event_listener_no_op_for_null_listener() {
    // DOM §2.7.5 step 2: null / undefined listener is a silent no-op.
    // (A non-null non-callable listener throws TypeError in the shared
    // core, symmetric with addEventListener — the bespoke shim was lax.)
    with_es_vm(|vm| {
        vm.eval(
            "globalThis.s = new EventSource('/events'); \
             s.removeEventListener('ping', null); \
             s.removeEventListener('ping', undefined);",
        )
        .expect("null/undefined removes are silent no-op");
    });
}

#[test]
fn add_event_listener_null_undefined_listener_is_no_op() {
    // WHATWG DOM §2.7.2 step 2: a null/undefined callback returns
    // silently and registers nothing.  (A non-null non-callable
    // listener throws TypeError in the shared core — see
    // `add_event_listener_non_callable_throws_type_error`; the bespoke
    // SSE shim silently dropped those, which was not spec-faithful.)
    with_es_vm(|vm| {
        vm.eval(
            "globalThis._count = 0; \
             globalThis.s = new EventSource('/events'); \
             s.addEventListener('ping', null); \
             s.addEventListener('ping', undefined);",
        )
        .expect("null/undefined listener is silent no-op");
        inject_sse_event_and_tick(
            vm,
            0,
            SseEvent::Event {
                event_type: "ping".to_string(),
                data: "x".to_string(),
                last_event_id: String::new(),
            },
        );
        assert_eval_number(vm, "_count", 0.0);
    });
}

#[test]
fn add_event_listener_non_callable_throws_type_error() {
    // Shared EventTarget core (WebIDL `EventListener?`): a non-null,
    // non-callable listener is a conversion failure → TypeError,
    // matching browsers.  EventSource now inherits this from
    // EventTarget.prototype (the bespoke shim was lax).
    with_es_vm(|vm| {
        vm.eval("globalThis.s = new EventSource('/events');")
            .unwrap();
        let err = vm
            .eval("s.addEventListener('ping', 'not-a-fn');")
            .unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.contains("Type"), "expected TypeError, got: {msg}");
    });
}

#[test]
fn add_event_listener_during_dispatch_does_not_fire_in_same_tick() {
    // WHATWG DOM §2.7 snapshot-then-iterate: a listener added
    // mid-dispatch (from inside another listener's body) must NOT
    // receive the in-flight event.  The minimal shim implements
    // this by cloning the Vec<ObjectId> snapshot inside the
    // borrow scope and dropping the borrow before fan-out, so
    // the user-handler's `addEventListener` mutates the live
    // registry without affecting the in-flight iteration.
    with_es_vm(|vm| {
        vm.eval(
            "globalThis._first_count = 0; \
             globalThis._second_count = 0; \
             globalThis.s = new EventSource('/events'); \
             globalThis.second = function() { globalThis._second_count++; }; \
             s.addEventListener('ping', function() { \
               globalThis._first_count++; \
               s.addEventListener('ping', globalThis.second); \
             });",
        )
        .unwrap();
        // First dispatch: only the original listener fires.
        inject_sse_event_and_tick(
            vm,
            0,
            SseEvent::Event {
                event_type: "ping".to_string(),
                data: "x".to_string(),
                last_event_id: String::new(),
            },
        );
        assert_eval_number(vm, "_first_count", 1.0);
        assert_eval_number(vm, "_second_count", 0.0);
        // Second dispatch: BOTH fire (the second listener was
        // registered during the first dispatch and is now part
        // of the registry for subsequent ticks).
        inject_sse_event_and_tick(
            vm,
            0,
            SseEvent::Event {
                event_type: "ping".to_string(),
                data: "y".to_string(),
                last_event_id: String::new(),
            },
        );
        assert_eval_number(vm, "_first_count", 2.0);
        assert_eval_number(vm, "_second_count", 1.0);
    });
}

#[test]
fn remove_event_listener_during_dispatch_does_not_fire_in_same_tick() {
    // WHATWG DOM §2.9 inner-invoke: the listener list is cloned before
    // the walk, but a listener whose `removed` flag is set mid-dispatch
    // is skipped ("If listener's removed is true, then continue").  So
    // `b`, removed by the first listener before its turn, does NOT fire
    // this tick — the shared §2.9 core implements this via the step 6
    // removed-field check (`resolve_callable` returns None).  The
    // bespoke SSE shim fired the pre-snapshot copy, which was not
    // spec-faithful.
    with_es_vm(|vm| {
        vm.eval(
            "globalThis._a = 0; \
             globalThis._b = 0; \
             globalThis.s = new EventSource('/events'); \
             globalThis.b = function() { globalThis._b++; }; \
             s.addEventListener('ping', function() { \
               globalThis._a++; \
               s.removeEventListener('ping', globalThis.b); \
             }); \
             s.addEventListener('ping', globalThis.b);",
        )
        .unwrap();
        inject_sse_event_and_tick(
            vm,
            0,
            SseEvent::Event {
                event_type: "ping".to_string(),
                data: "x".to_string(),
                last_event_id: String::new(),
            },
        );
        // First dispatch: the remover fires and removes `b` before
        // `b`'s turn → `b` is skipped this tick (§2.9 removed check).
        assert_eval_number(vm, "_a", 1.0);
        assert_eval_number(vm, "_b", 0.0);
        // Second dispatch — only the remover fires; b stays removed.
        inject_sse_event_and_tick(
            vm,
            0,
            SseEvent::Event {
                event_type: "ping".to_string(),
                data: "y".to_string(),
                last_event_id: String::new(),
            },
        );
        assert_eval_number(vm, "_a", 2.0);
        assert_eval_number(vm, "_b", 0.0);
    });
}

#[test]
fn add_event_listener_type_arg_to_string_first_runs_before_registry_mutation() {
    // WebIDL `USVString` coercion order: ToString runs FIRST, so
    // a Symbol arg throws TypeError BEFORE the registry is
    // touched.  Verify the registry remains empty afterwards.
    with_es_vm(|vm| {
        vm.eval(
            "globalThis.s = new EventSource('/events'); \
             globalThis._count = 0; \
             globalThis.cb = function() { globalThis._count++; }; \
             try { s.addEventListener(Symbol('foo'), cb); } catch (e) {}",
        )
        .unwrap();
        // Fire a "Symbol(foo)" stringification result — if the
        // registry had been mutated it would be under the
        // stringified key.  Inject under whatever key the user
        // tried to use; verify no fire.  Easiest check: fire a
        // common "ping" name; if it doesn't fire, registry is
        // intact.
        inject_sse_event_and_tick(
            vm,
            0,
            SseEvent::Event {
                event_type: "Symbol(foo)".to_string(),
                data: "x".to_string(),
                last_event_id: String::new(),
            },
        );
        assert_eval_number(vm, "_count", 0.0);
    });
}

// ---------------------------------------------------------------------------
// Brand check on prototype methods
// ---------------------------------------------------------------------------

#[test]
fn close_on_non_event_source_throws_type_error() {
    with_es_vm(|vm| {
        let err = vm
            .eval("EventSource.prototype.close.call({});")
            .unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("non-EventSource") || msg.contains("Type"),
            "expected brand-check TypeError, got: {msg}"
        );
    });
}

#[test]
fn add_event_listener_on_non_event_source_is_silent_no_op() {
    // The shared `EventTarget.prototype.addEventListener` is
    // detach-tolerant: a non-branded receiver (`call({})`) resolves to
    // no DispatchTarget and returns undefined silently — the same
    // policy AbortSignal / IndexedDB / Node use.  The migration makes
    // EventSource consistent with every other elidex EventTarget (the
    // bespoke SSE shim threw "Illegal invocation").
    with_es_vm(|vm| {
        vm.eval("EventSource.prototype.addEventListener.call({}, 'x', function() {});")
            .expect("non-branded receiver is a silent no-op, not a throw");
    });
}

// ---------------------------------------------------------------------------
// Shared §2.9 EventTarget integration (#11-realtime-event-listeners)
//
// EventSource's bespoke `addEventListener` + per-instance
// `event_listeners` registry were replaced by inherited
// `EventTarget.prototype` methods routing through the shared VmObject
// dispatch core.  Named events + the §9.2.6 "message vs named" fan-out
// are now emergent; capture / once / passive / {signal} work for free.
// ---------------------------------------------------------------------------

#[test]
fn named_event_delivers_to_add_event_listener_only_not_onmessage() {
    // WHATWG HTML §9.2.6: a server `event: notify` line fires a
    // MessageEvent of type "notify" — delivered to
    // addEventListener("notify") but NOT to onmessage (which is the
    // "message"-type EventHandler listener).  Emergent from dispatching
    // the parsed type at the shared core.
    with_es_vm(|vm| {
        vm.eval(
            "globalThis._named = []; globalThis._msg = 0; \
             globalThis.s = new EventSource('/events'); \
             s.onmessage = function() { globalThis._msg++; }; \
             s.addEventListener('notify', function(e) { \
               globalThis._named.push({ mev: e instanceof MessageEvent, d: e.data }); \
             });",
        )
        .unwrap();
        inject_sse_event_and_tick(
            vm,
            0,
            SseEvent::Event {
                event_type: "notify".to_string(),
                data: "hello".to_string(),
                last_event_id: String::new(),
            },
        );
        assert_eval_number(vm, "_named.length", 1.0);
        assert_eval_bool(vm, "_named[0].mev", true);
        assert_eval_string(vm, "_named[0].d", "hello");
        // onmessage must NOT receive the named event.
        assert_eval_number(vm, "_msg", 0.0);
    });
}

#[test]
fn default_message_fires_onmessage_and_add_event_listener() {
    // A default (no `event:` line) message has type "message" → fires
    // both onmessage and addEventListener("message").
    with_es_vm(|vm| {
        vm.eval(
            "globalThis._log = []; \
             globalThis.s = new EventSource('/events'); \
             s.onmessage = function(e) { globalThis._log.push('on:' + e.data); }; \
             s.addEventListener('message', function(e) { globalThis._log.push('ael:' + e.data); });",
        )
        .unwrap();
        inject_sse_event_and_tick(
            vm,
            0,
            SseEvent::Event {
                event_type: "message".to_string(),
                data: "x".to_string(),
                last_event_id: String::new(),
            },
        );
        assert_eval_number(vm, "_log.length", 2.0);
        assert_eval_bool(vm, "_log.indexOf('on:x') !== -1", true);
        assert_eval_bool(vm, "_log.indexOf('ael:x') !== -1", true);
    });
}

#[test]
fn add_event_listener_once_and_signal_now_supported() {
    // Capabilities the bespoke SSE shim lacked, gained by routing
    // through the shared EventTarget core: `{once}` and `{signal}`.
    with_es_vm(|vm| {
        vm.eval(
            "globalThis._once = 0; globalThis._sig = 0; \
             globalThis.s = new EventSource('/events'); \
             globalThis.ac = new AbortController(); \
             s.addEventListener('message', function() { globalThis._once++; }, { once: true }); \
             s.addEventListener('message', function() { globalThis._sig++; }, { signal: ac.signal }); \
             ac.abort();",
        )
        .unwrap();
        let msg = || SseEvent::Event {
            event_type: "message".to_string(),
            data: "x".to_string(),
            last_event_id: String::new(),
        };
        inject_sse_event_and_tick(vm, 0, msg());
        inject_sse_event_and_tick(vm, 0, msg());
        // once: fired exactly once across two ticks.
        assert_eval_number(vm, "_once", 1.0);
        // signal: aborted before any dispatch → never fired.
        assert_eval_number(vm, "_sig", 0.0);
    });
}

#[test]
fn eventsource_add_event_listener_is_inherited_not_shadowed() {
    // F10 structural single-home guard — the critical one for SSE,
    // which previously installed a bespoke OWN `addEventListener` that
    // wrote a parallel `event_listeners` registry.  After migration
    // EventSource.prototype must NOT own `addEventListener` (it is
    // inherited from EventTarget.prototype); a surviving own method
    // would shadow the shared one and split the listener home.
    with_es_vm(|vm| {
        vm.eval("globalThis.s = new EventSource('/events');")
            .unwrap();
        assert_eval_bool(vm, "typeof s.addEventListener === 'function'", true);
        assert_eval_bool(
            vm,
            "Object.getOwnPropertyNames(Object.getPrototypeOf(s))\
             .indexOf('addEventListener') === -1",
            true,
        );
    });
}

#[test]
fn ready_state_getter_on_non_event_source_throws() {
    with_es_vm(|vm| {
        let err = vm
            .eval(
                "let d = Object.getOwnPropertyDescriptor(EventSource.prototype, 'readyState'); \
                 d.get.call({});",
            )
            .unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("non-EventSource") || msg.contains("Type"),
            "expected brand-check TypeError, got: {msg}"
        );
    });
}

#[test]
fn unobserved_named_sse_events_do_not_intern_type_or_id() {
    // Regression (`#11-realtime-event-listeners`, Copilot R2 / IMPORTANT): a
    // page listening only to `onmessage` does NOT observe named events
    // (§9.2.6).  Their server-controlled `event:` type, `id:`, and `data:`
    // strings must be interned only past the listener gate — otherwise a named
    // keepalive stream with unique ids grows the permanent `StringPool`
    // unboundedly under server control (entries are never freed).
    with_es_vm(|vm| {
        vm.eval(
            "globalThis.s = new EventSource('/events'); \
             s.onmessage = function() {};",
        )
        .unwrap();
        let before = vm.inner.strings.len();
        for i in 0..6 {
            inject_sse_event_and_tick(
                vm,
                0,
                SseEvent::Event {
                    event_type: format!("keepalive-{i}"),
                    data: format!("payload-{i}"),
                    last_event_id: format!("id-{i}"),
                },
            );
        }
        assert_eq!(
            before,
            vm.inner.strings.len(),
            "unobserved named SSE events interned type/id/data into the permanent StringPool"
        );
    });
}
