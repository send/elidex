//! `DOMException` constructor + prototype + side-table state
//! (WebIDL §3.14).
//!
//! ## Shape
//!
//! ```text
//! DOMException instance (ObjectKind::Ordinary, no own data props)
//!   → DOMException.prototype            (name / message / code accessors)
//!     → Error.prototype
//!       → Object.prototype
//! ```
//!
//! Instance `name` / `message` / `code` are **prototype accessors**
//! that read out-of-band state keyed by the instance's `ObjectId`
//! (see [`VmInner::dom_exception_states`]).  The accessor layout
//! matches WebIDL §3.6.8 (attribute = accessor property, reads an
//! internal slot) rather than the own-data layout Error uses —
//! `Object.keys(new DOMException("m")) === []` holds, matching
//! browsers.
//!
//! ## Consumers
//!
//! Built via [`VmInner::vm_error_to_thrown`] whenever a
//! [`VmErrorKind::DomException`] bubbles up — callers just do
//! `Err(VmError::dom_exception(well_known.dom_exc_syntax_error, msg))`
//! without assembling any object themselves.

#![cfg(feature = "engine")]

use super::super::shape::{self, PropertyAttrs};
use super::super::value::{
    JsValue, NativeContext, Object, ObjectKind, PropertyKey, PropertyStorage, PropertyValue,
    StringId, VmError,
};
use super::super::{NativeFn, VmInner};

// ---------------------------------------------------------------------------
// Shared DOMException factories for DOM-method call sites
// ---------------------------------------------------------------------------

/// `DOMException("HierarchyRequestError")` factory.  `name_sid` is
/// pre-captured by the caller (see module-level contract — callers
/// holding a `ctx.host().dom()` mutable borrow can't reach
/// `ctx.vm.well_known` directly).  `interface` is the WebIDL
/// interface name that owns the method (`ChildNode` / `ParentNode`
/// / `Element` / ...); `method` is the member name;
/// `detail` is the spec-quoted failure message.
pub(super) fn hierarchy_request_error(
    name_sid: StringId,
    interface: &str,
    method: &str,
    detail: &str,
) -> VmError {
    VmError::dom_exception(
        name_sid,
        format!("Failed to execute '{method}' on '{interface}': {detail}"),
    )
}

/// `DOMException("InvalidStateError")` factory.  Used by the WHATWG
/// DOM §2.9 step 3 re-dispatch throw in `EventTarget.dispatchEvent`
/// (event whose dispatch flag is already set); future callers may
/// arrive as new algorithm steps pull in the error shape.
pub(super) fn invalid_state_error(
    name_sid: StringId,
    interface: &str,
    method: &str,
    detail: &str,
) -> VmError {
    VmError::dom_exception(
        name_sid,
        format!("Failed to execute '{method}' on '{interface}': {detail}"),
    )
}

/// Per-instance out-of-band state for a `DOMException`.  Storage
/// lives on [`super::super::VmInner::dom_exception_states`], keyed
/// by the instance's `ObjectId` (same pattern as
/// [`super::abort::AbortSignalState`]).
///
/// Both fields are interned `StringId`s (pool-permanent); no
/// `mark_value` pass needed during GC trace — the out-of-band
/// HashMap key pruning in `collect_garbage` handles cleanup.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DomExceptionState {
    pub(crate) name: StringId,
    pub(crate) message: StringId,
}

/// Resolve `this` to the `DomExceptionState` entry for the instance.
///
/// WebIDL §3.2 brand-check semantics:
/// - `null` / `undefined` and other non-object receivers → throw
///   `TypeError`, matching
///   `Object.getOwnPropertyDescriptor(DOMException.prototype,
///   'name').get.call(undefined)` behaviour on browsers.
/// - wrong-brand object receiver (no entry in
///   `dom_exception_states`) → throw `TypeError` ("Illegal
///   invocation").
/// - valid instance → return a copied `DomExceptionState` (cheap —
///   two `StringId`s).
fn require_dom_exception_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
    attr: &str,
) -> Result<DomExceptionState, VmError> {
    match this {
        JsValue::Object(id) => ctx
            .vm
            .dom_exception_states
            .get(&id)
            .copied()
            .ok_or_else(|| wrong_brand(attr)),
        _ => Err(wrong_brand(attr)),
    }
}

fn wrong_brand(attr: &str) -> VmError {
    VmError::type_error(format!(
        "Failed to get '{attr}' on 'DOMException': Illegal invocation"
    ))
}

// ---------------------------------------------------------------------------
// Accessors (name / message / code)
// ---------------------------------------------------------------------------

fn native_dom_exception_get_name(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let state = require_dom_exception_this(ctx, this, "name")?;
    Ok(JsValue::String(state.name))
}

fn native_dom_exception_get_message(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let state = require_dom_exception_this(ctx, this, "message")?;
    Ok(JsValue::String(state.message))
}

fn native_dom_exception_get_code(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let state = require_dom_exception_this(ctx, this, "code")?;
    let code = legacy_code_for_name(ctx, state.name);
    Ok(JsValue::Number(f64::from(code)))
}

