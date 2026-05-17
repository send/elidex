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
//! `ObjectKind::ShadowRoot` is payload-free; the shadow root `Entity`
//! lives in [`crate::vm::VmInner::shadow_root_states`] keyed by this
//! `ObjectId`.  Identity is preserved across `element.shadowRoot` reads
//! via [`crate::vm::VmInner::shadow_root_wrappers`] keyed by host
//! `Entity` (mirrors `template_content_wrappers`).
//!
//! ## Brand check
//!
//! Every accessor routes through [`require_shadow_root_receiver`];
//! non-ShadowRoot receivers throw "Illegal invocation" TypeError per
//! WebIDL brand semantics.

#![cfg(feature = "engine")]

use elidex_ecs::{Entity, ShadowRoot as EcsShadowRoot, ShadowRootMode, SlotAssignmentMode};

use super::super::shape;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyStorage, VmError,
};
use super::super::VmInner;

/// Backing state for `ObjectKind::ShadowRoot` wrappers — the
/// shadow root `Entity` each wrapper resolves to.  Kept slim
/// (single `Entity` field) so wrappers can be allocated freely
/// without `ObjectId` GC fan-out concerns; mirrors `AttrState`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ShadowRootState {
    /// The shadow root entity (NOT the host).  Use `EcsDom`'s
    /// `ShadowRoot` component lookup to recover host / mode / etc.
    pub(crate) shadow_root: Entity,
}

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
    }

    /// Allocate a fresh `ShadowRoot` wrapper backed by the given
    /// shadow root entity.  Identity caching is the caller's
    /// responsibility — use [`Self::cached_or_alloc_shadow_root`].
    pub(crate) fn alloc_shadow_root(&mut self, shadow_root: Entity) -> ObjectId {
        let proto = self
            .shadow_root_prototype
            .expect("alloc_shadow_root before register_shadow_root_prototype");
        let id = self.alloc_object(Object {
            kind: ObjectKind::ShadowRoot,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(proto),
            extensible: false,
        });
        self.shadow_root_states
            .insert(id, ShadowRootState { shadow_root });
        id
    }

    /// Identity-preserving allocation for `ShadowRoot` wrappers keyed
    /// by host element.  Repeated `el.shadowRoot` reads return the
    /// same `ObjectId` (matches Chrome / Firefox).  Callers must only
    /// invoke this for hosts known to carry an open shadow root —
    /// closed-mode short-circuits to `null` before this lookup.
    pub(crate) fn cached_or_alloc_shadow_root(
        &mut self,
        host: Entity,
        shadow_root: Entity,
    ) -> ObjectId {
        if let Some(&id) = self.shadow_root_wrappers.get(&host) {
            return id;
        }
        let id = self.alloc_shadow_root(shadow_root);
        self.shadow_root_wrappers.insert(host, id);
        id
    }
}

// -------------------------------------------------------------------------
// Brand check
// -------------------------------------------------------------------------

/// Recover the shadow root `Entity` for a receiver, or throw
/// "Illegal invocation" TypeError when the receiver isn't a
/// ShadowRoot.
fn require_shadow_root_receiver(
    ctx: &NativeContext<'_>,
    this: JsValue,
    accessor: &'static str,
) -> Result<Entity, VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "Failed to execute '{accessor}' on 'ShadowRoot': Illegal invocation"
        )));
    };
    if !matches!(ctx.vm.get_object(id).kind, ObjectKind::ShadowRoot) {
        return Err(VmError::type_error(format!(
            "Failed to execute '{accessor}' on 'ShadowRoot': Illegal invocation"
        )));
    }
    ctx.vm
        .shadow_root_states
        .get(&id)
        .map(|s| s.shadow_root)
        .ok_or_else(|| {
            VmError::type_error(format!(
                "Failed to execute '{accessor}' on 'ShadowRoot': Illegal invocation"
            ))
        })
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
