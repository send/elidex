//! [`CustomElementReactionConsumer`] — `MutationEvent` consumer that
//! enqueues custom element lifecycle reactions per WHATWG HTML §4.13.6.
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
//! ## Gating rules (HTML §4.13.6 "Custom element reactions")
//!
//! - `Connected` enqueued on every insertion into a connected position
//!   (`now-connected == true`), regardless of `was_connected`. Both
//!   Blink and Gecko fire connectedCallback on within-tree moves
//!   (the companion implicit Remove already fired its Disconnected,
//!   so suppressing the Insert side would leave the lifecycle in a
//!   stale disconnected state). `was_connected` is therefore NOT
//!   consulted in `handle_insert` — see the deliberate `_` binding.
//! - `Disconnected` enqueued only on the connected → disconnected
//!   transition (`was_connected == true`). Orphan-to-orphan moves are
//!   no-ops.
//! - `AttributeChanged` enqueued only when the attribute name is in
//!   the element's definition's `observed_attributes` (HTML §4.13.6
//!   "enqueue a custom element callback reaction" step 5.2: "If
//!   definition's observed attributes does not contain attributeName,
//!   then return").
//! - `Upgrade` enqueued on `MutationEvent::CloneCreated` for every element
//!   the clone created whose definition lookup is non-null (DOM §4.4
//!   "clone a node" → "clone a single node"), in the clone's creation
//!   order; the walk is `EcsDom::for_each_clone_created` and the registry-
//!   dependent half is [`crate::upgrade`]'s `apply_clone_creation_ce_semantics`.
//!   `handle_insert` enqueues `Upgrade` too (DOM "insert" step 7.7.3.3, "try
//!   to upgrade"), not only the `CloneCreated` path. The clone path also
//!   clears the retained `is` value of async-autonomous copies
//!   (`CustomElementState.is_value`, which no other consumer reads).
//! - `Adopted` enqueued on `MutationEvent::Adopt` for every Custom element
//!   of the adopted subtree whose pair is `Some` (DOM §4.5 *adopt* step
//!   3.3.3), each with its innermost contents segment's
//!   `(oldDocument, document)`, in the adopt walk's order (template contents
//!   right after their template).
//! - The first four gates (`Connected` / `Disconnected` /
//!   `AttributeChanged` / `Adopted`) ALSO require the element to be Custom
//!   ([`CEState::Custom`]) — pre-upgrade elements (`Undefined` /
//!   `Failed`) do NOT fire lifecycle callbacks per HTML §4.13.6. The
//!   `Upgrade` bullet is the exception: it enqueues for elements that are
//!   neither Custom nor Failed.
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

use elidex_ecs::{
    AdoptedContents, CloneScope, EcsDom, Entity, MutationEvent, TemplateContentsWalk,
};

