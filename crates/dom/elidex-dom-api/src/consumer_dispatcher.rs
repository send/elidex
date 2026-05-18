//! Typed [`MutationDispatcher`] impl composing all D-31-era consumers.
//!
//! Replaces the v4-era `MutationBridge` (2-consumer composer of
//! `LiveRangeBridge` + NodeIteratorState) and closes the
//! `#11-mutation-hook-multiplexer` defer slot (typed N-way composer).
//!
//! Field-order = dispatch-order = compile-time-visible. Adding a
//! consumer = adding a typed field + a `handle` call line. No runtime
//! registration API, no subscriber-list pattern, no implicit ordering
//! dependency.
//!
//! # Lock ordering (PR-A2 plan-v4 §A-NI-1 Round 2 IMP-3)
//!
//! Each consumer's lock is acquired in a **disjoint** scope — no
//! nested locking. The Range-side guard drops before the
//! NodeIterator-side guard is acquired (inside [`LiveRangeBridge`]
//! and [`NodeIteratorAdjuster`] respectively).

#![deny(clippy::significant_drop_tightening)]

use elidex_ecs::{EcsDom, MutationDispatcher, MutationEvent};

use crate::element::document_base::BaseUrlMaintainer;
use crate::range::LiveRangeBridge;
use crate::traversal::NodeIteratorAdjuster;

/// Typed composer of the D-31-era mutation consumers.
pub struct ConsumerDispatcher {
    /// Range live-tracking — Range IDL per WHATWG DOM §5.5
    /// "Interface Range"; boundary-point adjustment behavior lives
    /// in the mutation algorithms themselves (DOM §4.2.3 remove,
    /// §4.10 CharacterData "replace data", §4.11 Text "split a Text
    /// node" step 7, §4.4 Node `normalize()` step 6.4).
    live_range: LiveRangeBridge,
    /// NodeIterator pre-removing-steps (WHATWG DOM §6.1).
    /// Handles Remove with descendants snapshot + dom access.
    node_iter: NodeIteratorAdjuster,
    /// Base URL maintenance for `<base>` elements (HTML §2.4.3 +
    /// §4.2.3 — Phase B consumer).  Handles Insert / Remove /
    /// AttributeChange filtered by `dom.is_base_element`.
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
