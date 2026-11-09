//! WHATWG DOM childList mutation family: `appendChild` / `insertBefore` /
//! `removeChild` / `replaceChild` / "replace all" and their move
//! source-removal record helpers; `adoptNode`'s source-removal record.

use elidex_ecs::{EcsDom, Entity};

use super::{empty_record, MutationKind, MutationRecord};

/// The move source-removal context for a node passed to a childList op: its old
/// parent + exposed siblings, captured **before** the relink (which destroys
/// them), or `None` if the node was unparented (a fresh insert). This is the
/// move-vs-fresh source of truth for `appendChild` / `insertBefore` / `replaceChild`
/// (`apply_replace_child` captures it with `old_child` still in place: replace
/// step 6 adopt runs before step 7 remove). The trailing `bool` is
/// the old parent's connectedness (DOM "remove" step-12 `isParentConnected`),
/// captured pre-relink for the source-removal record's `disconnectedCallback`
/// gate.
type MoveSource = Option<(Entity, Option<Entity>, Option<Entity>, bool)>;

/// Capture the [`MoveSource`] for `node` (append/insert/replace): old parent + `node`'s
/// exposed prev/next + the old parent's connectedness. The exposed-sibling
/// helpers skip internal `ShadowRoot` entities (§4.8 encapsulation). Must run
/// before the `EcsDom` relink (both the siblings and — for step-12
/// `isParentConnected` — the connectedness are pre-removal state).
fn capture_move_source(dom: &EcsDom, node: Entity) -> MoveSource {
    dom.get_parent(node).map(|old_parent| {
        (
            old_parent,
            dom.prev_exposed_sibling(node),
            dom.next_exposed_sibling(node),
            dom.is_connected(old_parent),
        )
    })
}

/// The §4.5 adopt source-parent removal record for `node` leaving `old_parent`
/// (siblings captured pre-relink). One builder for the move source-removal shape
/// (One-issue-one-way), shared by `move_record_list`, [`source_removal_record`], and
/// the convert-into-fragment source path.
fn source_removal_record_from(
    old_parent: Entity,
    node: Entity,
    previous_sibling: Option<Entity>,
    next_sibling: Option<Entity>,
    parent_was_connected: bool,
) -> MutationRecord {
    MutationRecord {
        removed_nodes: vec![node],
        previous_sibling,
        next_sibling,
        parent_was_connected,
        ..empty_record(MutationKind::ChildList, old_parent)
    }
}

/// The §4.5 adopt source-parent removal record for `node` if it is currently
/// parented (read live, so call BEFORE the relink), else `None`. The adopt step's
/// remove is NOT covered by an enclosing insert's `suppressObservers` (§4.2.3 step
/// 7.1 → §4.5 adopt step 2 invokes a plain "remove"), so a move's source removal is
/// always observed — even from `replace all` and the "convert nodes into a node"
/// wrapper build.
fn source_removal_record(dom: &EcsDom, node: Entity) -> Option<MutationRecord> {
    dom.get_parent(node).map(|old_parent| {
        source_removal_record_from(
            old_parent,
            node,
            dom.prev_exposed_sibling(node),
            dom.next_exposed_sibling(node),
            dom.is_connected(old_parent),
        )
    })
}

/// Assemble the §4.3.2 childList record list: the source-parent removal of
/// `node` (when `source` is `Some`, i.e. a move) **precedes** the `primary`
/// (destination / coalesced) record, matching the WHATWG adopt→insert order.
/// One source of truth for the move two-record shape (One-issue-one-way).
fn move_record_list(
    source: MoveSource,
    node: Entity,
    primary: MutationRecord,
) -> Vec<MutationRecord> {
    match source {
        Some((old_parent, previous_sibling, next_sibling, parent_was_connected)) => {
            vec![
                source_removal_record_from(
                    old_parent,
                    node,
                    previous_sibling,
                    next_sibling,
                    parent_was_connected,
                ),
                primary,
            ]
        }
        None => vec![primary],
    }
}

