//! Attribute manipulation members of `Element.prototype`
//! (WHATWG DOM §4.9 + §4.9.2).
//!
//! Carries the attribute getter / setter / remover / toggle /
//! names natives, the Attr-typed entry points
//! (`getAttributeNode` / `setAttributeNode` /
//! `removeAttributeNode`), the `attributes` NamedNodeMap accessor,
//! `tagName`, and the reflected-string `id` / `className`
//! accessors.  Split out of `element_proto.rs` so that module
//! stays under the project's 1000-line convention.
//!
//! `install_element_attributes` on [`crate::vm::VmInner`] (defined
//! in `element_proto.rs`) walks the native-fn table exposed here
//! via `pub(super)` visibility.

#![cfg(feature = "engine")]

use super::super::value::{JsValue, NativeContext, ObjectKind, VmError};
use super::dom_bridge::{
    coerce_first_arg_to_string, coerce_first_arg_to_string_id, invoke_dom_api,
};
use super::event_target::entity_from_this;

use elidex_ecs::Entity;

// ---------------------------------------------------------------------------
// Natives: attribute manipulation + id / className / tagName
// ---------------------------------------------------------------------------

pub(super) fn native_element_get_tag_name(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    // WHATWG DOM §4.9 tagName: HTML elements are uppercase.  Every
    // document we bind is treated as HTML in Phase 2.  Uppercase the
    // tag inside the borrow so the eventual `intern` only sees the
    // already-uppercased copy.
    let upper = ctx
        .host()
        .dom()
        .with_tag_name(entity, |t| t.map(str::to_ascii_uppercase));
    match upper {
        Some(s) => {
            let sid = ctx.vm.strings.intern(&s);
            Ok(JsValue::String(sid))
        }
        None => Ok(JsValue::String(ctx.vm.well_known.empty)),
    }
}

/// Read attribute `name` on `entity` as a String, or `None` if absent.
///
/// Thin shim around [`elidex_ecs::EcsDom::get_attribute`]; retained here
/// to keep call sites terse and to enforce the `NativeContext` borrow
/// discipline.
pub(super) fn attr_get(ctx: &mut NativeContext<'_>, entity: Entity, name: &str) -> Option<String> {
    ctx.host().dom().get_attribute(entity, name)
}

/// Set attribute `name` = `value` on `entity`.  Shim around
/// [`elidex_ecs::EcsDom::set_attribute`].  Returns `false` when the
/// entity has been destroyed.
pub(super) fn attr_set(
    ctx: &mut NativeContext<'_>,
    entity: Entity,
    name: &str,
    value: String,
) -> bool {
    ctx.host().dom().set_attribute(entity, name, value)
}

/// Remove attribute `name` from `entity`.  Shim around
/// [`elidex_ecs::EcsDom::remove_attribute`] that also invalidates
/// the [`crate::vm::VmInner::attr_wrapper_cache`] entry for
/// `(entity, intern(name))` so any subsequent `getAttributeNode`
/// for the same name allocates a fresh wrapper (matches WHATWG
/// §4.9.2 identity semantics — the removed attribute's Attr is
/// no longer in the element's attribute list).
///
/// `name` is the UTF-8 form passed to the DOM; the cache is
/// keyed by `intern(utf8)` across every hit site (`getAttributeNode`,
/// `nnm.{item, getNamedItem, [Symbol.iterator]}`, `nnm[k]`),
/// so this re-intern lands on the same `StringId` they cached
/// under and the invalidation is correctly observed.
pub(super) fn attr_remove(ctx: &mut NativeContext<'_>, entity: Entity, name: &str) {
    ctx.host().dom().remove_attribute(entity, name);
    let qname_sid = ctx.vm.strings.intern(name);
    ctx.vm.invalidate_attr_cache_entry(entity, qname_sid);
}

pub(super) fn native_element_get_attribute(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Null);
    };
    // Spec-precise ToString runs at call site (handler's
    // `require_string_arg` rejects `ObjectRef`).
    let name_sid = coerce_first_arg_to_string_id(ctx, args)?;
    invoke_dom_api(ctx, "getAttribute", entity, &[JsValue::String(name_sid)])
}

pub(super) fn native_element_set_attribute(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };
    // Coerce BOTH args (name then value) per WebIDL ToString — handler
    // path expects pre-stringified values.
    let name_sid = coerce_first_arg_to_string_id(ctx, args)?;
    let value_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let value_sid = super::super::coerce::to_string(ctx.vm, value_arg)?;
    invoke_dom_api(
        ctx,
        "setAttribute",
        entity,
        &[JsValue::String(name_sid), JsValue::String(value_sid)],
    )
}

