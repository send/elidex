//! `window.visualViewport` / `VisualViewport` interface tests (CSSOM-View
//! §12.1) — S5-2 minor-window-parity.

#![cfg(feature = "engine")]

use elidex_css::media::{ColorScheme, ReducedMotion};
use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::host_data::HostData;
use super::super::value::JsValue;
use super::super::Vm;

/// A `Vm` with an (unbound) `HostData` installed so the inherited
/// `EventTarget.prototype.addEventListener` has a `listener_store` to write
/// into (the `MediaQueryList` test precedent).
fn new_vm_with_host() -> Vm {
    let mut v = Vm::new();
    v.install_host_data(super::super::host_data::HostData::new());
    v
}

/// `deliver_visual_viewport_events` no-ops while unbound (its `is_bound` guard,
/// the `deliver_media_query_changes` precedent), so the producer tests need a
/// real binding. `session` / `dom` live on this frame for the whole `body`,
/// keeping the bound raw pointers valid.
fn with_bound_vm(body: impl FnOnce(&mut Vm)) {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    vm.install_host_data(HostData::new());
    #[allow(unsafe_code)]
    unsafe {
        vm.bind(&raw mut session, &raw mut dom, doc);
    }
    body(&mut vm);
    vm.unbind();
}

fn eval_number(vm: &mut Vm, source: &str) -> f64 {
    match vm.eval(source).unwrap() {
        JsValue::Number(n) => n,
        other => panic!("expected number, got {other:?}"),
    }
}

fn eval_bool(vm: &mut Vm, source: &str) -> bool {
    match vm.eval(source).unwrap() {
        JsValue::Boolean(b) => b,
        other => panic!("expected bool, got {other:?}"),
    }
}

// --- presence + identity ---------------------------------------------------

#[test]
fn visual_viewport_is_an_object() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "typeof visualViewport === 'object' && visualViewport !== null"
    ));
}

#[test]
fn visual_viewport_is_same_object() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "window.visualViewport === window.visualViewport"
    ));
    assert!(eval_bool(
        &mut vm,
        "visualViewport === window.visualViewport"
    ));
}

#[test]
fn visual_viewport_is_visual_viewport_instance() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "visualViewport instanceof VisualViewport"
    ));
    // The EventTarget surface is inherited (the VM exposes no `EventTarget`
    // global constructor, so test the inherited method rather than `instanceof
    // EventTarget`): `addEventListener` resolves up the prototype chain.
    assert!(eval_bool(
        &mut vm,
        "typeof Object.getPrototypeOf(VisualViewport.prototype).addEventListener === 'function'"
    ));
}

#[test]
fn visual_viewport_constructor_is_illegal() {
    // WebIDL: no constructor → `new VisualViewport()` / `VisualViewport()` throw.
    super::assert_illegal_constructor("VisualViewport");
}

// --- geometry --------------------------------------------------------------

#[test]
fn geometry_defaults() {
    let mut vm = Vm::new();
    assert!((eval_number(&mut vm, "visualViewport.width") - 1024.0).abs() < f64::EPSILON);
    assert!((eval_number(&mut vm, "visualViewport.height") - 768.0).abs() < f64::EPSILON);
    assert!((eval_number(&mut vm, "visualViewport.offsetLeft")).abs() < f64::EPSILON);
    assert!((eval_number(&mut vm, "visualViewport.offsetTop")).abs() < f64::EPSILON);
    assert!((eval_number(&mut vm, "visualViewport.scale") - 1.0).abs() < f64::EPSILON);
    assert!((eval_number(&mut vm, "visualViewport.pageLeft")).abs() < f64::EPSILON);
    assert!((eval_number(&mut vm, "visualViewport.pageTop")).abs() < f64::EPSILON);
}

#[test]
fn width_height_track_transported_viewport() {
    let mut vm = Vm::new();
    vm.set_media_environment(
        1280.0,
        720.0,
        1.0,
        ColorScheme::Light,
        ReducedMotion::NoPreference,
    );
    assert!((eval_number(&mut vm, "visualViewport.width") - 1280.0).abs() < f64::EPSILON);
    assert!((eval_number(&mut vm, "visualViewport.height") - 720.0).abs() < f64::EPSILON);
}

