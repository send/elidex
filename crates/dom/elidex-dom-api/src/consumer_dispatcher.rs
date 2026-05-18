//! Typed [`MutationDispatcher`] impl composing mutation consumers.
//!
//! Field-order = dispatch-order = compile-time-visible.  Adding a
//! consumer = adding a typed field + a `handle` call line.  No
//! runtime registration API, no subscriber-list pattern, no implicit
//! ordering dependency.
//!
//! # Lock ordering
//!
//! Each consumer's lock is acquired in a **disjoint** scope — no
//! nested locking.  The Range-side guard drops before the
//! NodeIterator-side guard is acquired (inside [`LiveRangeBridge`]
//! and [`NodeIteratorAdjuster`] respectively).

#![deny(clippy::significant_drop_tightening)]

use elidex_ecs::{EcsDom, MutationDispatcher, MutationEvent};

use crate::element::document_base::BaseUrlMaintainer;
use crate::range::LiveRangeBridge;
use crate::traversal::NodeIteratorAdjuster;

/// Typed composer of the mutation consumers.
pub struct ConsumerDispatcher {
    /// Range live-tracking — Range IDL per WHATWG DOM §5.5; boundary-
    /// point adjustment lives in the mutation algorithms themselves
    /// (DOM §4.2.3 remove, §4.10 CharacterData "replace data",
    /// §4.11 Text "split a Text node" step 7, §4.4 Node `normalize()`
    /// step 6.4).
    live_range: LiveRangeBridge,
    /// NodeIterator pre-removing-steps (WHATWG DOM §6.1).
    node_iter: NodeIteratorAdjuster,
    /// Base URL maintenance for `<base>` elements (HTML §2.4.3 +
    /// §4.2.3).
    base_url: BaseUrlMaintainer,
}

impl ConsumerDispatcher {
    /// Construct a dispatcher from already-paired consumers.
    /// Caller (`Vm::bind` in `elidex-js`) constructs each consumer
    /// from the matching HostData state (LiveRangeRegistry pair,
    /// node_iterator_states_shared) and passes them in.
    #[must_use]
    pub fn new(live_range: LiveRangeBridge, node_iter: NodeIteratorAdjuster) -> Self {
        Self {
            live_range,
            node_iter,
            base_url: BaseUrlMaintainer,
        }
    }

    /// Initialize every consumer that needs to derive state from the
    /// current DOM tree at install time.  Called by the binding layer
    /// (e.g. `Vm::bind` in `elidex-js`) right BEFORE
    /// [`EcsDom::set_mutation_dispatcher`] hands ownership over —
    /// pre-bind nodes never produced [`MutationEvent`]s, so consumers
    /// that maintain ECS state derived from existing structure (today:
    /// [`BaseUrlMaintainer`]'s `BaseFrozenUrl` + `DocumentBaseUrl`
    /// components, both defined in `elidex_ecs`) would otherwise see
    /// an empty event stream and never observe the pre-bind subtree.
    ///
    /// Idempotent — each delegate is documented as a no-op on an
    /// already-initialized tree.  [`LiveRangeBridge`] and
    /// [`NodeIteratorAdjuster`] do not need init: their state lives
    /// outside the DOM tree (Range / NodeIterator handles are
    /// JS-allocated and can only exist post-bind), so they are not
    /// invoked here.  Add a delegate call only when a future consumer
    /// derives state from pre-bind tree structure.
    pub fn initialize_consumers(&mut self, dom: &mut EcsDom) {
        self.base_url.initialize_from_tree(dom);
    }

    /// Test-only constructor: only [`LiveRangeBridge`] is wired —
    /// [`NodeIteratorAdjuster`] gets a fresh default, [`BaseUrlMaintainer`]
    /// is stateless.  Used by Range-only test fixtures so they exercise
    /// the same composition path as production rather than a one-off
    /// back-door type alias.
    #[cfg(test)]
    #[must_use]
    pub fn for_range_only_test(live_range: LiveRangeBridge) -> Self {
        Self::new(live_range, NodeIteratorAdjuster::default())
    }
}

impl MutationDispatcher for ConsumerDispatcher {
    fn dispatch(&mut self, event: &MutationEvent<'_>, dom: &mut EcsDom) {
        // Field-order = dispatch-order.  All consumers see every
        // event; each pattern-matches variants of interest.
        // Lock-disjoint scope is preserved within each handler.
        self.live_range.handle(event, dom);
        self.node_iter.handle(event, dom);
        self.base_url.handle(event, dom);
    }
}