pub(super) fn native_element_remove_attribute(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };
    let name = coerce_first_arg_to_string(ctx, args)?;
    attr_remove(ctx, entity, &name);
    Ok(JsValue::Undefined)
}

pub(super) fn native_element_has_attribute(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Boolean(false));
    };
    let name = coerce_first_arg_to_string(ctx, args)?;
    let has = {
        let dom = ctx.host().dom();
        dom.world()
            .get::<&elidex_ecs::Attributes>(entity)
            .ok()
            .is_some_and(|attrs| attrs.contains(&name))
    };
    Ok(JsValue::Boolean(has))
}

pub(super) fn native_element_get_attribute_names(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        // WHATWG §4.9.2 getAttributeNames — returns a list; we return
        // an empty Array for unbound / non-HostObject receivers.
        return Ok(JsValue::Object(ctx.vm.create_array_object(Vec::new())));
    };
    let names: Vec<String> = {
        let dom = ctx.host().dom();
        dom.world()
            .get::<&elidex_ecs::Attributes>(entity)
            .ok()
            .map(|attrs| attrs.iter().map(|(k, _)| k.to_owned()).collect())
            .unwrap_or_default()
    };
    let values: Vec<JsValue> = names
        .into_iter()
        .map(|n| {
            let sid = ctx.vm.strings.intern(&n);
            JsValue::String(sid)
        })
        .collect();
    Ok(JsValue::Object(ctx.vm.create_array_object(values)))
}

// --- Attr-typed helpers (WHATWG §4.9.2) ------------------------------

/// `element.attributes` accessor — returns a live `NamedNodeMap`
/// keyed by the receiver's Entity.  Per-access allocation matches
/// the HTMLCollection pattern; identity is NOT preserved across
/// reads (`el.attributes !== el.attributes`).  Live semantics come
/// from the NamedNodeMap's re-resolution against the backing
/// `Attributes` ECS component on each method / accessor call.
pub(super) fn native_element_get_attributes(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Null);
    };
    let id = ctx.vm.alloc_named_node_map(entity);
    Ok(JsValue::Object(id))
}

/// `element.getAttributeNode(name)` — return an Attr wrapper for
/// the named attribute, or `null` when absent.  Repeated calls for
/// the same `(entity, qualified_name)` return the same `ObjectId`
/// via [`crate::vm::VmInner::cached_or_alloc_attr_live`].
pub(super) fn native_element_get_attribute_node(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Null);
    };
    let name = coerce_first_arg_to_string(ctx, args)?;
    if !ctx.host().dom().has_attribute(entity, &name) {
        return Ok(JsValue::Null);
    }
    // Cache key is `intern(utf8)` — the same form `nnm.item` /
    // `[Symbol.iterator]` derive from DOM-stored attribute names —
    // so identity holds across all paths even for lone-surrogate
    // inputs (the DOM stores UTF-8 verbatim, so the original
    // UCS-2 `StringId` would diverge from snapshot-derived keys).
    let qname_sid = ctx.vm.strings.intern(&name);
    let attr_id = ctx.vm.cached_or_alloc_attr_live(entity, qname_sid);
    Ok(JsValue::Object(attr_id))
}

