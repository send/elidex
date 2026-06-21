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
/// failure (e.g. entity not found, tree constraint violation, or stub
/// operation). A childList **move** (an already-parented node passed to
/// `appendChild`/`insertBefore`/`replaceChild`) yields **two** records (a
/// source-parent removal + a destination record, WHATWG DOM §4.5 adopt +
/// §4.2.3); every other successful mutation yields exactly one.
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

/// Append `child` to `parent` through the `EcsDom` chokepoint and build the
/// §4.3.2 "queue a tree mutation record" childList record list — empty on
/// failure, one record for a fresh node, **two** for a move (an already-parented
/// `child`: WHATWG DOM §4.5 adopt → §4.2.3 remove source record, NOT suppressed,
/// then the destination insertion record). Shared by the deferred-flush path
/// (`apply_mutation`) and the synchronous VM bridge handler (`AppendChild`) so
/// both runtimes produce the identical record shape — one record source
/// (One-issue-one-way).
pub fn apply_append_child(dom: &mut EcsDom, parent: Entity, child: Entity) -> Vec<MutationRecord> {
    // Move-vs-fresh source of truth: `child`'s old parent + siblings, captured
    // **before** the relink destroys them (`Some` ⇒ move, `None` ⇒ fresh).
    let source = capture_move_source(dom, child);
    if !dom.append_child(parent, child) {
        return Vec::new();
    }
    // Destination siblings: captured **post-write, node-relative** — correct for
    // both fresh and move (and never `child` itself, fixing the B1 R1 degenerate
    // `previousSibling == child` by construction). Uniform with insert/replace.
    let dest = MutationRecord {
        added_nodes: vec![child],
        previous_sibling: dom.prev_exposed_sibling(child),
        next_sibling: dom.next_exposed_sibling(child),
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
    // Move-vs-fresh SoT + source-removal context, captured before the relink.
    let source = capture_move_source(dom, new_child);
    if !dom.insert_before(parent, new_child, ref_child) {
        return Vec::new();
    }
    // Destination siblings: post-write, node-relative (`next` resolves to
    // `ref_child`; uniform with append/replace, fixes the move self-sibling case).
    let dest = MutationRecord {
        added_nodes: vec![new_child],
        previous_sibling: dom.prev_exposed_sibling(new_child),
        next_sibling: dom.next_exposed_sibling(new_child),
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
mod tests {
    use super::*;
    use elidex_ecs::Attributes;

    fn elem(dom: &mut EcsDom, tag: &str) -> Entity {
        dom.create_element(tag, Attributes::default())
    }

    /// Assert a mutation produced exactly one record and return it (the common
    /// single-record case; a childList move yields two — see the move tests).
    fn expect_one(records: Vec<MutationRecord>) -> MutationRecord {
        assert_eq!(records.len(), 1, "expected exactly one record");
        records.into_iter().next().unwrap()
    }

    #[test]
    fn apply_append_child() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let child = elem(&mut dom, "span");

        let m = Mutation::AppendChild { parent, child };
        let record = expect_one(apply_mutation(&m, &mut dom));
        assert_eq!(record.kind, MutationKind::ChildList);
        assert_eq!(record.target, parent);
        assert_eq!(record.added_nodes, vec![child]);
        assert!(record.removed_nodes.is_empty());
        assert_eq!(record.previous_sibling, None);
        assert_eq!(dom.children(parent), vec![child]);
    }

    #[test]
    fn apply_append_child_records_previous_sibling() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let first = elem(&mut dom, "span");
        let second = elem(&mut dom, "p");
        dom.append_child(parent, first);

        let m = Mutation::AppendChild {
            parent,
            child: second,
        };
        let record = expect_one(apply_mutation(&m, &mut dom));
        assert_eq!(record.previous_sibling, Some(first));
        assert_eq!(record.added_nodes, vec![second]);
    }

    #[test]
    fn apply_insert_before() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let a = elem(&mut dom, "span");
        let b = elem(&mut dom, "span");
        dom.append_child(parent, b);

        let m = Mutation::InsertBefore {
            parent,
            new_child: a,
            ref_child: b,
        };
        let record = expect_one(apply_mutation(&m, &mut dom));
        assert_eq!(record.kind, MutationKind::ChildList);
        assert_eq!(record.added_nodes, vec![a]);
        assert_eq!(record.next_sibling, Some(b));
        assert_eq!(dom.children(parent), vec![a, b]);
    }

    // --- B1.2a: move-record childList (already-parented node → two records) ---

    #[test]
    fn apply_append_child_same_parent_move_two_records() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let a = elem(&mut dom, "span");
        let b = elem(&mut dom, "span");
        dom.append_child(parent, a);
        dom.append_child(parent, b); // [a, b]

        // Move `a` to the end: [a, b] -> [b, a].
        let records = super::apply_append_child(&mut dom, parent, a);
        assert_eq!(
            records.len(),
            2,
            "a move emits source-removal + destination"
        );
        // Source-removal on the (same) parent: a left its old slot (prev None, next b).
        let src = &records[0];
        assert_eq!(src.target, parent);
        assert_eq!(src.removed_nodes, vec![a]);
        assert!(src.added_nodes.is_empty());
        assert_eq!(src.previous_sibling, None);
        assert_eq!(src.next_sibling, Some(b));
        // Destination: prev = b, NOT a — the B1 R1 self-sibling fix by construction.
        let dst = &records[1];
        assert_eq!(dst.target, parent);
        assert_eq!(dst.added_nodes, vec![a]);
        assert_eq!(dst.previous_sibling, Some(b));
        assert_eq!(dst.next_sibling, None);
        assert_eq!(dom.children(parent), vec![b, a]);
    }

    #[test]
    fn apply_append_child_cross_parent_move_two_records() {
        let mut dom = EcsDom::new();
        let p1 = elem(&mut dom, "div");
        let p2 = elem(&mut dom, "div");
        let a = elem(&mut dom, "span");
        let child = elem(&mut dom, "span");
        dom.append_child(p1, a);
        dom.append_child(p1, child); // p1 = [a, child]

        let records = super::apply_append_child(&mut dom, p2, child);
        assert_eq!(records.len(), 2);
        // Source-removal on the OLD parent p1.
        assert_eq!(records[0].target, p1);
        assert_eq!(records[0].removed_nodes, vec![child]);
        assert_eq!(records[0].previous_sibling, Some(a));
        assert_eq!(records[0].next_sibling, None);
        // Destination insertion on the NEW parent p2.
        assert_eq!(records[1].target, p2);
        assert_eq!(records[1].added_nodes, vec![child]);
        assert_eq!(records[1].previous_sibling, None);
        assert_eq!(records[1].next_sibling, None);
    }

    #[test]
    fn apply_insert_before_cross_parent_move_two_records() {
        let mut dom = EcsDom::new();
        let p1 = elem(&mut dom, "div");
        let p2 = elem(&mut dom, "div");
        let moved = elem(&mut dom, "span");
        let r = elem(&mut dom, "span");
        dom.append_child(p1, moved);
        dom.append_child(p2, r);

        let records = super::apply_insert_before(&mut dom, p2, moved, r);
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].target, p1);
        assert_eq!(records[0].removed_nodes, vec![moved]);
        let dst = &records[1];
        assert_eq!(dst.target, p2);
        assert_eq!(dst.added_nodes, vec![moved]);
        assert_eq!(dst.previous_sibling, None);
        assert_eq!(dst.next_sibling, Some(r));
        assert_eq!(dom.children(p2), vec![moved, r]);
    }

    #[test]
    fn apply_replace_child_cross_parent_move_source_plus_coalesced() {
        let mut dom = EcsDom::new();
        let p1 = elem(&mut dom, "div");
        let p2 = elem(&mut dom, "div");
        let before = elem(&mut dom, "span");
        let newc = elem(&mut dom, "span");
        let oldc = elem(&mut dom, "span");
        dom.append_child(p1, before);
        dom.append_child(p1, newc); // p1 = [before, newc]
        dom.append_child(p2, oldc); // p2 = [oldc]

        let records = super::apply_replace_child(&mut dom, p2, newc, oldc);
        assert_eq!(records.len(), 2);
        // Source-removal on newc's old parent p1.
        assert_eq!(records[0].target, p1);
        assert_eq!(records[0].removed_nodes, vec![newc]);
        assert_eq!(records[0].previous_sibling, Some(before));
        assert_eq!(records[0].next_sibling, None);
        // Coalesced replace record on p2.
        let c = &records[1];
        assert_eq!(c.target, p2);
        assert_eq!(c.added_nodes, vec![newc]);
        assert_eq!(c.removed_nodes, vec![oldc]);
        assert_eq!(dom.children(p2), vec![newc]);
    }

    #[test]
    fn apply_replace_child_move_step8_referencechild_adjustment() {
        // [A,B,C].replaceChild(C, B): newC (C) is oldC (B)'s next sibling.
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let a = elem(&mut dom, "span");
        let b = elem(&mut dom, "span");
        let c = elem(&mut dom, "span");
        dom.append_child(parent, a);
        dom.append_child(parent, b);
        dom.append_child(parent, c); // [A, B, C]

        let records = super::apply_replace_child(&mut dom, parent, c, b);
        assert_eq!(records.len(), 2);
        // Source-removal: C's siblings step over B (removed first) -> prev = A.
        assert_eq!(records[0].target, parent);
        assert_eq!(records[0].removed_nodes, vec![c]);
        assert_eq!(records[0].previous_sibling, Some(a));
        assert_eq!(records[0].next_sibling, None);
        // Coalesced: step-8 next = C's next (None), NOT C itself; prev = B's prev = A.
        let coalesced = &records[1];
        assert_eq!(coalesced.added_nodes, vec![c]);
        assert_eq!(coalesced.removed_nodes, vec![b]);
        assert_eq!(coalesced.previous_sibling, Some(a));
        assert_eq!(coalesced.next_sibling, None);
        assert_eq!(dom.children(parent), vec![a, c]);
    }

    #[test]
    fn apply_replace_child_move_prev_may_equal_new_child() {
        // [A,B,C].replaceChild(A, B): coalesced previousSibling == A == newChild
        // is spec-faithful (replace step 9 has no adjustment).
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let a = elem(&mut dom, "span");
        let b = elem(&mut dom, "span");
        let c = elem(&mut dom, "span");
        dom.append_child(parent, a);
        dom.append_child(parent, b);
        dom.append_child(parent, c); // [A, B, C]

        let records = super::apply_replace_child(&mut dom, parent, a, b);
        assert_eq!(records.len(), 2);
        // Source-removal: A's siblings step over B -> next = C, prev None.
        assert_eq!(records[0].removed_nodes, vec![a]);
        assert_eq!(records[0].previous_sibling, None);
        assert_eq!(records[0].next_sibling, Some(c));
        // Coalesced: previousSibling = B's prev = A = newChild (spec-faithful).
        let coalesced = &records[1];
        assert_eq!(coalesced.added_nodes, vec![a]);
        assert_eq!(coalesced.removed_nodes, vec![b]);
        assert_eq!(coalesced.previous_sibling, Some(a));
        assert_eq!(coalesced.next_sibling, Some(c));
    }

    #[test]
    fn apply_replace_child_self_replace_no_records() {
        // replaceChild(X, X): rejected by EcsDom::replace_child -> no record
        // (pre-existing browser-parity no-op, unchanged by B1.2a).
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let x = elem(&mut dom, "span");
        dom.append_child(parent, x);

        let records = super::apply_replace_child(&mut dom, parent, x, x);
        assert!(records.is_empty());
        assert_eq!(dom.children(parent), vec![x]);
    }

    #[test]
    fn apply_set_attribute() {
        let mut dom = EcsDom::new();
        let e = elem(&mut dom, "div");

        let m = Mutation::SetAttribute {
            entity: e,
            name: "class".into(),
            value: "active".into(),
        };
        let record = expect_one(apply_mutation(&m, &mut dom));
        assert_eq!(record.kind, MutationKind::Attribute);
        assert_eq!(record.attribute_name.as_deref(), Some("class"));
        assert_eq!(record.old_value, None);

        let attrs = dom.world().get::<&Attributes>(e).unwrap();
        assert_eq!(attrs.get("class"), Some("active"));
    }

    #[test]
    fn apply_set_attribute_records_old_value() {
        let mut dom = EcsDom::new();
        let e = elem(&mut dom, "div");
        {
            let mut attrs = dom.world_mut().get::<&mut Attributes>(e).unwrap();
            attrs.set("class", "old");
        }

        let m = Mutation::SetAttribute {
            entity: e,
            name: "class".into(),
            value: "new".into(),
        };
        let record = expect_one(apply_mutation(&m, &mut dom));
        assert_eq!(record.old_value.as_deref(), Some("old"));
    }

    #[test]
    fn apply_remove_attribute() {
        let mut dom = EcsDom::new();
        let e = elem(&mut dom, "div");
        {
            let mut attrs = dom.world_mut().get::<&mut Attributes>(e).unwrap();
            attrs.set("id", "test");
        }

        let m = Mutation::RemoveAttribute {
            entity: e,
            name: "id".into(),
        };
        let record = expect_one(apply_mutation(&m, &mut dom));
        assert_eq!(record.kind, MutationKind::Attribute);
        assert_eq!(record.attribute_name.as_deref(), Some("id"));
        assert_eq!(record.old_value.as_deref(), Some("test"));

        let attrs = dom.world().get::<&Attributes>(e).unwrap();
        assert!(!attrs.contains("id"));
    }

    #[test]
    fn apply_set_text_content() {
        let mut dom = EcsDom::new();
        let text = dom.create_text("hello");

        let m = Mutation::SetTextContent {
            entity: text,
            text: "world".into(),
        };
        let record = expect_one(apply_mutation(&m, &mut dom));
        assert_eq!(record.kind, MutationKind::CharacterData);
        assert_eq!(record.old_value.as_deref(), Some("hello"));

        let tc = dom.world().get::<&TextContent>(text).unwrap();
        assert_eq!(tc.0, "world");
    }

    #[test]
    fn apply_remove_child() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let a = elem(&mut dom, "span");
        let b = elem(&mut dom, "p");
        dom.append_child(parent, a);
        dom.append_child(parent, b);

        let m = Mutation::RemoveChild { parent, child: a };
        let record = expect_one(apply_mutation(&m, &mut dom));
        assert_eq!(record.kind, MutationKind::ChildList);
        assert_eq!(record.target, parent);
        assert_eq!(record.removed_nodes, vec![a]);
        assert_eq!(record.previous_sibling, None);
        assert_eq!(record.next_sibling, Some(b));
        assert_eq!(dom.children(parent), vec![b]);
    }

    #[test]
    fn apply_replace_child() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let old = elem(&mut dom, "span");
        let new = elem(&mut dom, "p");
        dom.append_child(parent, old);

        let m = Mutation::ReplaceChild {
            parent,
            new_child: new,
            old_child: old,
        };
        let record = expect_one(apply_mutation(&m, &mut dom));
        assert_eq!(record.kind, MutationKind::ChildList);
        assert_eq!(record.added_nodes, vec![new]);
        assert_eq!(record.removed_nodes, vec![old]);
        assert_eq!(dom.children(parent), vec![new]);
        assert_eq!(dom.get_parent(old), None);
    }

    #[test]
    fn apply_append_child_does_not_leak_shadow_root_as_previous_sibling() {
        // PR201 Copilot R4 / F3 regression: `apply_append_child` was
        // capturing `prev_sibling` via raw `get_last_child(parent)`,
        // which returns the internal ShadowRoot when the host has no
        // light-tree children yet. The fix walks via
        // `children_iter_rev` (which skips ShadowRoot entities).
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let host = elem(&mut dom, "div");
        let _ = dom.append_child(root, host);
        let shadow_root = dom
            .attach_shadow(host, elidex_ecs::ShadowRootMode::Closed)
            .expect("attach closed shadow");
        // Sanity: raw `get_last_child(host)` IS the shadow root —
        // confirms the helper would leak without the fix.
        assert_eq!(
            dom.get_last_child(host),
            Some(shadow_root),
            "shadow root is the only sibling at this point"
        );
        let new_child = elem(&mut dom, "span");
        let m = Mutation::AppendChild {
            parent: host,
            child: new_child,
        };
        let record = expect_one(apply_mutation(&m, &mut dom));
        assert_ne!(
            record.previous_sibling,
            Some(shadow_root),
            "MutationRecord.previous_sibling must not leak shadow root"
        );
        assert_eq!(
            record.previous_sibling, None,
            "no exposed prev sibling (shadow root skipped)"
        );
    }

    #[test]
    fn apply_remove_child_does_not_leak_shadow_root_as_previous_sibling() {
        // Pre-existing apply_remove_child path now uses
        // `prev_exposed_sibling` too. Lock the no-leak invariant.
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let host = elem(&mut dom, "div");
        let _ = dom.append_child(root, host);
        let shadow_root = dom
            .attach_shadow(host, elidex_ecs::ShadowRootMode::Closed)
            .expect("attach closed shadow");
        let child = elem(&mut dom, "span");
        let _ = dom.append_child(host, child);
        assert_eq!(dom.get_prev_sibling(child), Some(shadow_root));
        let m = Mutation::RemoveChild {
            parent: host,
            child,
        };
        let record = expect_one(apply_mutation(&m, &mut dom));
        assert_ne!(record.previous_sibling, Some(shadow_root));
        assert_eq!(record.previous_sibling, None);
    }

    /// Codex #335 R10 F31: a buffered `style` attribute mutation applied via
    /// the deferred flush (which bypasses `EcsDom::set_attribute`) must
    /// still invalidate a lazily-hydrated `InlineStyle` cache, else a later
    /// CSSOM read resurrects stale declarations.
    #[test]
    fn apply_style_attribute_invalidates_inline_style_cache() {
        let mut dom = EcsDom::new();
        let e = elem(&mut dom, "div");
        {
            let mut attrs = dom.world_mut().get::<&mut Attributes>(e).unwrap();
            attrs.set("style", "color: red");
        }
        // Simulate a prior `el.style.*` read that hydrated the cache.
        let mut style = elidex_ecs::InlineStyle::default();
        style.set("color", "red");
        dom.world_mut().insert_one(e, style).unwrap();
        assert!(dom.world().get::<&elidex_ecs::InlineStyle>(e).is_ok());

        // A buffered SetAttribute("style", …) must drop the stale cache.
        let m = Mutation::SetAttribute {
            entity: e,
            name: "style".into(),
            value: "color: blue".into(),
        };
        expect_one(apply_mutation(&m, &mut dom));
        assert!(
            dom.world().get::<&elidex_ecs::InlineStyle>(e).is_err(),
            "buffered SetAttribute('style') left a stale InlineStyle cache"
        );

        // Re-hydrate, then a buffered RemoveAttribute must also drop it.
        let mut style = elidex_ecs::InlineStyle::default();
        style.set("color", "blue");
        dom.world_mut().insert_one(e, style).unwrap();
        let m = Mutation::RemoveAttribute {
            entity: e,
            name: "style".into(),
        };
        expect_one(apply_mutation(&m, &mut dom));
        assert!(
            dom.world().get::<&elidex_ecs::InlineStyle>(e).is_err(),
            "buffered RemoveAttribute('style') left a stale InlineStyle cache"
        );
    }
}
