//! The traversable's **session history traversal queue** + the shared
//! **drain-coordinator** — the additive substrate of the session-history
//! task-boundary phase-separation
//! (`docs/plans/2026-07-session-history-task-queue-model.md`, Slice 1).
//!
//! elidex historically drained a *turn's* staged navigation intents in one
//! synchronous pass (window-opens → history FIFO → last-wins navigation),
//! collapsing the spec's two task-timing classes onto a single synchronous
//! return (plan §1). This module introduces, in its **final phase-separated
//! shape**, the primitive **both shells now drive** — content mode from
//! `content/event_loop.rs` through `content/drain_host.rs` (Slice A), app mode
//! from `app/drain_host/host.rs` (Slice B):
//!
//! - a [`TraversalQueue`] — the WHATWG HTML §7.3.1.1 *session history traversal
//!   queue* (`#tn-session-history-traversal-queue`) carrying the
//!   **"running nested apply history step" boolean** — realized as a
//!   **cooperative deferred queue on elidex's single-writer event loop**, NOT an
//!   OS parallel-queue thread (plan §4.1; CLAUDE.md "Concurrency by ownership and
//!   phases"); and
//! - a [`DrainCoordinator`] — the phase-partition driver, parameterized by the
//!   [`DrainHost`] trait so `ContentState` / `InteractiveState` / the pipeline /
//!   `EcsDom` stay **behind the trait** and never cross the `elidex-navigation`
//!   crate boundary (plan §4.5 "OO→ECS / layer map").
//!
//! Slice A co-designs the substrate with its first consumer (content mode) — the
//! peek-classify (`classify_traversal`), nav-suppression (`handle_navigation`
//! drain-and-discard), and deferred-`SyncUpdate` cancellation (Phase 2 drops a
//! straddle sync behind ANY traversal, Resolution D generalized) seams are each
//! designed **correct against the real shell state** the inert substrate lacked
//! (`docs/plans/2026-07-session-history-slice-A-content-phase-separation.md`).
//! The isolation unit tests below still pin the coordinator in isolation; **both
//! shells now drive it** — content mode from `content/event_loop.rs` (Slice A,
//! the split entry points) and app mode from `app/drain_host/mod.rs` (Slice B,
//! the same-turn entry point, iterated to quiescence by that drive site).
//!
//! ## The task-timing partition (plan §4.2)
//!
//! - **Phase 1 — synchronous, in-task:** window-opens (§7.2.2.1) → synchronous
//!   history *updates* (`pushState` / `replaceState`, WHATWG HTML §7.4.4 *URL and
//!   history update steps*) → last-wins navigation (`location.*`, §7.4.2). These
//!   mutate the session history / rebuild the pipeline in the current task.
//! - **Phase 2 — deferred traversal apply (a later task):** a `Back` / `Forward`
//!   / `Go` *traversal* (§7.4.3 *traverse the history by a delta* step 4 —
//!   "append … traversal steps to traversable") is **not** applied inline; it is
//!   appended to the [`TraversalQueue`] and applied *after* Phase 1's updates have
//!   landed, realizing §7.4.6.1 *apply the history step* step 12's two-part split
//!   ("synchronous navigations processed before documents unload").
//!
//! The two phases are **separately callable** so the shell can realize the task
//! boundary: [`DrainCoordinator::drain_synchronous_phase`] runs Phase 1 (window-
//! opens + sync updates + last-wins navigation) and enqueues traversals **without
//! applying them**; [`DrainCoordinator::run_deferred_traversals`] runs Phase 2
//! (the deferred traversal apply) on a **later turn**. Content-mode drives that
//! split pair (Phase 2 on a subsequent async-pump turn), plus
//! [`DrainCoordinator::drain_synchronous_updates`] as its top-of-turn settle.
//! **App-mode calls none of those three**: it has no async pump, so it drains
//! Phase 1 and Phase 2 back-to-back inside the input handler through
//! [`DrainCoordinator::drain_same_turn`], the **same-turn** entry point that
//! combines both phases in one call and ships once (the app-mode-degenerate path
//! plus the isolation tests) — **repeated to quiescence by the drive site**, which
//! iterates that call plus its own reinstatement tail until the turn's handlers
//! have staged nothing new. That loop is shell-side schedule policy and this
//! substrate is unchanged by it: every statement below holds PER ITERATION.
//! Content-mode adopting `drain_same_turn` wholesale
//! would collapse the very task boundary this substrate exists to remove, so it
//! drives the split entry points separately (see each method's doc).
//!
//! The **scope fence** (plan §0) is single-traversable (top-level) only: the
//! §7.4.6.1 multi-navigable fan-out (steps 3/4/6/7 + the per-navigable global
//! task of 8/12) is B1-gated and NOT modelled here.
//!
//! ## Module layout
//!
//! One concern per file, in dependency order (a strict chain — `step` → `queue` →
//! `host` → `coordinator`, no back-edges). The submodules are **private**, so the
//! split added no new path any code *outside* this module can name: every existing
//! public path still resolves unchanged — `traversal_queue::DrainHost` and the
//! crate-root `elidex_navigation::DrainHost` — via the `pub use` facade below.
//! *Within* `traversal_queue` and its descendants the submodule path is of course
//! nameable and is what the siblings use (`use super::host::DrainHost;`); the
//! facade's job is to keep that an implementation detail of this module rather
//! than a second spelling its dependents have to choose between:
//!
//! - `step.rs` — the deferred **step vocabulary** ([`TraversalDelta`],
//!   [`UserInvolvement`], [`PendingTraversal`], [`PendingHistoryStep`]) plus the
//!   traversal-vs-update discriminator the Phase-1b partition is built on.
//! - `queue.rs` — [`TraversalQueue`]: the §7.3.1.1 FIFO + nested-apply guard,
//!   **state only**.
//! - `host.rs` — the [`DrainHost`] contract, and with it the two divergences that
//!   are properties of the **contract** (the §7.4.3 issue-time hoist at
//!   [`DrainHost::classify_traversal`], the §7.4.2.2 step-19 suppression
//!   divergence at [`DrainHost::handle_navigation`]).
//! - `coordinator.rs` — [`DrainCoordinator`] + [`DrainOutcome`]: the phase
//!   ordering and the plan §4.5 I1/I2/I3 invariants — plus the substrate's
//!   **third** divergence, which is a property of the Phase-2 *drain* rather than
//!   of the contract: a §7.4.4 intent staged by a `popstate` handler mid-drain is
//!   not consumed before the next queued traversal runs, against §7.4.6.1 step
//!   14's note. It is the only one of the three still fenced to an open slot
//!   (`#11-sync-navigation-steps-queue-tagging`), so a divergence inventory must
//!   read both files.

mod coordinator;
mod host;
mod queue;
mod step;

pub use coordinator::{DrainCoordinator, DrainOutcome};
pub use host::DrainHost;
pub use queue::TraversalQueue;
pub use step::{PendingHistoryStep, PendingTraversal, TraversalDelta, UserInvolvement};

#[cfg(test)]
#[path = "../traversal_queue_tests.rs"]
mod tests;
