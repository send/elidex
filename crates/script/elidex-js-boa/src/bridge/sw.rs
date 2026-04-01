//! HostBridge methods for Service Worker state management.

use super::HostBridge;

/// Pending SW registration request (queued by navigator.serviceWorker.register()).
#[derive(Clone, Debug)]
pub struct SwRegisterRequest {
    pub script_url: String,
    /// Explicit scope from options, if provided.
    pub scope: Option<String>,
}

impl HostBridge {
    /// Queue a SW registration request (from navigator.serviceWorker.register()).
    ///
    /// The content thread drains these and sends `ContentToBrowser::SwRegister`.
    pub fn queue_sw_register(&self, script_url: String, scope: Option<String>) {
        let mut inner = self.inner.borrow_mut();
        inner
            .pending_sw_registers
            .push(SwRegisterRequest { script_url, scope });
    }

    /// Drain pending SW registration requests.
    pub fn drain_sw_register_requests(&self) -> Vec<SwRegisterRequest> {
        let mut inner = self.inner.borrow_mut();
        std::mem::take(&mut inner.pending_sw_registers)
    }

    /// Set the scope of the controlling SW for this page.
    pub fn set_sw_controller_scope(&self, scope: Option<url::Url>) {
        let mut inner = self.inner.borrow_mut();
        inner.sw_controller_scope = scope;
    }

    /// Get the scope of the controlling SW, if any.
    pub fn sw_controller_scope(&self) -> Option<url::Url> {
        let inner = self.inner.borrow();
        inner.sw_controller_scope.clone()
    }

    /// Enable the SW client message queue (WHATWG SW §3.4.6).
    ///
    /// Called by `startMessages()` or `onmessage` setter (first time only).
    /// Messages queued before this call will be delivered once enabled.
    pub fn enable_sw_messages(&self) {
        let mut inner = self.inner.borrow_mut();
        inner.sw_messages_enabled = true;
        // Drain any buffered messages — currently no buffering infrastructure,
        // so this is a flag for future message delivery integration.
    }

    /// Check if SW messages are enabled.
    pub fn sw_messages_enabled(&self) -> bool {
        let inner = self.inner.borrow();
        inner.sw_messages_enabled
    }

    /// Get the unique client ID (UUID v4) for this browsing context.
    pub fn client_id(&self) -> String {
        let inner = self.inner.borrow();
        inner.client_id.clone()
    }
}
