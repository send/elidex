//! `JSON.stringify` (ECMA-262 §25.5.4) — the serializer half.
//!
//! Also hosts [`stringify_for_structured_shortcut`]: the structured-serialize
//! shortcut mode behind worker / SW `postMessage` and `history.state`, which
//! fails on a `Date` instead of flattening it through `toJSON`.

use std::fmt::Write;

use super::super::coerce_format::{collect_own_keys_es_order, write_number_es};
use super::super::shape::{PropertyAttrs, ROOT_SHAPE};
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, StringId, VmError,
};
use super::MAX_JSON_DEPTH;

/// Serialization state for `JSON.stringify`.
struct JsonSerializer {
    output: String,
    /// Object stack for circular reference detection.  Also serves as the
    /// depth counter for recursion-limit enforcement.
    stack: Vec<ObjectId>,
    /// Current indentation prefix.
    indent: String,
    /// One level of indentation (derived from `space` argument).
    gap: String,
    /// Replacer function, if any.
    replacer_fn: Option<ObjectId>,
    /// Replacer property list (when replacer is an Array).
    property_list: Option<Vec<StringId>>,
    /// Cached `StringId` for `"toJSON"` (interned once at construction).
    to_json_key: StringId,
    /// Reusable buffer for array index → string conversion.
    index_buf: String,
    /// Structured-serialize shortcut mode: fail on a `Date` instead of rendering
    /// it through `toJSON` — see [`stringify_for_structured_shortcut`]. `false`
    /// for `JSON.stringify` / `Response.json()`, where an ISO String is correct.
    reject_date: bool,
}

impl JsonSerializer {
    /// §25.5.4.2 SerializeJSONProperty — returns `true` if a value was written,
    /// `false` if the value is `undefined`/Symbol/Function (i.e. should be skipped).
    fn serialize_property(
        &mut self,
        ctx: &mut NativeContext<'_>,
        value: JsValue,
        holder: ObjectId,
        key: JsValue,
    ) -> Result<bool, VmError> {
        // Normalize Empty (sparse hole) to Undefined so user code never sees it.
        let mut val = value.or_undefined();

        // Structured-serialize shortcut only: a Date has no faithful JSON form, so
        // fail BEFORE the step-2 `toJSON` hook would flatten it to an ISO String.
        // Sitting inside the walk — on the value `SerializeJSONProperty` actually
        // observes — means a Date reached through an accessor is caught too, the
        // walk's own depth cap still fires first on a deep payload, and a user
        // exception encountered earlier (a throwing getter / `toJSON`) still
        // propagates ahead of this. See [`stringify_for_structured_shortcut`].
        if self.reject_date {
            if let JsValue::Object(obj_id) = val {
                if matches!(ctx.get_object(obj_id).kind, ObjectKind::Date(_)) {
                    return Err(VmError::range_error(
                        "a Date is not representable by the JSON serialization shortcut",
                    ));
                }
            }
        }

        // Step 2: If value is Object, check for toJSON.
        if let JsValue::Object(obj_id) = val {
            let to_json_pk = PropertyKey::String(self.to_json_key);
            if let Some(JsValue::Object(to_json_obj)) =
                ctx.try_get_property_value(obj_id, to_json_pk)?
            {
                if ctx.get_object(to_json_obj).kind.is_callable() {
                    val = ctx.call_function(to_json_obj, val, &[key])?;
                }
            }
        }

        // Step 3: If replacer function, call it.
        if let Some(replacer) = self.replacer_fn {
            val = ctx.call_function(replacer, JsValue::Object(holder), &[key, val])?;
        }

        // Step 4 (§25.5.4.2 SerializeJSONProperty): unwrap wrapper objects.
        // Only an Object can be a wrapper, so guard here — a primitive leaf (the
        // common case) skips the out-of-line call entirely. `unwrap_wrapper_value`
        // is kept out of this *recursive* frame so its `? ToNumber` / `? ToString`
        // locals do not enlarge every nesting level — the JSON depth cap is tuned
        // to the per-level frame size (`MAX_JSON_DEPTH`).
        if let JsValue::Object(obj_id) = val {
            val = unwrap_wrapper_value(ctx, val, obj_id)?;
        }

        // Step 5+: Type-specific serialization.
        match val {
            JsValue::Null => {
                self.output.push_str("null");
                Ok(true)
            }
            JsValue::Boolean(true) => {
                self.output.push_str("true");
                Ok(true)
            }
            JsValue::Boolean(false) => {
                self.output.push_str("false");
                Ok(true)
            }
            JsValue::Number(n) => {
                if n.is_finite() {
                    write_number_es(n, &mut self.output);
                } else {
                    // NaN, Infinity, -Infinity → "null"
                    self.output.push_str("null");
                }
                Ok(true)
            }
            JsValue::String(id) => {
                let units = ctx.get_u16(id);
                quote(units, &mut self.output);
                Ok(true)
            }
            JsValue::BigInt(_) => Err(VmError::type_error("Do not know how to serialize a BigInt")),
            JsValue::Object(obj_id) => {
                if ctx.get_object(obj_id).kind.is_callable() {
                    return Ok(false); // skip
                }
                let is_array = matches!(ctx.get_object(obj_id).kind, ObjectKind::Array { .. });
                if is_array {
                    self.serialize_array(ctx, obj_id)
                } else {
                    self.serialize_object(ctx, obj_id)
                }
            }
            // undefined, Symbol → skip (holes normalized to Undefined above)
            JsValue::Empty | JsValue::Undefined | JsValue::Symbol(_) => Ok(false),
        }
    }

