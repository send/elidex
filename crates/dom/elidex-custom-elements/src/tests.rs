//! Tests for Custom Elements registry.

use super::*;
use elidex_ecs::EcsDom;

#[test]
fn define_and_get() {
    let mut registry = CustomElementRegistry::new();
    let def =
        CustomElementDefinition::new("my-element".to_string(), 1, vec!["title".to_string()], None);
    registry.define(def).unwrap();
    assert!(registry.is_defined("my-element"));
    assert!(registry.get("my-element").is_some());
    assert_eq!(registry.get("my-element").unwrap().constructor_id, 1);
}

#[test]
fn define_rejects_invalid_name() {
    let mut registry = CustomElementRegistry::new();
    let def = CustomElementDefinition::new("div".to_string(), 1, Vec::new(), None);
    assert!(registry.define(def).is_err());
}

#[test]
fn define_rejects_duplicate() {
    let mut registry = CustomElementRegistry::new();
    let def1 = CustomElementDefinition::new("my-el".to_string(), 1, Vec::new(), None);
    let def2 = CustomElementDefinition::new("my-el".to_string(), 2, Vec::new(), None);
    assert!(registry.define(def1).is_ok());
    assert!(matches!(
        registry.define(def2),
        Err(registry::DefineError::AlreadyDefined(_))
    ));
}

#[test]
fn collect_upgrade_candidates_is_document_scoped_and_tree_ordered() {
    // define()-time discovery = WHATWG HTML §4.13.4 "upgrade
    // particular elements within a document": shadow-including
    // descendants of the document, IN TREE ORDER, whose per-entity
    // `CustomElementState` is Undefined for `name` and whose local
    // name matches the definition. A *detached* element (not a
    // document descendant) is NOT a candidate — it upgrades on later
    // insertion, not here. Name/local-name mismatch, null-registry,
    // and already-upgraded elements are excluded.
    let mut registry = CustomElementRegistry::new();
    registry
        .define(CustomElementDefinition::new(
            "my-el".to_string(),
            1,
            Vec::new(),
            None,
        ))
        .unwrap();
    registry
        .define(CustomElementDefinition::new(
            "my-div".to_string(),
            2,
            Vec::new(),
            Some("div".to_string()),
        ))
        .unwrap();

    let mut dom = EcsDom::new();
    let document = dom.create_document_root();
    // Build a connected tree in a deliberate order so the returned
    // candidate order can be asserted against tree order (NOT entity
    // id / ECS iteration order). Tree:
    //   document
    //     wrapper
    //       e_first  (my-el, Undefined)        ← candidate #1
    //       other    (other-el, Undefined)     ← name mismatch
    //       e_second (my-el, Undefined)        ← candidate #2
    //       upgraded (my-el, Custom)           ← already upgraded
    //       null_reg (my-el, Undefined, Null)  ← null registry
    //       is_span  (span, Undefined "my-el") ← local-name mismatch
    //       builtin  (div,  Undefined "my-div")← customized built-in ✓ #3
    let wrapper =
        dom.create_element_ownerless_fixture("section", elidex_ecs::Attributes::default());
    let e_first = dom.create_element_ownerless_fixture("my-el", elidex_ecs::Attributes::default());
    let other = dom.create_element_ownerless_fixture("other-el", elidex_ecs::Attributes::default());
    let e_second = dom.create_element_ownerless_fixture("my-el", elidex_ecs::Attributes::default());
    let upgraded = dom.create_element_ownerless_fixture("my-el", elidex_ecs::Attributes::default());
    let null_reg = dom.create_element_ownerless_fixture("my-el", elidex_ecs::Attributes::default());
    let is_span = dom.create_element_ownerless_fixture("span", elidex_ecs::Attributes::default());
    let builtin = dom.create_element_ownerless_fixture("div", elidex_ecs::Attributes::default());
    // A detached my-el (never inserted) — spec defers it to insertion.
    let detached = dom.create_element_ownerless_fixture("my-el", elidex_ecs::Attributes::default());

    assert!(dom.append_child(document, wrapper));
    assert!(dom.append_child(wrapper, e_first));
    assert!(dom.append_child(wrapper, other));
    assert!(dom.append_child(wrapper, e_second));
    assert!(dom.append_child(wrapper, upgraded));
    assert!(dom.append_child(wrapper, null_reg));
    assert!(dom.append_child(wrapper, is_span));
    assert!(dom.append_child(wrapper, builtin));

    {
        let world = dom.world_mut();
        let mut insert =
            |entity, state: CustomElementState| world.insert_one(entity, state).unwrap();
        insert(e_first, CustomElementState::undefined("my-el"));
        insert(other, CustomElementState::undefined("other-el"));
        insert(e_second, CustomElementState::undefined("my-el"));
        insert(upgraded, CustomElementState::custom("my-el"));
        let mut null_state = CustomElementState::undefined("my-el");
        null_state.registry = RegistryAssociation::Null;
        insert(null_reg, null_state);
        insert(is_span, CustomElementState::undefined("my-el"));
        insert(builtin, CustomElementState::undefined("my-div"));
        insert(detached, CustomElementState::undefined("my-el"));
    }

    // Tree order preserved, detached excluded, mismatches excluded.
    assert_eq!(
        collect_upgrade_candidates(&dom, document, &registry, "my-el"),
        vec![e_first, e_second],
    );
    // Customized built-in: <div> matches extends base, <span is="my-div">
    // (had there been one) would not.
    assert_eq!(
        collect_upgrade_candidates(&dom, document, &registry, "my-div"),
        vec![builtin],
    );
    // A name without a definition (define() rejects invalid names, so an
    // invalid is-derived name can never acquire one) yields no candidates.
    assert!(collect_upgrade_candidates(&dom, document, &registry, "notvalid").is_empty());

    // The detached element is genuinely Undefined+my-el but is NOT a
    // document descendant, so define() does not upgrade it (it would
    // upgrade on insertion). Once appended it becomes a candidate.
    assert!(dom.append_child(wrapper, detached));
    assert_eq!(
        collect_upgrade_candidates(&dom, document, &registry, "my-el"),
        vec![e_first, e_second, detached],
    );
}

