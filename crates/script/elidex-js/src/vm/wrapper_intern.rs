//! Unified wrapper-identity seam (`#11-wrapper-identity-seam`).
//!
//! Collapses the per-purpose wrapper-IDENTITY caches that used to live as
//! ~24 separate side-maps (the primary node `wrapper_cache` + the 23
//! `VmInner::*_wrapper_cache` / `*_wrappers` fields) into ONE store keyed by
//! [`WrapperKey`], reached only through the seam API on `VmInner`
//! ([`super::VmInner::intern_wrapper`] get-or-create /
//! [`super::VmInner::get_wrapper`] read / [`super::VmInner::set_wrapper`]
//! eager-insert), one GC mark pass + one sweep pass dispatched by
//! [`WrapperKind::mark_agent`] / [`WrapperKind::retain`], and one
//! `retain` keeping only [`WrapperKind::Node`] on `Vm::unbind`.
//!
//! Each cache satisfied a Web IDL `[SameObject]` invariant: the same wrapper
//! object must be returned on every access (`el.classList === el.classList`,
//! `el.attributes`, `sheet.cssRules[i].style`, `input.files`, …). The seam
//! preserves that by construction — one identity per [`WrapperKey`].
//!
//! Store residence is **per-VM `HostData`**, cleared on `Vm::unbind`, NOT an
//! ECS component: the value is a per-VM JS wrapper [`ObjectId`] that aliases
//! across DOMs if hosted on the entity (an `EcsDom` shares its entity-index
//! space across VMs and rebinds — lesson #195). Migration to a per-entity
//! `WrapperRefs` component is deferred to `#11-wrapper-identity-component-migration`,
//! gated on the `world_id` discriminator; this seam keeps the store stable
//! behind the API so that later swap touches no call sites.
//!
//! **⚠ SUPERSEDED (2026-06-30, PR #434
//! `docs/plans/2026-06-agent-scoped-ecsdom-world.md`)**: the `world_id` gate is
//! retracted. Under the **agent-scoped `EcsDom` World** (1 agent = 1 World = 1
//! Vm) the cross-DOM aliasing above is unconstructable, so the migration is safe
//! without `world_id` — but the component must be keyed by **`(entity, realm)`**
//! (one Vm hosts multiple Window realms; see that decision §5 req 6/7). This
//! per-VM `HostData` residence stays the sanctioned interim form until B1.

use super::value::{ObjectId, StringId};
use elidex_ecs::Entity;

/// Owner-identity domain for an interned wrapper.
///
/// Entity-owned wrappers (classList / dataset / style / collections / …) hang
/// off a DOM [`Entity`]; the two wrapper-owned caches (`<input>.files` FileList,
/// `DataTransfer` items) hang off an owning JS wrapper [`ObjectId`]; the
/// string-keyed wrappers hang off an interned [`StringId`] — a Service Worker
/// registration scope URL (`ServiceWorkerRegistration` / `ServiceWorker`) or a
/// `Client` id (UUID).  These have no DOM `Entity` or owning `Object`; their
/// identity IS a string the UA mints (SW §3.1/§3.2/§4.2 per-realm object maps).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum WrapperOwner {
    Entity(Entity),
    Object(ObjectId),
    /// Interned-string identity (SW scope URL / `Client` id) — see type docs.
    Scope(StringId),
}

