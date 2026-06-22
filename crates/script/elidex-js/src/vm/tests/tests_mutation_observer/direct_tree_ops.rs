//! B1.2b — end-to-end `MutationObserver` integration for the **direct tree
//! manipulation methods** (ChildNode/ParentNode mixin:
//! `before`/`after`/`replaceWith`/`remove`/`prepend`/`append`/`replaceChildren`;
//! B1.2b-2 adds `insertAdjacentElement`/`insertAdjacentText`).
//!
//! These methods were MO-silent before B1.2b: the VM re-implemented the algorithm
//! and called `EcsDom` directly. The convergence routes them through the
//! engine-independent dom-api handler (`invoke_dom_api`) → the record-producing
//! `apply_*` primitives → `notify` → §4.3 microtask → callback. Here we drive a
//! **real JS mutation** and assert the delivered records, mirroring the
//! `appendChild`/`insertBefore`/`replaceChild` coverage in [`super::integration`].

use elidex_ecs::{Attributes, EcsDom};
use elidex_script_session::SessionCore;

use super::super::super::value::JsValue;
use super::super::super::Vm;
use super::setup_with_root;

fn string_global(vm: &mut Vm, expr: &str) -> String {
    let v = vm.eval(expr).unwrap();
    let JsValue::String(sid) = v else {
        panic!("expected string from `{expr}`, got {v:?}")
    };
    vm.inner.strings.get_utf8(sid).clone()
}

/// Append a fresh element child of `root` in Rust (no record) and expose its
/// wrapper as the JS global `name`.
fn seed_child(vm: &mut Vm, dom: &mut EcsDom, root: elidex_ecs::Entity, tag: &str, name: &str) {
    let e = dom.create_element(tag, Attributes::default());
    assert!(dom.append_child(root, e));
    let wrapper = vm.inner.create_element_wrapper(e);
    vm.set_global(name, JsValue::Object(wrapper));
}

#[test]
fn real_append_method_delivers_one_childlist_record_with_all_added() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         globalThis.a = document.createElement('a'); \
         globalThis.b = document.createElement('b'); \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {childList:true}); \
         root.append(a, b);",
    )
    .unwrap();

    // `append(a, b)` builds a transient fragment (convert nodes into a node) that
    // expand_fragment moves into root → ONE destination record, addedNodes = [a, b].
    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(string_global(&mut vm, "records[0].type"), "childList");
    assert_eq!(
        vm.eval("records[0].addedNodes.length").unwrap(),
        JsValue::Number(2.0)
    );
    assert_eq!(
        vm.eval("records[0].addedNodes[0] === a && records[0].addedNodes[1] === b")
            .unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("records[0].target === root").unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
}

#[test]
fn real_append_string_arg_creates_text_node_record() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {childList:true}); \
         root.append('hi');",
    )
    .unwrap();

    // The DOMString arg is converted to a Text node by the dom-api handler's
    // `collect_nodes` (one string→Text home), then delivered as the added node.
    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval("records[0].addedNodes.length").unwrap(),
        JsValue::Number(1.0)
    );
    // nodeType 3 = Text.
    assert_eq!(
        vm.eval("records[0].addedNodes[0].nodeType").unwrap(),
        JsValue::Number(3.0)
    );
    vm.unbind();
}

#[test]
fn real_prepend_method_records_added_before_first_child() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);
    seed_child(&mut vm, &mut dom, root, "b", "first");

    vm.eval(
        "globalThis.records = null; \
         globalThis.x = document.createElement('x'); \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {childList:true}); \
         root.prepend(x);",
    )
    .unwrap();

    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval("records[0].addedNodes[0] === x").unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("records[0].nextSibling === first").unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("root.childNodes[0] === x").unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
}

