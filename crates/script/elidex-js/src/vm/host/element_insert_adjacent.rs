//! `Element.prototype.insertAdjacentElement` /
//! `insertAdjacentText` (WHATWG DOM §4.9).
//!
//! Split out of [`super::element_proto`] to keep that file under
//! the project's 1000-line convention (PR5a C9).  The install-time
//! reference in `install_element_matches` reaches these natives
//! via their `pub(super)` re-export.

#![cfg(feature = "engine")]

use super::super::value::{JsValue, NativeContext, ObjectKind, VmError};

use elidex_ecs::{Entity, NodeKind};

// ---------------------------------------------------------------------------
// insertAdjacentElement / insertAdjacentText — WHATWG §4.9
// ---------------------------------------------------------------------------

/// Which of the four WHATWG `where` positions (ASCII case-insensitive)
/// the caller passed to `insertAdjacent*`.
#[derive(Clone, Copy)]
enum InsertAdjacentWhere {
    BeforeBegin,
    AfterBegin,
    BeforeEnd,
    AfterEnd,
}

/// Parse the `where` argument into an [`InsertAdjacentWhere`], matching
/// ASCII case-insensitively against the four WHATWG literals.
fn parse_adjacent_position(raw: &str) -> Option<InsertAdjacentWhere> {
    // `eq_ignore_ascii_case` is O(n) on byte length; there are four
    // six-to-ten-byte literals so no optimisation is worthwhile.
    if raw.eq_ignore_ascii_case("beforebegin") {
        Some(InsertAdjacentWhere::BeforeBegin)
    } else if raw.eq_ignore_ascii_case("afterbegin") {
        Some(InsertAdjacentWhere::AfterBegin)
    } else if raw.eq_ignore_ascii_case("beforeend") {
        Some(InsertAdjacentWhere::BeforeEnd)
    } else if raw.eq_ignore_ascii_case("afterend") {
        Some(InsertAdjacentWhere::AfterEnd)
    } else {
        None
    }
}

/// TypeError thrown when `where` is not one of the four spec literals.
///
/// `DOMException("SyntaxError")` thrown when the first argument of
/// `insertAdjacent*` is not one of the four recognised positions.
/// All callers embed the method name so the message stays aligned
/// with WHATWG `insertAdjacent*` step 1.
///
/// `where_value` is echoed into the message (matching Blink / Gecko)
/// so script debuggers see the exact literal the caller supplied.
fn adjacent_syntax_error(ctx: &NativeContext<'_>, method: &str, where_value: &str) -> VmError {
    VmError::dom_exception(
        ctx.vm.well_known.dom_exc_syntax_error,
        format!(
            "Failed to execute '{method}' on 'Element': \
             the value provided ('{where_value}') is not one of \
             'beforebegin', 'afterbegin', 'beforeend', or 'afterend'."
        ),
    )
}

/// True when `pos` is one of the two positions that require the
/// receiver to have a parent.  Used by `insertAdjacentText` to
/// pre-check before allocating a Text entity that would otherwise
/// leak into the ECS on early return.
fn position_requires_parent(pos: InsertAdjacentWhere) -> bool {
    matches!(
        pos,
        InsertAdjacentWhere::BeforeBegin | InsertAdjacentWhere::AfterEnd
    )
}

/// TypeError thrown when the second argument of `insertAdjacentElement`
/// is not an Element wrapper.  Matches the Blink / Gecko message form.
fn adjacent_element_arg_error() -> VmError {
    VmError::type_error(
        "Failed to execute 'insertAdjacentElement' on 'Element': \
         parameter 2 is not of type 'Element'."
            .to_owned(),
    )
}

// `DOMException("HierarchyRequestError")` for insertAdjacent* is
// assembled inline inside `perform_adjacent_insert` so the
// `HierarchyRequestError` StringId is captured *before* the EcsDom
// mutable borrow — see the closure at the top of that function.

/// TypeError thrown when `insertAdjacentElement`'s second argument
/// is a HostObject whose entity has been destroyed / recycled.
/// Separated from [`adjacent_element_arg_error`] so stale wrappers
/// surface the "detached" failure mode rather than being misreported
/// as non-Element (matches [`super::event_target::require_receiver`]
/// which also distinguishes destroyed vs. wrong-kind receivers).
fn adjacent_element_detached_error() -> VmError {
    VmError::type_error(
        "Failed to execute 'insertAdjacentElement' on 'Element': \
         parameter 2 is detached (invalid entity)."
            .to_owned(),
    )
}

