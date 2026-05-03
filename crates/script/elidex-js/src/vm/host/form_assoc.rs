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
