//! Live `HTMLCollection` accessor throughput (PR-spec-polish SP2).
//!
//! Measures the per-access cost of the three hot paths exercised when
//! JavaScript reads a live HTMLCollection wrapper retained across
//! many accesses:
//!
//! 1. `length` — repeated reads of `coll.length` over a populated tree.
//! 2. `iter`   — `for (const e of coll) ...` over the same tree.
//! 3. `item`   — repeated `coll.item(N/2)` calls.
//!
//! Today each access re-resolves the entire entity list via
//! `resolve_entities_for` (see
//! `vm/host/dom_collection.rs::resolve_entities_for`) — i.e. one fresh
//! `EcsDom::traverse_descendants()` walk per read.  The numbers here
//! are the input to the SP2-impl decision gate:
//!
//! - `≥ 50µs/op` on a 1000-node tree → caching is a clear win, proceed
//!   to SP2-impl + SP7.
//! - `< 5µs/op` even on a larger tree → defer indefinitely, close as
//!   no-op.
//!
//! The `after_mutation` worst-case (alternating `appendChild` + `length`
//! reads) is intentionally NOT included — methodology around tree
//! growth across criterion's sample iterations is brittle (the tree
//! either grows unboundedly across samples or requires `iter_custom`
//! gymnastics that obscure the per-op cost).  If SP2-impl lands, that
//! scenario is better expressed as a unit-style timing test: it is
//! definitionally the case where caching cannot help (every read
//! straddles a version bump), so its only role is as an upper-bound
//! reference, not as a decision input.
//!
//! Run with:
//!
//! ```sh
//! cargo bench -p elidex-js --features engine --bench dom_collection
//! ```
//!
//! Not included in `mise run bench` — the elidex-js benches require
//! `--features engine` (HostData / wrapper construction live behind
//! that feature).

#![cfg(feature = "engine")]
#![allow(unused_must_use)]

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

use elidex_ecs::{Attributes, EcsDom};
use elidex_js::vm::host_data::HostData;
use elidex_js::vm::value::{JsValue, ObjectId};
use elidex_js::vm::Vm;
use elidex_script_session::SessionCore;

/// Tree size used for every bench — chosen to be representative of a
/// "moderately populated" page region (e.g. a list of cards or a long
/// table row block) without inflating bench wall-time so far that
/// criterion's sampling becomes flaky.
const TREE_SIZE: usize = 1000;

/// Inner-loop iteration counts for each bench.  Throughput is reported
/// as `Elements(LOOP_…)` so the per-element time is the per-access
/// cost of the underlying `resolve_entities_for` walk plus JS dispatch
/// overhead.  Each criterion iteration calls a pre-resolved JS function
/// object via [`Vm::call`] (see `setup_globals`) — no per-iter parse
/// or microtask drain — so the only constant overhead is one JS call
/// dispatch per criterion iter, far below the inner-loop cost for any
/// meaningful tree size.
///
/// `length` and `item` are O(1) JS-side per access and amortise the
/// per-element cost easily at 10k iters.  `iter` allocates a fresh
/// Array snapshot of every matching element on every access (see
/// `collection_iterator_impl`), so 10k iters × ~800 elements pushes
/// criterion's default sample budget over the SIGKILL threshold —
/// 100 iters keeps the wall-time per sample comparable to the other
/// benches while still giving criterion enough samples for stable
/// statistics.
const LOOP_LENGTH: u64 = 10_000;
const LOOP_ITER: u64 = 100;
const LOOP_ITEM: u64 = 10_000;

/// Set up a Vm bound to a fresh DOM populated with a `<body>` of
/// `tree_size` element children — alternating tags so
/// `getElementsByTagName('div')` filters out a portion (matches
/// real-world tag-mix ratios where filtered collections are a strict
/// subset of the parent's element children).
///
/// Heap-owned (`Box`) like `event_dispatch::setup` so the raw `*mut`
/// passed to `Vm::bind` stays live across the bench's lifetime.
fn setup(tree_size: usize) -> (Box<Vm>, Box<SessionCore>, Box<EcsDom>) {
    let mut vm = Box::new(Vm::new());
    let mut session = Box::new(SessionCore::new());
    let mut dom = Box::new(EcsDom::new());
    let doc = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    let body = dom.create_element("body", Attributes::default());
    dom.append_child(doc, html);
    dom.append_child(html, body);

    // 4:1 div:span mix — divs become the `getElementsByTagName('div')`
    // result, spans are the "noise" that exercises the per-element
    // tag filter.  `class="x"` is shared so `getElementsByClassName('x')`
    // hits every node — useful for SP2-impl's ByClass invalidation
    // story, kept here so the same setup doubles for follow-up benches.
    for i in 0..tree_size {
        let tag = if i.is_multiple_of(5) { "span" } else { "div" };
        let mut attrs = Attributes::default();
        attrs.set("class", "x".to_string());
        let el = dom.create_element(tag, attrs);
        dom.append_child(body, el);
    }

    vm.install_host_data(HostData::new());
    #[allow(unsafe_code)]
    unsafe {
        vm.bind(&mut *session as *mut _, &mut *dom as *mut _, doc);
    }
    (vm, session, dom)
}

