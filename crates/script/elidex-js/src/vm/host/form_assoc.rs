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

use elidex_ecs::Entity;

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
    let dom = ctx.host().dom();
    let form_id = dom.with_attribute(entity, "form", |v| {
        v.filter(|s| !s.is_empty()).map(String::from)
    });
    if let Some(id) = form_id {
        // IDREF lookup scoped to the entity's tree.  Fall back to
        // the topmost reachable ancestor when `owner_document` is
        // None (detached subtree).
        let root = dom.owner_document(entity).unwrap_or_else(|| {
            let mut cur = entity;
            let mut depth: u32 = 0;
            while let Some(p) = dom.get_parent(cur) {
                if depth > MAX_ANCESTOR_DEPTH {
                    break;
                }
                cur = p;
                depth += 1;
            }
            cur
        });
        if let Some(target) = dom.find_by_id(root, &id) {
            if dom.has_tag(target, "form") {
                return Some(target);
            }
        }
        // Spec: if the form= IDREF fails to name a form element,
        // there is no association — do NOT fall back to the
        // ancestor walk.
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
/// Result is in document order with id-matched and ancestor-matched
/// labels merged.  Used by HTMLButtonElement / HTMLInputElement /
/// HTMLSelectElement / HTMLTextAreaElement / HTMLOutputElement /
/// HTMLMeterElement / HTMLProgressElement (i.e. every labelable
/// element per HTML §4.10.2).
pub(super) fn collect_labels_for(ctx: &mut NativeContext<'_>, control: Entity) -> Vec<Entity> {
    let dom = ctx.host().dom();
    let mut result: Vec<Entity> = Vec::new();

    // Pass 1 — id-based association.  Read the control's id once,
    // then walk owner document for `<label for="<id>">`.
    let control_id = dom.with_attribute(control, "id", |v| {
        v.filter(|s| !s.is_empty()).map(String::from)
    });
    if let Some(id) = control_id {
        // Scope the search to the control's tree — owner_document
        // when attached, otherwise the topmost ancestor (matches
        // the same scoping rule used by `resolve_form_association`).
        let root = dom.owner_document(control).unwrap_or_else(|| {
            let mut cur = control;
            let mut depth: u32 = 0;
            while let Some(p) = dom.get_parent(cur) {
                if depth > MAX_ANCESTOR_DEPTH {
                    break;
                }
                cur = p;
                depth += 1;
            }
            cur
        });
        dom.traverse_descendants(root, |e| {
            if e == root {
                return true;
            }
            if dom.has_tag(e, "label") {
                let matches = dom.with_attribute(e, "for", |v| v == Some(id.as_str()));
                if matches {
                    result.push(e);
                }
            }
            true
        });
    }

    // Pass 2 — ancestor `<label>` whose `for=` is absent or empty.
    let mut cur = dom.get_parent(control);
    let mut depth: u32 = 0;
    while let Some(p) = cur {
        if depth > MAX_ANCESTOR_DEPTH {
            break;
        }
        if dom.has_tag(p, "label") {
            let for_attr = dom.with_attribute(p, "for", |v| v.map(String::from));
            let no_for = match for_attr {
                None => true,
                Some(s) => s.is_empty(),
            };
            if no_for && !result.contains(&p) {
                result.push(p);
            }
        }
        cur = dom.get_parent(p);
        depth += 1;
    }

    result
}
