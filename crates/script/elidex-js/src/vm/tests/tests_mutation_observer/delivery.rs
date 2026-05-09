//! Phase C4 — actual `MutationRecord` delivery via
//! `Vm::deliver_mutation_records`, plus later-added scenarios:
//! `Plan §G #17` (characterData), `§G #26` (records survive GC),
//! `§H R4 #28` (re-entrancy), multi-observer / repeat-observe
//! semantics, callback-exception isolation, and `takeRecords`.
//!
//! Companion to [`super::setup`] (constructor + observe init parsing)
//! and [`super::lifecycle`] (post-unbind tolerance + rebind).

use elidex_ecs::EcsDom;
use elidex_script_session::{MutationKind, MutationRecord as SessionRecord, SessionCore};

use super::super::super::value::JsValue;
use super::super::super::Vm;
use super::{run, setup_with_root};

#[test]
fn mutation_observer_delivers_child_list_record() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         var mo = new MutationObserver(function(rec, obs){ \
           globalThis.records = rec; \
           globalThis.observerArg = obs; \
           globalThis.mo = mo; \
         }); \
         mo.observe(root, {childList:true});",
    )
    .unwrap();

    // Simulate a child append.
    let added = dom.create_element("span", elidex_ecs::Attributes::default());
    let record = SessionRecord {
        kind: MutationKind::ChildList,
        target: root,
        added_nodes: vec![added],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: None,
        old_value: None,
    };
    vm.deliver_mutation_records(std::slice::from_ref(&record));

    // Inspect the resulting JS records array.
    let length = vm.eval("records.length").unwrap();
    assert_eq!(length, JsValue::Number(1.0));
    let type_v = vm.eval("records[0].type").unwrap();
    let JsValue::String(sid) = type_v else {
        panic!("expected string, got {type_v:?}")
    };
    assert_eq!(vm.inner.strings.get_utf8(sid), "childList");

    let added_count = vm.eval("records[0].addedNodes.length").unwrap();
    assert_eq!(added_count, JsValue::Number(1.0));
    let target_eq = vm.eval("records[0].target === root").unwrap();
    assert_eq!(target_eq, JsValue::Boolean(true));
    let observer_eq = vm.eval("observerArg === mo").unwrap();
    assert_eq!(observer_eq, JsValue::Boolean(true));
    vm.unbind();
}

#[test]
fn mutation_observer_delivers_attribute_record_with_old_value() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         var mo = new MutationObserver(function(rec){ globalThis.records = rec; }); \
         mo.observe(root, {attributes:true, attributeOldValue:true});",
    )
    .unwrap();

    let record = SessionRecord {
        kind: MutationKind::Attribute,
        target: root,
        added_nodes: vec![],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: Some("class".to_string()),
        old_value: Some("old-class".to_string()),
    };
    vm.deliver_mutation_records(std::slice::from_ref(&record));

    let length = vm.eval("records.length").unwrap();
    assert_eq!(length, JsValue::Number(1.0));
    let attr_name = vm.eval("records[0].attributeName").unwrap();
    let JsValue::String(sid) = attr_name else {
        panic!("expected attribute_name string, got {attr_name:?}")
    };
    assert_eq!(vm.inner.strings.get_utf8(sid), "class");

    let old_value = vm.eval("records[0].oldValue").unwrap();
    let JsValue::String(sid) = old_value else {
        panic!("expected old_value string, got {old_value:?}")
    };
    assert_eq!(vm.inner.strings.get_utf8(sid), "old-class");
    vm.unbind();
}

