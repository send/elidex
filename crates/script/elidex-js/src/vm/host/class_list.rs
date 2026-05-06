//! `DOMTokenList.prototype` intrinsic + `Element.classList` accessor
//! plumbing (WHATWG DOM §3.5 / §7.1).
//!
//! Thin binding to the engine-independent `elidex_dom_api::class_list`
//! handlers (10 entries: `add` / `remove` / `toggle` / `contains` /
//! `replace` / `value.get` / `value.set` / `length` / `item` /
//! `supports`).  Every native body in this file is a single
//! [`invoke_dom_api`] dispatch — no DOM mutation algorithms,
//! tokenization, or attribute parsing live here, per the CLAUDE.md
//! Layering mandate.
//!
//! ## Backing state
//!
//! [`ObjectKind::DOMTokenList`] carries the owner `Entity` inline
//! (`entity_bits`); there is no per-wrapper side table.  Every
//! accessor / method recovers the entity from `this`'s `ObjectKind`
//! and forwards through `invoke_dom_api`, which routes through the
//! `dom_registry` to the engine-independent handler.
//!
//! ## Identity
//!
//! `el.classList === el.classList` is preserved via
//! [`VmInner::class_list_wrapper_cache`] keyed by owner `Entity` —
//! a hit returns the same `ObjectId`, a miss allocates and inserts.
//! The cache is weak through the owner element wrapper (see
//! `gc/roots.rs` step `(e3)`); a dropped element releases its
//! classList wrapper in the same GC.
//!
//! ## Indexed-property exotic
//!
//! `tokens[i]` returns the `i`th token as a string (or `undefined`
//! when out of range), matching WHATWG §7.1 indexed getter
//! semantics.  Implemented in [`try_indexed_get`] and dispatched
//! from `ops_element::get_element` alongside the live-collection /
//! NamedNodeMap branches.

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{
    ArrayIterState, JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey,
    PropertyStorage, PropertyValue, VmError, ARRAY_ITER_KIND_VALUES,
};
use super::super::{NativeFn, VmInner};
use super::dom_bridge::{coerce_first_arg_to_string_id, invoke_dom_api};

use elidex_ecs::Entity;

impl VmInner {
    /// Allocate `DOMTokenList.prototype` chained to `Object.prototype`.
    /// Must run after `register_object_prototype`.
    pub(in crate::vm) fn register_dom_token_list_prototype(&mut self) {
        let obj_proto = self.object_prototype;
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: obj_proto,
            extensible: true,
        });
        self.dom_token_list_prototype = Some(proto_id);

        // `length` / `value` accessors.  `length` is RO; `value` is RW.
        self.install_accessor_pair(
            proto_id,
            self.well_known.length,
            native_class_list_length_get,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.value,
            native_class_list_value_get,
            Some(native_class_list_value_set),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );

        // Methods.  `toString` is the WHATWG DOM §7.1
        // `stringifier;` IDL declaration — `String(el.classList)`
        // and `el.classList + ''` must return the underlying class
        // string (same as the `value` accessor).  Install as an
        // ordinary method that re-dispatches to `classList.value.get`.
        for (name_sid, func) in [
            (self.well_known.item, native_class_list_item as NativeFn),
            (self.well_known.contains, native_class_list_contains),
            (self.well_known.add, native_class_list_add),
            (self.well_known.remove, native_class_list_remove),
            (self.well_known.toggle, native_class_list_toggle),
            (self.well_known.replace, native_class_list_replace),
            (self.well_known.supports, native_class_list_supports),
            (
                self.well_known.to_string_method,
                native_class_list_to_string,
            ),
        ] {
            self.install_native_method(proto_id, name_sid, func, shape::PropertyAttrs::METHOD);
        }

        // `[Symbol.iterator]` — values iterator over the token strings.
        let iter_fn = self.create_native_function("[Symbol.iterator]", native_class_list_iterator);
        let iter_key = PropertyKey::Symbol(self.well_known_symbols.iterator);
        self.define_shaped_property(
            proto_id,
            iter_key,
            PropertyValue::Data(JsValue::Object(iter_fn)),
            shape::PropertyAttrs::METHOD,
        );
    }

    /// Allocate a `DOMTokenList` wrapper for `owner`, caching by
    /// `owner` so `el.classList === el.classList` (WHATWG DOM §3.5).
    pub(crate) fn alloc_or_cached_class_list(&mut self, owner: Entity) -> ObjectId {
        if let Some(&id) = self.class_list_wrapper_cache.get(&owner) {
            return id;
        }
        let proto = self
            .dom_token_list_prototype
            .expect("alloc_or_cached_class_list before register_dom_token_list_prototype");
        let id = self.alloc_object(Object {
            kind: ObjectKind::DOMTokenList {
                entity_bits: owner.to_bits().get(),
            },
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(proto),
            extensible: false,
        });
        self.class_list_wrapper_cache.insert(owner, id);
        id
    }
}