#[test]
fn real_before_method_records_on_parent_with_next_sibling() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);
    seed_child(&mut vm, &mut dom, root, "b", "anchor");

    vm.eval(
        "globalThis.records = null; \
         globalThis.x = document.createElement('x'); \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {childList:true}); \
         anchor.before(x);",
    )
    .unwrap();

    // `anchor.before(x)` inserts x into root before anchor → record targets root.
    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval("records[0].target === root").unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("records[0].addedNodes[0] === x").unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("records[0].nextSibling === anchor").unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
}

#[test]
fn real_after_method_records_on_parent() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);
    seed_child(&mut vm, &mut dom, root, "b", "anchor");

    vm.eval(
        "globalThis.records = null; \
         globalThis.x = document.createElement('x'); \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {childList:true}); \
         anchor.after(x);",
    )
    .unwrap();

    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval("records[0].addedNodes[0] === x").unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("records[0].previousSibling === anchor").unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
}

#[test]
fn real_replace_with_same_parent_delivers_one_coalesced_record() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);
    seed_child(&mut vm, &mut dom, root, "b", "victim");

    vm.eval(
        "globalThis.records = null; \
         globalThis.x = document.createElement('x'); \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {childList:true}); \
         victim.replaceWith(x);",
    )
    .unwrap();

    // §4.2.8 replaceWith step 5 = "replace this with node" = the §4.2.3 replace
    // algorithm = ONE coalesced record (removedNodes=[victim], addedNodes=[x]).
    assert_eq!(
        vm.eval("records.length").unwrap(),
        JsValue::Number(1.0),
        "same-parent replaceWith delivers ONE coalesced record, not remove+insert"
    );
    assert_eq!(
        vm.eval("records[0].addedNodes.length === 1 && records[0].addedNodes[0] === x")
            .unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("records[0].removedNodes.length === 1 && records[0].removedNodes[0] === victim")
            .unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
}

#[test]
fn real_child_node_remove_method_records_removed_node() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);
    seed_child(&mut vm, &mut dom, root, "b", "victim");

    vm.eval(
        "globalThis.records = null; \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {childList:true}); \
         victim.remove();",
    )
    .unwrap();

    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval("records[0].removedNodes.length === 1 && records[0].removedNodes[0] === victim")
            .unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("records[0].addedNodes.length").unwrap(),
        JsValue::Number(0.0)
    );
    vm.unbind();
}

#[test]
fn real_replace_children_delivers_single_combined_record() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);
    seed_child(&mut vm, &mut dom, root, "old1", "o1");
    seed_child(&mut vm, &mut dom, root, "old2", "o2");

    vm.eval(
        "globalThis.records = null; \
         globalThis.a = document.createElement('a'); \
         globalThis.b = document.createElement('b'); \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {childList:true}); \
         root.replaceChildren(a, b);",
    )
    .unwrap();

    // §4.2.3 "replace all": per-child removals are suppressObservers; exactly ONE
    // combined record is queued (removedNodes = old children, addedNodes = new).
    assert_eq!(
        vm.eval("records.length").unwrap(),
        JsValue::Number(1.0),
        "replace-all delivers ONE combined record, not per-node"
    );
    assert_eq!(
        vm.eval(
            "records[0].removedNodes.length === 2 \
                 && records[0].removedNodes[0] === o1 && records[0].removedNodes[1] === o2"
        )
        .unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval(
            "records[0].addedNodes.length === 2 \
                 && records[0].addedNodes[0] === a && records[0].addedNodes[1] === b"
        )
        .unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
}

#[test]
fn real_replace_children_empty_args_clears_with_one_record() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);
    seed_child(&mut vm, &mut dom, root, "old", "o1");

    vm.eval(
        "globalThis.records = null; \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {childList:true}); \
         root.replaceChildren();",
    )
    .unwrap();

    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval("records[0].removedNodes.length === 1 && records[0].removedNodes[0] === o1")
            .unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("records[0].addedNodes.length").unwrap(),
        JsValue::Number(0.0)
    );
    assert_eq!(
        vm.eval("root.childNodes.length").unwrap(),
        JsValue::Number(0.0)
    );
    vm.unbind();
}

