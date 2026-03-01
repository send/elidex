//! Timer globals: setTimeout, setInterval, clearTimeout, clearInterval,
//! requestAnimationFrame, cancelAnimationFrame.
//!
//! # Phase 2 limitation
//!
//! Timer callbacks are captured as strings via JS `toString()`, not as
//! function closures. Only string-form callbacks work correctly:
//! `setTimeout("code()", delay)`. Function callbacks like
//! `setTimeout(() => { ... }, delay)` will be stringified and won't
//! execute as expected. A future phase should store `JsFunction` values.

use std::cell::RefCell;
use std::rc::Rc;

use boa_engine::{js_string, Context, JsResult, JsValue, NativeFunction};

use crate::timer_queue::{TimerId, TimerQueue};

/// Shared timer queue handle.
#[derive(Clone)]
pub struct TimerQueueHandle {
    inner: Rc<RefCell<TimerQueue>>,
}

impl TimerQueueHandle {
    pub fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new(TimerQueue::new())),
        }
    }

    pub fn borrow(&self) -> std::cell::Ref<'_, TimerQueue> {
        self.inner.borrow()
    }

    pub fn borrow_mut(&self) -> std::cell::RefMut<'_, TimerQueue> {
        self.inner.borrow_mut()
    }
}

impl Default for TimerQueueHandle {
    fn default() -> Self {
        Self::new()
    }
}

// Trace/Finalize for boa GC compatibility.
impl_empty_trace!(TimerQueueHandle);

/// Stringify the first argument as a callback source string.
fn stringify_callback(args: &[JsValue], ctx: &mut Context) -> JsResult<String> {
    args.first()
        .map(|v| v.to_string(ctx))
        .transpose()
        .map(|opt| opt.map_or_else(String::new, |s| s.to_std_string_escaped()))
}

/// Extract a timer ID (u64) from the first argument.
#[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
fn extract_timer_id(args: &[JsValue], ctx: &mut Context) -> JsResult<u64> {
    args.first()
        .map(|v| v.to_number(ctx))
        .transpose()
        .map(|opt| opt.map_or(0, |n| n.max(0.0) as u64))
}

/// Extract a delay/interval (u64 ms) from the second argument.
#[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
fn extract_delay(args: &[JsValue], ctx: &mut Context) -> JsResult<u64> {
    args.get(1)
        .map(|v| v.to_number(ctx))
        .transpose()
        .map(|opt| opt.map_or(0, |n| n.max(0.0) as u64))
}

/// Return a timer ID as a JS f64 value.
fn timer_id_to_js(id: TimerId) -> JsValue {
    JsValue::from(id.to_raw() as f64)
}

/// Closure body for `clearTimeout`, `clearInterval`, `cancelAnimationFrame`.
fn clear_timer_impl(
    args: &[JsValue],
    timers: &TimerQueueHandle,
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let id = extract_timer_id(args, ctx)?;
    timers.borrow_mut().clear_timer(TimerId::from_raw(id));
    Ok(JsValue::undefined())
}

/// Register timer globals on the context.
pub fn register_timers(ctx: &mut Context, timers: &TimerQueueHandle) {
    // setTimeout(code, delay)
    let t = timers.clone();
    ctx.register_global_builtin_callable(
        js_string!("setTimeout"),
        2,
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, timers, ctx| -> JsResult<JsValue> {
                let code = stringify_callback(args, ctx)?;
                let delay = extract_delay(args, ctx)?;
                let id = timers.borrow_mut().set_timeout(code, delay);
                Ok(timer_id_to_js(id))
            },
            t,
        ),
    )
    .expect("failed to register setTimeout");

    // setInterval(code, interval)
    let t = timers.clone();
    ctx.register_global_builtin_callable(
        js_string!("setInterval"),
        2,
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, timers, ctx| -> JsResult<JsValue> {
                let code = stringify_callback(args, ctx)?;
                let interval = extract_delay(args, ctx)?;
                let id = timers.borrow_mut().set_interval(code, interval);
                Ok(timer_id_to_js(id))
            },
            t,
        ),
    )
    .expect("failed to register setInterval");

    // clearTimeout(id)
    let t = timers.clone();
    ctx.register_global_builtin_callable(
        js_string!("clearTimeout"),
        1,
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, timers, ctx| -> JsResult<JsValue> {
                clear_timer_impl(args, timers, ctx)
            },
            t,
        ),
    )
    .expect("failed to register clearTimeout");

    // clearInterval(id)
    let t = timers.clone();
    ctx.register_global_builtin_callable(
        js_string!("clearInterval"),
        1,
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, timers, ctx| -> JsResult<JsValue> {
                clear_timer_impl(args, timers, ctx)
            },
            t,
        ),
    )
    .expect("failed to register clearInterval");

    // requestAnimationFrame(callback)
    let t = timers.clone();
    ctx.register_global_builtin_callable(
        js_string!("requestAnimationFrame"),
        1,
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, timers, ctx| -> JsResult<JsValue> {
                let code = stringify_callback(args, ctx)?;
                let id = timers.borrow_mut().request_animation_frame(code);
                Ok(timer_id_to_js(id))
            },
            t,
        ),
    )
    .expect("failed to register requestAnimationFrame");

    // cancelAnimationFrame(id)
    let t = timers.clone();
    ctx.register_global_builtin_callable(
        js_string!("cancelAnimationFrame"),
        1,
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, timers, ctx| -> JsResult<JsValue> {
                clear_timer_impl(args, timers, ctx)
            },
            t,
        ),
    )
    .expect("failed to register cancelAnimationFrame");
}
