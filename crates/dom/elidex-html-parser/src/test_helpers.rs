//! Shared test helpers for DOM tree inspection.
//!
//! Used by tests in both `lib.rs` and `convert.rs` to avoid duplication.

use elidex_ecs::{EcsDom, Entity, TagType, TextContent};

/// Walk children recursively and find the first element with the given tag.
pub fn find_tag(dom: &EcsDom, parent: Entity, tag: &str) -> Option<Entity> {
    for child in dom.children_iter(parent) {
        if let Ok(t) = dom.world().get::<&TagType>(child) {
            if t.0 == tag {
                return Some(child);
            }
            if let Some(found) = find_tag(dom, child, tag) {
                return Some(found);
            }
        }
    }
    None
}

/// Collect text content from direct children.
pub fn child_text(dom: &EcsDom, parent: Entity) -> String {
    let mut result = String::new();
    for child in dom.children_iter(parent) {
        if let Ok(tc) = dom.world().get::<&TextContent>(child) {
            result.push_str(&tc.0);
        }
    }
    result
}
