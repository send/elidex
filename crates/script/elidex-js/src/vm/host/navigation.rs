//! Navigation state — `Location` / `History` / `document.URL` / reload.
//!
//! # Single session-history source of truth = the shell
//!
//! A `Vm` is bound to one document and does not own the network/render
//! pipeline, so it **cannot** navigate itself — navigation replaces the whole
//! pipeline, which the shell owns. The session history of record is therefore
//! the shell's `NavigationController`; the VM keeps only a **current-document
//! view** ([`NavigationState`]) plus the **pending intent** buffers the shell
//! drains after a script turn (S1c boa→VM cutover, the back-channel slice).
//!
//! - `location.assign`/`href=`/`replace`/`reload` and `history.back`/`forward`/
//!   `go` are *enqueue-only* (WHATWG HTML §7.4.2.2 "Beginning navigation" /
//!   §7.4.6 "Applying the history step" — async loads the shell performs): they
//!   set [`NavigationState::pending_navigation`] / `pending_history` and do NOT
//!   mutate `current_url` (it commits when the shell calls `set_current_url`
//!   after the load — so `location.href = "/x"; location.href` reads the OLD URL,
//!   matching browsers).
//! - `history.pushState`/`replaceState` (HTML §7.2.5 "shared history push/replace
//!   state steps" → the "URL and history update steps" (§7.4.4)) are *synchronous*: the
//!   VM updates `current_url` + `current_state` in place (and `pushState` also
//!   bumps `history_length`) AND enqueues a `HistoryAction::PushState/ReplaceState`
//!   for the shell to persist.  Each one independently mutates the joint session
//!   history, so the enqueue buffer is a **FIFO queue** (two same-turn pushStates
//!   must both reach the shell), unlike the last-wins async `pending_navigation`.
//! - `history.length` / `history.state` read `history_length` (shell-pushed, plus
//!   the synchronous `pushState` bump the shell reconciles) / `current_state` (set
//!   by `pushState`/`replaceState`; a traversal leaves it untouched — async, the
//!   shell restores the target entry's state on commit).

#![cfg(feature = "engine")]

use std::collections::VecDeque;

use elidex_script_session::{
    HistoryAction, NamedFrameNavigation, NavigationRequest, OpenTabRequest,
};
use url::Url;

use super::super::value::JsValue;

/// Upper bound on the per-turn pending-history queue (memory safety).  A tight
/// `while (true) history.pushState(...)` loop appends one `HistoryAction` per
/// call, and the shell only drains + applies its `NavigationController` cap
/// *after* the script turn — so without a VM-side bound a runaway loop could grow
/// memory without limit before the controller ever evicts.  Beyond this many
/// buffered actions the oldest is dropped (the session history keeps only its
/// most-recent entries on drain regardless), keeping the queue — and the last
/// entry, which matches the synchronously-updated `current_url` — bounded.  Set
/// well above any legitimate per-turn history-mutation count.
const MAX_PENDING_HISTORY_ACTIONS: usize = 1024;

/// Upper bound on the session-history entry count the synchronous `pushState`
/// view reports — a **best-effort estimate** that must track the shell's
/// `NavigationController` cap (`elidex-navigation`'s `MAX_HISTORY_ENTRIES`, 50).
/// The session history is bounded: over the cap the shell evicts the oldest
/// entry (HTML §7.2.5 note — a FIFO eviction buffer), so a tight `pushState`
/// loop must report the *capped* `history.length`, not an unbounded `5001` that
/// collapses to `50` the moment the shell drains.  The VM cannot depend on
/// `elidex-navigation` (a shell crate) — this duplicate is the deliberate
/// cross-layer estimate; the shell stays authoritative and reconciles the exact
/// `(index, length)` via
/// [`HostDriver::set_session_history`](elidex_script_session::HostDriver::set_session_history),
/// so a drift between the two constants only perturbs the within-turn estimate.
const SESSION_HISTORY_CAP: usize = 50;

