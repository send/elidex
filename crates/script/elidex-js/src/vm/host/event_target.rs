//! `EventTarget.prototype` intrinsic — holds the three native methods
//! (`addEventListener`, `removeEventListener`, `dispatchEvent`) that every
//! DOM wrapper inherits.
//!
//! ## Why a shared prototype?
//!
//! The alternative — registering the three methods directly on each
//! element wrapper at creation time — would allocate N × 3 native-function
//! objects for N elements.  A single shared prototype matches the spec
//! (WHATWG DOM §2.7 `EventTarget` interface) and aligns with how
//! `Promise.prototype` / `Array.prototype` are structured elsewhere in
//! the VM.
//!
//! ## Method status
//!
//! - `addEventListener` — fully implemented (PR3 C7); options-object
//!   form, capture/once/passive, duplicate check, ECS + listener_store
//!   wiring.
//! - `removeEventListener` — fully implemented (PR3 C8); WHATWG
//!   §2.7.7 (type, callback, capture) match + ECS / listener_store
//!   cleanup, options parsed via `parse_capture_only` (no spurious
//!   `once`/`passive` getter calls — see PR3 R7).
//! - `dispatchEvent` — **deferred to PR5a** alongside `Event`
//!   constructors, which are the only meaningful way to pass a
//!   JS-constructed event into a synchronous dispatch from script.
//!   Until then the stub returns `false` (the spec default for
//!   "event not dispatched"); the method itself is still resolvable
//!   via the prototype chain, which is enough for scripts that only
//!   feature-test its existence.

#[cfg(feature = "engine")]
use super::super::shape;
use super::super::value::{JsValue, NativeContext, VmError};
#[cfg(feature = "engine")]
use super::super::value::{ObjectId, ObjectKind, PropertyKey, PropertyValue};
use super::super::{NativeFn, VmInner};

impl VmInner {
    /// Populate `self.event_target_prototype` with the `EventTarget`
    /// interface methods (WHATWG DOM §2.7).
    ///
    /// Called from `register_globals()` after `Object.prototype` is in
    /// place (every DOM wrapper's prototype chain terminates in
    /// `Object.prototype`, so this intrinsic sits one level above it).
    ///
    /// The three method bodies are **stubs** at C0 — see module doc for
    /// the per-method replacement schedule.
    pub(in crate::vm) fn register_event_target_prototype(&mut self) {
        let proto_id = self.create_object_with_methods(&[
            (
                "addEventListener",
                native_event_target_add_event_listener as NativeFn,
            ),
            (
                "removeEventListener",
                native_event_target_remove_event_listener,
            ),
            ("dispatchEvent", native_event_target_dispatch_event),
        ]);
        self.event_target_prototype = Some(proto_id);
        // Node-level accessors (`parentNode`, `nodeType`, …) live on
        // `EventTarget.prototype` too so both Element and Text wrappers
        // surface them — see the module doc comment below.
        #[cfg(feature = "engine")]
        self.install_node_accessors_on_event_target();
    }
}

/// `EventTarget.prototype.addEventListener` — non-engine-feature stub.
///
/// Without the `engine` feature there is no `HostData` / DOM to record
/// the listener against; addEventListener becomes a no-op so JS that
/// merely feature-tests the method's existence still works.
#[cfg(not(feature = "engine"))]
pub(super) fn native_event_target_add_event_listener(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Undefined)
}