/// Source-side `MutationRecord`s the spec queues when "convert nodes into a node"
/// (WHATWG DOM §4.2.6 step 4) appends `node` into the transient (unobserved) wrapper
/// `DocumentFragment` — the wrapper's own destination records are unobserved, but the
/// source records the spec does NOT suppress must still be queued: a `DocumentFragment`
/// arg's §4.2.3 step-4.2 fragment record (its children leaving it), or an
/// already-parented plain node's §4.5 adopt source-parent removal. Empty for a fresh
/// plain node. **Call BEFORE the raw move** (reads pre-move siblings/children).
pub fn convert_arg_source_records(dom: &EcsDom, node: Entity) -> Vec<MutationRecord> {
    if dom.is_document_fragment(node) {
        // step 4.2 fragment record: removedNodes = the fragment's children (which are
        // about to move out). Empty fragment → no record (count-0 insert, step 3).
        let children = dom.child_list_uncapped(node);
        if children.is_empty() {
            return Vec::new();
        }
        return vec![MutationRecord {
            removed_nodes: children,
            ..empty_record(MutationKind::ChildList, node)
        }];
    }
    source_removal_record(dom, node).into_iter().collect()
}

/// Expand a `DocumentFragment` per WHATWG DOM §4.2.3 "insert" steps 1/4/7: take a
/// **static** snapshot of `fragment`'s children (step 1 binds `nodes` to the
/// fragment's children once, before step 4.1 removes them and step 7 relinks them;
/// iterating the live child chain while moving would skip nodes), emit the
/// step-4.2 fragment record
/// (`addedNodes`=«», `removedNodes`=nodes; queued even when the enclosing insert
/// is suppressed — the step "intentionally does not pay attention to
/// suppressObservers"), then move each child into the tree via `link_each` (the
/// step-7 "for each node in nodes" per-node relink, which runs the per-node
/// insertion steps + custom-element reactions through the `EcsDom` primitive).
///
/// Returns `None` for an **empty** fragment (count 0 → §4.2.3 insert step 3
/// early return: no records at all), or when **no** child actually relinked.
/// `link_each` performs the append or insert-before the caller needs, returning
/// whether the per-node relink succeeded, and is the One-issue-one-way home for
/// the fragment-children move (the canonical record-producing expansion — the
/// record-free `insert_node_expanding_fragment` that previously lived in
/// `elidex-dom-api` was retired onto this in B1.2b).
///
/// The records are built from the children that **actually relinked** (not the raw
/// step-1 snapshot): the per-node `EcsDom` primitive rejects a relink whose child
/// would create a cycle / is destroyed, in which case that child stays in the
/// fragment and must NOT appear in `addedNodes`/`removedNodes`. Callers run the
/// §4.2.3 step-2 ancestor check (`is_ancestor_or_self(fragment, parent)`) BEFORE
/// calling this, so no child can be an ancestor of `parent` and every child
/// relinks (`moved == nodes`); the per-child filter is then defence-in-depth that
/// keeps the records truthful by construction rather than trusting that invariant.
fn expand_fragment(
    dom: &mut EcsDom,
    fragment: Entity,
    mut link_each: impl FnMut(&mut EcsDom, Entity) -> bool,
) -> Option<(MutationRecord, Vec<Entity>)> {
    // step 1, static snapshot — `child_list_uncapped` (NOT `children_iter`, which
    // truncates at MAX_ANCESTOR_DEPTH): §4.2.3 operates on ALL the fragment's
    // children, so a >cap fragment must not silently drop its tail.
    let nodes: Vec<Entity> = dom.child_list_uncapped(fragment);
    if nodes.is_empty() {
        return None; // step 3: count 0 → return
    }
    // step 7: per-node adopt + insert + insertion steps + CE reactions. Keep only
    // the children that actually relinked, so the records reflect the real moves.
    let mut moved = Vec::with_capacity(nodes.len());
    for node in nodes {
        if link_each(dom, node) {
            moved.push(node);
        }
    }
    if moved.is_empty() {
        return None; // nothing relinked → no records
    }
    let frag_record = MutationRecord {
        removed_nodes: moved.clone(), // step 4.2: only the children actually removed
        ..empty_record(MutationKind::ChildList, fragment)
    };
    Some((frag_record, moved))
}

