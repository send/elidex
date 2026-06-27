//! Viewport / content-area placement coverage for the content thread — the
//! producer→consumer `SetViewport` path (`@media` flip, `resize`-listener MQL
//! freshness) and the single-builder geometry invariant. Split out of
//! `content_tests` (the catch-all content module) to keep each test file focused
//! and under the project's 1000-line guideline (axes.md Axis 5).

use super::test_support::{spawn_test_content, spawn_test_content_sized, test_network};
use crate::ipc::{self, BrowserToContent, ContentToBrowser};
use std::time::Duration;

/// A `SetViewport` that crosses an `@media (max-width: 900px)` boundary flips
/// the cascade so the div recolors — the carry-forward content test exercising
/// the producer→consumer viewport path end-to-end (the reordered `SetViewport`
/// consumer arm still re-evaluates media queries + restyles).
#[test]
fn content_thread_setviewport_flips_width_media_query() {
    let (browser, content) = ipc::channel_pair::<BrowserToContent, ContentToBrowser>();
    let (nh, jar, _np) = test_network();
    let handle = spawn_test_content(
        content,
        nh,
        jar,
        "<div id=\"box\">Box</div>".to_string(),
        "div { display: block; width: 100px; height: 100px; background-color: blue; }\
         @media (max-width: 900px) { div { background-color: red; } }"
            .to_string(),
    );

    let has_red = |dl: &elidex_render::DisplayList| {
        dl.iter().any(|item| {
            matches!(
                item,
                elidex_render::DisplayItem::SolidRect { color, .. }
                    if *color == elidex_plugin::CssColor::rgb(255, 0, 0)
            )
        })
    };

    // Initial frame at the 1024px default viewport → @media does not match → blue.
    let ContentToBrowser::DisplayListReady(initial) =
        browser.recv_timeout(Duration::from_secs(5)).unwrap()
    else {
        panic!("expected initial DisplayListReady");
    };
    assert!(
        !has_red(&initial),
        "default 1024px viewport must NOT match @media (max-width: 900px)"
    );

    // Resize to 800px wide → @media (max-width: 900px) now matches → red. `seq: 1`
    // clears the build's high-water mark (0).
    browser
        .send(BrowserToContent::SetViewport {
            width: 800.0,
            height: 600.0,
            seq: 1,
        })
        .unwrap();
    let ContentToBrowser::DisplayListReady(resized) =
        browser.recv_timeout(Duration::from_secs(5)).unwrap()
    else {
        panic!("expected post-resize DisplayListReady");
    };
    assert!(
        has_red(&resized),
        "800px viewport must match @media (max-width: 900px) → red div"
    );

    browser.send(BrowserToContent::Shutdown).unwrap();
    handle.join().unwrap();
}

/// C1 — a content thread spawned at a non-default viewport lays out its **first**
/// frame at that size, *before* any `SetViewport` arrives (the construction-input
/// "born at the real size" invariant: spawn → `content_thread_main` builds the
/// cascade/layout at the passed viewport). At 640px wide the
/// `@media (max-width: 900px)` already matches, so the initial `DisplayListReady`
/// is red without any resize — pre-C1 (build at the 1024px default) it would be
/// blue.
#[test]
fn content_thread_first_frame_at_spawn_viewport() {
    let (browser, content) = ipc::channel_pair::<BrowserToContent, ContentToBrowser>();
    let (nh, jar, _np) = test_network();
    let handle = crate::content::spawn_content_thread(
        content,
        nh,
        jar,
        "<div id=\"box\">Box</div>".to_string(),
        "div { display: block; width: 100px; height: 100px; background-color: blue; }\
         @media (max-width: 900px) { div { background-color: red; } }"
            .to_string(),
        crate::ipc::ViewportCell::new(elidex_plugin::Size::new(640.0, 480.0)),
        Box::new(|| {}),
    );

    let has_red = |dl: &elidex_render::DisplayList| {
        dl.iter().any(|item| {
            matches!(
                item,
                elidex_render::DisplayItem::SolidRect { color, .. }
                    if *color == elidex_plugin::CssColor::rgb(255, 0, 0)
            )
        })
    };

    let ContentToBrowser::DisplayListReady(initial) =
        browser.recv_timeout(Duration::from_secs(5)).unwrap()
    else {
        panic!("expected initial DisplayListReady");
    };
    assert!(
        has_red(&initial),
        "C1: first frame must lay out at the spawn viewport (640px), matching \
         @media (max-width: 900px) — without any SetViewport"
    );

    browser.send(BrowserToContent::Shutdown).unwrap();
    handle.join().unwrap();
}

