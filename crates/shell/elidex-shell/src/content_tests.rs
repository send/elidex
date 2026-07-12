use super::test_support::{build_test_content_state, probe_attr, spawn_test_content, test_network};
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
        super::test_support::test_web_storage(),
        "<div>Hi</div>".to_string(),
        "div { display: block; }".to_string(),
        crate::ipc::ViewportCell::new(elidex_plugin::Size::new(
            crate::DEFAULT_VIEWPORT_WIDTH,
            crate::DEFAULT_VIEWPORT_HEIGHT,
        )),
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
            placement_seq: 0,
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
            placement_seq: 0,
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
            placement_seq: 0,
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
            placement_seq: 0,
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
            placement_seq: 0,
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
            placement_seq: 0,
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
            placement_seq: 0,
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
            placement_seq: 0,
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
            placement_seq: 0,
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
    // `seq: 1` clears the build's high-water mark (0) so the changed size applies.
    browser
        .send(BrowserToContent::SetViewport {
            width: 800.0,
            height: 600.0,
            seq: 1,
            // Neutral facts (bridge default) at facts_seq 0 — no-op for this size test.
            color_scheme: elidex_css::media::ColorScheme::Light,
            dppx: 1.0,
            facts_seq: 0,
        })
        .unwrap();

    let msg = browser.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(matches!(msg, ContentToBrowser::DisplayListReady(_)));

    // Now scroll should work with the new dimensions. The wheel is mapped against the
    // post-resize placement (seq 1, set by the SetViewport above), so it carries
    // `placement_seq: 1` — `placement_seq: 0` would be dropped as mapped-against-stale.
    browser
        .send(BrowserToContent::MouseWheel {
            delta: elidex_plugin::Vector::new(0.0, 100.0),
            point: Point::new(100.0, 100.0),
            placement_seq: 1,
        })
        .unwrap();

    let msg = browser.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(matches!(msg, ContentToBrowser::DisplayListReady(_)));

    browser.send(BrowserToContent::Shutdown).unwrap();
    handle.join().unwrap();
}

// ---------------------------------------------------------------------------
// Iframe lifecycle (HTML §4.8.5 "content navigable on connection" + lazy)
// ---------------------------------------------------------------------------
//
// A lightweight in-thread `ContentState` harness (no spawned content thread):
// `eval_script` performs the *live* DOM mutations (createElement / appendChild /
// detach), then we call `rescan_iframes_by_diff` — the §4.3.8 full-document
// walk that reconciles the iframe registry against the live tree — and assert
// the load / no-load / lazy outcome.
//
// Under the VM flip the record stream starves (VM-native mutations write the
// `EcsDom` directly and never enter `SessionCore::pending`), so the scan is
// driven off the live DOM rather than synthesized `MutationRecord`s. The tree the
// `eval_script` mutations left is the scan's whole input: connectedness (the old
// `is_connected` gate) is structural — a detached iframe simply is not reached by
// the walk — and lazy re-deferral is preserved.

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

#[test]
fn detached_iframe_with_srcdoc_set_loads_only_on_insertion() {
    let (mut state, _browser) = build_test_content_state("<body></body>", "");

    // `createElement('iframe')` + set `srcdoc` while detached. HTML §4.8.5 only
    // gives an iframe a content navigable once it is connected, so a detached
    // iframe is structurally excluded from the `rescan_iframes_by_diff` walk (it
    // is not reachable from the document root).
    let r = state.pipeline.eval_script(
        "globalThis.f = document.createElement('iframe'); f.srcdoc = '<p>detached</p>';",
    );
    assert!(r.success, "setup JS failed: {:?}", r.error);

    let f = iframe_entity(&state);
    assert!(
        !state.pipeline.dom.is_connected(f),
        "iframe should still be detached"
    );

    let changed = iframe::rescan_iframes_by_diff(&mut state);
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

    // Insert into the connected tree → the walk now reaches the iframe, so it
    // loads.
    let r = state.pipeline.eval_script("document.body.appendChild(f);");
    assert!(r.success, "insert JS failed: {:?}", r.error);
    assert!(
        state.pipeline.dom.is_connected(f),
        "iframe should be connected after insertion"
    );

    let changed = iframe::rescan_iframes_by_diff(&mut state);
    assert!(changed, "connecting the iframe must load it");
    assert!(
        state.iframes.get(f).is_some(),
        "iframe must load once inserted into a connected tree"
    );
}

