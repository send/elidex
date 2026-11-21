//! `Selection` tree preconditions (tree-model `tsel` plan §7, cells
//! J1-J16): the setters abort (no exception, no `selectionchange`) unless
//! the associated document is a shadow-including inclusive ancestor of the
//! node; `selectAllChildren()` / `addRange()` need the root to be the
//! document; `deleteFromDocument()`, `getRangeAt()`, `containsNode()` and
//! the six attributes read "in the document tree" as the associated
//! document's tree. Every expected value is the Selection API / DOM text's.
//!
//! After the attributes' clause a selection outside the document tree
//! reads `rangeCount` 0 like an empty one, so a cell that must tell
//! "aborted" from "set outside the document tree" reads what has no clause:
//! `isCollapsed`, `toString()`, and `collapseToStart()`, which throws
//! "InvalidStateError" exactly when the selection is empty. Those cells call
//! it last, since on a non-empty selection it collapses the existing range
//! in place (the Selection API creates a new range and leaves the old
//! `Range` unchanged; that deviation is tracked with the PM).

#![cfg(feature = "engine")]
#![allow(unsafe_code)]

use super::super::test_helpers::{bind_vm, eval_num, eval_str, with_bound_vm};
use super::super::Vm;

/// The tree: `L` (the document element) > (`pre` text "pre", `H`, `post`
/// text "postTAIL"); `sr` is a closed shadow root of `H` > `se` > `st` text
/// "SHADOW"; `det` (detached) > `dt` text "detached"; a `DOMParser`
/// document's `fe` > `ft` text "foreign". A `selectionchange` listener
/// counts events in `n`.
const SETUP: &str = "globalThis.L = document.createElement('div');\
     document.appendChild(L);\
     globalThis.pre = document.createTextNode('pre');\
     globalThis.H = document.createElement('div');\
     globalThis.post = document.createTextNode('postTAIL');\
     L.appendChild(pre); L.appendChild(H); L.appendChild(post);\
     globalThis.sr = H.attachShadow({mode: 'closed'});\
     globalThis.se = document.createElement('span');\
     globalThis.st = document.createTextNode('SHADOW');\
     sr.appendChild(se); se.appendChild(st);\
     globalThis.det = document.createElement('p');\
     globalThis.dt = document.createTextNode('detached');\
     det.appendChild(dt);\
     globalThis.fd = new DOMParser().parseFromString('<p>foreign</p>', 'text/html');\
     globalThis.fe = fd.getElementsByTagName('p')[0];\
     globalThis.ft = fe.firstChild;\
     globalThis.s = window.getSelection();\
     globalThis.n = 0;\
     document.addEventListener('selectionchange', function() { globalThis.n += 1; });\
     globalThis.thr = function(f) { try { f(); return 'none'; } catch (e) { return e.name; } };";

fn with_tree(body: impl FnOnce(&mut Vm)) {
    with_bound_vm(
        |vm, session, dom, doc| {
            // SAFETY: `with_bound_vm` keeps `session` / `dom` live until its guard unbinds.
            unsafe { bind_vm(vm, session, dom, doc) };
            vm.eval(SETUP).unwrap();
        },
        body,
    );
}

/// `String(expr)`.
fn val(vm: &mut Vm, expr: &str) -> String {
    eval_str(vm, &format!("String({expr})"))
}

/// The `DOMException` name `stmt` throws, or `"none"`.
fn thrown(vm: &mut Vm, stmt: &str) -> String {
    eval_str(vm, &format!("thr(function() {{ {stmt} }})"))
}

fn run(vm: &mut Vm, script: &str) {
    vm.eval(script).unwrap();
}

fn count(vm: &mut Vm) -> f64 {
    eval_num(vm, "n")
}

/// Live-range registry size (`HostData::live_range_registry`).
fn registry_len(vm: &Vm) -> usize {
    vm.inner
        .host_data
        .as_ref()
        .unwrap()
        .live_range_registry
        .len()
}

/// The selection reads as an empty one does through the attributes.
fn assert_attributes_outside(vm: &mut Vm, what: &str) {
    assert_eq!(val(vm, "s.anchorNode"), "null", "{what}: anchorNode");
    assert_eq!(val(vm, "s.anchorOffset"), "0", "{what}: anchorOffset");
    assert_eq!(val(vm, "s.focusNode"), "null", "{what}: focusNode");
    assert_eq!(val(vm, "s.focusOffset"), "0", "{what}: focusOffset");
    assert_eq!(val(vm, "s.rangeCount"), "0", "{what}: rangeCount");
    assert_eq!(val(vm, "s.type"), "None", "{what}: type");
    assert_eq!(
        thrown(vm, "s.getRangeAt(0);"),
        "IndexSizeError",
        "{what}: getRangeAt"
    );
}

