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
    let out = run("customElements.define('my-el', class extends HTMLElement {}); typeof customElements.get('my-el');");
    assert_eq!(out, "function");
}

#[test]
fn define_invalid_name_throws_syntax_error() {
    // Names without a hyphen are invalid per HTML §4.13.3 `valid custom element name`.
    let err = run_throws("customElements.define('nohyphen', class extends HTMLElement {});");
    assert!(
        err.contains("SyntaxError") || err.contains("valid custom element name"),
        "expected SyntaxError for invalid name, got: {err}"
    );
}

#[test]
fn define_invalid_name_preempts_brand_check() {
    // HTML §4.13.4 step ordering: invalid name (step 2 → SyntaxError) must
    // fire BEFORE the HTMLConstructor brand check (out-of-spec addition).
    // Otherwise `define('nohyphen', class {})` returns TypeError
    // ("must extend HTMLElement") instead of SyntaxError
    // ("not a valid custom element name"). D-17b R14 G14-1 regression.
    let err = run_throws("customElements.define('nohyphen', class {});");
    assert!(
        err.contains("SyntaxError") || err.contains("valid custom element name"),
        "invalid name must throw SyntaxError before brand check fires; got: {err}"
    );
}

#[test]
fn define_reserved_name_throws() {
    // `font-face` is reserved per §4.13.3 `valid custom element name`.
    let err = run_throws("customElements.define('font-face', class extends HTMLElement {});");
    assert!(
        err.contains("SyntaxError") || err.contains("valid custom element name"),
        "expected SyntaxError for reserved name, got: {err}"
    );
}

#[test]
fn define_duplicate_constructor_throws_not_supported_error() {
    // HTML §4.13.4 step 4: same ctor passed under two different names
    // must throw NotSupportedError on the second call. Otherwise the
    // host-side reverse map (`HostData::ce_constructor_to_id`) would
    // overwrite the FIRST definition's binding and `new.target` from
    // a `new FirstCtor()` would resolve to the SECOND definition's
    // name (D-17b R5 G5-1).
    let err = run_throws(
        "class MyEl extends HTMLElement {} \
         customElements.define('a-el', MyEl); \
         customElements.define('b-el', MyEl);",
    );
    assert!(
        err.contains("NotSupportedError")
            || err.contains("has already been used with this registry"),
        "expected NotSupportedError for duplicate constructor, got: {err}"
    );
}

