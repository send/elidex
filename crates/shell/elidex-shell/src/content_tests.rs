use super::test_support::{spawn_test_content, test_network};
use super::*;
use crate::ipc::{self, BrowserToContent, ContentToBrowser, ModifierState};
use elidex_plugin::Point;
use std::time::Duration;

/// PR-A: a content-initiated frame must **wake** the browser event loop so it
/// reaches a rendering opportunity (WHATWG HTML §8.1.7.3) under
/// `ControlFlow::Wait`. Inject a counting `WakeHandle` and assert the initial
/// `DisplayListReady` send (via `notify_browser`) invoked it.
#[test]
fn content_thread_wake_fires_on_display_list() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    let (browser, content) = ipc::channel_pair::<BrowserToContent, ContentToBrowser>();
    let (nh, jar, _np) = test_network();
    let wakes = Arc::new(AtomicUsize::new(0));
    let wakes_for_thread = Arc::clone(&wakes);
    let wake: crate::WakeHandle = Box::new(move || {
        wakes_for_thread.fetch_add(1, Ordering::SeqCst);
    });
    let join = spawn_content_thread(
        content,
        nh,
        jar,
        "<div>Hi</div>".to_string(),
        "div { display: block; }".to_string(),
        elidex_plugin::Size::new(
            crate::DEFAULT_VIEWPORT_WIDTH,
            crate::DEFAULT_VIEWPORT_HEIGHT,
        ),
        wake,
    );

    // The content thread sends an initial DisplayListReady on startup.
    let msg = browser.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(matches!(msg, ContentToBrowser::DisplayListReady(_)));

    // The send routes through `notify_browser`, which calls `wake()` right after
    // the channel send — poll briefly for it to land (the wake runs on the
    // content thread a hair after the message is enqueued).
    let mut fired = false;
    for _ in 0..200 {
        if wakes.load(Ordering::SeqCst) >= 1 {
            fired = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(
        fired,
        "content-initiated DisplayListReady must invoke the injected wake"
    );

    browser.send(BrowserToContent::Shutdown).unwrap();
    join.join().unwrap();
}

/// PR-A D1/F4 layering guard: the content thread is the CSS/renderer owner
/// (*concurrency-by-ownership*) and must stay free of the windowing system. It
/// signals repaints via `crate::WakeHandle` (a boxed `Fn`), never a `winit` type
/// or the browser-internal `WakeEvent` payload. Fails if any non-comment line
/// under `src/content/` references either.
#[test]
fn content_module_is_winit_free() {
    fn walk(dir: &std::path::Path, offenders: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).expect("read content dir") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                walk(&path, offenders);
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let src = std::fs::read_to_string(&path).expect("read source");
            for (i, line) in src.lines().enumerate() {
                // Skip line/doc comments (`//`, `///`, `//!`) — prose like
                // "winit-free" is allowed; only real usage is forbidden.
                if line.trim_start().starts_with("//") {
                    continue;
                }
                if line.contains("winit") || line.contains("WakeEvent") {
                    offenders.push(format!("{}:{}: {}", path.display(), i + 1, line.trim()));
                }
            }
        }
    }

    let content_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/content");
    let mut offenders = Vec::new();
    walk(&content_dir, &mut offenders);
    assert!(
        offenders.is_empty(),
        "content/ must stay winit-free (PR-A D1/F4) — found windowing references:\n{}",
        offenders.join("\n")
    );
}

#[test]
fn content_thread_startup_and_shutdown() {
    let (browser, content) = ipc::channel_pair::<BrowserToContent, ContentToBrowser>();
    let (nh, jar, _np) = test_network();
    let handle = spawn_test_content(
        content,
        nh,
        jar,
        "<div>Hello</div>".to_string(),
        "div { display: block; }".to_string(),
    );

    // Should receive initial DisplayListReady.
    let msg = browser.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(matches!(msg, ContentToBrowser::DisplayListReady(_)));

    // Send shutdown.
    browser.send(BrowserToContent::Shutdown).unwrap();
    handle.join().unwrap();
}

#[test]
fn content_thread_mouse_move() {
    let (browser, content) = ipc::channel_pair::<BrowserToContent, ContentToBrowser>();
    let (nh, jar, _np) = test_network();
    let handle = spawn_test_content(
        content,
        nh,
        jar,
        "<div style=\"background-color: red; width: 200px; height: 100px;\">Test</div>".to_string(),
        "div { display: block; }".to_string(),
    );

    // Drain initial display list.
    let _ = browser.recv_timeout(Duration::from_secs(5)).unwrap();

    // Send mouse move.
    browser
        .send(BrowserToContent::MouseMove {
            point: Point::new(50.0, 50.0),
            client_point: Point::new(50.0, 86.0),
        })
        .unwrap();

    // Should get a DisplayListReady (hover state change triggers re-render).
    let msg = browser.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(matches!(msg, ContentToBrowser::DisplayListReady(_)));

    browser.send(BrowserToContent::Shutdown).unwrap();
    handle.join().unwrap();
}