#[test]
fn real_replace_children_empty_on_empty_parent_delivers_no_record() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {childList:true}); \
         root.replaceChildren();",
    )
    .unwrap();

    // §4.2.3 replace-all step 7: queue a record ONLY if addedNodes ∪ removedNodes
    // is non-empty. An empty parent + empty args ⇒ no record at all.
    assert_eq!(
        vm.eval("records === null").unwrap(),
        JsValue::Boolean(true),
        "empty parent + empty args = zero records (no spurious empty record)"
    );
    vm.unbind();
}

#[test]
fn real_append_multiple_fragments_flattens_into_added_nodes() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         globalThis.s1 = document.createElement('s1'); \
         globalThis.s2 = document.createElement('s2'); \
         globalThis.s3 = document.createElement('s3'); \
         var f1 = document.createDocumentFragment(); \
         var f2 = document.createDocumentFragment(); \
         f1.appendChild(s1); f1.appendChild(s2); f2.appendChild(s3); \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {childList:true}); \
         root.append(f1, f2);",
    )
    .unwrap();

    // `append(f1, f2)` — "convert nodes into a node" must EXPAND each fragment arg
    // (§4.2.6 step 4 uses the DOM "append") so the temp fragment is FLAT; no nested
    // fragment is ever linked into root. ONE record, addedNodes = [s1, s2, s3].
    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval("records[0].addedNodes.length").unwrap(),
        JsValue::Number(3.0),
        "all leaf children flattened into addedNodes, no fragment node"
    );
    assert_eq!(
        vm.eval(
            "records[0].addedNodes[0] === s1 && records[0].addedNodes[1] === s2 \
             && records[0].addedNodes[2] === s3"
        )
        .unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("root.childNodes.length").unwrap(),
        JsValue::Number(3.0),
        "exactly the three leaves are linked into root (no nested fragment)"
    );
    vm.unbind();
}

#[test]
fn real_append_multiarg_move_reports_source_removal_on_old_parent() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    // `root.append(kid, b)` where `kid` belongs to `other`: the "convert nodes into a
    // node" wrapper build moves `kid` out of `other`, and §4.5 adopt's unsuppressed
    // remove must record that source-parent removal — observable on `other`. (Codex
    // PR393 R1 finding 1: multi-arg moves under-reported the source removal.)
    vm.eval(
        "globalThis.records = null; \
         globalThis.other = document.createElement('section'); \
         globalThis.kid = document.createElement('kid'); \
         other.appendChild(kid); \
         globalThis.b = document.createElement('b'); \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(other, {childList:true}); \
         root.append(kid, b);",
    )
    .unwrap();

    assert_eq!(
        vm.eval("records.length").unwrap(),
        JsValue::Number(1.0),
        "observer on the old parent sees the source removal of the moved child"
    );
    assert_eq!(
        vm.eval("records[0].target === other").unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("records[0].removedNodes.length === 1 && records[0].removedNodes[0] === kid")
            .unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("other.childNodes.length").unwrap(),
        JsValue::Number(0.0)
    );
    vm.unbind();
}

#[test]
fn real_prepend_self_reference_first_child_is_noop_move_with_records() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);
    seed_child(&mut vm, &mut dom, root, "first", "first");
    seed_child(&mut vm, &mut dom, root, "second", "second");

    // `root.prepend(root.firstChild)` — the reference child IS the node, so §4.2.3
    // pre-insert step 3 advances the reference to its next sibling, making it a
    // no-position-change move (source removal + destination) rather than the
    // self-reference rejection that would drop it silently. (Codex PR393 R1 finding 3.)
    vm.eval(
        "globalThis.records = null; \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {childList:true}); \
         root.prepend(first);",
    )
    .unwrap();

    assert_eq!(
        vm.eval("records.length").unwrap(),
        JsValue::Number(2.0),
        "no-position-change self-move delivers source removal + destination records"
    );
    assert_eq!(
        vm.eval("records[0].removedNodes.length === 1 && records[0].removedNodes[0] === first")
            .unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("records[1].addedNodes.length === 1 && records[1].addedNodes[0] === first")
            .unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("root.childNodes[0] === first && root.childNodes[1] === second")
            .unwrap(),
        JsValue::Boolean(true),
        "first stays first (no position change)"
    );
    vm.unbind();
}

