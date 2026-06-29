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

use super::super::value::{JsValue, NativeContext, ObjectId, ObjectKind, StringId, VmError};
use super::super::wrapper_intern::{WrapperKey, WrapperKind};
use super::dom_bridge::{
    coerce_first_arg_to_string, coerce_first_arg_to_string_id, invoke_dom_api,
};
use super::event_target::entity_from_this;

use elidex_ecs::Entity;
use elidex_script_session::{apply_remove_attribute, apply_set_attribute};

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

/// HTML enumerated-attribute reflection helper (WebIDL `attribute
/// DOMString`, content-attribute is enumerated): read `attr` from
/// `entity`, lowercase the raw value, and return the canonical
/// keyword if any of `valid` matches.  Otherwise return `default`.
///
/// `default` is the *missing- and invalid-value default*, which the
/// spec defines per attribute:
///
/// - `<form>.method`: default `"get"`,
/// - `<form>.enctype`: default `"application/x-www-form-urlencoded"`,
/// - `<form>.autocomplete`: default `"on"`,
/// - `<button>.type`: default `"submit"`,
/// - submit-button overrides (`<button>.formMethod` /
///   `<input>.formMethod` / `<button>.formEnctype` /
///   `<input>.formEnctype`, HTML §4.10.5.4): default `""` —
///   distinct from the form-level case, where these surfaces are
///   "no override" sentinels rather than form defaults.
pub(super) fn enumerated_attr_reflect(
    ctx: &mut NativeContext<'_>,
    entity: Entity,
    attr: &str,
    valid: &[&'static str],
    default: &'static str,
) -> super::super::value::StringId {
    let raw = ctx
        .host()
        .dom()
        .get_attribute(entity, attr)
        .unwrap_or_default()
        .to_ascii_lowercase();
    let canonical: &str = valid
        .iter()
        .copied()
        .find(|v| v == &raw.as_str())
        .unwrap_or(default);
    ctx.vm.strings.intern(canonical)
}

/// Set attribute `name` = `value` on `entity`, emitting the WHATWG DOM
/// §4.9 "attributes" MutationObserver record. The record-producing
/// convergence point for **every** reflected IDL setter in `vm/host/`
/// (B2-Slice-2): the write routes through the shared
/// [`elidex_script_session::apply_set_attribute`] primitive, which calls the
/// `EcsDom::set_attribute` chokepoint (full fan-out preserved) and builds the
/// step-1 record from the surfaced pre-write `oldValue`. Returns `false` when
/// the entity has been destroyed / the host is unbound (no write landed).
pub(super) fn attr_set(
    ctx: &mut NativeContext<'_>,
    entity: Entity,
    name: &str,
    value: &str,
) -> bool {
    let record = ctx
        .host_if_bound()
        .and_then(|host| apply_set_attribute(host.dom(), entity, name, value));
    let did_set = record.is_some();
    // Commit (push + drain as one indivisible step) — the reflected setters
    // bypass `invoke_dom_api`, so they self-commit through the shared
    // `commit_notify_records` chokepoint (which binds the push to the drain so a
    // record can't be stranded for the flush leak-guard; plan §4.1 / I9).
    ctx.vm.commit_notify_records(record.into_iter().collect());
    did_set
}

/// VM-local `Attr`-wrapper bookkeeping captured BEFORE a wrapper-aware
/// attribute removal, applied by [`freeze_detached_attr_wrapper`] AFTER the
/// removal lands. This is the *marshalling* half of an attribute removal that
/// the engine-independent `removeAttribute` / `toggleAttribute` handler cannot
/// do — it has no access to the per-VM JS wrapper cache / `attr_states`.
struct AttrWrapperSnapshot {
    /// `intern(name)` — the wrapper-cache key (`getAttributeNode`,
    /// `nnm.{item,getNamedItem,…}` all cache under `intern(utf8)`, so this
    /// re-intern lands on the same `StringId` and the invalidation is observed).
    qname_sid: StringId,
    /// The JS-held `Attr` wrapper for `(entity, qname_sid)`, if any.
    cached_attr_id: Option<ObjectId>,
    /// The attribute's interned value at snapshot time (`None` = unbound OR the
    /// attribute was already absent — both skip the freeze).
    prev_sid: Option<StringId>,
}

/// Capture the [`AttrWrapperSnapshot`] for a pending removal of `name` on
/// `entity` (`qname_sid == intern(name)`, passed in so the caller can reuse it
/// for the handler dispatch). Snapshots the prior value via the disjoint
/// DOM/strings split borrow (intern lands on the borrowed `&str`, no
/// `String::from` clone) and probes the wrapper cache — `.copied()` drops the
/// `&ObjectId` so a later `attr_states.get_mut` is conflict-free.
fn snapshot_attr_wrapper(
    ctx: &mut NativeContext<'_>,
    entity: Entity,
    name: &str,
    qname_sid: StringId,
) -> AttrWrapperSnapshot {
    let empty = ctx.vm.well_known.empty;
    let prev_sid = ctx.dom_and_strings_if_bound().and_then(|(dom, strings)| {
        dom.with_attribute(entity, name, |v| {
            v.map(|s| strings.intern_or_alias(empty, s))
        })
    });
    let cached_attr_id = ctx.vm.get_wrapper(WrapperKey::entity_named(
        entity,
        WrapperKind::Attr,
        qname_sid,
    ));
    AttrWrapperSnapshot {
        qname_sid,
        cached_attr_id,
        prev_sid,
    }
}

/// Freeze any JS-held `Attr` wrapper at its removal-time value + invalidate the
/// wrapper cache, AFTER the removal has landed. Matches WHATWG DOM §4.9.2 +
/// Chrome / Firefox: through a `removeAttribute(name)` → optional same-name
/// `setAttribute` cycle, a previously-cached `Attr`'s `.value` keeps reporting
/// the value the attribute held when it was removed (without the snapshot the
/// JS-held Attr would read live DOM state and appear to re-attach to the new
/// write). `attr_set` deliberately does NOT snapshot — same-name value writes
/// preserve Attr identity, so removal is the only observable detach point.
fn freeze_detached_attr_wrapper(
    ctx: &mut NativeContext<'_>,
    entity: Entity,
    snap: &AttrWrapperSnapshot,
) {
    if let (Some(attr_id), Some(prev_sid)) = (snap.cached_attr_id, snap.prev_sid) {
        if let Some(state_mut) = ctx.vm.attr_states.get_mut(&attr_id) {
            state_mut.detached_value = Some(prev_sid);
        }
    }
    ctx.vm.invalidate_attr_cache_entry(entity, snap.qname_sid);
}

/// Remove attribute `name` from `entity` through the `EcsDom` chokepoint while
/// keeping any JS-held `Attr` wrapper in sync (snapshot → remove → freeze) and
/// emitting the WHATWG DOM §4.9 "attributes" MutationObserver record.
///
/// This is the wrapper-aware removal helper the **reflected boolean-attribute
/// detach** sites (`el.hidden = false`, `<input>.checked = false`, …) route
/// through; B2-Slice-2 makes it record-producing by routing the chokepoint
/// remove through [`elidex_script_session::apply_remove_attribute`] (records
/// only when something was removed — remove-of-absent → `None` → no record,
/// I11). The generic `removeAttribute` / `toggleAttribute(off)` natives use the
/// record-producing `invoke_dom_api` path instead (B2-Slice-1, F2), reusing the
/// same [`snapshot_attr_wrapper`] / [`freeze_detached_attr_wrapper`]
/// marshalling. Freeze (VM wrapper state) and drain (microtask queue) are
/// independent, so drain after freeze (plan §4.1 / I9).
pub(super) fn attr_remove(ctx: &mut NativeContext<'_>, entity: Entity, name: &str) {
    let qname_sid = ctx.vm.strings.intern(name);
    let snap = snapshot_attr_wrapper(ctx, entity, name, qname_sid);
    // snapshot → remove → freeze → commit. Post-unbind callers no-op (matching
    // the snapshot's `None` fall-through). The record is committed AFTER the
    // freeze: the freeze is VM wrapper state and the commit's drain is the
    // microtask queue, so their order is independent, but the snapshot→remove→
    // freeze sequence must be preserved (plan §4.1 / I9).
    let record = ctx
        .host_if_bound()
        .and_then(|host| apply_remove_attribute(host.dom(), entity, name));
    freeze_detached_attr_wrapper(ctx, entity, &snap);
    ctx.vm.commit_notify_records(record.into_iter().collect());
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
    // Lowercase the name so the VM-local Attr-wrapper snapshot/invalidation below
    // keys on the SAME name the `removeAttribute` handler removes — else
    // `removeAttribute('ID')` removes `id` via the handler but a raw-keyed snapshot
    // misses a cached `getAttributeNode('id')` wrapper (Codex R1). The handler
    // currently lowercases UNCONDITIONALLY (all elements); HTML-namespace-gating
    // the lowercase consistently across the whole attribute IDL surface (so SVG/
    // MathML case-preserved names survive) is deferred WHOLE to slot
    // `#11-attribute-name-html-namespace-casing` (plan §9).
    let name = coerce_first_arg_to_string(ctx, args)?.to_ascii_lowercase();
    // B2-Slice-1 / F2: route the removal through the record-producing
    // `removeAttribute` handler (chokepoint remove + §4.9 "attributes" record
    // + `AttrEntityCache` evict + record drain) instead of the bare
    // `attr_remove` chokepoint shim. The VM-local Attr-wrapper snapshot stays
    // here — the engine-independent handler cannot touch the per-VM
    // `attr_states` / wrapper cache (#399: identity bookkeeping is VM-side
    // marshalling): snapshot before, freeze after.
    let qname_sid = ctx.vm.strings.intern(&name);
    let snap = snapshot_attr_wrapper(ctx, entity, &name, qname_sid);
    invoke_dom_api(
        ctx,
        "removeAttribute",
        entity,
        &[JsValue::String(qname_sid)],
    )?;
    freeze_detached_attr_wrapper(ctx, entity, &snap);
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

/// `element.classList` getter — return an identity-preserving
/// [`crate::vm::value::ObjectKind::DOMTokenList`] wrapper backed by
/// the element's `class` attribute (WHATWG DOM §3.5).  Repeated
/// reads return the same `ObjectId` via
/// [`crate::vm::VmInner::alloc_or_cached_class_list`].
pub(super) fn native_element_get_class_list(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Null);
    };
    let id = ctx.vm.alloc_or_cached_class_list(entity);
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
    host.dom().set_attribute(entity, &name_str, &new_value);
    // Sync the identity cache for `(entity, qname_sid)`:
    // - Same-element source (`source_owner == entity`), whether live
    //   or detached: insert/refresh so reattachment after a prior
    //   `removeAttribute` keeps `el.getAttributeNode(name) === a`.
    //   The snapshot-on-`removeAttribute` path
    //   (`super::element_attrs::attr_remove`) sets
    //   `detached_value` on the cached wrapper for Chrome / Firefox
    //   `attr.value` parity; reattaching to the original owner
    //   revives the wrapper by clearing the snapshot so subsequent
    //   reads track the live attribute again.
    // - Cross-element source: the engine path doesn't retarget the
    //   passed-in Attr's `AttrState.owner` (Phase 2 limitation), so
    //   drop the cache entry instead and let the next
    //   `getAttributeNode` allocate a fresh canonical wrapper.
    if source_owner == entity {
        if source_detached.is_some() {
            if let Some(state_mut) = ctx.vm.attr_states.get_mut(&attr_id) {
                state_mut.detached_value = None;
            }
        }
        ctx.vm.set_wrapper(
            WrapperKey::entity_named(entity, WrapperKind::Attr, qname_sid),
            attr_id,
        );
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
    // Lowercase the name so the `currently_present` probe + the Attr-wrapper
    // detach snapshot below key on the SAME name the `toggleAttribute` handler
    // operates on — else `toggleAttribute('ID')` removes `id` via the handler but
    // a raw-keyed precheck/snapshot misses a cached `getAttributeNode('id')`
    // wrapper (Codex R1). The handler currently lowercases UNCONDITIONALLY (all
    // elements); HTML-namespace-gating the lowercase consistently across the whole
    // attribute IDL surface (so SVG/MathML case-preserved names survive) is
    // deferred WHOLE to slot `#11-attribute-name-html-namespace-casing` (plan §9).
    let name = coerce_first_arg_to_string(ctx, args)?.to_ascii_lowercase();

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

    let qname_sid = ctx.vm.strings.intern(&name);
    // If this toggle may REMOVE the attribute (present + not a force-add),
    // snapshot the JS-held Attr wrapper BEFORE the handler removes it — the
    // VM-local marshalling the engine-independent handler cannot do (mirrors
    // `attr_remove` / `native_element_remove_attribute`).
    let detach_snapshot = (currently_present && force != Some(true))
        .then(|| snapshot_attr_wrapper(ctx, entity, &name, qname_sid));

    // F2 (B2-Slice-1): converge onto the engine-independent `toggleAttribute`
    // handler — the §4.9 toggle algorithm + the "attributes" MutationObserver
    // record + record drain — instead of re-implementing the force /
    // present-check / set-remove dance here via the record-less `attr_set` /
    // `attr_remove` shims. The handler returns the final presence (Boolean).
    let mut handler_args = vec![JsValue::String(qname_sid)];
    if let Some(force) = force {
        handler_args.push(JsValue::Boolean(force));
    }
    let result = invoke_dom_api(ctx, "toggleAttribute", entity, &handler_args)?;

    // If the toggle actually removed the attribute (was present, now absent),
    // freeze the snapshotted Attr wrapper at its removal-time value.
    if let Some(snap) = detach_snapshot {
        if matches!(result, JsValue::Boolean(false)) {
            freeze_detached_attr_wrapper(ctx, entity, &snap);
        }
    }
    Ok(result)
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
    attr_set(ctx, entity, attr_name, &value);
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