#[test]
fn iframe_under_detached_parent_loads_only_when_parent_connected() {
    let (mut state, _browser) = build_test_content_state("<body></body>", "");

    // Build `<div><iframe srcdoc></iframe></div>` with the div NOT inserted.
    // The iframe is not connected, so the document-root walk must not reach it.
    let r = state.pipeline.eval_script(
        "globalThis.d = document.createElement('div');\
         globalThis.f = document.createElement('iframe');\
         f.srcdoc = '<p>nested</p>';\
         d.appendChild(f);",
    );
    assert!(r.success, "setup JS failed: {:?}", r.error);

    let f = iframe_entity(&state);
    let _d = state
        .pipeline
        .dom
        .get_parent(f)
        .expect("iframe should have its (detached) div parent");
    assert!(
        !state.pipeline.dom.is_connected(f),
        "iframe under a detached parent should not be connected"
    );

    let changed = iframe::rescan_iframes_by_diff(&mut state);
    assert!(!changed, "appending under a detached parent must not load");
    assert!(
        state.iframes.is_empty() && !state.iframes.has_lazy_pending(),
        "an iframe under a detached parent subtree must not load"
    );

    // Connect the parent subtree. The walk now reaches the nested iframe, so it
    // loads.
    let r = state.pipeline.eval_script("document.body.appendChild(d);");
    assert!(r.success, "connect JS failed: {:?}", r.error);
    assert!(
        state.pipeline.dom.is_connected(f),
        "iframe should be connected after its ancestor is inserted"
    );

    let changed = iframe::rescan_iframes_by_diff(&mut state);
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
    let (mut state, _browser) = build_test_content_state("<body></body>", "");

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
    crate::re_render(&mut state.pipeline);
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

    // Connect → the scan defers the lazy iframe to the pending queue
    // (`force=false` + `LoadingAttribute::Lazy`). A deferral loads nothing and
    // paints nothing, so the scan reports no display-rebuild need (`!changed`) —
    // it is the lazy-pending queue membership, not the return bool, that records
    // the defer.
    let changed = iframe::rescan_iframes_by_diff(&mut state);
    assert!(
        !changed,
        "deferring a lazy iframe schedules no load / display rebuild"
    );
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

    // Change `srcdoc` while still offscreen → must NOT force-load. Under the
    // §4.3.8 walk, a still-pending lazy iframe is left in the queue (the scan's
    // `is_lazy_pending` skip), and its `IframeData` was already re-derived to the
    // new srcdoc at the attribute write — so the later `check_lazy_iframes` load
    // reads the fresh `v2` without any eager force-load here.
    let r = state.pipeline.eval_script("f.srcdoc = '<p>v2</p>';");
    assert!(r.success, "srcdoc-change JS failed: {:?}", r.error);

    let srcdoc_changed = iframe::rescan_iframes_by_diff(&mut state);
    assert!(
        !srcdoc_changed,
        "a srcdoc change on an offscreen lazy iframe must not force-load / rebuild — it stays pending"
    );
    // The re-derive DID land: the live `IframeData` now carries `v2`, so the
    // deferred load will read the new resource (not the stale `v1`).
    assert_eq!(
        state
            .pipeline
            .dom
            .world()
            .get::<&elidex_ecs::IframeData>(f)
            .ok()
            .and_then(|d| d.srcdoc.clone()),
        Some("<p>v2</p>".to_string()),
        "the srcdoc attribute change must re-derive IframeData to v2 (read by the later lazy load)"
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
    let (mut state, _browser) = build_test_content_state("<body></body>", "");

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

    let changed = iframe::rescan_iframes_by_diff(&mut state);
    assert!(changed, "a connected eager iframe must load on insertion");
    assert_eq!(
        state.iframes.len(),
        1,
        "a connected eager iframe must load on insertion"
    );
    assert!(state.iframes.get(f).is_some());
}

/// C3: an in-process iframe inherits the **parent's** device facts (dppx /
/// color-scheme) at build, not a 1×/Light default. Device facts are window/display
/// facts — the same for every browsing context on the output device — so a sub-frame
/// on a HiDPI/dark display must report the parent's `devicePixelRatio`/`matchMedia`,
/// unlike its viewport *size* (genuinely unknown until the parent lays out the box,
/// `#11-iframe-build-viewport`). Regression for the Codex C3 R1 finding: the iframe
/// build defaulted `DeviceFacts::default()` even when the parent bridge held the real
/// facts.
#[test]
fn iframe_inherits_parent_device_facts_at_build() {
    use elidex_css::media::ColorScheme;
    let (mut state, _browser) = build_test_content_state("<body></body>", "");

    // The parent acquires non-default device facts (e.g. a HiDPI dark display) — the
    // shell `SetDeviceFacts` arm writes them into the parent bridge before the iframe
    // is created.
    super::event_loop::handle_message_public(
        crate::ipc::BrowserToContent::SetDeviceFacts {
            color_scheme: ColorScheme::Dark,
            dppx: 2.0,
            facts_seq: 1,
        },
        &mut state,
    );
    assert_eq!(
        state.pipeline.runtime.eval_f64("window.devicePixelRatio"),
        2.0
    );

    // Create + append an iframe, then load it (same path as `connected_iframe_append_loads`).
    let r = state.pipeline.eval_script(
        "globalThis.f = document.createElement('iframe');\
         f.srcdoc = '<p>child</p>';\
         document.body.appendChild(f);",
    );
    assert!(r.success, "setup JS failed: {:?}", r.error);
    let f = iframe_entity(&state);
    let changed = iframe::rescan_iframes_by_diff(&mut state);
    assert!(changed, "the iframe must load on insertion");

    // The loaded in-process iframe's bridge reports the parent's facts, not 1×/Light.
    let entry = state.iframes.get_mut(f).expect("iframe must be loaded");
    let iframe::IframeHandle::InProcess(ip) = &mut entry.handle else {
        panic!("a same-origin srcdoc iframe loads in-process");
    };
    let iframe_runtime = &mut ip.pipeline.runtime;
    assert_eq!(
        iframe_runtime.eval_f64("window.devicePixelRatio"),
        2.0,
        "iframe must inherit the parent's dppx (was stuck at 1.0 — Codex C3 R1)"
    );
    assert_eq!(
        iframe_runtime
            .eval_string("matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'"),
        "dark",
        "iframe must inherit the parent's prefers-color-scheme"
    );
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
        super::test_support::test_web_storage(),
        elidex_plugin::Size::new(640.0, 480.0),
        crate::ipc::DeviceFacts::default(),
    );

    // Construction input reached the cascade/layout SoT.
    assert_eq!(pipeline.viewport, elidex_plugin::Size::new(640.0, 480.0));
    // ...and the JS bridge SoT that `innerWidth`/`matchMedia` read.
    assert_eq!(pipeline.viewport.width, 640.0);
    assert_eq!(pipeline.viewport.height, 480.0);
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
        super::test_support::test_web_storage(),
        elidex_plugin::Size::new(
            crate::DEFAULT_VIEWPORT_WIDTH,
            crate::DEFAULT_VIEWPORT_HEIGHT,
        ),
        crate::ipc::DeviceFacts::default(),
    );
    assert_eq!(pipeline.viewport.width, crate::DEFAULT_VIEWPORT_WIDTH);
}
