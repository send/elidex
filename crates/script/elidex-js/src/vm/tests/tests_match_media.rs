//! `window.matchMedia` + `MediaQueryList` interface tests (CSSOM-View §4 /
//! §4.2) — Slices 2b-i + 2b-ii.
//!
//! **2b-i** covers the static-snapshot MQL: `matchMedia` returns a live
//! `MediaQueryList`, `.matches` / `.media` reads, the EventTarget
//! integration (`addEventListener('change')` / `onchange` with
//! `this === mql`), interface identity, and the ObjectId-keyed side-table
//! lifecycle (survives unbind, GC-pruned).
//!
//! **2b-ii** covers the host-driven transport + report-changes: the
//! `set_media_environment` device-facts push (and the `innerWidth` /
//! `innerHeight` / `devicePixelRatio` regression-fix that rides it) plus
//! `deliver_media_query_changes` firing `change` (a real
//! `MediaQueryListEvent`) **only on a boolean flip** (CSSOM-View §4.2
//! "evaluate media queries and report changes"). The deliver path needs a
//! *bound* VM (its `is_bound` guard mirrors `deliver_resize_observations`),
//! so those tests use [`with_bound_vm`].

#![cfg(feature = "engine")]

use elidex_css::media::{ColorScheme, ReducedMotion};
use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::host_data::HostData;
use super::super::value::JsValue;
use super::super::Vm;

/// A `Vm` with an (unbound) `HostData` installed — `MediaQueryList`'s
/// `change` listeners live in the unified `vm_event_listeners` home (no DOM
/// needed), exactly like `AbortSignal`. `matchMedia` itself reads only the
/// `VmInner::viewport` default (1024×768), so it works regardless.
fn new_vm() -> Vm {
    let mut v = Vm::new();
    v.install_host_data(super::super::host_data::HostData::new());
    v
}

fn eval_bool(vm: &mut Vm, source: &str) -> bool {
    match vm.eval(source).unwrap() {
        JsValue::Boolean(b) => b,
        other => panic!("expected bool, got {other:?}"),
    }
}

fn eval_string(vm: &mut Vm, source: &str) -> String {
    match vm.eval(source).unwrap() {
        JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string, got {other:?}"),
    }
}

// --- matchMedia + .matches / .media ----------------------------------------

#[test]
fn match_media_returns_object() {
    let mut vm = new_vm();
    assert!(eval_bool(
        &mut vm,
        "typeof matchMedia('(min-width: 1px)') === 'object';"
    ));
}

#[test]
fn matches_true_at_default_viewport() {
    // Default viewport = 1024×768; 1024 >= 500.
    let mut vm = new_vm();
    assert!(eval_bool(
        &mut vm,
        "matchMedia('(min-width: 500px)').matches;"
    ));
}

#[test]
fn matches_false_when_query_exceeds_viewport() {
    let mut vm = new_vm();
    assert!(!eval_bool(
        &mut vm,
        "matchMedia('(min-width: 2000px)').matches;"
    ));
}

#[test]
fn match_media_no_arg_throws() {
    // WebIDL: `query` is required → 0-arg call throws TypeError (arity),
    // not a "undefined"-query MQL.
    let mut vm = new_vm();
    assert!(eval_bool(
        &mut vm,
        "var threw = false; try { matchMedia(); } \
         catch (e) { threw = e instanceof TypeError; } threw;"
    ));
}

#[test]
fn empty_query_matches_true() {
    // mediaqueries §2.1: an empty media query list evaluates to `true`.
    let mut vm = new_vm();
    assert!(eval_bool(&mut vm, "matchMedia('').matches;"));
}

#[test]
fn media_serializes_canonically() {
    // `.media` returns the serialized (canonical) query (#364).
    let mut vm = new_vm();
    assert_eq!(
        eval_string(&mut vm, "matchMedia('(min-width: 500px)').media;"),
        "(min-width: 500px)"
    );
}

#[test]
fn media_normalizes_case_and_whitespace() {
    let mut vm = new_vm();
    assert_eq!(
        eval_string(&mut vm, "matchMedia('(MIN-WIDTH:500PX)').media;"),
        "(min-width: 500px)"
    );
}

