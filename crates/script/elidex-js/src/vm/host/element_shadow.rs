//! `Element.attachShadow(init)` + `Element.shadowRoot` getter
//! (WHATWG DOM §4.9 "Interface Element").
//!
//! These two natives are the JS-facing entry points for the Shadow
//! DOM surface; the wrapper / state-cache / prototype install lives
//! in [`super::shadow_root_proto`], and the validated engine-indep
//! mutation lives in [`elidex_ecs::EcsDom::attach_shadow_with_init`].
//!
//! ## Brand check
//!
//! Both natives gate on a WebIDL Element brand check via
//! [`super::event_target::require_receiver`]: non-Element receivers
//! throw "Illegal invocation" TypeError per spec, while
//! post-`Vm::unbind` retained wrappers silently no-op (matches
//! elidex's unbound-receiver policy for accessor/method dispatch).

#![cfg(feature = "engine")]

use elidex_ecs::{ShadowAttachError, ShadowInit, ShadowRootMode, SlotAssignmentMode};

use super::super::value::{JsValue, NativeContext, PropertyKey, VmError};

/// `element.attachShadow({mode, delegatesFocus?, slotAssignment?,
/// clonable?, serializable?, customElementRegistry?})` (WHATWG DOM
/// §4.9).
///
/// Returns the freshly-allocated `ShadowRoot` wrapper on success;
/// throws `TypeError` on missing/invalid `mode`, or
/// `NotSupportedError` (DOMException) when the host is not a valid
/// shadow host or already has a shadow root.
///
/// `customElementRegistry` (nullable member) is validated per
/// attachShadow steps 2-3: the document's global registry passes, a
/// foreign registry throws `NotSupportedError` (step 3), a
/// non-registry value throws the WebIDL conversion `TypeError`, and an
/// explicit `null` creates a null-registry shadow root (step 2 —
/// stored on the `ShadowRoot` component; per-context registry lookup
/// for fragment parsing inside the shadow tree is deferred with
/// scoped registries, slot `#11-shadow-scoped-custom-element-registry`).
pub(super) fn native_element_attach_shadow(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    // WebIDL Element brand check FIRST — calling
    // `Element.prototype.attachShadow.call(document, ...)` must throw
    // "Illegal invocation" TypeError before any init-dict parsing.
    // `require_receiver` returns `Ok(None)` post-unbind (silent no-op
    // matching elidex's retained-wrapper policy) and `Err(TypeError)`
    // when the receiver IS a HostObject but not Element-kind.
    let Some(host) =
        super::event_target::require_receiver(ctx, this, "Element", "attachShadow", |k| {
            k == elidex_ecs::NodeKind::Element
        })?
    else {
        return Err(VmError::type_error(
            "Failed to execute 'attachShadow' on 'Element': Illegal invocation".to_string(),
        ));
    };
    let init_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let init = parse_shadow_init(ctx, init_arg)?;
    let shadow_root_entity = match ctx.host().dom().attach_shadow_with_init(host, init) {
        Ok(entity) => entity,
        Err(err) => return Err(attach_shadow_error_to_dom_exception(ctx, err)),
    };
    // Even closed shadow roots return a wrapper from `attachShadow`
    // (the wrapper is what JS uses to populate the shadow tree); the
    // `Element.shadowRoot` getter is where the closed-mode hide
    // semantics apply.  Routing through `create_element_wrapper`
    // keys identity by the shadow root's own entity via the standard
    // `HostData::wrapper_cache` — a subsequent `element.shadowRoot`
    // returns the same wrapper for open mode.
    let wrapper = ctx.vm.create_element_wrapper(shadow_root_entity);
    Ok(JsValue::Object(wrapper))
}