/// A `ShadowRoot` entity is bound to its host by the unique, immovable host edge
/// (WHATWG DOM §4.8) — it is NOT a movable node, and `EcsDom::is_document_fragment`
/// deliberately excludes it, so without this guard the `apply_*` layer would treat a
/// shadow root passed as an insertion `node` as an ordinary node and reparent it out
/// of its host, corrupting the host↔shadow-root invariant. Inserting a shadow root is
/// a HierarchyRequestError; because it is a DOM tree-constraint (not WebIDL
/// marshalling), the guard lives here in the engine-independent record layer so EVERY
/// consumer (VM, boa, wasm) is protected — not only the VM's `normalize_mixin_arg` /
/// `reject_shadow_root_insertion`. Callers map the empty list to the error. (Codex PR393 R5.)
fn rejects_shadow_root_insertion(dom: &EcsDom, node: Entity) -> bool {
    dom.is_shadow_root(node)
}

/// Append `child` to `parent` through the `EcsDom` chokepoint and build the
/// §4.3.2 "queue a tree mutation record" childList record list — empty on
/// failure, one record for a fresh node, **two** for a move (an already-parented
/// `child`: WHATWG DOM §4.5 adopt → §4.2.3 remove source record, NOT suppressed,
/// then the destination insertion record). Shared by the deferred-flush path
/// (`apply_mutation`) and the synchronous VM bridge handler (`AppendChild`) so
/// both runtimes produce the identical record shape — one record source
/// (One-issue-one-way).
pub fn apply_append_child(dom: &mut EcsDom, parent: Entity, child: Entity) -> Vec<MutationRecord> {
    if rejects_shadow_root_insertion(dom, child) {
        return Vec::new();
    }
    // §4.2.3 insertion "if parent is connected" for the dest record's
    // `connectedCallback` gate. Capture up front (appending children never
    // changes `parent`'s own connectedness, so this equals insertion-time).
    let parent_was_connected = dom.is_connected(parent);
    if dom.is_document_fragment(child) {
        // §4.2.3 ensure pre-insertion validity step 2: a fragment that is a
        // host-including inclusive ancestor of `parent` (incl. `parent` itself, e.g.
        // `frag.appendChild(frag)`) is a HierarchyRequestError — reject ATOMICALLY
        // before moving any child. Without this a non-empty cyclic fragment would
        // move its non-cyclic children, skip the cyclic one, and report success
        // (partial mutation). Empty list → the handler maps it to a hierarchy error.
        if dom.is_ancestor_or_self(child, parent) {
            return Vec::new();
        }
        // DocumentFragment: §4.2.3 "insert" expands the fragment's children into
        // `parent` (the fragment itself is never linked). `previousSibling` =
        // step 6 = parent's last child, captured BEFORE the move (`children_iter_rev`
        // skips internal `ShadowRoot`, §4.8). `nextSibling` is null for an append.
        let prev_sibling = dom.children_iter_rev(parent).next();
        let Some((frag_record, nodes)) =
            expand_fragment(dom, child, |d, n| d.append_child(parent, n))
        else {
            return Vec::new(); // empty fragment → §4.2.3 step 3
        };
        let dest = MutationRecord {
            added_nodes: nodes,
            previous_sibling: prev_sibling,
            parent_was_connected,
            ..empty_record(MutationKind::ChildList, parent) // step 8
        };
        return vec![frag_record, dest]; // step 4.2 record THEN step 8 record
    }
    // Move-vs-fresh source of truth: `child`'s old parent + siblings, captured
    // **before** the relink destroys them (`Some` ⇒ move, `None` ⇒ fresh).
    let source = capture_move_source(dom, child);
    // Destination `previousSibling` per DOM §4.2.3 insert **step 6**
    // (`previousSibling` = parent's last child for an append) — captured **before**
    // the adopt at step 7.1, NOT after the relink. For `appendChild(<current last
    // child>)` (a no-position-change same-parent move) this is the moved node
    // itself: a spec-mandated self-sibling, NOT a bug (Codex PR384 R1). The append
    // reference child is null, so `nextSibling` is null. `children_iter_rev` skips
    // internal `ShadowRoot` entities so the captured sibling matches the DOM-visible
    // chain (§4.8 encapsulation), same as `prev_exposed_sibling` for insert/replace.
    let prev_sibling = dom.children_iter_rev(parent).next();
    if !dom.append_child(parent, child) {
        return Vec::new();
    }
    let dest = MutationRecord {
        added_nodes: vec![child],
        previous_sibling: prev_sibling,
        parent_was_connected,
        ..empty_record(MutationKind::ChildList, parent)
    };
    move_record_list(source, child, dest)
}

