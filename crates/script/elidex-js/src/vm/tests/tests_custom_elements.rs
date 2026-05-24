//! D-17 `#11-custom-elements-vm` — `customElements` thin VM binding tests.
//!
//! Covers WHATWG HTML §4.13.4 (define / get / whenDefined / upgrade) +
//! §4.13.3 lifecycle callbacks (connectedCallback / disconnectedCallback /
//! attributeChangedCallback) + CSS Selectors Level 4 §6.3 (`:defined`).

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

fn run(script: &str) -> String {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let result = vm.eval(script).unwrap();
    let JsValue::String(sid) = result else {
        panic!("expected string, got {result:?}")
    };
    let out = vm.inner.strings.get_utf8(sid);
    vm.unbind();
    out
}

fn run_throws(script: &str) -> String {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let err = vm.eval(script).expect_err("expected an error");
    vm.unbind();
    format!("{err:?}")
}

/// Run `setup` then read back a JS expression. The microtask + CE
/// reaction drains at the END of `eval`, so callbacks that write to
/// globals are not visible inside the SAME eval call's return value.
/// A two-step run lets the `read_expr` evaluate AFTER setup's tail
/// drain has completed.
fn run_then_read(setup: &str, read_expr: &str) -> String {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval(setup).expect("setup failed");
    let result = vm.eval(read_expr).expect("read failed");
    let JsValue::String(sid) = result else {
        panic!("expected string from read, got {result:?}")
    };
    let out = vm.inner.strings.get_utf8(sid);
    vm.unbind();
    out
}

// --- Surface --------------------------------------------------------

#[test]
fn customelements_global_installed() {
    let out = run("typeof customElements === 'object' && customElements !== null ? 'yes' : 'no';");
    assert_eq!(out, "yes");
}

#[test]
fn customelements_registry_constructor_throws() {
    let err = run_throws("new CustomElementRegistry();");
    assert!(
        err.contains("Illegal constructor"),
        "expected Illegal constructor TypeError, got: {err}"
    );
}

#[test]
fn customelements_method_brand_check_define() {
    let err = run_throws("CustomElementRegistry.prototype.define.call({}, 'my-el', function(){});");
    assert!(
        err.contains("Illegal invocation"),
        "expected brand-check TypeError, got: {err}"
    );
}

#[test]
fn customelements_method_brand_check_get() {
    let err = run_throws("CustomElementRegistry.prototype.get.call({}, 'my-el');");
    assert!(
        err.contains("Illegal invocation"),
        "expected brand-check TypeError, got: {err}"
    );
}

// --- define() ------------------------------------------------------

#[test]
fn define_accepts_valid_name() {
    let out = run("customElements.define('my-el', class {}); typeof customElements.get('my-el');");
    assert_eq!(out, "function");
}

#[test]
fn define_invalid_name_throws_syntax_error() {
    // Names without a hyphen are invalid per HTML §4.13.3 `valid custom element name`.
    let err = run_throws("customElements.define('nohyphen', class {});");
    assert!(
        err.contains("SyntaxError") || err.contains("valid custom element name"),
        "expected SyntaxError for invalid name, got: {err}"
    );
}

#[test]
fn define_reserved_name_throws() {
    // `font-face` is reserved per §4.13.3 `valid custom element name`.
    let err = run_throws("customElements.define('font-face', class {});");
    assert!(
        err.contains("SyntaxError") || err.contains("valid custom element name"),
        "expected SyntaxError for reserved name, got: {err}"
    );
}

#[test]
fn define_duplicate_throws_not_supported_error() {
    let err = run_throws(
        "customElements.define('my-el', class {}); \
         customElements.define('my-el', class {});",
    );
    assert!(
        err.contains("NotSupportedError") || err.contains("already been defined"),
        "expected NotSupportedError for duplicate define, got: {err}"
    );
}

#[test]
fn define_non_constructor_throws() {
    let err = run_throws("customElements.define('my-el', 42);");
    assert!(
        err.contains("not a constructor"),
        "expected non-constructor TypeError, got: {err}"
    );
}

