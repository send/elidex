use super::*;
use crate::ipc::{self, BrowserToContent, ContentToBrowser, ModifierState};
use elidex_plugin::Point;
use std::time::Duration;

#[test]
fn content_thread_startup_and_shutdown() {
    let (browser, content) = ipc::channel_pair::<BrowserToContent, ContentToBrowser>();
    let handle = spawn_content_thread(
        content,
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
    let handle = spawn_content_thread(
        content,
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
    let handle = spawn_content_thread(
        content,
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
    let handle = spawn_content_thread(
        content,
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
    let handle = spawn_content_thread(content, "<div>Hello</div>".to_string(), String::new());

    // Drain initial display list.
    let _ = browser.recv_timeout(Duration::from_secs(5)).unwrap();

    // Drop browser end — content thread should exit cleanly.
    drop(browser);
    handle.join().unwrap();
}

#[test]
fn content_thread_with_script() {
    let (browser, content) = ipc::channel_pair::<BrowserToContent, ContentToBrowser>();
    let handle = spawn_content_thread(
        content,
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
    let handle = spawn_content_thread(
        content,
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
    let handle = spawn_content_thread(
        content,
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
    let handle = spawn_content_thread(
        content,
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
    let handle = spawn_content_thread(
        content,
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
    let handle = spawn_content_thread(
        content,
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