/// Insert `new_child` before `ref_child` under `parent` through the `EcsDom`
/// chokepoint and build the §4.3.2 childList record list — empty on failure, one
/// for a fresh node, **two** for a move (source-parent removal + destination
/// insertion). Shared by the deferred-flush path and the VM `insertBefore`
/// handler.
pub fn apply_insert_before(
    dom: &mut EcsDom,
    parent: Entity,
    new_child: Entity,
    ref_child: Entity,
) -> Vec<MutationRecord> {
    // A shadow root is not a movable node (see `rejects_shadow_root_insertion`).
    if rejects_shadow_root_insertion(dom, new_child) {
        return Vec::new();
    }
    // §4.2.3 "ensure pre-insertion validity" step 3 (NotFoundError): referenceChild
    // (`child`) must be a child of `parent` — run from pre-insert step 1, BEFORE the
    // pre-insert step-3 self-reference advance below and before any move. So
    // `insertBefore(orphan, orphan)` / `insertBefore(otherChild, otherChild)` fail
    // (empty list → the handler maps it to an error) instead of being treated as a
    // valid self-reference no-op. Also the up-front guard the fragment path needs
    // (`EcsDom::insert_before` would reject a bad `ref_child`, but the per-node
    // closure ignores that). (Codex PR393 R2.)
    if dom.get_parent(ref_child) != Some(parent) {
        return Vec::new();
    }
    // §4.2.3 pre-insert step 3: "if referenceChild is node, set referenceChild to
    // node's next sibling" — a self-reference (`insertBefore(x, x)`,
    // `first.before(first)`, `parent.prepend(parent.firstChild)`) where `x ∈ parent`
    // (now established) is a no-position-change move, NOT an error. `EcsDom::insert_before`
    // rejects `new_child == ref_child` outright, so apply the spec advance here (the
    // single canonical home, shared by node_proto + the ChildNode/ParentNode mixin
    // path) before the rejection swallows the move + its records. Advancing to the next
    // exposed sibling (or appending when `new_child` is last) yields the spec-mandated
    // self-sibling move records (B1.2a). `next != new_child`, so no infinite loop.
    if new_child == ref_child {
        return match dom.next_exposed_sibling(new_child) {
            Some(next) => apply_insert_before(dom, parent, new_child, next),
            None => apply_append_child(dom, parent, new_child),
        };
    }
    // §4.2.3 insertion "if parent is connected" for the dest record's
    // `connectedCallback` gate (inserting children never changes `parent`'s own
    // connectedness, so this equals insertion-time). See `apply_append_child`.
    let parent_was_connected = dom.is_connected(parent);
    if dom.is_document_fragment(new_child) {
        // §4.2.3 step 2 (atomic, before any move): a fragment that is a
        // host-including inclusive ancestor of `parent` is a HierarchyRequestError.
        // See `apply_append_child`. (`ref_child ∈ parent` already established above.)
        if dom.is_ancestor_or_self(new_child, parent) {
            return Vec::new();
        }
        // DocumentFragment: expand its children before `ref_child`. `previousSibling`
        // = step 6 = `ref_child`'s previous sibling, captured BEFORE the move;
        // `nextSibling` = `ref_child` (the insert reference child).
        let prev_sibling = dom.prev_exposed_sibling(ref_child);
        let Some((frag_record, nodes)) =
            expand_fragment(dom, new_child, |d, n| d.insert_before(parent, n, ref_child))
        else {
            return Vec::new(); // empty fragment → §4.2.3 step 3
        };
        let dest = MutationRecord {
            added_nodes: nodes,
            previous_sibling: prev_sibling,
            next_sibling: Some(ref_child),
            parent_was_connected,
            ..empty_record(MutationKind::ChildList, parent) // step 8
        };
        return vec![frag_record, dest];
    }
    // Move-vs-fresh SoT + source-removal context, captured before the relink.
    let source = capture_move_source(dom, new_child);
    // Destination `previousSibling` per DOM §4.2.3 insert **step 6** = `ref_child`'s
    // previous sibling, captured **before** the adopt — so for an
    // `insertBefore(node, node.nextSibling)` no-op move it is the moved node itself
    // (spec-mandated self-sibling, Codex PR384 R1), not its post-move predecessor.
    // `nextSibling` is `ref_child` (the insert reference child). `prev_exposed_sibling`
    // skips a closed `ShadowRoot` (a real ECS sibling filtered from the DOM view, §4.8).
    let prev_sibling = dom.prev_exposed_sibling(ref_child);
    if !dom.insert_before(parent, new_child, ref_child) {
        return Vec::new();
    }
    let dest = MutationRecord {
        added_nodes: vec![new_child],
        previous_sibling: prev_sibling,
        next_sibling: Some(ref_child),
        parent_was_connected,
        ..empty_record(MutationKind::ChildList, parent)
    };
    move_record_list(source, new_child, dest)
}

