//! `CountQueuingStrategy` + `ByteLengthQueuingStrategy`
//! (WHATWG Streams §6.1 / §6.2).
//!
//! Each is a regular constructor that produces an Ordinary
//! instance with a `highWaterMark` own property and a `size`
//! method on the prototype.  The stream constructor then reads
//! them via the same path it uses for ad-hoc `{highWaterMark,
//! size}` objects.

use super::super::super::shape::{self, PropertyAttrs};
use super::super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, VmError,
};
use super::super::super::{NativeFn, VmInner};

/// Shared body for `new CountQueuingStrategy({highWaterMark})` and
/// `new ByteLengthQueuingStrategy({highWaterMark})`.  Spec §6.1.2 /
/// §6.2.2 — both ctors read the `highWaterMark` from the
/// init-object verbatim (no coercion / validation; the stream
/// constructor's normaliser handles that).
fn extract_strategy_high_water_mark(
    ctx: &mut NativeContext<'_>,
    init_arg: JsValue,
    iface: &str,
) -> Result<JsValue, VmError> {
    // WebIDL dictionary conversion (§3.2.17):
    //   - `undefined` / `null` → empty dictionary (then required
    //     member missing surfaces "highWaterMark is required").
    //   - non-Object primitive (Number / String / Boolean / …)
    //     → TypeError "init must be an object" (Copilot R10
    //     finding — `new CountQueuingStrategy(1)` should reject
    //     at the dict-conversion stage, not the missing-member
    //     stage).
    //   - Object → look up `highWaterMark`; absent or undefined
    //     → "highWaterMark is required".
    let lookup_value = match init_arg {
        JsValue::Undefined | JsValue::Null => None,
        JsValue::Object(obj_id) => {
            let key = PropertyKey::String(ctx.vm.well_known.high_water_mark);
            match super::super::super::coerce::get_property(ctx.vm, obj_id, key) {
                Some(prop) => match ctx.vm.resolve_property(prop, JsValue::Object(obj_id))? {
                    // An explicitly-present `undefined` is treated
                    // as missing per WebIDL dict member rules
                    // (Copilot R10 — symmetry with omitted member).
                    JsValue::Undefined => None,
                    other => Some(other),
                },
                None => None,
            }
        }
        _ => {
            return Err(VmError::type_error(format!(
                "Failed to construct '{iface}': init must be an object"
            )));
        }
    };
    lookup_value.ok_or_else(|| {
        VmError::type_error(format!(
            "Failed to construct '{iface}': init.highWaterMark is required"
        ))
    })
}

fn install_high_water_mark_own(vm: &mut VmInner, inst_id: ObjectId, hwm: JsValue) {
    let key = PropertyKey::String(vm.well_known.high_water_mark);
    // WebIDL readonly attribute (Streams §6.1.4 / §6.2.4) — uses
    // `WEBIDL_RO` so user code can't mutate
    // `strategy.highWaterMark` after construction (Copilot R10).
    vm.define_shaped_property(
        inst_id,
        key,
        PropertyValue::Data(hwm),
        PropertyAttrs::WEBIDL_RO,
    );
}

fn native_count_queuing_strategy_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if !ctx.is_construct() {
        return Err(VmError::type_error(
            "Failed to construct 'CountQueuingStrategy': Please use the 'new' operator",
        ));
    }
    let JsValue::Object(inst_id) = this else {
        unreachable!("ctor `this` always Object after `do_new`");
    };
    let init = args.first().copied().unwrap_or(JsValue::Undefined);
    let hwm = extract_strategy_high_water_mark(ctx, init, "CountQueuingStrategy")?;
    install_high_water_mark_own(ctx.vm, inst_id, hwm);
    Ok(JsValue::Object(inst_id))
}

