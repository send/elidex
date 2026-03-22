//! Observer API JS globals: `MutationObserver`, `ResizeObserver`, `IntersectionObserver`.

use boa_engine::object::builtins::JsArray;
use boa_engine::object::{FunctionObjectBuilder, ObjectInitializer};
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsNativeError, JsResult, JsValue, NativeFunction};
use elidex_api_observers::intersection::IntersectionObserverInit;
use elidex_api_observers::mutation::MutationObserverInit;
use elidex_api_observers::resize::{ResizeObserverBoxOptions, ResizeObserverOptions};

use crate::bridge::HostBridge;

use super::element::extract_entity;

/// Hidden property key for storing the observer ID on observer JS objects.
const OBSERVER_ID_KEY: &str = "__elidex_observer_id__";

/// Register `MutationObserver`, `ResizeObserver`, and `IntersectionObserver` on the global object.
pub fn register_observers(ctx: &mut Context, bridge: &HostBridge) {
    register_mutation_observer(ctx, bridge);
    register_resize_observer(ctx, bridge);
    register_intersection_observer(ctx, bridge);
}

/// Extract and validate the target entity from the first argument.
fn require_target(
    args: &[JsValue],
    method: &str,
    ctx: &mut Context,
) -> JsResult<elidex_ecs::Entity> {
    let arg = args.first().ok_or_else(|| {
        JsNativeError::typ().with_message(format!("{method}: target is required"))
    })?;
    extract_entity(arg, ctx)
}

// --- MutationObserver ---

fn register_mutation_observer(ctx: &mut Context, bridge: &HostBridge) {
    let bridge_c = bridge.clone();
    let constructor = NativeFunction::from_copy_closure_with_captures(
        |_this, args, captures: &(HostBridge,), ctx| {
            let bridge = &captures.0;
            let callback = args.first().and_then(JsValue::as_callable).ok_or_else(|| {
                JsNativeError::typ().with_message("MutationObserver: argument 1 must be a function")
            })?;

            let observer_id = bridge.with_mutation_observers(|reg| reg.register().raw());

            let observe_bridge = bridge.clone();
            let disconnect_bridge = bridge.clone();
            let take_records_bridge = bridge.clone();

            let observer_obj = ObjectInitializer::new(ctx)
                .property(
                    js_string!(OBSERVER_ID_KEY),
                    JsValue::from(observer_id as f64),
                    Attribute::empty(),
                )
                .function(
                    NativeFunction::from_copy_closure_with_captures(
                        mutation_observe,
                        (observe_bridge,),
                    ),
                    js_string!("observe"),
                    2,
                )
                .function(
                    NativeFunction::from_copy_closure_with_captures(
                        observer_disconnect::<MutationDisconnect>,
                        (disconnect_bridge,),
                    ),
                    js_string!("disconnect"),
                    0,
                )
                .function(
                    NativeFunction::from_copy_closure_with_captures(
                        mutation_take_records,
                        (take_records_bridge,),
                    ),
                    js_string!("takeRecords"),
                    0,
                )
                .build();

            bridge.store_observer_callback(observer_id, callback.clone(), observer_obj.clone());

            Ok(JsValue::from(observer_obj))
        },
        (bridge_c,),
    );

    let realm = ctx.realm().clone();
    let js_fn = FunctionObjectBuilder::new(&realm, constructor)
        .name(js_string!("MutationObserver"))
        .length(1)
        .constructor(true)
        .build();
    let global = ctx.global_object();
    global
        .set(js_string!("MutationObserver"), js_fn, false, ctx)
        .expect("failed to register MutationObserver");
}