/// Upper bound shared by the two per-turn `window.open` back-channel queues
/// ([`NavigationState::pending_open_tabs`] /
/// [`NavigationState::pending_frame_navigations`]) — the popup-spam clamp.
/// Unlike [`MAX_PENDING_HISTORY_ACTIONS`] (which evicts the OLDEST so the
/// queue's tail keeps matching the synchronously-updated `current_url`),
/// overflow here drops the **new** entry: each queued item is an independent
/// "open a window" effect with no newest-wins state to preserve, so refusing
/// further popups once a turn has buffered this many is the UA-conservative
/// choice — a runaway `while (true) window.open(...)` loop stops adding work
/// instead of silently rotating which popups survive.
const MAX_PENDING_WINDOW_OPENS: usize = 1024;

/// Per-`Vm` navigation state — the **current-document view** of the shell-owned
/// session history (see the module docs). Not a session-history stack: the
/// shell's `NavigationController` is the single source of truth.
///
/// These fields are a per-VM browsing-context interim. Under B1 they migrate to ECS
/// components at each field's spec-correct grain, per the decision's grain rule (PR
/// #434 §5 req 5; the per-field classification is the B1 plan-memo's, not asserted
/// here). `pending_navigation` / `pending_history` are transient drain-once intent
/// buffers; under B1 — where one `Vm` hosts several same-agent navigables — a
/// `location` / `history` action targets the *incumbent* navigable, so these are
/// **per-navigable** intents (a single VM-global buffer would let one frame/popup
/// drain a sibling's intent), and their exact keying is part of that same B1 grain
/// classification, not asserted per-VM here.
/// ⚠ SUPERSEDED 2026-06-30: this slot is FOLDED into the agent-scoped World
/// decision (PR #434 §5 req 5 / §6.1).
#[derive(Debug)]
pub(crate) struct NavigationState {
    /// The current browsing-context URL.  Backs `location.*`, `document.URL`,
    /// and `document.documentURI`.  Initialised to `about:blank` per WHATWG HTML
    /// §3.1.1 "The Document object" (the "is initial about:blank" concept; a
    /// browsing context always has an active document with a URL).  Held as
    /// [`Url`] so location getters call the
    /// WHATWG parser directly and relative setters use [`Url::join`].
    ///
    /// Committed by the shell's `set_current_url` after a navigation load, or
    /// synchronously by `pushState`/`replaceState` (§7.4.4). NOT mutated by the
    /// enqueue-only `assign`/`href=`/`replace`/`traverse` paths.
    pub(crate) current_url: Url,
    /// `history.length` — the count of session-history entries.  The shell's
    /// `NavigationController` owns the real count and pushes it (with the index,
    /// atomically) via `set_session_history` after a navigation/traversal commit;
    /// `pushState` also updates it synchronously to [`Self::current_index`] `+ 1`
    /// (§7.4.4 — a new entry is appended at the end in the same script turn, so a
    /// same-turn `history.length` read observes it).  Defaults to `1` (the
    /// spec-minimum: the current entry always exists).
    pub(crate) history_length: usize,
    /// The 0-based index of the current entry within the session history.  Pushed
    /// by the shell (with `history_length`, atomically) via `set_session_history`,
    /// and incremented synchronously by `pushState` (which appends after the
    /// current entry, discarding forward entries — so the new length is
    /// `current_index + 1`, **not** `history_length + 1`; the latter would
    /// over-count when the current entry is not the last, e.g. after a `back`).
    /// `replaceState` and traversals leave it unchanged (traversals commit
    /// async; the shell re-pushes both on commit).  Defaults to `0` (the single
    /// current entry).  Not exposed to script — internal to the synchronous
    /// length update.
    pub(crate) current_index: usize,
    /// `history.state` — the serialized state of the current session-history
    /// entry.  Set synchronously by `pushState`/`replaceState` (HTML §7.4.4).  A
    /// traversal (`back`/`forward`/`go`) leaves it **untouched**: the traversal
    /// is async (the shell loads the target entry), so a same-turn read still
    /// sees the current entry's state, and a no-op traversal (`back` at the first
    /// entry, `go(0)`) keeps it unchanged.  The target entry's state restoration
    /// on commit needs the shell back-channel (slot
    /// `#11-history-state-traversal-popstate-fidelity`).
    ///
    /// Held as a bare [`JsValue`] — `StructuredSerializeForStorage` (§7.2.5
    /// step 4) is part of the same deferred slot.  GC-rooted via the
    /// `gc::roots` visit so a `pushState`'d object is not collected before a
    /// later `history.state` read.
    pub(crate) current_state: JsValue,
    /// A pending navigation from `location.assign`/`href=`/`replace`/`reload`
    /// (WHATWG HTML §7.4.2.2), drained once per script turn by the shell's
    /// `take_pending_navigation`.  Single-slot last-wins (matches boa).
    pub(crate) pending_navigation: Option<NavigationRequest>,
    /// Pending history actions from `history.back`/`forward`/`go`/`pushState`/
    /// `replaceState` (WHATWG HTML §7.2.5), drained once per script turn by the
    /// shell's `take_pending_history`.  A **FIFO queue**, not a single slot:
    /// `pushState`/`replaceState` are *synchronous* and each independently
    /// mutates the joint session history (§7.4.4), so two in one turn
    /// (`pushState('/a'); pushState('/b')`) must both reach the shell in order —
    /// a last-wins slot would silently drop `/a`'s entry.  (Contrast
    /// `pending_navigation`, single-slot last-wins: navigations are async and
    /// supersede one another.)  Bounded at [`MAX_PENDING_HISTORY_ACTIONS`] so a
    /// runaway `pushState` loop cannot grow memory unbounded before the shell
    /// drains — but at the bound it evicts the oldest **`PushState`/`ReplaceState`**
    /// (which the shell's session-history cap would drop anyway), **never a
    /// traversal**, so a `back()` followed by a flood of pushes does not silently
    /// lose the traversal and reorder the operation sequence the shell replays
    /// (see [`Self::enqueue_history`]).
    pub(crate) pending_history: VecDeque<HistoryAction>,
    /// Gate-passed `window.open` popup / `_blank` requests (WHATWG HTML
    /// §7.2.2.1 window open steps → §7.3.1.7 step 8's
    /// create-a-new-top-level-traversable case), drained per turn by the
    /// shell's `take_pending_open_tabs`.  A **FIFO queue** like
    /// [`Self::pending_history`]: several `window.open` calls in one task
    /// each request an independent new tab and must all surface.  The
    /// enqueue is popup-gated at the `window.open` native (a sandbox-blocked
    /// popup never enters the queue — security by structure), and the queue
    /// is a transient per-browsing-context *event queue* of work items, not
    /// a per-entity fact (the `pending_history` shape), so it stays VM-side
    /// under B1 unchanged.  Bounded at [`MAX_PENDING_WINDOW_OPENS`]
    /// (overflow drops the NEW entry — see the const).
    pub(crate) pending_open_tabs: VecDeque<OpenTabRequest>,
    /// Named-target `window.open` navigations (WHATWG HTML §7.3.1.7),
    /// drained per turn by the shell's `take_pending_frame_navigations`.
    /// FIFO + bounded like [`Self::pending_open_tabs`].  Each entry carries
    /// the call-time §7.3.1.7 step-3 sandboxing-flag-set snapshot
    /// (`aux_nav_allowed`) the shell's named-MISS → new-tab promotion
    /// consults instead of re-reading live flags.
    pub(crate) pending_frame_navigations: VecDeque<NamedFrameNavigation>,
    /// URL of the previous Document, used to back `document.referrer` (WHATWG
    /// HTML §3.1.4 "Resource metadata management").  `None` when no previous
    /// Document is recorded — the spec maps this to the empty string at the JS
    /// surface.  [`super::super::Vm::set_navigation_referrer`] is the only
    /// writer; the VM never populates this field on its own.
    pub(crate) referrer: Option<Url>,
}