#[test]
fn content_thread_click() {
    let (browser, content) = ipc::channel_pair::<BrowserToContent, ContentToBrowser>();
    let (nh, jar, _np) = test_network();
    let handle = spawn_test_content(
        content,
        nh,
        jar,
        "<div style=\"background-color: blue; width: 200px; height: 100px;\">Click</div>"
            .to_string(),
        "div { display: block; }".to_string(),
    );

    // Drain initial display list.
    let _ = browser.recv_timeout(Duration::from_secs(5)).unwrap();

    // Send click.
    browser
        .send(BrowserToContent::MouseClick(crate::ipc::MouseClickEvent {
            point: Point::new(50.0, 50.0),
            client_point: Point::new(50.0, 86.0),
            button: 0,
            mods: ModifierState::default(),
        }))
        .unwrap();

    // Should get a DisplayListReady.
    let msg = browser.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(matches!(msg, ContentToBrowser::DisplayListReady(_)));

    browser.send(BrowserToContent::Shutdown).unwrap();
    handle.join().unwrap();
}

#[test]
fn content_thread_mouse_release_clears_active() {
    let (browser, content) = ipc::channel_pair::<BrowserToContent, ContentToBrowser>();
    let (nh, jar, _np) = test_network();
    let handle = spawn_test_content(
        content,
        nh,
        jar,
        "<div style=\"background-color: blue; width: 200px; height: 100px;\">Active</div>"
            .to_string(),
        "div { display: block; }".to_string(),
    );

    // Drain initial display list.
    let _ = browser.recv_timeout(Duration::from_secs(5)).unwrap();

    // Move cursor to set hover chain.
    browser
        .send(BrowserToContent::MouseMove {
            point: Point::new(50.0, 50.0),
            client_point: Point::new(50.0, 86.0),
        })
        .unwrap();
    let _ = browser.recv_timeout(Duration::from_secs(5)).unwrap();

    // Click (sets ACTIVE).
    browser
        .send(BrowserToContent::MouseClick(crate::ipc::MouseClickEvent {
            point: Point::new(50.0, 50.0),
            client_point: Point::new(50.0, 86.0),
            button: 0,
            mods: ModifierState::default(),
        }))
        .unwrap();
    let _ = browser.recv_timeout(Duration::from_secs(5)).unwrap();

    // Release (clears ACTIVE).
    browser
        .send(BrowserToContent::MouseRelease { button: 0 })
        .unwrap();
    let msg = browser.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(matches!(msg, ContentToBrowser::DisplayListReady(_)));

    browser.send(BrowserToContent::Shutdown).unwrap();
    handle.join().unwrap();
}

#[test]
fn content_thread_disconnect() {
    let (browser, content) = ipc::channel_pair::<BrowserToContent, ContentToBrowser>();
    let (nh, jar, _np) = test_network();
    let handle = spawn_test_content(
        content,
        nh,
        jar,
        "<div>Hello</div>".to_string(),
        String::new(),
    );

    // Drain initial display list.
    let _ = browser.recv_timeout(Duration::from_secs(5)).unwrap();

    // Drop browser end — content thread should exit cleanly.
    drop(browser);
    handle.join().unwrap();
}

#[test]
fn content_thread_with_script() {
    let (browser, content) = ipc::channel_pair::<BrowserToContent, ContentToBrowser>();
    let (nh, jar, _np) = test_network();
    let handle = spawn_test_content(
        content,
        nh,
        jar,
        "<div id=\"btn\" style=\"background-color: blue; width: 200px; height: 100px;\">Click</div>\
         <script>\
           document.getElementById('btn').addEventListener('click', function(e) {\
             e.target.style.setProperty('background-color', 'red');\
           });\
         </script>".to_string(),
        "div { display: block; }".to_string(),
    );

    // Drain initial display list.
    let _ = browser.recv_timeout(Duration::from_secs(5)).unwrap();

    // Click on the element.
    browser
        .send(BrowserToContent::MouseClick(crate::ipc::MouseClickEvent {
            point: Point::new(50.0, 50.0),
            client_point: Point::new(50.0, 86.0),
            button: 0,
            mods: ModifierState::default(),
        }))
        .unwrap();

    // Should get updated display list.
    let msg = browser.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(matches!(msg, ContentToBrowser::DisplayListReady(_)));

    browser.send(BrowserToContent::Shutdown).unwrap();
    handle.join().unwrap();
}

