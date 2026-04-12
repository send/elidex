//! Native RegExp.prototype methods.
//!
//! All regex matching operates on WTF-16 code unit slices (the VM's native
//! string representation) via `regress::Regex::find_from_utf16`. This avoids
//! UTF-8 ↔ UTF-16 round-trip conversions and ensures that `lastIndex`,
//! `.index`, and capture ranges are correct UTF-16 code unit indices.

use super::value::{
    JsValue, NativeContext, Object, ObjectKind, PropertyKey, PropertyStorage, VmError,
};

/// Helper: intern a WTF-16 sub-slice.
fn intern_u16_range(
    ctx: &mut NativeContext<'_>,
    units: &[u16],
    range: &std::ops::Range<usize>,
) -> super::value::StringId {
    ctx.vm.strings.intern_utf16(&units[range.start..range.end])
}

/// Run a regex match on a WTF-16 subject, handling lastIndex for g/y flags.
pub(super) fn run_regexp(
    ctx: &mut NativeContext<'_>,
    obj_id: super::value::ObjectId,
    subject: &[u16],
) -> Result<Option<regress::Match>, VmError> {
    // Extract flags.
    let (is_global, is_sticky) = {
        let obj = ctx.get_object(obj_id);
        if let ObjectKind::RegExp { flags, .. } = &obj.kind {
            let f = ctx.vm.strings.get_utf8(*flags);
            (f.contains('g'), f.contains('y'))
        } else {
            return Err(VmError::type_error("not a RegExp"));
        }
    };
    let uses_last_index = is_global || is_sticky;

    // Read lastIndex (already a UTF-16 code unit index).
    let start = if uses_last_index {
        let last_index_key = PropertyKey::String(ctx.vm.well_known.last_index);
        let obj = ctx.get_object(obj_id);
        let mut idx = 0usize;
        if let Some((super::value::PropertyValue::Data(JsValue::Number(n)), _)) =
            obj.storage.get(last_index_key, &ctx.vm.shapes)
        {
            // ToLength: NaN/negative → 0, Infinity → subject.len().
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            if *n > 0.0 {
                if n.is_finite() {
                    idx = (n.trunc() as usize).min(subject.len());
                } else {
                    idx = subject.len();
                }
            }
        }
        idx
    } else {
        0
    };

    // Run the compiled regex on WTF-16 data.
    let found = {
        let obj = ctx.get_object(obj_id);
        let ObjectKind::RegExp { ref compiled, .. } = obj.kind else {
            return Err(VmError::type_error("not a RegExp"));
        };
        let m = compiled.find_from_utf16(subject, start).next();
        if is_sticky {
            m.filter(|m| m.start() == start)
        } else {
            m
        }
    };

    // Update lastIndex (UTF-16 code unit index, no conversion needed).
    if uses_last_index {
        let new_idx = found.as_ref().map_or(0, regress::Match::end);
        super::natives_string::set_regexp_last_index(ctx, obj_id, new_idx);
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
    let subject = ctx.vm.strings.get(sid).to_vec();

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
    let subject = ctx.vm.strings.get(sid).to_vec();

    let Some(m) = run_regexp(ctx, obj_id, &subject)? else {
        return Ok(JsValue::Null);
    };

    // Build result array: [full_match, ...groups]
    let mut elements = vec![JsValue::String(intern_u16_range(ctx, &subject, &m.range))];
    for group in &m.captures {
        match group {
            Some(range) => elements.push(JsValue::String(intern_u16_range(ctx, &subject, range))),
            None => elements.push(JsValue::Undefined),
        }
    }

    let arr_id = ctx.alloc_object(Object {
        kind: ObjectKind::Array { elements },
        storage: PropertyStorage::shaped(super::shape::ROOT_SHAPE),
        prototype: ctx.vm.array_prototype,
        extensible: true,
    });

    // .index is already a UTF-16 code unit index (no conversion).
    let index_key = PropertyKey::String(ctx.vm.well_known.index);
    #[allow(clippy::cast_precision_loss)]
    ctx.vm.define_shaped_property(
        arr_id,
        index_key,
        super::value::PropertyValue::Data(JsValue::Number(m.start() as f64)),
        super::shape::PropertyAttrs::DATA,
    );
    let input_key = PropertyKey::String(ctx.vm.well_known.input);
    ctx.vm.define_shaped_property(
        arr_id,
        input_key,
        super::value::PropertyValue::Data(JsValue::String(sid)),
        super::shape::PropertyAttrs::DATA,
    );

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
