//! C0b (`docs/plans/2026-11-eh-attrs-c0b-node-document-fact.md` §6.2): DOM
//! *insert* step 7.1 adopts the node into the parent's node document (DOM
//! §4.2.3 *insert*; DOM §4.5 *adopt* steps 1–3).
//!
//! Red-first (§6.5): every cell is written against the base API —
//! `get_associated_document` (the raw stamp `node_document` will read) stands
//! for `node_document`. Red cells are `#[ignore]`d with their base failure in
//! the reason; each has a green `_control` sibling. The cells that name a NEW
//! API with no base formulation (U5, U9, U10: the walk counter / `adopt`) are
//! added with that API.

use std::sync::{Arc, Mutex};

use super::*;
use crate::components::AttrEntityCache;
use crate::dom::MutationDispatcher;
use crate::dom::MAX_ANCESTOR_DEPTH;

/// `D` (the receiving document) with a `<div>` parent in its tree, and a
/// second document `X`.
struct Docs {
    d: Entity,
    d_parent: Entity,
    x: Entity,
}

fn docs(dom: &mut EcsDom) -> Docs {
    let d = dom.create_document_root();
    let d_parent = dom.create_element_with_owner(
        "div",
        Attributes::default(),
        dom.node_document_for_new_nodes(d),
    );
    assert!(dom.append_child(d, d_parent));
    let x = dom.create_document_node();
    Docs { d, d_parent, x }
}

/// The U1 subtree, every node stamped `X`: `n` with a child, a shadow root
/// with content, a `<template>` with contents (and a contents child), and one
/// Attr entity in `n`'s `AttrEntityCache` (the state `GetAttributeNode`'s
/// materialization leaves). Returns `n` and every node the walk must visit.
fn x_subtree(dom: &mut EcsDom, x: Entity) -> (Entity, Vec<Entity>) {
    let n = dom.create_element_with_owner(
        "div",
        Attributes::default(),
        dom.node_document_for_new_nodes(x),
    );
    let child = dom.create_element_with_owner(
        "span",
        Attributes::default(),
        dom.node_document_for_new_nodes(x),
    );
    assert!(dom.append_child(n, child));
    let sr = dom
        .attach_shadow(n, ShadowRootMode::Open)
        .expect("div is a valid shadow host");
    // The base spawns the shadow root unstamped (`shadow.rs:189`); stamp it
    // `X` so the cell starts from "every node in X".
    assert!(dom.set_associated_document(sr, x));
    let sr_content = dom.create_element_with_owner(
        "b",
        Attributes::default(),
        dom.node_document_for_new_nodes(x),
    );
    assert!(dom.append_child(sr, sr_content));
    let tpl = dom.create_element_with_owner(
        "template",
        Attributes::default(),
        dom.node_document_for_new_nodes(x),
    );
    assert!(dom.append_child(n, tpl));
    let contents = dom.attach_template_contents(tpl, dom.node_document_for_new_nodes(x));
    let contents_child = dom.create_element_with_owner(
        "i",
        Attributes::default(),
        dom.node_document_for_new_nodes(x),
    );
    assert!(dom.append_child(contents, contents_child));
    let attr = dom.create_attribute("title", NewNodeOwner::NONE);
    assert!(dom.set_associated_document(attr, x));
    let mut cache = AttrEntityCache::default();
    cache.entries.insert("title".to_string(), attr);
    assert!(dom.world_mut().insert_one(n, cache).is_ok());
    let visited = vec![
        n,
        child,
        sr,
        sr_content,
        tpl,
        contents,
        contents_child,
        attr,
    ];
    (n, visited)
}