#[test]
fn boa_parity_min_max_width_height() {
    // Every query boa's string-splitter supported returns the same verdict
    // at 1024×768 (superset, no regression).
    let mut vm = new_vm();
    assert!(eval_bool(
        &mut vm,
        "matchMedia('(min-width: 1024px)').matches \
         && matchMedia('(max-width: 1024px)').matches \
         && matchMedia('(min-height: 768px)').matches \
         && matchMedia('(max-height: 768px)').matches \
         && !matchMedia('(min-width: 1025px)').matches \
         && !matchMedia('(max-width: 1023px)').matches;"
    ));
}

// --- interface identity ----------------------------------------------------

#[test]
fn instanceof_media_query_list() {
    let mut vm = new_vm();
    assert!(eval_bool(
        &mut vm,
        "matchMedia('(color)') instanceof MediaQueryList;"
    ));
}

#[test]
fn distinct_objects_per_call() {
    // CSSOM does not mandate identity across calls; boa parity = per-call.
    let mut vm = new_vm();
    assert!(eval_bool(
        &mut vm,
        "matchMedia('(color)') !== matchMedia('(color)');"
    ));
}

#[test]
fn new_media_query_list_throws() {
    // WebIDL: MediaQueryList has no constructor.
    let mut vm = new_vm();
    assert!(eval_bool(
        &mut vm,
        "var threw = false; try { new MediaQueryList(); } \
         catch (e) { threw = e instanceof TypeError; } threw;"
    ));
}

#[test]
fn matches_is_readonly() {
    // RO accessor (no setter) → strict-mode assignment throws (elidex is
    // strict-only).
    let mut vm = new_vm();
    assert!(eval_bool(
        &mut vm,
        "var m = matchMedia('(min-width: 1px)'); var threw = false; \
         try { m.matches = false; } catch (e) { threw = e instanceof TypeError; } \
         threw;"
    ));
}

#[test]
fn accessor_on_non_mql_this_throws() {
    let mut vm = new_vm();
    assert!(eval_bool(
        &mut vm,
        "var g = Object.getOwnPropertyDescriptor(MediaQueryList.prototype, 'matches').get; \
         var threw = false; try { g.call({}); } catch (e) { threw = e instanceof TypeError; } \
         threw;"
    ));
}

// --- EventTarget integration (this === mql; the boa fresh-`this` bug) ------

#[test]
fn add_event_listener_change_fires_with_mql_target_and_this() {
    let mut vm = new_vm();
    assert!(eval_bool(
        &mut vm,
        "var m = matchMedia('(min-width: 1px)'); var okThis = false, okTarget = false; \
         m.addEventListener('change', function (e) { okThis = (this === m); okTarget = (e.target === m); }); \
         m.dispatchEvent(new Event('change')); \
         okThis && okTarget;"
    ));
}

#[test]
fn onchange_fires_with_mql_this() {
    let mut vm = new_vm();
    assert!(eval_bool(
        &mut vm,
        "var m = matchMedia('(min-width: 1px)'); var okThis = false; \
         m.onchange = function () { okThis = (this === m); }; \
         m.dispatchEvent(new Event('change')); \
         okThis;"
    ));
}

#[test]
fn remove_event_listener_stops_delivery() {
    let mut vm = new_vm();
    assert!(eval_bool(
        &mut vm,
        "var m = matchMedia('(min-width: 1px)'); var n = 0; \
         function cb() { n++; } \
         m.addEventListener('change', cb); m.removeEventListener('change', cb); \
         m.dispatchEvent(new Event('change')); \
         n === 0;"
    ));
}

// --- legacy addListener / removeListener are OUT-OF-CORE (Codex R2) --------

