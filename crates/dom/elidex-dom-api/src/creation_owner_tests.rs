//! C0b U12 (`docs/plans/2026-11-eh-attrs-c0b-node-document-fact.md` §6.2):
//! every node a dom-api algorithm creates on a non-bound ("DOMParser")
//! document's tree is created in that document — DOM §4.4: the node document
//! is set when the node is created, from the node the algorithm names. A
//! value check over the creating handlers, **not** the creation census (D-17's
//! compile guarantee is).
//!
//! Red-first base formulation (§6.5): `get_associated_document` read `None`
//! for `createAttribute`, the table helpers, `cloneNode(false)`, `append('s')`
//! and Range extract (`splitText` and `textContent` were already stamped).
//! Control: `createElement` on the DOMParser document reads it.

use elidex_ecs::{Attributes, EcsDom, Entity};
use elidex_plugin::JsValue;
use elidex_script_session::{DomApiHandler, SessionCore};

use crate::char_data::{CreateAttribute, SplitText};
use crate::child_node::Append;
use crate::document::CreateElement;
use crate::element::table_mutation::{
    RowInsertCell, SectionInsertRow, TableCreateCaption, TableCreateTHead, TableInsertRow,
};
use crate::node_methods::{CloneNode, SetTextContentNodeKind};
use crate::range::Range;
use crate::test_util::entity_of;

/// A non-bound document `dp` (no `create_document_root`: the DOMParser
/// shape) with a `<body>`, and an element of `dp` under it.
fn dp_setup() -> (EcsDom, SessionCore, Entity, Entity) {
    let mut dom = EcsDom::new();
    let _page = dom.create_document_root();
    let dp = dom.create_document_node();
    let body = dom.create_element_with_owner(
        "body",
        Attributes::default(),
        dom.node_document_for_new_nodes(dp),
    );
    assert!(dom.append_child(dp, body));
    (dom, SessionCore::new(), dp, body)
}

fn el(dom: &mut EcsDom, parent: Entity, tag: &str) -> Entity {
    let e = dom.create_element_with_owner(
        tag,
        Attributes::default(),
        dom.node_document_for_new_nodes(parent),
    );
    assert!(dom.append_child(parent, e));
    e
}

#[test]
fn u12_dom_api_creation_sites_stamp_the_domparser_document() {
    let (mut dom, mut session, dp, body) = dp_setup();
    let mut created: Vec<(&str, Entity)> = Vec::new();
    let sess = &mut session;

    // createAttribute (DOM §4.5: "creating an attribute given this").
    let res = CreateAttribute
        .invoke(dp, &[JsValue::String("a".into())], sess, &mut dom)
        .unwrap();
    created.push(("createAttribute", entity_of(&res, sess)));

    // cloneNode(false).
    let res = CloneNode
        .invoke(body, &[JsValue::Bool(false)], sess, &mut dom)
        .unwrap();
    created.push(("cloneNode(false)", entity_of(&res, sess)));

    // The table helpers.
    let table = el(&mut dom, body, "table");
    let res = TableCreateTHead.invoke(table, &[], sess, &mut dom).unwrap();
    created.push(("createTHead", entity_of(&res, sess)));
    let res = TableCreateCaption
        .invoke(table, &[], sess, &mut dom)
        .unwrap();
    created.push(("createCaption", entity_of(&res, sess)));
    let res = TableInsertRow.invoke(table, &[], sess, &mut dom).unwrap();
    let row = entity_of(&res, sess);
    created.push(("table.insertRow", row));
    let tbody = dom.get_parent(row).expect("the row is in a section");
    created.push(("table.insertRow's tbody", tbody));
    let res = SectionInsertRow.invoke(tbody, &[], sess, &mut dom).unwrap();
    created.push(("section.insertRow", entity_of(&res, sess)));
    let res = RowInsertCell.invoke(row, &[], sess, &mut dom).unwrap();
    created.push(("insertCell", entity_of(&res, sess)));

    // append('s'): the string is converted to a Text node.
    let append_host = el(&mut dom, body, "div");
    Append
        .invoke(append_host, &[JsValue::String("s".into())], sess, &mut dom)
        .unwrap();
    let text = dom.get_last_child(append_host).expect("appended text");
    created.push(("append('s')", text));

    // splitText.
    let split_source =
        dom.create_text_with_owner("ab", dom.node_document_for_new_nodes(append_host));
    assert!(dom.append_child(append_host, split_source));
    let res = SplitText
        .invoke(split_source, &[JsValue::Number(1.0)], sess, &mut dom)
        .unwrap();
    created.push(("splitText", entity_of(&res, sess)));

    // textContent.
    let text_content_host = el(&mut dom, body, "div");
    SetTextContentNodeKind
        .invoke(
            text_content_host,
            &[JsValue::String("t".into())],
            sess,
            &mut dom,
        )
        .unwrap();
    created.push((
        "textContent",
        dom.get_first_child(text_content_host).expect("text set"),
    ));

    // Range extract: `<p>ab<i>c</i></p>`, [ab|1, p|2] → the fragment, the
    // cloned partial Text ("b") and the moved `<i>`.
    let para = el(&mut dom, body, "p");
    let ab = dom.create_text_with_owner("ab", dom.node_document_for_new_nodes(para));
    assert!(dom.append_child(para, ab));
    let _i = el(&mut dom, para, "i");
    let mut range = Range::new(ab);
    range.set_start(ab, 1);
    range.set_end(para, 2);
    let (frag, _records) = range.extract_contents(&mut dom);
    created.push(("extract fragment", frag));
    created.push((
        "extract partial Text clone",
        dom.get_first_child(frag).expect("extracted text"),
    ));

    let off: Vec<&str> = created
        .iter()
        .filter(|(_, n)| dom.node_document(*n) != Some(dp))
        .map(|(name, _)| *name)
        .collect();
    assert_eq!(off, Vec::<&str>::new(), "created nodes not stamped dp");
}

#[test]
fn u12_control_create_element_reads_the_domparser_document() {
    let (mut dom, mut session, dp, _body) = dp_setup();
    let res = CreateElement
        .invoke(dp, &[JsValue::String("div".into())], &mut session, &mut dom)
        .unwrap();
    let div = entity_of(&res, &session);
    assert_eq!(dom.node_document(div), Some(dp));
}