/// `EventTarget.prototype.addEventListener(type, listener, options)`.
///
/// WHATWG DOM §2.6 / §2.7.  Behaviour:
///
/// 1. `this` must be a `HostObject` — extract the entity.  Calls on
///    non-HostObject receivers are silent no-ops, matching browsers'
///    "doesn't do anything visible" handling for objects that
///    feature-test the prototype method.
/// 2. `type` is `ToString(arg[0])`.
/// 3. `listener` (`arg[1]`) — `null`/`undefined` returns silently
///    (spec §2.6 step 2: "If callback is null, then return.").
///    Otherwise must be callable; non-callable throws `TypeError`.
/// 4. `options` (`arg[2]`):
///    - boolean → `capture` flag.
///    - object → `{capture, once, passive}` properties read as
///      booleans.  Missing keys default to `false`.
///    - undefined / missing → all flags `false`.
/// 5. Duplicate check (§2.6 step 4): `(type, callback, capture)` —
///    `once` and `passive` are NOT part of the identity tuple.  A
///    second `addEventListener` with the same triple is silently
///    discarded.
/// 6. The new listener is recorded in two places:
///    - ECS `EventListeners` component on `entity` (metadata: type,
///      capture, once, passive).
///    - `HostData::listener_store` (`ListenerId` → JS function
///      `ObjectId`).  Both are GC-rooted via
///      `HostData::gc_root_object_ids()`.
///
/// Deferred:
/// - `signal` option (AbortSignal) → PR4 once `AbortController` lands.
/// - Object-form callback with `handleEvent` method (§2.7 step 8) →
///   not yet a hot-path.
#[cfg(feature = "engine")]
pub(super) fn native_event_target_add_event_listener(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };

    // Argument 1: callback — null/undefined silently returns.
    let callback_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let listener_obj_id = match callback_arg {
        JsValue::Null | JsValue::Undefined => return Ok(JsValue::Undefined),
        JsValue::Object(id) if ctx.vm.get_object(id).kind.is_callable() => id,
        _ => {
            return Err(VmError::type_error(
                "Failed to execute 'addEventListener' on 'EventTarget': \
                 parameter 2 is not of type 'EventListener'.",
            ));
        }
    };

    // Argument 0: type — coerced via ToString (a non-string type
    // argument is rare but spec-allowed).
    let type_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let type_sid = super::super::coerce::to_string(ctx.vm, type_arg)?;
    // Materialise the type as a Rust String for ECS storage; the
    // ECS component owns it independently of the StringPool's WTF-16
    // representation.
    let event_type = ctx.vm.strings.get_utf8(type_sid);

    // Argument 2: options — boolean (capture only) or object form.
    let options = parse_listener_options(ctx, args.get(2).copied().unwrap_or(JsValue::Undefined))?;

    // Duplicate check: WHATWG §2.6 step 4 excludes `once` / `passive`
    // from identity, so we look up by (type, capture, callback).
    if find_listener_id(ctx, entity, &event_type, options.capture, listener_obj_id).is_some() {
        return Ok(JsValue::Undefined);
    }

    // Register in the ECS component (creating the component if absent).
    // `insert_one` can fail if the entity has been despawned between
    // `entity_from_this`'s extraction and now (e.g. a concurrent DOM
    // mutation observer removed it).  In that case, skip the
    // listener_store insert entirely — storing it would create an
    // orphan entry that ECS could never reach for dispatch and that
    // would hold the JS function ObjectId rooted via gc until the
    // VM dies.
    let listener_id: Option<elidex_script_session::ListenerId> = {
        let dom = ctx.host().dom();
        if dom
            .world()
            .get::<&elidex_script_session::EventListeners>(entity)
            .is_ok()
        {
            Some(
                dom.world_mut()
                    .get::<&mut elidex_script_session::EventListeners>(entity)
                    .expect("just-checked component must exist")
                    .add_with_options(event_type, options.capture, options.once, options.passive),
            )
        } else {
            let mut listeners = elidex_script_session::EventListeners::new();
            let id = listeners.add_with_options(
                event_type,
                options.capture,
                options.once,
                options.passive,
            );
            if dom.world_mut().insert_one(entity, listeners).is_ok() {
                Some(id)
            } else {
                None
            }
        }
    };

    let Some(listener_id) = listener_id else {
        return Ok(JsValue::Undefined);
    };

    // Stash the JS function ObjectId so dispatch can look it up.
    ctx.host().store_listener(listener_id, listener_obj_id);

    Ok(JsValue::Undefined)
}

/// Extract the ECS Entity from `this` if it is a `HostObject` wrapper
/// AND `HostData` is currently bound.  Returns `None` (silent no-op)
/// for either failure mode, so callers can early-return without
/// further conditionals.
///
/// The bound-check covers a real scenario: JS code retains a
/// `HostObject` wrapper (e.g. saves `document` to a global, then
/// `Vm::unbind()` runs at end of the tick) and later invokes a
/// method on it across the unbind boundary.  Without the check,
/// the subsequent `host.dom()` call would panic on null pointers.
/// Treating it as a no-op matches browser behaviour for unattached
/// document references.
#[cfg(feature = "engine")]
pub(super) fn entity_from_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
) -> Option<elidex_ecs::Entity> {
    if !ctx.vm.host_data.as_deref().is_some_and(|h| h.is_bound()) {
        return None;
    }
    let JsValue::Object(id) = this else {
        return None;
    };
    let ObjectKind::HostObject { entity_bits } = ctx.vm.get_object(id).kind else {
        return None;
    };
    elidex_ecs::Entity::from_bits(entity_bits)
}

