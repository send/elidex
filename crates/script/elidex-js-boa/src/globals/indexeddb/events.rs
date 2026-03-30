//! `IDBVersionChangeEvent` JS object builder.

use boa_engine::object::ObjectInitializer;
use boa_engine::{js_string, Context, JsObject, JsValue};

/// Build an `IDBVersionChangeEvent` object.
///
/// `event_type`: `"upgradeneeded"` or `"versionchange"`.
/// `new_version`: `None` for `deleteDatabase` (maps to JS `null`).
#[allow(clippy::cast_precision_loss)]
pub fn build_version_change_event(
    event_type: &str,
    old_version: u64,
    new_version: Option<u64>,
    target: &JsObject,
    ctx: &mut Context,
) -> JsObject {
    let new_ver_val = new_version.map_or(JsValue::null(), |v| JsValue::from(v as f64));

    ObjectInitializer::new(ctx)
        .property(
            js_string!("type"),
            js_string!(event_type),
            boa_engine::property::Attribute::all(),
        )
        .property(
            js_string!("oldVersion"),
            JsValue::from(old_version as f64),
            boa_engine::property::Attribute::all(),
        )
        .property(
            js_string!("newVersion"),
            new_ver_val,
            boa_engine::property::Attribute::all(),
        )
        .property(
            js_string!("target"),
            JsValue::from(target.clone()),
            boa_engine::property::Attribute::all(),
        )
        .build()
}
