//! Typed [`MutationDispatcher`] impl composing mutation consumers.
//!
//! Field-order = dispatch-order = compile-time-visible.  Adding a
//! consumer = adding a typed field + a `handle` call line.  No
//! runtime registration API, no subscriber-list pattern, no implicit
//! ordering dependency.
//!
//! # Crate placement
//!
//! Originally placed in `elidex_dom_api` alongside its first 3
//! consumers (D-31).  Relocated to `elidex-js` (the binding crate)
//! when [`FormControlReconciler`] from `elidex-form` was added as
//! the 4th consumer — `elidex-form` already depends on
//! `elidex-dom-api`, so embedding a form-control-typed field in a
//! composer that lived in `elidex-dom-api` would have introduced a
//! circular cargo dep.  The composer is binding-layer infrastructure
//! (wires every consumer at VM-init time), so its natural home is the
//! binding crate that depends on both `elidex-dom-api` AND
//! `elidex-form`.
//!
//! # Lock ordering
//!
//! Each consumer's lock is acquired in a **disjoint** scope — no
//! nested locking.  The Range-side guard drops before the
//! NodeIterator-side guard is acquired (inside [`LiveRangeBridge`]
//! and [`NodeIteratorAdjuster`] respectively).

#![deny(clippy::significant_drop_tightening)]

use elidex_api_canvas::CanvasReconciler;
use elidex_dom_api::{BaseUrlMaintainer, LiveRangeBridge, NodeIteratorAdjuster};
use elidex_ecs::{EcsDom, MutationDispatcher, MutationEvent};
use elidex_form::FormControlReconciler;
use elidex_script_session::EventHandlerAttributeConsumer;

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
    /// Form-control derived-state reconciliation
    /// ([`elidex_form::FormControlState`] fields re-derived on
    /// attribute mutations + FCS attach on form-control element
    /// insertion).  HTML §4.10.18.3 form-associated element insertion
    /// steps + WHATWG DOM §4.9 attribute change steps.  Added last in
    /// field order so previous consumers' invariants are preserved
    /// before form-derived state is updated.
    form_control: FormControlReconciler,
    /// Inline event-handler content attribute detection (`<button
    /// onclick="...">`). Dual-arm (AttributeChange + Insert) consumer
    /// recording uncompiled handler source into the [`EventListeners`]
    /// component; lazy compile happens VM-side at first read / dispatch.
    /// WHATWG HTML §8.1.8.1. Added last — it only writes
    /// [`EventListeners`], which no earlier consumer reads, so order is
    /// independent.
    ///
    /// [`EventListeners`]: elidex_script_session::EventListeners
    event_handler_attrs: EventHandlerAttributeConsumer,
    /// Canvas bitmap reset on `<canvas>` `width`/`height` content-attribute
    /// change (HTML §4.12.5 "set bitmap dimensions" — even a same-value write
    /// clears the bitmap + resets state). Driven from the `AttributeChange` SoT
    /// (not the IDL setter) so `setAttribute` + parser-baked attributes are
    /// covered. Order-independent — it only touches the `Canvas2dContext`
    /// component, which no other consumer reads.
    canvas: CanvasReconciler,
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
            form_control: FormControlReconciler,
            event_handler_attrs: EventHandlerAttributeConsumer,
            canvas: CanvasReconciler,
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
    /// invoked here.  [`FormControlReconciler`] also skips init —
    /// `create_form_control_state` is invoked at element-creation
    /// time (`Document::createElement` / parser path) for the pre-
    /// bind tree, so FCS attach is already complete by the time the
    /// dispatcher is installed; post-install Insert events handle
    /// dynamic additions.  Add a delegate call only when a future
    /// consumer derives state from pre-bind tree structure.
    pub fn initialize_consumers(&mut self, dom: &mut EcsDom) {
        self.base_url.initialize_from_tree(dom);
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
        self.form_control.handle(event, dom);
        self.event_handler_attrs.handle(event, dom);
        self.canvas.handle(event, dom);
    }
}
