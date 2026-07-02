//! S5-4b — iframe security-install **ordering** tests.
//!
//! Invariant under test (closes `#11-iframe-origin-before-initial-scripts`):
//! the security installs (`set_sandbox_flags` + `set_origin` +
//! `set_iframe_depth`) precede the FIRST eval on every in-process iframe
//! shape — srcdoc, about:blank, no-src, URL-load, and the `blank_entry`
//! fallback. They land as one block at the `run_scripts_and_finalize`
//! pre-eval chokepoint (`crate::FrameSecurity`), so an order-proof against
//! that block covers all three setters at once.
//!
//! Oracle notes (boa is the live shell engine until the S5-6 flip):
//!
//! - **Flags ordering** is observed through the eval gate itself
//!   (`elidex-js-boa` `JsRuntime::eval` skips evaluation when
//!   `scripts_allowed()` is false): a sandboxed iframe without
//!   `allow-scripts` must NOT run its initial scripts (WHATWG HTML §7.1.5
//!   sandboxed scripts flag). Pre-fix, the flags were installed only after
//!   the build already ran the scripts, so the gate never fired for them.
//! - **Origin ordering** is observed through the WebSocket **mixed-content
//!   gate** (`elidex-js-boa` `globals/websocket.rs` reads `bridge.origin()`
//!   at construction): under an `https:` parent, a tuple-origin document gets
//!   a synchronous "mixed content blocked" throw for `ws://`, while an
//!   opaque-origin (sandboxed, no `allow-same-origin`) document skips that
//!   gate and fails later on the disconnected test network instead. The two
//!   error messages discriminate which origin the initial script observed.
//!   The plan-memo's storage-bucket sentinel (`elidex-js`
//!   `vm/host/storage.rs` routing through `document_origin()`) is a
//!   VM-surface oracle: boa keys localStorage off the `current_url`-derived
//!   `cached_origin` (`bridge/document_state.rs`), not off `set_origin`, so
//!   the storage sentinel only becomes observable in the shell at the S5-6
//!   engine flip.
//!
//! The OOP path (`make_out_of_process_entry`) is intentionally untouched by
//! S5-4b and keeps its own install sequence.

use super::iframe::{IframeEntry, IframeHandle, InProcessIframe};
use super::test_support::{
    build_test_content_state, build_test_content_state_with_url, probe_attr,
};
use super::ContentState;

/// The single `<iframe>` entity in the parent DOM.
fn iframe_entity(state: &ContentState) -> elidex_ecs::Entity {
    (&mut state
        .pipeline
        .dom
        .world()
        .query::<(elidex_ecs::Entity, &elidex_ecs::IframeData)>())
        .into_iter()
        .next()
        .map(|(e, _)| e)
        .expect("an <iframe> entity carrying IframeData should exist")
}

/// The loaded entry for the single iframe, which must be in-process.
fn in_process_entry(state: &ContentState) -> &InProcessIframe {
    let entity = iframe_entity(state);
    let entry: &IframeEntry = state
        .iframes
        .get(entity)
        .expect("the iframe should have loaded an entry");
    match &entry.handle {
        IframeHandle::InProcess(ip) => ip,
        IframeHandle::OutOfProcess(_) => panic!("expected an in-process iframe entry"),
    }
}

