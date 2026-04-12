//! Runtime state types for Promise / Generator / async-function coroutines.
//!
//! Referenced by `ObjectKind` variants in `value.rs`, by the coroutine
//! dispatch paths (`dispatch.rs` / `interpreter.rs`), and by the
//! implementation modules (`natives_promise`, `natives_promise_combinator`,
//! `natives_generator`).  `value.rs` re-exports everything here so
//! external `vm::value::X` paths continue to work.

use super::value::{CallFrame, JsValue, ObjectId, UpvalueId};

/// `[[GeneratorState]]` internal slot.  Tracks where in its lifecycle the
/// generator is, so `.next()` can reject invalid re-entry (running into
/// yourself).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeneratorStatus {
    /// Just created, body hasn't started.  First `.next()` enters the body.
    SuspendedStart,
    /// Paused at a `yield` expression.  Next `.next(arg)` pushes `arg` as
    /// the value of that `yield`.
    SuspendedYield,
    /// Currently executing body.  Re-entering via `.next()` throws TypeError.
    Running,
    /// Body returned or threw.  Subsequent `.next()` returns
    /// `{value: undefined, done: true}`.
    Completed,
}

/// Runtime state of a Generator object.  The inactive-phase frame state
/// (when `status != Running`) lives in `suspended`.
///
/// Async functions share this type: they're generators whose yielded
/// values are awaited Promises and whose completion settles an outer
/// wrapper Promise.  `wrapper` is `Some(promise_id)` for async
/// coroutines and `None` for user-visible generators.
pub struct GeneratorState {
    pub status: GeneratorStatus,
    pub suspended: Option<SuspendedFrame>,
    pub wrapper: Option<ObjectId>,
}

/// Saved state of a generator's call frame while the generator is paused.
///
/// `frame` is the original [`CallFrame`] moved out of `VmInner::frames`;
/// on resume it goes back, after rebasing `base`, `cleanup_base`, handler
/// stack depths, and any open upvalues pointing at this frame (stored in
/// `upvalue_slots`).  `stack_slice` is the portion of `VmInner::stack`
/// from `frame.base` up to `yield`'s pop point.
pub struct SuspendedFrame {
    pub frame: CallFrame,
    pub stack_slice: Vec<JsValue>,
    /// `(upvalue id, local slot)` pairs for every open upvalue that was
    /// referring to this frame's locals when the yield ran.  On save the
    /// upvalue is temporarily `Closed(value)`; on resume we write the
    /// closed value back into the stack slot and reopen.
    pub upvalue_slots: Vec<(UpvalueId, u16)>,
}

/// Shared state for a Promise combinator (§25.6.4.1–3).  `values` doubles
/// as the output array for `all`/`allSettled` and as the rejection-reasons
/// array for `any`; a single field keeps the variant compact.
pub struct PromiseCombinatorState {
    pub kind: CombinatorKind,
    pub result: ObjectId,
    pub values: Vec<JsValue>,
    pub remaining: u32,
    pub total: u32,
    /// Set to `true` on first short-circuit settle (e.g. `all` reject,
    /// `any` fulfill) to make subsequent steps no-ops — mirrors the
    /// spec's `alreadyCalled` record pattern.
    pub settled: bool,
}

/// Which combinator this state serves.  `Race` is absent because
/// `Promise.race` reuses the existing [`ObjectKind::PromiseResolver`]
/// machinery directly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombinatorKind {
    All,
    AllSettled,
    Any,
}

/// A per-item combinator callback.  Inline `state: ObjectId` + `index` keep
/// allocation O(1) per input, and the enum tag selects the semantic.
#[derive(Clone, Copy, Debug)]
pub enum PromiseCombinatorStep {
    /// `all`: fulfill → `values[index] = value`, dec counter, maybe resolve.
    AllFulfill { state: ObjectId, index: u32 },
    /// `all`: reject → short-circuit reject the result promise with the
    /// reason.  A single reject step is shared across all items.
    AllReject { state: ObjectId },
    /// `allSettled`: fulfill → build `{status:'fulfilled', value}` at index.
    AllSettledFulfill { state: ObjectId, index: u32 },
    /// `allSettled`: reject → build `{status:'rejected', reason}` at index.
    AllSettledReject { state: ObjectId, index: u32 },
    /// `any`: fulfill → short-circuit resolve the result promise.
    AnyFulfill { state: ObjectId },
    /// `any`: reject → store reason at index; last reject constructs
    /// AggregateError.
    AnyReject { state: ObjectId, index: u32 },
}

/// State of a Promise (ES2020 §25.6.6).
///
/// - `status` is the `[[PromiseState]]` internal slot.
/// - `result` is the `[[PromiseResult]]` (fulfilment value or rejection reason).
/// - `fulfill_reactions` / `reject_reactions` are appended by `.then()` while
///   the promise is Pending, and drained (queued as microtasks) on settle.
///   The lists are emptied on settle so they cannot hold GC roots beyond
///   that point.
/// - `handled` tracks whether a reject reaction has been attached — the
///   end-of-microtask-drain scan in `natives_promise` uses it to decide
///   whether to emit an unhandled-rejection warning.
pub struct PromiseState {
    pub status: PromiseStatus,
    pub result: JsValue,
    pub fulfill_reactions: Vec<Reaction>,
    pub reject_reactions: Vec<Reaction>,
    pub handled: bool,
}

/// `[[PromiseState]]` (ES2020 §25.6.6): Pending until the first resolve/reject,
/// then latched to Fulfilled or Rejected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PromiseStatus {
    Pending,
    Fulfilled,
    Rejected,
}

/// Which side of a reaction this is: the fulfill handler or the reject handler.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReactionKind {
    Fulfill,
    Reject,
}

/// A PromiseReaction Record (ES2020 §25.6.1.2).
///
/// - `handler` is the user callback; `None` indicates the default passthrough
///   (identity for Fulfill, rethrow for Reject) used when `.then()` omits an
///   argument.
/// - `capability` is the derived promise that the reaction resolves/rejects.
#[derive(Clone, Copy, Debug)]
pub struct Reaction {
    pub kind: ReactionKind,
    pub handler: Option<ObjectId>,
    pub capability: ObjectId,
}
