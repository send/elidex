//! `window.history` object registration.
//!
//! Provides `back()`, `forward()`, `go(delta)`, `pushState()`, `replaceState()`,
//! and a `length` getter. Actions are queued as `HistoryAction` on the bridge
//! for the shell to process after eval completes.

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsResult, JsValue, NativeFunction};

use elidex_navigation::HistoryAction;

use crate::bridge::HostBridge;

/// Extract `(title, url?)` from pushState/replaceState args (state arg is ignored in Phase 2).
fn extract_state_args(args: &[JsValue], ctx: &mut Context) -> JsResult<(String, Option<String>)> {
    let title = args
        .get(1)
        .map(|v| v.to_string(ctx))
        .transpose()?
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();
    let url = args
        .get(2)
        .filter(|v| !v.is_undefined() && !v.is_null())
        .map(|v| v.to_string(ctx))
        .transpose()?
        .map(|s| s.to_std_string_escaped());
    Ok((title, url))
}

/// Shared implementation for pushState/replaceState.
///
/// `make_action` converts `(url, title)` into the appropriate `HistoryAction` variant.
fn push_or_replace_state(
    args: &[JsValue],
    bridge: &HostBridge,
    ctx: &mut Context,
    make_action: fn(Option<String>, String) -> HistoryAction,
) -> JsResult<JsValue> {
    let (title, url) = extract_state_args(args, ctx)?;
    bridge.set_pending_history(make_action(url, title));
    Ok(JsValue::undefined())
}

/// Register the `window.history` object.
pub fn register_history(ctx: &mut Context, bridge: &HostBridge) -> JsValue {
    // Clone the realm before creating ObjectInitializer to avoid borrow conflict.
    let realm = ctx.realm().clone();
    let mut init = ObjectInitializer::new(ctx);

    // state getter (always null in Phase 2 — state is not stored).
    let b = bridge.clone();
    init.accessor(
        js_string!("state"),
        Some(
            NativeFunction::from_copy_closure_with_captures(
                |_this, _args, _bridge, _ctx| -> JsResult<JsValue> { Ok(JsValue::null()) },
                b,
            )
            .to_js_function(&realm),
        ),
        None,
        Attribute::CONFIGURABLE,
    );

    // length getter
    let b = bridge.clone();
    init.accessor(
        js_string!("length"),
        Some(
            NativeFunction::from_copy_closure_with_captures(
                |_this, _args, bridge, _ctx| -> JsResult<JsValue> {
                    Ok(JsValue::from(bridge.history_length() as f64))
                },
                b,
            )
            .to_js_function(&realm),
        ),
        None,
        Attribute::CONFIGURABLE,
    );

    // back()
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, _args, bridge, _ctx| -> JsResult<JsValue> {
                bridge.set_pending_history(HistoryAction::Back);
                Ok(JsValue::undefined())
            },
            b,
        ),
        js_string!("back"),
        0,
    );

    // forward()
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, _args, bridge, _ctx| -> JsResult<JsValue> {
                bridge.set_pending_history(HistoryAction::Forward);
                Ok(JsValue::undefined())
            },
            b,
        ),
        js_string!("forward"),
        0,
    );

    // go(delta)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, bridge, ctx| -> JsResult<JsValue> {
                let delta = args
                    .first()
                    .map(|v| v.to_i32(ctx))
                    .transpose()?
                    .unwrap_or(0);
                bridge.set_pending_history(HistoryAction::Go(delta));
                Ok(JsValue::undefined())
            },
            b,
        ),
        js_string!("go"),
        1,
    );

    // pushState(state, title, url?)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, bridge, ctx| -> JsResult<JsValue> {
                push_or_replace_state(args, bridge, ctx, |url, title| {
                    HistoryAction::PushState { url, title }
                })
            },
            b,
        ),
        js_string!("pushState"),
        3,
    );

    // replaceState(state, title, url?)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, bridge, ctx| -> JsResult<JsValue> {
                push_or_replace_state(args, bridge, ctx, |url, title| {
                    HistoryAction::ReplaceState { url, title }
                })
            },
            b,
        ),
        js_string!("replaceState"),
        3,
    );

    init.build().into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use boa_engine::Source;

    fn setup() -> (Context, HostBridge) {
        let bridge = HostBridge::new();
        bridge.set_history_length(3);

        let mut ctx = Context::default();
        let history_obj = register_history(&mut ctx, &bridge);

        let global = ctx.global_object();
        global
            .set(js_string!("history"), history_obj, false, &mut ctx)
            .unwrap();

        (ctx, bridge)
    }

    #[test]
    fn history_length() {
        let (mut ctx, _) = setup();
        let result = ctx.eval(Source::from_bytes("history.length")).unwrap();
        assert_eq!(result.to_number(&mut ctx).unwrap(), 3.0);
    }

    #[test]
    fn history_back() {
        let (mut ctx, bridge) = setup();
        ctx.eval(Source::from_bytes("history.back()")).unwrap();
        let action = bridge.take_pending_history().unwrap();
        assert!(matches!(action, HistoryAction::Back));
    }

    #[test]
    fn history_forward() {
        let (mut ctx, bridge) = setup();
        ctx.eval(Source::from_bytes("history.forward()")).unwrap();
        let action = bridge.take_pending_history().unwrap();
        assert!(matches!(action, HistoryAction::Forward));
    }

    #[test]
    fn history_go() {
        let (mut ctx, bridge) = setup();
        ctx.eval(Source::from_bytes("history.go(-2)")).unwrap();
        let action = bridge.take_pending_history().unwrap();
        assert!(matches!(action, HistoryAction::Go(-2)));
    }

    #[test]
    fn history_push_state() {
        let (mut ctx, bridge) = setup();
        ctx.eval(Source::from_bytes(
            "history.pushState(null, '', '/new-path')",
        ))
        .unwrap();
        let action = bridge.take_pending_history().unwrap();
        match action {
            HistoryAction::PushState { url, .. } => {
                assert_eq!(url.as_deref(), Some("/new-path"));
            }
            _ => panic!("expected PushState"),
        }
    }

    #[test]
    fn history_replace_state() {
        let (mut ctx, bridge) = setup();
        ctx.eval(Source::from_bytes(
            "history.replaceState(null, '', '/replaced')",
        ))
        .unwrap();
        let action = bridge.take_pending_history().unwrap();
        match action {
            HistoryAction::ReplaceState { url, .. } => {
                assert_eq!(url.as_deref(), Some("/replaced"));
            }
            _ => panic!("expected ReplaceState"),
        }
    }

    #[test]
    fn history_push_state_no_url() {
        let (mut ctx, bridge) = setup();
        ctx.eval(Source::from_bytes("history.pushState(null, '')"))
            .unwrap();
        let action = bridge.take_pending_history().unwrap();
        match action {
            HistoryAction::PushState { url, .. } => {
                assert!(url.is_none());
            }
            _ => panic!("expected PushState"),
        }
    }

    #[test]
    fn history_state_is_null() {
        let (mut ctx, _) = setup();
        let result = ctx.eval(Source::from_bytes("history.state")).unwrap();
        assert!(result.is_null());
    }
}