fn mutation_observe(
    this: &JsValue,
    args: &[JsValue],
    captures: &(HostBridge,),
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let bridge = &captures.0;
    let observer_id = extract_observer_id(this, ctx)?;
    let target = require_target(args, "MutationObserver.observe", ctx)?;

    let mut init = MutationObserverInit::default();
    if let Some(opts) = args.get(1).and_then(JsValue::as_object) {
        if let Ok(v) = opts.get(js_string!("childList"), ctx) {
            init.child_list = v.to_boolean();
        }
        if let Ok(v) = opts.get(js_string!("attributes"), ctx) {
            init.attributes = v.to_boolean();
        }
        if let Ok(v) = opts.get(js_string!("characterData"), ctx) {
            init.character_data = v.to_boolean();
        }
        if let Ok(v) = opts.get(js_string!("subtree"), ctx) {
            init.subtree = v.to_boolean();
        }
        if let Ok(v) = opts.get(js_string!("attributeOldValue"), ctx) {
            init.attribute_old_value = v.to_boolean();
        }
        if let Ok(v) = opts.get(js_string!("characterDataOldValue"), ctx) {
            init.character_data_old_value = v.to_boolean();
        }
        if let Ok(v) = opts.get(js_string!("attributeFilter"), ctx) {
            if let Some(arr_obj) = v.as_object() {
                if let Ok(len) = arr_obj.get(js_string!("length"), ctx) {
                    let len = len.to_u32(ctx).unwrap_or(0);
                    let mut filter = Vec::new();
                    for i in 0..len {
                        if let Ok(item) = arr_obj.get(i, ctx) {
                            filter.push(item.to_string(ctx)?.to_std_string_escaped());
                        }
                    }
                    init.attribute_filter = Some(filter);
                }
            }
        }
        // DOM spec: if attributes not set but attributeOldValue or attributeFilter present,
        // set attributes to true.
        if !init.attributes && (init.attribute_old_value || init.attribute_filter.is_some()) {
            init.attributes = true;
        }
        // DOM spec: if characterData not set but characterDataOldValue present,
        // set characterData to true.
        if !init.character_data && init.character_data_old_value {
            init.character_data = true;
        }
    }

    // DOM spec: at least one of childList, attributes, characterData must be true.
    if !init.child_list && !init.attributes && !init.character_data {
        return Err(JsNativeError::typ()
            .with_message("MutationObserver.observe: at least one of childList, attributes, or characterData must be true")
            .into());
    }

    let mo_id = elidex_api_observers::mutation::MutationObserverId::from_raw(observer_id);
    bridge.with_mutation_observers(|reg| reg.observe(mo_id, target, init));
    Ok(JsValue::undefined())
}

fn mutation_take_records(
    this: &JsValue,
    _args: &[JsValue],
    captures: &(HostBridge,),
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let bridge = &captures.0;
    let observer_id = extract_observer_id(this, ctx)?;
    let mo_id = elidex_api_observers::mutation::MutationObserverId::from_raw(observer_id);
    let records = bridge.with_mutation_observers(|reg| reg.take_records(mo_id));
    let arr = JsArray::new(ctx);
    for record in records {
        let obj = mutation_record_to_js(&record, ctx);
        let _ = arr.push(obj, ctx);
    }
    Ok(JsValue::from(arr))
}

/// Convert a `MutationRecord` to a JS object.
pub(crate) fn mutation_record_to_js(
    record: &elidex_api_observers::mutation::MutationRecord,
    ctx: &mut Context,
) -> JsValue {
    // Build arrays first to avoid double-borrowing ctx through ObjectInitializer.
    let added = JsArray::new(ctx);
    for &e in &record.added_nodes {
        let _ = added.push(JsValue::from(e.to_bits().get() as f64), ctx);
    }
    let removed = JsArray::new(ctx);
    for &e in &record.removed_nodes {
        let _ = removed.push(JsValue::from(e.to_bits().get() as f64), ctx);
    }
    let added_val = JsValue::from(added);
    let removed_val = JsValue::from(removed);
    let attr_name_val = record
        .attribute_name
        .as_ref()
        .map_or(JsValue::null(), |n| JsValue::from(js_string!(n.as_str())));
    let old_value_val = record
        .old_value
        .as_ref()
        .map_or(JsValue::null(), |v| JsValue::from(js_string!(v.as_str())));

    let obj = ObjectInitializer::new(ctx)
        .property(
            js_string!("type"),
            JsValue::from(js_string!(record.mutation_type.as_str())),
            Attribute::all(),
        )
        .property(
            js_string!("target"),
            JsValue::from(record.target.to_bits().get() as f64),
            Attribute::all(),
        )
        .property(js_string!("addedNodes"), added_val, Attribute::all())
        .property(js_string!("removedNodes"), removed_val, Attribute::all())
        .property(js_string!("attributeName"), attr_name_val, Attribute::all())
        .property(js_string!("oldValue"), old_value_val, Attribute::all())
        .build();
    JsValue::from(obj)
}

// --- ResizeObserver ---

