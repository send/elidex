//! S5-4b — iframe security-install **ordering** tests.
//!
//! Invariant under test (closes `#11-iframe-origin-before-initial-scripts`):
//! the security installs (`set_sandbox_flags` + `set_origin` +
//! `set_iframe_depth`) precede the FIRST eval on every iframe shape —
//! in-process (srcdoc, about:blank, no-src, URL-load, and the `blank_entry`
//! fallback) AND out-of-process (`make_out_of_process_entry`). They land as
//! one block at the `run_scripts_and_finalize` pre-eval chokepoint
//! (`crate::PreEvalFrameState`), so an order-proof against that block covers all
//! three setters at once.
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
//!   VM-surface oracle. Boa now keys localStorage off the installed origin too
//!   (`set_origin` syncs the `current_url`-derived `cached_origin`,
//!   `bridge/iframe_bridge.rs`), so the sandbox storage sentinel is exercised
//!   directly by the boa unit tests
//!   (`bridge/document_state.rs` `installed_opaque_origin_wins_localstorage_partition_over_url`);
//!   the *shell*-driven URL-load path only becomes observable at the S5-6
//!   engine flip (the harness's disconnected `NetworkHandle` can't fetch a
//!   URL-load iframe to the storage read — see the OOP reachability note).
//!
//! **OOP path reachability.** The production route into
//! `make_out_of_process_entry` needs a successful cross-origin
//! `load_document` — a real network fetch, so it is unreachable over the
//! test harness's disconnected `NetworkHandle` (a URL-load iframe here falls
//! into `blank_entry` instead). The OOP tests therefore drive the entry
//! constructor directly with a synthesized `LoadedDocument`; the caller-side
//! origin/flags computation they skip (`apply_sandbox_origin`,
//! `parse_sandbox`) is covered by the in-process tests above and the
//! `elidex-plugin` unit tests. The OOP pipeline lives on its own thread, so
//! the oracle is the IPC channel: `postMessage` from the initial script is
//! forwarded as `IframeToBrowser::PostMessage` whose `origin` field is
//! `bridge.origin().serialize()` captured **at call time** — a direct probe
//! of the origin the initial script observed (and message presence/absence
//! probes the `allow-scripts` eval gate). The thread's `Navigate` re-build
//! (`iframe/thread.rs` `handle_navigate`) rides the same `PreEvalFrameState`
//! seam but is not drivable here (`build_pipeline_from_url` spawns a real
//! network broker), so its ordering is guaranteed by the shared chokepoint
//! rather than a dedicated test.
//!
//! **Navigate origin derivation (F-a/F-c).** `handle_navigate` no longer
//! precomputes the origin from the *requested* URL; it hands
//! `PreEvalFrameInputs` to `build_pipeline_from_url`, which derives
//! `PreEvalFrameState.origin` from the **post-redirect** `loaded.url`
//! (`PreEvalFrameInputs::into_pre_eval_state`) with the persisted
//! credentialless flag applied. The full path (redirect follow) needs the live
//! broker `build_pipeline_from_url` spawns — unreachable here — so the
//! `navigate_inputs_*` unit tests pin the reachable equivalent: the derivation
//! reads the loaded (final) URL, not the requested one, and a credentialless
//! frame stays opaque across the navigation.

use super::iframe::{
    make_out_of_process_entry, BrowserToIframe, IframeEntry, IframeHandle, IframeToBrowser,
    InProcessIframe,
};
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
/// `PreEvalFrameState` threading).
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
/// gets the security installs through the same `PreEvalFrameState` threading.
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

