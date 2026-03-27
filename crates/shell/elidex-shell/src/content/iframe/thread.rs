//! Cross-origin iframe thread event loop.

use elidex_plugin::Size;

use super::load::apply_sandbox_origin_from_flags;
use super::types::{BrowserToIframe, IframeToBrowser};

use crate::ipc::LocalChannel;

/// Build a `MouseEventInit` from a `MouseClickEvent`, preserving modifier keys.
///
/// Shared between parent `handle_click` and iframe event routing to avoid
/// divergence in modifier key handling (B3 fix).
pub(in crate::content) fn mouse_event_init_from_click(
    click: &crate::ipc::MouseClickEvent,
) -> elidex_plugin::MouseEventInit {
    elidex_plugin::MouseEventInit {
        client_x: click.client_point.x,
        client_y: click.client_point.y,
        button: i16::from(click.button),
        alt_key: click.mods.alt,
        ctrl_key: click.mods.ctrl,
        meta_key: click.mods.meta,
        shift_key: click.mods.shift,
        ..Default::default()
    }
}

/// Select event types for a mouse click based on button number.
///
/// DOM spec: click fires only for the primary button (button 0);
/// auxclick fires for non-primary buttons (UI Events §3.5).
pub(in crate::content) fn click_event_types(button: u8) -> &'static [&'static str] {
    if button == 0 {
        &["mousedown", "mouseup", "click"]
    } else {
        &["mousedown", "mouseup", "auxclick"]
    }
}

/// Event loop for a cross-origin iframe running in its own thread.
///
/// Processes `BrowserToIframe` messages and sends `IframeToBrowser` responses.
/// The thread owns the full `PipelineResult` (DOM, JS, styles, layout).
pub(super) fn iframe_thread_main(
    mut pipeline: crate::PipelineResult,
    channel: &LocalChannel<IframeToBrowser, BrowserToIframe>,
) {
    use std::time::{Duration, Instant};

    let mut last_frame = Instant::now();
    let frame_interval = Duration::from_millis(16);

    loop {
        let timeout = pipeline
            .runtime
            .next_timer_deadline()
            .map_or(Duration::from_millis(100), |d| {
                d.saturating_duration_since(Instant::now())
                    .max(Duration::from_millis(1))
            })
            .min(frame_interval);

        match channel.recv_timeout(timeout) {
            Ok(msg) => match msg {
                BrowserToIframe::Shutdown => break,
                BrowserToIframe::MouseClick(click) => {
                    dispatch_click_in_pipeline(&mut pipeline, &click);
                }
                BrowserToIframe::KeyEvent {
                    event_type,
                    key,
                    code,
                    repeat,
                    mods,
                } => {
                    dispatch_key_in_pipeline(&mut pipeline, &event_type, &key, &code, repeat, mods);
                }
                BrowserToIframe::SetViewport { width, height } => {
                    pipeline.viewport = Size::new(width, height);
                }
                BrowserToIframe::PostMessage { data, origin } => {
                    dispatch_message_in_pipeline(&mut pipeline, &data, &origin);
                }
                BrowserToIframe::Navigate(url) => {
                    handle_navigate(&mut pipeline, &url, channel);
                }
            },
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
        }

        // Frame tick: drain timers.
        let now = Instant::now();
        if pipeline
            .runtime
            .next_timer_deadline()
            .is_some_and(|d| d <= now)
        {
            pipeline.runtime.drain_timers(
                &mut pipeline.session,
                &mut pipeline.dom,
                pipeline.document,
            );
        }

        // Re-render and send updated display list.
        let dt = now.duration_since(last_frame);
        if dt >= frame_interval {
            last_frame = now;
            crate::re_render(&mut pipeline);
            let _ = channel.send(IframeToBrowser::DisplayListReady(
                pipeline.display_list.clone(),
            ));
        }

        // Forward postMessage from iframe JS to parent.
        for (data, origin) in pipeline.runtime.bridge().drain_post_messages() {
            let _ = channel.send(IframeToBrowser::PostMessage { data, origin });
        }
    }
}

