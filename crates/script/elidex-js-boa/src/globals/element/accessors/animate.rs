//! `element.animate()` and `element.getAnimations()` (Web Animations API §4.4).
//!
//! Creates `Animation` JS objects that correspond to animations managed by the
//! CSS `AnimationEngine`. Since the engine is owned by the content thread's
//! `PipelineResult`, pending animations are buffered on the bridge and applied
//! by the content thread in the next frame.

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, JsValue, NativeFunction};

use crate::bridge::HostBridge;
use crate::globals::element::core::extract_entity;

/// Register `element.animate()` and `element.getAnimations()` on the element prototype.
pub(in crate::globals::element) fn register_animate_methods(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
) {
    // element.animate(keyframes, options) — Web Animations API §4.4.1.
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let entity_id = entity.to_bits().get();

                // Parse keyframes (arg 0).
                let keyframes_val = args.first().cloned().unwrap_or(JsValue::undefined());
                let keyframes = parse_keyframes(&keyframes_val, ctx)?;

                // Parse options (arg 1) — either number (duration ms) or object.
                let options = parse_animation_options(args.get(1), ctx)?;

                // Buffer the animation request on the bridge.
                bridge.queue_script_animation(ScriptAnimation {
                    entity_id,
                    keyframes,
                    options: options.clone(),
                });

                // Build and return an Animation JS object.
                Ok(JsValue::from(build_animation_object(
                    entity_id, &options, ctx,
                )?))
            },
            b,
        ),
        js_string!("animate"),
        1,
    );

    // element.getAnimations() — Web Animations API §4.4.11.
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, _args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let entity_id = entity.to_bits().get();

                // Query active animations from the bridge.
                let count = bridge.animation_count(entity_id);

                let arr = boa_engine::object::builtins::JsArray::new(ctx);
                for i in 0..count {
                    let info = bridge.animation_info(entity_id, i);
                    if let Some(info) = info {
                        let anim = build_animation_from_info(entity_id, &info, ctx)?;
                        arr.push(JsValue::from(anim), ctx)?;
                    }
                }
                Ok(JsValue::from(arr))
            },
            b,
        ),
        js_string!("getAnimations"),
        0,
    );
}

/// A parsed keyframe for the animate() API.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct ParsedKeyframe {
    /// The offset (0.0 to 1.0), or None for auto-distributed.
    pub offset: Option<f64>,
    /// The easing for this keyframe interval.
    pub easing: String,
    /// Property declarations as (name, value) pairs.
    pub declarations: Vec<(String, String)>,
}

/// Parsed animation options.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct AnimationOptions {
    /// Duration in milliseconds.
    pub duration: f64,
    /// Number of iterations (f64::INFINITY for infinite).
    pub iterations: f64,
    /// Easing function name.
    pub easing: String,
    /// Fill mode: "none", "forwards", "backwards", "both".
    pub fill: String,
    /// Direction: "normal", "reverse", "alternate", "alternate-reverse".
    pub direction: String,
    /// Delay in milliseconds.
    pub delay: f64,
    /// Animation ID (optional).
    pub id: String,
    /// Composite operation: "replace", "add", or "accumulate".
    pub composite: String,
}

impl Default for AnimationOptions {
    fn default() -> Self {
        Self {
            duration: 0.0,
            iterations: 1.0,
            easing: "linear".into(),
            fill: "none".into(),
            direction: "normal".into(),
            delay: 0.0,
            id: String::new(),
            composite: "replace".into(),
        }
    }
}

/// A buffered script-initiated animation request.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct ScriptAnimation {
    /// Entity ID bits.
    pub entity_id: u64,
    /// Parsed keyframes.
    pub keyframes: Vec<ParsedKeyframe>,
    /// Animation options.
    pub options: AnimationOptions,
}

/// Info about an active animation, returned by the bridge.
#[derive(Clone, Debug)]
pub(crate) struct AnimationInfo {
    /// Animation name or id.
    pub id: String,
    /// Current play state.
    pub play_state: String,
    /// Current time in ms.
    pub current_time: f64,
}

