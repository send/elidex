//! Buffered DOM mutations and their application to the ECS DOM.
//!
//! The WHATWG HTML §8.5 HTML-fragment setters (`innerHTML` / `setHTMLUnsafe` /
//! `outerHTML` / `insertAdjacentHTML`) live in the [`html_fragment`] submodule;
//! the WHATWG DOM childList family (`apply_append_child` / `apply_insert_before` /
//! `apply_remove_child` / `apply_replace_child` / `apply_replace_all` + the move
//! source-removal helpers) lives in the [`child_list`] submodule; everything
//! else (the [`Mutation`] queue + the attribute / text `apply_*` node mutations)
//! stays here.

use elidex_ecs::{EcsDom, Entity, TextContent};

mod child_list;
pub use child_list::{
    apply_append_child, apply_insert_before, apply_remove_child, apply_replace_all,
    apply_replace_child, convert_arg_source_records,
};
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
    /// The record `target` (the mutation parent)'s connectedness captured
    /// SYNCHRONOUSLY at **mutation time** — the same pre-mutation seam as
    /// `EcsDom::remove_child`'s `was_connected` on the mutation-event leg. It
    /// drives BOTH custom-element gates on the record leg, because the parent's
    /// own connectedness is unchanged by adding or removing its children:
    /// - `removed_nodes` → `disconnectedCallback`: WHATWG DOM "remove"
    ///   (`#concept-node-remove`) step 12 `isParentConnected`, captured BEFORE
    ///   the removal.
    /// - `added_nodes` → `connectedCallback`: §4.2.3 insertion steps "if parent
    ///   is connected", evaluated AT INSERTION.
    ///
    /// Enqueuing on this captured value (NOT connectivity re-derived at
    /// batch-final delivery) is what lets a reaction survive a **later record in
    /// the same batch detaching `target`** — the bug a delivery-time
    /// `is_connected(target)` gate causes on either side. Every `ChildList`
    /// apply path captures it; `false` only where `target` genuinely was not
    /// connected (e.g. a fragment source, never in the tree).
    pub parent_was_connected: bool,
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

/// The single canonical `characterData` record-shape builder (One-issue-one-way).
///
/// Every site that produces a DOM §4.10 "replace data"–shaped record — the
/// `apply_replace_data` primitive (mutate-and-record) and `Text.splitText`'s
/// record-only head-truncate hand-assembly (`split_text_at_offset`, where the
/// truncate is `set_text_data`-last and MUST NOT be re-mutated) — routes through
/// this builder so the `kind == CharacterData` / `old_value = Some(..)` coupling
/// is never hand-rolled. `old_value` MUST be the node's FULL data captured BEFORE
/// the splice (§4.10 step 4 queues the record with the pre-mutation data).
pub fn character_data_record(target: Entity, old_value: String) -> MutationRecord {
    MutationRecord {
        old_value: Some(old_value),
        ..empty_record(MutationKind::CharacterData, target)
    }
}

/// The single canonical `attributes` record-shape builder (One-issue-one-way).
///
/// Every site that produces a DOM §4.9 "attributes" record — the
/// [`apply_set_attribute`] / [`apply_remove_attribute`] primitives, which
/// mutate via the `EcsDom::set_attribute` / `remove_attribute` chokepoint and
/// record from the surfaced `oldValue` — routes through this builder so the
/// `kind == Attribute` / `attribute_name = Some(..)` / `old_value` coupling is
/// never hand-rolled. `name` is the already-resolved local name; `old_value`
/// is the value BEFORE the change (`None` for a newly-added attribute), which
/// §4.3.2 exposes only to observers with `attributeOldValue:true`.
/// `attributeNamespace` is parked (`#11-mutation-observer-extras`): this slice
/// records the namespace-less shape.
pub fn attribute_record(target: Entity, name: String, old_value: Option<String>) -> MutationRecord {
    MutationRecord {
        attribute_name: Some(name),
        old_value,
        ..empty_record(MutationKind::Attribute, target)
    }
}

// `pub(super)`: the cross-crate need is served by `character_data_record`
// (which couples kind + old_value), so `empty_record` stays in-crate
// (in-crate use by `apply_*` + tests + `html_fragment`).
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
        parent_was_connected: false,
    }
}

