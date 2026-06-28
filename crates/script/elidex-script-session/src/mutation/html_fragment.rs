//! HTML-fragment DOM mutation setters — the WHATWG HTML §8.5 "DOM parsing and
//! serialization APIs" family (`innerHTML` / `setHTMLUnsafe` / `outerHTML` /
//! `insertAdjacentHTML`).
//!
//! Split out of [`super`] (`mutation/mod.rs`) to keep the §8.5 strict-first
//! fragment-parse + placement slice as a focused, cohesive submodule rather
//! than growing the general mutation module (One-issue-one-way; addresses the
//! elidex-review Axis 5 1000-line-file concern). All of these parse via the
//! §11.3 strict-first dispatcher `elidex_html_parser::parse_fragment_progressive`
//! and place the returned detached nodes.

use elidex_ecs::{EcsDom, Entity};

use super::{empty_record, MutationKind, MutationRecord};

/// Options controlling [`apply_set_inner_html`] fragment-parse semantics.
///
/// Per HTML §8.5.4 (innerHTML setter) vs §8.5.2 (setHTMLUnsafe), the only
/// behavioural difference is whether `<template shadowrootmode>` children
/// are converted into declarative shadow roots on their parent host or
/// left as plain `<template>` elements. The `_unsafe` JS API name refers
/// to Trusted Types sanitization which is unrelated to engine semantics;
/// from the algorithm's perspective the distinction is purely
/// "honour declarative shadow root markup yes/no".
#[derive(Default, Clone, Copy, Debug)]
pub struct SetInnerHtmlOptions {
    /// When true, `<template shadowrootmode="open|closed">` children
    /// attach as a shadow root on the parent host (per HTML §4.12.3
    /// `<template shadowrootmode>` declarative shadow DOM algorithm). When false (the default,
    /// matching plain `innerHTML` semantics), the templates are left
    /// as ordinary `<template>` elements.
    pub allow_declarative_shadow: bool,
    /// When true, parse with the §13.2.4.5 scripting flag DISABLED, so
    /// `<noscript>` content is parsed as ordinary elements rather than raw
    /// text. The default (`false`, matching `innerHTML` / `outerHTML` /
    /// `insertAdjacentHTML` on a live element) parses with scripting enabled.
    /// Only an inert parse — `DOMParser.parseFromString` (HTML §8.5.1, no
    /// browsing context, scripting disabled) — opts in.
    pub scripting_disabled: bool,
}

/// Apply the `innerHTML` / `setHTMLUnsafe` setter algorithm (§8.5.4): parse
/// `html` as a fragment in the context element, then replace all of the
/// target's children with the parsed nodes.
///
/// Per §8.5.4 the fragment-parse **context** is `entity` for an Element, but
/// the shadow root's **host** for `ShadowRoot.innerHTML` (step 2 "Let context
/// be this's host") — the parsing rules follow the host's tag/namespace while
/// the parsed nodes still replace the shadow root's own children. The
/// placement **target** is `entity`, except for a `<template>`: "set the inner
/// HTML" redirects the replacement to the template's **content fragment**
/// (HTML §4.12.3 — "if context is a template element, then set context to the
/// template's template contents"), so `template.innerHTML = …` populates
/// `template.content`, not the (always-empty) template light children.
///
/// The single `opts` parameter selects between the two JS-visible APIs:
/// `opts.allow_declarative_shadow = false` (the default) implements
/// `innerHTML`, while `true` implements `setHTMLUnsafe` (which honours
/// `<template shadowrootmode>` markup per HTML §4.12.3).
///
/// `pub` so VM bindings can invoke the algorithm directly (CLAUDE.md
/// layering mandate — DOM mutation logic lives engine-indep, not in
/// `vm/host/`); the [`Mutation::SetInnerHtml`](super::Mutation::SetInnerHtml)
/// queue path also routes through here for boa-compat (defaulted opts).
#[allow(clippy::unnecessary_wraps)] // Signature matches apply_mutation's Option<> convention.
pub fn apply_set_inner_html(
    dom: &mut EcsDom,
    entity: Entity,
    html: &str,
    opts: SetInnerHtmlOptions,
) -> Option<MutationRecord> {
    // §8.5.4: ShadowRoot.innerHTML parses in the host's context; Element
    // parses in its own. Placement target stays `entity` either way.
    let context = if dom.is_shadow_root(entity) {
        dom.shadow_host(entity).unwrap_or(entity)
    } else {
        entity
    };

    // §11.3 strict-first fragment parse → detached nodes. Parse first (reads
    // only the context's tag/namespace/ancestry, never the target's
    // children), then replace all of the target's children with the result.
    let parse_opts = elidex_html_parser::ParseFragmentOptions {
        allow_declarative_shadow: opts.allow_declarative_shadow,
        scripting_disabled: opts.scripting_disabled,
    };
    let added = elidex_html_parser::parse_fragment_progressive(html, context, dom, parse_opts);

    // HTML "set the inner HTML": for a `<template>` the parsed fragment
    // replaces the template's *content* fragment children (§4.12.3), not the
    // template element's (always-empty) light children. Non-templates and
    // shadow roots have no content fragment, so the target stays `entity`.
    let placement_target = dom.template_contents_fragment(entity).unwrap_or(entity);
    let removed: Vec<Entity> = dom.children(placement_target);
    for &child in &removed {
        let _ = dom.remove_child(placement_target, child);
    }
    for &node in &added {
        let _ = dom.append_child(placement_target, node);
    }

    Some(MutationRecord {
        added_nodes: added,
        removed_nodes: removed,
        ..empty_record(MutationKind::ChildList, placement_target)
    })
}

