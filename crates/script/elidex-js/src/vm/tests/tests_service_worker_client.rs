//! `navigator.serviceWorker` client tests (window realm; D-19 PR-3).
//!
//! The `register()` / `update()` / `unregister()` promises settle via the
//! inbound `Vm::deliver_sw_client_update` back-channel (DR-B'), NOT at the eval
//! tail.  So the pattern is: eval `register().then(...)` (leaving the promise
//! pending), drive the matching `SwClientUpdate`, then read the
//! `.then`-written `globalThis.__*` slot — the deliver runs its own trailing
//! microtask checkpoint, so the reaction has fired by the time it returns.
//! Synchronously-resolved ops (`getRegistration` / `ready`) settle at the eval
//! tail (`Vm::eval` drains microtasks).

#![cfg(feature = "engine")]

use elidex_api_sw::{SwClientRequest, SwClientUpdate, SwState, SwWorkerSnapshot, UpdateViaCache};
use elidex_ecs::{Attributes, EcsDom};
use elidex_script_session::SessionCore;
use url::Url;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;

const BASE: &str = "https://example.com/app/page.html";
const SCOPE: &str = "https://example.com/app/";
const SCRIPT: &str = "https://example.com/app/sw.js";

fn build_min_fixture(dom: &mut EcsDom) -> elidex_ecs::Entity {
    let doc = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    let body = dom.create_element("body", Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, body));
    doc
}

struct UnbindOnDrop<'a>(&'a mut Vm);
impl Drop for UnbindOnDrop<'_> {
    fn drop(&mut self) {
        self.0.unbind();
    }
}