// ---------------------------------------------------------------------------
// The root check (facet 1)
// ---------------------------------------------------------------------------

#[test]
fn cross_root_set_base_and_extent_is_collapsed_and_empty() {
    for script in [
        "s.setBaseAndExtent(pre, 1, st, 3);",
        "s.setBaseAndExtent(st, 3, pre, 1);",
    ] {
        with_tree(|vm| {
            run(vm, script);
            assert_eq!(val(vm, "s.isCollapsed"), "true", "{script}");
            assert_eq!(val(vm, "s.toString()"), "", "{script}");
            // What the spec fixes wherever the collapse lands (Q-1).
            assert_eq!(
                val(vm, "(s.rangeCount === 0) === (s.anchorNode === null)"),
                "true"
            );
            assert_eq!(
                val(vm, "(s.anchorNode === null) === (s.focusNode === null)"),
                "true"
            );
            assert_eq!(
                val(vm, "s.type === (s.rangeCount === 0 ? 'None' : 'Caret')"),
                "true"
            );
            assert_eq!(
                val(
                    vm,
                    "(s.rangeCount === 0) === (thr(function() { s.getRangeAt(0); }) === 'IndexSizeError')"
                ),
                "true"
            );
            // Set (not empty): collapseToStart() does not throw.
            assert_eq!(thrown(vm, "s.collapseToStart();"), "none", "{script}");
        });
    }
}

/// `addRange()` step 1 reads the root of BOTH boundary points. A shadow
/// root's child moved out is not adjusted in the live ranges, so a
/// registered Range can span the document and a shadow tree; it is aborted.
#[test]
fn add_range_two_root_range_aborts() {
    with_tree(|vm| {
        run(
            vm,
            "var host = document.createElement('div'); L.appendChild(host);\
             var sr2 = host.attachShadow({mode: 'open'});\
             var a = document.createElement('p'); a.appendChild(document.createTextNode('AAA'));\
             var b = document.createElement('p'); b.appendChild(document.createTextNode('BBB'));\
             sr2.appendChild(a); sr2.appendChild(b);\
             var r = document.createRange();\
             r.setStart(a.firstChild, 1); r.setEnd(b.firstChild, 2);\
             document.documentElement.appendChild(a);\
             s.addRange(r);",
        );
        assert_eq!(val(vm, "s.rangeCount"), "0");
        assert_eq!(val(vm, "s.anchorNode"), "null");
        assert_eq!(thrown(vm, "s.collapseToStart();"), "InvalidStateError");
    });
}

#[test]
fn cross_root_extend_collapses_at_focus() {
    for script in [
        "s.collapse(pre, 1); s.extend(st, 3);",
        "s.collapse(post, 2); s.extend(st, 3);",
    ] {
        with_tree(|vm| {
            run(vm, script);
            assert_eq!(val(vm, "s.isCollapsed"), "true", "{script}");
            assert_eq!(val(vm, "s.toString()"), "", "{script}");
            // The caret is at (st, 3), inside the shadow tree.
            assert_attributes_outside(vm, script);
        });
    }
}

#[test]
fn cross_root_delete_from_document_removes_nothing() {
    for script in [
        "s.setBaseAndExtent(pre, 1, st, 3);",
        "s.collapse(pre, 1); s.extend(st, 3);",
    ] {
        with_tree(|vm| {
            run(vm, script);
            run(vm, "s.deleteFromDocument();");
            assert_eq!(val(vm, "pre.data"), "pre", "{script}");
            assert_eq!(val(vm, "st.data"), "SHADOW", "{script}");
            assert_eq!(val(vm, "se.isConnected"), "true", "{script}");
        });
    }
}

// ---------------------------------------------------------------------------
// "Root is the document" (facet 2) and the shadow-including check (facet 3)
// ---------------------------------------------------------------------------

