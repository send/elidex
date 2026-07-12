//! Custom-Elements registry lifetime across the BATCH-BIND unbind
//! (`#11-per-batch-unbind-document-lifetime-state`) — carved out of
//! `tests_custom_elements.rs` (Codex #459 R1 P3: focused sibling module rather
//! than growing the 1000+ line file). Covers WHATWG HTML §4.13.4/§4.13.5:
//! a `customElements.define()` survives a per-turn `Vm::unbind` and is cleared
//! only at `Vm::teardown_document`.

#![cfg(feature = "engine")]

use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;

fn build_doc(dom: &mut EcsDom) -> elidex_ecs::Entity {
    let doc = dom.create_document_root();
    let html = dom.create_element("html", elidex_ecs::Attributes::default());
    let body = dom.create_element("body", elidex_ecs::Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, body));
    doc
}

fn get_defined(vm: &mut Vm, name: &str) -> String {
    let expr = format!("customElements.get('{name}') !== undefined ? 'defined' : 'gone';");
    let JsValue::String(sid) = vm.eval(&expr).expect("get failed") else {
        panic!("expected string from customElements.get probe")
    };
    vm.inner.strings.get_utf8(sid)
}

/// Re-bind the VM to the same fixture — the start of a fresh BATCH-BIND
/// bracket (centralizes the `unsafe` for the multi-batch survival tests).
fn rebind(vm: &mut Vm, session: &mut SessionCore, dom: &mut EcsDom, doc: elidex_ecs::Entity) {
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(vm, session, dom, doc);
    }
}

/// A `customElements.define()` in one script batch must survive the batch's
/// per-turn (BATCH-BIND) `unbind` and be visible to a later batch — the
/// authoritative registry is document-lifetime, cleared only at
/// `teardown_document` (HTML §4.13.4/§4.13.5).
#[test]
fn custom_element_definition_survives_per_turn_unbind() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    // Batch A: define, then end the batch (per-turn unbind).
    vm.eval("customElements.define('my-el', class extends HTMLElement {});")
        .expect("define failed");
    vm.unbind();

    // Batch B: a fresh bracket — the definition must still be present.
    rebind(&mut vm, &mut session, &mut dom, doc);
    assert_eq!(
        get_defined(&mut vm, "my-el"),
        "defined",
        "define() must survive one per-turn unbind"
    );

    // A second per-turn unbind + rebind still preserves it (≥2 turns).
    vm.unbind();
    rebind(&mut vm, &mut session, &mut dom, doc);
    assert_eq!(
        get_defined(&mut vm, "my-el"),
        "defined",
        "define() must survive ≥2 per-turn unbinds"
    );
    vm.unbind();
}

/// The `CustomElementRegistry` WRAPPER is realm-structural — its identity must
/// survive the per-turn unbind (Codex #459 R3-1; the teardown half is R4).
/// `globalThis.customElements` is installed ONCE as an eager data property
/// (`register_globals` at `Vm::new`, never re-run per bind), so if the per-turn
/// `unbind` dropped `custom_element_registry_instance` a fresh access would mint
/// a SECOND wrapper — and the page's own `customElements` would then be
/// classified `Foreign`, making
/// `createElement(x, { customElementRegistry: customElements })` throw
/// NotSupportedError instead of the document no-op. The wrapper must survive
/// the per-turn unbind in lockstep with the surviving registry data.
#[test]
fn custom_element_registry_wrapper_identity_survives_per_turn_unbind() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    // Batch A ends with a per-turn unbind (the CE registry wrapper must survive).
    vm.unbind();

    // Batch B: passing the page's own `customElements` as the creation-options
    // registry must be the document no-op, NOT a Foreign rejection.
    rebind(&mut vm, &mut session, &mut dom, doc);
    let JsValue::String(sid) = vm
        .eval(
            "try { document.createElement('div', { customElementRegistry: customElements }); \
                   'ok'; } catch (e) { e.name; }",
        )
        .expect("createElement probe failed")
    else {
        panic!("expected string from createElement probe")
    };
    assert_eq!(
        vm.inner.strings.get_utf8(sid),
        "ok",
        "the page's own customElements must stay the document registry across a per-turn unbind",
    );
    vm.unbind();
}

