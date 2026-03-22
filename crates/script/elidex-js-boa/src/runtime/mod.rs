//! `JsRuntime` — owns a boa `Context` and provides eval with error isolation.

use std::cell::Cell;
use std::rc::Rc;

use boa_engine::{js_string, Context, JsValue, Source};
use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::EventPayload;
use elidex_script_session::{ComponentKind, DispatchEvent, ScriptEngine, SessionCore};

use elidex_net::FetchHandle;

use crate::bridge::HostBridge;
use crate::globals::console::ConsoleOutput;
use crate::globals::timers::TimerQueueHandle;

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
        Self::with_fetch(None)
    }

    /// Create a new JS runtime with optional fetch support.
    ///
    /// If `fetch_handle` is `Some`, the `fetch()` global is registered.
    /// The `Rc<FetchHandle>` is shared with the navigation layer so that
    /// cookies and connection pools are reused across `fetch()` and navigation.
    pub fn with_fetch(fetch_handle: Option<Rc<FetchHandle>>) -> Self {
        let bridge = HostBridge::new();
        let console_output = ConsoleOutput::new();
        let timer_queue = TimerQueueHandle::new();

        let mut ctx = Context::default();

        // Register globals.
        crate::globals::register_all_globals(
            &mut ctx,
            &bridge,
            &console_output,
            &timer_queue,
            fetch_handle,
        );

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
        self.bridge.bind(session, dom, document_entity);
        let guard = UnbindGuard(&self.bridge);

        let result = self.ctx.eval(Source::from_bytes(source));

        // Run microtask queue (Promise .then() callbacks) while bridge is bound.
        let jobs_result = self.ctx.run_jobs();

        drop(guard);

        // Drain any events queued during eval (e.g. checkValidity → "invalid").
        self.drain_queued_events(session, dom, document_entity);

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
        prevented
    }

    /// Internal dispatch without draining the event queue.
    ///
    /// Used by `drain_queued_events` to avoid recursion.
    fn dispatch_event_inner(
        &mut self,
        event: &mut DispatchEvent,
        session: &mut SessionCore,
        dom: &mut EcsDom,
        document_entity: Entity,
    ) -> bool {
        self.bridge.bind(session, dom, document_entity);
        let _guard = UnbindGuard(&self.bridge);

        // Shared flags for JS event methods to write back into the dispatch loop.
        let prevent_default_flag = Rc::new(Cell::new(event.flags.default_prevented));
        let stop_propagation_flag = Rc::new(Cell::new(event.flags.propagation_stopped));
        let stop_immediate_flag = Rc::new(Cell::new(event.flags.immediate_propagation_stopped));

        let bridge = self.bridge.clone();
        let ctx = &mut self.ctx;

        elidex_script_session::dispatch_event(dom, event, &mut |listener_id, _entity, ev| {
            // Sync flags from Rc<Cell> into the event before checking.
            ev.flags.default_prevented = prevent_default_flag.get();
            ev.flags.propagation_stopped = stop_propagation_flag.get();
            ev.flags.immediate_propagation_stopped = stop_immediate_flag.get();

            let Some(js_func) = bridge.get_listener(listener_id) else {
                return;
            };

            // Create element wrapper for target and current_target.
            let target_wrapper = bridge.with(|session, _dom| {
                let obj_ref = session.get_or_create_wrapper(ev.target, ComponentKind::Element);
                crate::globals::element::create_element_wrapper(ev.target, &bridge, obj_ref, ctx)
            });
            let current_target_wrapper = if let Some(ct) = ev.current_target {
                bridge.with(|session, _dom| {
                    let obj_ref = session.get_or_create_wrapper(ct, ComponentKind::Element);
                    crate::globals::element::create_element_wrapper(ct, &bridge, obj_ref, ctx)
                })
            } else {
                JsValue::null()
            };

            // H1+M5: Build composedPath() array using per-listener filtering.
            // This ensures closed shadow DOM internals are not leaked to
            // listeners outside the shadow tree.
            let filtered_path =
                bridge.with(|_session, dom| elidex_script_session::composed_path_for_js(ev, dom));
            let composed_path_array = if filtered_path.is_empty() {
                None
            } else {
                let arr = boa_engine::object::builtins::JsArray::new(ctx);
                for &path_entity in &filtered_path {
                    let wrapper = bridge.with(|session, _dom| {
                        let obj_ref =
                            session.get_or_create_wrapper(path_entity, ComponentKind::Element);
                        crate::globals::element::create_element_wrapper(
                            path_entity,
                            &bridge,
                            obj_ref,
                            ctx,
                        )
                    });
                    let _ = arr.push(wrapper, ctx);
                }
                Some(JsValue::from(arr))
            };

            let event_flags = crate::globals::events::EventFlags {
                prevent_default: Rc::clone(&prevent_default_flag),
                stop_propagation: Rc::clone(&stop_propagation_flag),
                stop_immediate: Rc::clone(&stop_immediate_flag),
            };
            let event_obj = crate::globals::events::create_event_object(
                ev,
                &target_wrapper,
                &current_target_wrapper,
                &event_flags,
                composed_path_array,
                ctx,
            );

            // UI Events §5.2: resolve relatedTarget for focus events.
            if let EventPayload::Focus(ref f) = ev.payload {
                if let Some(related_bits) = f.related_target {
                    if let Some(related_entity) = Entity::from_bits(related_bits) {
                        let wrapper = bridge.with(|session, _dom| {
                            let obj_ref = session
                                .get_or_create_wrapper(related_entity, ComponentKind::Element);
                            crate::globals::element::create_element_wrapper(
                                related_entity,
                                &bridge,
                                obj_ref,
                                ctx,
                            )
                        });
                        if let Some(obj) = event_obj.as_object() {
                            let _ = obj.set(js_string!("relatedTarget"), wrapper, false, ctx);
                        }
                    }
                }
            }

            // Call the listener function with `this` = currentTarget.
            if let Err(err) = js_func.call(&current_target_wrapper, &[event_obj], ctx) {
                eprintln!("[JS Event Error] {err}");
            }

            // Sync flags back from Rc<Cell> into the event.
            ev.flags.default_prevented = prevent_default_flag.get();
            ev.flags.propagation_stopped = stop_propagation_flag.get();
            ev.flags.immediate_propagation_stopped = stop_immediate_flag.get();
        });

        // Run microtask queue (Promise .then() callbacks) while bridge is bound.
        if let Err(err) = ctx.run_jobs() {
            eprintln!("[JS Microtask Error] {err}");
        }

        // Sync flags after microtask queue processing — microtasks may have
        // called preventDefault() via the shared Rc<Cell> flags.
        event.flags.default_prevented = prevent_default_flag.get();
        event.flags.propagation_stopped = stop_propagation_flag.get();
        event.flags.immediate_propagation_stopped = stop_immediate_flag.get();

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

    /// Deliver mutation records to all `MutationObserver` callbacks.
    ///
    /// Feeds session-level `MutationRecord`s to the observer registries,
    /// then invokes JS callbacks for observers with pending records.
    pub fn deliver_mutation_records(
        &mut self,
        records: &[elidex_script_session::MutationRecord],
        session: &mut SessionCore,
        dom: &mut EcsDom,
        document_entity: Entity,
    ) {
        // Feed records to the registry.
        for record in records {
            self.bridge.with_mutation_observers(|reg| {
                reg.notify(record, &|target, ancestor| {
                    // Walk up the tree from target to check if ancestor is an ancestor.
                    let mut current = dom.get_parent(target);
                    while let Some(node) = current {
                        if node == ancestor {
                            return true;
                        }
                        current = dom.get_parent(node);
                    }
                    false
                });
            });
        }

        // Collect observer IDs with pending records.
        let observer_ids: Vec<u64> = self.bridge.with_mutation_observers(|reg| {
            reg.observers_with_records()
                .map(elidex_api_observers::mutation::MutationObserverId::raw)
                .collect()
        });

        if observer_ids.is_empty() {
            return;
        }

        self.bridge.bind(session, dom, document_entity);
        let _guard = UnbindGuard(&self.bridge);

        for observer_id in observer_ids {
            let mo_id = elidex_api_observers::mutation::MutationObserverId::from_raw(observer_id);
            let records = self
                .bridge
                .with_mutation_observers(|reg| reg.take_records(mo_id));
            if records.is_empty() {
                continue;
            }

            let Some(callback) = self.bridge.get_observer_callback(observer_id) else {
                continue;
            };
            let observer_obj = self
                .bridge
                .get_observer_object(observer_id)
                .map_or(JsValue::undefined(), JsValue::from);

            let arr = boa_engine::object::builtins::JsArray::new(&mut self.ctx);
            for record in &records {
                let obj = crate::globals::observers::mutation_record_to_js(record, &mut self.ctx);
                let _ = arr.push(obj, &mut self.ctx);
            }

            if let Err(err) = callback.call(
                &observer_obj,
                &[JsValue::from(arr), observer_obj.clone()],
                &mut self.ctx,
            ) {
                eprintln!("[JS MutationObserver Error] {err}");
            }
        }

        if let Err(err) = self.ctx.run_jobs() {
            eprintln!("[JS Microtask Error] {err}");
        }
    }

    /// Deliver resize observations to all `ResizeObserver` callbacks.
    ///
    /// Compares current element sizes against last known sizes and invokes
    /// callbacks for observers with changed targets.
    pub fn deliver_resize_observations(
        &mut self,
        session: &mut SessionCore,
        dom: &mut EcsDom,
        document_entity: Entity,
    ) {
        let observations = self.bridge.with_resize_observers(|reg| {
            reg.gather_observations(&|entity| {
                let lb = dom.world().get::<&elidex_plugin::LayoutBox>(entity).ok()?;
                let bb = lb.border_box();
                Some((lb.content.size, bb.size))
            })
        });

        if observations.is_empty() {
            return;
        }

        self.bridge.bind(session, dom, document_entity);
        let _guard = UnbindGuard(&self.bridge);

        for (observer_id_typed, entries) in &observations {
            let observer_id = observer_id_typed.raw();
            let Some(callback) = self.bridge.get_observer_callback(observer_id) else {
                continue;
            };
            let observer_obj = self
                .bridge
                .get_observer_object(observer_id)
                .map_or(JsValue::undefined(), JsValue::from);

            let arr = boa_engine::object::builtins::JsArray::new(&mut self.ctx);
            for entry in entries {
                let obj = resize_entry_to_js(entry, &mut self.ctx);
                let _ = arr.push(obj, &mut self.ctx);
            }

            if let Err(err) = callback.call(
                &observer_obj,
                &[JsValue::from(arr), observer_obj.clone()],
                &mut self.ctx,
            ) {
                eprintln!("[JS ResizeObserver Error] {err}");
            }
        }

        if let Err(err) = self.ctx.run_jobs() {
            eprintln!("[JS Microtask Error] {err}");
        }
    }

    /// Deliver intersection observations to all `IntersectionObserver` callbacks.
    pub fn deliver_intersection_observations(
        &mut self,
        session: &mut SessionCore,
        dom: &mut EcsDom,
        document_entity: Entity,
        viewport: elidex_plugin::Rect,
    ) {
        let observations = self.bridge.with_intersection_observers(|reg| {
            reg.gather_observations(
                &|entity| {
                    let lb = dom.world().get::<&elidex_plugin::LayoutBox>(entity).ok()?;
                    let bb = lb.border_box();
                    Some(elidex_plugin::Rect::new(
                        lb.content.origin.x,
                        lb.content.origin.y,
                        bb.size.width,
                        bb.size.height,
                    ))
                },
                viewport,
            )
        });

        if observations.is_empty() {
            return;
        }

        self.bridge.bind(session, dom, document_entity);
        let _guard = UnbindGuard(&self.bridge);

        for (observer_id_typed, entries) in &observations {
            let observer_id = observer_id_typed.raw();
            let Some(callback) = self.bridge.get_observer_callback(observer_id) else {
                continue;
            };
            let observer_obj = self
                .bridge
                .get_observer_object(observer_id)
                .map_or(JsValue::undefined(), JsValue::from);

            let arr = boa_engine::object::builtins::JsArray::new(&mut self.ctx);
            for entry in entries {
                let obj = intersection_entry_to_js(entry, &mut self.ctx);
                let _ = arr.push(obj, &mut self.ctx);
            }

            if let Err(err) = callback.call(
                &observer_obj,
                &[JsValue::from(arr), observer_obj.clone()],
                &mut self.ctx,
            ) {
                eprintln!("[JS IntersectionObserver Error] {err}");
            }
        }

        if let Err(err) = self.ctx.run_jobs() {
            eprintln!("[JS Microtask Error] {err}");
        }
    }

    /// Dispatch "change" events to `MediaQueryList` listeners whose result changed.
    ///
    /// `changed` is a list of `(media_query_id, new_matches)` pairs returned
    /// by `HostBridge::re_evaluate_media_queries()`.
    pub fn deliver_media_query_changes(
        &mut self,
        changed: &[(u64, bool)],
        session: &mut SessionCore,
        dom: &mut EcsDom,
        document_entity: Entity,
    ) {
        if changed.is_empty() {
            return;
        }

        self.bridge.bind(session, dom, document_entity);
        let _guard = UnbindGuard(&self.bridge);

        for &(id, new_matches) in changed {
            let listeners = self.bridge.media_query_listeners(id);
            if listeners.is_empty() {
                continue;
            }
            let media = self.bridge.media_query_string(id).unwrap_or_default();

            // Build a MediaQueryListEvent-like object.
            let event = ObjectInitializer::new(&mut self.ctx)
                .property(
                    js_string!("matches"),
                    JsValue::from(new_matches),
                    Attribute::READONLY,
                )
                .property(
                    js_string!("media"),
                    JsValue::from(js_string!(media.as_str())),
                    Attribute::READONLY,
                )
                .build();
            let event_val = JsValue::from(event);

            // Build a MediaQueryList-like object to use as `this` per spec.
            let mql_this = ObjectInitializer::new(&mut self.ctx)
                .property(
                    js_string!("matches"),
                    JsValue::from(new_matches),
                    Attribute::READONLY,
                )
                .property(
                    js_string!("media"),
                    JsValue::from(js_string!(media.as_str())),
                    Attribute::READONLY,
                )
                .build();
            let this_val = JsValue::from(mql_this);

            for listener in &listeners {
                if let Err(err) =
                    listener.call(&this_val, std::slice::from_ref(&event_val), &mut self.ctx)
                {
                    eprintln!("[JS MediaQueryList Error] {err}");
                }
            }
        }

        if let Err(err) = self.ctx.run_jobs() {
            eprintln!("[JS Microtask Error] {err}");
        }
    }
}

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;

