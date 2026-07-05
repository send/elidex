//! Navigation state methods for `HostBridge`.

use elidex_script_session::{HistoryAction, NavigationRequest};

use super::HostBridge;

impl HostBridge {
    /// Set the current page URL.
    ///
    /// Re-derives the URL-tuple `cached_origin` (the localStorage partition key)
    /// **only when the installed origin is not opaque**. On a fresh load
    /// `set_origin` runs right after and authoritatively syncs `cached_origin`
    /// from the installed origin (`iframe_bridge.rs`), so the tuple seed here is
    /// its provisional value; but a same-document (fragment) navigation
    /// (`content/navigation.rs::fragment_navigate`) calls this WITHOUT a
    /// following `set_origin` (no pipeline rebuild), so re-deriving from the URL
    /// would switch an installed OPAQUE origin's isolated `null:<id>` partition
    /// to the URL tuple — a sandbox origin-isolation bypass (§7.1.1 origin is
    /// stable document state; §12.2.3 storage partitions by document origin).
    /// Skipping the re-derive for an opaque origin preserves the partition; a
    /// tuple origin is unaffected by a fragment (only the fragment differs), so
    /// the re-derive is a harmless no-op there.
    pub fn set_current_url(&self, url: Option<url::Url>) {
        let mut inner = self.inner.borrow_mut();
        if !matches!(
            inner.iframe.origin,
            elidex_plugin::SecurityOrigin::Opaque(_)
        ) {
            inner.cached_origin = url
                .as_ref()
                .map_or("null".to_string(), |u| u.origin().ascii_serialization());
        }
        inner.current_url = url;
    }

    /// Get the current page URL.
    pub fn current_url(&self) -> Option<url::Url> {
        self.inner.borrow().current_url.clone()
    }

    /// Set a pending navigation request.
    pub fn set_pending_navigation(&self, request: NavigationRequest) {
        self.inner.borrow_mut().pending_navigation = Some(request);
    }

    /// Take (remove) the pending navigation request.
    pub fn take_pending_navigation(&self) -> Option<NavigationRequest> {
        self.inner.borrow_mut().pending_navigation.take()
    }

    /// Set a pending history action.
    pub fn set_pending_history(&self, action: HistoryAction) {
        self.inner.borrow_mut().pending_history = Some(action);
    }

    /// Take (remove) the pending history action.
    pub fn take_pending_history(&self) -> Option<HistoryAction> {
        self.inner.borrow_mut().pending_history.take()
    }

    /// Set the session history length.
    pub fn set_history_length(&self, len: usize) {
        self.inner.borrow_mut().history_length = len;
    }

    /// Get the session history length.
    pub fn history_length(&self) -> usize {
        self.inner.borrow().history_length
    }

    /// Set a URL to open in a new tab (from `window.open`).
    pub fn queue_open_tab(&self, url: url::Url) {
        self.inner.borrow_mut().iframe.pending_open_tabs.push(url);
    }

    /// Drain all pending new-tab URLs.
    pub fn drain_pending_open_tabs(&self) -> Vec<url::Url> {
        std::mem::take(&mut self.inner.borrow_mut().iframe.pending_open_tabs)
    }

    /// Queue a named-target iframe navigation from `window.open`.
    pub fn set_pending_navigate_iframe(&self, name: String, url: url::Url) {
        self.inner
            .borrow_mut()
            .iframe
            .pending_navigate_iframe
            .push((name, url));
    }

    /// Drain pending named-target iframe navigations.
    pub fn drain_pending_navigate_iframe(&self) -> Vec<(String, url::Url)> {
        std::mem::take(&mut self.inner.borrow_mut().iframe.pending_navigate_iframe)
    }

    /// Set a pending script dispatch event (from `dispatchEvent()`).
    pub fn set_pending_script_dispatch(&self, event: elidex_script_session::DispatchEvent) {
        self.inner.borrow_mut().pending_script_dispatch = Some(event);
    }

    /// Take (remove) the pending script dispatch event.
    pub fn take_pending_script_dispatch(&self) -> Option<elidex_script_session::DispatchEvent> {
        self.inner.borrow_mut().pending_script_dispatch.take()
    }
}
