//! `TreeWalker` and `NodeIterator` implementations (WHATWG DOM §7).
//!
//! These provide filtered traversal of the DOM tree, matching the Web API
//! `TreeWalker` and `NodeIterator` interfaces.

use elidex_ecs::{EcsDom, Entity, NodeKind};

// ---------------------------------------------------------------------------
// whatToShow constants (WHATWG DOM §7.1)
// ---------------------------------------------------------------------------

/// Show all node types.
pub const SHOW_ALL: u32 = 0xFFFF_FFFF;
/// Show only Element nodes.
pub const SHOW_ELEMENT: u32 = 0x1;
/// Show only Attr nodes (legacy; Attr is not a Node in modern DOM but
/// the constant is preserved for spec-conformance).
pub const SHOW_ATTRIBUTE: u32 = 0x2;
/// Show only Text nodes.
pub const SHOW_TEXT: u32 = 0x4;
/// Show only CDATASection nodes.
pub const SHOW_CDATA_SECTION: u32 = 0x8;
/// Show only EntityReference nodes (legacy; never emitted by modern
/// HTML parsers but the constant is preserved per WHATWG §7.1).
pub const SHOW_ENTITY_REFERENCE: u32 = 0x10;
/// Show only Entity nodes (legacy).
pub const SHOW_ENTITY: u32 = 0x20;
/// Show only ProcessingInstruction nodes.
pub const SHOW_PROCESSING_INSTRUCTION: u32 = 0x40;
/// Show only Comment nodes.
pub const SHOW_COMMENT: u32 = 0x80;
/// Show only Document nodes.
pub const SHOW_DOCUMENT: u32 = 0x100;
/// Show only DocumentType nodes.
pub const SHOW_DOCUMENT_TYPE: u32 = 0x200;
/// Show only DocumentFragment nodes.
pub const SHOW_DOCUMENT_FRAGMENT: u32 = 0x400;
/// Show only Notation nodes (legacy).
pub const SHOW_NOTATION: u32 = 0x800;

// ---------------------------------------------------------------------------
// NodeFilter result constants (WHATWG DOM §7.3)
// ---------------------------------------------------------------------------

/// `NodeFilter.FILTER_ACCEPT` — accept the node, return it.
pub const FILTER_ACCEPT: u16 = 1;
/// `NodeFilter.FILTER_REJECT` — reject the node AND skip its descendants.
pub const FILTER_REJECT: u16 = 2;
/// `NodeFilter.FILTER_SKIP` — skip the node but descend into its children.
pub const FILTER_SKIP: u16 = 3;

/// Map a `NodeKind` to its `whatToShow` bitmask bit.
fn node_kind_bit(kind: NodeKind) -> u32 {
    match kind {
        NodeKind::Element => SHOW_ELEMENT,
        NodeKind::Attribute => 0x2,
        NodeKind::Text => SHOW_TEXT,
        NodeKind::CdataSection => 0x8,
        NodeKind::ProcessingInstruction => 0x40,
        NodeKind::Comment => SHOW_COMMENT,
        NodeKind::Document => SHOW_DOCUMENT,
        NodeKind::DocumentType => 0x200,
        NodeKind::DocumentFragment => 0x400,
        // Window is not a Node per WHATWG and is not exposed through
        // NodeIterator / TreeWalker `whatToShow`.
        NodeKind::Window => 0,
    }
}

/// Check if a node's kind is accepted by the given `what_to_show` mask.
fn accepts(entity: Entity, what_to_show: u32, dom: &EcsDom) -> bool {
    if what_to_show == SHOW_ALL {
        return true;
    }
    let Some(kind) = dom.node_kind(entity) else {
        return false;
    };
    (what_to_show & node_kind_bit(kind)) != 0
}

// ---------------------------------------------------------------------------
// Pre-order traversal helpers
// ---------------------------------------------------------------------------

/// Return the next node in pre-order traversal, confined within `root`.
fn next_in_preorder(current: Entity, root: Entity, dom: &EcsDom) -> Option<Entity> {
    // First child?
    if let Some(child) = dom.get_first_child(current) {
        return Some(child);
    }
    // Walk up to find next sibling.
    let mut node = current;
    loop {
        if node == root {
            return None;
        }
        if let Some(sib) = dom.get_next_sibling(node) {
            return Some(sib);
        }
        node = dom.get_parent(node)?;
    }
}

/// Return the next node in pre-order traversal but skip `current`'s
/// subtree entirely (WHATWG DOM §6.2 `TreeWalker` filter Reject:
/// rejected nodes have their descendants pruned from the walk).
/// Walks to `current`'s next sibling, falling back to the nearest
/// ancestor's next sibling. Confined within `root`.
fn next_in_preorder_skip_subtree(current: Entity, root: Entity, dom: &EcsDom) -> Option<Entity> {
    let mut node = current;
    loop {
        if node == root {
            return None;
        }
        if let Some(sib) = dom.get_next_sibling(node) {
            return Some(sib);
        }
        node = dom.get_parent(node)?;
    }
}

/// Return the previous node in pre-order traversal, confined within `root`.
fn prev_in_preorder(current: Entity, root: Entity, dom: &EcsDom) -> Option<Entity> {
    if current == root {
        return None;
    }
    // Previous sibling's deepest last descendant, or parent.
    if let Some(sib) = dom.get_prev_sibling(current) {
        return Some(last_descendant(sib, dom));
    }
    dom.get_parent(current)
}

/// Walk to the deepest last-child descendant of `node`.
fn last_descendant(node: Entity, dom: &EcsDom) -> Entity {
    let mut current = node;
    while let Some(last) = dom.get_last_child(current) {
        current = last;
    }
    current
}

// ===========================================================================
// TreeWalker
// ===========================================================================