/// Discriminates *which kind* of wrapper a given owner carries, so one owner
/// can hold many wrappers without collision (the pre-seam design used a
/// separate `HashMap` per kind to keep `classList` / `dataset` / `style` for
/// the same `Entity` apart — the kind field replaces that disambiguation).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum WrapperKind {
    /// Primary node-entity → its own DOM wrapper (the anchor; was
    /// `HostData::wrapper_cache`).
    Node,
    /// `Attr` per (element, attribute-name) — `subkey = Name(qname)`.
    Attr,
    ClassList,
    Dataset,
    RelList,
    LinkRelList,
    LinkSizes,
    OutputHtmlFor,
    InlineStyle,
    StyleSheet,
    /// `CSSStyleRule` per (sheet-entity, rule_id) — `subkey = RuleId`.
    CssStyleRule,
    /// rule-source `CSSRuleStyleDeclaration` per (sheet-entity, rule_id) —
    /// `subkey = RuleId`.
    RuleStyle,
    ValidityState,
    OptionsCollection,
    FormControlsCollection,
    MapAreas,
    TableRows,
    TableBodies,
    TableSectionRows,
    TableRowCells,
    DatalistOptions,
    /// `canvas.getContext('2d')` rendering-context wrapper — `owner =
    /// Entity(canvas)`. The wrapper shares the canvas `Element` entity in its
    /// `entity_bits`, so this seam entry doubles as the brand: a HostObject is a
    /// 2D context iff it is the interned `Canvas2dContext` wrapper for its
    /// entity (`vm/host/canvas/mod.rs`). Weak-through-owner: kept alive while
    /// the canvas element wrapper is.
    Canvas2dContext,
    /// `offscreenCanvas.getContext('2d')` rendering-context wrapper — `owner =
    /// Entity(OC entity)`. Mirror of [`Self::Canvas2dContext`] but owned by the
    /// `NodeKind::OffscreenCanvas` entity instead of a canvas `Element`. The
    /// wrapper shares the OC entity in its `entity_bits`, so this seam entry
    /// doubles as the brand: a HostObject is an `OffscreenCanvasRenderingContext2D`
    /// iff it is the interned `OffscreenCanvas2dContext` wrapper for its entity
    /// (`vm/host/offscreen_canvas/mod.rs`). Weak-through-owner: kept alive while
    /// the primary OC wrapper is cached. The primary OC wrapper itself goes
    /// through `cache_wrapper` / `WrapperKind::Node` (TRUE Worker precedent,
    /// see `worker.rs:400`) — no separate `WrapperKind::OffscreenCanvas`
    /// variant.
    OffscreenCanvas2dContext,
    /// `<input>.files` FileList — `owner = Object(input wrapper)`.
    FileList,
    /// `DataTransferItem` per (DataTransfer wrapper, index) —
    /// `owner = Object(dt wrapper)`, `subkey = Index`.
    DataTransferItem,
    /// `navigator.serviceWorker.getRegistration()` / `.ready` etc. → the per-
    /// realm `ServiceWorkerRegistration` for a scope (`owner = Scope(scope)`).
    /// One object per scope satisfies the SW §3.2 registration object map
    /// (`reg === getRegistration()`).
    ServiceWorkerRegistration,
    /// A registration's `installing`/`waiting`/`active` worker + the
    /// `controller` (`owner = Scope(scope)`).  One object per scope satisfies
    /// the SW §3.1 service worker object map and keeps identity across state
    /// transitions (`#update-worker-state` mutates in place).
    ServiceWorker,
    /// `clients.get(id)` / `matchAll()` → the per-realm `Client` for a client
    /// id (`owner = Scope(client id)`).  One object per id satisfies the SW
    /// §4.2 `[SameObject]` invariant (`clients.get(id) === clients.get(id)`).
    Client,
}

/// The optional secondary discriminator, folding the heterogeneous key tails
/// of the pre-seam maps (`(Entity, StringId)` / `(Entity, u64)` /
/// `(ObjectId, u32)`) into the single key.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum WrapperSubkey {
    None,
    /// Attribute qualified-name (`attr_wrapper_cache`).
    Name(StringId),
    /// CSSOM rule id (`css_style_rule_wrapper_cache` / `rule_style_wrapper_cache`).
    RuleId(u64),
    /// DataTransfer item index (`data_transfer_item_wrapper_cache`).
    Index(u32),
}

/// Unified key. Replaces the 6 distinct pre-seam key shapes
/// (`u64` / `Entity` / `(Entity, StringId)` / `(Entity, u64)` / `ObjectId` /
/// `(ObjectId, u32)`).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) struct WrapperKey {
    pub(crate) owner: WrapperOwner,
    pub(crate) kind: WrapperKind,
    pub(crate) subkey: WrapperSubkey,
}