/// F-b: the referrer rides the pre-eval chokepoint, so the iframe's *initial*
/// script reads the parent document URL as `document.referrer` (WHATWG HTML
/// §4.8.5), not the empty default. Pre-fix, `set_referrer` ran only AFTER the
/// pipeline build had already evaluated the initial scripts, so an initial-load
/// script saw `""`. Falsify by reverting the fold into `PreEvalFrameState`.
///
/// This is the **same-origin (in-process)** path: it keeps the FULL parent URL
/// (the cross-origin OOP path trims to the parent ORIGIN — see
/// `oop_cross_origin_iframe_initial_script_observes_parent_origin_referrer`).
#[test]
fn iframe_initial_script_observes_parent_referrer() {
    let (state, _browser) = build_test_content_state_with_url(
        r#"<iframe srcdoc='<div id="p"></div><script>document.getElementById("p").setAttribute("data-ref",document.referrer);</script>'></iframe>"#,
        url::Url::parse("https://parent.example/").unwrap(),
    );
    let ip = in_process_entry(&state);
    assert_eq!(
        probe_attr(&ip.pipeline, "p", "data-ref").as_deref(),
        Some("https://parent.example/"),
        "the initial script must read the parent URL as document.referrer, not the empty default"
    );
}

/// R5 (end-to-end): the iframe element's own `referrerpolicy="no-referrer"`
/// attribute is HONORED — the same-origin srcdoc frame that
/// `iframe_initial_script_observes_parent_referrer` shows exposing the full
/// parent URL under the DEFAULT policy now reads the EMPTY `document.referrer`,
/// because the author's directive suppresses it. The attribute is parsed into
/// `IframeData::referrer_policy` at DOM-build time and threaded into
/// `compute_referrer`. Falsify by reverting `compute_referrer` to the hardcoded
/// default policy (ignoring the parsed attribute) — the probe would read
/// `https://parent.example/` and this leak would reopen.
#[test]
fn iframe_referrerpolicy_no_referrer_suppresses_referrer() {
    let (state, _browser) = build_test_content_state_with_url(
        r#"<iframe referrerpolicy="no-referrer" srcdoc='<div id="p"></div><script>document.getElementById("p").setAttribute("data-ref",document.referrer);</script>'></iframe>"#,
        url::Url::parse("https://parent.example/").unwrap(),
    );
    let ip = in_process_entry(&state);
    assert_eq!(
        probe_attr(&ip.pipeline, "p", "data-ref").as_deref(),
        Some(""),
        "referrerpolicy=no-referrer must yield an empty document.referrer, not the parent URL"
    );
}

/// R3-F3 (end-to-end, same-origin/in-process path): a **local-scheme /
/// opaque-origin** parent (`data:` here) discloses NO referrer — its URL must
/// never leak into the child's `document.referrer` (W3C Referrer Policy §8.3
/// step 2.2 opaque-origin document / §8.4 step 2 local scheme). The child
/// srcdoc inherits the opaque origin and reads the empty referrer default.
/// Falsify by dropping the "no valid referrer source" precondition in
/// `compute_referrer` — the `data:` URL would leak here.
#[test]
fn local_scheme_parent_discloses_no_referrer() {
    let (state, _browser) = build_test_content_state_with_url(
        r#"<iframe srcdoc='<div id="p"></div><script>document.getElementById("p").setAttribute("data-ref",document.referrer);</script>'></iframe>"#,
        url::Url::parse("data:text/html,<p>parent</p>").unwrap(),
    );
    let ip = in_process_entry(&state);
    assert_eq!(
        probe_attr(&ip.pipeline, "p", "data-ref").as_deref(),
        Some(""),
        "a data: (opaque-origin/local-scheme) parent must disclose no referrer, \
         not leak its data: URL"
    );
}

/// F-c (persistence half): a `credentialless` iframe stores the flag on its
/// bridge and carries the opaque origin. The persisted flag is exactly what the
/// OOP `Navigate` re-build reads (`bridge.credentialless()`) to keep the origin
/// opaque across a navigation — see the `navigate_inputs_*` unit tests for the
/// derivation half. Pre-S5-4b the bridge held no credentialless bit at all.
#[test]
fn credentialless_iframe_persists_flag_and_opaque_origin() {
    let (state, _browser) = build_test_content_state_with_url(
        r#"<iframe credentialless srcdoc='<div id="p"></div>'></iframe>"#,
        url::Url::parse("https://parent.example/").unwrap(),
    );
    let ip = in_process_entry(&state);
    assert!(
        ip.pipeline.runtime.bridge().credentialless(),
        "a credentialless iframe must persist the flag on its bridge"
    );
    assert!(
        matches!(
            ip.pipeline.runtime.bridge().origin(),
            elidex_plugin::SecurityOrigin::Opaque(_)
        ),
        "credentialless yields an opaque origin"
    );
}