/// Error variants for [`apply_set_outer_html`] per WHATWG HTML §8.5.5
/// `outerHTML` setter algorithm.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum OuterHtmlError {
    /// The target entity has no parent, or its parent is the Document
    /// (Document children cannot be replaced via `outerHTML`).
    /// Surfaces as `DOMException("NoModificationAllowedError")` in JS.
    NoModificationAllowed,
}

/// Apply the `outerHTML` setter algorithm (HTML §8.5.5): parse `html`
/// in the parent's context, then replace `entity` with the parsed
/// fragment in the parent's child list.
///
/// Returns the resulting `MutationRecord` on success (`ChildList`
/// targeting the parent, with `removed_nodes = [entity]` and
/// `added_nodes` = the freshly parsed fragment roots).
///
/// When the parent is a `DocumentFragment` the §8.5.5 algorithm reads the
/// parent's `localName` (the spec documents `<body>` as the fallback context);
/// `DocumentFragment` has no tag, so `parse_fragment_progressive`'s "div"
/// fallback applies — observationally equivalent (both parse in the "in body"
/// insertion mode), so the resulting tree matches.
pub fn apply_set_outer_html(
    dom: &mut EcsDom,
    entity: Entity,
    html: &str,
) -> Result<MutationRecord, OuterHtmlError> {
    let parent = dom
        .get_parent(entity)
        .ok_or(OuterHtmlError::NoModificationAllowed)?;
    // Document parent is rejected per spec — the Document's children
    // are the doctype + root element and cannot be replaced wholesale
    // via outerHTML on an arbitrary descendant.
    if matches!(dom.node_kind(parent), Some(elidex_ecs::NodeKind::Document)) {
        return Err(OuterHtmlError::NoModificationAllowed);
    }
    // §8.5.5: the fragment-parse context is the parent. A `DocumentFragment`
    // parent has no tag; `parse_fragment_progressive` derives the context tag
    // from `parent` and falls back to "div" — which parses in the same
    // "in body" insertion mode + Data tokenizer state as the spec's documented
    // "body" fallback context, so the tree is identical (the old explicit
    // "body" string and a "div" fallback are observationally equivalent here).
    // Capture exposed siblings before placement (the inserts below change
    // `entity`'s previous sibling). The exposed-sibling helpers skip internal
    // `ShadowRoot` entities so `MutationRecord.previousSibling/.nextSibling`
    // never leak a closed shadow across the §4.8 encapsulation boundary.
    let prev_sibling = dom.prev_exposed_sibling(entity);
    let next_sibling = dom.next_exposed_sibling(entity);
    let parse_opts = elidex_html_parser::ParseFragmentOptions::default();
    let added = elidex_html_parser::parse_fragment_progressive(html, parent, dom, parse_opts);
    // The parsed roots come back detached; insert them in order before
    // `entity`, then unhook `entity`.
    for &node in &added {
        let _ = dom.insert_before(parent, node, entity);
    }
    let _ = dom.remove_child(parent, entity);
    Ok(MutationRecord {
        added_nodes: added,
        removed_nodes: vec![entity],
        previous_sibling: prev_sibling,
        next_sibling,
        ..empty_record(MutationKind::ChildList, parent)
    })
}

