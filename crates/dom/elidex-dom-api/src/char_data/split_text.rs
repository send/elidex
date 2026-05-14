//! Engine-independent `Text.splitText` algorithm (WHATWG DOM §4.10
//! "split a Text node").
//!
//! Hoisted from `vm/host/text_proto.rs::native_text_split_text` so the
//! VM-side binding is marshalling-only per the CLAUDE.md layering
//! mandate. The boa-side and engine-indep `DomApiHandler::SplitText`
//! both call into this function — the bespoke order
//! `insert → fire_after_split_text → set_text_data` ensures Range
//! live-tracking on the original `entity` migrates boundaries to the
//! new node BEFORE `set_text_data` clamps remaining boundaries to the
//! truncated head length.
//!
//! # Why `insert` before `fire_after_split_text`
//!
//! The hook callback (`MutationHook::after_split_text`) only needs to
//! migrate boundaries from `entity` → `new_node`; it does not need
//! `new_node` to be reachable from the tree. But firing the hook
//! AFTER `set_text_data(entity, head)` would mean
//! `after_text_change` fires first and clamps any `(entity, off)`
//! boundary with `off > offset` down to `head_len = offset`, losing
//! the `off - offset` migration target. Order is therefore
//! `set_text_data` LAST.
//!
//! The `insert_before` / `append_child` step fires `after_insert` for
//! the new sibling — `parent`-side Range boundaries adjust via the
//! standard insertion-step rule (off > new_node_idx → +1). Spec
//! §4.10 step 7.2 requires `off > entity_idx → +1`, which differs at
//! the single offset `entity_idx + 1`. The
//! [`elidex_ecs::MutationHook::after_split_text`] callback that
//! follows the insert is fired with the pre-split `parent` +
//! `node_index` so the consumer (`LiveRangeRegistry::Bridge`) tops up
//! that exact slot — the standard bridge therefore implements
//! §4.10 step 7 in full. Callers that install a CUSTOM hook which
//! ignores the parent / node_index args inherit the after_insert-only
//! behaviour (lag at `entity_idx + 1`); document the limitation on
//! such hooks if the gap matters for their use case.

use elidex_ecs::{EcsDom, Entity, NodeKind};

/// Error variants returned by [`split_text_at_offset`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SplitTextError {
    /// Receiver entity is not a Text / CDATASection node (failed
    /// `NodeKind::Text | NodeKind::CdataSection` brand check via
    /// `node_kind_inferred`).
    NotTextNode,
    /// Receiver was branded as Text by `NodeKind` but is missing the
    /// `TextContent` payload — internal invariant breach.
    MissingTextContent,
    /// `offset` exceeds the entity's UTF-16 data length. Surfaced as
    /// `IndexSizeError` by VM-side callers, `RangeError` by the
    /// `Text.prototype.splitText` natives path.
    OffsetOutOfBounds { offset: usize, len: usize },
    /// Engine `insert_before` / `append_child` rejected the trailing
    /// node — should be impossible since `entity`'s parent was
    /// verified above. Surfaced as `TypeError` ("could not insert
    /// the trailing Text node") for symmetry with the legacy VM-side
    /// behaviour.
    InsertFailed,
    /// `set_text_data` returned `None` after a successful create +
    /// insert — internal invariant breach (the `TextContent`
    /// component disappeared mid-operation).
    InternalInvariant,
}

