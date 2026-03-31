//! `JsRuntime` — owns a boa `Context` and provides eval with error isolation.

use std::cell::Cell;
use std::rc::Rc;

use boa_engine::{js_string, Context, JsValue, Source};
use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::EventPayload;
use elidex_script_session::{ComponentKind, DispatchEvent, ScriptEngine, SessionCore};

use crate::bridge::HostBridge;
use crate::globals::console::ConsoleOutput;
use crate::globals::timers::TimerQueueHandle;

mod ce;
mod observers;
mod realtime;

/// Drop guard that calls `HostBridge::unbind()` on drop.
///
/// Ensures `unbind()` is called even if boa panics during eval or dispatch,
/// preventing dangling raw pointers from surviving stack unwinding.
struct UnbindGuard<'a>(&'a HostBridge);
impl Drop for UnbindGuard<'_> {
    fn drop(&mut self) {
        self.0.unbind();
    }
}

/// JavaScript runtime wrapping a boa `Context` with elidex globals.
pub struct JsRuntime {
    ctx: Context,
    bridge: HostBridge,
    console_output: ConsoleOutput,
    timer_queue: TimerQueueHandle,
}

/// Re-export `EvalResult` from the engine-agnostic script session crate.
pub use elidex_script_session::EvalResult;

impl JsRuntime {
    /// Create a new JS runtime with elidex globals registered (no fetch support).
    ///
    /// The `document_entity` must be passed to `eval()` and `drain_timers()`
    /// to bind the bridge to the correct document root.
    pub fn new() -> Self {
        Self::with_network(None)
    }

    /// Create a new JS runtime with optional network support.
    ///
    /// If `network_handle` is `Some`, the `fetch()` global is registered and
    /// the bridge is configured to route WS/SSE through the Network Process.
    /// TODO(M4-12): Also wire `Arc<CookieJar>` for `document.cookie` support.
    pub fn with_network(network_handle: Option<Rc<elidex_net::broker::NetworkHandle>>) -> Self {
        let bridge = HostBridge::new();
        if let Some(ref nh) = network_handle {
            bridge.set_network_handle(Rc::clone(nh));
        }
        let console_output = ConsoleOutput::new();
        let timer_queue = TimerQueueHandle::new();

        let mut ctx = Context::default();

        // Register globals.
        crate::globals::register_all_globals(
            &mut ctx,
            &bridge,
            &console_output,
            &timer_queue,
            network_handle,
        );

        // Store timer queue handle in bridge for window.stop() support.
        bridge.set_timer_queue(timer_queue.clone());

        Self {
            ctx,
            bridge,
            console_output,
            timer_queue,
        }
    }

    /// Create a new JS runtime for a dedicated worker thread.
    ///
    /// Registers only the subset of globals available in `WorkerGlobalScope`
    /// (no `document`, `window`, DOM API). The bridge is initialized with
    /// worker-specific state (name, script URL, outgoing message queue).
    pub fn for_worker(
        network_handle: Option<Rc<elidex_net::broker::NetworkHandle>>,
        name: String,
        script_url: url::Url,
    ) -> Self {
        let bridge = HostBridge::new();
        if let Some(ref nh) = network_handle {
            bridge.set_network_handle(Rc::clone(nh));
        }
        bridge.init_worker_state(name, script_url);

        let console_output = ConsoleOutput::new();
        let timer_queue = TimerQueueHandle::new();

        let mut ctx = Context::default();

        // Register worker globals (subset of Web Platform APIs + worker-specific).
        crate::globals::worker_scope::register_worker_globals(
            &mut ctx,
            &bridge,
            &console_output,
            &timer_queue,
            network_handle,
        );

        // Store timer queue handle in bridge for close() timer cleanup.
        bridge.set_timer_queue(timer_queue.clone());

        Self {
            ctx,
            bridge,
            console_output,
            timer_queue,
        }
    }

