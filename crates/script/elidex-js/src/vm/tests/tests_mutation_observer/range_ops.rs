//! B1.2d-ii — end-to-end `MutationObserver` integration for the **Range
//! tree-mutation methods** (`insertNode` / `deleteContents` /
//! `extractContents`) plus the **`Selection.deleteFromDocument`** caller that
//! delegates to `Range::delete_contents` (it routes the same childList records
//! through the shared `commit_range_mutation_records` chokepoint).
//!
//! These methods were MO-silent before B1.2d-ii: the VM natives called the
//! engine-independent `Range::*` impl, which in turn called `EcsDom`
//! primitives directly — bypassing the record-producing `apply_*` layer. The
//! convergence routes the **childList** facet of each through `apply_*` →
//! `session.notify_records` → the single `drain_notify_records` chokepoint →
//! `notify` → §4.3 microtask → callback, mirroring the direct-tree-op coverage
//! in [`super::direct_tree_ops`].
//!
//! The **characterData** facet (boundary text `replaceData`, `splitText`) now
//! routes through `apply_replace_data` / the `character_data_record` builder
//! (B1.3-ii): a Range wholly inside a single Text node delivers ONE
//! characterData record — locked by
//! [`intra_single_text_delete_contents_delivers_one_character_data_record`].

use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::super::value::JsValue;
use super::super::super::Vm;
use super::setup_with_root;

/// Scenario 1 — `insertNode` delivers a childList **insertion** record.
#[test]
fn insert_node_delivers_childlist_insertion_record() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    // root has one child `anchor`; insert `x` at offset 1 (before nothing →
    // appended after anchor when collapsed at root[1]).
    vm.eval(
        "globalThis.records = null; \
         globalThis.anchor = document.createElement('b'); root.appendChild(anchor); \
         globalThis.x = document.createElement('x'); \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {childList:true}); \
         var rg = document.createRange(); rg.setStart(root, 1); rg.setEnd(root, 1); \
         rg.insertNode(x);",
    )
    .unwrap();

    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval("records[0].type === 'childList'").unwrap(),
        JsValue::Boolean(true),
    );
    assert_eq!(
        vm.eval("records[0].target === root").unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("records[0].addedNodes.length === 1 && records[0].addedNodes[0] === x")
            .unwrap(),
        JsValue::Boolean(true)
    );
    // Inserted at root[1], i.e. after the existing anchor child.
    assert_eq!(
        vm.eval("records[0].previousSibling === anchor").unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("root.childNodes.length === 2 && root.childNodes[1] === x")
            .unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
}

/// Scenario 1b — `insertNode` of a `DocumentFragment` reuses the §4.2.3
/// fragment-expand record shape (one destination record listing all children).
#[test]
fn insert_node_fragment_flattens_into_one_record() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         globalThis.s1 = document.createElement('s1'); \
         globalThis.s2 = document.createElement('s2'); \
         var frag = document.createDocumentFragment(); \
         frag.appendChild(s1); frag.appendChild(s2); \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {childList:true}); \
         var rg = document.createRange(); rg.setStart(root, 0); rg.setEnd(root, 0); \
         rg.insertNode(frag);",
    )
    .unwrap();

    // The fragment is expanded by `apply_*`; the observer on root sees ONE
    // destination record with both leaves (the unobserved frag-emptying record
    // targets the fragment).
    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval(
            "records[0].target === root && records[0].addedNodes.length === 2 \
             && records[0].addedNodes[0] === s1 && records[0].addedNodes[1] === s2"
        )
        .unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
}

/// Scenario 2 — `deleteContents` of fully-contained children delivers one
/// removal record per top-level child and fires the observer.
#[test]
fn delete_contents_delivers_removal_record_per_child() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = []; \
         globalThis.c1 = document.createElement('c1'); root.appendChild(c1); \
         globalThis.c2 = document.createElement('c2'); root.appendChild(c2); \
         var mo = new MutationObserver(function(r){ \
             for (var i=0;i<r.length;i++) globalThis.records.push(r[i]); }); \
         mo.observe(root, {childList:true}); \
         var rg = document.createRange(); rg.setStart(root, 0); rg.setEnd(root, 2); \
         rg.deleteContents();",
    )
    .unwrap();

    // One removal record per top-level removed child (§5.5 deleteContents
    // step 10), all targeting root.
    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(2.0));
    assert_eq!(
        vm.eval(
            "records[0].target === root && records[0].removedNodes.length === 1 \
             && records[0].removedNodes[0] === c1"
        )
        .unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval(
            "records[1].target === root && records[1].removedNodes.length === 1 \
             && records[1].removedNodes[0] === c2"
        )
        .unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("root.childNodes.length").unwrap(),
        JsValue::Number(0.0)
    );
    vm.unbind();
}

