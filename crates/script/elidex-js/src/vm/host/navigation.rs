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
//!   state steps" → §7.4.4 "URL and history update steps") are *synchronous*: the
//!   VM updates `current_url` + `current_state` in place AND enqueues a
//!   `HistoryAction::PushState/ReplaceState` for the shell to persist.
//! - `history.length` / `history.state` read the shell-pushed `history_length` /
//!   the synchronously-maintained `current_state`.

#![cfg(feature = "engine")]

use elidex_script_session::{HistoryAction, NavigationRequest};
use url::Url;

use super::super::value::JsValue;

/// Per-`Vm` navigation state — the **current-document view** of the shell-owned
/// session history (see the module docs). Not a session-history stack: the
/// shell's `NavigationController` is the single source of truth.
///
/// These fields are a per-VM browsing-context interim. `current_url` /
/// `history_length` / `current_state` are per-Document facts whose ECS-native
/// ideal home is a per-entity component on the document entity (deferred slice
/// `#11-browsing-context-state-ecs-components`); `pending_navigation` /
/// `pending_history` are transient drain-once intent buffers that are per-VM by
/// nature (a VM↔shell message channel, not per-entity state — boa stores them
/// identically on its `HostBridge`).
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
    /// `NavigationController` owns the real count and pushes it via
    /// `set_history_length`; defaults to `1` (the spec-minimum: the current
    /// entry always exists).
    pub(crate) history_length: usize,
    /// `history.state` — the serialized state of the current session-history
    /// entry.  Set synchronously by `pushState`/`replaceState` (HTML §7.4.4) and
    /// reset to `Null` on a traversal (`back`/`forward`/`go`), whose target
    /// entry's state restoration needs the shell back-channel (slot
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
    /// A pending history action from `history.back`/`forward`/`go`/`pushState`/
    /// `replaceState` (WHATWG HTML §7.2.5), drained once per script turn by the
    /// shell's `take_pending_history`.  Single-slot last-wins (matches boa).
    pub(crate) pending_history: Option<HistoryAction>,
    /// URL of the previous Document, used to back `document.referrer` (WHATWG
    /// HTML §3.1.4 "Resource metadata management").  `None` when no previous
    /// Document is recorded — the spec maps this to the empty string at the JS
    /// surface.  [`super::super::Vm::set_navigation_referrer`] is the only
    /// writer; the VM never populates this field on its own.
    pub(crate) referrer: Option<Url>,
}

/// Parse `"about:blank"` once at construction — a panic here would
/// indicate a broken `url` crate build (the literal is WHATWG-valid).
fn parse_about_blank() -> Url {
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
            current_state: JsValue::Null,
            pending_navigation: None,
            pending_history: None,
            referrer: None,
        }
    }

    /// Commit a navigation's URL — the shell calls this (via
    /// [`ElidexJsEngine::set_current_url`](crate::ElidexJsEngine::set_current_url))
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

    /// Enqueue a history action for the shell (last-wins single slot, matching
    /// boa).  Used by `back`/`forward`/`go` (pure intent) and by
    /// `pushState`/`replaceState` (after their synchronous URL+state update).
    pub(crate) fn enqueue_history(&mut self, action: HistoryAction) {
        self.pending_history = Some(action);
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
    /// At the S5 flip the iframe pipeline must install the override **before**
    /// running a frame's initial scripts: `iframe/load.rs` currently builds the
    /// pipeline (which runs scripts) before `make_in_process_entry` calls
    /// `set_origin`, so a sandboxed iframe's first script would read the
    /// fallback / parent origin instead of its opaque `"null"`. This is a
    /// pre-existing shell-ordering gap shared with the live boa path (no S1b
    /// regression) → slot `#11-iframe-origin-before-initial-scripts`.
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