use crate::reaction::CustomElementReaction;
use crate::registry::CustomElementRegistry;
use crate::state::{CEState, CustomElementState};
use crate::upgrade::apply_clone_creation_ce_semantics;

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
    /// `AttributeChange` / `CloneCreated` / `Adopt`; other variants are
    /// no-ops.
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
            MutationEvent::CloneCreated { root, scope } => {
                self.handle_clone_created(root, scope, dom);
            }
            MutationEvent::Adopt {
                node,
                documents,
                contents,
            } => self.handle_adopt(node, documents, contents, dom),
            _ => {}
        }
    }

    fn handle_insert(&mut self, node: Entity, _was_connected: bool, dom: &EcsDom) {
        // Skip when post-insert position is still disconnected
        // (orphan→orphan move): the insertion-steps loop only enqueues
        // when the inclusive descendant is connected per WHATWG DOM §4.2.3
        // "insert" step 7.7.2 ("If inclusiveDescendant is not connected, then
        // continue").
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
        // Single-homed classification (H1): both this mutation-event leg
        // and the VM record leg (`deliver_mutation_records`) route through
        // [`classify_connected_subtree`].
        let enqueued = {
            let registry = self.registry.lock().expect("CE registry mutex poisoned");
            classify_connected_subtree(node, dom, &registry)
        };
        if !enqueued.is_empty() {
            let mut queue = self.queue.lock().expect("CE reaction queue mutex poisoned");
            queue.extend(enqueued);
        }
    }

    /// DOM §4.4 "clone a node": the copy now exists. Apply the registry-
    /// dependent creation semantics (DOM §4.9 "create an element" step 5.2
    /// null-is rule) to every created element and enqueue one `Upgrade`
    /// reaction per upgrade candidate, in creation order
    /// (`for_each_clone_created`'s order).
    fn handle_clone_created(&mut self, root: Entity, scope: CloneScope, dom: &mut EcsDom) {
        // Unlike the other arms, both locks recover from poison: this arm
        // keeps the deleted VM seam's behaviour (`enqueue_subtree_upgrade_reactions`
        // in `vm/host/custom_elements/upgrade.rs` recovered poisoned guards),
        // so `cloneNode` of a detached tree does not panic, as in the base.
        // The other arms' `.expect` policy is unchanged.
        let candidates = {
            let registry = self
                .registry
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            apply_clone_creation_ce_semantics(dom, &registry, root, scope)
        };
        if !candidates.is_empty() {
            let mut queue = self
                .queue
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            queue.extend(candidates.into_iter().map(CustomElementReaction::Upgrade));
        }
    }

    /// DOM §4.5 *adopt* step 3.3.3: for each shadow-including inclusive
    /// descendant that is custom, "enqueue a custom element callback reaction
    /// with inclusiveDescendant, callback name "adoptedCallback", and
    /// « oldDocument, document »" (HTML §4.13.6 *enqueue a custom element
    /// callback reaction*).
    ///
    /// One `Adopted` per `CEState::Custom` element whose pair is `Some`, in
    /// DOM §4.8 *shadow-including tree order* with each template's contents
    /// right after the template (adopt step 3.4; the walk's `Include` order).
    /// An element's pair is its innermost segment's (see
    /// [`MutationEvent::Adopt`]): the root's while every contents fragment
    /// reached so far is a segment with the root's pair, else the lookup.
    ///
    /// Batch equivalence: the spec interleaves the stamp and the enqueue per
    /// descendant; enqueueing runs no script (reactions run at the next
    /// drain), so enqueueing after the walk, in the same order, is
    /// unobservable.
    fn handle_adopt(
        &mut self,
        node: Entity,
        documents: Option<(Entity, Entity)>,
        contents: &[AdoptedContents],
        dom: &EcsDom,
    ) {
        // `is_custom` needs a `CustomElementState`: with none in the World the
        // walk can enqueue nothing.
        if dom
            .world()
            .query::<&CustomElementState>()
            .iter()
            .next()
            .is_none()
        {
            return;
        }
        // A segment whose pair differs from the root's, or a reached
        // fragment without a segment (one the walk skipped), needs the lookup.
        let mut lookup = contents.iter().any(|s| s.documents != documents);
        // `contents` is in the walk's template order, so while `lookup` is
        // false the k-th template reached has its fragment at `contents[k]`;
        // a skipped fragment breaks that correspondence and sets `lookup`.
        let mut next = 0;
        let mut enqueued = Vec::new();
        dom.for_each_uncapped_shadow_inclusive(node, TemplateContentsWalk::Include, &mut |e| {
            if !lookup {
                if let Some(fragment) = dom.template_contents_fragment(e) {
                    lookup = contents.get(next).map(|s| s.fragment) != Some(fragment);
                    next += 1;
                }
            }
            if !is_custom(e, dom) {
                return;
            }
            let pair = if lookup {
                adopt_pair(dom, e, node, documents, contents)
            } else {
                documents
            };
            if let Some((old_document, new_document)) = pair {
                enqueued.push(CustomElementReaction::Adopted {
                    entity: e,
                    old_document,
                    new_document,
                });
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
        // Single-homed classification (H1): shared with the record leg.
        let enqueued = classify_disconnected_subtree(node, dom);
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
        // Single-homed classification (H1): shared with the record leg.
        // Scope the registry guard so it drops before the queue guard is
        // acquired (lock-disjoint, sibling-consumer convention).
        let reaction = {
            let registry = self.registry.lock().expect("CE registry mutex poisoned");
            classify_attribute_change(node, name, old_value, new_value, dom, &registry)
        };
        if let Some(reaction) = reaction {
            self.queue
                .lock()
                .expect("CE reaction queue mutex poisoned")
                .push_back(reaction);
        }
    }
}

// ---------------------------------------------------------------------------
// Single-homed record→CE classification (S5-6b §4.3.1, single-homing pin H1)
// ---------------------------------------------------------------------------
//
// The following three free functions are the ONE classification
// implementation shared by the two legs that turn a DOM mutation into
// custom element lifecycle reactions:
//
// - the [`CustomElementReactionConsumer`] mutation-event leg above
//   (`MutationEvent` from the bind-installed dispatcher), and
// - the VM's record leg
//   (`Vm::deliver_mutation_records` → `enqueue_ce_reactions_from_records`)
//   which marshals `elidex_script_session::MutationRecord`s into these
//   calls.
//
// Neither leg re-implements the per-entity classification: connectivity
// gating (the input each leg derives from its own source — an event's
// `was_connected` flag vs a record's post-mutation target connectivity)
// stays at the call site, but the subtree walk + `observed_attributes`
// gating live here once.

/// Classify the shadow-inclusive subtree rooted at `inserted_root` into
/// `Connected` / `Upgrade` reactions (HTML §4.13.6 + WHATWG DOM §4.2.3
/// insertion steps). The caller has already verified the post-insert
/// position is connected.
///
/// Snapshots the registry's defined-name set ONCE before the walk so
/// each descendant is an O(1) `HashSet::contains` classification rather
/// than a per-entity registry lock (large inserted subtrees drop O(N)
/// lock/unlock pairs to 1).
#[must_use]
pub fn classify_connected_subtree(
    inserted_root: Entity,
    dom: &EcsDom,
    registry: &CustomElementRegistry,
) -> Vec<CustomElementReaction> {
    let defined: std::collections::HashSet<String> = registry.names().map(str::to_owned).collect();
    let mut enqueued = Vec::new();
    dom.for_each_shadow_inclusive_descendant(inserted_root, &mut |entity| {
        match try_to_upgrade_target(entity, &defined, dom) {
            UpgradeTarget::Connected => {
                enqueued.push(CustomElementReaction::Connected(entity));
            }
            UpgradeTarget::Upgrade => {
                // Per WHATWG DOM §4.2.3 insertion-steps + HTML §4.13.5
                // "try to upgrade element": an Undefined element with a
                // registered definition gets `try to upgrade` on
                // insertion. The Upgrade reaction's `invoke_upgrade`
                // itself enqueues Connected once the constructor returns.
                enqueued.push(CustomElementReaction::Upgrade(entity));
            }
            UpgradeTarget::None => {}
        }
    });
    enqueued
}

/// Classify the shadow-inclusive subtree rooted at `removed_root` into
/// `Disconnected` reactions (one per Custom element). The caller has
/// verified the pre-removal position was connected — `Disconnected`
/// fires only on the connected → disconnected transition.
#[must_use]
pub fn classify_disconnected_subtree(
    removed_root: Entity,
    dom: &EcsDom,
) -> Vec<CustomElementReaction> {
    let mut enqueued = Vec::new();
    dom.for_each_shadow_inclusive_descendant(removed_root, &mut |entity| {
        if is_custom(entity, dom) {
            enqueued.push(CustomElementReaction::Disconnected(entity));
        }
    });
    enqueued
}

/// Classify an attribute change into an `AttributeChanged` reaction,
/// gated on `CEState::Custom` + the definition's `observed_attributes`
/// (HTML §4.13.6 "enqueue a custom element callback reaction" step 5.2 —
/// "If definition's observed attributes does not contain attributeName,
/// then return"). Returns `None` when the target is not a
/// Custom element or the attribute is not observed.
#[must_use]
pub fn classify_attribute_change(
    entity: Entity,
    name: &str,
    old_value: Option<&str>,
    new_value: Option<&str>,
    dom: &EcsDom,
    registry: &CustomElementRegistry,
) -> Option<CustomElementReaction> {
    // Only Custom-state elements receive `attributeChangedCallback`.
    let def_name = custom_definition_name(entity, dom)?;
    let def = registry.get(&def_name)?;
    // observed_attributes filter — O(1) via the definition's parallel
    // `observed_set` (mutation hot path runs this on every setAttribute).
    if !def.observes(name) {
        return None;
    }
    Some(CustomElementReaction::AttributeChanged {
        entity,
        name: name.to_string(),
        old_value: old_value.map(str::to_owned),
        new_value: new_value.map(str::to_owned),
    })
}

/// The pair of `entity`'s innermost adopt segment (C0c D-6's lookup, consumer
/// side); the rule is stated on [`MutationEvent::Adopt`].
fn adopt_pair(
    dom: &EcsDom,
    entity: Entity,
    root: Entity,
    documents: Option<(Entity, Entity)>,
    contents: &[AdoptedContents],
) -> Option<(Entity, Entity)> {
    let r = dom.find_tree_root_composed(entity);
    if r == root {
        return documents;
    }
    contents
        .iter()
        .find(|s| s.fragment == r)
        .and_then(|s| s.documents)
}

/// Returns `true` iff `entity` is a Custom element ready for lifecycle
/// callbacks (CEState::Custom). Pre-upgrade entities (Undefined / Failed)
/// do NOT receive callbacks per HTML §4.13.6.
fn is_custom(entity: Entity, dom: &EcsDom) -> bool {
    dom.world()
        .get::<&CustomElementState>(entity)
        .is_ok_and(|s| matches!(s.state, CEState::Custom))
}

/// Per-entity classification used by [`CustomElementReactionConsumer::
/// handle_insert`] to route between `Connected` and `Upgrade` per
/// WHATWG DOM §4.2.3 insertion-steps + HTML §4.13.6 reactions:
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
    defined: &std::collections::HashSet<String>,
    dom: &EcsDom,
) -> UpgradeTarget {
    let Ok(state) = dom.world().get::<&CustomElementState>(entity) else {
        return UpgradeTarget::None;
    };
    match state.state {
        CEState::Custom => UpgradeTarget::Connected,
        CEState::Undefined => {
            // Null-registry elements are outside every registry — the
            // insertion path must not upgrade them either (the
            // executor's `prepare_upgrade` would skip anyway; gating
            // here avoids enqueueing dead reactions).
            if matches!(state.registry, crate::RegistryAssociation::Document)
                && defined.contains(&state.definition_name)
            {
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