/// Parsed listener options — capture / once / passive.  AbortSignal is
/// not represented yet (PR4).
#[cfg(feature = "engine")]
#[derive(Default)]
pub(super) struct ListenerOptions {
    pub(super) capture: bool,
    pub(super) once: bool,
    pub(super) passive: bool,
}

/// WHATWG DOM §2.6 step 1 — extract listener flags from the third
/// argument of add/removeEventListener.
///
/// - undefined / missing → all flags false.
/// - boolean → capture only (legacy form).
/// - object  → read `capture`, `once`, `passive` as booleans (missing
///   keys default to false).
#[cfg(feature = "engine")]
fn parse_listener_options(
    ctx: &mut NativeContext<'_>,
    val: JsValue,
) -> Result<ListenerOptions, VmError> {
    match val {
        JsValue::Undefined | JsValue::Null => Ok(ListenerOptions::default()),
        JsValue::Boolean(capture) => Ok(ListenerOptions {
            capture,
            ..Default::default()
        }),
        JsValue::Object(opts_id) => {
            // Property names pre-interned in `WellKnownStrings` to avoid
            // a HashMap lookup per addEventListener options object.
            let mut out = ListenerOptions::default();
            for (key_sid, slot) in [
                (ctx.vm.well_known.capture, &mut out.capture),
                (ctx.vm.well_known.once, &mut out.once),
                (ctx.vm.well_known.passive, &mut out.passive),
            ] {
                let v = ctx
                    .vm
                    .get_property_value(opts_id, PropertyKey::String(key_sid))?;
                *slot = super::super::coerce::to_boolean(ctx.vm, v);
            }
            Ok(out)
        }
        // Other types coerce via ToBoolean per WHATWG (treated as
        // capture flag).  Browsers accept e.g. `true === 1` style
        // calls; mirror that.
        other => Ok(ListenerOptions {
            capture: super::super::coerce::to_boolean(ctx.vm, other),
            ..Default::default()
        }),
    }
}

/// Extract ONLY the `capture` flag from `removeEventListener`'s
/// third argument.
///
/// WHATWG DOM §2.7.7's "flatten options" step for removal is
/// narrower than addEventListener's: it only consults the `capture`
/// property.  Reading `once` / `passive` (which `parse_listener_options`
/// does for addEventListener) would be observable through user
/// getters or Proxy traps, deviating from browser behaviour.  This
/// helper preserves that observability invariant by touching only
/// the capture slot.
#[cfg(feature = "engine")]
fn parse_capture_only(ctx: &mut NativeContext<'_>, val: JsValue) -> Result<bool, VmError> {
    match val {
        JsValue::Undefined | JsValue::Null => Ok(false),
        JsValue::Boolean(capture) => Ok(capture),
        JsValue::Object(opts_id) => {
            let key_sid = ctx.vm.well_known.capture;
            let v = ctx
                .vm
                .get_property_value(opts_id, PropertyKey::String(key_sid))?;
            Ok(super::super::coerce::to_boolean(ctx.vm, v))
        }
        // Other primitive types coerce via ToBoolean per WHATWG —
        // browsers accept e.g. `el.removeEventListener('click', f, 1)`.
        other => Ok(super::super::coerce::to_boolean(ctx.vm, other)),
    }
}