#[test]
fn page_offset_tracks_scroll() {
    // `pageLeft`/`pageTop` = layout-viewport scroll + visual offset(0).
    let mut vm = Vm::new();
    vm.set_scroll_offset(40.0, 90.0);
    assert!((eval_number(&mut vm, "visualViewport.pageLeft") - 40.0).abs() < f64::EPSILON);
    assert!((eval_number(&mut vm, "visualViewport.pageTop") - 90.0).abs() < f64::EPSILON);
}

#[test]
fn attribute_getter_brand_checks_receiver() {
    // WebIDL attribute getter on an alien receiver → TypeError.
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var d = Object.getOwnPropertyDescriptor(VisualViewport.prototype, 'width'); \
         var threw = false; try { d.get.call({}); } catch (e) { threw = e instanceof TypeError; } \
         threw"
    ));
}

// --- EventTarget surface ---------------------------------------------------

#[test]
fn event_handler_attributes_present() {
    let mut vm = Vm::new();
    // `onresize` / `onscroll` / `onscrollend` are accessor IDL attributes,
    // default `null` (no handler set).
    assert!(eval_bool(&mut vm, "visualViewport.onresize === null"));
    assert!(eval_bool(&mut vm, "visualViewport.onscroll === null"));
    assert!(eval_bool(&mut vm, "visualViewport.onscrollend === null"));
}

#[test]
fn add_event_listener_is_real_not_stub() {
    // boa exposed a no-op stub; the VM inherits the real EventTarget method.
    let mut vm = new_vm_with_host();
    assert!(eval_bool(
        &mut vm,
        "typeof visualViewport.addEventListener === 'function' \
         && typeof visualViewport.removeEventListener === 'function'"
    ));
    // Registering / removing a listener must not throw.
    assert!(eval_bool(
        &mut vm,
        "var cb = function () {}; visualViewport.addEventListener('resize', cb); \
         visualViewport.removeEventListener('resize', cb); true"
    ));
}

#[test]
fn onresize_handler_roundtrips() {
    let mut vm = new_vm_with_host();
    assert!(eval_bool(
        &mut vm,
        "var f = function () {}; visualViewport.onresize = f; visualViewport.onresize === f"
    ));
}

// --- platform-object rigor (T3 / T4) ---------------------------------------

#[test]
fn visual_viewport_assignment_leaves_singleton_intact() {
    // T3: `window.visualViewport` is a no-setter RO accessor (not a writable
    // global). elidex-js core is strict-mode-only, so assigning `visualViewport
    // = null` throws a TypeError (inherited-no-setter branch) and must NOT
    // replace the cached singleton.
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var before = window.visualViewport; var threw = false; \
         try { visualViewport = null; } catch (e) { threw = e instanceof TypeError; } \
         threw && window.visualViewport === before \
         && (window.visualViewport instanceof VisualViewport)"
    ));
}

#[test]
fn structured_clone_visual_viewport_throws_data_clone_error() {
    // T4: `VisualViewport` is not [Serializable] — `structuredClone` throws.
    let mut vm = Vm::new();
    let src = "var caught = null; \
               try { structuredClone(visualViewport); } catch (e) { caught = e.name; } caught;";
    match vm.eval(src).unwrap() {
        JsValue::String(id) => assert_eq!(vm.get_string(id), "DataCloneError"),
        other => panic!("expected DataCloneError, got {other:?}"),
    }
}

// --- event producer (T2 / M2) ----------------------------------------------

#[test]
fn deliver_no_ops_while_unbound() {
    // The producer's `is_bound` guard: an unbound deliver must not panic / fire.
    let mut vm = new_vm_with_host();
    vm.eval(
        "globalThis.fired = 0; \
         visualViewport.addEventListener('resize', function () { fired++; });",
    )
    .unwrap();
    vm.set_media_environment(
        1280.0,
        720.0,
        1.0,
        ColorScheme::Light,
        ReducedMotion::NoPreference,
    );
    vm.deliver_visual_viewport_events(); // unbound → no-op
    assert!(eval_bool(&mut vm, "fired === 0"));
}