/// `element.setAttributeNode(attr)` — write the Attr's value onto
/// the receiver under the Attr's name.  Returns the previous Attr
/// (wrapper over the old value) or `null` when no attribute of
/// that name existed.
pub(super) fn native_element_set_attribute_node(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Null);
    };
    let arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let JsValue::Object(attr_id) = arg else {
        return Err(VmError::type_error(
            "Failed to execute 'setAttributeNode' on 'Element': argument is not an Attr"
                .to_string(),
        ));
    };
    if !matches!(ctx.vm.get_object(attr_id).kind, ObjectKind::Attr) {
        return Err(VmError::type_error(
            "Failed to execute 'setAttributeNode' on 'Element': argument is not an Attr"
                .to_string(),
        ));
    }
    let Some(state) = ctx.vm.attr_states.get(&attr_id) else {
        return Err(VmError::type_error(
            "Failed to execute 'setAttributeNode' on 'Element': Attr has no backing state"
                .to_string(),
        ));
    };
    let source_owner = state.owner;
    let qname_sid = state.qualified_name;
    let source_detached = state.detached_value;
    let empty = ctx.vm.well_known.empty;
    // Mirror `Attr.prototype.value`: detached snapshot first, else
    // the source owner's current attribute value.  Capture both
    // values + the prior-target snapshot in one split-borrow pass
    // so prev_value can be interned directly from the borrowed
    // `&str` (no `String::from` clone).
    let (name_str, new_value, prev_sid) = match ctx.dom_and_strings_if_bound() {
        Some((dom, strings)) => {
            let name_str = strings.get_utf8(qname_sid);
            let new_value = if let Some(snapshot_sid) = source_detached {
                strings.get_utf8(snapshot_sid)
            } else {
                dom.with_attribute(source_owner, &name_str, |v| {
                    v.map(str::to_owned).unwrap_or_default()
                })
            };
            let prev_sid = dom.with_attribute(entity, &name_str, |v| {
                v.map(|s| strings.intern_or_alias(empty, s))
            });
            (name_str, new_value, prev_sid)
        }
        None => return Ok(JsValue::Null),
    };
    // Snapshot the prev value BEFORE overwriting so the returned
    // detached Attr observes the replaced value, not the just-written
    // one (WHATWG §4.9.2).  Surface a post-snapshot unbind as `Null`
    // (no mutation, no "previous" Attr) instead of panicking via
    // `HostData::dom()`'s `is_bound` assert.
    let Some(host) = ctx.host_if_bound() else {
        return Ok(JsValue::Null);
    };
    host.dom().set_attribute(entity, &name_str, new_value);
    // Sync the identity cache for `(entity, qname_sid)`:
    // - Live Attrs already attached to `entity` (`source_owner ==
    //   entity`, `source_detached.is_none()`) become / stay
    //   canonical: insert/refresh so reattachment after a prior
    //   `removeAttribute` (which empties the cache) still keeps
    //   `el.getAttributeNode(name) === a`.
    // - Cross-element or detached Attrs cannot be made canonical
    //   here (the engine path doesn't retarget their
    //   `AttrState.owner`), so drop the entry instead.
    if source_owner == entity && source_detached.is_none() {
        ctx.vm
            .attr_wrapper_cache
            .insert((entity, qname_sid), attr_id);
    } else {
        ctx.vm.invalidate_attr_cache_entry(entity, qname_sid);
    }
    Ok(match prev_sid {
        Some(sid) => {
            let prev = ctx.vm.alloc_attr(super::attr_proto::AttrState {
                owner: entity,
                qualified_name: qname_sid,
                detached_value: Some(sid),
            });
            JsValue::Object(prev)
        }
        None => JsValue::Null,
    })
}