#[test]
fn mutation_observer_record_properties_are_readonly() {
    // WHATWG DOM §4.3.5: every `MutationRecord` member is a
    // `readonly attribute` — non-strict assignment should silently
    // fail, strict-mode assignment should TypeError.  Regression for
    // the prior `PropertyAttrs::DATA` (writable) installation.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         var mo = new MutationObserver(function(rec){ globalThis.records = rec; }); \
         mo.observe(root, {attributes:true});",
    )
    .unwrap();

    let record = SessionRecord {
        kind: MutationKind::Attribute,
        target: root,
        added_nodes: vec![],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: Some("class".to_string()),
        old_value: None,
    };
    vm.deliver_mutation_records(std::slice::from_ref(&record));

    // Engine semantics: writes to a non-writable property throw
    // TypeError ("Cannot assign to read only property") regardless
    // of strict mode.  The test asserts the WEBIDL_RO descriptor is
    // in effect on every MutationRecord member; the prior
    // `PropertyAttrs::DATA` would silently allow the write to land
    // and `record.type === 'mutated'` would read back the new value.
    for member in [
        "type",
        "target",
        "addedNodes",
        "removedNodes",
        "previousSibling",
        "nextSibling",
        "attributeName",
        "oldValue",
    ] {
        let err = vm.eval(&format!("records[0].{member} = 'x';")).unwrap_err();
        let err_text = format!("{err:?}");
        assert!(
            err_text.contains("read only") || err_text.contains("read-only"),
            "expected read-only TypeError for `records[0].{member}`, got: {err_text}"
        );
    }
    vm.unbind();
}

#[test]
fn mutation_observer_attribute_filter_excludes_unmatched() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         var mo = new MutationObserver(function(rec){ globalThis.records = rec; }); \
         mo.observe(root, {attributeFilter:['class']});",
    )
    .unwrap();

    let id_record = SessionRecord {
        kind: MutationKind::Attribute,
        target: root,
        added_nodes: vec![],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: Some("id".to_string()),
        old_value: None,
    };
    vm.deliver_mutation_records(std::slice::from_ref(&id_record));

    // No records should have been queued because `id` doesn't match the filter.
    let records_v = vm.eval("records").unwrap();
    assert!(matches!(records_v, JsValue::Null));
    vm.unbind();
}

#[test]
fn mutation_observer_subtree_observes_descendant() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);
    let child = dom.create_element("span", elidex_ecs::Attributes::default());
    assert!(dom.append_child(root, child));

    vm.eval(
        "globalThis.records = null; \
         var mo = new MutationObserver(function(rec){ globalThis.records = rec; }); \
         mo.observe(root, {attributes:true, subtree:true});",
    )
    .unwrap();

    let record = SessionRecord {
        kind: MutationKind::Attribute,
        target: child,
        added_nodes: vec![],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: Some("class".to_string()),
        old_value: None,
    };
    vm.deliver_mutation_records(std::slice::from_ref(&record));

    let length = vm.eval("records ? records.length : 0").unwrap();
    assert_eq!(length, JsValue::Number(1.0));
    vm.unbind();
}

#[test]
fn mutation_observer_subtree_off_skips_descendant() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);
    let child = dom.create_element("span", elidex_ecs::Attributes::default());
    assert!(dom.append_child(root, child));

    vm.eval(
        "globalThis.records = null; \
         var mo = new MutationObserver(function(rec){ globalThis.records = rec; }); \
         mo.observe(root, {attributes:true});",
    )
    .unwrap();

    let record = SessionRecord {
        kind: MutationKind::Attribute,
        target: child,
        added_nodes: vec![],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: Some("class".to_string()),
        old_value: None,
    };
    vm.deliver_mutation_records(std::slice::from_ref(&record));

    let records_v = vm.eval("records").unwrap();
    assert!(matches!(records_v, JsValue::Null));
    vm.unbind();
}

#[test]
fn mutation_observer_take_records_drains_pending() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.mo = new MutationObserver(function(){ /* never called via takeRecords */ }); \
         mo.observe(root, {attributes:true});",
    )
    .unwrap();

    let record = SessionRecord {
        kind: MutationKind::Attribute,
        target: root,
        added_nodes: vec![],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: Some("class".to_string()),
        old_value: None,
    };
    // Notify but do NOT drive the callback; instead use takeRecords()
    // from JS.  Mimic the no-callback branch by feeding the record
    // straight into the registry (skip deliver, which would call the
    // callback).
    vm.inner
        .host_data
        .as_deref_mut()
        .unwrap()
        .mutation_observers
        .notify(&record, &|_, _| false);

    let drained = vm.eval("mo.takeRecords().length").unwrap();
    assert_eq!(drained, JsValue::Number(1.0));
    let drained_again = vm.eval("mo.takeRecords().length").unwrap();
    assert_eq!(drained_again, JsValue::Number(0.0));
    vm.unbind();
}