/// The four U1 links: `append_child`, `insert_before`, `replace_child` under
/// `D_parent`, and an append into a contents fragment stamped `D`.
fn u1_link(dom: &mut EcsDom, docs: &Docs, variant: usize, n: Entity) {
    match variant {
        0 => assert!(dom.append_child(docs.d_parent, n)),
        1 => {
            let r = dom.create_element_with_owner(
                "p",
                Attributes::default(),
                dom.node_document_for_new_nodes(docs.d),
            );
            assert!(dom.append_child(docs.d_parent, r));
            assert!(dom.insert_before(docs.d_parent, n, r));
        }
        2 => {
            let old = dom.create_element_with_owner(
                "p",
                Attributes::default(),
                dom.node_document_for_new_nodes(docs.d),
            );
            assert!(dom.append_child(docs.d_parent, old));
            assert!(dom.replace_child(docs.d_parent, n, old));
        }
        _ => {
            let tpl = dom.create_element_with_owner(
                "template",
                Attributes::default(),
                dom.node_document_for_new_nodes(docs.d),
            );
            assert!(dom.append_child(docs.d_parent, tpl));
            let contents =
                dom.attach_template_contents(tpl, dom.node_document_for_new_nodes(docs.d));
            assert!(dom.append_child(contents, n));
        }
    }
}

// ---- U1 (red): every visited node, the contents and the cached Attr entity
// are stamped `D` after the link (adopt 3.1, 3.3.1, 3.4) ----
#[test]
#[ignore = "C0b red-first: the base links without adopting — every node keeps X"]
fn u1_insertion_adopts_subtree_shadow_contents_and_attr() {
    // Per variant (append, insert_before, replace_child, into D-contents):
    // whether each of n, child, shadow root, its content, template, contents,
    // contents child, Attr reads D.
    let mut got = Vec::new();
    for variant in 0..4 {
        let mut dom = EcsDom::new();
        let docs = docs(&mut dom);
        let (n, visited) = x_subtree(&mut dom, docs.x);
        u1_link(&mut dom, &docs, variant, n);
        let reads: Vec<bool> = visited
            .iter()
            .map(|&node| dom.get_associated_document(node) == Some(docs.d))
            .collect();
        got.push(reads);
    }
    assert_eq!(got, vec![vec![true; 8]; 4]);
}

#[test]
fn u1_control_every_node_reads_x_before_the_link() {
    for variant in 0..4 {
        let mut dom = EcsDom::new();
        let docs = docs(&mut dom);
        let (_n, visited) = x_subtree(&mut dom, docs.x);
        for node in visited {
            assert_eq!(
                dom.get_associated_document(node),
                Some(docs.x),
                "variant {variant}: {node:?} starts in X"
            );
        }
    }
}

// ---- U2 (red): an ownerless node appended under `D` is stamped `D` (the
// silent re-home of a `None` oldDocument) ----
#[test]
#[ignore = "C0b red-first: the base leaves the ownerless node unstamped (None)"]
fn u2_ownerless_node_appended_under_d_is_stamped_d() {
    let mut dom = EcsDom::new();
    let docs = docs(&mut dom);
    let n = elem(&mut dom, "div");
    assert!(dom.append_child(docs.d_parent, n));
    assert_eq!(dom.get_associated_document(n), Some(docs.d));
}

#[test]
fn u2_control_a_stamped_node_reads_its_stamp() {
    let mut dom = EcsDom::new();
    let docs = docs(&mut dom);
    let m = elem(&mut dom, "div");
    assert!(dom.set_associated_document(m, docs.d));
    assert_eq!(dom.get_associated_document(m), Some(docs.d));
}

// ---- U3 (guard): a node stamped `D` appended under an ownerless parent keeps
// `D` (no target document → no adopt) ----
#[test]
fn u3_append_under_ownerless_parent_keeps_the_stamp() {
    let mut dom = EcsDom::new();
    let docs = docs(&mut dom);
    let n = dom.create_element_with_owner(
        "div",
        Attributes::default(),
        dom.node_document_for_new_nodes(docs.d),
    );
    let ownerless_parent = elem(&mut dom, "section");
    assert!(dom.append_child(ownerless_parent, n));
    assert_eq!(dom.get_associated_document(n), Some(docs.d));
}

