//! `<details>.name` multi-disclosure exclusion algorithm
//! (HTML §4.11.1 — name attribute change steps).
//!
//! When a `<details>` element with a non-empty `name` attribute opens,
//! every other `<details>` in the same tree (document or shadow root)
//! whose `name` attribute is **byte-for-byte equal** to the opening
//! element's name attribute must auto-close.  The opener also fires a
//! `toggle` event on each closed sibling (`oldState="open"`,
//! `newState="closed"`) — that dispatch lives in the VM-side
//! `dispatch_toggle_event` helper; this module only walks the tree
//! and returns the set of siblings to close.
//!
//! ## Layering
//!
//! Pure-read tree walk over `&EcsDom` — no mutation, no script-side
//! API surface.  Engine-bound code (`html_details_proto.rs::details_set_open`)
//! calls this directly + applies attribute mutations + dispatches
//! ToggleEvents on each returned entity.  Per the CLAUDE.md "Layering
//! mandate" DOM tree walking belongs in engine-indep crates.
//!
//! ## Equality semantics (load-bearing)
//!
//! HTML §4.11.1 says the `name` attribute equality is **byte-for-byte**
//! (NOT ASCII-case-insensitive, NOT case-folded).  `name=g` and
//! `name=G` are distinct accordion groups.  Rust `&str` `==` on byte
//! slices already implements this; tests cover the case-distinct
//! requirement explicitly.
//!
//! ## Empty / missing name
//!
//! Per spec, `<details>` with empty or absent `name` does NOT
//! participate in exclusion: any number of nameless `<details>` can
//! be open simultaneously.  The walker exits early when `name` is
//! empty so the caller doesn't need to special-case the no-name path.
//!
//! ## Snapshot semantics
//!
//! The returned `Vec<Entity>` is a snapshot.  The caller iterates the
//! snapshot, removing the `open` attribute and dispatching a
//! ToggleEvent on each sibling.  If a ToggleEvent listener mutates
//! the DOM (e.g. opens another `<details name=X>` from inside the
//! handler), the new opening triggers its own exclusion walk — but
//! the snapshot insulates the in-flight close loop from
//! re-entry on already-collected entities.

use elidex_ecs::{Attributes, EcsDom, Entity, TagType};

/// Walk the inclusive descendants of `root`, collecting open
/// `<details>` entities whose `name` attribute byte-equals `name` and
/// which are not `exclude` (the opening element itself).
///
/// Returns `Vec::new()` when `name.is_empty()` — empty-named
/// `<details>` do not participate in exclusion per HTML §4.11.1.
///
/// `exclude` lets the caller pass the opening element's own entity so
/// it doesn't appear in the close-list (the caller fires ToggleEvent
/// on itself separately, AFTER closing siblings).
#[must_use]
pub fn collect_open_details_by_name(
    dom: &EcsDom,
    root: Entity,
    name: &str,
    exclude: Entity,
) -> Vec<Entity> {
    if name.is_empty() {
        return Vec::new();
    }
    let mut result = Vec::new();
    walk_inclusive(dom, root, &mut |entity| {
        if entity == exclude {
            return;
        }
        // Tag check first — cheap discriminator before attribute lookup.
        let Ok(tag) = dom.world().get::<&TagType>(entity) else {
            return;
        };
        if !tag.0.eq_ignore_ascii_case("details") {
            return;
        }
        let Ok(attrs) = dom.world().get::<&Attributes>(entity) else {
            return;
        };
        // Open siblings only — closed `<details>` carry no `open`
        // attribute per HTML §4.11.1, so absent attribute === closed.
        if attrs.get("open").is_none() {
            return;
        }
        // Byte-for-byte name comparison (NOT ASCII-CI / NOT
        // case-folded).  `name=g` and `name=G` are distinct groups.
        if attrs.get("name") != Some(name) {
            return;
        }
        result.push(entity);
    });
    result
}

