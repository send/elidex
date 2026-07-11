//! Cross-origin iframe thread event loop.

use elidex_plugin::Size;
use elidex_script_session::HostDriver;

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
            pipeline.drain_timers();
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
    // Move focus to the clicked element before dispatching the mouse events,
    // mirroring the parent `handle_click` order (a non-focusable target blurs the
    // current focus inside `set_focus`). Without this an OOP iframe's
    // `activeElement` / `:focus` never tracked clicks — focus routing was the
    // parent's job, but the OOP iframe owns its own `EcsDom`.
    crate::content::focus::set_focus(pipeline, hit.entity);
    let mouse_init = mouse_event_init_from_click(click);
    for &event_type in click_event_types(click.button) {
        let mut event = elidex_script_session::DispatchEvent::new_composed(event_type, hit.entity);
        if event_type == "auxclick" {
            event.cancelable = false;
        }
        event.payload = elidex_plugin::EventPayload::Mouse(mouse_init.clone());
        pipeline.dispatch_event(&mut event);
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
    // Route the key event to the focused element (fallback: the document) so a
    // focused control in the OOP iframe receives keystrokes, instead of the event
    // always hard-targeting the document root. `current_focus` applies its own
    // connectedness filter, so a stale-detached holder never wins.
    let target = elidex_dom_api::focus::current_focus(&pipeline.dom, pipeline.document)
        .unwrap_or(pipeline.document);
    let mut event = elidex_script_session::DispatchEvent::new_composed(event_type, target);
    event.payload = elidex_plugin::EventPayload::Keyboard(init);
    pipeline.dispatch_event(&mut event);
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
    pipeline.dispatch_event(&mut event);
}

/// Handle Navigate in OOP iframe: rebuild pipeline, applying sandbox origin (B1 fix).
fn handle_navigate(
    pipeline: &mut crate::PipelineResult,
    url: &url::Url,
    channel: &LocalChannel<IframeToBrowser, BrowserToIframe>,
) {
    // Rebuild at the iframe's current box (tracked in `pipeline.viewport` via
    // SetViewport), not DEFAULT — consistent with the top-level navigation fix (C1).
    let viewport = pipeline.viewport;
    // Carry the frame's *persistent* security state from the old pipeline into
    // the re-build — these are properties of the browsing context / frame, not
    // of the retired document, so they survive the navigation:
    //   - sandbox flags: the `<iframe sandbox>` attribute governs the context
    //     (a sandboxed iframe without allow-same-origin stays opaque);
    //   - credentialless: a credentialless context keeps its opaque origin
    //     across navigations (F-c) — hardcoding `false` here regained a tuple
    //     origin on navigation;
    //   - depth: a property of the frame's position;
    //   - referrer: the parent document URL (§4.8.5), unchanged by a same-frame
    //     navigation (the OOP path never set it pre-S5-4b — F-b).
    // The origin itself is NOT precomputed here: it must derive from the
    // POST-redirect `loaded.url`, which only `build_pipeline_from_url` knows
    // after resolving the fetch (F-a). Passing [`crate::PreEvalFrameInputs`]
    // defers that derivation to the builder, which then installs the resulting
    // [`crate::PreEvalFrameState`] at the pre-eval chokepoint so the navigated
    // document's *initial* scripts already observe the final origin — where
    // previously this re-build precomputed the origin from the *requested* URL
    // (mis-attributing redirected loads) and installed after the scripts ran.
    let bridge = pipeline.runtime.bridge();
    let inputs = crate::PreEvalFrameInputs {
        sandbox_flags: bridge.sandbox_flags(),
        credentialless: bridge.credentialless(),
        iframe_depth: bridge.iframe_depth(),
        referrer: bridge.referrer(),
    };
    // KNOWN-INCOMPLETE (slot #11-oop-iframe-navigate-completeness — a new carve;
    // ledger registration is a landing deliverable). This `Navigate` path has NO
    // production sender today (nothing sends `BrowserToIframe::Navigate` yet), so
    // both gaps below are latent, not live:
    //   (a) referrer: `inputs.referrer` carries the INITIAL parent referrer
    //       (`bridge.referrer()`) across the frame's OWN navigation. A real
    //       in-frame navigation should instead source the referrer from the
    //       frame's PREVIOUS document URL (per-navigation referrer chain,
    //       HTML §7.4.2 Navigation),
    //       not the embedder's original referrer.
    //   (b) cookies: `build_pipeline_from_url` spawns a fresh standalone broker
    //       with an EMPTY cookie jar (relocated verbatim, pre-existing) — a real
    //       navigation must hand off the frame's existing cookie jar / network
    //       handle instead of starting cookie-less.
    // Neither is fixed here: referrer-chain tracking + per-navigation cookie-jar
    // handoff are out of scope until a production Navigate sender exists.
    match crate::build_pipeline_from_url(url, viewport, Some(inputs)) {
        Ok(new_pipeline) => {
            *pipeline = new_pipeline;
            let _ = channel.send(IframeToBrowser::DisplayListReady(
                pipeline.display_list.clone(),
            ));
        }
        Err(e) => {
            eprintln!("OOP iframe navigate to {url} failed: {e}");
        }
    }
}
