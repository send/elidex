//! S5-3a — the keepalive-predicate seam, exercised through `MediaQueryList`
//! (`#11-eventtarget-listener-keepalive-rooting`).
//!
//! An MQL anchored ONLY by a `change` listener has its callback rooted
//! (`listener_store`) but not its own wrapper; the seam
//! (`gc::keepalive::keepalive_survivors`, run in `collect_garbage`) keeps it
//! alive iff it has a LIVE `change` listener — the dispatch-time
//! `vm_path_has_listener` predicate, so kept-alive ⇔ would-actually-fire —
//! AND `MediaQueryEntry::keepalive_worthy` holds (the GC-liveness gate:
//! deliverable to the bound document, or — while unbound — a `document`-tagged
//! MQL preserved for the next same-document rebind, since GC liveness ≠ dispatch
//! deliverability; Codex PR#430 R5).
//!
//! Split out of [`super::tests_match_media`] (the matchMedia/MQL-interface
//! suite) to keep that file under the 1000-line convention (CLAUDE.md
//! touch-time split; Codex PR#430 R1).

#![cfg(feature = "engine")]

use elidex_css::media::{ColorScheme, ReducedMotion};
use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::host_data::HostData;
use super::super::value::JsValue;
use super::super::Vm;

/// Run `body` against a fully **bound** VM (session + DOM + document root) —
/// the report-changes path no-ops while unbound, so the deliver-driven
/// keepalive tests need a real binding (see `tests_match_media::with_bound_vm`).
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

fn eval_bool(vm: &mut Vm, source: &str) -> bool {
    match vm.eval(source).unwrap() {
        JsValue::Boolean(b) => b,
        other => panic!("expected bool, got {other:?}"),
    }
}

#[test]
fn listener_only_mql_survives_gc_and_still_delivers() {
    // Headline: an MQL kept alive ONLY by a `change` listener
    // (`matchMedia(q).addEventListener('change', cb)`, no retained reference)
    // must survive GC so a later report-changes pass can still deliver. Before
    // the seam, `listener_store` rooted the callback but not the MQL, so a GC
    // before the flip swept it and the `change` was silently lost.
    with_bound_vm(|vm| {
        vm.eval(
            "globalThis.fired = 0; \
             matchMedia('(min-width: 1500px)') \
                 .addEventListener('change', function () { globalThis.fired++; });",
        )
        .unwrap();
        // No JS reference survives the eval — only the `change` listener anchors
        // the MQL. Without the seam this GC would sweep it.
        vm.inner.collect_garbage();
        assert_eq!(
            vm.inner.media_query_list_registry.len(),
            1,
            "a listener-only MQL must survive GC via the keepalive seam",
        );
        vm.set_media_environment(
            1600.0,
            900.0,
            1.0,
            ColorScheme::Light,
            ReducedMotion::NoPreference,
        );
        vm.deliver_media_query_changes();
        assert!(
            eval_bool(vm, "fired === 1;"),
            "the survived listener-only MQL must still deliver `change`",
        );
    });
}

#[test]
fn listener_less_mql_is_collected_no_over_rooting() {
    // Negative control: the seam roots only an MQL with a LIVE `change`
    // listener — a listener-less unreferenced MQL is still collected (the
    // predicate is NOT a blanket registry-membership root; DOM §2.8). The
    // bound-context companion of `gc_prunes_dropped_mql`.
    with_bound_vm(|vm| {
        vm.eval("globalThis.m = matchMedia('(min-width: 1500px)');")
            .unwrap();
        vm.eval("globalThis.m = null;").unwrap();
        vm.inner.collect_garbage();
        assert_eq!(
            vm.inner.media_query_list_registry.len(),
            0,
            "a listener-less unreferenced MQL must be collected (no over-root)",
        );
    });
}

