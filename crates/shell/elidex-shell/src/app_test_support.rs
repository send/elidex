//! Shared test helpers for the app-mode (legacy inline) test modules
//! (`app_fragment_nav_tests`, `app_history_drain_tests`,
//! `app_history_phase_sep_tests`, `app_turn_completion_tests`) — building a
//! driveable `App` over a
//! disconnected network, seeding the two-entry session histories the drain suite
//! traverses, plus the history/URL probes those modules assert on. Kept in one
//! place so no test module owns the scaffolding, and so the long
//! `build_pipeline_interactive_shared` call (whose signature has churned) has ONE
//! app-mode caller to update.
//!
//! **Harness reachability — the contract every drain assertion rests on.** These
//! tests build an `App` via [`App::new_interactive_with_url`] (no winit —
//! `render_state` is `None`) over a **disconnected** network, so a *successful*
//! cross-document rebuild is not reachable: `load_document` always fails, leaving
//! the pipeline + cursor unchanged. Traversals between entries that share a
//! `document_sequence` (seeded with [`seed_same_document_pair`], or created by
//! `pushState`) take the no-fetch `same_document_step` path, so their Phase-2
//! apply SUCCEEDS here — that is the path every "the traversal landed" assertion
//! uses. Because `render_state` is `None` the frame-ship is unobservable as an OS
//! repaint, so ship-once is asserted on the coordinator's own bookkeeping
//! (`DrainOutcome::shipped` / `DrainOutcome::own_context_action`), which is what
//! gates `DrainHost::ship_frame`.
//!
//! **App-local on purpose — NOT a fold into `content_test_support`.** That module
//! is content-thread specific: it spawns content threads over a test broker,
//! builds `ContentState`, and its `drain_browser` consumes a browser IPC channel
//! an inline `App` does not have (inline mode is synchronous). Folding these
//! helpers there would couple app-mode tests to that scaffolding for no gain
//! (plan §5: a FALSE unification target).

use elidex_script_session::HostDriver;

use super::App;

/// The top-level document URL every app-mode test builds against.
pub(super) fn base() -> url::Url {
    url::Url::parse("https://example.com/").unwrap()
}

pub(super) fn url(s: &str) -> url::Url {
    url::Url::parse(s).unwrap()
}

/// Build an app-mode `App` at `url` over a disconnected network, laid out (so hit
/// testing and fragment scroll-resolution see `LayoutBox`es).
/// `new_interactive_with_url` seeds the initial history entry from `pipeline.url`
/// (so `len` starts at 1, cursor at index 0).
pub(super) fn app_at(html: &str, url: url::Url) -> App {
    let pipeline = crate::build_pipeline_interactive_shared(
        html,
        Some(url),
        std::sync::Arc::new(elidex_text::FontDatabase::new()),
        std::rc::Rc::new(elidex_net::broker::NetworkHandle::disconnected()),
        std::sync::Arc::new(crate::create_css_property_registry()),
        None,
        None, // No WebStorageManager (app-mode test → in-memory fallback).
        elidex_plugin::Size::new(1024.0, 768.0),
        crate::ipc::DeviceFacts::default(),
        None,
    );
    let mut app = App::new_interactive_with_url(pipeline, "elidex".to_string());
    crate::re_render(&mut app.interactive.as_mut().unwrap().pipeline);
    app
}

/// The value of `attr` on the first `<tag>` element carrying it — the one DOM
/// reach-through behind every "did the listener run?" probe (a handler stamps an
/// attribute, the assertion reads it back).
///
/// Goes through [`EcsDom::get_attribute`](elidex_ecs::EcsDom::get_attribute)
/// rather than open-coding the `Attributes` component read, so it inherits that
/// method's stated contract — `None` means "no READABLE attribute" and folds
/// three cases together (component absent, key absent, `World::get` failure such
/// as a destroyed entity or a borrow conflict). The sibling
/// `content_test_support::probe_attr` still open-codes it; that copy predates this
/// one and is not this PR's to change.
///
/// Every path that fires a listener also `re_render`s, which flushes the script
/// session, so a stamp is committed by assertion time.
pub(super) fn attr_value(app: &App, tag: &str, attr: &str) -> Option<String> {
    let dom = &app.interactive.as_ref().unwrap().pipeline.dom;
    dom.query_by_tag(tag)
        .into_iter()
        .find_map(|e| dom.get_attribute(e, attr))
}