// ---------------------------------------------------------------------------
// OOP path (see the module doc for reachability + oracle notes)
// ---------------------------------------------------------------------------

/// Synthesize the `LoadedDocument` a successful cross-origin fetch would
/// produce (the disconnected test network cannot produce a real one).
fn synth_cross_origin_loaded(html: &str, url: &str) -> elidex_navigation::LoadedDocument {
    let parsed = elidex_html_parser::parse_progressive_str(html);
    let scripts = elidex_js_boa::extract_scripts(&parsed.dom, parsed.document)
        .into_iter()
        .map(|s| elidex_navigation::ResolvedScript {
            source: s.source,
            entity: s.entity,
        })
        .collect();
    elidex_navigation::LoadedDocument {
        dom: parsed.dom,
        document: parsed.document,
        stylesheets: Vec::new(),
        scripts,
        url: url::Url::parse(url).expect("test URL should parse"),
        response_headers: std::collections::HashMap::new(),
        manifest_url: None,
    }
}

/// Pump the OOP entry's channel until two `DisplayListReady` frames arrive,
/// then shut the thread down and return the collected `PostMessage`s.
///
/// Two frames, not one: the iframe thread's loop sends the frame's display
/// list BEFORE draining the bridge's pending `postMessage` queue in the same
/// iteration, so posts queued by the *initial* scripts (during the build,
/// before the loop) are on the channel only once the SECOND frame is —
/// channel delivery is FIFO. The two-frame requirement also proves the
/// pipeline built and the thread is alive (non-vacuity for the
/// no-message-expected case).
fn run_oop_and_collect_posts(entry: IframeEntry) -> Vec<(String, String)> {
    let IframeHandle::OutOfProcess(mut oop) = entry.handle else {
        panic!("expected an out-of-process iframe entry");
    };
    let mut posts = Vec::new();
    let mut frames = 0_usize;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while frames < 2 && std::time::Instant::now() < deadline {
        match oop
            .channel
            .recv_timeout(std::time::Duration::from_millis(100))
        {
            Ok(IframeToBrowser::DisplayListReady(_)) => frames += 1,
            Ok(IframeToBrowser::PostMessage { data, origin }) => posts.push((data, origin)),
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
        }
    }
    assert!(
        frames >= 2,
        "the OOP iframe thread never produced two frames — pipeline build \
         failed or thread died"
    );
    let _ = oop.channel.send(BrowserToIframe::Shutdown);
    if let Some(thread) = oop.thread.take() {
        thread
            .join()
            .expect("OOP iframe thread should join cleanly");
    }
    posts
}

/// OOP eval-gate order-proof: sandbox WITHOUT `allow-scripts` → the initial
/// script must NOT run on the iframe thread (WHATWG HTML §7.1.5 sandboxed
/// scripts flag). Pre-fix, `make_out_of_process_entry` passed `None` to the
/// build and installed the flags only after `build_pipeline_from_loaded` had
/// already evaluated the scripts — the probe message arrived and this test
/// fails under that ordering (verified by temporary inversion).
#[test]
fn oop_iframe_flags_installed_before_initial_scripts() {
    let loaded = synth_cross_origin_loaded(
        r#"<script>postMessage("s5-4b-gate-probe","*");</script>"#,
        "https://other.example/",
    );
    let entry = make_out_of_process_entry(
        loaded,
        crate::PreEvalFrameState {
            origin: elidex_plugin::SecurityOrigin::opaque(),
            sandbox_flags: Some(elidex_plugin::IframeSandboxFlags::empty()),
            iframe_depth: 1,
            credentialless: false,
            referrer: None,
        },
        crate::ipc::DeviceFacts::default(),
    );
    let posts = run_oop_and_collect_posts(entry);
    assert!(
        posts.is_empty(),
        "a sandboxed (no allow-scripts) OOP iframe must not run its initial \
         scripts, but a probe postMessage arrived: {posts:?}"
    );
}