/// Parse `"about:blank"` once at construction — a panic here would
/// indicate a broken `url` crate build (the literal is WHATWG-valid).
/// Also used by the `window.open` native (`super::window`): an empty `url`
/// argument opens `about:blank` (§7.2.2.1 window open steps step 15.3).
pub(super) fn parse_about_blank() -> Url {
    Url::parse("about:blank").expect("`about:blank` must parse as a WHATWG URL")
}

impl NavigationState {
    /// Create a fresh navigation state pointing at `about:blank`, with an empty
    /// current-document view (history length 1 = the current entry, null state,
    /// no pending intents).
    pub(crate) fn new() -> Self {
        Self {
            current_url: parse_about_blank(),
            history_length: 1,
            current_index: 0,
            current_state: JsValue::Null,
            pending_navigation: None,
            pending_history: VecDeque::new(),
            pending_open_tabs: VecDeque::new(),
            pending_frame_navigations: VecDeque::new(),
            referrer: None,
        }
    }

    /// Commit a navigation's URL — the shell calls this (via
    /// [`HostDriver::set_current_url`](elidex_script_session::HostDriver::set_current_url))
    /// after a load completes.  `None` resets to `about:blank` (the spec's "no
    /// active document" maps to the initial `about:blank`).
    pub(crate) fn set_current_url(&mut self, url: Option<Url>) {
        self.current_url = url.unwrap_or_else(parse_about_blank);
    }

