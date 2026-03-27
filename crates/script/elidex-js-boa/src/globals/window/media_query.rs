//! `matchMedia()` registration and `MediaQueryList` object construction.

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsResult, JsValue, NativeFunction};

use crate::bridge::HostBridge;

/// Register `matchMedia(query)` global function.
pub(super) fn register_media_query(ctx: &mut Context, bridge: &HostBridge) {
    let b_mm = bridge.clone();
    let match_media = NativeFunction::from_copy_closure_with_captures(
        |_this, args, bridge, ctx| {
            let query = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map_or(String::new(), |s| s.to_std_string_escaped());

            let matches = evaluate_media_query(&query, bridge);
            let mq_id = bridge.create_media_query(&query, matches);

            build_media_query_list_object(mq_id, &query, bridge, ctx)
        },
        b_mm,
    );
    ctx.register_global_builtin_callable(js_string!("matchMedia"), 1, match_media)
        .expect("failed to register matchMedia");
}

/// Evaluate a basic media query string against the current viewport.
///
/// Supports:
/// - `(max-width: Npx)` / `(min-width: Npx)`
/// - `(max-height: Npx)` / `(min-height: Npx)`
/// - `(prefers-color-scheme: dark|light)` → false (no theme support yet)
/// - Other queries → false
fn evaluate_media_query(query: &str, bridge: &HostBridge) -> bool {
    crate::bridge::evaluate_media_query_raw(
        query,
        bridge.viewport_width(),
        bridge.viewport_height(),
    )
}

/// Hidden property key for the media query list ID.
const MQ_ID_KEY: &str = "__elidex_mq_id__";

/// Build a `MediaQueryList`-like JS object with dynamic `matches` getter
/// and `addEventListener`/`removeEventListener` for "change" events.
#[allow(clippy::too_many_lines, clippy::unnecessary_wraps)]
fn build_media_query_list_object(
    mq_id: u64,
    query: &str,
    bridge: &HostBridge,
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let mut obj = ObjectInitializer::new(ctx);

    // Store the media query ID as a hidden property.
    #[allow(clippy::cast_precision_loss)]
    obj.property(
        js_string!(MQ_ID_KEY),
        JsValue::from(mq_id as f64),
        Attribute::empty(),
    );

    obj.property(
        js_string!("media"),
        JsValue::from(js_string!(query)),
        Attribute::READONLY,
    );

    // matches — dynamic getter that re-evaluates against current viewport.
    let realm = obj.context().realm().clone();
    let b_matches = bridge.clone();
    let matches_getter = NativeFunction::from_copy_closure_with_captures(
        |this, _args, bridge, ctx| {
            let id = extract_mq_id(this, ctx)?;
            Ok(JsValue::from(bridge.media_query_matches(id)))
        },
        b_matches,
    )
    .to_js_function(&realm);
    obj.accessor(
        js_string!("matches"),
        Some(matches_getter),
        None,
        Attribute::CONFIGURABLE,
    );

    // addEventListener(type, callback)
    let b_add = bridge.clone();
    obj.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let event_type = args
                    .first()
                    .map(|v| v.to_string(ctx))
                    .transpose()?
                    .map_or(String::new(), |s| s.to_std_string_escaped());
                if event_type == "change" {
                    if let Some(callback) = args.get(1).and_then(JsValue::as_object) {
                        let id = extract_mq_id(this, ctx)?;
                        bridge.add_media_query_listener(id, callback.clone());
                    }
                }
                Ok(JsValue::undefined())
            },
            b_add,
        ),
        js_string!("addEventListener"),
        2,
    );

    // removeEventListener(type, callback)
    let b_rm = bridge.clone();
    obj.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let event_type = args
                    .first()
                    .map(|v| v.to_string(ctx))
                    .transpose()?
                    .map_or(String::new(), |s| s.to_std_string_escaped());
                if event_type == "change" {
                    if let Some(callback) = args.get(1).and_then(JsValue::as_object) {
                        let id = extract_mq_id(this, ctx)?;
                        bridge.remove_media_query_listener(id, &callback);
                    }
                }
                Ok(JsValue::undefined())
            },
            b_rm,
        ),
        js_string!("removeEventListener"),
        2,
    );

    // Legacy aliases: addListener / removeListener (CSSOM View spec §4.2)
    let b_add_legacy = bridge.clone();
    obj.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                if let Some(callback) = args.first().and_then(JsValue::as_object) {
                    let id = extract_mq_id(this, ctx)?;
                    bridge.add_media_query_listener(id, callback.clone());
                }
                Ok(JsValue::undefined())
            },
            b_add_legacy,
        ),
        js_string!("addListener"),
        1,
    );

    let b_rm_legacy = bridge.clone();
    obj.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                if let Some(callback) = args.first().and_then(JsValue::as_object) {
                    let id = extract_mq_id(this, ctx)?;
                    bridge.remove_media_query_listener(id, &callback);
                }
                Ok(JsValue::undefined())
            },
            b_rm_legacy,
        ),
        js_string!("removeListener"),
        1,
    );

    Ok(obj.build().into())
}

/// Extract the media query ID from a JS object's hidden property.
fn extract_mq_id(this: &JsValue, ctx: &mut Context) -> JsResult<u64> {
    let obj = this.as_object().ok_or_else(|| {
        boa_engine::JsNativeError::typ().with_message("matchMedia method called on non-object")
    })?;
    let id_val = obj.get(js_string!(MQ_ID_KEY), ctx)?;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let id = id_val.as_number().ok_or_else(|| {
        boa_engine::JsNativeError::typ().with_message("invalid MediaQueryList object")
    })? as u64;
    Ok(id)
}
