//! Pre-layout generated-content resolution — the single source of CSS
//! generated text (One-issue-one-way).
//!
//! Runs as the final phase of style resolution, after the cascade walk has
//! attached `ComputedStyle` and spawned the `::before`/`::after` pseudo-element
//! entities. A single document-order walk drives the CSS counter state machine
//! (CSS Lists 3 §4 Automatic Numbering With Counters) and resolves all
//! generated text, writing results to ECS components that **both** layout and
//! render read:
//!
//! - pseudo-element `content` (CSS Content 3 §2 Generated Content Values) →
//!   overwrites the pseudo entity's [`TextContent`];
//! - list-item marker text (CSS Lists 3 §4.6 The Implicit list-item Counter,
//!   §4.7 the `counter()` function) → a reconciled [`ListItemMarker`] component
//!   on the list-item element.
//!
//! Because this resolves counters once, before layout, layout measures the
//! *resolved* text (not a `[counter:…]` placeholder) and render no longer runs
//! a counter machine for document content — it reads the components.
//! (Paged-media margin-box counters stay in render: per-page running-header
//! values require post-pagination page assignment and cannot be precomputed.)

use std::fmt::Write as _;

use elidex_ecs::{
    Attributes, EcsDom, Entity, ListItemMarker, PseudoElementMarker, TagType, TextContent,
    MAX_ANCESTOR_DEPTH,
};
use elidex_plugin::{ComputedStyle, ContentItem, ContentValue, Display, ListStyleType};

use crate::counter::{apply_implicit_list_counters_from_dom, CounterState};
use crate::walk::find_roots;

/// Resolve CSS counters + generated content (pseudo `content`, list-item
/// markers) in document order, writing the results to ECS components.
///
/// Invoked once per style resolution (as the final phase of
/// [`resolve_styles_with_compat`](crate::resolve_styles_with_compat)), after the
/// cascade walk (so pseudo entities exist and every element carries
/// `ComputedStyle`) and before layout (so layout reads resolved text). Exposed
/// for callers that build a styled DOM directly and need only this phase.
pub fn resolve_generated_content(dom: &mut EcsDom) {
    // A fresh counter state per root: `find_roots` may return multiple disconnected
    // trees (the fallback scan), which behave as independent documents — counter
    // scope (CSS Lists 3 §4.3) must not leak across them.
    for root in find_roots(dom) {
        let mut state = CounterState::new();
        resolve_tree(dom, root, &mut state, 0);
    }
}