    /// Evaluate a JavaScript source string.
    ///
    /// The bridge is bound to `session` and `dom` for the duration of eval,
    /// then unbound. Errors are caught and returned (never propagated).
    ///
    /// A drop guard ensures `unbind()` is called even if boa panics, preventing
    /// dangling raw pointers from surviving stack unwinding.
    pub fn eval(
        &mut self,
        source: &str,
        session: &mut SessionCore,
        dom: &mut EcsDom,
        document_entity: Entity,
    ) -> EvalResult {
        // Sandbox allow-scripts check: if scripts are blocked, skip evaluation.
        if !self.bridge.scripts_allowed() {
            return EvalResult {
                success: true,
                error: None,
            };
        }
        self.bridge.bind(session, dom, document_entity);
        let guard = UnbindGuard(&self.bridge);

        let result = self.ctx.eval(Source::from_bytes(source));

        // Run microtask queue (Promise .then() callbacks) while bridge is bound.
        let jobs_result = self.ctx.run_jobs();

        drop(guard);

        // Drain any events queued during eval (e.g. checkValidity → "invalid").
        self.drain_queued_events(session, dom, document_entity);

        // Drain custom element reactions (upgrade/connected/disconnected/attributeChanged).
        self.drain_custom_element_reactions(session, dom, document_entity);

        match (result, jobs_result) {
            (Err(err), _) => {
                let msg = err.to_string();
                eprintln!("[JS Error] {msg}");
                EvalResult {
                    success: false,
                    error: Some(msg),
                }
            }
            (Ok(_), Err(err)) => {
                let msg = format!("Microtask error: {err}");
                eprintln!("[JS Microtask Error] {err}");
                EvalResult {
                    success: false,
                    error: Some(msg),
                }
            }
            (Ok(_), Ok(())) => EvalResult {
                success: true,
                error: None,
            },
        }
    }

    /// Drain and execute all ready timers.
    ///
    /// Returns a `Vec<EvalResult>` for each timer callback executed.
    /// Failed callbacks are logged but do not prevent subsequent timers
    /// from executing.
    pub fn drain_timers(
        &mut self,
        session: &mut SessionCore,
        dom: &mut EcsDom,
        document_entity: Entity,
    ) -> Vec<EvalResult> {
        let ready = self.timer_queue.borrow_mut().drain_ready();
        let mut results = Vec::with_capacity(ready.len());
        for (_id, callback) in ready {
            results.push(self.eval(&callback, session, dom, document_entity));
        }
        results
    }

    /// Dispatch a DOM event through the propagation path, invoking JS listeners.
    ///
    /// After dispatch completes, drains any queued events (e.g. from
    /// `checkValidity()`) and dispatches them. Returns `true` if
    /// `preventDefault()` was called on the original event.
    pub fn dispatch_event(
        &mut self,
        event: &mut DispatchEvent,
        session: &mut SessionCore,
        dom: &mut EcsDom,
        document_entity: Entity,
    ) -> bool {
        let prevented = self.dispatch_event_inner(event, session, dom, document_entity);
        self.drain_queued_events(session, dom, document_entity);
        self.drain_custom_element_reactions(session, dom, document_entity);
        prevented
    }

    /// Check if an entity is an `<iframe>` element.
    ///
    /// Used by `dispatch_event_inner` to pass the `is_iframe` flag to
    /// `create_element_wrapper` (which registers iframe-specific JS properties).
    fn is_iframe_entity(dom: &elidex_ecs::EcsDom, entity: Entity) -> bool {
        dom.world()
            .get::<&elidex_ecs::TagType>(entity)
            .ok()
            .is_some_and(|t| t.0 == "iframe")
    }

