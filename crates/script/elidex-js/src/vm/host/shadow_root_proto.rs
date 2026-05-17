//! `ShadowRoot.prototype` intrinsic (WHATWG DOM §4.8).
//!
//! `ShadowRoot` is the document-fragment-shaped wrapper returned by
//! `Element.attachShadow({mode})` and `Element.shadowRoot` (open mode
//! only).  It exposes the shadow encapsulation surface:
//!
//! - `host` — back-reference to the shadow host Element.
//! - `mode` — string `"open"` / `"closed"`.
//! - `delegatesFocus` — boolean, defaults `false`.
//! - `slotAssignment` — string `"named"` / `"manual"`, defaults `"named"`.
//! - `clonable` — boolean, defaults `false`.
//! - `serializable` — boolean, defaults `false`.
//!
//! Inherits `DocumentFragment.prototype`, which currently carries the
//! ParentNode-mixin **mutation methods** (`prepend` / `append` /
//! `replaceChildren`); so `shadow.append(...)` etc. work without
//! per-method install here.  Selector / children accessors
//! (`querySelector`, `firstElementChild`, `children`, …) live on
//! `Element.prototype` only and are NOT yet exposed through this
//! chain — tracked by defer slot `#11-shadow-parent-node-accessors`.
//!
//! ## Backing state
//!
//! `ShadowRoot` wrappers are `ObjectKind::HostObject { entity_bits }`
//! carrying the shadow root's own `Entity` — same as Element / Text
//! wrappers.  The `ShadowRoot` kind is identified by the engine ECS
//! component (`elidex_ecs::ShadowRoot`) on the entity, not by a
//! distinct `ObjectKind` variant ([feedback_objectkind-resolution-uniformity]).
//! Identity across `element.shadowRoot` reads is preserved by the
//! standard `HostData::wrapper_cache` entity-keyed cache.
//!
//! ## Brand check
//!
//! Every accessor routes through [`require_shadow_root_receiver`];
//! non-ShadowRoot receivers throw "Illegal invocation" TypeError per
//! WebIDL brand semantics.  Brand check = recover entity bits from
//! `HostObject`, then verify the entity carries the
//! `elidex_ecs::ShadowRoot` component.

#![cfg(feature = "engine")]

use elidex_ecs::{Entity, ShadowRoot as EcsShadowRoot, ShadowRootMode, SlotAssignmentMode};

use super::super::shape;
use super::super::value::{JsValue, NativeContext, Object, ObjectKind, PropertyStorage, VmError};
use super::super::VmInner;

