//! Buffered DOM mutations and their application to the ECS DOM.
//!
//! The WHATWG HTML §8.5 HTML-fragment setters (`innerHTML` / `setHTMLUnsafe` /
//! `outerHTML` / `insertAdjacentHTML`) live in the [`html_fragment`] submodule;
//! everything else (the [`Mutation`] queue + the generic `apply_*` node
//! mutations) stays here.

use elidex_ecs::{Attributes, EcsDom, Entity, TextContent};

mod html_fragment;
use html_fragment::apply_insert_adjacent_html;
pub use html_fragment::{
    apply_set_inner_html, apply_set_outer_html, OuterHtmlError, SetInnerHtmlOptions,
};

/// A buffered DOM mutation recorded by script code.
///
/// Mutations are collected in [`SessionCore`](crate::SessionCore) and applied
/// atomically via [`flush()`](crate::SessionCore::flush).
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Mutation {
    /// Append `child` as the last child of `parent`.
    AppendChild {
        /// Parent entity.
        parent: Entity,
        /// Child entity to append.
        child: Entity,
    },
    /// Insert `new_child` before `ref_child` under `parent`.
    InsertBefore {
        /// Parent entity.
        parent: Entity,
        /// New child entity to insert.
        new_child: Entity,
        /// Existing child to insert before.
        ref_child: Entity,
    },
    /// Remove `child` from `parent`.
    RemoveChild {
        /// Parent entity.
        parent: Entity,
        /// Child entity to remove.
        child: Entity,
    },
    /// Replace `old_child` with `new_child` under `parent`.
    ReplaceChild {
        /// Parent entity.
        parent: Entity,
        /// New child entity.
        new_child: Entity,
        /// Old child entity to replace.
        old_child: Entity,
    },
    /// Set an attribute on an element.
    SetAttribute {
        /// Target entity.
        entity: Entity,
        /// Attribute name.
        name: String,
        /// Attribute value.
        value: String,
    },
    /// Remove an attribute from an element.
    RemoveAttribute {
        /// Target entity.
        entity: Entity,
        /// Attribute name to remove.
        name: String,
    },
    /// Set the text content of a text node.
    ///
    /// Currently only updates the [`TextContent`](elidex_ecs::TextContent)
    /// component directly. Full DOM `textContent` setter semantics for
    /// element nodes (removing all children and inserting a single text
    /// node) will be implemented in a later milestone.
    SetTextContent {
        /// Target entity.
        entity: Entity,
        /// New text content.
        text: String,
    },
    /// Set innerHTML — parses HTML fragment and replaces children.
    SetInnerHtml {
        /// Target entity.
        entity: Entity,
        /// HTML string to parse and insert.
        html: String,
    },
    /// Insert parsed HTML at a position relative to an element.
    ///
    /// Position: `"beforebegin"`, `"afterbegin"`, `"beforeend"`, `"afterend"`.
    InsertAdjacentHtml {
        /// Target entity.
        entity: Entity,
        /// Insertion position.
        position: String,
        /// HTML string to parse and insert.
        html: String,
    },
    /// Insert a CSS rule into a stylesheet (legacy variant, CSSOM uses bridge).
    InsertCssRule {
        /// Stylesheet entity.
        stylesheet: Entity,
        /// Index at which to insert.
        index: usize,
        /// CSS rule text.
        rule: String,
    },
    /// Delete a CSS rule from a stylesheet (legacy variant, CSSOM uses bridge).
    DeleteCssRule {
        /// Stylesheet entity.
        stylesheet: Entity,
        /// Index of the rule to delete.
        index: usize,
    },
}

/// The kind of mutation that was applied, for observer notifications.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MutationKind {
    /// A child was added, removed, or replaced.
    ChildList,
    /// An attribute was set or removed.
    Attribute,
    /// Text content was changed.
    CharacterData,
    /// A CSS rule was inserted or deleted.
    CssRule,
}