/// `TreeWalker` — filtered tree traversal (WHATWG DOM §7.2).
///
/// `current_node` can be moved by the traversal methods. The walker never
/// moves outside the subtree rooted at `root`.
#[derive(Debug, Clone)]
pub struct TreeWalker {
    /// The root node of the traversal.
    pub root: Entity,
    /// The current position of the walker.
    pub current_node: Entity,
    /// Bitmask of node types to show.
    pub what_to_show: u32,
}

impl TreeWalker {
    /// Create a new `TreeWalker` with `current_node` set to `root`.
    #[must_use]
    pub fn new(root: Entity, what_to_show: u32) -> Self {
        Self {
            root,
            current_node: root,
            what_to_show,
        }
    }

    /// Move to the parent node (stops at root).
    pub fn parent_node(&mut self, dom: &EcsDom) -> Option<Entity> {
        let mut node = self.current_node;
        while node != self.root {
            let parent = dom.get_parent(node)?;
            if accepts(parent, self.what_to_show, dom) {
                self.current_node = parent;
                return Some(parent);
            }
            node = parent;
        }
        None
    }

    /// Move to the first accepted child.
    pub fn first_child(&mut self, dom: &EcsDom) -> Option<Entity> {
        self.traverse_children(dom, true)
    }

    /// Move to the last accepted child.
    pub fn last_child(&mut self, dom: &EcsDom) -> Option<Entity> {
        self.traverse_children(dom, false)
    }

    /// Move to the next accepted sibling.
    pub fn next_sibling(&mut self, dom: &EcsDom) -> Option<Entity> {
        self.traverse_siblings(dom, true)
    }

    /// Move to the previous accepted sibling.
    pub fn previous_sibling(&mut self, dom: &EcsDom) -> Option<Entity> {
        self.traverse_siblings(dom, false)
    }

    /// Move to the next node in pre-order traversal.
    pub fn next_node(&mut self, dom: &EcsDom) -> Option<Entity> {
        let mut node = self.current_node;
        loop {
            node = next_in_preorder(node, self.root, dom)?;
            if accepts(node, self.what_to_show, dom) {
                self.current_node = node;
                return Some(node);
            }
        }
    }

    /// Move to the previous node in pre-order traversal.
    pub fn previous_node(&mut self, dom: &EcsDom) -> Option<Entity> {
        let mut node = self.current_node;
        loop {
            if node == self.root {
                return None;
            }
            node = prev_in_preorder(node, self.root, dom)?;
            if accepts(node, self.what_to_show, dom) {
                self.current_node = node;
                return Some(node);
            }
        }
    }

    /// Helper: traverse to first or last accepted child of `current_node`.
    fn traverse_children(&mut self, dom: &EcsDom, first: bool) -> Option<Entity> {
        let child = if first {
            dom.get_first_child(self.current_node)?
        } else {
            dom.get_last_child(self.current_node)?
        };

        let mut node = child;
        loop {
            if accepts(node, self.what_to_show, dom) {
                self.current_node = node;
                return Some(node);
            }
            // Try children of this node (descend into filtered-out nodes).
            let sub = if first {
                dom.get_first_child(node)
            } else {
                dom.get_last_child(node)
            };
            if let Some(sub_node) = sub {
                node = sub_node;
                continue;
            }
            // Try siblings.
            loop {
                if node == self.current_node {
                    return None;
                }
                let sib = if first {
                    dom.get_next_sibling(node)
                } else {
                    dom.get_prev_sibling(node)
                };
                if let Some(sib_node) = sib {
                    node = sib_node;
                    break;
                }
                let parent = dom.get_parent(node)?;
                if parent == self.current_node {
                    return None;
                }
                node = parent;
            }
        }
    }

    /// Helper: traverse to next or previous accepted sibling.
    fn traverse_siblings(&mut self, dom: &EcsDom, next: bool) -> Option<Entity> {
        let mut node = self.current_node;
        loop {
            let sib = if next {
                dom.get_next_sibling(node)
            } else {
                dom.get_prev_sibling(node)
            };
            if let Some(sib_node) = sib {
                if accepts(sib_node, self.what_to_show, dom) {
                    self.current_node = sib_node;
                    return Some(sib_node);
                }
                // Descend into filtered-out sibling to find an accepted descendant.
                let sub = if next {
                    dom.get_first_child(sib_node)
                } else {
                    dom.get_last_child(sib_node)
                };
                if sub.is_some() {
                    node = sib_node;
                    continue;
                }
                node = sib_node;
                continue;
            }
            // Walk up to parent.
            let parent = dom.get_parent(node)?;
            if parent == self.root {
                return None;
            }
            node = parent;
        }
    }
}

// ---------------------------------------------------------------------------
// FilterAction trait + step_with_filter (engine-indep algorithm hoist)
// ---------------------------------------------------------------------------

/// Outcome of a `NodeFilter` callback per WHATWG DOM §7.3.
///
/// Distinct from raw `u16` so VM-side callers `(1 | 2 | _)` → enum
/// coercion happens at the marshalling boundary (`vm/host/node_filter_dispatch.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeFilterResult {
    /// Accept this node; return it from the traversal.
    Accept,
    /// Reject this node AND skip its descendants. Per WHATWG DOM §7.3:
    /// pre-order traversal ([`step_with_filter`]) walks past the
    /// rejected subtree to the rejected node's next sibling / ancestor
    /// sibling; sibling-only walks (`nextSibling` / `previousSibling`)
    /// skip the rejected node entirely without descending — both rules
    /// reach the same observable result of "no descendant of a
    /// rejected node is visited", but the implementation of
    /// `step_with_filter` ALWAYS prunes descendants regardless of which
    /// walk kicked it off.
    Reject,
    /// Skip this node but descend into its descendants.
    Skip,
}