#[test]
fn select_all_children_outside_document_tree_is_noop() {
    for node in ["se", "sr", "det"] {
        with_tree(|vm| {
            assert_eq!(thrown(vm, &format!("s.selectAllChildren({node});")), "none");
            assert_eq!(val(vm, "s.toString()"), "", "{node}");
            assert_eq!(
                thrown(vm, "s.collapseToStart();"),
                "InvalidStateError",
                "{node}: still empty"
            );
        });
    }
}

#[test]
fn add_range_outside_document_tree_is_noop() {
    for node in ["st", "dt"] {
        with_tree(|vm| {
            run(
                vm,
                &format!("globalThis.r = document.createRange(); r.setStart({node}, 0); r.setEnd({node}, 2);"),
            );
            assert_eq!(thrown(vm, "s.addRange(r);"), "none");
            assert_eq!(val(vm, "s.isCollapsed"), "true", "{node}");
            assert_eq!(val(vm, "s.toString()"), "", "{node}");
            assert_eq!(thrown(vm, "s.collapseToStart();"), "InvalidStateError");
        });
    }
}

#[test]
fn setters_outside_document_abort_without_throw() {
    with_tree(|vm| {
        for call in [
            "s.collapse(dt, 1);",
            "s.setBaseAndExtent(dt, 0, dt, 3);",
            "s.extend(dt, 1);",
            "s.collapse(ft, 1);",
            "s.setBaseAndExtent(ft, 1, pre, 0);",
            "s.selectAllChildren(fe);",
            "s.extend(ft, 1);",
        ] {
            assert_eq!(thrown(vm, call), "none", "{call}");
        }
        assert_eq!(val(vm, "s.toString()"), "");
        assert_eq!(thrown(vm, "s.collapseToStart();"), "InvalidStateError");
    });
    with_tree(|vm| {
        run(vm, "s.collapse(pre, 1);");
        assert_eq!(thrown(vm, "s.extend(dt, 1);"), "none");
        assert_eq!(val(vm, "s.anchorNode === pre"), "true");
        assert_eq!(val(vm, "s.anchorOffset"), "1");
        assert_eq!(val(vm, "s.focusNode === pre"), "true");
        assert_eq!(val(vm, "s.type"), "Caret");
    });
}

#[test]
fn aborted_setter_schedules_no_selectionchange() {
    for (setup, call) in [
        ("", "s.collapse(dt, 1);"),
        ("", "s.setBaseAndExtent(dt, 0, dt, 3);"),
        ("s.collapse(pre, 1);", "s.extend(dt, 1);"),
        ("", "s.selectAllChildren(se);"),
        (
            "globalThis.r = document.createRange(); r.setStart(st, 0); r.setEnd(st, 2);",
            "s.addRange(r);",
        ),
    ] {
        with_tree(|vm| {
            run(vm, setup);
            let before = count(vm);
            run(vm, call);
            assert_eq!(count(vm), before, "{call}: an abort schedules no event");
        });
    }
    with_tree(|vm| {
        run(vm, "s.collapse(pre, 1);");
        assert_eq!(count(vm), 1.0, "control: a set schedules one");
    });
}

#[test]
fn setter_step_order_exceptions() {
    with_tree(|vm| {
        // Offset before the document check (collapse step 3 before step 4).
        assert_eq!(thrown(vm, "s.collapse(ft, 99);"), "IndexSizeError");
        assert_eq!(thrown(vm, "s.collapse(dt, 99);"), "IndexSizeError");
        assert_eq!(
            thrown(vm, "s.setBaseAndExtent(dt, 99, pre, 0);"),
            "IndexSizeError"
        );
        // The document check before the emptiness check, then emptiness.
        assert_eq!(thrown(vm, "s.extend(pre, 1);"), "InvalidStateError");
    });
}

#[test]
fn shadow_tree_selection_is_accepted() {
    with_tree(|vm| {
        run(vm, "s.collapse(st, 1);");
        assert_eq!(count(vm), 1.0, "set, not aborted");
        assert_eq!(val(vm, "s.anchorNode"), "null");
        assert_eq!(val(vm, "s.anchorOffset"), "0");
        assert_eq!(val(vm, "s.rangeCount"), "0");
        assert_eq!(val(vm, "s.type"), "None");
        assert_eq!(thrown(vm, "s.collapseToStart();"), "none", "not empty");
        run(vm, "s.setBaseAndExtent(st, 0, st, 3);");
        assert_eq!(val(vm, "s.isCollapsed"), "false");
        assert_eq!(val(vm, "s.toString()"), "SHA");
        assert_eq!(val(vm, "s.anchorNode"), "null");
        assert_eq!(val(vm, "s.focusNode"), "null");
        assert_eq!(val(vm, "s.focusOffset"), "0");
        assert_eq!(val(vm, "s.rangeCount"), "0");
        assert_eq!(val(vm, "s.type"), "None");
    });
}