/// Sandbox WITHOUT `allow-scripts`: the initial script must NOT run — the
/// flags reach the bridge before the first eval, so the `allow-scripts` eval
/// gate applies to the *initial* scripts (WHATWG HTML §7.1.5 sandboxed
/// scripts flag). Pre-S5-4b this failed: the flags landed only in
/// `make_in_process_entry`, after the build had already evaluated the script.
///
/// This is the order-proof for the whole install block: all three setters
/// land together at the `run_scripts_and_finalize` pre-eval chokepoint.
#[test]
fn sandboxed_iframe_flags_installed_before_initial_scripts() {
    let (state, _browser) = build_test_content_state(
        r#"<iframe sandbox="allow-same-origin"
            srcdoc='<div id="p"></div><script>document.getElementById("p").setAttribute("data-ran","1");</script>'>
           </iframe>"#,
        "",
    );
    let ip = in_process_entry(&state);
    assert_eq!(
        probe_attr(&ip.pipeline, "p", "data-ran"),
        None,
        "a sandboxed iframe without allow-scripts must not run its initial scripts"
    );
    // The flags themselves are installed on the entry's bridge.
    assert_eq!(
        ip.pipeline.runtime.bridge().sandbox_flags(),
        Some(elidex_plugin::IframeSandboxFlags::ALLOW_SAME_ORIGIN),
        "parsed sandbox flags must be installed on the iframe bridge"
    );
}

/// Sandbox WITH `allow-scripts` (regression pin): the eval gate must not
/// over-block — the initial script still runs, and the entry carries the
/// opaque origin (no `allow-same-origin`).
#[test]
fn sandboxed_iframe_with_allow_scripts_still_runs_initial_scripts() {
    let (state, _browser) = build_test_content_state(
        r#"<iframe sandbox="allow-scripts"
            srcdoc='<div id="p"></div><script>document.getElementById("p").setAttribute("data-ran","1");</script>'>
           </iframe>"#,
        "",
    );
    let ip = in_process_entry(&state);
    assert_eq!(
        probe_attr(&ip.pipeline, "p", "data-ran").as_deref(),
        Some("1"),
        "allow-scripts must keep the initial scripts running"
    );
    assert!(
        matches!(
            ip.pipeline.runtime.bridge().origin(),
            elidex_plugin::SecurityOrigin::Opaque(_)
        ),
        "sandbox without allow-same-origin must yield an opaque origin"
    );
}

/// Sandboxed (no `allow-same-origin`) srcdoc iframe under an `https:` parent:
/// the initial script observes the **opaque** origin, not the parent/URL
/// tuple origin. Observed through the WebSocket mixed-content gate (reads
/// `bridge.origin()` synchronously at construction): a tuple `https` origin
/// throws "mixed content blocked" for `ws://`; the opaque origin skips that
/// gate and fails on the disconnected test network instead. Pre-S5-4b this
/// failed with the mixed-content message — the initial script saw the
/// URL-derived tuple origin (`run_scripts_and_finalize` installed
/// `SecurityOrigin::from_url` and the opaque override only landed after the
/// scripts had run).
#[test]
fn sandboxed_iframe_initial_script_observes_opaque_origin() {
    let (state, _browser) = build_test_content_state_with_url(
        // boa registers `WebSocket` as a plain callable — invoke without `new`.
        r#"<iframe sandbox="allow-scripts"
            srcdoc='<div id="p"></div><script>let r;try{WebSocket("ws://wsoracle.invalid/");r="constructed";}catch(e){r=String(e.message||e);}document.getElementById("p").setAttribute("data-ws",r);</script>'>
           </iframe>"#,
        url::Url::parse("https://parent.example/").unwrap(),
    );
    let ip = in_process_entry(&state);
    let observed = probe_attr(&ip.pipeline, "p", "data-ws")
        .expect("the sandboxed (allow-scripts) initial script should have run");
    assert!(
        !observed.contains("mixed content"),
        "initial script observed the URL tuple origin (mixed-content gate fired) \
         instead of the opaque sandbox origin: {observed}"
    );
    // Not vacuous: the opaque-origin path gets PAST the mixed-content gate and
    // fails on the disconnected test network instead.
    assert!(
        observed.contains("network"),
        "expected the disconnected-network failure past the mixed-content gate, \
         got: {observed}"
    );
    assert!(
        matches!(
            ip.pipeline.runtime.bridge().origin(),
            elidex_plugin::SecurityOrigin::Opaque(_)
        ),
        "the entry must carry the opaque origin"
    );
}