/// Remove `child` from `parent` through the `EcsDom` chokepoint and build the
/// §4.3.2 childList record (or `None` if `child` is not a child). Shared by the
/// deferred-flush path and the VM `removeChild` handler.
pub fn apply_remove_child(
    dom: &mut EcsDom,
    parent: Entity,
    child: Entity,
) -> Option<MutationRecord> {
    let prev_sibling = dom.prev_exposed_sibling(child);
    let next_sibling = dom.next_exposed_sibling(child);
    // DOM "remove" step 12 `isParentConnected`: capture the parent's
    // connectedness BEFORE the removal. Removing a child does not change the
    // parent's own connectedness, but capturing here (not at record delivery)
    // is what makes the record leg's disconnectedCallback gate survive a later
    // same-batch record detaching `parent`.
    let parent_was_connected = dom.is_connected(parent);
    if !dom.remove_child(parent, child) {
        return None;
    }
    Some(MutationRecord {
        removed_nodes: vec![child],
        previous_sibling: prev_sibling,
        next_sibling,
        parent_was_connected,
        ..empty_record(MutationKind::ChildList, parent)
    })
}

/// DOM §4.5 `adoptNode(node)` step 3 (`#dom-document-adoptnode`): *adopt*
/// `node` into `document` through [`EcsDom::adopt`] and return the records
/// it queues — the source-parent removal record when `node` is parented
/// (adopt step 2 runs *remove*, whose step 16 queues it, DOM §4.2.3
/// `#concept-node-remove`; not suppressed), else none. Adoption itself queues
/// no record: the node-document change is not a mutation record.
///
/// Steps 1–2 (a Document → `NotSupportedError`, a shadow root →
/// `HierarchyRequestError`) are the caller's (`elidex-dom-api`'s
/// `AdoptNode`); `node` reaching here is neither.
pub fn apply_adopt_node(dom: &mut EcsDom, node: Entity, document: Entity) -> Vec<MutationRecord> {
    // Captured before the write: the removal destroys the siblings.
    let record = source_removal_record(dom, node);
    dom.adopt(node, document);
    record.into_iter().collect()
}