// ---------------------------------------------------------------------------
// deleteFromDocument (facet 4)
// ---------------------------------------------------------------------------

#[test]
fn delete_from_document_inside_shadow_tree_does_nothing() {
    with_tree(|vm| {
        run(
            vm,
            "globalThis.records = [];\
             new MutationObserver(function(r) { for (var i = 0; i < r.length; i++) records.push(r[i]); })\
                 .observe(sr, {characterData: true, subtree: true});\
             s.setBaseAndExtent(st, 0, st, 3);",
        );
        let before = count(vm);
        run(vm, "s.deleteFromDocument();");
        assert_eq!(val(vm, "st.data"), "SHADOW");
        assert_eq!(val(vm, "s.isCollapsed"), "false");
        assert_eq!(val(vm, "s.toString()"), "SHA");
        assert_eq!(val(vm, "s.rangeCount"), "0");
        assert_eq!(val(vm, "s.type"), "None");
        assert_eq!(val(vm, "s.anchorNode"), "null");
        assert_eq!(val(vm, "records.length"), "0", "no MutationRecord");
        assert_eq!(count(vm), before, "no selectionchange from the no-op");
    });
}

// ---------------------------------------------------------------------------
// The read paths (facets 5-7)
// ---------------------------------------------------------------------------

#[test]
fn contains_node_outside_document_tree_is_false() {
    with_tree(|vm| {
        run(vm, "s.selectAllChildren(L);");
        assert_eq!(val(vm, "s.containsNode(st)"), "false");
        assert_eq!(val(vm, "s.containsNode(st, true)"), "false");
        assert_eq!(val(vm, "s.containsNode(dt, true)"), "false");
        assert_eq!(val(vm, "s.containsNode(pre)"), "true", "control");
    });
}

#[test]
fn get_range_at_outside_document_tree_throws() {
    with_tree(|vm| {
        run(vm, "s.setBaseAndExtent(st, 0, st, 3);");
        assert_eq!(thrown(vm, "s.getRangeAt(0);"), "IndexSizeError");
    });
    with_tree(|vm| {
        run(vm, "s.setBaseAndExtent(pre, 0, pre, 2);");
        assert_eq!(val(vm, "s.getRangeAt(0) === s.getRangeAt(0)"), "true");
        assert_eq!(thrown(vm, "s.getRangeAt(1);"), "IndexSizeError");
    });
}

#[test]
fn attributes_outside_document_tree_are_null_zero_none() {
    with_tree(|vm| {
        run(vm, "s.setBaseAndExtent(st, 0, st, 3);");
        assert_attributes_outside(vm, "range in the shadow tree");
        // No clause on these: the concepts.
        assert_eq!(val(vm, "s.isCollapsed"), "false");
        assert_eq!(val(vm, "s.toString()"), "SHA");
        run(vm, "s.collapse(st, 1);");
        assert_eq!(val(vm, "s.anchorOffset"), "0");
        assert_eq!(val(vm, "s.focusOffset"), "0");
        assert_eq!(val(vm, "s.type"), "None");
    });
    with_tree(|vm| {
        run(vm, "s.setBaseAndExtent(pre, 0, pre, 2);");
        assert_eq!(val(vm, "s.anchorNode === pre"), "true");
        assert_eq!(val(vm, "s.anchorOffset"), "0");
        assert_eq!(val(vm, "s.focusNode === pre"), "true");
        assert_eq!(val(vm, "s.focusOffset"), "2");
        assert_eq!(val(vm, "s.rangeCount"), "1");
        assert_eq!(val(vm, "s.type"), "Range");
    });
}

// ---------------------------------------------------------------------------
// addRange over a selection outside the document tree (PM 901)
// ---------------------------------------------------------------------------