#[test]
fn onchange_only_mql_survives_gc_and_delivers() {
    // The `onchange` event-handler attribute also anchors the MQL: the seam's
    // `vm_path_has_listener` predicate counts the EventHandler listener entry,
    // not just `addEventListener` registrations. No reference beyond the
    // handler.
    with_bound_vm(|vm| {
        vm.eval(
            "globalThis.got = null; \
             matchMedia('(max-width: 1000px)').onchange = \
                 function (e) { globalThis.got = e.matches; };",
        )
        .unwrap();
        vm.inner.collect_garbage();
        assert_eq!(
            vm.inner.media_query_list_registry.len(),
            1,
            "an onchange-only MQL must survive GC via the keepalive seam",
        );
        // 1024 (false: 1024 > 1000) → 800 (true): flip → onchange fires.
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
fn cleared_onchange_mql_is_collected() {
    // The predicate is callable-liveness aware: a cleared `onchange = null`
    // handler keeps its `EventListeners` metadata entry but retires the
    // callable from `listener_store`, so `vm_path_has_listener` returns false
    // and the seam does NOT keep an MQL whose only `change` registration was
    // nulled out — it is collected. (Guards against a naive metadata-presence
    // test that would over-root a cleared handler.)
    with_bound_vm(|vm| {
        vm.eval(
            "globalThis.m = matchMedia('(min-width: 1500px)'); \
             m.onchange = function () {}; \
             m.onchange = null;",
        )
        .unwrap();
        vm.eval("globalThis.m = null;").unwrap();
        vm.inner.collect_garbage();
        assert_eq!(
            vm.inner.media_query_list_registry.len(),
            0,
            "an MQL whose only `change` handler was cleared (onchange=null) must \
             be collected (callable retired from listener_store)",
        );
    });
}

#[test]
fn prior_document_listener_only_mql_collected_after_rebind() {
    // Document-scope (no cross-DOM keepalive leak): the registry survives
    // unbind and `vm_event_listeners` is NOT unbind-cleared, so a retained
    // prior-document listener-only MQL keeps its `change` listener across a
    // rebind. Without the document gate the keepalive would root it forever
    // (the cross-DOM survival the deliver pass already guards against). Gating
    // on the creating-document `Entity` collects it once a *different* document
    // is bound — only the listener (no JS ref) anchored it.
    let mut vm = Vm::new();
    vm.install_host_data(HostData::new());
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc_a = dom.create_document_root();
    let doc_b = dom.create_document_root();
    assert_ne!(doc_a, doc_b, "two document roots must be distinct entities");

    // Document A: a listener-only MQL (no retained reference), then unbind.
    #[allow(unsafe_code)]
    unsafe {
        vm.bind(&raw mut session, &raw mut dom, doc_a);
    }
    vm.eval("matchMedia('(min-width: 1500px)').addEventListener('change', function () {});")
        .unwrap();
    assert_eq!(vm.inner.media_query_list_registry.len(), 1);
    vm.unbind();

    // Document B bound: the doc_A MQL's listener still lives in
    // `vm_event_listeners`, but its document is doc_A ≠ doc_B, so the keepalive
    // gate skips it and the next GC collects it.
    #[allow(unsafe_code)]
    unsafe {
        vm.bind(&raw mut session, &raw mut dom, doc_b);
    }
    vm.inner.collect_garbage();
    assert_eq!(
        vm.inner.media_query_list_registry.len(),
        0,
        "a prior-document listener-only MQL must be collected after rebind (no \
         cross-DOM keepalive leak)",
    );
    vm.unbind();
}

#[test]
fn same_document_listener_only_mql_survives_unbound_inter_batch_gc() {
    // GC LIVENESS ≠ dispatch deliverability (Codex PR#430 R5): the BATCH-BIND
    // model unbinds between batches, so a GC fired while UNBOUND
    // (`current_document == None`) must NOT collect a `document`-tagged
    // listener-only MQL — it has to survive so the NEXT same-document rebind's
    // `deliver` can still fire it. Collecting it (the strict dispatch gate) would
    // reintroduce the silent lost-`change` the seam exists to fix. The
    // cross-`EcsDom`-rebind case keepalive_worthy cannot distinguish is the
    // deferred world_id concern (`#11-wrapper-cache-cross-dom-discriminator`,
    // strictly AFTER S5).
    let mut vm = Vm::new();
    vm.install_host_data(HostData::new());
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();

    // Batch 1: a listener-only MQL (no retained reference), then unbind.
    #[allow(unsafe_code)]
    unsafe {
        vm.bind(&raw mut session, &raw mut dom, doc);
    }
    vm.eval(
        "globalThis.fired = 0; \
         matchMedia('(min-width: 1500px)') \
             .addEventListener('change', function () { globalThis.fired++; });",
    )
    .unwrap();
    vm.unbind();

    // An inter-batch GC while unbound must KEEP the same-document MQL alive.
    vm.inner.collect_garbage();
    assert_eq!(
        vm.inner.media_query_list_registry.len(),
        1,
        "an unbound inter-batch GC must not collect a listener-only same-document MQL",
    );

    // Batch 2: rebind the SAME document, flip, deliver → the listener fires.
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
        "the MQL that survived the unbound GC must still deliver after rebind",
    );
    vm.unbind();
}

#[test]
fn unbound_created_listener_only_mql_is_collected() {
    // An MQL created through the UNBOUND path stores `document == None`. It can
    // never be delivered (deliver no-ops while unbound, and once a real document
    // binds `None != Some(doc)`), so `keepalive_worthy`'s `document.is_some()`
    // guard collects it even though the GC runs unbound — a listener-only
    // unbound-created MQL is NOT leaked (the `document`-tagged survival is
    // same-document-rebind-pending; a document-less MQL has no rebind to pend).
    // Guards the `None == None` case a bare `document == current_document` filter
    // would wrongly root (Codex PR#430 R2 P2).
    let mut vm = Vm::new();
    vm.install_host_data(HostData::new()); // HostData installed but NOT bound
    vm.eval("matchMedia('(min-width: 1500px)').addEventListener('change', function () {});")
        .unwrap();
    assert_eq!(vm.inner.media_query_list_registry.len(), 1);
    vm.inner.collect_garbage();
    assert_eq!(
        vm.inner.media_query_list_registry.len(),
        0,
        "an unbound-created listener-only MQL (document=None) must be collected, \
         not rooted by a None==None deliverability match",
    );
}