/// `element.removeAttributeNode(attr)` — detach the attribute
/// identified by the Attr from the receiver.  Throws
/// `NotFoundError` when the receiver has no attribute with the
/// matching qualified name (WHATWG §4.9.2 step 2).
pub(super) fn native_element_remove_attribute_node(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Null);
    };
    let arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let JsValue::Object(attr_id) = arg else {
        return Err(VmError::type_error(
            "Failed to execute 'removeAttributeNode' on 'Element': argument is not an Attr"
                .to_string(),
        ));
    };
    if !matches!(ctx.vm.get_object(attr_id).kind, ObjectKind::Attr) {
        return Err(VmError::type_error(
            "Failed to execute 'removeAttributeNode' on 'Element': argument is not an Attr"
                .to_string(),
        ));
    }
    let Some(state) = ctx.vm.attr_states.get(&attr_id) else {
        return Err(VmError::type_error(
            "Failed to execute 'removeAttributeNode' on 'Element': Attr has no backing state"
                .to_string(),
        ));
    };
    let attr_owner = state.owner;
    let qname_sid = state.qualified_name;
    // WHATWG §4.9.2 step 1: the Attr must be attached to THIS
    // element.  Without the owner check, passing an Attr from a
    // different Element that happens to share a qualified name
    // would remove the wrong attribute.
    let name_str = ctx.vm.strings.get_utf8(qname_sid);
    if attr_owner != entity {
        let not_found = ctx.vm.well_known.dom_exc_not_found_error;
        return Err(VmError::dom_exception(
            not_found,
            format!(
                "Failed to execute 'removeAttributeNode' on 'Element': \
                 '{name_str}' is not an attribute of this element"
            ),
        ));
    }
    let empty = ctx.vm.well_known.empty;
    // Snapshot the prior value via the split-borrow path so the
    // intern happens directly on the borrowed `&str` (no
    // `String::from` clone).  Absence is the spec's
    // `NotFoundError` trigger — an unbound receiver is treated the
    // same way (no readable attribute).
    let prev_sid = ctx.dom_and_strings_if_bound().and_then(|(dom, strings)| {
        dom.with_attribute(entity, &name_str, |v| {
            v.map(|s| strings.intern_or_alias(empty, s))
        })
    });
    let Some(prev_sid) = prev_sid else {
        let not_found = ctx.vm.well_known.dom_exc_not_found_error;
        return Err(VmError::dom_exception(
            not_found,
            format!("Failed to execute 'removeAttributeNode' on 'Element': '{name_str}' not found"),
        ));
    };
    // Apply the removal through `host_if_bound` BEFORE mutating
    // the Attr's `attr_states` snapshot — if the host happens to
    // be unbound between the snapshot and the write, surface the
    // recoverable `NotFoundError` without leaving the passed Attr
    // observably detached.  The wrapper-detach step matches WHATWG
    // §4.9.2 "remove an attribute"'s requirement that the removed
    // Attr report its prior value through `attr.value` afterwards.
    let Some(host) = ctx.host_if_bound() else {
        let not_found = ctx.vm.well_known.dom_exc_not_found_error;
        return Err(VmError::dom_exception(
            not_found,
            format!("Failed to execute 'removeAttributeNode' on 'Element': '{name_str}' not found"),
        ));
    };
    host.dom().remove_attribute(entity, &name_str);
    ctx.vm.invalidate_attr_cache_entry(entity, qname_sid);
    if let Some(state_mut) = ctx.vm.attr_states.get_mut(&attr_id) {
        state_mut.detached_value = Some(prev_sid);
    }
    // Return the same Attr — now detached with a snapshot of the
    // value at removal time.  Caller-side stashing for
    // reinsertion continues to work because `attr.value` reads
    // the snapshot.
    Ok(JsValue::Object(attr_id))
}

pub(super) fn native_element_toggle_attribute(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Boolean(false));
    };
    let name = coerce_first_arg_to_string(ctx, args)?;

    // `force` (second arg): undefined = toggle, true = ensure present,
    // false = ensure absent.  WHATWG §4.9.2 toggleAttribute.
    let force: Option<bool> = match args.get(1).copied().unwrap_or(JsValue::Undefined) {
        JsValue::Undefined => None,
        v => Some(super::super::coerce::to_boolean(ctx.vm, v)),
    };

    let currently_present = {
        let dom = ctx.host().dom();
        dom.world()
            .get::<&elidex_ecs::Attributes>(entity)
            .ok()
            .is_some_and(|attrs| attrs.contains(&name))
    };

    let final_present = match force {
        Some(true) => {
            if !currently_present {
                // WHATWG §4.9.2: when force=true and absent, set value to
                // empty string.
                attr_set(ctx, entity, &name, String::new());
            }
            true
        }
        Some(false) => {
            if currently_present {
                attr_remove(ctx, entity, &name);
            }
            false
        }
        None => {
            if currently_present {
                attr_remove(ctx, entity, &name);
                false
            } else {
                attr_set(ctx, entity, &name, String::new());
                true
            }
        }
    };
    Ok(JsValue::Boolean(final_present))
}

// ---------------------------------------------------------------------------
// id / className (reflected as the underlying attribute)
// ---------------------------------------------------------------------------

/// Shared body for reflected-string-attribute getters (`id`,
/// `className`).  Missing attribute returns the empty string (not
/// `null` like `getAttribute`) per WHATWG §4.9.
pub(super) fn reflected_string_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    attr_name: &str,
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    let val = attr_get(ctx, entity, attr_name).unwrap_or_default();
    if val.is_empty() {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    let sid = ctx.vm.strings.intern(&val);
    Ok(JsValue::String(sid))
}

/// Shared body for reflected-string-attribute setters.
pub(super) fn reflected_string_set(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
    attr_name: &str,
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };
    let value = coerce_first_arg_to_string(ctx, args)?;
    attr_set(ctx, entity, attr_name, value);
    Ok(JsValue::Undefined)
}

pub(super) fn native_element_get_id(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    reflected_string_get(ctx, this, "id")
}

pub(super) fn native_element_set_id(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    reflected_string_set(ctx, this, args, "id")
}

pub(super) fn native_element_get_class_name(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    reflected_string_get(ctx, this, "class")
}

pub(super) fn native_element_set_class_name(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    reflected_string_set(ctx, this, args, "class")
}
