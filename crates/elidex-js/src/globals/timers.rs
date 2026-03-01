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
#[allow(unsafe_code)]
unsafe impl boa_gc::Trace for TimerQueueHandle {
    boa_gc::custom_trace!(this, mark, {
        let _ = this;
    });
}
impl boa_gc::Finalize for TimerQueueHandle {
    fn finalize(&self) {}
}

/// Register timer globals on the context.
#[allow(clippy::too_many_lines)]
pub fn register_timers(ctx: &mut Context, timers: &TimerQueueHandle) {
    // setTimeout(code, delay)
    let t = timers.clone();
    ctx.register_global_builtin_callable(
        js_string!("setTimeout"),
        2,
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, timers, ctx| -> JsResult<JsValue> {
                let code = args
                    .first()
                    .map(|v| v.to_string(ctx))
                    .transpose()?
                    .map(|s| s.to_std_string_escaped())
                    .unwrap_or_default();
                #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                let delay = args
                    .get(1)
                    .map(|v| v.to_number(ctx))
                    .transpose()?
                    .map_or(0, |n| n.max(0.0) as u64);
                let id = timers.borrow_mut().set_timeout(code, delay);
                Ok(JsValue::from(id.to_raw() as f64))
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
                let code = args
                    .first()
                    .map(|v| v.to_string(ctx))
                    .transpose()?
                    .map(|s| s.to_std_string_escaped())
                    .unwrap_or_default();
                #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                let interval = args
                    .get(1)
                    .map(|v| v.to_number(ctx))
                    .transpose()?
                    .map_or(0, |n| n.max(0.0) as u64);
                let id = timers.borrow_mut().set_interval(code, interval);
                Ok(JsValue::from(id.to_raw() as f64))
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
                #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                let id = args
                    .first()
                    .map(|v| v.to_number(ctx))
                    .transpose()?
                    .map_or(0, |n| n.max(0.0) as u64);
                timers.borrow_mut().clear_timer(TimerId::from_raw(id));
                Ok(JsValue::undefined())
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
                #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                let id = args
                    .first()
                    .map(|v| v.to_number(ctx))
                    .transpose()?
                    .map_or(0, |n| n.max(0.0) as u64);
                timers.borrow_mut().clear_timer(TimerId::from_raw(id));
                Ok(JsValue::undefined())
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
                let code = args
                    .first()
                    .map(|v| v.to_string(ctx))
                    .transpose()?
                    .map(|s| s.to_std_string_escaped())
                    .unwrap_or_default();
                let id = timers.borrow_mut().request_animation_frame(code);
                Ok(JsValue::from(id.to_raw() as f64))
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
                #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                let id = args
                    .first()
                    .map(|v| v.to_number(ctx))
                    .transpose()?
                    .map_or(0, |n| n.max(0.0) as u64);
                timers.borrow_mut().clear_timer(TimerId::from_raw(id));
                Ok(JsValue::undefined())
            },
            t,
        ),
    )
    .expect("failed to register cancelAnimationFrame");
}