// ---- U4 (red): the stamp is written before `Insert` fires, and in
// `replace_child` `new_child`'s walk precedes `old_child`'s removal ----

#[derive(Debug, Clone, PartialEq, Eq)]
enum Rec {
    Remove { node: Entity, parent: Entity },
    Insert { node: Entity, parent: Entity },
}

/// Records `Remove` / `Insert`; the `Insert` arm reads the inserted node's
/// stamp, the `Remove` arm reads `probe`'s stamp when `node == probe_on`.
struct Recorder {
    events: Arc<Mutex<Vec<Rec>>>,
    insert_reads: Arc<Mutex<Vec<Option<Entity>>>>,
    remove_reads: Arc<Mutex<Vec<Option<Entity>>>>,
    probe_on: Option<Entity>,
    probe: Option<Entity>,
}

impl MutationDispatcher for Recorder {
    fn dispatch(&mut self, event: &MutationEvent<'_>, dom: &mut EcsDom) {
        match *event {
            MutationEvent::Insert { node, parent, .. } => {
                self.events
                    .lock()
                    .unwrap()
                    .push(Rec::Insert { node, parent });
                self.insert_reads
                    .lock()
                    .unwrap()
                    .push(dom.get_associated_document(node));
            }
            MutationEvent::Remove { node, parent, .. } => {
                self.events
                    .lock()
                    .unwrap()
                    .push(Rec::Remove { node, parent });
                if Some(node) == self.probe_on {
                    if let Some(p) = self.probe {
                        self.remove_reads
                            .lock()
                            .unwrap()
                            .push(dom.get_associated_document(p));
                    }
                }
            }
            _ => {}
        }
    }
}

struct U4Run {
    events: Vec<Rec>,
    insert_reads: Vec<Option<Entity>>,
    remove_reads: Vec<Option<Entity>>,
    d: Entity,
    d_parent: Entity,
    x_parent: Entity,
    n: Entity,
    old: Option<Entity>,
}

fn take<T>(m: &Arc<Mutex<Vec<T>>>) -> Vec<T> {
    std::mem::take(&mut *m.lock().unwrap())
}

/// An `X`-stamped node `n` under an `X` parent, moved under `D_parent` by
/// `append_child` (`replace = false`) or by `replace_child` over `old`.
fn u4_run(replace: bool) -> U4Run {
    let mut dom = EcsDom::new();
    let docs = docs(&mut dom);
    let x_parent = dom.create_element_with_owner(
        "div",
        Attributes::default(),
        dom.node_document_for_new_nodes(docs.x),
    );
    let n = dom.create_element_with_owner(
        "span",
        Attributes::default(),
        dom.node_document_for_new_nodes(docs.x),
    );
    assert!(dom.append_child(x_parent, n));
    let old = replace.then(|| {
        let old = dom.create_element_with_owner(
            "p",
            Attributes::default(),
            dom.node_document_for_new_nodes(docs.d),
        );
        assert!(dom.append_child(docs.d_parent, old));
        old
    });
    let events = Arc::new(Mutex::new(Vec::new()));
    let insert_reads = Arc::new(Mutex::new(Vec::new()));
    let remove_reads = Arc::new(Mutex::new(Vec::new()));
    let _ = dom.set_mutation_dispatcher(Box::new(Recorder {
        events: Arc::clone(&events),
        insert_reads: Arc::clone(&insert_reads),
        remove_reads: Arc::clone(&remove_reads),
        probe_on: old,
        probe: Some(n),
    }));
    match old {
        Some(old) => assert!(dom.replace_child(docs.d_parent, n, old)),
        None => assert!(dom.append_child(docs.d_parent, n)),
    }
    U4Run {
        events: take(&events),
        insert_reads: take(&insert_reads),
        remove_reads: take(&remove_reads),
        d: docs.d,
        d_parent: docs.d_parent,
        x_parent,
        n,
        old,
    }
}