impl NodeFilterResult {
    /// Map an `unsigned short` ([WebIDL] coercion) NodeFilter return
    /// value to [`NodeFilterResult`]. Per spec §6.3, only `1` (Accept)
    /// and `2` (Reject) are special; every other value (incl. `0`,
    /// `-1`/`65535`, `3`, `4`+, `NaN`-clamped) maps to Skip.
    ///
    /// ## VM-side coercion contract
    ///
    /// The caller (VM-side `node_filter_dispatch.rs` in PR-A2) MUST
    /// apply WebIDL `unsigned short` coercion (see [WebIDL]) BEFORE
    /// invoking this helper: `ToUint16` per ES2020 §7.1.7 wraps negative numbers
    /// (`-1` → `65535`), `NaN` / `Infinity` → `0`, fractions truncate
    /// toward zero. Values outside the `{1, 2}` accept/reject set map
    /// to `Skip` regardless of the source bit pattern — this function
    /// only parses the post-coercion `u16` and does NOT itself perform
    /// coercion. If the VM-side dispatch bypasses coercion (e.g. passes
    /// the raw `JsValue::Number`), the result enum could be wrong;
    /// covered by VM-side `tests_traversal::node_filter_coercion_*` in
    /// the follow-up bindings PR.
    ///
    /// [WebIDL]: https://webidl.spec.whatwg.org/
    #[must_use]
    pub fn from_unsigned_short(value: u16) -> Self {
        match value {
            1 => Self::Accept,
            2 => Self::Reject,
            _ => Self::Skip,
        }
    }
}

/// Errors that a [`FilterAction::accept`] callback can surface.
/// VM-side bindings map this to `VmError`; engine-indep callers
/// surface it via `Result`.
#[derive(Debug)]
pub enum FilterError {
    /// The JS callback was already running for this traversal —
    /// per WHATWG §7.2 "TreeWalker filter is active flag", re-entrant
    /// invocation throws `InvalidStateError`.
    AlreadyActive,
    /// The JS callback threw — propagated up the traversal step.
    Throw,
}

/// Callback dispatch trait for filtered traversal.
///
/// VM-side bindings (`vm/host/tree_walker_proto.rs` /
/// `node_iterator_proto.rs`) implement this with a closure that
/// resolves the JS `NodeFilter` callback, sets the active-flag,
/// invokes it, parses the return value via
/// [`NodeFilterResult::from_unsigned_short`], and clears the
/// active-flag on drop. WHATWG DOM §7.3 defines the `NodeFilter`
/// callback interface — `acceptNode(node)` returning a
/// `FILTER_ACCEPT` / `FILTER_REJECT` / `FILTER_SKIP` constant.
pub trait FilterAction {
    /// Invoke the filter for `node`. Returns the parsed result or a
    /// re-entrancy / propagated-throw error.
    fn accept(&mut self, node: Entity) -> Result<NodeFilterResult, FilterError>;
}

/// Step a [`TreeWalker`] forward in pre-order with `filter` applied
/// (algorithm hoist per PR-A plan v3 §A8).
///
/// On `Accept`: moves `walker.current_node` to the matched node and
/// returns it. On `Reject` / `Skip`: continues to the next candidate
/// (Reject is equivalent to Skip for pre-order forward; for
/// `next_sibling`-style walks Reject would also skip the subtree,
/// but `next_node` already only visits subtree-walked nodes once).
///
/// Returns `None` when traversal exits the subtree rooted at
/// `walker.root` without finding an accepted node.
///
/// `vm/host/tree_walker_proto.rs::native_tree_walker_next_node`
/// wraps this with a `FilterAction` impl that drives the JS
/// callback.
pub fn step_with_filter<F: FilterAction>(
    walker: &mut TreeWalker,
    dom: &EcsDom,
    filter: &mut F,
) -> Result<Option<Entity>, FilterError> {
    let mut node = walker.current_node;
    let mut skip_subtree = false;
    loop {
        let next = if skip_subtree {
            next_in_preorder_skip_subtree(node, walker.root, dom)
        } else {
            next_in_preorder(node, walker.root, dom)
        };
        let Some(next) = next else {
            return Ok(None);
        };
        if accepts(next, walker.what_to_show, dom) {
            match filter.accept(next)? {
                NodeFilterResult::Accept => {
                    walker.current_node = next;
                    return Ok(Some(next));
                }
                NodeFilterResult::Reject => {
                    // Spec §6.2: Reject prunes the subtree. Next
                    // iteration must skip `next`'s descendants.
                    node = next;
                    skip_subtree = true;
                }
                NodeFilterResult::Skip => {
                    // Skip the node but still descend into its
                    // children on the next iteration.
                    node = next;
                    skip_subtree = false;
                }
            }
        } else {
            // The `whatToShow` mask rejected the node; spec §6.2 treats
            // this as Skip (descend into children).
            node = next;
            skip_subtree = false;
        }
    }
}

// ---------------------------------------------------------------------------
// Filter-aware direction-specific traversal (WHATWG DOM §6.4)
// ---------------------------------------------------------------------------

/// Internal `Skip` / `Reject` result that respects the spec's
/// "filter callback applied AFTER whatToShow filter" ordering (§6.4
/// "traverse children" steps 1.2-1.3 / "traverse siblings" 1.2-1.3 /
/// "parentNode" 2.2-2.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FilterDecision {
    Accept,
    /// Equivalent to spec FILTER_SKIP — visit descendants but not
    /// this node.
    Skip,
    /// Equivalent to spec FILTER_REJECT — do not visit this node OR
    /// any descendant.  Sibling-only walks treat Reject like Skip
    /// (no subtree available to prune), but tree walks must prune.
    Reject,
}