#[test]
fn content_thread_keyboard() {
    let (browser, content) = ipc::channel_pair::<BrowserToContent, ContentToBrowser>();
    let (nh, jar, _np) = test_network();
    let handle = spawn_test_content(
        content,
        nh,
        jar,
        "<div id=\"box\" tabindex=\"0\" style=\"width: 100px; height: 100px;\">Key</div>\
         <script>\
           document.getElementById('box').addEventListener('keydown', function(e) {\
             console.log('key=' + e.key);\
           });\
         </script>"
            .to_string(),
        "div { display: block; }".to_string(),
    );

    // Drain initial display list.
    let _ = browser.recv_timeout(Duration::from_secs(5)).unwrap();

    // Click first to set focus.
    browser
        .send(BrowserToContent::MouseClick(crate::ipc::MouseClickEvent {
            point: Point::new(50.0, 50.0),
            client_point: Point::new(50.0, 86.0),
            button: 0,
            mods: ModifierState::default(),
        }))
        .unwrap();
    let _ = browser.recv_timeout(Duration::from_secs(5)).unwrap();

    // Send key down.
    browser
        .send(BrowserToContent::KeyDown {
            key: "a".to_string(),
            code: "KeyA".to_string(),
            repeat: false,
            mods: ModifierState::default(),
        })
        .unwrap();

    let msg = browser.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(matches!(msg, ContentToBrowser::DisplayListReady(_)));

    browser.send(BrowserToContent::Shutdown).unwrap();
    handle.join().unwrap();
}

// --- Scroll tests ---

#[test]
fn content_thread_mouse_wheel_scrolls_viewport() {
    let (browser, content) = ipc::channel_pair::<BrowserToContent, ContentToBrowser>();
    // Tall content that exceeds default viewport height (768px).
    let (nh, jar, _np) = test_network();
    let handle = spawn_test_content(
        content,
        nh,
        jar,
        "<div style=\"width: 200px; height: 2000px; background-color: red;\">Tall</div>"
            .to_string(),
        "div { display: block; }".to_string(),
    );

    // Drain initial display list.
    let _ = browser.recv_timeout(Duration::from_secs(5)).unwrap();

    // Send mouse wheel (scroll down).
    browser
        .send(BrowserToContent::MouseWheel {
            delta: elidex_plugin::Vector::new(0.0, 100.0),
            point: Point::new(100.0, 100.0),
        })
        .unwrap();

    // Should get a DisplayListReady from the re-render triggered by scroll.
    let msg = browser.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(matches!(msg, ContentToBrowser::DisplayListReady(_)));

    browser.send(BrowserToContent::Shutdown).unwrap();
    handle.join().unwrap();
}

#[test]
fn content_thread_mouse_wheel_no_scroll_overflow_hidden() {
    let (browser, content) = ipc::channel_pair::<BrowserToContent, ContentToBrowser>();
    let (nh, jar, _np) = test_network();
    let handle = spawn_test_content(
        content,
        nh,
        jar,
        "<div style=\"width: 200px; height: 2000px;\">Tall</div>".to_string(),
        "html { overflow: hidden; } div { display: block; }".to_string(),
    );

    // Drain initial display list.
    let _ = browser.recv_timeout(Duration::from_secs(5)).unwrap();

    // Send mouse wheel — should NOT trigger re-render because overflow: hidden.
    browser
        .send(BrowserToContent::MouseWheel {
            delta: elidex_plugin::Vector::new(0.0, 100.0),
            point: Point::new(100.0, 100.0),
        })
        .unwrap();

    // Should timeout (no DisplayListReady because scroll was blocked).
    let result = browser.recv_timeout(Duration::from_millis(200));
    assert!(result.is_err());

    browser.send(BrowserToContent::Shutdown).unwrap();
    handle.join().unwrap();
}

