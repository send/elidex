//! Sync-construct entity-spawn helper for the `HTMLElement`
//! constructor's empty-construction-stack branch (\[C1\] §3.2.3
//! step 9: "internally create a new object implementing the
//! interface defined by the element interface").
//!
//! Mirrors the existing `EcsDom::create_element_with_owner` shape
//! (TagType / Attributes / TreeRelation / NodeKind::Element) but
//! atomically attaches [`CustomElementState`] in the `Custom`
//! lifecycle state so the returned entity is a fully-formed Custom
//! element ready to insert into the DOM. The sync-construct path
//! skips the upgrade algorithm entirely — \[C4\] §4.13.5 only runs
//! when there is a *pre-existing* parser-baked entity awaiting
//! upgrade. `new MyEl()` instead allocates a brand-new entity, so
//! `CEState::Custom` is the correct initial state.
//!
//! This helper is the single entity-creation site for sync-construct
//! custom elements — paralleling
//! `elidex_api_canvas::spawn_offscreen_canvas_entity` and the
//! D-24 OffscreenCanvas precedent: keeping the 5-component
//! invariant by construction; the node document is decided here too.

use elidex_ecs::{Attributes, EcsDom, Entity};

use crate::state::CustomElementState;

/// Spawn a fresh Custom element entity for the HTMLElement
/// constructor's sync-construct branch (\[C1\] §3.2.3 step 9).
///
/// Routes through [`EcsDom::create_element_with_owner`] so the four
/// Element-shape components (`NodeKind::Element`, `TagType`,
/// `Attributes`, `TreeRelation`) come from the same factory the
/// parser + `document.createElement` use, then atomically attaches
/// [`CustomElementState`] in the `Custom` lifecycle state — the
/// returned entity is a fully-formed Custom element ready for the
/// JS wrapper to splice prototype + return to user code.
///
/// `qualified_name` is the tag the entity reports via `el.tagName`
/// (e.g. `"my-el"`); `definition_name` is the CE registry key (same
/// string for autonomous custom elements; differs only for the
/// not-yet-implemented customized-built-in path where the tag is
/// the parent built-in and the registry key is the `is="..."`
/// value).
///
/// `document` is the current global object's associated Document (HTML
/// §3.2.3 *HTML element constructors* step 9.2, `#html-element-constructors`):
/// the new element is created in its node document (DOM §4.4 *node
/// document*, set upon creation), through
/// [`EcsDom::node_document_for_new_nodes`]. `None` — an unbound VM, an
/// elidex-only state with no spec counterpart — spawns nothing and returns
/// `None` (fail-closed: production code cannot create an ownerless node); the
/// caller throws.
pub fn spawn_custom_element_entity(
    dom: &mut EcsDom,
    qualified_name: &str,
    definition_name: &str,
    document: Option<Entity>,
) -> Option<Entity> {
    let owner = dom.node_document_for_new_nodes(document?);
    let entity = dom.create_element_with_owner(qualified_name, Attributes::default(), owner);
    // `create_element_with_owner` returned a freshly-spawned entity;
    // `insert_one` only fails on `NoSuchEntity`, which is impossible
    // here by construction. `.expect()` (rather than `let _ =`) makes
    // the invariant explicit so a future refactor that defers spawn
    // (e.g. tombstoned IDs) trips loudly instead of silently
    // dropping the `CustomElementState` component.
    dom.world_mut()
        .insert_one(entity, CustomElementState::custom(definition_name))
        .expect("entity was just spawned by create_element_with_owner");
    Some(entity)
}