/// A `whenDefined(name)` promise minted in one batch must SURVIVE the per-turn
/// unbind and be resolved by a `define(name)` in a LATER batch (HTML §4.13.4
/// whenDefined + define step 19 "resolve promise / remove map entry"). The
/// `ce_when_defined_promises` map is document-lifetime; pre-survivor-set the
/// per-turn unbind cleared it, orphaning the pending promise so it hung forever
/// — surviving is the spec-correct fix (Codex #459 R5 CE-audit coverage gap).
#[test]
fn when_defined_promise_survives_unbind_and_resolves_on_later_define() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    // Batch A: request whenDefined for a not-yet-defined name (promise pending).
    vm.eval(
        "globalThis.__wd = 'pending'; \
         customElements.whenDefined('my-el').then(() => { globalThis.__wd = 'resolved'; });",
    )
    .expect("whenDefined failed");
    let JsValue::String(sid) = vm.eval("globalThis.__wd").expect("read") else {
        panic!("expected string")
    };
    assert_eq!(vm.inner.strings.get_utf8(sid), "pending", "not defined yet");
    vm.unbind();

    // Batch B: define in a later batch resolves the SURVIVING promise (its
    // `.then` runs on the eval-tail microtask drain).
    rebind(&mut vm, &mut session, &mut dom, doc);
    vm.eval("customElements.define('my-el', class extends HTMLElement {});")
        .expect("define failed");
    let JsValue::String(sid) = vm.eval("globalThis.__wd").expect("read") else {
        panic!("expected string")
    };
    assert_eq!(
        vm.inner.strings.get_utf8(sid),
        "resolved",
        "whenDefined promise minted before the unbind must resolve on a later-batch define",
    );
    vm.unbind();
}

/// Document teardown (navigation / engine drop) releases the CE registry.
#[test]
fn custom_element_registry_cleared_on_teardown_document() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval("customElements.define('my-el', class extends HTMLElement {});")
        .expect("define failed");

    // teardown_document clears the registry (then unbinds).
    vm.teardown_document();

    // A fresh bind sees an empty registry.
    rebind(&mut vm, &mut session, &mut dom, doc);
    assert_eq!(
        get_defined(&mut vm, "my-el"),
        "gone",
        "teardown_document must clear the CE registry"
    );
    vm.unbind();
}

/// The CE registry WRAPPER is realm-structural — it must survive
/// `teardown_document` too, NOT only the per-turn unbind (Codex #459 R4,
/// correcting R3-1's teardown over-reach). `teardown_document` clears the
/// registry DATA but the `globalThis.customElements` singleton wrapper (an
/// install-once data property) persists; dropping its cached id would re-mint a
/// second wrapper and misclassify the page's own `customElements` as `Foreign`,
/// so `createElement(x, { customElementRegistry: customElements })` would throw
/// after a teardown+rebind even though the registry is merely empty. (Reachable
/// only via the unit harness — production always drops the `Vm` at teardown —
/// but pins the realm-structural classification against regression.)
#[test]
fn custom_element_registry_wrapper_identity_survives_teardown_document() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval("customElements.define('my-el', class extends HTMLElement {});")
        .expect("define failed");

    // Document destruction clears the registry DATA (but not the wrapper).
    vm.teardown_document();

    // Fresh bind: the registry is empty AND the page's own `customElements`
    // still classifies as the document registry (creation-options no-op, not a
    // Foreign NotSupportedError).
    rebind(&mut vm, &mut session, &mut dom, doc);
    assert_eq!(
        get_defined(&mut vm, "my-el"),
        "gone",
        "registry DATA cleared"
    );
    let JsValue::String(sid) = vm
        .eval(
            "try { document.createElement('div', { customElementRegistry: customElements }); \
                   'ok'; } catch (e) { e.name; }",
        )
        .expect("createElement probe failed")
    else {
        panic!("expected string from createElement probe")
    };
    assert_eq!(
        vm.inner.strings.get_utf8(sid),
        "ok",
        "customElements must stay the document registry across teardown+rebind (realm-structural wrapper)",
    );
    vm.unbind();
}
