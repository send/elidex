//! Record-leg custom-element reaction enqueue (S5-6b §4.3.1).
//!
//! The VM's `MutationObserver` delivery entry
//! ([`super::super::super::VmInner::deliver_mutation_records`]) owns the
//! record→CE conversion for **externally-delivered**
//! [`elidex_script_session::MutationRecord`]s (layout-derived /
//! shell-buffered) — the records that never rode the bind-installed
//! `CustomElementReactionConsumer` because they were not applied through
//! the VM's own `apply_*` primitives.
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", this file is **marshalling only**:
//! it reads the bound `EcsDom` + per-realm `ce_registry` + `ce_reaction_queue`
//! and derives each record's post-mutation connectivity, then calls the
//! ONE engine-independent classification in
//! [`elidex_custom_elements`] (`classify_connected_subtree` /
//! `classify_disconnected_subtree` / `classify_attribute_change`) — the
//! same functions the mutation-event leg routes through (single-homing
//! pin H1). No classification algorithm lives here.
//!
//! ## No-double-enqueue invariant
//!
//! VM-native mutations write the `EcsDom` immediately via `apply_*`, so
//! they reach the bind-installed dispatcher (CE custody) and never enter
//! `SessionCore::pending` — `session.flush` returns them EMPTY, so they
//! do not appear in the `records` slice here. External records reach CE
//! ONLY through this path; the dispatcher cannot double-hear them because
//! the shell runs `flush` UNBOUND (dispatcher cleared at `unbind`). The
//! two legs partition the mutation stream by construction.

#![cfg(feature = "engine")]

use std::sync::PoisonError;

use super::super::super::VmInner;

impl VmInner {
    /// Convert externally-delivered `MutationRecord`s into custom-element
    /// reactions and enqueue them on `ce_reaction_queue`, using the
    /// single [`elidex_custom_elements`] classification (H1). Enqueue
    /// only — the caller (`deliver_mutation_records`) leaves draining to
    /// the shell's post-deliver `drain_reactions`.
    ///
    /// A silent no-op when unbound (mirrors the sibling `notify_one` /
    /// `flush_ce_reactions` guards).
    pub(crate) fn enqueue_ce_reactions_from_records(
        &mut self,
        records: &[elidex_script_session::MutationRecord],
    ) {
        use elidex_script_session::MutationKind;

        if !self
            .host_data
            .as_deref()
            .is_some_and(super::super::super::host_data::HostData::is_bound)
        {
            return;
        }
        let host = self
            .host_data
            .as_deref_mut()
            .expect("enqueue_ce_reactions_from_records: HostData required when bound");
        // Clone the shared `Arc<Mutex<>>` handles up front so the bound
        // `&EcsDom` borrow (below) is the sole outstanding borrow of
        // `host` through the classification loop.
        let registry_arc = std::sync::Arc::clone(&host.ce_registry);
        let queue_arc = std::sync::Arc::clone(&host.ce_reaction_queue);
        let dom: &elidex_ecs::EcsDom = host.dom();

        let registry = registry_arc.lock().unwrap_or_else(PoisonError::into_inner);
        let mut reactions = Vec::new();
        for record in records {
            match record.kind {
                // Connectivity is derived from the record's target
                // (the mutation parent) post-mutation: added nodes are
                // its children (share its connectivity), and a still-
                // connected target means a removed child WAS connected
                // before removal. Records carry no per-node
                // connectivity, so this is the record leg's local gate
                // (the mutation-event leg uses the event's
                // `was_connected`); the subtree classification itself
                // is shared.
                MutationKind::ChildList if dom.is_connected(record.target) => {
                    for &added in &record.added_nodes {
                        reactions.extend(elidex_custom_elements::classify_connected_subtree(
                            added, dom, &registry,
                        ));
                    }
                    for &removed in &record.removed_nodes {
                        reactions.extend(elidex_custom_elements::classify_disconnected_subtree(
                            removed, dom,
                        ));
                    }
                }
                MutationKind::Attribute => {
                    if let Some(name) = record.attribute_name.as_deref() {
                        // The record carries `old_value`; the new value is
                        // the current DOM state (the record is delivered
                        // post-mutation).
                        let new_value = dom
                            .world()
                            .get::<&elidex_ecs::Attributes>(record.target)
                            .ok()
                            .and_then(|attrs| attrs.get(name).map(str::to_owned));
                        if let Some(reaction) = elidex_custom_elements::classify_attribute_change(
                            record.target,
                            name,
                            record.old_value.as_deref(),
                            new_value.as_deref(),
                            dom,
                            &registry,
                        ) {
                            reactions.push(reaction);
                        }
                    }
                }
                // CharacterData / CssRule carry no CE lifecycle reaction.
                _ => {}
            }
        }
        drop(registry);

        if !reactions.is_empty() {
            queue_arc
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .extend(reactions);
        }
    }
}
