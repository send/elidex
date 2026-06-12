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
    let wrapper = dom.create_element("section", elidex_ecs::Attributes::default());
    let e_first = dom.create_element("my-el", elidex_ecs::Attributes::default());
    let other = dom.create_element("other-el", elidex_ecs::Attributes::default());
    let e_second = dom.create_element("my-el", elidex_ecs::Attributes::default());
    let upgraded = dom.create_element("my-el", elidex_ecs::Attributes::default());
    let null_reg = dom.create_element("my-el", elidex_ecs::Attributes::default());
    let is_span = dom.create_element("span", elidex_ecs::Attributes::default());
    let builtin = dom.create_element("div", elidex_ecs::Attributes::default());
    // A detached my-el (never inserted) — spec defers it to insertion.
    let detached = dom.create_element("my-el", elidex_ecs::Attributes::default());

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

    let e1 = dom.create_element("my-el", elidex_ecs::Attributes::default());
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
    let e1 = dom.create_element("my-el", elidex_ecs::Attributes::default());
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
    let e_a = dom.create_element("el-a", elidex_ecs::Attributes::default());
    let e_b = dom.create_element("el-b", elidex_ecs::Attributes::default());

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
    let e = dom.create_element("never-defined", elidex_ecs::Attributes::default());
    assert!(!registry.push_construction_stack("never-defined", e));
}

// ── spawn_custom_element_entity ([C1] §3.2.3 step 9 sync construct) ─────

#[test]
fn spawn_custom_element_entity_attaches_all_components() {
    let mut dom = EcsDom::new();
    let e = spawn_custom_element_entity(&mut dom, "my-el", "my-el", None);

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
    let e = spawn_custom_element_entity(&mut dom, "my-el", "my-el", None);
    assert!(!dom.is_connected(e));
}
