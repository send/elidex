//! Promise combinators (ES2020 §25.6.4.1-3) + `prototype.finally`.
//!
//! Split out of `natives_promise` to keep each file under the 1000-line
//! project convention.  The core state machine (Promise / settle / then /
//! drain / queueMicrotask) stays in `natives_promise`; this file owns
//! the aggregator-style combinators plus `finally`, which share the
//! ObjectKind-variant-based closure pattern documented in the plan.

use super::natives_promise::{create_promise, create_resolver_pair, settle_promise, then_impl};
use super::shape::{self, PropertyAttrs};
use super::value::{
    CombinatorKind, JsValue, NativeContext, Object, ObjectId, ObjectKind, PromiseCombinatorState,
    PromiseCombinatorStep, PropertyKey, PropertyStorage, PropertyValue, VmError, VmErrorKind,
};
use super::VmInner;

// ---------------------------------------------------------------------------
// Combinators: Promise.all / allSettled / race / any + prototype.finally
// ---------------------------------------------------------------------------

/// Allocate a fresh `PromiseCombinatorState` object.  Pre-fills `values`
/// with `Undefined` placeholders so each step can write its own slot
/// without further resizing.
fn alloc_combinator_state(
    vm: &mut VmInner,
    kind: CombinatorKind,
    result: ObjectId,
    total: u32,
) -> ObjectId {
    let placeholder = vec![JsValue::Undefined; total as usize];
    vm.alloc_object(Object {
        kind: ObjectKind::PromiseCombinatorState(PromiseCombinatorState {
            kind,
            result,
            values: placeholder,
            remaining: total,
            total,
            settled: false,
        }),
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: None,
        extensible: false,
    })
}

/// Allocate a step object as a standalone callable.
fn alloc_step(vm: &mut VmInner, step: PromiseCombinatorStep) -> ObjectId {
    let proto = vm.function_prototype;
    vm.alloc_object(Object {
        kind: ObjectKind::PromiseCombinatorStep(step),
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    })
}

/// Invoke a combinator step on `value`.  Mutates the shared state, and
/// settles the result promise once the last step has run.
pub(super) fn step_combinator(
    vm: &mut VmInner,
    step: PromiseCombinatorStep,
    value: JsValue,
) -> Result<JsValue, VmError> {
    use PromiseCombinatorStep as Step;

    let state_id = match step {
        Step::AllFulfill { state, .. }
        | Step::AllReject { state }
        | Step::AllSettledFulfill { state, .. }
        | Step::AllSettledReject { state, .. }
        | Step::AnyFulfill { state }
        | Step::AnyReject { state, .. } => state,
    };

    match step {
        Step::AllFulfill { index, .. } => {
            let (result, finished, values) = {
                let ObjectKind::PromiseCombinatorState(state) =
                    &mut vm.get_object_mut(state_id).kind
                else {
                    return Ok(JsValue::Undefined);
                };
                if state.settled {
                    return Ok(JsValue::Undefined);
                }
                state.values[index as usize] = value;
                state.remaining -= 1;
                if state.remaining == 0 {
                    state.settled = true;
                    (state.result, true, std::mem::take(&mut state.values))
                } else {
                    (state.result, false, Vec::new())
                }
            };
            if finished {
                let arr = vm.create_array_object(values);
                let _ = settle_promise(vm, result, false, JsValue::Object(arr));
            }
        }
        Step::AllReject { .. } => {
            let result = {
                let ObjectKind::PromiseCombinatorState(state) =
                    &mut vm.get_object_mut(state_id).kind
                else {
                    return Ok(JsValue::Undefined);
                };
                if state.settled {
                    return Ok(JsValue::Undefined);
                }
                state.settled = true;
                state.result
            };
            let _ = settle_promise(vm, result, true, value);
        }
        Step::AllSettledFulfill { index, .. } => {
            let entry = make_settled_entry(vm, true, value);
            settle_all_settled_slot(vm, state_id, index, entry);
        }
        Step::AllSettledReject { index, .. } => {
            let entry = make_settled_entry(vm, false, value);
            settle_all_settled_slot(vm, state_id, index, entry);
        }
        Step::AnyFulfill { .. } => {
            let result = {
                let ObjectKind::PromiseCombinatorState(state) =
                    &mut vm.get_object_mut(state_id).kind
                else {
                    return Ok(JsValue::Undefined);
                };
                if state.settled {
                    return Ok(JsValue::Undefined);
                }
                state.settled = true;
                state.result
            };
            let _ = settle_promise(vm, result, false, value);
        }
        Step::AnyReject { index, .. } => {
            let (result, finished, errors) = {
                let ObjectKind::PromiseCombinatorState(state) =
                    &mut vm.get_object_mut(state_id).kind
                else {
                    return Ok(JsValue::Undefined);
                };
                if state.settled {
                    return Ok(JsValue::Undefined);
                }
                state.values[index as usize] = value;
                state.remaining -= 1;
                if state.remaining == 0 {
                    state.settled = true;
                    (state.result, true, std::mem::take(&mut state.values))
                } else {
                    (state.result, false, Vec::new())
                }
            };
            if finished {
                let agg = build_aggregate_error(vm, errors);
                let _ = settle_promise(vm, result, true, agg);
            }
        }
    }
    Ok(JsValue::Undefined)
}

