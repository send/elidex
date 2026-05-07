//! Label association for form controls.

use elidex_ecs::{Attributes, EcsDom, Entity, TagType, MAX_ANCESTOR_DEPTH};

use crate::FormControlState;

/// Returns `true` if the entity is a "labelable element" per HTML
/// §4.10.4 — `<button>`, `<input>` (any non-hidden type),
/// `<meter>`, `<output>`, `<progress>`, `<select>`, `<textarea>`.
/// Tag-based check rather than `FormControlState`-based so a
/// JS-created element (`document.createElement('input')`) without
/// the side-table component still resolves correctly.  Falls back
/// to the `type` content attribute (ASCII-CI) for the
/// `<input type=hidden>` exclusion when no `FormControlState` is
/// attached.
#[must_use]
pub fn is_labelable_element(dom: &EcsDom, entity: Entity) -> bool {
    let Ok(tag) = dom.world().get::<&TagType>(entity) else {
        return false;
    };
    let tag_str = tag.0.as_str();
    // ASCII-case-insensitive: HTML parser already lowers, but
    // `EcsDom::create_element` is reachable from non-parser callers
    // (tests, internal builders) and `is_labelable_element` is exposed
    // for those paths, so tolerate uppercase / mixed case.
    let is_input = tag_str.eq_ignore_ascii_case("input");
    if !is_input
        && ![
            "button", "meter", "output", "progress", "select", "textarea",
        ]
        .iter()
        .any(|labelable| tag_str.eq_ignore_ascii_case(labelable))
    {
        return false;
    }
    if is_input {
        // `<input type=hidden>` is explicitly NOT labelable.  Prefer
        // `FormControlState.kind` (already ASCII-lowered at attach
        // time); fall back to the raw content attribute for
        // JS-created inputs without state.
        drop(tag);
        if let Ok(state) = dom.world().get::<&FormControlState>(entity) {
            return state.kind != crate::FormControlKind::Hidden;
        }
        if let Ok(attrs) = dom.world().get::<&Attributes>(entity) {
            if attrs
                .get("type")
                .is_some_and(|v| v.eq_ignore_ascii_case("hidden"))
            {
                return false;
            }
        }
    }
    true
}

/// Resolve the `for` attribute of a `<label>` to a target form
/// control entity.  WHATWG HTML §4.10.4: returns the first
/// **labelable** element in tree order within the label's tree
/// whose `id` matches.  Hidden inputs are explicitly not labelable
/// (`is_labelable_element` excludes them), so a hidden input with
/// the matching id is rejected even though it carries
/// `FormControlState`.
#[must_use]
pub fn resolve_label_for(dom: &EcsDom, label_entity: Entity) -> Option<Entity> {
    let for_id: String = {
        let attrs = dom.world().get::<&Attributes>(label_entity).ok()?;
        let v = attrs.get("for")?;
        if v.is_empty() {
            return None;
        }
        v.to_owned()
    };

    // Pre-order DFS over the label's tree (root inclusive) seeking
    // the FIRST entity that has the matching id AND is labelable.
    //
    // The previous implementation used `EcsDom::find_by_id`, which
    // returns the first id-match regardless of labelable status —
    // that masked tree-order valid matches when an earlier
    // non-labelable element shared the same id (HTML treats
    // duplicate ids as malformed markup but browsers fall back to
    // returning the first labelable match in tree order).
    //
    // `find_tree_root` returns the label itself when detached, so
    // detached labels also work.  `traverse_descendants` skips
    // `root` itself, so we check `root` explicitly before walking.
    let root = dom.find_tree_root(label_entity);
    if matches_id_and_labelable(dom, root, for_id.as_str()) {
        return Some(root);
    }
    let mut candidate = None;
    dom.traverse_descendants(root, |entity| {
        if matches_id_and_labelable(dom, entity, for_id.as_str()) {
            candidate = Some(entity);
            return false;
        }
        true
    });
    candidate
}

