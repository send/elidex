//! Service Worker thread harness tests (WHATWG SW §4; slot
//! `#11-service-workers-vm` / D-19 PR-2).
//!
//! Each test spawns the VM `sw_thread::run_service_worker` on a worker
//! thread driven over the *exact* `ContentToSw` / `SwToContent` IPC contract
//! the shell coordinator uses (so the D-26 boa→VM spawn swap is mechanical),
//! and asserts the replies.  The matrix is densest on the DR-C
//! `respondWith` real-promise drain — the central novelty boa cannot do.

#![cfg(feature = "engine")]

use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use elidex_api_sw::{
    ClientSnapshot, ClientType, ContentToSw, FrameType, LifecycleEvent, SwRequest, SwResponse,
    SwToContent, VisibilityState,
};
use elidex_ecs::EcsDom;
use elidex_net::broker::NetworkHandle;
use elidex_net::{HttpVersion, Response as NetResponse};
use elidex_plugin::{channel_pair, LocalChannel};
use elidex_script_session::SessionCore;
use elidex_storage_core::SqliteConnection;

use super::super::host::cache::CacheBackend;
use super::super::host_data::HostData;
use super::super::sw_thread::run_service_worker;
use super::super::Vm;

const SCRIPT_URL: &str = "https://example.com/sw.js";
const SCOPE_URL: &str = "https://example.com/";

/// A spawned SW thread + the content-side channel driving it.
struct Harness {
    content: LocalChannel<ContentToSw, SwToContent>,
    join: Option<JoinHandle<()>>,
}

impl Harness {
    fn spawn(source: &str) -> Self {
        Self::spawn_full(source, Vec::new(), Vec::new(), Duration::from_secs(2))
    }

    fn spawn_full(
        source: &str,
        clients: Vec<ClientSnapshot>,
        responses: Vec<(url::Url, Result<NetResponse, String>)>,
        pump_timeout: Duration,
    ) -> Self {
        Self::spawn_shared(source, clients, responses, pump_timeout, shared_conn())
    }

    fn spawn_shared(
        source: &str,
        clients: Vec<ClientSnapshot>,
        responses: Vec<(url::Url, Result<NetResponse, String>)>,
        pump_timeout: Duration,
        cache_conn: Arc<Mutex<SqliteConnection>>,
    ) -> Self {
        let (sw_ch, content_ch) = channel_pair::<SwToContent, ContentToSw>();
        let source = source.to_string();
        let join = std::thread::spawn(move || {
            let script_url = url::Url::parse(SCRIPT_URL).unwrap();
            let scope = url::Url::parse(SCOPE_URL).unwrap();
            let nh = NetworkHandle::mock_with_responses(responses);
            run_service_worker(
                &source,
                &script_url,
                &scope,
                &sw_ch,
                nh,
                cache_conn,
                clients,
                pump_timeout,
                elidex_plugin::EngineMode::BrowserCompat,
            );
        });
        Harness {
            content: content_ch,
            join: Some(join),
        }
    }

    fn send(&self, msg: ContentToSw) {
        self.content.send(msg).expect("SW thread alive");
    }

    /// Read the next reply (5 s ceiling — generous vs the 150 ms test pump).
    fn recv(&self) -> SwToContent {
        self.content
            .recv_timeout(Duration::from_secs(5))
            .expect("SW reply within 5s")
    }

    /// Read replies until one satisfies `pred`, returning it (skips unrelated
    /// outbound messages like `SkipWaiting` that may precede the result).
    fn recv_find(&self, pred: impl Fn(&SwToContent) -> bool) -> SwToContent {
        for _ in 0..8 {
            let msg = self.recv();
            if pred(&msg) {
                return msg;
            }
        }
        panic!("no matching SwToContent within 8 messages");
    }

    fn shutdown(mut self) {
        let _ = self.content.send(ContentToSw::Shutdown);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        if let Some(join) = self.join.take() {
            let _ = self.content.send(ContentToSw::Shutdown);
            let _ = join.join();
        }
    }
}

/// A fresh shared in-memory cache connection (the DR-A `Arc<Mutex<…>>`).
fn shared_conn() -> Arc<Mutex<SqliteConnection>> {
    Arc::new(Mutex::new(
        SqliteConnection::open_in_memory().expect("in-memory sqlite"),
    ))
}