/// Build a `{status: ..., value|reason: ...}` result object used by
/// `Promise.allSettled`.  Uses an Ordinary object with Dictionary storage
/// to avoid allocating a dedicated shape for every entry.
fn make_settled_entry(vm: &mut VmInner, fulfilled: bool, value: JsValue) -> JsValue {
    let obj = vm.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: vm.object_prototype,
        extensible: true,
    });
    let status_key = PropertyKey::String(vm.strings.intern("status"));
    let status_str = if fulfilled { "fulfilled" } else { "rejected" };
    let status_val = JsValue::String(vm.strings.intern(status_str));
    vm.define_shaped_property(
        obj,
        status_key,
        PropertyValue::Data(status_val),
        PropertyAttrs::DATA,
    );
    let payload_name = if fulfilled { "value" } else { "reason" };
    let payload_key = PropertyKey::String(vm.strings.intern(payload_name));
    vm.define_shaped_property(
        obj,
        payload_key,
        PropertyValue::Data(value),
        PropertyAttrs::DATA,
    );
    JsValue::Object(obj)
}

/// Shared tail for `AllSettledFulfill` / `AllSettledReject`: write the
/// `{status,value|reason}` entry at `index`, dec the counter, and resolve
/// when every slot has arrived.
fn settle_all_settled_slot(vm: &mut VmInner, state_id: ObjectId, index: u32, entry: JsValue) {
    let (result, finished, values) = {
        let ObjectKind::PromiseCombinatorState(state) = &mut vm.get_object_mut(state_id).kind
        else {
            return;
        };
        if state.settled {
            return;
        }
        state.values[index as usize] = entry;
        state.remaining -= 1;
        if state.remaining == 0 {
            state.settled = true;
            (state.result, true, std::mem::take(&mut state.values))
        } else {
            (state.result, false, Vec::new())
        }
    };
    if finished {
        let arr = vm.create_array_object(values);
        let _ = settle_promise(vm, result, false, JsValue::Object(arr));
    }
}

/// Build an `AggregateError` for `Promise.any` when every input rejects.
/// The shape here is a minimal `Error` object carrying `.errors` and a
/// fixed message — full AggregateError wiring (inheritance chain, proper
/// `[[Prototype]]`) comes with the rest of the Error cleanup in PR4.
fn build_aggregate_error(vm: &mut VmInner, errors: Vec<JsValue>) -> JsValue {
    let name_id = vm.strings.intern("AggregateError");
    let obj = vm.alloc_object(Object {
        kind: ObjectKind::Error { name: name_id },
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: vm.object_prototype,
        extensible: true,
    });
    let name_key = PropertyKey::String(vm.well_known.name);
    vm.define_shaped_property(
        obj,
        name_key,
        PropertyValue::Data(JsValue::String(name_id)),
        PropertyAttrs::DATA,
    );
    let message_key = PropertyKey::String(vm.well_known.message);
    let message_val = JsValue::String(vm.strings.intern("All promises were rejected"));
    vm.define_shaped_property(
        obj,
        message_key,
        PropertyValue::Data(message_val),
        PropertyAttrs::DATA,
    );
    let errors_arr = vm.create_array_object(errors);
    let errors_key = PropertyKey::String(vm.strings.intern("errors"));
    vm.define_shaped_property(
        obj,
        errors_key,
        PropertyValue::Data(JsValue::Object(errors_arr)),
        PropertyAttrs::DATA,
    );
    JsValue::Object(obj)
}

