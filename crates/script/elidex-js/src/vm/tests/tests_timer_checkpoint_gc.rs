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
//! after the callback returned, i.e. inside the checkpoint.

use std::time::{Duration, Instant};

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
    let t0 = Instant::now();
    vm.inner.drain_timers(t0 + Duration::from_millis(10));
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
    vm.inner.drain_timers(t0 + Duration::from_millis(25));
    assert_eq!(
        vm.eval("seen.join(',') === 'x,x'").unwrap(),
        JsValue::Boolean(true),
        "second fire must run the original callback with the original arg"
    );
}