/// DOM §4.9 "set an attribute value" — record-producing primitive.
///
/// Routes the write through the real [`EcsDom::set_attribute`] chokepoint —
/// the FULL `MutationEvent::AttributeChange` → `ConsumerDispatcher` fan-out +
/// derived-component reconcile + materialized-`Attr` sync + CE tap (§4.9 steps
/// 2–3), all preserved — and builds the §4.9 step-1 "attributes" record from
/// the `oldValue` the chokepoint surfaces. (This REPLACES the prior buffered
/// bypass that mutated `Attributes` directly + hand-rolled a partial reconcile,
/// silently DROPPING the fan-out.) `name` is the already-resolved local name
/// (lowercased by the §4.9 `setAttribute()`-method layer where required); this
/// primitive does NOT re-case it. Returns `None` only when the write did not
/// land (destroyed / non-Element receiver); a successful write ALWAYS records,
/// including a same-value write (§4.9 "change an attribute" queues a record
/// unconditionally — invariant I4).
pub fn apply_set_attribute(
    dom: &mut EcsDom,
    entity: Entity,
    name: &str,
    value: &str,
) -> Option<MutationRecord> {
    let write = dom.set_attribute(entity, name, value);
    if !write.did_set {
        return None;
    }
    Some(attribute_record(entity, name.to_owned(), write.old_value))
}

/// DOM §4.9 "remove an attribute by name" — record-producing primitive.
///
/// Routes through the real [`EcsDom::remove_attribute`] chokepoint (fan-out +
/// reconcile + `Attr` sync preserved) and builds the §4.9 step-1 "attributes"
/// record from the removed value the chokepoint surfaces. Returns `None` — and
/// queues NO record — when nothing was removed: a destroyed / non-Element
/// receiver OR an absent attribute (`removeAttribute("missing")` performs no
/// mutation per §4.9 "remove an attribute by name", so observers see no
/// phantom removal — invariant I4). `name` is the resolved local name (not
/// re-cased here).
pub fn apply_remove_attribute(
    dom: &mut EcsDom,
    entity: Entity,
    name: &str,
) -> Option<MutationRecord> {
    dom.remove_attribute(entity, name)
        .map(|old_value| attribute_record(entity, name.to_owned(), Some(old_value)))
}

fn apply_set_text(dom: &mut EcsDom, entity: Entity, text: &str) -> Option<MutationRecord> {
    let old_value = dom
        .world()
        .get::<&TextContent>(entity)
        .ok()
        .map(|tc| tc.0.clone());
    // `set_text_data` bumps `rev_version(entity)` internally, so we
    // do not call it here. It returns `None` (→ early return) for a
    // non-Text entity, so reaching the record below guarantees the
    // entity carried `TextContent` both before and after — i.e.
    // `old_value` is `Some`. Route through the canonical
    // `character_data_record` builder (One-issue-one-way; same shape as
    // `apply_replace_data`).
    dom.set_text_data(entity, text)?;
    old_value.map(|ov| character_data_record(entity, ov))
}

/// DOM §4.10 "replace data" — record-producing primitive.
///
/// `old_value` MUST be the entity's FULL data string captured by the caller
/// BEFORE this call. DOM §4.10 "replace data" step 4 queues the characterData
/// mutation record passing node's data as the oldValue argument BEFORE steps 5-7
/// mutate the data; "queue a mutation record" (§4.3.2 step 3.3) then exposes that
/// oldValue only to observers with characterDataOldValue:true.
///
/// Precondition: `offset <= utf16_len(entity's data)` (caller validates and raises
/// IndexSizeError; this fn `debug_assert!`s via `replace_text_data` / the splice).
///
/// Returns `Some(CharacterData record)` on success, or `None` if `entity`
/// is neither a Text/CDATASection nor a Comment node (mirrors the
/// `Option`-returning convention of the sibling `apply_set_text` /
/// `apply_set_attribute`, so a non-CharacterData entity produces NO record
/// rather than a spurious one — wired callers brand-check, but the `Option`
/// makes the no-op structurally non-notifying). Text / CDATASection route
/// through `EcsDom::replace_text_data`; Comment routes through
/// `EcsDom::replace_comment_data`. Both fire `MutationEvent::ReplaceData`, so
/// the §4.10 boundary steps 8-11 live-range fixup applies uniformly to all
/// CharacterData incl. Comment (Codex PR426 R1 — the Comment event was added
/// once the F5-broadened `deleteContents` made the Comment splice reachable).
pub fn apply_replace_data(
    dom: &mut EcsDom,
    entity: Entity,
    offset: usize,
    count: usize,
    replacement: &str,
    old_value: String,
) -> Option<MutationRecord> {
    // Both EcsDom chokepoints return Some(new_len) iff the entity carries the
    // matching component, doubling as the Text-vs-Comment discriminator. Try
    // Text first, then Comment; `None` from both = not CharacterData = no record.
    let applied = dom
        .replace_text_data(entity, offset, count, replacement)
        .is_some()
        || dom
            .replace_comment_data(entity, offset, count, replacement)
            .is_some();
    applied.then(|| character_data_record(entity, old_value))
}

#[cfg(test)]
#[allow(unused_must_use)] // Test setup calls dom.append_child() etc. without checking return values
mod tests;

#[cfg(test)]
#[allow(unused_must_use)] // see `tests`
mod tests_replace_all;