#[test]
fn real_replace_children_fragment_arg_records_fragment_emptying() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    // `root.replaceChildren(frag)` empties `frag` via §4.2.3 insert step 4.2, whose
    // fragment record is NOT suppressed even under replace-all's suppressObservers —
    // so an observer on `frag` sees its children leave. (Codex PR393 R1 finding 2.)
    vm.eval(
        "globalThis.fragRecords = null; \
         globalThis.frag = document.createDocumentFragment(); \
         globalThis.c1 = document.createElement('c1'); \
         globalThis.c2 = document.createElement('c2'); \
         frag.appendChild(c1); frag.appendChild(c2); \
         var mo = new MutationObserver(function(r){ globalThis.fragRecords = r; }); \
         mo.observe(frag, {childList:true}); \
         root.replaceChildren(frag);",
    )
    .unwrap();

    assert_eq!(
        vm.eval("fragRecords.length").unwrap(),
        JsValue::Number(1.0),
        "observer on the fragment sees the step-4.2 fragment record"
    );
    assert_eq!(
        vm.eval("fragRecords[0].target === frag").unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval(
            "fragRecords[0].removedNodes.length === 2 \
             && fragRecords[0].removedNodes[0] === c1 && fragRecords[0].removedNodes[1] === c2"
        )
        .unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("root.childNodes.length").unwrap(),
        JsValue::Number(2.0),
        "frag's children moved into root"
    );
    vm.unbind();
}

// ---------------------------------------------------------------------------
// B1.2b-2 — insertAdjacentElement / insertAdjacentText (WHATWG DOM §4.9).
// Previously MO-silent (the VM re-implemented the position+insert algorithm and
// called `EcsDom` directly); the convergence routes both through the
// engine-independent handler → `apply_*` → records.
// ---------------------------------------------------------------------------

#[test]
fn real_insert_adjacent_element_beforeend_delivers_record() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         globalThis.x = document.createElement('x'); \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {childList:true}); \
         globalThis.r = root.insertAdjacentElement('beforeend', x);",
    )
    .unwrap();

    // beforeend appends a fresh element into root → ONE childList record on root.
    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval(
            "records[0].target === root && records[0].addedNodes.length === 1 \
                 && records[0].addedNodes[0] === x"
        )
        .unwrap(),
        JsValue::Boolean(true)
    );
    // Returns the inserted element (identity-stable wrapper).
    assert_eq!(vm.eval("r === x").unwrap(), JsValue::Boolean(true));
    vm.unbind();
}

#[test]
fn real_insert_adjacent_element_afterend_records_on_parent() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    // afterend inserts into root's PARENT (body) after root → record targets the
    // parent, with previousSibling === root. Observe body to see it.
    vm.eval(
        "globalThis.records = null; \
         globalThis.par = root.parentNode; \
         globalThis.x = document.createElement('x'); \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(par, {childList:true}); \
         root.insertAdjacentElement('afterend', x);",
    )
    .unwrap();

    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval(
            "records[0].target === par && records[0].addedNodes[0] === x \
                 && records[0].previousSibling === root"
        )
        .unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
}

#[test]
fn real_insert_adjacent_text_delivers_text_record() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {childList:true}); \
         globalThis.r = root.insertAdjacentText('beforeend', 'hi');",
    )
    .unwrap();

    // The DOMString is materialised into a fresh Text by the handler → ONE record.
    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval(
            "records[0].addedNodes.length === 1 \
                 && records[0].addedNodes[0].nodeType === 3 \
                 && records[0].addedNodes[0].data === 'hi'"
        )
        .unwrap(),
        JsValue::Boolean(true)
    );
    // insertAdjacentText is void.
    assert_eq!(vm.eval("r === undefined").unwrap(), JsValue::Boolean(true));
    vm.unbind();
}

