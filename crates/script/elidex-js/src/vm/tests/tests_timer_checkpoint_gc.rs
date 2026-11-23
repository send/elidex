//! GC-window pin for an interval's callback / args across the per-fire
//! checkpoint (`#11-vm-timer-interval-callback-checkpoint-rooting`).
//!
//! `drain_timers` pops the entry, fires it through the GC-rooted
//! `current_timer` slot, then runs the per-fire checkpoint (microtasks,
//! and with `engine` tasks / CE reactions) before re-arming an interval
//! from the entry's `callback` / `args`.  If the entry is held only by a
//! Rust local during that checkpoint, a collection there frees a callback
//! (and args) reachable only through the timer, and the re-arm pushes a
//! dead `ObjectId`.
//!
//! Placement: the interval's callback and its extra arg are created inline
//! in the `setInterval` call, so only the timer holds them.  The first fire
//! queues a microtask whose body allocates; `force_gc_before_next_alloc`
//! (set after setup) places one collection at the first `alloc_object`
//! after the callback returned, i.e. inside the checkpoint.  That placement
//! assumes the callback body (`seen.push`, `queueMicrotask`) allocates no
//! object; if it ever does, the collection moves into the callback (where
//! the entry was always rooted) and this cell no longer discriminates —
//! re-run it with `drain_timers`' `current_timer.take()` moved back before
//! the checkpoint and confirm it goes red.  The checkpoint is one unit for
//! this cell: the entry leaves the slot only after the whole checkpoint, so
//! the microtask leg stands for the task / CE-reaction legs.
//!
//! The drains run at the entry's own deadline (read from the queue), so the
//! re-armed deadline (`+ 10 ms`) is never due in the first drain.

use std::time::Duration;

use super::Vm;
use crate::vm::value::JsValue;

#[test]
fn interval_callback_and_args_survive_per_fire_checkpoint_gc() {
    let mut vm = Vm::new();
    vm.eval(
        "globalThis.seen = []; \
         globalThis.mfn = function () { return {}; }; \
         setInterval(function (a) { \
             globalThis.seen.push(a.tag); \
             queueMicrotask(globalThis.mfn); \
         }, 10, { tag: 'x' });",
    )
    .unwrap();
    vm.inner.force_gc_before_next_alloc = true;
    let first = vm
        .inner
        .timer_queue
        .peek()
        .map(|entry| entry.deadline)
        .expect("setInterval queued an entry");
    vm.inner.drain_timers(first);
    assert!(
        !vm.inner.force_gc_before_next_alloc,
        "no collection was placed — the checkpoint never allocated"
    );
    assert_eq!(
        vm.eval("seen.length").unwrap(),
        JsValue::Number(1.0),
        "first fire"
    );
    // Second drain: the re-armed entry must still hold the live callback+arg.
    vm.inner.drain_timers(first + Duration::from_millis(10));
    assert_eq!(
        vm.eval("seen.join(',') === 'x,x'").unwrap(),
        JsValue::Boolean(true),
        "second fire must run the original callback with the original arg"
    );
}