// ---------------------------------------------------------------------------
// Post-unbind tolerance
// ---------------------------------------------------------------------------

/// `DOMTokenList` wrappers are plain JS objects, so user code can
/// retain `el.classList` across a `Vm::unbind()` boundary; calling
/// [`invoke_dom_api`] in that state panics at the
/// `HostData::with_session_and_dom` assert.  Each native checks
/// `ctx.host_if_bound()` first and returns a safe default
/// (length=0 / value="" / contains=false / item=null / mutations
/// no-op).  Mirrors `Attr` / `NamedNodeMap` post-unbind handling.
fn vm_is_bound(vm: &super::super::VmInner) -> bool {
    vm.host_data
        .as_deref()
        .is_some_and(super::super::host_data::HostData::is_bound)
}

// ---------------------------------------------------------------------------
// Brand check
// ---------------------------------------------------------------------------

fn require_dom_token_list_receiver(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &'static str,
) -> Result<Entity, VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'DOMTokenList': Illegal invocation"
        )));
    };
    let ObjectKind::DOMTokenList { entity_bits } = ctx.vm.get_object(id).kind else {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'DOMTokenList': Illegal invocation"
        )));
    };
    Entity::from_bits(entity_bits).ok_or_else(|| {
        VmError::type_error(format!(
            "Failed to execute '{method}' on 'DOMTokenList': stale entity"
        ))
    })
}

// ---------------------------------------------------------------------------
// Natives — one-line invoke_dom_api dispatch each.
// ---------------------------------------------------------------------------

fn native_class_list_length_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_dom_token_list_receiver(ctx, this, "length")?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Number(0.0));
    }
    invoke_dom_api(ctx, "classList.length", entity, &[])
}

fn native_class_list_value_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_dom_token_list_receiver(ctx, this, "value")?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    invoke_dom_api(ctx, "classList.value.get", entity, &[])
}

fn native_class_list_value_set(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_dom_token_list_receiver(ctx, this, "value")?;
    let sid = coerce_first_arg_to_string_id(ctx, args)?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    invoke_dom_api(ctx, "classList.value.set", entity, &[JsValue::String(sid)])
}

fn native_class_list_item(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_dom_token_list_receiver(ctx, this, "item")?;
    let arg = args.first().copied().unwrap_or(JsValue::Undefined);
    // WebIDL §3.10.13 indexed-property getter (`unsigned long`) → ToUint32
    // (mod 2^32).  Without this, negative inputs like `tokens.item(-1)`
    // would reach the handler as negative `f64` and the float→usize
    // cast in `parse_array_index_u32` would map them to 0, returning
    // the first token instead of `null`.
    let idx = super::super::coerce::to_uint32(ctx.vm, arg)?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Null);
    }
    invoke_dom_api(
        ctx,
        "classList.item",
        entity,
        &[JsValue::Number(f64::from(idx))],
    )
}

fn native_class_list_contains(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_dom_token_list_receiver(ctx, this, "contains")?;
    let sid = coerce_first_arg_to_string_id(ctx, args)?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Boolean(false));
    }
    invoke_dom_api(ctx, "classList.contains", entity, &[JsValue::String(sid)])
}

/// `classList.add(...tokens)` — variadic.  Each token is added in
/// sequence; spec §7.1 step "Run the validation algorithm" runs once
/// per token, so a single mid-list InvalidCharacterError aborts the
/// remaining adds.  Implemented by looping per arg through the
/// single-token handler.
fn native_class_list_add(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_dom_token_list_receiver(ctx, this, "add")?;
    // Coerce every arg up front so the only `TypeError` that can
    // surface — `ToString(Symbol)` — is still observable post-unbind.
    // Handler-side `SyntaxError` / `InvalidCharacterError` checks
    // are gated by the bound-state guard below: post-unbind callers
    // get a silent no-op (no DOM exists to mutate or validate
    // against), matching the rest of the post-unbind tolerance
    // contract (NamedNodeMap mutators / Attr setters etc.).
    let mut sids = Vec::with_capacity(args.len());
    for arg in args {
        sids.push(super::super::coerce::to_string(ctx.vm, *arg)?);
    }
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    for sid in sids {
        invoke_dom_api(ctx, "classList.add", entity, &[JsValue::String(sid)])?;
    }
    Ok(JsValue::Undefined)
}