/// Bind a window-realm VM at a secure `https://example.com/app/` base (so
/// `register()` validation passes) and run `f`.
fn with_vm<F, R>(f: F) -> R
where
    F: FnOnce(&mut Vm) -> R,
{
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_min_fixture(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.inner.navigation.current_url = Url::parse(BASE).unwrap();
    let guard = UnbindOnDrop(&mut vm);
    f(guard.0)
}

fn url(s: &str) -> Url {
    Url::parse(s).unwrap()
}

fn worker(state: SwState) -> SwWorkerSnapshot {
    SwWorkerSnapshot {
        script_url: SCRIPT.to_owned(),
        state,
    }
}

/// Deliver a successful `Registered` for the standard scope carrying a worker
/// in `state`.
fn deliver_registered(vm: &mut Vm, state: SwState) {
    vm.deliver_sw_client_update(SwClientUpdate::Registered {
        scope: url(SCOPE),
        success: true,
        error: None,
        worker: Some(worker(state)),
        update_via_cache: UpdateViaCache::default(),
    });
}

fn eval_bool(vm: &mut Vm, src: &str) -> bool {
    match vm.eval(src).unwrap() {
        JsValue::Boolean(b) => b,
        other => panic!("expected bool, got {other:?} for `{src}`"),
    }
}

fn eval_string(vm: &mut Vm, src: &str) -> String {
    match vm.eval(src).unwrap() {
        JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string, got {other:?} for `{src}`"),
    }
}

/// Eval `register_expr` (a register() call) and report the rejection's
/// exception `name` (or `"resolved"` / `"none"`).
fn reject_name(vm: &mut Vm, register_expr: &str) -> String {
    let src = format!(
        "globalThis.__err = 'none'; ({register_expr}).then(\
            () => {{ globalThis.__err = 'resolved'; }}, \
            e => {{ globalThis.__err = (e && e.name) || String(e); }});"
    );
    vm.eval(&src).unwrap();
    eval_string(vm, "globalThis.__err")
}

// ---------------------------------------------------------------------------
// register() — pending → deliver
// ---------------------------------------------------------------------------

#[test]
fn register_resolves_with_registration_on_deliver() {
    with_vm(|vm| {
        vm.eval(
            "globalThis.__state = 'pending'; \
             navigator.serviceWorker.register('sw.js').then(r => { \
                 globalThis.__reg = r; \
                 globalThis.__state = r.installing ? r.installing.state : 'no-worker'; \
             });",
        )
        .unwrap();

        // The request was staged for the coordinator (D-26 forwards it), with
        // the canonical resolved script + default scope.
        let reqs = vm.drain_sw_client_requests();
        match reqs.as_slice() {
            [SwClientRequest::Register {
                script_url, scope, ..
            }] => {
                assert_eq!(script_url, SCRIPT);
                assert_eq!(scope, SCOPE);
            }
            other => panic!("expected one Register, got {other:?}"),
        }

        // Pending until the deliver, then resolved with the installing worker (F1).
        assert_eq!(eval_string(vm, "globalThis.__state"), "pending");
        deliver_registered(vm, SwState::Installing);
        assert_eq!(eval_string(vm, "globalThis.__state"), "installing");
    });
}

#[test]
fn register_failure_rejects_with_typed_exception() {
    with_vm(|vm| {
        // (i) bad scheme (data:) → TypeError [Start Register].
        assert_eq!(
            reject_name(
                vm,
                "navigator.serviceWorker.register('data:text/javascript,1')"
            ),
            "TypeError"
        );
        // (ii) cross-origin script → SecurityError [Register].
        assert_eq!(
            reject_name(
                vm,
                "navigator.serviceWorker.register('https://cdn.example.com/sw.js')"
            ),
            "SecurityError"
        );
        // (iii) scope outside the script directory → SecurityError [Update].
        assert_eq!(
            reject_name(
                vm,
                "navigator.serviceWorker.register('sw.js', { scope: '/' })"
            ),
            "SecurityError"
        );
        // (iv) non-secure context → SecurityError [Register].
        vm.inner.navigation.current_url = url("http://example.com/page.html");
        assert_eq!(
            reject_name(
                vm,
                "navigator.serviceWorker.register('http://example.com/sw.js')"
            ),
            "SecurityError"
        );
    });
}

#[test]
fn concurrent_register_same_scope_all_resolve() {
    with_vm(|vm| {
        vm.eval(
            "globalThis.__n = 0; \
             navigator.serviceWorker.register('sw.js').then(() => { globalThis.__n++; }); \
             navigator.serviceWorker.register('sw.js').then(() => { globalThis.__n++; });",
        )
        .unwrap();
        vm.drain_sw_client_requests();
        deliver_registered(vm, SwState::Installing);
        // One deliver settles every same-scope waiter (D2).
        assert_eq!(eval_string(vm, "String(globalThis.__n)"), "2");
    });
}

#[test]
fn register_update_via_cache_round_trips() {
    with_vm(|vm| {
        vm.eval(
            "navigator.serviceWorker.register('sw.js', { updateViaCache: 'none' }) \
                .then(r => { globalThis.__reg = r; });",
        )
        .unwrap();
        // The requested updateViaCache is carried in the outbound request.
        let reqs = vm.drain_sw_client_requests();
        match reqs.as_slice() {
            [SwClientRequest::Register {
                update_via_cache, ..
            }] => assert_eq!(*update_via_cache, UpdateViaCache::None),
            other => panic!("expected one Register, got {other:?}"),
        }
        // The deliver carries it back → the getter reflects it.
        vm.deliver_sw_client_update(SwClientUpdate::Registered {
            scope: url(SCOPE),
            success: true,
            error: None,
            worker: Some(worker(SwState::Activated)),
            update_via_cache: UpdateViaCache::None,
        });
        assert_eq!(eval_string(vm, "globalThis.__reg.updateViaCache"), "none");
    });
}

// ---------------------------------------------------------------------------
// Identity (per-realm object maps)
// ---------------------------------------------------------------------------

#[test]
fn registration_and_worker_identity() {
    with_vm(|vm| {
        vm.eval("navigator.serviceWorker.register('sw.js').then(r => { globalThis.__reg = r; });")
            .unwrap();
        vm.drain_sw_client_requests();
        deliver_registered(vm, SwState::Activated);

        // reg === getRegistration() (D1 — the §3.2.1 service worker registration object map).
        vm.eval(
            "globalThis.__sameReg = false; \
             navigator.serviceWorker.getRegistration('page.html').then(r => { \
                 globalThis.__sameReg = (r === globalThis.__reg); });",
        )
        .unwrap();
        assert!(eval_bool(vm, "globalThis.__sameReg"));

        // reg.active === controller (D1 — both intern the same worker by scope).
        vm.deliver_sw_client_update(SwClientUpdate::ControllerSet {
            scope: Some(url(SCOPE)),
        });
        assert!(eval_bool(
            vm,
            "globalThis.__reg.active === navigator.serviceWorker.controller"
        ));
        assert_eq!(
            eval_string(vm, "globalThis.__reg.active.state"),
            "activated"
        );
    });
}

// Codex #459 R5-#1: a cross-batch update to a DIFFERENT script must refresh the
// Scope-keyed `ServiceWorker` wrapper so `reg.active.scriptURL` reflects the new
// script (SW §3.1.1 — a new script is a new worker object with its own immutable
// scriptURL). R2 retains the wrapper across the per-turn unbind, so without the
// script-URL-change evict in `deliver_registered`, `worker_object` returns the
// cached wrapper and the frozen `scriptURL` own-prop stays stale.
#[test]
fn cross_batch_script_update_refreshes_worker_script_url() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_min_fixture(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.inner.navigation.current_url = url(BASE);

    // Batch 1: register + activate with the standard script (SCRIPT = sw.js).
    vm.eval("navigator.serviceWorker.register('sw.js').then(r => { globalThis.reg = r; });")
        .unwrap();
    vm.drain_sw_client_requests();
    deliver_registered(&mut vm, SwState::Activated);
    assert_eq!(
        eval_string(&mut vm, "globalThis.reg.active.scriptURL"),
        SCRIPT
    );

    // Cross-batch: end the batch, rebind, deliver an update carrying a DIFFERENT
    // script for the SAME scope.
    vm.unbind();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.inner.navigation.current_url = url(BASE);
    let new_script = "https://example.com/app/sw-v2.js";
    vm.deliver_sw_client_update(SwClientUpdate::Registered {
        scope: url(SCOPE),
        success: true,
        error: None,
        worker: Some(SwWorkerSnapshot {
            script_url: new_script.to_owned(),
            state: SwState::Activated,
        }),
        update_via_cache: UpdateViaCache::default(),
    });
    assert_eq!(
        eval_string(&mut vm, "globalThis.reg.active.scriptURL"),
        new_script,
        "a cross-batch script update must refresh reg.active.scriptURL (wrapper re-minted)",
    );
    vm.unbind();
}

#[test]
fn worker_identity_survives_state_transition() {
    with_vm(|vm| {
        vm.eval("navigator.serviceWorker.register('sw.js').then(r => { globalThis.__reg = r; });")
            .unwrap();
        vm.drain_sw_client_requests();
        deliver_registered(vm, SwState::Installing);

        // Capture the installing worker; after a transition it is the SAME
        // object with the new state (D3 — `#update-worker-state` mutates in place).
        vm.eval("globalThis.__w = globalThis.__reg.installing;")
            .unwrap();
        assert_eq!(eval_string(vm, "globalThis.__w.state"), "installing");
        vm.deliver_sw_client_update(SwClientUpdate::StateChanged {
            scope: url(SCOPE),
            state: SwState::Activated,
        });
        assert!(eval_bool(vm, "globalThis.__w === globalThis.__reg.active"));
        assert_eq!(eval_string(vm, "globalThis.__w.state"), "activated");
    });
}

// ---------------------------------------------------------------------------
// ready / controller / construction-init seed
// ---------------------------------------------------------------------------

#[test]
fn ready_is_same_promise_and_resolves_on_active() {
    with_vm(|vm| {
        // `[SameObject]` — the same coalesced promise on every access.
        assert!(eval_bool(
            vm,
            "navigator.serviceWorker.ready === navigator.serviceWorker.ready"
        ));
        vm.eval(
            "globalThis.__ready = false; \
             navigator.serviceWorker.ready.then(r => { \
                 globalThis.__ready = !!(r && r.active && r.active.state === 'activated'); });",
        )
        .unwrap();
        vm.eval("navigator.serviceWorker.register('sw.js');")
            .unwrap();
        vm.drain_sw_client_requests();
        deliver_registered(vm, SwState::Activated);
        assert!(eval_bool(vm, "globalThis.__ready"));
    });
}

#[test]
fn ready_lazily_resolves_for_activating_worker() {
    with_vm(|vm| {
        // A worker reaches the `active` slot at `activating` (SW §3.2.4), so a
        // `.ready` accessed AFTER that must resolve via the lazy
        // `active_registration` path — not just the runtime deliver path.
        vm.eval("navigator.serviceWorker.register('sw.js');")
            .unwrap();
        vm.drain_sw_client_requests();
        deliver_registered(vm, SwState::Activating);
        vm.eval(
            "globalThis.__ready = false; \
             navigator.serviceWorker.ready.then(() => { globalThis.__ready = true; });",
        )
        .unwrap();
        assert!(eval_bool(vm, "globalThis.__ready"));
    });
}

#[test]
fn seed_controller_visible_before_deliver() {
    with_vm(|vm| {
        // A page controlled AT navigation — seeded, no runtime deliver (F2).
        vm.seed_sw_client(
            Some(url(SCOPE)),
            &[(url(SCOPE), worker(SwState::Activated))],
        );
        assert!(eval_bool(vm, "navigator.serviceWorker.controller !== null"));
        assert_eq!(
            eval_string(vm, "navigator.serviceWorker.controller.state"),
            "activated"
        );
        vm.eval(
            "globalThis.__has = false; \
             navigator.serviceWorker.getRegistration().then(r => { globalThis.__has = !!r; });",
        )
        .unwrap();
        assert!(eval_bool(vm, "globalThis.__has"));
    });
}

// ---------------------------------------------------------------------------
// statechange / updatefound
// ---------------------------------------------------------------------------

#[test]
fn state_changed_fires_statechange() {
    with_vm(|vm| {
        vm.eval("navigator.serviceWorker.register('sw.js').then(r => { globalThis.__reg = r; });")
            .unwrap();
        vm.drain_sw_client_requests();
        deliver_registered(vm, SwState::Installing);
        vm.eval(
            "globalThis.__sc = 0; globalThis.__uf = 0; \
             globalThis.__reg.installing.onstatechange = () => { globalThis.__sc++; }; \
             globalThis.__reg.onupdatefound = () => { globalThis.__uf++; };",
        )
        .unwrap();
        vm.deliver_sw_client_update(SwClientUpdate::StateChanged {
            scope: url(SCOPE),
            state: SwState::Installed,
        });
        // statechange fired; no updatefound (not a fresh installing worker).
        assert_eq!(eval_string(vm, "String(__sc)"), "1");
        assert_eq!(eval_string(vm, "String(__uf)"), "0");
    });
}

#[test]
fn new_installing_worker_fires_updatefound() {
    with_vm(|vm| {
        vm.eval("navigator.serviceWorker.register('sw.js').then(r => { globalThis.__reg = r; });")
            .unwrap();
        vm.drain_sw_client_requests();
        deliver_registered(vm, SwState::Activated);
        vm.eval(
            "globalThis.__uf = 0; globalThis.__reg.onupdatefound = () => { globalThis.__uf++; };",
        )
        .unwrap();
        // An update brings a new installing worker (prev was activated).
        vm.deliver_sw_client_update(SwClientUpdate::StateChanged {
            scope: url(SCOPE),
            state: SwState::Installing,
        });
        assert_eq!(eval_string(vm, "String(__uf)"), "1");
    });
}

// ---------------------------------------------------------------------------
// controllerchange / message
// ---------------------------------------------------------------------------

#[test]
fn controller_set_ignored_when_out_of_scope() {
    with_vm(|vm| {
        // The shell broadcasts ControllerSet to all same-origin tabs; a tab with
        // no registration for that scope must NOT fire controllerchange or adopt
        // a controller it isn't controlled by.
        vm.eval(
            "globalThis.__cc = 0; \
             navigator.serviceWorker.oncontrollerchange = () => { globalThis.__cc++; };",
        )
        .unwrap();
        vm.deliver_sw_client_update(SwClientUpdate::ControllerSet {
            scope: Some(url("https://example.com/other/")),
        });
        assert_eq!(eval_string(vm, "String(__cc)"), "0");
        assert!(eval_bool(vm, "navigator.serviceWorker.controller === null"));
    });
}

#[test]
fn controller_set_ignored_for_non_controlling_registration() {
    with_vm(|vm| {
        // A registration THIS realm knows of, but whose scope (/other/) does not
        // contain the document URL (/app/page.html), must not become the
        // controller — `contains_key` alone is insufficient with multiple
        // same-origin registrations (Copilot).
        vm.deliver_sw_client_update(SwClientUpdate::Registered {
            scope: url("https://example.com/other/"),
            success: true,
            error: None,
            worker: Some(SwWorkerSnapshot {
                script_url: "https://example.com/other/sw.js".to_owned(),
                state: SwState::Activated,
            }),
            update_via_cache: UpdateViaCache::default(),
        });
        vm.eval(
            "globalThis.__cc = 0; \
             navigator.serviceWorker.oncontrollerchange = () => { globalThis.__cc++; };",
        )
        .unwrap();
        vm.deliver_sw_client_update(SwClientUpdate::ControllerSet {
            scope: Some(url("https://example.com/other/")),
        });
        assert_eq!(eval_string(vm, "String(__cc)"), "0");
        assert!(eval_bool(vm, "navigator.serviceWorker.controller === null"));
    });
}

#[test]
fn message_enables_queue_via_onmessage_and_flushes_buffer() {
    with_vm(|vm| {
        // A message arriving before any listener is buffered (queue disabled).
        vm.deliver_sw_client_update(SwClientUpdate::Message {
            data: "\"first\"".to_owned(),
            source_scope: url(SCOPE),
        });
        // Adding a `message` listener enables the queue (SW §3.4.6) — the next
        // deliver latches it, flushes the buffered message, then fires the new one.
        vm.eval(
            "globalThis.__msgs = []; \
             navigator.serviceWorker.onmessage = e => { globalThis.__msgs.push(e.data); };",
        )
        .unwrap();
        vm.deliver_sw_client_update(SwClientUpdate::Message {
            data: "\"second\"".to_owned(),
            source_scope: url(SCOPE),
        });
        assert_eq!(
            eval_string(vm, "globalThis.__msgs.join(',')"),
            "first,second"
        );
    });
}

// ---------------------------------------------------------------------------
// getRegistration / unregister / postMessage
// ---------------------------------------------------------------------------

#[test]
fn get_registration_scope_match_and_miss() {
    with_vm(|vm| {
        vm.eval("navigator.serviceWorker.register('sw.js').then(r => { globalThis.__reg = r; });")
            .unwrap();
        vm.drain_sw_client_requests();
        deliver_registered(vm, SwState::Activated);

        vm.eval(
            "globalThis.__hit = false; \
             navigator.serviceWorker.getRegistration('https://example.com/app/sub/x').then(r => { \
                 globalThis.__hit = (r === globalThis.__reg); });",
        )
        .unwrap();
        assert!(eval_bool(vm, "globalThis.__hit"));

        vm.eval(
            "globalThis.__miss = false; \
             navigator.serviceWorker.getRegistration('https://example.com/other/y').then(r => { \
                 globalThis.__miss = (r === undefined); });",
        )
        .unwrap();
        assert!(eval_bool(vm, "globalThis.__miss"));
    });
}

#[test]
fn unregister_resolves_and_removes_registration() {
    with_vm(|vm| {
        vm.eval("navigator.serviceWorker.register('sw.js').then(r => { globalThis.__reg = r; });")
            .unwrap();
        vm.drain_sw_client_requests();
        deliver_registered(vm, SwState::Activated);

        vm.eval(
            "globalThis.__un = 'pending'; \
             globalThis.__reg.unregister().then(b => { globalThis.__un = String(b); });",
        )
        .unwrap();
        let reqs = vm.drain_sw_client_requests();
        assert!(matches!(
            reqs.as_slice(),
            [SwClientRequest::Unregister { .. }]
        ));
        assert_eq!(eval_string(vm, "globalThis.__un"), "pending");

        vm.deliver_sw_client_update(SwClientUpdate::Unregistered {
            scope: url(SCOPE),
            success: true,
        });
        assert_eq!(eval_string(vm, "globalThis.__un"), "true");

        // getRegistration now misses (the registry entry was removed).
        vm.eval(
            "globalThis.__gone = false; \
             navigator.serviceWorker.getRegistration().then(r => { \
                 globalThis.__gone = (r === undefined); });",
        )
        .unwrap();
        assert!(eval_bool(vm, "globalThis.__gone"));
    });
}

#[test]
fn worker_post_message_stages_request() {
    with_vm(|vm| {
        vm.eval("navigator.serviceWorker.register('sw.js').then(r => { globalThis.__reg = r; });")
            .unwrap();
        vm.drain_sw_client_requests();
        deliver_registered(vm, SwState::Activated);

        vm.eval("globalThis.__reg.active.postMessage({ hello: 'world' });")
            .unwrap();
        let reqs = vm.drain_sw_client_requests();
        match reqs.as_slice() {
            [SwClientRequest::PostMessage { scope, data }] => {
                assert_eq!(scope, SCOPE);
                assert!(data.contains("hello"));
            }
            other => panic!("expected one PostMessage, got {other:?}"),
        }
    });
}

#[test]
fn worker_script_url_is_immutable_across_unregister() {
    with_vm(|vm| {
        vm.eval("navigator.serviceWorker.register('sw.js').then(r => { globalThis.__reg = r; });")
            .unwrap();
        vm.drain_sw_client_requests();
        deliver_registered(vm, SwState::Activated);
        vm.eval("globalThis.__w = globalThis.__reg.active;")
            .unwrap();
        assert_eq!(eval_string(vm, "globalThis.__w.scriptURL"), SCRIPT);

        // Unregister drops the registry entry; the JS-held worker's scriptURL is
        // immutable (SW §3.1.2 — must NOT become "") while `state` becomes
        // `redundant` (SW §3.1.3, mutable).
        vm.eval("globalThis.__reg.unregister();").unwrap();
        vm.drain_sw_client_requests();
        vm.deliver_sw_client_update(SwClientUpdate::Unregistered {
            scope: url(SCOPE),
            success: true,
        });
        assert_eq!(eval_string(vm, "globalThis.__w.scriptURL"), SCRIPT);
        assert_eq!(eval_string(vm, "globalThis.__w.state"), "redundant");
    });
}

// ---------------------------------------------------------------------------
// GC + unbind
// ---------------------------------------------------------------------------

#[test]
fn pending_register_survives_gc() {
    with_vm(|vm| {
        vm.eval(
            "globalThis.__got = false; \
             navigator.serviceWorker.register('sw.js').then(r => { globalThis.__got = !!r; });",
        )
        .unwrap();
        vm.drain_sw_client_requests();
        // The pending promise is reachable ONLY through the force-marked
        // `pending_registration_promises` list — a GC here must not sweep it.
        vm.inner.collect_garbage();
        deliver_registered(vm, SwState::Installing);
        assert!(eval_bool(vm, "globalThis.__got"));
    });
}

// `#11-per-batch-unbind-document-lifetime-state`: the `navigator.serviceWorker`
// client state is document-lifetime, so a per-turn (BATCH-BIND) `unbind` must
// PRESERVE it — a `register()` staged in a script batch must survive the
// batch's unbind so the out-of-bracket event-loop drain still sees it, and the
// client registry a page reads across batches must stay stable.  (Was cleared
// per-turn pre-#11-per-batch-unbind; the clear MOVED to `teardown_document`.)
#[test]
fn sw_client_state_survives_per_turn_unbind() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_min_fixture(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.inner.navigation.current_url = url(BASE);

    // Register + deliver → an established registration in the client registry.
    vm.eval("navigator.serviceWorker.register('sw.js');")
        .unwrap();
    vm.drain_sw_client_requests();
    deliver_registered(&mut vm, SwState::Installing);
    assert!(!vm.inner.sw_registrations.is_empty());

    // R4-#5 core: a register() staged in this batch, NOT yet drained, must
    // survive the batch unbind so the out-of-bracket drain finds it.
    vm.eval("navigator.serviceWorker.register('sw2.js');")
        .unwrap();
    assert!(
        !vm.inner.sw_client_outgoing.is_empty(),
        "register() must stage an outbound request"
    );
    vm.unbind();
    assert!(
        !vm.inner.sw_client_outgoing.is_empty(),
        "staged register() must survive the per-turn unbind (R4-#5)"
    );
    assert!(
        !vm.inner.sw_registrations.is_empty(),
        "the client registry must survive the per-turn unbind"
    );

    // A second per-turn unbind (rebind between) also preserves the state.
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.unbind();
    assert!(!vm.inner.sw_registrations.is_empty());
    assert!(!vm.inner.sw_client_outgoing.is_empty());

    // The out-of-bracket drain (unbound) still sees the surviving request.
    let drained = vm.drain_sw_client_requests();
    assert!(
        !drained.is_empty(),
        "the surviving staged register() reaches the event-loop drain"
    );
}

// Document teardown (navigation / engine drop) releases the SW client state.
#[test]
fn teardown_document_clears_sw_client_state() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_min_fixture(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.inner.navigation.current_url = url(BASE);

    vm.eval("navigator.serviceWorker.register('sw.js');")
        .unwrap();
    vm.drain_sw_client_requests();
    deliver_registered(&mut vm, SwState::Installing);
    vm.eval("navigator.serviceWorker.register('sw.js');")
        .unwrap();
    vm.drain_sw_client_requests();
    assert!(!vm.inner.pending_registration_promises.is_empty());

    // teardown_document clears every SW client side-store (then unbinds).
    vm.teardown_document();
    assert!(vm.inner.pending_registration_promises.is_empty());
    assert!(vm.inner.pending_unregister_promises.is_empty());
    assert!(vm.inner.sw_registrations.is_empty());
    assert!(vm.inner.sw_registration_states.is_empty());
    assert!(vm.inner.service_worker_states.is_empty());
    assert!(vm.inner.sw_ready_promise.is_none());
    assert!(vm.inner.sw_controller_scope.is_none());
}

// `#11-per-batch-unbind-document-lifetime-state` / Codex R1 P1: a JS-retained
// `ServiceWorkerRegistration` wrapper must stay a VALID RECEIVER across per-turn
// unbinds. Its wrapper-brand entry (`sw_registration_states`) is document-
// lifetime (survives unbind, cleared at `teardown_document`); the GC sweep
// prunes only a COLLECTED wrapper's entry, so a retained wrapper keeps its
// brand. Clearing the brand per-turn would make `reg.scope` / `reg.unregister()`
// an illegal receiver after the first unbind.
#[test]
fn retained_sw_registration_wrapper_survives_per_turn_unbind() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_min_fixture(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.inner.navigation.current_url = url(BASE);

    // Retain the registration wrapper in JS across batches.
    vm.eval("navigator.serviceWorker.register('sw.js').then(r => { globalThis.reg = r; });")
        .unwrap();
    vm.drain_sw_client_requests();
    deliver_registered(&mut vm, SwState::Installing);
    assert!(
        !vm.inner.sw_registration_states.is_empty(),
        "resolving register() should mint the registration wrapper + its brand entry"
    );

    // ≥2 per-turn unbinds (rebind between). The retained wrapper's brand must
    // survive so it stays a valid receiver (the brand-check in
    // `require_registration_scope`).
    for _ in 0..2 {
        vm.unbind();
        #[allow(unsafe_code)]
        unsafe {
            bind_vm(&mut vm, &mut session, &mut dom, doc);
        }
    }
    vm.eval(
        "globalThis.__ok = false; \
         try { globalThis.__ok = typeof globalThis.reg.scope === 'string' \
               && globalThis.reg.scope.length > 0; } catch (e) {}",
    )
    .unwrap();
    assert!(
        eval_bool(&mut vm, "globalThis.__ok"),
        "a retained ServiceWorkerRegistration must stay a valid receiver across per-turn unbinds"
    );
    // ...and its backing data survived too.
    assert!(!vm.inner.sw_registrations.is_empty());

    // Codex #459 R2: `reg === getRegistration()` must ALSO hold across the
    // unbinds — the Scope-owned wrapper survived the `wrapper_store.retain`, so
    // no SECOND object is minted for the same scope (SW §3.2.1 service worker
    // registration object map).
    vm.eval(
        "globalThis.__same = false; \
         navigator.serviceWorker.getRegistration().then(r => { \
             globalThis.__same = (r === globalThis.reg); });",
    )
    .unwrap();
    assert!(
        eval_bool(&mut vm, "globalThis.__same"),
        "reg === getRegistration() must hold across per-turn unbinds (no duplicate wrapper)"
    );
}

// Codex #459 R3-2: the Scope-owned registration/worker WRAPPERS that survive a
// per-turn `unbind` must be DROPPED at `teardown_document`, in lockstep with
// their data + brand rows. Otherwise a later same-`Vm` re-`register()` of the
// same scope hits `intern_wrapper`'s cached `ObjectId`, SKIPS the allocation
// closure that repopulates `sw_registration_states` / `service_worker_states`,
// and returns a registration that immediately fails its own brand check.
#[test]
fn teardown_document_drops_sw_registration_wrapper_so_reregister_is_valid() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_min_fixture(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.inner.navigation.current_url = url(BASE);

    // First document: register the scope so its registration/worker wrappers are
    // interned into `wrapper_store` (+ their brand rows).
    vm.eval("navigator.serviceWorker.register('sw.js').then(r => { globalThis.reg = r; });")
        .unwrap();
    vm.drain_sw_client_requests();
    deliver_registered(&mut vm, SwState::Activated);
    assert!(!vm.inner.sw_registration_states.is_empty());

    // Document destruction drops the whole identity unit — data, brand, AND the
    // Scope-keyed `wrapper_store` entries.
    vm.teardown_document();
    assert!(vm.inner.sw_registrations.is_empty());
    assert!(vm.inner.sw_registration_states.is_empty());
    assert!(vm.inner.service_worker_states.is_empty());

    // Fresh document on the SAME `Vm`: re-register the SAME scope. The wrapper
    // must be a freshly-allocated valid receiver (brand rows repopulated by the
    // alloc closure), NOT the stale cached `ObjectId` whose closure was skipped.
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.inner.navigation.current_url = url(BASE);
    vm.eval("navigator.serviceWorker.register('sw.js').then(r => { globalThis.reg2 = r; });")
        .unwrap();
    vm.drain_sw_client_requests();
    deliver_registered(&mut vm, SwState::Activated);
    assert!(
        !vm.inner.sw_registration_states.is_empty(),
        "re-register after teardown must repopulate the wrapper-brand rows (alloc closure ran)",
    );
    vm.eval(
        "globalThis.__ok = false; \
         try { globalThis.__ok = typeof globalThis.reg2.scope === 'string' \
               && globalThis.reg2.scope.length > 0; } catch (e) {}",
    )
    .unwrap();
    assert!(
        eval_bool(&mut vm, "globalThis.__ok"),
        "a re-registered ServiceWorkerRegistration after teardown must be a valid receiver \
         (teardown dropped the stale wrapper_store entry)",
    );
    vm.unbind();
}