    /// §25.5.4.6 SerializeJSONArray
    fn serialize_array(
        &mut self,
        ctx: &mut NativeContext<'_>,
        obj_id: ObjectId,
    ) -> Result<bool, VmError> {
        if self.stack.len() >= MAX_JSON_DEPTH {
            return Err(VmError::range_error("Maximum JSON nesting depth exceeded"));
        }
        // Circular reference check.
        if self.stack.contains(&obj_id) {
            return Err(VmError::type_error("Converting circular structure to JSON"));
        }
        self.stack.push(obj_id);

        let len = match &ctx.get_object(obj_id).kind {
            ObjectKind::Array { elements } => elements.len(),
            _ => 0,
        };

        self.output.push('[');
        if len == 0 {
            self.output.push(']');
            self.stack.pop();
            return Ok(true);
        }

        let has_gap = !self.gap.is_empty();
        let prev_indent = if has_gap {
            let prev = self.indent.clone();
            self.indent.push_str(&self.gap);
            prev
        } else {
            String::new()
        };

        for i in 0..len {
            if i > 0 {
                self.output.push(',');
            }
            if has_gap {
                self.output.push('\n');
                self.output.push_str(&self.indent);
            }

            // Read element via Get semantics (includes prototype lookup for holes).
            #[allow(clippy::cast_precision_loss)]
            let elem = ctx
                .vm
                .get_element(JsValue::Object(obj_id), JsValue::Number(i as f64))?;

            // Only intern the index string when toJSON or replacer needs it.
            // This avoids permanently growing the StringPool for large arrays.
            let needs_key = self.replacer_fn.is_some() || matches!(elem, JsValue::Object(_));
            let key = if needs_key {
                self.index_buf.clear();
                let _ = write!(self.index_buf, "{i}");
                JsValue::String(ctx.intern(&self.index_buf))
            } else {
                JsValue::Undefined
            };
            let wrote = self.serialize_property(ctx, elem, obj_id, key)?;
            if !wrote {
                // undefined/Symbol/Function in array → "null"
                self.output.push_str("null");
            }
        }

        if has_gap {
            self.indent = prev_indent;
            self.output.push('\n');
            self.output.push_str(&self.indent);
        }
        self.output.push(']');

        self.stack.pop();
        Ok(true)
    }