#[test]
fn content_thread_mouse_wheel_small_content() {
    let (browser, content) = ipc::channel_pair::<BrowserToContent, ContentToBrowser>();
    // Content smaller than viewport — no scroll needed.
    let (nh, jar, _np) = test_network();
    let handle = spawn_test_content(
        content,
        nh,
        jar,
        "<div style=\"width: 200px; height: 100px;\">Small</div>".to_string(),
        "div { display: block; }".to_string(),
    );

    // Drain initial display list.
    let _ = browser.recv_timeout(Duration::from_secs(5)).unwrap();

    // Send mouse wheel — content fits, so scroll_y stays 0 → no change → no re-render.
    browser
        .send(BrowserToContent::MouseWheel {
            delta: elidex_plugin::Vector::new(0.0, 50.0),
            point: Point::new(50.0, 50.0),
        })
        .unwrap();

    let result = browser.recv_timeout(Duration::from_millis(200));
    assert!(result.is_err());

    browser.send(BrowserToContent::Shutdown).unwrap();
    handle.join().unwrap();
}

#[test]
fn content_thread_viewport_resize_updates_scroll() {
    let (browser, content) = ipc::channel_pair::<BrowserToContent, ContentToBrowser>();
    let (nh, jar, _np) = test_network();
    let handle = spawn_test_content(
        content,
        nh,
        jar,
        "<div style=\"width: 200px; height: 2000px;\">Tall</div>".to_string(),
        "div { display: block; }".to_string(),
    );

    // Drain initial display list.
    let _ = browser.recv_timeout(Duration::from_secs(5)).unwrap();

    // Resize viewport — triggers re_render which calls update_viewport_scroll_dimensions.
    browser
        .send(BrowserToContent::SetViewport {
            width: 800.0,
            height: 600.0,
        })
        .unwrap();

    let msg = browser.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(matches!(msg, ContentToBrowser::DisplayListReady(_)));

    // Now scroll should work with the new dimensions.
    browser
        .send(BrowserToContent::MouseWheel {
            delta: elidex_plugin::Vector::new(0.0, 100.0),
            point: Point::new(100.0, 100.0),
        })
        .unwrap();

    let msg = browser.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(matches!(msg, ContentToBrowser::DisplayListReady(_)));

    browser.send(BrowserToContent::Shutdown).unwrap();
    handle.join().unwrap();
}

#[test]
fn focusable_cache_invalidates_on_all_focusability_attributes() {
    // The shell's Tab-order `focusable_cache` must rebuild whenever a mutation
    // touches any attribute that `is_focusable` reads. `href` (link default) and
    // `type` (an `<input type=hidden>` flip) were previously missing, leaving a
    // stale Tab order after a script changed them (Codex S2).
    use elidex_script_session::{MutationKind, MutationRecord};

    let mut dom = elidex_ecs::EcsDom::new();
    let target = dom.create_element("a", elidex_ecs::Attributes::default());
    let attr_record = |name: &str| MutationRecord {
        kind: MutationKind::Attribute,
        target,
        added_nodes: vec![],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: Some(name.to_string()),
        old_value: None,
    };

    for attr in [
        "tabindex",
        "disabled",
        "contenteditable",
        "hidden",
        "href",
        "type",
    ] {
        assert!(
            should_invalidate_focusable_cache(&[attr_record(attr)]),
            "a `{attr}` attribute mutation must invalidate the focusable cache"
        );
    }
    // A focus-irrelevant attribute does not invalidate.
    assert!(
        !should_invalidate_focusable_cache(&[attr_record("class")]),
        "a `class` mutation must not invalidate the focusable cache"
    );
    // A ChildList mutation always invalidates (elements added/removed).
    let child_list = MutationRecord {
        kind: MutationKind::ChildList,
        target,
        added_nodes: vec![],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: None,
        old_value: None,
    };
    assert!(should_invalidate_focusable_cache(&[child_list]));
}

// ---------------------------------------------------------------------------
// Iframe lifecycle (HTML §4.8.5 "content navigable on connection" + lazy)
// ---------------------------------------------------------------------------
//
// A lightweight in-thread `ContentState` harness (no spawned content thread):
// `eval_script` performs the *live* DOM mutations (createElement / appendChild /
// detach), then we feed `detect_iframe_mutations` the `MutationRecord`s the
// mutation pipeline would emit and assert the load / no-load / lazy outcome.
//
// We drive `detect_iframe_mutations` directly rather than relying on
// `re_render`'s `session.flush` because JS `appendChild` / `setAttribute` go
// through `DomApiHandler`s that perform direct DOM ops and do NOT record
// `ChildList` / `Attribute` mutations (documented at
// `elidex-dom-api/src/child_node/mutations.rs`; the broader fix is tracked by
// slot `#11-tree-mutation-record-pipeline`, PR #373 R2). `MutationRecord` is the
// function's real input contract, so synthesizing it tests exactly the
// `is_connected` gate + lazy re-deferral PR #373 added, independent of that
// upstream recording gap.

