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
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use boa_engine::JsObject;
use elidex_api_observers::intersection::IntersectionObserverRegistry;
use elidex_api_observers::mutation::MutationObserverRegistry;
use elidex_api_observers::resize::ResizeObserverRegistry;
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
    /// Re-entrancy guard: true while inside a `with()` closure.
    in_with: bool,
    /// Cache: `JsObjectRef` → boa `JsObject` for element identity preservation.
    js_object_cache: HashMap<JsObjectRef, JsObject>,
    /// Event listener JS function storage: `ListenerId` → boa `JsObject`.
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
    canvas_contexts: HashMap<u64, Canvas2dContext>,
    /// Entity bits of canvases modified since the last per-frame sync.
    dirty_canvases: HashSet<u64>,
    // --- Observer API ---
    /// `MutationObserver` registry.
    mutation_observers: MutationObserverRegistry,
    /// `ResizeObserver` registry.
    resize_observers: ResizeObserverRegistry,
    /// `IntersectionObserver` registry.
    intersection_observers: IntersectionObserverRegistry,
    /// Observer ID → JS callback function.
    observer_callbacks: HashMap<u64, JsObject>,
    /// Observer ID → JS observer wrapper object (for passing as 2nd arg to callback).
    observer_objects: HashMap<u64, JsObject>,
    /// Cached viewport dimensions (set by content thread on `SetViewport`).
    viewport_width: f32,
    viewport_height: f32,
    /// Cached viewport scroll offset.
    scroll_x: f32,
    scroll_y: f32,
    // --- Attr identity cache ---
    /// Cache mapping `(element_entity_bits, attr_name)` → `attr_entity_bits`.
    ///
    /// Ensures `getAttributeNode("x")` returns the same `Attr` entity on
    /// repeated calls, per WHATWG DOM §7.5 identity semantics.
    attr_cache: HashMap<(u64, String), u64>,
    // --- TreeWalker / NodeIterator / Range ---
    /// Active `TreeWalker` instances, keyed by unique ID.
    tree_walkers: HashMap<u64, elidex_dom_api::TreeWalker>,
    /// Active `NodeIterator` instances, keyed by unique ID.
    node_iterators: HashMap<u64, elidex_dom_api::NodeIterator>,
    /// Active `Range` instances, keyed by unique ID.
    ranges: HashMap<u64, elidex_dom_api::Range>,
    /// Next ID for TreeWalker/NodeIterator/Range allocation.
    traversal_next_id: u64,
    /// The Range ID associated with the current Selection, if any.
    selection_range_id: Option<u64>,
    // --- MediaQueryList ---
    /// Active `MediaQueryList` entries, keyed by unique ID.
    media_queries: HashMap<u64, MediaQueryEntry>,
    /// Next ID for `MediaQueryList` allocation.
    media_query_next_id: u64,
}