fn matches_id_and_labelable(dom: &EcsDom, entity: Entity, id: &str) -> bool {
    dom.world()
        .get::<&Attributes>(entity)
        .is_ok_and(|a| a.get("id") == Some(id))
        && is_labelable_element(dom, entity)
}

/// Find the first descendant labelable element of a label element.
///
/// Used when `<label>` wraps a control without a `for` attribute.
#[must_use]
pub fn find_label_target(dom: &EcsDom, label_entity: Entity) -> Option<Entity> {
    // First check for explicit `for` attribute.
    if let Some(target) = resolve_label_for(dom, label_entity) {
        return Some(target);
    }

    // Otherwise, search descendants for the first labelable element.
    find_first_descendant_control(dom, label_entity, 0)
}

/// Recursively find the first descendant that is a labelable element
/// per HTML §4.10.4.  Hidden inputs (`<input type=hidden>`) are NOT
/// labelable even though they carry `FormControlState`, so the
/// `is_labelable_element` predicate (which excludes them) is the
/// authoritative check — no FormControlState fallback.
fn find_first_descendant_control(dom: &EcsDom, entity: Entity, depth: usize) -> Option<Entity> {
    if depth >= MAX_ANCESTOR_DEPTH {
        return None;
    }
    let mut child = dom.get_first_child(entity)?;
    loop {
        if is_labelable_element(dom, child) {
            return Some(child);
        }
        // Recurse into subtree.
        if let Some(found) = find_first_descendant_control(dom, child, depth + 1) {
            return Some(found);
        }
        child = dom.get_next_sibling(child)?;
    }
}