#[test]
fn define_arrow_function_throws_not_constructor() {
    // Arrow functions are callable but NOT constructable per ES2020
    // §9.2.1 [[Construct]] — WebIDL CustomElementConstructor requires
    // [[Construct]], so arrow functions must be rejected here.
    // (Copilot R2 #1: prior `is_callable` accepted them.)
    let err = run_throws("customElements.define('my-el', () => {});");
    assert!(
        err.contains("not a constructor"),
        "expected arrow function to be rejected as non-constructor, got: {err}"
    );
}

#[test]
fn define_extends_rejected_as_not_supported() {
    // Customized built-in elements are defer slot `#11-customized-
    // built-in-elements` — reject with NotSupportedError per spec
    // (subset declaration in plan §1).
    let err = run_throws("customElements.define('my-button', class {}, { extends: 'button' });");
    assert!(
        err.contains("NotSupportedError") || err.contains("customized built-in"),
        "expected customized-built-in NotSupportedError, got: {err}"
    );
}

// --- get() ---------------------------------------------------------

#[test]
fn get_returns_undefined_for_unknown() {
    let out = run("typeof customElements.get('never-defined');");
    assert_eq!(out, "undefined");
}

#[test]
fn get_returns_constructor_after_define() {
    let out = run("class MyEl {} \
         customElements.define('my-el', MyEl); \
         (customElements.get('my-el') === MyEl) ? 'same' : 'different';");
    assert_eq!(out, "same");
}

// --- whenDefined() ------------------------------------------------

#[test]
fn whendefined_resolved_for_defined_name() {
    let out = run_then_read(
        "globalThis.MyEl = class {}; \
         customElements.define('my-el', MyEl); \
         globalThis.__result = 'pending'; \
         customElements.whenDefined('my-el').then(function(ctor){ \
             globalThis.__result = (ctor === MyEl) ? 'resolved-same' : 'resolved-different'; \
         });",
        "globalThis.__result;",
    );
    assert_eq!(out, "resolved-same");
}

#[test]
fn whendefined_invalid_name_rejects() {
    let out = run_then_read(
        "globalThis.__status = 'pending'; \
         customElements.whenDefined('nohyphen').then( \
             function(){ globalThis.__status = 'resolved'; }, \
             function(e){ globalThis.__status = 'rejected:' + e.name; } \
         );",
        "globalThis.__status;",
    );
    assert!(
        out.starts_with("rejected"),
        "expected rejected promise for invalid name, got: {out}"
    );
}

#[test]
fn whendefined_pending_resolves_on_define() {
    let out = run_then_read(
        "globalThis.MyEl = class {}; \
         globalThis.__status = 'pending'; \
         customElements.whenDefined('my-el').then(function(ctor){ \
             globalThis.__status = (ctor === MyEl) ? 'resolved-same' : 'resolved-other'; \
         }); \
         customElements.define('my-el', MyEl);",
        "globalThis.__status;",
    );
    assert_eq!(out, "resolved-same");
}

#[test]
fn whendefined_same_promise_for_repeat_calls() {
    let out = run("var p1 = customElements.whenDefined('my-el'); \
         var p2 = customElements.whenDefined('my-el'); \
         (p1 === p2) ? 'same' : 'different';");
    assert_eq!(out, "same");
}

// --- createElement integration ------------------------------------

#[test]
fn createelement_hyphenated_tag_gets_pending_state() {
    // Pre-define: element should be created in Undefined state and
    // queued for upgrade. After define(), the queued upgrade fires
    // and the element transitions to Custom.
    let out = run_then_read(
        "var el = document.createElement('my-el'); \
         globalThis.__upgraded = 'no'; \
         class MyEl { constructor() { globalThis.__upgraded = 'yes'; } } \
         customElements.define('my-el', MyEl);",
        "globalThis.__upgraded;",
    );
    assert_eq!(out, "yes");
}