/// A `resize` listener that reads a pre-existing `MediaQueryList`'s `matches`
/// must see the **post-resize** value (CSSOM View: the `matches` getter returns
/// the *current* matches state). The `SetViewport` consumer refreshes the MQL
/// cache **before** dispatching `resize`, so `mql.matches` read inside the
/// listener is already current; the MQL `change` *events* still fire after
/// `resize` (§8.1.7.3 step 8 < step 10). Regression for the
/// cache-refresh-before-resize ordering.
///
/// The query is `(min-width: 900px)` and the resize grows the viewport across
/// that boundary: the content thread is spawned at an explicit **800 px**
/// viewport (C1 seeds the JS bridge from the spawn viewport in
/// `run_scripts_and_finalize`, so the DEFAULT 1024 px spawn would start
/// `mql.matches` already `true` and erase the threshold crossing), so at script
/// time `mql.matches` is **false** (`800 < 900`). Resizing to 1000 px must flip
/// the cached state to **true** before the listener runs. The listener recolors
/// the box from that read alone
/// (no `@media` cascade, no MQL `change` event) — `red` ⇒ the listener saw the
/// fresh `true`; `lime` ⇒ it ran but read the **stale** cached `false` (the bug);
/// `blue` ⇒ the listener never ran. (A `max-width` query the other way is
/// already `true` at the 800 px default and cannot detect staleness.)
#[test]
fn content_thread_resize_listener_sees_fresh_matchmedia() {
    let (browser, content) = ipc::channel_pair::<BrowserToContent, ContentToBrowser>();
    let (nh, jar, _np) = test_network();
    // Spawn below the 900 px threshold so the resize to 1000 px crosses it (C1
    // seeds the bridge from the spawn viewport, so the DEFAULT 1024 px would
    // start `mql.matches` true and defeat this regression).
    let handle = spawn_test_content_sized(
        content,
        nh,
        jar,
        "<div id=\"box\">Box</div>\
         <script>\
           var mql = matchMedia('(min-width: 900px)');\
           window.addEventListener('resize', function() {\
             if (mql.matches) {\
               document.getElementById('box').style.setProperty('background-color', 'red');\
             } else {\
               document.getElementById('box').style.setProperty('background-color', 'lime');\
             }\
           });\
         </script>"
            .to_string(),
        "div { display: block; width: 100px; height: 100px; background-color: blue; }".to_string(),
        elidex_plugin::Size::new(800.0, 600.0),
    );

    let has_color = |dl: &elidex_render::DisplayList, c: elidex_plugin::CssColor| {
        dl.iter().any(|item| {
            matches!(item, elidex_render::DisplayItem::SolidRect { color, .. } if *color == c)
        })
    };
    let red = elidex_plugin::CssColor::rgb(255, 0, 0);
    let lime = elidex_plugin::CssColor::rgb(0, 255, 0);

    // Initial frame: `min-width: 900px` is false at the 800 px spawn viewport and
    // the listener has not run → box is its blue default (neither red nor lime).
    let ContentToBrowser::DisplayListReady(initial) =
        browser.recv_timeout(Duration::from_secs(5)).unwrap()
    else {
        panic!("expected initial DisplayListReady");
    };
    assert!(
        !has_color(&initial, red) && !has_color(&initial, lime),
        "box must start blue (listener not yet run)"
    );

    // Grow to 1000 px (crosses min-width: 900): during the resize listener
    // `mql.matches` must already be true (cache refreshed before dispatch) → red.
    // `seq: 1` clears the build's high-water mark (0).
    browser
        .send(BrowserToContent::SetViewport {
            width: 1000.0,
            height: 600.0,
            seq: 1,
        })
        .unwrap();
    let ContentToBrowser::DisplayListReady(resized) =
        browser.recv_timeout(Duration::from_secs(5)).unwrap()
    else {
        panic!("expected post-resize DisplayListReady");
    };
    assert!(
        has_color(&resized, red),
        "resize listener read stale mql.matches (cache not refreshed before resize dispatch): \
         box is {}",
        if has_color(&resized, lime) {
            "lime (stale false)"
        } else {
            "blue (listener did not run)"
        }
    );

    browser.send(BrowserToContent::Shutdown).unwrap();
    handle.join().unwrap();
}