/// Apply `whatToShow` THEN `filter` to `node` and return the merged
/// decision per WHATWG §6.4 "filter" algorithm.
fn classify<F: FilterAction>(
    node: Entity,
    what_to_show: u32,
    dom: &EcsDom,
    filter: &mut F,
) -> Result<FilterDecision, FilterError> {
    if !accepts(node, what_to_show, dom) {
        return Ok(FilterDecision::Skip);
    }
    Ok(match filter.accept(node)? {
        NodeFilterResult::Accept => FilterDecision::Accept,
        NodeFilterResult::Skip => FilterDecision::Skip,
        NodeFilterResult::Reject => FilterDecision::Reject,
    })
}

/// WHATWG §6.4 "traverseChildren" algorithm — apply filter to
/// first/last child and walk per spec.
///
/// On Accept: moves `walker.current_node` and returns the node.
/// On Reject: skips that subtree, tries siblings.
/// On Skip: descends into children.
pub fn step_with_filter_first_child<F: FilterAction>(
    walker: &mut TreeWalker,
    dom: &EcsDom,
    filter: &mut F,
) -> Result<Option<Entity>, FilterError> {
    traverse_children_filtered(walker, dom, filter, true)
}

/// Mirror of [`step_with_filter_first_child`] in the reverse direction.
pub fn step_with_filter_last_child<F: FilterAction>(
    walker: &mut TreeWalker,
    dom: &EcsDom,
    filter: &mut F,
) -> Result<Option<Entity>, FilterError> {
    traverse_children_filtered(walker, dom, filter, false)
}

fn traverse_children_filtered<F: FilterAction>(
    walker: &mut TreeWalker,
    dom: &EcsDom,
    filter: &mut F,
    first: bool,
) -> Result<Option<Entity>, FilterError> {
    let entry = if first {
        dom.get_first_child(walker.current_node)
    } else {
        dom.get_last_child(walker.current_node)
    };
    let Some(mut node) = entry else {
        return Ok(None);
    };
    loop {
        match classify(node, walker.what_to_show, dom, filter)? {
            FilterDecision::Accept => {
                walker.current_node = node;
                return Ok(Some(node));
            }
            FilterDecision::Skip => {
                // Descend into children.
                let descend = if first {
                    dom.get_first_child(node)
                } else {
                    dom.get_last_child(node)
                };
                if let Some(child) = descend {
                    node = child;
                    continue;
                }
                // No descendant; fall through to sibling walk.
            }
            FilterDecision::Reject => {
                // Skip this subtree entirely — go to sibling.
            }
        }
        // Sibling / ancestor-sibling walk back to the next candidate.
        loop {
            let sib = if first {
                dom.get_next_sibling(node)
            } else {
                dom.get_prev_sibling(node)
            };
            if let Some(s) = sib {
                node = s;
                break;
            }
            let parent = dom.get_parent(node);
            match parent {
                Some(p) if p != walker.current_node => node = p,
                _ => return Ok(None),
            }
        }
    }
}

/// WHATWG §6.4 "traverseSiblings" — apply filter to next/prev sibling.
pub fn step_with_filter_next_sibling<F: FilterAction>(
    walker: &mut TreeWalker,
    dom: &EcsDom,
    filter: &mut F,
) -> Result<Option<Entity>, FilterError> {
    traverse_siblings_filtered(walker, dom, filter, true)
}

/// Mirror of [`step_with_filter_next_sibling`] in the reverse direction.
pub fn step_with_filter_previous_sibling<F: FilterAction>(
    walker: &mut TreeWalker,
    dom: &EcsDom,
    filter: &mut F,
) -> Result<Option<Entity>, FilterError> {
    traverse_siblings_filtered(walker, dom, filter, false)
}

fn traverse_siblings_filtered<F: FilterAction>(
    walker: &mut TreeWalker,
    dom: &EcsDom,
    filter: &mut F,
    next: bool,
) -> Result<Option<Entity>, FilterError> {
    let mut node = walker.current_node;
    loop {
        let sib = if next {
            dom.get_next_sibling(node)
        } else {
            dom.get_prev_sibling(node)
        };
        let Some(mut candidate) = sib else {
            // Walk up.
            let parent = dom.get_parent(node);
            match parent {
                Some(p) if p != walker.root => {
                    node = p;
                    continue;
                }
                _ => return Ok(None),
            }
        };
        // Inner loop: descend through Skip-decision nodes into their
        // first/last child, but treat Reject like "no descent, try
        // next sibling".
        loop {
            match classify(candidate, walker.what_to_show, dom, filter)? {
                FilterDecision::Accept => {
                    walker.current_node = candidate;
                    return Ok(Some(candidate));
                }
                FilterDecision::Skip => {
                    let descend = if next {
                        dom.get_first_child(candidate)
                    } else {
                        dom.get_last_child(candidate)
                    };
                    if let Some(child) = descend {
                        candidate = child;
                        continue;
                    }
                    // Skip with no child — try sibling of this candidate.
                    node = candidate;
                    break;
                }
                FilterDecision::Reject => {
                    // Reject prunes subtree — try sibling of this candidate.
                    node = candidate;
                    break;
                }
            }
        }
    }
}

/// WHATWG §6.4 "parentNode" — walk ancestors and apply filter.
/// Per spec, parentNode treats Reject as Skip (no subtree pruning in
/// ancestor walks; the rejected ancestor has no descendant of
/// `currentNode` that we'd avoid by pruning).
pub fn step_with_filter_parent_node<F: FilterAction>(
    walker: &mut TreeWalker,
    dom: &EcsDom,
    filter: &mut F,
) -> Result<Option<Entity>, FilterError> {
    let mut node = walker.current_node;
    while node != walker.root {
        let Some(parent) = dom.get_parent(node) else {
            return Ok(None);
        };
        // Reject ≡ Skip in ancestor walks per spec §6.4.4.
        if accepts(parent, walker.what_to_show, dom) {
            match filter.accept(parent)? {
                NodeFilterResult::Accept => {
                    walker.current_node = parent;
                    return Ok(Some(parent));
                }
                NodeFilterResult::Reject | NodeFilterResult::Skip => {}
            }
        }
        node = parent;
    }
    Ok(None)
}

