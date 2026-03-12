//! `HostBridge`: provides native functions access to `SessionCore` and `EcsDom`.
//!
//! The bridge uses raw pointers that are valid only during `JsRuntime::eval()`.
//! `bind()` sets the pointers before eval, `unbind()` clears them after.
//! `with()` dereferences the pointers for the duration of a closure.
//!
//! # Safety
//!
//! - eval is synchronous (single-threaded, no re-entrancy)
//! - bind/unbind bracket every eval call
//! - `HostBridge` is `!Send` (via `Rc`)

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use boa_engine::JsObject;
use elidex_dom_api::registry::{CssomHandlerRegistry, DomHandlerRegistry};
use elidex_ecs::{EcsDom, Entity};
use elidex_navigation::{HistoryAction, NavigationRequest};
use elidex_script_session::{JsObjectRef, ListenerId, SessionCore};
use elidex_web_canvas::Canvas2dContext;

/// Bridge providing boa native functions access to `SessionCore` and `EcsDom`.
///
/// Clone is cheap (`Rc` increment). Each native function closure captures a
/// clone of this bridge.
#[derive(Clone)]
pub struct HostBridge {
    inner: Rc<RefCell<HostBridgeInner>>,
    dom_registry: Rc<DomHandlerRegistry>,
    cssom_registry: Rc<CssomHandlerRegistry>,
}

struct HostBridgeInner {
    session_ptr: *mut SessionCore,
    dom_ptr: *mut EcsDom,
    document_entity: Option<Entity>,
    /// Cache: `JsObjectRef` → boa `JsObject` for element identity preservation.
    js_object_cache: HashMap<JsObjectRef, JsObject>,
    /// Event listener JS function storage: `ListenerId` → boa `JsObject`.
    ///
    /// TODO(Phase 4): entries are not cleaned up when entities are destroyed.
    /// In Phase 3.5 entities are rarely destroyed at runtime so this is acceptable,
    /// but long-running applications with dynamic DOM updates may accumulate
    /// orphaned entries. Consider adding an entity-destruction hook that
    /// bulk-removes listeners for the destroyed entity.
    listener_store: HashMap<ListenerId, JsObject>,
    /// The URL of the currently loaded page.
    current_url: Option<url::Url>,
    /// A navigation request pending after script execution.
    pending_navigation: Option<NavigationRequest>,
    /// A history action pending after script execution.
    pending_history: Option<HistoryAction>,
    /// The number of entries in the session history.
    history_length: usize,
    /// Canvas 2D rendering contexts, keyed by entity bits.
    ///
    /// TODO(Phase 4): like `listener_store`, entries are not cleaned up when
    /// canvas elements are destroyed. Each `Canvas2dContext` owns a `Pixmap`
    /// (potentially megabytes), so this leak is more significant than listener
    /// entries. Add entity-destruction hooks to release canvas contexts.
    canvas_contexts: HashMap<u64, Canvas2dContext>,
}

// Safety: HostBridge is !Send via Rc<RefCell<_>>. This is correct — it should
// only be used on the thread that created it.

