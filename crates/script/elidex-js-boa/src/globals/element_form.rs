//! Form control JS accessors for element wrapper objects.
//!
//! Registers value, checked, disabled, required, readOnly, name, defaultValue,
//! type, selectedIndex, `selectionStart`/End, `checkValidity()`, `select()`,
//! `setSelectionRange()` on element objects.

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, JsValue, NativeFunction};
use elidex_ecs::Entity;

use super::element::extract_entity;
use crate::bridge::HostBridge;

/// Convert a JS number (f64) to usize, clamping NaN/negative/infinity to 0.
#[must_use]
fn js_number_to_usize(n: f64) -> usize {
    if n.is_nan() || !n.is_finite() || n < 0.0 {
        0
    } else {
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss
        )]
        {
            n as usize
        }
    }
}

/// Register form control accessors (value, checked, disabled, type, name, etc.).
pub(super) fn register_form_accessors(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
    realm: &boa_engine::realm::Realm,
) {
    register_value_accessor(init, bridge, realm);
    register_bool_fcs_accessor(
        init,
        bridge,
        realm,
        "checked",
        |fcs| fcs.checked,
        |fcs, v| {
            fcs.checked = v;
        },
        Some(elidex_ecs::ElementState::CHECKED),
    );
    register_bool_fcs_accessor(
        init,
        bridge,
        realm,
        "disabled",
        |fcs| fcs.disabled,
        |fcs, v| {
            fcs.disabled = v;
        },
        Some(elidex_ecs::ElementState::DISABLED),
    );
    register_bool_fcs_accessor(
        init,
        bridge,
        realm,
        "required",
        |fcs| fcs.required,
        |fcs, v| {
            fcs.required = v;
        },
        Some(elidex_ecs::ElementState::REQUIRED),
    );
    register_bool_fcs_accessor(
        init,
        bridge,
        realm,
        "readOnly",
        |fcs| fcs.readonly,
        |fcs, v| {
            fcs.readonly = v;
        },
        Some(elidex_ecs::ElementState::READ_ONLY),
    );
    register_string_fcs_accessor(
        init,
        bridge,
        realm,
        "name",
        |fcs| fcs.name.clone(),
        |fcs, v| {
            fcs.name = v;
        },
    );
    register_string_fcs_accessor(
        init,
        bridge,
        realm,
        "defaultValue",
        |fcs| fcs.default_value.clone(),
        |fcs, v| {
            fcs.default_value = v;
        },
    );
    register_type_accessor(init, bridge, realm);
    register_selected_index_accessor(init, bridge, realm);
    register_selection_accessors(init, bridge, realm);
    register_check_validity_method(init, bridge);
    register_select_method(init, bridge);
    register_set_selection_range_method(init, bridge);
}

fn register_value_accessor(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
    realm: &boa_engine::realm::Realm,
) {
    let b = bridge.clone();
    let get_fn = NativeFunction::from_copy_closure_with_captures(
        |this, _args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            bridge.with(|_session, dom| {
                let val = dom
                    .world()
                    .get::<&elidex_form::FormControlState>(entity)
                    .ok()
                    .map(|fcs| fcs.value.clone());
                match val {
                    Some(v) => Ok(JsValue::from(js_string!(v))),
                    None => Ok(JsValue::undefined()),
                }
            })
        },
        b,
    )
    .to_js_function(realm);

    let b = bridge.clone();
    let set_fn = NativeFunction::from_copy_closure_with_captures(
        |this, args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            let text = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            bridge.with(|_session, dom| {
                if let Ok(mut fcs) = dom
                    .world_mut()
                    .get::<&mut elidex_form::FormControlState>(entity)
                {
                    // HTML spec §4.10.5.4: single-line inputs strip newlines from value.
                    let sanitized = if fcs.kind.is_single_line_text() {
                        text.replace(['\n', '\r'], "")
                    } else {
                        text
                    };
                    // HTML spec §4.10.5.4: setting .value IDL does NOT enforce
                    // maxlength (intentional per spec). maxlength only constrains
                    // user input (keyboard, paste).
                    fcs.cursor_pos = sanitized.len();
                    fcs.value = sanitized;
                    fcs.dirty_value = true;
                    fcs.update_char_count();
                }
                Ok(JsValue::undefined())
            })
        },
        b,
    )
    .to_js_function(realm);

    init.accessor(
        js_string!("value"),
        Some(get_fn),
        Some(set_fn),
        Attribute::CONFIGURABLE,
    );
}