/// Idempotent viewport delivery (CSSOM View §13.1 "run the resize steps"
/// `#document-run-the-resize-steps` step 1): a `SetViewport` whose size equals the
/// content thread's current viewport must **not** fire `resize`, re-evaluate media
/// queries, or repaint. This is the invariant that lets C1's `broadcast_viewport`
/// fan the cached size to every tab on `resumed` unconditionally — the
/// just-spawned initial tab, already born at that size, drops the redundant
/// delivery instead of dispatching a spurious `resize`/double-painting.
///
/// A `resize` listener paints the box red. After the initial frame (blue) we send
/// a **same-size** `SetViewport` (must no-op) followed by a `VisibilityChanged`
/// fence (always produces a frame): a blue fenced frame ⇒ the same-size delivery
/// did not fire `resize`. A later **changed-size** `SetViewport` then flips the
/// box red, proving genuine resizes still fire.
#[test]
fn content_thread_same_size_setviewport_is_idempotent() {
    let (browser, content) = ipc::channel_pair::<BrowserToContent, ContentToBrowser>();
    let (nh, jar, _np) = test_network();
    let handle = spawn_test_content_sized(
        content,
        nh,
        jar,
        "<div id=\"box\">Box</div>\
         <script>\
           window.addEventListener('resize', function() {\
             document.getElementById('box').style.setProperty('background-color', 'red');\
           });\
         </script>"
            .to_string(),
        "div { display: block; width: 100px; height: 100px; background-color: blue; }".to_string(),
        elidex_plugin::Size::new(800.0, 600.0),
    );

    let has_color = |dl: &elidex_render::DisplayList, c: elidex_plugin::CssColor| {
        dl.iter().any(|item| {
            matches!(item, elidex_render::DisplayItem::SolidRect { color, .. } if *color == c)
        })
    };
    let red = elidex_plugin::CssColor::rgb(255, 0, 0);

    // Initial frame: the listener has not run → box is its blue default.
    let ContentToBrowser::DisplayListReady(initial) =
        browser.recv_timeout(Duration::from_secs(5)).unwrap()
    else {
        panic!("expected initial DisplayListReady");
    };
    assert!(
        !has_color(&initial, red),
        "box must start blue (listener not yet run)"
    );

    // Same-size `SetViewport` must NOT fire `resize` (§13.1) — a fresh `seq: 1`
    // (> the build's mark 0) clears the *staleness* guard, so it is the *value*
    // guard under test that drops it. The following `VisibilityChanged` is a fence
    // that always produces a frame; processed in order, it follows the no-op
    // SetViewport, so the first frame received is still blue. Had the same-size
    // delivery fired `resize`, its (red) frame would be received first instead.
    browser
        .send(BrowserToContent::SetViewport {
            width: 800.0,
            height: 600.0,
            seq: 1,
        })
        .unwrap();
    browser
        .send(BrowserToContent::VisibilityChanged { visible: true })
        .unwrap();
    let ContentToBrowser::DisplayListReady(fenced) =
        browser.recv_timeout(Duration::from_secs(5)).unwrap()
    else {
        panic!("expected fenced DisplayListReady");
    };
    assert!(
        !has_color(&fenced, red),
        "same-size SetViewport fired a spurious resize (box went red before any size change)"
    );

    // A genuine size change still fires `resize` → box red. `seq: 2` (> the prior
    // delivery's 1) clears the staleness guard so the changed size applies.
    browser
        .send(BrowserToContent::SetViewport {
            width: 1000.0,
            height: 600.0,
            seq: 2,
        })
        .unwrap();
    let ContentToBrowser::DisplayListReady(resized) =
        browser.recv_timeout(Duration::from_secs(5)).unwrap()
    else {
        panic!("expected post-resize DisplayListReady");
    };
    assert!(
        has_color(&resized, red),
        "changed-size SetViewport must fire resize (box stayed blue — resize not dispatched)"
    );

    browser.send(BrowserToContent::Shutdown).unwrap();
    handle.join().unwrap();
}

