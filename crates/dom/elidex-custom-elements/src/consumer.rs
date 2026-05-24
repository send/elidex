//! [`CustomElementReactionConsumer`] — `MutationEvent` consumer that
//! enqueues custom element lifecycle reactions per WHATWG HTML §4.13.3.
//!
//! ECS-native first: lifecycle reactions are derived state — the source
//! of truth is the mutation stream from [`EcsDom`]. Per ECS first
//! principles, reaction enqueue belongs to a system subscribed to the
//! mutations that trigger lifecycle callbacks, NOT to ad-hoc IDL setter
//! call sites. Single consumer path uniformly covers
//! `setAttribute` / `removeAttribute` / parser-baked attributes /
//! `appendChild` / `removeChild` / `replaceChild` / `innerHTML`
//! mutations.
//!
//! ## Gating rules (HTML §4.13.3 "Custom element reactions")
//!
//! - `Connected` enqueued only on the disconnected → connected
//!   transition (`!was_connected && now-connected`). Within-connected
//!   subtree re-parents are no-ops per spec — `was_connected == true`
//!   skips the enqueue regardless of new parent.
//! - `Disconnected` enqueued only on the connected → disconnected
//!   transition (`was_connected == true`). Orphan-to-orphan moves are
//!   no-ops.
//! - `AttributeChanged` enqueued only when the attribute name is in
//!   the element's definition's `observed_attributes` (HTML §4.13.4
//!   "attribute change steps" — "for each attribute in element's
//!   attribute list that is in observedAttributes").
//! - All three gates ALSO require the element to be Custom
//!   ([`CEState::Custom`]) — pre-upgrade elements (`Undefined` /
//!   `Failed`) do NOT fire lifecycle callbacks per HTML §4.13.3.
//!
//! ## Subtree walk for Connected / Disconnected
//!
//! WHATWG DOM insertion / removal steps iterate shadow-including
//! inclusive descendants. The `Insert` / `Remove` mutation events fire
//! once per direct mutation root, so this consumer walks the shadow-
//! including subtree internally to enqueue per-custom-element
//! reactions for every descendant.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use elidex_ecs::{EcsDom, Entity, MutationEvent};

use crate::reaction::CustomElementReaction;
use crate::registry::CustomElementRegistry;
use crate::state::{CEState, CustomElementState};

/// Mutation consumer maintaining the custom element reaction queue.
///
/// Plain `pub fn handle(&mut self, event, dom)` entry, per the
/// sibling-consumer convention (`FormControlReconciler`,
/// `LiveRangeBridge`). Composed as a typed field of the binding-crate
/// `ConsumerDispatcher` (`elidex_js::vm::consumer_dispatcher`).
pub struct CustomElementReactionConsumer {
    registry: Arc<Mutex<CustomElementRegistry>>,
    queue: Arc<Mutex<VecDeque<CustomElementReaction>>>,
}

impl CustomElementReactionConsumer {
    /// Construct a consumer sharing `registry` and `queue` handles with
    /// the VM-side reaction-flush + `customElements.define` paths.
    #[must_use]
    pub fn new(
        registry: Arc<Mutex<CustomElementRegistry>>,
        queue: Arc<Mutex<VecDeque<CustomElementReaction>>>,
    ) -> Self {
        Self { registry, queue }
    }

    /// Single-method dispatch entry invoked by the binding-crate
    /// `ConsumerDispatcher`. Pattern-matches `Insert` / `Remove` /
    /// `AttributeChange`; other variants are no-ops.
    pub fn handle(&mut self, event: &MutationEvent<'_>, dom: &mut EcsDom) {
        match *event {
            MutationEvent::Insert {
                node,
                was_connected,
                ..
            } => self.handle_insert(node, was_connected, dom),
            MutationEvent::Remove {
                node,
                was_connected,
                ..
            } => self.handle_remove(node, was_connected, dom),
            MutationEvent::AttributeChange {
                node,
                name,
                old_value,
                new_value,
                ..
            } => self.handle_attribute_change(node, name, old_value, new_value, dom),
            _ => {}
        }
    }

