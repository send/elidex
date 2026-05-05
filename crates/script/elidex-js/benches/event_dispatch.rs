//! Event-object construction throughput (PR3.6).
//!
//! Measures the per-dispatch cost of [`Vm::create_event_object`] for
//! the three dominant payload variants (Mouse, Keyboard, None).  The
//! whole point of PR3.6 is to collapse ~17 hashmap lookups + ~8 intern
//! calls per dispatched event down to one `define_with_precomputed_shape`
//! call — the numbers here expose that delta and let future PRs
//! (e.g. PR4's HostObject wrapper precomputed shapes) reuse the same
//! methodology.
//!
//! Run with:
//!
//! ```sh
//! cargo bench -p elidex-js --features engine --bench event_dispatch
//! ```
//!
//! Not included in `mise run bench` — that task runs the default
//! elidex-css/elidex-style/elidex-layout benches, none of which need
//! feature flags.  The elidex-js bench requires `--features engine`
//! (DispatchEvent / HostData / create_element_wrapper all live behind
//! that feature).

#![allow(unused_must_use)]

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

use elidex_ecs::{Attributes, EcsDom, Entity};
use elidex_js::vm::host_data::HostData;
use elidex_js::vm::test_helpers::make_event;
use elidex_js::vm::value::ObjectId;
use elidex_js::vm::Vm;
use elidex_plugin::{EventPayload, KeyboardEventInit, MouseEventInit};
use elidex_script_session::SessionCore;

/// Stand up a Vm bound to a fresh DOM with a single `<button>` that
/// we reuse as event target/currentTarget for every sample.
///
/// Returns `(vm, session, dom, target_wrapper_id, button_entity)`.
/// `session` and `dom` are heap-owned (Box) so the raw `*mut` passed
/// to `Vm::bind` stays live for the benchmark's duration.  The bench
/// keeps its own `setup()` rather than reusing `test_helpers::bind_vm`
/// because only the bench needs the stable-address `Box` pattern; the
/// stack-based test helpers (see `vm::test_helpers`) would force the
/// caller to keep matching `let`-bindings alive manually.
fn setup() -> (Box<Vm>, Box<SessionCore>, Box<EcsDom>, ObjectId, Entity) {
    let mut vm = Box::new(Vm::new());
    let mut session = Box::new(SessionCore::new());
    let mut dom = Box::new(EcsDom::new());
    let doc = dom.create_document_root();
    let el = dom.create_element("button", Attributes::default());

    vm.install_host_data(HostData::new());
    #[allow(unsafe_code)]
    unsafe {
        vm.bind(&raw mut *session, &raw mut *dom, doc);
    }
    let target = vm.create_element_wrapper(el);
    (vm, session, dom, target, el)
}

fn bench_mouse(c: &mut Criterion) {
    let (mut vm, _session, _dom, target, el) = setup();
    let ev = make_event(
        "click",
        true,
        EventPayload::Mouse(MouseEventInit {
            client_x: 42.0,
            client_y: 17.0,
            button: 0,
            buttons: 1,
            alt_key: false,
            ctrl_key: true,
            meta_key: false,
            shift_key: false,
        }),
        el,
    );

    let mut group = c.benchmark_group("event_dispatch/mouse");
    group.throughput(Throughput::Elements(1));
    group.bench_function(BenchmarkId::from_parameter("create_event_object"), |b| {
        b.iter(|| {
            let obj = vm.create_event_object(&ev, target, target, false);
            std::hint::black_box(obj);
        });
    });
    group.finish();

    vm.unbind();
}

fn bench_keyboard(c: &mut Criterion) {
    let (mut vm, _session, _dom, target, el) = setup();
    let ev = make_event(
        "keydown",
        true,
        EventPayload::Keyboard(KeyboardEventInit {
            key: "Enter".to_string(),
            code: "Enter".to_string(),
            alt_key: false,
            ctrl_key: false,
            meta_key: false,
            shift_key: false,
            repeat: false,
        }),
        el,
    );

    let mut group = c.benchmark_group("event_dispatch/keyboard");
    group.throughput(Throughput::Elements(1));
    group.bench_function(BenchmarkId::from_parameter("create_event_object"), |b| {
        b.iter(|| {
            let obj = vm.create_event_object(&ev, target, target, false);
            std::hint::black_box(obj);
        });
    });
    group.finish();

    vm.unbind();
}

fn bench_no_payload(c: &mut Criterion) {
    let (mut vm, _session, _dom, target, el) = setup();
    let ev = make_event("load", false, EventPayload::None, el);

    let mut group = c.benchmark_group("event_dispatch/none");
    group.throughput(Throughput::Elements(1));
    group.bench_function(BenchmarkId::from_parameter("create_event_object"), |b| {
        b.iter(|| {
            let obj = vm.create_event_object(&ev, target, target, false);
            std::hint::black_box(obj);
        });
    });
    group.finish();

    vm.unbind();
}

criterion_group!(benches, bench_mouse, bench_keyboard, bench_no_payload);
criterion_main!(benches);