fn native_class_list_remove(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_dom_token_list_receiver(ctx, this, "remove")?;
    // Same coerce-then-gate pattern as `add`: ToString TypeError
    // surfaces regardless of bound state; handler validation only
    // runs when bound.
    let mut sids = Vec::with_capacity(args.len());
    for arg in args {
        sids.push(super::super::coerce::to_string(ctx.vm, *arg)?);
    }
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    for sid in sids {
        invoke_dom_api(ctx, "classList.remove", entity, &[JsValue::String(sid)])?;
    }
    Ok(JsValue::Undefined)
}

fn native_class_list_toggle(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_dom_token_list_receiver(ctx, this, "toggle")?;
    let token = args.first().copied().unwrap_or(JsValue::Undefined);
    let token_sid = super::super::coerce::to_string(ctx.vm, token)?;
    // Optional `force` (ToBoolean per WebIDL).  WHATWG §7.1 step 4
    // distinguishes "force given" (apply ToBoolean) vs "force not
    // given" (flip current state).  `args.get(1) == Some(Undefined)`
    // is the *given-as-undefined* case → ToBoolean(undefined) = false.
    // `JsValue::Empty` is the internal sparse-array hole sentinel and
    // is treated as "not given" because user code can never observe
    // it; the call frame substitutes `Undefined` for missing
    // positional args.
    let mut vm_args: Vec<JsValue> = Vec::with_capacity(2);
    vm_args.push(JsValue::String(token_sid));
    if let Some(&force) = args.get(1) {
        if !matches!(force, JsValue::Empty) {
            let b = super::super::coerce::to_boolean(ctx.vm, force);
            vm_args.push(JsValue::Boolean(b));
        }
    }
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Boolean(false));
    }
    invoke_dom_api(ctx, "classList.toggle", entity, &vm_args)
}

fn native_class_list_replace(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_dom_token_list_receiver(ctx, this, "replace")?;
    let old_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let new_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let old_sid = super::super::coerce::to_string(ctx.vm, old_arg)?;
    let new_sid = super::super::coerce::to_string(ctx.vm, new_arg)?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Boolean(false));
    }
    invoke_dom_api(
        ctx,
        "classList.replace",
        entity,
        &[JsValue::String(old_sid), JsValue::String(new_sid)],
    )
}

/// `DOMTokenList.prototype.toString()` — the WebIDL `stringifier;`
/// declaration on WHATWG DOM §7.1 maps to a `toString` method that
/// returns the same string as the `value` accessor.  Without this,
/// `String(el.classList)` falls back to `Object.prototype.toString`
/// → `"[object Object]"`, breaking template-literal interpolation
/// and string-coercion semantics.
fn native_class_list_to_string(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_dom_token_list_receiver(ctx, this, "toString")?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    invoke_dom_api(ctx, "classList.value.get", entity, &[])
}

fn native_class_list_supports(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_dom_token_list_receiver(ctx, this, "supports")?;
    // Spec: classList.supports() always throws TypeError; the handler
    // returns a `DomApiErrorKind::TypeError` which maps to ECMA
    // `TypeError` in `dom_api_error_to_vm_error`.
    let sid = coerce_first_arg_to_string_id(ctx, args)?;
    if ctx.host_if_bound().is_none() {
        // Match the handler's message verbatim so bound and unbound
        // paths surface the same observable error.
        return Err(VmError::type_error(
            "classList.supports() is not supported for classList",
        ));
    }
    invoke_dom_api(ctx, "classList.supports", entity, &[JsValue::String(sid)])
}