#[test]
#[ignore = "C0b red-first: the base never adopts — the Insert arm (and replace's Remove(old_child) arm) read X"]
fn u4_stamp_is_written_before_insert_and_before_old_child_removal() {
    let r = u4_run(false);
    assert_eq!(
        r.insert_reads,
        vec![Some(r.d)],
        "append: Insert arm reads D"
    );
    let r = u4_run(true);
    assert_eq!(
        r.insert_reads,
        vec![Some(r.d)],
        "replace: Insert arm reads D"
    );
    assert_eq!(
        r.remove_reads,
        vec![Some(r.d)],
        "replace: new_child's walk precedes old_child's removal (D-4)"
    );
}

#[test]
fn u4_control_event_sequence() {
    let r = u4_run(false);
    assert_eq!(
        r.events,
        vec![
            Rec::Remove {
                node: r.n,
                parent: r.x_parent
            },
            Rec::Insert {
                node: r.n,
                parent: r.d_parent
            },
        ]
    );
    let r = u4_run(true);
    let old = r.old.expect("replace run");
    assert_eq!(
        r.events,
        vec![
            Rec::Remove {
                node: r.n,
                parent: r.x_parent
            },
            Rec::Remove {
                node: old,
                parent: r.d_parent
            },
            Rec::Insert {
                node: r.n,
                parent: r.d_parent
            },
        ]
    );
}

// ---- U6 (red): the walk is uncapped — an X subtree past the depth and the
// breadth caps (the shapes of `destroy.rs` `despawn_subtree_tears_down_past_*`)
// is re-homed in full ----

/// Returns (top, deepest, last): a chain `MAX_ANCESTOR_DEPTH + 5` deep under
/// `top`, and `MAX_ANCESTOR_DEPTH + 5` siblings under `top`.
fn u6_x_subtree(dom: &mut EcsDom, x: Entity) -> (Entity, Entity, Entity) {
    let mk = |dom: &mut EcsDom| {
        dom.create_element_with_owner(
            "div",
            Attributes::default(),
            dom.node_document_for_new_nodes(x),
        )
    };
    let deepest = mk(dom);
    let mut chain = deepest;
    for _ in 0..(MAX_ANCESTOR_DEPTH + 5) {
        let parent = mk(dom);
        assert!(dom.append_child(parent, chain));
        chain = parent;
    }
    let top = mk(dom);
    assert!(dom.append_child(top, chain));
    let mut last = chain;
    for _ in 0..(MAX_ANCESTOR_DEPTH + 5) {
        last = mk(dom);
        assert!(dom.append_child(top, last));
    }
    (top, deepest, last)
}

#[test]
#[ignore = "C0b red-first: the base never adopts — the deepest and the last child keep X"]
fn u6_adopt_walk_reaches_past_depth_and_breadth_caps() {
    let mut dom = EcsDom::new();
    let docs = docs(&mut dom);
    let (top, deepest, last) = u6_x_subtree(&mut dom, docs.x);
    assert!(dom.append_child(docs.d_parent, top));
    assert_eq!(dom.get_associated_document(deepest), Some(docs.d));
    assert_eq!(dom.get_associated_document(last), Some(docs.d));
}

#[test]
fn u6_control_reads_before_the_append() {
    let mut dom = EcsDom::new();
    let docs = docs(&mut dom);
    let (_top, deepest, last) = u6_x_subtree(&mut dom, docs.x);
    assert_eq!(dom.get_associated_document(deepest), Some(docs.x));
    assert_eq!(dom.get_associated_document(last), Some(docs.x));
}

