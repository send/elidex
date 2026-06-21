//! ChildNode/ParentNode mixin handlers: before, after, remove, replaceWith,
//! prepend, append, replaceChildren (WHATWG DOM §4.2.6 / §4.2.8).
//!
//! These handlers route every tree mutation through the canonical
//! record-producing primitives in `elidex-script-session`
//! (`apply_append_child`/`apply_insert_before`/`apply_remove_child`/
//! `apply_replace_child`/`apply_replace_all`) and push the resulting
//! `MutationRecord`s to the session for §4.3 delivery — one record source shared
//! with the Node insert methods (One-issue-one-way). The VM dispatches these
//! methods here via `invoke_dom_api` (B1.2b convergence); boa/wasm share the same
//! handlers. Custom-element reactions are driven by the `EcsDom` primitive
//! (Mechanism A `ConsumerDispatcher`) inside `apply_*`, unchanged by the routing.

use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::JsValue;
use elidex_script_session::{
    apply_append_child, apply_insert_before, apply_remove_child, apply_replace_all,
    apply_replace_child, DomApiError, DomApiHandler, MutationRecord, SessionCore,
};

use super::{
    collect_nodes, convert_nodes_into_node, ensure_pre_insertion_validity, ensure_replace_validity,
    viable_next_sibling, viable_prev_sibling,
};

/// Route the post-validity insertion of `node` (a single node, or a temp
/// `DocumentFragment` from [`convert_nodes_into_node`]) before `ref_child` — or an
/// append when `None` — through the canonical record-producing `apply_*` primitives,
/// pushing every resulting [`MutationRecord`] to the session for §4.3 delivery.
///
/// The mixin methods run `ensure_pre_insertion_validity` (or `ensure_replace_validity`)
/// **before** calling this, so an empty record list here is only the §4.2.3-insert
/// step-3 empty-`DocumentFragment` no-op (e.g. `el.append(emptyFrag)`), never a
/// validity failure — so it maps to a silent no-op, not an error.
fn route_insert(
    session: &mut SessionCore,
    dom: &mut EcsDom,
    parent: Entity,
    node: Entity,
    ref_child: Option<Entity>,
    is_temp: bool,
) {
    let records = match ref_child {
        Some(r) => apply_insert_before(dom, parent, node, r),
        None => apply_append_child(dom, parent, node),
    };
    push_records(session, records);
    destroy_temp_fragment(dom, node, is_temp);
}

/// Push every record produced by an `apply_*` builder to the session.
fn push_records(session: &mut SessionCore, records: Vec<MutationRecord>) {
    for record in records {
        session.push_notify_record(record);
    }
}

/// Free the transient wrapper `DocumentFragment` that [`convert_nodes_into_node`]
/// builds for a multi-node argument list, once `apply_*` has expanded its children
/// into the tree (leaving it empty). Without this, every multi-arg
/// `append`/`prepend`/`before`/`after`/`replaceWith`/`replaceChildren` orphans a
/// fragment entity — the VM path used to free it via the now-deleted `finalize_pair`
/// and the dom-api path never did, so the convergence fixes the leak for both
/// runtimes here (One-issue-one-way). No-op when `node` is a single user node
/// (`is_temp == false`); the emptiness guard is defence-in-depth so a wrapper that
/// still holds user nodes (e.g. left populated by a pre-expansion throw) is never
/// destroyed with its contents.
fn destroy_temp_fragment(dom: &mut EcsDom, node: Entity, is_temp: bool) {
    if is_temp && dom.children_iter(node).next().is_none() {
        let _ = dom.destroy_entity(node);
    }
}

/// `node.before(...nodes)` — inserts nodes before this node.
pub struct Before;