    /// §25.5.4.5 SerializeJSONObject
    fn serialize_object(
        &mut self,
        ctx: &mut NativeContext<'_>,
        obj_id: ObjectId,
    ) -> Result<bool, VmError> {
        if self.stack.len() >= MAX_JSON_DEPTH {
            return Err(VmError::range_error("Maximum JSON nesting depth exceeded"));
        }
        // Circular reference check.
        if self.stack.contains(&obj_id) {
            return Err(VmError::type_error("Converting circular structure to JSON"));
        }
        self.stack.push(obj_id);

        // Collect keys.
        let keys: Vec<StringId> = if let Some(ref pl) = self.property_list {
            // Replacer array: §25.5.4.5 step 5.a uses PropertyList as-is.
            // Values are retrieved via Get(holder, key), so non-enumerable
            // and inherited properties are included if present in the list.
            pl.clone()
        } else {
            // §25.5.4.5 step 5: EnumerableOwnPropertyNames / OrdinaryOwnPropertyKeys.
            // Array-index keys come first in ascending numeric order, then
            // other string keys in insertion order.
            collect_own_keys_es_order(ctx.vm, obj_id)?
        };

        self.output.push('{');

        let has_gap = !self.gap.is_empty();
        let prev_indent = if has_gap {
            let prev = self.indent.clone();
            self.indent.push_str(&self.gap);
            prev
        } else {
            String::new()
        };

        let mut first = true;
        for key_sid in keys {
            let key_pk = PropertyKey::String(key_sid);
            let val = ctx.get_property_value(obj_id, key_pk)?;

            // Try to serialize. If skip, don't emit this key at all.
            let before_len = self.output.len();
            if !first {
                self.output.push(',');
            }
            if has_gap {
                self.output.push('\n');
                self.output.push_str(&self.indent);
            }
            // Write key.
            let key_units = ctx.get_u16(key_sid);
            quote(key_units, &mut self.output);
            self.output.push(':');
            if has_gap {
                self.output.push(' ');
            }

            let wrote = self.serialize_property(ctx, val, obj_id, JsValue::String(key_sid))?;
            if wrote {
                first = false;
            } else {
                // Revert: this property produces undefined/Symbol/Function.
                self.output.truncate(before_len);
            }
        }

        if !first && has_gap {
            self.indent = prev_indent;
            self.output.push('\n');
            self.output.push_str(&self.indent);
        } else if first && has_gap {
            self.indent = prev_indent;
        }
        self.output.push('}');

        self.stack.pop();
        Ok(true)
    }
}

/// JSON string escaping: surround with `"` and escape special characters.
/// Operates on WTF-16 code units to correctly handle lone surrogates.
fn quote(units: &[u16], output: &mut String) {
    output.reserve(units.len() + 2);
    output.push('"');
    let mut i = 0;
    while i < units.len() {
        let c = units[i];
        match c {
            0x08 => output.push_str("\\b"),
            0x09 => output.push_str("\\t"),
            0x0A => output.push_str("\\n"),
            0x0C => output.push_str("\\f"),
            0x0D => output.push_str("\\r"),
            0x22 => output.push_str("\\\""),
            0x5C => output.push_str("\\\\"),
            0x00..=0x1F => {
                // Other control characters → \uXXXX
                let _ = write!(output, "\\u{c:04x}");
            }
            // Surrogate pair handling
            0xD800..=0xDBFF => {
                if let Some(&lo) = units.get(i + 1) {
                    if (0xDC00..=0xDFFF).contains(&lo) {
                        // Valid surrogate pair → decode to char and emit UTF-8.
                        let cp =
                            0x10000 + ((u32::from(c) - 0xD800) << 10) + (u32::from(lo) - 0xDC00);
                        if let Some(ch) = char::from_u32(cp) {
                            output.push(ch);
                        }
                        i += 2;
                        continue;
                    }
                }
                // Lone high surrogate → \uXXXX
                let _ = write!(output, "\\u{c:04x}");
            }
            0xDC00..=0xDFFF => {
                // Lone low surrogate → \uXXXX
                let _ = write!(output, "\\u{c:04x}");
            }
            _ => {
                // BMP character → emit as UTF-8.
                if let Some(ch) = char::from_u32(u32::from(c)) {
                    output.push(ch);
                }
            }
        }
        i += 1;
    }
    output.push('"');
}

/// Entry point for `JSON.stringify(value, replacer?, space?)`.
pub(in crate::vm) fn native_json_stringify(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let value = args.first().copied().unwrap_or(JsValue::Undefined);
    let replacer_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let space_arg = args.get(2).copied().unwrap_or(JsValue::Undefined);
    match stringify_to_string(ctx, value, replacer_arg, space_arg)? {
        Some(s) => Ok(JsValue::String(ctx.intern(&s))),
        None => Ok(JsValue::Undefined),
    }
}

/// JSON-serialize `value` into an owned Rust `String` (ECMA-262 §25.5.4
/// stringify core — `replacer` / `space` honored), **without interning** the
/// result into the permanent `StringPool`. `Ok(None)` mirrors `JSON.stringify`
/// yielding `undefined` (a top-level function / symbol / `undefined`).
///
/// [`native_json_stringify`] wraps this and interns the result into a live JS
/// string. Cross-thread IPC paths (`Worker.postMessage` /
/// `DedicatedWorkerGlobalScope.postMessage`, WHATWG HTML §10.2.6.3 /
/// §10.2.1.2) call
/// this directly: their JSON blobs are transient (copied straight into the
/// crossbeam channel), so interning them would grow the GC-less `StringPool`
/// unboundedly for chatty workers.
pub(in crate::vm) fn stringify_to_string(
    ctx: &mut NativeContext<'_>,
    value: JsValue,
    replacer_arg: JsValue,
    space_arg: JsValue,
) -> Result<Option<String>, VmError> {
    stringify_impl(ctx, value, replacer_arg, space_arg, false)
}