/// A `GET` `FetchEvent` for `url` from client `client_id`.
fn fetch_event(url: &str, client_id: &str) -> ContentToSw {
    ContentToSw::FetchEvent {
        fetch_id: 7,
        request: Box::new(SwRequest {
            url: url::Url::parse(url).unwrap(),
            method: "GET".to_string(),
            headers: vec![("x-test".to_string(), "1".to_string())],
            body: Vec::new(),
            mode: "same-origin".to_string(),
            destination: "document".to_string(),
            integrity: None,
            redirect: "follow".to_string(),
            referrer: "about:client".to_string(),
            referrer_policy: String::new(),
            cache_mode: "default".to_string(),
            keepalive: false,
        }),
        client_id: client_id.to_string(),
        resulting_client_id: String::new(),
    }
}

fn client(id: &str, url: &str, ty: ClientType) -> ClientSnapshot {
    ClientSnapshot {
        id: id.to_string(),
        url: url.to_string(),
        client_type: ty,
        frame_type: FrameType::TopLevel,
        visibility: VisibilityState::Visible,
        focused: true,
    }
}

fn net_ok(url: &str, body: &'static str) -> (url::Url, Result<NetResponse, String>) {
    let parsed = url::Url::parse(url).unwrap();
    (
        parsed.clone(),
        Ok(NetResponse {
            status: 200,
            headers: vec![("content-type".to_string(), "text/plain".to_string())],
            body: bytes::Bytes::from_static(body.as_bytes()),
            url: parsed.clone(),
            version: HttpVersion::H1,
            url_list: vec![parsed],
            is_redirect_tainted: false,
            credentialed_network: false,
        }),
    )
}

fn body_string(resp: &SwResponse) -> String {
    String::from_utf8(resp.body.clone()).unwrap()
}

// ---------------------------------------------------------------------------
// Lifecycle (install / activate + waitUntil)
// ---------------------------------------------------------------------------

#[test]
fn install_no_handler_succeeds() {
    let h = Harness::spawn("// no listeners");
    h.send(ContentToSw::Install);
    assert!(matches!(
        h.recv(),
        SwToContent::LifecycleComplete {
            event: LifecycleEvent::Install,
            success: true
        }
    ));
    h.shutdown();
}

#[test]
fn install_wait_until_resolve_succeeds() {
    let h = Harness::spawn("self.oninstall = e => e.waitUntil(Promise.resolve(42));");
    h.send(ContentToSw::Install);
    assert!(matches!(
        h.recv(),
        SwToContent::LifecycleComplete { success: true, .. }
    ));
    h.shutdown();
}

#[test]
fn install_wait_until_reject_fails() {
    // The real boa-gap fix: a rejected `waitUntil` promise fails install
    // (boa stubs `waitUntil`, so this would have spuriously succeeded).
    let h = Harness::spawn("self.oninstall = e => e.waitUntil(Promise.reject(new Error('x')));");
    h.send(ContentToSw::Install);
    assert!(matches!(
        h.recv(),
        SwToContent::LifecycleComplete {
            event: LifecycleEvent::Install,
            success: false
        }
    ));
    h.shutdown();
}

#[test]
fn install_wait_until_timer_resolve_within_bound_succeeds() {
    // `waitUntil` settled via a real timer (drained by the pump), not a
    // microtask — proves the pump advances timers.
    let h = Harness::spawn(
        "self.oninstall = e => e.waitUntil(new Promise(r => setTimeout(() => r(1), 5)));",
    );
    h.send(ContentToSw::Install);
    assert!(matches!(
        h.recv(),
        SwToContent::LifecycleComplete { success: true, .. }
    ));
    h.shutdown();
}

#[test]
fn install_wait_until_never_settles_times_out() {
    let h = Harness::spawn_full(
        "self.oninstall = e => e.waitUntil(new Promise(() => {}));",
        Vec::new(),
        Vec::new(),
        Duration::from_millis(150),
    );
    h.send(ContentToSw::Install);
    assert!(matches!(
        h.recv(),
        SwToContent::LifecycleComplete {
            event: LifecycleEvent::Install,
            success: false
        }
    ));
    h.shutdown();
}

#[test]
fn install_throwing_handler_without_wait_until_succeeds() {
    // Spec-faithful (parity-or-better vs boa): a synchronous handler throw
    // without `waitUntil` is merely reported — install still succeeds
    // (WHATWG SW Install: only an extend-lifetime-promise rejection fails it;
    // Chrome parity).  Boa's `dispatch_sw_event` would (non-spec) fail here.
    let h = Harness::spawn("self.oninstall = () => { throw new Error('boom'); };");
    h.send(ContentToSw::Install);
    assert!(matches!(
        h.recv(),
        SwToContent::LifecycleComplete { success: true, .. }
    ));
    h.shutdown();
}