impl HostBridge {
    /// Create a new unbound bridge.
    pub fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new(HostBridgeInner {
                session_ptr: std::ptr::null_mut(),
                dom_ptr: std::ptr::null_mut(),
                document_entity: None,
                js_object_cache: HashMap::new(),
                listener_store: HashMap::new(),
                current_url: None,
                pending_navigation: None,
                pending_history: None,
                history_length: 0,
                canvas_contexts: HashMap::new(),
            })),
            dom_registry: Rc::new(elidex_dom_api::registry::create_dom_registry()),
            cssom_registry: Rc::new(elidex_dom_api::registry::create_cssom_registry()),
        }
    }

    /// Bind the bridge to live `SessionCore` and `EcsDom` references.
    ///
    /// Must be called before `JsRuntime::eval()` and paired with `unbind()`.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `session` and `dom` outlive the eval call.
    #[allow(unsafe_code)]
    pub fn bind(&self, session: &mut SessionCore, dom: &mut EcsDom, document_entity: Entity) {
        let mut inner = self.inner.borrow_mut();
        debug_assert!(
            inner.session_ptr.is_null(),
            "HostBridge::bind() called while already bound — missing unbind()?"
        );
        inner.session_ptr = std::ptr::from_mut(session);
        inner.dom_ptr = std::ptr::from_mut(dom);
        inner.document_entity = Some(document_entity);
    }

    /// Clear the bridge pointers after eval completes.
    pub fn unbind(&self) {
        let mut inner = self.inner.borrow_mut();
        inner.session_ptr = std::ptr::null_mut();
        inner.dom_ptr = std::ptr::null_mut();
        inner.document_entity = None;
    }

    /// Returns `true` if the bridge is currently bound.
    pub fn is_bound(&self) -> bool {
        let inner = self.inner.borrow();
        !inner.session_ptr.is_null()
    }

    /// Access `SessionCore` and `EcsDom` for the duration of the closure.
    ///
    /// The `RefCell` borrow is released before calling `f`, so the closure
    /// may call `cache_js_object()` / `get_cached_js_object()` freely.
    ///
    /// # Panics
    ///
    /// Panics if the bridge is not bound (programming error).
    pub fn with<R>(&self, f: impl FnOnce(&mut SessionCore, &mut EcsDom) -> R) -> R {
        // Extract raw pointers and drop the borrow immediately so that
        // the closure can call borrow()/borrow_mut() on the inner RefCell
        // (e.g. cache_js_object, get_cached_js_object).
        let (session_ptr, dom_ptr) = {
            let inner = self.inner.borrow();
            assert!(
                !inner.session_ptr.is_null(),
                "HostBridge::with() called while unbound"
            );
            (inner.session_ptr, inner.dom_ptr)
        };
        // Safety: pointers are valid for the duration of eval (bind/unbind bracket).
        // The RefCell borrow is dropped above, so no borrow conflicts.
        // Re-entrancy of with() is NOT safe (would create aliased &mut);
        // current call structure ensures no nesting.
        #[allow(unsafe_code)]
        unsafe {
            let session = &mut *session_ptr;
            let dom = &mut *dom_ptr;
            f(session, dom)
        }
    }

    /// Returns the document root entity.
    ///
    /// # Panics
    ///
    /// Panics if the bridge is not bound.
    pub fn document_entity(&self) -> Entity {
        self.inner
            .borrow()
            .document_entity
            .expect("HostBridge::document_entity() called while unbound")
    }

    /// Cache a boa `JsObject` for an element's `JsObjectRef`.
    pub fn cache_js_object(&self, obj_ref: JsObjectRef, obj: JsObject) {
        self.inner.borrow_mut().js_object_cache.insert(obj_ref, obj);
    }

    /// Look up a cached boa `JsObject` for a `JsObjectRef`.
    pub fn get_cached_js_object(&self, obj_ref: JsObjectRef) -> Option<JsObject> {
        self.inner.borrow().js_object_cache.get(&obj_ref).cloned()
    }

    /// Store a JS function object for an event listener.
    pub fn store_listener(&self, id: ListenerId, func: JsObject) {
        self.inner.borrow_mut().listener_store.insert(id, func);
    }

    /// Retrieve the JS function for an event listener.
    pub fn get_listener(&self, id: ListenerId) -> Option<JsObject> {
        self.inner.borrow().listener_store.get(&id).cloned()
    }

    /// Remove the JS function for an event listener.
    pub fn remove_listener(&self, id: ListenerId) -> Option<JsObject> {
        self.inner.borrow_mut().listener_store.remove(&id)
    }

    /// Check if a JS object pointer-equals the stored listener for a given ID.
    ///
    /// Uses reference identity (`JsObject::equals`), matching the DOM spec
    /// requirement that `removeEventListener` identifies listeners by the
    /// same function reference passed to `addEventListener`.
    ///
    /// Used by `removeEventListener` to find the matching listener entry.
    pub fn listener_matches(&self, id: ListenerId, func: &JsObject) -> bool {
        self.inner
            .borrow()
            .listener_store
            .get(&id)
            .is_some_and(|stored| JsObject::equals(stored, func))
    }

    // --- Navigation state ---

    /// Set the current page URL.
    pub fn set_current_url(&self, url: Option<url::Url>) {
        self.inner.borrow_mut().current_url = url;
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

    // --- Registry access ---

    /// Access the DOM API handler registry.
    #[must_use]
    pub fn dom_registry(&self) -> &DomHandlerRegistry {
        &self.dom_registry
    }

    /// Access the CSSOM API handler registry.
    #[must_use]
    pub fn cssom_registry(&self) -> &CssomHandlerRegistry {
        &self.cssom_registry
    }

    // --- Canvas 2D context ---

    /// Get or create a Canvas 2D context for an entity.
    ///
    /// Returns `true` if a new context was created (first call for this entity).
    pub fn ensure_canvas_context(&self, entity_bits: u64, width: u32, height: u32) -> bool {
        let mut inner = self.inner.borrow_mut();
        if inner.canvas_contexts.contains_key(&entity_bits) {
            return false;
        }
        if let Some(ctx) = Canvas2dContext::new(width, height) {
            inner.canvas_contexts.insert(entity_bits, ctx);
            true
        } else {
            false
        }
    }

    /// Access a canvas context for the duration of a closure.
    ///
    /// Returns `None` if no context exists for the entity.
    pub fn with_canvas<R>(
        &self,
        entity_bits: u64,
        f: impl FnOnce(&mut Canvas2dContext) -> R,
    ) -> Option<R> {
        let mut inner = self.inner.borrow_mut();
        inner.canvas_contexts.get_mut(&entity_bits).map(f)
    }
}