/// Build a `ContentState` directly for synchronous iframe-lifecycle driving.
///
/// Returns the live browser channel end alongside the state so it stays in
/// scope — dropping it would disconnect the content channel.
fn build_iframe_test_state(
    html: &str,
    css: &str,
) -> (
    ContentState,
    ipc::LocalChannel<BrowserToContent, ContentToBrowser>,
) {
    let (browser, content) = ipc::channel_pair::<BrowserToContent, ContentToBrowser>();
    let nh = std::rc::Rc::new(elidex_net::broker::NetworkHandle::disconnected());
    let jar = std::sync::Arc::new(elidex_net::CookieJar::new());
    let pipeline = crate::build_pipeline_interactive_with_network(
        html,
        css,
        nh,
        jar,
        elidex_plugin::Size::new(
            crate::DEFAULT_VIEWPORT_WIDTH,
            crate::DEFAULT_VIEWPORT_HEIGHT,
        ),
    );
    let mut state = ContentState::new(
        content,
        NavigationController::new(),
        pipeline,
        Box::new(|| {}),
    );
    scroll::update_viewport_scroll_dimensions(&mut state);
    iframe::scan_initial_iframes(&mut state);
    state.re_render();
    (state, browser)
}

/// F9: a `Shutdown` replayed after a blocking load sets `pending_shutdown` (its
/// `handle_message` return is consumed by the replay loop); `run_event_loop` must
/// honor that flag and exit. Otherwise `Tab::shutdown()` holds the channel sender
/// across `join()`, so there is no disconnect to fall back on and the join blocks
/// forever.
///
/// Build a state, set the flag, and run the loop on this thread (the pipeline is
/// `!Send`, so it cannot move to another thread). A watchdog keeps the channel
/// alive then force-sends `Shutdown` after 2s — so a regression (no flag check)
/// terminates and **fails** on elapsed time instead of hanging the suite, while
/// the fix returns essentially instantly.
#[test]
fn run_event_loop_honors_pending_shutdown_flag() {
    let (mut state, browser) = build_iframe_test_state("<div>x</div>", "");
    state.pending_shutdown = true;

    let watchdog = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(2));
        // If the flag was honored, the content side already exited and this send
        // hits a closed channel — harmless. If not, it breaks the otherwise-
        // infinite loop so the assert below fails cleanly rather than hanging.
        let _ = browser.send(BrowserToContent::Shutdown);
    });

    let start = std::time::Instant::now();
    event_loop::run_event_loop(&mut state);
    let elapsed = start.elapsed();
    let _ = watchdog.join();

    assert!(
        elapsed < Duration::from_secs(1),
        "run_event_loop did not honor pending_shutdown ({elapsed:?}); \
         Tab::shutdown()'s join() would deadlock"
    );
}

/// The single `<iframe>` entity in the parent DOM (the one carrying `IframeData`).
fn iframe_entity(state: &ContentState) -> elidex_ecs::Entity {
    (&mut state
        .pipeline
        .dom
        .world()
        .query::<(elidex_ecs::Entity, &elidex_ecs::IframeData)>())
        .into_iter()
        .next()
        .map(|(e, _)| e)
        .expect("a createElement('iframe') entity carrying IframeData should exist")
}

/// Whether `entity` is queued for lazy load.
fn is_lazy_pending(state: &ContentState, entity: elidex_ecs::Entity) -> bool {
    state.iframes.lazy_pending_iter().any(|&e| e == entity)
}

/// The `ChildList` record the mutation pipeline would emit for inserting
/// `child` under its (current) parent — `added_nodes = [child]`.
fn child_added_record(
    state: &ContentState,
    child: elidex_ecs::Entity,
) -> elidex_script_session::MutationRecord {
    let target = state.pipeline.dom.get_parent(child).unwrap_or(child);
    elidex_script_session::MutationRecord {
        kind: elidex_script_session::MutationKind::ChildList,
        target,
        added_nodes: vec![child],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: None,
        old_value: None,
    }
}

/// The `Attribute` record the mutation pipeline would emit for a `name`
/// attribute change on `target` (e.g. the `iframe.src` / `iframe.srcdoc` setter).
fn attribute_record(
    target: elidex_ecs::Entity,
    name: &str,
) -> elidex_script_session::MutationRecord {
    elidex_script_session::MutationRecord {
        kind: elidex_script_session::MutationKind::Attribute,
        target,
        added_nodes: vec![],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: Some(name.to_string()),
        old_value: None,
    }
}

