//! The `EcsDom` attribute accessor / write cluster.
//!
//! Extracted from `dom/mod.rs` (which is over the 1000-line review
//! convention) so the attribute read/write API lives in a focused module.
//! These methods form the attribute-write chokepoint:
//! [`set_attribute`](EcsDom::set_attribute) /
//! [`set_attribute_without_dispatch`](EcsDom::set_attribute_without_dispatch) /
//! [`remove_attribute`](EcsDom::remove_attribute) are the only sanctioned
//! `Attributes`-mutation entry points (each routes through the private
//! `write_attribute_no_dispatch` core, fires the derived-component reconcile
//! seam in [`super::attribute_reconcile`], syncs any materialized `Attr`
//! node, and — except the `_without_dispatch` variant — dispatches a
//! `MutationEvent`), and `get_attribute` / `with_attribute` / `has_attribute`
//! are their read siblings. Rust permits the inherent `impl EcsDom` to be
//! split across files in the same module.

use hecs::Entity;

use super::{EcsDom, MutationEvent};
use crate::components::{AttrData, AttrEntityCache, Attributes, NodeKind};

/// Outcome of an [`EcsDom::set_attribute`] chokepoint write, surfacing the
/// data the MutationObserver "attributes" record needs — WHATWG DOM §4.9
/// "handle attribute changes" step 1 (queue an "attributes" mutation record
/// carrying the pre-write `oldValue`). The write itself + the derived-state
/// fan-out (steps 2–3) still happen inside `set_attribute`; this return value
/// only lets the record-producing `ScriptSession` seam build the record from
/// the `oldValue` the chokepoint already captured, without re-reading the
/// `Attributes` component or re-forking the chokepoint.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AttributeWrite {
    /// `true` when the attribute was written (entity live + Element);
    /// `false` for a destroyed or non-Element receiver, where no mutation
    /// occurred and therefore no record must be produced.
    pub did_set: bool,
    /// The attribute's value immediately BEFORE this write, or `None` when
    /// the attribute was newly added (the record's `oldValue` is then null).
    pub old_value: Option<String>,
}

impl EcsDom {
    // ---- Attribute accessors ----

    /// Read attribute `name` on `entity`.
    ///
    /// Returns `None` when the value is not readable — covering the
    /// `Attributes` component absent / key not present cases AND any
    /// `World::get::<&Attributes>` failure (entity destroyed, hecs
    /// borrow conflict).  Callers cannot distinguish these from a
    /// genuinely-absent attribute; treat `None` as "no readable
    /// attribute" rather than "definitely no attribute".
    ///
    /// Allocates a fresh `String` for the present-value arm; prefer
    /// [`Self::with_attribute`] for borrow-only consumers (existence
    /// checks, equality comparisons, intern-on-Some) — that path
    /// keeps the value as `Option<&str>` and skips the `String::from`
    /// clone.
    #[must_use]
    pub fn get_attribute(&self, entity: Entity, name: &str) -> Option<String> {
        self.with_attribute(entity, name, |v| v.map(String::from))
    }