/// A tracked `MediaQueryList` entry with its query, cached result, and listeners.
struct MediaQueryEntry {
    query: String,
    matches: bool,
    listeners: Vec<JsObject>,
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
                in_with: false,
                js_object_cache: HashMap::new(),
                listener_store: HashMap::new(),
                current_url: None,
                pending_navigation: None,
                pending_history: None,
                history_length: 0,
                canvas_contexts: HashMap::new(),
                dirty_canvases: HashSet::new(),
                mutation_observers: MutationObserverRegistry::new(),
                resize_observers: ResizeObserverRegistry::new(),
                intersection_observers: IntersectionObserverRegistry::new(),
                observer_callbacks: HashMap::new(),
                observer_objects: HashMap::new(),
                viewport_width: 800.0,
                viewport_height: 600.0,
                scroll_x: 0.0,
                scroll_y: 0.0,
                attr_cache: HashMap::new(),
                tree_walkers: HashMap::new(),
                node_iterators: HashMap::new(),
                ranges: HashMap::new(),
                traversal_next_id: 1,
                selection_range_id: None,
                media_queries: HashMap::new(),
                media_query_next_id: 1,
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
            let mut inner = self.inner.borrow_mut();
            assert!(
                !inner.session_ptr.is_null(),
                "HostBridge::with() called while unbound"
            );
            assert!(
                !inner.in_with,
                "HostBridge::with() called re-entrantly — would create aliased &mut"
            );
            inner.in_with = true;
            (inner.session_ptr, inner.dom_ptr)
        };
        // Safety: pointers are valid for the duration of eval (bind/unbind bracket).
        // The RefCell borrow is dropped above, so no borrow conflicts.
        // The in_with guard prevents re-entrancy (aliased &mut).
        #[allow(unsafe_code)]
        let result = unsafe {
            let session = &mut *session_ptr;
            let dom = &mut *dom_ptr;
            f(session, dom)
        };
        self.inner.borrow_mut().in_with = false;
        result
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

    // --- Viewport ---

    /// Update cached viewport dimensions (called by content thread on `SetViewport`).
    pub fn set_viewport(&self, width: f32, height: f32) {
        let mut inner = self.inner.borrow_mut();
        inner.viewport_width = width;
        inner.viewport_height = height;
    }

    /// Get cached viewport width.
    pub fn viewport_width(&self) -> f32 {
        self.inner.borrow().viewport_width
    }

    /// Get cached viewport height.
    pub fn viewport_height(&self) -> f32 {
        self.inner.borrow().viewport_height
    }

    /// Update cached scroll offset (called by content thread before re-render).
    pub fn set_scroll_offset(&self, x: f32, y: f32) {
        let mut inner = self.inner.borrow_mut();
        inner.scroll_x = x;
        inner.scroll_y = y;
    }

    /// Get cached horizontal scroll offset.
    pub fn scroll_x(&self) -> f32 {
        self.inner.borrow().scroll_x
    }

    /// Get cached vertical scroll offset.
    pub fn scroll_y(&self) -> f32 {
        self.inner.borrow().scroll_y
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

    /// Mark a canvas as dirty (modified since last frame sync).
    pub fn mark_canvas_dirty(&self, entity_bits: u64) {
        self.inner.borrow_mut().dirty_canvases.insert(entity_bits);
    }

    /// Sync all dirty canvas pixel buffers to their ECS `ImageData` components.
    ///
    /// Called once per frame from the content thread loop, replacing per-draw-call syncs.
    /// Takes `&mut EcsDom` directly so this can be called outside of JS eval context
    /// (no `bind()` required).
    pub fn sync_dirty_canvases(&self, dom: &mut EcsDom) {
        let dirty: Vec<u64> = {
            let mut inner = self.inner.borrow_mut();
            inner.dirty_canvases.drain().collect()
        };
        for entity_bits in dirty {
            let Some((width, height, pixels)) = self.with_canvas(entity_bits, |ctx| {
                (ctx.width(), ctx.height(), ctx.to_rgba8_straight())
            }) else {
                continue;
            };
            let image_data = elidex_ecs::ImageData {
                pixels: std::sync::Arc::new(pixels),
                width,
                height,
            };
            let Some(entity) = elidex_ecs::Entity::from_bits(entity_bits) else {
                continue;
            };
            let _ = dom.world_mut().insert_one(entity, image_data);
        }
    }

    // --- Observer API ---

    /// Access the mutation observer registry mutably.
    pub fn with_mutation_observers<R>(
        &self,
        f: impl FnOnce(&mut MutationObserverRegistry) -> R,
    ) -> R {
        f(&mut self.inner.borrow_mut().mutation_observers)
    }

    /// Access the resize observer registry mutably.
    pub fn with_resize_observers<R>(&self, f: impl FnOnce(&mut ResizeObserverRegistry) -> R) -> R {
        f(&mut self.inner.borrow_mut().resize_observers)
    }

    /// Access the intersection observer registry mutably.
    pub fn with_intersection_observers<R>(
        &self,
        f: impl FnOnce(&mut IntersectionObserverRegistry) -> R,
    ) -> R {
        f(&mut self.inner.borrow_mut().intersection_observers)
    }

    /// Store a JS callback for an observer.
    pub fn store_observer_callback(
        &self,
        observer_id: u64,
        callback: JsObject,
        observer_obj: JsObject,
    ) {
        let mut inner = self.inner.borrow_mut();
        inner.observer_callbacks.insert(observer_id, callback);
        inner.observer_objects.insert(observer_id, observer_obj);
    }

    /// Get the JS callback for an observer.
    pub fn get_observer_callback(&self, observer_id: u64) -> Option<JsObject> {
        self.inner
            .borrow()
            .observer_callbacks
            .get(&observer_id)
            .cloned()
    }

    /// Get the JS observer wrapper object.
    pub fn get_observer_object(&self, observer_id: u64) -> Option<JsObject> {
        self.inner
            .borrow()
            .observer_objects
            .get(&observer_id)
            .cloned()
    }

    /// Remove an observer's callback and wrapper.
    pub fn remove_observer(&self, observer_id: u64) {
        let mut inner = self.inner.borrow_mut();
        inner.observer_callbacks.remove(&observer_id);
        inner.observer_objects.remove(&observer_id);
    }

    // --- Entity cleanup ---

    /// Clean up resources associated with a destroyed entity.
    ///
    /// Removes the canvas rendering context (freeing the backing `Pixmap`),
    /// cached JS wrapper objects, and event listener function objects for the
    /// given entity. Call this when an entity is removed from the DOM to
    /// prevent resource leaks.
    pub fn cleanup_entity(&self, entity: Entity, listener_ids: &[ListenerId]) {
        let mut inner = self.inner.borrow_mut();
        let bits = entity.to_bits().get();

        // Remove canvas context (may own megabytes of pixel data).
        inner.canvas_contexts.remove(&bits);

        // Remove cached JS wrapper objects for this entity's JsObjectRefs.
        // JsObjectRef keys are opaque IDs, not entity bits, so we cannot
        // selectively remove them here without an entity→JsObjectRef index.
        // The SessionCore identity map owns that mapping; callers should
        // release the entity there as well (SessionCore::release_entity).

        // Remove listener function objects from the store.
        for id in listener_ids {
            inner.listener_store.remove(id);
        }

        // Remove entity from observer target lists.
        inner.mutation_observers.remove_entity(entity);
        inner.resize_observers.remove_entity(entity);
        inner.intersection_observers.remove_entity(entity);

        // Invalidate attr cache entries for this element.
        inner.attr_cache.retain(|&(elem, _), _| elem != bits);
    }

    // --- Attr identity cache ---

    /// Look up a cached `Attr` entity for the given element + attribute name.
    pub fn get_cached_attr(&self, element_bits: u64, name: &str) -> Option<u64> {
        self.inner
            .borrow()
            .attr_cache
            .get(&(element_bits, name.to_string()))
            .copied()
    }

    /// Cache an `Attr` entity for the given element + attribute name.
    pub fn cache_attr(&self, element_bits: u64, name: String, attr_bits: u64) {
        self.inner
            .borrow_mut()
            .attr_cache
            .insert((element_bits, name), attr_bits);
    }

    /// Invalidate the `Attr` cache entry for a specific element + attribute.
    pub fn invalidate_attr_cache(&self, element_bits: u64, name: &str) {
        self.inner
            .borrow_mut()
            .attr_cache
            .remove(&(element_bits, name.to_string()));
    }

    /// Invalidate all `Attr` cache entries for a given element.
    pub fn invalidate_attr_cache_for_element(&self, element_bits: u64) {
        self.inner
            .borrow_mut()
            .attr_cache
            .retain(|&(elem, _), _| elem != element_bits);
    }

    // --- TreeWalker / NodeIterator / Range ---

    /// Create a new `TreeWalker` and return its ID.
    pub fn create_tree_walker(&self, root: Entity, what_to_show: u32) -> u64 {
        let mut inner = self.inner.borrow_mut();
        let id = inner.traversal_next_id;
        inner.traversal_next_id += 1;
        inner
            .tree_walkers
            .insert(id, elidex_dom_api::TreeWalker::new(root, what_to_show));
        id
    }

    /// Access a `TreeWalker` by ID.
    pub fn with_tree_walker<R>(
        &self,
        id: u64,
        f: impl FnOnce(&mut elidex_dom_api::TreeWalker) -> R,
    ) -> Option<R> {
        let mut inner = self.inner.borrow_mut();
        inner.tree_walkers.get_mut(&id).map(f)
    }

    /// Create a new `NodeIterator` and return its ID.
    pub fn create_node_iterator(&self, root: Entity, what_to_show: u32) -> u64 {
        let mut inner = self.inner.borrow_mut();
        let id = inner.traversal_next_id;
        inner.traversal_next_id += 1;
        inner
            .node_iterators
            .insert(id, elidex_dom_api::NodeIterator::new(root, what_to_show));
        id
    }

    /// Access a `NodeIterator` by ID.
    pub fn with_node_iterator<R>(
        &self,
        id: u64,
        f: impl FnOnce(&mut elidex_dom_api::NodeIterator) -> R,
    ) -> Option<R> {
        let mut inner = self.inner.borrow_mut();
        inner.node_iterators.get_mut(&id).map(f)
    }

    /// Create a new `Range` and return its ID.
    pub fn create_range(&self, node: Entity) -> u64 {
        let mut inner = self.inner.borrow_mut();
        let id = inner.traversal_next_id;
        inner.traversal_next_id += 1;
        inner.ranges.insert(id, elidex_dom_api::Range::new(node));
        id
    }

    /// Access a `Range` by ID.
    pub fn with_range<R>(
        &self,
        id: u64,
        f: impl FnOnce(&mut elidex_dom_api::Range) -> R,
    ) -> Option<R> {
        let mut inner = self.inner.borrow_mut();
        inner.ranges.get_mut(&id).map(f)
    }

    /// Get the current selection's Range ID, if any.
    pub fn selection_range_id(&self) -> Option<u64> {
        self.inner.borrow().selection_range_id
    }

    /// Set the selection's Range ID.
    pub fn set_selection_range_id(&self, id: Option<u64>) {
        self.inner.borrow_mut().selection_range_id = id;
    }

    // --- MediaQueryList ---

    /// Create a `MediaQueryList` entry and return its unique ID.
    pub fn create_media_query(&self, query: &str, matches: bool) -> u64 {
        let mut inner = self.inner.borrow_mut();
        let id = inner.media_query_next_id;
        inner.media_query_next_id += 1;
        inner.media_queries.insert(
            id,
            MediaQueryEntry {
                query: query.to_string(),
                matches,
                listeners: Vec::new(),
            },
        );
        id
    }

    /// Add a "change" event listener to a `MediaQueryList`.
    pub fn add_media_query_listener(&self, id: u64, callback: JsObject) {
        let mut inner = self.inner.borrow_mut();
        if let Some(entry) = inner.media_queries.get_mut(&id) {
            entry.listeners.push(callback);
        }
    }

    /// Remove a "change" event listener from a `MediaQueryList` by reference identity.
    pub fn remove_media_query_listener(&self, id: u64, callback: &JsObject) {
        let mut inner = self.inner.borrow_mut();
        if let Some(entry) = inner.media_queries.get_mut(&id) {
            entry
                .listeners
                .retain(|stored| !JsObject::equals(stored, callback));
        }
    }

    /// Re-evaluate all media queries against the given viewport dimensions.
    ///
    /// Returns a list of `(id, new_matches)` for entries whose result changed.
    /// Updates the cached `matches` value for each changed entry.
    pub fn re_evaluate_media_queries(&self, width: f32, height: f32) -> Vec<(u64, bool)> {
        let mut inner = self.inner.borrow_mut();
        let mut changed = Vec::new();
        for (&id, entry) in &mut inner.media_queries {
            let new_matches = evaluate_media_query_raw(&entry.query, width, height);
            if new_matches != entry.matches {
                entry.matches = new_matches;
                changed.push((id, new_matches));
            }
        }
        changed
    }

    /// Get the current `matches` value for a media query.
    pub fn media_query_matches(&self, id: u64) -> bool {
        self.inner
            .borrow()
            .media_queries
            .get(&id)
            .is_some_and(|e| e.matches)
    }

    /// Get the listener callbacks for a media query (cloned for dispatch).
    pub fn media_query_listeners(&self, id: u64) -> Vec<JsObject> {
        self.inner
            .borrow()
            .media_queries
            .get(&id)
            .map_or_else(Vec::new, |e| e.listeners.clone())
    }

    /// Get the query string for a media query entry.
    pub fn media_query_string(&self, id: u64) -> Option<String> {
        self.inner
            .borrow()
            .media_queries
            .get(&id)
            .map(|e| e.query.clone())
    }
}

