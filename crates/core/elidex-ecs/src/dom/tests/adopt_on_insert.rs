//! C0b (`docs/plans/2026-11-eh-attrs-c0b-node-document-fact.md` §6.2): DOM
//! *insert* step 7.1 adopts the node into the parent's node document (DOM
//! §4.2.3 *insert*; DOM §4.5 *adopt* steps 1–3).
//!
//! Red-first (§6.5): the red cells were run on the base in their base
//! formulation (`get_associated_document` standing for `node_document`), each
//! with a green `_control` sibling, and are green at C0b's head. The guards
//! (U3, U5, U9, U10) name C0b's API (the walk counter, `adopt`) and have no
//! base run.

use std::sync::{Arc, Mutex};

use super::*;
use crate::components::{AssociatedDocument, AttrEntityCache, FragmentParseRoot};
use crate::dom::MAX_ANCESTOR_DEPTH;
use crate::dom::{AdoptedContents, MutationDispatcher};

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
    // The shadow root is created in its host's node document (DOM §4.9
    // *attach a shadow root* step 5: created "given element's node document").
    assert_eq!(dom.node_document(sr), Some(x));
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
    let attr = dom.create_attribute("title", dom.node_document_for_new_nodes(x));
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
            .map(|&node| dom.node_document(node) == Some(docs.d))
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
                dom.node_document(node),
                Some(docs.x),
                "variant {variant}: {node:?} starts in X"
            );
        }
    }
}

// ---- U2 (red): an ownerless node appended under `D` is stamped `D` (the
// silent re-home of a `None` oldDocument) ----
#[test]
fn u2_ownerless_node_appended_under_d_is_stamped_d() {
    let mut dom = EcsDom::new();
    let docs = docs(&mut dom);
    let n = elem(&mut dom, "div");
    assert!(dom.append_child(docs.d_parent, n));
    assert_eq!(dom.node_document(n), Some(docs.d));
}

#[test]
fn u2_control_a_stamped_node_reads_its_stamp() {
    let mut dom = EcsDom::new();
    let docs = docs(&mut dom);
    let m = elem(&mut dom, "div");
    assert!(dom.set_associated_document(m, docs.d));
    assert_eq!(dom.node_document(m), Some(docs.d));
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
    assert_eq!(dom.node_document(n), Some(docs.d));
}

// ---- U4 (red): the stamp is written before `Insert` fires, and in
// `replace_child` `new_child`'s walk precedes `old_child`'s removal ----

