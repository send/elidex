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

mod canvas;
mod ce;
mod cssom;
mod document_state;
mod iframe_bridge;
pub mod local_storage;
mod media;
mod navigation;
mod observers;
pub(crate) mod realtime;
mod traversal;
mod viewport;

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use indexmap::IndexMap;
use std::rc::Rc;

use boa_engine::{JsObject, JsValue};
use elidex_api_observers::intersection::IntersectionObserverRegistry;
use elidex_api_observers::mutation::MutationObserverRegistry;
use elidex_api_observers::resize::ResizeObserverRegistry;
use elidex_custom_elements::CustomElementRegistry;
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

/// Monotonic counter for assigning unique IDs to each `HostBridgeInner`.
///
/// Used to isolate opaque-origin ("null") localStorage: each bridge gets
/// a unique key like `"null:42"` instead of sharing a single `"null"`.
static BRIDGE_ID_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

pub(crate) struct HostBridgeInner {
    session_ptr: *mut SessionCore,
    dom_ptr: *mut EcsDom,
    document_entity: Option<Entity>,
    /// Unique ID for this bridge instance, used for opaque origin isolation.
    bridge_id: u64,
    /// Re-entrancy guard: true while inside a `with()` closure.
    in_with: bool,
    /// Cache: `JsObjectRef` → boa `JsObject` for element identity preservation.
    js_object_cache: HashMap<JsObjectRef, JsObject>,
    /// Event listener JS function storage: `ListenerId` → boa `JsObject`.
    listener_store: HashMap<ListenerId, JsObject>,
    /// The URL of the currently loaded page.
    current_url: Option<url::Url>,
    /// Cached origin string derived from `current_url` (avoids re-parsing on every localStorage op).
    cached_origin: String,
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
    /// Device pixel ratio (set by content thread from winit `scale_factor`).
    device_pixel_ratio: f32,
    /// Window screen position X (set by content thread from winit).
    screen_x: i32,
    /// Window screen position Y (set by content thread from winit).
    screen_y: i32,
    /// Monitor width in CSS pixels (set by content thread from winit).
    monitor_width: f32,
    /// Monitor height in CSS pixels (set by content thread from winit).
    monitor_height: f32,
    /// Screen color depth in bits (set by content thread from GPU surface format).
    color_depth: u32,
    /// Cached viewport scroll offset.
    scroll_x: f32,
    scroll_y: f32,
    /// Pending scroll offset set by JS `scrollTo`/`scrollBy`, picked up by content thread.
    pending_scroll: Option<(f32, f32)>,
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
    // --- CSSOM ---
    /// Lightweight stylesheet representations for JS access.
    stylesheets: Vec<CssomSheet>,
    /// Pending CSSOM mutations to be picked up by the content thread.
    cssom_mutations: Vec<CssomMutation>,
    // --- Custom Elements ---
    /// Custom element registry (WHATWG HTML §4.13.4).
    custom_element_registry: CustomElementRegistry,
    /// JS constructor storage: `constructor_id` → boa `JsObject`.
    custom_element_constructors: HashMap<u64, JsObject>,
    /// Queued custom element lifecycle reactions.
    custom_element_reactions: Vec<elidex_custom_elements::CustomElementReaction>,
    /// Next constructor ID for custom element definitions.
    ce_next_constructor_id: u64,
    /// Pending `whenDefined()` resolve functions, keyed by custom element name.
    when_defined_resolvers: HashMap<String, Vec<boa_engine::object::builtins::JsFunction>>,
    /// Cached pending `whenDefined()` promises, keyed by custom element name.
    /// Returned for repeated calls with the same name before `define()`.
    when_defined_promises: HashMap<String, JsValue>,
    // --- iframe / multi-document ---
    iframe: IframeBridgeState,
    // --- WebSocket / SSE ---
    realtime: realtime::RealtimeState,
    // --- Timer queue ---
    /// Reference to the timer queue for `window.stop()`.
    timer_queue: Option<crate::globals::timers::TimerQueueHandle>,
    // --- Script dispatch ---
    /// Pending script-dispatched event (from `dispatchEvent()`).
    /// Set by `dispatch_event_for()`, consumed by runtime after eval.
    pending_script_dispatch: Option<elidex_script_session::DispatchEvent>,
    // --- Document state ---
    /// The currently focused entity, synced from `ContentState` before eval.
    focus_target: Option<elidex_ecs::Entity>,
    /// Whether the tab is hidden (not the active tab).
    tab_hidden: bool,
    /// Window name (getter/setter per WHATWG HTML).
    window_name: String,
    // --- Storage ---
    /// Session storage (tab-scoped, insertion-order-preserving for `key(n)`).
    session_storage: IndexMap<String, String>,
    /// Cached byte size of session storage (sum of `key.len()` + `value.len()` for all entries).
    session_storage_bytes: usize,
    /// Pending localStorage change notifications for cross-tab broadcast.
    pending_storage_changes: Vec<StorageChange>,
    // --- Animations (Web Animations API) ---
    /// Pending script-initiated animations, consumed by content thread.
    pending_script_animations: Vec<crate::globals::element::accessors::animate::ScriptAnimation>,
    // --- Window focus ---
    /// Pending window focus request from `window.focus()`.
    pending_focus: bool,
    // --- Script execution ---
    /// The `<script>` element entity currently being evaluated (WHATWG HTML §4.12.1.1).
    /// Set before each script evaluation, cleared after. Used by `document.currentScript`.
    current_script_entity: Option<Entity>,
}