/// Whether the FIRST `<tag>` element carrying `attr` carries it as `"1"` — the
/// boolean "the listener ran" case of [`attr_value`], and first-match for the same
/// reason it is: a probe that scanned for a matching value anywhere would report
/// "the listener ran" for a page where a DIFFERENT element than the one under test
/// carries the stamp.
pub(super) fn stamped(app: &App, tag: &str, attr: &str) -> bool {
    attr_value(app, tag, attr).as_deref() == Some("1")
}

/// Place the inline cursor over content-area point `(x, y)` (winit client coords
/// are chrome-inclusive, so the chrome bar height is added back).
pub(super) fn cursor_over_content(app: &mut App, x: f64, y: f64) {
    app.interactive.as_mut().unwrap().cursor_pos = Some(elidex_plugin::Point::new(
        x,
        y + f64::from(crate::chrome::CHROME_HEIGHT),
    ));
}

/// The session-history entry count (`history.length`'s shell-side source).
pub(super) fn history_len(app: &App) -> usize {
    app.interactive.as_ref().unwrap().nav_controller.len()
}

/// The ACTIVE DOCUMENT's URL (`pipeline.url`) — distinct from the history cursor's
/// entry URL, which is what makes it the probe for "did the document actually
/// navigate" as opposed to "did the cursor move".
pub(super) fn pipeline_url(app: &App) -> Option<String> {
    app.interactive
        .as_ref()
        .unwrap()
        .pipeline
        .url
        .as_ref()
        .map(|u| u.as_str().to_string())
}

// ---------------------------------------------------------------------------
// Session-history seeds + drain probes (the history drain suite)
// ---------------------------------------------------------------------------

/// Seed `[base, /a]` sharing ONE `document_sequence`, cursor on `/a` — the
/// same-document pair whose `back()` applies in the disconnected harness (no fetch).
/// The app-mode mirror of `content_test_support::seed_same_document_pair`.
pub(super) fn seed_same_document_pair(app: &mut App) {
    let a = url("https://example.com/a");
    // index 0 = base was seeded by `new_interactive_with_url`; index 1 = /a inherits
    // its `document_sequence`, so a traversal between them is SameDocument.
    app.interactive
        .as_mut()
        .unwrap()
        .nav_controller
        .push_same_document(a.clone());
    activate_seeded_entry(app, a);
}

/// Seed `[base, /a, /b]` sharing ONE `document_sequence`, cursor on `/b` — the
/// three-entry extension of [`seed_same_document_pair`], for the scenarios that
/// need a forward entry to destroy (or a third entry for a mover to land on) as
/// well as a back target.
pub(super) fn seed_same_document_triple(app: &mut App) {
    let a = url("https://example.com/a");
    let b = url("https://example.com/b");
    for entry in [a, b.clone()] {
        app.interactive
            .as_mut()
            .unwrap()
            .nav_controller
            .push_same_document(entry);
    }
    activate_seeded_entry(app, b);
}

/// Seed `[base, /a]` as two DISTINCT documents (fresh `document_sequence`s), cursor
/// on `/a` — a `back()` here classifies `Rebuild`, and its cross-document load FAILS
/// in the disconnected harness (the failed-load / cursor-atomicity path).
///
/// The ONLY difference from [`seed_same_document_pair`] is `push` vs
/// `push_same_document`; everything downstream of the cursor move is the shared
/// [`activate_seeded_entry`] tail.
pub(super) fn seed_cross_document_pair(app: &mut App) {
    let a = url("https://example.com/a");
    app.interactive
        .as_mut()
        .unwrap()
        .nav_controller
        .push(a.clone());
    activate_seeded_entry(app, a);
}

/// The shared tail of the two seeds: point the ACTIVE-DOCUMENT facts at the entry
/// the cursor was just moved onto — pipeline URL + VM current-URL + VM
/// session-history `(index, length)`.
///
/// Mirrors what production does on every cursor move (`navigate` /
/// `same_document_step` / `navigate_to_history_url` / `apply_state_change` all
/// end in `set_session_history`), so neither seed leaves the
/// production-impossible state where `history.length` still describes the
/// pre-push list while the cursor sits on `/a`. Factored out because the
/// cross-document seed silently OMITTED the `set_session_history` call — the exact
/// bug shape Codex fixed in `content_test_support::seed_same_document_pair` on
/// PR #469 R18, where a controller-only seed let spurious passes through.
pub(super) fn activate_seeded_entry(app: &mut App, url: url::Url) {
    let interactive = app.interactive.as_mut().unwrap();
    interactive.pipeline.url = Some(url.clone());
    interactive.pipeline.runtime.set_current_url(Some(url));
    interactive.pipeline.runtime.set_session_history(
        interactive.nav_controller.current_index(),
        interactive.nav_controller.len(),
    );
}

