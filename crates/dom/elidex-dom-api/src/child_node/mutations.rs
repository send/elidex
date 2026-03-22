//! ChildNode/ParentNode mixin handlers: before, after, remove, replaceWith,
//! prepend, append, replaceChildren.

use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::JsValue;
use elidex_script_session::{DomApiError, DomApiHandler, SessionCore};

use super::{
    collect_nodes, convert_nodes_into_node, ensure_pre_insertion_validity, ensure_replace_validity,
    insert_node_expanding_fragment, viable_next_sibling, viable_prev_sibling,
};

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
        let (node, _) = convert_nodes_into_node(nodes, dom);

        // Re-derive actual_ref from viable_prev AFTER conversion, using the
        // now-stable tree. viable_prev itself was not moved (it's not in the
        // exclude list), so its next_sibling is valid.
        let actual_ref = match viable_prev {
            None => dom.get_first_child(parent),
            Some(prev) => dom.get_next_sibling(prev),
        };

        ensure_pre_insertion_validity(parent, node, actual_ref, dom)?;
        insert_node_expanding_fragment(parent, node, actual_ref, dom)?;
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
        let (node, _) = convert_nodes_into_node(nodes, dom);

        ensure_pre_insertion_validity(parent, node, ref_sibling, dom)?;
        insert_node_expanding_fragment(parent, node, ref_sibling, dom)?;
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
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        if let Some(parent) = dom.get_parent(this) {
            let ok = dom.remove_child(parent, this);
            debug_assert!(ok, "remove_child: parent verified via get_parent");
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
            let ok = dom.remove_child(parent, this);
            debug_assert!(ok, "remove_child: parent verified via get_parent");
            return Ok(JsValue::Undefined);
        }

        // Compute viable_next BEFORE convert, same rationale as Before/After.
        let ref_sibling = viable_next_sibling(this, &nodes, dom);
        let (node, _) = convert_nodes_into_node(nodes, dom);

        // If `this` is still under the same parent (wasn't moved by node conversion),
        // use replace semantics.
        if dom.get_parent(this) == Some(parent) {
            ensure_replace_validity(parent, node, this, dom)?;
            let ok = dom.remove_child(parent, this);
            debug_assert!(ok, "remove_child: parent verified via get_parent");
        } else {
            ensure_pre_insertion_validity(parent, node, ref_sibling, dom)?;
        }

        insert_node_expanding_fragment(parent, node, ref_sibling, dom)?;
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

        let (node, _) = convert_nodes_into_node(nodes, dom);
        let first_child = dom.get_first_child(this);
        ensure_pre_insertion_validity(this, node, first_child, dom)?;
        insert_node_expanding_fragment(this, node, first_child, dom)?;
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

        let (node, _) = convert_nodes_into_node(nodes, dom);
        ensure_pre_insertion_validity(this, node, None, dom)?;
        insert_node_expanding_fragment(this, node, None, dom)?;
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
        let nodes = collect_nodes(args, session, dom)?;
        let (node, _is_temp) = if nodes.is_empty() {
            (None, false)
        } else {
            let (n, t) = convert_nodes_into_node(nodes, dom);
            (Some(n), t)
        };

        // Validate before removing existing children.
        if let Some(ref n) = node {
            ensure_pre_insertion_validity(this, *n, None, dom)?;
        }

        // Remove all existing children.
        let existing = dom.children(this);
        for child in existing {
            let ok = dom.remove_child(this, child);
            debug_assert!(ok, "remove_child: child from children() must be removable");
        }

        if let Some(n) = node {
            insert_node_expanding_fragment(this, n, None, dom)?;
        }
        Ok(JsValue::Undefined)
    }
}