/// Extract an Element [`Entity`] from a method argument, throwing a
/// WebIDL-style `TypeError` on any non-Element value (including
/// `null` / `undefined` / non-HostObject objects / HostObjects that
/// are not `NodeKind::Element`).  A HostObject whose entity has been
/// destroyed surfaces a distinct "detached" error so scripts can
/// distinguish stale wrappers from genuine type mismatches.
fn require_element_arg(ctx: &mut NativeContext<'_>, value: JsValue) -> Result<Entity, VmError> {
    let JsValue::Object(id) = value else {
        return Err(adjacent_element_arg_error());
    };
    let ObjectKind::HostObject { entity_bits } = ctx.vm.get_object(id).kind else {
        return Err(adjacent_element_arg_error());
    };
    let entity = Entity::from_bits(entity_bits).ok_or_else(adjacent_element_detached_error)?;
    // Stale-entity check BEFORE the kind lookup: a destroyed entity
    // has no components, so `node_kind_inferred` would return None
    // and masquerade as "wrong type".  Catching it here keeps the
    // error message aligned with `require_receiver` (which makes the
    // same split for stale receivers).
    if !ctx.host().dom().contains(entity) {
        return Err(adjacent_element_detached_error());
    }
    match ctx.host().dom().node_kind_inferred(entity) {
        Some(NodeKind::Element) => Ok(entity),
        _ => Err(adjacent_element_arg_error()),
    }
}

/// Perform the insertion step of `insertAdjacent*` (WHATWG §4.9).
/// `target` is the method receiver; `node` is the Element / Text to
/// insert.  Returns `Ok(Some(node))` when the insert succeeded,
/// `Ok(None)` when `where` is `beforebegin` / `afterend` but the
/// receiver has no parent (spec: return `null` without throwing), and
/// `Err` when `EcsDom` rejects the insertion (cycle / destroyed).
fn perform_adjacent_insert(
    ctx: &mut NativeContext<'_>,
    target: Entity,
    node: Entity,
    pos: InsertAdjacentWhere,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    // Capture the interned `"HierarchyRequestError"` StringId up
    // front — once `ctx.host().dom()` holds the mutable borrow,
    // reaching back into `ctx.vm.well_known` would collide.  The
    // helper that uses it (`make_hierarchy_err`) runs only on the
    // cold error path, so the up-front capture is free on the
    // success path.
    let hier_name = ctx.vm.well_known.dom_exc_hierarchy_request_error;
    let make_hierarchy_err = |method: &str| -> VmError {
        VmError::dom_exception(
            hier_name,
            format!(
                "Failed to execute '{method}' on 'Element': \
                 the new child node cannot be inserted at this position."
            ),
        )
    };
    let dom = ctx.host().dom();
    // WHATWG `Node.insertBefore(x, x)` and its `x, x.nextSibling`
    // sibling form treat "insert a node before itself" as a no-op
    // that succeeds (§4.2.3 pre-insertion step 2).  `EcsDom::insert_before`
    // rejects `new_child == ref_child` as invalid, so every position
    // that would reduce to that edge case returns Ok(Some(node))
    // before the rejecting call — matching the ChildNode mixin's
    // `insert_before(parent, x, x)` accommodation in
    // `vm/host/childnode.rs`.
    match pos {
        InsertAdjacentWhere::BeforeBegin => {
            let Some(parent) = dom.get_parent(target) else {
                return Ok(None);
            };
            // `parent.insertBefore(target, target)` — no-op move.
            if node == target {
                return Ok(Some(node));
            }
            if !dom.insert_before(parent, node, target) {
                return Err(make_hierarchy_err(method));
            }
            Ok(Some(node))
        }
        InsertAdjacentWhere::AfterBegin => {
            if let Some(first) = dom.children_iter(target).next() {
                // `target.insertBefore(first, first)` — no-op move.
                if node == first {
                    return Ok(Some(node));
                }
                if !dom.insert_before(target, node, first) {
                    return Err(make_hierarchy_err(method));
                }
            } else if !dom.append_child(target, node) {
                return Err(make_hierarchy_err(method));
            }
            Ok(Some(node))
        }
        InsertAdjacentWhere::BeforeEnd => {
            // No spec-allowed no-op here: `target.appendChild(target)`
            // is a genuine cycle and must fail.
            if !dom.append_child(target, node) {
                return Err(make_hierarchy_err(method));
            }
            Ok(Some(node))
        }
        InsertAdjacentWhere::AfterEnd => {
            let Some(parent) = dom.get_parent(target) else {
                return Ok(None);
            };
            match dom.get_next_sibling(target) {
                Some(next) => {
                    // `parent.insertBefore(next, next)` — no-op move.
                    if node == next {
                        return Ok(Some(node));
                    }
                    if !dom.insert_before(parent, node, next) {
                        return Err(make_hierarchy_err(method));
                    }
                }
                None => {
                    if !dom.append_child(parent, node) {
                        return Err(make_hierarchy_err(method));
                    }
                }
            }
            Ok(Some(node))
        }
    }
}