/// Find the `ListenerId` on `entity` whose (type, capture, callback)
/// triple matches the given arguments — returns the id if such a
/// listener is registered, `None` otherwise.
///
/// Used by both `addEventListener` (`.is_some()` for the §2.6 step 4
/// duplicate check) and `removeEventListener` (the actual id to drop).
/// Keeping a single helper prevents the two sites from drifting on
/// what counts as a "match" — historically `removeEventListener`
/// picked the first (type, capture) entry and bailed if its callback
/// didn't match, missing later matching entries when an element had
/// multiple listeners of the same type+capture.
///
/// **Precondition**: caller must have verified `HostData` is bound
/// (typically by passing `entity` from a successful
/// [`entity_from_this`] call).  This helper calls `ctx.host().dom()`
/// directly, which would panic on a null dom pointer.
#[cfg(feature = "engine")]
fn find_listener_id(
    ctx: &mut NativeContext<'_>,
    entity: elidex_ecs::Entity,
    event_type: &str,
    capture: bool,
    candidate_obj_id: ObjectId,
) -> Option<elidex_script_session::ListenerId> {
    // Two-pass: collect candidate ids while holding the world borrow
    // (scoped to the inner block), then cross-reference each against
    // `listener_store` (which goes through `host_data` and would
    // otherwise conflict with the world borrow).
    let candidate_ids: Vec<_> = {
        let dom = ctx.host().dom();
        dom.world()
            .get::<&elidex_script_session::EventListeners>(entity)
            .ok()
            .map(|listeners| {
                // `iter_matching` skips the `Vec<&ListenerEntry>`
                // intermediate alloc that `matching_all` would
                // require — `addEventListener` (duplicate check) and
                // `removeEventListener` (find target) call this on
                // every invocation, so the alloc savings add up.
                listeners
                    .iter_matching(event_type)
                    .filter(|e| e.capture == capture)
                    .map(|e| e.id)
                    .collect()
            })
            .unwrap_or_default()
    };
    let host = ctx.host();
    candidate_ids
        .into_iter()
        .find(|id| host.get_listener(*id) == Some(candidate_obj_id))
}

/// `EventTarget.prototype.removeEventListener` — non-engine-feature stub.
#[cfg(not(feature = "engine"))]
pub(super) fn native_event_target_remove_event_listener(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Undefined)
}

/// `EventTarget.prototype.removeEventListener(type, listener, options)`.
///
/// WHATWG DOM §2.7.7: locate any listener whose (type, callback,
/// capture) tuple matches and remove it from the entity's
/// `EventListeners` component plus `HostData::listener_store`.
///
/// Behaviour parallels [`native_event_target_add_event_listener`]:
/// - Non-HostObject `this` → silent no-op.
/// - `null` / `undefined` callback → silent no-op (§2.7.7 step 2).
/// - Non-callable callback → silent no-op (cannot match anything,
///   spec just no-ops here too — only addEventListener throws).
/// - Options third arg parsed identically (only `capture` is
///   meaningful for removal — `once` / `passive` are not part of
///   identity per §2.6 step 4).
#[cfg(feature = "engine")]
pub(super) fn native_event_target_remove_event_listener(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };

    // WebIDL `EventListener? callback`: null/undefined are explicitly
    // allowed and become a silent no-op (§2.7.7 step 2); any other
    // non-callable value is a conversion failure that throws
    // `TypeError`, matching `addEventListener` and browser behaviour
    // (silently dropping `el.removeEventListener('click', 42)` would
    // mask user bugs).
    let callback_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let listener_obj_id = match callback_arg {
        JsValue::Null | JsValue::Undefined => return Ok(JsValue::Undefined),
        JsValue::Object(id) if ctx.vm.get_object(id).kind.is_callable() => id,
        _ => {
            return Err(VmError::type_error(
                "Failed to execute 'removeEventListener' on 'EventTarget': \
                 parameter 2 is not of type 'EventListener'.",
            ));
        }
    };

    let type_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let type_sid = super::super::coerce::to_string(ctx.vm, type_arg)?;
    let event_type = ctx.vm.strings.get_utf8(type_sid);

    // Spec §2.7.7: only `capture` is read from options for removal.
    // `parse_listener_options` would also read `once` / `passive`,
    // which would fire user getters / Proxy traps that browsers
    // don't trigger here.
    let capture = parse_capture_only(ctx, args.get(2).copied().unwrap_or(JsValue::Undefined))?;

    // Locate the matching listener via the shared (type, capture,
    // callback) lookup.  WHATWG §2.6 step 4 forbids duplicates so at
    // most one match exists; if none matches, it's a silent no-op.
    let Some(listener_id) = find_listener_id(ctx, entity, &event_type, capture, listener_obj_id)
    else {
        return Ok(JsValue::Undefined);
    };

    // Remove from ECS component first (scoped block so the world
    // borrow is released before we re-grab `host` for listener_store
    // cleanup), then from listener_store.
    {
        let dom = ctx.host().dom();
        if let Ok(mut listeners) = dom
            .world_mut()
            .get::<&mut elidex_script_session::EventListeners>(entity)
        {
            listeners.remove(listener_id);
        }
    }
    ctx.host().remove_listener(listener_id);

    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// Node-common accessors (WHATWG DOM §4.4)