#[test]
fn add_range_replaces_selection_outside_document_tree() {
    with_tree(|vm| {
        run(
            vm,
            "s.setBaseAndExtent(st, 0, st, 3);\
             globalThis.r = document.createRange(); r.setStart(pre, 0); r.setEnd(pre, 2);",
        );
        let (events, len) = (count(vm), registry_len(vm));
        run(vm, "s.addRange(r);");
        assert_eq!(val(vm, "s.getRangeAt(0) === r"), "true");
        assert_eq!(val(vm, "s.rangeCount"), "1");
        assert_eq!(val(vm, "s.anchorNode === pre"), "true");
        assert_eq!(count(vm), events + 1.0, "one selectionchange");
        assert_eq!(
            registry_len(vm),
            len - 1,
            "the displaced range had no wrapper, so it is freed"
        );
    });
    // Control: a selection in the document tree still blocks addRange().
    with_tree(|vm| {
        run(
            vm,
            "s.setBaseAndExtent(post, 0, post, 2);\
             globalThis.r = document.createRange(); r.setStart(pre, 0); r.setEnd(pre, 2);",
        );
        let (events, len) = (count(vm), registry_len(vm));
        run(vm, "s.addRange(r);");
        assert_eq!(val(vm, "s.getRangeAt(0) !== r"), "true");
        assert_eq!(count(vm), events);
        assert_eq!(registry_len(vm), len);
    });
}

// ---------------------------------------------------------------------------
// The third write source (§0.1 item 8): Range methods on the held wrapper
// ---------------------------------------------------------------------------

#[test]
fn range_method_moves_selection_out_of_document_tree() {
    for (dest, data) in [("st", "SHADOW"), ("dt", "detached"), ("ft", "foreign")] {
        with_tree(|vm| {
            run(
                vm,
                &format!(
                    "s.setBaseAndExtent(pre, 0, pre, 2); globalThis.r = s.getRangeAt(0); r.setStart({dest}, 0);"
                ),
            );
            assert_eq!(val(vm, "r.collapsed"), "true", "{dest}");
            assert_eq!(val(vm, &format!("r.startContainer === {dest}")), "true");
            assert_attributes_outside(vm, dest);
            assert_eq!(val(vm, "s.isCollapsed"), "true", "{dest}");
            run(vm, "s.deleteFromDocument();");
            assert_eq!(val(vm, &format!("{dest}.data")), data, "{dest}");
            if dest == "ft" {
                assert_eq!(val(vm, "s.containsNode(ft)"), "false");
            }
        });
    }
    // (d) A non-collapsed destination: deleteContents() would remove text.
    with_tree(|vm| {
        run(
            vm,
            "s.setBaseAndExtent(pre, 0, pre, 2); globalThis.r = s.getRangeAt(0);\
             r.setStart(st, 0); r.setEnd(st, 3);",
        );
        assert_eq!(val(vm, "r.collapsed"), "false");
        assert_eq!(val(vm, "s.isCollapsed"), "false");
        assert_eq!(val(vm, "s.toString()"), "SHA");
        assert_eq!(val(vm, "s.rangeCount"), "0");
        assert_eq!(val(vm, "s.type"), "None");
        let before = count(vm);
        run(vm, "s.deleteFromDocument();");
        assert_eq!(val(vm, "st.data"), "SHADOW");
        assert_eq!(
            count(vm),
            before,
            "the do-nothing branch schedules no event"
        );
    });
}

#[test]
fn add_range_displacing_held_wrapper_keeps_it_live() {
    for dest in ["st", "dt", "ft"] {
        with_tree(|vm| {
            run(
                vm,
                &format!(
                    "s.setBaseAndExtent(pre, 0, pre, 2); globalThis.r0 = s.getRangeAt(0); r0.setStart({dest}, 0);\
                     globalThis.r = document.createRange(); r.setStart(post, 0); r.setEnd(post, 2);"
                ),
            );
            let (events, len) = (count(vm), registry_len(vm));
            run(vm, "s.addRange(r);");
            assert_eq!(val(vm, "s.getRangeAt(0) === r"), "true", "{dest}");
            assert_eq!(val(vm, "s.rangeCount"), "1", "{dest}");
            assert_eq!(count(vm), events + 1.0, "{dest}: one selectionchange");
            assert_eq!(
                registry_len(vm),
                len,
                "{dest}: the held wrapper keeps the displaced range registered"
            );
            assert_eq!(thrown(vm, "r0.startContainer;"), "none", "{dest}");
            assert_eq!(val(vm, &format!("r0.startContainer === {dest}")), "true");
            assert_eq!(val(vm, "r0.collapsed"), "true", "{dest}");
        });
    }
}