fn resize_entry_to_js(
    entry: &elidex_api_observers::resize::ResizeObserverEntry,
    ctx: &mut Context,
) -> JsValue {
    let obj = ObjectInitializer::new(ctx)
        .property(
            js_string!("target"),
            JsValue::from(entry.target.to_bits().get() as f64),
            Attribute::all(),
        )
        .property(
            js_string!("contentBoxWidth"),
            JsValue::from(f64::from(entry.content_box_size.width)),
            Attribute::all(),
        )
        .property(
            js_string!("contentBoxHeight"),
            JsValue::from(f64::from(entry.content_box_size.height)),
            Attribute::all(),
        )
        .property(
            js_string!("borderBoxWidth"),
            JsValue::from(f64::from(entry.border_box_size.width)),
            Attribute::all(),
        )
        .property(
            js_string!("borderBoxHeight"),
            JsValue::from(f64::from(entry.border_box_size.height)),
            Attribute::all(),
        )
        .build();
    JsValue::from(obj)
}

fn intersection_entry_to_js(
    entry: &elidex_api_observers::intersection::IntersectionObserverEntry,
    ctx: &mut Context,
) -> JsValue {
    let obj = ObjectInitializer::new(ctx)
        .property(
            js_string!("target"),
            JsValue::from(entry.target.to_bits().get() as f64),
            Attribute::all(),
        )
        .property(
            js_string!("intersectionRatio"),
            JsValue::from(entry.intersection_ratio),
            Attribute::all(),
        )
        .property(
            js_string!("isIntersecting"),
            JsValue::from(entry.is_intersecting),
            Attribute::all(),
        )
        .build();
    JsValue::from(obj)
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