#[test]
fn createelement_after_define_upgrades_immediately() {
    let out = run_then_read(
        "globalThis.__upgraded = 'no'; \
         class MyEl { constructor() { globalThis.__upgraded = 'yes'; } } \
         customElements.define('my-el', MyEl); \
         document.createElement('my-el');",
        "globalThis.__upgraded;",
    );
    assert_eq!(out, "yes");
}

#[test]
fn createelement_non_hyphenated_not_a_custom_element() {
    // Plain `<div>` should not be classified as a custom element —
    // no state component, no upgrade reactions.
    let out = run("var defined_called = 'no'; \
         class MyEl { constructor() { defined_called = 'yes'; } } \
         customElements.define('my-el', MyEl); \
         var d = document.createElement('div'); \
         defined_called;");
    assert_eq!(out, "no");
}

// --- Lifecycle callbacks ------------------------------------------

#[test]
fn connected_callback_fires_on_append() {
    let out = run_then_read(
        "globalThis.__log = []; \
         class MyEl { connectedCallback() { globalThis.__log.push('connected'); } } \
         customElements.define('my-el', MyEl); \
         var el = document.createElement('my-el'); \
         document.body.appendChild(el);",
        "globalThis.__log.join(',');",
    );
    assert_eq!(out, "connected");
}

#[test]
fn disconnected_callback_fires_on_remove() {
    let out = run_then_read(
        "globalThis.__log = []; \
         class MyEl { disconnectedCallback() { globalThis.__log.push('disconnected'); } } \
         customElements.define('my-el', MyEl); \
         var el = document.createElement('my-el'); \
         document.body.appendChild(el); \
         document.body.removeChild(el);",
        "globalThis.__log.join(',');",
    );
    assert_eq!(out, "disconnected");
}

#[test]
fn attribute_changed_callback_fires_for_observed() {
    let out = run_then_read(
        "globalThis.__log = []; \
         class MyEl { \
             static get observedAttributes() { return ['x']; } \
             attributeChangedCallback(name, oldVal, newVal) { \
                 globalThis.__log.push(name + ':' + (oldVal === null ? 'null' : oldVal) + '->' + (newVal === null ? 'null' : newVal)); \
             } \
         } \
         customElements.define('my-el', MyEl); \
         var el = document.createElement('my-el'); \
         document.body.appendChild(el); \
         el.setAttribute('x', 'hello'); \
         el.setAttribute('y', 'ignored');",
        "globalThis.__log.join(',');",
    );
    assert_eq!(out, "x:null->hello");
}

#[test]
fn attribute_changed_callback_skipped_for_unobserved() {
    let out = run_then_read(
        "globalThis.__fired = 'no'; \
         class MyEl { \
             static get observedAttributes() { return ['x']; } \
             attributeChangedCallback() { globalThis.__fired = 'yes'; } \
         } \
         customElements.define('my-el', MyEl); \
         var el = document.createElement('my-el'); \
         document.body.appendChild(el); \
         el.setAttribute('y', 'value');",
        "globalThis.__fired;",
    );
    assert_eq!(out, "no");
}

// --- upgrade() ----------------------------------------------------

#[test]
fn customelements_upgrade_walks_descendants() {
    let out = run_then_read(
        "globalThis.__log = []; \
         class MyEl { constructor() { globalThis.__log.push('upgraded'); } } \
         var el = document.createElement('my-el'); \
         document.body.appendChild(el); \
         customElements.define('my-el', MyEl);",
        "globalThis.__log.join(',');",
    );
    assert_eq!(out, "upgraded");
}

#[test]
fn customelements_upgrade_handles_constructor_throw_as_failed() {
    // Constructor throws → state should be Failed; subsequent
    // setAttribute on observed attr should not fire callback.
    let out = run_then_read(
        "globalThis.__ctor_attempts = 0; \
         class MyEl { \
             static get observedAttributes() { return ['x']; } \
             constructor() { globalThis.__ctor_attempts++; throw new Error('boom'); } \
             attributeChangedCallback() { globalThis.__ctor_attempts = 99; } \
         } \
         var el = document.createElement('my-el'); \
         try { customElements.define('my-el', MyEl); } catch (e) {} \
         el.setAttribute('x', 'after');",
        "'' + globalThis.__ctor_attempts;",
    );
    // The constructor should run once (during upgrade) then fail. The
    // setAttribute should not trigger attributeChangedCallback because
    // the element is in Failed state.
    assert_eq!(out, "1");
}