#[test]
fn detached_iframe_with_srcdoc_set_loads_only_on_insertion() {
    let (mut state, _browser) = build_iframe_test_state("<body></body>", "");

    // `createElement('iframe')` + set `srcdoc` while detached. HTML §4.8.5 only
    // gives an iframe a content navigable once it is connected, so the
    // srcdoc-change record must be gated out (the `is_connected(target)` check
    // in the Attribute branch).
    let r = state.pipeline.eval_script(
        "globalThis.f = document.createElement('iframe'); f.srcdoc = '<p>detached</p>';",
    );
    assert!(r.success, "setup JS failed: {:?}", r.error);

    let f = iframe_entity(&state);
    assert!(
        !state.pipeline.dom.is_connected(f),
        "iframe should still be detached"
    );

    let changed = iframe::detect_iframe_mutations(&[attribute_record(f, "srcdoc")], &mut state);
    assert!(
        !changed,
        "a detached srcdoc change must not change iframe state"
    );
    assert!(
        state.iframes.is_empty(),
        "a detached iframe must not load when its srcdoc is set"
    );
    assert!(
        !state.iframes.has_lazy_pending(),
        "a detached iframe must not be queued for lazy load"
    );

    // Insert into the connected tree → the ChildList record now sees
    // `is_connected`, so the iframe loads.
    let r = state.pipeline.eval_script("document.body.appendChild(f);");
    assert!(r.success, "insert JS failed: {:?}", r.error);
    assert!(
        state.pipeline.dom.is_connected(f),
        "iframe should be connected after insertion"
    );

    let changed = iframe::detect_iframe_mutations(&[child_added_record(&state, f)], &mut state);
    assert!(changed, "connecting the iframe must load it");
    assert!(
        state.iframes.get(f).is_some(),
        "iframe must load once inserted into a connected tree"
    );
}

#[test]
fn iframe_under_detached_parent_loads_only_when_parent_connected() {
    let (mut state, _browser) = build_iframe_test_state("<body></body>", "");

    // Build `<div><iframe srcdoc></iframe></div>` with the div NOT inserted.
    // The ChildList record for `d.appendChild(f)` reaches detection, but the
    // iframe is not connected, so its subtree walk must be gated out.
    let r = state.pipeline.eval_script(
        "globalThis.d = document.createElement('div');\
         globalThis.f = document.createElement('iframe');\
         f.srcdoc = '<p>nested</p>';\
         d.appendChild(f);",
    );
    assert!(r.success, "setup JS failed: {:?}", r.error);

    let f = iframe_entity(&state);
    let d = state
        .pipeline
        .dom
        .get_parent(f)
        .expect("iframe should have its (detached) div parent");
    assert!(
        !state.pipeline.dom.is_connected(f),
        "iframe under a detached parent should not be connected"
    );

    let changed = iframe::detect_iframe_mutations(&[child_added_record(&state, f)], &mut state);
    assert!(!changed, "appending under a detached parent must not load");
    assert!(
        state.iframes.is_empty() && !state.iframes.has_lazy_pending(),
        "an iframe under a detached parent subtree must not load"
    );

    // Connect the parent subtree. The ChildList record for inserting `d` carries
    // the iframe in its subtree, which is now connected, so it loads.
    let r = state.pipeline.eval_script("document.body.appendChild(d);");
    assert!(r.success, "connect JS failed: {:?}", r.error);
    assert!(
        state.pipeline.dom.is_connected(f),
        "iframe should be connected after its ancestor is inserted"
    );

    let changed = iframe::detect_iframe_mutations(&[child_added_record(&state, d)], &mut state);
    assert!(
        changed,
        "connecting the ancestor must load the nested iframe"
    );
    assert!(
        state.iframes.get(f).is_some(),
        "iframe must load once its ancestor is connected"
    );
}

