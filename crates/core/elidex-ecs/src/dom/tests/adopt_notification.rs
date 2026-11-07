//! C0c notification cells (`docs/plans/2026-11-eh-attrs-c0c-adopt-notification-adoptnode.md`
//! §6.2), split from `adopt_on_insert.rs`: the `Adopt` event and the walk's
//! contents segments.

use std::sync::{Arc, Mutex};

use super::adopt_on_insert::{docs, take, x_subtree};
use super::*;
use crate::components::AttrEntityCache;
use crate::dom::{AdoptedContents, MutationDispatcher};

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

// ---- U22 (guard; D-13): `adopt(attr, D)` re-homes only the Attr. *Adopt*
// step 2 removes nothing (an Attr has no parent); step 3 stamps the Attr, not
// its owner element ----
#[test]
fn u22_adopt_attr_stamps_only_the_attr() {
    let mut dom = EcsDom::new();
    let docs = docs(&mut dom);
    let x_owner = dom.node_document_for_new_nodes(docs.x);
    let el = dom.create_element_with_owner("div", Attributes::default(), x_owner);
    let attr = dom.create_attribute("id", dom.node_document_for_new_nodes(docs.x));
    dom.world_mut()
        .get::<&mut AttrData>(attr)
        .expect("an Attr entity carries AttrData")
        .owner_element = Some(el);
    let log = install_adopt_log(&mut dom);
    dom.adopt(attr, docs.d);
    assert_eq!(dom.node_document(attr), Some(docs.d));
    assert_eq!(
        dom.node_document(el),
        Some(docs.x),
        "owner element unchanged"
    );
    assert_eq!(
        take(&log),
        vec![AdoptRec {
            node: attr,
            documents: Some((docs.x, docs.d)),
            contents: Vec::new(),
        }]
    );
}

// ---- U21 (guard; D-3 (c)): a contents cycle (`tpl` inside its own contents
// `f`, which the pre-insert validator accepts today,
// `#11-template-contents-host-link`) — `adopt(f, D)` terminates and records
// no segment for its own root ----
#[test]
fn u21_contents_cycle_root_is_not_a_segment() {
    let mut dom = EcsDom::new();
    let docs = docs(&mut dom);
    let tpl = dom.create_element_with_owner(
        "template",
        Attributes::default(),
        dom.node_document_for_new_nodes(docs.x),
    );
    let f = dom.attach_template_contents(tpl, dom.node_document_for_new_nodes(docs.x));
    assert!(dom.append_child(f, tpl), "the primitive allows the cycle");
    let log = install_adopt_log(&mut dom);
    dom.adopt(f, docs.d);
    assert_eq!(
        take(&log),
        vec![AdoptRec {
            node: f,
            documents: Some((docs.x, docs.d)),
            contents: Vec::new(),
        }]
    );
    assert_eq!([f, tpl].map(|n| dom.node_document(n)), [Some(docs.d); 2]);
}