/// Parse keyframes from JS value.
///
/// Two formats (Web Animations §5.1):
/// 1. Array of keyframe objects: `[{ opacity: 0 }, { opacity: 1 }]`
/// 2. Property arrays: `{ opacity: [0, 1] }`
fn parse_keyframes(
    val: &JsValue,
    ctx: &mut boa_engine::Context,
) -> boa_engine::JsResult<Vec<ParsedKeyframe>> {
    let obj = match val.as_object() {
        Some(o) => o,
        None => return Ok(Vec::new()),
    };

    // Check if it's an array (format 1) by checking for numeric length.
    let len_val = obj.get(js_string!("length"), ctx)?;
    let is_array = len_val.as_number().is_some_and(|n| n >= 0.0);

    if is_array {
        // Format 1: array of keyframe objects.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let len = len_val.to_number(ctx)? as u32;
        let mut keyframes = Vec::with_capacity(len as usize);

        for i in 0..len {
            let kf_val = obj.get(i, ctx)?;
            let kf_obj = match kf_val.as_object() {
                Some(o) => o,
                None => continue,
            };

            let offset = kf_obj
                .get(js_string!("offset"), ctx)?
                .as_number()
                .filter(|n| n.is_finite());

            let easing = kf_obj
                .get(js_string!("easing"), ctx)
                .ok()
                .and_then(|v| {
                    if v.is_undefined() || v.is_null() {
                        None
                    } else {
                        Some(v.to_string(ctx).ok()?.to_std_string_escaped())
                    }
                })
                .unwrap_or_else(|| "linear".into());

            // Collect all properties (excluding offset, easing, composite).
            let own_keys = kf_obj.own_property_keys(ctx)?;
            let mut declarations = Vec::new();
            for key in own_keys {
                let key_str = match &key {
                    boa_engine::property::PropertyKey::String(s) => s.to_std_string_escaped(),
                    _ => continue,
                };
                if matches!(
                    key_str.as_str(),
                    "offset" | "easing" | "composite"
                ) {
                    continue;
                }
                let val_str = kf_obj
                    .get(key.clone(), ctx)?
                    .to_string(ctx)?
                    .to_std_string_escaped();
                // Convert camelCase to kebab-case for CSS properties.
                let css_prop = camel_to_kebab(&key_str);
                declarations.push((css_prop, val_str));
            }

            keyframes.push(ParsedKeyframe {
                offset,
                easing,
                declarations,
            });
        }

        Ok(keyframes)
    } else {
        // Format 2: property arrays { opacity: [0, 1], transform: ["...", "..."] }.
        let own_keys = obj.own_property_keys(ctx)?;
        let mut prop_arrays: Vec<(String, Vec<String>)> = Vec::new();
        let mut max_len = 0u32;

        for key in own_keys {
            let key_str = match &key {
                boa_engine::property::PropertyKey::String(s) => s.to_std_string_escaped(),
                _ => continue,
            };
            if matches!(key_str.as_str(), "easing" | "offset" | "composite") {
                continue;
            }
            let arr_val = obj.get(key.clone(), ctx)?;
            let arr_obj = match arr_val.as_object() {
                Some(o) => o,
                None => {
                    // Single value → treat as [value].
                    let v = arr_val.to_string(ctx)?.to_std_string_escaped();
                    prop_arrays.push((camel_to_kebab(&key_str), vec![v]));
                    max_len = max_len.max(1);
                    continue;
                }
            };
            let arr_len = arr_obj
                .get(js_string!("length"), ctx)?
                .to_number(ctx)
                .unwrap_or(0.0) as u32;
            max_len = max_len.max(arr_len);
            let mut vals = Vec::with_capacity(arr_len as usize);
            for j in 0..arr_len {
                let v = arr_obj.get(j, ctx)?.to_string(ctx)?.to_std_string_escaped();
                vals.push(v);
            }
            prop_arrays.push((camel_to_kebab(&key_str), vals));
        }

        let mut keyframes = Vec::with_capacity(max_len as usize);
        for i in 0..max_len {
            let mut declarations = Vec::new();
            for (prop, vals) in &prop_arrays {
                if let Some(v) = vals.get(i as usize) {
                    declarations.push((prop.clone(), v.clone()));
                }
            }
            keyframes.push(ParsedKeyframe {
                offset: if max_len <= 1 {
                    Some(1.0)
                } else {
                    Some(f64::from(i) / f64::from(max_len - 1))
                },
                easing: "linear".into(),
                declarations,
            });
        }

        Ok(keyframes)
    }
}