    /// Internal dispatch without draining the event queue.
    ///
    /// Used by `drain_queued_events` to avoid recursion.
    #[allow(clippy::too_many_lines)]
    fn dispatch_event_inner(
        &mut self,
        event: &mut DispatchEvent,
        session: &mut SessionCore,
        dom: &mut EcsDom,
        document_entity: Entity,
    ) -> bool {
        self.bridge.bind(session, dom, document_entity);
        let _guard = UnbindGuard(&self.bridge);

        // Phase 1: Pre-compute dispatch plan and composed path under a short
        // DOM borrow. The borrow is released before any listener callbacks run,
        // eliminating the aliased &EcsDom / &mut EcsDom UB.
        let plan = self.bridge.with(|_session, dom| {
            let plan = elidex_script_session::build_dispatch_plan(dom, event);
            event.composed_path =
                elidex_script_session::build_propagation_path(dom, event.target, event.composed);
            event.dispatch_flag = true;
            plan
        });

        // Shared flags for JS event methods to write back into the dispatch loop.
        let prevent_default_flag = Rc::new(Cell::new(event.flags.default_prevented));
        let stop_propagation_flag = Rc::new(Cell::new(event.flags.propagation_stopped));
        let stop_immediate_flag = Rc::new(Cell::new(event.flags.immediate_propagation_stopped));

        let bridge = self.bridge.clone();
        let ctx = &mut self.ctx;
        let saved_target = event.target;

        // Phase 2: Iterate the pre-built plan, invoking listeners with bridge.with()
        // for each DOM access. No aliased borrows.
        let at_target_slice: &[(Entity, Vec<elidex_script_session::ListenerId>)] = plan
            .at_target
            .as_ref()
            .map_or(&[], |t| std::slice::from_ref(t));
        #[allow(clippy::type_complexity)]
        let phases: [(
            &[(Entity, Vec<elidex_script_session::ListenerId>)],
            elidex_plugin::EventPhase,
        ); 3] = [
            (&plan.capture, elidex_plugin::EventPhase::Capturing),
            (at_target_slice, elidex_plugin::EventPhase::AtTarget),
            (&plan.bubble, elidex_plugin::EventPhase::Bubbling),
        ];

        for (entries, phase) in &phases {
            // Skip bubble phase if event doesn't bubble.
            if *phase == elidex_plugin::EventPhase::Bubbling && !event.bubbles {
                continue;
            }

            for (entity, listener_ids) in *entries {
                if stop_propagation_flag.get() {
                    break;
                }

                // Retarget for shadow DOM.
                let retargeted_target = bridge.with(|_session, dom| {
                    elidex_script_session::retarget(dom, saved_target, *entity)
                });
                event.target = retargeted_target;
                event.current_target = Some(*entity);
                event.phase = *phase;

                for listener_id in listener_ids {
                    if stop_immediate_flag.get() {
                        break;
                    }

                    // Look up listener metadata for once/passive options.
                    let (is_once, is_passive) = bridge.with(|_session, dom| {
                        dom.world()
                            .get::<&elidex_script_session::EventListeners>(*entity)
                            .ok()
                            .map_or((false, false), |listeners| {
                                listeners
                                    .find_entry(*listener_id)
                                    .map_or((false, false), |e| (e.once, e.passive))
                            })
                    });

                    let Some(js_func) = bridge.get_listener(*listener_id) else {
                        continue;
                    };

                    // WHATWG DOM §2.10 step 15: remove once listeners BEFORE invoking.
                    if is_once {
                        bridge.with(|_session, dom| {
                            if let Ok(mut listeners) =
                                dom.world_mut()
                                    .get::<&mut elidex_script_session::EventListeners>(*entity)
                            {
                                listeners.remove(*listener_id);
                            }
                        });
                        bridge.remove_listener(*listener_id);
                    }

                    // Create element wrappers for target and currentTarget.
                    let target_wrapper = bridge.with(|session, dom| {
                        let obj_ref =
                            session.get_or_create_wrapper(event.target, ComponentKind::Element);
                        let is_iframe = Self::is_iframe_entity(dom, event.target);
                        crate::globals::element::create_element_wrapper(
                            event.target,
                            &bridge,
                            obj_ref,
                            ctx,
                            is_iframe,
                        )
                    });
                    let current_target_wrapper = bridge.with(|session, dom| {
                        let obj_ref =
                            session.get_or_create_wrapper(*entity, ComponentKind::Element);
                        let is_iframe = Self::is_iframe_entity(dom, *entity);
                        crate::globals::element::create_element_wrapper(
                            *entity, &bridge, obj_ref, ctx, is_iframe,
                        )
                    });

                    // Build composedPath() array with per-listener filtering.
                    let filtered_path = bridge.with(|_session, dom| {
                        elidex_script_session::composed_path_for_js(event, dom)
                    });
                    let composed_path_array = if filtered_path.is_empty() {
                        None
                    } else {
                        let arr = boa_engine::object::builtins::JsArray::new(ctx);
                        for &path_entity in &filtered_path {
                            let wrapper = bridge.with(|session, dom| {
                                let obj_ref = session
                                    .get_or_create_wrapper(path_entity, ComponentKind::Element);
                                let is_iframe = Self::is_iframe_entity(dom, path_entity);
                                crate::globals::element::create_element_wrapper(
                                    path_entity,
                                    &bridge,
                                    obj_ref,
                                    ctx,
                                    is_iframe,
                                )
                            });
                            let _ = arr.push(wrapper, ctx);
                        }
                        Some(JsValue::from(arr))
                    };

                    // Passive listeners: event.cancelable stays true (observable),
                    // but preventDefault() is a no-op — use a disconnected Cell
                    // seeded with the current state so defaultPrevented is readable.
                    let event_flags = crate::globals::events::EventFlags {
                        prevent_default: if is_passive {
                            Rc::new(Cell::new(prevent_default_flag.get()))
                        } else {
                            Rc::clone(&prevent_default_flag)
                        },
                        stop_propagation: Rc::clone(&stop_propagation_flag),
                        stop_immediate: Rc::clone(&stop_immediate_flag),
                    };
                    let event_obj = crate::globals::events::create_event_object(
                        event,
                        &target_wrapper,
                        &current_target_wrapper,
                        &event_flags,
                        composed_path_array,
                        ctx,
                    );

                    // UI Events §5.2: resolve relatedTarget for focus events.
                    if let EventPayload::Focus(ref f) = event.payload {
                        if let Some(related_bits) = f.related_target {
                            if let Some(related_entity) = Entity::from_bits(related_bits) {
                                let wrapper = bridge.with(|session, dom| {
                                    let obj_ref = session.get_or_create_wrapper(
                                        related_entity,
                                        ComponentKind::Element,
                                    );
                                    let is_iframe = Self::is_iframe_entity(dom, related_entity);
                                    crate::globals::element::create_element_wrapper(
                                        related_entity,
                                        &bridge,
                                        obj_ref,
                                        ctx,
                                        is_iframe,
                                    )
                                });
                                if let Some(obj) = event_obj.as_object() {
                                    let _ =
                                        obj.set(js_string!("relatedTarget"), wrapper, false, ctx);
                                }
                            }
                        }
                    }

                    // Call the listener function with `this` = currentTarget.
                    if let Err(err) = js_func.call(&current_target_wrapper, &[event_obj], ctx) {
                        eprintln!("[JS Event Error] {err}");
                    }

                    // Microtask checkpoint after each listener (HTML §8.1.7.3).
                    if let Err(err) = ctx.run_jobs() {
                        eprintln!("[JS Microtask Error] {err}");
                    }

                    // Sync flags back.
                    event.flags.default_prevented = prevent_default_flag.get();
                    event.flags.propagation_stopped = stop_propagation_flag.get();
                    event.flags.immediate_propagation_stopped = stop_immediate_flag.get();
                }
            }
        }

        event.dispatch_flag = false;
        event.phase = elidex_plugin::EventPhase::None;
        event.current_target = None;
        // Restore original target after dispatch (retargeting is per-listener only).
        event.target = saved_target;

        event.flags.default_prevented
    }