#[test]
fn customelements_upgrade_method_requires_node_arg() {
    let err = run_throws("customElements.upgrade();");
    assert!(
        err.contains("argument required") || err.contains("not of type 'Node'"),
        "expected upgrade() arg error, got: {err}"
    );
}

#[test]
fn customelements_ctor_returning_different_object_marks_failed() {
    // HTML §4.13.5 "upgrade an element" step 12.2 — if the
    // constructor returns an object that is not SameValue with the
    // element, mark Failed + throw NotSupportedError. The throw is
    // swallowed at the upgrade-flush boundary (Window.onerror path);
    // the Failed state is observable via subsequent setAttribute on an
    // observed attribute NOT firing attributeChangedCallback (callback
    // gating per HTML §4.13.6 requires CEState::Custom).
    let out = run_then_read(
        "globalThis.__ctor_attempts = 0; \
         globalThis.__cb_attempts = 0; \
         class MyEl { \
             static get observedAttributes() { return ['x']; } \
             constructor() { globalThis.__ctor_attempts++; return {}; } \
             attributeChangedCallback() { globalThis.__cb_attempts++; } \
         } \
         var el = document.createElement('my-el'); \
         customElements.define('my-el', MyEl); \
         el.setAttribute('x', 'after');",
        "globalThis.__ctor_attempts + ':' + globalThis.__cb_attempts;",
    );
    assert_eq!(out, "1:0");
}

// --- F1: within-tree moves fire BOTH disconnected + connected -----

#[test]
fn within_tree_move_fires_both_disconnected_and_connected() {
    // F1 fix: per WHATWG DOM §4.2.3 insertion-steps, a Custom
    // element moved within the connected tree fires
    // disconnectedCallback (from the implicit detach) AND
    // connectedCallback (from the new insert). Both Blink and Gecko
    // fire both callbacks.
    //
    // Uses 3 evals so the initial Connected fires (and drains) BEFORE
    // the log reset, isolating the move's own dispatch.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval(
        "globalThis.__log = []; \
         class MyEl { \
             connectedCallback() { globalThis.__log.push('C'); } \
             disconnectedCallback() { globalThis.__log.push('D'); } \
         } \
         customElements.define('my-el', MyEl); \
         var el = document.createElement('my-el'); \
         document.body.appendChild(el); \
         globalThis.el = el; \
         globalThis.other = document.body.appendChild(document.createElement('div'));",
    )
    .expect("setup failed");
    vm.eval("globalThis.__log = [];").expect("reset failed");
    vm.eval("globalThis.other.appendChild(globalThis.el);")
        .expect("move failed");
    let result = vm.eval("globalThis.__log.join(',');").expect("read failed");
    let JsValue::String(sid) = result else {
        panic!("expected string");
    };
    let out = vm.inner.strings.get_utf8(sid);
    vm.unbind();
    assert_eq!(
        out, "D,C",
        "within-tree move should fire D then C (got: {out})"
    );
}

// --- F2: Insert→try_to_upgrade for detached-pre-define elements ---

#[test]
fn re_insert_after_define_triggers_upgrade() {
    // F2 fix: an Undefined CE that was orphaned at define() time
    // (and therefore missed by the document-rooted upgrade walk —
    // now world-wide via the hecs query) must still upgrade when
    // later inserted into the document.
    let out = run_then_read(
        "globalThis.__upgraded = 'no'; \
         var el = document.createElement('my-el'); \
         document.body.removeChild(document.body.appendChild(el)); \
         class MyEl { constructor() { globalThis.__upgraded = 'yes'; } } \
         customElements.define('my-el', MyEl); \
         document.body.appendChild(el);",
        "globalThis.__upgraded;",
    );
    assert_eq!(out, "yes");
}

// --- F5: cloneNode re-attaches CustomElementState ----------------