/// Scenario 2b — `Selection.deleteFromDocument()` routes the underlying
/// `Range::delete_contents` childList records through the SAME
/// `commit_range_mutation_records` chokepoint as the Range natives, so the
/// Selection caller is MO-observable too (One-issue-one-way: a
/// record-producing primitive's records are never silently dropped).  This
/// locks the convergence that wired the previously-MO-silent Selection path.
#[test]
fn selection_delete_from_document_delivers_removal_records() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = []; \
         globalThis.c1 = document.createElement('c1'); root.appendChild(c1); \
         globalThis.c2 = document.createElement('c2'); root.appendChild(c2); \
         var mo = new MutationObserver(function(r){ \
             for (var i=0;i<r.length;i++) globalThis.records.push(r[i]); }); \
         mo.observe(root, {childList:true}); \
         var sel = window.getSelection(); \
         sel.setBaseAndExtent(root, 0, root, 2); \
         sel.deleteFromDocument();",
    )
    .unwrap();

    // Same childList removal records as the Range native path (§5.5
    // deleteContents step 10) — one per top-level removed child, targeting
    // root, in tree order.
    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(2.0));
    assert_eq!(
        vm.eval(
            "records[0].target === root && records[0].removedNodes.length === 1 \
             && records[0].removedNodes[0] === c1 \
             && records[1].target === root && records[1].removedNodes.length === 1 \
             && records[1].removedNodes[0] === c2"
        )
        .unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("root.childNodes.length").unwrap(),
        JsValue::Number(0.0)
    );
    vm.unbind();
}

/// Scenario 3 — `extractContents` move delivers the OBSERVED source-removal
/// record (target = original source parent, NOT the fragment); the
/// fragment-insertion record is unobserved.
///
/// The old-parent-target assertion is what makes an "add-alongside" mis-impl
/// (plan §5 F2) fail by construction.
#[test]
fn extract_contents_move_records_source_removal_on_old_parent() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = []; \
         globalThis.child = document.createElement('c'); root.appendChild(child); \
         var mo = new MutationObserver(function(r){ \
             for (var i=0;i<r.length;i++) globalThis.records.push(r[i]); }); \
         mo.observe(root, {childList:true}); \
         var rg = document.createRange(); rg.setStart(root, 0); rg.setEnd(root, 1); \
         globalThis.frag = rg.extractContents();",
    )
    .unwrap();

    // The observer on root sees exactly the OBSERVED source-removal — the
    // fragment-insertion record targets the fresh (unobserved) fragment.
    assert_eq!(
        vm.eval("records.length").unwrap(),
        JsValue::Number(1.0),
        "observer on the source parent sees the move's source-removal only"
    );
    assert_eq!(
        vm.eval("records[0].type === 'childList'").unwrap(),
        JsValue::Boolean(true),
    );
    assert_eq!(
        vm.eval(
            "records[0].target === root && records[0].removedNodes.length === 1 \
             && records[0].removedNodes[0] === child"
        )
        .unwrap(),
        JsValue::Boolean(true),
        "removal record targets the ORIGINAL source parent, not the fragment"
    );
    // No delivered record targets the fragment (it has no observers).
    assert_eq!(
        vm.eval("records[0].target === frag").unwrap(),
        JsValue::Boolean(false)
    );
    // The child actually moved into the fragment.
    assert_eq!(
        vm.eval("frag.childNodes.length === 1 && frag.childNodes[0] === child")
            .unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
}

/// Scenario 4 — a `subtree:true` observer on an ancestor of removed nodes gets
/// a **transient registered observer** on the detached subtree; a subsequent
/// mutation inside the now-detached subtree is delivered.
#[test]
fn extract_creates_transient_observer_on_detached_subtree() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    // root -> wrap -> inner ; observe root with subtree. `extractContents`
    // moves `wrap` (and its `inner`) into a fragment. The §4.2.3 "remove"
    // step-15 transient observer keeps observing the detached `wrap`, so a
    // subsequent append into `wrap` is still delivered — PROVIDED both happen
    // in the SAME microtask window (the transient is cleared at the §4.3 step
    // 6.3 delivery that ends each `vm.eval`). `wrapHits` counts records
    // targeting the detached `wrap`, which only the transient can carry.
    vm.eval(
        "globalThis.wrapHits = 0; \
         globalThis.wrap = document.createElement('wrap'); root.appendChild(wrap); \
         globalThis.inner = document.createElement('inner'); wrap.appendChild(inner); \
         globalThis.leaf = document.createElement('leaf'); \
         var mo = new MutationObserver(function(r){ \
             for (var i=0;i<r.length;i++) { if (r[i].target === wrap) globalThis.wrapHits++; } }); \
         mo.observe(root, {childList:true, subtree:true}); \
         var rg = document.createRange(); rg.setStart(root, 0); rg.setEnd(root, 1); \
         globalThis.frag = rg.extractContents(); \
         wrap.appendChild(leaf);",
    )
    .unwrap();

    assert_eq!(
        vm.eval("wrapHits").unwrap(),
        JsValue::Number(1.0),
        "the extract's source-removal created a transient that delivered the \
         detached-subtree append"
    );
    // The append landed in the now-detached subtree.
    assert_eq!(
        vm.eval("wrap.childNodes.length === 2 && wrap.childNodes[1] === leaf")
            .unwrap(),
        JsValue::Boolean(true)
    );

    // Next microtask window: the transient was cleared at the previous
    // delivery, so a further detached-subtree mutation reaches no observer.
    vm.eval("globalThis.leaf2 = document.createElement('leaf2'); wrap.appendChild(leaf2);")
        .unwrap();
    assert_eq!(
        vm.eval("wrapHits").unwrap(),
        JsValue::Number(1.0),
        "after the transient is cleared, the detached subtree is no longer observed"
    );
    vm.unbind();
}

