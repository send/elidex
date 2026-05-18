//! Shared inclusive-descendants pre-order DFS walkers.
//!
//! Iterative (NOT recursive) to survive deep trees up to
//! [`elidex_ecs::MAX_ANCESTOR_DEPTH`] (10000), which would overflow
//! the thread stack with recursion frames stacked on top of an
//! existing JS / dispatch call stack.
//!
//! Uses [`elidex_ecs::EcsDom::children_iter_rev`] to push children
//! in reverse without allocating an intermediate Vec per visited
//! node — leftmost child ends up on top of the stack and pops first,
//! preserving document-order traversal.

use std::ops::ControlFlow;

use elidex_ecs::{EcsDom, Entity};

/// Walk `root` and its inclusive descendants in pre-order DFS,
/// invoking `visit` once per entity.  `visit` returns `ControlFlow`:
/// [`ControlFlow::Break`] aborts the walk and returns the carried
/// value, [`ControlFlow::Continue`] keeps walking.
///
/// Returns the broken-out value, or `None` if the walk completed.
pub(crate) fn walk_inclusive_until<B>(
    dom: &EcsDom,
    root: Entity,
    mut visit: impl FnMut(Entity) -> ControlFlow<B>,
) -> Option<B> {
    let mut stack: Vec<Entity> = vec![root];
    while let Some(entity) = stack.pop() {
        if let ControlFlow::Break(b) = visit(entity) {
            return Some(b);
        }
        // children_iter_rev yields right-to-left; pushing each in
        // turn lands the leftmost child on top of the stack so the
        // next pop continues pre-order.
        for child in dom.children_iter_rev(entity) {
            stack.push(child);
        }
    }
    None
}

/// Walk-everything variant: invokes `visit` on every inclusive
/// descendant without early-exit.
pub(crate) fn walk_inclusive(dom: &EcsDom, root: Entity, mut visit: impl FnMut(Entity)) {
    walk_inclusive_until(dom, root, |e| {
        visit(e);
        ControlFlow::<()>::Continue(())
    });
}

/// Filtered + early-exit variant: invokes `visit` on `root` and each
/// inclusive descendant; `should_recurse(node)` returning `false`
/// means "visit this node but DO NOT recurse into its children" —
/// the subtree below the rejected node is excluded from the walk.
///
/// `visit` returns [`ControlFlow::Break`] to abort and return the
/// carried value.  Used for spec algorithms with subtree-skip
/// semantics (e.g. WHATWG HTML §2.4.3 "first base element in the
/// document" — template contents form a separate document and are
/// skipped via `should_recurse = |n| !dom.is_template_element(n)`).
pub(crate) fn walk_inclusive_filtered_until<B>(
    dom: &EcsDom,
    root: Entity,
    should_recurse: impl Fn(Entity) -> bool,
    mut visit: impl FnMut(Entity) -> ControlFlow<B>,
) -> Option<B> {
    let mut stack: Vec<Entity> = vec![root];
    while let Some(entity) = stack.pop() {
        if let ControlFlow::Break(b) = visit(entity) {
            return Some(b);
        }
        if !should_recurse(entity) {
            continue;
        }
        for child in dom.children_iter_rev(entity) {
            stack.push(child);
        }
    }
    None
}
