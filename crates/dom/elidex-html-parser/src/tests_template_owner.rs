//! C1 unit cells U3, U4, U5 and U12
//! (`docs/plans/2026-10-eh-attrs-c1-template-owner-document.md` §6.2) that
//! need a parser: both tiers parse template contents into the inert template
//! contents owner document (WHATWG HTML §4.12.3 *appropriate template contents
//! owner document*), lazily, and never for the HTML §13.4 throwaway document.
//! This crate is the lowest that reaches both tiers.
//!
//! Red-first: the cells name C1's NEW API (the rule, the marker component),
//! so on the base they are red by absence (compile error).

use elidex_ecs::{Attributes, EcsDom, Entity, InertTemplateDocument, TagType};

use crate::{parse_fragment_strict, parse_html_fragment, ParseFragmentOptions};

/// How many entities carry [`InertTemplateDocument`].
fn marked(dom: &EcsDom) -> usize {
    dom.world().query::<&InertTemplateDocument>().iter().count()
}

fn tag_of(dom: &EcsDom, n: Entity) -> Option<String> {
    dom.world().get::<&TagType>(n).ok().map(|t| t.0.clone())
}

/// Every node under `root` (light tree, template contents, shadow trees),
/// `root` included.
fn all_nodes(dom: &EcsDom, root: Entity) -> Vec<Entity> {
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(n) = stack.pop() {
        out.push(n);
        stack.extend(dom.children(n));
        if let Some(contents) = dom.template_contents_fragment(n) {
            stack.push(contents);
        }
        if let Some(sr) = dom.get_shadow_root(n) {
            stack.push(sr);
        }
    }
    out
}

fn parse_document(strict: bool, html: &str) -> crate::ParseResult {
    if strict {
        crate::parse_strict(html).expect("conforming markup")
    } else {
        crate::parse_html(html)
    }
}

// ---- U3 (×2): no template, no inert document ----
#[test]
fn u3_no_inert_document_without_a_template() {
    for strict in [true, false] {
        let mut parsed = parse_document(
            strict,
            "<!DOCTYPE html><html><head></head><body><div><p>x</p></div></body></html>",
        );
        assert_eq!(marked(&parsed.dom), 0, "strict={strict}: after the parse");
        // `createElement('div')`: the element is created through the entry.
        let owner = parsed.dom.node_document_for_new_nodes(parsed.document);
        let _div = parsed
            .dom
            .create_element_with_owner("div", Attributes::default(), owner);
        assert_eq!(
            marked(&parsed.dom),
            0,
            "strict={strict}: after createElement"
        );
    }
}

const FRAGMENT_OPTS: ParseFragmentOptions = ParseFragmentOptions {
    allow_declarative_shadow: false,
    scripting_disabled: false,
};

/// `D` with a `<div>` context in its tree; returns (dom, D, context).
fn fragment_setup() -> (EcsDom, Entity, Entity) {
    let mut dom = EcsDom::new();
    let d = dom.create_document_root();
    let context = dom.create_element_with_owner(
        "div",
        Attributes::default(),
        dom.node_document_for_new_nodes(d),
    );
    assert!(dom.append_child(d, context));
    (dom, d, context)
}

// ---- U4 (×2): 100 fragment parses with a template create one I, D's ----
// The throwaway read as a component before its destruction is the ECS half
// (`elidex-ecs` `dom/tests/template_owner.rs`).
#[test]
fn u4_fragment_parses_create_only_the_contexts_inert_document() {
    for strict in [true, false] {
        let (mut dom, d, context) = fragment_setup();
        for _ in 0..100 {
            let roots = if strict {
                parse_fragment_strict(
                    "<template><p></p></template>",
                    context,
                    &mut dom,
                    FRAGMENT_OPTS,
                )
                .expect("conforming")
            } else {
                parse_html_fragment(
                    "<template><p></p></template>",
                    context,
                    &mut dom,
                    FRAGMENT_OPTS,
                )
            };
            let contents = dom
                .template_contents_fragment(roots[0])
                .expect("a template parsed");
            let i = dom.appropriate_template_contents_owner_document(d);
            assert_eq!(dom.node_document(contents), Some(i), "strict={strict}");
        }
        assert_eq!(marked(&dom), 1, "strict={strict}");
    }
}

// ---- U5: a strict parse error leaves no I and no throwaway entity ----
#[test]
fn u5_strict_rollback_with_a_template_leaves_the_dom_as_it_was() {
    let (mut dom, d, context) = fragment_setup();
    // D's I exists already (the "already there" case), so the count and the
    // entity total are both exact.
    let _ = dom.appropriate_template_contents_owner_document(d);
    let entities = dom.world().len();
    assert!(parse_fragment_strict(
        "<template><p></p></template></br>",
        context,
        &mut dom,
        FRAGMENT_OPTS
    )
    .is_err());
    assert_eq!(marked(&dom), 1);
    assert_eq!(
        dom.world().len(),
        entities,
        "no entity of the build survives"
    );
}

// ---- U12 (×2): a document parse creates contents nodes in f(D), no walk ----
// A document parse builds into a fresh `EcsDom`, so no dispatcher can be
// installed before it; the walk counter (C0b's `adopt_walk_count`) stands
// for the recorder: an `Adopt` event fires only from a walk.
#[test]
fn u12_document_parse_creates_contents_in_the_inert_document() {
    for strict in [true, false] {
        let mut parsed = parse_document(
            strict,
            "<!DOCTYPE html><html><head><template><svg></svg><b>t</b><!--c--></template>\
             </head><body></body></html>",
        );
        let d = parsed.document;
        assert_eq!(parsed.dom.adopt_walk_count(), 0, "strict={strict}: no walk");
        let tpl = all_nodes(&parsed.dom, d)
            .into_iter()
            .find(|&n| tag_of(&parsed.dom, n).as_deref() == Some("template"))
            .expect("template parsed");
        let contents = parsed.dom.template_contents_fragment(tpl).unwrap();
        let i = parsed.dom.appropriate_template_contents_owner_document(d);
        let nodes = all_nodes(&parsed.dom, contents);
        assert_eq!(
            nodes.len(),
            5,
            "strict={strict}: fragment, svg, b, text, comment"
        );
        let off: Vec<Option<String>> = nodes
            .iter()
            .filter(|&&n| parsed.dom.node_document(n) != Some(i))
            .map(|&n| tag_of(&parsed.dom, n))
            .collect();
        assert_eq!(off, Vec::<Option<String>>::new(), "strict={strict}");
    }
}
