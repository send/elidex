//! `performance` object registration (W3C HR-Time + User Timing).

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsNativeError, JsResult, JsValue, NativeFunction};

use crate::bridge::HostBridge;

/// Wrapper around `Instant` to implement `boa_gc::Trace` (no GC objects inside).
#[derive(Clone, Copy)]
struct TracedInstant(std::time::Instant);

#[allow(unsafe_code)]
unsafe impl boa_gc::Trace for TracedInstant {
    boa_gc::custom_trace!(this, mark, {
        let _ = this;
    });
}
impl boa_gc::Finalize for TracedInstant {
    fn finalize(&self) {}
}

/// Register `performance` object (W3C HR-Time §4 + User Timing §3-4).
#[allow(clippy::too_many_lines)]
pub(crate) fn register_performance(ctx: &mut Context, _bridge: &HostBridge) {
    // Capture time origin at registration (approximates navigation start).
    let origin = TracedInstant(std::time::Instant::now());

    // Pre-build closures that capture origin before ObjectInitializer borrows ctx.
    let now_fn = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, origin, _ctx| {
            let elapsed_ms = origin.0.elapsed().as_secs_f64() * 1000.0;
            // Round to 100us for security (W3C HR-Time §4.4).
            let rounded = (elapsed_ms * 10.0).floor() / 10.0;
            Ok(JsValue::from(rounded))
        },
        origin,
    );

    // timeOrigin — Unix epoch milliseconds at navigation start.
    let time_origin = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0.0, |d| d.as_secs_f64() * 1000.0);

    // Performance entries stored in a shared JsArray.
    let entries = boa_engine::object::builtins::JsArray::new(ctx);
    let entries_obj = JsValue::from(entries);

    let mut init = ObjectInitializer::new(ctx);

    init.function(now_fn, js_string!("now"), 0);

    init.property(
        js_string!("timeOrigin"),
        JsValue::from(time_origin),
        Attribute::READONLY,
    );

    // Hidden entries storage (writable so clearMarks/clearMeasures can replace).
    init.property(
        js_string!("__entries__"),
        entries_obj,
        Attribute::WRITABLE | Attribute::CONFIGURABLE,
    );

    // performance.mark(name, options?) — W3C User Timing §3.
    let o2 = origin;
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, origin, ctx| {
                let name = crate::globals::require_js_string_arg(args, 0, "performance.mark", ctx)?;

                let start_time = args
                    .get(1)
                    .and_then(boa_engine::JsValue::as_object)
                    .and_then(|o| {
                        o.get(js_string!("startTime"), ctx)
                            .ok()
                            .and_then(|v| v.as_number())
                    })
                    .unwrap_or_else(|| {
                        let elapsed_ms = origin.0.elapsed().as_secs_f64() * 1000.0;
                        (elapsed_ms * 10.0).floor() / 10.0
                    });

                // Build PerformanceMark entry.
                let mut entry = ObjectInitializer::new(ctx);
                entry.property(
                    js_string!("entryType"),
                    JsValue::from(js_string!("mark")),
                    Attribute::READONLY | Attribute::CONFIGURABLE,
                );
                entry.property(
                    js_string!("name"),
                    JsValue::from(js_string!(name.as_str())),
                    Attribute::READONLY | Attribute::CONFIGURABLE,
                );
                entry.property(
                    js_string!("startTime"),
                    JsValue::from(start_time),
                    Attribute::READONLY | Attribute::CONFIGURABLE,
                );
                entry.property(
                    js_string!("duration"),
                    JsValue::from(0.0),
                    Attribute::READONLY | Attribute::CONFIGURABLE,
                );
                let mark_obj = entry.build();

                // Append to entries list.
                let perf = this.as_object().ok_or_else(|| {
                    JsNativeError::typ().with_message("performance: this is not an object")
                })?;
                let entries_val = perf.get(js_string!("__entries__"), ctx)?;
                if let Some(arr) = entries_val.as_object() {
                    let len = arr.get(js_string!("length"), ctx)?.to_number(ctx)? as u32;
                    arr.set(len, JsValue::from(mark_obj.clone()), false, ctx)?;
                }

                Ok(JsValue::from(mark_obj))
            },
            o2,
        ),
        js_string!("mark"),
        1,
    );

    // performance.measure(name, startOrOptions?, endMark?) — W3C User Timing §4.
    init.function(
        NativeFunction::from_copy_closure(|this, args, ctx| {
            let name = crate::globals::require_js_string_arg(args, 0, "performance.measure", ctx)?;

            let perf = this.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("performance: this is not an object")
            })?;
            let entries_val = perf.get(js_string!("__entries__"), ctx)?;
            let entries_arr = entries_val
                .as_object()
                .ok_or_else(|| JsNativeError::typ().with_message("performance: internal error"))?;

            // Read performance.now() once for all "use current time" branches.
            let current_now = perf_now(&perf, ctx);

            // Helper: find a mark by name.
            let find_mark = |mark_name: &str, ctx: &mut Context| -> JsResult<f64> {
                let len = entries_arr.get(js_string!("length"), ctx)?.to_number(ctx)? as u32;
                // Search from end (latest mark with this name wins).
                let mut i = len;
                while i > 0 {
                    i -= 1;
                    let e = entries_arr.get(i, ctx)?;
                    if let Some(e_obj) = e.as_object() {
                        let e_type = e_obj
                            .get(js_string!("entryType"), ctx)?
                            .to_string(ctx)?
                            .to_std_string_escaped();
                        let e_name = e_obj
                            .get(js_string!("name"), ctx)?
                            .to_string(ctx)?
                            .to_std_string_escaped();
                        if e_type == "mark" && e_name == mark_name {
                            return e_obj.get(js_string!("startTime"), ctx)?.to_number(ctx);
                        }
                    }
                }
                Err(JsNativeError::syntax()
                    .with_message(format!(
                        "SyntaxError: performance.measure: mark '{mark_name}' not found"
                    ))
                    .into())
            };

            // Resolve start and end times.
            let (start_time, end_time) = match args.get(1) {
                Some(v) if v.is_object() => {
                    // Options object form: { start, end, duration }.
                    let opts = v.as_object().unwrap();
                    let s = opts.get(js_string!("start"), ctx)?;
                    let e = opts.get(js_string!("end"), ctx)?;

                    let st = if let Some(n) = s.as_number() {
                        n
                    } else if !s.is_undefined() && !s.is_null() {
                        find_mark(&s.to_string(ctx)?.to_std_string_escaped(), ctx)?
                    } else {
                        0.0
                    };

                    let et = if let Some(n) = e.as_number() {
                        n
                    } else if !e.is_undefined() && !e.is_null() {
                        find_mark(&e.to_string(ctx)?.to_std_string_escaped(), ctx)?
                    } else {
                        current_now
                    };
                    (st, et)
                }
                Some(v) if !v.is_undefined() && !v.is_null() => {
                    // String form: startMark name.
                    let start_mark = v.to_string(ctx)?.to_std_string_escaped();
                    let st = find_mark(&start_mark, ctx)?;

                    let et = if let Some(end_v) = args.get(2) {
                        if !end_v.is_undefined() && !end_v.is_null() {
                            let end_mark = end_v.to_string(ctx)?.to_std_string_escaped();
                            find_mark(&end_mark, ctx)?
                        } else {
                            current_now
                        }
                    } else {
                        current_now
                    };
                    (st, et)
                }
                _ => {
                    // No start specified -> start from 0.
                    (0.0, current_now)
                }
            };

            let duration = end_time - start_time;

            let mut entry = ObjectInitializer::new(ctx);
            entry.property(
                js_string!("entryType"),
                JsValue::from(js_string!("measure")),
                Attribute::READONLY | Attribute::CONFIGURABLE,
            );
            entry.property(
                js_string!("name"),
                JsValue::from(js_string!(name.as_str())),
                Attribute::READONLY | Attribute::CONFIGURABLE,
            );
            entry.property(
                js_string!("startTime"),
                JsValue::from(start_time),
                Attribute::READONLY | Attribute::CONFIGURABLE,
            );
            entry.property(
                js_string!("duration"),
                JsValue::from(duration),
                Attribute::READONLY | Attribute::CONFIGURABLE,
            );
            let measure_obj = entry.build();

            // Append to entries list.
            let len = entries_arr.get(js_string!("length"), ctx)?.to_number(ctx)? as u32;
            entries_arr.set(len, JsValue::from(measure_obj.clone()), false, ctx)?;

            Ok(JsValue::from(measure_obj))
        }),
        js_string!("measure"),
        1,
    );

    // performance.getEntries() — W3C Performance Timeline §4.
    init.function(
        NativeFunction::from_copy_closure(|this, _args, ctx| {
            let perf = this.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("performance: this is not an object")
            })?;
            let entries_val = perf.get(js_string!("__entries__"), ctx)?;
            // Return a copy of the entries array.
            let result = boa_engine::object::builtins::JsArray::new(ctx);
            if let Some(arr) = entries_val.as_object() {
                let len = arr.get(js_string!("length"), ctx)?.to_number(ctx)? as u32;
                for i in 0..len {
                    result.push(arr.get(i, ctx)?, ctx)?;
                }
            }
            Ok(JsValue::from(result))
        }),
        js_string!("getEntries"),
        0,
    );

    // performance.getEntriesByType(type) — W3C Performance Timeline §4.
    init.function(
        NativeFunction::from_copy_closure(|this, args, ctx| {
            let perf = this.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("performance: this is not an object")
            })?;
            let entry_type =
                crate::globals::require_js_string_arg(args, 0, "getEntriesByType", ctx)?;
            let entries_val = perf.get(js_string!("__entries__"), ctx)?;
            let result = boa_engine::object::builtins::JsArray::new(ctx);
            if let Some(arr) = entries_val.as_object() {
                let len = arr.get(js_string!("length"), ctx)?.to_number(ctx)? as u32;
                for i in 0..len {
                    let e = arr.get(i, ctx)?;
                    if let Some(e_obj) = e.as_object() {
                        let t = e_obj
                            .get(js_string!("entryType"), ctx)?
                            .to_string(ctx)?
                            .to_std_string_escaped();
                        if t == entry_type {
                            result.push(e, ctx)?;
                        }
                    }
                }
            }
            Ok(JsValue::from(result))
        }),
        js_string!("getEntriesByType"),
        1,
    );

    // performance.getEntriesByName(name, type?) — W3C Performance Timeline §4.
    init.function(
        NativeFunction::from_copy_closure(|this, args, ctx| {
            let perf = this.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("performance: this is not an object")
            })?;
            let name = crate::globals::require_js_string_arg(args, 0, "getEntriesByName", ctx)?;
            let type_filter = args.get(1).and_then(|v| {
                if v.is_undefined() || v.is_null() {
                    None
                } else {
                    Some(v.to_string(ctx).ok()?.to_std_string_escaped())
                }
            });
            let entries_val = perf.get(js_string!("__entries__"), ctx)?;
            let result = boa_engine::object::builtins::JsArray::new(ctx);
            if let Some(arr) = entries_val.as_object() {
                let len = arr.get(js_string!("length"), ctx)?.to_number(ctx)? as u32;
                for i in 0..len {
                    let e = arr.get(i, ctx)?;
                    if let Some(e_obj) = e.as_object() {
                        let n = e_obj
                            .get(js_string!("name"), ctx)?
                            .to_string(ctx)?
                            .to_std_string_escaped();
                        if n != name {
                            continue;
                        }
                        if let Some(ref tf) = type_filter {
                            let t = e_obj
                                .get(js_string!("entryType"), ctx)?
                                .to_string(ctx)?
                                .to_std_string_escaped();
                            if &t != tf {
                                continue;
                            }
                        }
                        result.push(e, ctx)?;
                    }
                }
            }
            Ok(JsValue::from(result))
        }),
        js_string!("getEntriesByName"),
        1,
    );

    // performance.clearMarks(name?) — W3C User Timing §3.
    init.function(
        NativeFunction::from_copy_closure(|this, args, ctx| {
            clear_entries_by_type(this, args, "mark", ctx)
        }),
        js_string!("clearMarks"),
        0,
    );

    // performance.clearMeasures(name?) — W3C User Timing §4.
    init.function(
        NativeFunction::from_copy_closure(|this, args, ctx| {
            clear_entries_by_type(this, args, "measure", ctx)
        }),
        js_string!("clearMeasures"),
        0,
    );

    let perf = init.build();
    ctx.register_global_property(js_string!("performance"), perf, Attribute::all())
        .expect("failed to register performance");
}