// ---------------------------------------------------------------------------
//
// These live on `EventTarget.prototype` rather than a dedicated
// `Node.prototype` because elidex does not yet distinguish Node from
// EventTarget in its prototype chain.  Keeping them here avoids
// introducing a third prototype level while preserving the invariant
// that both Element and Text wrappers expose `parentNode`, `nodeType`,
// `textContent`, etc. — the Node-level APIs that are valid on any DOM
// node, independent of whether the node is an Element.
//
// Element-specific members (`children`, `getAttribute`, …) live on
// the separate `Element.prototype` (`vm/host/element_proto.rs`),
// which chains to `EventTarget.prototype`; Text / Comment wrappers
// therefore do NOT see those members.

#[cfg(feature = "engine")]
pub(super) fn native_node_get_parent_node(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Null);
    };
    let parent = ctx.host().dom().get_parent(entity);
    Ok(match parent {
        Some(p) => JsValue::Object(ctx.vm.create_element_wrapper(p)),
        None => JsValue::Null,
    })
}

#[cfg(feature = "engine")]
pub(super) fn native_node_get_next_sibling(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Null);
    };
    let sib = ctx.host().dom().get_next_sibling(entity);
    Ok(match sib {
        Some(s) => JsValue::Object(ctx.vm.create_element_wrapper(s)),
        None => JsValue::Null,
    })
}

#[cfg(feature = "engine")]
pub(super) fn native_node_get_previous_sibling(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Null);
    };
    let sib = ctx.host().dom().get_prev_sibling(entity);
    Ok(match sib {
        Some(s) => JsValue::Object(ctx.vm.create_element_wrapper(s)),
        None => JsValue::Null,
    })
}

/// `Node.prototype.nodeType` — WHATWG DOM §4.4.
///
/// Returns the numeric `NodeKind::node_type()` or `0` when the entity
/// has no `NodeKind` component (e.g. the raw Window placeholder before
/// bind).
#[cfg(feature = "engine")]
pub(super) fn native_node_get_node_type(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Number(0.0));
    };
    let kind = ctx.host().dom().node_kind(entity);
    let n = kind.map_or(0, elidex_ecs::NodeKind::node_type);
    Ok(JsValue::Number(f64::from(n)))
}

/// `Node.prototype.nodeName` — WHATWG DOM §4.4.
///
/// - Element: upper-case tag name.
/// - Text: `"#text"`.
/// - Comment: `"#comment"`.
/// - Document: `"#document"`.
/// - DocumentFragment: `"#document-fragment"`.
/// - Everything else: `""` (best-effort; no `#document-type` support yet).
#[cfg(feature = "engine")]
pub(super) fn native_node_get_node_name(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    let name: String = {
        let dom = ctx.host().dom();
        if let Some(tag) = dom.get_tag_name(entity) {
            tag.to_ascii_uppercase()
        } else {
            match dom.node_kind(entity) {
                Some(elidex_ecs::NodeKind::Text) => "#text".to_string(),
                Some(elidex_ecs::NodeKind::Comment) => "#comment".to_string(),
                Some(elidex_ecs::NodeKind::Document) => "#document".to_string(),
                Some(elidex_ecs::NodeKind::DocumentFragment) => "#document-fragment".to_string(),
                _ => String::new(),
            }
        }
    };
    if name.is_empty() {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    let sid = ctx.vm.strings.intern(&name);
    Ok(JsValue::String(sid))
}

/// `Node.prototype.nodeValue` — WHATWG DOM §4.4.
///
/// For character-data nodes (Text, Comment), returns their data;
/// for everything else, returns `null`.
#[cfg(feature = "engine")]
pub(super) fn native_node_get_node_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Null);
    };
    let data: Option<String> = {
        let dom = ctx.host().dom();
        if let Ok(text) = dom.world().get::<&elidex_ecs::TextContent>(entity) {
            Some(text.0.clone())
        } else if let Ok(c) = dom.world().get::<&elidex_ecs::CommentData>(entity) {
            Some(c.0.clone())
        } else {
            None
        }
    };
    match data {
        Some(s) => {
            let sid = ctx.vm.strings.intern(&s);
            Ok(JsValue::String(sid))
        }
        None => Ok(JsValue::Null),
    }
}

