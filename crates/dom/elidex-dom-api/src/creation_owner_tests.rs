//! C0b U12 (`docs/plans/2026-11-eh-attrs-c0b-node-document-fact.md` §6.2):
//! every node a dom-api algorithm creates on a non-bound ("DOMParser")
//! document's tree is created in that document — DOM §4.4: the node document
//! is set when the node is created, from the node the algorithm names. A
//! value check over the creating handlers, **not** the creation census (D-17's
//! compile guarantee is).
//!
//! Red-first base formulation (§6.5): `get_associated_document` read `None`
//! for the table helpers, `cloneNode(false)`, `append('s')` and Range extract.

use elidex_ecs::{Attributes, EcsDom, Entity};
use elidex_plugin::JsValue;
use elidex_script_session::{DomApiHandler, SessionCore};

use crate::char_data::{CreateAttribute, SplitText};
use crate::child_node::Append;
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
    let s = &mut session;

    // createAttribute (DOM §4.5: "creating an attribute given this").
    let r = CreateAttribute
        .invoke(dp, &[JsValue::String("a".into())], s, &mut dom)
        .unwrap();
    created.push(("createAttribute", entity_of(&r, s)));

    // cloneNode(false).
    let r = CloneNode
        .invoke(body, &[JsValue::Bool(false)], s, &mut dom)
        .unwrap();
    created.push(("cloneNode(false)", entity_of(&r, s)));

    // The table helpers.
    let table = el(&mut dom, body, "table");
    let r = TableCreateTHead.invoke(table, &[], s, &mut dom).unwrap();
    created.push(("createTHead", entity_of(&r, s)));
    let r = TableCreateCaption.invoke(table, &[], s, &mut dom).unwrap();
    created.push(("createCaption", entity_of(&r, s)));
    let r = TableInsertRow.invoke(table, &[], s, &mut dom).unwrap();
    let row = entity_of(&r, s);
    created.push(("table.insertRow", row));
    let tbody = dom.get_parent(row).expect("the row is in a section");
    created.push(("table.insertRow's tbody", tbody));
    let r = SectionInsertRow.invoke(tbody, &[], s, &mut dom).unwrap();
    created.push(("section.insertRow", entity_of(&r, s)));
    let r = RowInsertCell.invoke(row, &[], s, &mut dom).unwrap();
    created.push(("insertCell", entity_of(&r, s)));

    // append('s'): the string is converted to a Text node.
    let x = el(&mut dom, body, "div");
    Append
        .invoke(x, &[JsValue::String("s".into())], s, &mut dom)
        .unwrap();
    let text = dom.get_last_child(x).expect("appended text");
    created.push(("append('s')", text));

    // splitText.
    let t = dom.create_text_with_owner("ab", dom.node_document_for_new_nodes(x));
    assert!(dom.append_child(x, t));
    let r = SplitText
        .invoke(t, &[JsValue::Number(1.0)], s, &mut dom)
        .unwrap();
    created.push(("splitText", entity_of(&r, s)));

    // textContent.
    let y = el(&mut dom, body, "div");
    SetTextContentNodeKind
        .invoke(y, &[JsValue::String("t".into())], s, &mut dom)
        .unwrap();
    created.push(("textContent", dom.get_first_child(y).expect("text set")));

    // Range extract: `<p>ab<i>c</i></p>`, [ab|1, p|2] → the fragment, the
    // cloned partial Text ("b") and the moved `<i>`.
    let p = el(&mut dom, body, "p");
    let ab = dom.create_text_with_owner("ab", dom.node_document_for_new_nodes(p));
    assert!(dom.append_child(p, ab));
    let _i = el(&mut dom, p, "i");
    let mut range = Range::new(ab);
    range.set_start(ab, 1);
    range.set_end(p, 2);
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