#[test]
fn define_duplicate_throws_not_supported_error() {
    let err = run_throws(
        "customElements.define('my-el', class extends HTMLElement {}); \
         customElements.define('my-el', class extends HTMLElement {});",
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
    // Arrow functions are callable but NOT constructable per ECMA-262
    // §10.2.1 [[Construct]] — WebIDL CustomElementConstructor requires
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
    let err = run_throws(
        "customElements.define('my-button', class extends HTMLElement {}, { extends: 'button' });",
    );
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
    let out = run("class MyEl extends HTMLElement {} \
         customElements.define('my-el', MyEl); \
         (customElements.get('my-el') === MyEl) ? 'same' : 'different';");
    assert_eq!(out, "same");
}

// --- whenDefined() ------------------------------------------------

#[test]
fn whendefined_resolved_for_defined_name() {
    let out = run_then_read(
        "globalThis.MyEl = class extends HTMLElement {}; \
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
        "globalThis.MyEl = class extends HTMLElement {}; \
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
    // Pre-define: element created in Undefined state, then connected to
    // the document. define() upgrades it as a shadow-including
    // document descendant (HTML §4.13.4 "upgrade particular elements
    // within a document") and it transitions to Custom. (A *detached*
    // element would instead upgrade on its later insertion, not at
    // define() time — see `re_insert_after_define_triggers_upgrade`.)
    let out = run_then_read(
        "var el = document.createElement('my-el'); \
         document.body.appendChild(el); \
         globalThis.__upgraded = 'no'; \
         class MyEl extends HTMLElement { constructor() { globalThis.__upgraded = 'yes'; } } \
         customElements.define('my-el', MyEl);",
        "globalThis.__upgraded;",
    );
    assert_eq!(out, "yes");
}

#[test]
fn createelement_after_define_upgrades_immediately() {
    let out = run_then_read(
        "globalThis.__upgraded = 'no'; \
         class MyEl extends HTMLElement { constructor() { globalThis.__upgraded = 'yes'; } } \
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
         class MyEl extends HTMLElement { constructor() { defined_called = 'yes'; } } \
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
         class MyEl extends HTMLElement { connectedCallback() { globalThis.__log.push('connected'); } } \
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
         class MyEl extends HTMLElement { disconnectedCallback() { globalThis.__log.push('disconnected'); } } \
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
         class MyEl extends HTMLElement { \
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
         class MyEl extends HTMLElement { \
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

#[test]
fn observed_attributes_string_primitive_rejected() {
    // WebIDL §3.2.21 step 1: `observedAttributes` is converted to a
    // `sequence<DOMString>`, so a string primitive (not an Object) is a
    // TypeError during `define()` — it is NOT iterated per code point into
    // single-char attribute names (the cross-cutting effect of the shared
    // converter gaining step 1).
    let err = run_throws(
        "customElements.define('my-el', class extends HTMLElement { \
             static get observedAttributes() { return 'abc'; } });",
    );
    assert!(
        err.contains("TypeError") && err.contains("observedAttributes"),
        "string observedAttributes must reject with a TypeError (WebIDL §3.2.21 step 1); got: {err}"
    );
}

// --- upgrade() ----------------------------------------------------

#[test]
fn customelements_upgrade_walks_descendants() {
    let out = run_then_read(
        "globalThis.__log = []; \
         class MyEl extends HTMLElement { constructor() { globalThis.__log.push('upgraded'); } } \
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
         class MyEl extends HTMLElement { \
             static get observedAttributes() { return ['x']; } \
             constructor() { globalThis.__ctor_attempts++; throw new Error('boom'); } \
             attributeChangedCallback() { globalThis.__ctor_attempts = 99; } \
         } \
         var el = document.createElement('my-el'); \
         document.body.appendChild(el); \
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
    // HTML §4.13.5 "upgrade an element" step 9.4 — if the
    // constructor returns an object that is not SameValue with the
    // element, mark Failed + throw NotSupportedError. The throw is
    // swallowed at the upgrade-flush boundary (Window.onerror path);
    // the Failed state is observable via subsequent setAttribute on an
    // observed attribute NOT firing attributeChangedCallback (callback
    // gating per HTML §4.13.6 requires CEState::Custom).
    let out = run_then_read(
        "globalThis.__ctor_attempts = 0; \
         globalThis.__cb_attempts = 0; \
         class MyEl extends HTMLElement { \
             static get observedAttributes() { return ['x']; } \
             constructor() { globalThis.__ctor_attempts++; return {}; } \
             attributeChangedCallback() { globalThis.__cb_attempts++; } \
         } \
         var el = document.createElement('my-el'); \
         document.body.appendChild(el); \
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
         class MyEl extends HTMLElement { \
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
         class MyEl extends HTMLElement { constructor() { globalThis.__upgraded = 'yes'; } } \
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
         class MyEl extends HTMLElement { constructor() { globalThis.__count++; } } \
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
         class MyEl extends HTMLElement { constructor() { globalThis.__upgraded = 'yes'; } } \
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
         class BrokenEl extends HTMLElement {} \
         BrokenEl.prototype.connectedCallback = 42; \
         class GoodEl extends HTMLElement { connectedCallback() { globalThis.__log.push('good'); } } \
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
         class MyEl extends HTMLElement { \
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
         class MyEl extends HTMLElement { \
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
         class MyEl extends HTMLElement { \
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
         class MyEl extends HTMLElement { \
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

// ---------------------------------------------------------------------------
// D-17b CE / HTMLElement integration tests (slot
// `#11-html-element-constructor-base-vm`). Covers wrapper-prototype
// splice ([C1] §3.2.3 step 14), HTMLElement-on-globalThis ([C1] step
// 1 illegal-ctor + WebIDL §3.7.1 Interface object), the HTMLConstructor
// chain check at define time ([C3] §4.13.4), and `this.method()`
// reachability inside lifecycle callbacks after upgrade.
// ---------------------------------------------------------------------------

#[test]
fn html_element_installed_as_global_constructor() {
    let out = run("typeof HTMLElement === 'function' ? 'yes' : 'no';");
    assert_eq!(out, "yes");
}

#[test]
fn new_html_element_direct_throws() {
    // [C1] §3.2.3 step 1 — `new HTMLElement()` with `NewTarget`
    // pointing at the HTMLElement constructor itself throws TypeError.
    let err = run_throws("new HTMLElement();");
    assert!(
        err.contains("Illegal constructor"),
        "expected Illegal constructor TypeError, got: {err}"
    );
}

#[test]
fn define_accepts_html_element_ctor() {
    // Positive base case — sibling of `define_rejects_non_html_element_ctor`.
    let out = run(
        "customElements.define('my-el', class extends HTMLElement {}); \
         typeof customElements.get('my-el');",
    );
    assert_eq!(out, "function");
}

#[test]
fn define_rejects_non_html_element_ctor() {
    // [C1] §3.2.3 HTMLConstructor brand check invoked from
    // [C3] §4.13.4 — the constructor's `[[Prototype]]` chain must
    // reach `globalThis.HTMLElement`.
    let err = run_throws("customElements.define('my-el', class {});");
    assert!(
        err.contains("must extend HTMLElement") || err.contains("HTMLElement"),
        "expected HTMLElement chain-check TypeError, got: {err}"
    );
}

#[test]
fn define_rejects_html_element_itself() {
    // [C1] §3.2.3 + HTML §4.13.4 brand check: a registered CE ctor
    // must EXTEND HTMLElement, not BE HTMLElement itself. The chain
    // walk trivially finds HTMLElement in its own [[Prototype]] chain
    // at hop 0, so without an explicit check
    // `define('x-a', HTMLElement)` would pass and only fail later at
    // sync-construct / upgrade via the illegal-direct-ctor branch
    // ([C1] step 1). Surface the constraint at define-time
    // (D-17b R10 G10-1).
    let err = run_throws("customElements.define('x-a', HTMLElement);");
    assert!(
        err.contains("must extend HTMLElement, not be HTMLElement itself")
            || err.contains("HTMLElement itself"),
        "expected reject-HTMLElement-itself TypeError, got: {err}"
    );
}

#[test]
fn registered_ctor_exposes_no_brand_symbol_to_js() {
    // [C1] §3.2.3 step 5 reverse-map (new.target → constructor_id)
    // lives on `HostData::ce_constructor_to_id` (host-side Rust
    // state). Asserts that NO Symbol-keyed brand leaks onto the
    // ctor JS object — replaces the earlier symbol-brand which
    // `Object.getOwnPropertySymbols` could discover + copy to
    // another ctor for spoofing (D-17b R2 G1).
    let out = run("class MyEl extends HTMLElement {} \
         customElements.define('my-el', MyEl); \
         String(Object.getOwnPropertySymbols(MyEl).length);");
    assert_eq!(
        out, "0",
        "registered CE ctor should expose no brand symbols; got Object.getOwnPropertySymbols length = {out}"
    );
}

#[test]
fn unregistered_ctor_cannot_spoof_registered_definition() {
    // [C1] §3.2.3 step 5 reverse-map is keyed by host-side ObjectId
    // with no JS-visible counterpart, so a user cannot copy/synthesize
    // a brand onto an unregistered ctor to impersonate a registered
    // one. `new Fake()` where Fake extends HTMLElement but was never
    // `define`d must TypeError on the brand check (D-17b R2 G1 + G4
    // joint regression).
    let err = run_throws(
        "class MyEl extends HTMLElement {} \
         customElements.define('my-el', MyEl); \
         class Fake extends HTMLElement {} \
         const sym = Symbol('attacker-brand'); \
         Object.defineProperty(Fake, sym, { value: 0, writable: false, configurable: false }); \
         new Fake();",
    );
    assert!(
        err.contains("not a registered custom element"),
        "expected 'not a registered custom element' rejection; got: {err}"
    );
}

#[test]
fn define_accepts_deep_class_extends_chain() {
    // [C1] §3.2.3 HTMLConstructor brand walk is bounded by the
    // VM-wide `coerce::PROTO_CHAIN_LIMIT` (10_000), not a smaller
    // bespoke cap. A valid `class extends` chain reaching
    // `HTMLElement` only after many hops must still be accepted at
    // `customElements.define` time. Mirrors `tests_canvas.rs::
    // put_image_data_accepts_deep_prototype_chain` for the sibling
    // brand-check (ImageData).
    let out = run("var Base = HTMLElement; \
         for (var i = 0; i < 100; i++) { Base = class extends Base { }; } \
         customElements.define('deep-el', Base); \
         typeof customElements.get('deep-el');");
    assert_eq!(out, "function");
}

#[test]
fn instanceof_post_upgrade_via_parser_baked() {
    // [C1] §3.2.3 step 14 + D-17b §6 pre-publication invariant —
    // after upgrade the wrapper's prototype chain reaches
    // `MyEl.prototype`, so `instanceof MyEl` returns true.
    let out = run_then_read(
        "globalThis.MyEl = class extends HTMLElement {}; \
         customElements.define('my-el', globalThis.MyEl); \
         var el = document.createElement('my-el'); \
         globalThis.__r = (el instanceof globalThis.MyEl) ? 'yes' : 'no';",
        "globalThis.__r;",
    );
    assert_eq!(out, "yes");
}

#[test]
fn callback_this_dot_helper_method_resolves() {
    // [C1] §3.2.3 step 14 + D-17b §6 pre-publication — the wrapper
    // has `MyEl.prototype` in its chain after upgrade, so
    // `this.helper()` inside a lifecycle callback reaches the
    // class-declared method (Copilot D-17 R6 #1 / R12 #1 closure).
    let out = run_then_read(
        "globalThis.__log = []; \
         class MyEl extends HTMLElement { \
             connectedCallback() { this.helper(); } \
             helper() { globalThis.__log.push('helper-called'); } \
         } \
         customElements.define('my-el', MyEl); \
         var el = document.createElement('my-el'); \
         document.body.appendChild(el);",
        "globalThis.__log.join(',');",
    );
    assert_eq!(out, "helper-called");
}

#[test]
fn user_ctor_super_call_works() {
    // `class extends HTMLElement { constructor() { super(); ... } }`
    // — user-written derived ctor calling super() reaches the
    // HTMLElement constructor's upgrade branch ([C1] step 12+) and
    // produces a fully-upgraded wrapper.
    let out = run_then_read(
        "globalThis.__log = []; \
         class MyEl extends HTMLElement { \
             constructor() { super(); globalThis.__log.push('ctor-ran'); } \
         } \
         customElements.define('my-el', MyEl); \
         document.createElement('my-el');",
        "globalThis.__log.join(',');",
    );
    assert_eq!(out, "ctor-ran");
}

#[test]
fn error_dot_call_does_not_pollute_construct_mode() {
    // Copilot D-17 R6 #1 / R12 #1 closure — `Error.call(this)`
    // inside a class ctor body that was invoked via `new MyEl()`
    // must NOT enter Error's construct branch (no `is_construct()
    // === true` leak from an outer construct frame's
    // `native_construct_stack` entry). Verifies that `Vm::call`'s
    // call-mode entry boundary correctly pushes `None` onto the
    // stack so Error's native ctor sees call mode — without this
    // push, the outer CE-upgrade's `Some(MyEl)` push would still
    // be the stack top when ensure_instance_or_alloc reads it,
    // causing Error to reuse the wrapper as the Error instance
    // and write `name='Error'` / `message='...'` onto the CE
    // wrapper.
    //
    // Probes BOTH the side-effect (constructor ran to completion)
    // AND the wrapper itself (no Error-shaped contamination on
    // the CE entity / wrapper). Earlier draft only checked
    // `__upgraded === 'yes'`, which would silently pass even if
    // the wrapper got polluted — that assertion was orthogonal to
    // the test's docstring claim.
    let out = run_then_read(
        "globalThis.__upgraded = 'no'; \
         globalThis.__poll = 'unset'; \
         class MyEl extends HTMLElement { \
             constructor() { \
                 super(); \
                 try { Error.call(this, 'should not pollute'); } catch (e) {} \
                 globalThis.__upgraded = 'yes'; \
             } \
         } \
         customElements.define('my-el', MyEl); \
         var el = document.createElement('my-el'); \
         globalThis.__poll = (el.message === undefined && el.name === undefined) \
             ? 'clean' : 'polluted:' + el.message + '/' + el.name;",
        "globalThis.__upgraded + '|' + globalThis.__poll;",
    );
    assert_eq!(out, "yes|clean");
}

#[test]
fn new_target_inside_ce_ctor_is_constructor() {
    // [C11] [[Construct]] step 4 — inside a CE class ctor body
    // (post-`super()`), `new.target` is the MyEl constructor
    // (the originally-invoked class).
    let out = run_then_read(
        "globalThis.MyEl = class extends HTMLElement { \
             constructor() { super(); globalThis.__nt = new.target; } \
         }; \
         customElements.define('my-el', globalThis.MyEl); \
         document.createElement('my-el');",
        "(globalThis.__nt === globalThis.MyEl) ? 'same' : 'different';",
    );
    assert_eq!(out, "same");
}

#[test]
fn html_element_call_mode_throws() {
    // `HTMLElement()` invoked as a call (no `new`) — WebIDL
    // constructable-only function requires `new`; rejects with
    // TypeError per D-17b §4.2 step 0.
    let err = run_throws("HTMLElement();");
    assert!(
        err.contains("'new' operator")
            || err.contains("'new' is required")
            || err.contains("Illegal constructor"),
        "expected call-without-new TypeError, got: {err}"
    );
}

// ── S5-6b §4.3.1: no-double-fire pin (§7.2) ───────────────────────────
//
// The mutation stream is partitioned into two disjoint custody chains by
// construction:
//   - VM-native mutations (via `apply_*` → the bind-installed dispatcher)
//     enqueue CE reactions + queue observer records INTERNALLY, and never
//     enter `SessionCore::pending` (so `flush` returns them EMPTY);
//   - external records reach CE + observers ONLY through
//     `deliver_mutation_records` (record→CE via the single classification +
//     observer delivery), which the shell runs on the flush output.
// This test drives BOTH in one turn and asserts each mutation produces
// exactly ONE observer record + (for the custom element) one CE reaction —
// no path double-hears the other's mutation.

/// Read a JS expression that must evaluate to a string.
fn eval_string(vm: &mut Vm, expr: &str) -> String {
    match vm.eval(expr).expect("eval failed") {
        JsValue::String(sid) => vm.inner.strings.get_utf8(sid),
        other => panic!("expected string from `{expr}`, got {other:?}"),
    }
}

#[test]
fn no_double_fire_vm_native_and_external_record_same_turn() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    // Build the tree by hand so `body`'s entity is captured pre-bind for the
    // synthetic external record below.
    let doc = dom.create_document_root();
    let html = dom.create_element("html", elidex_ecs::Attributes::default());
    let body = dom.create_element("body", elidex_ecs::Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, body));

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    vm.eval(
        "globalThis.moLog = []; \
         globalThis.ceLog = []; \
         class XEl extends HTMLElement { \
             static get observedAttributes() { return ['data-x']; } \
             attributeChangedCallback(n, o, v) { globalThis.ceLog.push(n + '=' + v); } \
         } \
         customElements.define('x-el', XEl); \
         globalThis.el = document.createElement('x-el'); \
         document.body.appendChild(globalThis.el); \
         globalThis.mo = new MutationObserver(function (recs) { \
             for (const r of recs) { globalThis.moLog.push(r.type + ':' + (r.attributeName || '')); } \
         }); \
         globalThis.mo.observe(document.body, { subtree: true, attributes: true });",
    )
    .expect("setup failed");
    // Drop the setup's connectedCallback / any records so the two mutations
    // below are isolated.
    vm.eval("globalThis.moLog = []; globalThis.ceLog = [];")
        .expect("reset failed");

    // (1) VM-native mutation on the custom element: rides the dispatcher (CE)
    //     + the VM's internal observer queue — settles at this eval's tail.
    vm.eval("globalThis.el.setAttribute('data-x', 'native');")
        .expect("native mutate failed");

    // (2) External record for `body` (a synthetic layout/shell-buffered
    //     record) delivered via the embedder entry — record→CE (no CE here,
    //     body is not custom) + observer delivery.
    let external = elidex_script_session::MutationRecord {
        kind: elidex_script_session::MutationKind::Attribute,
        target: body,
        added_nodes: Vec::new(),
        removed_nodes: Vec::new(),
        previous_sibling: None,
        next_sibling: None,
        attribute_name: Some("data-ext".to_string()),
        old_value: None,
        parent_was_connected: false,
    };
    vm.deliver_mutation_records(&[external]);

    // Exactly one observer record per mutation — no duplication from either
    // custody chain double-hearing the other's mutation.
    let mo_log = eval_string(&mut vm, "globalThis.moLog.join('|')");
    assert_eq!(
        mo_log, "attributes:data-x|attributes:data-ext",
        "each mutation must deliver exactly one observer record (got: {mo_log})"
    );
    // The custom element's observed-attribute change fired exactly one CE
    // reaction (via the dispatcher); the external body record produced none.
    let ce_log = eval_string(&mut vm, "globalThis.ceLog.join('|')");
    assert_eq!(
        ce_log, "data-x=native",
        "the VM-native custom-element mutation must fire exactly one CE reaction (got: {ce_log})"
    );

    // The VM-native mutation never entered `SessionCore::pending`, so a shell
    // `flush` sees nothing to re-deliver through `deliver_mutation_records` —
    // the structural guarantee that the record leg cannot double-hear it.
    vm.unbind();
    let residual = session.flush(&mut dom);
    assert!(
        residual.is_empty(),
        "VM-native mutations must not buffer in SessionCore::pending (got {} records)",
        residual.len()
    );
}

// ── S5-6b: DOM "remove" step-12 `isParentConnected` (record leg) ───────
//
// WHATWG DOM `#concept-node-remove` step 12 captures `isParentConnected`
// SYNCHRONOUSLY at removal time; step 13 enqueues the custom-element
// `disconnectedCallback` on THAT captured value. The record leg must
// therefore gate `removed_nodes` on `MutationRecord::parent_was_connected`
// (captured at mutation time by the engine-independent apply path), NOT on
// the target's connectivity re-derived at delivery — otherwise a *later*
// record in the same batch that detaches the parent wrongly suppresses the
// earlier removal's reaction.
//
// Both tests drive the record leg (`deliver_mutation_records`) with
// hand-built `MutationRecord`s and the VM bound throughout — the record leg
// is exercised in isolation because no `apply_*` mutation is run to trip the
// bind-installed CE mutation-event dispatcher (the established
// `tests_mutation_observer::delivery` pattern). The setup builds a custom
// element under a DISCONNECTED parent so its connectivity at delivery is
// `false`, forcing the gate to consult the captured `parent_was_connected`.

/// Read the sole child of `entity` from the VM's bound DOM (via `dom_shared`,
/// not the raw `dom` alias — honours `bind_vm`'s no-alias contract).
fn only_child_of(vm: &mut Vm, entity: elidex_ecs::Entity) -> elidex_ecs::Entity {
    let hd = vm.host_data().expect("bound");
    let children = hd.dom_shared().children(entity);
    assert_eq!(children.len(), 1, "expected exactly one child");
    children[0]
}

#[test]
fn record_leg_disconnected_gate_uses_captured_parent_connectedness() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    // doc > html > body is connected; `p` is a DETACHED div — so `p`'s
    // connectivity at record-delivery time is `false`, but a removal of its
    // child was captured while `p` was (hypothetically) connected.
    let doc = dom.create_document_root();
    let html = dom.create_element("html", elidex_ecs::Attributes::default());
    let body = dom.create_element("body", elidex_ecs::Attributes::default());
    let p = dom.create_element("div", elidex_ecs::Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, body));
    // NB: `p` is deliberately NOT attached to `body`.

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let p_wrapper = vm.inner.create_element_wrapper(p);
    vm.set_global("pdiv", JsValue::Object(p_wrapper));

    vm.eval(
        "globalThis.log = []; \
         class DEl extends HTMLElement { \
             connectedCallback() { globalThis.log.push('C'); } \
             disconnectedCallback() { globalThis.log.push('D'); } \
         } \
         customElements.define('d-el', DEl); \
         globalThis.ce = document.createElement('d-el'); \
         globalThis.pdiv.appendChild(globalThis.ce);",
    )
    .expect("setup failed");
    // ce sits under the detached `p`, so no connectedCallback fired.
    assert_eq!(
        eval_string(&mut vm, "globalThis.log.join('|')"),
        "",
        "ce under a disconnected parent must not fire connectedCallback"
    );

    let ce = only_child_of(&mut vm, p);
    assert!(
        !vm.host_data().expect("bound").dom_shared().is_connected(p),
        "p is disconnected at delivery time"
    );

    // A removal record whose target `p` is DISCONNECTED at delivery but whose
    // `parent_was_connected` was captured `true` at mutation time (DOM remove
    // step-12 isParentConnected). The OLD batch-final `is_connected(target)`
    // gate would wrongly skip this record; the split gate consults
    // `parent_was_connected`.
    let rec = elidex_script_session::MutationRecord {
        kind: elidex_script_session::MutationKind::ChildList,
        target: p,
        added_nodes: Vec::new(),
        removed_nodes: vec![ce],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: None,
        old_value: None,
        parent_was_connected: true,
    };
    vm.deliver_mutation_records(&[rec]);
    vm.inner.flush_ce_reactions();

    let log = eval_string(&mut vm, "globalThis.log.join('|')");
    assert_eq!(
        log, "D",
        "disconnectedCallback must fire from the captured `parent_was_connected` \
         even though the record's target is disconnected at delivery (got: {log:?})"
    );
    vm.unbind();
}

#[test]
fn record_leg_added_gate_independent_of_parent_was_connected() {
    // The symmetric add side: the split KEEPS the added_nodes gate on the
    // target's POST-insert `is_connected`, NOT on `parent_was_connected`. Two
    // records — one targeting a CONNECTED parent, one a DISCONNECTED parent —
    // both carry `parent_was_connected: true`; only the connected-target record
    // may fire connectedCallback. This proves the disconnected-side split did
    // not cross-wire the added-side gate.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let html = dom.create_element("html", elidex_ecs::Attributes::default());
    let body = dom.create_element("body", elidex_ecs::Attributes::default());
    // `p_conn` connected under body; `p_disc` detached; `holder` detached (used
    // only to create + resolve the two custom elements while disconnected).
    let p_conn = dom.create_element("div", elidex_ecs::Attributes::default());
    let p_disc = dom.create_element("div", elidex_ecs::Attributes::default());
    let holder = dom.create_element("div", elidex_ecs::Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, body));
    assert!(dom.append_child(body, p_conn));

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let holder_wrapper = vm.inner.create_element_wrapper(holder);
    vm.set_global("holder", JsValue::Object(holder_wrapper));

    vm.eval(
        "globalThis.log = []; \
         class DEl extends HTMLElement { \
             connectedCallback() { globalThis.log.push('C'); } \
             disconnectedCallback() { globalThis.log.push('D'); } \
         } \
         customElements.define('d-el', DEl); \
         globalThis.holder.appendChild(document.createElement('d-el')); \
         globalThis.holder.appendChild(document.createElement('d-el'));",
    )
    .expect("setup failed");
    assert_eq!(
        eval_string(&mut vm, "globalThis.log.join('|')"),
        "",
        "custom elements under the detached holder fire no connectedCallback"
    );

    let (ce_conn, ce_disc) = {
        let hd = vm.host_data().expect("bound");
        let kids = hd.dom_shared().children(holder);
        assert_eq!(kids.len(), 2);
        (kids[0], kids[1])
    };
    let mk_add = |target, added| elidex_script_session::MutationRecord {
        kind: elidex_script_session::MutationKind::ChildList,
        target,
        added_nodes: vec![added],
        removed_nodes: Vec::new(),
        previous_sibling: None,
        next_sibling: None,
        attribute_name: None,
        old_value: None,
        // Deliberately `true` on BOTH: the added gate must ignore this field.
        parent_was_connected: true,
    };
    vm.deliver_mutation_records(&[mk_add(p_conn, ce_conn), mk_add(p_disc, ce_disc)]);
    vm.inner.flush_ce_reactions();

    let log = eval_string(&mut vm, "globalThis.log.join('|')");
    assert_eq!(
        log, "C",
        "exactly one connectedCallback — from the connected-target record; the \
         disconnected-target record fires none despite parent_was_connected=true \
         (added gate stays on post-insert is_connected, got: {log:?})"
    );
    vm.unbind();
}
