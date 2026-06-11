//! Navigation state — `Location` / `History` / `document.URL` / reload.
//!
//! The VM owns a single [`NavigationState`] per `Vm`, read and written
//! by the `location` / `history` host globals (PR4b C6 / C7).  Until the
//! shell integration lands (PR6), the state is purely VM-internal:
//! assignments to `location.href` and `history.pushState` update the
//! current URL and the history entry list in place, but do not trigger
//! an actual browser navigation.  `history.back()` / `history.forward()`
//! move within the in-memory stack.
//!
//! WHATWG HTML §7.4 "The History interface" uses a structured clone
//! for `state`, but at this tier we store it as a bare [`JsValue`] so
//! the getter round-trip is identity — structured-clone serialisation
//! lands alongside the shell navigation bridge.

#![cfg(feature = "engine")]

use url::Url;

use super::super::value::JsValue;

/// Maximum number of session-history entries retained by the
/// in-memory [`NavigationState`].  Matches Chrome / Firefox's
/// approximate cap.  When `push_entry` would exceed the limit,
/// the oldest entry is evicted and [`NavigationState::history_index`]
/// shifts accordingly — this keeps pathological
/// `for (;;) history.pushState(...)` loops from growing the `Vec`
/// (and its GC-rooted `state: JsValue` slots) unbounded.
pub(crate) const MAX_HISTORY_ENTRIES: usize = 50;

/// A single entry in [`NavigationState::history_entries`] (WHATWG HTML
/// §7.4.1 "session history entry").
#[derive(Clone, Debug)]
pub(crate) struct HistoryEntry {
    /// The URL the entry points at.  Held as [`Url`] so that
    /// relative-URL resolution (`history.pushState(…, '/new')`) is
    /// a WHATWG-conformant `base.join(input)` call.
    pub(crate) url: Url,
    /// The serialised state associated with the entry (`history.state`).
    ///
    /// Held as a bare [`JsValue`] — structured clone happens when the
    /// shell navigation bridge is wired (PR6).  GC-roots this entry's
    /// values are traced because `NavigationState` lives inside
    /// `VmInner` and is iterated by the GC root set (see `gc.rs`
    /// `NavigationState` visit when wired).
    pub(crate) state: JsValue,
}

/// Per-`Vm` navigation state.
///
/// Phase 2 scope — the fields are read by the `location` / `history`
/// host objects and written by their setters.  Shell navigation
/// coupling (actual URL loads, popstate firing) is deferred to PR6 per
/// the PR4b plan.
#[derive(Debug)]
pub(crate) struct NavigationState {
    /// The current browsing-context URL.  Backs `location.href`,
    /// `document.URL`, and `document.documentURI`.  Initialised to
    /// `about:blank` per WHATWG HTML §7.3.3 "Creating documents".
    /// Held as [`Url`] so location getters call the WHATWG parser
    /// directly (scheme / host / port / path / query / fragment
    /// accessors) and relative URL setters (`location.href = "foo"`)
    /// use [`Url::join`] for base-relative resolution.
    pub(crate) current_url: Url,
    /// The in-memory session history stack.
    pub(crate) history_entries: Vec<HistoryEntry>,
    /// The index of the current entry within [`Self::history_entries`].
    /// Always a valid index (invariant: `history_entries` is non-empty
    /// after construction).
    pub(crate) history_index: usize,
    /// URL of the previous Document, used to back
    /// `document.referrer` (WHATWG HTML §3.1.5).  `None` when no
    /// previous Document is recorded — the spec maps this to the
    /// empty string at the JS surface (directly-loaded top-level
    /// navigations, opened-without-opener windows, and reloads
    /// where the referrer policy stripped the previous URL).
    /// [`super::super::Vm::set_navigation_referrer`] is the only
    /// writer; the VM never populates this field on its own.
    pub(crate) referrer: Option<Url>,
}

/// Parse `"about:blank"` once at construction — a panic here would
/// indicate a broken `url` crate build (the literal is WHATWG-valid).
fn parse_about_blank() -> Url {
    Url::parse("about:blank").expect("`about:blank` must parse as a WHATWG URL")
}

impl NavigationState {
    /// Create a fresh navigation state pointing at `about:blank`.
    pub(crate) fn new() -> Self {
        let initial_url = parse_about_blank();
        Self {
            current_url: initial_url.clone(),
            history_entries: vec![HistoryEntry {
                url: initial_url,
                state: JsValue::Null,
            }],
            history_index: 0,
            referrer: None,
        }
    }

    /// Push a new entry (truncating any forward history) and apply
    /// the [`MAX_HISTORY_ENTRIES`] cap by dropping the oldest entry
    /// when the vec would otherwise exceed the limit.  Returns the
    /// new index for convenience.
    ///
    /// Called by `location.assign` / `location.href=` / `history.pushState`.
    pub(crate) fn push_entry(&mut self, url: Url, state: JsValue) -> usize {
        self.history_entries.truncate(self.history_index + 1);
        self.history_entries.push(HistoryEntry { url, state });
        if self.history_entries.len() > MAX_HISTORY_ENTRIES {
            // Drop the oldest; shift the index down to keep pointing
            // at the just-pushed entry.  Worst case this is O(len),
            // but `len == MAX_HISTORY_ENTRIES+1` so it's bounded.
            self.history_entries.remove(0);
        }
        self.history_index = self.history_entries.len() - 1;
        self.history_index
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