#[test]
fn lookup_by_is_attribute() {
    let mut registry = CustomElementRegistry::new();
    let def =
        CustomElementDefinition::new("my-div".to_string(), 1, Vec::new(), Some("div".to_string()));
    registry.define(def).unwrap();

    // Matching: is="my-div" on <div>.
    assert!(registry.lookup_by_is("my-div", "div").is_some());
    // Non-matching: is="my-div" on <span> (wrong base element).
    assert!(registry.lookup_by_is("my-div", "span").is_none());
    // Non-matching: unknown is value.
    assert!(registry.lookup_by_is("unknown-el", "div").is_none());
}

// ── lookup_by_constructor ([C1] §3.2.3 step 5 reverse lookup) ───────────

#[test]
fn lookup_by_constructor_finds_match() {
    let mut registry = CustomElementRegistry::new();
    let def_a = CustomElementDefinition::new("el-a".to_string(), 100, Vec::new(), None);
    let def_b = CustomElementDefinition::new("el-b".to_string(), 200, Vec::new(), None);
    registry.define(def_a).unwrap();
    registry.define(def_b).unwrap();

    assert_eq!(registry.lookup_by_constructor(100).unwrap().name, "el-a");
    assert_eq!(registry.lookup_by_constructor(200).unwrap().name, "el-b");
}

#[test]
fn lookup_by_constructor_returns_none_for_unknown() {
    let mut registry = CustomElementRegistry::new();
    let def = CustomElementDefinition::new("my-el".to_string(), 1, Vec::new(), None);
    registry.define(def).unwrap();
    assert!(registry.lookup_by_constructor(999).is_none());
}

// ── Construction stack ([C2] / [C1] §3.2.3 / [C4] §4.13.5) ──────────────