// ---------------------------------------------------------------------------
// popstate page builders (the staging fixtures the drain suite runs on)
// ---------------------------------------------------------------------------

/// A page whose `popstate` listener runs `script` **once**.
///
/// Guarded by a flag rather than `removeEventListener`, so a later traversal's
/// `popstate` re-entering the handler is a plain no-op: with removal, the outcome
/// depends on listener-removal semantics, which is never what these tests are
/// about.
pub(super) fn popstate_once(script: &str) -> String {
    format!(
        "<p>doc</p>\
         <script>window.__staged = false;\
         window.addEventListener('popstate', function () {{\
           if (window.__staged) {{ return; }}\
           window.__staged = true; {script}\
         }});</script>"
    )
}

/// A page whose `popstate` listener runs `script` on EVERY fire and stamps the
/// fire count onto `<p data-n>` — the adversarial re-stager shape, plus the probe
/// that counts turn-completion loop iterations (each iteration applies exactly one
/// traversal, and every same-document apply fires `popstate` once).
///
/// Read the stamp back with [`popstate_fires`].
pub(super) fn popstate_every(script: &str) -> String {
    format!(
        "<p>doc</p>\
         <script>window.__n = 0;\
         window.addEventListener('popstate', function () {{\
           window.__n++;\
           document.querySelector('p').setAttribute('data-n', String(window.__n));\
           {script}\
         }});</script>"
    )
}

/// How many times [`popstate_every`]'s listener fired.
pub(super) fn popstate_fires(app: &App) -> usize {
    attr_value(app, "p", "data-n")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

/// Run `script` in the page's VM, **failing loudly on a thrown script**. Discarding
/// the `Result` (the earlier shape) made "the intent was staged" and "the script
/// threw before staging anything" indistinguishable — every caller's assertion about
/// a *drained* intent silently degrades into an assertion about an empty drain.
pub(super) fn eval(app: &mut App, script: &str) {
    if let Err(e) = app
        .interactive
        .as_mut()
        .unwrap()
        .pipeline
        .runtime
        .vm()
        .eval(script)
    {
        panic!("test script threw, so nothing was staged: {script}\n  {e:?}");
    }
}

/// The history CURSOR's entry URL — the drain-specific complement of
/// [`pipeline_url`] (the active document's URL): a traversal that moved the cursor
/// but whose document load failed leaves the two disagreeing.
pub(super) fn current_url(app: &App) -> Option<String> {
    app.interactive
        .as_ref()
        .unwrap()
        .nav_controller
        .current_url()
        .map(|u| u.as_str().to_string())
}

/// The §4.4 quiescence predicate as the tests read it — **forwarded to the drive
/// site's own `App::staged_work_pending`**, not a copy of its reach-through, so an
/// assertion can never keep passing against a path the loop no longer takes.
///
/// The probe for "did the drive reach quiescence?" — and for the degraded exits,
/// where the assertion must be "work is still STAGED", never "a flag was set"
/// (there is no flag; the channels are the SoT).
pub(super) fn staged_session_history_work(app: &App) -> bool {
    app.staged_work_pending()
}

/// The loop's **document-swap marker** — forwarded to `App::current_document_marker`
/// itself, because the pin that reads it is a regression guard ON the swap exit and
/// must therefore read the exact function that exit reads. Its stability across a
/// FAILED mid-loop load is what keeps the swap exit from firing on a navigate
/// *attempt*.
pub(super) fn document_marker(app: &App) -> Option<u64> {
    app.current_document_marker()
}

/// The session-history CURSOR's 0-based index — the probe that survives an
/// assertion the entry-URL probes cannot make, namely "the cursor did not move,
/// the entry under it changed".
pub(super) fn current_index(app: &App) -> usize {
    app.interactive
        .as_ref()
        .unwrap()
        .nav_controller
        .current_index()
}

/// The URL of session-history ENTRY `index`. The discriminating probe wherever
/// `history_len` is not: a same-document `pushState`/fragment nav from a
/// cursor-moved position TRUNCATES the forward entries and appends its own, so a
/// counterfactual that applied one lands at the *same length* with a *different*
/// entry list.
pub(super) fn entry_url(app: &App, index: usize) -> Option<String> {
    app.interactive
        .as_ref()
        .unwrap()
        .nav_controller
        .entry(index)
        .map(|e| e.url.as_str().to_string())
}