    fn handle_insert(&mut self, node: Entity, _was_connected: bool, dom: &EcsDom) {
        // Skip when post-insert position is still disconnected
        // (orphan→orphan move): the insertion-steps loop only enqueues
        // when "parent is connected" per WHATWG DOM §4.2.3 step 7.
        //
        // NOTE: we deliberately do NOT short-circuit on `was_connected`
        // — both Blink and Gecko fire connectedCallback on every
        // insertion into a connected position, including within-tree
        // moves (the companion Remove already fired its Disconnected,
        // so suppressing the Insert side would leave the lifecycle in
        // a stale disconnected state).
        if !dom.is_connected(node) {
            return;
        }
        let mut enqueued = Vec::new();
        dom.for_each_shadow_inclusive_descendant(node, &mut |entity| {
            match try_to_upgrade_target(entity, &self.registry, dom) {
                UpgradeTarget::Connected => {
                    enqueued.push(CustomElementReaction::Connected(entity));
                }
                UpgradeTarget::Upgrade => {
                    // Per WHATWG DOM §4.2.3 insertion-steps + HTML
                    // §4.13.5 "try to upgrade element": an Undefined
                    // element with a registered definition gets
                    // `try to upgrade` on insertion. The Upgrade
                    // reaction's invoke_upgrade itself enqueues
                    // Connected once the constructor returns.
                    enqueued.push(CustomElementReaction::Upgrade(entity));
                }
                UpgradeTarget::None => {}
            }
        });
        if !enqueued.is_empty() {
            let mut queue = self.queue.lock().expect("CE reaction queue mutex poisoned");
            queue.extend(enqueued);
        }
    }

    fn handle_remove(&mut self, node: Entity, was_connected: bool, dom: &EcsDom) {
        // Disconnected fires only on connected→disconnected transition.
        if !was_connected {
            return;
        }
        let mut enqueued = Vec::new();
        dom.for_each_shadow_inclusive_descendant(node, &mut |entity| {
            if is_custom(entity, dom) {
                enqueued.push(CustomElementReaction::Disconnected(entity));
            }
        });
        if !enqueued.is_empty() {
            let mut queue = self.queue.lock().expect("CE reaction queue mutex poisoned");
            queue.extend(enqueued);
        }
    }

    fn handle_attribute_change(
        &mut self,
        node: Entity,
        name: &str,
        old_value: Option<&str>,
        new_value: Option<&str>,
        dom: &EcsDom,
    ) {
        // Only Custom-state elements receive `attributeChangedCallback`.
        let Some(def_name) = custom_definition_name(node, dom) else {
            return;
        };
        let registry = self.registry.lock().expect("CE registry mutex poisoned");
        let Some(def) = registry.get(&def_name) else {
            return;
        };
        // observed_attributes filter — HTML §4.13.4 "attribute change steps".
        if !def.observed_attributes.iter().any(|n| n == name) {
            return;
        }
        // Drop the registry guard before acquiring the queue guard
        // (lock-disjoint scope, sibling-consumer convention).
        drop(registry);
        let reaction = CustomElementReaction::AttributeChanged {
            entity: node,
            name: name.to_string(),
            old_value: old_value.map(str::to_owned),
            new_value: new_value.map(str::to_owned),
        };
        self.queue
            .lock()
            .expect("CE reaction queue mutex poisoned")
            .push_back(reaction);
    }
}

/// Returns `true` iff `entity` is a Custom element ready for lifecycle
/// callbacks (CEState::Custom). Pre-upgrade entities (Undefined / Failed)
/// do NOT receive callbacks per HTML §4.13.3.
fn is_custom(entity: Entity, dom: &EcsDom) -> bool {
    dom.world()
        .get::<&CustomElementState>(entity)
        .is_ok_and(|s| matches!(s.state, CEState::Custom))
}

/// Per-entity classification used by [`CustomElementReactionConsumer::
/// handle_insert`] to route between `Connected` and `Upgrade` per
/// WHATWG DOM §4.2.3 insertion-steps + HTML §4.13.3 reactions:
///
/// - `Connected` — Custom element already upgraded → enqueue
///   connectedCallback.
/// - `Upgrade` — Undefined element whose definition IS registered
///   (`try to upgrade element` branch). The Upgrade reaction's
///   `invoke_upgrade` will subsequently enqueue Connected on success.
/// - `None` — built-in element, Failed / Precustomized element, or
///   Undefined element without a registered definition.
enum UpgradeTarget {
    Connected,
    Upgrade,
    None,
}

fn try_to_upgrade_target(
    entity: Entity,
    registry: &Mutex<CustomElementRegistry>,
    dom: &EcsDom,
) -> UpgradeTarget {
    let Ok(state) = dom.world().get::<&CustomElementState>(entity) else {
        return UpgradeTarget::None;
    };
    match state.state {
        CEState::Custom => UpgradeTarget::Connected,
        CEState::Undefined => {
            let registry = registry.lock().expect("CE registry mutex poisoned");
            if registry.is_defined(&state.definition_name) {
                UpgradeTarget::Upgrade
            } else {
                UpgradeTarget::None
            }
        }
        // Failed / Precustomized / Uncustomized: no reaction.
        _ => UpgradeTarget::None,
    }
}

/// Returns the definition name iff `entity` is a Custom element.
fn custom_definition_name(entity: Entity, dom: &EcsDom) -> Option<String> {
    let state = dom.world().get::<&CustomElementState>(entity).ok()?;
    if matches!(state.state, CEState::Custom) {
        Some(state.definition_name.clone())
    } else {
        None
    }
}