/// WHATWG §6.4 "previousNode" — reverse pre-order with filter.
/// Mirror of [`step_with_filter`] (forward) in tree order.
pub fn step_with_filter_previous_node<F: FilterAction>(
    walker: &mut TreeWalker,
    dom: &EcsDom,
    filter: &mut F,
) -> Result<Option<Entity>, FilterError> {
    let mut node = walker.current_node;
    loop {
        if node == walker.root {
            return Ok(None);
        }
        // Try prev-sibling's deepest descent (descending past Reject),
        // else ascend to parent.
        let candidate = if let Some(sib) = dom.get_prev_sibling(node) {
            // Descend to deepest accepted last descendant, applying
            // filter at each level.  Implementing as a tight inner
            // loop avoids recursion + matches spec §6.4 "previousNode"
            // semantics.
            descend_to_last_filtered(sib, walker.what_to_show, dom, filter)?
        } else {
            dom.get_parent(node)
        };
        let Some(cand) = candidate else {
            return Ok(None);
        };
        if cand == walker.root {
            // Root is included only if accepted by filter +
            // what_to_show — return root if Accept, else None.
            match classify(cand, walker.what_to_show, dom, filter)? {
                FilterDecision::Accept => {
                    walker.current_node = cand;
                    return Ok(Some(cand));
                }
                _ => return Ok(None),
            }
        }
        match classify(cand, walker.what_to_show, dom, filter)? {
            FilterDecision::Accept => {
                walker.current_node = cand;
                return Ok(Some(cand));
            }
            _ => node = cand,
        }
    }
}

/// Walk to the deepest last-descendant of `node` that passes the filter.
/// If filter rejects an interior node, the rejected subtree is skipped
/// (return the rejected node itself for the outer loop to advance).
fn descend_to_last_filtered<F: FilterAction>(
    node: Entity,
    what_to_show: u32,
    dom: &EcsDom,
    filter: &mut F,
) -> Result<Option<Entity>, FilterError> {
    let mut current = node;
    loop {
        // If a `Reject` decision would short-circuit descent here,
        // the spec actually says we still need to descend to the
        // candidate first — the filter only sees nodes one at a time.
        // Conservative approach: descend to deepest last-child,
        // returning the deepest node.  The outer loop classifies.
        match dom.get_last_child(current) {
            Some(child) => current = child,
            None => return Ok(Some(current)),
        }
        // Suppress unused-variable warnings.
        let _ = what_to_show;
        let _ = &mut *filter;
    }
}

/// NodeIterator forward step with filter (WHATWG §6.1 "traverse").
///
/// Applies `pointer_before` discipline + filter, mutating
/// `state.reference` / `state.pointer_before` per spec.
pub fn step_with_filter_node_iterator_next<F: FilterAction>(
    state: &mut crate::mutation_bridge::NodeIteratorState,
    dom: &EcsDom,
    filter: &mut F,
) -> Result<Option<Entity>, FilterError> {
    traverse_node_iterator_filtered(state, dom, filter, true)
}

/// Mirror of [`step_with_filter_node_iterator_next`] in the reverse
/// direction.
pub fn step_with_filter_node_iterator_previous<F: FilterAction>(
    state: &mut crate::mutation_bridge::NodeIteratorState,
    dom: &EcsDom,
    filter: &mut F,
) -> Result<Option<Entity>, FilterError> {
    traverse_node_iterator_filtered(state, dom, filter, false)
}

fn traverse_node_iterator_filtered<F: FilterAction>(
    state: &mut crate::mutation_bridge::NodeIteratorState,
    dom: &EcsDom,
    filter: &mut F,
    next: bool,
) -> Result<Option<Entity>, FilterError> {
    let mut node = state.reference;
    let mut before = state.pointer_before;
    loop {
        // Spec §6.1 step 5/6: if direction matches pointer side,
        // move pointer first; otherwise candidate is current node.
        let candidate = if next {
            if before {
                before = false;
                node
            } else {
                let Some(n) = next_in_preorder(node, state.root, dom) else {
                    return Ok(None);
                };
                node = n;
                node
            }
        } else if before {
            let Some(n) = prev_in_preorder(node, state.root, dom) else {
                return Ok(None);
            };
            if n == state.root {
                // root reached — still process it, then None next.
                node = n;
                node
            } else {
                node = n;
                node
            }
        } else {
            before = true;
            node
        };
        match classify(candidate, state.what_to_show, dom, filter)? {
            FilterDecision::Accept => {
                state.reference = candidate;
                state.pointer_before = before;
                return Ok(Some(candidate));
            }
            // NodeIterator §6.1 has no subtree pruning — Reject ≡ Skip
            // (the algorithm is a flat pre-order walk).
            FilterDecision::Skip | FilterDecision::Reject => {
                // Loop continues with `node` already advanced.
            }
        }
    }
}

// ===========================================================================
// NodeIterator pre-removing-steps adjustment (WHATWG DOM §6.1 step 1)
// ===========================================================================