fn native_byte_length_queuing_strategy_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if !ctx.is_construct() {
        return Err(VmError::type_error(
            "Failed to construct 'ByteLengthQueuingStrategy': Please use the 'new' operator",
        ));
    }
    let JsValue::Object(inst_id) = this else {
        unreachable!("ctor `this` always Object after `do_new`");
    };
    let init = args.first().copied().unwrap_or(JsValue::Undefined);
    let hwm = extract_strategy_high_water_mark(ctx, init, "ByteLengthQueuingStrategy")?;
    install_high_water_mark_own(ctx.vm, inst_id, hwm);
    Ok(JsValue::Object(inst_id))
}

/// `CountQueuingStrategy.prototype.size(_chunk)` — always returns 1.
fn native_count_queuing_strategy_size(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Number(1.0))
}

/// `ByteLengthQueuingStrategy.prototype.size(chunk)` — returns
/// `chunk.byteLength`.  Spec §6.2.4: returns the chunk's
/// `byteLength` IDL property if it has one; otherwise the
/// algorithm propagates whatever value (or undefined) the lookup
/// yields.
fn native_byte_length_queuing_strategy_size(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let chunk = args.first().copied().unwrap_or(JsValue::Undefined);
    let JsValue::Object(chunk_id) = chunk else {
        return Ok(JsValue::Undefined);
    };
    let key = PropertyKey::String(ctx.vm.well_known.byte_length);
    match super::super::super::coerce::get_property(ctx.vm, chunk_id, key) {
        Some(prop) => ctx.vm.resolve_property(prop, chunk),
        None => Ok(JsValue::Undefined),
    }
}

impl VmInner {
    /// Register `CountQueuingStrategy` + `ByteLengthQueuingStrategy`
    /// (WHATWG Streams §6.1 / §6.2).  Each is a regular constructor
    /// that produces an Ordinary instance with a `highWaterMark`
    /// own property and a `size` method on the prototype.  The
    /// stream constructor then reads them via the same path it
    /// uses for ad-hoc `{highWaterMark, size}` objects.
    pub(in crate::vm) fn register_queuing_strategy_globals(&mut self) {
        let object_proto = self
            .object_prototype
            .expect("register_queuing_strategy_globals called before register_prototypes");

        // CountQueuingStrategy
        let count_proto = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });
        self.install_native_method(
            count_proto,
            self.well_known.size,
            native_count_queuing_strategy_size as NativeFn,
            PropertyAttrs::METHOD,
        );
        self.count_queuing_strategy_prototype = Some(count_proto);
        let count_ctor = self.create_constructable_function(
            "CountQueuingStrategy",
            native_count_queuing_strategy_constructor,
        );
        let proto_key = PropertyKey::String(self.well_known.prototype);
        self.define_shaped_property(
            count_ctor,
            proto_key,
            PropertyValue::Data(JsValue::Object(count_proto)),
            PropertyAttrs::BUILTIN,
        );
        let ctor_key = PropertyKey::String(self.well_known.constructor);
        self.define_shaped_property(
            count_proto,
            ctor_key,
            PropertyValue::Data(JsValue::Object(count_ctor)),
            PropertyAttrs::METHOD,
        );
        self.globals.insert(
            self.well_known.count_queuing_strategy_global,
            JsValue::Object(count_ctor),
        );

        // ByteLengthQueuingStrategy
        let byte_proto = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });
        self.install_native_method(
            byte_proto,
            self.well_known.size,
            native_byte_length_queuing_strategy_size as NativeFn,
            PropertyAttrs::METHOD,
        );
        self.byte_length_queuing_strategy_prototype = Some(byte_proto);
        let byte_ctor = self.create_constructable_function(
            "ByteLengthQueuingStrategy",
            native_byte_length_queuing_strategy_constructor,
        );
        self.define_shaped_property(
            byte_ctor,
            proto_key,
            PropertyValue::Data(JsValue::Object(byte_proto)),
            PropertyAttrs::BUILTIN,
        );
        self.define_shaped_property(
            byte_proto,
            ctor_key,
            PropertyValue::Data(JsValue::Object(byte_ctor)),
            PropertyAttrs::METHOD,
        );
        self.globals.insert(
            self.well_known.byte_length_queuing_strategy_global,
            JsValue::Object(byte_ctor),
        );
    }
}
