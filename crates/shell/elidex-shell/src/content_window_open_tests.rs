//! S5-4c — `window.open` drain wiring + named-target routing tests.
//!
//! Two invariants under test:
//!
//! - **Drain wiring (edge E4)**: both pumps drain the ordered `window.open`
//!   intent queue through the engine-agnostic session trait surface
//!   (`JsRuntime::take_pending_window_opens`), not the boa bridge, and route it
//!   via `route_window_opens` (one home, call order preserved). A gate-passed
//!   `_blank` open surfaces as `ContentToBrowser::OpenNewTab`; a sandbox-blocked
//!   open (boa's entry gate at `globals/window/mod.rs:359` blocks ALL of
//!   `window.open` without `allow-popups`) surfaces nothing — pinning the boa
//!   path end-to-end until the S5-6 flip.
//! - **Named-target MISS gating (edge E3)**: `route_window_opens` promotes
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

use elidex_script_session::{NamedFrameNavigation, OpenTabRequest, WindowOpenIntent};

use super::navigation::{process_pending_actions, route_window_opens};
use super::test_support::{build_test_content_state, build_test_content_state_with_url};
use super::ContentState;
use crate::ipc::{BrowserToContent, ContentToBrowser, LocalChannel};

/// Wrap named-target navigations as ordered `window.open` intents (the shape
/// `route_window_opens` consumes).
fn named(navs: Vec<NamedFrameNavigation>) -> Vec<WindowOpenIntent> {
    navs.into_iter().map(WindowOpenIntent::NamedFrame).collect()
}

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
    // A `_blank` popup opens ANOTHER context — it is applied (OpenNewTab below)
    // but does NOT count as an own-context action, so `process_pending_actions`
    // reports `false` (a co-located link's default navigation must not be
    // suppressed by the popup).
    assert!(
        !processed,
        "a _blank popup is not an own-context navigation"
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
    route_window_opens(
        &mut state,
        named(vec![NamedFrameNavigation {
            name: "nonexistent".to_string(),
            url: Some("https://example.com/".to_string()),
            aux_nav_allowed: false,
        }]),
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
    route_window_opens(
        &mut state,
        named(vec![NamedFrameNavigation {
            name: "nonexistent".to_string(),
            url: Some("https://example.com/".to_string()),
            aux_nav_allowed: true,
        }]),
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
    route_window_opens(
        &mut state,
        named(vec![NamedFrameNavigation {
            name: "child".to_string(),
            url: Some("https://navigated.example/".to_string()),
            aux_nav_allowed: false,
        }]),
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

/// Async-pump drain symmetry (edge E4, Codex R1): the async `run_event_loop`
/// pump must drain the NAMED `window.open` intents too, not only `_blank`.
/// A named-target open queued outside an input turn (timer / postMessage)
/// would otherwise stall forever. Drive the async pump's drain — take the
/// runtime's ordered window.open queue and route it — with a named nav
/// enqueued on the runtime back-channel → the matching iframe is navigated
/// (HIT), and `route_window_opens` reports `true` (re-render needed). Before
/// the R1 fix the async pump never drained this channel, so the enqueued nav
/// was silently stranded.
#[test]
fn async_pump_drains_named_window_open_channel() {
    let (mut state, browser) = build_test_content_state_with_url(
        r#"<iframe name="child" srcdoc="<p>child</p>"></iframe>"#,
        url::Url::parse("https://parent.example/").unwrap(),
    );
    // Enqueue a named open on the runtime back-channel (what a timer-driven
    // `window.open('/x', 'child')` produces), then run ONLY the async pump's
    // drain — no `process_pending_actions` (the input-driven path).
    state.pipeline.runtime.bridge().set_pending_navigate_iframe(
        "child".to_string(),
        url::Url::parse("https://navigated.example/").unwrap(),
    );
    let intents = state.pipeline.runtime.take_pending_window_opens();
    let outcome = route_window_opens(&mut state, intents);
    assert!(
        outcome.navigated_iframe,
        "a named HIT re-navigates an iframe → re-render"
    );
    assert!(outcome.any_effect, "a named HIT is a real effect");
    assert!(
        drain_open_new_tabs(&browser).is_empty(),
        "a named HIT navigates the iframe, never promotes to a new tab"
    );
    assert_eq!(
        iframe_src(&state).as_deref(),
        Some("https://navigated.example/"),
        "the async pump routed the named open to the matching iframe"
    );
}

/// Codex R5-F1: a batch whose every intent is a dropped no-op (a sandbox-blocked
/// named MISS) reports `any_effect == false`, so `process_pending_actions` does
/// not claim an action and a caller's default (a link's `<a href>` navigation)
/// is not suppressed by a `window.open` that did nothing.
#[test]
fn route_window_opens_reports_no_effect_for_dropped_noop() {
    let (mut state, browser) = build_test_content_state("<div>no iframe here</div>", "");
    let outcome = route_window_opens(
        &mut state,
        named(vec![NamedFrameNavigation {
            name: "missing".to_string(),
            url: Some("https://x.example/".to_string()),
            aux_nav_allowed: false,
        }]),
    );
    assert!(
        !outcome.any_effect,
        "a sandbox-blocked MISS is dropped — not a real effect"
    );
    assert!(!outcome.navigated_iframe);
    assert!(drain_open_new_tabs(&browser).is_empty());
}

/// Codex R5-F3: a `javascript:` / `vbscript:` `window.open` URL is blocked by the
/// shell navigation chokepoint (`resolve_nav_url`), NOT forwarded as an
/// `OpenNewTab` — the same scheme filter link / location navigation applies.
#[test]
fn route_window_opens_blocks_javascript_scheme_popup() {
    let (mut state, browser) = build_test_content_state("<div>doc</div>", "");
    let outcome = route_window_opens(
        &mut state,
        vec![WindowOpenIntent::Popup(OpenTabRequest {
            url: "javascript:alert(1)".to_string(),
        })],
    );
    assert!(
        !outcome.any_effect,
        "a blocked-scheme popup produces no OpenNewTab"
    );
    assert!(
        drain_open_new_tabs(&browser).is_empty(),
        "javascript: must never reach the browser as a new tab"
    );
}

/// Codex R5-F2: a same-turn `_blank` popup + own-context (`_self` / `location`)
/// navigation must BOTH surface. `process_pending_actions` drains the
/// (non-destructive) window.open queue BEFORE the pipeline-replacing navigation,
/// so the popup is not stranded on the old pipeline's runtime. Drives the boa
/// back-channels directly (a `_blank` open + a `_self`-style pending navigation
/// to a `data:` URL — inline, no network) and asserts the popup's `OpenNewTab`
/// still fires. Before the fix, the navigation was drained first and replaced
/// the pipeline, losing the popup.
#[test]
fn window_open_popup_survives_same_turn_self_navigation() {
    let (mut state, browser) = build_test_content_state("<div>doc</div>", "");
    let bridge = state.pipeline.runtime.bridge();
    bridge.queue_open_tab(url::Url::parse("https://popup.example/").unwrap());
    bridge.set_pending_navigation(elidex_script_session::NavigationRequest {
        url: "data:text/html,<p>next</p>".to_string(),
        nav_type: elidex_script_session::NavigationType::Push,
    });
    let acted = process_pending_actions(&mut state);
    assert!(
        acted,
        "the own-context _self navigation is reported as an own-context action"
    );
    assert_eq!(
        drain_open_new_tabs(&browser),
        vec![url::Url::parse("https://popup.example/").unwrap()],
        "the _blank popup must open even though a same-turn _self navigation ran"
    );
}

/// Global ordering (Codex R2): a named-MISS and a `_blank` popup interleaved
/// on the ordered intent queue route their `OpenNewTab`s in QUEUE order — the
/// shell preserves whatever call order the engine recorded. A named MISS
/// (promoted) followed by a popup must open /first then /second, not reversed.
#[test]
fn route_window_opens_preserves_intent_order_across_popup_and_named() {
    let (mut state, browser) = build_test_content_state("<div>no iframe here</div>", "");
    route_window_opens(
        &mut state,
        vec![
            WindowOpenIntent::NamedFrame(NamedFrameNavigation {
                name: "missing".to_string(),
                url: Some("https://first.example/".to_string()),
                aux_nav_allowed: true,
            }),
            WindowOpenIntent::Popup(OpenTabRequest {
                url: "https://second.example/".to_string(),
            }),
        ],
    );
    assert_eq!(
        drain_open_new_tabs(&browser),
        vec![
            url::Url::parse("https://first.example/").unwrap(),
            url::Url::parse("https://second.example/").unwrap(),
        ],
        "OpenNewTab order must match intent (call) order — named MISS before popup"
    );
}

/// Empty-url named MISS (`url: None`) → new navigable defaulting to
/// about:blank (§7.2.2.1 step 15.3), gated on the aux-nav grant.
#[test]
fn named_miss_empty_url_promotes_to_about_blank_tab() {
    let (mut state, browser) = build_test_content_state("<div>no iframe here</div>", "");
    route_window_opens(
        &mut state,
        named(vec![NamedFrameNavigation {
            name: "nonexistent".to_string(),
            url: None,
            aux_nav_allowed: true,
        }]),
    );
    assert_eq!(
        drain_open_new_tabs(&browser),
        vec![url::Url::parse("about:blank").unwrap()],
        "an empty-url named MISS opens an about:blank tab (new-navigable default)"
    );
}

/// Empty-url named HIT (`url: None`) → NO-OP: §7.2.2.1 step 16.1 navigates an
/// existing navigable only for a non-null urlRecord, so the found iframe is
/// left untouched (its `src` unchanged) and no tab opens.
#[test]
fn named_hit_empty_url_is_a_noop() {
    let (mut state, browser) = build_test_content_state_with_url(
        r#"<iframe name="child" srcdoc="<p>child</p>"></iframe>"#,
        url::Url::parse("https://parent.example/").unwrap(),
    );
    let src_before = iframe_src(&state);
    route_window_opens(
        &mut state,
        named(vec![NamedFrameNavigation {
            name: "child".to_string(),
            url: None,
            aux_nav_allowed: true,
        }]),
    );
    assert!(drain_open_new_tabs(&browser).is_empty());
    assert_eq!(
        iframe_src(&state),
        src_before,
        "an empty-url named HIT is a no-op — the existing iframe is not navigated"
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