/// Engine-independent `Text.splitText(offset)` per WHATWG DOM §4.10
/// "split a Text node" steps 1-8.
///
/// Splits `entity`'s `TextContent` at the UTF-16 `offset_utf16`:
/// `entity` retains the head `[..offset]`; a new Text node carrying
/// the tail `[offset..]` is created and inserted as `entity`'s next
/// sibling. Returns the new node entity on success.
///
/// Steps (with hook ordering):
/// 1. Brand-check `entity` is Text / CDATASection.
/// 2. Read `entity.TextContent`, verify `offset_utf16 ≤ utf16_len`.
/// 3. Split UTF-16 view at `offset_utf16` → head / tail strings
///    (surrogate-pair split lossy-coerces to U+FFFD per the engine
///    contract on `EcsDom::set_text_data`).
/// 4. Allocate `new_node` carrying the tail, inheriting `entity`'s
///    `AssociatedDocument` via `create_text_with_owner`.
/// 5. If `entity` has a parent: insert `new_node` as the next sibling.
///    Fires [`elidex_ecs::MutationHook::after_insert`].
/// 6. Fire [`elidex_ecs::MutationHook::after_split_text`] — boundaries
///    on `entity` at `off > offset` migrate to `(new_node, off -
///    offset)`. MUST run AFTER insert (so `new_node` is alive) but
///    BEFORE `set_text_data` (so the clamp does not destroy the
///    migration target).
/// 7. `set_text_data(entity, head)` — truncates entity to head, fires
///    [`elidex_ecs::MutationHook::after_text_change`] which clamps any
///    boundaries still on `entity` (i.e. `off ≤ offset`) to
///    `head_len`. Those boundaries are by definition unaffected
///    (off ≤ head_len = offset).
///
/// On failure: rolls back the inserted `new_node` and destroys it so
/// the tree shape and `entity`'s data are unchanged.
pub fn split_text_at_offset(
    entity: Entity,
    offset_utf16: usize,
    dom: &mut EcsDom,
) -> Result<Entity, SplitTextError> {
    // Step 1: brand check (`node_kind_inferred` accepts legacy entities
    // tagged Text via TextContent without a NodeKind component —
    // matches the routing in HostData::prototype_kind_for).
    if !matches!(
        dom.node_kind_inferred(entity),
        Some(NodeKind::Text | NodeKind::CdataSection)
    ) {
        return Err(SplitTextError::NotTextNode);
    }

    // Step 2: read entity data + verify offset.
    let original = match dom.world().get::<&elidex_ecs::TextContent>(entity) {
        Ok(tc) => tc.0.clone(),
        Err(_) => return Err(SplitTextError::MissingTextContent),
    };
    let units: Vec<u16> = original.encode_utf16().collect();
    let len = units.len();
    if offset_utf16 > len {
        return Err(SplitTextError::OffsetOutOfBounds {
            offset: offset_utf16,
            len,
        });
    }

    // Step 3: split UTF-16 view (surrogate-pair split is spec-valid and
    // produces U+FFFD via `from_utf16_lossy` — same lossy contract as
    // `EcsDom::set_text_data`).
    let (head_units, tail_units) = units.split_at(offset_utf16);
    let head = String::from_utf16_lossy(head_units);
    let tail = String::from_utf16_lossy(tail_units);

    // Step 4: allocate new_node with the tail, inheriting
    // entity's AssociatedDocument so the spec "fragment node document
    // = context's node document" invariant holds (§4.4 "node
    // document").
    let owner = dom.get_associated_document(entity);
    let new_node = dom.create_text_with_owner(tail, owner);

    // Step 5: capture entity's pre-insert parent + index (used by the
    // after_split_text hook for parent-side boundary adjustment per
    // spec §4.10 step 7.2), then insert new_node as next sibling (or
    // append if entity is the last child). Fires
    // `after_insert(new_node, parent, idx+1)`. Skips entirely when
    // entity is orphan — parent-side adjustment is vacuous in that case.
    let parent_opt = dom.get_parent(entity);
    let node_index = parent_opt.and_then(|_| dom.index_in_parent(entity));
    if let Some(parent) = parent_opt {
        let inserted = if let Some(next) = dom.get_next_sibling(entity) {
            dom.insert_before(parent, new_node, next)
        } else {
            dom.append_child(parent, new_node)
        };
        if !inserted {
            let _ = dom.destroy_entity(new_node);
            return Err(SplitTextError::InsertFailed);
        }
    }

    // Step 6: fire after_split_text BEFORE truncate. Boundaries on
    // entity at off > offset migrate to (new_node, off - offset)
    // (strict-greater per spec §4.10 step 7.2/7.3 — equality boundaries
    // stay on the original node); parent-side boundary at exactly
    // `node_index + 1` shifts +1 (the `after_insert` hook fired by
    // step 5 already handled the `off > node_index + 1` cases).
    dom.fire_after_split_text(entity, new_node, offset_utf16, parent_opt, node_index);

    // Step 7: truncate entity to head. Fires after_text_change which
    // clamps boundaries still on entity (those with off ≤ offset) to
    // head_len — vacuous for off ≤ offset by definition. The
    // `Option::None` arm covers a defensive-only branch (TextContent
    // disappeared after the brand check); roll back the insertion to
    // leave the tree shape unchanged.
    if dom.set_text_data(entity, &head).is_none() {
        if let Some(parent) = dom.get_parent(new_node) {
            let _ = dom.remove_child(parent, new_node);
        }
        let _ = dom.destroy_entity(new_node);
        return Err(SplitTextError::InternalInvariant);
    }

    Ok(new_node)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::range::{LiveRangeRegistry, Range};
    use elidex_ecs::Attributes;

    fn build_tree() -> (EcsDom, Entity, Entity) {
        let mut dom = EcsDom::new();
        let parent = dom.create_element("p", Attributes::default());
        let t = dom.create_text("hello world");
        let _ = dom.append_child(parent, t);
        (dom, parent, t)
    }

    #[test]
    fn splits_text_in_half() {
        let (mut dom, parent, t) = build_tree();
        let new_node = split_text_at_offset(t, 5, &mut dom).expect("split ok");

        let tc_t = dom
            .world()
            .get::<&elidex_ecs::TextContent>(t)
            .expect("entity TextContent");
        assert_eq!(tc_t.0, "hello");

        let tc_new = dom
            .world()
            .get::<&elidex_ecs::TextContent>(new_node)
            .expect("new_node TextContent");
        assert_eq!(tc_new.0, " world");

        // new_node is next sibling of entity.
        let children: Vec<_> = dom.children_iter(parent).collect();
        assert_eq!(children, vec![t, new_node]);
    }

    #[test]
    fn rejects_non_text() {
        let mut dom = EcsDom::new();
        let div = dom.create_element("div", Attributes::default());
        let err = split_text_at_offset(div, 0, &mut dom).unwrap_err();
        assert_eq!(err, SplitTextError::NotTextNode);
    }

    #[test]
    fn rejects_offset_beyond_length() {
        let (mut dom, _parent, t) = build_tree();
        let err = split_text_at_offset(t, 999, &mut dom).unwrap_err();
        assert!(matches!(
            err,
            SplitTextError::OffsetOutOfBounds {
                offset: 999,
                len: 11
            }
        ));

        // Original data preserved.
        let tc = dom.world().get::<&elidex_ecs::TextContent>(t).unwrap();
        assert_eq!(tc.0, "hello world");
    }

    #[test]
    fn split_at_zero_keeps_original_empty() {
        let (mut dom, parent, t) = build_tree();
        let new_node = split_text_at_offset(t, 0, &mut dom).expect("split ok");

        let tc_t = dom.world().get::<&elidex_ecs::TextContent>(t).unwrap();
        assert_eq!(tc_t.0, "");
        let tc_new = dom
            .world()
            .get::<&elidex_ecs::TextContent>(new_node)
            .unwrap();
        assert_eq!(tc_new.0, "hello world");

        let children: Vec<_> = dom.children_iter(parent).collect();
        assert_eq!(children, vec![t, new_node]);
    }

    #[test]
    fn split_at_end_keeps_new_node_empty() {
        let (mut dom, parent, t) = build_tree();
        let new_node = split_text_at_offset(t, 11, &mut dom).expect("split ok");

        let tc_t = dom.world().get::<&elidex_ecs::TextContent>(t).unwrap();
        assert_eq!(tc_t.0, "hello world");
        let tc_new = dom
            .world()
            .get::<&elidex_ecs::TextContent>(new_node)
            .unwrap();
        assert_eq!(tc_new.0, "");

        let children: Vec<_> = dom.children_iter(parent).collect();
        assert_eq!(children, vec![t, new_node]);
    }

    #[test]
    fn orphan_text_node_split_skips_insert_and_returns_new_node() {
        let mut dom = EcsDom::new();
        let t = dom.create_text("hello world");
        let new_node = split_text_at_offset(t, 5, &mut dom).expect("split ok");

        let tc_t = dom.world().get::<&elidex_ecs::TextContent>(t).unwrap();
        assert_eq!(tc_t.0, "hello");
        let tc_new = dom
            .world()
            .get::<&elidex_ecs::TextContent>(new_node)
            .unwrap();
        assert_eq!(tc_new.0, " world");

        // new_node has no parent (orphan path).
        assert!(dom.get_parent(new_node).is_none());
    }

    #[test]
    fn split_text_migrates_range_boundary_to_new_node() {
        // WHATWG §4.10 step 8: boundary on `entity` at off > offset
        // migrates to (new_node, off - offset). With the insert →
        // fire_after_split_text → set_text_data ordering, the
        // migration runs BEFORE set_text_data's after_text_change
        // would clamp the boundary down to `head_len = offset`.
        let (mut dom, _parent, t) = build_tree();
        let (mut reg, bridge) = LiveRangeRegistry::new_pair();
        dom.set_mutation_hook(Box::new(bridge));

        let mut r = Range::new(t);
        r.set_start(t, 8); // inside "hello world", past offset 5
        r.set_end(t, 8);
        let id = reg.register(r);

        let new_node = split_text_at_offset(t, 5, &mut dom).expect("split ok");

        reg.with_range(id, &dom, |range, _| {
            assert_eq!(range.start_container, new_node, "migrated to new_node");
            assert_eq!(range.start_offset, 3, "offset rebased to off - 5");
            assert_eq!(range.end_container, new_node);
            assert_eq!(range.end_offset, 3);
        })
        .expect("range present");
    }

    #[test]
    fn split_text_boundary_at_or_before_offset_stays_on_entity_and_clamps() {
        // Boundary at off ≤ offset stays on entity. The post-truncate
        // after_text_change fires with new_utf16_len = offset, so a
        // boundary at off > offset would be clamped — but our test
        // case has off ≤ offset so the clamp is a no-op.
        let (mut dom, _parent, t) = build_tree();
        let (mut reg, bridge) = LiveRangeRegistry::new_pair();
        dom.set_mutation_hook(Box::new(bridge));

        let mut r = Range::new(t);
        r.set_start(t, 2);
        r.set_end(t, 4);
        let id = reg.register(r);

        let _new_node = split_text_at_offset(t, 5, &mut dom).expect("split ok");

        reg.with_range(id, &dom, |range, _| {
            assert_eq!(range.start_container, t);
            assert_eq!(range.start_offset, 2);
            assert_eq!(range.end_container, t);
            assert_eq!(range.end_offset, 4);
        })
        .expect("range present");
    }
}