fn register_resize_observer(ctx: &mut Context, bridge: &HostBridge) {
    let bridge_c = bridge.clone();
    let constructor = NativeFunction::from_copy_closure_with_captures(
        |_this, args, captures: &(HostBridge,), ctx| {
            let bridge = &captures.0;
            let callback = args.first().and_then(JsValue::as_callable).ok_or_else(|| {
                JsNativeError::typ().with_message("ResizeObserver: argument 1 must be a function")
            })?;

            let observer_id = bridge.with_resize_observers(|reg| reg.register().raw());

            let observe_bridge = bridge.clone();
            let unobserve_bridge = bridge.clone();
            let disconnect_bridge = bridge.clone();

            let observer_obj = ObjectInitializer::new(ctx)
                .property(
                    js_string!(OBSERVER_ID_KEY),
                    JsValue::from(observer_id as f64),
                    Attribute::empty(),
                )
                .function(
                    NativeFunction::from_copy_closure_with_captures(
                        resize_observe,
                        (observe_bridge,),
                    ),
                    js_string!("observe"),
                    2,
                )
                .function(
                    NativeFunction::from_copy_closure_with_captures(
                        resize_unobserve,
                        (unobserve_bridge,),
                    ),
                    js_string!("unobserve"),
                    1,
                )
                .function(
                    NativeFunction::from_copy_closure_with_captures(
                        observer_disconnect::<ResizeDisconnect>,
                        (disconnect_bridge,),
                    ),
                    js_string!("disconnect"),
                    0,
                )
                .build();

            bridge.store_observer_callback(observer_id, callback.clone(), observer_obj.clone());

            Ok(JsValue::from(observer_obj))
        },
        (bridge_c,),
    );

    let realm = ctx.realm().clone();
    let js_fn = FunctionObjectBuilder::new(&realm, constructor)
        .name(js_string!("ResizeObserver"))
        .length(1)
        .constructor(true)
        .build();
    let global = ctx.global_object();
    global
        .set(js_string!("ResizeObserver"), js_fn, false, ctx)
        .expect("failed to register ResizeObserver");
}

fn resize_observe(
    this: &JsValue,
    args: &[JsValue],
    captures: &(HostBridge,),
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let bridge = &captures.0;
    let observer_id = extract_observer_id(this, ctx)?;
    let target = require_target(args, "ResizeObserver.observe", ctx)?;

    let mut options = ResizeObserverOptions::default();
    // Per Resize Observer §2.3: parse optional `box` property from second argument.
    if let Some(opts) = args.get(1).and_then(JsValue::as_object) {
        if let Ok(v) = opts.get(js_string!("box"), ctx) {
            if let Ok(s) = v.to_string(ctx) {
                match s.to_std_string_escaped().as_str() {
                    "border-box" => {
                        options.box_model = ResizeObserverBoxOptions::BorderBox;
                    }
                    "device-pixel-content-box" => {
                        options.box_model = ResizeObserverBoxOptions::DevicePixelContentBox;
                    }
                    // "content-box" or anything else → default (ContentBox)
                    _ => {}
                }
            }
        }
    }
    let ro_id = elidex_api_observers::resize::ResizeObserverId::from_raw(observer_id);
    bridge.with_resize_observers(|reg| reg.observe(ro_id, target, options));
    Ok(JsValue::undefined())
}

fn resize_unobserve(
    this: &JsValue,
    args: &[JsValue],
    captures: &(HostBridge,),
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let bridge = &captures.0;
    let observer_id = extract_observer_id(this, ctx)?;
    let target = require_target(args, "ResizeObserver.unobserve", ctx)?;

    let ro_id = elidex_api_observers::resize::ResizeObserverId::from_raw(observer_id);
    bridge.with_resize_observers(|reg| reg.unobserve(ro_id, target));
    Ok(JsValue::undefined())
}

// --- IntersectionObserver ---

fn register_intersection_observer(ctx: &mut Context, bridge: &HostBridge) {
    let bridge_c = bridge.clone();
    let constructor = NativeFunction::from_copy_closure_with_captures(
        |_this, args, captures: &(HostBridge,), ctx| {
            let bridge = &captures.0;
            let callback = args.first().and_then(JsValue::as_callable).ok_or_else(|| {
                JsNativeError::typ()
                    .with_message("IntersectionObserver: argument 1 must be a function")
            })?;

            let mut init = IntersectionObserverInit::default();
            if let Some(opts) = args.get(1).and_then(JsValue::as_object) {
                if let Ok(v) = opts.get(js_string!("rootMargin"), ctx) {
                    if !v.is_undefined() {
                        init.root_margin = v.to_string(ctx)?.to_std_string_escaped();
                    }
                }
                if let Ok(v) = opts.get(js_string!("threshold"), ctx) {
                    if let Some(arr_obj) = v.as_object() {
                        if let Ok(len) = arr_obj.get(js_string!("length"), ctx) {
                            let len = len.to_u32(ctx).unwrap_or(0);
                            for i in 0..len {
                                if let Ok(item) = arr_obj.get(i, ctx) {
                                    if let Ok(n) = item.to_number(ctx) {
                                        // Per Intersection Observer §3.2: threshold values
                                        // must be in [0.0, 1.0] and finite.
                                        if n.is_finite() && (0.0..=1.0).contains(&n) {
                                            init.threshold.push(n);
                                        }
                                    }
                                }
                            }
                        }
                    } else if let Ok(n) = v.to_number(ctx) {
                        if n.is_finite() && (0.0..=1.0).contains(&n) {
                            init.threshold.push(n);
                        }
                    }
                }
            }
            if init.threshold.is_empty() {
                init.threshold.push(0.0);
            }
            // Per Intersection Observer §3.2: thresholds must be sorted
            // in ascending order and deduplicated.
            init.threshold
                .sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            init.threshold.dedup();

            let observer_id = bridge.with_intersection_observers(|reg| reg.register(init).raw());

            let observe_bridge = bridge.clone();
            let unobserve_bridge = bridge.clone();
            let disconnect_bridge = bridge.clone();

            let observer_obj = ObjectInitializer::new(ctx)
                .property(
                    js_string!(OBSERVER_ID_KEY),
                    JsValue::from(observer_id as f64),
                    Attribute::empty(),
                )
                .function(
                    NativeFunction::from_copy_closure_with_captures(
                        intersection_observe,
                        (observe_bridge,),
                    ),
                    js_string!("observe"),
                    1,
                )
                .function(
                    NativeFunction::from_copy_closure_with_captures(
                        intersection_unobserve,
                        (unobserve_bridge,),
                    ),
                    js_string!("unobserve"),
                    1,
                )
                .function(
                    NativeFunction::from_copy_closure_with_captures(
                        observer_disconnect::<IntersectionDisconnect>,
                        (disconnect_bridge,),
                    ),
                    js_string!("disconnect"),
                    0,
                )
                .build();

            bridge.store_observer_callback(observer_id, callback.clone(), observer_obj.clone());

            Ok(JsValue::from(observer_obj))
        },
        (bridge_c,),
    );

    let realm = ctx.realm().clone();
    let js_fn = FunctionObjectBuilder::new(&realm, constructor)
        .name(js_string!("IntersectionObserver"))
        .length(1)
        .constructor(true)
        .build();
    let global = ctx.global_object();
    global
        .set(js_string!("IntersectionObserver"), js_fn, false, ctx)
        .expect("failed to register IntersectionObserver");
}