/// `Node.prototype.nodeValue` setter — spec-defined only for
/// character-data (Text / Comment) nodes; no-op otherwise.
#[cfg(feature = "engine")]
pub(super) fn native_node_set_node_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };
    let arg = args.first().copied().unwrap_or(JsValue::Undefined);
    // WHATWG §4.4: nodeValue setter treats null as empty string; every
    // other value is coerced via ToString.
    let data: String = match arg {
        JsValue::Null => String::new(),
        other => {
            let sid = super::super::coerce::to_string(ctx.vm, other)?;
            ctx.vm.strings.get_utf8(sid)
        }
    };
    {
        let dom = ctx.host().dom();
        let is_text = dom.world().get::<&elidex_ecs::TextContent>(entity).is_ok();
        if is_text {
            if let Ok(mut text) = dom.world_mut().get::<&mut elidex_ecs::TextContent>(entity) {
                text.0 = data;
            }
        } else if dom.world().get::<&elidex_ecs::CommentData>(entity).is_ok() {
            if let Ok(mut c) = dom.world_mut().get::<&mut elidex_ecs::CommentData>(entity) {
                c.0 = data;
            }
        }
    }
    Ok(JsValue::Undefined)
}

/// `Node.prototype.textContent` getter — WHATWG DOM §4.4.
///
/// - Element / DocumentFragment: concatenation of descendant Text
///   node data (pre-order DFS, excluding shadow roots).
/// - Text / Comment: own data.
/// - Everything else: `null`.
#[cfg(feature = "engine")]
pub(super) fn native_node_get_text_content(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Null);
    };
    let result: Option<String> = {
        let dom = ctx.host().dom();
        // Text / Comment nodes return their own data directly
        // (shortcut for the common case + spec-correct: character
        // data nodes are not "traversed" — they return their `data`).
        if let Ok(text) = dom.world().get::<&elidex_ecs::TextContent>(entity) {
            Some(text.0.clone())
        } else if let Ok(c) = dom.world().get::<&elidex_ecs::CommentData>(entity) {
            Some(c.0.clone())
        } else {
            // Element / DocumentFragment / Document: concatenate
            // descendant Text data.  Null for every other node kind
            // (Attribute, DocumentType — none currently reachable via
            // a wrapper but the spec requires Null).
            let kind = dom.node_kind(entity);
            match kind {
                Some(
                    elidex_ecs::NodeKind::Element
                    | elidex_ecs::NodeKind::DocumentFragment
                    | elidex_ecs::NodeKind::Document,
                ) => {
                    let mut buf = String::new();
                    dom.traverse_descendants(entity, |e| {
                        if let Ok(text) = dom.world().get::<&elidex_ecs::TextContent>(e) {
                            buf.push_str(&text.0);
                        }
                        true
                    });
                    Some(buf)
                }
                _ => None,
            }
        }
    };
    match result {
        Some(s) => {
            if s.is_empty() {
                Ok(JsValue::String(ctx.vm.well_known.empty))
            } else {
                let sid = ctx.vm.strings.intern(&s);
                Ok(JsValue::String(sid))
            }
        }
        None => Ok(JsValue::Null),
    }
}