    /// Drain and dispatch all queued events from the session's event queue.
    ///
    /// Queued events are dispatched via `dispatch_event_inner` (which does
    /// not recurse into this method). Iterates up to `MAX_EVENT_DRAIN_ITERATIONS`
    /// to prevent infinite loops from events that enqueue more events.
    fn drain_queued_events(
        &mut self,
        session: &mut SessionCore,
        dom: &mut EcsDom,
        document_entity: Entity,
    ) {
        const MAX_EVENT_DRAIN_ITERATIONS: usize = 16;

        for _ in 0..MAX_EVENT_DRAIN_ITERATIONS {
            let queued = session.drain_event_queue();
            if queued.is_empty() {
                break;
            }
            for qe in queued {
                if !dom.contains(qe.target) {
                    continue;
                }
                let mut event = DispatchEvent::new(&qe.event_type, qe.target);
                event.cancelable = qe.cancelable;
                event.payload = qe.payload;
                // Non-composed by default for form validation events.
                self.dispatch_event_inner(&mut event, session, dom, document_entity);
            }
        }
    }

    /// Returns captured console output.
    pub fn console_output(&self) -> &ConsoleOutput {
        &self.console_output
    }

    /// Returns a reference to the timer queue handle.
    pub fn timer_queue(&self) -> &TimerQueueHandle {
        &self.timer_queue
    }

    /// Returns a reference to the bridge.
    pub fn bridge(&self) -> &HostBridge {
        &self.bridge
    }

