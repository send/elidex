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
use super::dom_bridge::coerce_first_arg_to_string;
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
/// [`elidex_ecs::EcsDom::remove_attribute`].
pub(super) fn attr_remove(ctx: &mut NativeContext<'_>, entity: Entity, name: &str) {
    ctx.host().dom().remove_attribute(entity, name);
}

pub(super) fn native_element_get_attribute(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Null);
    };
    let name = coerce_first_arg_to_string(ctx, args)?;
    // Split the dom + strings borrow so `with_attribute` can intern
    // the borrowed `&str` without first allocating an owned `String`
    // through `attr_get`.  `entity_from_this` above already
    // short-circuited unbound receivers with `Null`, so the `None`
    // arm of `dom_and_strings_if_bound` is a defensive fallback only.
    let sid = match ctx.dom_and_strings_if_bound() {
        Some((dom, strings)) => dom.with_attribute(entity, &name, |v| v.map(|s| strings.intern(s))),
        None => None,
    };
    Ok(match sid {
        Some(sid) => JsValue::String(sid),
        None => JsValue::Null,
    })
}

pub(super) fn native_element_set_attribute(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };
    // Coerce BOTH args (name then value) per WebIDL even though the
    // spec name-validation step runs on a qualified name; we accept
    // any string here and defer validation to a future HTML5 parser
    // upgrade.
    let name = coerce_first_arg_to_string(ctx, args)?;
    let value_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let value_sid = super::super::coerce::to_string(ctx.vm, value_arg)?;
    let value = ctx.vm.strings.get_utf8(value_sid);
    attr_set(ctx, entity, &name, value);
    Ok(JsValue::Undefined)
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
/// the named attribute, or `null` when absent.
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
    let qname_sid = ctx.vm.strings.intern(&name);
    let attr_id = ctx.vm.alloc_attr(super::attr_proto::AttrState {
        owner: entity,
        qualified_name: qname_sid,
        detached_value: None,
    });
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
    let name_str = ctx.vm.strings.get_utf8(qname_sid);
    // Mirror `Attr.prototype.value`: detached snapshot first,
    // else the owner's current attribute value.  Without the
    // snapshot branch, `element.setAttributeNode(detachedAttr)`
    // would write empty / stale data instead of the attribute
    // value the author observed on the source Attr.
    let new_value = if let Some(snapshot_sid) = source_detached {
        ctx.vm.strings.get_utf8(snapshot_sid)
    } else {
        ctx.host()
            .dom()
            .get_attribute(source_owner, &name_str)
            .unwrap_or_default()
    };
    // Snapshot the prev value BEFORE overwriting so the returned
    // detached Attr observes the replaced value, not the
    // just-written one (WHATWG §4.9.2).
    let prev_value: Option<String> = ctx.host().dom().get_attribute(entity, &name_str);
    ctx.host().dom().set_attribute(entity, &name_str, new_value);
    Ok(if let Some(prev_val) = prev_value {
        let prev_sid = if prev_val.is_empty() {
            ctx.vm.well_known.empty
        } else {
            ctx.vm.strings.intern(&prev_val)
        };
        let prev = ctx.vm.alloc_attr(super::attr_proto::AttrState {
            owner: entity,
            qualified_name: qname_sid,
            detached_value: Some(prev_sid),
        });
        JsValue::Object(prev)
    } else {
        JsValue::Null
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
    let Some(prev_value) = ctx.host().dom().get_attribute(entity, &name_str) else {
        let not_found = ctx.vm.well_known.dom_exc_not_found_error;
        return Err(VmError::dom_exception(
            not_found,
            format!("Failed to execute 'removeAttributeNode' on 'Element': '{name_str}' not found"),
        ));
    };
    // Detach-snapshot the prior value + update the passed Attr's
    // state to detached before mutating the element, so the
    // passed-in wrapper itself sees the detached view afterward
    // (WHATWG §4.9.2 "remove an attribute" mutates the Attr being
    // removed).
    let prev_sid = if prev_value.is_empty() {
        ctx.vm.well_known.empty
    } else {
        ctx.vm.strings.intern(&prev_value)
    };
    if let Some(state_mut) = ctx.vm.attr_states.get_mut(&attr_id) {
        state_mut.detached_value = Some(prev_sid);
    }
    ctx.host().dom().remove_attribute(entity, &name_str);
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