#[test]
fn mutation_observer_disconnect_clears_pending() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.calls = 0; \
         globalThis.mo = new MutationObserver(function(){ calls++; }); \
         mo.observe(root, {childList:true});",
    )
    .unwrap();

    let added = dom.create_element("span", elidex_ecs::Attributes::default());
    let record = SessionRecord {
        kind: MutationKind::ChildList,
        target: root,
        added_nodes: vec![added],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: None,
        old_value: None,
    };
    // Notify but don't deliver yet; then disconnect.
    vm.inner
        .host_data
        .as_deref_mut()
        .unwrap()
        .mutation_observers
        .notify(&record, &|_, _| false);
    vm.eval("mo.disconnect();").unwrap();
    vm.deliver_mutation_records(&[]);
    let calls = vm.eval("calls").unwrap();
    assert_eq!(calls, JsValue::Number(0.0));
    vm.unbind();
}

#[test]
fn mutation_observer_added_nodes_are_element_wrappers() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         var mo = new MutationObserver(function(rec){ globalThis.records = rec; }); \
         mo.observe(root, {childList:true});",
    )
    .unwrap();

    let added = dom.create_element("span", elidex_ecs::Attributes::default());
    let record = SessionRecord {
        kind: MutationKind::ChildList,
        target: root,
        added_nodes: vec![added],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: None,
        old_value: None,
    };
    vm.deliver_mutation_records(std::slice::from_ref(&record));

    let added_node_kind = vm.eval("typeof records[0].addedNodes[0]").unwrap();
    let JsValue::String(sid) = added_node_kind else {
        panic!("expected string, got {added_node_kind:?}")
    };
    assert_eq!(vm.inner.strings.get_utf8(sid), "object");

    // tagName accessor confirms it's an Element wrapper, not a raw number/null.
    let tag = vm.eval("records[0].addedNodes[0].tagName").unwrap();
    let JsValue::String(sid) = tag else {
        panic!("expected tagName string, got {tag:?}")
    };
    assert_eq!(vm.inner.strings.get_utf8(sid), "SPAN");
    vm.unbind();
}

// --- Plan §G #17 — characterData record ----------------------------

#[test]
fn mutation_observer_delivers_character_data_record() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);
    let text_node = dom.create_text("hello");
    assert!(dom.append_child(root, text_node));

    vm.eval(
        "globalThis.records = null; \
         var mo = new MutationObserver(function(rec){ globalThis.records = rec; }); \
         mo.observe(root, {characterData:true, characterDataOldValue:true, subtree:true});",
    )
    .unwrap();

    let record = SessionRecord {
        kind: MutationKind::CharacterData,
        target: text_node,
        added_nodes: vec![],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: None,
        old_value: Some("hello".to_string()),
    };
    vm.deliver_mutation_records(std::slice::from_ref(&record));

    let length = vm.eval("records.length").unwrap();
    assert_eq!(length, JsValue::Number(1.0));
    let type_v = vm.eval("records[0].type").unwrap();
    let JsValue::String(sid) = type_v else {
        panic!("expected string, got {type_v:?}")
    };
    assert_eq!(vm.inner.strings.get_utf8(sid), "characterData");
    let old_v = vm.eval("records[0].oldValue").unwrap();
    let JsValue::String(sid) = old_v else {
        panic!("expected oldValue string, got {old_v:?}")
    };
    assert_eq!(vm.inner.strings.get_utf8(sid), "hello");
    vm.unbind();
}

// --- Plan §G #26 — records array survives GC during callback --------