#[test]
fn construction_stack_push_peek_pop() {
    let mut registry = CustomElementRegistry::new();
    let mut dom = EcsDom::new();
    let def = CustomElementDefinition::new("my-el".to_string(), 1, Vec::new(), None);
    registry.define(def).unwrap();

    // Empty stack peek = None ([C1] step 9 sync-construct trigger).
    assert!(registry.peek_construction_stack("my-el").is_none());

    let e1 = dom.create_element_ownerless_fixture("my-el", elidex_ecs::Attributes::default());
    assert!(registry.push_construction_stack("my-el", e1));
    let top = registry.peek_construction_stack("my-el").unwrap();
    assert_eq!(top, &ConstructionStackEntry::Element(e1));

    // Pop after constructor cleanup ([C4] step 9).
    let popped = registry.pop_construction_stack("my-el").unwrap();
    assert_eq!(popped, ConstructionStackEntry::Element(e1));
    assert!(registry.peek_construction_stack("my-el").is_none());
}

#[test]
fn construction_stack_replace_top_with_marker() {
    let mut registry = CustomElementRegistry::new();
    let mut dom = EcsDom::new();
    let def = CustomElementDefinition::new("my-el".to_string(), 1, Vec::new(), None);
    registry.define(def).unwrap();
    let e1 = dom.create_element_ownerless_fixture("my-el", elidex_ecs::Attributes::default());
    registry.push_construction_stack("my-el", e1);

    // [C1] step 15: replace returns the element that was at the top.
    let replaced = registry.replace_construction_stack_top_with_marker("my-el");
    assert_eq!(replaced, Some(e1));

    // Top is now AlreadyConstructed; a second replace returns None
    // (cannot extract an Entity from a marker — [C1] step 13 throws).
    let top = registry.peek_construction_stack("my-el").unwrap();
    assert_eq!(top, &ConstructionStackEntry::AlreadyConstructed);
    assert_eq!(
        registry.replace_construction_stack_top_with_marker("my-el"),
        None,
    );
}

#[test]
fn construction_stack_isolated_per_definition() {
    // Re-entrant define / upgrade across two definitions must not
    // share a construction stack ([C2] "per-definition list").
    let mut registry = CustomElementRegistry::new();
    let mut dom = EcsDom::new();
    let def_a = CustomElementDefinition::new("el-a".to_string(), 1, Vec::new(), None);
    let def_b = CustomElementDefinition::new("el-b".to_string(), 2, Vec::new(), None);
    registry.define(def_a).unwrap();
    registry.define(def_b).unwrap();
    let e_a = dom.create_element_ownerless_fixture("el-a", elidex_ecs::Attributes::default());
    let e_b = dom.create_element_ownerless_fixture("el-b", elidex_ecs::Attributes::default());

    registry.push_construction_stack("el-a", e_a);
    registry.push_construction_stack("el-b", e_b);

    assert_eq!(
        registry.peek_construction_stack("el-a"),
        Some(&ConstructionStackEntry::Element(e_a)),
    );
    assert_eq!(
        registry.peek_construction_stack("el-b"),
        Some(&ConstructionStackEntry::Element(e_b)),
    );
}

#[test]
fn construction_stack_push_to_unknown_name_returns_false() {
    let mut registry = CustomElementRegistry::new();
    let mut dom = EcsDom::new();
    let e =
        dom.create_element_ownerless_fixture("never-defined", elidex_ecs::Attributes::default());
    assert!(!registry.push_construction_stack("never-defined", e));
}

// ── spawn_custom_element_entity ([C1] §3.2.3 step 9 sync construct) ─────

#[test]
fn spawn_custom_element_entity_attaches_all_components() {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let e = spawn_custom_element_entity(&mut dom, "my-el", "my-el", Some(doc))
        .expect("spawned for a document");

    // Element shape: TagType + Attributes present, NodeKind reads
    // as Element through the `EcsDom::node_kind` accessor (the
    // `TreeRelation` component is internal to elidex-ecs and not
    // re-exported).
    assert_eq!(dom.node_kind(e), Some(elidex_ecs::NodeKind::Element));
    let world = dom.world();
    assert_eq!(world.get::<&elidex_ecs::TagType>(e).unwrap().0, "my-el");
    assert!(world.get::<&elidex_ecs::Attributes>(e).is_ok());

    // CE shape.
    let ce_state = world.get::<&CustomElementState>(e).unwrap();
    assert_eq!(ce_state.state, CEState::Custom);
    assert_eq!(ce_state.definition_name, "my-el");
}