#[test]
fn activate_wait_until_reject_fails() {
    let h = Harness::spawn("self.onactivate = e => e.waitUntil(Promise.reject(1));");
    h.send(ContentToSw::Activate);
    assert!(matches!(
        h.recv(),
        SwToContent::LifecycleComplete {
            event: LifecycleEvent::Activate,
            success: false
        }
    ));
    h.shutdown();
}

#[test]
fn wait_until_after_dispatch_throws_invalid_state() {
    // A late `waitUntil` that runs *during* the lifetime pump (dispatch flag
    // cleared, brand still alive) throws InvalidStateError rather than being
    // silently dropped — the loop snapshotted `lifetime_promises` right after
    // dispatch (SW §4.4.1 "active": the dispatch-flag half; the pending-promises
    // half is deferred `#11-sw-fetchevent-waituntil`).  A *timer* callback is
    // the genuine post-dispatch window: a microtask (`.then`) would instead run
    // inside `dispatch_script_event`'s trailing checkpoint (flag still set) and
    // be correctly accepted.  The `catch` keeps the outer `waitUntil` promise
    // fulfilled (install succeeds); the captured name is surfaced via a fetch.
    let h = Harness::spawn(
        "self.lateName = 'unset';
         self.oninstall = e => {
             e.waitUntil(new Promise(resolve => {
                 setTimeout(() => {
                     try { e.waitUntil(Promise.resolve()); self.lateName = 'no-throw'; }
                     catch (err) { self.lateName = err.name; }
                     resolve(1);
                 }, 0);
             }));
         };
         self.onfetch = e => e.respondWith(new Response(self.lateName));",
    );
    h.send(ContentToSw::Install);
    assert!(matches!(
        h.recv(),
        SwToContent::LifecycleComplete {
            event: LifecycleEvent::Install,
            success: true
        }
    ));
    h.send(fetch_event("https://example.com/x", "c1"));
    match h.recv() {
        SwToContent::FetchResponse { response, .. } => {
            assert_eq!(body_string(&response), "InvalidStateError");
        }
        other => panic!("expected FetchResponse, got {other:?}"),
    }
    h.shutdown();
}

// ---------------------------------------------------------------------------
// respondWith (DR-C) — the densest surface
// ---------------------------------------------------------------------------

#[test]
fn respond_with_sync_response() {
    let h = Harness::spawn("self.onfetch = e => e.respondWith(new Response('sync'));");
    h.send(fetch_event("https://example.com/x", "c1"));
    match h.recv() {
        SwToContent::FetchResponse { response, .. } => assert_eq!(body_string(&response), "sync"),
        other => panic!("expected FetchResponse, got {other:?}"),
    }
    h.shutdown();
}

#[test]
fn respond_with_resolved_promise() {
    // The microtask-settle path.
    let h = Harness::spawn(
        "self.onfetch = e => e.respondWith(Promise.resolve(new Response('promised')));",
    );
    h.send(fetch_event("https://example.com/x", "c1"));
    match h.recv() {
        SwToContent::FetchResponse { response, .. } => {
            assert_eq!(body_string(&response), "promised");
        }
        other => panic!("expected FetchResponse, got {other:?}"),
    }
    h.shutdown();
}

#[test]
fn respond_with_fetch_network_path() {
    // The central proof: `respondWith(fetch(req))` resolves via the network
    // tick inside the pump (boa reads the response synchronously → a fetched
    // Response never resolves).
    let h = Harness::spawn_full(
        "self.onfetch = e => e.respondWith(fetch('https://example.com/api'));",
        Vec::new(),
        vec![net_ok("https://example.com/api", "from-network")],
        Duration::from_secs(2),
    );
    h.send(fetch_event("https://example.com/page", "c1"));
    match h.recv() {
        SwToContent::FetchResponse { response, .. } => {
            assert_eq!(body_string(&response), "from-network");
        }
        other => panic!("expected FetchResponse, got {other:?}"),
    }
    h.shutdown();
}

#[test]
fn no_respond_with_passes_through() {
    let h = Harness::spawn("self.onfetch = () => { /* observe only */ };");
    h.send(fetch_event("https://example.com/x", "c1"));
    assert!(matches!(
        h.recv(),
        SwToContent::FetchPassthrough { fetch_id: 7 }
    ));
    h.shutdown();
}

