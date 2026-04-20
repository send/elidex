//! [`Vm::new`] — VM construction and built-in registration.
//!
//! Extracted from `vm/mod.rs` to keep that file under the project's
//! 1000-line convention.  Construction logic is self-contained
//! (touches every `VmInner` field once and then hands off to
//! `register_globals`), so isolating it here keeps `mod.rs` focused
//! on the type definitions.

use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};

use super::pools::{BigIntPool, StringPool};
use super::shape;
use super::value::{JsValue, ObjectId};
use super::well_known::{WellKnownStrings, WellKnownSymbols};
use super::{Vm, VmInner};

#[cfg(feature = "engine")]
use super::host;

impl Vm {
    /// Create a new VM with built-in globals registered.
    #[allow(clippy::too_many_lines)]
    pub fn new() -> Self {
        let mut strings = StringPool::new();

        let well_known = WellKnownStrings::intern_all(&mut strings);
        let (well_known_symbols, symbols) = WellKnownSymbols::alloc_all(&mut strings);

        let mut vm = Vm {
            inner: VmInner {
                stack: Vec::with_capacity(256),
                frames: Vec::with_capacity(16),
                strings,
                bigints: BigIntPool::new(),
                objects: Vec::new(),
                free_objects: Vec::new(),
                compiled_functions: Vec::new(),
                upvalues: Vec::new(),
                free_upvalues: Vec::new(),
                globals: HashMap::new(),
                symbols,
                symbol_registry: HashMap::new(),
                symbol_reverse_registry: HashMap::new(),
                well_known,
                well_known_symbols,
                string_prototype: None,
                symbol_prototype: None,
                object_prototype: None,
                array_prototype: None,
                number_prototype: None,
                boolean_prototype: None,
                bigint_prototype: None,
                function_prototype: None,
                regexp_prototype: None,
                array_iterator_prototype: None,
                string_iterator_prototype: None,
                // Placeholder — immediately replaced by register_globals().
                global_object: ObjectId(0),
                completion_value: JsValue::Undefined,
                current_exception: JsValue::Undefined,
                rng_state: {
                    // Seed from OS-RNG via RandomState so each Vm gets a
                    // unique sequence without requiring `rand`.
                    use std::collections::hash_map::RandomState;
                    use std::hash::{BuildHasher, Hasher};
                    let mut hasher = RandomState::new().build_hasher();
                    hasher.write_u64(0);
                    let seed = hasher.finish();
                    // Ensure non-zero (xorshift64 fixpoint).
                    if seed == 0 {
                        1
                    } else {
                        seed
                    }
                },
                shapes: vec![shape::Shape::root()],
                gc_object_marks: Vec::new(),
                gc_upvalue_marks: Vec::new(),
                gc_work_list: Vec::new(),
                gc_bytes_since_last: 0,
                gc_threshold: 65536,
                gc_enabled: false,
                in_construct: false,
                host_data: None,
                promise_prototype: None,
                microtask_queue: VecDeque::new(),
                microtask_drain_depth: 0,
                pending_rejections: Vec::new(),
                error_prototype: None,
                aggregate_error_prototype: None,
                generator_prototype: None,
                event_target_prototype: None,
                node_prototype: None,
                element_prototype: None,
                #[cfg(feature = "engine")]
                character_data_prototype: None,
                #[cfg(feature = "engine")]
                text_prototype: None,
                #[cfg(feature = "engine")]
                document_type_prototype: None,
                #[cfg(feature = "engine")]
                html_iframe_prototype: None,
                #[cfg(feature = "engine")]
                dom_exception_prototype: None,
                #[cfg(feature = "engine")]
                dom_exception_states: HashMap::new(),
                window_prototype: None,
                event_prototype: None,
                #[cfg(feature = "engine")]
                custom_event_prototype: None,
                #[cfg(feature = "engine")]
                ui_event_prototype: None,
                #[cfg(feature = "engine")]
                mouse_event_prototype: None,
                #[cfg(feature = "engine")]
                keyboard_event_prototype: None,
                #[cfg(feature = "engine")]
                focus_event_prototype: None,
                #[cfg(feature = "engine")]
                input_event_prototype: None,
                #[cfg(feature = "engine")]
                promise_rejection_event_prototype: None,
                #[cfg(feature = "engine")]
                error_event_prototype: None,
                #[cfg(feature = "engine")]
                hash_change_event_prototype: None,
                #[cfg(feature = "engine")]
                pop_state_event_prototype: None,
                #[cfg(feature = "engine")]
                abort_signal_prototype: None,
                #[cfg(feature = "engine")]
                abort_signal_states: HashMap::new(),
                #[cfg(feature = "engine")]
                abort_listener_back_refs: HashMap::new(),
                #[cfg(feature = "engine")]
                pending_timeout_signals: HashMap::new(),
                #[cfg(feature = "engine")]
                precomputed_event_shapes: None,
                generator_yielded: None,
                current_microtask: None,
                timer_queue: BinaryHeap::new(),
                current_timer: None,
                next_timer_id: 1,
                active_timer_ids: HashSet::new(),
                cancelled_timers: HashSet::new(),
                #[cfg(feature = "engine")]
                start_instant: std::time::Instant::now(),
                #[cfg(feature = "engine")]
                navigation: host::navigation::NavigationState::new(),
                #[cfg(feature = "engine")]
                viewport: host::window::ViewportState::new(),
            },
        };

        vm.inner.register_globals();
        vm.inner.gc_enabled = true;
        vm
    }
}