#[test]
fn spawn_custom_element_entity_disconnected_by_default() {
    // [C1] step 9: sync-constructed element is not connected to any
    // tree until script explicitly inserts it.
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let e = spawn_custom_element_entity(&mut dom, "my-el", "my-el", Some(doc))
        .expect("spawned for a document");
    assert!(!dom.is_connected(e));
}

// ── S5-6b §4.3.1: single-homed record→CE classification (pin H1) ──────
//
// The `classify_*` functions are the ONE classification shared by the
// mutation-event leg (`CustomElementReactionConsumer`) and the record
// leg (`Vm::deliver_mutation_records`). These tests pin that (a) each
// classification yields exactly ONE reaction per mutation (no
// duplication → no double-enqueue), and (b) the two legs produce the
// SAME reaction (single-homing, no parallel re-implementation).

/// A connected Custom `x-el` element + a registry defining it with
/// `data-x` observed.
fn setup_custom_el() -> (EcsDom, elidex_ecs::Entity, CustomElementRegistry) {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let el = dom.create_element_ownerless_fixture("x-el", elidex_ecs::Attributes::default());
    assert!(dom.append_child(doc, el));
    dom.world_mut()
        .insert_one(el, CustomElementState::custom("x-el"))
        .unwrap();
    assert!(dom.is_connected(el));
    let mut registry = CustomElementRegistry::new();
    registry
        .define(CustomElementDefinition::new(
            "x-el".to_string(),
            1,
            vec!["data-x".to_string()],
            None,
        ))
        .unwrap();
    (dom, el, registry)
}

#[test]
fn classify_attribute_change_one_reaction_observed_none_unobserved() {
    let (dom, el, registry) = setup_custom_el();
    // Observed attribute on a Custom element → exactly one AttributeChanged.
    match classify_attribute_change(el, "data-x", Some("old"), Some("new"), &dom, &registry) {
        Some(CustomElementReaction::AttributeChanged {
            entity,
            name,
            old_value,
            new_value,
        }) => {
            assert_eq!(entity, el);
            assert_eq!(name, "data-x");
            assert_eq!(old_value.as_deref(), Some("old"));
            assert_eq!(new_value.as_deref(), Some("new"));
        }
        other => panic!("expected one AttributeChanged, got {other:?}"),
    }
    // Unobserved attribute → no reaction (observedAttributes gate).
    assert!(classify_attribute_change(el, "data-y", None, Some("v"), &dom, &registry).is_none());
}

#[test]
fn classify_connected_and_disconnected_subtree_one_reaction_each() {
    let (dom, el, registry) = setup_custom_el();
    let connected = classify_connected_subtree(el, &dom, &registry);
    assert_eq!(connected.len(), 1, "one Connected per custom element");
    assert!(matches!(connected[0], CustomElementReaction::Connected(e) if e == el));

    let disconnected = classify_disconnected_subtree(el, &dom);
    assert_eq!(disconnected.len(), 1, "one Disconnected per custom element");
    assert!(matches!(disconnected[0], CustomElementReaction::Disconnected(e) if e == el));
}