/// `element.shadowRoot` getter (WHATWG DOM §4.9).
///
/// Returns the cached `ShadowRoot` wrapper for the host when its
/// mode is `Open`; returns `null` when the host has no shadow root
/// or when the mode is `Closed` (encapsulation — closed shadows are
/// only reachable via the wrapper handle returned by
/// `attachShadow`).  WebIDL Element brand check fires first — a
/// non-Element receiver (e.g. `Element.prototype.__lookupGetter__('shadowRoot').call(document)`)
/// throws "Illegal invocation" TypeError per spec.
pub(super) fn native_element_get_shadow_root(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // Pre-throw for non-wrapper receivers (`{}` / primitives /
    // unrelated Object) so the spec's WebIDL brand check fires
    // before falling into the unbound-wrapper silent-null branch.
    // `require_receiver`'s `Ok(None)` covers both cases (non-wrapper
    // AND unbound wrapper), but only the latter should silent-null
    // per the elidex unbound-receiver policy.
    if !super::event_target::this_is_node_wrapper(ctx.vm, this) {
        return Err(VmError::type_error(
            "Failed to execute 'shadowRoot' on 'Element': Illegal invocation".to_string(),
        ));
    }
    let Some(host) =
        super::event_target::require_receiver(ctx, this, "Element", "shadowRoot", |k| {
            k == elidex_ecs::NodeKind::Element
        })?
    else {
        return Ok(JsValue::Null);
    };
    let Some(host_data) = ctx.host_if_bound() else {
        return Ok(JsValue::Null);
    };
    let Some(shadow_root_entity) = host_data.dom().get_shadow_root(host) else {
        return Ok(JsValue::Null);
    };
    // Closed-mode encapsulation: `element.shadowRoot` returns null
    // even when a shadow root exists (only `attachShadow` exposes
    // the wrapper for closed shadows, the caller is expected to
    // retain that reference).
    let is_open = host_data
        .dom()
        .world()
        .get::<&elidex_ecs::ShadowRoot>(shadow_root_entity)
        .is_ok_and(|sr| sr.mode == ShadowRootMode::Open);
    if !is_open {
        return Ok(JsValue::Null);
    }
    let wrapper = ctx.vm.create_element_wrapper(shadow_root_entity);
    Ok(JsValue::Object(wrapper))
}

// -------------------------------------------------------------------------
// ShadowRootInit dict parsing
// -------------------------------------------------------------------------

/// Parse the WebIDL `ShadowRootInit` dictionary from the JS argument.
/// Throws TypeError when:
/// - the argument isn't an Object (or `undefined` — which has no `mode`)
/// - `mode` is missing / not a string / not "open" or "closed"
/// - `customElementRegistry` is neither null nor a registry object
///
/// Members are got AND converted in WebIDL lexicographic dictionary
/// order — `clonable`, `customElementRegistry`, `delegatesFocus`,
/// `mode`, `serializable`, `slotAssignment` — so getter side effects
/// and conversion exceptions fire in spec order (e.g. an invalid
/// registry TypeErrors before the `mode` getter runs).
///
/// `slotAssignment` defaults to "named" when missing; non-"named"/"manual"
/// throws TypeError per WebIDL enum semantics.
/// Other boolean fields default to `false`.
/// The converted `customElementRegistry` is then gated by attachShadow
/// steps 2-3 (doc comment on `native_element_attach_shadow`) — an
/// algorithm step, so it runs after the whole dictionary converted.
fn parse_shadow_init(
    ctx: &mut NativeContext<'_>,
    init_arg: JsValue,
) -> Result<ShadowInit, VmError> {
    const PREFIX: &str = "Failed to execute 'attachShadow' on 'Element'";
    let JsValue::Object(init_id) = init_arg else {
        return Err(VmError::type_error(
            "Failed to execute 'attachShadow' on 'Element': \
             'mode' is required and must be 'open' or 'closed'"
                .to_string(),
        ));
    };
    // CONVERSION PHASE — lexicographic member order.
    let clonable = read_optional_bool(ctx, init_id, "clonable")?;
    let registry_key = PropertyKey::String(ctx.vm.strings.intern("customElementRegistry"));
    let registry_raw = ctx.vm.get_property_value(init_id, registry_key)?;
    let registry_member = if matches!(registry_raw, JsValue::Undefined) {
        None
    } else {
        Some(
            super::custom_elements::convert_custom_element_registry_member(
                ctx,
                registry_raw,
                PREFIX,
            )?,
        )
    };
    let delegates_focus = read_optional_bool(ctx, init_id, "delegatesFocus")?;
    let mode = read_required_mode(ctx, init_id)?;
    let serializable = read_optional_bool(ctx, init_id, "serializable")?;
    let slot_assignment = read_optional_slot_assignment(ctx, init_id)?;
    // ALGORITHM PHASE — attachShadow steps 2-3: a foreign registry
    // throws NotSupportedError (step 3, only fires for NON-null
    // registries); an explicit null threads through as a
    // null-registry shadow root (step 2 — stored on the `ShadowRoot`
    // component).
    let mut null_registry = false;
    if let Some(member) = registry_member {
        super::custom_elements::reject_foreign_registry_member(ctx, &member, PREFIX)?;
        null_registry = matches!(member, super::custom_elements::RegistryMember::Null);
    }
    Ok(ShadowInit {
        mode,
        delegates_focus,
        slot_assignment,
        clonable,
        serializable,
        null_registry,
    })
}