/// Unsandboxed control under the same `https:` parent (regression pin): the
/// initial script runs and observes the inherited **tuple** origin — the
/// mixed-content gate fires for `ws://`. Also proves the origin oracle above
/// discriminates (the gate DOES fire when the origin really is the tuple).
#[test]
fn unsandboxed_iframe_initial_script_observes_tuple_origin() {
    let (state, _browser) = build_test_content_state_with_url(
        // boa registers `WebSocket` as a plain callable — invoke without `new`.
        r#"<iframe
            srcdoc='<div id="p"></div><script>let r;try{WebSocket("ws://wsoracle.invalid/");r="constructed";}catch(e){r=String(e.message||e);}document.getElementById("p").setAttribute("data-ws",r);</script>'>
           </iframe>"#,
        url::Url::parse("https://parent.example/").unwrap(),
    );
    let ip = in_process_entry(&state);
    let observed = probe_attr(&ip.pipeline, "p", "data-ws")
        .expect("an unsandboxed iframe's initial script should have run");
    assert!(
        observed.contains("mixed content"),
        "an unsandboxed iframe inherits the parent https tuple origin, so the \
         ws:// mixed-content gate must fire: {observed}"
    );
    assert!(
        matches!(
            ip.pipeline.runtime.bridge().origin(),
            elidex_plugin::SecurityOrigin::Tuple { .. }
        ),
        "the unsandboxed entry must keep the inherited tuple origin"
    );
    assert_eq!(ip.pipeline.runtime.bridge().sandbox_flags(), None);
}

/// `src="about:blank"` arm: the reordered plumbing still installs the
/// security state (there are no initial scripts on this arm — the install
/// itself is the pin, guarding against the arm being missed by the
/// `FrameSecurity` threading).
#[test]
fn about_blank_iframe_installs_sandbox_state() {
    let (state, _browser) = build_test_content_state(
        r#"<iframe sandbox="allow-scripts" src="about:blank"></iframe>"#,
        "",
    );
    let ip = in_process_entry(&state);
    assert_eq!(
        ip.pipeline.runtime.bridge().sandbox_flags(),
        Some(elidex_plugin::IframeSandboxFlags::ALLOW_SCRIPTS)
    );
    assert!(matches!(
        ip.pipeline.runtime.bridge().origin(),
        elidex_plugin::SecurityOrigin::Opaque(_)
    ));
}

/// No-src arm: same install pin as the about:blank arm.
#[test]
fn no_src_iframe_installs_sandbox_state() {
    let (state, _browser) =
        build_test_content_state(r#"<iframe sandbox="allow-scripts"></iframe>"#, "");
    let ip = in_process_entry(&state);
    assert_eq!(
        ip.pipeline.runtime.bridge().sandbox_flags(),
        Some(elidex_plugin::IframeSandboxFlags::ALLOW_SCRIPTS)
    );
    assert!(matches!(
        ip.pipeline.runtime.bridge().origin(),
        elidex_plugin::SecurityOrigin::Opaque(_)
    ));
}

/// `blank_entry` fallback arm (invalid `src` URL): the blank document still
/// gets the security installs through the same `FrameSecurity` threading.
#[test]
fn blank_entry_fallback_installs_sandbox_state() {
    let (state, _browser) = build_test_content_state_with_url(
        // "http://[" fails to parse/join → load_iframe_from_url → blank_entry.
        r#"<iframe sandbox="" src="http://["></iframe>"#,
        url::Url::parse("https://parent.example/").unwrap(),
    );
    let ip = in_process_entry(&state);
    assert_eq!(
        ip.pipeline.runtime.bridge().sandbox_flags(),
        Some(elidex_plugin::IframeSandboxFlags::empty()),
        "sandbox=\"\" must install maximally-restrictive (empty) flags"
    );
    assert!(
        matches!(
            ip.pipeline.runtime.bridge().origin(),
            elidex_plugin::SecurityOrigin::Opaque(_)
        ),
        "sandbox without allow-same-origin must yield an opaque origin on the \
         blank fallback too"
    );
}
