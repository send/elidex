//! Embedder-installed browsing-context configuration on `HostData`.
//!
//! The Web Storage / Cache API backends, the cookie jar, the sandboxing
//! flag set, the document-origin override and opaque fallbacks, the
//! iframe depth and page visibility — state the shell installs and the
//! natives read.
//! Carved out of `host_data/mod.rs` as a cohesion seam (pure move).

use super::super::super::host::cache::CacheBackend;
use super::HostData;
#[cfg(feature = "compat-webapi")]
use elidex_storage_core::WebStorageManager;
use std::sync::Arc;

impl HostData {
    /// Install the shell-owned `WebStorageManager` (idempotent
    /// replace).  Tests that exercise `localStorage` persistence
    /// across VM lifetimes pass a `tempfile::tempdir()`-based
    /// manager here; production embedders share one
    /// `Arc<WebStorageManager>` across browsing-context VMs.
    #[cfg(feature = "compat-webapi")]
    pub fn install_web_storage(&mut self, manager: Arc<WebStorageManager>) {
        self.web_storage = Some(manager);
    }

    /// Borrow the installed `WebStorageManager`, if any.  Used by
    /// the `vm/host/storage.rs` natives to route `getItem` /
    /// `setItem` / `removeItem` / `clear` / `length` / `key` for
    /// `localStorage`.  Returns `None` when no backend is
    /// installed; callers fall back to
    /// [`Self::fallback_local_storage`] in that case.
    #[cfg(feature = "compat-webapi")]
    pub(crate) fn web_storage(&self) -> Option<&Arc<WebStorageManager>> {
        self.web_storage.as_ref()
    }

    /// Install the shell-owned origin Cache API backend (DR-A,
    /// `#11-cache-api-vm`).  Production embedders share one
    /// `Arc<CacheBackend>` per origin across browsing-context VMs (and,
    /// in PR-2, the service-worker thread) so the `caches` store is
    /// consistent within a session; tests pass an in-memory one.
    /// Idempotent replace.
    ///
    /// `pub(crate)` for PR-1 (window-realm only — the lazy in-memory
    /// fallback in `VmInner::ensure_cache_backend` is the sole caller).
    /// PR-2 / D-26 promotes this to `pub` + re-exports `CacheBackend`
    /// when the shell wires the origin-shared handle across the
    /// service-worker spawn boundary (§4.1).
    pub(crate) fn install_cache_storage(&mut self, backend: Arc<CacheBackend>) {
        self.cache_backend = Some(backend);
    }

    /// Borrow the installed Cache API backend, if any.  `None` until the
    /// shell installs one or [`super::super::super::VmInner::ensure_cache_backend`]
    /// lazily mints an in-memory fallback.
    pub(crate) fn cache_backend(&self) -> Option<&Arc<CacheBackend>> {
        self.cache_backend.as_ref()
    }

    /// Stable per-VM opaque-origin string (e.g. `"opaque-origin:7"`).
    /// The identity-preserving storage-partition key for an opaque document,
    /// shared by `localStorage` scoping and the IndexedDB versionchange
    /// broadcast key (`VmInner::storage_origin_key`).
    pub(crate) fn opaque_origin_sentinel(&self) -> &str {
        &self.opaque_origin_sentinel
    }

    /// Install the shell-owned cookie jar.  Idempotent —
    /// installing the same `Arc` twice is fine; replacing an
    /// existing jar swaps the pointer (cookies in the previous
    /// jar are not migrated).  Tests typically call this with a
    /// fresh `CookieJar::new()` after `bind_vm` so
    /// `document.cookie` round-trips can be observed.
    pub fn install_cookie_jar(&mut self, jar: std::sync::Arc<elidex_net::CookieJar>) {
        self.cookie_jar = Some(jar);
    }

    /// Borrow the installed cookie jar, or `None` when the shell
    /// did not call [`Self::install_cookie_jar`] (cookie-averse
    /// fallback, WHATWG HTML §3.1.4).
    pub(crate) fn cookie_jar(&self) -> Option<&std::sync::Arc<elidex_net::CookieJar>> {
        self.cookie_jar.as_ref()
    }

    /// Install this document's sandboxing flag set (WHATWG HTML
    /// §7.1.5).  Pass `SandboxingFlagSet::unsandboxed()` for a
    /// top-level / unsandboxed document (the default).  The shell parses
    /// the iframe `sandbox=""` attribute with
    /// `parse_sandboxing_directive` and installs the set here when a
    /// document loads inside a sandboxed iframe.
    pub fn set_sandbox_flags(&mut self, flags: elidex_plugin::SandboxingFlagSet) {
        self.sandbox_flags = flags;
    }