/// Parse animation options from the second argument of animate().
fn parse_animation_options(
    val: Option<&JsValue>,
    ctx: &mut boa_engine::Context,
) -> boa_engine::JsResult<AnimationOptions> {
    let Some(v) = val else {
        return Ok(AnimationOptions::default());
    };

    // Number form: just duration in ms.
    if let Some(n) = v.as_number() {
        return Ok(AnimationOptions {
            duration: n,
            ..Default::default()
        });
    }

    let obj = match v.as_object() {
        Some(o) => o,
        None => return Ok(AnimationOptions::default()),
    };

    let duration = obj
        .get(js_string!("duration"), ctx)?
        .to_number(ctx)
        .unwrap_or(0.0);
    let iterations = obj
        .get(js_string!("iterations"), ctx)?
        .to_number(ctx)
        .unwrap_or(1.0);
    let delay = obj
        .get(js_string!("delay"), ctx)?
        .to_number(ctx)
        .unwrap_or(0.0);

    let easing = obj
        .get(js_string!("easing"), ctx)
        .ok()
        .and_then(|v| {
            if v.is_undefined() {
                None
            } else {
                Some(v.to_string(ctx).ok()?.to_std_string_escaped())
            }
        })
        .unwrap_or_else(|| "linear".into());

    let fill = obj
        .get(js_string!("fill"), ctx)
        .ok()
        .and_then(|v| {
            if v.is_undefined() {
                None
            } else {
                Some(v.to_string(ctx).ok()?.to_std_string_escaped())
            }
        })
        .unwrap_or_else(|| "none".into());

    let direction = obj
        .get(js_string!("direction"), ctx)
        .ok()
        .and_then(|v| {
            if v.is_undefined() {
                None
            } else {
                Some(v.to_string(ctx).ok()?.to_std_string_escaped())
            }
        })
        .unwrap_or_else(|| "normal".into());

    let id = obj
        .get(js_string!("id"), ctx)
        .ok()
        .and_then(|v| {
            if v.is_undefined() {
                None
            } else {
                Some(v.to_string(ctx).ok()?.to_std_string_escaped())
            }
        })
        .unwrap_or_default();

    let composite = obj
        .get(js_string!("composite"), ctx)
        .ok()
        .and_then(|v| {
            if v.is_undefined() {
                None
            } else {
                Some(v.to_string(ctx).ok()?.to_std_string_escaped())
            }
        })
        .unwrap_or_else(|| "replace".into());

    Ok(AnimationOptions {
        duration,
        iterations,
        easing,
        fill,
        direction,
        delay,
        id,
        composite,
    })
}