/// Iframe-related state for the JS bridge.
///
/// Grouped to reduce field count on `HostBridgeInner` (~35 -> ~28+1).
///
/// Note: `parent_bridge` and `iframe_bridges` fields are intentionally
/// omitted. Boa uses per-`Context` `JsObject` references that cannot cross
/// `Context` boundaries, so `contentDocument`/`contentWindow` return null for
/// all iframes. Cross-context document/window proxies require the
/// self-hosted JS engine (M4-9+).
struct IframeBridgeState {
    /// Security origin of this document (WHATWG HTML §7.5).
    origin: elidex_plugin::SecurityOrigin,
    /// The `<iframe>` element entity in the parent DOM that contains this window.
    /// `None` for top-level documents.
    frame_element: Option<Entity>,
    /// Referrer URL for this document (set from parent URL when loaded as iframe).
    referrer: Option<String>,
    /// Iframe sandbox flags (if this document is inside a sandboxed iframe).
    /// `None` for top-level documents or unsandboxed iframes.
    sandbox_flags: Option<elidex_plugin::IframeSandboxFlags>,
    /// Iframe nesting depth (0 for top-level, incremented per nested iframe).
    /// Used for `MAX_IFRAME_DEPTH` enforcement across separate `EcsDom` instances.
    iframe_depth: usize,
    /// Queued postMessage events for delivery in the next event loop tick.
    pending_post_messages: Vec<(String, String)>,
    /// URLs to open in new tabs (from `window.open` with `_blank` target).
    /// Vec to support multiple window.open calls before the event loop drains.
    pending_open_tabs: Vec<url::Url>,
    /// Pending iframe navigations from `window.open` with named targets.
    /// Each entry is `(iframe_name, url)`.
    pending_navigate_iframe: Vec<(String, url::Url)>,
}

impl Default for IframeBridgeState {
    fn default() -> Self {
        Self {
            origin: elidex_plugin::SecurityOrigin::opaque(),
            frame_element: None,
            referrer: None,
            sandbox_flags: None,
            iframe_depth: 0,
            pending_post_messages: Vec::new(),
            pending_open_tabs: Vec::new(),
            pending_navigate_iframe: Vec::new(),
        }
    }
}

/// A pending `localStorage` change notification for cross-tab broadcast.
#[derive(Clone, Debug)]
pub struct StorageChange {
    /// The origin that owns the storage area.
    pub origin: String,
    /// The key that changed (`None` for `clear()`).
    pub key: Option<String>,
    /// The old value (`None` if the key was newly set or cleared).
    pub old_value: Option<String>,
    /// The new value (`None` if the key was removed or cleared).
    pub new_value: Option<String>,
    /// The URL of the document that triggered the change.
    pub url: String,
}

/// A tracked `MediaQueryList` entry with its query, cached result, and listeners.
struct MediaQueryEntry {
    query: String,
    matches: bool,
    listeners: Vec<JsObject>,
}

/// A lightweight representation of a CSS rule for CSSOM JS access.
///
/// Stores serialized selector and declaration text so the JS layer can
/// expose `selectorText`, `cssText`, and `style` without depending on
/// the CSS parser crate.
#[derive(Clone, Debug)]
pub struct CssomRule {
    /// The selector text (e.g. `"div.foo"`).
    pub selector_text: String,
    /// Individual declarations as `(property, value)` pairs.
    pub declarations: Vec<(String, String)>,
}