#[test]
fn legacy_add_remove_listener_not_in_core() {
    // Codex R2: addListener/removeListener are CSSOM-View §4.2 legacy aliases
    // ("basically aliases for addEventListener", superseded) → out-of-core per
    // the core/compat/deprecated tiering (docs/design §14.1.1/§14.4.2). The
    // modern addEventListener('change') is the core surface.
    let mut vm = new_vm();
    assert!(eval_bool(
        &mut vm,
        "var m = matchMedia('(min-width: 1px)'); \
         typeof m.addListener === 'undefined' \
         && typeof m.removeListener === 'undefined' \
         && typeof m.addEventListener === 'function';"
    ));
}

#[test]
fn media_query_list_not_exposed_in_worker_scope() {
    // Codex R1: MediaQueryList is `[Exposed=Window]` (CSSOM-View §4.2) — a
    // worker realm must NOT get the global constructor (nor matchMedia).
    let mut vm = Vm::new_worker(
        "w".to_string(),
        url::Url::parse("https://example.com/w.js").unwrap(),
        true,
        elidex_net::CredentialsMode::SameOrigin,
        elidex_plugin::EngineMode::BrowserCompat,
    );
    vm.install_host_data(super::super::host_data::HostData::new());
    assert!(eval_bool(
        &mut vm,
        "typeof MediaQueryList === 'undefined' && typeof matchMedia === 'undefined' \
         && typeof MediaQueryListEvent === 'undefined';"
    ));
}

#[test]
fn prototype_survives_severed_global_and_gc() {
    // Codex R1: media_query_list_prototype is GC-rooted, so severing the
    // `MediaQueryList` global + a GC cannot sweep the cached prototype out
    // from under a later matchMedia() call.
    let mut vm = new_vm();
    vm.eval("globalThis.MediaQueryList = null;").unwrap();
    vm.inner.collect_garbage();
    // The cached prototype survived → matchMedia still yields a working MQL.
    assert_eq!(
        eval_string(&mut vm, "matchMedia('(min-width: 1px)').media;"),
        "(min-width: 1px)"
    );
}

// --- MediaQueryListEvent (CSSOM-View §4.2) — Codex R2 -----------------------

#[test]
fn media_query_list_event_constructible() {
    // CSSOM-View §4.2: MediaQueryListEvent is [Exposed=Window] + constructible,
    // a sibling of MediaQueryList. matches/media from the init dict; chains to
    // Event.
    let mut vm = new_vm();
    assert!(eval_bool(
        &mut vm,
        "typeof MediaQueryListEvent === 'function'; \
         var e = new MediaQueryListEvent('change', { matches: true, media: '(min-width: 1px)' }); \
         e.type === 'change' && e.matches === true && e.media === '(min-width: 1px)' \
         && e instanceof MediaQueryListEvent && e instanceof Event;"
    ));
}

#[test]
fn media_query_list_event_defaults_and_readonly() {
    // Defaults: matches=false, media="". matches/media are RO (strict assign
    // throws).
    let mut vm = new_vm();
    assert!(eval_bool(
        &mut vm,
        "var e = new MediaQueryListEvent('change'); \
         var okDefault = e.matches === false && e.media === ''; \
         var threw = false; \
         try { e.matches = true; } catch (err) { threw = err instanceof TypeError; } \
         okDefault && threw;"
    ));
}

#[test]
fn media_query_list_event_dispatches_to_mql() {
    // A constructed MediaQueryListEvent dispatched on an MQL reaches a
    // 'change' listener with this === mql and the event's matches/media.
    let mut vm = new_vm();
    assert!(eval_bool(
        &mut vm,
        "var m = matchMedia('(min-width: 1px)'); var okThis = false, okMatches = false; \
         m.addEventListener('change', function (e) { okThis = (this === m); okMatches = (e.matches === true); }); \
         m.dispatchEvent(new MediaQueryListEvent('change', { matches: true })); \
         okThis && okMatches;"
    ));
}

#[test]
fn mql_accepted_as_event_related_target() {
    // MediaQueryList is a non-Node EventTarget, so it is a valid WebIDL
    // `EventTarget?` relatedTarget — exercises the unified
    // `ObjectKind::is_non_node_event_target` accept-list (the new brand must
    // be recognized by the relatedTarget coercion, not just listener routing).
    let mut vm = new_vm();
    assert!(eval_bool(
        &mut vm,
        "var m = matchMedia('(min-width: 1px)'); \
         var e = new MouseEvent('click', { relatedTarget: m }); \
         e.relatedTarget === m;"
    ));
}

