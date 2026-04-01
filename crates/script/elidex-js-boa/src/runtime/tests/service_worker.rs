//! Service Worker E2E tests.
//!
//! Tests SW lifecycle dispatch, FetchEvent handling, and respondWith().

use crate::runtime::sw::FetchEventResult;
use crate::JsRuntime;
use boa_engine::{js_string, JsValue};
use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

/// Create a SW-mode runtime for testing.
fn setup_sw() -> (JsRuntime, SessionCore, EcsDom, elidex_ecs::Entity) {
    let np = elidex_net::broker::spawn_network_process(elidex_net::NetClient::new());
    let nh = std::rc::Rc::new(np.create_renderer_handle());
    std::mem::forget(np);
    let scope = url::Url::parse("https://example.com/").unwrap();
    let script_url = url::Url::parse("https://example.com/sw.js").unwrap();
    let runtime = JsRuntime::for_service_worker(Some(nh), &scope, script_url);
    let session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    (runtime, session, dom, doc)
}

/// Test that the install event dispatches successfully.
#[test]
fn install_event_dispatch() {
    let (mut runtime, mut session, mut dom, doc) = setup_sw();

    runtime.eval(
        "self.addEventListener('install', function(e) { console.log('installed'); });",
        &mut session,
        &mut dom,
        doc,
    );

    let success = runtime.dispatch_sw_event(&mut session, &mut dom, doc, "install", &[]);
    assert!(success);

    let messages = runtime.console_output().messages();
    assert!(
        messages
            .iter()
            .any(|(_level, msg)| msg.contains("installed")),
        "messages: {messages:?}"
    );
}

/// Test that the activate event dispatches successfully.
#[test]
fn activate_event_dispatch() {
    let (mut runtime, mut session, mut dom, doc) = setup_sw();

    runtime.eval(
        "self.addEventListener('activate', function(e) { console.log('activated'); });",
        &mut session,
        &mut dom,
        doc,
    );

    let success = runtime.dispatch_sw_event(&mut session, &mut dom, doc, "activate", &[]);
    assert!(success);

    let messages = runtime.console_output().messages();
    assert!(
        messages
            .iter()
            .any(|(_level, msg)| msg.contains("activated")),
        "messages: {messages:?}"
    );
}

/// Test FetchEvent dispatch with respondWith() returning a string.
#[test]
fn fetch_event_respond_with_string() {
    let (mut runtime, mut session, mut dom, doc) = setup_sw();

    runtime.eval(
        "self.addEventListener('fetch', function(e) { \
            e.respondWith('cached response'); \
        });",
        &mut session,
        &mut dom,
        doc,
    );

    let request_obj = boa_engine::object::ObjectInitializer::new(runtime.context_mut())
        .property(
            js_string!("url"),
            JsValue::from(js_string!("https://example.com/")),
            boa_engine::property::Attribute::READONLY,
        )
        .property(
            js_string!("method"),
            JsValue::from(js_string!("GET")),
            boa_engine::property::Attribute::READONLY,
        )
        .build();

    let props = [
        ("request", JsValue::from(request_obj)),
        ("clientId", JsValue::from(js_string!("test-client"))),
        ("resultingClientId", JsValue::from(js_string!(""))),
        ("replacesClientId", JsValue::from(js_string!(""))),
    ];

    let result = runtime.dispatch_fetch_event(&mut session, &mut dom, doc, &props);
    match result {
        FetchEventResult::Responded { body, status, .. } => {
            assert_eq!(body, "cached response");
            assert_eq!(status, 200);
        }
        other => panic!("expected Responded, got {other:?}"),
    }
}

