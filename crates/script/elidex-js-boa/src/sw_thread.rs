//! Service Worker thread event loop.
//!
//! Runs a `JsRuntime::for_service_worker()` in a dedicated thread,
//! communicating with the browser/content thread via `crossbeam_channel`.
//!
//! Follows the `worker_thread.rs` pattern but handles SW-specific events:
//! install, activate, fetch, sync, and notification events.

use std::time::{Duration, Instant};

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
                        // Dispatch 'install' event via JS eval.
                        let result = runtime.eval(
                            "if (typeof __elidex_sw_dispatch__ === 'function') \
                             __elidex_sw_dispatch__('install');",
                            &mut session,
                            &mut dom,
                            doc,
                        );
                        let _ = channel.send(SwToContent::LifecycleComplete {
                            event: LifecycleEvent::Install,
                            success: result.success,
                        });
                    }
                    ContentToSw::Activate => {
                        let result = runtime.eval(
                            "if (typeof __elidex_sw_dispatch__ === 'function') \
                             __elidex_sw_dispatch__('activate');",
                            &mut session,
                            &mut dom,
                            doc,
                        );
                        let _ = channel.send(SwToContent::LifecycleComplete {
                            event: LifecycleEvent::Activate,
                            success: result.success,
                        });
                    }
                    ContentToSw::FetchEvent {
                        fetch_id,
                        request,
                        client_id,
                        resulting_client_id,
                    } => {
                        // Set up fetch event data for JS dispatch.
                        let fetch_js = format!(
                            "if (typeof __elidex_sw_fetch__ === 'function') \
                             __elidex_sw_fetch__({}, '{}', '{}', '{}', '{}');",
                            fetch_id, request.url, request.method, client_id, resulting_client_id,
                        );
                        let result = runtime.eval(&fetch_js, &mut session, &mut dom, doc);
                        if !result.success {
                            let _ = channel.send(SwToContent::FetchPassthrough { fetch_id });
                        }
                        // Response is sent via bridge mechanism (pending_sw_response).
                    }
                    ContentToSw::SyncEvent {
                        tag,
                        last_chance: _,
                    } => {
                        let sync_js = format!(
                            "if (typeof __elidex_sw_sync__ === 'function') \
                             __elidex_sw_sync__('{}');",
                            tag.replace('\'', "\\'")
                        );
                        let result = runtime.eval(&sync_js, &mut session, &mut dom, doc);
                        let _ = channel.send(SwToContent::SyncComplete {
                            tag,
                            success: result.success,
                        });
                    }
                    ContentToSw::PeriodicSyncEvent { tag } => {
                        let sync_js = format!(
                            "if (typeof __elidex_sw_periodic_sync__ === 'function') \
                             __elidex_sw_periodic_sync__('{}');",
                            tag.replace('\'', "\\'")
                        );
                        let result = runtime.eval(&sync_js, &mut session, &mut dom, doc);
                        let _ = channel.send(SwToContent::PeriodicSyncComplete {
                            tag,
                            success: result.success,
                        });
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
                        let js = format!(
                            "if (typeof __elidex_sw_dispatch__ === 'function') \
                             __elidex_sw_dispatch__('{}', '{}');",
                            event_type,
                            tag_str.replace('\'', "\\'")
                        );
                        runtime.eval(&js, &mut session, &mut dom, doc);
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