/// [`stringify_to_string`] in **structured-serialize shortcut** mode: identical,
/// except that a `Date` anywhere the JSON walk reaches is a hard failure instead
/// of an ISO String.
///
/// The JSON shortcut stands in for StructuredSerialize on two paths — worker / SW
/// `postMessage` ([`super::super::host::worker_scope::serialize_message`]) and
/// `history.state`
/// ([`super::super::host::structured_serialize::structured_serialize_for_storage`]). A
/// `Date` is [Serializable], so real structured clone round-trips it, but
/// `JSON.stringify` flattens it through `toJSON` (ECMA-262 §21.4.4.37) into an ISO
/// String — the peer's `JSON.parse` / `StructuredDeserialize` would then hand back
/// a **String** where structured clone hands back a Date. Every *other* way the
/// shortcut departs from structured clone fails **loudly** (a `BigInt`, a cycle,
/// and the depth cap all throw), which is why only a Date needs an explicit arm.
///
/// The check lives **inside** the walk, on the value `SerializeJSONProperty`
/// observes and before the `toJSON` hook. That placement is what makes it correct
/// (Codex R5): a Date returned by an **accessor** is seen, the walk's own **depth
/// cap** still fires first on a deep payload, and a **user exception** reached
/// earlier (a throwing getter / `toJSON`) still propagates ahead of it. A pre-scan
/// outside the walk duplicates the traversal and gets all three wrong.
///
/// The failure is a non-`ThrowValue` [`VmError`], so each caller's existing "cannot
/// represent this value" branch handles it unchanged: worker → `DataCloneError`,
/// history → degrade to `None`. Faithful encoding lands with
/// `#11-worker-structured-serialize` /
/// `#11-history-state-structured-serialize-fidelity`, which replace this shortcut
/// wholesale.
pub(in crate::vm) fn stringify_for_structured_shortcut(
    ctx: &mut NativeContext<'_>,
    value: JsValue,
) -> Result<Option<String>, VmError> {
    stringify_impl(ctx, value, JsValue::Undefined, JsValue::Undefined, true)
}

fn stringify_impl(
    ctx: &mut NativeContext<'_>,
    value: JsValue,
    replacer_arg: JsValue,
    space_arg: JsValue,
    reject_date: bool,
) -> Result<Option<String>, VmError> {
    // Step 4: Process replacer.
    let mut replacer_fn = None;
    let mut property_list = None;
    if let JsValue::Object(obj_id) = replacer_arg {
        if ctx.get_object(obj_id).kind.is_callable() {
            replacer_fn = Some(obj_id);
        } else if matches!(ctx.get_object(obj_id).kind, ObjectKind::Array { .. }) {
            // §25.5.4 step 5.b.ii: build the PropertyList inclusion set. `len` is
            // read once (step 2, LengthOfArrayLike), but each element is fetched
            // FRESH per iteration via `? Get(replacer, ToString(k))` (step 4.b) —
            // matching `serialize_array`, NOT a cloned snapshot. The 4.f.i
            // coercion below now runs a user-overridden `toString` / `valueOf`,
            // which can mutate a later replacer index; a snapshot would keep the
            // stale value and diverge from spec.
            let len = match &ctx.get_object(obj_id).kind {
                ObjectKind::Array { elements } => elements.len(),
                _ => 0,
            };
            let mut list = Vec::new();
            for k in 0..len {
                #[allow(clippy::cast_precision_loss)]
                let elem = ctx
                    .vm
                    .get_element(JsValue::Object(obj_id), JsValue::Number(k as f64))?;
                // step 4.d/4.e/4.f.i: a String / Number element — including a
                // `[[StringData]]` OR `[[NumberData]]` wrapper — becomes an
                // included key via `? ToString(propertyValue)` (BOTH wrapper kinds
                // → ToString, honoring an override), not a direct slot read.
                let sid = match elem {
                    JsValue::String(s) => s,
                    JsValue::Number(_) => ctx.to_string_val(elem)?,
                    JsValue::Object(oid) => match ctx.get_object(oid).kind {
                        ObjectKind::NumberWrapper(_) | ObjectKind::StringWrapper(_) => {
                            ctx.to_string_val(elem)?
                        }
                        _ => continue,
                    },
                    _ => continue,
                };
                if !list.contains(&sid) {
                    list.push(sid);
                }
            }
            property_list = Some(list);
        }
    }

    // Step 6-9: Process space (fallible — a boxed `space` whose overridden
    // `valueOf` / `toString` throws propagates the abrupt completion).
    let gap = compute_gap(ctx, space_arg)?;

    // Build wrapper object: { "": value } (§25.5.4 steps 10-11)
    let wrapper_id = ctx.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        storage: PropertyStorage::shaped(ROOT_SHAPE),
        prototype: ctx.vm.object_prototype,
        extensible: true,
    });
    let empty_key = ctx.vm.well_known.empty;
    ctx.vm.define_shaped_property(
        wrapper_id,
        PropertyKey::String(empty_key),
        PropertyValue::Data(value),
        PropertyAttrs::DATA,
    );

    let to_json_key = ctx.vm.well_known.to_json;
    let mut serializer = JsonSerializer {
        output: String::with_capacity(128),
        stack: Vec::new(),
        indent: String::new(),
        gap,
        replacer_fn,
        property_list,
        to_json_key,
        index_buf: String::with_capacity(8),
        reject_date,
    };

    let wrote =
        serializer.serialize_property(ctx, value, wrapper_id, JsValue::String(empty_key))?;

    if wrote {
        Ok(Some(serializer.output))
    } else {
        Ok(None)
    }
}

