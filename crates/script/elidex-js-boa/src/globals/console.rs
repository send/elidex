//! `console.log()`, `console.error()`, `console.warn()` global registration.

use std::cell::RefCell;
use std::rc::Rc;

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsResult, JsValue, NativeFunction};

/// Captured console output (for testing) + timer/counter state.
#[derive(Clone, Default)]
pub struct ConsoleOutput {
    pub(crate) inner: Rc<RefCell<Vec<(String, String)>>>,
    timers: Rc<RefCell<std::collections::HashMap<String, std::time::Instant>>>,
    counters: Rc<RefCell<std::collections::HashMap<String, usize>>>,
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
#[allow(clippy::too_many_lines)]
pub fn register_console(ctx: &mut Context, output: &ConsoleOutput) {
    // ConsoleOutput now carries timers/counters for shared state.

    macro_rules! reg {
        ($init:expr, $name:expr, $out:expr) => {{
            let o = $out.clone();
            $init.function(
                NativeFunction::from_copy_closure_with_captures(
                    |_this, args, out, ctx| console_impl($name, args, out, ctx),
                    o,
                ),
                js_string!($name),
                0,
            )
        }};
    }

    let mut init = ObjectInitializer::new(ctx);
    reg!(init, "log", output);
    reg!(init, "error", output);
    reg!(init, "warn", output);
    reg!(init, "info", output);
    reg!(init, "debug", output);

    // trace(...args) — log + "Trace:" prefix.
    let o = output.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, out, ctx| {
                let parts: Vec<String> = args
                    .iter()
                    .map(|a| {
                        a.to_string(ctx)
                            .map_or("[error]".into(), |s| s.to_std_string_escaped())
                    })
                    .collect();
                let msg = format!("Trace: {}", parts.join(" "));
                eprintln!("[console.trace] {msg}");
                out.push("trace", msg);
                Ok(JsValue::undefined())
            },
            o,
        ),
        js_string!("trace"),
        0,
    );

    // assert(condition, ...args)
    let o = output.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, out, ctx| {
                let condition = args.first().is_some_and(JsValue::to_boolean);
                if !condition {
                    let parts: Vec<String> = args
                        .iter()
                        .skip(1)
                        .map(|a| {
                            a.to_string(ctx)
                                .map_or("[error]".into(), |s| s.to_std_string_escaped())
                        })
                        .collect();
                    let msg = if parts.is_empty() {
                        "Assertion failed".to_string()
                    } else {
                        format!("Assertion failed: {}", parts.join(" "))
                    };
                    eprintln!("[console.assert] {msg}");
                    out.push("assert", msg);
                }
                Ok(JsValue::undefined())
            },
            o,
        ),
        js_string!("assert"),
        1,
    );

    // dir(obj) — log object properties.
    let o = output.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, out, ctx| console_impl("dir", args, out, ctx),
            o,
        ),
        js_string!("dir"),
        1,
    );

    // table(data) — log as table format.
    let o = output.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, out, ctx| console_impl("table", args, out, ctx),
            o,
        ),
        js_string!("table"),
        1,
    );

    // time(label), timeEnd(label), timeLog(label)
    let o = output.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, out, ctx| {
                let label = args
                    .first()
                    .map(|v| v.to_string(ctx))
                    .transpose()?
                    .map_or("default".into(), |s| s.to_std_string_escaped());
                if out.timers.borrow().contains_key(&label) {
                    let msg = format!("Timer '{label}' already exists");
                    eprintln!("[console.time] {msg}");
                    out.push("warn", msg);
                } else {
                    out.timers
                        .borrow_mut()
                        .insert(label, std::time::Instant::now());
                }
                Ok(JsValue::undefined())
            },
            o,
        ),
        js_string!("time"),
        0,
    );

    let o = output.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, out, ctx| {
                let label = args
                    .first()
                    .map(|v| v.to_string(ctx))
                    .transpose()?
                    .map_or("default".into(), |s| s.to_std_string_escaped());
                if let Some(start) = out.timers.borrow_mut().remove(&label) {
                    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                    let msg = format!("{label}: {elapsed:.3}ms");
                    eprintln!("[console.timeEnd] {msg}");
                    out.push("timeEnd", msg);
                } else {
                    let msg = format!("Timer '{label}' does not exist");
                    eprintln!("[console.timeEnd] {msg}");
                    out.push("warn", msg);
                }
                Ok(JsValue::undefined())
            },
            o,
        ),
        js_string!("timeEnd"),
        0,
    );

    let o = output.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, out, ctx| {
                let label = args
                    .first()
                    .map(|v| v.to_string(ctx))
                    .transpose()?
                    .map_or("default".into(), |s| s.to_std_string_escaped());
                if let Some(start) = out.timers.borrow().get(&label) {
                    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                    let msg = format!("{label}: {elapsed:.3}ms");
                    eprintln!("[console.timeLog] {msg}");
                    out.push("timeLog", msg);
                } else {
                    let msg = format!("Timer '{label}' does not exist");
                    eprintln!("[console.timeLog] {msg}");
                    out.push("warn", msg);
                }
                Ok(JsValue::undefined())
            },
            o,
        ),
        js_string!("timeLog"),
        0,
    );

    // count(label), countReset(label)
    let o = output.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, out, ctx| {
                let label = args
                    .first()
                    .map(|v| v.to_string(ctx))
                    .transpose()?
                    .map_or("default".into(), |s| s.to_std_string_escaped());
                let count = {
                    let mut map = out.counters.borrow_mut();
                    let entry = map.entry(label.clone()).or_insert(0);
                    *entry += 1;
                    *entry
                };
                let msg = format!("{label}: {count}");
                eprintln!("[console.count] {msg}");
                out.push("count", msg);
                Ok(JsValue::undefined())
            },
            o,
        ),
        js_string!("count"),
        0,
    );

    let o = output.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, out, ctx| {
                let label = args
                    .first()
                    .map(|v| v.to_string(ctx))
                    .transpose()?
                    .map_or("default".into(), |s| s.to_std_string_escaped());
                out.counters.borrow_mut().remove(&label);
                Ok(JsValue::undefined())
            },
            o,
        ),
        js_string!("countReset"),
        0,
    );

    // group(label), groupEnd() — no-op for now (indent level not used in output).
    init.function(
        NativeFunction::from_fn_ptr(|_this, _args, _ctx| Ok(JsValue::undefined())),
        js_string!("group"),
        0,
    );
    init.function(
        NativeFunction::from_fn_ptr(|_this, _args, _ctx| Ok(JsValue::undefined())),
        js_string!("groupEnd"),
        0,
    );

    // clear()
    let o = output.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, _args, out, _ctx| {
                out.inner.borrow_mut().clear();
                Ok(JsValue::undefined())
            },
            o,
        ),
        js_string!("clear"),
        0,
    );

    let console = init.build();

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