impl WrapperKey {
    /// Entity-owned wrapper with no secondary discriminator (classList,
    /// dataset, style, the collections, the primary Node, …).
    pub(crate) fn entity(owner: Entity, kind: WrapperKind) -> Self {
        Self {
            owner: WrapperOwner::Entity(owner),
            kind,
            subkey: WrapperSubkey::None,
        }
    }
    /// Entity-owned wrapper with a name subkey (`Attr`).
    pub(crate) fn entity_named(owner: Entity, kind: WrapperKind, name: StringId) -> Self {
        Self {
            owner: WrapperOwner::Entity(owner),
            kind,
            subkey: WrapperSubkey::Name(name),
        }
    }
    /// Entity-owned wrapper with a CSSOM rule-id subkey.
    pub(crate) fn entity_rule(owner: Entity, kind: WrapperKind, rule_id: u64) -> Self {
        Self {
            owner: WrapperOwner::Entity(owner),
            kind,
            subkey: WrapperSubkey::RuleId(rule_id),
        }
    }
    /// Object-owned wrapper with no secondary discriminator (`FileList`).
    pub(crate) fn object(owner: ObjectId, kind: WrapperKind) -> Self {
        Self {
            owner: WrapperOwner::Object(owner),
            kind,
            subkey: WrapperSubkey::None,
        }
    }
    /// Object-owned wrapper with an index subkey (`DataTransferItem`).
    pub(crate) fn object_indexed(owner: ObjectId, kind: WrapperKind, index: u32) -> Self {
        Self {
            owner: WrapperOwner::Object(owner),
            kind,
            subkey: WrapperSubkey::Index(index),
        }
    }
    /// String-keyed wrapper with no secondary discriminator (SW
    /// `ServiceWorkerRegistration` / `ServiceWorker` by scope, `Client` by id).
    pub(crate) fn scope(owner: StringId, kind: WrapperKind) -> Self {
        Self {
            owner: WrapperOwner::Scope(owner),
            kind,
            subkey: WrapperSubkey::None,
        }
    }
}

/// Who marks an interned wrapper during GC mark. Read from
/// [`WrapperKind::mark_agent`]; faithful to the pre-seam behavior of each cache
/// (verified against `gc/roots.rs` / `gc/trace.rs` / `host_data.rs`).
///
/// The seam unifies the *store*, the *retain pass*, and the get-or-create
/// *API* — not the number of mark agents, of which there were always several.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum MarkAgent {
    /// `Node` (#1): strong root — the seam mark loop marks it unconditionally
    /// (replaces the `wrapper_cache.values()` chain in
    /// `HostData::gc_root_object_ids`). Pruned only via
    /// `HostData::remove_wrapper` on entity despawn.
    StrongRoot,
    /// Entity-owned secondaries: marked iff the owner element's primary `Node`
    /// wrapper is still cached (weak-through-owner gate).
    WeakViaOwnerEntity,
    /// CSSOM rule wrappers: `WeakViaOwnerEntity` gate AND the rule_id is still
    /// live in the sheet (`active_cssom_rule_ids`).
    WeakViaOwnerEntityAndRuleId,
    /// `FileList` (#23): marked by the owning `<input>` `HostObject`'s trace
    /// fan-out, NOT the seam mark loop. The seam loop skips it.
    ViaOwnerTrace,
    /// `DataTransferItem` (#24): no proactive mark anywhere — the item wrapper
    /// survives only if independently JS-reachable. The seam loop skips it.
    NoProactiveMark,
}

/// How an interned wrapper is pruned during GC sweep. Read from
/// [`WrapperKind::retain`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum RetainPredicate {
    /// `Node` (#1): never value-swept; dropped only via
    /// `HostData::remove_wrapper` on entity despawn.
    NeverSweep,
    /// Drop iff the wrapper `ObjectId` value was collected this cycle.
    ValueMark,
    /// Drop iff the wrapper value OR its owner/parent `ObjectId` was collected
    /// (`FileList` / `DataTransferItem` two-predicate prune).
    ValueAndOwnerMark,
}