#[test]
fn real_insert_adjacent_element_move_reports_source_removal_then_dest() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    // `kid` lives in `other`; both `other` and `root` are under body. Inserting an
    // already-parented element is a MOVE = §4.5 adopt (source removal, NOT
    // suppressed) + destination insertion = TWO records, source-removal THEN dest.
    // A subtree observer on the common ancestor (body) sees both, in order.
    vm.eval(
        "globalThis.records = null; \
         globalThis.body = root.parentNode; \
         globalThis.other = document.createElement('section'); \
         body.appendChild(other); \
         globalThis.kid = document.createElement('kid'); \
         other.appendChild(kid); \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(body, {childList:true, subtree:true}); \
         root.insertAdjacentElement('beforeend', kid);",
    )
    .unwrap();

    assert_eq!(
        vm.eval("records.length").unwrap(),
        JsValue::Number(2.0),
        "a move delivers source-removal + destination records"
    );
    assert_eq!(
        vm.eval(
            "records[0].target === other \
                 && records[0].removedNodes.length === 1 && records[0].removedNodes[0] === kid"
        )
        .unwrap(),
        JsValue::Boolean(true),
        "record 0 = source-parent removal"
    );
    assert_eq!(
        vm.eval(
            "records[1].target === root \
                 && records[1].addedNodes.length === 1 && records[1].addedNodes[0] === kid"
        )
        .unwrap(),
        JsValue::Boolean(true),
        "record 1 = destination insertion"
    );
    vm.unbind();
}

#[test]
fn real_insert_adjacent_element_parent_null_noop_delivers_no_record() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    // `beforebegin` on a parent-less receiver is a silent no-op (DOM "return
    // null"), NOT a HierarchyRequestError — and produces NO mutation record.
    // Observe the detached receiver itself to confirm nothing is delivered.
    vm.eval(
        "globalThis.records = null; \
         globalThis.d = document.createElement('div'); \
         globalThis.x = document.createElement('x'); \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(d, {childList:true}); \
         globalThis.ret = d.insertAdjacentElement('beforebegin', x);",
    )
    .unwrap();

    assert_eq!(
        vm.eval("records === null").unwrap(),
        JsValue::Boolean(true),
        "parent-null no-op delivers no record"
    );
    assert_eq!(
        vm.eval("ret === null && d.childNodes.length === 0")
            .unwrap(),
        JsValue::Boolean(true),
        "no-op returns null and inserts nothing"
    );
    vm.unbind();
}

#[test]
fn real_insert_adjacent_element_afterbegin_on_shadow_host_skips_shadow_root() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    // DOM §4.8: a ShadowRoot is NOT a light-tree child of its host. `attachShadow`
    // links it into the host's physical child chain (as the first child when no
    // light children exist yet), but `afterbegin` must resolve its reference child
    // against the EXPOSED chain — the inserted node becomes the first light child
    // and the delivered record's `nextSibling` must be null, NOT the shadow root.
    // (A raw `get_first_child` would point at the shadow root and leak it into the
    // observable record's `nextSibling`.)
    vm.eval(
        "globalThis.records = null; \
         globalThis.sr = root.attachShadow({mode:'open'}); \
         globalThis.x = document.createElement('x'); \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {childList:true}); \
         root.insertAdjacentElement('afterbegin', x);",
    )
    .unwrap();

    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval("records[0].addedNodes.length === 1 && records[0].addedNodes[0] === x")
            .unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("records[0].nextSibling === null").unwrap(),
        JsValue::Boolean(true),
        "afterbegin must skip the shadow root, not insert before it (no shadow leak)"
    );
    assert_eq!(
        vm.eval("root.childNodes.length === 1 && root.childNodes[0] === x")
            .unwrap(),
        JsValue::Boolean(true),
        "x is the host's sole light child; the shadow root stays encapsulated"
    );
    vm.unbind();
}