/// OOP origin order-proof: sandboxed (`allow-scripts`, no `allow-same-origin`)
/// → the initial script observes the **opaque** origin at eval time. The
/// forwarded message's `origin` field is `bridge.origin().serialize()` read
/// synchronously inside `postMessage`, so `"null"` here means the opaque
/// origin was installed before the eval. Pre-fix the eval saw the
/// URL-derived tuple (`run_scripts_and_finalize`'s `None` arm installs
/// `SecurityOrigin::from_url`), the message carried
/// `"https://other.example"`, and this test fails under that ordering
/// (verified by temporary inversion).
#[test]
fn oop_sandboxed_iframe_initial_script_observes_opaque_origin() {
    let loaded = synth_cross_origin_loaded(
        r#"<script>postMessage("s5-4b-origin-probe","*");</script>"#,
        "https://other.example/",
    );
    let entry = make_out_of_process_entry(
        loaded,
        crate::PreEvalFrameState {
            origin: elidex_plugin::SecurityOrigin::opaque(),
            sandbox_flags: Some(elidex_plugin::IframeSandboxFlags::ALLOW_SCRIPTS),
            iframe_depth: 1,
            credentialless: false,
            referrer: None,
        },
        crate::ipc::DeviceFacts::default(),
    );
    let posts = run_oop_and_collect_posts(entry);
    assert_eq!(
        posts.len(),
        1,
        "the allow-scripts initial script should have posted exactly one probe: {posts:?}"
    );
    assert_eq!(posts[0].0, "s5-4b-origin-probe");
    assert_eq!(
        posts[0].1, "null",
        "the initial script must observe the opaque sandbox origin at eval \
         time, not the URL-derived tuple origin"
    );
}

/// Unsandboxed OOP control (regression pin + oracle discrimination): a
/// cross-origin document without sandbox keeps its URL-derived tuple origin,
/// and the origin oracle above really reports the eval-time origin (the
/// serialized tuple shows up when the origin IS the tuple).
#[test]
fn oop_unsandboxed_iframe_initial_script_observes_tuple_origin() {
    let url = "https://other.example/";
    let loaded = synth_cross_origin_loaded(
        r#"<script>postMessage("s5-4b-tuple-probe","*");</script>"#,
        url,
    );
    let entry = make_out_of_process_entry(
        loaded,
        crate::PreEvalFrameState {
            origin: elidex_plugin::SecurityOrigin::from_url(&url::Url::parse(url).unwrap()),
            sandbox_flags: None,
            iframe_depth: 1,
            credentialless: false,
            referrer: None,
        },
        crate::ipc::DeviceFacts::default(),
    );
    let posts = run_oop_and_collect_posts(entry);
    assert_eq!(
        posts.len(),
        1,
        "the initial script should have posted: {posts:?}"
    );
    assert_eq!(posts[0].0, "s5-4b-tuple-probe");
    assert_eq!(
        posts[0].1, "https://other.example",
        "an unsandboxed cross-origin document keeps its URL-derived tuple origin"
    );
}