#[test]
fn first_deliver_after_seed_fires_nothing() {
    // F3: the diff prior is seeded at singleton allocation, so the FIRST deliver
    // (with no intervening geometry change) fires nothing spuriously — even
    // though the resize/scroll listeners are registered before any change.
    with_bound_vm(|vm| {
        vm.eval(
            "globalThis.resizes = 0; globalThis.scrolls = 0; \
             visualViewport.addEventListener('resize', function () { resizes++; }); \
             visualViewport.addEventListener('scroll', function () { scrolls++; });",
        )
        .unwrap();
        vm.deliver_visual_viewport_events();
        assert!(eval_bool(vm, "resizes === 0 && scrolls === 0"));
    });
}

#[test]
fn size_change_fires_resize_only() {
    // M2: a viewport size change fires `resize` (and only `resize` — not
    // `scroll`/`scrollend`, since the offset is unchanged).
    with_bound_vm(|vm| {
        vm.eval(
            "globalThis.resizes = 0; globalThis.scrolls = 0; globalThis.scrollends = 0; \
             globalThis.okTarget = false; \
             visualViewport.addEventListener('resize', function (e) { \
                 resizes++; okTarget = (e.target === visualViewport); }); \
             visualViewport.addEventListener('scroll', function () { scrolls++; }); \
             visualViewport.addEventListener('scrollend', function () { scrollends++; });",
        )
        .unwrap();
        // Seed the prior, then change only the size.
        vm.deliver_visual_viewport_events();
        vm.set_media_environment(
            1280.0,
            720.0,
            1.0,
            ColorScheme::Light,
            ReducedMotion::NoPreference,
        );
        vm.deliver_visual_viewport_events();
        assert!(eval_bool(
            vm,
            "resizes === 1 && okTarget && scrolls === 0 && scrollends === 0"
        ));
    });
}

#[test]
fn scroll_change_fires_scroll_and_scrollend() {
    // M2: a scroll-offset change fires `scroll` + `scrollend` (a settled
    // discrete echo), and NOT `resize` (size unchanged).
    with_bound_vm(|vm| {
        vm.eval(
            "globalThis.resizes = 0; globalThis.scrolls = 0; globalThis.scrollends = 0; \
             visualViewport.addEventListener('resize', function () { resizes++; }); \
             visualViewport.addEventListener('scroll', function () { scrolls++; }); \
             visualViewport.addEventListener('scrollend', function () { scrollends++; });",
        )
        .unwrap();
        vm.deliver_visual_viewport_events();
        vm.set_scroll_offset(40.0, 90.0);
        vm.deliver_visual_viewport_events();
        assert!(eval_bool(
            vm,
            "scrolls === 1 && scrollends === 1 && resizes === 0"
        ));
    });
}

#[test]
fn resize_only_deliver_does_not_fire_scroll() {
    // The load-bearing per-axis distinction: a resize-only deliver must NOT fire
    // scroll/scrollend (the §2-T2 / §4-M2 invariant stated explicitly).
    with_bound_vm(|vm| {
        vm.eval(
            "globalThis.scrolls = 0; globalThis.scrollends = 0; \
             visualViewport.addEventListener('scroll', function () { scrolls++; }); \
             visualViewport.addEventListener('scrollend', function () { scrollends++; });",
        )
        .unwrap();
        vm.deliver_visual_viewport_events();
        vm.set_media_environment(
            1600.0,
            900.0,
            1.0,
            ColorScheme::Light,
            ReducedMotion::NoPreference,
        );
        vm.deliver_visual_viewport_events();
        assert!(eval_bool(vm, "scrolls === 0 && scrollends === 0"));
    });
}