/// Shared body for `Promise.all` / `allSettled` / `any` / `race`.  Reads
/// the iterable, allocates a result promise + (optional) aggregator state,
/// and subscribes per-item reactions via `.then(...)`.  `race` passes
/// `None` for `kind` and uses outer resolve/reject directly.
fn run_combinator(
    ctx: &mut NativeContext<'_>,
    args: &[JsValue],
    kind: Option<CombinatorKind>,
) -> Result<JsValue, VmError> {
    let iterable = args.first().copied().unwrap_or(JsValue::Undefined);

    let iterator = match ctx.vm.resolve_iterator(iterable)? {
        Some(JsValue::Object(id)) => JsValue::Object(id),
        Some(_) => return Err(VmError::type_error("@@iterator must return an object")),
        None => {
            return Err(VmError::type_error(
                "Promise.<combinator> input is not iterable",
            ))
        }
    };

    let result = create_promise(ctx.vm);
    let (resolve, reject) = create_resolver_pair(ctx.vm, result);

    // Collect all items into a buffer; step allocation needs to know
    // `total` up front to pre-size the state's values vec.  This also
    // matches the spec's eager `IteratorStep` loop — values are awaited
    // via `.then` attachment, not pulled lazily.
    let items = collect_items(ctx.vm, iterator)?;
    let total = u32::try_from(items.len())
        .map_err(|_| VmError::range_error("Promise combinator input exceeded u32 length limit"))?;

    // Empty iterable: spec-specific resolution.  For all/allSettled, resolve
    // immediately with []; for any, reject immediately with an empty
    // AggregateError; for race, stay Pending forever (resolve/reject never
    // called).  Returning eagerly also avoids allocating a no-op state.
    if total == 0 {
        match kind {
            Some(CombinatorKind::All | CombinatorKind::AllSettled) => {
                let empty = ctx.vm.create_array_object(Vec::new());
                let _ = settle_promise(ctx.vm, result, false, JsValue::Object(empty));
            }
            Some(CombinatorKind::Any) => {
                let agg = build_aggregate_error(ctx.vm, Vec::new());
                let _ = settle_promise(ctx.vm, result, true, agg);
            }
            None => {} // race: stays pending
        }
        return Ok(JsValue::Object(result));
    }

    match kind {
        None => {
            // race: attach outer resolve/reject to every input.
            for item in items {
                subscribe(ctx, item, resolve, reject)?;
            }
        }
        Some(k) => {
            let state = alloc_combinator_state(ctx.vm, k, result, total);
            // Pre-allocate the shared reject step for `all` so every item
            // shares the same `AllReject` callable (spec doesn't mandate
            // identity but saving allocations makes per-iteration cheaper).
            let shared_all_reject = if k == CombinatorKind::All {
                Some(alloc_step(
                    ctx.vm,
                    PromiseCombinatorStep::AllReject { state },
                ))
            } else {
                None
            };
            let shared_any_fulfill = if k == CombinatorKind::Any {
                Some(alloc_step(
                    ctx.vm,
                    PromiseCombinatorStep::AnyFulfill { state },
                ))
            } else {
                None
            };

            for (i, item) in items.into_iter().enumerate() {
                let idx = u32::try_from(i).expect("items length already bounded by u32");
                let (on_fulfilled_id, on_rejected_id) = match k {
                    CombinatorKind::All => (
                        alloc_step(
                            ctx.vm,
                            PromiseCombinatorStep::AllFulfill { state, index: idx },
                        ),
                        shared_all_reject.expect("AllReject step allocated above for All kind"),
                    ),
                    CombinatorKind::AllSettled => (
                        alloc_step(
                            ctx.vm,
                            PromiseCombinatorStep::AllSettledFulfill { state, index: idx },
                        ),
                        alloc_step(
                            ctx.vm,
                            PromiseCombinatorStep::AllSettledReject { state, index: idx },
                        ),
                    ),
                    CombinatorKind::Any => (
                        shared_any_fulfill.expect("AnyFulfill step allocated above for Any kind"),
                        alloc_step(
                            ctx.vm,
                            PromiseCombinatorStep::AnyReject { state, index: idx },
                        ),
                    ),
                };
                subscribe(ctx, item, on_fulfilled_id, on_rejected_id)?;
            }
        }
    }

    Ok(JsValue::Object(result))
}

