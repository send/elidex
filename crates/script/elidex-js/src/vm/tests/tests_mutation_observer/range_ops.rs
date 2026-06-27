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
//! The **characterData** facet (boundary text `replaceData`, `splitText`) is
//! deferred to B1.3 (no record producer exists yet); a Range wholly inside a
//! single Text node therefore delivers ZERO records here — locked by
//! [`intra_single_text_delete_contents_delivers_no_record`].

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

/// Scenario 5 — a Range wholly inside a single Text node delivers ZERO records
/// (its only mutation is characterData `replaceData`, deferred to B1.3).
///
/// **Lock this**: the zero is intentional, not a missing-record bug. A future
/// B1.3 author who adds characterData records should expect this assertion to
/// change to one record, and should read this test before doing so.
#[test]
fn intra_single_text_delete_contents_delivers_no_record() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    // root -> text("hello"). Range [1..4] is wholly inside the Text node →
    // deleteContents splices the text (characterData), no childList op.
    vm.eval(
        "globalThis.records = null; \
         globalThis.t = document.createTextNode('hello'); root.appendChild(t); \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {childList:true, characterData:true, subtree:true}); \
         var rg = document.createRange(); rg.setStart(t, 1); rg.setEnd(t, 4); \
         rg.deleteContents();",
    )
    .unwrap();

    // characterData record production does not exist yet (B1.3); the only
    // mutation is a text splice, so the observer fires no records.
    assert_eq!(
        vm.eval("records === null").unwrap(),
        JsValue::Boolean(true),
        "intra-Text deleteContents is characterData-only — ZERO records until B1.3"
    );
    // The deletion still happened (live-range / text splice via EcsDom hook).
    assert_eq!(vm.eval("t.data === 'ho'").unwrap(), JsValue::Boolean(true),);
    vm.unbind();
}