/// Helper: clear performance entries by type, optionally filtered by name.
fn clear_entries_by_type(
    this: &JsValue,
    args: &[JsValue],
    entry_type: &str,
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let perf = this
        .as_object()
        .ok_or_else(|| JsNativeError::typ().with_message("performance: this is not an object"))?;
    let name_filter = args.first().and_then(|v| {
        if v.is_undefined() || v.is_null() {
            None
        } else {
            Some(v.to_string(ctx).ok()?.to_std_string_escaped())
        }
    });

    let entries_val = perf.get(js_string!("__entries__"), ctx)?;
    if let Some(arr) = entries_val.as_object() {
        let new_arr = boa_engine::object::builtins::JsArray::new(ctx);
        let len = arr.get(js_string!("length"), ctx)?.to_number(ctx)? as u32;
        for i in 0..len {
            let e = arr.get(i, ctx)?;
            let mut keep = true;
            if let Some(e_obj) = e.as_object() {
                let t = e_obj
                    .get(js_string!("entryType"), ctx)?
                    .to_string(ctx)?
                    .to_std_string_escaped();
                if t == entry_type {
                    if let Some(ref nf) = name_filter {
                        let n = e_obj
                            .get(js_string!("name"), ctx)?
                            .to_string(ctx)?
                            .to_std_string_escaped();
                        if &n == nf {
                            keep = false;
                        }
                    } else {
                        keep = false;
                    }
                }
            }
            if keep {
                new_arr.push(e, ctx)?;
            }
        }
        perf.set(
            js_string!("__entries__"),
            JsValue::from(new_arr),
            false,
            ctx,
        )?;
    }
    Ok(JsValue::undefined())
}

/// Call `performance.now()` on a performance object, returning 0.0 on failure.
fn perf_now(perf: &boa_engine::JsObject, ctx: &mut Context) -> f64 {
    let Ok(now_val) = perf.get(js_string!("now"), ctx) else {
        return 0.0;
    };
    let Some(now_fn) = now_val.as_callable() else {
        return 0.0;
    };
    now_fn
        .call(&JsValue::from(perf.clone()), &[], ctx)
        .ok()
        .and_then(|v| v.as_number())
        .unwrap_or(0.0)
}
