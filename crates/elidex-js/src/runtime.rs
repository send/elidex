//! `JsRuntime` — owns a boa `Context` and provides eval with error isolation.

use boa_engine::{Context, Source};
use elidex_ecs::{EcsDom, Entity};
use elidex_script_session::SessionCore;

use crate::bridge::HostBridge;
use crate::globals::console::ConsoleOutput;
use crate::globals::timers::TimerQueueHandle;

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
    /// Create a new JS runtime with elidex globals registered.
    ///
    /// The `document_entity` must be passed to `eval()` and `drain_timers()`
    /// to bind the bridge to the correct document root.
    pub fn new() -> Self {
        let bridge = HostBridge::new();
        let console_output = ConsoleOutput::new();
        let timer_queue = TimerQueueHandle::new();

        let mut ctx = Context::default();

        // Register globals.
        crate::globals::register_all_globals(&mut ctx, &bridge, &console_output, &timer_queue);

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
        // Guard ensures unbind() on both normal return and panic unwind.
        struct UnbindGuard<'a>(&'a HostBridge);
        impl Drop for UnbindGuard<'_> {
            fn drop(&mut self) {
                self.0.unbind();
            }
        }

        self.bridge.bind(session, dom, document_entity);
        let guard = UnbindGuard(&self.bridge);

        let result = self.ctx.eval(Source::from_bytes(source));

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
}

impl Default for JsRuntime {
    fn default() -> Self {
        Self::new()
    }
}