#[test]
fn event_leg_and_record_leg_share_one_classification() {
    // Single-homing pin (H1): the mutation-event consumer and the
    // record-leg `classify_attribute_change` produce the SAME reaction
    // for the same attribute change — one classification, no divergence.
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    let (mut dom, el, registry) = setup_custom_el();
    let registry = Arc::new(Mutex::new(registry));
    let queue: Arc<Mutex<VecDeque<CustomElementReaction>>> = Arc::new(Mutex::new(VecDeque::new()));
    let mut consumer =
        CustomElementReactionConsumer::new(Arc::clone(&registry), Arc::clone(&queue));

    // Mutation-event leg: drive the consumer.
    consumer.handle(
        &elidex_ecs::MutationEvent::AttributeChange {
            node: el,
            name: "data-x",
            old_value: Some("old"),
            new_value: Some("new"),
        },
        &mut dom,
    );
    let event_leg: Vec<_> = queue.lock().unwrap().drain(..).collect();

    // Record leg: the shared classification function directly.
    let record_leg = {
        let reg = registry.lock().unwrap();
        classify_attribute_change(el, "data-x", Some("old"), Some("new"), &dom, &reg)
    };

    assert_eq!(
        event_leg.len(),
        1,
        "event leg enqueues exactly one reaction"
    );
    let record_leg = record_leg.expect("record leg produces exactly one reaction");
    match (&event_leg[0], &record_leg) {
        (
            CustomElementReaction::AttributeChanged {
                entity: e1,
                name: n1,
                old_value: o1,
                new_value: v1,
            },
            CustomElementReaction::AttributeChanged {
                entity: e2,
                name: n2,
                old_value: o2,
                new_value: v2,
            },
        ) => {
            assert_eq!(e1, e2);
            assert_eq!(n1, n2);
            assert_eq!(o1, o2);
            assert_eq!(v1, v2);
        }
        other => panic!("legs diverged: {other:?}"),
    }
}

/// Engine-level pin for the `CloneCreated` arm: exactly ONE `Upgrade` per
/// created candidate (a duplicated enqueue is hidden at the JS level, where
/// the second `Upgrade` is a `prepare_upgrade` Skip and the constructor still
/// runs once), in creation order, and `scope` decides whether the root's light
/// descendants are candidates at all.
#[test]
fn clone_created_arm_enqueues_one_upgrade_per_candidate_and_honours_scope() {
    use elidex_ecs::{CloneScope, MutationEvent, Namespace};
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    let mut dom = EcsDom::new();
    let root = dom.create_element_ownerless_fixture("x-el", elidex_ecs::Attributes::default());
    let child = dom.create_element_ownerless_fixture("x-el", elidex_ecs::Attributes::default());
    assert!(dom.append_child(root, child));
    for e in [root, child] {
        dom.world_mut()
            .insert_one(
                e,
                CustomElementState::for_created_element("x-el", None, Namespace::Html).unwrap(),
            )
            .unwrap();
    }
    let mut registry = CustomElementRegistry::new();
    registry
        .define(CustomElementDefinition::new(
            "x-el".to_string(),
            1,
            Vec::new(),
            None,
        ))
        .unwrap();
    let registry = Arc::new(Mutex::new(registry));
    let queue: Arc<Mutex<VecDeque<CustomElementReaction>>> = Arc::new(Mutex::new(VecDeque::new()));
    let mut consumer =
        CustomElementReactionConsumer::new(Arc::clone(&registry), Arc::clone(&queue));

    let upgrades =
        |queue: &Arc<Mutex<VecDeque<CustomElementReaction>>>| -> Vec<elidex_ecs::Entity> {
            queue
                .lock()
                .unwrap()
                .drain(..)
                .map(|r| match r {
                    CustomElementReaction::Upgrade(e) => e,
                    other => panic!("expected only Upgrade reactions, got {other:?}"),
                })
                .collect()
        };

    consumer.handle(
        &MutationEvent::CloneCreated {
            root,
            scope: CloneScope::Subtree,
        },
        &mut dom,
    );
    assert_eq!(
        upgrades(&queue),
        vec![root, child],
        "Subtree: root, then child, once each"
    );

    consumer.handle(
        &MutationEvent::CloneCreated {
            root,
            scope: CloneScope::SelfOnly,
        },
        &mut dom,
    );
    assert_eq!(
        upgrades(&queue),
        vec![root],
        "SelfOnly: the light child is not visited"
    );
}

// ── C0b U20 (red-first, §6.5): the CE helper makes the owner decision ────
// `Some(doc)` → an element in `doc`; `None` (an unbound VM) → no element.
// Base formulation: HEAD's `(…, owner: Option<Entity>) -> Entity`.

#[test]
fn u20_spawn_with_no_document_spawns_nothing() {
    let mut dom = EcsDom::new();
    let before = dom.world().len();
    let _ = spawn_custom_element_entity(&mut dom, "x-e", "x-e", None);
    assert_eq!(dom.world().len(), before, "no entity spawned for None");
}