#[test]
fn mutation_observer_records_array_survives_gc() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);

    // Callback forces an explicit GC mid-delivery via a stress allocation
    // sequence + a direct `collect_garbage` shot from Rust below.  The
    // `push_temp_root` rooting in `build_mutation_records_array` /
    // `Vm::deliver_mutation_records` must keep the records array and its
    // embedded element wrappers alive across the cycle.
    vm.eval(
        "globalThis.recordsLen = -1; \
         globalThis.firstAddedTag = ''; \
         var mo = new MutationObserver(function(rec){ \
           /* stress allocation so any un-rooted intermediate would be \
              collectable before we read the record fields */ \
           var noise = []; \
           for (var i = 0; i < 64; i++) noise.push({i: i, s: 'x' + i}); \
           globalThis.recordsLen = rec.length; \
           globalThis.firstAddedTag = rec[0].addedNodes[0].tagName; \
         }); \
         mo.observe(root, {childList:true});",
    )
    .unwrap();

    let added = dom.create_element("section", elidex_ecs::Attributes::default());
    let record = SessionRecord {
        kind: MutationKind::ChildList,
        target: root,
        added_nodes: vec![added],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: None,
        old_value: None,
    };
    // Force GC immediately before delivery so the rooting path is the
    // only thing holding the just-allocated records array.
    vm.inner.collect_garbage();
    vm.deliver_mutation_records(std::slice::from_ref(&record));
    // And again after, to catch any post-callback dangling references.
    vm.inner.collect_garbage();

    let len = vm.eval("recordsLen").unwrap();
    assert_eq!(len, JsValue::Number(1.0));
    let tag = vm.eval("firstAddedTag").unwrap();
    let JsValue::String(sid) = tag else {
        panic!("expected tag string, got {tag:?}")
    };
    assert_eq!(vm.inner.strings.get_utf8(sid), "SECTION");
    vm.unbind();
}

// --- Plan §H R4 #28 — re-entrancy: callback mutates registry --------

#[test]
fn mutation_observer_callback_can_re_observe_other_target() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);
    let other = dom.create_element("aside", elidex_ecs::Attributes::default());
    assert!(dom.append_child(root, other));
    let other_wrapper = vm.inner.create_element_wrapper(other);
    vm.set_global("other", JsValue::Object(other_wrapper));

    // First delivery: callback adds `other` as a second observe target.
    // Verifies the registry borrow is released between iterations so a
    // re-entrant `observe` from inside the callback does not deadlock or
    // panic (Plan §L Finding 2).
    vm.eval(
        "globalThis.firstCount = 0; \
         globalThis.secondCount = 0; \
         globalThis.mo = new MutationObserver(function(rec){ \
           if (firstCount === 0) { \
             firstCount = rec.length; \
             mo.observe(other, {attributes:true}); \
           } else { \
             secondCount = rec.length; \
           } \
         }); \
         mo.observe(root, {childList:true});",
    )
    .unwrap();

    let added = dom.create_element("p", elidex_ecs::Attributes::default());
    let r1 = SessionRecord {
        kind: MutationKind::ChildList,
        target: root,
        added_nodes: vec![added],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: None,
        old_value: None,
    };
    vm.deliver_mutation_records(std::slice::from_ref(&r1));

    // Second delivery — `other` is now observed because the first
    // callback called `mo.observe(other, ...)`.
    let r2 = SessionRecord {
        kind: MutationKind::Attribute,
        target: other,
        added_nodes: vec![],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: Some("class".to_string()),
        old_value: None,
    };
    vm.deliver_mutation_records(std::slice::from_ref(&r2));

    let first = vm.eval("firstCount").unwrap();
    assert_eq!(first, JsValue::Number(1.0));
    let second = vm.eval("secondCount").unwrap();
    assert_eq!(
        second,
        JsValue::Number(1.0),
        "second delivery must include the re-attached target"
    );
    vm.unbind();
}

// --- Multi-observer / repeat-observe semantics -----------------------

#[test]
fn mutation_observer_two_observers_both_receive_record() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.aLen = 0; globalThis.bLen = 0; \
         var moA = new MutationObserver(function(rec){ aLen = rec.length; }); \
         var moB = new MutationObserver(function(rec){ bLen = rec.length; }); \
         moA.observe(root, {childList:true}); \
         moB.observe(root, {childList:true});",
    )
    .unwrap();

    let added = dom.create_element("p", elidex_ecs::Attributes::default());
    let record = SessionRecord {
        kind: MutationKind::ChildList,
        target: root,
        added_nodes: vec![added],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: None,
        old_value: None,
    };
    vm.deliver_mutation_records(std::slice::from_ref(&record));

    let a = vm.eval("aLen").unwrap();
    let b = vm.eval("bLen").unwrap();
    assert_eq!(a, JsValue::Number(1.0));
    assert_eq!(b, JsValue::Number(1.0));
    vm.unbind();
}

