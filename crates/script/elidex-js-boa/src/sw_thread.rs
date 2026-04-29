//! Service Worker thread event loop.
//!
//! Runs a `JsRuntime::for_service_worker()` in a dedicated thread,
//! communicating with the browser/content thread via `crossbeam_channel`.
//!
//! Follows the `worker_thread.rs` pattern but handles SW-specific events:
//! install, activate, fetch, sync, and notification events.

use std::time::{Duration, Instant};

use boa_engine::{js_string, JsValue};
use crossbeam_channel::RecvTimeoutError;
use elidex_api_sw::{ContentToSw, LifecycleEvent, SwToContent};
use elidex_ecs::EcsDom;
use elidex_plugin::LocalChannel;
use elidex_script_session::SessionCore;

use crate::JsRuntime;

/// 16ms frame interval for timer drain.
const FRAME_INTERVAL: Duration = Duration::from_millis(16);

/// Default idle timeout before terminating an idle SW thread.
const IDLE_TIMEOUT: Duration = Duration::from_secs(30);

/// Entry point for the Service Worker thread.
///
/// - Fetches the SW script via `network_handle`.
/// - Evaluates the script in a `JsRuntime::for_service_worker()`.
/// - Enters an event loop handling Install/Activate/FetchEvent/Sync/Notification.
/// - Exits on `Shutdown`, channel disconnect, or idle timeout.
#[allow(clippy::needless_pass_by_value)]
pub fn sw_thread_main(
    script_url: url::Url,
    scope: url::Url,
    channel: LocalChannel<SwToContent, ContentToSw>,
    network_handle: elidex_net::broker::NetworkHandle,
) {
    // 1. Fetch the SW script.
    let request = elidex_net::Request {
        method: "GET".to_string(),
        url: script_url.clone(),
        headers: vec![("Service-Worker".to_string(), "script".to_string())],
        body: bytes::Bytes::new(),
        ..Default::default()
    };

    let response = match network_handle.fetch_blocking(request) {
        Ok(resp) => resp,
        Err(e) => {
            let _ = channel.send(SwToContent::Error {
                message: format!("Failed to fetch SW script: {e}"),
                filename: script_url.to_string(),
                lineno: 0,
                colno: 0,
            });
            let _ = channel.send(SwToContent::LifecycleComplete {
                event: LifecycleEvent::Install,
                success: false,
            });
            return;
        }
    };

    // 2. Validate MIME type.
    let script_source = match crate::globals::worker_constructor::validate_worker_script_response(
        &response,
        &script_url,
    ) {
        Ok(source) => source,
        Err(msg) => {
            let _ = channel.send(SwToContent::Error {
                message: msg,
                filename: script_url.to_string(),
                lineno: 0,
                colno: 0,
            });
            let _ = channel.send(SwToContent::LifecycleComplete {
                event: LifecycleEvent::Install,
                success: false,
            });
            return;
        }
    };

    // 3. Create runtime and evaluate script.
    let worker_net = std::rc::Rc::new(network_handle);
    sw_thread_run(script_source, script_url, scope, channel, worker_net);
}

/// Entry point with pre-fetched script source (for testing).
#[allow(clippy::needless_pass_by_value)]
pub fn sw_thread_main_with_source(
    script_source: String,
    script_url: url::Url,
    scope: url::Url,
    channel: LocalChannel<SwToContent, ContentToSw>,
) {
    let broker = elidex_net::broker::spawn_network_process(elidex_net::NetClient::new());
    let nh = std::rc::Rc::new(broker.create_renderer_handle());
    sw_thread_run(script_source, script_url, scope, channel, nh);
}

