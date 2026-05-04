//! Shared form-association resolver — HTML §4.10.18.3.
//!
//! Form-associated elements (button / fieldset / input / object /
//! output / select / textarea) determine their form owner via:
//!
//! 1. The `form="<id>"` content attribute (when present and
//!    non-empty) names the form by id within the element's tree.
//!    A failed lookup OR an id that resolves to a non-form
//!    element yields `None` — there is **no** ancestor-walk
//!    fallback in this branch (per spec).
//! 2. Otherwise, the form is the nearest `<form>` ancestor.
//!
//! Centralised here so HTMLFormElement subclass prototypes
//! (HTMLFieldSetElement / HTMLButtonElement / HTMLInputElement /
//! HTMLSelectElement / HTMLTextAreaElement / HTMLOutputElement) all
//! observe the same algorithm — keeping the cross-element behaviour
//! consistent without N copies of the ancestor walk.

#![cfg(feature = "engine")]

use super::super::value::NativeContext;

use elidex_ecs::{EcsDom, Entity};

/// Maximum ancestor depth before the walk gives up — guards against
/// pathological / cyclic trees.
const MAX_ANCESTOR_DEPTH: u32 = 1024;

/// Resolve the form association of a form-associated `entity` per
/// HTML §4.10.18.3.  Returns `Some(form)` when a `<form>` is named
/// (or found by ancestor walk), `None` when there is no association.
pub(super) fn resolve_form_association(
    ctx: &mut NativeContext<'_>,
    entity: Entity,
) -> Option<Entity> {
    resolve_form_owner_dom(ctx.host().dom(), entity)
}

/// `EcsDom`-only form-owner resolver — same algorithm as
/// [`resolve_form_association`] but runs without a
/// [`NativeContext`].  Used by [`super::dom_collection`]'s
/// `FormControls` walker, which already holds the disjoint
/// `&EcsDom` borrow that the `NativeContext` split-field pattern
/// rules out at that site.
pub(super) fn resolve_form_owner_dom(dom: &EcsDom, entity: Entity) -> Option<Entity> {
    let form_attr = dom.with_attribute(entity, "form", |v| v.map(String::from));
    if let Some(id) = form_attr {
        // HTML §4.10.18.3 — when the `form` content attribute is
        // PRESENT (regardless of value, including the empty string),
        // the ancestor fallback is suppressed.  An empty IDREF can
        // never match by id-equality so the lookup returns `None`,
        // and we surface that as "no association" without falling
        // through.
        if !id.is_empty() {
            // IDREF lookup scoped to the entity's actual physical
            // tree root — `find_tree_root` returns the doc for
            // attached entities and the topmost detached ancestor
            // otherwise.
            let root = dom.find_tree_root(entity);
            if let Some(target) = dom.find_by_id(root, &id) {
                if dom.has_tag(target, "form") {
                    return Some(target);
                }
            }
        }
        return None;
    }

    // No form= attribute — climb the ancestor chain for the
    // nearest `<form>`.
    let mut cur = dom.get_parent(entity);
    let mut depth: u32 = 0;
    while let Some(p) = cur {
        if depth > MAX_ANCESTOR_DEPTH {
            return None;
        }
        if dom.has_tag(p, "form") {
            return Some(p);
        }
        cur = dom.get_parent(p);
        depth += 1;
    }
    None
}

/// Collect every `<label>` element associated with `control` per
/// HTML §4.10.4 (label / labelable element association):
///
/// 1. Labels whose `for=` IDREF resolves to `control` (id-based
///    association — requires `control` to have a non-empty `id`).
/// 2. Labels that are ancestors of `control` AND have no `for=`
///    attribute (descendant-control association — the label
///    "wraps" the control).
///
/// Result is in **document (tree) order** — `.labels.item(0)` is
/// the first matching label encountered in a pre-order descendant
/// walk of the control's root, regardless of which of the two
/// association forms matched.  Used by HTMLButtonElement /
/// HTMLInputElement / HTMLSelectElement / HTMLTextAreaElement /
/// HTMLOutputElement / HTMLMeterElement / HTMLProgressElement
/// (i.e. every labelable element per HTML §4.10.2).
pub(super) fn collect_labels_for(ctx: &mut NativeContext<'_>, control: Entity) -> Vec<Entity> {
    let dom = ctx.host().dom();
    let mut result: Vec<Entity> = Vec::new();

    let control_id = dom.with_attribute(control, "id", |v| {
        v.filter(|s| !s.is_empty()).map(String::from)
    });
    let id_str = control_id.as_deref();

    // For wrapping-label (Form 2) association the spec says the
    // label must be the FIRST `<label>` ancestor — when labels are
    // nested (`<label><label><input></label></label>`), only the
    // innermost wrapping label claims the control (HTML §4.10.4
    // "first such ancestor" clause).  Pre-compute that innermost
    // label by walking up from the control; later in the tree-order
    // pass we only push the label entity that equals it.
    let innermost_wrapping_label = {
        let mut cur = dom.get_parent(control);
        let mut depth: u32 = 0;
        let mut found: Option<Entity> = None;
        while let Some(p) = cur {
            if depth > MAX_ANCESTOR_DEPTH {
                break;
            }
            if dom.has_tag(p, "label") {
                let for_present = dom.with_attribute(p, "for", |v| v.is_some());
                if !for_present {
                    found = Some(p);
                }
                break;
            }
            cur = dom.get_parent(p);
            depth += 1;
        }
        found
    };

    // Single tree-order walk: every `<label>` in the control's tree
    // is classified once as either id-matched (Form 1) or the
    // innermost wrapping ancestor (Form 2).  Tree order is
    // preserved automatically because both forms collapse onto the
    // same pre-order walk.
    let root = dom.find_tree_root(control);
    dom.traverse_descendants(root, |e| {
        if e == root || !dom.has_tag(e, "label") {
            return true;
        }
        let for_attr = dom.with_attribute(e, "for", |v| v.map(String::from));
        match for_attr.as_deref() {
            Some(f) => {
                // HTML §4.10.4 — once `for=` is PRESENT (regardless
                // of value, including empty), the wrapping-ancestor
                // path is suppressed.  Empty for matches no id, so
                // the label is unassociated.
                if !f.is_empty() && id_str == Some(f) {
                    result.push(e);
                }
            }
            None => {
                // Form 2 — only the innermost wrapping label
                // (computed above) qualifies; outer label
                // ancestors are blocked by their inner sibling.
                if Some(e) == innermost_wrapping_label {
                    result.push(e);
                }
            }
        }
        true
    });

    result
}
