//! Global object registration for the boa JS context.

pub mod console;
pub mod document;
pub mod element;
pub mod events;
pub mod fetch;
pub mod timers;
pub mod window;

use std::rc::Rc;

use boa_engine::{Context, JsNativeError, JsResult, JsValue};

use crate::bridge::HostBridge;
use crate::fetch_handle::FetchHandle;
use console::ConsoleOutput;
use timers::TimerQueueHandle;

/// Extract a required string argument from boa args.
///
/// Returns `TypeError` if the argument is missing, matching browser behavior
/// for required DOM method parameters.
pub(crate) fn require_js_string_arg(
    args: &[JsValue],
    index: usize,
    method: &str,
    ctx: &mut Context,
) -> JsResult<String> {
    match args.get(index) {
        Some(v) => Ok(v.to_string(ctx)?.to_std_string_escaped()),
        None => Err(JsNativeError::typ()
            .with_message(format!("{method}: argument {index} is required"))
            .into()),
    }
}

/// Register all elidex globals on the boa context.
pub fn register_all_globals(
    ctx: &mut Context,
    bridge: &HostBridge,
    console_output: &ConsoleOutput,
    timer_queue: &TimerQueueHandle,
    fetch_handle: Option<Rc<FetchHandle>>,
) {
    console::register_console(ctx, console_output);
    document::register_document(ctx, bridge);
    window::register_window(ctx, bridge);
    timers::register_timers(ctx, timer_queue);
    fetch::register_fetch(ctx, fetch_handle);
}