    /// Drain outgoing messages from the worker bridge, converting them to
    /// `WorkerToParent` IPC messages.
    ///
    /// Returns an empty vec if this is not a worker runtime.
    pub fn drain_worker_outgoing(&self) -> Vec<elidex_api_workers::WorkerToParent> {
        use crate::bridge::worker_state::OutgoingMessage;
        self.bridge
            .worker_drain_messages()
            .into_iter()
            .map(|msg| match msg {
                OutgoingMessage::Data(data) => {
                    let origin = self
                        .bridge
                        .worker_script_url()
                        .origin()
                        .ascii_serialization();
                    elidex_api_workers::WorkerToParent::PostMessage { data, origin }
                }
                OutgoingMessage::SerializationError => {
                    elidex_api_workers::WorkerToParent::MessageError
                }
            })
            .collect()
    }

    // --- Navigation state delegates ---

    /// Set the current page URL on the bridge.
    pub fn set_current_url(&self, url: Option<url::Url>) {
        self.bridge.set_current_url(url);
    }

    /// Take the pending navigation request (if any).
    pub fn take_pending_navigation(&self) -> Option<elidex_navigation::NavigationRequest> {
        self.bridge.take_pending_navigation()
    }

    /// Take the pending history action (if any).
    pub fn take_pending_history(&self) -> Option<elidex_navigation::HistoryAction> {
        self.bridge.take_pending_history()
    }

    /// Set the session history length on the bridge.
    pub fn set_history_length(&self, len: usize) {
        self.bridge.set_history_length(len);
    }

    /// Returns the deadline of the next pending timer, if any.
    pub fn next_timer_deadline(&self) -> Option<std::time::Instant> {
        self.timer_queue.borrow().next_deadline()
    }

    /// Dispatch pending WebSocket and SSE events to JS callbacks.
    ///
    /// Called from the content thread frame loop after draining events from
    /// the realtime connection registry.
    pub fn dispatch_realtime_events(
        &mut self,
        ws_events: Vec<(u64, elidex_net::ws::WsEvent)>,
        sse_events: Vec<(u64, elidex_net::sse::SseEvent)>,
        session: &mut elidex_script_session::SessionCore,
        dom: &mut elidex_ecs::EcsDom,
        document: elidex_ecs::Entity,
    ) {
        self.bridge.bind(session, dom, document);
        let _guard = UnbindGuard(&self.bridge);

        realtime::dispatch_realtime_events(ws_events, sse_events, &self.bridge, &mut self.ctx);
    }

    /// Fire `versionchange` event on all open IDB connections for a database.
    ///
    /// Called from the content thread when the browser thread broadcasts
    /// `BrowserToContent::IdbVersionChange` (cross-tab coordination).
    pub fn dispatch_idb_versionchange(
        &mut self,
        db_name: &str,
        old_version: u64,
        new_version: Option<u64>,
        session: &mut elidex_script_session::SessionCore,
        dom: &mut elidex_ecs::EcsDom,
        document: elidex_ecs::Entity,
    ) {
        self.bridge.bind(session, dom, document);
        let _guard = UnbindGuard(&self.bridge);

        self.bridge
            .fire_idb_versionchange(db_name, old_version, new_version, &mut self.ctx);
    }

    /// Build a JS source string that evaluates to a MessageEvent-like object.
    fn build_message_event_script(data_json: &str, origin: &str) -> String {
        format!(
            r#"(function() {{ var __data = JSON.parse({}); return {{ data: __data, origin: {}, lastEventId: "", source: null, ports: [], type: "message", isTrusted: true, bubbles: false, cancelable: false }}; }})()"#,
            serde_json::to_string(data_json).unwrap_or_else(|_| "null".to_string()),
            serde_json::to_string(origin).unwrap_or_else(|_| "\"\"".to_string()),
        )
    }

    /// Build a JS source string that evaluates to an ErrorEvent-like object.
    fn build_error_event_script(message: &str, filename: &str, error_value: &str) -> String {
        format!(
            r#"({{ type: "error", message: {}, filename: {}, lineno: 0, colno: 0, error: {}, isTrusted: true, bubbles: false, cancelable: true }})"#,
            serde_json::to_string(message).unwrap_or_else(|_| "\"\"".to_string()),
            serde_json::to_string(filename).unwrap_or_else(|_| "\"\"".to_string()),
            serde_json::to_string(error_value).unwrap_or_else(|_| "null".to_string()),
        )
    }