/// Apply `insertAdjacentHTML`: parse HTML fragment and insert at position.
#[allow(clippy::unnecessary_wraps)]
pub(super) fn apply_insert_adjacent_html(
    dom: &mut EcsDom,
    entity: Entity,
    position: &str,
    html: &str,
) -> Option<MutationRecord> {
    // insertAdjacentHTML does not honour declarative shadow root markup
    // per HTML §8.5.6 — only setHTMLUnsafe / innerHTML w/ that opt-in do.
    // §11.3 strict-first parse returns detached nodes; the context element
    // (parent for before/afterend, the element itself for after/beforeend)
    // is read by `parse_fragment_progressive` to derive the parsing rules.
    let parse_opts = elidex_html_parser::ParseFragmentOptions::default();
    let added = match position {
        "beforebegin" => {
            let parent = dom.get_parent(entity)?;
            let nodes =
                elidex_html_parser::parse_fragment_progressive(html, parent, dom, parse_opts);
            for &node in &nodes {
                let _ = dom.insert_before(parent, node, entity);
            }
            nodes
        }
        "afterbegin" => {
            let first_child = dom.get_first_child(entity);
            let nodes =
                elidex_html_parser::parse_fragment_progressive(html, entity, dom, parse_opts);
            match first_child {
                Some(ref_child) => {
                    for &node in &nodes {
                        let _ = dom.insert_before(entity, node, ref_child);
                    }
                }
                None => {
                    for &node in &nodes {
                        let _ = dom.append_child(entity, node);
                    }
                }
            }
            nodes
        }
        "beforeend" => {
            let nodes =
                elidex_html_parser::parse_fragment_progressive(html, entity, dom, parse_opts);
            for &node in &nodes {
                let _ = dom.append_child(entity, node);
            }
            nodes
        }
        "afterend" => {
            let parent = dom.get_parent(entity)?;
            let next = dom.get_next_sibling(entity);
            let nodes =
                elidex_html_parser::parse_fragment_progressive(html, parent, dom, parse_opts);
            match next {
                // Constant ref_child preserves document order: insert A before
                // ref → [... A ref], insert B before ref → [... A B ref].
                Some(ref_child) => {
                    for &node in &nodes {
                        let _ = dom.insert_before(parent, node, ref_child);
                    }
                }
                // `entity` is the last child, so afterend == append to parent.
                None => {
                    for &node in &nodes {
                        let _ = dom.append_child(parent, node);
                    }
                }
            }
            nodes
        }
        _ => return None,
    };

    let target = match position {
        "beforebegin" | "afterend" => dom.get_parent(entity).unwrap_or(entity),
        _ => entity,
    };

    Some(MutationRecord {
        added_nodes: added,
        ..empty_record(MutationKind::ChildList, target)
    })
}

#[cfg(test)]
#[allow(unused_must_use)] // Test setup calls dom.append_child() etc. without checking return values
mod tests {
    use super::*;
    // `Mutation` / `apply_mutation` live in the parent `mutation` module; the
    // `SetInnerHtml` queue path routes through `apply_mutation`.
    use super::super::{apply_mutation, Mutation};
    use elidex_ecs::Attributes;

    fn elem(dom: &mut EcsDom, tag: &str) -> Entity {
        dom.create_element(tag, Attributes::default())
    }

    #[test]
    fn set_inner_html_parses_and_replaces() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let e = elem(&mut dom, "div");
        let _ = dom.append_child(root, e);
        let old_child = dom.create_text("old");
        let _ = dom.append_child(e, old_child);

