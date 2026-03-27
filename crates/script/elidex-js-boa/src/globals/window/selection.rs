//! `getSelection()` global function registration.

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsValue, NativeFunction};

use crate::bridge::HostBridge;

/// Register `getSelection()` global function returning a `Selection`-like object.
#[allow(clippy::too_many_lines)]
#[allow(clippy::similar_names)] // b_ar/b_rar/b_gc etc. are per-method captures.
pub(super) fn register_selection(ctx: &mut Context, bridge: &HostBridge) {
    let b_sel = bridge.clone();
    let get_selection = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, ctx| {
            let has_range = bridge.selection_range_id().is_some();
            let range_count = i32::from(has_range);

            let mut obj = ObjectInitializer::new(ctx);

            let sel_type = if has_range {
                js_string!("Range")
            } else {
                js_string!("None")
            };
            obj.property(
                js_string!("type"),
                JsValue::from(sel_type),
                Attribute::READONLY,
            );
            obj.property(
                js_string!("rangeCount"),
                JsValue::from(range_count),
                Attribute::READONLY,
            );

            // isCollapsed — read from the underlying Range if present.
            let collapsed = bridge
                .selection_range_id()
                .is_none_or(|rid| bridge.with_range(rid, |r| r.collapsed()).unwrap_or(true));
            obj.property(
                js_string!("isCollapsed"),
                JsValue::from(collapsed),
                Attribute::READONLY,
            );

            // toString() → return selected text from the underlying Range, if any.
            let b_tostr = bridge.clone();
            obj.function(
                NativeFunction::from_copy_closure_with_captures(
                    |_this, _args, bridge, _ctx| {
                        let text = bridge.selection_range_id().and_then(|rid| {
                            bridge
                                .with(|_session, dom| bridge.with_range(rid, |r| r.to_string(dom)))
                        });
                        Ok(JsValue::from(js_string!(text.unwrap_or_default().as_str())))
                    },
                    b_tostr,
                ),
                js_string!("toString"),
                0,
            );

            // getRangeAt(index) → returns the Range JS object if selection exists.
            let b_gra = bridge.clone();
            obj.function(
                NativeFunction::from_copy_closure_with_captures(
                    |_this, args, bridge, ctx| {
                        let index = args.first().and_then(JsValue::as_number).unwrap_or(0.0);
                        if index as u32 != 0 {
                            return Err(boa_engine::JsNativeError::range()
                                .with_message("index out of range")
                                .into());
                        }
                        match bridge.selection_range_id() {
                            Some(rid) => {
                                crate::globals::document::build_range_object(rid, bridge, ctx)
                            }
                            None => Err(boa_engine::JsNativeError::range()
                                .with_message("no range in selection")
                                .into()),
                        }
                    },
                    b_gra,
                ),
                js_string!("getRangeAt"),
                1,
            );

            // addRange(range) → store range_id in bridge.selection_range_id.
            let b_ar = bridge.clone();
            obj.function(
                NativeFunction::from_copy_closure_with_captures(
                    |_this, args, bridge, ctx| {
                        if let Some(range_obj) = args.first().and_then(JsValue::as_object) {
                            if let Ok(id_val) =
                                range_obj.get(js_string!("__elidex_traversal_id__"), ctx)
                            {
                                if let Some(id) = id_val.as_number() {
                                    #[allow(
                                        clippy::cast_possible_truncation,
                                        clippy::cast_sign_loss
                                    )]
                                    bridge.set_selection_range_id(Some(id as u64));
                                }
                            }
                        }
                        Ok(JsValue::undefined())
                    },
                    b_ar,
                ),
                js_string!("addRange"),
                1,
            );

            // removeAllRanges() → clears selection_range_id.
            let b_rar = bridge.clone();
            obj.function(
                NativeFunction::from_copy_closure_with_captures(
                    |_this, _args, bridge, _ctx| {
                        bridge.set_selection_range_id(None);
                        Ok(JsValue::undefined())
                    },
                    b_rar,
                ),
                js_string!("removeAllRanges"),
                0,
            );

            Ok(obj.build().into())
        },
        b_sel,
    );
    ctx.register_global_builtin_callable(js_string!("getSelection"), 0, get_selection)
        .expect("failed to register getSelection");
}