/// Generic inclusive-descendants pre-order walker used by
/// [`collect_open_details_by_name`].  Local to this module — the
/// existing `tree.rs` walkers (selector matching / collect-text) all
/// have caller-specific filters, so a small ad-hoc walker keeps this
/// helper self-contained.
///
/// **Iterative, NOT recursive.**  DOM trees in this codebase can be up
/// to [`elidex_ecs::MAX_ANCESTOR_DEPTH`] (10000) deep, which would blow
/// the default Rust thread stack with a recursive walker (~100 bytes
/// per frame × 10000 frames ≈ 1 MB; safe in isolation but susceptible
/// to overflow when invoked deep inside an existing JS / dispatch call
/// stack).  Explicit `Vec` stack with reverse-push of children
/// preserves pre-order semantics: pop yields the next node in
/// document-order, children pushed in reverse so the leftmost child
/// pops first.
fn walk_inclusive(dom: &EcsDom, root: Entity, visit: &mut impl FnMut(Entity)) {
    let mut stack: Vec<Entity> = vec![root];
    while let Some(entity) = stack.pop() {
        visit(entity);
        // Collect children first, then push in reverse so the leftmost
        // child is on top of the stack (pops first → pre-order walk).
        // Allocating a per-node Vec is wasteful for shallow trees but
        // keeps the iterative invariant simple; in practice the DOM
        // walker fires only on `<details>.open` mutation paths, not
        // hot-loop scenarios.
        let children: Vec<Entity> = dom.children_iter(entity).collect();
        for child in children.into_iter().rev() {
            stack.push(child);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_ecs::{Attributes, EcsDom};

    fn make_details(dom: &mut EcsDom, name: Option<&str>, open: bool) -> Entity {
        let entity = dom.create_element("details", Attributes::default());
        if let Some(n) = name {
            dom.set_attribute(entity, "name", n);
        }
        if open {
            dom.set_attribute(entity, "open", "");
        }
        entity
    }

    fn make_div_parent(dom: &mut EcsDom, children: &[Entity]) -> Entity {
        let parent = dom.create_element("div", Attributes::default());
        for &child in children {
            assert!(dom.append_child(parent, child));
        }
        parent
    }

    #[test]
    fn empty_name_returns_empty_vec() {
        let mut dom = EcsDom::new();
        let opener = make_details(&mut dom, None, false);
        let other = make_details(&mut dom, None, true);
        let parent = make_div_parent(&mut dom, &[opener, other]);
        let result = collect_open_details_by_name(&dom, parent, "", opener);
        assert!(result.is_empty(), "empty name → no exclusion");
    }

    #[test]
    fn exclude_self_from_collection() {
        let mut dom = EcsDom::new();
        let opener = make_details(&mut dom, Some("g"), true);
        let parent = make_div_parent(&mut dom, &[opener]);
        let result = collect_open_details_by_name(&dom, parent, "g", opener);
        assert!(result.is_empty(), "opener itself must not be collected");
    }

    #[test]
    fn collects_open_sibling_with_matching_name() {
        let mut dom = EcsDom::new();
        let opener = make_details(&mut dom, Some("g"), false);
        let sibling = make_details(&mut dom, Some("g"), true);
        let parent = make_div_parent(&mut dom, &[opener, sibling]);
        let result = collect_open_details_by_name(&dom, parent, "g", opener);
        assert_eq!(result, vec![sibling]);
    }

    #[test]
    fn ignores_closed_sibling_with_matching_name() {
        let mut dom = EcsDom::new();
        let opener = make_details(&mut dom, Some("g"), false);
        let closed_sibling = make_details(&mut dom, Some("g"), false);
        let parent = make_div_parent(&mut dom, &[opener, closed_sibling]);
        let result = collect_open_details_by_name(&dom, parent, "g", opener);
        assert!(
            result.is_empty(),
            "closed sibling must not be in the close-list"
        );
    }

    #[test]
    fn ignores_open_sibling_with_different_name() {
        let mut dom = EcsDom::new();
        let opener = make_details(&mut dom, Some("g"), false);
        let other_group = make_details(&mut dom, Some("h"), true);
        let parent = make_div_parent(&mut dom, &[opener, other_group]);
        let result = collect_open_details_by_name(&dom, parent, "g", opener);
        assert!(result.is_empty(), "different-name sibling stays open");
    }

    #[test]
    fn name_equality_is_byte_exact_not_case_insensitive() {
        let mut dom = EcsDom::new();
        let opener = make_details(&mut dom, Some("g"), false);
        let upper_sibling = make_details(&mut dom, Some("G"), true);
        let parent = make_div_parent(&mut dom, &[opener, upper_sibling]);
        let result = collect_open_details_by_name(&dom, parent, "g", opener);
        assert!(
            result.is_empty(),
            "name=G must not match name=g (byte equality, not ASCII-CI)"
        );
    }

    #[test]
    fn ignores_non_details_elements() {
        let mut dom = EcsDom::new();
        let opener = make_details(&mut dom, Some("g"), false);
        // A non-`<details>` element with name=g + open attr should NOT
        // participate.  Tag check guards against grouping/section
        // elements that happen to share the attribute names.
        let div = dom.create_element("div", Attributes::default());
        dom.set_attribute(div, "name", "g");
        dom.set_attribute(div, "open", "");
        let parent = make_div_parent(&mut dom, &[opener, div]);
        let result = collect_open_details_by_name(&dom, parent, "g", opener);
        assert!(result.is_empty(), "non-details elements ignored");
    }

    #[test]
    fn collects_descendant_not_just_sibling() {
        // `<div><div><details name=g open></details></div></div>`
        // — exclusion walk descends through the wrapper.
        let mut dom = EcsDom::new();
        let opener = make_details(&mut dom, Some("g"), false);
        let nested = make_details(&mut dom, Some("g"), true);
        let inner_div = make_div_parent(&mut dom, &[nested]);
        let outer_div = make_div_parent(&mut dom, &[opener, inner_div]);
        let result = collect_open_details_by_name(&dom, outer_div, "g", opener);
        assert_eq!(result, vec![nested]);
    }

    #[test]
    fn deep_nesting_does_not_overflow_stack() {
        // Regression for R4 IMP: pre-fix the walker was recursive and
        // would overflow the thread stack at deep nesting.  Build a
        // 5000-deep `<div>` chain with the opener at the bottom, and
        // assert the walker completes without panic.  5000 is well
        // under MAX_ANCESTOR_DEPTH (10000) but enough to overflow the
        // recursive variant on a default-stack thread.
        let mut dom = EcsDom::new();
        let opener = make_details(&mut dom, Some("g"), false);
        // Wrap opener in 5000 nested divs.
        let mut current = opener;
        for _ in 0..5000 {
            let outer = dom.create_element("div", Attributes::default());
            assert!(dom.append_child(outer, current));
            current = outer;
        }
        let result = collect_open_details_by_name(&dom, current, "g", opener);
        // No matching open siblings — opener is the only details, and
        // it's the exclude.
        assert!(result.is_empty(), "deep walk completed without overflow");
    }

    #[test]
    fn details_tag_match_is_case_insensitive() {
        // ECS stores the tag verbatim from the parser; HTML element
        // names are ASCII-case-insensitive per WHATWG DOM, so the
        // walker uses `eq_ignore_ascii_case`.  Verify uppercase tag
        // is recognised.
        let mut dom = EcsDom::new();
        let opener = make_details(&mut dom, Some("g"), false);
        let upper_tag = dom.create_element("DETAILS", Attributes::default());
        dom.set_attribute(upper_tag, "name", "g");
        dom.set_attribute(upper_tag, "open", "");
        let parent = make_div_parent(&mut dom, &[opener, upper_tag]);
        let result = collect_open_details_by_name(&dom, parent, "g", opener);
        assert_eq!(result, vec![upper_tag]);
    }
}