/// Document-order recursion: drive the counter machine and resolve generated
/// content for `entity`, then recurse into composed children.
fn resolve_tree(dom: &mut EcsDom, entity: Entity, state: &mut CounterState, depth: usize) {
    // Match the cap convention of the other document-order walks (render's `walk`,
    // layout) — `> MAX` with an initial depth of 0 — so generated-content
    // resolution covers exactly the depths layout/render process (not one shallower).
    if depth > MAX_ANCESTOR_DEPTH {
        return;
    }

    // Cheap probe: extract only the Copy marker fields + whether this element
    // carries any explicit counter property, releasing the `ComputedStyle` borrow
    // before any `&mut`. The common (non-counter, non-list) element never clones
    // the full `ComputedStyle` — this pass runs on every restyle.
    //
    // Style-less entities — the document root (no `TagType`), text nodes — carry
    // no counters or generated content of their own, but their children must
    // still be visited in document order (the root is style-less yet contains the
    // whole tree). Recurse without opening a scope; text nodes simply have no
    // children. (Mirrors render's walk, which recurses style-less nodes.)
    let Some((display, list_style_type, has_counter_props)) =
        dom.world().get::<&ComputedStyle>(entity).ok().map(|s| {
            (
                s.display,
                s.list_style_type,
                !s.counter_reset.is_empty()
                    || !s.counter_set.is_empty()
                    || !s.counter_increment.is_empty(),
            )
        })
    else {
        for child in dom.composed_children(entity) {
            resolve_tree(dom, child, state, depth + 1);
        }
        return;
    };

    // RECONCILE the list-item marker (CSS Lists 3 §4.6/§4.7): a marker is written
    // ONLY for a visible-type list-item (below, after counter processing). Remove
    // any stale marker from a prior resolve for every OTHER element NOW — before the
    // display:none / display:contents early-returns — so the "insert-or-remove every
    // pass" invariant holds on ALL exit paths (element entities persist across
    // re-resolves; same explicit-clear discipline as `InlineFlow`). Visibility is
    // render's paint concern; the marker text is written independently of it.
    let is_visible_list_item =
        display == Display::ListItem && list_style_type != ListStyleType::None;
    if !is_visible_list_item {
        let _ = dom.world_mut().remove_one::<ListItemMarker>(entity);
    }

    // CSS Lists 3 §4.5 (Counters in elements that do not generate boxes): an
    // element with `display: none` cannot set, reset, or increment a counter —
    // and it generates no subtree to render. Skip it entirely.
    if display == Display::None {
        return;
    }

    // CSS Display 3 §2.5 + §4.5: a `display: contents` element generates no box
    // of its own, so its own counter properties have no effect, but its children
    // participate in the parent's formatting context. Recurse children in the
    // current scope without pushing one or processing this element's counters
    // (matches layout/render `composed_children_flat` contents-flattening).
    if display == Display::Contents {
        for child in dom.composed_children(entity) {
            resolve_tree(dom, child, state, depth + 1);
        }
        return;
    }

    // CSS Lists 3 §4.3 (Nested Counters and Scope): each box opens a scope.
    state.push_scope();

    let is_list_tag = dom
        .world()
        .get::<&TagType>(entity)
        .is_ok_and(|t| matches!(t.0.as_str(), "ol" | "ul" | "li"));

    // CSS Lists 3 §4.1/§4.2/§4.6: process counter-reset (incl. reversed) → set →
    // increment, but only clone the style when there is actual counter influence —
    // an explicit counter-* property or an implicit list counter (ol/ul/li). For
    // everything else the empty counter vecs make `process_element` a no-op, so the
    // clone is skipped. `apply_implicit_list_counters_from_dom` is **load-bearing**,
    // not redundant: the parallel cascade path (`walk::walk_children`, `parallel`
    // feature) inserts the style WITHOUT baking implicit counters — only the
    // sequential path bakes them; its `already_has` guard makes this idempotent when
    // they were. No fragmentation continuation pre-layout (paged continuation, which
    // suppresses re-increment, is handled by render's per-fragment walk).
    if is_list_tag || has_counter_props {
        if let Ok(s) = dom.world().get::<&ComputedStyle>(entity) {
            let mut style = (*s).clone();
            apply_implicit_list_counters_from_dom(dom, entity, &mut style);
            state.process_element(&style, false);
        }
    }

    // CSS Lists 3 §4.6/§4.7: write the resolved marker text for a visible-type
    // list-item (the stale-removal half of the reconcile ran early, above, so it
    // also covers the display:none / display:contents exit paths).
    if is_visible_list_item {
        let text = state.evaluate_counter("list-item", list_style_type);
        let _ = dom.world_mut().insert_one(entity, ListItemMarker(text));
    }

    // CSS Content 3 §2: resolve pseudo-element `content` into TextContent.
    if dom.world().get::<&PseudoElementMarker>(entity).is_ok() {
        let text = resolve_pseudo_content(dom, entity, state);
        let _ = dom.world_mut().insert_one(entity, TextContent(text));
    }

    // Recurse children in composed document order (::before is the first child,
    // ::after the last — so each resolves with the right counter state).
    for child in dom.composed_children(entity) {
        resolve_tree(dom, child, state, depth + 1);
    }

    state.pop_scope();
}

