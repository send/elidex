use super::*;
use crate::ipc::{self, BrowserToContent, ContentToBrowser, ModifierState};
use elidex_plugin::Point;
use std::time::Duration;

/// Create a `NetworkHandle` + `CookieJar` backed by a test broker.
/// Returns the `NetworkProcessHandle` so the caller keeps the broker alive.
fn test_network() -> (
    elidex_net::broker::NetworkHandle,
    std::sync::Arc<elidex_net::CookieJar>,
    elidex_net::broker::NetworkProcessHandle,
) {
    let np = elidex_net::broker::spawn_network_process(elidex_net::NetClient::new());
    let nh = np.create_renderer_handle();
    let jar = std::sync::Arc::clone(np.cookie_jar());
    (nh, jar, np)
}

#[test]
fn content_thread_startup_and_shutdown() {
    let (browser, content) = ipc::channel_pair::<BrowserToContent, ContentToBrowser>();
    let (nh, jar, _np) = test_network();
    let handle = spawn_content_thread(
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
    let handle = spawn_content_thread(
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
    let handle = spawn_content_thread(
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
    let handle = spawn_content_thread(
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
    let handle = spawn_content_thread(
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
    let handle = spawn_content_thread(
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
    let handle = spawn_content_thread(
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
    let handle = spawn_content_thread(
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
    let handle = spawn_content_thread(
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
    let handle = spawn_content_thread(
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
    let handle = spawn_content_thread(
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
    let pipeline = crate::build_pipeline_interactive_with_network(html, css, nh, jar);
    let mut state = ContentState::new(content, NavigationController::new(), pipeline);
    scroll::update_viewport_scroll_dimensions(&mut state);
    iframe::scan_initial_iframes(&mut state);
    state.re_render();
    (state, browser)
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

    // A connected `loading="lazy"` iframe. On connection it is deferred to the
    // lazy-pending queue (`try_load_iframe_entity` with `force=false` sees
    // `LoadingAttribute::Lazy`) rather than loaded.
    let r = state.pipeline.eval_script(
        "globalThis.f = document.createElement('iframe');\
         f.loading = 'lazy';\
         f.srcdoc = '<p>v1</p>';\
         document.body.appendChild(f);",
    );
    assert!(r.success, "setup JS failed: {:?}", r.error);

    let f = iframe_entity(&state);
    assert!(
        state.pipeline.dom.is_connected(f),
        "lazy iframe should be connected"
    );

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

    // Change `srcdoc` while still offscreen (still only lazy-pending, never
    // scrolled into view) → must RE-DEFER, not force-load: PR #373 runs the
    // attribute-change reload with `force=false`, so the lazy iframe re-enters
    // the lazy queue instead of loading eagerly.
    let r = state.pipeline.eval_script("f.srcdoc = '<p>v2</p>';");
    assert!(r.success, "srcdoc-change JS failed: {:?}", r.error);

    let srcdoc_changed =
        iframe::detect_iframe_mutations(&[attribute_record(f, "srcdoc")], &mut state);
    // Assert the srcdoc record was actually *processed* (the Attribute branch
    // ran: re-derive → remove-from-pending → re-defer). Without this the test
    // would pass vacuously — `f` is already lazy-pending from the insertion
    // above, so the "still pending" assertion alone holds even if the detector
    // regressed to matching only `src` and ignored the `srcdoc` record
    // (precisely the PR #373 path this test pins). `detect_iframe_mutations`
    // only returns `true` when a matched mutation drove a load/defer, so the
    // `srcdoc`-ignored regression flips this to `false`.
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