/// `Element.prototype.insertAdjacentElement(where, element)` —
/// WHATWG DOM §4.9.
pub(super) fn native_element_insert_adjacent_element(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(target) = super::event_target::require_receiver(
        ctx,
        this,
        "Element",
        "insertAdjacentElement",
        |k| k == NodeKind::Element,
    )?
    else {
        return Ok(JsValue::Null);
    };
    let where_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let where_raw = super::super::coerce::to_string(ctx.vm, where_arg)?;
    let where_str = ctx.vm.strings.get_utf8(where_raw);
    let pos = match parse_adjacent_position(&where_str) {
        Some(p) => p,
        None => {
            return Err(adjacent_syntax_error(
                ctx,
                "insertAdjacentElement",
                &where_str,
            ))
        }
    };

    let element_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let node = require_element_arg(ctx, element_arg)?;
    // `node` is user-supplied — on `perform_adjacent_insert` failure
    // the caller still holds a JS handle to it, so we must NOT
    // destroy the entity here (that would invalidate live wrappers).
    match perform_adjacent_insert(ctx, target, node, pos, "insertAdjacentElement")? {
        Some(entity) => Ok(JsValue::Object(ctx.vm.create_element_wrapper(entity))),
        None => Ok(JsValue::Null),
    }
}

/// `Element.prototype.insertAdjacentText(where, data)` —
/// WHATWG DOM §4.9.
pub(super) fn native_element_insert_adjacent_text(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(target) =
        super::event_target::require_receiver(ctx, this, "Element", "insertAdjacentText", |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(JsValue::Undefined);
    };
    let where_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let where_raw = super::super::coerce::to_string(ctx.vm, where_arg)?;
    let where_str = ctx.vm.strings.get_utf8(where_raw);
    let pos = match parse_adjacent_position(&where_str) {
        Some(p) => p,
        None => return Err(adjacent_syntax_error(ctx, "insertAdjacentText", &where_str)),
    };

    // Parent-less short-circuit: `beforebegin` / `afterend` require
    // the receiver to have a parent, and the spec treats the missing-
    // parent case as a silent no-op.  Check BEFORE allocating a Text
    // entity — otherwise the allocation leaks into the ECS because
    // no JS handle is returned and the entity never reaches GC.
    if position_requires_parent(pos) && ctx.host().dom().get_parent(target).is_none() {
        return Ok(JsValue::Undefined);
    }

    // `where` + parent-existence validity are already checked; allocate
    // the Text now.  WHATWG §4.9 step 2: the Text's node document is
    // *target's* node document, so thread the receiver's owner
    // document through the owner-aware creator (PR4f C2).
    let data_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let data_sid = super::super::coerce::to_string(ctx.vm, data_arg)?;
    let data = ctx.vm.strings.get_utf8(data_sid);
    let owner_doc = ctx.host().dom().owner_document(target);
    let text_entity = ctx.host().dom().create_text_with_owner(data, owner_doc);
    // Cycle / destroyed-receiver paths still fail inside
    // `perform_adjacent_insert` (parent exists but insertion is
    // otherwise invalid).  Destroy the unreferenced Text so the
    // error path does not leak an ECS entity — nothing outside this
    // function holds a handle to it.
    match perform_adjacent_insert(ctx, target, text_entity, pos, "insertAdjacentText") {
        Ok(_) => Ok(JsValue::Undefined),
        Err(e) => {
            let _ = ctx.host().dom().destroy_entity(text_entity);
            Err(e)
        }
    }
}