/// The build reads the **latest published** cell value, not the seed (the
/// pull-source invariant, plan-memo §2.1). Construct a cell seeded at 1024 px
/// (`@media (max-width: 900px)` does *not* match → blue), then `publish_if_changed` 640 px
/// (matches → red) **before** spawning. The first frame is red ⇒ the build read the
/// published 640 px from the cell; a stale-snapshot build at the 1024 px seed would
/// be blue. This is the deterministic stand-in for "a resize landed during the
/// blocking load" — the publish-before-read ordering is forced on the test thread.
#[test]
fn content_thread_builds_at_latest_published_cell_size() {
    let (browser, content) = ipc::channel_pair::<BrowserToContent, ContentToBrowser>();
    let (nh, jar, _np) = test_network();

    // Seed at 1024 px (no @media match), then publish 640 px (matches) before spawn.
    // 640 ≠ 1024 → `publish_if_changed` bumps to seq 1.
    let cell = crate::ipc::ViewportCell::new(elidex_plugin::Size::new(1024.0, 768.0));
    assert!(cell.publish_if_changed(elidex_plugin::Size::new(640.0, 480.0)));

    let handle = crate::content::spawn_content_thread(
        content,
        nh,
        jar,
        "<div id=\"box\">Box</div>".to_string(),
        "div { display: block; width: 100px; height: 100px; background-color: blue; }\
         @media (max-width: 900px) { div { background-color: red; } }"
            .to_string(),
        cell,
        Box::new(|| {}),
    );

    let has_red = |dl: &elidex_render::DisplayList| {
        dl.iter().any(|item| {
            matches!(
                item,
                elidex_render::DisplayItem::SolidRect { color, .. }
                    if *color == elidex_plugin::CssColor::rgb(255, 0, 0)
            )
        })
    };

    let ContentToBrowser::DisplayListReady(initial) =
        browser.recv_timeout(Duration::from_secs(5)).unwrap()
    else {
        panic!("expected initial DisplayListReady");
    };
    assert!(
        has_red(&initial),
        "build must read the LATEST published cell size (640px → red), not the 1024px seed (blue)"
    );

    browser.send(BrowserToContent::Shutdown).unwrap();
    handle.join().unwrap();
}