#[test]
fn double_respond_with_throws_invalid_state() {
    // First `respondWith` wins; the second throws InvalidStateError (caught
    // and surfaced through the first response's body).
    let h = Harness::spawn(
        "self.onfetch = e => {
            let resolve;
            e.respondWith(new Promise(r => { resolve = r; }));
            let name = 'no-throw';
            try { e.respondWith(new Response('second')); }
            catch (err) { name = err.name; }
            resolve(new Response(name));
        };",
    );
    h.send(fetch_event("https://example.com/x", "c1"));
    match h.recv() {
        SwToContent::FetchResponse { response, .. } => {
            assert_eq!(body_string(&response), "InvalidStateError");
        }
        other => panic!("expected FetchResponse, got {other:?}"),
    }
    h.shutdown();
}

#[test]
fn respond_with_rejected_promise_passes_through() {
    let h = Harness::spawn("self.onfetch = e => e.respondWith(Promise.reject(new Error('no')));");
    h.send(fetch_event("https://example.com/x", "c1"));
    assert!(matches!(h.recv(), SwToContent::FetchPassthrough { .. }));
    h.shutdown();
}

#[test]
fn respond_with_never_settles_times_out_to_passthrough() {
    let h = Harness::spawn_full(
        "self.onfetch = e => e.respondWith(new Promise(() => {}));",
        Vec::new(),
        Vec::new(),
        Duration::from_millis(150),
    );
    h.send(fetch_event("https://example.com/x", "c1"));
    assert!(matches!(h.recv(), SwToContent::FetchPassthrough { .. }));
    h.shutdown();
}

#[test]
fn handler_throws_before_respond_with_passes_through() {
    let h = Harness::spawn("self.onfetch = () => { throw new Error('pre'); };");
    h.send(fetch_event("https://example.com/x", "c1"));
    assert!(matches!(h.recv(), SwToContent::FetchPassthrough { .. }));
    h.shutdown();
}

#[test]
fn non_response_fulfilled_value_passes_through() {
    let h = Harness::spawn("self.onfetch = e => e.respondWith(Promise.resolve('not a response'));");
    h.send(fetch_event("https://example.com/x", "c1"));
    assert!(matches!(h.recv(), SwToContent::FetchPassthrough { .. }));
    h.shutdown();
}

#[test]
fn network_error_response_passes_through() {
    // SW §4.6.7: respondWith(Response.error()) is a network error → passthrough,
    // not a bogus status-0 response delivered to the page.
    let h = Harness::spawn("self.onfetch = e => e.respondWith(Response.error());");
    h.send(fetch_event("https://example.com/x", "c1"));
    assert!(matches!(h.recv(), SwToContent::FetchPassthrough { .. }));
    h.shutdown();
}

#[test]
fn respond_with_survives_gc_in_listener() {
    // GC-safety regression: the respondWith promise lives ONLY in the
    // fetch_event_states side-store between the native and the SW loop reading
    // it. The listener allocates heavily AFTER respondWith (forcing a GC while
    // the side-store is the sole owner), so the promise — and its Response —
    // must be marked as a root by the GC mark phase, else it is swept and the
    // fetch wrongly passes through (or the loop panics on a recycled id).
    let h = Harness::spawn(
        "self.onfetch = e => {
            e.respondWith(new Response('survived'));
            let sink = [];
            for (let i = 0; i < 200000; i++) { sink.push({ n: i }); }
            globalThis.__sink = sink.length;
        };",
    );
    h.send(fetch_event("https://example.com/x", "c1"));
    match h.recv() {
        SwToContent::FetchResponse { response, .. } => {
            assert_eq!(body_string(&response), "survived");
        }
        other => panic!("expected FetchResponse (promise swept by GC?), got {other:?}"),
    }
    h.shutdown();
}

// ---------------------------------------------------------------------------
// FetchEvent.request marshalling
// ---------------------------------------------------------------------------

#[test]
fn fetch_event_request_marshals_url_method_headers() {
    let h = Harness::spawn(
        "self.onfetch = e => e.respondWith(new Response(
            e.request.method + ' ' + e.request.url + ' ' + e.request.headers.get('x-test') +
            ' ' + e.clientId));",
    );
    h.send(fetch_event("https://example.com/data?q=1", "client-9"));
    match h.recv() {
        SwToContent::FetchResponse { response, .. } => {
            assert_eq!(
                body_string(&response),
                "GET https://example.com/data?q=1 1 client-9"
            );
        }
        other => panic!("expected FetchResponse, got {other:?}"),
    }
    h.shutdown();
}