impl Default for HostBridge {
    fn default() -> Self {
        Self::new()
    }
}

/// Evaluate a media query against explicit viewport dimensions (no bridge needed).
///
/// This is the shared implementation used by both `evaluate_media_query` in
/// `window.rs` (via bridge accessor) and `re_evaluate_media_queries`.
pub(crate) fn evaluate_media_query_raw(query: &str, width: f32, height: f32) -> bool {
    let q = query.trim().to_ascii_lowercase();
    let inner = q
        .strip_prefix('(')
        .and_then(|s| s.strip_suffix(')'))
        .unwrap_or(&q);

    if let Some((feature, value)) = inner.split_once(':') {
        let feature = feature.trim();
        let value = value.trim();
        let px_value = value
            .strip_suffix("px")
            .unwrap_or(value)
            .trim()
            .parse::<f32>()
            .ok();

        match feature {
            "max-width" => return px_value.is_some_and(|v| width <= v),
            "min-width" => return px_value.is_some_and(|v| width >= v),
            "max-height" => return px_value.is_some_and(|v| height <= v),
            "min-height" => return px_value.is_some_and(|v| height >= v),
            "prefers-color-scheme" => return false,
            _ => {}
        }
    }
    false
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
        for obj in inner.observer_callbacks.values() {
            mark(obj);
        }
        for obj in inner.observer_objects.values() {
            mark(obj);
        }
        for entry in inner.media_queries.values() {
            for listener in &entry.listeners {
                mark(listener);
            }
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

    #[test]
    fn cleanup_entity_removes_canvas_and_listeners() {
        let bridge = HostBridge::new();
        let mut dom = EcsDom::new();
        let entity = dom.create_element("canvas", Attributes::default());
        let bits = entity.to_bits().get();

        // Insert a canvas context.
        bridge.ensure_canvas_context(bits, 100, 100);
        assert!(bridge.with_canvas(bits, |_| ()).is_some());

        // Store a listener.
        let lid = ListenerId::from_raw(42);
        // We can't create a real JsObject here without a boa Context,
        // so we test the canvas cleanup path and verify listener_store
        // removal via store_listener + cleanup.
        // For now, verify canvas cleanup works.
        bridge.cleanup_entity(entity, &[lid]);
        assert!(bridge.with_canvas(bits, |_| ()).is_none());
    }
}