// --- ObjectId-keyed side-table lifecycle (F2 survive-unbind / F3 GC) -------

#[test]
fn registry_survives_unbind() {
    // F2: the registry value is DOM-free, so a retained MQL survives unbind
    // (AbortSignal parity) — it is NOT in the unbind clear-set.
    let mut vm = new_vm();
    vm.eval("globalThis.m = matchMedia('(min-width: 1px)');")
        .unwrap();
    assert_eq!(vm.inner.media_query_list_registry.len(), 1);
    vm.unbind();
    assert_eq!(
        vm.inner.media_query_list_registry.len(),
        1,
        "MQL registry must survive unbind (DOM-free, AbortSignal parity)"
    );
}

#[test]
fn gc_prunes_dropped_mql() {
    // F3: dropping the only JS reference + a GC prunes the registry entry
    // (the sweep-prune is the sole delete-path; no trace root).
    let mut vm = new_vm();
    vm.eval("globalThis.m = matchMedia('(min-width: 1px)');")
        .unwrap();
    assert_eq!(vm.inner.media_query_list_registry.len(), 1);
    vm.eval("globalThis.m = null;").unwrap();
    vm.inner.collect_garbage();
    assert_eq!(
        vm.inner.media_query_list_registry.len(),
        0,
        "collected MQL must leave no stale registry entry"
    );
}

// --- Slice 2b-ii: transport + report-changes -------------------------------

/// Run `body` against a fully **bound** VM (session + DOM + document root).
/// `deliver_media_query_changes` no-ops while unbound (its `is_bound` guard,
/// mirroring `deliver_resize_observations` — no JS may run between documents),
/// so the report-changes tests need a real binding. `session` / `dom` live on
/// this frame for the whole `body`, keeping the bound raw pointers valid.
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

#[test]
fn matches_reflects_transported_viewport() {
    // `.matches` is live-computed, so a transported viewport change is visible
    // immediately (no deliver needed) — the read side of the regression-fix.
    with_bound_vm(|vm| {
        vm.eval("globalThis.m = matchMedia('(min-width: 1500px)');")
            .unwrap();
        assert!(!eval_bool(vm, "m.matches;")); // 1024 < 1500
        vm.set_media_environment(
            1600.0,
            900.0,
            1.0,
            ColorScheme::Light,
            ReducedMotion::NoPreference,
        );
        assert!(eval_bool(vm, "m.matches;")); // 1600 >= 1500
    });
}

#[test]
fn inner_width_height_dppr_reflect_transported_env() {
    // Regression-fix: the window getters used to lie at the 1024/768/1
    // defaults (no setter existed); they now derive from the transported
    // `ViewportState`.
    with_bound_vm(|vm| {
        vm.set_media_environment(
            1440.0,
            900.0,
            2.0,
            ColorScheme::Light,
            ReducedMotion::NoPreference,
        );
        assert!(eval_bool(
            vm,
            "innerWidth === 1440 && innerHeight === 900 && devicePixelRatio === 2;"
        ));
    });
}

#[test]
fn change_fires_on_flip_with_media_query_list_event() {
    with_bound_vm(|vm| {
        vm.eval(
            "globalThis.fired = 0; globalThis.okThis = false; globalThis.okEvt = false; \
             globalThis.m = matchMedia('(min-width: 1500px)'); \
             m.addEventListener('change', function (e) { \
                 fired++; okThis = (this === m); \
                 okEvt = (e instanceof MediaQueryListEvent) && e.matches === true \
                         && e.media === '(min-width: 1500px)'; \
             });",
        )
        .unwrap();
        vm.set_media_environment(
            1600.0,
            900.0,
            1.0,
            ColorScheme::Light,
            ReducedMotion::NoPreference,
        );
        vm.deliver_media_query_changes();
        assert!(eval_bool(vm, "fired === 1 && okThis && okEvt;"));
    });
}