#[test]
fn u20_control_spawn_with_document_is_in_that_document() {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let e = spawn_custom_element_entity(&mut dom, "x-e", "x-e", Some(doc))
        .expect("spawned for a document");
    assert_eq!(dom.node_document(e), Some(doc));
}

// ── C0c U13 (red-first, §6.5): the `Adopt` arm's order and per-node pairs ──
// A `<template>` in `X` with a custom light child `L` and a custom element
// `C` in its contents (stamped `Y`), adopted into `D`: DOM §4.5 *adopt* step
// 3.4 adopts the contents right after the template (HTML §4.12.3 adopting
// steps 2), so `C`'s `adoptedCallback` is enqueued before `L`'s, and `C`
// carries its segment's pair `(Y, D)`, not the root's `(X, D)`.

/// The consumer installed as the DOM's dispatcher (the binding crate's
/// `ConsumerDispatcher` stand-in).
struct U13Dispatcher(CustomElementReactionConsumer);

impl elidex_ecs::MutationDispatcher for U13Dispatcher {
    fn dispatch(&mut self, event: &elidex_ecs::MutationEvent<'_>, dom: &mut EcsDom) {
        self.0.handle(event, dom);
    }
}

struct U13 {
    dom: EcsDom,
    queue: std::sync::Arc<std::sync::Mutex<std::collections::VecDeque<CustomElementReaction>>>,
    tpl: elidex_ecs::Entity,
    c: elidex_ecs::Entity,
    l: elidex_ecs::Entity,
    x: elidex_ecs::Entity,
    y: elidex_ecs::Entity,
    d: elidex_ecs::Entity,
}

fn u13_fixture() -> U13 {
    use elidex_ecs::Attributes;
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    let mut dom = EcsDom::new();
    let doc_d = dom.create_document_root();
    let doc_x = dom.create_document_node();
    let doc_y = dom.create_document_node();
    let tpl = dom.create_element_with_owner(
        "template",
        Attributes::default(),
        dom.node_document_for_new_nodes(doc_x),
    );
    let l = dom.create_element_with_owner(
        "x-el",
        Attributes::default(),
        dom.node_document_for_new_nodes(doc_x),
    );
    assert!(dom.append_child(tpl, l));
    let contents = dom.attach_template_contents(tpl, dom.node_document_for_new_nodes(doc_y));
    let c = dom.create_element_with_owner(
        "x-el",
        Attributes::default(),
        dom.node_document_for_new_nodes(doc_y),
    );
    assert!(dom.append_child(contents, c));
    for e in [c, l] {
        dom.world_mut()
            .insert_one(e, CustomElementState::custom("x-el"))
            .unwrap();
    }
    let mut registry = CustomElementRegistry::new();
    registry
        .define(CustomElementDefinition::new(
            "x-el".to_string(),
            1,
            Vec::new(),
            None,
        ))
        .unwrap();
    let queue: Arc<Mutex<VecDeque<CustomElementReaction>>> = Arc::new(Mutex::new(VecDeque::new()));
    let consumer =
        CustomElementReactionConsumer::new(Arc::new(Mutex::new(registry)), Arc::clone(&queue));
    let _ = dom.set_mutation_dispatcher(Box::new(U13Dispatcher(consumer)));
    U13 {
        dom,
        queue,
        tpl,
        c,
        l,
        x: doc_x,
        y: doc_y,
        d: doc_d,
    }
}

#[test]
fn u13_adopt_enqueues_contents_before_light_child_each_with_its_pair() {
    let mut f = u13_fixture();
    f.dom.adopt(f.tpl, f.d);
    let adopted: Vec<_> = f
        .queue
        .lock()
        .unwrap()
        .drain(..)
        .filter_map(|r| match r {
            CustomElementReaction::Adopted {
                entity,
                old_document,
                new_document,
            } => Some((entity, old_document, new_document)),
            _ => None,
        })
        .collect();
    assert_eq!(
        adopted,
        vec![(f.c, f.y, f.d), (f.l, f.x, f.d)],
        "C (its segment's pair) before L (the root's pair)"
    );
}

