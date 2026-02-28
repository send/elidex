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
use elidex_ecs::{EcsDom, Entity};
use elidex_script_session::{JsObjectRef, SessionCore};

/// Bridge providing boa native functions access to `SessionCore` and `EcsDom`.
///
/// Clone is cheap (`Rc` increment). Each native function closure captures a
/// clone of this bridge.
#[derive(Clone)]
pub struct HostBridge {
    inner: Rc<RefCell<HostBridgeInner>>,
}

struct HostBridgeInner {
    session_ptr: *mut SessionCore,
    dom_ptr: *mut EcsDom,
    document_entity: Option<Entity>,
    /// Cache: `JsObjectRef` → boa `JsObject` for element identity preservation.
    js_object_cache: HashMap<JsObjectRef, JsObject>,
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
            })),
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