#[test]
fn mutation_observer_observe_replaces_existing_options_for_same_target() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         var mo = new MutationObserver(function(rec){ globalThis.records = rec; }); \
         mo.observe(root, {childList:true}); \
         mo.observe(root, {attributes:true});",
    )
    .unwrap();

    // ChildList mutation must NOT fire because the second observe
    // replaced childList:true with attributes:true (WHATWG §4.3.3
    // step 7).
    let added = dom.create_element("p", elidex_ecs::Attributes::default());
    let r1 = SessionRecord {
        kind: MutationKind::ChildList,
        target: root,
        added_nodes: vec![added],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: None,
        old_value: None,
    };
    vm.deliver_mutation_records(std::slice::from_ref(&r1));
    assert!(matches!(vm.eval("records").unwrap(), JsValue::Null));

    // Attribute mutation MUST fire under the replaced options.
    let r2 = SessionRecord {
        kind: MutationKind::Attribute,
        target: root,
        added_nodes: vec![],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: Some("class".to_string()),
        old_value: None,
    };
    vm.deliver_mutation_records(std::slice::from_ref(&r2));
    assert_eq!(
        vm.eval("records.length").unwrap(),
        JsValue::Number(1.0),
        "second mutation must fire under the replacing options"
    );
    vm.unbind();
}

#[test]
fn mutation_observer_observes_multiple_distinct_targets() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);
    let other = dom.create_element("aside", elidex_ecs::Attributes::default());
    assert!(dom.append_child(root, other));
    let other_wrapper = vm.inner.create_element_wrapper(other);
    vm.set_global("other", JsValue::Object(other_wrapper));

    vm.eval(
        "globalThis.calls = 0; \
         var mo = new MutationObserver(function(){ calls++; }); \
         mo.observe(root, {childList:true}); \
         mo.observe(other, {attributes:true});",
    )
    .unwrap();

    let added = dom.create_element("p", elidex_ecs::Attributes::default());
    vm.deliver_mutation_records(&[SessionRecord {
        kind: MutationKind::ChildList,
        target: root,
        added_nodes: vec![added],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: None,
        old_value: None,
    }]);
    vm.deliver_mutation_records(&[SessionRecord {
        kind: MutationKind::Attribute,
        target: other,
        added_nodes: vec![],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: Some("data-x".to_string()),
        old_value: None,
    }]);

    assert_eq!(vm.eval("calls").unwrap(), JsValue::Number(2.0));
    vm.unbind();
}

// --- Callback exception isolation ------------------------------------

#[test]
fn mutation_observer_callback_throw_does_not_block_sibling_observer() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);

    // moA throws; moB must still fire (WHATWG §4.3.4 / §8.1.5
    // "report the exception" — does not abort sibling delivery).
    vm.eval(
        "globalThis.bRan = 0; \
         var moA = new MutationObserver(function(){ throw new Error('boom'); }); \
         var moB = new MutationObserver(function(){ bRan++; }); \
         moA.observe(root, {childList:true}); \
         moB.observe(root, {childList:true});",
    )
    .unwrap();

    let added = dom.create_element("p", elidex_ecs::Attributes::default());
    vm.deliver_mutation_records(&[SessionRecord {
        kind: MutationKind::ChildList,
        target: root,
        added_nodes: vec![added],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: None,
        old_value: None,
    }]);

    assert_eq!(
        vm.eval("bRan").unwrap(),
        JsValue::Number(1.0),
        "sibling observer must run despite first observer throwing"
    );
    vm.unbind();
}

// --- takeRecords semantics ------------------------------------------

#[test]
fn mutation_observer_take_records_after_disconnect_is_empty() {
    let out = run("var mo = new MutationObserver(function(){}); \
         mo.observe(document, {childList:true}); \
         mo.disconnect(); \
         '' + mo.takeRecords().length;");
    assert_eq!(out, "0");
}

#[test]
fn mutation_observer_take_records_returns_fresh_array_each_call() {
    let out = run("var mo = new MutationObserver(function(){}); \
         var a = mo.takeRecords(); \
         var b = mo.takeRecords(); \
         (a !== b) ? 'fresh' : 'same';");
    assert_eq!(out, "fresh");
}
