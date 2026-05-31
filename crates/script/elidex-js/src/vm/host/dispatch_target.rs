//! Polymorphic `EventTarget` identity + the **listener-home adapter**
//! (WHATWG DOM §2.7 / §2.9).
//!
//! The spec models an `EventTarget` as exactly *(an event listener list, a
//! get-the-parent algorithm)*.  elidex realizes the listener list with one
//! storage type — [`elidex_script_session::EventListeners`] (the full §2.7
//! tuple: type / capture / once / passive + `ListenerKind::{Normal,
//! EventHandler}`) — living in one of **two homes**:
//!
//! - **`Node`** — the per-entity ECS `EventListeners` *component* (every DOM
//!   node, `Window`, `Document`, plus the entity-backed `Worker` /
//!   `MessagePort` / `Selection`).
//! - **`VmObject`** — the per-`ObjectId` [`VmInner::vm_event_listeners`]
//!   registry, for the non-entity `EventTarget`s (`AbortSignal`, the three
//!   IndexedDB targets, …).
//!
//! [`DispatchTarget`] names which home a receiver maps to, and this module's
//! adapter methods are the **single place** the per-home branch lives: the
//! shared dispatch core ([`super::event_target_dispatch`]) and the shared
//! `EventTarget.prototype` natives consult the adapter for every
//! listener-home touchpoint (read AND write) and carry no `match target
//! kind` of their own.  A new non-Node `EventTarget` added later supplies
//! only its get-the-parent (the propagation chain) and its `on<type>` attr
//! set, and inherits correct §2.7/§2.9 by construction.

#![cfg(feature = "engine")]

use elidex_script_session::event_dispatch::ListenerPlanEntry;
use elidex_script_session::{EventListeners, ListenerId};

use super::super::value::{JsValue, NativeContext, ObjectId, ObjectKind};

/// A dispatchable `EventTarget`'s identity (WHATWG DOM §2.7).  The closed
/// two-variant set keeps every `ObjectId` typing VM-side (no engine-type
/// leak into the `Entity`-typed session crate) and avoids `dyn` in the hot
/// dispatch loop.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum DispatchTarget {
    /// Entity-backed EventTarget — listeners live in the ECS
    /// `EventListeners` component on this entity.
    Node(elidex_ecs::Entity),
    /// Non-entity EventTarget — listeners live in
    /// [`VmInner::vm_event_listeners`] keyed by this `ObjectId`.
    VmObject(ObjectId),
}

/// Resolve `this` to its [`DispatchTarget`], or `None` for a receiver that
/// is not a dispatchable EventTarget (silent no-op surface, unchanged from
/// the pre-generalization `entity_from_this` policy).
///
/// - A `HostObject { entity_bits }` wrapper → [`DispatchTarget::Node`], but
///   ONLY while `HostData` is bound (a post-unbind retained wrapper must
///   not panic on the later `host.dom()` deref — the unbound-receiver
///   policy returns `None` here, same as `entity_from_this`).
/// - A non-Node `EventTarget` brand (`AbortSignal` / the IDB targets) →
///   [`DispatchTarget::VmObject`], but ONLY while `HostData` is installed.
///   A VmObject keeps its listener *metadata* in
///   [`VmInner::vm_event_listeners`] (no bind needed — it has no document),
///   but its callbacks live in `HostData::listener_store`; with no HostData
///   the `addEventListener` → `store_listener` write would panic, so this
///   collapses to the same silent-no-op `None` surface the unbound Node arm
///   uses (production installs HostData at engine construction).  Presence
///   suffices — unlike the Node arm, no *bound* check applies.
/// - Anything else → `None`.
pub(crate) fn target_from_this(ctx: &NativeContext<'_>, this: JsValue) -> Option<DispatchTarget> {
    let JsValue::Object(id) = this else {
        return None;
    };
    match ctx.vm.get_object(id).kind {
        ObjectKind::HostObject { entity_bits } => {
            if !ctx
                .vm
                .host_data
                .as_deref()
                .is_some_and(super::super::host_data::HostData::is_bound)
            {
                return None;
            }
            elidex_ecs::Entity::from_bits(entity_bits).map(DispatchTarget::Node)
        }
        ObjectKind::AbortSignal
        | ObjectKind::IdbRequest
        | ObjectKind::IdbTransaction
        | ObjectKind::IdbDatabase => ctx
            .vm
            .host_data
            .as_ref()
            .map(|_| DispatchTarget::VmObject(id)),
        _ => None,
    }
}

