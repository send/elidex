use super::*;
use crate::ipc::{self, BrowserToContent, ContentToBrowser};
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
            x: 50.0,
            y: 50.0,
            client_x: 50.0,
            client_y: 86.0,
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
            x: 50.0,
            y: 50.0,
            client_x: 50.0,
            client_y: 86.0,
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
            x: 50.0,
            y: 50.0,
            client_x: 50.0,
            client_y: 86.0,
        })
        .unwrap();
    let _ = browser.recv_timeout(Duration::from_secs(5)).unwrap();

    // Click (sets ACTIVE).
    browser
        .send(BrowserToContent::MouseClick(crate::ipc::MouseClickEvent {
            x: 50.0,
            y: 50.0,
            client_x: 50.0,
            client_y: 86.0,
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
            x: 50.0,
            y: 50.0,
            client_x: 50.0,
            client_y: 86.0,
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
        "<div id=\"box\" style=\"width: 100px; height: 100px;\">Key</div>\
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
            x: 50.0,
            y: 50.0,
            client_x: 50.0,
            client_y: 86.0,
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