/// Collect every value produced by `iterator` into a `Vec`.  Honours
/// IteratorClose on error: if iteration panics (JS throw), the error
/// propagates — `resolve_iterator` / `iter_next` already close via their
/// own error paths when the next step throws, so we don't need a manual
/// IteratorClose here.
fn collect_items(vm: &mut VmInner, iterator: JsValue) -> Result<Vec<JsValue>, VmError> {
    let mut out = Vec::new();
    while let Some(v) = vm.iter_next(iterator)? {
        out.push(v);
    }
    Ok(out)
}

/// `item.then(on_fulfilled, on_rejected)` after `Promise.resolve(item)`
/// normalisation.  Used by every combinator to wire per-item reactions
/// onto the outer state machine.
fn subscribe(
    ctx: &mut NativeContext<'_>,
    item: JsValue,
    on_fulfilled: ObjectId,
    on_rejected: ObjectId,
) -> Result<(), VmError> {
    // Normalise non-promise inputs via Promise.resolve.
    let promise_id = if let JsValue::Object(id) = item {
        if matches!(ctx.get_object(id).kind, ObjectKind::Promise(_)) {
            id
        } else {
            let p = create_promise(ctx.vm);
            let _ = settle_promise(ctx.vm, p, false, item);
            p
        }
    } else {
        let p = create_promise(ctx.vm);
        let _ = settle_promise(ctx.vm, p, false, item);
        p
    };
    then_impl(ctx.vm, promise_id, Some(on_fulfilled), Some(on_rejected))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// finally
// ---------------------------------------------------------------------------

/// Run the `finally` step: invoke `on_finally()`, then pass through the
/// original value (fulfill path) or re-throw the original reason (reject
/// path).  If `on_finally` itself throws, its error propagates as the
/// reaction result and the capability rejects with it — spec §25.6.5.3.1/2
/// semantics under the simplification that the `on_finally` return value
/// is not awaited (see PR2 plan "Test262 alignment").
pub(super) fn run_finally_step(
    vm: &mut VmInner,
    on_finally: ObjectId,
    is_reject: bool,
    value: JsValue,
) -> Result<JsValue, VmError> {
    vm.call(on_finally, JsValue::Undefined, &[])?;
    if is_reject {
        // Re-throw so the promise reaction rejects the derived capability
        // with the original reason.
        Err(VmError {
            kind: VmErrorKind::ThrowValue(value),
            message: String::new(),
        })
    } else {
        Ok(value)
    }
}

// ---------------------------------------------------------------------------
// Native entry points
// ---------------------------------------------------------------------------

/// `Promise.all(iterable)` — §25.6.4.1
pub(super) fn native_promise_all(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    run_combinator(ctx, args, Some(CombinatorKind::All))
}

/// `Promise.allSettled(iterable)` — §25.6.4.2
pub(super) fn native_promise_all_settled(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    run_combinator(ctx, args, Some(CombinatorKind::AllSettled))
}

/// `Promise.race(iterable)` — §25.6.4.5
pub(super) fn native_promise_race(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    run_combinator(ctx, args, None)
}

/// `Promise.any(iterable)` — §25.6.4.3
pub(super) fn native_promise_any(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    run_combinator(ctx, args, Some(CombinatorKind::Any))
}

/// `Promise.prototype.finally(onFinally)` — §25.6.5.3
pub(super) fn native_promise_prototype_finally(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let JsValue::Object(src) = this else {
        return Err(VmError::type_error(
            "Promise.prototype.finally called on non-object",
        ));
    };
    if !matches!(ctx.get_object(src).kind, ObjectKind::Promise(_)) {
        return Err(VmError::type_error(
            "Promise.prototype.finally called on non-Promise",
        ));
    }
    let on_finally = match args.first().copied() {
        Some(JsValue::Object(id)) if ctx.get_object(id).kind.is_callable() => Some(id),
        _ => None,
    };

    // Short-circuit: if onFinally isn't callable, finally is a pure
    // passthrough — `then(undefined, undefined)` already propagates in
    // then_impl.
    let Some(on_finally) = on_finally else {
        return then_impl(ctx.vm, src, None, None);
    };

    let proto = ctx.vm.function_prototype;
    let fulfill_step = ctx.vm.alloc_object(Object {
        kind: ObjectKind::PromiseFinallyStep {
            on_finally,
            is_reject: false,
        },
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });
    let reject_step = ctx.vm.alloc_object(Object {
        kind: ObjectKind::PromiseFinallyStep {
            on_finally,
            is_reject: true,
        },
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });
    then_impl(ctx.vm, src, Some(fulfill_step), Some(reject_step))
}