#[test]
fn clone_node_attaches_ce_state_so_upgrade_fires() {
    // F5 fix: cloneNode on a Custom element must re-attach
    // CustomElementState::undefined so that subsequent insert + flush
    // queues the clone for upgrade. Without the fix, the clone has
    // no CE state component and the upgrade pipeline silently skips
    // it.
    let out = run_then_read(
        "globalThis.__count = 0; \
         class MyEl { constructor() { globalThis.__count++; } } \
         customElements.define('my-el', MyEl); \
         var src = document.createElement('my-el'); \
         var clone = src.cloneNode(); \
         document.body.appendChild(clone);",
        "'' + globalThis.__count;",
    );
    // src constructor (immediate sync upgrade in createElement) +
    // clone constructor (upgrade triggered by Insert) = 2.
    assert_eq!(out, "2");
}

// --- F4: createElement('MY-EL') case-folds before CE validation --

#[test]
fn createelement_mixed_case_tag_still_attaches_ce_state() {
    // F4 fix: createElement('MY-EL') case-folds tag to 'my-el' before
    // attaching CustomElementState (matching the engine-indep handler
    // which stores TagType lowercased). Without the fold, the
    // mixed-case input would silently skip CE state attachment.
    let out = run_then_read(
        "globalThis.__upgraded = 'no'; \
         class MyEl { constructor() { globalThis.__upgraded = 'yes'; } } \
         customElements.define('my-el', MyEl); \
         document.createElement('MY-EL');",
        "globalThis.__upgraded;",
    );
    assert_eq!(out, "yes");
}

// --- F14: workers do NOT expose customElements ---------------
// (no test — worker VM construction requires bind_worker setup
//  that the regular test fixture doesn't provide; gating verified by
//  inspection of register_globals.)

// --- Copilot R1 #1: non-callable lifecycle property reports rather than silently drops

#[test]
fn non_callable_lifecycle_property_does_not_silently_drop_subsequent_callbacks() {
    // The original silent-return path would also abort the drain loop's
    // sibling reactions if a follow-up callback was attached to the same
    // wave. With the eprintln-and-continue fix, the broken element still
    // fires nothing, but a sibling element's connectedCallback in the
    // same wave runs normally.
    let out = run_then_read(
        "globalThis.__log = []; \
         class BrokenEl {} \
         BrokenEl.prototype.connectedCallback = 42; \
         class GoodEl { connectedCallback() { globalThis.__log.push('good'); } } \
         customElements.define('broken-el', BrokenEl); \
         customElements.define('good-el', GoodEl); \
         document.body.appendChild(document.createElement('broken-el')); \
         document.body.appendChild(document.createElement('good-el'));",
        "globalThis.__log.join(',');",
    );
    assert_eq!(out, "good");
}

#[test]
fn absent_lifecycle_callbacks_are_silent_noop() {
    // Copilot R4 #1 regression guard: an absent lifecycle callback
    // (property resolves to `undefined`) MUST be a silent no-op per
    // HTML §4.13.4 step 2 ("If callback is null, then return"). The
    // R1 fix accidentally over-corrected by eprintln-ing on every
    // non-Object cb_value, which fired for every absent callback —
    // the common case. The fix adds an undefined/null fast-path.
    //
    // Test: define a class with ONLY connectedCallback (no
    // disconnectedCallback / attributeChangedCallback). Connect +
    // disconnect + setAttribute. Expect only the present callback
    // to fire; no stderr noise for the absent ones (we can't easily
    // assert stderr here, but the positive assertion that the flow
    // completes cleanly catches the regression alongside the
    // sibling test above).
    let out = run_then_read(
        "globalThis.__log = []; \
         class MyEl { \
             static get observedAttributes() { return ['x']; } \
             connectedCallback() { globalThis.__log.push('C'); } \
         } \
         customElements.define('my-el', MyEl); \
         var el = document.createElement('my-el'); \
         document.body.appendChild(el); \
         document.body.removeChild(el); \
         el.setAttribute('x', 'val');",
        "globalThis.__log.join(',');",
    );
    assert_eq!(out, "C");
}