    /// Enqueue a navigation intent for the shell (last-wins single slot, matching
    /// boa).  The enqueue-only `location` setters route through here so they
    /// never mutate `current_url` in place (the navigation commits when the shell
    /// loads the document and calls `set_current_url`).
    pub(crate) fn enqueue_navigation(&mut self, request: NavigationRequest) {
        self.pending_navigation = Some(request);
    }

    /// Enqueue a history action for the shell, appending to the FIFO queue (so a
    /// turn's synchronous `pushState`/`replaceState` mutations all reach the shell
    /// in order — see [`Self::pending_history`]).  Used by `back`/`forward`/`go`
    /// (pure intent) and by `pushState`/`replaceState` (after their synchronous
    /// URL+state update).  Bounded at [`MAX_PENDING_HISTORY_ACTIONS`]: a runaway
    /// loop stays at the cap, and the newest action — matching the synchronously
    /// updated `current_url` — is always retained.
    ///
    /// At the bound it evicts the oldest **evictable** action — a
    /// `PushState`/`ReplaceState`, which the shell's session-history cap would
    /// drop anyway, so the survivors are the same last-N the shell commits.  A
    /// traversal (`Back`/`Forward`/`Go`) is **not** evictable: dropping it would
    /// change the ordered operation sequence the shell replays (`back(); pushState
    /// ×N` must still go back first), so it is preserved.  Only if every queued
    /// action is a traversal — pathological — does eviction fall back to the front.
    pub(crate) fn enqueue_history(&mut self, action: HistoryAction) {
        if self.pending_history.len() >= MAX_PENDING_HISTORY_ACTIONS {
            let evictable = self.pending_history.iter().position(|a| {
                matches!(
                    a,
                    HistoryAction::PushState { .. } | HistoryAction::ReplaceState { .. }
                )
            });
            match evictable {
                Some(pos) => {
                    self.pending_history.remove(pos);
                }
                None => {
                    self.pending_history.pop_front();
                }
            }
        }
        self.pending_history.push_back(action);
    }

    /// Enqueue a gate-passed `window.open` popup / `_blank` request for the
    /// shell (FIFO — see [`Self::pending_open_tabs`]).  The caller (the
    /// `window.open` native) has already run the popup gate: only an
    /// [`elidex_script_session::WindowOpenDisposition::OpenTab`] disposition
    /// reaches here.  At [`MAX_PENDING_WINDOW_OPENS`] the NEW request is
    /// dropped (refuse further popups — the spam clamp; see the const).
    pub(crate) fn enqueue_open_tab(&mut self, request: OpenTabRequest) {
        if self.pending_open_tabs.len() >= MAX_PENDING_WINDOW_OPENS {
            return;
        }
        self.pending_open_tabs.push_back(request);
    }

    /// Enqueue a named-target `window.open` navigation for the shell (FIFO —
    /// see [`Self::pending_frame_navigations`]), carrying the call-time
    /// `aux_nav_allowed` snapshot.  Same overflow policy as
    /// [`Self::enqueue_open_tab`].
    pub(crate) fn enqueue_frame_navigation(&mut self, request: NamedFrameNavigation) {
        if self.pending_frame_navigations.len() >= MAX_PENDING_WINDOW_OPENS {
            return;
        }
        self.pending_frame_navigations.push_back(request);
    }

    /// Advance the current-document view for a synchronous `pushState` append
    /// (the "URL and history update steps", §7.4.4): move to the newly-appended
    /// entry (`current_index += 1`) and recompute `history_length = index + 1`
    /// (the new entry is now the last).  Saturates at [`SESSION_HISTORY_CAP`] — a
    /// tight loop reports the capped count, matching what the shell commits after
    /// eviction (HTML §7.2.5 note), not an unbounded value that collapses on
    /// drain.  `replaceState` does NOT call this (it overwrites the current entry
    /// in place, changing neither index nor length).
    pub(crate) fn record_push_state(&mut self) {
        self.current_index = self
            .current_index
            .saturating_add(1)
            .min(SESSION_HISTORY_CAP - 1);
        self.history_length = self.current_index + 1;
    }
}