/// U13's control (green on the base): `C` and `L` read `CEState::Custom`.
#[test]
fn u13_control_c_and_l_are_custom() {
    let f = u13_fixture();
    for e in [f.c, f.l] {
        let state = f.dom.world().get::<&CustomElementState>(e).map(|s| s.state);
        assert!(matches!(state, Ok(CEState::Custom)), "{e:?} is custom");
    }
}

// ── C0c U19, consumer side: the reactions of the three nested-contents legs ──
// (the `elidex-ecs` U19 cells pin the segments and stamps). Outer template
// `t0` in `D0`; its contents `f1` hold `t1`, whose contents `f2` hold the
// custom `e`.

/// `f1_owner` / `f2_owner`: 0 = the target, 1 = `X`, 2 = `Y`, `None` =
/// ownerless. Returns the queued `Adopted` reactions after `adopt(t0, target)`
/// and the document table.
fn u19_adopted(
    f1_owner: Option<usize>,
    f2_owner: usize,
) -> (
    Vec<(elidex_ecs::Entity, elidex_ecs::Entity, elidex_ecs::Entity)>,
    elidex_ecs::Entity,
    [elidex_ecs::Entity; 3],
) {
    use elidex_ecs::Attributes;

    let mut f = u13_fixture();
    let target = f.d;
    let docs = [target, f.x, f.y];
    let d0 = f.dom.create_document_node();
    let dom = &mut f.dom;
    let new_owner = |dom: &mut EcsDom, i: Option<usize>| {
        if let Some(i) = i {
            return dom.node_document_for_new_nodes(docs[i]);
        }
        let ownerless = dom.create_element_ownerless_fixture("span", Attributes::default());
        dom.node_document_for_new_nodes(ownerless)
    };
    let t0 = dom.create_element_with_owner(
        "template",
        Attributes::default(),
        dom.node_document_for_new_nodes(d0),
    );
    let o = new_owner(dom, f1_owner);
    let f1 = dom.attach_template_contents(t0, o);
    let o = new_owner(dom, f1_owner);
    let t1 = dom.create_element_with_owner("template", Attributes::default(), o);
    assert!(dom.append_child(f1, t1));
    let o = new_owner(dom, Some(f2_owner));
    let f2 = dom.attach_template_contents(t1, o);
    let o = new_owner(dom, Some(f2_owner));
    let e = dom.create_element_with_owner("x-el", Attributes::default(), o);
    assert!(dom.append_child(f2, e));
    dom.world_mut()
        .insert_one(e, CustomElementState::custom("x-el"))
        .unwrap();
    f.queue.lock().unwrap().clear();
    f.dom.adopt(t0, target);
    let adopted = f
        .queue
        .lock()
        .unwrap()
        .drain(..)
        .filter_map(|r| match r {
            CustomElementReaction::Adopted {
                entity,
                old_document,
                new_document,
            } => Some((entity, old_document, new_document)),
            _ => None,
        })
        .collect();
    (adopted, e, docs)
}

/// First leg (`f1` in `Y`, `f2` in `X`): `e` gets its innermost segment's
/// pair `(X, target)`.
#[test]
fn u19_consumer_innermost_segment_pair() {
    let (adopted, e, [target, x, _]) = u19_adopted(Some(2), 1);
    assert_eq!(adopted, vec![(e, x, target)]);
}

/// Second leg (`f1` already in the target): both contents are skipped, no
/// segment, so `e` gets no reaction (not the root's `(D0, target)`).
#[test]
fn u19_consumer_skipped_contents_no_reaction() {
    let (adopted, _, _) = u19_adopted(Some(0), 1);
    assert_eq!(adopted, Vec::new());
}

/// Third leg (`f1` ownerless, `f2` in `X`): both segments are `None`, so `e`
/// gets no reaction (its own pair would give `(X, target)`).
#[test]
fn u19_consumer_segment_under_none_no_reaction() {
    let (adopted, _, _) = u19_adopted(None, 1);
    assert_eq!(adopted, Vec::new());
}