impl CssomRule {
    /// Serialize the rule to its full CSS text representation.
    #[must_use]
    pub fn css_text(&self) -> String {
        let decls: Vec<String> = self
            .declarations
            .iter()
            .map(|(prop, val)| format!("{prop}: {val}"))
            .collect();
        format!("{} {{ {} }}", self.selector_text, decls.join("; "))
    }
}

/// A lightweight representation of a CSS stylesheet for CSSOM JS access.
#[derive(Clone, Debug, Default)]
pub struct CssomSheet {
    /// Rules in source order.
    pub rules: Vec<CssomRule>,
}

/// A pending CSSOM mutation to be applied by the content thread.
#[derive(Clone, Debug)]
pub enum CssomMutation {
    /// Insert a rule at the given index in the given sheet.
    InsertRule {
        /// Sheet index in the `stylesheets` list.
        sheet_index: usize,
        /// Rule index within the sheet.
        rule_index: usize,
        /// Raw CSS rule text to parse.
        rule_text: String,
    },
    /// Delete a rule at the given index in the given sheet.
    DeleteRule {
        /// Sheet index in the `stylesheets` list.
        sheet_index: usize,
        /// Rule index to delete.
        rule_index: usize,
    },
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
                bridge_id: BRIDGE_ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                in_with: false,
                js_object_cache: HashMap::new(),
                listener_store: HashMap::new(),
                current_url: None,
                cached_origin: "null".to_string(),
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
                device_pixel_ratio: 1.0,
                screen_x: 0,
                screen_y: 0,
                monitor_width: 800.0,
                monitor_height: 600.0,
                color_depth: 24,
                scroll_x: 0.0,
                scroll_y: 0.0,
                pending_scroll: None,
                tree_walkers: HashMap::new(),
                node_iterators: HashMap::new(),
                ranges: HashMap::new(),
                traversal_next_id: 1,
                selection_range_id: None,
                media_queries: HashMap::new(),
                media_query_next_id: 1,
                stylesheets: Vec::new(),
                cssom_mutations: Vec::new(),
                custom_element_registry: CustomElementRegistry::new(),
                custom_element_constructors: HashMap::new(),
                custom_element_reactions: Vec::new(),
                ce_next_constructor_id: 1,
                when_defined_resolvers: HashMap::new(),
                when_defined_promises: HashMap::new(),
                iframe: IframeBridgeState::default(),
                realtime: realtime::RealtimeState::default(),
                timer_queue: None,
                pending_script_dispatch: None,
                focus_target: None,
                tab_hidden: false,
                window_name: String::new(),
                session_storage: IndexMap::new(),
                session_storage_bytes: 0,
                pending_storage_changes: Vec::new(),
                pending_script_animations: Vec::new(),
                pending_focus: false,
                current_script_entity: None,
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

    // Navigation state methods are in navigation.rs
    // Iframe bridge methods are in iframe_bridge.rs

    // --- Timer queue ---

    /// Store the timer queue handle for `window.stop()`.
    pub fn set_timer_queue(&self, tq: crate::globals::timers::TimerQueueHandle) {
        self.inner.borrow_mut().timer_queue = Some(tq);
    }

    /// Clear all pending timers (`window.stop()` support).
    pub fn clear_all_timers(&self) {
        if let Some(ref tq) = self.inner.borrow().timer_queue {
            tq.borrow_mut().clear_all();
        }
    }

    /// Set the currently executing `<script>` element entity.
    ///
    /// Called before evaluating each script; cleared after. Used by
    /// `document.currentScript` (WHATWG HTML §4.12.1.1).
    pub fn set_current_script_entity(&self, entity: Option<Entity>) {
        self.inner.borrow_mut().current_script_entity = entity;
    }

    /// Get the currently executing `<script>` element entity, if any.
    #[must_use]
    pub fn current_script_entity(&self) -> Option<Entity> {
        self.inner.borrow().current_script_entity
    }

    // --- Web Animations API ---

    /// Queue a script-initiated animation for the content thread to apply.
    pub(crate) fn queue_script_animation(
        &self,
        anim: crate::globals::element::accessors::animate::ScriptAnimation,
    ) {
        self.inner.borrow_mut().pending_script_animations.push(anim);
    }

    /// Drain pending script-initiated animations.
    pub fn drain_script_animations(
        &self,
    ) -> Vec<crate::globals::element::accessors::animate::ScriptAnimation> {
        std::mem::take(&mut self.inner.borrow_mut().pending_script_animations)
    }