// ---------------------------------------------------------------------------
// Internal dispatch helpers
// ---------------------------------------------------------------------------

fn dispatch_click_in_pipeline(
    pipeline: &mut crate::PipelineResult,
    click: &crate::ipc::MouseClickEvent,
) {
    let query = elidex_layout::HitTestQuery {
        point: click.point,
        scroll: elidex_plugin::Vector::<f32>::ZERO,
    };
    let Some(hit) = elidex_layout::hit_test_with_scroll(&pipeline.dom, &query) else {
        return;
    };
    let mouse_init = mouse_event_init_from_click(click);
    for &event_type in click_event_types(click.button) {
        let mut event = elidex_script_session::DispatchEvent::new_composed(event_type, hit.entity);
        if event_type == "auxclick" {
            event.cancelable = false;
        }
        event.payload = elidex_plugin::EventPayload::Mouse(mouse_init.clone());
        pipeline.runtime.dispatch_event(
            &mut event,
            &mut pipeline.session,
            &mut pipeline.dom,
            pipeline.document,
        );
    }
}

fn dispatch_key_in_pipeline(
    pipeline: &mut crate::PipelineResult,
    event_type: &str,
    key: &str,
    code: &str,
    repeat: bool,
    mods: crate::ipc::ModifierState,
) {
    let init = elidex_plugin::KeyboardEventInit {
        key: key.to_string(),
        code: code.to_string(),
        repeat,
        alt_key: mods.alt,
        ctrl_key: mods.ctrl,
        meta_key: mods.meta,
        shift_key: mods.shift,
    };
    let mut event =
        elidex_script_session::DispatchEvent::new_composed(event_type, pipeline.document);
    event.payload = elidex_plugin::EventPayload::Keyboard(init);
    pipeline.runtime.dispatch_event(
        &mut event,
        &mut pipeline.session,
        &mut pipeline.dom,
        pipeline.document,
    );
}

fn dispatch_message_in_pipeline(pipeline: &mut crate::PipelineResult, data: &str, origin: &str) {
    let mut event =
        elidex_script_session::DispatchEvent::new_composed("message", pipeline.document);
    event.bubbles = false;
    event.cancelable = false;
    event.payload = elidex_plugin::EventPayload::Message {
        data: data.to_string(),
        origin: origin.to_string(),
        last_event_id: String::new(),
    };
    pipeline.runtime.dispatch_event(
        &mut event,
        &mut pipeline.session,
        &mut pipeline.dom,
        pipeline.document,
    );
}

/// Handle Navigate in OOP iframe: rebuild pipeline, applying sandbox origin (B1 fix).
fn handle_navigate(
    pipeline: &mut crate::PipelineResult,
    url: &url::Url,
    channel: &LocalChannel<IframeToBrowser, BrowserToIframe>,
) {
    match crate::build_pipeline_from_url(url) {
        Ok(new_pipeline) => {
            // Preserve sandbox flags from the old pipeline.
            let sandbox = pipeline.runtime.bridge().sandbox_flags();
            *pipeline = new_pipeline;
            pipeline.runtime.bridge().set_sandbox_flags(sandbox);
            // Apply sandbox origin override (B1 fix): sandboxed iframes without
            // allow-same-origin must get opaque origin even after navigation.
            pipeline
                .runtime
                .bridge()
                .set_origin(apply_sandbox_origin_from_flags(
                    elidex_plugin::SecurityOrigin::from_url(url),
                    sandbox,
                ));
            let _ = channel.send(IframeToBrowser::DisplayListReady(
                pipeline.display_list.clone(),
            ));
        }
        Err(e) => {
            eprintln!("OOP iframe navigate to {url} failed: {e}");
        }
    }
}