#[test]
fn stable_redeliver_does_not_refire() {
    // After a change is delivered, a redeliver with no further change fires
    // nothing (the prior advanced past the change).
    with_bound_vm(|vm| {
        vm.eval(
            "globalThis.resizes = 0; \
             visualViewport.addEventListener('resize', function () { resizes++; });",
        )
        .unwrap();
        vm.deliver_visual_viewport_events();
        vm.set_media_environment(
            1280.0,
            720.0,
            1.0,
            ColorScheme::Light,
            ReducedMotion::NoPreference,
        );
        vm.deliver_visual_viewport_events();
        vm.deliver_visual_viewport_events(); // unchanged → no second fire
        assert!(eval_bool(vm, "resizes === 1"));
    });
}

#[test]
fn combined_size_and_scroll_change_fires_all_three() {
    // A deliver where both axes moved fires resize + scroll + scrollend.
    with_bound_vm(|vm| {
        vm.eval(
            "globalThis.log = []; \
             visualViewport.addEventListener('resize', function () { log.push('r'); }); \
             visualViewport.addEventListener('scroll', function () { log.push('s'); }); \
             visualViewport.addEventListener('scrollend', function () { log.push('e'); });",
        )
        .unwrap();
        vm.deliver_visual_viewport_events();
        vm.set_media_environment(
            1280.0,
            720.0,
            1.0,
            ColorScheme::Light,
            ReducedMotion::NoPreference,
        );
        vm.set_scroll_offset(10.0, 20.0);
        vm.deliver_visual_viewport_events();
        // resize before scroll before scrollend.
        assert!(eval_bool(vm, "log.join('') === 'rse'"));
    });
}

#[test]
fn same_object_identity_stable_across_reads() {
    // `[SameObject]`: the singleton id is stable across reads within a bind.
    with_bound_vm(|vm| {
        assert!(eval_bool(
            vm,
            "window.visualViewport === window.visualViewport \
             && visualViewport === window.visualViewport"
        ));
    });
}

#[test]
fn singleton_and_prior_reset_on_unbind() {
    // F4 cross-DOM safety: the cached singleton + the producer's diff prior are
    // cleared on `Vm::unbind` (the `localStorage` precedent), so a rebind gets a
    // FRESH singleton (not a stale `ObjectId` from a prior `EcsDom`) and the
    // first deliver after rebind re-seeds against the new starting geometry and
    // fires nothing — even though geometry differs from the pre-unbind state.
    let mut vm = Vm::new();
    vm.install_host_data(HostData::new());

    // --- first bind cycle: allocate the singleton + seed at 1024×768. ---
    let mut session1 = SessionCore::new();
    let mut dom1 = EcsDom::new();
    let root1 = dom1.create_document_root();
    #[allow(unsafe_code)]
    unsafe {
        vm.bind(&raw mut session1, &raw mut dom1, root1);
    }
    vm.eval("globalThis.first = window.visualViewport;")
        .unwrap();
    vm.deliver_visual_viewport_events(); // seeds prior; fires nothing
                                         // Change geometry while bound (so a leaked prior would mis-fire after rebind).
    vm.set_media_environment(
        1600.0,
        900.0,
        1.0,
        ColorScheme::Light,
        ReducedMotion::NoPreference,
    );
    vm.unbind();

    // --- second bind cycle: a fresh singleton + a re-seeded prior. ---
    let mut session2 = SessionCore::new();
    let mut dom2 = EcsDom::new();
    let root2 = dom2.create_document_root();
    #[allow(unsafe_code)]
    unsafe {
        vm.bind(&raw mut session2, &raw mut dom2, root2);
    }
    vm.eval(
        "globalThis.fired = 0; \
         globalThis.second = window.visualViewport; \
         second.addEventListener('resize', function () { fired++; }); \
         second.addEventListener('scroll', function () { fired++; });",
    )
    .unwrap();
    // The post-unbind singleton must NOT be the pre-unbind one (cache cleared).
    assert!(eval_bool(&mut vm, "first !== second"));
    // First deliver after rebind re-seeds (prior was reset to None) → no fire,
    // despite the geometry being 1600×900 (different from the first cycle's
    // pre-change 1024×768 seed).
    vm.deliver_visual_viewport_events();
    assert!(eval_bool(&mut vm, "fired === 0"));
    vm.unbind();
}