#[test]
fn change_does_not_fire_without_flip() {
    with_bound_vm(|vm| {
        vm.eval(
            "globalThis.fired = 0; \
             globalThis.m = matchMedia('(min-width: 500px)'); \
             m.addEventListener('change', function () { fired++; });",
        )
        .unwrap();
        // 1024 → 1200: both still satisfy (min-width: 500px) → no flip.
        vm.set_media_environment(
            1200.0,
            768.0,
            1.0,
            ColorScheme::Light,
            ReducedMotion::NoPreference,
        );
        vm.deliver_media_query_changes();
        assert!(eval_bool(vm, "fired === 0;"));
    });
}

#[test]
fn onchange_fires_on_flip() {
    with_bound_vm(|vm| {
        vm.eval(
            "globalThis.got = null; \
             globalThis.m = matchMedia('(max-width: 1000px)'); \
             m.onchange = function (e) { got = e.matches; };",
        )
        .unwrap();
        // 1024 (false: 1024 > 1000) → 800 (true: 800 <= 1000): flip → true.
        vm.set_media_environment(
            800.0,
            768.0,
            1.0,
            ColorScheme::Light,
            ReducedMotion::NoPreference,
        );
        vm.deliver_media_query_changes();
        assert!(eval_bool(vm, "got === true;"));
    });
}

#[test]
fn deliver_does_not_refire_on_stable_redelivery() {
    // A second deliver with no further env change must NOT re-fire —
    // `last_matches` was advanced to the reported value on the first delivery.
    with_bound_vm(|vm| {
        vm.eval(
            "globalThis.fired = 0; \
             globalThis.m = matchMedia('(min-width: 1500px)'); \
             m.addEventListener('change', function () { fired++; });",
        )
        .unwrap();
        vm.set_media_environment(
            1600.0,
            900.0,
            1.0,
            ColorScheme::Light,
            ReducedMotion::NoPreference,
        );
        vm.deliver_media_query_changes();
        vm.deliver_media_query_changes(); // env unchanged → no second fire
        assert!(eval_bool(vm, "fired === 1;"));
    });
}

#[test]
fn removed_listener_no_change_but_matches_tracks() {
    with_bound_vm(|vm| {
        vm.eval(
            "globalThis.fired = 0; \
             globalThis.m = matchMedia('(min-width: 1500px)'); \
             globalThis.cb = function () { fired++; }; \
             m.addEventListener('change', cb); m.removeEventListener('change', cb);",
        )
        .unwrap();
        vm.set_media_environment(
            1600.0,
            900.0,
            1.0,
            ColorScheme::Light,
            ReducedMotion::NoPreference,
        );
        vm.deliver_media_query_changes();
        // No listener → no fire, but `.matches` still reflects the new env.
        assert!(eval_bool(vm, "fired === 0 && m.matches === true;"));
    });
}

#[test]
fn multiple_mqls_each_deliver_independently() {
    with_bound_vm(|vm| {
        vm.eval(
            "globalThis.log = []; \
             globalThis.a = matchMedia('(min-width: 1500px)'); \
             globalThis.b = matchMedia('(min-width: 1800px)'); \
             a.addEventListener('change', function (e) { log.push('a' + e.matches); }); \
             b.addEventListener('change', function (e) { log.push('b' + e.matches); });",
        )
        .unwrap();
        // 1024 → 1600: a flips true (>=1500); b stays false (<1800).
        vm.set_media_environment(
            1600.0,
            900.0,
            1.0,
            ColorScheme::Light,
            ReducedMotion::NoPreference,
        );
        vm.deliver_media_query_changes();
        assert_eq!(eval_string(vm, "log.join(',');"), "atrue");
        // 1600 → 1900: b flips true; a already true → no re-fire.
        vm.set_media_environment(
            1900.0,
            900.0,
            1.0,
            ColorScheme::Light,
            ReducedMotion::NoPreference,
        );
        vm.deliver_media_query_changes();
        assert_eq!(eval_string(vm, "log.join(',');"), "atrue,btrue");
    });
}