/// Scenario 5 — a Range wholly inside a single Text node delivers exactly ONE
/// `characterData` record (§5.5 deleteContents step 3.1 = "replace data" on the
/// start CharacterData node). B1.3-ii flipped this from the pre-slice ZERO:
/// the intra-Text splice now routes through `apply_replace_data` and queues a
/// characterData record on the Text node. `characterDataOldValue:true` exposes
/// the pre-splice full data as `oldValue`.
#[test]
fn intra_single_text_delete_contents_delivers_one_character_data_record() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    // root -> text("hello"). Range [1..4] is wholly inside the Text node →
    // deleteContents splices the text (characterData) via apply_replace_data.
    vm.eval(
        "globalThis.records = null; \
         globalThis.t = document.createTextNode('hello'); root.appendChild(t); \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {childList:true, characterData:true, \
                           characterDataOldValue:true, subtree:true}); \
         var rg = document.createRange(); rg.setStart(t, 1); rg.setEnd(t, 4); \
         rg.deleteContents();",
    )
    .unwrap();

    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval("records[0].type === 'characterData'").unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("records[0].target === t").unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("records[0].oldValue === 'hello'").unwrap(),
        JsValue::Boolean(true)
    );
    // The deletion still happened (live-range / text splice via EcsDom hook).
    assert_eq!(vm.eval("t.data === 'ho'").unwrap(), JsValue::Boolean(true),);
    vm.unbind();
}

/// Scenario 6 (B1.3-ii) — `deleteContents` over a single **Comment** container
/// (F5): a Range wholly inside a Comment now splices the comment data AND
/// delivers one characterData record. Pre-slice this was a silent no-op (the
/// same-container guard was `TextContent`-only, so a Comment fell to the
/// vacuous children-removal branch). Locks the broadened CharacterData guard +
/// the latent-no-op fix.
#[test]
fn intra_single_comment_delete_contents_delivers_one_character_data_record() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         globalThis.c = document.createComment('hello'); root.appendChild(c); \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {characterData:true, characterDataOldValue:true, subtree:true}); \
         var rg = document.createRange(); rg.setStart(c, 1); rg.setEnd(c, 4); \
         rg.deleteContents();",
    )
    .unwrap();

    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval("records[0].type === 'characterData' && records[0].target === c")
            .unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("records[0].oldValue === 'hello'").unwrap(),
        JsValue::Boolean(true)
    );
    // The splice actually happened (was a no-op pre-slice).
    assert_eq!(vm.eval("c.data === 'ho'").unwrap(), JsValue::Boolean(true));
    vm.unbind();
}

/// Scenario 7 (B1.3-ii) — `deleteContents` cross-container delivers records in
/// §5.5 order: step 9 start-trunc (characterData) → step 10 removal(s)
/// (childList) → step 11 end-trunc (characterData). Locks the reorder (the
/// pre-slice impl did start → end → removals, harmless only while truncs were
/// record-silent).
#[test]
fn cross_container_delete_contents_records_in_spec_order() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    // root -> [ t1("hello"), mid(<m/>), t2("world") ]. Range [t1:1 .. t2:4]
    // → start-trunc t1, remove mid, end-trunc t2.
    vm.eval(
        "globalThis.records = []; \
         globalThis.t1 = document.createTextNode('hello'); root.appendChild(t1); \
         globalThis.mid = document.createElement('m'); root.appendChild(mid); \
         globalThis.t2 = document.createTextNode('world'); root.appendChild(t2); \
         var mo = new MutationObserver(function(r){ \
             for (var i=0;i<r.length;i++) globalThis.records.push(r[i]); }); \
         mo.observe(root, {childList:true, characterData:true, \
                           characterDataOldValue:true, subtree:true}); \
         var rg = document.createRange(); rg.setStart(t1, 1); rg.setEnd(t2, 4); \
         rg.deleteContents();",
    )
    .unwrap();

    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(3.0));
    // Step 9: characterData start-trunc on t1.
    assert_eq!(
        vm.eval(
            "records[0].type === 'characterData' && records[0].target === t1 \
                 && records[0].oldValue === 'hello'"
        )
        .unwrap(),
        JsValue::Boolean(true)
    );
    // Step 10: childList removal of `mid` on root.
    assert_eq!(
        vm.eval(
            "records[1].type === 'childList' && records[1].target === root \
                 && records[1].removedNodes[0] === mid"
        )
        .unwrap(),
        JsValue::Boolean(true)
    );
    // Step 11: characterData end-trunc on t2.
    assert_eq!(
        vm.eval(
            "records[2].type === 'characterData' && records[2].target === t2 \
                 && records[2].oldValue === 'world'"
        )
        .unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(vm.eval("t1.data === 'h'").unwrap(), JsValue::Boolean(true));
    assert_eq!(vm.eval("t2.data === 'd'").unwrap(), JsValue::Boolean(true));
    vm.unbind();
}

