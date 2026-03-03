//! `JsRuntime` — owns a boa `Context` and provides eval with error isolation.

use std::cell::Cell;
use std::rc::Rc;

use boa_engine::{Context, JsValue, Source};
use elidex_ecs::{EcsDom, Entity};
use elidex_script_session::{ComponentKind, DispatchEvent, SessionCore};

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

/// Result of evaluating a script.
#[derive(Debug)]
pub struct EvalResult {
    /// `true` if the script completed without error.
    pub success: bool,
    /// Error message if the script failed, `None` if success.
    pub error: Option<String>,
}

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
        self.ctx.run_jobs();

        drop(guard);

        match result {
            Ok(_) => EvalResult {
                success: true,
                error: None,
            },
            Err(err) => {
                let msg = err.to_string();
                eprintln!("[JS Error] {msg}");
                EvalResult {
                    success: false,
                    error: Some(msg),
                }
            }
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
    /// The bridge is bound for the duration of dispatch, then unbound.
    /// Returns `true` if `preventDefault()` was called.
    pub fn dispatch_event(
        &mut self,
        event: &mut DispatchEvent,
        session: &mut SessionCore,
        dom: &mut EcsDom,
        document_entity: Entity,
    ) -> bool {
        self.bridge.bind(session, dom, document_entity);
        let _guard = UnbindGuard(&self.bridge);

        // Shared flags for JS event methods to write back into the dispatch loop.
        let prevent_default_flag = Rc::new(Cell::new(event.default_prevented));
        let stop_propagation_flag = Rc::new(Cell::new(event.propagation_stopped));
        let stop_immediate_flag = Rc::new(Cell::new(event.immediate_propagation_stopped));

        let bridge = self.bridge.clone();
        let ctx = &mut self.ctx;

        elidex_script_session::dispatch_event(dom, event, &mut |listener_id, _entity, ev| {
            // Sync flags from Rc<Cell> into the event before checking.
            ev.default_prevented = prevent_default_flag.get();
            ev.propagation_stopped = stop_propagation_flag.get();
            ev.immediate_propagation_stopped = stop_immediate_flag.get();

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

            let event_obj = crate::globals::events::create_event_object(
                ev,
                &target_wrapper,
                &current_target_wrapper,
                &prevent_default_flag,
                &stop_propagation_flag,
                &stop_immediate_flag,
                ctx,
            );

            // Call the listener function with `this` = currentTarget.
            if let Err(err) = js_func.call(&current_target_wrapper, &[event_obj], ctx) {
                eprintln!("[JS Event Error] {err}");
            }

            // Sync flags back from Rc<Cell> into the event.
            ev.default_prevented = prevent_default_flag.get();
            ev.propagation_stopped = stop_propagation_flag.get();
            ev.immediate_propagation_stopped = stop_immediate_flag.get();
        });

        // Run microtask queue (Promise .then() callbacks) while bridge is bound.
        ctx.run_jobs();

        // Sync flags after microtask queue processing — microtasks may have
        // called preventDefault() via the shared Rc<Cell> flags.
        event.default_prevented = prevent_default_flag.get();
        event.propagation_stopped = stop_propagation_flag.get();
        event.immediate_propagation_stopped = stop_immediate_flag.get();

        event.default_prevented
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
}

impl Default for JsRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_ecs::{Attributes, EcsDom};
    use elidex_plugin::{EventPayload, MouseEventInit};

    fn setup() -> (JsRuntime, SessionCore, EcsDom, Entity) {
        let runtime = JsRuntime::new();
        let session = SessionCore::new();
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        (runtime, session, dom, doc)
    }

    #[test]
    fn add_event_listener_registers_in_ecs() {
        let (mut runtime, mut session, mut dom, doc) = setup();
        let div = dom.create_element("div", Attributes::default());
        let _ = dom.append_child(doc, div);

        runtime.eval(
            r"
            var el = document.querySelector('div');
            el.addEventListener('click', function() {});
            ",
            &mut session,
            &mut dom,
            doc,
        );

        let listeners = dom
            .world()
            .get::<&elidex_script_session::EventListeners>(div)
            .unwrap();
        assert_eq!(listeners.len(), 1);
    }

    #[test]
    fn remove_event_listener_clears() {
        let (mut runtime, mut session, mut dom, doc) = setup();
        let div = dom.create_element("div", Attributes::default());
        let _ = dom.append_child(doc, div);

        runtime.eval(
            r"
            var handler = function() {};
            var el = document.querySelector('div');
            el.addEventListener('click', handler);
            el.removeEventListener('click', handler);
            ",
            &mut session,
            &mut dom,
            doc,
        );

        let listeners = dom
            .world()
            .get::<&elidex_script_session::EventListeners>(div)
            .unwrap();
        assert_eq!(listeners.len(), 0);
    }

    #[test]
    fn duplicate_add_event_listener_ignored() {
        let (mut runtime, mut session, mut dom, doc) = setup();
        let div = dom.create_element("div", Attributes::default());
        let _ = dom.append_child(doc, div);

        runtime.eval(
            r"
            var handler = function() {};
            var el = document.querySelector('div');
            el.addEventListener('click', handler);
            el.addEventListener('click', handler);
            ",
            &mut session,
            &mut dom,
            doc,
        );

        let listeners = dom
            .world()
            .get::<&elidex_script_session::EventListeners>(div)
            .unwrap();
        assert_eq!(listeners.len(), 1);
    }

    #[test]
    fn capture_flag_mismatch_keeps_listener() {
        let (mut runtime, mut session, mut dom, doc) = setup();
        let div = dom.create_element("div", Attributes::default());
        let _ = dom.append_child(doc, div);

        runtime.eval(
            r"
            var handler = function() {};
            var el = document.querySelector('div');
            el.addEventListener('click', handler, true);
            el.removeEventListener('click', handler, false);
            ",
            &mut session,
            &mut dom,
            doc,
        );

        let listeners = dom
            .world()
            .get::<&elidex_script_session::EventListeners>(div)
            .unwrap();
        assert_eq!(listeners.len(), 1);
    }

    #[test]
    fn dispatch_event_invokes_listener() {
        let (mut runtime, mut session, mut dom, doc) = setup();
        let div = dom.create_element("div", Attributes::default());
        let _ = dom.append_child(doc, div);

        runtime.eval(
            r"
            var el = document.querySelector('div');
            el.addEventListener('click', function(e) {
                e.target.textContent = 'clicked';
            });
            ",
            &mut session,
            &mut dom,
            doc,
        );
        session.flush(&mut dom);

        let mut event = DispatchEvent::new("click", div);
        event.payload = EventPayload::Mouse(MouseEventInit {
            client_x: 50.0,
            client_y: 50.0,
            ..Default::default()
        });

        runtime.dispatch_event(&mut event, &mut session, &mut dom, doc);
        session.flush(&mut dom);

        let text = dom
            .world()
            .get::<&elidex_ecs::TextContent>(dom.get_first_child(div).unwrap())
            .map(|t| t.0.clone())
            .unwrap_or_default();
        assert_eq!(text, "clicked");
    }

    #[test]
    fn dispatch_event_prevent_default() {
        let (mut runtime, mut session, mut dom, doc) = setup();
        let div = dom.create_element("div", Attributes::default());
        let _ = dom.append_child(doc, div);

        runtime.eval(
            r"
            var el = document.querySelector('div');
            el.addEventListener('click', function(e) {
                e.preventDefault();
            });
            ",
            &mut session,
            &mut dom,
            doc,
        );
        session.flush(&mut dom);

        let mut event = DispatchEvent::new("click", div);
        let prevented = runtime.dispatch_event(&mut event, &mut session, &mut dom, doc);
        assert!(prevented);
    }

    #[test]
    fn dispatch_event_stop_propagation() {
        let (mut runtime, mut session, mut dom, doc) = setup();
        let outer = dom.create_element("div", Attributes::default());
        let inner = dom.create_element("span", Attributes::default());
        let _ = dom.append_child(doc, outer);
        let _ = dom.append_child(outer, inner);

        // Listener on inner that stops propagation.
        runtime.eval(
            r"
            var inner = document.querySelector('span');
            inner.addEventListener('click', function(e) {
                e.stopPropagation();
                console.log('inner-click');
            });
            ",
            &mut session,
            &mut dom,
            doc,
        );
        // Register outer listener separately.
        runtime.eval(
            r"
            var outer = document.querySelector('div');
            outer.addEventListener('click', function(e) {
                console.log('outer-click');
            });
            ",
            &mut session,
            &mut dom,
            doc,
        );
        session.flush(&mut dom);

        let mut event = DispatchEvent::new("click", inner);
        runtime.dispatch_event(&mut event, &mut session, &mut dom, doc);

        let output = runtime.console_output().messages();
        let has_inner = output.iter().any(|m| m.1.contains("inner-click"));
        let has_outer = output.iter().any(|m| m.1.contains("outer-click"));
        assert!(
            has_inner,
            "inner listener should fire, messages: {output:?}"
        );
        assert!(
            !has_outer,
            "outer listener should NOT fire due to stopPropagation, messages: {output:?}"
        );
    }

    #[test]
    fn event_mouse_properties() {
        let (mut runtime, mut session, mut dom, doc) = setup();
        let div = dom.create_element("div", Attributes::default());
        let _ = dom.append_child(doc, div);

        runtime.eval(
            r"
            var el = document.querySelector('div');
            el.addEventListener('click', function(e) {
                console.log('x=' + e.clientX + ' y=' + e.clientY);
            });
            ",
            &mut session,
            &mut dom,
            doc,
        );
        session.flush(&mut dom);

        let mut event = DispatchEvent::new("click", div);
        event.payload = EventPayload::Mouse(MouseEventInit {
            client_x: 123.0,
            client_y: 456.0,
            ..Default::default()
        });
        runtime.dispatch_event(&mut event, &mut session, &mut dom, doc);

        let output = runtime.console_output().messages();
        assert!(output
            .iter()
            .any(|m| m.1.contains("x=123") && m.1.contains("y=456")));
    }

    #[test]
    fn event_keyboard_properties() {
        use elidex_plugin::KeyboardEventInit;

        let (mut runtime, mut session, mut dom, doc) = setup();
        let div = dom.create_element("div", Attributes::default());
        let _ = dom.append_child(doc, div);

        runtime.eval(
            r"
            var el = document.querySelector('div');
            el.addEventListener('keydown', function(e) {
                console.log('key=' + e.key + ' code=' + e.code);
            });
            ",
            &mut session,
            &mut dom,
            doc,
        );
        session.flush(&mut dom);

        let mut event = DispatchEvent::new("keydown", div);
        event.payload = EventPayload::Keyboard(KeyboardEventInit {
            key: "Enter".into(),
            code: "Enter".into(),
            ..Default::default()
        });
        runtime.dispatch_event(&mut event, &mut session, &mut dom, doc);

        let output = runtime.console_output().messages();
        assert!(output
            .iter()
            .any(|m| m.1.contains("key=Enter") && m.1.contains("code=Enter")));
    }

    #[test]
    fn event_bubbles_to_parent() {
        let (mut runtime, mut session, mut dom, doc) = setup();
        let outer = dom.create_element("div", Attributes::default());
        let inner = dom.create_element("span", Attributes::default());
        let _ = dom.append_child(doc, outer);
        let _ = dom.append_child(outer, inner);

        runtime.eval(
            r"
            var outer = document.querySelector('div');
            outer.addEventListener('click', function(e) {
                console.log('bubbled');
            });
            ",
            &mut session,
            &mut dom,
            doc,
        );
        session.flush(&mut dom);

        // Dispatch on inner — should bubble to outer.
        let mut event = DispatchEvent::new("click", inner);
        runtime.dispatch_event(&mut event, &mut session, &mut dom, doc);

        let output = runtime.console_output().messages();
        assert!(output.iter().any(|m| m.1.contains("bubbled")));
    }

    #[test]
    fn listener_store_gc_trace() {
        // Verify that creating a runtime with listeners doesn't panic
        // during boa's GC cycle (which would happen if Trace is wrong).
        let (mut runtime, mut session, mut dom, doc) = setup();
        let div = dom.create_element("div", Attributes::default());
        let _ = dom.append_child(doc, div);

        runtime.eval(
            r"
            var el = document.querySelector('div');
            el.addEventListener('click', function() {});
            el.addEventListener('keydown', function() {});
            // Force some allocations to potentially trigger GC.
            for (var i = 0; i < 100; i++) {
                var obj = { value: i };
            }
            ",
            &mut session,
            &mut dom,
            doc,
        );
        // If we get here without panic, GC trace is working.
    }

    // --- Promise / run_jobs integration tests ---

    #[test]
    fn eval_runs_promise_microtasks() {
        // Promise.resolve().then() callback should fire during eval
        // because run_jobs() is called while bridge is still bound.
        let (mut runtime, mut session, mut dom, doc) = setup();

        let result = runtime.eval(
            "var resolved = false;\
             Promise.resolve(42).then(function(v) { resolved = v; });",
            &mut session,
            &mut dom,
            doc,
        );
        assert!(result.success);

        // Check that the .then() callback ran.
        runtime.eval(
            "console.log('resolved=' + resolved);",
            &mut session,
            &mut dom,
            doc,
        );
        let messages = runtime.console_output().messages();
        assert!(
            messages.iter().any(|m| m.1.contains("resolved=42")),
            "Expected resolved=42 in console output, got: {messages:?}"
        );
    }

    #[test]
    fn eval_promise_chain() {
        // Multi-step promise chain should fully resolve.
        let (mut runtime, mut session, mut dom, doc) = setup();

        runtime.eval(
            "var result = 0;\
             Promise.resolve(1)\
                 .then(function(v) { return v + 1; })\
                 .then(function(v) { return v * 3; })\
                 .then(function(v) { result = v; });",
            &mut session,
            &mut dom,
            doc,
        );

        runtime.eval(
            "console.log('chain=' + result);",
            &mut session,
            &mut dom,
            doc,
        );
        let messages = runtime.console_output().messages();
        assert!(
            messages.iter().any(|m| m.1.contains("chain=6")),
            "Expected chain=6 (1+1=2, 2*3=6), got: {messages:?}"
        );
    }

    #[test]
    fn dispatch_event_runs_promise_microtasks() {
        // Promise microtasks in event handlers should fire during dispatch.
        let (mut runtime, mut session, mut dom, doc) = setup();
        let div = dom.create_element("div", Attributes::default());
        let _ = dom.append_child(doc, div);

        runtime.eval(
            "var asyncResult = '';\
             var el = document.querySelector('div');\
             el.addEventListener('click', function(e) {\
                 Promise.resolve('async-ok').then(function(v) {\
                     asyncResult = v;\
                 });\
             });",
            &mut session,
            &mut dom,
            doc,
        );
        session.flush(&mut dom);

        let mut event = DispatchEvent::new("click", div);
        event.payload = EventPayload::Mouse(MouseEventInit {
            client_x: 10.0,
            client_y: 10.0,
            ..Default::default()
        });
        runtime.dispatch_event(&mut event, &mut session, &mut dom, doc);

        // Read the result.
        runtime.eval(
            "console.log('async=' + asyncResult);",
            &mut session,
            &mut dom,
            doc,
        );
        let messages = runtime.console_output().messages();
        assert!(
            messages.iter().any(|m| m.1.contains("async=async-ok")),
            "Expected async=async-ok, got: {messages:?}"
        );
    }

    #[test]
    fn with_fetch_none_is_same_as_new() {
        // JsRuntime::new() and JsRuntime::with_fetch(None) should behave identically.
        let mut runtime = JsRuntime::with_fetch(None);
        let mut session = SessionCore::new();
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();

        let result = runtime.eval("1 + 2", &mut session, &mut dom, doc);
        assert!(result.success);
    }

    // --- document.addEventListener / removeEventListener ---

    #[test]
    fn document_add_event_listener() {
        let (mut runtime, mut session, mut dom, doc) = setup();

        runtime.eval(
            r"
            document.addEventListener('DOMContentLoaded', function() {
                console.log('dcl-handler');
            });
            ",
            &mut session,
            &mut dom,
            doc,
        );

        let listeners = dom
            .world()
            .get::<&elidex_script_session::EventListeners>(doc)
            .unwrap();
        assert_eq!(listeners.len(), 1);
    }

    #[test]
    fn document_remove_event_listener() {
        let (mut runtime, mut session, mut dom, doc) = setup();

        runtime.eval(
            r"
            var handler = function() {};
            document.addEventListener('load', handler);
            document.removeEventListener('load', handler);
            ",
            &mut session,
            &mut dom,
            doc,
        );

        let listeners = dom
            .world()
            .get::<&elidex_script_session::EventListeners>(doc)
            .unwrap();
        assert_eq!(listeners.len(), 0);
    }

    #[test]
    fn document_event_listener_dispatch() {
        let (mut runtime, mut session, mut dom, doc) = setup();

        runtime.eval(
            r"
            document.addEventListener('DOMContentLoaded', function() {
                console.log('dcl-fired');
            });
            ",
            &mut session,
            &mut dom,
            doc,
        );
        session.flush(&mut dom);

        let mut event = DispatchEvent::new("DOMContentLoaded", doc);
        event.cancelable = false;
        runtime.dispatch_event(&mut event, &mut session, &mut dom, doc);

        let output = runtime.console_output().messages();
        assert!(
            output.iter().any(|m| m.1.contains("dcl-fired")),
            "Expected dcl-fired in console output, got: {output:?}"
        );
    }

    // --- M3.5-3: Legacy DOM API stubs ---

    #[test]
    fn document_all_is_undefined() {
        let (mut runtime, mut session, mut dom, doc) = setup();
        runtime.eval(
            r"console.log(typeof document.all);",
            &mut session,
            &mut dom,
            doc,
        );
        let output = runtime.console_output().messages();
        assert!(
            output.iter().any(|m| m.1.contains("undefined")),
            "Expected document.all to be undefined, got: {output:?}"
        );
    }

    #[test]
    fn document_write_does_not_throw() {
        let (mut runtime, mut session, mut dom, doc) = setup();
        runtime.eval(
            r"
            document.write('<p>test</p>');
            document.writeln('test');
            console.log('survived');
            ",
            &mut session,
            &mut dom,
            doc,
        );
        let output = runtime.console_output().messages();
        assert!(
            output.iter().any(|m| m.1.contains("survived")),
            "Expected document.write to not throw, got: {output:?}"
        );
    }
}
