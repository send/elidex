//! `console.log()`, `console.error()`, `console.warn()` global registration.

use std::cell::RefCell;
use std::rc::Rc;

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsResult, JsValue, NativeFunction};

/// Captured console output (for testing).
#[derive(Clone, Default)]
pub struct ConsoleOutput {
    inner: Rc<RefCell<Vec<(String, String)>>>,
}

impl ConsoleOutput {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get all captured `(level, message)` pairs.
    pub fn messages(&self) -> Vec<(String, String)> {
        self.inner.borrow().clone()
    }

    fn push(&self, level: &str, message: String) {
        self.inner.borrow_mut().push((level.into(), message));
    }
}

// Safety: ConsoleOutput only contains Rc (not GC-managed), so empty trace is correct.
impl_empty_trace!(ConsoleOutput);

/// Register the `console` global object.
pub fn register_console(ctx: &mut Context, output: &ConsoleOutput) {
    let log_output = output.clone();
    let error_output = output.clone();
    let warn_output = output.clone();

    let console = ObjectInitializer::new(ctx)
        .function(
            NativeFunction::from_copy_closure_with_captures(
                |_this, args, out, ctx| console_impl("log", args, out, ctx),
                log_output,
            ),
            js_string!("log"),
            0,
        )
        .function(
            NativeFunction::from_copy_closure_with_captures(
                |_this, args, out, ctx| console_impl("error", args, out, ctx),
                error_output,
            ),
            js_string!("error"),
            0,
        )
        .function(
            NativeFunction::from_copy_closure_with_captures(
                |_this, args, out, ctx| console_impl("warn", args, out, ctx),
                warn_output,
            ),
            js_string!("warn"),
            0,
        )
        .build();

    ctx.register_global_property(js_string!("console"), console, Attribute::all())
        .expect("failed to register console");
}

#[allow(clippy::unnecessary_wraps)] // Must return JsResult for boa NativeFunction closure registration.
fn console_impl(
    level: &str,
    args: &[JsValue],
    output: &ConsoleOutput,
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let parts: Vec<String> = args
        .iter()
        .map(|arg| {
            arg.to_string(ctx)
                .map_or_else(|_| "[error]".into(), |s| s.to_std_string_escaped())
        })
        .collect();
    let message = parts.join(" ");

    // Print to stderr (like browsers print to devtools console).
    eprintln!("[console.{level}] {message}");

    // Capture for testing.
    output.push(level, message);

    Ok(JsValue::undefined())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn console_log_captures_output() {
        let mut ctx = Context::default();
        let output = ConsoleOutput::new();
        register_console(&mut ctx, &output);

        ctx.eval(boa_engine::Source::from_bytes("console.log('hello', 42)"))
            .unwrap();

        let msgs = output.messages();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].0, "log");
        assert_eq!(msgs[0].1, "hello 42");
    }

    #[test]
    fn console_error_and_warn() {
        let mut ctx = Context::default();
        let output = ConsoleOutput::new();
        register_console(&mut ctx, &output);

        ctx.eval(boa_engine::Source::from_bytes(
            "console.error('err'); console.warn('wrn')",
        ))
        .unwrap();

        let msgs = output.messages();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].0, "error");
        assert_eq!(msgs[1].0, "warn");
    }
}
