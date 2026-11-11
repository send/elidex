//! C1 unit cell U14 (`docs/plans/2026-10-eh-attrs-c1-template-owner-document.md`
//! §6.2, D-C6): a deep clone of a template creates every node of the copy's
//! contents in the copy's contents' node document (HTML §4.12.3 template
//! *cloning steps* step 2; DOM §4.4 *clone a node* steps 2, 5, 6.7), so no
//! node is created in one document and adopted into another: the clone runs
//! no adopt walk and fires no `MutationEvent::Adopt`.
//!
//! A guard (green on the base and on C1's head). Its kill of the two clone
//! mutants of §6.3 (pass 3's contents children, pass 4's shadow children
//! cloned with the operation-wide document) is recorded in C1's PR.

use std::sync::{Arc, Mutex};

use super::*;
use crate::node_methods::clone::clone_node_with_shadow_honor;
use elidex_ecs::{MutationDispatcher, MutationEvent, ShadowInit};

/// Counts `MutationEvent::Adopt`.
struct AdoptCount(Arc<Mutex<usize>>);

impl MutationDispatcher for AdoptCount {
    fn dispatch(&mut self, event: &MutationEvent<'_>, _dom: &mut EcsDom) {
        if matches!(event, MutationEvent::Adopt { .. }) {
            *self.0.lock().unwrap() += 1;
        }
    }
}

fn el_in(dom: &mut EcsDom, tag: &str, document: Entity) -> Entity {
    let owner = dom.node_document_for_new_nodes(document);
    dom.create_element_with_owner(tag, Attributes::default(), owner)
}

fn template_in(dom: &mut EcsDom, document: Entity) -> (Entity, Entity) {
    let tpl = el_in(dom, "template", document);
    let owner = dom.node_document_for_new_nodes(tpl);
    let contents = dom.attach_template_contents(tpl, owner);
    (tpl, contents)
}

#[test]
fn u14_template_clone_runs_no_adopt() {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    // tpl's contents: a nested template (with a child in its contents) and a
    // clonable shadow host (with a shadow child).
    let (tpl, contents) = template_in(&mut dom, doc);
    let (ntpl, nf) = template_in(&mut dom, doc);
    assert!(dom.append_child(contents, ntpl));
    let para = el_in(&mut dom, "p", doc);
    assert!(dom.append_child(nf, para));
    let host = el_in(&mut dom, "div", doc);
    assert!(dom.append_child(contents, host));
    let sr = dom
        .attach_shadow_with_init(
            host,
            ShadowInit {
                clonable: true,
                ..ShadowInit::default()
            },
        )
        .expect("div is a valid shadow host");
    let italic = el_in(&mut dom, "i", doc);
    assert!(dom.append_child(sr, italic));

    let adopts = Arc::new(Mutex::new(0));
    let _ = dom.set_mutation_dispatcher(Box::new(AdoptCount(Arc::clone(&adopts))));
    let walks = dom.adopt_walk_count();
    let copy = clone_node_with_shadow_honor(tpl, &mut dom, true).expect("tpl exists");

    // The copy's nested contents and shadow tree exist.
    let cf = dom.template_contents_fragment(copy).expect("copy contents");
    let copied = dom.children(cf);
    let cntpl = *copied.first().expect("nested template copied");
    let cnf = dom
        .template_contents_fragment(cntpl)
        .expect("nested contents");
    assert_eq!(dom.children(cnf).len(), 1, "nested contents child copied");
    let chost = *copied.last().expect("shadow host copied");
    let csr = dom.get_shadow_root(chost).expect("shadow root replicated");
    assert_eq!(dom.children(csr).len(), 1, "shadow child copied");

    assert_eq!(*adopts.lock().unwrap(), 0, "no Adopt during the clone");
    assert_eq!(
        dom.adopt_walk_count(),
        walks,
        "no adopt walk during the clone"
    );

    // Positive control: a node of another document appended into the copy's
    // contents is adopted (one walk, one event).
    let x = dom.create_document_node();
    let stranger = el_in(&mut dom, "b", x);
    assert!(dom.append_child(cf, stranger));
    assert_eq!(*adopts.lock().unwrap(), 1, "control: one Adopt");
    assert_eq!(dom.adopt_walk_count(), walks + 1, "control: one walk");
}