        let m = Mutation::SetInnerHtml {
            entity: e,
            html: "<p>new</p>".into(),
        };
        let records = apply_mutation(&m, &mut dom);
        assert_eq!(records.len(), 1, "SetInnerHtml should return one record");
        let record = &records[0];
        assert_eq!(record.removed_nodes.len(), 1, "should remove old child");
        assert_eq!(record.added_nodes.len(), 1, "should add <p>");
    }

    // -------------------------------------------------------------
    // D-15 PR-B: declarative shadow root + outerHTML coverage
    // -------------------------------------------------------------

    #[test]
    fn set_inner_html_default_opts_leaves_template_shadowroot_as_plain_element() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let host = elem(&mut dom, "div");
        let _ = dom.append_child(root, host);
        let _ = apply_set_inner_html(
            &mut dom,
            host,
            r#"<template shadowrootmode="open"><p>x</p></template>"#,
            SetInnerHtmlOptions::default(),
        );
        // Default opts → no shadow attached, template remains as a
        // child element of the host.
        assert!(dom.get_shadow_root(host).is_none());
        let kids = dom.children(host);
        assert_eq!(kids.len(), 1);
        let tag = dom
            .world()
            .get::<&elidex_ecs::TagType>(kids[0])
            .expect("template entity should exist")
            .0
            .clone();
        assert_eq!(tag, "template");
    }

    #[test]
    fn set_html_unsafe_dsd_on_context_preserves_shadow_and_excludes_it_from_removed() {
        // PR337 Codex R2 (P2) FP guard: setHTMLUnsafe with a top-level
        // declarative-shadow template attaches a shadow to the context during
        // parse (DSD-on-context). `apply_set_inner_html` parses BEFORE snapshotting
        // the children to remove, so the snapshot runs after the shadow is
        // attached — but `EcsDom::children` excludes `ShadowRoot` entities, so the
        // removal loop never detaches the shadow root and it never appears in
        // `removed_nodes`. The host↔shadow-root invariant holds; only the
        // pre-existing light children are removed.
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let host = elem(&mut dom, "div");
        let _ = dom.append_child(root, host);
        // Pre-existing light children so the removal loop runs over a non-empty set.
        let old1 = elem(&mut dom, "span");
        let old2 = elem(&mut dom, "b");
        let _ = dom.append_child(host, old1);
        let _ = dom.append_child(host, old2);

        let record = apply_set_inner_html(
            &mut dom,
            host,
            r#"<template shadowrootmode="open"><p>x</p></template>"#,
            SetInnerHtmlOptions {
                allow_declarative_shadow: true,
                scripting_disabled: false,
            },
        )
        .expect("record");

        // Shadow attached to the context and survives intact.
        let sr = dom
            .get_shadow_root(host)
            .expect("shadow root attached to host");
        assert!(
            dom.children(sr).iter().any(|c| dom
                .world()
                .get::<&elidex_ecs::TagType>(*c)
                .is_ok_and(|t| t.0 == "p")),
            "shadow tree holds the parsed <p>"
        );
        // The shadow root is NOT reported as removed (children() excludes it);
        // exactly the pre-existing light children were removed.
        assert!(
            !record.removed_nodes.contains(&sr),
            "shadow root must not appear in removed_nodes"
        );
        assert_eq!(
            record.removed_nodes,
            vec![old1, old2],
            "only the pre-existing light children are removed"
        );
        // Host↔shadow-root invariant intact (round-trips).
        assert_eq!(dom.get_shadow_root(host), Some(sr));
    }

    #[test]
    fn set_inner_html_allow_declarative_shadow_attaches_open_shadow_root() {
        // §13.4 fragment case + §13.2.6.4.4 step 9–10: a *top-level*
        // `<template shadowrootmode>` via setHTMLUnsafe attaches its declarative
        // shadow to the context (here the host element) — the fragment adjusted
        // current node is the context while the stack holds only the synthetic
        // root (DSD-on-context).
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let host = elem(&mut dom, "div");
        let _ = dom.append_child(root, host);
        let _ = apply_set_inner_html(
            &mut dom,
            host,
            r#"<template shadowrootmode="open"><p>shadow content</p></template>"#,
            SetInnerHtmlOptions {
                allow_declarative_shadow: true,
                scripting_disabled: false,
            },
        );
        let sr = dom
            .get_shadow_root(host)
            .expect("declarative shadow should attach to the context host");
        let sr_component = dom.world().get::<&elidex_ecs::ShadowRoot>(sr).unwrap();
        assert_eq!(sr_component.mode, elidex_ecs::ShadowRootMode::Open);
        // The <template> element itself is consumed; the host's light tree has
        // no surviving template child.
        for kid in dom.children(host) {
            if let Ok(tag) = dom.world().get::<&elidex_ecs::TagType>(kid) {
                assert_ne!(tag.0, "template", "template should be consumed");
            }
        }
        // The shadow tree contains the parsed contents (a <p>).
        let shadow_kids = dom.children(sr);
        assert!(
            shadow_kids.iter().any(|c| dom
                .world()
                .get::<&elidex_ecs::TagType>(*c)
                .is_ok_and(|t| t.0 == "p")),
            "shadow root should hold the parsed <p>"
        );
    }

    #[test]
    fn set_inner_html_allow_declarative_shadow_attaches_closed_shadow_root() {
        // Top-level DSD-on-context — mode "closed".
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let host = elem(&mut dom, "div");
        let _ = dom.append_child(root, host);
        let _ = apply_set_inner_html(
            &mut dom,
            host,
            r#"<template shadowrootmode="closed"></template>"#,
            SetInnerHtmlOptions {
                allow_declarative_shadow: true,
                scripting_disabled: false,
            },
        );
        let sr = dom
            .get_shadow_root(host)
            .expect("should attach to context host");
        let mode = dom.world().get::<&elidex_ecs::ShadowRoot>(sr).unwrap().mode;
        assert_eq!(mode, elidex_ecs::ShadowRootMode::Closed);
    }

    #[test]
    fn set_inner_html_invalid_shadowrootmode_leaves_template_as_element() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let host = elem(&mut dom, "div");
        let _ = dom.append_child(root, host);
        let _ = apply_set_inner_html(
            &mut dom,
            host,
            r#"<template shadowrootmode="weird"></template>"#,
            SetInnerHtmlOptions {
                allow_declarative_shadow: true,
                scripting_disabled: false,
            },
        );
        // Invalid mode → silently no shadow + template remains.
        assert!(dom.get_shadow_root(host).is_none());
        let kids = dom.children(host);
        assert_eq!(kids.len(), 1);
        let tag = dom.world().get::<&elidex_ecs::TagType>(kids[0]).unwrap();
        assert_eq!(tag.0, "template");
    }

    #[test]
    fn set_inner_html_declarative_shadow_already_attached_falls_back_silently() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let host = elem(&mut dom, "div");
        let _ = dom.append_child(root, host);
        // Pre-attach a shadow root so the declarative attach is rejected.
        let existing = dom
            .attach_shadow(host, elidex_ecs::ShadowRootMode::Open)
            .expect("first attach");
        let _ = apply_set_inner_html(
            &mut dom,
            host,
            r#"<template shadowrootmode="open"><p>x</p></template>"#,
            SetInnerHtmlOptions {
                allow_declarative_shadow: true,
                scripting_disabled: false,
            },
        );
        // The existing shadow root is unchanged; the template becomes a
        // plain child of the host.
        assert_eq!(
            dom.get_shadow_root(host),
            Some(existing),
            "pre-existing shadow root must be preserved"
        );
    }

    #[test]
    fn set_inner_html_case_insensitive_shadowrootmode() {
        // Top-level DSD-on-context — `shadowrootmode` is an
        // ASCII-case-insensitive enumerated attribute (HTML §2.3.3).
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let host = elem(&mut dom, "div");
        let _ = dom.append_child(root, host);
        let _ = apply_set_inner_html(
            &mut dom,
            host,
            r#"<template shadowrootmode="OPEN"></template>"#,
            SetInnerHtmlOptions {
                allow_declarative_shadow: true,
                scripting_disabled: false,
            },
        );
        let sr = dom
            .get_shadow_root(host)
            .expect("case-insensitive 'OPEN' should attach to context host");
        let mode = dom.world().get::<&elidex_ecs::ShadowRoot>(sr).unwrap().mode;
        assert_eq!(mode, elidex_ecs::ShadowRootMode::Open);
    }

    #[test]
    fn set_inner_html_on_shadow_root_parses_in_host_context_and_fills_shadow() {
        // §8.5.4 ShadowRoot.innerHTML step 2: the fragment-parse context is the
        // shadow root's *host*, and the parsed nodes replace the SHADOW ROOT's
        // children (not the host's light tree). Passing the shadow-root entity
        // resolves the context to its host via `EcsDom::shadow_host`.
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let host = elem(&mut dom, "div");
        let _ = dom.append_child(root, host);
        let sr = dom
            .attach_shadow(host, elidex_ecs::ShadowRootMode::Open)
            .expect("attach shadow");
        let _ = apply_set_inner_html(
            &mut dom,
            sr,
            "<p>in shadow</p>",
            SetInnerHtmlOptions::default(),
        );
        assert!(
            dom.children(sr).iter().any(|c| dom
                .world()
                .get::<&elidex_ecs::TagType>(*c)
                .is_ok_and(|t| t.0 == "p")),
            "the parsed <p> lands in the shadow root"
        );
        assert!(
            dom.children(host).is_empty(),
            "the host's light tree is unchanged (content went into the shadow root)"
        );
    }

    #[test]
    fn set_inner_html_fires_insert_at_caller_placement() {
        // §11.3 slice 2b: the fragment parser builds in isolation (events
        // suppressed) and returns detached nodes — so the real
        // `MutationEvent::Insert` fires when `apply_set_inner_html` places them
        // (caller-driven), not during the parse. Confirms the suppress-then-
        // re-fire contract end-to-end at the caller layer.
        use elidex_ecs::{MutationDispatcher, MutationEvent, TagType};
        use std::sync::{Arc, Mutex};

        struct InsertProbe(Arc<Mutex<Vec<String>>>);
        impl MutationDispatcher for InsertProbe {
            fn dispatch(&mut self, event: &MutationEvent<'_>, dom: &mut EcsDom) {
                if let MutationEvent::Insert { node, .. } = *event {
                    let tag = dom
                        .world()
                        .get::<&TagType>(node)
                        .map(|t| t.0.clone())
                        .unwrap_or_default();
                    self.0.lock().unwrap().push(tag);
                }
            }
        }

        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let host = elem(&mut dom, "div");
        let _ = dom.append_child(root, host);
        let log = Arc::new(Mutex::new(Vec::new()));
        let _ = dom.set_mutation_dispatcher(Box::new(InsertProbe(Arc::clone(&log))));
        let _ = apply_set_inner_html(
            &mut dom,
            host,
            "<span>a</span><b>c</b>",
            SetInnerHtmlOptions::default(),
        );
        let tags = log.lock().unwrap();
        assert!(
            tags.iter().any(|t| t == "span") && tags.iter().any(|t| t == "b"),
            "placement of the detached fragment roots fires Insert events, got {tags:?}"
        );
    }

    #[test]
    fn apply_set_outer_html_replaces_element_in_parent() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let parent = elem(&mut dom, "div");
        let _ = dom.append_child(root, parent);
        let target = elem(&mut dom, "span");
        let _ = dom.append_child(parent, target);

        let record =
            apply_set_outer_html(&mut dom, target, "<p>new</p>").expect("outer html should apply");
        assert_eq!(record.removed_nodes, vec![target]);
        assert_eq!(record.added_nodes.len(), 1);
        assert_ne!(
            dom.get_parent(target),
            Some(parent),
            "target should be unhooked from parent"
        );
        // The new <p> is now a child of parent.
        let kids = dom.children(parent);
        assert_eq!(kids.len(), 1);
        let tag = dom.world().get::<&elidex_ecs::TagType>(kids[0]).unwrap();
        assert_eq!(tag.0, "p");
    }

    #[test]
    fn apply_set_outer_html_no_parent_returns_error() {
        let mut dom = EcsDom::new();
        let orphan = elem(&mut dom, "div");
        let err = apply_set_outer_html(&mut dom, orphan, "<p></p>").unwrap_err();
        assert_eq!(err, OuterHtmlError::NoModificationAllowed);
    }

    #[test]
    fn apply_set_outer_html_document_fragment_parent_uses_body_context() {
        // PR201 Copilot R14 regression: `apply_set_outer_html` falls
        // back to the synthetic `<body>` fragment-parse context when
        // parent is a `DocumentFragment` (per HTML §4.4.5 — fragment
        // parents have no tag name, so the spec uses `<body>` as the
        // fallback context). The branch was added without a direct
        // test; this case locks it so a future refactor cannot
        // silently regress to e.g. `<div>` context (which would parse
        // table-context fragments incorrectly).
        let mut dom = EcsDom::new();
        let frag = dom.create_document_fragment();
        let target = elem(&mut dom, "span");
        let _ = dom.append_child(frag, target);

        let record = apply_set_outer_html(&mut dom, target, "<p>new</p>")
            .expect("outerHTML on a DocumentFragment child should apply");
        assert_eq!(record.removed_nodes, vec![target]);
        assert_eq!(record.added_nodes.len(), 1, "one parsed root expected");
        // Verify the parsed root is a `<p>` element — confirming the
        // fragment-parse context didn't drop or restructure the input.
        let kids = dom.children(frag);
        assert_eq!(kids.len(), 1);
        let tag = dom.world().get::<&elidex_ecs::TagType>(kids[0]).unwrap();
        assert_eq!(tag.0, "p");
    }

    #[test]
    fn apply_set_outer_html_document_parent_returns_error() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let html_elem = elem(&mut dom, "html");
        let _ = dom.append_child(doc, html_elem);
        let err = apply_set_outer_html(&mut dom, html_elem, "<html></html>").unwrap_err();
        assert_eq!(err, OuterHtmlError::NoModificationAllowed);
    }

    // -------------------------------------------------------------
    // PR201 Copilot R2: MutationRecord sibling fields must not leak
    // internal ShadowRoot entities (encapsulation lock).
    // -------------------------------------------------------------

    #[test]
    fn apply_set_outer_html_captures_next_sibling_before_parsing() {
        // PR201 Copilot R10 regression lock: `apply_set_outer_html`
        // captures `prev_sibling` / `next_sibling` BEFORE placing the
        // parsed roots. Since §11.3 slice 2b the fragment parser returns
        // detached nodes (it no longer appends to `parent`), so the old
        // "post-parse next-sibling sees the appended output" hazard is
        // gone — but the inserts still change `entity`'s previous sibling,
        // so the capture order still matters. Lock that `MutationRecord`
        // reflects the pre-mutation tree state per the §8.5.5 `outerHTML`
        // replacement algorithm.
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let parent = elem(&mut dom, "div");
        let _ = dom.append_child(root, parent);
        let first = elem(&mut dom, "span");
        let _ = dom.append_child(parent, first);
        // `target` is the LAST child of `parent` — the bug case.
        let target = elem(&mut dom, "p");
        let _ = dom.append_child(parent, target);

        let record =
            apply_set_outer_html(&mut dom, target, "<b>new</b>").expect("outer html should apply");
        assert_eq!(
            record.next_sibling, None,
            "MutationRecord.next_sibling for the last-child case must \
             stay None — must not surface the parser's temporary append"
        );
        assert_eq!(
            record.previous_sibling,
            Some(first),
            "previous_sibling reflects pre-mutation tree"
        );
    }

    #[test]
    fn apply_set_outer_html_does_not_leak_shadow_root_as_previous_sibling() {
        // Scenario: parent is a shadow host; the shadow root entity
        // becomes the FIRST child of the host (attach_shadow appends
        // it before light tree children). Then a light child is
        // appended. When that light child is replaced via outerHTML,
        // its raw `prev_sibling` would be the ShadowRoot entity —
        // surfacing as `MutationRecord.previousSibling` would leak the
        // closed shadow to JS. The fix walks past ShadowRoot entities
        // via `prev_exposed_sibling`.
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let host = elem(&mut dom, "div");
        let _ = dom.append_child(root, host);
        let shadow_root = dom
            .attach_shadow(host, elidex_ecs::ShadowRootMode::Closed)
            .expect("attach closed shadow");
        let target = elem(&mut dom, "span");
        let _ = dom.append_child(host, target);
        // Sanity: raw prev_sibling for `target` IS the shadow root —
        // confirms the helper would leak without the fix.
        assert_eq!(
            dom.get_prev_sibling(target),
            Some(shadow_root),
            "shadow root sits between host and target in the raw sibling chain"
        );
        let record =
            apply_set_outer_html(&mut dom, target, "<p>new</p>").expect("outer html should apply");
        assert_ne!(
            record.previous_sibling,
            Some(shadow_root),
            "MutationRecord.previous_sibling must not leak shadow root"
        );
        assert_eq!(
            record.previous_sibling, None,
            "no exposed prev sibling for target (shadow root skipped)"
        );
    }
}