    /// Whether scripting is enabled for this browsing context
    /// (WHATWG HTML §8.1.3.4 "scripting is disabled" — gated by the
    /// §7.1.5 sandboxed scripts flag).  `true` when not sandboxed or
    /// when the sandbox grants `allow-scripts`; the eval gate
    /// short-circuits to a silent success when this is `false`.
    /// Delegates to the canonical predicate home
    /// [`elidex_plugin::sandbox`].
    pub(crate) fn scripts_allowed(&self) -> bool {
        elidex_plugin::sandbox::scripts_allowed(self.sandbox_flags)
    }

    /// The sandboxing flag set for this document's browsing context
    /// (`unsandboxed()` for top-level / unsandboxed).  Read by the shell
    /// (via the `ElidexJsEngine` forwarder) to gate `target="_blank"`
    /// navigation; the per-capability predicates below are the
    /// preferred entry points.
    pub(crate) fn sandbox_flags(&self) -> elidex_plugin::SandboxingFlagSet {
        self.sandbox_flags
    }

    /// Whether form submission is allowed (sandbox `allow-forms`; WHATWG
    /// HTML §7.1.5).  Delegates to the canonical predicate home
    /// [`elidex_plugin::sandbox`], like [`Self::scripts_allowed`].
    pub(crate) fn forms_allowed(&self) -> bool {
        elidex_plugin::sandbox::forms_allowed(self.sandbox_flags)
    }

    /// Whether popups are allowed (sandbox `allow-popups` = the §7.1.5
    /// *sandboxed auxiliary navigation* flag; WHATWG HTML §7.1.5).
    /// Delegates to the canonical predicate home
    /// [`elidex_plugin::sandbox`].
    pub(crate) fn popups_allowed(&self) -> bool {
        elidex_plugin::sandbox::popups_allowed(self.sandbox_flags)
    }

    /// Whether simple dialogs are allowed (sandbox `allow-modals` = the
    /// §7.1.5 *sandboxed modals flag*; WHATWG HTML §7.1.5).  Delegates to
    /// the canonical predicate home [`elidex_plugin::sandbox`].  Read by
    /// the §8.9.1 *cannot show simple dialogs* step-1 gate in the
    /// `alert` / `confirm` / `prompt` natives.
    pub(crate) fn modals_allowed(&self) -> bool {
        elidex_plugin::sandbox::modals_allowed(self.sandbox_flags)
    }

    /// Install the document's security origin (WHATWG HTML §7.1.1).  The
    /// embedder's load path computes it (`SecurityOrigin::from_url`, or the
    /// opaque sandbox origin via the shell's `apply_sandbox_origin`)
    /// and installs it before scripts run; [`super::VmInner::document_origin`]
    /// reads it (falling back to the `current_url`-derived origin when unset).
    pub(crate) fn set_origin(&mut self, origin: elidex_plugin::SecurityOrigin) {
        self.document_origin_override = Some(origin);
    }

    /// The installed document-origin override, if any.  `None` ⇒ the
    /// resolver derives the origin from `current_url`
    /// (see [`super::VmInner::document_origin`]).
    pub(crate) fn document_origin_override(&self) -> Option<&elidex_plugin::SecurityOrigin> {
        self.document_origin_override.as_ref()
    }

    /// The per-VM stable opaque fallback origin (see the field docs) — used
    /// by [`super::VmInner::document_origin`] for the no-override +
    /// opaque-`current_url` case so the origin stays identity-stable.
    pub(crate) fn fallback_opaque_origin(&self) -> &elidex_plugin::SecurityOrigin {
        &self.fallback_opaque_origin
    }

    /// Set the iframe nesting depth (`0` = top-level).
    pub(crate) fn set_iframe_depth(&mut self, depth: usize) {
        self.iframe_depth = depth;
    }

    /// The iframe nesting depth of this document's browsing context.
    pub(crate) fn iframe_depth(&self) -> usize {
        self.iframe_depth
    }

    /// Set the page-visibility state (WHATWG HTML §6.2), driven by the
    /// embedding shell on tab show/hide.  `visible = false` ⇒ hidden.
    pub(crate) fn set_visibility(&mut self, visible: bool) {
        self.tab_hidden = !visible;
    }

    /// Whether this document's top-level browsing context is hidden
    /// (WHATWG HTML §6.2) — backs `document.hidden`.
    pub(crate) fn is_tab_hidden(&self) -> bool {
        self.tab_hidden
    }
}