// ---------------------------------------------------------------------------
// Clients + skipWaiting
// ---------------------------------------------------------------------------

#[test]
fn clients_match_all_returns_seeded_snapshot() {
    let h = Harness::spawn_full(
        "self.onfetch = e => e.respondWith(
            clients.matchAll().then(cs => new Response(cs.map(c => c.id).join(','))));",
        vec![
            client("a", "https://example.com/a", ClientType::Window),
            client("b", "https://example.com/b", ClientType::Window),
        ],
        Vec::new(),
        Duration::from_secs(2),
    );
    h.send(fetch_event("https://example.com/x", "a"));
    match h.recv() {
        SwToContent::FetchResponse { response, .. } => assert_eq!(body_string(&response), "a,b"),
        other => panic!("expected FetchResponse, got {other:?}"),
    }
    h.shutdown();
}

#[test]
fn client_list_message_replaces_snapshot() {
    let h = Harness::spawn_full(
        "self.onfetch = e => e.respondWith(
            clients.matchAll().then(cs => new Response(cs.map(c => c.id).join(','))));",
        vec![client("old", "https://example.com/o", ClientType::Window)],
        Vec::new(),
        Duration::from_secs(2),
    );
    h.send(ContentToSw::ClientList {
        clients: vec![client("new", "https://example.com/n", ClientType::Window)],
    });
    h.send(fetch_event("https://example.com/x", "new"));
    match h.recv() {
        SwToContent::FetchResponse { response, .. } => assert_eq!(body_string(&response), "new"),
        other => panic!("expected FetchResponse, got {other:?}"),
    }
    h.shutdown();
}

#[test]
fn clients_get_hit_and_miss() {
    let h = Harness::spawn_full(
        "self.onfetch = e => e.respondWith(
            clients.get('hit').then(a =>
                clients.get('absent').then(b =>
                    new Response((a ? a.id : 'none') + '/' + (b ? b.id : 'none')))));",
        vec![client("hit", "https://example.com/h", ClientType::Window)],
        Vec::new(),
        Duration::from_secs(2),
    );
    h.send(fetch_event("https://example.com/x", "hit"));
    match h.recv() {
        SwToContent::FetchResponse { response, .. } => {
            assert_eq!(body_string(&response), "hit/none");
        }
        other => panic!("expected FetchResponse, got {other:?}"),
    }
    h.shutdown();
}

#[test]
fn clients_get_returns_same_object_for_same_id() {
    // NG-1: `clients.get(id)` returns the SAME object on every access — PR-2
    // minted a fresh `Client` each call; PR-3 routes `build_client_object`
    // through `intern_wrapper` (`WrapperKind::Client`, keyed by client id) so
    // the §4.2 `[SameObject]` invariant holds.
    let h = Harness::spawn_full(
        "self.onfetch = e => e.respondWith(
            Promise.all([clients.get('hit'), clients.get('hit')]).then(
                arr => new Response(String(arr[0] === arr[1]))));",
        vec![client("hit", "https://example.com/h", ClientType::Window)],
        Vec::new(),
        Duration::from_secs(2),
    );
    h.send(fetch_event("https://example.com/x", "hit"));
    match h.recv() {
        SwToContent::FetchResponse { response, .. } => assert_eq!(body_string(&response), "true"),
        other => panic!("expected FetchResponse, got {other:?}"),
    }
    h.shutdown();
}

#[test]
fn clients_claim_emits_claim_message() {
    let h = Harness::spawn("self.onactivate = e => e.waitUntil(clients.claim());");
    h.send(ContentToSw::Activate);
    // `ClientsClaim` (queued during the handler) + `LifecycleComplete` arrive;
    // assert the claim wire signal is present.
    assert!(matches!(
        h.recv_find(|m| matches!(m, SwToContent::ClientsClaim)),
        SwToContent::ClientsClaim
    ));
    h.shutdown();
}

#[test]
fn skip_waiting_emits_skip_message() {
    let h = Harness::spawn("self.oninstall = () => { self.skipWaiting(); };");
    h.send(ContentToSw::Install);
    assert!(matches!(
        h.recv_find(|m| matches!(m, SwToContent::SkipWaiting)),
        SwToContent::SkipWaiting
    ));
    h.shutdown();
}