/// Test FetchEvent passthrough (no respondWith called).
#[test]
fn fetch_event_passthrough() {
    let (mut runtime, mut session, mut dom, doc) = setup_sw();

    runtime.eval(
        "self.addEventListener('fetch', function(e) { \
            console.log('fetch event received'); \
        });",
        &mut session,
        &mut dom,
        doc,
    );

    let request_obj = boa_engine::object::ObjectInitializer::new(runtime.context_mut())
        .property(
            js_string!("url"),
            JsValue::from(js_string!("https://example.com/page")),
            boa_engine::property::Attribute::READONLY,
        )
        .build();

    let props = [
        ("request", JsValue::from(request_obj)),
        ("clientId", JsValue::from(js_string!(""))),
        ("resultingClientId", JsValue::from(js_string!(""))),
        ("replacesClientId", JsValue::from(js_string!(""))),
    ];

    let result = runtime.dispatch_fetch_event(&mut session, &mut dom, doc, &props);
    assert!(matches!(result, FetchEventResult::Passthrough));
}

/// Test FetchEvent with no listeners returns Passthrough.
#[test]
fn fetch_event_no_listeners() {
    let (mut runtime, mut session, mut dom, doc) = setup_sw();

    let request_obj = boa_engine::object::ObjectInitializer::new(runtime.context_mut())
        .property(
            js_string!("url"),
            JsValue::from(js_string!("https://example.com/")),
            boa_engine::property::Attribute::READONLY,
        )
        .build();

    let props = [
        ("request", JsValue::from(request_obj)),
        ("clientId", JsValue::from(js_string!(""))),
        ("resultingClientId", JsValue::from(js_string!(""))),
        ("replacesClientId", JsValue::from(js_string!(""))),
    ];

    let result = runtime.dispatch_fetch_event(&mut session, &mut dom, doc, &props);
    assert!(matches!(result, FetchEventResult::Passthrough));
}

/// Test that FetchEvent.request properties are accessible in JS.
#[test]
fn fetch_event_request_properties() {
    let (mut runtime, mut session, mut dom, doc) = setup_sw();

    runtime.eval(
        "self.addEventListener('fetch', function(e) { \
            console.log('url:' + e.request.url); \
            console.log('method:' + e.request.method); \
            console.log('clientId:' + e.clientId); \
            console.log('replacesClientId:' + e.replacesClientId); \
        });",
        &mut session,
        &mut dom,
        doc,
    );

    let request_obj = boa_engine::object::ObjectInitializer::new(runtime.context_mut())
        .property(
            js_string!("url"),
            JsValue::from(js_string!("https://test.com/api")),
            boa_engine::property::Attribute::READONLY,
        )
        .property(
            js_string!("method"),
            JsValue::from(js_string!("POST")),
            boa_engine::property::Attribute::READONLY,
        )
        .build();

    let props = [
        ("request", JsValue::from(request_obj)),
        ("clientId", JsValue::from(js_string!("abc-123-def-456"))),
        ("resultingClientId", JsValue::from(js_string!(""))),
        ("replacesClientId", JsValue::from(js_string!(""))),
    ];

    let _ = runtime.dispatch_fetch_event(&mut session, &mut dom, doc, &props);

    let messages = runtime.console_output().messages();
    assert!(
        messages
            .iter()
            .any(|(_l, m)| m.contains("url:https://test.com/api")),
        "messages: {messages:?}"
    );
    assert!(
        messages.iter().any(|(_l, m)| m.contains("method:POST")),
        "messages: {messages:?}"
    );
    assert!(
        messages
            .iter()
            .any(|(_l, m)| m.contains("clientId:abc-123-def-456")),
        "messages: {messages:?}"
    );
}

/// Test that Client.id is UUID v4 format.
#[test]
fn client_id_is_uuid_v4() {
    let (runtime, _session, _dom, _doc) = setup_sw();
    let client_id = runtime.bridge().client_id();
    assert_eq!(client_id.len(), 36, "client_id={client_id}");
    assert_eq!(
        client_id.chars().filter(|c| *c == '-').count(),
        4,
        "expected 4 hyphens in UUID"
    );
    assert!(
        client_id.chars().all(|c| c.is_ascii_hexdigit() || c == '-'),
        "non-hex chars in UUID: {client_id}"
    );
}