/// `[Symbol.iterator]()` — return a values iterator over the token
/// list snapshot.  Uses the existing Array iterator infrastructure
/// (mirrors NamedNodeMap's `@@iterator`): build a JS array of token
/// strings from `classList.length` + per-index `classList.item`,
/// then alloc an `ArrayIterator` over it.
fn native_class_list_iterator(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_dom_token_list_receiver(ctx, this, "@@iterator")?;
    if ctx.host_if_bound().is_none() {
        // Empty iterator post-unbind — token snapshot is the empty list.
        let array_id = ctx.vm.create_array_object(Vec::new());
        let proto = ctx.vm.array_iterator_prototype;
        let iter_obj = ctx.vm.alloc_object(Object {
            kind: ObjectKind::ArrayIterator(ArrayIterState {
                array_id,
                index: 0,
                kind: ARRAY_ITER_KIND_VALUES,
            }),
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: proto,
            extensible: true,
        });
        return Ok(JsValue::Object(iter_obj));
    }
    let len_val = invoke_dom_api(ctx, "classList.length", entity, &[])?;
    let JsValue::Number(len) = len_val else {
        return Err(VmError::type_error(
            "DOMTokenList iterator: classList.length must be a number",
        ));
    };
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let len_usize = len as usize;
    let mut values = Vec::with_capacity(len_usize);
    for i in 0..len_usize {
        #[allow(clippy::cast_precision_loss)]
        let v = invoke_dom_api(ctx, "classList.item", entity, &[JsValue::Number(i as f64)])?;
        values.push(v);
    }
    let array_id = ctx.vm.create_array_object(values);
    let proto = ctx.vm.array_iterator_prototype;
    let iter_obj = ctx.vm.alloc_object(Object {
        kind: ObjectKind::ArrayIterator(ArrayIterState {
            array_id,
            index: 0,
            kind: ARRAY_ITER_KIND_VALUES,
        }),
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });
    Ok(JsValue::Object(iter_obj))
}

// ---------------------------------------------------------------------------
// Indexed property access — dispatched from `ops_element::get_element`.
// ---------------------------------------------------------------------------

/// Handle `tokens[i]` for a DOMTokenList receiver.  Returns
/// `Some(JsValue)` for a valid integer index — token string for
/// in-range, `Undefined` for out-of-range (the spec's indexed-getter
/// semantics mean an out-of-range integer key is *not* an own
/// property; falling through to the prototype chain would surface
/// inherited `Object.prototype` members at numeric indices, which
/// browsers do not).  Returns `None` for non-numeric /
/// non-canonical-string keys so `.length` / `.item` resolve via the
/// accessor chain.
///
/// Mirrors [`super::dom_collection::try_indexed_get`]'s shape so the
/// `ops_element::get_element` dispatch can branch by `ObjectKind`
/// without per-helper plumbing.
pub(crate) fn try_indexed_get(
    vm: &mut VmInner,
    id: ObjectId,
    key: JsValue,
) -> Option<Result<JsValue, VmError>> {
    let ObjectKind::DOMTokenList { entity_bits } = vm.get_object(id).kind else {
        return None;
    };
    let entity = Entity::from_bits(entity_bits)?;
    let idx_u32 = match key {
        JsValue::Number(n) if n.is_finite() => {
            // ECMA §7.1.21 canonical-numeric-index-string requires
            // an *exact* integer round-trip; an EPSILON test admits
            // values like `1.0000000000000002` whose `ToString` yields
            // a non-canonical key like `"1.0000000000000002"` rather
            // than `"1"`.  Use the same exact-integer pattern as
            // `ops::try_as_array_index` so non-canonical numbers fall
            // through to the regular property lookup.
            if !(n >= 0.0 && n <= f64::from(u32::MAX - 1)) {
                return None;
            }
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let idx = n as u32;
            if f64::from(idx) != n {
                return None;
            }
            idx
        }
        JsValue::String(sid) => {
            let units = vm.strings.get(sid);
            super::super::coerce_format::parse_array_index_u32(units)?
        }
        _ => return None,
    };
    // Dispatch through invoke_dom_api so error mapping is consistent
    // with the prototype-resident `item()`.  Allocate a NativeContext
    // inline; the bridge needs `&mut NativeContext` and `try_indexed_get`
    // is called from `ops_element::get_element` with a clean `&mut VmInner`.
    if !vm_is_bound(vm) {
        // Post-unbind: indexed access on a retained `el.classList`
        // wrapper returns `undefined` (matches Copilot R1 guidance —
        // out-of-range integer-typed indices fall to `Undefined`,
        // which is what an empty token list yields for every index).
        return Some(Ok(JsValue::Undefined));
    }
    let mut ctx = NativeContext { vm };
    let result = invoke_dom_api(
        &mut ctx,
        "classList.item",
        entity,
        &[JsValue::Number(f64::from(idx_u32))],
    );
    Some(result.map(|v| {
        // `classList.item` returns Null for out-of-bounds; indexed
        // getter spec wants Undefined for OOB integer-typed indices
        // because OOB is "no token at this index" rather than "null
        // token".  Convert here so `tokens[999] === undefined` rather
        // than `null`.
        if matches!(v, JsValue::Null) {
            JsValue::Undefined
        } else {
            v
        }
    }))
}