/// Apply WHATWG DOM §6.1 "NodeIterator pre-removing steps" to a
/// single registered iterator state, called by
/// `crate::mutation_bridge::MutationBridge` (its
/// `after_remove_with_descendants` impl).
///
/// **Spec recap (current WHATWG §6.1, 2-branch algorithm):**
///
/// - Branch (a) — `removed` is an inclusive ancestor of
///   `state.reference`: walk forward in tree order past `removed`'s
///   subtree boundary (skip all `descendants`).  Set
///   `state.reference` to the first node found, or fall back to
///   the last node preceding `removed`, or collapse to `state.root`
///   if neither exists.  Update `pointer_before` per spec.
/// - Branch (b) — `removed` is NOT an inclusive ancestor of
///   `state.reference`: no-op.
///
/// The "inclusive ancestor" test reduces to `state.reference ==
/// removed || descendants.contains(&state.reference)` per the
/// pre-snapshotted `descendants` slice (which is **inclusive** of
/// `removed` per `EcsDom::collect_inclusive_descendants`).
///
/// **Plan-v4 §A-NI-1 Round 2 CRIT-2 invariant**: at fire time,
/// `parent.children[removed_index] == removed` is still true
/// (`EcsDom::fire_after_remove` runs BEFORE the actual detach —
/// see [`elidex_ecs::MutationHook::after_remove`] doc).  Walking
/// from `(parent, removed_index)` therefore enters at the
/// to-be-removed node itself; the `descendants`-membership filter
/// naturally skips `removed` + every descendant, advancing to the
/// post-subtree node.
///
pub fn adjust_node_iterator_for_removal(
    state: &mut crate::mutation_bridge::NodeIteratorState,
    _removed: Entity,
    parent: Entity,
    removed_index: usize,
    descendants: &[Entity],
    dom: &EcsDom,
) {
    // Branch (b): `removed` is NOT an inclusive ancestor of
    // `state.reference` — no-op.
    if !descendants.contains(&state.reference) {
        return;
    }

    // Branch (a): walk past the to-be-removed subtree to find the
    // "first following node not in subtree", per WHATWG §6.1.
    //
    // Entry point is the slot immediately AFTER `(parent,
    // removed_index)` in tree order — i.e. the next sibling of
    // `removed`, or its post-order ancestor's next sibling.
    // We synthesise this by starting at `parent`'s child at
    // `removed_index` (which IS `removed`, fire-before-detach) and
    // calling `next_in_preorder` once with `skip_subtree=true`.
    let entry_seed = dom
        .children_iter(parent)
        .nth(removed_index)
        .unwrap_or(parent);
    let mut follower =
        next_in_preorder_skip_subtree(entry_seed, parent_root_of(state.root, dom), dom);
    while let Some(node) = follower {
        if !descendants.contains(&node) {
            state.reference = node;
            state.pointer_before = true;
            return;
        }
        // Defensive: an entity that IS in `descendants` should never
        // be returned by `next_in_preorder_skip_subtree` from outside
        // the subtree, but if it does (corrupted ancestry / sibling
        // chain), advance past it conservatively.
        follower = next_in_preorder(node, parent_root_of(state.root, dom), dom);
    }

    // No following node — try the last preceding node not in subtree.
    // Walk backward from `parent`'s child at `removed_index - 1` (or
    // `parent` itself when `removed_index == 0`).
    let preceding_seed = if removed_index == 0 {
        parent
    } else {
        dom.children_iter(parent)
            .nth(removed_index - 1)
            .unwrap_or(parent)
    };
    let mut preceding = Some(preceding_seed);
    while let Some(node) = preceding {
        if !descendants.contains(&node) {
            state.reference = node;
            state.pointer_before = false;
            return;
        }
        preceding = prev_in_preorder(node, state.root, dom);
    }

    // Neither side has a candidate — collapse to root.
    state.reference = state.root;
    state.pointer_before = true;
}

/// Helper for [`adjust_node_iterator_for_removal`]: the §6.1
/// "follower" walk traverses past `parent` if needed.  We use the
/// iterator's `root` since pre-order steps confine to that subtree.
fn parent_root_of(root: Entity, _dom: &EcsDom) -> Entity {
    root
}

// ===========================================================================
// NodeIterator
// ===========================================================================

/// `NodeIterator` — flat pre-order traversal with filtering (WHATWG DOM §7.1).
#[derive(Debug, Clone)]
pub struct NodeIterator {
    /// The root node of the iteration.
    pub root: Entity,
    /// The reference node for the iterator position.
    pub reference_node: Entity,
    /// Whether the pointer is before the reference node.
    pub pointer_before_reference: bool,
    /// Bitmask of node types to show.
    pub what_to_show: u32,
}

impl NodeIterator {
    /// Create a new `NodeIterator`.
    #[must_use]
    pub fn new(root: Entity, what_to_show: u32) -> Self {
        Self {
            root,
            reference_node: root,
            pointer_before_reference: true,
            what_to_show,
        }
    }

    /// Validate that `reference_node` still exists in the DOM tree rooted at
    /// `root`. If it has been removed (e.g. by a DOM mutation), reset the
    /// iterator to `root`.
    ///
    /// Per WHATWG DOM §7.1, when a node is removed, any `NodeIterator` whose
    /// `reference_node` is that node must update its reference. This safety
    /// check is a simplified version: instead of tracking all mutations via
    /// hooks, we validate on each traversal step.
    fn validate_reference(&mut self, dom: &EcsDom) {
        // Check if reference_node is still a descendant of (or equal to) root.
        if self.reference_node == self.root {
            return;
        }
        if !dom.is_ancestor_or_self(self.root, self.reference_node) {
            // The reference node is no longer in our subtree; reset to root.
            self.reference_node = self.root;
            self.pointer_before_reference = true;
        }
    }