#[test]
fn change_fires_on_prefers_color_scheme_flip() {
    // The transport drives the FULL env: a `prefers-color-scheme` MQL flips
    // when `color_scheme` is transported (VM path complete + test-exercised;
    // the shell theme producer is carved to `#11-media-prefers-features`).
    with_bound_vm(|vm| {
        vm.eval(
            "globalThis.got = null; \
             globalThis.m = matchMedia('(prefers-color-scheme: dark)'); \
             m.addEventListener('change', function (e) { got = e.matches; });",
        )
        .unwrap();
        assert!(!eval_bool(vm, "m.matches;")); // default Light
        vm.set_media_environment(
            1024.0,
            768.0,
            1.0,
            ColorScheme::Dark,
            ReducedMotion::NoPreference,
        );
        vm.deliver_media_query_changes();
        assert!(eval_bool(vm, "got === true && m.matches === true;"));
    });
}

#[test]
fn change_listener_reentrancy_is_snapshot_safe() {
    // A `change` listener that mutates the registry mid-dispatch (here: calls
    // `matchMedia`, inserting a new MQL) must neither perturb this turn's flip
    // set (phase-A snapshot iterates an owned Vec) nor panic. The re-entrantly
    // created MQL is seeded to the current env, so it is NOT delivered this
    // turn — only the original `m` fires.
    with_bound_vm(|vm| {
        vm.eval(
            "globalThis.fired = 0; globalThis.added = null; \
             globalThis.m = matchMedia('(min-width: 1500px)'); \
             m.addEventListener('change', function () { \
                 fired++; \
                 added = matchMedia('(min-width: 1500px)'); \
             });",
        )
        .unwrap();
        vm.set_media_environment(
            1600.0,
            900.0,
            1.0,
            ColorScheme::Light,
            ReducedMotion::NoPreference,
        );
        vm.deliver_media_query_changes();
        assert!(eval_bool(
            vm,
            "fired === 1 && added !== null && added.matches === true;"
        ));
    });
}

#[test]
fn deliver_is_noop_while_unbound() {
    // Parity with `deliver_resize_observations`: a stray delivery on an unbound
    // VM must not panic (no `host_data().dom()` deref) and must not fire.
    let mut vm = new_vm(); // HostData installed but NOT bound
    vm.eval(
        "globalThis.fired = 0; \
         globalThis.m = matchMedia('(min-width: 1500px)'); \
         m.addEventListener('change', function () { fired++; });",
    )
    .unwrap();
    vm.set_media_environment(
        1600.0,
        900.0,
        1.0,
        ColorScheme::Light,
        ReducedMotion::NoPreference,
    );
    vm.deliver_media_query_changes(); // unbound → no-op
    assert!(eval_bool(&mut vm, "fired === 0;"));
}

// --- Codex R1: ObjectId is recycle-prone, not a stable/monotonic identity ---

#[test]
fn report_order_follows_creation_seq_not_recycled_object_id() {
    // C2 (CSSOM-View §4.2 creation order): `alloc_object` recycles a collected
    // MQL's `ObjectId` via the GC free-list, so a *newer* MQL can hold a
    // *lower* id than an older live one. Report order must follow creation
    // order (`MediaQueryEntry::seq`), not the recycle-prone `ObjectId` key.
    with_bound_vm(|vm| {
        // `a` (lowest id) will be dropped; `b` stays live (higher id).
        vm.eval(
            "globalThis.log = []; \
             globalThis.a = matchMedia('(min-width: 1500px)'); \
             globalThis.b = matchMedia('(min-width: 1500px)'); \
             b.addEventListener('change', function () { log.push('b'); });",
        )
        .unwrap();
        let id_b = match vm.eval("globalThis.b;").unwrap() {
            JsValue::Object(id) => id,
            other => panic!("expected object, got {other:?}"),
        };
        // Drop `a` + GC: its (low) `ObjectId` slot goes on the free-list.
        vm.eval("globalThis.a = null;").unwrap();
        vm.inner.collect_garbage();
        // `c` is created LAST (highest `seq`) but its MQL is the first object
        // allocated after the GC, so it reuses `a`'s freed (low) slot.
        vm.eval(
            "globalThis.c = matchMedia('(min-width: 1500px)'); \
             c.addEventListener('change', function () { log.push('c'); });",
        )
        .unwrap();
        let id_c = match vm.eval("globalThis.c;").unwrap() {
            JsValue::Object(id) => id,
            other => panic!("expected object, got {other:?}"),
        };
        assert!(
            id_c.0 < id_b.0,
            "precondition: `c` must reuse `a`'s recycled (lower) slot so id \
             order (c,b) inverts creation order (b,c) — id_c={}, id_b={}",
            id_c.0,
            id_b.0,
        );
        // 1024 → 1600: both `b` and `c` flip true. Report order must be the
        // creation order b, then c (`seq`), NOT the id order c, then b.
        vm.set_media_environment(
            1600.0,
            900.0,
            1.0,
            ColorScheme::Light,
            ReducedMotion::NoPreference,
        );
        vm.deliver_media_query_changes();
        assert_eq!(eval_string(vm, "log.join(',');"), "b,c");
    });
}