/// Record of a successfully applied mutation (WHATWG DOM §4.3.3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MutationRecord {
    /// The kind of mutation.
    pub kind: MutationKind,
    /// The primary target entity.
    pub target: Entity,
    /// Nodes added (for `ChildList` mutations).
    pub added_nodes: Vec<Entity>,
    /// Nodes removed (for `ChildList` mutations).
    pub removed_nodes: Vec<Entity>,
    /// The previous sibling of the mutation site.
    pub previous_sibling: Option<Entity>,
    /// The next sibling of the mutation site.
    pub next_sibling: Option<Entity>,
    /// The attribute name (for `Attribute` mutations).
    pub attribute_name: Option<String>,
    /// The old value (for `Attribute` or `CharacterData` mutations when requested).
    pub old_value: Option<String>,
}

/// Apply a single [`Mutation`] to the ECS DOM.
///
/// This is a low-level function. Prefer recording mutations via
/// [`SessionCore::record_mutation()`](crate::SessionCore::record_mutation)
/// and applying them with [`SessionCore::flush()`](crate::SessionCore::flush)
/// to ensure consistent buffering and future `MutationObserver` support.
///
/// Returns the list of [`MutationRecord`]s the mutation produced — empty on
/// failure (e.g. entity not found or tree constraint violation) and also for a
/// no-op such as appending an empty `DocumentFragment` (WHATWG DOM §4.2.3 insert
/// step 3, count 0 → return). A childList **move** (an already-parented node
/// passed to `appendChild`/`insertBefore`/`replaceChild`) yields **two** records
/// (a source-parent removal + a destination record, WHATWG DOM §4.5 adopt +
/// §4.2.3). A **`DocumentFragment`** insert/replace also yields **two** (the
/// step-4.2 fragment record + the destination/coalesced record carrying the
/// expanded children); every other successful mutation yields exactly one.
pub fn apply_mutation(mutation: &Mutation, dom: &mut EcsDom) -> Vec<MutationRecord> {
    match mutation {
        Mutation::AppendChild { parent, child } => apply_append_child(dom, *parent, *child),
        Mutation::InsertBefore {
            parent,
            new_child,
            ref_child,
        } => apply_insert_before(dom, *parent, *new_child, *ref_child),
        Mutation::RemoveChild { parent, child } => apply_remove_child(dom, *parent, *child)
            .into_iter()
            .collect(),
        Mutation::ReplaceChild {
            parent,
            new_child,
            old_child,
        } => apply_replace_child(dom, *parent, *new_child, *old_child),
        Mutation::SetAttribute {
            entity,
            name,
            value,
        } => apply_set_attribute(dom, *entity, name, value)
            .into_iter()
            .collect(),
        Mutation::RemoveAttribute { entity, name } => apply_remove_attribute(dom, *entity, name)
            .into_iter()
            .collect(),
        Mutation::SetTextContent { entity, text } => {
            apply_set_text(dom, *entity, text).into_iter().collect()
        }
        Mutation::SetInnerHtml { entity, html } => {
            apply_set_inner_html(dom, *entity, html, SetInnerHtmlOptions::default())
                .into_iter()
                .collect()
        }
        Mutation::InsertAdjacentHtml {
            entity,
            position,
            html,
        } => apply_insert_adjacent_html(dom, *entity, position, html)
            .into_iter()
            .collect(),
        // CSS rule mutations are handled directly by the HostBridge CSSOM layer
        // (not through the EcsDom mutation system). These variants are kept for
        // backward compat but are no longer reached in normal operation.
        Mutation::InsertCssRule { .. } | Mutation::DeleteCssRule { .. } => Vec::new(),
    }
}

pub(super) fn empty_record(kind: MutationKind, target: Entity) -> MutationRecord {
    MutationRecord {
        kind,
        target,
        added_nodes: Vec::new(),
        removed_nodes: Vec::new(),
        previous_sibling: None,
        next_sibling: None,
        attribute_name: None,
        old_value: None,
    }
}