/// A `SetViewport` whose `seq` is `≤` the seq the document **built** at is dropped
/// as stale — the cell-read build already absorbed that resize, so re-applying it
/// would flash the document backward (plan-memo §2.3 staleness guard). Publish once
/// (cell seq 1) before spawning, so the build's high-water mark is 1. A
/// `SetViewport(changed size, seq: 1)` — the canonical resume-time re-delivery of
/// the seq the build consumed — must NOT fire `resize`, even though the *size*
/// differs. A `VisibilityChanged` fence (always a frame) then proves the dropped
/// delivery left the box blue; a later `seq: 2` proves genuine newer resizes apply.
#[test]
fn content_thread_drops_stale_seq_viewport() {
    let (browser, content) = ipc::channel_pair::<BrowserToContent, ContentToBrowser>();
    let (nh, jar, _np) = test_network();

    // Reach cell (800px, seq 1) via a *real* size change: seed at 640px, then
    // `publish_if_changed(800px)`. A same-size publish no longer bumps seq (C2), so the
    // pre-C2 seed-then-republish-same-size trick would leave seq 0; the build must read
    // 800px at seq 1 for the stale-seq drop asserted below.
    let cell = crate::ipc::ViewportCell::new(elidex_plugin::Size::new(640.0, 480.0));
    assert!(cell.publish_if_changed(elidex_plugin::Size::new(800.0, 600.0)));

    let handle = crate::content::spawn_content_thread(
        content,
        nh,
        jar,
        "<div id=\"box\">Box</div>\
         <script>\
           window.addEventListener('resize', function() {\
             document.getElementById('box').style.setProperty('background-color', 'red');\
           });\
         </script>"
            .to_string(),
        "div { display: block; width: 100px; height: 100px; background-color: blue; }".to_string(),
        cell,
        Box::new(|| {}),
    );

    let has_red = |dl: &elidex_render::DisplayList| {
        dl.iter().any(|item| {
            matches!(
                item,
                elidex_render::DisplayItem::SolidRect { color, .. }
                    if *color == elidex_plugin::CssColor::rgb(255, 0, 0)
            )
        })
    };

    // Initial frame: listener not yet run → blue.
    let ContentToBrowser::DisplayListReady(initial) =
        browser.recv_timeout(Duration::from_secs(5)).unwrap()
    else {
        panic!("expected initial DisplayListReady");
    };
    assert!(
        !has_red(&initial),
        "box must start blue (listener not yet run)"
    );

    // Stale `seq: 1` (== the build mark) with a CHANGED size must be dropped — no
    // resize. The `VisibilityChanged` fence follows in FIFO order and always frames;
    // a blue fenced frame ⇒ the stale delivery fired no `resize`.
    browser
        .send(BrowserToContent::SetViewport {
            width: 1200.0,
            height: 600.0,
            seq: 1,
        })
        .unwrap();
    browser
        .send(BrowserToContent::VisibilityChanged { visible: true })
        .unwrap();
    let ContentToBrowser::DisplayListReady(fenced) =
        browser.recv_timeout(Duration::from_secs(5)).unwrap()
    else {
        panic!("expected fenced DisplayListReady");
    };
    assert!(
        !has_red(&fenced),
        "stale-seq SetViewport (seq ≤ build mark) fired a spurious resize / backward flash"
    );

    // A genuinely newer `seq: 2` applies → red, proving the thread still resizes.
    browser
        .send(BrowserToContent::SetViewport {
            width: 1200.0,
            height: 600.0,
            seq: 2,
        })
        .unwrap();
    let ContentToBrowser::DisplayListReady(resized) =
        browser.recv_timeout(Duration::from_secs(5)).unwrap()
    else {
        panic!("expected post-resize DisplayListReady");
    };
    assert!(
        has_red(&resized),
        "fresh seq (> build mark) must apply the resize (box stayed blue)"
    );

    browser.send(BrowserToContent::Shutdown).unwrap();
    handle.join().unwrap();
}