#[test]
fn client_post_message_routes_to_client_id() {
    // F7: the routed `client_id` is the client's own id (NOT boa's empty
    // string).
    let h = Harness::spawn_full(
        "self.onfetch = e => e.respondWith(
            clients.get('target').then(c => { c.postMessage('ping'); return new Response('ok'); }));",
        vec![client("target", "https://example.com/t", ClientType::Window)],
        Vec::new(),
        Duration::from_secs(2),
    );
    h.send(fetch_event("https://example.com/x", "target"));
    match h.recv_find(|m| matches!(m, SwToContent::PostMessage { .. })) {
        SwToContent::PostMessage { client_id, data } => {
            assert_eq!(client_id, "target");
            assert_eq!(data, "\"ping\"");
        }
        other => panic!("expected PostMessage, got {other:?}"),
    }
    h.shutdown();
}

// ---------------------------------------------------------------------------
// postMessage delivery (ContentToSw::PostMessage → message event)
// ---------------------------------------------------------------------------

#[test]
fn inbound_post_message_fires_message_event() {
    // Regression pin (S5-4e contrast): the SW channel KEEPS carrying the
    // sender's origin — `ExtendableMessageEvent.origin` is spec-required
    // (Service Workers §3.1.5 `postMessage(message, options)`: "Let origin be
    // incumbentSettings's origin"), the opposite polarity of the
    // dedicated-worker port channel (HTML §9.4.4 step 7.7: origin stays "").
    let h = Harness::spawn(
        "self.__last = '';
         self.onmessage = e => { self.__last = e.origin + '|' + e.data; };
         self.onfetch = e => e.respondWith(new Response(String(self.__last)));",
    );
    h.send(ContentToSw::PostMessage {
        data: "\"hello-sw\"".to_string(),
        origin: "https://example.com".to_string(),
        client_id: "c1".to_string(),
    });
    h.send(fetch_event("https://example.com/x", "c1"));
    match h.recv() {
        SwToContent::FetchResponse { response, .. } => {
            assert_eq!(body_string(&response), "https://example.com|hello-sw");
        }
        other => panic!("expected FetchResponse, got {other:?}"),
    }
    h.shutdown();
}

// ---------------------------------------------------------------------------
// Cache API in the SW realm — CROSS-realm visibility (F2)
// ---------------------------------------------------------------------------

#[test]
fn cache_shared_window_put_then_sw_match() {
    // Install the SAME backend connection into a window VM and the SW thread;
    // a window-realm `put` must be visible to an SW-realm `match` (asserts
    // cross-realm sharing — a same-realm round-trip would pass even with a
    // private in-memory fallback, masking a DR-A regression).
    let conn = shared_conn();
    window_put(
        conn.clone(),
        "globalThis.__out = caches.open('v1').then(c =>
            c.put(new Request('https://example.com/shared'), new Response('cross-realm'))
        ).then(() => 'done');",
    );

    let h = Harness::spawn_shared(
        "self.onfetch = e => e.respondWith(
            caches.open('v1')
                .then(c => c.match('https://example.com/shared'))
                .then(r => r ? r.text() : Promise.resolve('MISS'))
                .then(t => new Response(t)));",
        Vec::new(),
        Vec::new(),
        Duration::from_secs(2),
        conn,
    );
    h.send(fetch_event("https://example.com/x", "c1"));
    match h.recv() {
        SwToContent::FetchResponse { response, .. } => {
            assert_eq!(body_string(&response), "cross-realm");
        }
        other => panic!("expected FetchResponse, got {other:?}"),
    }
    h.shutdown();
}

/// Run a window-realm cache script against the shared `conn` (the `put`
/// half of the cross-realm test).
fn window_put(conn: Arc<Mutex<SqliteConnection>>, script: &str) {
    let mut vm = Vm::new();
    vm.install_host_data(HostData::new());
    if let Some(hd) = vm.inner.host_data.as_deref_mut() {
        hd.install_cache_storage(Arc::new(CacheBackend::new(conn)));
    }
    vm.inner.navigation.current_url = url::Url::parse("https://example.com/page").unwrap();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    // SAFETY: `session` / `dom` outlive the VM (unbound below before they drop).
    #[allow(unsafe_code)]
    unsafe {
        super::super::test_helpers::bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval(script).expect("window cache put evaluates");
    // `eval` drains the `.then` tail; a few extra task/microtask passes make
    // the commit robust regardless of chain depth.
    for _ in 0..20 {
        vm.inner.drain_tasks();
        vm.inner.drain_microtasks();
    }
    let _ = vm.eval("String(globalThis.__out)");
    vm.unbind();
    drop((session, dom));
    let _ = doc;
}