#[test]
fn recycle_during_dispatch_is_safe_and_fires_no_stale_change() {
    // C1: a `change` listener drops another flipped MQL mid-dispatch and, under
    // a GC that runs between phase A (snapshot) and phase B (fire), that MQL's
    // `ObjectId` can be collected + recycled. The phase-B liveness check must
    // re-verify the snapshotted `seq` (recycle-immune), not just `contains_key`,
    // so a recycled slot never receives this turn's stale snapshot — and the
    // pass must not panic on the freed entry.
    //
    // NOTE: forcing the recycled slot to be re-occupied by a *new MQL* is not
    // deterministic here — MQL allocation runs inside the native `matchMedia`
    // call where GC is disabled, so the collecting GC is necessarily a separate
    // JS-level allocation (which itself consumes the freed slot first). The
    // discriminating "`seq` is the post-recycle identity / report order"
    // property is covered by `report_order_follows_creation_seq_*`; this test
    // guards the GC-during-dispatch safety + no-spurious-fire half.
    with_bound_vm(|vm| {
        // `keeper` flips first (lowest `seq`); its listener drops `victim` then
        // allocates at JS level to drive a GC while phase B is still in flight.
        vm.eval(
            "globalThis.recycledFired = false; \
             globalThis.keeper = matchMedia('(min-width: 1500px)'); \
             globalThis.victim = matchMedia('(min-width: 1500px)'); \
             keeper.addEventListener('change', function () { \
                 victim = null; \
                 let sink = []; \
                 for (let i = 0; i < 8; i++) { sink.push({}); } \
                 globalThis.recycled = matchMedia('(min-width: 1500px)'); \
                 recycled.addEventListener('change', function () { \
                     recycledFired = true; \
                 }); \
             });",
        )
        .unwrap();
        // GC on every JS-level allocation so the listener's `{}` literals
        // collect the just-dropped `victim` mid-dispatch (the C1 race window).
        vm.inner.gc_threshold = 0;
        // 1024 → 1600: both `keeper` and `victim` flip; phase B fires `keeper`
        // first, whose listener performs the drop + GC before the loop reaches
        // `victim`'s snapshotted id.
        vm.set_media_environment(
            1600.0,
            900.0,
            1.0,
            ColorScheme::Light,
            ReducedMotion::NoPreference,
        );
        vm.deliver_media_query_changes(); // must not panic on the freed entry
        assert!(
            eval_bool(vm, "recycledFired === false;"),
            "a slot recycled mid-dispatch must not receive the stale snapshot's \
             `change` (seq identity re-check, not contains_key)",
        );
    });
}

