//! S5-4c — `window.open` drain wiring + named-target routing tests.
//!
//! Two invariants under test:
//!
//! - **Drain wiring (edge E4)**: `process_pending_actions` drains the
//!   `window.open` back-channels through the engine-agnostic session trait
//!   surface (`JsRuntime::take_pending_open_tabs`), not the boa bridge. A
//!   gate-passed `_blank` open surfaces as `ContentToBrowser::OpenNewTab`; a
//!   sandbox-blocked open (boa's entry gate at `globals/window/mod.rs:359`
//!   blocks ALL of `window.open` without `allow-popups`) surfaces nothing —
//!   pinning the boa path end-to-end until the S5-6 flip.
//! - **Named-target MISS gating (edge E3)**: `route_frame_navigations` promotes
//!   a named-target MISS to a new tab **only** when the payload's call-time
//!   `aux_nav_allowed` snapshot permits (HTML §7.3.1.7 step 3 / step 8
//!   sandboxed auxiliary navigation); a HIT navigates the found iframe ungated
//!   (§7.4.2.4 step 2 — the descendant-only lookup makes the source an ancestor
//!   of the target). Since the boa path can only ever produce
//!   `aux_nav_allowed: true` (its entry gate blocks the sandboxed case
//!   upstream), the gate is exercised directly on synthesized payloads.
//!
//! Boa is the live shell engine until the S5-6 flip, so the oracle is the
//! `ContentToBrowser` channel (drain wiring) + the iframe `src` attribute
//! (HIT navigate) — the same seams the S5-4b ordering tests observe.

use super::navigation::{process_pending_actions, route_frame_navigations};
use super::test_support::{build_test_content_state, build_test_content_state_with_url};
use super::ContentState;
use crate::ipc::{BrowserToContent, ContentToBrowser, LocalChannel};

/// Collect every `OpenNewTab` URL currently queued on the browser channel end.
fn drain_open_new_tabs(
    browser: &LocalChannel<BrowserToContent, ContentToBrowser>,
) -> Vec<url::Url> {
    let mut urls = Vec::new();
    while let Ok(msg) = browser.try_recv() {
        if let ContentToBrowser::OpenNewTab(url) = msg {
            urls.push(url);
        }
    }
    urls
}

/// The `src` attribute currently on the single `<iframe>` entity.
fn iframe_src(state: &ContentState) -> Option<String> {
    let entity = (&mut state
        .pipeline
        .dom
        .world()
        .query::<(elidex_ecs::Entity, &elidex_ecs::IframeData)>())
        .into_iter()
        .next()
        .map(|(e, _)| e)?;
    state
        .pipeline
        .dom
        .world()
        .get::<&elidex_ecs::Attributes>(entity)
        .ok()
        .and_then(|a| a.get("src").map(str::to_owned))
}

/// UNsandboxed `window.open(url, "_blank")` in an initial script → the drain
/// surfaces a `ContentToBrowser::OpenNewTab` (through the engine-agnostic
/// trait surface, not the boa bridge).
#[test]
fn window_open_blank_drains_to_open_new_tab() {
    let (mut state, browser) = build_test_content_state(
        "<script>window.open('https://example.com/', '_blank');</script>",
        "",
    );
    let processed = process_pending_actions(&mut state);
    assert!(
        processed,
        "a queued open-tab must be drained by process_pending_actions"
    );
    let tabs = drain_open_new_tabs(&browser);
    assert_eq!(
        tabs,
        vec![url::Url::parse("https://example.com/").unwrap()],
        "an unsandboxed _blank open must surface exactly one OpenNewTab"
    );
}

/// A sandboxed (`allow-scripts`, NO `allow-popups`) document's
/// `window.open(url, "_blank")` is blocked by boa's entry gate → nothing is
/// queued, so the drain surfaces no `OpenNewTab`. Pins the boa path end-to-end:
/// the sandbox verdict is enforced at the enqueue chokepoint, not the drain.
#[test]
fn sandboxed_window_open_blank_surfaces_no_open_new_tab() {
    let (mut state, browser) = build_test_content_state("", "");
    // Install allow-scripts (so the script runs) WITHOUT allow-popups.
    state
        .pipeline
        .runtime
        .bridge()
        .set_sandbox_flags(Some(elidex_plugin::IframeSandboxFlags::ALLOW_SCRIPTS));
    {
        let p = &mut state.pipeline;
        p.runtime.eval(
            "window.open('https://example.com/', '_blank');",
            &mut p.session,
            &mut p.dom,
            p.document,
        );
    }
    let processed = process_pending_actions(&mut state);
    assert!(
        !processed,
        "a sandbox-blocked open queues nothing, so process_pending_actions has no action"
    );
    assert!(
        drain_open_new_tabs(&browser).is_empty(),
        "a sandboxed no-allow-popups open must not surface an OpenNewTab"
    );
}