/// Replace `old_child` with `new_child` under `parent` through the `EcsDom`
/// chokepoint and build the §4.2.3 "replace" childList record list — empty on
/// failure, the **single coalesced** record for a fresh `new_child`, and **two**
/// when `new_child` is already parented (a move into the replace slot): the
/// coalesced record (step 10) **plus** a source-parent removal record from
/// `new_child`'s adopt (step 6: adopt step 2 → remove, with
/// `suppressObservers` left at its default, so the source removal is observed).
///
/// Capture follows `#concept-node-replace` literally and is **distinct** from the
/// append/insert node-relative post-write rule — replace reads siblings
/// pre-removal, `old_child`-relative (steps 2–4). Shared by the deferred-flush
/// path and the VM `replaceChild` handler.
pub fn apply_replace_child(
    dom: &mut EcsDom,
    parent: Entity,
    new_child: Entity,
    old_child: Entity,
) -> Vec<MutationRecord> {
    // A shadow root is not a movable node (see `rejects_shadow_root_insertion`) —
    // reject before removing `old_child`, so a failed `replaceChild(shadowRoot, old)`
    // leaves the tree intact.
    if rejects_shadow_root_insertion(dom, new_child) {
        return Vec::new();
    }
    // DOM "remove" step 12 `isParentConnected` for the coalesced record's
    // `removedNodes = «old_child»` disconnected gate: `parent`'s connectedness
    // is unchanged by replacing its children, so capture once up front (before
    // any mutation) and reuse in both the fragment and non-fragment arms.
    let parent_was_connected = dom.is_connected(parent);
    if dom.is_document_fragment(new_child) {
        // DocumentFragment newChild — §4.2.3 "replace" steps 2–10 with expansion.
        // §4.2.3 replace step 1 (ensure pre-insert validity step 2; atomic, before
        // removing oldChild): a fragment that is a host-including inclusive
        // ancestor of `parent` is a HierarchyRequestError. See `apply_append_child`.
        if dom.is_ancestor_or_self(new_child, parent) {
            return Vec::new();
        }
        // Deferred-flush safety: `apply_mutation(Mutation::ReplaceChild)` skips the
        // dom-api handler's oldChild∈parent precheck, and this branch bypasses
        // `EcsDom::replace_child` (which would otherwise validate), so re-check here
        // (replace step 1: ensure pre-insert validity step 3).
        if dom.get_parent(old_child) != Some(parent) {
            return Vec::new();
        }
        // step 2: referenceChild = oldChild's next sibling. Step 3's
        // "if referenceChild is node" adjustment cannot fire — a fragment is
        // detached, never a sibling of oldChild (a fragment is never a live child;
        // the only producer of a live fragment child is the legacy variadic path,
        // converged in B1.2b — see PR #387 Codex R1 F4 defer).
        let reference_child = dom.next_exposed_sibling(old_child);
        // step 4: previousSibling = oldChild's previous sibling (pre-removal).
        let previous_sibling = dom.prev_exposed_sibling(old_child);
        // *replace* step 6: adopt node (the fragment itself) into parent's node
        // document. The fragment is parentless (adopt step 2 is a no-op), so
        // this re-homes the fragment and its children before step 7; the
        // children's own insertions at step 9 then find them in place.
        if let Some(document) = dom.node_document(parent) {
            dom.adopt(new_child, document);
        }
        // step 7: remove oldChild (suppressObservers — no standalone record).
        if !dom.remove_child(parent, old_child) {
            return Vec::new();
        }
        // steps 8–9: nodes = fragment's children; insert each before referenceChild
        // (or append when oldChild was last). Step 9's insert is suppressObservers,
        // so its destination record is withheld; its step-4.2 fragment record is not.
        let frag_and_nodes = expand_fragment(dom, new_child, |d, n| match reference_child {
            Some(rc) => d.insert_before(parent, n, rc),
            None => d.append_child(parent, n),
        });
        // step 10: one coalesced record (addedNodes = expanded children, removedNodes
        // = «oldChild»). Build it inside each arm so the expanded `nodes` move in (no
        // extra clone). For an empty fragment `addedNodes` is «» but the record is
        // still queued — oldChild was removed at step 7.
        let coalesced = |added_nodes: Vec<Entity>| MutationRecord {
            added_nodes,
            removed_nodes: vec![old_child],
            previous_sibling,
            next_sibling: reference_child,
            parent_was_connected,
            ..empty_record(MutationKind::ChildList, parent)
        };
        return match frag_and_nodes {
            // Order: step 9's fragment record (insert 4.2) THEN step 10's coalesced.
            Some((frag_record, nodes)) => vec![frag_record, coalesced(nodes)],
            // Empty fragment: no fragment record (nested insert returned at step 3),
            // but oldChild was still removed, so the coalesced record stands alone.
            None => vec![coalesced(Vec::new())],
        };
    }
    // Coalesced-record siblings (steps 2–4), captured pre-removal, old_child-relative:
    //  - previousSibling = step 4 = old_child's prev. NO adjustment exists, so this
    //    MAY legitimately equal `new_child` (e.g. `[A,B,C].replaceChild(A,B)` →
    //    previousSibling == A == new_child) — spec-faithful, do not "fix" it.
    let coalesced_prev = dom.prev_exposed_sibling(old_child);
    //  - nextSibling = step 2 (old_child's next) WITH the step-3 adjustment: if that
    //    next is `new_child` itself (a move where new_child was old_child's next
    //    sibling), referenceChild becomes new_child's next sibling instead. Step 3
    //    only fires on a move, so it is first observable here (B1 deferred moves).
    let coalesced_next = match dom.next_exposed_sibling(old_child) {
        Some(next) if next == new_child => dom.next_exposed_sibling(new_child),
        other => other,
    };
    // Source-removal record context from new_child's adopt (replace step 6 ->
    // adopt step 2 -> remove), captured pre-write with `old_child` still in place:
    // the spec removes `old_child` at step 7, after the adopt. Self-replace
    // (`replaceChild(X, X)`) is rejected by `EcsDom::replace_child` below (returns
    // false -> empty list), so it produces no record at all; elidex queues no record
    // here while the spec queues two (`#11-replace-child-self-records`: Chromium
    // unmeasured, C0a memo section 7); this context is simply discarded in that case.
    let source = capture_move_source(dom, new_child);
    if !dom.replace_child(parent, new_child, old_child) {
        return Vec::new();
    }
    let coalesced = MutationRecord {
        added_nodes: vec![new_child],
        removed_nodes: vec![old_child],
        previous_sibling: coalesced_prev,
        next_sibling: coalesced_next,
        parent_was_connected,
        ..empty_record(MutationKind::ChildList, parent)
    };
    // Order: source-removal (from step 6's adopt) THEN coalesced (step 10).
    move_record_list(source, new_child, coalesced)
}