/// §25.5.4.2 SerializeJSONProperty step 4: unwrap a primitive-wrapper `value`
/// to the primitive the serializer emits.
///
/// 4.b `[[NumberData]]` → `? ToNumber(value)` and 4.c `[[StringData]]` →
/// `? ToString(value)` route through the spec AO so a user-overridden `valueOf`
/// / `toString` / `@@toPrimitive` is honored; 4.d `[[BooleanData]]` / 4.e
/// `[[BigIntData]]` read the slot directly (no override path). A non-wrapper
/// value is returned unchanged.
///
/// Kept `#[inline(never)]` and out of the *recursive* `serialize_property`
/// frame so the coercion's locals do not enlarge every nesting level's stack
/// frame — the JSON depth cap (`MAX_JSON_DEPTH`) is tuned to that per-level size.
/// The caller guards on `JsValue::Object` and passes the extracted `obj_id`.
#[inline(never)]
fn unwrap_wrapper_value(
    ctx: &mut NativeContext<'_>,
    val: JsValue,
    obj_id: ObjectId,
) -> Result<JsValue, VmError> {
    Ok(match ctx.get_object(obj_id).kind {
        ObjectKind::NumberWrapper(_) => JsValue::Number(ctx.to_number(val)?),
        ObjectKind::StringWrapper(_) => JsValue::String(ctx.to_string_val(val)?),
        ObjectKind::BooleanWrapper(b) => JsValue::Boolean(b),
        ObjectKind::BigIntWrapper(id) => JsValue::BigInt(id),
        _ => val,
    })
}

/// Compute the `gap` string from the `space` argument (§25.5.4 steps 6-9).
///
/// Fallible because §25.5.4 step 6 unwraps a Number/String wrapper via
/// `? ToNumber` / `? ToString`, which invoke a user-overridden `valueOf` /
/// `toString` and can throw (abrupt completion must propagate).
fn compute_gap(ctx: &mut NativeContext<'_>, space: JsValue) -> Result<String, VmError> {
    // Step 6: a Number/String wrapper unwraps through the spec AO (6.a
    // `? ToNumber` / 6.b `? ToString`), honoring a user override — not a direct
    // slot read.
    let space = match space {
        JsValue::Object(obj_id) => match ctx.get_object(obj_id).kind {
            ObjectKind::NumberWrapper(_) => JsValue::Number(ctx.to_number(space)?),
            ObjectKind::StringWrapper(_) => JsValue::String(ctx.to_string_val(space)?),
            _ => space,
        },
        other => other,
    };

    Ok(match space {
        JsValue::Number(n) => {
            #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
            let count = n.clamp(0.0, 10.0) as usize;
            " ".repeat(count)
        }
        JsValue::String(id) => {
            let units = ctx.get_u16(id);
            let len = units.len().min(10);
            String::from_utf16_lossy(&units[..len])
        }
        _ => String::new(),
    })
}