/// Named-target MISS with `aux_nav_allowed: false` → dropped silently (HTML
/// §7.3.1.7 step 8 sandboxed auxiliary navigation): no `OpenNewTab`. This is
/// the sandbox bypass the slice closes — the pre-S5-4c shell promoted every
/// miss unconditionally.
#[test]
fn named_miss_without_aux_nav_grant_drops_silently() {
    let (mut state, browser) = build_test_content_state("<div>no iframe here</div>", "");
    route_frame_navigations(
        &mut state,
        vec![elidex_script_session::NamedFrameNavigation {
            name: "nonexistent".to_string(),
            url: "https://example.com/".to_string(),
            aux_nav_allowed: false,
        }],
    );
    assert!(
        drain_open_new_tabs(&browser).is_empty(),
        "a named MISS without an aux-nav grant must not promote to a new tab"
    );
}

/// Named-target MISS with `aux_nav_allowed: true` → promoted to a new tab.
#[test]
fn named_miss_with_aux_nav_grant_promotes_to_open_new_tab() {
    let (mut state, browser) = build_test_content_state("<div>no iframe here</div>", "");
    route_frame_navigations(
        &mut state,
        vec![elidex_script_session::NamedFrameNavigation {
            name: "nonexistent".to_string(),
            url: "https://example.com/".to_string(),
            aux_nav_allowed: true,
        }],
    );
    assert_eq!(
        drain_open_new_tabs(&browser),
        vec![url::Url::parse("https://example.com/").unwrap()],
        "a named MISS with an aux-nav grant promotes to exactly one OpenNewTab"
    );
}

/// Named-target HIT stays ungated even with `aux_nav_allowed: false`: the found
/// iframe is navigated (its `src` attribute updates) and NO `OpenNewTab` is
/// sent. The descendant-only `find_iframe_by_name` makes the source an ancestor
/// of the target, discharging §7.4.2.4 step 2 unconditionally.
#[test]
fn named_hit_navigates_iframe_ungated() {
    let (mut state, browser) = build_test_content_state_with_url(
        r#"<iframe name="child" srcdoc="<p>child</p>"></iframe>"#,
        url::Url::parse("https://parent.example/").unwrap(),
    );
    route_frame_navigations(
        &mut state,
        vec![elidex_script_session::NamedFrameNavigation {
            name: "child".to_string(),
            url: "https://navigated.example/".to_string(),
            aux_nav_allowed: false,
        }],
    );
    assert!(
        drain_open_new_tabs(&browser).is_empty(),
        "a named HIT navigates the iframe, never promotes to a new tab"
    );
    assert_eq!(
        iframe_src(&state).as_deref(),
        Some("https://navigated.example/"),
        "a named HIT updates the target iframe's src (ungated navigate), even with aux_nav_allowed=false"
    );
}

/// Link-target top-navigation re-key (edge E3, §4.3.3): the shell's
/// `event_handlers.rs` `_top`/`_parent` link-click gate is re-keyed onto
/// `elidex_plugin::sandbox::top_navigation_allowed(flags, true)` — a click IS a
/// user gesture, so `activation = true`. This delivers the 2-flag fidelity: a
/// sandboxed iframe with ONLY `allow-top-navigation-by-user-activation`
/// permits a `_top` click (whereas the pre-S5-4c raw
/// `contains(ALLOW_TOP_NAVIGATION)` would have blocked it), while a sandbox
/// with no top-nav token blocks it.
///
/// **Harness gap**: there is no shell click-simulation harness for a real
/// `<a target="_top">` click (blocked vs allowed both terminate in
/// `send_display_list`, indistinguishable on the channel), so this pins the
/// exact predicate decision the re-keyed site makes. Full end-to-end click
/// coverage lands with a click harness (noted in the S5-4c report).
#[test]
fn link_top_nav_rekey_honours_user_activation_flag() {
    use elidex_plugin::IframeSandboxFlags as F;
    // The re-keyed shell site passes activation=true (a click is a gesture).
    assert!(
        elidex_plugin::sandbox::top_navigation_allowed(
            Some(F::ALLOW_TOP_NAVIGATION_BY_USER_ACTIVATION),
            true,
        ),
        "allow-top-navigation-by-user-activation must permit a _top link CLICK (activation=true)"
    );
    assert!(
        !elidex_plugin::sandbox::top_navigation_allowed(Some(F::empty()), true),
        "a sandbox with no top-nav token blocks a _top link click"
    );
}