#[derive(Debug, Clone, PartialEq, Eq)]
enum Rec {
    Remove { node: Entity, parent: Entity },
    Adopt { node: Entity },
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
                    .push(dom.node_document(node));
            }
            MutationEvent::Remove { node, parent, .. } => {
                self.events
                    .lock()
                    .unwrap()
                    .push(Rec::Remove { node, parent });
                if Some(node) == self.probe_on {
                    if let Some(p) = self.probe {
                        self.remove_reads.lock().unwrap().push(dom.node_document(p));
                    }
                }
            }
            MutationEvent::Adopt { node, .. } => {
                self.events.lock().unwrap().push(Rec::Adopt { node });
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

// C0c U4 (red; A2×A4): `Adopt` comes between the old parent's `Remove` and
// the `Insert`; in `replace_child`, `Adopt(new_child)` precedes
// `Remove(old_child)` (DOM §4.2.3 *replace* steps 6–7). Base proxy (C0b's
// head, no `Adopt` variant): `Remove`, then `Insert`, nothing between.
#[test]
fn u4_event_sequence_adopt_between_remove_and_insert() {
    let r = u4_run(false);
    assert_eq!(
        r.events,
        vec![
            Rec::Remove {
                node: r.n,
                parent: r.x_parent
            },
            Rec::Adopt { node: r.n },
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
            Rec::Adopt { node: r.n },
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
fn u6_adopt_walk_reaches_past_depth_and_breadth_caps() {
    let mut dom = EcsDom::new();
    let docs = docs(&mut dom);
    let (top, deepest, last) = u6_x_subtree(&mut dom, docs.x);
    assert!(dom.append_child(docs.d_parent, top));
    assert_eq!(dom.node_document(deepest), Some(docs.d));
    assert_eq!(dom.node_document(last), Some(docs.d));
}

#[test]
fn u6_control_reads_before_the_append() {
    let mut dom = EcsDom::new();
    let docs = docs(&mut dom);
    let (_top, deepest, last) = u6_x_subtree(&mut dom, docs.x);
    assert_eq!(dom.node_document(deepest), Some(docs.x));
    assert_eq!(dom.node_document(last), Some(docs.x));
}

// ---- U7 (red): the reader has no fallback ----
//
// Legs 3–5 are states the primitives no longer produce (every insertion
// adopts: a node created in `gone` and appended under `D_parent` reads `D`,
// and so does an ownerless one, U2), so they are constructed crate-internally:
// the stamp is written / removed after the link. The reader must still read
// `None` there, not derive a document from the position.
#[test]
fn u7_node_document_has_no_fallback() {
    let mut dom = EcsDom::new();
    let docs = docs(&mut dom);
    // Leg 1: a Document is its own node document.
    let leg1 = dom.node_document(docs.d);
    // Leg 3: a node in D's tree whose stamped document was destroyed reads
    // None (no tree-root fallback past a dangling stamp).
    let gone = dom.create_document_node();
    let orphaned = dom.create_element_with_owner(
        "i",
        Attributes::default(),
        dom.node_document_for_new_nodes(docs.d),
    );
    assert!(dom.append_child(docs.d_parent, orphaned));
    assert!(dom.set_associated_document(orphaned, gone));
    assert!(dom.destroy_entity(gone));
    let leg3 = dom.node_document(orphaned);
    // Leg 4: an unstamped node in a Document's tree reads None.
    let unstamped = elem(&mut dom, "u");
    assert!(dom.append_child(docs.d_parent, unstamped));
    assert!(dom
        .world_mut()
        .remove_one::<AssociatedDocument>(unstamped)
        .is_ok());
    let leg4 = dom.node_document(unstamped);
    // Leg 5: an unstamped node under a fragment stamped D reads None.
    let frag = dom.create_document_fragment_with_owner(dom.node_document_for_new_nodes(docs.d));
    let in_frag = elem(&mut dom, "s");
    assert!(dom.append_child(frag, in_frag));
    assert!(dom
        .world_mut()
        .remove_one::<AssociatedDocument>(in_frag)
        .is_ok());
    let leg5 = dom.node_document(in_frag);
    assert_eq!(
        [leg1, leg3, leg4, leg5],
        [Some(docs.d), None, None, None],
        "legs 1 (self), 3 (destroyed document), 4 (tree-root fallback), 5 (fragment-root fallback)"
    );
    assert_eq!(
        [
            dom.owner_document(orphaned),
            dom.owner_document(unstamped),
            dom.owner_document(in_frag),
        ],
        [None, None, None],
        "legs 3, 4, 5 via owner_document"
    );
}

#[test]
fn u7_control_a_document_has_no_owner_document() {
    let mut dom = EcsDom::new();
    let docs = docs(&mut dom);
    assert_eq!(dom.owner_document(docs.d), None, "leg 2");
}

// ---- U11 (red): `node_document_for_new_nodes` — a Document → itself; a node
// → its stamp; the fragment-parse root → the context's document (*initialize
// an HTML fragment parser* step 15's redirect); the throwaway → the throwaway
// (root is created in it, step 12); a documentless context → `None`. The legs of nodes created under root
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
    let root_stamp = dom.node_document(root);
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

    let documentless = dom.create_element_ownerless_fixture("div", Attributes::default());
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

// ---- U5 (guard): a same-document move leaves every stamp as it is and runs
// no walk (adopt step 3 runs only when document is not oldDocument) ----
#[test]
fn u5_same_document_move_runs_no_walk() {
    let mut dom = EcsDom::new();
    let docs = docs(&mut dom);
    let (n, visited) = x_subtree(&mut dom, docs.d);
    assert!(dom.append_child(docs.d_parent, n));
    let other_parent = dom.create_element_with_owner(
        "section",
        Attributes::default(),
        dom.node_document_for_new_nodes(docs.d),
    );
    assert!(dom.append_child(docs.d, other_parent));
    let before = dom.adopt_walk_count();
    assert!(dom.append_child(other_parent, n));
    assert_eq!(
        dom.adopt_walk_count(),
        before,
        "no walk on a same-document move"
    );
    for node in visited {
        assert_eq!(dom.node_document(node), Some(docs.d), "{node:?} keeps D");
    }
}

// ---- U9 (guard, the elidex-ecs leg): a fragment build into a context in D
// walks nothing — every node created under root takes D at creation, so every
// build link is a compare — and leaves no `FragmentParseRoot` after either
// teardown (the parser crates' U9 run both tiers) ----
#[test]
fn u9_fragment_build_runs_no_walk_and_leaves_no_parse_root() {
    let fragment_roots = |dom: &EcsDom| dom.world().query::<&FragmentParseRoot>().iter().count();
    for rollback in [false, true] {
        let mut dom = EcsDom::new();
        let docs = docs(&mut dom);
        let before = dom.adopt_walk_count();
        let (throwaway, root, saved) = dom.begin_detached_fragment(docs.d_parent);
        let a = dom.create_element_with_owner(
            "p",
            Attributes::default(),
            dom.node_document_for_new_nodes(root),
        );
        assert!(dom.append_child(root, a));
        let b = dom.create_text_with_owner("t", dom.node_document_for_new_nodes(a));
        assert!(dom.append_child(a, b));
        assert_eq!(fragment_roots(&dom), 1, "the build's root carries it");
        let returned = if rollback {
            // The strict parse-error path's teardown.
            let _ = dom.despawn_subtree(throwaway);
            Vec::new()
        } else {
            dom.finish_detached_fragment(root, throwaway)
        };
        if let Some(saved) = saved {
            let _ = dom.set_mutation_dispatcher(saved);
        }
        assert_eq!(
            dom.adopt_walk_count(),
            before,
            "rollback={rollback}: no walk"
        );
        assert_eq!(fragment_roots(&dom), 0, "rollback={rollback}: root gone");
        if !rollback {
            assert_eq!(returned, vec![a]);
            assert_eq!(dom.node_document(a), Some(docs.d));
            assert_eq!(dom.node_document(b), Some(docs.d));
        }
    }
}

// ---- U10 (guard): `adopt` of a parentless fragment with children stamps the
// fragment and its children (C0c adds the event: `u10_adopt_fires_one_event`) ----
#[test]
fn u10_adopt_parentless_fragment_stamps_it_and_its_children() {
    let mut dom = EcsDom::new();
    let docs = docs(&mut dom);
    let frag = dom.create_document_fragment_with_owner(dom.node_document_for_new_nodes(docs.x));
    let c1 = dom.create_element_with_owner(
        "i",
        Attributes::default(),
        dom.node_document_for_new_nodes(docs.x),
    );
    let c2 = dom.create_text_with_owner("t", dom.node_document_for_new_nodes(docs.x));
    assert!(dom.append_child(frag, c1));
    assert!(dom.append_child(frag, c2));
    let before = dom.adopt_walk_count();
    dom.adopt(frag, docs.d);
    assert_eq!(dom.adopt_walk_count(), before + 1, "one walk");
    assert_eq!(
        [frag, c1, c2].map(|n| dom.node_document(n)),
        [Some(docs.d); 3]
    );
    assert_eq!(dom.children(frag), vec![c1, c2], "the children stay in it");
}

// ===========================================================================
// C0c (`docs/plans/2026-11-eh-attrs-c0c-adopt-notification-adoptnode.md`
// §6.2): the `Adopt` event (D-6) and the walk's contents segments (D-3 (c)).
// U10 is red by absence on the base (proxy: no event from `adopt`); U5, U2′
// and U19 are guards with no base run.
// ===========================================================================

/// One recorded `MutationEvent::Adopt`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct AdoptRec {
    node: Entity,
    documents: Option<(Entity, Entity)>,
    contents: Vec<AdoptedContents>,
}

/// Records every `Adopt` event; ignores the rest.
struct AdoptLog(Arc<Mutex<Vec<AdoptRec>>>);

impl MutationDispatcher for AdoptLog {
    fn dispatch(&mut self, event: &MutationEvent<'_>, _dom: &mut EcsDom) {
        if let MutationEvent::Adopt {
            node,
            documents,
            contents,
        } = *event
        {
            self.0.lock().unwrap().push(AdoptRec {
                node,
                documents,
                contents: contents.to_vec(),
            });
        }
    }
}

fn install_adopt_log(dom: &mut EcsDom) -> Arc<Mutex<Vec<AdoptRec>>> {
    let log = Arc::new(Mutex::new(Vec::new()));
    let _ = dom.set_mutation_dispatcher(Box::new(AdoptLog(Arc::clone(&log))));
    log
}

// ---- U10 (red; C0c part): one `Adopt` from `adopt(fragment, D)`, with the
// root's pair `(X, D)` ----
#[test]
fn u10_adopt_fires_one_event() {
    let mut dom = EcsDom::new();
    let docs = docs(&mut dom);
    let frag = dom.create_document_fragment_with_owner(dom.node_document_for_new_nodes(docs.x));
    let c1 = dom.create_element_with_owner(
        "i",
        Attributes::default(),
        dom.node_document_for_new_nodes(docs.x),
    );
    assert!(dom.append_child(frag, c1));
    let log = install_adopt_log(&mut dom);
    dom.adopt(frag, docs.d);
    assert_eq!(
        take(&log),
        vec![AdoptRec {
            node: frag,
            documents: Some((docs.x, docs.d)),
            contents: Vec::new(),
        }]
    );
}

// ---- U5 (guard; C0c part): a same-document move fires no `Adopt` ----
#[test]
fn u5_same_document_move_fires_no_adopt() {
    let mut dom = EcsDom::new();
    let docs = docs(&mut dom);
    let (n, _) = x_subtree(&mut dom, docs.d);
    assert!(dom.append_child(docs.d_parent, n));
    let log = install_adopt_log(&mut dom);
    let other_parent = dom.create_element_with_owner(
        "section",
        Attributes::default(),
        dom.node_document_for_new_nodes(docs.d),
    );
    assert!(dom.append_child(docs.d, other_parent));
    assert!(dom.append_child(other_parent, n));
    dom.adopt(n, docs.d);
    assert_eq!(take(&log), Vec::new());
}

// ---- U2′ (guard): an ownerless root appended under `D` is re-homed silently:
// stamped `D`, no `Adopt` (adopt step 3's guard has no oldDocument) ----
#[test]
fn u2_prime_ownerless_root_fires_no_adopt() {
    let mut dom = EcsDom::new();
    let docs = docs(&mut dom);
    let n = elem(&mut dom, "div");
    let log = install_adopt_log(&mut dom);
    assert!(dom.append_child(docs.d_parent, n));
    assert_eq!(dom.node_document(n), Some(docs.d));
    assert_eq!(take(&log), Vec::new());
}

/// U19's nested contents: outer template `t0` (in `D0`) whose contents `f1`
/// hold an inner template `t1` whose contents `f2` hold `e`, with `e`'s
/// materialized Attr `a`. `f1_owner` stamps `f1` and `t1`; `f2_owner` stamps
/// `f2`, `e` and `a`.
struct U19 {
    dom: EcsDom,
    target: Entity,
    d0: Entity,
    t0: Entity,
    f1: Entity,
    t1: Entity,
    f2: Entity,
    e: Entity,
    a: Entity,
}

fn u19_fixture(f1_owner: Option<usize>, f2_owner: usize) -> (U19, [Entity; 3]) {
    let mut dom = EcsDom::new();
    let target = dom.create_document_root();
    let d0 = dom.create_document_node();
    let x = dom.create_document_node();
    let y = dom.create_document_node();
    let docs = [target, x, y];
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
    let o1 = new_owner(&mut dom, f1_owner);
    let f1 = dom.attach_template_contents(t0, o1);
    let o1 = new_owner(&mut dom, f1_owner);
    let t1 = dom.create_element_with_owner("template", Attributes::default(), o1);
    assert!(dom.append_child(f1, t1));
    let o2 = new_owner(&mut dom, Some(f2_owner));
    let f2 = dom.attach_template_contents(t1, o2);
    let o2 = new_owner(&mut dom, Some(f2_owner));
    let e = dom.create_element_with_owner("x-el", Attributes::default(), o2);
    assert!(dom.append_child(f2, e));
    let o2 = new_owner(&mut dom, Some(f2_owner));
    let a = dom.create_attribute("id", o2);
    let mut cache = AttrEntityCache::default();
    cache.entries.insert("id".to_string(), a);
    assert!(dom.world_mut().insert_one(e, cache).is_ok());
    (
        U19 {
            dom,
            target,
            d0,
            t0,
            f1,
            t1,
            f2,
            e,
            a,
        },
        docs,
    )
}

fn u19_stamps(f: &U19) -> [Option<Entity>; 6] {
    [f.t0, f.f1, f.t1, f.f2, f.e, f.a].map(|n| f.dom.node_document(n))
}

// ---- U19 (guard; D-3 (c), D-6): nested contents segments ----

/// First leg: `t0` in `D0`, `f1` in `Y`, `f2` in `X` (three distinct
/// documents). Each fragment is its own segment with its own old document;
/// `e`'s innermost segment is `f2`'s, `(X, target)`.
#[test]
fn u19_nested_segments_each_carry_their_own_pair() {
    let (mut f, [target, x, y]) = u19_fixture(Some(2), 1);
    let log = install_adopt_log(&mut f.dom);
    f.dom.adopt(f.t0, target);
    assert_eq!(
        take(&log),
        vec![AdoptRec {
            node: f.t0,
            documents: Some((f.d0, target)),
            contents: vec![
                AdoptedContents {
                    fragment: f.f1,
                    documents: Some((y, target)),
                },
                AdoptedContents {
                    fragment: f.f2,
                    documents: Some((x, target)),
                },
            ],
        }]
    );
    assert_eq!(u19_stamps(&f), [Some(f.target); 6]);
}

/// Second leg: `f1` already in the target, `f2` (with `e` and `a`) in `X`.
/// `f1`'s nested adopt does not run (its old equals new), so neither does
/// `f2`'s: no segment for either, and `f2`, `e` and `a` still read `X` (the
/// skip gates the Attr re-home too).
#[test]
fn u19_skipped_contents_are_left_as_they_are() {
    let (mut f, [target, x, _]) = u19_fixture(Some(0), 1);
    let log = install_adopt_log(&mut f.dom);
    f.dom.adopt(f.t0, target);
    assert_eq!(
        take(&log),
        vec![AdoptRec {
            node: f.t0,
            documents: Some((f.d0, target)),
            contents: Vec::new(),
        }]
    );
    assert_eq!(
        u19_stamps(&f),
        [
            Some(target),
            Some(target),
            Some(target),
            Some(x),
            Some(x),
            Some(x)
        ]
    );
}

/// Third leg (fixture): `f1` ownerless (a silent re-home), `f2` in `X`, not
/// the target. Both segments are recorded with `None` (the inner one because
/// its template lies in a `None` segment), and everything is stamped.
#[test]
fn u19_segment_under_a_none_segment_is_none() {
    let (mut f, [target, _, _]) = u19_fixture(None, 1);
    let log = install_adopt_log(&mut f.dom);
    f.dom.adopt(f.t0, target);
    assert_eq!(
        take(&log),
        vec![AdoptRec {
            node: f.t0,
            documents: Some((f.d0, target)),
            contents: vec![
                AdoptedContents {
                    fragment: f.f1,
                    documents: None,
                },
                AdoptedContents {
                    fragment: f.f2,
                    documents: None,
                },
            ],
        }]
    );
    assert_eq!(u19_stamps(&f), [Some(target); 6]);
}