/// Check if an entity is a `<label>` element.
#[must_use]
pub fn is_label(dom: &EcsDom, entity: Entity) -> bool {
    dom.world()
        .get::<&TagType>(entity)
        .is_ok_and(|t| t.0 == "label")
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_ecs::EcsDom;

    #[test]
    fn resolve_label_for_attribute() {
        let mut dom = EcsDom::new();

        // HTML §4.10.4 restricts `for=` lookup to the same tree as
        // the label.  Build a tiny shared container so both nodes
        // share a tree root.
        let container = dom.create_element("div", Attributes::default());
        let mut input_attrs = Attributes::default();
        input_attrs.set("id", "name");
        let input = dom.create_element("input", input_attrs.clone());
        let _ = dom.world_mut().insert_one(
            input,
            FormControlState::from_element("input", &input_attrs).unwrap(),
        );
        let _ = dom.append_child(container, input);

        let mut label_attrs = Attributes::default();
        label_attrs.set("for", "name");
        let label = dom.create_element("label", label_attrs);
        let _ = dom.append_child(container, label);

        assert_eq!(resolve_label_for(&dom, label), Some(input));
    }

    #[test]
    fn resolve_label_for_returns_none_when_not_in_same_tree() {
        // HTML §4.10.4 — `for=` must resolve to an entity in the
        // same tree as the label.  Detached label + detached target
        // share no tree, so the lookup returns None even when an `id`
        // match exists somewhere else in the world.
        let mut dom = EcsDom::new();
        let mut input_attrs = Attributes::default();
        input_attrs.set("id", "name");
        let _input = dom.create_element("input", input_attrs);

        let mut label_attrs = Attributes::default();
        label_attrs.set("for", "name");
        let label = dom.create_element("label", label_attrs);

        assert_eq!(resolve_label_for(&dom, label), None);
    }

    #[test]
    fn resolve_label_for_rejects_hidden_input_target() {
        // R11 F1 regression — `<input type=hidden>` carries
        // `FormControlState` but is explicitly NOT labelable per
        // HTML §4.10.4.  The pre-fix FormControlState fallback
        // would resolve `<label for="hidden">` to the hidden
        // input; the new strict-labelable predicate must reject.
        let mut dom = EcsDom::new();
        let container = dom.create_element("div", Attributes::default());

        let mut input_attrs = Attributes::default();
        input_attrs.set("id", "hidden");
        input_attrs.set("type", "hidden");
        let input = dom.create_element("input", input_attrs.clone());
        let _ = dom.world_mut().insert_one(
            input,
            FormControlState::from_element("input", &input_attrs).unwrap(),
        );
        let _ = dom.append_child(container, input);

        let mut label_attrs = Attributes::default();
        label_attrs.set("for", "hidden");
        let label = dom.create_element("label", label_attrs);
        let _ = dom.append_child(container, label);

        assert_eq!(resolve_label_for(&dom, label), None);
    }

    #[test]
    fn find_label_target_descendant_skips_hidden_input() {
        // R11 F2 regression — `find_first_descendant_control` must
        // also use the labelable predicate exclusively (no
        // FormControlState fallback) so a wrapping label with a
        // hidden input as descendant doesn't pick it up.
        let mut dom = EcsDom::new();
        let label = dom.create_element("label", Attributes::default());

        let mut input_attrs = Attributes::default();
        input_attrs.set("type", "hidden");
        let hidden = dom.create_element("input", input_attrs.clone());
        let _ = dom.world_mut().insert_one(
            hidden,
            FormControlState::from_element("input", &input_attrs).unwrap(),
        );
        let _ = dom.append_child(label, hidden);

        // Append a real labelable later so we can verify the
        // hidden input is skipped, not just absent.
        let textarea_attrs = Attributes::default();
        let textarea = dom.create_element("textarea", textarea_attrs);
        let _ = dom.append_child(label, textarea);

        assert_eq!(find_label_target(&dom, label), Some(textarea));
    }

    #[test]
    fn resolve_label_for_skips_non_labelable_when_id_collision() {
        // R12 F1 regression — when the tree root (or any earlier
        // entity in tree order) carries the matching id but isn't
        // labelable, the walker must continue rather than treating
        // the non-labelable id-match as the final candidate.
        // Spec is "first labelable element in tree order whose id
        // matches" (HTML §4.10.4) — even with duplicate ids
        // (malformed markup) browsers return the first labelable.
        let mut dom = EcsDom::new();
        // Root <div id="foo"> — has matching id but not labelable.
        let mut div_attrs = Attributes::default();
        div_attrs.set("id", "foo");
        let div_root = dom.create_element("div", div_attrs);
        // Descendant <input id="foo"> — also has the same id and IS
        // labelable.  Should be returned.
        let mut input_attrs = Attributes::default();
        input_attrs.set("id", "foo");
        let input = dom.create_element("input", input_attrs);
        let _ = dom.append_child(div_root, input);

        // Label sits inside the same tree.
        let mut label_attrs = Attributes::default();
        label_attrs.set("for", "foo");
        let label = dom.create_element("label", label_attrs);
        let _ = dom.append_child(div_root, label);

        assert_eq!(resolve_label_for(&dom, label), Some(input));
    }

    #[test]
    fn resolve_label_for_includes_tree_root() {
        // R8 F1 regression — `find_by_id` only searches descendants,
        // so when the label's `for=` target is the tree root itself
        // (a button containing the label), the lookup must
        // explicitly check the root before falling back to
        // descendant DFS.
        let mut dom = EcsDom::new();
        let mut button_attrs = Attributes::default();
        button_attrs.set("id", "btn");
        let button = dom.create_element("button", button_attrs);

        let mut label_attrs = Attributes::default();
        label_attrs.set("for", "btn");
        let label = dom.create_element("label", label_attrs);
        let _ = dom.append_child(button, label);

        assert_eq!(resolve_label_for(&dom, label), Some(button));
    }

    #[test]
    fn resolve_label_for_returns_first_in_document_order() {
        // HTML §4.10.4 — pre-order DFS, first labelable match wins.
        let mut dom = EcsDom::new();
        let container = dom.create_element("div", Attributes::default());

        let mut earlier = Attributes::default();
        earlier.set("id", "x");
        let first = dom.create_element("input", earlier);
        let _ = dom.append_child(container, first);

        // Sibling later in tree order also has the same id (invalid
        // markup, but the match must pick the first one).
        let mut later = Attributes::default();
        later.set("id", "x");
        let second = dom.create_element("textarea", later);
        let _ = dom.append_child(container, second);

        let mut label_attrs = Attributes::default();
        label_attrs.set("for", "x");
        let label = dom.create_element("label", label_attrs);
        let _ = dom.append_child(container, label);

        assert_eq!(resolve_label_for(&dom, label), Some(first));
    }

    #[test]
    fn find_label_target_wrapping() {
        let mut dom = EcsDom::new();

        let label = dom.create_element("label", Attributes::default());

        let input_attrs = Attributes::default();
        let input = dom.create_element("input", input_attrs.clone());
        let _ = dom.world_mut().insert_one(
            input,
            FormControlState::from_element("input", &input_attrs).unwrap(),
        );

        let _ = dom.append_child(label, input);

        assert_eq!(find_label_target(&dom, label), Some(input));
    }

    #[test]
    fn no_label_target() {
        let mut dom = EcsDom::new();
        let label = dom.create_element("label", Attributes::default());
        assert_eq!(find_label_target(&dom, label), None);
    }

    #[test]
    fn is_labelable_element_excludes_hidden_input_via_state() {
        let mut dom = EcsDom::new();
        let mut attrs = Attributes::default();
        attrs.set("type", "hidden");
        let input = dom.create_element("input", attrs.clone());
        let _ = dom.world_mut().insert_one(
            input,
            FormControlState::from_element("input", &attrs).unwrap(),
        );
        assert!(!is_labelable_element(&dom, input));
    }

    #[test]
    fn is_labelable_element_excludes_hidden_input_via_attribute() {
        // JS-created element without FormControlState — falls back
        // to the raw `type` attribute.
        let mut dom = EcsDom::new();
        let mut attrs = Attributes::default();
        attrs.set("type", "Hidden"); // ASCII-CI
        let input = dom.create_element("input", attrs);
        assert!(!is_labelable_element(&dom, input));
    }

    #[test]
    fn is_labelable_element_accepts_text_input_without_state() {
        let mut dom = EcsDom::new();
        let input = dom.create_element("input", Attributes::default());
        assert!(is_labelable_element(&dom, input));
    }

    #[test]
    fn is_labelable_element_accepts_button_select_textarea() {
        let mut dom = EcsDom::new();
        let button = dom.create_element("button", Attributes::default());
        let select = dom.create_element("select", Attributes::default());
        let textarea = dom.create_element("textarea", Attributes::default());
        assert!(is_labelable_element(&dom, button));
        assert!(is_labelable_element(&dom, select));
        assert!(is_labelable_element(&dom, textarea));
    }

    #[test]
    fn is_labelable_element_rejects_non_labelable_tags() {
        let mut dom = EcsDom::new();
        let div = dom.create_element("div", Attributes::default());
        let label = dom.create_element("label", Attributes::default());
        assert!(!is_labelable_element(&dom, div));
        assert!(!is_labelable_element(&dom, label));
    }

    #[test]
    fn is_labelable_element_ascii_ci_tag_match() {
        // Non-parser paths can store tags in mixed case; the matcher
        // tolerates that per the function's documented contract.
        let mut dom = EcsDom::new();
        let upper_input = dom.create_element("INPUT", Attributes::default());
        let mixed_button = dom.create_element("BuTToN", Attributes::default());
        let upper_textarea = dom.create_element("TEXTAREA", Attributes::default());
        assert!(is_labelable_element(&dom, upper_input));
        assert!(is_labelable_element(&dom, mixed_button));
        assert!(is_labelable_element(&dom, upper_textarea));
    }
}