#[test]
fn same_document_mql_delivers_across_a_batch_rebind() {
    // R2-1 (Codex): the engine's BATCH-BIND model brackets every batch
    // (script-exec / event-dispatch / frame-drain) with bind/unbind, bumping
    // `bind_epoch` each time. So an MQL created in one batch
    // (`eval(matchMedia + listener)`) MUST still deliver in a LATER batch
    // (`set_media_environment` + `deliver_media_query_changes`) for the SAME
    // document. Scoping on the document `Entity` (not the per-batch
    // `bind_epoch`) makes this hold — an epoch filter would skip it (the R1
    // regression this guards against).
    let mut vm = Vm::new();
    vm.install_host_data(HostData::new());
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();

    // Batch 1: create the MQL + listener, then close the batch (unbind bumps
    // `bind_epoch`).
    #[allow(unsafe_code)]
    unsafe {
        vm.bind(&raw mut session, &raw mut dom, doc);
    }
    vm.eval(
        "globalThis.fired = 0; \
         globalThis.m = matchMedia('(min-width: 1500px)'); \
         m.addEventListener('change', function () { fired++; });",
    )
    .unwrap();
    vm.unbind();

    // Batch 2: SAME document, later env push + deliver.
    #[allow(unsafe_code)]
    unsafe {
        vm.bind(&raw mut session, &raw mut dom, doc);
    }
    vm.set_media_environment(
        1600.0,
        900.0,
        1.0,
        ColorScheme::Light,
        ReducedMotion::NoPreference,
    );
    vm.deliver_media_query_changes();
    assert!(
        eval_bool(&mut vm, "fired === 1;"),
        "an MQL created in an earlier batch must still deliver in a later batch \
         for the SAME document (document-Entity scope, not per-batch bind_epoch)",
    );
    vm.unbind();
}

#[test]
fn retained_prior_document_mql_is_inert_in_a_different_document() {
    // C3 (CSSOM-View §4.2 associated-document scope): the registry survives
    // unbind, so a retained MQL from a prior document persists. Its `change`
    // listener must NOT fire during a *different* document's report-changes
    // pass — the entry's `document` `Entity` scopes delivery — while a fresh
    // MQL created in the new document fires normally.
    //
    // Two distinct document entities in ONE `EcsDom`, so their `Entity`s are
    // provably different (no cross-`EcsDom`-world index aliasing — that edge is
    // the deferred world_id concern).
    // ⚠ SUPERSEDED 2026-06-30: world_id retracted → agent-scoped EcsDom World
    // (PR #434 docs/plans/2026-06-agent-scoped-ecsdom-world.md §6); interim form
    // unchanged until B1.
    let mut vm = Vm::new();
    vm.install_host_data(HostData::new());
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc_a = dom.create_document_root();
    let doc_b = dom.create_document_root();
    assert_ne!(doc_a, doc_b, "two document roots must be distinct entities");

    // Document A: create a retained MQL + listener, then unbind.
    #[allow(unsafe_code)]
    unsafe {
        vm.bind(&raw mut session, &raw mut dom, doc_a);
    }
    vm.eval(
        "globalThis.oldFired = 0; \
         globalThis.oldM = matchMedia('(min-width: 1500px)'); \
         oldM.addEventListener('change', function () { oldFired++; });",
    )
    .unwrap();
    vm.unbind();

    // Document B: a fresh MQL + listener — the control proving the deliver path
    // still fires in the new document, so a missing `oldM` fire is the
    // document scope, not a broken deliver.
    #[allow(unsafe_code)]
    unsafe {
        vm.bind(&raw mut session, &raw mut dom, doc_b);
    }
    vm.eval(
        "globalThis.newFired = 0; \
         globalThis.newM = matchMedia('(min-width: 1500px)'); \
         newM.addEventListener('change', function () { newFired++; });",
    )
    .unwrap();
    // 1024 → 1600 flips BOTH queries' results, but only the doc_b MQL is in
    // doc_b's pass.
    vm.set_media_environment(
        1600.0,
        900.0,
        1.0,
        ColorScheme::Light,
        ReducedMotion::NoPreference,
    );
    vm.deliver_media_query_changes();
    assert!(
        eval_bool(&mut vm, "oldFired === 0 && newFired === 1;"),
        "the doc_a MQL must be inert in doc_b (oldFired=0) while the doc_b MQL \
         fires (newFired=1)",
    );
    vm.unbind();
}