impl super::VmInner {
    /// Get-or-create the wrapper interned at `key`. Returns the cached wrapper
    /// if present, else runs `alloc` (the per-kind wrapper constructor) and
    /// interns the result. The single entry point that replaced the ~23
    /// `alloc_or_cached_*` per-cache helpers (each now a thin shim over this).
    ///
    /// The store lives on `HostData`; `alloc` runs against `&mut VmInner`. The
    /// store read is dropped before `alloc` so the closure's `&mut self` does
    /// not alias the store borrow.
    pub(crate) fn intern_wrapper(
        &mut self,
        key: WrapperKey,
        alloc: impl FnOnce(&mut Self) -> ObjectId,
    ) -> ObjectId {
        if let Some(hd) = self.host_data.as_deref() {
            if let Some(&id) = hd.wrapper_store.get(&key) {
                return id;
            }
        }
        let id = alloc(self);
        if let Some(hd) = self.host_data.as_deref_mut() {
            hd.wrapper_store.insert(key, id);
        }
        id
    }

    /// Read-only lookup of an interned wrapper.
    pub(crate) fn get_wrapper(&self, key: WrapperKey) -> Option<ObjectId> {
        self.host_data
            .as_deref()
            .and_then(|hd| hd.wrapper_store.get(&key).copied())
    }

    /// Intern an already-allocated wrapper `id` under `key`, overwriting
    /// any prior entry. For the sites that compute the wrapper first
    /// (e.g. reattaching an existing `Attr`, or a `<template>.content`
    /// fragment built through the full `NativeContext`) and so cannot
    /// use the `intern_wrapper` get-or-create closure. No-op when no
    /// `HostData` is installed.
    pub(crate) fn set_wrapper(&mut self, key: WrapperKey, id: ObjectId) {
        if let Some(hd) = self.host_data.as_deref_mut() {
            hd.wrapper_store.insert(key, id);
        }
    }

    /// Remove an interned wrapper by [`WrapperKey`], returning the dropped
    /// `ObjectId`.  The `Entity`+`Node`-hardwired `HostData::remove_wrapper`
    /// cannot prune the `Scope`-owned SW wrappers, so a `ServiceWorker`
    /// registration removed by `unregister` / replacement is pruned here
    /// (R2-2).  No-op when no `HostData` is installed.
    pub(crate) fn remove_wrapper_keyed(&mut self, key: WrapperKey) -> Option<ObjectId> {
        self.host_data
            .as_deref_mut()
            .and_then(|hd| hd.wrapper_store.remove(&key))
    }
}

impl WrapperKind {
    pub(crate) fn mark_agent(self) -> MarkAgent {
        match self {
            Self::Node => MarkAgent::StrongRoot,
            Self::CssStyleRule | Self::RuleStyle => MarkAgent::WeakViaOwnerEntityAndRuleId,
            Self::FileList => MarkAgent::ViaOwnerTrace,
            // The SW client wrappers are scope/id-owned, not Entity-owned, so the
            // seam mark loop (which extracts an owner `Entity`) must SKIP them.
            // Live `ServiceWorkerRegistration`/`ServiceWorker` are kept by the
            // explicit `sw_registrations` registry-walk mark loop (`gc/collect.rs`);
            // a `Client` survives only while independently JS-reachable.
            Self::DataTransferItem
            | Self::ServiceWorkerRegistration
            | Self::ServiceWorker
            | Self::Client => MarkAgent::NoProactiveMark,
            // All other entity-owned secondaries.
            Self::Attr
            | Self::ClassList
            | Self::Dataset
            | Self::RelList
            | Self::LinkRelList
            | Self::LinkSizes
            | Self::OutputHtmlFor
            | Self::InlineStyle
            | Self::StyleSheet
            | Self::ValidityState
            | Self::OptionsCollection
            | Self::FormControlsCollection
            | Self::MapAreas
            | Self::TableRows
            | Self::TableBodies
            | Self::TableSectionRows
            | Self::TableRowCells
            | Self::DatalistOptions
            | Self::Canvas2dContext
            | Self::OffscreenCanvas2dContext => MarkAgent::WeakViaOwnerEntity,
        }
    }

    pub(crate) fn retain(self) -> RetainPredicate {
        match self {
            Self::Node => RetainPredicate::NeverSweep,
            Self::FileList | Self::DataTransferItem => RetainPredicate::ValueAndOwnerMark,
            _ => RetainPredicate::ValueMark,
        }
    }
}