// --- /review R-LOW6: detached DocumentFragment does NOT fire Connected

#[test]
fn document_fragment_subtree_upgrade_without_connected_callback() {
    // /review R-LOW6: A custom element inside a DocumentFragment that
    // is never inserted into the document must NOT receive a
    // connectedCallback even after `customElements.define()` upgrades
    // it. attributeChangedCallback / Upgrade still fire as appropriate
    // (definition + observedAttributes match), but the document-
    // disconnected gate suppresses the Connected enqueue.
    let out = run_then_read(
        "globalThis.__log = []; \
         class MyEl { \
             constructor() { globalThis.__log.push('U'); } \
             connectedCallback() { globalThis.__log.push('C'); } \
         } \
         customElements.define('my-el', MyEl); \
         var frag = document.createDocumentFragment(); \
         var el = document.createElement('my-el'); \
         frag.appendChild(el);",
        "globalThis.__log.join(',');",
    );
    // Upgrade fires (immediate-sync at createElement post-define), but
    // the appendChild into the fragment leaves `el` disconnected — no
    // Connected enqueue.
    assert_eq!(out, "U");
}

// --- /review R-RISK1: insertBefore / replaceChild move semantics --

#[test]
fn insert_before_move_fires_both_disconnected_and_connected() {
    // /review R-RISK1: within-tree move via insertBefore (different
    // code path than append_child) — must fire D then C, same as
    // append_child move (covered by the F1 test).
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval(
        "globalThis.__log = []; \
         class MyEl { \
             connectedCallback() { globalThis.__log.push('C'); } \
             disconnectedCallback() { globalThis.__log.push('D'); } \
         } \
         customElements.define('my-el', MyEl); \
         var el = document.createElement('my-el'); \
         document.body.appendChild(el); \
         var ref = document.body.appendChild(document.createElement('span')); \
         globalThis.el = el; globalThis.ref = ref;",
    )
    .expect("setup failed");
    vm.eval("globalThis.__log = [];").expect("reset failed");
    vm.eval("document.body.insertBefore(globalThis.el, globalThis.ref);")
        .expect("insertBefore failed");
    let JsValue::String(sid) = vm.eval("globalThis.__log.join(',');").expect("read failed") else {
        panic!("expected string");
    };
    let out = vm.inner.strings.get_utf8(sid);
    vm.unbind();
    assert_eq!(
        out, "D,C",
        "insertBefore move should fire D then C (got: {out})"
    );
}

#[test]
fn replace_child_fires_disconnected_for_replaced_element() {
    // /review R-RISK1: replaceChild replaces an old element with a new
    // one — the old (if Custom) fires disconnectedCallback, the new
    // (if Custom) fires connectedCallback.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval(
        "globalThis.__log = []; \
         globalThis.__nextId = 0; \
         class MyEl { \
             constructor() { globalThis.__nextId++; this._id = globalThis.__nextId; } \
             connectedCallback() { globalThis.__log.push('C' + this._id); } \
             disconnectedCallback() { globalThis.__log.push('D' + this._id); } \
         } \
         customElements.define('my-el', MyEl); \
         var oldEl = document.createElement('my-el'); \
         document.body.appendChild(oldEl); \
         globalThis.oldEl = oldEl;",
    )
    .expect("setup failed");
    vm.eval("globalThis.__log = [];").expect("reset failed");
    vm.eval(
        "var newEl = document.createElement('my-el'); \
         document.body.replaceChild(newEl, globalThis.oldEl);",
    )
    .expect("replaceChild failed");
    let JsValue::String(sid) = vm.eval("globalThis.__log.join(',');").expect("read failed") else {
        panic!("expected string");
    };
    let out = vm.inner.strings.get_utf8(sid);
    vm.unbind();
    // oldEl was instance 1 (sync upgrade at createElement); newEl is
    // instance 2 (sync upgrade at createElement). replaceChild fires
    // Remove(oldEl)→D1, Insert(newEl)→C2.
    assert_eq!(out, "D1,C2");
}