/// Scenario 8 (B1.3-ii) — `extractContents` same-container Text delivers one
/// characterData splice record (§5.5 extract step 4.4) on the original node.
#[test]
fn intra_text_extract_contents_delivers_one_character_data_record() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         globalThis.t = document.createTextNode('hello'); root.appendChild(t); \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {characterData:true, characterDataOldValue:true, subtree:true}); \
         var rg = document.createRange(); rg.setStart(t, 1); rg.setEnd(t, 4); \
         globalThis.frag = rg.extractContents();",
    )
    .unwrap();

    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval(
            "records[0].type === 'characterData' && records[0].target === t \
                 && records[0].oldValue === 'hello'"
        )
        .unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(vm.eval("t.data === 'ho'").unwrap(), JsValue::Boolean(true));
    vm.unbind();
}

/// Scenario 9 (B1.3-ii) — `Range.insertNode` at a **Text boundary** splits the
/// text node (§5.5 step 7) BEFORE the step-12 insert, so the split's two
/// records (childList new-tail + characterData head-trunc) precede the insert
/// record(s).
#[test]
fn insert_node_at_text_boundary_split_records_precede_insert() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    // root -> t("hello"). insertNode(<x/>) at t:2 → split t into "he" + "llo",
    // then insert x between them.
    vm.eval(
        "globalThis.records = []; \
         globalThis.t = document.createTextNode('hello'); root.appendChild(t); \
         globalThis.x = document.createElement('x'); \
         var mo = new MutationObserver(function(r){ \
             for (var i=0;i<r.length;i++) globalThis.records.push(r[i]); }); \
         mo.observe(root, {childList:true, characterData:true, \
                           characterDataOldValue:true, subtree:true}); \
         var rg = document.createRange(); rg.setStart(t, 2); rg.setEnd(t, 2); \
         rg.insertNode(x);",
    )
    .unwrap();

    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(3.0));
    // Record 0: step-7 split childList insert (new tail node) on root.
    assert_eq!(
        vm.eval(
            "records[0].type === 'childList' && records[0].target === root \
                 && records[0].addedNodes.length === 1"
        )
        .unwrap(),
        JsValue::Boolean(true)
    );
    // Record 1: step-7/8 split characterData head-trunc on t.
    assert_eq!(
        vm.eval(
            "records[1].type === 'characterData' && records[1].target === t \
                 && records[1].oldValue === 'hello'"
        )
        .unwrap(),
        JsValue::Boolean(true)
    );
    // Record 2: step-12 childList insert of x on root.
    assert_eq!(
        vm.eval(
            "records[2].type === 'childList' && records[2].target === root \
                 && records[2].addedNodes[0] === x"
        )
        .unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(vm.eval("t.data === 'he'").unwrap(), JsValue::Boolean(true));
    vm.unbind();
}

/// Scenario 10 (B1.3-ii) — `Selection.deleteFromDocument()` over an intra-Text
/// range delivers one characterData record (parity with the Range path), since
/// it routes through the same `delete_contents` returned vec.
#[test]
fn selection_delete_from_document_intra_text_delivers_character_data_record() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         globalThis.t = document.createTextNode('hello'); root.appendChild(t); \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {characterData:true, characterDataOldValue:true, subtree:true}); \
         var sel = window.getSelection(); \
         sel.setBaseAndExtent(t, 1, t, 4); \
         sel.deleteFromDocument();",
    )
    .unwrap();

    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval(
            "records[0].type === 'characterData' && records[0].target === t \
                 && records[0].oldValue === 'hello'"
        )
        .unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(vm.eval("t.data === 'ho'").unwrap(), JsValue::Boolean(true));
    vm.unbind();
}