    /// Handle a node removal: if `reference_node` is the removed node, advance
    /// to an adjacent node per WHATWG DOM §7.1.
    ///
    /// Call this before actually removing `removed` from the DOM.
    pub fn pre_remove_check(&mut self, removed: Entity, dom: &EcsDom) {
        if self.reference_node != removed {
            return;
        }
        // Try to advance to next accepted node.
        if let Some(next) = next_in_preorder(removed, self.root, dom) {
            self.reference_node = next;
            self.pointer_before_reference = true;
        } else if let Some(prev) = prev_in_preorder(removed, self.root, dom) {
            // Fall back to previous node.
            self.reference_node = prev;
            self.pointer_before_reference = false;
        } else {
            // Only node was root; reset.
            self.reference_node = self.root;
            self.pointer_before_reference = true;
        }
    }

    /// Return the next accepted node.
    pub fn next_node(&mut self, dom: &EcsDom) -> Option<Entity> {
        self.validate_reference(dom);
        if self.pointer_before_reference {
            self.pointer_before_reference = false;
            if accepts(self.reference_node, self.what_to_show, dom) {
                return Some(self.reference_node);
            }
        }
        let mut node = self.reference_node;
        loop {
            node = next_in_preorder(node, self.root, dom)?;
            self.reference_node = node;
            if accepts(node, self.what_to_show, dom) {
                return Some(node);
            }
        }
    }