    /// Dispatch a `message` event to the worker's global scope handlers.
    ///
    /// Called from the worker thread event loop when a `PostMessage` is received
    /// from the parent. Builds a `MessageEvent`-like object and invokes all
    /// registered `onmessage` / `addEventListener("message", ...)` callbacks.
    pub fn dispatch_worker_message(
        &mut self,
        session: &mut SessionCore,
        dom: &mut EcsDom,
        document_entity: Entity,
        data_json: &str,
        origin: &str,
    ) {
        self.bridge.bind(session, dom, document_entity);
        let _guard = UnbindGuard(&self.bridge);

        let callbacks = self.bridge.worker_get_callbacks("message");
        if callbacks.is_empty() {
            return;
        }

        // Build MessageEvent object via JS eval.
        let event_script = Self::build_message_event_script(data_json, origin);

        let event_result = self.ctx.eval(Source::from_bytes(event_script.as_bytes()));
        let Ok(event_obj) = event_result else {
            return;
        };

        let global = self.ctx.global_object();
        for cb in callbacks {
            let _ = cb.call(
                &JsValue::from(global.clone()),
                std::slice::from_ref(&event_obj),
                &mut self.ctx,
            );
        }

        let _ = self.ctx.run_jobs();
    }

    /// Drain and dispatch all pending worker messages in the parent context.
    ///
    /// Called from the content thread. Returns `true` if any events were dispatched.
    pub fn drain_and_dispatch_worker_events(
        &mut self,
        session: &mut SessionCore,
        dom: &mut EcsDom,
        document_entity: Entity,
    ) -> bool {
        let messages = self.bridge.drain_worker_messages();
        if messages.is_empty() {
            return false;
        }

        let mut had_events = false;

        for (worker_id, msg) in messages {
            match msg {
                elidex_api_workers::WorkerToParent::PostMessage { data, origin } => {
                    let event_script = Self::build_message_event_script(&data, &origin);
                    self.dispatch_parent_worker_event(
                        session,
                        dom,
                        document_entity,
                        worker_id,
                        "message",
                        &event_script,
                    );
                    had_events = true;
                }
                elidex_api_workers::WorkerToParent::Error {
                    message,
                    filename,
                    error_value,
                    ..
                } => {
                    let event_script =
                        Self::build_error_event_script(&message, &filename, &error_value);
                    self.dispatch_parent_worker_event(
                        session,
                        dom,
                        document_entity,
                        worker_id,
                        "error",
                        &event_script,
                    );
                    had_events = true;
                }
                elidex_api_workers::WorkerToParent::Closed => {
                    self.bridge.remove_worker(worker_id);
                }
                elidex_api_workers::WorkerToParent::MessageError => {
                    let event_script = r#"({ type: "messageerror", isTrusted: true, bubbles: false, cancelable: false, data: null, origin: "", lastEventId: "", source: null, ports: [] })"#;
                    self.dispatch_parent_worker_event(
                        session,
                        dom,
                        document_entity,
                        worker_id,
                        "messageerror",
                        event_script,
                    );
                    had_events = true;
                }
            }
        }

        had_events
    }

    /// Dispatch a worker event to parent-side callbacks (IDL handler + addEventListener).
    fn dispatch_parent_worker_event(
        &mut self,
        session: &mut SessionCore,
        dom: &mut EcsDom,
        document_entity: Entity,
        worker_id: u64,
        event_type: &str,
        event_script: &str,
    ) {
        let callbacks = self
            .bridge
            .get_parent_worker_callbacks(worker_id, event_type);
        if callbacks.is_empty() {
            // WHATWG HTML §10.1.3: if no error handler is registered, report
            // the error to the console so it is not silently swallowed.
            if event_type == "error" {
                tracing::warn!("Unhandled worker error (worker_id={worker_id}): {event_script}");
            }
            return;
        }

        self.bridge.bind(session, dom, document_entity);
        let _guard = UnbindGuard(&self.bridge);

        let Ok(event_obj) = self.ctx.eval(Source::from_bytes(event_script.as_bytes())) else {
            return;
        };

        let this_val = self
            .bridge
            .get_worker_js_object(worker_id)
            .map_or_else(|| JsValue::from(self.ctx.global_object()), JsValue::from);
        for cb in callbacks {
            let _ = cb.call(&this_val, std::slice::from_ref(&event_obj), &mut self.ctx);
        }
        let _ = self.ctx.run_jobs();
    }