/// Resolve a pseudo-element's `content` value (CSS Content 3 §2).
///
/// `attr()` (CSS Content 3 §2.1) refers to an attribute of the **originating
/// element** — the pseudo's parent — not the pseudo entity itself.
fn resolve_pseudo_content(dom: &EcsDom, entity: Entity, state: &CounterState) -> String {
    let Ok(style) = dom.world().get::<&ComputedStyle>(entity) else {
        return String::new();
    };
    let ContentValue::Items(items) = &style.content else {
        return String::new();
    };
    let originating = dom.get_parent(entity);
    let mut result = String::new();
    for item in items {
        match item {
            ContentItem::String(s) => result.push_str(s),
            ContentItem::Attr(name) => {
                if let Some(orig) = originating {
                    if let Ok(attrs) = dom.world().get::<&Attributes>(orig) {
                        if let Some(val) = attrs.get(name) {
                            result.push_str(val);
                        }
                    }
                }
            }
            ContentItem::Counter { name, style: ls } => {
                let _ = write!(result, "{}", state.evaluate_counter(name, *ls));
            }
            ContentItem::Counters {
                name,
                separator,
                style: ls,
            } => {
                let _ = write!(result, "{}", state.evaluate_counters(name, separator, *ls));
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_ecs::Attributes;
    use elidex_plugin::{CounterResetEntry, ListStyleType};

    /// Build `<ol>` with `n` `<li>` children, each `display: list-item` decimal.
    /// Returns `(dom, ol, [li…])`. Counter props are implicit (set by the pass via
    /// `apply_implicit_list_counters` from the ol/li tags).
    fn ol_with_items(n: usize) -> (EcsDom, Entity, Vec<Entity>) {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let ol = dom.create_element("ol", Attributes::default());
        let _ = dom.world_mut().insert_one(
            ol,
            ComputedStyle {
                display: Display::Block,
                ..Default::default()
            },
        );
        let _ = dom.append_child(root, ol);
        let mut lis = Vec::new();
        for _ in 0..n {
            let li = dom.create_element("li", Attributes::default());
            let _ = dom.world_mut().insert_one(
                li,
                ComputedStyle {
                    display: Display::ListItem,
                    list_style_type: ListStyleType::Decimal,
                    ..Default::default()
                },
            );
            let _ = dom.append_child(ol, li);
            lis.push(li);
        }
        (dom, ol, lis)
    }

    fn marker(dom: &EcsDom, e: Entity) -> Option<String> {
        dom.world()
            .get::<&elidex_ecs::ListItemMarker>(e)
            .ok()
            .map(|m| m.0.clone())
    }

    #[test]
    fn list_item_markers_number_in_document_order() {
        let (mut dom, _ol, lis) = ol_with_items(3);
        resolve_generated_content(&mut dom);
        assert_eq!(marker(&dom, lis[0]).as_deref(), Some("1"));
        assert_eq!(marker(&dom, lis[1]).as_deref(), Some("2"));
        assert_eq!(marker(&dom, lis[2]).as_deref(), Some("3"));
    }

    #[test]
    fn display_none_li_skips_counter_css_lists_4_5() {
        // CSS Lists 3 §4.5: a display:none element cannot increment the counter.
        let (mut dom, _ol, lis) = ol_with_items(3);
        // Make the middle li display:none.
        let _ = dom.world_mut().insert_one(
            lis[1],
            ComputedStyle {
                display: Display::None,
                list_style_type: ListStyleType::Decimal,
                ..Default::default()
            },
        );
        resolve_generated_content(&mut dom);
        assert_eq!(marker(&dom, lis[0]).as_deref(), Some("1"));
        // The display:none li gets no marker and does NOT advance the counter…
        assert_eq!(marker(&dom, lis[1]), None, "display:none li → no marker");
        // …so the third li is "2", not "3".
        assert_eq!(
            marker(&dom, lis[2]).as_deref(),
            Some("2"),
            "display:none li must not increment list-item (§4.5)"
        );
    }

    #[test]
    fn marker_reconciled_removed_when_no_longer_list_item() {
        let (mut dom, _ol, lis) = ol_with_items(1);
        resolve_generated_content(&mut dom);
        assert_eq!(marker(&dom, lis[0]).as_deref(), Some("1"));
        // The li becomes a plain block (e.g. display change) — re-resolve must
        // clear the stale ListItemMarker (slice-1 reconcile discipline).
        let _ = dom.world_mut().insert_one(
            lis[0],
            ComputedStyle {
                display: Display::Block,
                ..Default::default()
            },
        );
        resolve_generated_content(&mut dom);
        assert_eq!(
            marker(&dom, lis[0]),
            None,
            "stale ListItemMarker must be removed on gate-out"
        );
    }

    #[test]
    fn marker_removed_when_element_becomes_display_none() {
        // Copilot PR#273 R2: the reconcile-remove must fire even on the
        // display:none / display:contents early-return paths (the remove now runs
        // before those returns).
        for hidden in [Display::None, Display::Contents] {
            let (mut dom, _ol, lis) = ol_with_items(1);
            resolve_generated_content(&mut dom);
            assert_eq!(marker(&dom, lis[0]).as_deref(), Some("1"));
            let _ = dom.world_mut().insert_one(
                lis[0],
                ComputedStyle {
                    display: hidden,
                    ..Default::default()
                },
            );
            resolve_generated_content(&mut dom);
            assert_eq!(
                marker(&dom, lis[0]),
                None,
                "stale marker must be removed when the element becomes {hidden:?}"
            );
        }
    }

    #[test]
    fn counters_independent_across_roots() {
        // Copilot PR#273 R2: a fresh CounterState per root — disconnected trees are
        // independent documents, so `list-item` must restart, not leak across them.
        let mut dom = EcsDom::new();
        let mut lis = Vec::new();
        for _ in 0..2 {
            // Parentless <ol> → a separate root (find_roots fallback scan).
            let ol = dom.create_element("ol", Attributes::default());
            let _ = dom.world_mut().insert_one(
                ol,
                ComputedStyle {
                    display: Display::Block,
                    ..Default::default()
                },
            );
            let li = dom.create_element("li", Attributes::default());
            let _ = dom.world_mut().insert_one(
                li,
                ComputedStyle {
                    display: Display::ListItem,
                    list_style_type: ListStyleType::Decimal,
                    ..Default::default()
                },
            );
            let _ = dom.append_child(ol, li);
            lis.push(li);
        }
        resolve_generated_content(&mut dom);
        assert_eq!(marker(&dom, lis[0]).as_deref(), Some("1"));
        assert_eq!(
            marker(&dom, lis[1]).as_deref(),
            Some("1"),
            "second root's list-item must restart at 1 (no cross-root counter leak)"
        );
    }

    #[test]
    fn nested_ol_resets_inner_counter() {
        // <ol><li/><li><ol><li/></ol></li></ol> — inner li restarts at 1.
        let (mut dom, _ol, lis) = ol_with_items(2);
        let inner_ol = dom.create_element("ol", Attributes::default());
        let _ = dom.world_mut().insert_one(
            inner_ol,
            ComputedStyle {
                display: Display::Block,
                ..Default::default()
            },
        );
        let _ = dom.append_child(lis[1], inner_ol);
        let inner_li = dom.create_element("li", Attributes::default());
        let _ = dom.world_mut().insert_one(
            inner_li,
            ComputedStyle {
                display: Display::ListItem,
                list_style_type: ListStyleType::Decimal,
                ..Default::default()
            },
        );
        let _ = dom.append_child(inner_ol, inner_li);
        resolve_generated_content(&mut dom);
        assert_eq!(marker(&dom, lis[0]).as_deref(), Some("1"));
        assert_eq!(marker(&dom, lis[1]).as_deref(), Some("2"));
        assert_eq!(
            marker(&dom, inner_li).as_deref(),
            Some("1"),
            "nested <ol> resets the list-item counter (§4.3 scope)"
        );
    }

    /// Build `<p>` with a `::before` pseudo whose `content` is `items`. Returns
    /// `(dom, pseudo)`. The originating `<p>` carries `attrs` for attr() tests.
    fn p_with_before(items: Vec<ContentItem>, attrs: Attributes) -> (EcsDom, Entity) {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let p = dom.create_element("p", attrs);
        let _ = dom.world_mut().insert_one(
            p,
            ComputedStyle {
                display: Display::Block,
                ..Default::default()
            },
        );
        let _ = dom.append_child(root, p);
        // The pseudo entity, as the cascade would spawn it (empty text + marker).
        let pseudo = dom.create_text(String::new());
        let _ = dom.world_mut().insert_one(
            pseudo,
            ComputedStyle {
                display: Display::Inline,
                content: ContentValue::Items(items),
                ..Default::default()
            },
        );
        let _ = dom
            .world_mut()
            .insert_one(pseudo, elidex_ecs::PseudoElementMarker);
        let first = dom.get_first_child(p);
        if let Some(fc) = first {
            let _ = dom.insert_before(p, pseudo, fc);
        } else {
            let _ = dom.append_child(p, pseudo);
        }
        (dom, pseudo)
    }

    fn pseudo_text(dom: &EcsDom, e: Entity) -> String {
        dom.world()
            .get::<&TextContent>(e)
            .map(|t| t.0.clone())
            .unwrap_or_default()
    }

    #[test]
    fn pseudo_counter_content_resolved() {
        // p { counter-reset: c 4 } p::before { content: counter(c) }
        let (mut dom, pseudo) = p_with_before(
            vec![ContentItem::Counter {
                name: "c".to_string(),
                style: ListStyleType::Decimal,
            }],
            Attributes::default(),
        );
        // Put the counter-reset on the originating <p>.
        let p = dom.get_parent(pseudo).unwrap();
        let _ = dom.world_mut().insert_one(
            p,
            ComputedStyle {
                display: Display::Block,
                counter_reset: vec![CounterResetEntry::new("c", 4)],
                ..Default::default()
            },
        );
        resolve_generated_content(&mut dom);
        assert_eq!(pseudo_text(&dom, pseudo), "4");
    }

    #[test]
    fn pseudo_attr_resolves_against_originating_element() {
        let mut attrs = Attributes::default();
        attrs.set("data-x", "hi");
        let (mut dom, pseudo) = p_with_before(vec![ContentItem::Attr("data-x".to_string())], attrs);
        resolve_generated_content(&mut dom);
        assert_eq!(
            pseudo_text(&dom, pseudo),
            "hi",
            "attr() in pseudo content resolves against the originating element"
        );
    }
}
