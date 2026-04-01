//! HostBridge methods for Service Worker state management.

use super::HostBridge;

/// Pending SW registration request (queued by navigator.serviceWorker.register()).
#[derive(Clone, Debug)]
pub struct SwRegisterRequest {
    pub script_url: String,
}

impl HostBridge {
    /// Queue a SW registration request (from navigator.serviceWorker.register()).
    ///
    /// The content thread drains these and sends `ContentToBrowser::SwRegister`.
    pub fn queue_sw_register(&self, script_url: String) {
        let mut inner = self.inner.borrow_mut();
        inner
            .pending_sw_registers
            .push(SwRegisterRequest { script_url });
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
}