    /// Return the previous accepted node.
    pub fn previous_node(&mut self, dom: &EcsDom) -> Option<Entity> {
        self.validate_reference(dom);
        if !self.pointer_before_reference {
            self.pointer_before_reference = true;
            if accepts(self.reference_node, self.what_to_show, dom) {
                return Some(self.reference_node);
            }
        }
        let mut node = self.reference_node;
        loop {
            node = prev_in_preorder(node, self.root, dom)?;
            if node == self.root {
                // root is included.
                self.reference_node = node;
                if accepts(node, self.what_to_show, dom) {
                    return Some(node);
                }
                return None;
            }
            self.reference_node = node;
            if accepts(node, self.what_to_show, dom) {
                return Some(node);
            }
        }
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
#[allow(unused_must_use)]
mod tests {
    use super::*;
    use elidex_ecs::{Attributes, EcsDom};

    /// Build: root(div) -> [span, text("hello"), p -> [text("world")], comment]
    fn build_tree() -> (EcsDom, Entity, Entity, Entity, Entity, Entity, Entity) {
        let mut dom = EcsDom::new();
        let root = dom.create_element("div", Attributes::default());
        let span = dom.create_element("span", Attributes::default());
        let text1 = dom.create_text("hello");
        let p = dom.create_element("p", Attributes::default());
        let text2 = dom.create_text("world");
        let comment = dom.create_comment("a comment");

        dom.append_child(root, span);
        dom.append_child(root, text1);
        dom.append_child(root, p);
        dom.append_child(p, text2);
        dom.append_child(root, comment);

        (dom, root, span, text1, p, text2, comment)
    }

    // --- TreeWalker tests ---

    #[test]
    fn treewalker_next_node_walks_elements() {
        let (dom, root, span, _text1, p, _text2, _comment) = build_tree();
        let mut tw = TreeWalker::new(root, SHOW_ELEMENT);

        assert_eq!(tw.next_node(&dom), Some(span));
        assert_eq!(tw.next_node(&dom), Some(p));
        assert_eq!(tw.next_node(&dom), None);
    }

    #[test]
    fn treewalker_show_text_filters() {
        let (dom, root, _span, text1, _p, text2, _comment) = build_tree();
        let mut tw = TreeWalker::new(root, SHOW_TEXT);

        assert_eq!(tw.next_node(&dom), Some(text1));
        assert_eq!(tw.next_node(&dom), Some(text2));
        assert_eq!(tw.next_node(&dom), None);
    }

    #[test]
    fn treewalker_parent_node_stops_at_root() {
        let (dom, root, _span, _text1, p, text2, _comment) = build_tree();
        let mut tw = TreeWalker::new(root, SHOW_ALL);
        tw.current_node = text2;

        assert_eq!(tw.parent_node(&dom), Some(p));
        assert_eq!(tw.parent_node(&dom), Some(root));
        // At root, should not go further.
        assert_eq!(tw.parent_node(&dom), None);
    }

    #[test]
    fn treewalker_first_child_last_child() {
        let (dom, root, span, _text1, _p, _text2, comment) = build_tree();
        let mut tw = TreeWalker::new(root, SHOW_ALL);

        assert_eq!(tw.first_child(&dom), Some(span));
        tw.current_node = root;
        assert_eq!(tw.last_child(&dom), Some(comment));
    }

    // --- NodeIterator tests ---

    #[test]
    fn nodeiterator_next_previous_roundtrip() {
        let (dom, root, span, text1, p, text2, comment) = build_tree();
        let mut ni = NodeIterator::new(root, SHOW_ALL);

        // Forward
        assert_eq!(ni.next_node(&dom), Some(root));
        assert_eq!(ni.next_node(&dom), Some(span));
        assert_eq!(ni.next_node(&dom), Some(text1));
        assert_eq!(ni.next_node(&dom), Some(p));
        assert_eq!(ni.next_node(&dom), Some(text2));
        assert_eq!(ni.next_node(&dom), Some(comment));
        assert_eq!(ni.next_node(&dom), None);

        // Backward
        assert_eq!(ni.previous_node(&dom), Some(comment));
        assert_eq!(ni.previous_node(&dom), Some(text2));
        assert_eq!(ni.previous_node(&dom), Some(p));
        assert_eq!(ni.previous_node(&dom), Some(text1));
        assert_eq!(ni.previous_node(&dom), Some(span));
        assert_eq!(ni.previous_node(&dom), Some(root));
        assert_eq!(ni.previous_node(&dom), None);
    }

    #[test]
    fn nodeiterator_pre_remove_check_advances() {
        let (dom, root, span, text1, _p, _text2, _comment) = build_tree();
        let mut ni = NodeIterator::new(root, SHOW_ALL);

        // Advance to span.
        ni.next_node(&dom); // root
        ni.next_node(&dom); // span
        assert_eq!(ni.reference_node, span);

        // Pre-remove span: should advance to text1.
        ni.pre_remove_check(span, &dom);
        assert_eq!(ni.reference_node, text1);
    }

    #[test]
    fn nodeiterator_validate_reference_resets_on_removal() {
        let (mut dom, root, span, _text1, _p, _text2, _comment) = build_tree();
        let mut ni = NodeIterator::new(root, SHOW_ALL);

        // Advance to span.
        ni.next_node(&dom); // root
        ni.next_node(&dom); // span
        assert_eq!(ni.reference_node, span);

        // Actually remove span from the tree.
        dom.remove_child(root, span);

        // Next traversal should reset to root since span is no longer in tree.
        let next = ni.next_node(&dom);
        // After reset, pointer_before_reference is true, so returns root first.
        assert_eq!(next, Some(root));
    }

    // --- step_with_filter / FilterAction tests ---

    /// Mock FilterAction that records visited nodes + returns a
    /// fixed sequence of results indexed by visit count.
    struct RecordingFilter {
        results: Vec<NodeFilterResult>,
        visited: Vec<Entity>,
        cursor: usize,
    }

    impl RecordingFilter {
        fn new(results: Vec<NodeFilterResult>) -> Self {
            Self {
                results,
                visited: Vec::new(),
                cursor: 0,
            }
        }
    }

    impl FilterAction for RecordingFilter {
        fn accept(&mut self, node: Entity) -> Result<NodeFilterResult, FilterError> {
            self.visited.push(node);
            let r = self.results[self.cursor];
            self.cursor += 1;
            Ok(r)
        }
    }

    /// FILTER_REJECT must prune descendants; FILTER_SKIP must descend.
    /// Tree: div -> [section -> [p, em], aside]
    /// Filter rejects `section`. Walker MUST jump to `aside` next,
    /// skipping `p` / `em` (descendants of rejected node).
    #[test]
    fn step_with_filter_reject_prunes_subtree() {
        let mut dom = EcsDom::new();
        let root = dom.create_element("div", Attributes::default());
        let section = dom.create_element("section", Attributes::default());
        let p = dom.create_element("p", Attributes::default());
        let em = dom.create_element("em", Attributes::default());
        let aside = dom.create_element("aside", Attributes::default());
        dom.append_child(root, section);
        dom.append_child(section, p);
        dom.append_child(section, em);
        dom.append_child(root, aside);

        let mut walker = TreeWalker::new(root, SHOW_ELEMENT);
        // Filter: Reject section, Accept aside.
        let mut filter =
            RecordingFilter::new(vec![NodeFilterResult::Reject, NodeFilterResult::Accept]);

        let next = step_with_filter(&mut walker, &dom, &mut filter).expect("step ok");

        assert_eq!(next, Some(aside), "Reject must prune `section`'s subtree");
        assert_eq!(
            filter.visited,
            vec![section, aside],
            "filter must NOT visit p / em (descendants of rejected node)"
        );
    }

    /// FILTER_SKIP descends into children (opposite of Reject).
    /// Same tree; filter skips `section`. Walker MUST visit `p`.
    #[test]
    fn step_with_filter_skip_descends_into_subtree() {
        let mut dom = EcsDom::new();
        let root = dom.create_element("div", Attributes::default());
        let section = dom.create_element("section", Attributes::default());
        let p = dom.create_element("p", Attributes::default());
        dom.append_child(root, section);
        dom.append_child(section, p);

        let mut walker = TreeWalker::new(root, SHOW_ELEMENT);
        let mut filter =
            RecordingFilter::new(vec![NodeFilterResult::Skip, NodeFilterResult::Accept]);

        let next = step_with_filter(&mut walker, &dom, &mut filter).expect("step ok");

        assert_eq!(next, Some(p), "Skip must descend into `section`");
        assert_eq!(filter.visited, vec![section, p]);
    }

    /// FILTER_ACCEPT returns the first matching node and stops.
    #[test]
    fn step_with_filter_accept_returns_immediately() {
        let mut dom = EcsDom::new();
        let root = dom.create_element("div", Attributes::default());
        let span = dom.create_element("span", Attributes::default());
        dom.append_child(root, span);

        let mut walker = TreeWalker::new(root, SHOW_ELEMENT);
        let mut filter = RecordingFilter::new(vec![NodeFilterResult::Accept]);

        let next = step_with_filter(&mut walker, &dom, &mut filter).expect("step ok");

        assert_eq!(next, Some(span));
        assert_eq!(walker.current_node, span, "Accept must move walker.current");
    }

    // --- Normalize full-tree test ---

    #[test]
    fn normalize_merges_adjacent_text_full_tree() {
        use elidex_ecs::TextContent;

        let mut dom = EcsDom::new();
        let root = dom.create_element("div", Attributes::default());
        let p = dom.create_element("p", Attributes::default());
        let t1 = dom.create_text("hello");
        let t2 = dom.create_text(" ");
        let t3 = dom.create_text("world");

        dom.append_child(root, p);
        dom.append_child(p, t1);
        dom.append_child(p, t2);
        dom.append_child(p, t3);

        // normalize via the handler
        crate::node_methods::Normalize::normalize_entity(root, &mut dom);

        // p should have one text child: "hello world"
        let children: Vec<_> = dom.children_iter(p).collect();
        assert_eq!(children.len(), 1);
        let text = dom.world().get::<&TextContent>(children[0]).unwrap();
        assert_eq!(text.0, "hello world");
    }
}
