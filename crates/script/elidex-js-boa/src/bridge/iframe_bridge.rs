//! Iframe and cross-document bridge methods for `HostBridge`.

use elidex_ecs::Entity;

use super::HostBridge;

impl HostBridge {
    /// Set the security origin for this document.
    pub fn set_origin(&self, origin: elidex_plugin::SecurityOrigin) {
        self.inner.borrow_mut().iframe.origin = origin;
    }

    /// Get the security origin of this document.
    #[must_use]
    pub fn origin(&self) -> elidex_plugin::SecurityOrigin {
        self.inner.borrow().iframe.origin.clone()
    }

    /// Set the `<iframe>` element entity in the parent DOM that contains this window.
    pub fn set_frame_element(&self, entity: Option<Entity>) {
        self.inner.borrow_mut().iframe.frame_element = entity;
    }

    /// Get the `<iframe>` element entity in the parent DOM.
    #[must_use]
    pub fn frame_element(&self) -> Option<Entity> {
        self.inner.borrow().iframe.frame_element
    }

    /// Set the iframe nesting depth of this document.
    pub fn set_iframe_depth(&self, depth: usize) {
        self.inner.borrow_mut().iframe.iframe_depth = depth;
    }

    /// Get the iframe nesting depth of this document (0 for top-level).
    #[must_use]
    pub fn iframe_depth(&self) -> usize {
        self.inner.borrow().iframe.iframe_depth
    }

    /// Set the referrer URL for this document (parent URL when loaded as iframe).
    pub fn set_referrer(&self, referrer: Option<String>) {
        self.inner.borrow_mut().iframe.referrer = referrer;
    }

    /// Get the referrer URL for this document.
    #[must_use]
    pub fn referrer(&self) -> Option<String> {
        self.inner.borrow().iframe.referrer.clone()
    }

    /// Set sandbox flags for this document (if inside a sandboxed iframe).
    pub fn set_sandbox_flags(&self, flags: Option<elidex_plugin::IframeSandboxFlags>) {
        self.inner.borrow_mut().iframe.sandbox_flags = flags;
    }

    /// Get sandbox flags for this document.
    #[must_use]
    pub fn sandbox_flags(&self) -> Option<elidex_plugin::IframeSandboxFlags> {
        self.inner.borrow().iframe.sandbox_flags
    }

    /// Check if scripts are allowed (sandbox allow-scripts flag).
    /// Returns `true` if not sandboxed or if allow-scripts is set.
    #[must_use]
    pub fn scripts_allowed(&self) -> bool {
        self.inner
            .borrow()
            .iframe
            .sandbox_flags
            .is_none_or(|f| f.contains(elidex_plugin::IframeSandboxFlags::ALLOW_SCRIPTS))
    }

    /// Check if forms are allowed (sandbox allow-forms flag).
    #[must_use]
    pub fn forms_allowed(&self) -> bool {
        self.inner
            .borrow()
            .iframe
            .sandbox_flags
            .is_none_or(|f| f.contains(elidex_plugin::IframeSandboxFlags::ALLOW_FORMS))
    }

    /// Check if popups are allowed (sandbox allow-popups flag).
    #[must_use]
    pub fn popups_allowed(&self) -> bool {
        self.inner
            .borrow()
            .iframe
            .sandbox_flags
            .is_none_or(|f| f.contains(elidex_plugin::IframeSandboxFlags::ALLOW_POPUPS))
    }

    /// Check if modals (alert/confirm/prompt) are allowed.
    #[must_use]
    pub fn modals_allowed(&self) -> bool {
        self.inner
            .borrow()
            .iframe
            .sandbox_flags
            .is_none_or(|f| f.contains(elidex_plugin::IframeSandboxFlags::ALLOW_MODALS))
    }

    /// Queue a postMessage for delivery in the next event loop tick.
    pub fn queue_post_message(&self, data: String, origin: String) {
        self.inner
            .borrow_mut()
            .iframe
            .pending_post_messages
            .push((data, origin));
    }

    /// Drain all queued postMessage events.
    pub fn drain_post_messages(&self) -> Vec<(String, String)> {
        std::mem::take(&mut self.inner.borrow_mut().iframe.pending_post_messages)
    }
}