/// The move source-removal context for a node passed to a childList op: its old
/// parent + exposed siblings, captured **before** the relink (which destroys
/// them), or `None` if the node was unparented (a fresh insert). This is the
/// move-vs-fresh source of truth for `appendChild` / `insertBefore`; `replace`
/// needs the `old_child` step-over and captures its own.
type MoveSource = Option<(Entity, Option<Entity>, Option<Entity>)>;

/// Capture the [`MoveSource`] for `node` (append/insert): old parent + `node`'s
/// exposed prev/next. The exposed-sibling helpers skip internal `ShadowRoot`
/// entities (§4.8 encapsulation). Must run before the `EcsDom` relink.
fn capture_move_source(dom: &EcsDom, node: Entity) -> MoveSource {
    dom.get_parent(node).map(|old_parent| {
        (
            old_parent,
            dom.prev_exposed_sibling(node),
            dom.next_exposed_sibling(node),
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
        Some((old_parent, previous_sibling, next_sibling)) => vec![
            MutationRecord {
                removed_nodes: vec![node],
                previous_sibling,
                next_sibling,
                ..empty_record(MutationKind::ChildList, old_parent)
            },
            primary,
        ],
        None => vec![primary],
    }
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
/// record-free `insert_node_expanding_fragment` in `elidex-dom-api` migrates onto
/// this in the B1.2b slice).
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

/// Append `child` to `parent` through the `EcsDom` chokepoint and build the
/// §4.3.2 "queue a tree mutation record" childList record list — empty on
/// failure, one record for a fresh node, **two** for a move (an already-parented
/// `child`: WHATWG DOM §4.5 adopt → §4.2.3 remove source record, NOT suppressed,
/// then the destination insertion record). Shared by the deferred-flush path
/// (`apply_mutation`) and the synchronous VM bridge handler (`AppendChild`) so
/// both runtimes produce the identical record shape — one record source
/// (One-issue-one-way).
pub fn apply_append_child(dom: &mut EcsDom, parent: Entity, child: Entity) -> Vec<MutationRecord> {
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
    if dom.is_document_fragment(new_child) {
        // §4.2.3 step 2 (atomic, before any move): a fragment that is a
        // host-including inclusive ancestor of `parent` is a HierarchyRequestError.
        // See `apply_append_child`.
        if dom.is_ancestor_or_self(new_child, parent) {
            return Vec::new();
        }
        // Validate the reference child up front: `EcsDom::insert_before` rejects a
        // `ref_child` that is not a child of `parent` (returns false), but the
        // fragment path drives the per-node relink through a closure that ignores
        // that failure — so without this check a bad reference would emit records
        // for children that were never inserted. Empty list ⇒ failure (the handler
        // maps it to an error; a *valid* empty-fragment no-op is distinguished by
        // the handler via fragment-ness, see `element/tree.rs`).
        if dom.get_parent(ref_child) != Some(parent) {
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
    if !dom.remove_child(parent, child) {
        return None;
    }
    Some(MutationRecord {
        removed_nodes: vec![child],
        previous_sibling: prev_sibling,
        next_sibling,
        ..empty_record(MutationKind::ChildList, parent)
    })
}

/// Replace `old_child` with `new_child` under `parent` through the `EcsDom`
/// chokepoint and build the §4.2.3 "replace" childList record list — empty on
/// failure, the **single coalesced** record for a fresh `new_child`, and **two**
/// when `new_child` is already parented (a move into the replace slot): the
/// coalesced record (step 14) **plus** a source-parent removal record from
/// `new_child`'s adopt (step 13's insert calls adopt → remove with
/// `suppressObservers` left at its default, so the source removal is observed).
///
/// Capture follows `#concept-node-replace` literally and is **distinct** from the
/// append/insert node-relative post-write rule — replace reads siblings
/// pre-removal, `old_child`-relative (steps 7–9). Shared by the deferred-flush
/// path and the VM `replaceChild` handler.
pub fn apply_replace_child(
    dom: &mut EcsDom,
    parent: Entity,
    new_child: Entity,
    old_child: Entity,
) -> Vec<MutationRecord> {
    if dom.is_document_fragment(new_child) {
        // DocumentFragment newChild — §4.2.3 "replace" steps 7-14 with expansion.
        // §4.2.3 replace step 2 (atomic, before removing oldChild): a fragment that
        // is a host-including inclusive ancestor of `parent` is a
        // HierarchyRequestError. See `apply_append_child`.
        if dom.is_ancestor_or_self(new_child, parent) {
            return Vec::new();
        }
        // Deferred-flush safety: `apply_mutation(Mutation::ReplaceChild)` skips the
        // dom-api handler's oldChild∈parent precheck, and this branch bypasses
        // `EcsDom::replace_child` (which would otherwise validate), so re-check here
        // (replace step 3).
        if dom.get_parent(old_child) != Some(parent) {
            return Vec::new();
        }
        // step 7: referenceChild = oldChild's next sibling. Step 8's
        // "if referenceChild is node" adjustment cannot fire — a fragment is
        // detached, never a sibling of oldChild (a fragment is never a live child;
        // the only producer of a live fragment child is the legacy variadic path,
        // converged in B1.2b — see PR #387 Codex R1 F4 defer).
        let reference_child = dom.next_exposed_sibling(old_child);
        // step 9: previousSibling = oldChild's previous sibling (pre-removal).
        let previous_sibling = dom.prev_exposed_sibling(old_child);
        // step 11: remove oldChild (suppressObservers — no standalone record).
        if !dom.remove_child(parent, old_child) {
            return Vec::new();
        }
        // steps 12-13: nodes = fragment's children; insert each before referenceChild
        // (or append when oldChild was last). Step 13's insert is suppressObservers,
        // so its destination record is withheld; its step-4.2 fragment record is not.
        let frag_and_nodes = expand_fragment(dom, new_child, |d, n| match reference_child {
            Some(rc) => d.insert_before(parent, n, rc),
            None => d.append_child(parent, n),
        });
        // step 14: one coalesced record (addedNodes = expanded children, removedNodes
        // = «oldChild»). Build it inside each arm so the expanded `nodes` move in (no
        // extra clone). For an empty fragment `addedNodes` is «» but the record is
        // still queued — oldChild was removed at step 11.
        let coalesced = |added_nodes: Vec<Entity>| MutationRecord {
            added_nodes,
            removed_nodes: vec![old_child],
            previous_sibling,
            next_sibling: reference_child,
            ..empty_record(MutationKind::ChildList, parent)
        };
        return match frag_and_nodes {
            // Order: step-13's fragment record (4.2) THEN step-14 coalesced.
            Some((frag_record, nodes)) => vec![frag_record, coalesced(nodes)],
            // Empty fragment: no fragment record (nested insert returned at step 3),
            // but oldChild was still removed, so the coalesced record stands alone.
            None => vec![coalesced(Vec::new())],
        };
    }
    // Coalesced-record siblings (steps 7–9), captured pre-removal, old_child-relative:
    //  - previousSibling = step 9 = old_child's prev. NO adjustment exists, so this
    //    MAY legitimately equal `new_child` (e.g. `[A,B,C].replaceChild(A,B)` →
    //    previousSibling == A == new_child) — spec-faithful, do not "fix" it.
    let coalesced_prev = dom.prev_exposed_sibling(old_child);
    //  - nextSibling = step 7 (old_child's next) WITH the step-8 adjustment: if that
    //    next is `new_child` itself (a move where new_child was old_child's next
    //    sibling), referenceChild becomes new_child's next sibling instead. Step 8
    //    only fires on a move, so it is first observable here (B1 deferred moves).
    let coalesced_next = match dom.next_exposed_sibling(old_child) {
        Some(next) if next == new_child => dom.next_exposed_sibling(new_child),
        other => other,
    };
    // Source-removal record context from new_child's adopt, captured pre-write
    // when new_child is already parented. Self-replace (`replaceChild(X, X)`) is
    // rejected by `EcsDom::replace_child` below (returns false → empty list), so
    // it produces no record at all — matching the VM handler's browser-parity
    // no-op short-circuit; this context is simply discarded in that case.
    let source = dom.get_parent(new_child).map(|old_parent| {
        // Source siblings = new_child's exposed prev/next, **stepping over
        // old_child** (spec removes old_child at step 11, before new_child's adopt
        // at step 13, so old_child is gone from new_child's sibling chain by adopt
        // time). The EcsDom primitive detaches new_child BEFORE old_child (reverse
        // order), so the step-over must be applied here.
        let prev = match dom.prev_exposed_sibling(new_child) {
            Some(s) if s == old_child => dom.prev_exposed_sibling(old_child),
            other => other,
        };
        let next = match dom.next_exposed_sibling(new_child) {
            Some(s) if s == old_child => dom.next_exposed_sibling(old_child),
            other => other,
        };
        (old_parent, prev, next)
    });
    if !dom.replace_child(parent, new_child, old_child) {
        return Vec::new();
    }
    let coalesced = MutationRecord {
        added_nodes: vec![new_child],
        removed_nodes: vec![old_child],
        previous_sibling: coalesced_prev,
        next_sibling: coalesced_next,
        ..empty_record(MutationKind::ChildList, parent)
    };
    // Order: source-removal (from step 13's adopt) THEN coalesced (step 14).
    move_record_list(source, new_child, coalesced)
}

fn apply_set_attribute(
    dom: &mut EcsDom,
    entity: Entity,
    name: &str,
    value: &str,
) -> Option<MutationRecord> {
    let mut attrs = dom.world_mut().get::<&mut Attributes>(entity).ok()?;
    let name = name.to_ascii_lowercase();
    let old_value = attrs.get(&name).map(str::to_owned);
    attrs.set(name.clone(), value.to_owned());
    drop(attrs);
    // This deferred-flush path mutates `Attributes` directly instead of
    // entering `EcsDom::set_attribute`, so it must run that chokepoint's
    // attribute-derived-component reconcile: drop a stale `InlineStyle` on a
    // buffered `style` write (else a later CSSOM write could resurrect the old
    // declarations — Codex #335 R10 F31) AND re-derive `IframeData` on a
    // buffered iframe-attribute write (else a flushed `setAttribute("src", …)`
    // would leave the component stale).
    dom.reconcile_attribute_derived_components(entity, &name);
    dom.rev_version(entity);
    Some(MutationRecord {
        attribute_name: Some(name),
        old_value,
        ..empty_record(MutationKind::Attribute, entity)
    })
}

fn apply_remove_attribute(dom: &mut EcsDom, entity: Entity, name: &str) -> Option<MutationRecord> {
    let mut attrs = dom.world_mut().get::<&mut Attributes>(entity).ok()?;
    let name = name.to_ascii_lowercase();
    let old_value = attrs.get(&name).map(str::to_owned);
    attrs.remove(&name);
    drop(attrs);
    // Same attribute-derived-component reconcile as `apply_set_attribute`
    // (Codex #335 R10 F31) — a buffered `removeAttribute("style")` drops the
    // hydrated `InlineStyle`, and a buffered iframe-attribute removal re-derives
    // `IframeData`.
    dom.reconcile_attribute_derived_components(entity, &name);
    dom.rev_version(entity);
    Some(MutationRecord {
        attribute_name: Some(name),
        old_value,
        ..empty_record(MutationKind::Attribute, entity)
    })
}

fn apply_set_text(dom: &mut EcsDom, entity: Entity, text: &str) -> Option<MutationRecord> {
    let old_value = dom
        .world()
        .get::<&TextContent>(entity)
        .ok()
        .map(|tc| tc.0.clone());
    // `set_text_data` bumps `rev_version(entity)` internally, so we
    // do not call it here.
    dom.set_text_data(entity, text)?;
    Some(MutationRecord {
        old_value,
        ..empty_record(MutationKind::CharacterData, entity)
    })
}

#[cfg(test)]
#[allow(unused_must_use)] // Test setup calls dom.append_child() etc. without checking return values
mod tests;