fn read_required_mode(
    ctx: &mut NativeContext<'_>,
    init_id: super::super::value::ObjectId,
) -> Result<ShadowRootMode, VmError> {
    let mode_key = PropertyKey::String(ctx.vm.strings.intern("mode"));
    let raw = ctx.vm.get_property_value(init_id, mode_key)?;
    if matches!(raw, JsValue::Undefined) {
        return Err(VmError::type_error(
            "Failed to execute 'attachShadow' on 'Element': \
             'mode' is required and must be 'open' or 'closed'"
                .to_string(),
        ));
    }
    // WebIDL enum conversion: ToString-coerce arbitrary JS values
    // (so `new String('open')`, Symbol.toPrimitive objects, etc.
    // route through their conversion methods).
    let s_sid = super::super::coerce::to_string(ctx.vm, raw)?;
    let s = ctx.vm.strings.get_utf8(s_sid);
    match s.as_str() {
        "open" => Ok(ShadowRootMode::Open),
        "closed" => Ok(ShadowRootMode::Closed),
        _ => Err(VmError::type_error(format!(
            "Failed to execute 'attachShadow' on 'Element': \
             '{s}' is not a valid mode (must be 'open' or 'closed')"
        ))),
    }
}

fn read_optional_bool(
    ctx: &mut NativeContext<'_>,
    init_id: super::super::value::ObjectId,
    field: &str,
) -> Result<bool, VmError> {
    let key = PropertyKey::String(ctx.vm.strings.intern(field));
    let raw = ctx.vm.get_property_value(init_id, key)?;
    if matches!(raw, JsValue::Undefined) {
        return Ok(false);
    }
    Ok(super::super::coerce::to_boolean(ctx.vm, raw))
}

fn read_optional_slot_assignment(
    ctx: &mut NativeContext<'_>,
    init_id: super::super::value::ObjectId,
) -> Result<SlotAssignmentMode, VmError> {
    let key = PropertyKey::String(ctx.vm.strings.intern("slotAssignment"));
    let raw = ctx.vm.get_property_value(init_id, key)?;
    if matches!(raw, JsValue::Undefined) {
        return Ok(SlotAssignmentMode::Named);
    }
    // WebIDL enum conversion is ToString-first (see
    // [`read_required_mode`]).
    let s_sid = super::super::coerce::to_string(ctx.vm, raw)?;
    let s = ctx.vm.strings.get_utf8(s_sid);
    match s.as_str() {
        "named" => Ok(SlotAssignmentMode::Named),
        "manual" => Ok(SlotAssignmentMode::Manual),
        _ => Err(VmError::type_error(format!(
            "Failed to execute 'attachShadow' on 'Element': \
             '{s}' is not a valid slotAssignment (must be 'named' or 'manual')"
        ))),
    }
}

fn attach_shadow_error_to_dom_exception(
    ctx: &mut NativeContext<'_>,
    err: ShadowAttachError,
) -> VmError {
    let not_supported = ctx.vm.well_known.dom_exc_not_supported_error;
    let message = match err {
        ShadowAttachError::InvalidEntity => {
            "Failed to execute 'attachShadow' on 'Element': \
             host element is invalid"
        }
        ShadowAttachError::InvalidTag => {
            "Failed to execute 'attachShadow' on 'Element': \
             this element does not support attachShadow"
        }
        ShadowAttachError::AlreadyAttached => {
            "Failed to execute 'attachShadow' on 'Element': \
             Shadow root cannot be created on a host which already hosts a shadow tree"
        }
    };
    VmError::dom_exception(not_supported, message.to_string())
}