    /// Get the number of active animations for an entity.
    ///
    /// Currently returns 0 (pending animations not yet applied). In the full
    /// implementation, the content thread would sync animation state back.
    pub(crate) fn animation_count(&self, entity_id: u64) -> usize {
        // Count pending + active (from engine). For now, count pending only.
        self.inner
            .borrow()
            .pending_script_animations
            .iter()
            .filter(|a| a.entity_id == entity_id)
            .count()
    }

    /// Get info about an active animation for an entity.
    pub(crate) fn animation_info(
        &self,
        entity_id: u64,
        index: usize,
    ) -> Option<crate::globals::element::accessors::animate::AnimationInfo> {
        let inner = self.inner.borrow();
        let mut count = 0;
        for a in &inner.pending_script_animations {
            if a.entity_id == entity_id {
                if count == index {
                    return Some(crate::globals::element::accessors::animate::AnimationInfo {
                        id: a.options.id.clone(),
                        play_state: "running".into(),
                        current_time: 0.0,
                    });
                }
                count += 1;
            }
        }
        None
    }

    /// Collect form control name/value pairs from a form entity.
    ///
    /// Walks child elements of the given entity, collecting submittable
    /// controls (input, select, textarea) with a name attribute.
    pub(crate) fn collect_form_data(&self, entity_bits: u64) -> Vec<(String, String)> {
        let inner = self.inner.borrow();
        #[allow(unsafe_code)]
        let Some(dom) = (unsafe { inner.dom_ptr.as_ref() }) else {
            return Vec::new();
        };
        let Some(entity) = Entity::from_bits(entity_bits) else {
            return Vec::new();
        };

        let mut pairs = Vec::new();
        collect_form_data_recursive(dom, entity, &mut pairs);
        pairs
    }

    /// Drain all pending WebSocket and SSE events.
    pub fn drain_realtime_events(&self) -> realtime::RealtimeEvents {
        self.inner.borrow_mut().realtime.drain_realtime_events()
    }

    /// Shut down all WebSocket and SSE connections.
    pub fn shutdown_all_realtime(&self) {
        self.inner.borrow_mut().realtime.shutdown_all();
    }

    // --- WebSocket API ---

    /// Open a WebSocket connection. Returns connection ID or error.
    pub fn open_websocket(
        &self,
        url: url::Url,
        protocols: Vec<String>,
        origin: String,
        js_object: JsObject,
    ) -> Result<u64, String> {
        self.inner
            .borrow_mut()
            .realtime
            .open_websocket(url, protocols, origin, js_object)
    }

    /// Read a WebSocket callback field via a closure.
    /// The closure receives the `WsCallbacks` reference.
    pub(crate) fn with_ws_callbacks<F, R>(&self, id: u64, f: F) -> Option<R>
    where
        F: FnOnce(&realtime::WsCallbacks) -> R,
    {
        self.inner.borrow().realtime.ws_callbacks(id).map(f)
    }

    /// Mutate a WebSocket callback field via a closure.
    pub(crate) fn with_ws_callbacks_mut<F, R>(&self, id: u64, f: F) -> Option<R>
    where
        F: FnOnce(&mut realtime::WsCallbacks) -> R,
    {
        self.inner.borrow_mut().realtime.ws_callbacks_mut(id).map(f)
    }

    /// Send text on a WebSocket.
    #[must_use]
    pub fn ws_send_text(&self, id: u64, data: String) -> bool {
        self.inner.borrow().realtime.ws_send_text(id, data)
    }

    /// Close a WebSocket.
    pub fn ws_close(&self, id: u64, code: u16, reason: String) {
        self.inner.borrow().realtime.ws_close(id, code, reason);
    }

    /// Remove a WebSocket from the registry.
    pub fn remove_ws(&self, id: u64) {
        self.inner.borrow_mut().realtime.remove_ws(id);
    }

    // --- EventSource API ---

    /// Open an `EventSource` connection.
    pub fn open_event_source(
        &self,
        url: url::Url,
        with_credentials: bool,
        origin: Option<String>,
        js_object: JsObject,
    ) -> Result<u64, String> {
        self.inner
            .borrow_mut()
            .realtime
            .open_event_source(url, with_credentials, origin, js_object)
    }

    /// Read an SSE callback field via a closure.
    pub(crate) fn with_sse_callbacks<F, R>(&self, id: u64, f: F) -> Option<R>
    where
        F: FnOnce(&realtime::SseCallbacks) -> R,
    {
        self.inner.borrow().realtime.sse_callbacks(id).map(f)
    }

    /// Mutate an SSE callback field via a closure.
    pub(crate) fn with_sse_callbacks_mut<F, R>(&self, id: u64, f: F) -> Option<R>
    where
        F: FnOnce(&mut realtime::SseCallbacks) -> R,
    {
        self.inner
            .borrow_mut()
            .realtime
            .sse_callbacks_mut(id)
            .map(f)
    }