    /// Borrow attribute `name` on `entity` and project through `f`.
    ///
    /// `f` is called with `Some(value)` when the `Attributes`
    /// component is reachable and contains `name`, and `None`
    /// otherwise — covering not just absent-component / missing-key
    /// but every `World::get::<&Attributes>` failure (entity
    /// destroyed, borrow conflict).  Callers cannot distinguish
    /// these cases from `None`; treat it as "no readable attribute"
    /// rather than "definitely no attribute".  This is the
    /// zero-allocation sibling of [`Self::get_attribute`] —
    /// callers that only need to compare, parse, or hash the value
    /// can avoid the `String::from` clone the owned getter performs.
    /// Mirrors the closure-borrow `read_rel` pattern used internally
    /// for `TreeRelation` reads.
    ///
    /// The closure parameter is `for<'b> FnOnce(Option<&'b str>) -> R`
    /// so the borrowed `&str` cannot escape `f`'s scope: `hecs::World`
    /// supports interior-mutable borrows via `&World`, so leaking the
    /// `&str` past the internal `Ref<'_, Attributes>` guard could
    /// allow a later `&mut Attributes` borrow to alias it.
    pub fn with_attribute<R>(
        &self,
        entity: Entity,
        name: &str,
        f: impl for<'b> FnOnce(Option<&'b str>) -> R,
    ) -> R {
        match self.world.get::<&Attributes>(entity) {
            Ok(attrs) => f(attrs.get(name)),
            Err(_) => f(None),
        }
    }

    /// Returns `true` if `entity` has an `Attributes` component
    /// with `name` present.  Equivalent to
    /// `self.get_attribute(entity, name).is_some()` but skips the
    /// `String::from` clone.
    #[must_use]
    pub fn has_attribute(&self, entity: Entity, name: &str) -> bool {
        self.world
            .get::<&Attributes>(entity)
            .ok()
            .is_some_and(|attrs| attrs.contains(name))
    }

    /// Set attribute `name = value` on `entity`, inserting an
    /// `Attributes` component if one does not exist.
    ///
    /// On success, bumps [`rev_version`](Self::rev_version) so that
    /// live collections filtering on attribute state (e.g.
    /// `getElementsByClassName`, `getElementsByName`,
    /// `document.links`) invalidate any cached entity list at the
    /// next read.  Tag-only / topology-only filters (e.g.
    /// `getElementsByTagName`) over-invalidate harmlessly — the
    /// next read pays one walk and re-caches.  See the SP2 entity-
    /// list cache in `elidex-js::vm::host::dom_collection`.
    ///
    /// Returns an [`AttributeWrite`]: `did_set == false` (with `old_value ==
    /// None`) when the entity has been destroyed or is not an Element, else
    /// `did_set == true` with `old_value` = the pre-write value (`None` for a
    /// newly-added attribute). The `old_value` lets the record-producing
    /// `ScriptSession` seam build the §4.9 "attributes" record without
    /// re-reading the `Attributes` component.
    pub fn set_attribute(&mut self, entity: Entity, name: &str, value: &str) -> AttributeWrite {
        let (did_set, old_value) = self.write_attribute_no_dispatch(entity, name, value);
        if !did_set {
            return AttributeWrite::default();
        }
        // Fire `MutationEvent::AttributeChange` per DOM §4.3.2 +
        // §4.3.3; same-value writes still fire because spec
        // requires same-value records be queued for
        // MutationObserver consumers.  Per-consumer suppression
        // (e.g. `BaseUrlMaintainer` idempotent bump) lives in
        // the dispatcher's handle, not here.
        let event = MutationEvent::AttributeChange {
            node: entity,
            name,
            old_value: old_value.as_deref(),
            new_value: Some(value),
        };
        self.dispatch_event(&event);
        AttributeWrite {
            did_set: true,
            old_value,
        }
    }

    /// Like [`set_attribute`](Self::set_attribute) but WITHOUT firing the
    /// `MutationEvent::AttributeChange` dispatch — the strict subset
    /// (write `Attributes`, then
    /// [`reconcile_attribute_derived_components`](Self::reconcile_attribute_derived_components),
    /// then [`rev_version`](Self::rev_version)) that
    /// [`set_attribute`](Self::set_attribute) shares via the common
    /// (private) `write_attribute_no_dispatch` core.
    ///
    /// **Use only from INSIDE a [`MutationDispatcher`](super::MutationDispatcher) consumer**, where
    /// calling [`set_attribute`](Self::set_attribute) would violate the
    /// re-entry contract on the private `dispatch_event` mutation primitive
    /// (its `debug_assert!(dispatch_depth == 0)`).  HTML §4.10.5
    /// type-change step 1 (set the `value` content attribute from the
    /// `FormControlReconciler`) is the first such caller.
    ///
    /// ⚠ Suppressing the dispatch suppresses the **ENTIRE**
    /// `AttributeChange` consumer fan-out (every [`MutationDispatcher`](super::MutationDispatcher)
    /// consumer AND the MutationObserver record), not merely the observer
    /// record.  Any derived state a consumer would have maintained must
    /// be reproduced by the caller.  Reuse only where that total
    /// suppression is intended.
    ///
    /// Returns `false` if the entity has been destroyed or is not an
    /// Element (same contract as [`set_attribute`](Self::set_attribute)).
    pub fn set_attribute_without_dispatch(
        &mut self,
        entity: Entity,
        name: &str,
        value: &str,
    ) -> bool {
        self.write_attribute_no_dispatch(entity, name, value).0
    }

    /// Shared core of [`set_attribute`](Self::set_attribute) /
    /// [`set_attribute_without_dispatch`](Self::set_attribute_without_dispatch):
    /// write the attribute, reconcile inline derived components, and bump
    /// `rev_version` — but DO NOT dispatch.  Returns `(did_set, old_value)`
    /// where `old_value` is the pre-write attribute value (for the
    /// `MutationObserver` record), captured in the SAME `Attributes` borrow
    /// that decides insert-vs-set (the single-lookup fast path).
    fn write_attribute_no_dispatch(
        &mut self,
        entity: Entity,
        name: &str,
        value: &str,
    ) -> (bool, Option<String>) {
        if !self.contains(entity) {
            return (false, None);
        }
        // Engine-internal hardening (pre-D-31 `require_attrs_mut`
        // semantics): only Element entities carry `Attributes`.
        // Silently auto-attaching `Attributes` to Document / Text /
        // ShadowRoot / Comment entities would corrupt downstream
        // attribute readers; bail with `false` so caller sees the
        // mis-routed write the same way it sees a destroyed entity.
        if !matches!(self.node_kind(entity), Some(NodeKind::Element)) {
            return (false, None);
        }
        // Single component lookup: capture old_value AND component
        // presence from one borrow; if absent, insert a fresh
        // Attributes default below.
        let (old_value, has_component) = match self.world.get::<&Attributes>(entity) {
            Ok(a) => (a.get(name).map(String::from), true),
            Err(_) => (None, false),
        };
        let did_set = if has_component {
            if let Ok(mut attrs) = self.world.get::<&mut Attributes>(entity) {
                attrs.set(name, value);
                true
            } else {
                false
            }
        } else {
            let mut attrs = Attributes::default();
            attrs.set(name, value);
            self.world.insert_one(entity, attrs).is_ok()
        };
        if did_set {
            self.reconcile_attribute_derived_components(entity, name);
            self.rev_version(entity);
            self.sync_cached_attr_value(entity, name, value);
        }
        (did_set, old_value)
    }

    /// Keep any materialized `Attr` node (the entity `getAttributeNode(name)`
    /// returns) in sync with a chokepoint attribute write, so a captured
    /// `attr.value` reflects the new value without breaking Attr-node
    /// identity (WHATWG DOM §4.9 — the same object is returned across reads).
    ///
    /// This belongs in the [`set_attribute`](Self::set_attribute) chokepoint
    /// (not only the IDL `Element.setAttribute` handler) so that EVERY
    /// attribute write routed through the chokepoint — reflected IDL setters
    /// (`input.value` default mode, `defaultValue`, `formMethod`, …), the
    /// parser, and the reconciler's non-dispatching writes — keeps cached
    /// Attr nodes consistent.  A no-op when no Attr node was materialized for
    /// `name` (the common case).
    fn sync_cached_attr_value(&mut self, entity: Entity, name: &str, value: &str) {
        let cached_attr = self
            .world
            .get::<&AttrEntityCache>(entity)
            .ok()
            .and_then(|cache| cache.entries.get(name).copied());
        if let Some(attr_entity) = cached_attr {
            if let Ok(mut ad) = self.world.get::<&mut AttrData>(attr_entity) {
                value.clone_into(&mut ad.value);
            }
        }
    }

    /// Remove attribute `name` from `entity` if present, then bump
    /// [`rev_version`](Self::rev_version) — both gated on the
    /// entity still being live AND being an Element.
    ///
    /// Destroyed entities short-circuit before either write,
    /// matching [`set_attribute`](Self::set_attribute)'s contract.
    /// Non-Element entities (Document / Text / Comment / ShadowRoot)
    /// also short-circuit — symmetric to `set_attribute`'s
    /// Element-only guard.  Without this, a stray
    /// `remove_attribute(non_element, ...)` would still bump
    /// `inclusive_descendants_version` and dispatch
    /// [`MutationEvent::AttributeChange`], cascading version bumps
    /// to attribute-filtered live collections and triggering
    /// downstream `MutationEvent` consumers (e.g. `BaseUrlMaintainer`,
    /// living in `elidex-dom-api`) on a receiver that cannot
    /// semantically own attributes.
    ///
    /// The attribute-storage write is itself a no-op when the
    /// `Attributes` component is absent or the key is missing,
    /// but the version bump still fires for live Element entities
    /// so attribute-filtered live collections invalidate cleanly
    /// even on spurious removals (the next read pays one walk and
    /// re-caches under the freshly bumped version).  See the SP2
    /// entity-list cache in `elidex-js::vm::host::dom_collection`;
    /// the `set_attribute` rationale on over-invalidation applies
    /// here too.
    ///
    /// Returns the removed value (`Some` only when an attribute was actually
    /// present and removed), or `None` for a destroyed / non-Element receiver
    /// OR an absent attribute. The record-producing `ScriptSession` seam uses
    /// this `Some`/`None` to gate the §4.9 "attributes" record: `None` ⇒ no
    /// record (`removeAttribute("missing")` queues nothing), `Some(old)` ⇒
    /// record with `oldValue = old`.
    pub fn remove_attribute(&mut self, entity: Entity, name: &str) -> Option<String> {
        if !self.contains(entity) {
            return None;
        }
        // Symmetric to `set_attribute`'s Element-only guard
        // (line ~939): non-Element entities never own `Attributes`,
        // so a remove on them is meaningless and must not cascade
        // version bumps / mutation events.
        if !matches!(self.node_kind(entity), Some(NodeKind::Element)) {
            return None;
        }
        let old_value = self
            .world
            .get::<&mut Attributes>(entity)
            .ok()
            .and_then(|mut attrs| attrs.remove(name));
        self.reconcile_attribute_derived_components(entity, name);
        self.rev_version(entity);
        // Fire `MutationEvent::AttributeChange` ONLY when an attribute was
        // actually removed. DOM "remove an attribute by name" (§"remove an
        // attribute by name", step 2) removes — and thus queues a mutation
        // record via "handle attribute changes" — only when the attribute
        // is non-null; `removeAttribute("missing")` performs no mutation,
        // so MutationObserver consumers must not see a phantom removal.
        // (Unlike `set_attribute`, which always performs the mutation and
        // so queues even same-value writes.) The unconditional
        // `rev_version` above is a deliberate over-invalidation for
        // attribute-filtered live collections — distinct from the
        // observable mutation record gated here.
        if old_value.is_some() {
            let event = MutationEvent::AttributeChange {
                node: entity,
                name,
                old_value: old_value.as_deref(),
                new_value: None,
            };
            self.dispatch_event(&event);
        }
        old_value
    }
}