impl Default for HostBridge {
    fn default() -> Self {
        Self::new()
    }
}

// Implement Trace/Finalize for boa_gc compatibility (used in from_copy_closure_with_captures).
// We must mark cached JsObjects so the GC knows they are reachable.
//
// Safety: `with()` drops its borrow before calling closures, and
// `cache_js_object` / `get_cached_js_object` borrows are short-lived.
// GC tracing occurs at boa allocation safepoints, which are outside
// those brief borrow scopes. The `borrow()` here should always succeed.
#[allow(unsafe_code)]
unsafe impl boa_gc::Trace for HostBridge {
    boa_gc::custom_trace!(this, mark, {
        let inner = this.inner.borrow();
        for obj in inner.js_object_cache.values() {
            mark(obj);
        }
        for obj in inner.listener_store.values() {
            mark(obj);
        }
        // canvas_contexts intentionally not traced: Canvas2dContext contains only
        // Pixmap + DrawingState (no GC-managed JsObjects). If Canvas2dContext ever
        // stores JsObjects, this Trace implementation must be updated.
    });
}

impl boa_gc::Finalize for HostBridge {
    fn finalize(&self) {
        // No cleanup needed.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_ecs::Attributes;
    use elidex_script_session::ComponentKind;

    #[test]
    fn new_bridge_is_unbound() {
        let bridge = HostBridge::new();
        assert!(!bridge.is_bound());
    }

    #[test]
    fn bind_and_unbind() {
        let bridge = HostBridge::new();
        let mut session = SessionCore::new();
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();

        bridge.bind(&mut session, &mut dom, doc);
        assert!(bridge.is_bound());
        assert_eq!(bridge.document_entity(), doc);

        bridge.unbind();
        assert!(!bridge.is_bound());
    }

    #[test]
    fn with_accesses_session_and_dom() {
        let bridge = HostBridge::new();
        let mut session = SessionCore::new();
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();

        bridge.bind(&mut session, &mut dom, doc);
        bridge.with(|session, dom| {
            let e = dom.create_element("div", Attributes::default());
            session.get_or_create_wrapper(e, ComponentKind::Element);
            assert_eq!(session.identity_map().len(), 1);
        });
        bridge.unbind();
    }

    #[test]
    #[should_panic(expected = "unbound")]
    fn with_panics_when_unbound() {
        let bridge = HostBridge::new();
        bridge.with(|_, _| {});
    }

    #[test]
    fn clone_shares_state() {
        let bridge = HostBridge::new();
        let clone = bridge.clone();
        let mut session = SessionCore::new();
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();

        bridge.bind(&mut session, &mut dom, doc);
        assert!(clone.is_bound());
        bridge.unbind();
    }

    #[test]
    fn js_object_cache() {
        let bridge = HostBridge::new();
        let ref1 = JsObjectRef::from_raw(1);
        assert!(bridge.get_cached_js_object(ref1).is_none());
        // We can't easily test with a real JsObject without a Context,
        // but the HashMap operations are straightforward.
    }
}