#[test]
fn lazy_iframe_srcdoc_change_while_offscreen_re_defers() {
    let (mut state, _browser) = build_iframe_test_state("<body></body>", "");

    // A connected `loading="lazy"` iframe laid out *below the viewport* (a
    // 5000px spacer pushes it past the 1024x768 default viewport + 200px lazy
    // margin), so `check_lazy_iframes` — the lazy-visibility pass the real
    // content loop runs right after detection (`content/mod.rs`) — must keep it
    // pending rather than load it. Exercising that pass (not just the detector)
    // is what makes the "while offscreen" condition real: a regression that
    // force-loads offscreen pending iframes would otherwise stay green.
    let r = state.pipeline.eval_script(
        "globalThis.spacer = document.createElement('div');\
         spacer.setAttribute('style', 'display:block;height:5000px');\
         document.body.appendChild(spacer);\
         globalThis.f = document.createElement('iframe');\
         f.loading = 'lazy';\
         f.setAttribute('style', 'display:block;width:100px;height:100px');\
         f.srcdoc = '<p>v1</p>';\
         document.body.appendChild(f);",
    );
    assert!(r.success, "setup JS failed: {:?}", r.error);

    let f = iframe_entity(&state);
    assert!(
        state.pipeline.dom.is_connected(f),
        "lazy iframe should be connected"
    );

    // Run layout (the free `re_render` flushes + lays out *without* the
    // `ContentState::re_render` wrapper's detect/lazy side-effects, keeping this
    // test in control of when those run). Then assert the iframe really is laid
    // out below the viewport + lazy margin — the precondition `check_lazy_iframes`
    // keys on — so the "offscreen" assertions below are not vacuous.
    let _ = crate::re_render(&mut state.pipeline);
    let frame_top = state
        .pipeline
        .dom
        .world()
        .get::<&elidex_plugin::LayoutBox>(f)
        .expect("the lazy iframe should have a layout box")
        .content
        .origin
        .y;
    assert!(
        frame_top > crate::DEFAULT_VIEWPORT_HEIGHT + 200.0,
        "iframe must be laid out below the viewport + lazy margin to test the offscreen path \
         (top {frame_top}, viewport {})",
        crate::DEFAULT_VIEWPORT_HEIGHT
    );

    // Connect → detector defers the lazy iframe to the pending queue
    // (`force=false` + `LoadingAttribute::Lazy`).
    let changed = iframe::detect_iframe_mutations(&[child_added_record(&state, f)], &mut state);
    assert!(changed, "connecting a lazy iframe registers it as pending");
    assert!(
        state.iframes.get(f).is_none(),
        "a lazy iframe must not load eagerly on connection"
    );
    assert!(
        is_lazy_pending(&state, f),
        "a lazy iframe must be queued as lazy-pending on connection"
    );

    // The lazy-visibility pass must NOT load the offscreen iframe — it stays
    // pending. This is the half PR #373's lazy contract depends on; running it
    // here (instead of only the detector) is what verifies the offscreen state.
    let loaded_offscreen = iframe::check_lazy_iframes(&mut state);
    assert!(
        !loaded_offscreen,
        "check_lazy_iframes must not load an iframe below the viewport"
    );
    assert!(
        is_lazy_pending(&state, f),
        "an offscreen lazy iframe must remain pending after the lazy-visibility pass"
    );

    // Change `srcdoc` while still offscreen → must RE-DEFER, not force-load:
    // PR #373 runs the attribute-change reload with `force=false`, so the lazy
    // iframe re-enters the lazy queue instead of loading eagerly.
    let r = state.pipeline.eval_script("f.srcdoc = '<p>v2</p>';");
    assert!(r.success, "srcdoc-change JS failed: {:?}", r.error);

    let srcdoc_changed =
        iframe::detect_iframe_mutations(&[attribute_record(f, "srcdoc")], &mut state);
    // Assert the srcdoc record was actually *processed* (the Attribute branch
    // ran: re-derive → remove-from-pending → re-defer). Without this the test
    // would pass vacuously — `f` is already lazy-pending, so the "still pending"
    // assertion alone holds even if the detector regressed to matching only
    // `src` and ignored the `srcdoc` record (precisely the PR #373 path this
    // test pins). `detect_iframe_mutations` only returns `true` when a matched
    // mutation drove a load/defer, so the `srcdoc`-ignored regression flips this
    // to `false`.
    assert!(
        srcdoc_changed,
        "a srcdoc attribute change on a connected iframe must be processed (PR #373 srcdoc reload path), not ignored"
    );
    assert!(
        state.iframes.get(f).is_none(),
        "a lazy iframe must not force-load on a srcdoc change while offscreen"
    );
    assert!(
        is_lazy_pending(&state, f),
        "a lazy iframe must remain lazy-pending after a srcdoc change"
    );

    // And the lazy-visibility pass still keeps it pending post-re-defer.
    let loaded_after_change = iframe::check_lazy_iframes(&mut state);
    assert!(
        !loaded_after_change,
        "check_lazy_iframes must still not load the offscreen iframe after the srcdoc re-defer"
    );
    assert!(
        is_lazy_pending(&state, f),
        "an offscreen lazy iframe must remain pending after a srcdoc re-defer + lazy-visibility pass"
    );
}