/// Register a boolean `FormControlState` accessor with optional `ElementState` sync.
fn register_bool_fcs_accessor(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
    realm: &boa_engine::realm::Realm,
    prop_name: &str,
    getter: fn(&elidex_form::FormControlState) -> bool,
    setter: fn(&mut elidex_form::FormControlState, bool),
    element_flag: Option<u16>,
) {
    let b = bridge.clone();
    let get_fn = NativeFunction::from_copy_closure_with_captures(
        move |this, _args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            bridge.with(|_session, dom| {
                let val = dom
                    .world()
                    .get::<&elidex_form::FormControlState>(entity)
                    .ok()
                    .is_some_and(|fcs| getter(&fcs));
                Ok(JsValue::from(val))
            })
        },
        b,
    )
    .to_js_function(realm);

    let b = bridge.clone();
    let set_fn = NativeFunction::from_copy_closure_with_captures(
        move |this, args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            let flag = args.first().is_some_and(JsValue::to_boolean);
            bridge.with(|_session, dom| {
                if let Ok(mut fcs) = dom
                    .world_mut()
                    .get::<&mut elidex_form::FormControlState>(entity)
                {
                    setter(&mut fcs, flag);
                }
                if let Some(ef) = element_flag {
                    sync_element_flag(dom, entity, ef, flag);
                }
                Ok(JsValue::undefined())
            })
        },
        b,
    )
    .to_js_function(realm);

    init.accessor(
        js_string!(prop_name),
        Some(get_fn),
        Some(set_fn),
        Attribute::CONFIGURABLE,
    );
}

/// Register a string `FormControlState` accessor.
fn register_string_fcs_accessor(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
    realm: &boa_engine::realm::Realm,
    prop_name: &str,
    getter: fn(&elidex_form::FormControlState) -> String,
    setter: fn(&mut elidex_form::FormControlState, String),
) {
    let b = bridge.clone();
    let get_fn = NativeFunction::from_copy_closure_with_captures(
        move |this, _args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            bridge.with(|_session, dom| {
                let val = dom
                    .world()
                    .get::<&elidex_form::FormControlState>(entity)
                    .ok()
                    .map(|fcs| getter(&fcs));
                match val {
                    Some(v) => Ok(JsValue::from(js_string!(v))),
                    None => Ok(JsValue::from(js_string!(""))),
                }
            })
        },
        b,
    )
    .to_js_function(realm);

    let b = bridge.clone();
    let set_fn = NativeFunction::from_copy_closure_with_captures(
        move |this, args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            let text = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            bridge.with(|_session, dom| {
                if let Ok(mut fcs) = dom
                    .world_mut()
                    .get::<&mut elidex_form::FormControlState>(entity)
                {
                    setter(&mut fcs, text);
                }
                Ok(JsValue::undefined())
            })
        },
        b,
    )
    .to_js_function(realm);

    init.accessor(
        js_string!(prop_name),
        Some(get_fn),
        Some(set_fn),
        Attribute::CONFIGURABLE,
    );
}

/// Register the `type` accessor (getter + setter).
///
/// Setter changes the `FormControlKind` and reinitializes state (S14).
fn register_type_accessor(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
    realm: &boa_engine::realm::Realm,
) {
    let b = bridge.clone();
    let get_fn = NativeFunction::from_copy_closure_with_captures(
        |this, _args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            bridge.with(|_session, dom| {
                let type_str = dom
                    .world()
                    .get::<&elidex_form::FormControlState>(entity)
                    .ok()
                    .map(|fcs| {
                        // HTML spec: <select multiple> has type "select-multiple".
                        if fcs.kind == elidex_form::FormControlKind::Select && fcs.multiple {
                            "select-multiple"
                        } else {
                            fcs.kind.as_str()
                        }
                    });
                if let Some(s) = type_str {
                    Ok(JsValue::from(js_string!(s)))
                } else {
                    // Non-form elements: return tag name or empty.
                    let tag = dom
                        .world()
                        .get::<&elidex_ecs::TagType>(entity)
                        .ok()
                        .map(|t| t.0.clone())
                        .unwrap_or_default();
                    Ok(JsValue::from(js_string!(tag)))
                }
            })
        },
        b,
    )
    .to_js_function(realm);

    let b = bridge.clone();
    let set_fn = NativeFunction::from_copy_closure_with_captures(
        |this, args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            let type_str = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default()
                .to_ascii_lowercase();
            bridge.with(|_session, dom| {
                if let Ok(mut fcs) = dom
                    .world_mut()
                    .get::<&mut elidex_form::FormControlState>(entity)
                {
                    fcs.kind = elidex_form::FormControlKind::from_type_str(&type_str);
                }
                Ok(JsValue::undefined())
            })
        },
        b,
    )
    .to_js_function(realm);

    init.accessor(
        js_string!("type"),
        Some(get_fn),
        Some(set_fn),
        Attribute::CONFIGURABLE,
    );
}