    /// Dispatch an `error` event to the worker's global scope handlers.
    pub fn dispatch_worker_error(
        &mut self,
        session: &mut SessionCore,
        dom: &mut EcsDom,
        document_entity: Entity,
        message: &str,
        filename: &str,
    ) {
        self.bridge.bind(session, dom, document_entity);
        let _guard = UnbindGuard(&self.bridge);

        let callbacks = self.bridge.worker_get_callbacks("error");
        if callbacks.is_empty() {
            // WHATWG HTML §10.2.4: if no error handler is registered, report
            // the error to the console so it is not silently swallowed.
            tracing::warn!("Unhandled error in worker: {message} ({filename})");
            return;
        }

        let event_script = Self::build_error_event_script(message, filename, message);

        let event_result = self.ctx.eval(Source::from_bytes(event_script.as_bytes()));
        let Ok(event_obj) = event_result else {
            return;
        };

        let global = self.ctx.global_object();
        for cb in callbacks {
            let _ = cb.call(
                &JsValue::from(global.clone()),
                std::slice::from_ref(&event_obj),
                &mut self.ctx,
            );
        }

        let _ = self.ctx.run_jobs();
    }
}

/// Walk a subtree and enqueue `Upgrade` reactions for undefined custom elements
/// that have a registered definition.
///
/// This handles the case where innerHTML-parsed custom elements are inserted
/// into a connected tree after their definition has already been registered.
pub(crate) fn walk_subtree_for_upgrade(
    entity: Entity,
    bridge: &HostBridge,
    dom: &EcsDom,
    depth: usize,
) {
    use elidex_custom_elements::{CEState, CustomElementReaction, CustomElementState};

    if depth > elidex_ecs::MAX_ANCESTOR_DEPTH {
        return;
    }
    if let Ok(ce_state) = dom.world().get::<&CustomElementState>(entity) {
        if ce_state.state == CEState::Undefined {
            let should_upgrade = bridge.with_ce_definition(&ce_state.definition_name, |def| {
                def.extends.as_ref().is_none_or(|ext| {
                    dom.world()
                        .get::<&elidex_ecs::TagType>(entity)
                        .ok()
                        .is_some_and(|tag| tag.0.eq_ignore_ascii_case(ext))
                })
            });
            if should_upgrade {
                bridge.enqueue_ce_reaction(CustomElementReaction::Upgrade(entity));
            }
        }
    }
    let mut child = dom.get_first_child(entity);
    while let Some(c) = child {
        walk_subtree_for_upgrade(c, bridge, dom, depth + 1);
        child = dom.get_next_sibling(c);
    }
}

/// Check if an entity is connected to the document (has a parent chain to a
/// `NodeKind::Document` root).
pub(crate) fn is_connected_to_document(entity: Entity, dom: &EcsDom) -> bool {
    let mut current = entity;
    let mut depth = 0;
    loop {
        if depth > elidex_ecs::MAX_ANCESTOR_DEPTH {
            return false; // Safety limit.
        }
        match dom.get_parent(current) {
            Some(parent) => {
                current = parent;
                depth += 1;
            }
            None => {
                // Reached root — check if it's a Document node.
                return dom
                    .world()
                    .get::<&elidex_ecs::NodeKind>(current)
                    .is_ok_and(|nk| *nk == elidex_ecs::NodeKind::Document);
            }
        }
    }
}

impl ScriptEngine for JsRuntime {
    fn eval(
        &mut self,
        source: &str,
        session: &mut SessionCore,
        dom: &mut EcsDom,
        document: Entity,
    ) -> EvalResult {
        self.eval(source, session, dom, document)
    }

    fn dispatch_event(
        &mut self,
        event: &mut DispatchEvent,
        session: &mut SessionCore,
        dom: &mut EcsDom,
        document: Entity,
    ) -> bool {
        self.dispatch_event(event, session, dom, document)
    }

    fn drain_timers(
        &mut self,
        session: &mut SessionCore,
        dom: &mut EcsDom,
        document: Entity,
    ) -> Vec<EvalResult> {
        self.drain_timers(session, dom, document)
    }
}

impl Default for JsRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
