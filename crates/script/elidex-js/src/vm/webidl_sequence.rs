//! WebIDL §3.2.21 ("`sequence<T>`") — convert a JS value to an IDL
//! sequence.
//!
//! Shared replacement for hand-rolled `sequence<T>` converters. Each
//! site previously inlined the same four pieces — `@@iterator`
//! resolution, `iter_next` loop, abrupt-completion `IteratorClose`
//! (§7.4.11), and a per-call cap — with subtle drift between copies
//! (notably the dense-`ObjectKind::Array` fast path, which skips
//! `Array.prototype[@@iterator]` overrides in violation of §3.2.21
//! step 2's `GetMethod`).
//!
//! Two entry points:
//! - [`webidl_sequence_to_vec`] resolves the iterator from `raw`; use
//!   it for plain `sequence<T>` members.
//! - [`webidl_iter_to_vec`] accepts a pre-resolved iterator object;
//!   use it for union resolution (`HeadersInit`, `URLSearchParamsInit`)
//!   where the caller probes `@@iterator` first to pick the sequence
//!   vs. record branch — the iterator factory must not be invoked
//!   twice.
//!
//! Strict spec compliance: no dense-Array fast path. An Array whose
//! `[Symbol.iterator]` is overridden honours the override at the cost
//! of one virtual call per element; the cap parameter bounds runaway
//! iterators independently of the dispatch.

#![cfg(feature = "engine")]

use super::value::{JsValue, NativeContext, VmError};

/// Caller-supplied wording for [`webidl_sequence_to_vec`]'s three
/// iterator-protocol failure modes.  Pre-formatted so each interface
/// can keep its own legacy message (`"Failed to construct 'X'"` vs
/// `"Failed to execute 'm' on 'X'"`).
pub(crate) struct SeqMessages<'a> {
    /// Thrown when the value cannot become a sequence — either it is
    /// not an Object (WebIDL §3.2.21 step 1, e.g. a **string primitive**,
    /// which is rejected outright rather than iterated per code point) or
    /// it has no `@@iterator` at all (step 3 → ES `GetIterator` step 2 —
    /// numbers, booleans, plain objects without the symbol).
    pub not_iterable: &'a str,
    /// Thrown when `@@iterator` resolves to a callable that returns a
    /// non-Object value (ES `GetIterator` step 5).
    pub iter_not_object: &'a str,
    /// Thrown when the iterator yields more than `cap` items.  Custom
    /// iterables' `.return()` cleanup runs first per §7.4.11.
    pub cap_exceeded: &'a str,
}

/// Convert `raw` to an IDL `sequence<T>` per WebIDL §3.2.21,
/// validating each element via `validator` and collecting the results.
///
/// A non-Object `raw` (notably a string primitive) is rejected up front
/// per step 1, before `@@iterator` is consulted — it is **not** iterated
/// per code point.
///
/// `validator` receives `(ctx, index, value)` so per-element errors can
/// reference the failing index. A validator throw triggers
/// `IteratorClose` on the iterator before propagation; a `.return()`
/// throw takes precedence per §7.4.11 step 6-7.
///
/// **GC invariant.** `validator` must not trigger GC — each `JsValue`
/// yielded by `iter_next` is held only as a Rust local until the
/// validator returns. Brand-check + coercion to `Entity` / `ObjectId`
/// / string IDs all satisfy this.
pub(crate) fn webidl_sequence_to_vec<T, F>(
    ctx: &mut NativeContext<'_>,
    raw: JsValue,
    cap: usize,
    msgs: &SeqMessages<'_>,
    validator: F,
) -> Result<Vec<T>, VmError>
where
    F: FnMut(&mut NativeContext<'_>, usize, JsValue) -> Result<T, VmError>,
{
    // WebIDL §3.2.21 step 1: a non-Object value is a TypeError *before*
    // `@@iterator` is looked up (step 2).  Without this guard,
    // `resolve_iterator` would resolve `String.prototype[@@iterator]` for a
    // string primitive and walk its code points — which the spec forbids
    // for `sequence<T>` conversion (a string is not an Array/iterable
    // sequence source).
    if !matches!(raw, JsValue::Object(_)) {
        return Err(VmError::type_error(msgs.not_iterable.to_owned()));
    }
    let iter = match ctx.vm.resolve_iterator(raw)? {
        Some(iter @ JsValue::Object(_)) => iter,
        Some(_) => return Err(VmError::type_error(msgs.iter_not_object.to_owned())),
        None => return Err(VmError::type_error(msgs.not_iterable.to_owned())),
    };
    drain_iter_to_vec(ctx, iter, cap, msgs.cap_exceeded, validator)
}

/// Variant of [`webidl_sequence_to_vec`] that accepts a pre-resolved
/// iterator object.  Use when the caller has already invoked
/// `@@iterator` (e.g. for union resolution that needs to inspect
/// whether the value is iterable before committing to the sequence
/// branch).  `iter` must be `JsValue::Object(_)` — the caller is
/// responsible for the "iterator must return an object" check on its
/// side because callers vary in error wording at that boundary.
pub(crate) fn webidl_iter_to_vec<T, F>(
    ctx: &mut NativeContext<'_>,
    iter: JsValue,
    cap: usize,
    cap_exceeded_msg: &str,
    validator: F,
) -> Result<Vec<T>, VmError>
where
    F: FnMut(&mut NativeContext<'_>, usize, JsValue) -> Result<T, VmError>,
{
    debug_assert!(
        matches!(iter, JsValue::Object(_)),
        "webidl_iter_to_vec requires a resolved iterator object",
    );
    drain_iter_to_vec(ctx, iter, cap, cap_exceeded_msg, validator)
}

fn drain_iter_to_vec<T, F>(
    ctx: &mut NativeContext<'_>,
    iter: JsValue,
    cap: usize,
    cap_exceeded_msg: &str,
    mut validator: F,
) -> Result<Vec<T>, VmError>
where
    F: FnMut(&mut NativeContext<'_>, usize, JsValue) -> Result<T, VmError>,
{
    let mut out: Vec<T> = Vec::new();
    let mut index = 0usize;
    loop {
        // `iter_next`'s own throw — per ECMA-262 §7.4.9 / §7.4.10 (IteratorStep /
        // IteratorStepValue), the spec sets `iteratorRecord.[[Done]] = true` and
        // propagates the completion WITHOUT invoking `IteratorClose` / `.return()`.
        let Some(elem) = ctx.vm.iter_next(iter)? else {
            return Ok(out);
        };
        if index >= cap {
            // Cap exceeded is an abrupt completion from OUR loop body
            // (not from `iter_next` itself), so `IteratorClose` runs.
            // A `.return()` throw wins over the cap error.
            let close_err = ctx.vm.iter_close(iter).err();
            return Err(
                close_err.unwrap_or_else(|| VmError::type_error(cap_exceeded_msg.to_owned()))
            );
        }
        match validator(ctx, index, elem) {
            Ok(v) => out.push(v),
            Err(err) => {
                let close_err = ctx.vm.iter_close(iter).err();
                return Err(close_err.unwrap_or(err));
            }
        }
        index += 1;
    }
}