/// Register `selectedIndex` accessor for `<select>` elements.
fn register_selected_index_accessor(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
    realm: &boa_engine::realm::Realm,
) {
    let b = bridge.clone();
    let get_fn = NativeFunction::from_copy_closure_with_captures(
        |this, _args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            bridge.with(|_session, dom| {
                let idx = dom
                    .world()
                    .get::<&elidex_form::FormControlState>(entity)
                    .ok()
                    .map_or(-1, |fcs| fcs.selected_index);
                Ok(JsValue::from(idx))
            })
        },
        b,
    )
    .to_js_function(realm);

    let b = bridge.clone();
    let set_fn = NativeFunction::from_copy_closure_with_captures(
        |this, args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            let idx = args
                .first()
                .map(|v| v.to_number(ctx))
                .transpose()?
                .map_or(-1, |n| {
                    if n.is_nan() || n.is_infinite() {
                        -1
                    } else {
                        #[allow(clippy::cast_possible_truncation)]
                        {
                            n.clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32
                        }
                    }
                });
            bridge.with(|_session, dom| {
                if let Ok(mut fcs) = dom
                    .world_mut()
                    .get::<&mut elidex_form::FormControlState>(entity)
                {
                    elidex_form::select_option(&mut fcs, idx);
                }
                Ok(JsValue::undefined())
            })
        },
        b,
    )
    .to_js_function(realm);

    init.accessor(
        js_string!("selectedIndex"),
        Some(get_fn),
        Some(set_fn),
        Attribute::CONFIGURABLE,
    );
}

/// Register a usize `FormControlState` accessor with UTF-16 conversion.
///
/// Used for `selectionStart`/`selectionEnd` — both follow the same pattern:
/// getter reads a byte offset from FCS and returns a UTF-16 code unit index;
/// setter receives a UTF-16 code unit index and converts to a byte offset.
/// Per HTML spec, selection offsets are in UTF-16 code units.
///
/// WHATWG §4.10.5.2.8: returns `null` for input types that don't support
/// the selection API (only text/password/textarea/email/url/tel/search).
fn register_usize_fcs_accessor(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
    realm: &boa_engine::realm::Realm,
    prop_name: &str,
    getter: fn(&elidex_form::FormControlState) -> usize,
    setter: fn(&mut elidex_form::FormControlState, usize),
) {
    let b = bridge.clone();
    let get_fn = NativeFunction::from_copy_closure_with_captures(
        move |this, _args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            bridge.with(|_session, dom| {
                let fcs = dom
                    .world()
                    .get::<&elidex_form::FormControlState>(entity)
                    .ok();
                // WHATWG §4.10.5.2.8: return null for non-applicable types.
                let supports = fcs.as_ref().is_some_and(|f| f.kind.supports_selection());
                if !supports {
                    return Ok(JsValue::null());
                }
                let val = fcs.map_or(0, |fcs| {
                    let byte_pos = getter(&fcs);
                    elidex_form::util::byte_offset_to_utf16(&fcs.value, byte_pos)
                });
                Ok(JsValue::from(val as f64))
            })
        },
        b,
    )
    .to_js_function(realm);

    let b = bridge.clone();
    let set_fn = NativeFunction::from_copy_closure_with_captures(
        move |this, args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            let val = args
                .first()
                .map(|v| v.to_number(ctx))
                .transpose()?
                .map_or(0, js_number_to_usize);
            bridge.with(|_session, dom| {
                if let Ok(mut fcs) = dom
                    .world_mut()
                    .get::<&mut elidex_form::FormControlState>(entity)
                {
                    let byte_offset = elidex_form::util::utf16_to_byte_offset(&fcs.value, val);
                    setter(&mut fcs, byte_offset);
                }
                Ok(JsValue::undefined())
            })
        },
        b,
    )
    .to_js_function(realm);

    init.accessor(
        js_string!(prop_name),
        Some(get_fn),
        Some(set_fn),
        Attribute::CONFIGURABLE,
    );
}