    /// Close and remove an SSE connection.
    ///
    /// SSE has no close handshake (unlike WebSocket), so the connection
    /// is removed immediately to prevent resource leaks.
    pub fn sse_close(&self, id: u64) {
        self.inner.borrow_mut().realtime.sse_close(id);
    }

    /// Set the cookie jar for SSE `withCredentials` support.
    pub fn set_realtime_cookie_jar(&self, jar: Option<std::sync::Arc<elidex_net::CookieJar>>) {
        self.inner.borrow_mut().realtime.set_cookie_jar(jar);
    }

    // Viewport/scroll methods are in viewport.rs
    // Navigation queue methods (open_tab, navigate_iframe, pending_navigation,
    // pending_history, history_length) are in navigation.rs

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

    // Canvas 2D context methods are in canvas.rs

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
    }
}

/// Parse a raw CSS rule string into a `CssomRule`.
///
/// Recursively walk children of a form entity, collecting submittable name/value pairs.
fn collect_form_data_recursive(dom: &EcsDom, parent: Entity, pairs: &mut Vec<(String, String)>) {
    let mut child_opt = dom.get_first_child(parent);
    while let Some(child) = child_opt {
        // Check if this child has a FormControlState.
        if let Ok(fcs) = dom.world().get::<&elidex_form::FormControlState>(child) {
            // Skip disabled controls and controls without a name.
            if !fcs.disabled && !fcs.name.is_empty() {
                // For checkbox/radio, only include if checked.
                if fcs.kind == elidex_form::FormControlKind::Checkbox
                    || fcs.kind == elidex_form::FormControlKind::Radio
                {
                    if fcs.checked {
                        let value = if fcs.value().is_empty() {
                            "on".to_string()
                        } else {
                            fcs.value().to_string()
                        };
                        pairs.push((fcs.name.clone(), value));
                    }
                } else if fcs.kind.is_submittable() {
                    pairs.push((fcs.name.clone(), fcs.value().to_string()));
                }
            }
        }
        // Recurse into children (fieldset, div, etc. can contain form controls).
        collect_form_data_recursive(dom, child, pairs);
        child_opt = dom.get_next_sibling(child);
    }
}

/// Performs lightweight parsing without the full CSS parser: splits on `{`
/// to extract the selector and the declaration block. Returns `None` if
/// the text doesn't contain a valid `selector { declarations }` structure.
fn parse_cssom_rule_from_text(text: &str) -> Option<CssomRule> {
    let text = text.trim();
    let brace_pos = text.find('{')?;
    let selector_text = text[..brace_pos].trim().to_string();
    if selector_text.is_empty() {
        return None;
    }
    let body = text[brace_pos + 1..].trim();
    let body = body.strip_suffix('}').unwrap_or(body).trim();
    let declarations: Vec<(String, String)> = body
        .split(';')
        .filter_map(|decl| {
            let decl = decl.trim();
            if decl.is_empty() {
                return None;
            }
            let (prop, val) = decl.split_once(':')?;
            Some((prop.trim().to_string(), val.trim().to_string()))
        })
        .collect();
    Some(CssomRule {
        selector_text,
        declarations,
    })
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
        for obj in inner.custom_element_constructors.values() {
            mark(obj);
        }
        for resolvers in inner.when_defined_resolvers.values() {
            for resolver in resolvers {
                mark(resolver);
            }
        }
        for promise in inner.when_defined_promises.values() {
            mark(promise);
        }
        for conn in inner.realtime.ws_iter() {
            if let Some(ref cb) = conn.onopen {
                mark(cb);
            }
            if let Some(ref cb) = conn.onmessage {
                mark(cb);
            }
            if let Some(ref cb) = conn.onerror {
                mark(cb);
            }
            if let Some(ref cb) = conn.onclose {
                mark(cb);
            }
            mark(&conn.js_object);
            for listeners in conn.listener_registry.values() {
                for listener in listeners {
                    mark(listener);
                }
            }
        }
        for conn in inner.realtime.sse_iter() {
            if let Some(ref cb) = conn.onopen {
                mark(cb);
            }
            if let Some(ref cb) = conn.onmessage {
                mark(cb);
            }
            if let Some(ref cb) = conn.onerror {
                mark(cb);
            }
            mark(&conn.js_object);
            for listeners in conn.listener_registry.values() {
                for listener in listeners {
                    mark(listener);
                }
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