/// WHATWG DOM §4.2.3 "replace all" (`#concept-node-replace-all`): remove **all** of
/// `parent`'s children (step 5, `suppressObservers` — no per-child removal records),
/// then insert `node` (step 6, `suppressObservers` — destination record folded into
/// the step-7 combined record), then queue the **one** coalesced `ChildList` record
/// for `parent` (step 7) **iff** `addedNodes ∪ removedNodes` is non-empty:
/// `removedNodes` = the prior children, `addedNodes` = `node`'s children (a
/// `DocumentFragment`) or «`node`» (any other) or «» (`None`), prev/next both null.
///
/// `suppressObservers` only suppresses the step-8 **destination** record. The records
/// the insert (step 6) still queues are emitted **before** the combined record:
/// a `DocumentFragment`'s step-4.2 fragment record (§4.2.3 "intentionally does not pay
/// attention to suppressObservers"), and an already-parented `node`'s §4.5 adopt
/// source-parent removal (the adopt's "remove" is unsuppressed). So a cross-parent
/// `replaceChildren(existingChild)` reports the source removal, and
/// `replaceChildren(frag)` reports `frag` emptying — both observable, matching spec.
///
/// The algorithm makes no tree-constraint checks (the spec note); callers
/// (`replaceChildren`, later `textContent`) run `ensure_pre_insertion_validity` first.
/// The sole b-1 consumer is `replaceChildren`; B1.2c reuses it for `textContent`.
pub fn apply_replace_all(
    dom: &mut EcsDom,
    parent: Entity,
    node: Option<Entity>,
) -> Vec<MutationRecord> {
    // A shadow root is not a movable node (see `rejects_shadow_root_insertion`) —
    // reject BEFORE the remove-all so `replaceChildren(shadowRoot)` leaves the parent
    // intact rather than clearing it then failing.
    if node.is_some_and(|n| rejects_shadow_root_insertion(dom, n)) {
        return Vec::new();
    }
    // step 1: removedNodes = parent's children. `child_list_uncapped` (NOT
    // `children`/`children_iter`, which truncate at MAX_ANCESTOR_DEPTH) so a >cap
    // parent does not silently drop its removedNodes tail (the #387 discipline).
    let removed_nodes: Vec<Entity> = dom.child_list_uncapped(parent);
    // step-12 `isParentConnected` for the combined record's `removedNodes`
    // disconnected gate: `parent`'s connectedness is unchanged by clearing its
    // children, captured before the remove-all.
    let parent_was_connected = dom.is_connected(parent);
    // step 5: remove all children, in tree order, suppressObservers (no per-child
    // record). `removed_nodes` is a static snapshot, so removing as we go is safe.
    for &child in &removed_nodes {
        let ok = dom.remove_child(parent, child);
        debug_assert!(
            ok,
            "replace all: child from child_list_uncapped must be removable"
        );
    }
    // step 6 insert: the unsuppressed source records (fragment step-4.2 record /
    // adopt source-parent removal) precede the step-7 combined record.
    let mut records: Vec<MutationRecord> = Vec::new();
    let added_nodes: Vec<Entity> = match node {
        None => Vec::new(),
        Some(n) if dom.is_document_fragment(n) => {
            // Fragment: its children move into parent (the fragment is never linked).
            // `expand_fragment` is the canonical snapshot+relink+moved-filter home
            // (#387 `child_list_uncapped`/never-truncate discipline); its step-4.2
            // fragment record (`frag` emptied) is NOT suppressed, so emit it.
            match expand_fragment(dom, n, |d, c| d.append_child(parent, c)) {
                Some((frag_record, moved)) => {
                    records.push(frag_record);
                    moved
                }
                None => Vec::new(),
            }
        }
        Some(n) => {
            // adopt source-parent removal (unsuppressed) for an already-parented node,
            // captured before the relink and pushed only once the relink lands.
            let source = source_removal_record(dom, n);
            if dom.append_child(parent, n) {
                records.extend(source);
                vec![n]
            } else {
                Vec::new()
            }
        }
    };
    // step 7: queue the combined record iff either set is non-empty.
    if !(added_nodes.is_empty() && removed_nodes.is_empty()) {
        records.push(MutationRecord {
            added_nodes,
            removed_nodes,
            parent_was_connected,
            ..empty_record(MutationKind::ChildList, parent)
        });
    }
    records
}