/// Coordinate-bearing input mapped against a placement the seq guard has
/// **superseded** is dropped, not hit-tested against the current layout (Codex R2 /
/// plan-memo §10 — the input half of the `ViewportCell` seq reconciliation). A resize
/// during a blocked load can leave a queued click whose coordinates were mapped
/// against a placement the build dropped; hit-testing it against the build layout
/// would target the wrong element, so it is dropped. Here: advance the high-water mark
/// with `SetViewport(seq 1)`, then a `placement_seq: 0` click (mapped against the
/// superseded placement) must fire no click handler, while a fresh `placement_seq: 1`
/// click is processed.
#[test]
fn content_thread_drops_input_mapped_against_stale_placement() {
    let (browser, content) = ipc::channel_pair::<BrowserToContent, ContentToBrowser>();
    let (nh, jar, _np) = test_network();
    let handle = spawn_test_content_sized(
        content,
        nh,
        jar,
        "<div id=\"box\" style=\"width:100px;height:100px;background-color:blue\"></div>\
         <script>\
           document.getElementById('box').addEventListener('click', function() {\
             document.getElementById('box').style.setProperty('background-color', 'red');\
           });\
         </script>"
            .to_string(),
        "div { display: block; }".to_string(),
        elidex_plugin::Size::new(800.0, 600.0),
    );

    let has_red = |dl: &elidex_render::DisplayList| {
        dl.iter().any(|item| {
            matches!(item, elidex_render::DisplayItem::SolidRect { color, .. }
                if *color == elidex_plugin::CssColor::rgb(255, 0, 0))
        })
    };
    let click_at = |placement_seq: u64| {
        BrowserToContent::MouseClick(crate::ipc::MouseClickEvent {
            point: elidex_plugin::Point::new(50.0, 50.0),
            client_point: elidex_plugin::Point::new(50.0, 50.0),
            button: 0,
            mods: crate::ipc::ModifierState::default(),
            placement_seq,
        })
    };

    // Initial frame: blue (no click yet).
    let ContentToBrowser::DisplayListReady(initial) =
        browser.recv_timeout(Duration::from_secs(5)).unwrap()
    else {
        panic!("expected initial DisplayListReady");
    };
    assert!(!has_red(&initial), "box must start blue");

    // Advance the high-water mark to seq 1 (a genuine resize).
    browser
        .send(BrowserToContent::SetViewport {
            width: 1000.0,
            height: 600.0,
            seq: 1,
        })
        .unwrap();
    let _resize_frame = browser.recv_timeout(Duration::from_secs(5)).unwrap();

    // Stale click (placement_seq 0 < applied 1) must be DROPPED → click handler never
    // runs. The `VisibilityChanged` fence always frames; a blue fenced frame ⇒ dropped.
    browser.send(click_at(0)).unwrap();
    browser
        .send(BrowserToContent::VisibilityChanged { visible: true })
        .unwrap();
    let ContentToBrowser::DisplayListReady(fenced) =
        browser.recv_timeout(Duration::from_secs(5)).unwrap()
    else {
        panic!("expected fenced DisplayListReady");
    };
    assert!(
        !has_red(&fenced),
        "stale-placement click fired the handler (should be dropped: placement_seq < applied)"
    );

    // Fresh click (placement_seq 1 == applied 1, not stale) is processed → box red.
    browser.send(click_at(1)).unwrap();
    let ContentToBrowser::DisplayListReady(clicked) =
        browser.recv_timeout(Duration::from_secs(5)).unwrap()
    else {
        panic!("expected post-click DisplayListReady");
    };
    assert!(
        has_red(&clicked),
        "fresh-placement click must be processed (placement_seq == applied) → red"
    );

    browser.send(BrowserToContent::Shutdown).unwrap();
    handle.join().unwrap();
}