/// Build an `Animation` JS object (Web Animations §4.4).
fn build_animation_object(
    _entity_id: u64,
    options: &AnimationOptions,
    ctx: &mut boa_engine::Context,
) -> boa_engine::JsResult<boa_engine::JsObject> {
    // Pre-build promises before ObjectInitializer borrows ctx.
    let ready = boa_engine::object::builtins::JsPromise::resolve(JsValue::undefined(), ctx);
    let (finished, _resolvers) = boa_engine::object::builtins::JsPromise::new_pending(ctx);

    let mut init = ObjectInitializer::new(ctx);

    // id — read/write.
    init.property(
        js_string!("id"),
        JsValue::from(js_string!(options.id.as_str())),
        Attribute::WRITABLE | Attribute::CONFIGURABLE,
    );

    // playState — "running" initially.
    init.property(
        js_string!("playState"),
        JsValue::from(js_string!("running")),
        Attribute::WRITABLE | Attribute::CONFIGURABLE,
    );

    // playbackRate.
    init.property(
        js_string!("playbackRate"),
        JsValue::from(1.0),
        Attribute::WRITABLE | Attribute::CONFIGURABLE,
    );

    // currentTime — starts at 0.
    init.property(
        js_string!("currentTime"),
        JsValue::from(0.0),
        Attribute::WRITABLE | Attribute::CONFIGURABLE,
    );

    // startTime — null initially (set when animation starts playing).
    init.property(
        js_string!("startTime"),
        JsValue::null(),
        Attribute::WRITABLE | Attribute::CONFIGURABLE,
    );

    // pending — false initially.
    init.property(
        js_string!("pending"),
        JsValue::from(true),
        Attribute::WRITABLE | Attribute::CONFIGURABLE,
    );

    // onfinish / oncancel callbacks.
    init.property(
        js_string!("onfinish"),
        JsValue::null(),
        Attribute::WRITABLE | Attribute::CONFIGURABLE,
    );
    init.property(
        js_string!("oncancel"),
        JsValue::null(),
        Attribute::WRITABLE | Attribute::CONFIGURABLE,
    );

    // ready — Promise that resolves when animation starts.
    init.property(
        js_string!("ready"),
        JsValue::from(ready),
        Attribute::READONLY | Attribute::CONFIGURABLE,
    );

    // finished — Promise (pending, resolved when animation finishes).
    init.property(
        js_string!("finished"),
        JsValue::from(finished),
        Attribute::READONLY | Attribute::CONFIGURABLE,
    );

    // play()
    init.function(
        NativeFunction::from_copy_closure(|this, _args, ctx| {
            if let Some(obj) = this.as_object() {
                obj.set(
                    js_string!("playState"),
                    JsValue::from(js_string!("running")),
                    false,
                    ctx,
                )?;
            }
            Ok(JsValue::undefined())
        }),
        js_string!("play"),
        0,
    );

    // pause()
    init.function(
        NativeFunction::from_copy_closure(|this, _args, ctx| {
            if let Some(obj) = this.as_object() {
                obj.set(
                    js_string!("playState"),
                    JsValue::from(js_string!("paused")),
                    false,
                    ctx,
                )?;
            }
            Ok(JsValue::undefined())
        }),
        js_string!("pause"),
        0,
    );

    // cancel()
    init.function(
        NativeFunction::from_copy_closure(|this, _args, ctx| {
            if let Some(obj) = this.as_object() {
                obj.set(
                    js_string!("playState"),
                    JsValue::from(js_string!("idle")),
                    false,
                    ctx,
                )?;
                obj.set(js_string!("currentTime"), JsValue::null(), false, ctx)?;
                // Fire oncancel if set.
                let oncancel = obj.get(js_string!("oncancel"), ctx)?;
                if let Some(f) = oncancel.as_callable() {
                    let _ = f.call(&JsValue::from(obj.clone()), &[], ctx);
                }
            }
            Ok(JsValue::undefined())
        }),
        js_string!("cancel"),
        0,
    );

    // finish()
    init.function(
        NativeFunction::from_copy_closure(|this, _args, ctx| {
            if let Some(obj) = this.as_object() {
                obj.set(
                    js_string!("playState"),
                    JsValue::from(js_string!("finished")),
                    false,
                    ctx,
                )?;
                // Fire onfinish if set.
                let onfinish = obj.get(js_string!("onfinish"), ctx)?;
                if let Some(f) = onfinish.as_callable() {
                    let _ = f.call(&JsValue::from(obj.clone()), &[], ctx);
                }
            }
            Ok(JsValue::undefined())
        }),
        js_string!("finish"),
        0,
    );

    // reverse()
    init.function(
        NativeFunction::from_copy_closure(|this, _args, ctx| {
            if let Some(obj) = this.as_object() {
                let rate = obj
                    .get(js_string!("playbackRate"), ctx)?
                    .to_number(ctx)
                    .unwrap_or(1.0);
                obj.set(
                    js_string!("playbackRate"),
                    JsValue::from(-rate),
                    false,
                    ctx,
                )?;
            }
            Ok(JsValue::undefined())
        }),
        js_string!("reverse"),
        0,
    );

    // addEventListener / removeEventListener stubs.
    init.function(
        NativeFunction::from_fn_ptr(|_this, _args, _ctx| Ok(JsValue::undefined())),
        js_string!("addEventListener"),
        2,
    );
    init.function(
        NativeFunction::from_fn_ptr(|_this, _args, _ctx| Ok(JsValue::undefined())),
        js_string!("removeEventListener"),
        2,
    );

    Ok(init.build())
}

/// Build an Animation object from bridge info (for getAnimations).
fn build_animation_from_info(
    _entity_id: u64,
    info: &AnimationInfo,
    ctx: &mut boa_engine::Context,
) -> boa_engine::JsResult<boa_engine::JsObject> {
    let opts = AnimationOptions {
        id: info.id.clone(),
        ..Default::default()
    };
    let obj = build_animation_object(_entity_id, &opts, ctx)?;
    obj.set(
        js_string!("playState"),
        JsValue::from(js_string!(info.play_state.as_str())),
        false,
        ctx,
    )?;
    obj.set(
        js_string!("currentTime"),
        JsValue::from(info.current_time),
        false,
        ctx,
    )?;
    Ok(obj)
}

/// Convert camelCase to kebab-case (e.g., "backgroundColor" → "background-color").
fn camel_to_kebab(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 4);
    for (i, c) in s.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i > 0 {
                result.push('-');
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
}