/// Install the live HTMLCollection on `globalThis.__coll` and define
/// the per-bench JS hot loops once, so each criterion iteration only
/// pays a single [`Vm::call`] dispatch instead of re-parsing source or
/// running `eval`'s microtask drain on every sample.  `__mid` is the
/// item-bench index (middle of the div-only result).
fn setup_globals(vm: &mut Vm, tree_size: usize) {
    let div_count = tree_size - tree_size.div_ceil(5); // matches the i%5==0 pattern in `setup`
    let mid = div_count / 2;
    // Functions are assigned to `globalThis` rather than declared via
    // `function __fn() {}` because each `vm.eval` call runs in its own
    // script scope — top-level `function` bindings don't leak onto
    // globalThis the way they would in a real `<script>` element, but
    // assignments to `globalThis.X` persist across evals.
    let bootstrap = format!(
        "globalThis.__coll = document.getElementsByTagName('div');
         globalThis.__mid  = {mid};
         globalThis.__lenLoop  = function() {{ var n = 0; for (var i = 0; i < {LEN}; i++) {{ n += __coll.length; }} return n; }};
         globalThis.__iterLoop = function() {{ var n = 0; for (var i = 0; i < {ITER}; i++) {{ for (var e of __coll) n++; }} return n; }};
         globalThis.__itemLoop = function() {{ var n = 0; for (var i = 0; i < {ITEM}; i++) {{ if (__coll.item(__mid)) n++; }} return n; }};",
        LEN = LOOP_LENGTH,
        ITER = LOOP_ITER,
        ITEM = LOOP_ITEM,
    );
    vm.eval(&bootstrap)
        .expect("bench bootstrap script must compile and run");
}

/// Resolve a function pre-installed on `globalThis` by name to a raw
/// `ObjectId` callable via [`Vm::call`].  Panics if the global is
/// missing or non-Object — both are bench-wiring bugs.
fn resolve_global_fn(vm: &Vm, name: &str) -> ObjectId {
    match vm.get_global(name) {
        Some(JsValue::Object(id)) => id,
        other => panic!("bench: globalThis.{name} must be a function, got {other:?}"),
    }
}

fn bench_length(c: &mut Criterion) {
    let (mut vm, _session, _dom) = setup(TREE_SIZE);
    setup_globals(&mut vm, TREE_SIZE);
    let fn_id = resolve_global_fn(&vm, "__lenLoop");

    let mut group = c.benchmark_group("dom_collection/length");
    group.throughput(Throughput::Elements(LOOP_LENGTH));
    group.bench_function(
        BenchmarkId::from_parameter(format!("tree_{TREE_SIZE}_loop_{LOOP_LENGTH}")),
        |b| {
            b.iter(|| {
                std::hint::black_box(vm.call(fn_id, JsValue::Undefined, &[]).unwrap());
            });
        },
    );
    group.finish();

    vm.unbind();
}

fn bench_iter(c: &mut Criterion) {
    let (mut vm, _session, _dom) = setup(TREE_SIZE);
    setup_globals(&mut vm, TREE_SIZE);
    let fn_id = resolve_global_fn(&vm, "__iterLoop");

    // Iter throughput is `Elements(LOOP_ITER)` so per-element time
    // is the per-iter-construction cost (each iteration of the outer
    // loop builds a fresh Array snapshot + iterator wrapper — see
    // `collection_iterator_impl`).  The inner `for/of` walk over the
    // captured snapshot itself is cheap and not the SP2 target.
    let mut group = c.benchmark_group("dom_collection/iter");
    group.throughput(Throughput::Elements(LOOP_ITER));
    group.bench_function(
        BenchmarkId::from_parameter(format!("tree_{TREE_SIZE}_loop_{LOOP_ITER}")),
        |b| {
            b.iter(|| {
                std::hint::black_box(vm.call(fn_id, JsValue::Undefined, &[]).unwrap());
            });
        },
    );
    group.finish();

    vm.unbind();
}

fn bench_item(c: &mut Criterion) {
    let (mut vm, _session, _dom) = setup(TREE_SIZE);
    setup_globals(&mut vm, TREE_SIZE);
    let fn_id = resolve_global_fn(&vm, "__itemLoop");

    let mut group = c.benchmark_group("dom_collection/item");
    group.throughput(Throughput::Elements(LOOP_ITEM));
    group.bench_function(
        BenchmarkId::from_parameter(format!("tree_{TREE_SIZE}_loop_{LOOP_ITEM}")),
        |b| {
            b.iter(|| {
                std::hint::black_box(vm.call(fn_id, JsValue::Undefined, &[]).unwrap());
            });
        },
    );
    group.finish();

    vm.unbind();
}

criterion_group!(benches, bench_length, bench_iter, bench_item);
criterion_main!(benches);
