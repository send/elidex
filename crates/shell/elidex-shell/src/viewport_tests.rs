//! Viewport / content-area placement coverage for the content thread — the
//! producer→consumer `SetViewport` path (`@media` flip, `resize`-listener MQL
//! freshness) and the single-builder geometry invariant. Split out of
//! `content_tests` (the catch-all content module) to keep each test file focused
//! and under the project's 1000-line guideline (axes.md Axis 5).

use super::test_support::{spawn_test_content, test_network};
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

    // Resize to 800px wide → @media (max-width: 900px) now matches → red.
    browser
        .send(BrowserToContent::SetViewport {
            width: 800.0,
            height: 600.0,
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
        elidex_plugin::Size::new(640.0, 480.0),
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
/// that boundary: the boa `HostBridge` viewport defaults to 800 px (`bridge/mod`
/// `viewport_width: 800.0`), so at script time `mql.matches` is **false**
/// (`800 < 900`). Resizing to 1000 px must flip the cached state to **true**
/// before the listener runs. The listener recolors the box from that read alone
/// (no `@media` cascade, no MQL `change` event) — `red` ⇒ the listener saw the
/// fresh `true`; `lime` ⇒ it ran but read the **stale** cached `false` (the bug);
/// `blue` ⇒ the listener never ran. (A `max-width` query the other way is
/// already `true` at the 800 px default and cannot detect staleness.)
#[test]
fn content_thread_resize_listener_sees_fresh_matchmedia() {
    let (browser, content) = ipc::channel_pair::<BrowserToContent, ContentToBrowser>();
    let (nh, jar, _np) = test_network();
    let handle = spawn_test_content(
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
    );

    let has_color = |dl: &elidex_render::DisplayList, c: elidex_plugin::CssColor| {
        dl.iter().any(|item| {
            matches!(item, elidex_render::DisplayItem::SolidRect { color, .. } if *color == c)
        })
    };
    let red = elidex_plugin::CssColor::rgb(255, 0, 0);
    let lime = elidex_plugin::CssColor::rgb(0, 255, 0);

    // Initial frame: `min-width: 900px` is false at the 800 px default and the
    // listener has not run → box is its blue default (neither red nor lime).
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
    browser
        .send(BrowserToContent::SetViewport {
            width: 1000.0,
            height: 600.0,
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
