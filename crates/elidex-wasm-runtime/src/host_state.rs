//! Host state stored in the wasmtime `Store`.
//!
//! Uses the same bind/unbind raw pointer pattern as `HostBridge` in elidex-js-boa.
//! Pointers are valid only during `WasmInstance::call_export()`.

use std::sync::Arc;

use elidex_dom_api::registry::{CssomHandlerRegistry, DomHandlerRegistry};
use elidex_ecs::{EcsDom, Entity};
use elidex_script_session::SessionCore;

/// User data stored in `wasmtime::Store<HostState>`.
///
/// Raw pointers to `SessionCore` and `EcsDom` are set via `bind()` before
/// a Wasm export call and cleared via `unbind()` after.
pub struct HostState {
    session_ptr: *mut SessionCore,
    dom_ptr: *mut EcsDom,
    document_entity: Option<Entity>,
    pub(crate) dom_registry: Arc<DomHandlerRegistry>,
    #[allow(dead_code)] // Will be used for CSSOM host functions in a future milestone.
    pub(crate) cssom_registry: Arc<CssomHandlerRegistry>,
}

// HostState is !Send due to raw pointers, but wasmtime Store is used
// on a single thread. We explicitly opt in to Send so wasmtime can hold it.
// Safety: all access is single-threaded (bind/unbind bracket eval).
#[allow(unsafe_code)]
unsafe impl Send for HostState {}

impl HostState {
    /// Create a new unbound host state.
    pub fn new(
        dom_registry: Arc<DomHandlerRegistry>,
        cssom_registry: Arc<CssomHandlerRegistry>,
    ) -> Self {
        Self {
            session_ptr: std::ptr::null_mut(),
            dom_ptr: std::ptr::null_mut(),
            document_entity: None,
            dom_registry,
            cssom_registry,
        }
    }

    /// Bind the host state to live `SessionCore` and `EcsDom` references.
    #[allow(unsafe_code)]
    pub fn bind(&mut self, session: &mut SessionCore, dom: &mut EcsDom, document: Entity) {
        assert!(
            self.session_ptr.is_null(),
            "HostState::bind() called while already bound"
        );
        self.session_ptr = std::ptr::from_mut(session);
        self.dom_ptr = std::ptr::from_mut(dom);
        self.document_entity = Some(document);
    }

    /// Clear the pointers after the call completes.
    pub fn unbind(&mut self) {
        self.session_ptr = std::ptr::null_mut();
        self.dom_ptr = std::ptr::null_mut();
        self.document_entity = None;
    }

    /// Returns `true` if the host state is currently bound.
    #[cfg(test)]
    pub fn is_bound(&self) -> bool {
        !self.session_ptr.is_null()
    }

    /// Access `SessionCore` and `EcsDom` for the duration of a closure.
    ///
    /// # Panics
    ///
    /// Panics if the host state is not bound.
    #[allow(unsafe_code)]
    pub fn with<R>(&mut self, f: impl FnOnce(&mut SessionCore, &mut EcsDom) -> R) -> R {
        assert!(
            !self.session_ptr.is_null(),
            "HostState::with() called while unbound"
        );
        unsafe {
            let session = &mut *self.session_ptr;
            let dom = &mut *self.dom_ptr;
            f(session, dom)
        }
    }

    /// Returns the document root entity.
    ///
    /// # Panics
    ///
    /// Panics if the host state is not bound.
    pub fn document_entity(&self) -> Entity {
        self.document_entity
            .expect("HostState::document_entity() called while unbound")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_dom_api::registry::{create_cssom_registry, create_dom_registry};

    fn make_registries() -> (Arc<DomHandlerRegistry>, Arc<CssomHandlerRegistry>) {
        (
            Arc::new(create_dom_registry()),
            Arc::new(create_cssom_registry()),
        )
    }

    #[test]
    fn new_is_unbound() {
        let (dom_r, cssom_r) = make_registries();
        let state = HostState::new(dom_r, cssom_r);
        assert!(!state.is_bound());
    }

    #[test]
    fn bind_and_unbind() {
        let (dom_r, cssom_r) = make_registries();
        let mut state = HostState::new(dom_r, cssom_r);
        let mut session = SessionCore::new();
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();

        state.bind(&mut session, &mut dom, doc);
        assert!(state.is_bound());
        assert_eq!(state.document_entity(), doc);

        state.unbind();
        assert!(!state.is_bound());
    }

    #[test]
    fn with_accesses_session_and_dom() {
        let (dom_r, cssom_r) = make_registries();
        let mut state = HostState::new(dom_r, cssom_r);
        let mut session = SessionCore::new();
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();

        state.bind(&mut session, &mut dom, doc);
        state.with(|_session, dom| {
            let _e = dom.create_element("div", elidex_ecs::Attributes::default());
        });
        state.unbind();
    }

    #[test]
    #[should_panic(expected = "unbound")]
    fn with_panics_when_unbound() {
        let (dom_r, cssom_r) = make_registries();
        let mut state = HostState::new(dom_r, cssom_r);
        state.with(|_, _| {});
    }
}