impl DomApiHandler for Before {
    fn method_name(&self) -> &str {
        "before"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let Some(parent) = dom.get_parent(this) else {
            return Ok(JsValue::Undefined); // orphan: noop
        };

        let nodes = collect_nodes(args, session, dom)?;
        if nodes.is_empty() {
            return Ok(JsValue::Undefined);
        }

        // Compute viable_prev BEFORE convert_nodes_into_node, which may reparent
        // nodes (changing the sibling chain). The exclude list ensures that nodes
        // about to be moved are skipped during the walk.
        let viable_prev = viable_prev_sibling(this, &nodes, dom);
        let (node, is_temp) = convert_nodes_into_node(nodes, session, dom);

        // Re-derive actual_ref from viable_prev AFTER conversion, using the
        // now-stable tree. viable_prev itself was not moved (it's not in the
        // exclude list), so its next sibling is valid. Use EXPOSED navigation
        // (first exposed child / `next_exposed_sibling`) so an internal `ShadowRoot`
        // never becomes the reference child / `nextSibling` record field (§4.8;
        // Codex PR393 R2).
        let actual_ref = match viable_prev {
            None => dom.children_iter(parent).next(),
            Some(prev) => dom.next_exposed_sibling(prev),
        };

        ensure_pre_insertion_validity(parent, node, actual_ref, dom)?;
        route_insert(session, dom, parent, node, actual_ref, is_temp);
        Ok(JsValue::Undefined)
    }
}

// ---------------------------------------------------------------------------
// ChildNode mixin: after
// ---------------------------------------------------------------------------

/// `node.after(...nodes)` — inserts nodes after this node.
pub struct After;

impl DomApiHandler for After {
    fn method_name(&self) -> &str {
        "after"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let Some(parent) = dom.get_parent(this) else {
            return Ok(JsValue::Undefined); // orphan: noop
        };

        let nodes = collect_nodes(args, session, dom)?;
        if nodes.is_empty() {
            return Ok(JsValue::Undefined);
        }

        // Compute viable_next BEFORE convert, same rationale as Before.
        let ref_sibling = viable_next_sibling(this, &nodes, dom);
        let (node, is_temp) = convert_nodes_into_node(nodes, session, dom);

        ensure_pre_insertion_validity(parent, node, ref_sibling, dom)?;
        route_insert(session, dom, parent, node, ref_sibling, is_temp);
        Ok(JsValue::Undefined)
    }
}

// ---------------------------------------------------------------------------
// ChildNode mixin: remove
// ---------------------------------------------------------------------------

/// `node.remove()` — removes this node from its parent.
pub struct ChildNodeRemove;

impl DomApiHandler for ChildNodeRemove {
    fn method_name(&self) -> &str {
        "remove"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        // §4.2.8 remove steps: if parent is null return; else remove this.
        if let Some(parent) = dom.get_parent(this) {
            if let Some(record) = apply_remove_child(dom, parent, this) {
                session.push_notify_record(record);
            }
        }
        // Per spec, no error if already an orphan.
        Ok(JsValue::Undefined)
    }
}

// ---------------------------------------------------------------------------
// ChildNode mixin: replaceWith
// ---------------------------------------------------------------------------

/// `node.replaceWith(...nodes)` — replaces this node with the given nodes.
pub struct ReplaceWith;

impl DomApiHandler for ReplaceWith {
    fn method_name(&self) -> &str {
        "replaceWith"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let Some(parent) = dom.get_parent(this) else {
            return Ok(JsValue::Undefined); // orphan: noop
        };

        let nodes = collect_nodes(args, session, dom)?;

        if nodes.is_empty() {
            // No replacement nodes: §4.2.8 step 5 "replace this with node" where node
            // is the empty fragment from convert ⇒ this is removed, nothing added. The
            // §4.2.3 replace record for that case (removedNodes=«this», prev/next =
            // this's siblings) is identical to a plain removal record, so route through
            // `apply_remove_child` (no transient empty fragment needed).
            if let Some(record) = apply_remove_child(dom, parent, this) {
                session.push_notify_record(record);
            }
            return Ok(JsValue::Undefined);
        }

        // step 3: viableNextSibling captured BEFORE convert (which reparents the args).
        let ref_sibling = viable_next_sibling(this, &nodes, dom);
        let (node, is_temp) = convert_nodes_into_node(nodes, session, dom);

        if dom.get_parent(this) == Some(parent) {
            // step 5: this is still under parent → "replace this with node within
            // parent" = the §4.2.3 replace algorithm = ONE coalesced record (plus a
            // fragment record when node is the temp fragment), via `apply_replace_child`.
            ensure_replace_validity(parent, node, this, dom)?;
            push_records(session, apply_replace_child(dom, parent, node, this));
            destroy_temp_fragment(dom, node, is_temp);
        } else {
            // step 6: this was moved out of parent by the node conversion (this was a
            // descendant of an arg) → pre-insert node before viableNextSibling.
            // (`route_insert` frees the temp fragment after expanding it.)
            ensure_pre_insertion_validity(parent, node, ref_sibling, dom)?;
            route_insert(session, dom, parent, node, ref_sibling, is_temp);
        }
        Ok(JsValue::Undefined)
    }
}