impl DispatchTarget {
    /// Read-only access to this target's listener home.  `f` runs with the
    /// borrowed [`EventListeners`]; returns `None` (treated as the empty
    /// list) when the home is absent — a zero-listener target, an unbound
    /// Node, or a despawned entity.
    pub(crate) fn with_listeners<R>(
        self,
        ctx: &mut NativeContext<'_>,
        f: impl FnOnce(&EventListeners) -> R,
    ) -> Option<R> {
        match self {
            DispatchTarget::Node(entity) => {
                let dom = ctx.host_if_bound()?.dom();
                let listeners = dom.world().get::<&EventListeners>(entity).ok()?;
                Some(f(&listeners))
            }
            DispatchTarget::VmObject(id) => ctx.vm.vm_event_listeners.get(&id).map(f),
        }
    }

    /// Mutable access to this target's listener home **without** creating
    /// it.  `None` (no-op) when the home is absent.  Used by the removal
    /// touchpoints (removeEventListener, dispatch-path `once`-removal,
    /// `{signal}`-abort detach, on\*-clear).
    pub(crate) fn with_listeners_mut<R>(
        self,
        ctx: &mut NativeContext<'_>,
        f: impl FnOnce(&mut EventListeners) -> R,
    ) -> Option<R> {
        match self {
            DispatchTarget::Node(entity) => {
                let dom = ctx.host_if_bound()?.dom();
                let mut listeners = dom.world_mut().get::<&mut EventListeners>(entity).ok()?;
                Some(f(&mut listeners))
            }
            DispatchTarget::VmObject(id) => ctx.vm.vm_event_listeners.get_mut(&id).map(f),
        }
    }

    /// Mutable access to this target's listener home, **lazily creating** it
    /// on first use (mirrors the node `EventListeners`-component lazy
    /// `insert_one`).  Used by the add touchpoints (addEventListener,
    /// on\*-set).  `None` only when a `Node` entity was despawned between
    /// receiver resolution and now (insert fails) — the caller then skips
    /// the paired `store_listener` so no orphan is left.
    pub(crate) fn with_listeners_mut_or_insert<R>(
        self,
        ctx: &mut NativeContext<'_>,
        f: impl FnOnce(&mut EventListeners) -> R,
    ) -> Option<R> {
        match self {
            DispatchTarget::Node(entity) => {
                let dom = ctx.host_if_bound()?.dom();
                if dom.world().get::<&EventListeners>(entity).is_err()
                    && dom
                        .world_mut()
                        .insert_one(entity, EventListeners::new())
                        .is_err()
                {
                    return None;
                }
                let mut listeners = dom.world_mut().get::<&mut EventListeners>(entity).ok()?;
                Some(f(&mut listeners))
            }
            DispatchTarget::VmObject(id) => {
                Some(f(ctx.vm.vm_event_listeners.entry(id).or_default()))
            }
        }
    }

    /// Remove a listener entry from this target's home (the §2.9 step 15
    /// `once` remove-before-call write, home-dispatched so the shared inner
    /// invoke carries no `match target kind`).
    pub(crate) fn remove_listener_entry(self, ctx: &mut NativeContext<'_>, id: ListenerId) {
        self.with_listeners_mut(ctx, |listeners| {
            listeners.remove(id);
        });
    }

    /// Bring an event-handler IDL attribute backing up to date before its
    /// callable is resolved (WHATWG HTML §8.1.8.1 "getting the current
    /// value of the event handler") — the **A-hoist**.  Node-only: the
    /// reconcile (inline-source lazy-compile / cleared-drop) is a
    /// content-attribute concern that is provably a no-op for the
    /// IDL-setter-only `VmObject` handlers (their `uncompiled`/`cleared`
    /// stay at default), so the `VmObject` arm is intentionally empty and
    /// the shared dispatch core stays free of any `match target kind`.
    pub(crate) fn reconcile_handler(self, ctx: &mut NativeContext<'_>, id: ListenerId) {
        if let DispatchTarget::Node(entity) = self {
            ctx.vm.ensure_event_handler_current(entity, id);
        }
    }

    /// Resolve the callable for a planned listener (the §2.9 dispatch
    /// callable-resolve + reconcile-read touchpoint).  Runs the handler
    /// reconcile (Node only, via [`Self::reconcile_handler`]) then reads the
    /// engine-side `listener_store`.  `None` = the listener was removed
    /// between plan-freeze and now (the §2.9 step 6 removed-field check) →
    /// the caller silently skips it.
    pub(crate) fn resolve_callable(
        self,
        ctx: &mut NativeContext<'_>,
        entry: &ListenerPlanEntry,
    ) -> Option<ObjectId> {
        if entry.is_handler {
            self.reconcile_handler(ctx, entry.id);
        }
        ctx.vm
            .host_data
            .as_deref()
            .and_then(|h| h.get_listener(entry.id))
    }
}