fn intersection_observe(
    this: &JsValue,
    args: &[JsValue],
    captures: &(HostBridge,),
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let bridge = &captures.0;
    let observer_id = extract_observer_id(this, ctx)?;
    let target = require_target(args, "IntersectionObserver.observe", ctx)?;

    let io_id = elidex_api_observers::intersection::IntersectionObserverId::from_raw(observer_id);
    bridge.with_intersection_observers(|reg| reg.observe(io_id, target));
    Ok(JsValue::undefined())
}

fn intersection_unobserve(
    this: &JsValue,
    args: &[JsValue],
    captures: &(HostBridge,),
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let bridge = &captures.0;
    let observer_id = extract_observer_id(this, ctx)?;
    let target = require_target(args, "IntersectionObserver.unobserve", ctx)?;

    let io_id = elidex_api_observers::intersection::IntersectionObserverId::from_raw(observer_id);
    bridge.with_intersection_observers(|reg| reg.unobserve(io_id, target));
    Ok(JsValue::undefined())
}

// --- Shared helpers ---

/// Extract the observer ID from `this.__elidex_observer_id__`.
fn extract_observer_id(this: &JsValue, ctx: &mut Context) -> JsResult<u64> {
    let obj = this
        .as_object()
        .ok_or_else(|| JsNativeError::typ().with_message("Observer method called on non-object"))?;
    let val = obj.get(js_string!(OBSERVER_ID_KEY), ctx)?;
    let n = val.to_number(ctx)?;
    Ok(n as u64)
}

/// Trait for type-safe disconnect dispatch.
trait DisconnectKind {
    fn disconnect(bridge: &HostBridge, observer_id: u64);
}

struct MutationDisconnect;
impl DisconnectKind for MutationDisconnect {
    fn disconnect(bridge: &HostBridge, observer_id: u64) {
        let id = elidex_api_observers::mutation::MutationObserverId::from_raw(observer_id);
        bridge.with_mutation_observers(|reg| reg.disconnect(id));
    }
}

struct ResizeDisconnect;
impl DisconnectKind for ResizeDisconnect {
    fn disconnect(bridge: &HostBridge, observer_id: u64) {
        let id = elidex_api_observers::resize::ResizeObserverId::from_raw(observer_id);
        bridge.with_resize_observers(|reg| reg.disconnect(id));
    }
}

struct IntersectionDisconnect;
impl DisconnectKind for IntersectionDisconnect {
    fn disconnect(bridge: &HostBridge, observer_id: u64) {
        let id = elidex_api_observers::intersection::IntersectionObserverId::from_raw(observer_id);
        bridge.with_intersection_observers(|reg| reg.disconnect(id));
    }
}

fn observer_disconnect<D: DisconnectKind>(
    this: &JsValue,
    _args: &[JsValue],
    captures: &(HostBridge,),
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let bridge = &captures.0;
    let observer_id = extract_observer_id(this, ctx)?;
    D::disconnect(bridge, observer_id);
    Ok(JsValue::undefined())
}