// ---------------------------------------------------------------------------
// ParentNode mixin: prepend
// ---------------------------------------------------------------------------

/// `parent.prepend(...nodes)` — inserts nodes before the first child.
pub struct Prepend;

impl DomApiHandler for Prepend {
    fn method_name(&self) -> &str {
        "prepend"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let nodes = collect_nodes(args, session, dom)?;
        if nodes.is_empty() {
            return Ok(JsValue::Undefined);
        }

        let (node, is_temp) = convert_nodes_into_node(nodes, session, dom);
        // First EXPOSED child (`children_iter` skips an internal `ShadowRoot`, §4.8) so
        // `prepend` on a shadow host with no light children inserts before null (append)
        // rather than before the shadow root — which would leak into the record's
        // `nextSibling`. (Codex PR393 R2.)
        let first_child = dom.children_iter(this).next();
        ensure_pre_insertion_validity(this, node, first_child, dom)?;
        route_insert(session, dom, this, node, first_child, is_temp);
        Ok(JsValue::Undefined)
    }
}

// ---------------------------------------------------------------------------
// ParentNode mixin: append
// ---------------------------------------------------------------------------

/// `parent.append(...nodes)` — appends nodes after the last child.
pub struct Append;

impl DomApiHandler for Append {
    fn method_name(&self) -> &str {
        "append"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let nodes = collect_nodes(args, session, dom)?;
        if nodes.is_empty() {
            return Ok(JsValue::Undefined);
        }

        let (node, is_temp) = convert_nodes_into_node(nodes, session, dom);
        ensure_pre_insertion_validity(this, node, None, dom)?;
        route_insert(session, dom, this, node, None, is_temp);
        Ok(JsValue::Undefined)
    }
}

// ---------------------------------------------------------------------------
// ParentNode mixin: replaceChildren
// ---------------------------------------------------------------------------

/// `parent.replaceChildren(...nodes)` — removes all children and appends nodes.
pub struct ReplaceChildren;

impl DomApiHandler for ReplaceChildren {
    fn method_name(&self) -> &str {
        "replaceChildren"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        // §4.2.6 replaceChildren: step 1 convert, step 2 validate, step 3 replace all.
        let nodes = collect_nodes(args, session, dom)?;
        let converted = if nodes.is_empty() {
            None
        } else {
            Some(convert_nodes_into_node(nodes, session, dom)) // (entity, is_temp)
        };
        let node = converted.map(|(n, _)| n);

        // step 2: ensure pre-insertion validity of node into this before null
        // (replace-all itself makes no tree-constraint checks — the spec note).
        if let Some(n) = node {
            ensure_pre_insertion_validity(this, n, None, dom)?;
        }

        // step 3: replace all with node within this — remove all children +
        // insert node with suppressObservers, yielding ONE coalesced record.
        push_records(session, apply_replace_all(dom, this, node));
        // Free the transient multi-arg wrapper after replace-all emptied it.
        if let Some((n, is_temp)) = converted {
            destroy_temp_fragment(dom, n, is_temp);
        }
        Ok(JsValue::Undefined)
    }
}
