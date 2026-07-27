//! The deferred **step vocabulary** — what a step on the [`TraversalQueue`] IS.
//!
//! [`TraversalDelta`] (the §7.4.3 delta that defers to a later task),
//! [`UserInvolvement`] (the §7.4.2.1 value §7.4.3 steps 2–3 resolve at issue time —
//! step 2 defaults it to "browser UI", step 3.3 overrides it to "none" for the
//! scripted case both shells take),
//! [`PendingTraversal`] (the two together — one deferred apply), and
//! [`PendingHistoryStep`] (the *tagged step-set* the one queue carries, §7.4.1.3
//! *Centralized modifications of session history*).
//!
//! [`TraversalDelta::from_history_action`] is the **traversal-vs-update
//! discriminator** the Phase-1b partition is built on: it splits a staged
//! [`HistoryAction`] into a §7.4.3 *traverse the history by a delta* and a §7.4.4
//! *URL and history update steps*. It is **not** by itself the defer / in-task
//! decision — an update issued *after* a same-turn traversal defers too, in issue
//! order (plan §4.5 I2), and an out-of-range first traversal is discarded rather
//! than deferred. Those calls belong to the coordinator's `seen_traversal` barrier
//! and to [`DrainHost::classify_traversal`]; the task-timing partition all of this
//! serves is described on [`super`].
//!
//! [`TraversalQueue`]: super::TraversalQueue
//! [`DrainHost::classify_traversal`]: super::DrainHost::classify_traversal

use elidex_script_session::HistoryAction;

/// A resolved session-history **traversal** delta — the subset of
/// [`HistoryAction`] that defers to a later task (WHATWG HTML §7.4.3 *traverse
/// the history by a delta*), separated from the synchronous
/// `PushState` / `ReplaceState` *updates* (§7.4.4) that stay in-task.
///
/// The delta is carried un-resolved: §7.4.6.1 *apply the history step* resolves
/// the target step index at **apply** time against the (possibly Phase-1-mutated)
/// entry list, so a deferred traversal must NOT pre-resolve a concrete index at
/// issue time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TraversalDelta {
    /// `history.back()` — delta −1.
    Back,
    /// `history.forward()` — delta +1.
    Forward,
    /// `history.go(delta)` — the raw signed delta (`0` = reload, History.go
    /// step 4).
    Go(i32),
}

impl TraversalDelta {
    /// Classify a staged [`HistoryAction`] as a deferred traversal, or `None` for
    /// a synchronous `PushState` / `ReplaceState` *update* (the Phase-1 /
    /// Phase-2 partition predicate, plan §4.5 I2).
    #[must_use]
    pub fn from_history_action(action: &HistoryAction) -> Option<Self> {
        match action {
            HistoryAction::Back => Some(Self::Back),
            HistoryAction::Forward => Some(Self::Forward),
            HistoryAction::Go(delta) => Some(Self::Go(*delta)),
            HistoryAction::PushState { .. } | HistoryAction::ReplaceState { .. } => None,
        }
    }
}

/// User navigation involvement (WHATWG HTML §7.4.2.1 *user navigation
/// involvement*, `#user-navigation-involvement`) — the §7.4.3 step-2 snapshot a
/// deferred traversal captures at **issue** time so the later §7.4.6.1 apply
/// reads the value as it was when the traversal was issued, not when it applies.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UserInvolvement {
    /// The traversal was initiated via browser UI (a chrome Back/Forward button).
    BrowserUi,
    /// Initiated via an element's activation behavior (a trusted click).
    Activation,
    /// Not user-initiated — the default for a scripted `history.back()` / `go()`.
    #[default]
    None,
}

/// A pending deferred **traversal apply** (WHATWG HTML §7.4.3 step 4 — the
/// traversal appended onto the traversable, applied as a later task via
/// §7.4.6.1). Carries the resolved [`TraversalDelta`] and the §7.4.3 **step-2
/// [`UserInvolvement`]** input captured at issue time.
///
/// The *fuller* §7.4.3 steps-1–3 **source snapshot** (source document / initiator
/// — consumed by §7.4.6.1 for the sandbox check and cross-document target
/// population) is **NOT** captured here: it references the shell's document
/// identity, a type the engine-agnostic substrate does not have. **Neither shell
/// slice threads it** — content (Slice A) and app (Slice B) both wire
/// `UserInvolvement` alone; the wire-time capture stays fenced to
/// `#11-sync-navigation-steps-queue-tagging` (the same document-identity boundary
/// as a deferred `SyncUpdate`, Codex PR#464 R3-D). So a deferred traversal's apply
/// reads live document state, not an issue-time source snapshot. Only the `Copy`
/// `UserInvolvement` input (no shell type) is capturable here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PendingTraversal {
    /// The resolved traversal delta (`Back` / `Forward` / `Go(delta)`).
    pub delta: TraversalDelta,
    /// The §7.4.3 step-2 [`UserInvolvement`] snapshot. **Both shells supply
    /// [`UserInvolvement::None`]**: only *scripted* `history.back()` /
    /// `forward()` / `go()` reaches the coordinator (§7.4.3 step 3.3 — a given
    /// sourceDocument overrides step 2's "browser UI" default), and the VM
    /// staging carries no involvement fact (Q-VM-MODEL = shell-drain-only). The
    /// [`UserInvolvement::BrowserUi`] traversals (chrome toolbar Back/Forward,
    /// Alt+←/→) bypass the queue entirely in BOTH shells — they call the shell's
    /// traversal body directly — and are fenced to Slice 4's canonical DIRECT-nav
    /// serialization (`#11-session-history-task-queue-model`).
    pub user_involvement: UserInvolvement,
}

/// One deferred step on the [`TraversalQueue`]. The spec's **one** session
/// history traversal queue carries *tagged step-sets* (WHATWG HTML §7.4.1.3
/// *Centralized modifications of session history* — Q-SYNC-FINALIZE): *traversal
/// steps* (§7.4.3 step 4) and *synchronous navigation steps* (§7.4.4 step 13).
///
/// elidex defers a step-set onto this queue **from the first traversal of a turn
/// onward**, preserving issue order (plan §4.5 I2 — *never reorder a sync update
/// ahead of a traversal issued before it*). A synchronous update issued **after**
/// a same-turn traversal therefore rides this queue as a tagged
/// [`Self::SyncUpdate`] rather than jumping ahead into Phase 1.
///
/// (No `PartialEq`: [`HistoryAction`] carries serialized state and is not `Eq`;
/// tests assert the coordinator's *observed apply order*, not step equality.)
///
/// [`TraversalQueue`]: super::TraversalQueue
#[derive(Clone, Debug)]
pub enum PendingHistoryStep {
    /// A deferred *traversal* (§7.4.3 → §7.4.6.1 *apply the history step*).
    Traversal(PendingTraversal),
    /// A synchronous `pushState` / `replaceState` *update* (§7.4.4) issued
    /// **after** a same-turn traversal, deferred onto the queue in issue order
    /// (plan §4.5 I2) rather than applied in Phase 1. Its exact same-turn
    /// *straddle* outcome is deliberately NOT pinned here (plan §4.5 I2 / §7
    /// Q-SYNC-FINALIZE — Slice 1/2 conformance-test territory); Slice 1 fixes only
    /// the issue-order-preserving **structure**.
    SyncUpdate(HistoryAction),
}