/// Register `selectionStart` and `selectionEnd` accessors.
fn register_selection_accessors(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
    realm: &boa_engine::realm::Realm,
) {
    register_usize_fcs_accessor(
        init,
        bridge,
        realm,
        "selectionStart",
        |fcs| fcs.selection_start,
        |fcs, v| {
            fcs.selection_start = v;
        },
    );
    register_usize_fcs_accessor(
        init,
        bridge,
        realm,
        "selectionEnd",
        |fcs| fcs.selection_end,
        |fcs, v| {
            fcs.selection_end = v;
        },
    );
}

/// Register `checkValidity()` method.
///
/// WHATWG §4.10.15.5: if the control is invalid, fires an `invalid` event
/// (cancelable, not composed). Returns `true` if valid, `false` if invalid.
///
/// Limitation: the `invalid` event is dispatched via `dispatch_event()` with
/// a no-op callback (boa does not support re-entering the JS engine from a
/// native function). JS `addEventListener("invalid", ...)` listeners will
/// fire once event queue deferred dispatch is implemented (M4-3.7+).
fn register_check_validity_method(init: &mut ObjectInitializer<'_>, bridge: &HostBridge) {
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, _args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                bridge.with(|_session, dom| {
                    let valid = dom
                        .world()
                        .get::<&elidex_form::FormControlState>(entity)
                        .ok()
                        .is_none_or(|fcs| elidex_form::validate_control(&fcs).is_valid());
                    if !valid {
                        // WHATWG §4.10.15.5: fire "invalid" event (cancelable, not composed).
                        let mut event =
                            elidex_script_session::DispatchEvent::new("invalid", entity);
                        event.cancelable = true;
                        elidex_script_session::dispatch_event(dom, &mut event, &mut |_, _, _| {});
                    }
                    Ok(JsValue::from(valid))
                })
            },
            b,
        ),
        js_string!("checkValidity"),
        0,
    );
}

/// Register `select()` method — selects all text in the control.
fn register_select_method(init: &mut ObjectInitializer<'_>, bridge: &HostBridge) {
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, _args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                bridge.with(|_session, dom| {
                    if let Ok(mut fcs) = dom
                        .world_mut()
                        .get::<&mut elidex_form::FormControlState>(entity)
                    {
                        elidex_form::select_all(&mut fcs);
                    }
                    Ok(JsValue::undefined())
                })
            },
            b,
        ),
        js_string!("select"),
        0,
    );
}

/// Register `setSelectionRange(start, end, direction?)` method.
fn register_set_selection_range_method(init: &mut ObjectInitializer<'_>, bridge: &HostBridge) {
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let start = args
                    .first()
                    .map(|v| v.to_number(ctx))
                    .transpose()?
                    .map_or(0, js_number_to_usize);
                let end = args
                    .get(1)
                    .map(|v| v.to_number(ctx))
                    .transpose()?
                    .map_or(0, js_number_to_usize);
                let direction = args
                    .get(2)
                    .filter(|v| !v.is_undefined())
                    .map(|v| v.to_string(ctx))
                    .transpose()?
                    .map(|s| s.to_std_string_escaped());
                bridge.with(|_session, dom| {
                    if let Ok(mut fcs) = dom
                        .world_mut()
                        .get::<&mut elidex_form::FormControlState>(entity)
                    {
                        let byte_start = elidex_form::util::utf16_to_byte_offset(&fcs.value, start);
                        let byte_end = elidex_form::util::utf16_to_byte_offset(&fcs.value, end);
                        fcs.selection_start =
                            elidex_form::util::snap_to_char_boundary(&fcs.value, byte_start);
                        fcs.selection_end =
                            elidex_form::util::snap_to_char_boundary(&fcs.value, byte_end);
                        // HTML spec §4.10.5.2.10: if direction is omitted (undefined),
                        // preserve the existing direction. Only reset to None when
                        // explicitly set to "none".
                        if let Some(ref dir) = direction {
                            fcs.selection_direction = match dir.as_str() {
                                "forward" => elidex_form::SelectionDirection::Forward,
                                "backward" => elidex_form::SelectionDirection::Backward,
                                _ => elidex_form::SelectionDirection::None,
                            };
                        }
                    }
                    Ok(JsValue::undefined())
                })
            },
            b,
        ),
        js_string!("setSelectionRange"),
        2,
    );
}

/// Sync an `ElementState` flag on an entity (for CSS pseudo-class matching).
fn sync_element_flag(dom: &mut elidex_ecs::EcsDom, entity: Entity, flag: u16, value: bool) {
    if let Ok(mut es) = dom.world_mut().get::<&mut elidex_ecs::ElementState>(entity) {
        es.set(flag, value);
    }
}