impl super::super::VmInner {
    /// The document's security origin (WHATWG HTML §7.1.1) — the canonical
    /// value every *settings-object-origin* surface serializes.
    ///
    /// Returns the embedder-installed override
    /// ([`super::super::host_data::HostData::set_origin`]) when present —
    /// opaque for a sandboxed iframe, so the document reports `"null"` — and
    /// otherwise derives it from [`NavigationState::current_url`] (the spec
    /// default: a document's origin is its URL's origin unless overridden).
    /// This is the single resolution point the **window.postMessage**
    /// (§9.3.3) / **WebSocket** (WebSockets §2.2) / **EventSource** (§9.2.2)
    /// `Origin` / **localStorage** (§12.2.3) readers consume, so none of them
    /// re-derives `current_url.origin()` ad hoc (the S1b §5 unification).
    ///
    /// NB `location.origin` does **not** read this — HTML §7.2.4 returns the
    /// Location *URL's* origin, which differs from the document origin for a
    /// sandboxed doc (it stays `current_url`-derived).
    ///
    /// **Idempotency contract.** The returned value is identity-stable in every
    /// state (a document's origin is stable document state, HTML §7.1.1): an
    /// installed override returns the stored `SecurityOrigin`; a tuple
    /// `current_url` derives deterministically (`from_url` is stable for
    /// http/https); and the no-override **opaque** fallback returns the per-VM
    /// [`HostData::fallback_opaque_origin`](super::super::host_data::HostData::fallback_opaque_origin)
    /// (minted once) rather than a fresh `Opaque(n)` per call. This matters for
    /// the standalone / `about:blank` pipeline path (`current_url: None` → the
    /// shell never calls `set_origin`): `iframe/lifecycle.rs` reads
    /// `bridge().origin()` and propagates it parent→child, so a re-minting
    /// fallback would hand the child a different origin on each read. A bare
    /// engine with no `HostData` cannot store the fallback, so it keeps a fresh
    /// opaque — but it has no propagating consumer and serializes to `"null"`
    /// either way.
    ///
    /// The iframe pipeline must install the override **before** running a
    /// frame's initial scripts — and the shell now does, on ALL iframe paths
    /// (in-process and out-of-process, including the OOP thread's `Navigate`
    /// re-build): the security state rides the pipeline build as
    /// `elidex-shell` `PreEvalFrameState` and is installed at the
    /// `run_scripts_and_finalize` pre-eval chokepoint, so a sandboxed
    /// iframe's first script reads its opaque `"null"` origin, never the
    /// fallback / parent origin (S5-4b, closed slot
    /// `#11-iframe-origin-before-initial-scripts`). At the S5-6 flip the VM
    /// inherits that ordering unchanged — the contract stays: install the
    /// override before the first eval.
    ///
    /// Relatedly, a *tuple* override installed at load is pinned for the
    /// document's lifetime. S1c makes `location` navigation enqueue-only (no
    /// in-place `current_url` mutation), so the in-VM origin-staleness root is
    /// gone; the remaining work — the shell re-pushing `set_origin` alongside
    /// `set_current_url` after a content-thread navigation (`content/navigation.rs`
    /// commits the URL without re-deriving origin) — is shell-side at the S5 flip
    /// → slot `#11-vm-navigation-origin-resync`.
    pub(crate) fn document_origin(&self) -> elidex_plugin::SecurityOrigin {
        let host_data = self.host_data.as_deref();
        if let Some(over) =
            host_data.and_then(super::super::host_data::HostData::document_origin_override)
        {
            return over.clone();
        }
        match elidex_plugin::SecurityOrigin::from_url(&self.navigation.current_url) {
            // Pin the no-override opaque fallback to the per-VM stable opaque
            // (HTML §7.1.1 — origin is stable document state; matches boa's
            // single stored default). Tuple origins from `current_url` are
            // already deterministic and pass through unchanged.
            opaque @ elidex_plugin::SecurityOrigin::Opaque(_) => {
                host_data.map_or(opaque, |hd| hd.fallback_opaque_origin().clone())
            }
            tuple => tuple,
        }
    }
}
