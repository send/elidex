//! Native RegExp.prototype methods.

use super::value::{JsValue, NativeContext, Object, ObjectKind, Property, PropertyKey, VmError};

/// Convert a UTF-8 byte offset to a UTF-16 code unit index.
fn byte_offset_to_utf16(s: &str, byte_offset: usize) -> usize {
    s[..byte_offset].encode_utf16().count()
}

/// Convert a UTF-16 code unit index to a UTF-8 byte offset.
fn utf16_to_byte_offset(s: &str, utf16_idx: usize) -> usize {
    let mut units = 0;
    for (byte_idx, ch) in s.char_indices() {
        if units >= utf16_idx {
            return byte_idx;
        }
        units += ch.len_utf16();
    }
    s.len()
}

/// Run a regex match on a subject string, handling lastIndex for g/y flags.
/// Returns the Match if found.
fn run_regexp(
    ctx: &mut NativeContext<'_>,
    obj_id: super::value::ObjectId,
    subject: &str,
) -> Result<Option<regress::Match>, VmError> {
    // Extract flags string and determine global/sticky.
    let flags_str = {
        let obj = ctx.get_object(obj_id);
        if let ObjectKind::RegExp { flags, .. } = &obj.kind {
            ctx.vm.strings.get_utf8(*flags)
        } else {
            return Err(VmError::type_error("not a RegExp"));
        }
    };
    let is_global = flags_str.contains('g');
    let is_sticky = flags_str.contains('y');
    let uses_last_index = is_global || is_sticky;

    // Read lastIndex (UTF-16 code units) and convert to byte offset for global/sticky.
    let start = if uses_last_index {
        let last_index_key = PropertyKey::String(ctx.vm.strings.intern("lastIndex"));
        let obj = ctx.get_object(obj_id);
        let mut utf16_idx = 0usize;
        for (k, p) in &obj.properties {
            if *k == last_index_key {
                if let super::value::PropertyValue::Data(JsValue::Number(n)) = p.slot {
                    // ToLength-like coercion: non-finite/negative → 0.
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    if n.is_finite() && n > 0.0 {
                        utf16_idx = n.trunc() as usize;
                    }
                }
            }
        }
        utf16_to_byte_offset(subject, utf16_idx)
    } else {
        0
    };

    // Run the compiled regex. We need to borrow the compiled regex immutably
    // while also potentially holding a mutable reference later — so clone the
    // match result before mutating.
    let found = {
        let obj = ctx.get_object(obj_id);
        let ObjectKind::RegExp { ref compiled, .. } = obj.kind else {
            return Err(VmError::type_error("not a RegExp"));
        };
        let m = compiled.find_from(subject, start).next();
        // Sticky: the match must start exactly at `start`.
        if is_sticky {
            m.filter(|m| m.start() == start)
        } else {
            m
        }
    };

    // Update lastIndex for global/sticky.
    if uses_last_index {
        let last_index_key = PropertyKey::String(ctx.vm.strings.intern("lastIndex"));
        // Convert byte offset back to UTF-16 code units for lastIndex.
        #[allow(clippy::cast_precision_loss)]
        let new_idx = if let Some(ref m) = found {
            byte_offset_to_utf16(subject, m.end()) as f64
        } else {
            0.0
        };
        let obj = ctx.get_object_mut(obj_id);
        let mut updated = false;
        for prop in &mut obj.properties {
            if prop.0 == last_index_key {
                prop.1.slot = super::value::PropertyValue::Data(JsValue::Number(new_idx));
                updated = true;
                break;
            }
        }
        if !updated {
            obj.properties
                .push((last_index_key, Property::data(JsValue::Number(new_idx))));
        }
    }

    Ok(found)
}

pub(super) fn native_regexp_test(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let JsValue::Object(obj_id) = this else {
        return Err(VmError::type_error(
            "RegExp.prototype.test called on non-object",
        ));
    };
    let arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = super::coerce::to_string(ctx.vm, arg)?;
    let subject = ctx.vm.strings.get_utf8(sid);

    let found = run_regexp(ctx, obj_id, &subject)?;
    Ok(JsValue::Boolean(found.is_some()))
}

pub(super) fn native_regexp_exec(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let JsValue::Object(obj_id) = this else {
        return Err(VmError::type_error(
            "RegExp.prototype.exec called on non-object",
        ));
    };
    let arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = super::coerce::to_string(ctx.vm, arg)?;
    let subject = ctx.vm.strings.get_utf8(sid);

    let Some(m) = run_regexp(ctx, obj_id, &subject)? else {
        return Ok(JsValue::Null);
    };

    // Build result array: [full_match, ...groups]
    let full_match = &subject[m.start()..m.end()];
    let mut elements = vec![JsValue::String(ctx.intern(full_match))];

    // Capture groups.
    for group in &m.captures {
        match group {
            Some(range) => {
                let s = &subject[range.start..range.end];
                elements.push(JsValue::String(ctx.intern(s)));
            }
            None => elements.push(JsValue::Undefined),
        }
    }

    let arr_id = ctx.alloc_object(Object {
        kind: ObjectKind::Array { elements },
        properties: Vec::new(),
        prototype: ctx.vm.array_prototype,
    });

    // Set `.index` (UTF-16 code unit index) and `.input` properties.
    let index_key = PropertyKey::String(ctx.intern("index"));
    let index_utf16 = byte_offset_to_utf16(&subject, m.start());
    #[allow(clippy::cast_precision_loss)]
    ctx.get_object_mut(arr_id).properties.push((
        index_key,
        Property::data(JsValue::Number(index_utf16 as f64)),
    ));
    let input_key = PropertyKey::String(ctx.intern("input"));
    let input_str = ctx.intern(&subject);
    ctx.get_object_mut(arr_id)
        .properties
        .push((input_key, Property::data(JsValue::String(input_str))));

    Ok(JsValue::Object(arr_id))
}

pub(super) fn native_regexp_to_string(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let JsValue::Object(obj_id) = this else {
        return Err(VmError::type_error(
            "RegExp.prototype.toString called on non-object",
        ));
    };
    let obj = ctx.get_object(obj_id);
    let ObjectKind::RegExp { pattern, flags, .. } = &obj.kind else {
        return Err(VmError::type_error("not a RegExp"));
    };
    let pat_str = ctx.vm.strings.get_utf8(*pattern);
    let flags_str = ctx.vm.strings.get_utf8(*flags);
    let result = format!("/{pat_str}/{flags_str}");
    let id = ctx.intern(&result);
    Ok(JsValue::String(id))
}
