//! The attribute-write chokepoint's derived-component reconcile seam.
//!
//! Extracted from `dom/mod.rs` (which is already over the 1000-line review
//! convention) so the attribute→component reconcile logic lives in a focused
//! module. See [`EcsDom::reconcile_attribute_derived_components`].

use hecs::Entity;

use super::EcsDom;
use crate::components::{Attributes, IframeData, InlineStyle};

impl EcsDom {
    /// Re-make every **inline-reconciled** attribute-derived component
    /// consistent with the post-write `Attributes`, after the `name` content
    /// attribute is written or removed. The invariant is a single one —
    /// *component = f(`Attributes`)* — and the two arms below are just its two
    /// realizations (lazy-drop vs eager-rederive) given each component's
    /// materialization policy. Called by both
    /// [`set_attribute`](Self::set_attribute) /
    /// [`remove_attribute`](Self::remove_attribute) (after the `Attributes`
    /// mutation) and the deferred session-mutation flush in
    /// `elidex_script_session::mutation::apply_mutation` (which is `pub`'s
    /// reason — it writes `Attributes` without entering the chokepoints).
    ///
    /// **Why an inline seam in `elidex-ecs` core, not a `MutationEvent`
    /// consumer** — the mechanism higher layers use (e.g. `DocumentBaseUrl` via
    /// `elidex-dom-api`'s `BaseUrlMaintainer`, which subscribes to the
    /// `MutationEvent::AttributeChange` the chokepoint *dispatches* right after
    /// this call): these two components' consistency is a **core** invariant
    /// that must hold even when no consumer layer is composed, and two callers
    /// (`navigate_iframe`, the deferred flush) reconcile deliberately *without*
    /// dispatching an event (double-load avoidance) — neither is reducible to a
    /// consumer. So the split is a layering boundary, not a duplicate path.
    /// The two components:
    /// - **`InlineStyle`** (memoized parse of `attrs("style")`, materialized
    ///   lazily on first CSSOM access via `elidex_dom_api::ensure_inline_style`):
    ///   a `style` write changes the source of truth, so **drop the cache** —
    ///   the next `el.style.*` read re-hydrates. CSSOM mutators re-warm after
    ///   their own `set_attribute` (`sync_to_attribute`), so this is
    ///   perf-neutral for `el.style.*` sequences. (Closes the InlineStyle half
    ///   of slot `#11-derived-component-attr-maintenance`.)
    /// - **[`IframeData`]** (a pure projection of the iframe content attributes,
    ///   [`IframeData::from_attributes`], HTML §4.8.5): **re-derive eagerly**,
    ///   but only for entities that *already* carry it (i.e. `<iframe>`) — never
    ///   attach to a non-iframe that happens to get a `src`/`name`/… attribute.
    ///   Mirrors the clone-policy re-derive (`dom::tree_clone`). This closes the
    ///   IframeData half of the same slot: a generic `setAttribute("src", …)`
    ///   now keeps `IframeData` consistent with its attributes (the prior path
    ///   left the component stale, so the next load used the old URL).
    pub fn reconcile_attribute_derived_components(&mut self, entity: Entity, name: &str) {
        if name == "style" {
            let _ = self.world.remove_one::<InlineStyle>(entity);
        }
        // Presence-gated: `IframeData` exists ⇔ the entity is an `<iframe>`
        // (attached at parse / clone). Re-derive from the post-write attributes;
        // an iframe always has an `Attributes` component, but fall back to the
        // default projection if it somehow does not.
        if self.world.get::<&IframeData>(entity).is_ok() {
            let derived = self.world.get::<&Attributes>(entity).map_or_else(
                |_| IframeData::default(),
                |a| IframeData::from_attributes(&a),
            );
            let _ = self.world.insert_one(entity, derived);
        }
    }
}