/// `Node.prototype.textContent` setter — WHATWG DOM §4.4.
///
/// - Element / DocumentFragment: remove every existing child, then
///   append a single Text child if the new value is non-empty.
/// - Text / Comment: replace own data (null → empty string).
/// - Everything else: no-op.
#[cfg(feature = "engine")]
pub(super) fn native_node_set_text_content(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };
    let arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let data: String = match arg {
        JsValue::Null => String::new(),
        other => {
            let sid = super::super::coerce::to_string(ctx.vm, other)?;
            ctx.vm.strings.get_utf8(sid)
        }
    };
    {
        let dom = ctx.host().dom();
        // Character-data fast path.  Perform the type check with a
        // shared borrow, then upgrade to a mutable one for the
        // actual mutation — hecs's `RefMut` destructor re-enters the
        // world, so two back-to-back `get::<&mut _>` calls in the
        // same scope would clash even with ok/err short-circuits.
        let is_text = dom.world().get::<&elidex_ecs::TextContent>(entity).is_ok();
        if is_text {
            if let Ok(mut text) = dom.world_mut().get::<&mut elidex_ecs::TextContent>(entity) {
                text.0 = data;
            }
            return Ok(JsValue::Undefined);
        }
        let is_comment = dom.world().get::<&elidex_ecs::CommentData>(entity).is_ok();
        if is_comment {
            if let Ok(mut c) = dom.world_mut().get::<&mut elidex_ecs::CommentData>(entity) {
                c.0 = data;
            }
            return Ok(JsValue::Undefined);
        }
        // Only Element / Fragment / Document replace children.
        let kind = dom.node_kind(entity);
        if !matches!(
            kind,
            Some(
                elidex_ecs::NodeKind::Element
                    | elidex_ecs::NodeKind::DocumentFragment
                    | elidex_ecs::NodeKind::Document
            )
        ) {
            return Ok(JsValue::Undefined);
        }
        // Remove every existing child.  Collect first to avoid mutating
        // the sibling chain mid-iteration.
        let existing: Vec<elidex_ecs::Entity> = dom.children_iter(entity).collect();
        for child in existing {
            let _ = dom.remove_child(entity, child);
        }
        if !data.is_empty() {
            let text_entity = dom.create_text(data);
            let _ = dom.append_child(entity, text_entity);
        }
    }
    Ok(JsValue::Undefined)
}

impl VmInner {
    /// Install the Node-common accessors on `EventTarget.prototype`.
    ///
    /// Called from `register_event_target_prototype` after the
    /// EventTarget methods are in place.  `EventTarget.prototype` is
    /// the common root for every DOM wrapper, so placing Node-level
    /// accessors here means both Element and Text wrappers surface
    /// them via their prototype chains without any per-wrapper work.
    #[cfg(feature = "engine")]
    pub(in crate::vm) fn install_node_accessors_on_event_target(&mut self) {
        let proto = self
            .event_target_prototype
            .expect("install_node_accessors_on_event_target needs EventTarget.prototype");
        // Read-only accessors: the value is computed on every get
        // from live DOM state, so there is no slot to cache it in.
        for (name_sid, getter) in [
            (
                self.well_known.parent_node,
                native_node_get_parent_node as NativeFn,
            ),
            (self.well_known.next_sibling, native_node_get_next_sibling),
            (
                self.well_known.previous_sibling,
                native_node_get_previous_sibling,
            ),
            (self.well_known.node_type, native_node_get_node_type),
            (self.well_known.node_name, native_node_get_node_name),
        ] {
            let name = self.strings.get_utf8(name_sid);
            let gid = self.create_native_function(&format!("get {name}"), getter);
            self.define_shaped_property(
                proto,
                PropertyKey::String(name_sid),
                PropertyValue::Accessor {
                    getter: Some(gid),
                    setter: None,
                },
                shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }
        // Read/write accessors (nodeValue, textContent).
        let rw_attrs = shape::PropertyAttrs {
            writable: false,
            enumerable: true,
            configurable: true,
            is_accessor: true,
        };
        for (name_sid, getter, setter) in [
            (
                self.well_known.node_value,
                native_node_get_node_value as NativeFn,
                native_node_set_node_value as NativeFn,
            ),
            (
                self.well_known.text_content,
                native_node_get_text_content,
                native_node_set_text_content,
            ),
        ] {
            let name = self.strings.get_utf8(name_sid);
            let gid = self.create_native_function(&format!("get {name}"), getter);
            let sid = self.create_native_function(&format!("set {name}"), setter);
            self.define_shaped_property(
                proto,
                PropertyKey::String(name_sid),
                PropertyValue::Accessor {
                    getter: Some(gid),
                    setter: Some(sid),
                },
                rw_attrs,
            );
        }
    }
}

/// `EventTarget.prototype.dispatchEvent(event)` — stub.
///
/// Real implementation is deferred to **PR5a** (which lands `new Event(...)`
/// and the Event constructor family).  Until then, invoking this returns
/// `false` (the spec default for "event not dispatched").
pub(super) fn native_event_target_dispatch_event(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Boolean(false))
}