impl VmInner {
    /// Allocate `ShadowRoot.prototype` with `DocumentFragment.prototype`
    /// as parent and install the 6 ShadowRoot accessors (WHATWG DOM
    /// §4.8).  Must run after `register_document_fragment_prototype`.
    pub(in crate::vm) fn register_shadow_root_prototype(&mut self) {
        let df_proto = self
            .document_fragment_prototype
            .expect("register_shadow_root_prototype before register_document_fragment_prototype");
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(df_proto),
            extensible: true,
        });
        self.shadow_root_prototype = Some(proto_id);

        // Install all 6 ShadowRoot accessors individually rather than
        // via a `for` loop array literal — the loop form was hitting
        // an install-no-op pattern during D-15 PR-A bring-up.  Match
        // the named_node_map.rs sibling shape exactly.
        let host_sid = self.strings.intern("host");
        self.install_accessor_pair(
            proto_id,
            host_sid,
            native_shadow_root_get_host,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        let mode_sid = self.well_known.mode;
        self.install_accessor_pair(
            proto_id,
            mode_sid,
            native_shadow_root_get_mode,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        let delegates_focus_sid = self.well_known.delegates_focus;
        self.install_accessor_pair(
            proto_id,
            delegates_focus_sid,
            native_shadow_root_get_delegates_focus,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        let slot_assignment_sid = self.well_known.slot_assignment;
        self.install_accessor_pair(
            proto_id,
            slot_assignment_sid,
            native_shadow_root_get_slot_assignment,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        let clonable_sid = self.well_known.clonable;
        self.install_accessor_pair(
            proto_id,
            clonable_sid,
            native_shadow_root_get_clonable,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        let serializable_sid = self.well_known.serializable;
        self.install_accessor_pair(
            proto_id,
            serializable_sid,
            native_shadow_root_get_serializable,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        // innerHTML mixin: HTML §4.4.5 / §4.4.6 / §4.4.7 — the shared
        // native bodies (closure-keyed brand checks) live in
        // [`super::dom_inner_html`].
        self.install_accessor_pair(
            proto_id,
            self.well_known.inner_html,
            super::dom_inner_html::native_shadow_root_get_inner_html,
            Some(super::dom_inner_html::native_shadow_root_set_inner_html),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        self.install_native_method(
            proto_id,
            self.well_known.set_html_unsafe,
            super::dom_inner_html::native_shadow_root_set_html_unsafe,
            shape::PropertyAttrs::METHOD,
        );
        self.install_native_method(
            proto_id,
            self.well_known.get_html,
            super::dom_inner_html::native_shadow_root_get_html,
            shape::PropertyAttrs::METHOD,
        );
    }
}

// -------------------------------------------------------------------------
// Brand check
// -------------------------------------------------------------------------

/// Recover the shadow root `Entity` for a receiver, or throw
/// "Illegal invocation" TypeError when the receiver isn't a
/// ShadowRoot.
///
/// Brand check = the receiver is a `HostObject` wrapper whose backing
/// entity carries the `elidex_ecs::ShadowRoot` component.  Per
/// [feedback_objectkind-resolution-uniformity] the discriminator is
/// the ECS component, not an `ObjectKind` variant.
fn require_shadow_root_receiver(
    ctx: &NativeContext<'_>,
    this: JsValue,
    accessor: &'static str,
) -> Result<Entity, VmError> {
    let illegal = || -> VmError {
        VmError::type_error(format!(
            "Failed to execute '{accessor}' on 'ShadowRoot': Illegal invocation"
        ))
    };
    let JsValue::Object(id) = this else {
        return Err(illegal());
    };
    let ObjectKind::HostObject { entity_bits } = ctx.vm.get_object(id).kind else {
        return Err(illegal());
    };
    let entity = Entity::from_bits(entity_bits).ok_or_else(illegal)?;
    if !super::event_target::is_shadow_root_entity(ctx.vm, entity) {
        return Err(illegal());
    }
    Ok(entity)
}

/// Read the `ShadowRoot` ECS component for a shadow-root `Entity`.
/// Returns `None` if the entity has been destroyed or its component
/// is unreachable (post-unbind).  Callers fall back to spec-default
/// values when the component is missing.
fn read_shadow_component<R>(
    ctx: &mut NativeContext<'_>,
    shadow_root: Entity,
    extract: impl FnOnce(&EcsShadowRoot) -> R,
) -> Option<R> {
    let host = ctx.host_if_bound()?;
    host.dom()
        .world()
        .get::<&EcsShadowRoot>(shadow_root)
        .ok()
        .map(|sr| extract(&sr))
}

// -------------------------------------------------------------------------
// Accessors
// -------------------------------------------------------------------------

fn native_shadow_root_get_host(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let shadow_root = require_shadow_root_receiver(ctx, this, "host")?;
    let host = read_shadow_component(ctx, shadow_root, |sr| sr.host);
    let Some(host) = host else {
        return Ok(JsValue::Null);
    };
    let wrapper = ctx.vm.create_element_wrapper(host);
    Ok(JsValue::Object(wrapper))
}

fn native_shadow_root_get_mode(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let shadow_root = require_shadow_root_receiver(ctx, this, "mode")?;
    let mode =
        read_shadow_component(ctx, shadow_root, |sr| sr.mode).unwrap_or(ShadowRootMode::Open);
    let sid = match mode {
        ShadowRootMode::Open => ctx.vm.strings.intern("open"),
        ShadowRootMode::Closed => ctx.vm.strings.intern("closed"),
    };
    Ok(JsValue::String(sid))
}

fn native_shadow_root_get_delegates_focus(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let shadow_root = require_shadow_root_receiver(ctx, this, "delegatesFocus")?;
    let value = read_shadow_component(ctx, shadow_root, |sr| sr.delegates_focus).unwrap_or(false);
    Ok(JsValue::Boolean(value))
}

fn native_shadow_root_get_slot_assignment(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let shadow_root = require_shadow_root_receiver(ctx, this, "slotAssignment")?;
    let mode = read_shadow_component(ctx, shadow_root, |sr| sr.slot_assignment)
        .unwrap_or(SlotAssignmentMode::Named);
    let sid = match mode {
        SlotAssignmentMode::Named => ctx.vm.strings.intern("named"),
        SlotAssignmentMode::Manual => ctx.vm.strings.intern("manual"),
    };
    Ok(JsValue::String(sid))
}

fn native_shadow_root_get_clonable(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let shadow_root = require_shadow_root_receiver(ctx, this, "clonable")?;
    let value = read_shadow_component(ctx, shadow_root, |sr| sr.clonable).unwrap_or(false);
    Ok(JsValue::Boolean(value))
}

fn native_shadow_root_get_serializable(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let shadow_root = require_shadow_root_receiver(ctx, this, "serializable")?;
    let value = read_shadow_component(ctx, shadow_root, |sr| sr.serializable).unwrap_or(false);
    Ok(JsValue::Boolean(value))
}