/// Legacy numeric code for a WHATWG §2.1 exception name, or `0` for
/// names outside the legacy set (WebIDL §3.14.3 "legacy
/// DOMException" codes).
///
/// The arms below cover every well-known DOMException name the VM
/// can currently throw via `WellKnownStrings::dom_exc_*` (plus the
/// shared `abort_error`).  Adding a new `dom_exc_*` field WITHOUT
/// also adding the corresponding arm here would surface a
/// spec-named exception with `.code === 0` — silently breaking
/// scripts that branch on the legacy code.  Keep the two in sync.
fn legacy_code_for_name(ctx: &NativeContext<'_>, name: StringId) -> u32 {
    let wk = &ctx.vm.well_known;
    if name == wk.dom_exc_index_size_error {
        1
    } else if name == wk.dom_exc_hierarchy_request_error {
        3
    } else if name == wk.dom_exc_wrong_document_error {
        4
    } else if name == wk.dom_exc_invalid_character_error {
        5
    } else if name == wk.dom_exc_not_found_error {
        8
    } else if name == wk.dom_exc_not_supported_error {
        9
    } else if name == wk.dom_exc_in_use_attribute_error {
        10
    } else if name == wk.dom_exc_invalid_state_error {
        11
    } else if name == wk.dom_exc_syntax_error {
        12
    } else if name == wk.abort_error {
        20
    } else if name == wk.dom_exc_timeout_error {
        23
    } else if name == wk.dom_exc_data_clone_error {
        25
    } else if name == wk.dom_exc_quota_exceeded_error {
        22
    } else {
        0
    }
}

// ---------------------------------------------------------------------------
// Constructor
// ---------------------------------------------------------------------------

/// `new DOMException(message = "", name = "Error")` — WebIDL §3.14.1.
///
/// Argument order is **message first, then name** — opposite of the
/// JS `Error` constructor's `new Error(message)` convention (which
/// doesn't take a name at all).  Easy to flip on the implementation
/// side; tests pin the spec order.
fn native_dom_exception_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    // WebIDL §3.7 + browser parity: DOMException is `[Constructor]`
    // only — `DOMException("m")` without `new` throws TypeError in
    // every major browser.  Matches the `AbortController` and
    // `Promise` constructor contract in this crate.
    if !ctx.is_construct() {
        return Err(VmError::type_error(
            "Failed to construct 'DOMException': Please use the 'new' operator",
        ));
    }

    let message_sid = match args.first().copied() {
        Some(JsValue::Undefined) | None => ctx.vm.well_known.empty,
        Some(v) => super::super::coerce::to_string(ctx.vm, v)?,
    };
    let name_sid = match args.get(1).copied() {
        Some(JsValue::Undefined) | None => ctx.vm.strings.intern("Error"),
        Some(v) => super::super::coerce::to_string(ctx.vm, v)?,
    };

    // `new` pre-allocated an instance whose prototype already chains
    // to `DOMException.prototype` via `do_new`'s lookup.  Reuse that
    // instance (as `AbortController` does) so the ECS / shape state
    // lines up with the receiver the caller will see.
    let proto = ctx.vm.dom_exception_prototype;
    let receiver = ctx.vm.ensure_instance_or_alloc(this, proto);
    let JsValue::Object(id) = receiver else {
        // `ensure_instance_or_alloc` always returns Object — this
        // arm is unreachable in practice, but keeps the match
        // exhaustive without a panic.
        return Err(VmError::internal(
            "DOMException constructor: receiver allocation did not yield an object",
        ));
    };
    ctx.vm.dom_exception_states.insert(
        id,
        DomExceptionState {
            name: name_sid,
            message: message_sid,
        },
    );
    Ok(JsValue::Object(id))
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

impl VmInner {
    /// Install `DOMException.prototype` (chained to `Error.prototype`),
    /// the `DOMException` global constructor, and the three prototype
    /// accessors (`name` / `message` / `code`).
    ///
    /// Ordering: must run **after**
    /// `register_error_constructors` so `error_prototype` is populated.
    ///
    /// # Panics
    ///
    /// Panics if `error_prototype` is `None` (would mean
    /// `register_error_constructors` was skipped or run after this).
    pub(in crate::vm) fn register_dom_exception_global(&mut self) {
        let error_proto = self
            .error_prototype
            .expect("register_dom_exception_global called before register_error_constructors");

        // ---- DOMException.prototype ----
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(error_proto),
            extensible: true,
        });
        for (name_sid, getter) in [
            (
                self.well_known.name,
                native_dom_exception_get_name as NativeFn,
            ),
            (self.well_known.message, native_dom_exception_get_message),
            (self.well_known.code, native_dom_exception_get_code),
        ] {
            self.install_accessor_pair(
                proto_id,
                name_sid,
                getter,
                None,
                PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }
        self.dom_exception_prototype = Some(proto_id);

        // ---- DOMException constructor + global ----
        let ctor =
            self.create_constructable_function("DOMException", native_dom_exception_constructor);
        let proto_key = PropertyKey::String(self.well_known.prototype);
        self.define_shaped_property(
            ctor,
            proto_key,
            PropertyValue::Data(JsValue::Object(proto_id)),
            PropertyAttrs::BUILTIN,
        );
        let ctor_key = PropertyKey::String(self.well_known.constructor);
        self.define_shaped_property(
            proto_id,
            ctor_key,
            PropertyValue::Data(JsValue::Object(ctor)),
            PropertyAttrs::METHOD,
        );
        let name = self.well_known.dom_exception;
        self.globals.insert(name, JsValue::Object(ctor));
    }

    /// Materialise a `VmErrorKind::DomException` variant into a JS
    /// `DOMException` instance and register its side-table state.
    /// Called by `vm_error_to_thrown` when the unified dispatch path
    /// hands up a DOMException-kind error.
    ///
    /// `name` is a pre-interned `StringId`; `message` is not
    /// pre-interned (it varies per call site — each `VmError`
    /// message string).
    pub(crate) fn build_dom_exception(&mut self, name: StringId, message: &str) -> JsValue {
        let proto = self.dom_exception_prototype;
        let id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: proto,
            extensible: true,
        });
        let msg_sid = self.strings.intern(message);
        self.dom_exception_states.insert(
            id,
            DomExceptionState {
                name,
                message: msg_sid,
            },
        );
        JsValue::Object(id)
    }
}