#[allow(clippy::needless_pass_by_value, clippy::too_many_lines)]
fn sw_thread_run(
    script_source: String,
    script_url: url::Url,
    scope: url::Url,
    channel: LocalChannel<SwToContent, ContentToSw>,
    worker_net: std::rc::Rc<elidex_net::broker::NetworkHandle>,
) {
    let mut runtime = JsRuntime::for_service_worker(Some(worker_net), &scope, script_url.clone());

    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let mut session = SessionCore::new();

    // Evaluate SW script.
    let eval_result = runtime.eval(&script_source, &mut session, &mut dom, doc);
    if !eval_result.success {
        let msg = eval_result
            .error
            .unwrap_or_else(|| "SW script evaluation failed".into());
        let _ = channel.send(SwToContent::Error {
            message: msg,
            filename: script_url.to_string(),
            lineno: 0,
            colno: 0,
        });
        let _ = channel.send(SwToContent::LifecycleComplete {
            event: LifecycleEvent::Install,
            success: false,
        });
        return;
    }

    // Event loop.
    let mut last_activity = Instant::now();

    loop {
        let timeout = FRAME_INTERVAL.min(
            IDLE_TIMEOUT
                .checked_sub(last_activity.elapsed())
                .unwrap_or(Duration::ZERO),
        );

        match channel.recv_timeout(timeout) {
            Ok(msg) => {
                last_activity = Instant::now();
                match msg {
                    ContentToSw::Install => {
                        let success =
                            runtime.dispatch_sw_event(&mut session, &mut dom, doc, "install", &[]);
                        let _ = channel.send(SwToContent::LifecycleComplete {
                            event: LifecycleEvent::Install,
                            success,
                        });
                    }
                    ContentToSw::Activate => {
                        let success =
                            runtime.dispatch_sw_event(&mut session, &mut dom, doc, "activate", &[]);
                        let _ = channel.send(SwToContent::LifecycleComplete {
                            event: LifecycleEvent::Activate,
                            success,
                        });
                    }
                    ContentToSw::FetchEvent {
                        fetch_id,
                        request,
                        client_id,
                        resulting_client_id,
                    } => {
                        use crate::runtime::sw::FetchEventResult;

                        // Build request object for the FetchEvent.
                        let request_obj =
                            boa_engine::object::ObjectInitializer::new(runtime.context_mut())
                                .property(
                                    js_string!("url"),
                                    JsValue::from(js_string!(request.url.as_str())),
                                    boa_engine::property::Attribute::READONLY,
                                )
                                .property(
                                    js_string!("method"),
                                    JsValue::from(js_string!(request.method.as_str())),
                                    boa_engine::property::Attribute::READONLY,
                                )
                                .property(
                                    js_string!("mode"),
                                    JsValue::from(js_string!(request.mode.as_str())),
                                    boa_engine::property::Attribute::READONLY,
                                )
                                .property(
                                    js_string!("destination"),
                                    JsValue::from(js_string!(request.destination.as_str())),
                                    boa_engine::property::Attribute::READONLY,
                                )
                                .build();

                        let props = [
                            ("request", JsValue::from(request_obj)),
                            ("clientId", JsValue::from(js_string!(client_id.as_str()))),
                            (
                                "resultingClientId",
                                JsValue::from(js_string!(resulting_client_id.as_str())),
                            ),
                            // WHATWG SW §4.6: replacesClientId (empty string for non-navigation).
                            ("replacesClientId", JsValue::from(js_string!(""))),
                        ];

                        match runtime.dispatch_fetch_event(&mut session, &mut dom, doc, &props) {
                            FetchEventResult::Responded {
                                body,
                                status,
                                status_text,
                                headers,
                            } => {
                                let _ = channel.send(SwToContent::FetchResponse {
                                    fetch_id,
                                    response: elidex_api_sw::SwResponse {
                                        status,
                                        status_text,
                                        headers,
                                        body: body.into_bytes(),
                                        url: request.url,
                                    },
                                });
                            }
                            FetchEventResult::Passthrough | FetchEventResult::Error => {
                                let _ = channel.send(SwToContent::FetchPassthrough { fetch_id });
                            }
                        }
                    }
                    ContentToSw::SyncEvent { tag, last_chance } => {
                        let props = [
                            ("tag", JsValue::from(js_string!(tag.as_str()))),
                            ("lastChance", JsValue::from(last_chance)),
                        ];
                        let success =
                            runtime.dispatch_sw_event(&mut session, &mut dom, doc, "sync", &props);
                        let _ = channel.send(SwToContent::SyncComplete { tag, success });
                    }
                    ContentToSw::PeriodicSyncEvent { tag } => {
                        let props = [("tag", JsValue::from(js_string!(tag.as_str())))];
                        let success = runtime.dispatch_sw_event(
                            &mut session,
                            &mut dom,
                            doc,
                            "periodicsync",
                            &props,
                        );
                        let _ = channel.send(SwToContent::PeriodicSyncComplete { tag, success });
                    }
                    ContentToSw::PostMessage {
                        data,
                        origin,
                        client_id: _,
                    } => {
                        runtime.dispatch_worker_message(
                            &mut session,
                            &mut dom,
                            doc,
                            &data,
                            &origin,
                        );
                    }
                    ContentToSw::NotificationEvent {
                        action,
                        tag,
                        notification_data: _,
                    } => {
                        let event_type = match &action {
                            elidex_api_sw::types::NotificationAction::Click { .. } => {
                                "notificationclick"
                            }
                            elidex_api_sw::types::NotificationAction::Close => "notificationclose",
                        };
                        let tag_str = tag.as_deref().unwrap_or("");
                        let props = [("tag", JsValue::from(js_string!(tag_str)))];
                        runtime.dispatch_sw_event(&mut session, &mut dom, doc, event_type, &props);
                    }
                    ContentToSw::Shutdown => {
                        runtime.bridge().clear_all_timers();
                        break;
                    }
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                // Check idle timeout.
                if last_activity.elapsed() >= IDLE_TIMEOUT {
                    // SW idle for too long — terminate.
                    runtime.bridge().clear_all_timers();
                    break;
                }
            }
            Err(RecvTimeoutError::Disconnected) => {
                break;
            }
        }

        // Drain timers.
        runtime.drain_timers(&mut session, &mut dom, doc);

        // Drain outgoing messages (postMessage from SW to clients).
        for msg in runtime.drain_worker_outgoing() {
            if let elidex_api_workers::WorkerToParent::PostMessage { data, origin: _ } = msg {
                let _ = channel.send(SwToContent::PostMessage {
                    client_id: String::new(), // TODO: route to correct client
                    data,
                });
            }
        }
    }
}