/// Grep-guard for the single-builder invariant (F1, §2.3): the content-area
/// geometry primitives each have exactly one **production** caller — the
/// placement builder `App::content_area_placement` — so the cached `placement`
/// is the sole source of content-area size/origin/scale. `window.scale_factor()`
/// has two (the builder + egui's own render-init DPI read, the one documented
/// exception). A second caller of any (a re-computation instead of reading
/// `self.placement`) is the #716 strangler restated and fails here.
#[test]
fn placement_builder_is_sole_caller_of_geometry_primitives() {
    use std::path::Path;

    fn scan(dir: &Path, out: &mut Vec<(String, String)>) {
        for entry in std::fs::read_dir(dir).expect("read src dir") {
            let p = entry.expect("dir entry").path();
            if p.is_dir() {
                scan(&p, out);
                continue;
            }
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            // Skip non-Rust + test files (this file's needle string-literals
            // must not self-count; in-crate `#[cfg(test)]` chrome.rs uses the
            // bare unqualified form, which the `chrome::` needles already miss).
            if p.extension().and_then(|e| e.to_str()) != Some("rs") || name.contains("test") {
                continue;
            }
            for (i, line) in std::fs::read_to_string(&p)
                .expect("read source")
                .lines()
                .enumerate()
            {
                let t = line.trim_start();
                if t.starts_with("//") {
                    continue; // skip line/doc comments (prose mentions in backticks)
                }
                out.push((format!("{}:{}", p.display(), i + 1), t.to_string()));
            }
        }
    }

    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut lines = Vec::new();
    scan(&src, &mut lines);
    let callers = |needle: &str| -> Vec<String> {
        lines
            .iter()
            .filter(|(_, t)| t.contains(needle))
            .map(|(loc, t)| format!("{loc}: {t}"))
            .collect()
    };

    let offset = callers("chrome::chrome_content_offset(");
    assert_eq!(
        offset.len(),
        1,
        "chrome_content_offset must have exactly one production caller (the \
         placement builder); found: {offset:#?}"
    );
    let size = callers("chrome::content_size(");
    assert_eq!(
        size.len(),
        1,
        "content_size must have exactly one production caller (the placement \
         builder); found: {size:#?}"
    );
    let scale = callers("window.scale_factor()");
    assert_eq!(
        scale.len(),
        2,
        "window.scale_factor() must have exactly two production callers (the \
         placement builder + the egui render-init exception); found: {scale:#?}"
    );
}

/// `ViewportCell::publish_if_changed` bumps the seq **iff** the size differs (C2): the
/// seq identifies `size_logical` generations, so a same-size publish — which a pure
/// DPI/scale `Resized` produces (CSS px is scale-invariant) — must not advance it. This
/// pins the **producer-side** invariant whose violation was the §2 bug: a phantom seq
/// generation that lets `input_placement_stale` drop queued input mapped against the
/// still-current layout (the downstream input-survival is covered by
/// `content_thread_drops_stale_seq_viewport`). The returned `bool` is the gate the
/// producer uses to skip `broadcast_viewport`, so a no-op publish emits no `SetViewport`.
/// Pure cell-level guard — no window needed.
#[test]
fn publish_if_changed_bumps_seq_only_on_size_change() {
    let cell = ipc::ViewportCell::new(elidex_plugin::Size::new(800.0, 600.0));
    assert_eq!(cell.read(), (elidex_plugin::Size::new(800.0, 600.0), 0));

    // Same size as the seed → no change, no bump; gate returns false (broadcast skipped).
    assert!(!cell.publish_if_changed(elidex_plugin::Size::new(800.0, 600.0)));
    assert_eq!(cell.read().1, 0, "same-size publish must not advance seq");

    // A real size change → bump to seq 1; gate returns true (broadcast fires).
    assert!(cell.publish_if_changed(elidex_plugin::Size::new(1024.0, 768.0)));
    assert_eq!(cell.read(), (elidex_plugin::Size::new(1024.0, 768.0), 1));

    // Republishing the now-current size is again a no-op — seq stays 1 (idempotent).
    assert!(!cell.publish_if_changed(elidex_plugin::Size::new(1024.0, 768.0)));
    assert!(!cell.publish_if_changed(elidex_plugin::Size::new(1024.0, 768.0)));
    assert_eq!(
        cell.read().1,
        1,
        "repeated same-size publishes must not advance seq"
    );

    // Alternating sizes bump on each genuine change — assert relative to the current
    // seq so the claim ("these 2 changes advance by exactly 2") stays local and survives
    // edits to the publishes above.
    let before = cell.read().1;
    assert!(cell.publish_if_changed(elidex_plugin::Size::new(640.0, 480.0)));
    assert!(cell.publish_if_changed(elidex_plugin::Size::new(1024.0, 768.0)));
    assert_eq!(
        cell.read().1,
        before + 2,
        "each genuine size change advances seq by exactly one"
    );
}