#[test]
fn connected_iframe_append_loads() {
    let (mut state, _browser) = build_iframe_test_state("<body></body>", "");

    // Baseline positive control: a normal eager iframe appended into the
    // connected body loads immediately.
    let r = state.pipeline.eval_script(
        "globalThis.f = document.createElement('iframe');\
         f.srcdoc = '<p>loaded</p>';\
         document.body.appendChild(f);",
    );
    assert!(r.success, "setup JS failed: {:?}", r.error);

    let f = iframe_entity(&state);
    assert!(
        state.pipeline.dom.is_connected(f),
        "appended iframe should be connected"
    );

    let changed = iframe::detect_iframe_mutations(&[child_added_record(&state, f)], &mut state);
    assert!(changed, "a connected eager iframe must load on insertion");
    assert_eq!(
        state.iframes.len(),
        1,
        "a connected eager iframe must load on insertion"
    );
    assert!(state.iframes.get(f).is_some());
}

/// Read the value of an attribute on the `<div>` with the given `id`.
fn probe_attr(pipeline: &crate::PipelineResult, id: &str, attr: &str) -> Option<String> {
    let entity = pipeline.dom.query_by_tag("div").into_iter().find(|&e| {
        pipeline
            .dom
            .world()
            .get::<&elidex_ecs::Attributes>(e)
            .ok()
            .is_some_and(|a| a.get("id") == Some(id))
    })?;
    pipeline
        .dom
        .world()
        .get::<&elidex_ecs::Attributes>(entity)
        .ok()
        .and_then(|a| a.get(attr).map(str::to_owned))
}

/// C1 (F1): a pipeline built at a non-default viewport must seed BOTH the CSS
/// cascade AND the JS bridge **before** initial scripts run, so an inline script
/// reading `window.innerWidth`/`innerHeight` at load observes the real size —
/// not the bridge's `800×600` default nor the cascade's `1024×768` default.
/// This is the path `content_thread_main` uses for the initial tab (C1).
#[test]
fn initial_scripts_observe_real_viewport() {
    let nh = std::rc::Rc::new(elidex_net::broker::NetworkHandle::disconnected());
    let jar = std::sync::Arc::new(elidex_net::CookieJar::new());
    let pipeline = crate::build_pipeline_interactive_with_network(
        "<div id=\"probe\"></div>\
         <script>\
           var p = document.getElementById('probe');\
           p.setAttribute('data-w', String(window.innerWidth));\
           p.setAttribute('data-h', String(window.innerHeight));\
         </script>",
        "",
        nh,
        jar,
        elidex_plugin::Size::new(640.0, 480.0),
    );

    // Construction input reached the cascade/layout SoT.
    assert_eq!(pipeline.viewport, elidex_plugin::Size::new(640.0, 480.0));
    // ...and the JS bridge SoT that `innerWidth`/`matchMedia` read.
    assert_eq!(pipeline.runtime.bridge().viewport_width(), 640.0);
    assert_eq!(pipeline.runtime.bridge().viewport_height(), 480.0);
    // The initial script OBSERVED the real size — proves the bridge was seeded
    // BEFORE the script-eval loop (the F1 ordering, not merely set post-build).
    assert_eq!(
        probe_attr(&pipeline, "probe", "data-w").as_deref(),
        Some("640")
    );
    assert_eq!(
        probe_attr(&pipeline, "probe", "data-h").as_deref(),
        Some("480")
    );
}

/// C1 (D6/F1): the single construction input retires the pre-C1 split where the
/// JS bridge defaulted to `800×600` while the cascade used `1024×768`. A
/// window-less build that passes `DEFAULT` explicitly now feeds both, so the
/// bridge and `PipelineResult.viewport` agree.
#[test]
fn default_viewport_unifies_bridge_and_cascade() {
    let nh = std::rc::Rc::new(elidex_net::broker::NetworkHandle::disconnected());
    let jar = std::sync::Arc::new(elidex_net::CookieJar::new());
    let pipeline = crate::build_pipeline_interactive_with_network(
        "<div></div>",
        "",
        nh,
        jar,
        elidex_plugin::Size::new(
            crate::DEFAULT_VIEWPORT_WIDTH,
            crate::DEFAULT_VIEWPORT_HEIGHT,
        ),
    );
    assert_eq!(
        pipeline.runtime.bridge().viewport_width(),
        crate::DEFAULT_VIEWPORT_WIDTH
    );
    assert_eq!(pipeline.viewport.width, crate::DEFAULT_VIEWPORT_WIDTH);
}