/// OOP cross-origin referrer default (`strict-origin-when-cross-origin`): a
/// cross-origin OOP iframe's initial script reads `document.referrer` == the
/// parent's ORIGIN, not the full parent URL. The referrer rides the same
/// pre-eval chokepoint as origin/flags (`PreEvalFrameState`), so the *initial*
/// script already observes it — the probe posts `document.referrer` as its
/// message data. The initial-load trim source (`load.rs` OOP branch →
/// `compute_referrer`) is unit-tested in `content/iframe/load.rs` and
/// shares the same network-gated reachability as the OOP origin derivation (see
/// the module doc), so this test pins delivery through the OOP chokepoint.
/// Falsify by forwarding a full-URL referrer here.
#[test]
fn oop_cross_origin_iframe_initial_script_observes_parent_origin_referrer() {
    let loaded = synth_cross_origin_loaded(
        r#"<script>postMessage(document.referrer,"*");</script>"#,
        "https://other.example/",
    );
    let entry = make_out_of_process_entry(
        loaded,
        crate::PreEvalFrameState {
            origin: elidex_plugin::SecurityOrigin::from_url(
                &url::Url::parse("https://other.example/").unwrap(),
            ),
            sandbox_flags: None,
            iframe_depth: 1,
            credentialless: false,
            // The cross-origin default: parent ORIGIN-as-URL (trailing slash,
            // R3-F1), not the full parent URL.
            referrer: Some("https://parent.example/".to_string()),
        },
        crate::ipc::DeviceFacts::default(),
    );
    let posts = run_oop_and_collect_posts(entry);
    assert_eq!(
        posts.len(),
        1,
        "the initial script should have posted: {posts:?}"
    );
    assert_eq!(
        posts[0].0, "https://parent.example/",
        "a cross-origin OOP iframe reads the parent ORIGIN-as-URL as document.referrer, not the full URL"
    );
}

// ---------------------------------------------------------------------------
// Navigate origin derivation — post-redirect + credentialless (F-a / F-c)
// ---------------------------------------------------------------------------
// Reachable equivalent of the OOP `Navigate` re-build (the full redirect-follow
// path needs the live broker `build_pipeline_from_url` spawns — see module doc).
// `handle_navigate` builds `PreEvalFrameInputs` from the persisted bridge state
// and the builder resolves the origin via `into_pre_eval_state(&loaded.url)`.

/// F-a: the navigated document's origin comes from the **post-redirect**
/// `loaded.url`, not the requested URL. A load whose fetch resolves to
/// `https://final.example/` must attribute the document there — pre-fix,
/// `handle_navigate` derived the origin from the *requested* URL before the
/// fetch, mis-attributing any redirected load. Falsify: deriving from a
/// different (requested) URL yields a different origin.
#[test]
fn navigate_inputs_derive_origin_from_loaded_url() {
    let inputs = crate::PreEvalFrameInputs {
        sandbox_flags: None,
        credentialless: false,
        iframe_depth: 2,
        referrer: Some("https://parent.example/".to_string()),
    };
    let loaded = url::Url::parse("https://final.example/page").unwrap();
    let state = inputs.into_pre_eval_state(&loaded);
    assert_eq!(
        state.origin.serialize(),
        "https://final.example",
        "origin must be derived from the post-redirect loaded URL, not the requested one"
    );
    // The persisted frame facts ride through the rebuild unchanged.
    assert_eq!(state.referrer.as_deref(), Some("https://parent.example/"));
    assert_eq!(state.iframe_depth, 2);
    assert!(!state.credentialless);
}

/// F-c (derivation half): a credentialless frame keeps its opaque origin across
/// a navigation even when the loaded URL is a real tuple. Pre-fix,
/// `handle_navigate` hardcoded `credentialless = false` into the origin
/// derivation, so a credentialless frame regained a tuple origin on navigation.
/// Falsify: dropping the credentialless input yields the `https://final.example`
/// tuple.
#[test]
fn navigate_inputs_credentialless_stays_opaque() {
    let inputs = crate::PreEvalFrameInputs {
        sandbox_flags: None,
        credentialless: true,
        iframe_depth: 1,
        referrer: None,
    };
    let loaded = url::Url::parse("https://final.example/page").unwrap();
    let state = inputs.into_pre_eval_state(&loaded);
    assert!(
        matches!(state.origin, elidex_plugin::SecurityOrigin::Opaque(_)),
        "a credentialless frame must keep an opaque origin after navigation, got {:?}",
        state.origin
    );
    assert!(
        state.credentialless,
        "the credentialless flag rides through so the NEXT navigation stays opaque too"
    );
}