// ---- U7 (red): the reader has no fallback ----
#[test]
#[ignore = "C0b red-first: the base has no Document self-stamp (None) and owner_document falls back to the tree root / fragment root (Some)"]
fn u7_node_document_has_no_fallback() {
    let mut dom = EcsDom::new();
    let docs = docs(&mut dom);
    // Leg 1: a Document is its own node document (base: `get_associated_document`).
    let leg1 = dom.get_associated_document(docs.d);
    // Leg 3: a node whose document was destroyed reads None, even in D's tree.
    let gone = dom.create_document_node();
    let orphaned = dom.create_element_with_owner(
        "i",
        Attributes::default(),
        dom.node_document_for_new_nodes(gone),
    );
    assert!(dom.destroy_entity(gone));
    assert!(dom.append_child(docs.d_parent, orphaned));
    let leg3 = dom.owner_document(orphaned);
    // Leg 4: an unstamped node in a Document's tree reads None.
    let unstamped = elem(&mut dom, "u");
    assert!(dom.append_child(docs.d_parent, unstamped));
    let leg4 = dom.owner_document(unstamped);
    // Leg 5: an unstamped node under a fragment stamped D reads None.
    let frag = dom.create_document_fragment_with_owner(dom.node_document_for_new_nodes(docs.d));
    let in_frag = elem(&mut dom, "s");
    assert!(dom.append_child(frag, in_frag));
    let leg5 = dom.owner_document(in_frag);
    assert_eq!(
        [leg1, leg3, leg4, leg5],
        [Some(docs.d), None, None, None],
        "legs 1 (self), 3 (destroyed document), 4 (tree-root fallback), 5 (fragment-root fallback)"
    );
}

#[test]
fn u7_control_a_document_has_no_owner_document() {
    let mut dom = EcsDom::new();
    let docs = docs(&mut dom);
    assert_eq!(dom.owner_document(docs.d), None, "leg 2");
}

// ---- U11 (red): `node_document_for_new_nodes` — a Document → itself; a node
// → its stamp; the fragment-parse root → the context's document (§13.4 step
// 15's redirect); the throwaway → the throwaway (root is created in it, step
// 11); a documentless context → `None`. The legs of nodes created under root
// during a build are the parser crates' (U8 / U9) ----
#[test]
fn u11_fragment_root_is_created_in_the_throwaway() {
    let mut dom = EcsDom::new();
    let docs = docs(&mut dom);
    assert_eq!(
        dom.node_document_for_new_nodes(docs.d).document(),
        Some(docs.d)
    );
    assert_eq!(
        dom.node_document_for_new_nodes(docs.d_parent).document(),
        Some(docs.d)
    );
    let (throwaway, root, saved) = dom.begin_detached_fragment(docs.d_parent);
    let root_stamp = dom.get_associated_document(root);
    let root_entry = dom.node_document_for_new_nodes(root).document();
    let throwaway_entry = dom.node_document_for_new_nodes(throwaway).document();
    let _ = dom.finish_detached_fragment(root, throwaway);
    if let Some(saved) = saved {
        let _ = dom.set_mutation_dispatcher(saved);
    }
    assert_eq!(
        root_stamp,
        Some(throwaway),
        "root is created in the throwaway"
    );
    assert_eq!(
        root_entry,
        Some(docs.d),
        "root redirects to the context's document"
    );
    assert_eq!(throwaway_entry, Some(throwaway));

    let documentless = dom.create_element("div", Attributes::default());
    let (throwaway, root, saved) = dom.begin_detached_fragment(documentless);
    let root_entry = dom.node_document_for_new_nodes(root).document();
    let _ = dom.finish_detached_fragment(root, throwaway);
    if let Some(saved) = saved {
        let _ = dom.set_mutation_dispatcher(saved);
    }
    assert_eq!(root_entry, None, "documentless context");
}

#[test]
fn u11_control_the_throwaway_is_a_document() {
    let mut dom = EcsDom::new();
    let docs = docs(&mut dom);
    let (throwaway, root, saved) = dom.begin_detached_fragment(docs.d_parent);
    let kind = dom.node_kind(throwaway);
    let _ = dom.finish_detached_fragment(root, throwaway);
    if let Some(saved) = saved {
        let _ = dom.set_mutation_dispatcher(saved);
    }
    assert_eq!(kind, Some(NodeKind::Document));
}
