//! D-10 `#11-events-misc` — 10 NEW Event constructor classes
//! (SubmitEvent / FormDataEvent / ToggleEvent / CompositionEvent /
//! ClipboardEvent / ProgressEvent / BeforeUnloadEvent / MessageEvent /
//! WheelEvent / PageTransitionEvent) + InputEvent extension
//! (dataTransfer / getTargetRanges) + systemic UA-brand fix
//! (`prototype_for_payload` per `EventPayload` variant).
//!
//! Coverage matches the D-10 plan §C-9 test plan:
//! - per-ctor brand check + prototype chain + init dict defaults +
//!   EventInit propagation + ctor-as-call throws + interface-specific
//!   round-trips
//! - FormDataEvent required-member TypeError
//! - BeforeUnloadEvent constructor-disabled (always throws "Illegal
//!   constructor") + mutable returnValue accessor round-trip
//! - WheelEvent DOM_DELTA_* prototype constants
//! - MessageEvent source/ports any-pass-through
//! - InputEvent.dataTransfer null stub + getTargetRanges() empty Array
//! - UA-brand systemic fix — UA-dispatched events of every payload
//!   variant chain through the matching subclass prototype
//!   (`instanceof <SubclassEvent>` returns `true`)

#![cfg(feature = "engine")]

use elidex_ecs::{Attributes, EcsDom};
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;

fn build_empty_doc(dom: &mut EcsDom) -> elidex_ecs::Entity {
    let doc = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    let body = dom.create_element("body", Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, body));
    doc
}

fn run(script: &str) -> String {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_empty_doc(&mut dom);
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

// =====================================================================
// SubmitEvent
// =====================================================================

#[test]
fn submit_event_global_present() {
    let out = run("(typeof SubmitEvent === 'function') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn submit_event_brand_check() {
    let out = run("var e = new SubmitEvent('submit'); \
         (e instanceof SubmitEvent && e instanceof Event) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn submit_event_prototype_chain() {
    let out = run(
        "(Object.getPrototypeOf(new SubmitEvent('submit')) === SubmitEvent.prototype \
         && Object.getPrototypeOf(SubmitEvent.prototype) === Event.prototype) \
         ? 'ok' : 'fail';",
    );
    assert_eq!(out, "ok");
}

#[test]
fn submit_event_default_submitter_null() {
    let out = run("var e = new SubmitEvent('submit'); \
         (e.submitter === null) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn submit_event_submitter_round_trip() {
    let out = run("var btn = document.createElement('button'); \
         var e = new SubmitEvent('submit', { submitter: btn }); \
         (e.submitter === btn) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn submit_event_call_mode_throws() {
    let out = run("var ok = false; \
         try { SubmitEvent('submit'); } catch (_) { ok = true; } \
         ok ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn submit_event_init_propagates_bubbles_cancelable() {
    let out = run("var e = new SubmitEvent('submit', \
             { bubbles: true, cancelable: true }); \
         (e.bubbles === true && e.cancelable === true) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// =====================================================================
// FormDataEvent
// =====================================================================

#[test]
fn formdata_event_global_present() {
    let out = run("(typeof FormDataEvent === 'function') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn formdata_event_brand_check() {
    let out = run(
        "var e = new FormDataEvent('formdata', { formData: new FormData() }); \
         (e instanceof FormDataEvent && e instanceof Event) ? 'ok' : 'fail';",
    );
    assert_eq!(out, "ok");
}

#[test]
fn formdata_event_required_form_data_throws() {
    // Empty dict triggers the required-member check (the no-2nd-arg
    // case bails on arity earlier — separate code path).
    let out = run("var ok = false; var msg = ''; \
         try { new FormDataEvent('formdata', {}); } \
         catch (e) { ok = true; msg = String(e); } \
         (ok && msg.indexOf('formData') !== -1) ? 'ok' : 'fail:msg=' + msg;");
    assert_eq!(out, "ok");
}

#[test]
fn formdata_event_no_second_arg_throws_arity() {
    // 1-arg form throws arity error (separate from the required-member
    // check that fires on empty-dict / null / undefined).
    let out = run("var ok = false; var msg = ''; \
         try { new FormDataEvent('formdata'); } \
         catch (e) { ok = true; msg = String(e); } \
         (ok && msg.indexOf('arguments required') !== -1) ? 'ok' : 'fail:msg=' + msg;");
    assert_eq!(out, "ok");
}

#[test]
fn formdata_event_empty_dict_throws() {
    let out = run("var ok = false; \
         try { new FormDataEvent('formdata', {}); } catch (_) { ok = true; } \
         ok ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn formdata_event_rejects_non_form_data_object() {
    // R7 IMP regression: WebIDL dictionary conversion requires the
    // `formData` member to be an actual FormData instance.  Plain
    // `{}` / non-FormData objects must throw TypeError at conversion
    // time — matches Chrome / Firefox.
    let out = run("var ok = false; var msg = ''; \
         try { new FormDataEvent('formdata', { formData: {} }); } \
         catch (e) { ok = true; msg = String(e); } \
         (ok && msg.indexOf(\"not of type 'FormData'\") !== -1) ? 'ok' : 'fail:' + msg;");
    assert_eq!(out, "ok");
}

#[test]
fn formdata_event_rejects_form_data_primitive_string() {
    let out = run("var ok = false; \
         try { new FormDataEvent('formdata', { formData: 'hello' }); } \
         catch (_) { ok = true; } \
         ok ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn formdata_event_form_data_round_trip() {
    let out = run("var fd = new FormData(); \
         var e = new FormDataEvent('formdata', { formData: fd }); \
         (e.formData === fd) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn formdata_event_call_mode_throws() {
    let out = run("var ok = false; \
         try { FormDataEvent('formdata', { formData: new FormData() }); } \
         catch (_) { ok = true; } \
         ok ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// =====================================================================
// ToggleEvent
// =====================================================================

#[test]
fn toggle_event_global_present() {
    let out = run("(typeof ToggleEvent === 'function') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn toggle_event_brand_check() {
    let out = run("var e = new ToggleEvent('toggle'); \
         (e instanceof ToggleEvent && e instanceof Event) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn toggle_event_default_states_empty() {
    let out = run("var e = new ToggleEvent('toggle'); \
         (e.oldState === '' && e.newState === '') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn toggle_event_state_round_trip() {
    let out = run("var e = new ToggleEvent('toggle', \
             { oldState: 'closed', newState: 'open' }); \
         (e.oldState === 'closed' && e.newState === 'open') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn toggle_event_call_mode_throws() {
    let out = run("var ok = false; \
         try { ToggleEvent('toggle'); } catch (_) { ok = true; } \
         ok ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// =====================================================================
// CompositionEvent (UIEvent base)
// =====================================================================

#[test]
fn composition_event_global_present() {
    let out = run("(typeof CompositionEvent === 'function') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn composition_event_brand_check() {
    let out = run("var e = new CompositionEvent('compositionstart'); \
         (e instanceof CompositionEvent && e instanceof UIEvent && e instanceof Event) \
             ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn composition_event_prototype_chain_through_ui_event() {
    let out = run(
        "(Object.getPrototypeOf(CompositionEvent.prototype) === UIEvent.prototype) \
         ? 'ok' : 'fail';",
    );
    assert_eq!(out, "ok");
}

#[test]
fn composition_event_default_data_empty() {
    let out = run("var e = new CompositionEvent('compositionstart'); \
         (e.data === '') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn composition_event_data_round_trip() {
    let out = run(
        "var e = new CompositionEvent('compositionupdate', { data: 'あ' }); \
         (e.data === 'あ') ? 'ok' : 'fail';",
    );
    assert_eq!(out, "ok");
}

#[test]
fn composition_event_inherits_view_detail_from_ui_event() {
    let out = run(
        "var e = new CompositionEvent('compositionend', { detail: 5 }); \
         (e.view === null && e.detail === 5) ? 'ok' : 'fail';",
    );
    assert_eq!(out, "ok");
}

// =====================================================================
// ClipboardEvent
// =====================================================================

#[test]
fn clipboard_event_global_present() {
    let out = run("(typeof ClipboardEvent === 'function') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn clipboard_event_brand_check() {
    let out = run("var e = new ClipboardEvent('cut'); \
         (e instanceof ClipboardEvent && e instanceof Event) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn clipboard_event_default_clipboard_data_null() {
    let out = run("var e = new ClipboardEvent('paste'); \
         (e.clipboardData === null) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn clipboard_event_data_brand_check() {
    // D-9 upgrade: `clipboardData` must be a DataTransfer brand OR
    // null / undefined.  Non-DataTransfer Objects throw TypeError
    // per WebIDL §3.10.21 interface-type coercion.
    let out = run("var threw = false; \
         try { new ClipboardEvent('copy', { clipboardData: { kind: 'fake' } }); } \
         catch (e) { threw = (e instanceof TypeError); } \
         threw ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn clipboard_event_data_accepts_data_transfer() {
    let out = run("var dt = new DataTransfer(); \
         var e = new ClipboardEvent('copy', { clipboardData: dt }); \
         (e.clipboardData === dt) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn clipboard_event_data_null_default() {
    let out = run("(new ClipboardEvent('copy').clipboardData === null) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// =====================================================================
// ProgressEvent
// =====================================================================

#[test]
fn progress_event_global_present() {
    let out = run("(typeof ProgressEvent === 'function') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn progress_event_brand_check() {
    let out = run("var e = new ProgressEvent('progress'); \
         (e instanceof ProgressEvent && e instanceof Event) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn progress_event_defaults() {
    let out = run("var e = new ProgressEvent('progress'); \
         (e.lengthComputable === false && e.loaded === 0 && e.total === 0) \
             ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn progress_event_round_trip() {
    let out = run("var e = new ProgressEvent('progress', \
             { lengthComputable: true, loaded: 100, total: 200 }); \
         (e.lengthComputable === true && e.loaded === 100 && e.total === 200) \
             ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn progress_event_call_mode_throws() {
    let out = run("var ok = false; \
         try { ProgressEvent('progress'); } catch (_) { ok = true; } \
         ok ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// =====================================================================
// BeforeUnloadEvent
// =====================================================================

#[test]
fn before_unload_event_global_present() {
    let out = run("(typeof BeforeUnloadEvent === 'function') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn before_unload_event_constructor_throws() {
    // Per WHATWG WebIDL §3.5 absence-of-constructor — `new
    // BeforeUnloadEvent(...)` throws TypeError "Illegal constructor".
    let out = run("var ok = false; var msg = ''; \
         try { new BeforeUnloadEvent('beforeunload'); } \
         catch (e) { ok = true; msg = String(e); } \
         (ok && msg.indexOf('Illegal constructor') !== -1) ? 'ok' : 'fail:' + msg;");
    assert_eq!(out, "ok");
}

#[test]
fn before_unload_event_call_mode_also_throws() {
    let out = run("var ok = false; \
         try { BeforeUnloadEvent('beforeunload'); } catch (_) { ok = true; } \
         ok ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn before_unload_event_prototype_chain() {
    // The global is registered for `instanceof` brand recognition
    // even though `new` throws.
    let out = run(
        "(Object.getPrototypeOf(BeforeUnloadEvent.prototype) === Event.prototype) \
         ? 'ok' : 'fail';",
    );
    assert_eq!(out, "ok");
}

#[test]
fn before_unload_event_return_value_is_accessor_on_prototype() {
    let out = run("var d = Object.getOwnPropertyDescriptor( \
             BeforeUnloadEvent.prototype, 'returnValue'); \
         (d && typeof d.get === 'function' && typeof d.set === 'function') \
             ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// =====================================================================
// MessageEvent
// =====================================================================

#[test]
fn message_event_global_present() {
    let out = run("(typeof MessageEvent === 'function') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn message_event_brand_check() {
    let out = run("var e = new MessageEvent('message'); \
         (e instanceof MessageEvent && e instanceof Event) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn message_event_defaults() {
    let out = run("var e = new MessageEvent('message'); \
         (e.data === null && e.origin === '' && e.lastEventId === '' \
             && e.source === null && Array.isArray(e.ports) && e.ports.length === 0) \
             ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn message_event_data_round_trip() {
    let out = run(
        "var e = new MessageEvent('message', { data: { hello: 'world' } }); \
         (e.data && e.data.hello === 'world') ? 'ok' : 'fail';",
    );
    assert_eq!(out, "ok");
}

#[test]
fn message_event_string_fields_round_trip() {
    let out = run("var e = new MessageEvent('message', \
             { origin: 'https://example.com', lastEventId: '42' }); \
         (e.origin === 'https://example.com' && e.lastEventId === '42') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn message_event_source_ports_pass_through() {
    // any-pass-through — no brand check (MessagePort wrapper deferred
    // to `#11b` M4-12 cutover residual).
    let out = run("var sentinel = { kind: 'window-ish' }; \
         var portsArr = [1, 2, 3]; \
         var e = new MessageEvent('message', { source: sentinel, ports: portsArr }); \
         (e.source === sentinel && e.ports === portsArr) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// =====================================================================
// WheelEvent
// =====================================================================

#[test]
fn wheel_event_global_present() {
    let out = run("(typeof WheelEvent === 'function') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn wheel_event_brand_check() {
    let out = run("var e = new WheelEvent('wheel'); \
         (e instanceof WheelEvent && e instanceof MouseEvent \
             && e instanceof UIEvent && e instanceof Event) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn wheel_event_prototype_chain_through_mouse_event() {
    let out = run(
        "(Object.getPrototypeOf(WheelEvent.prototype) === MouseEvent.prototype \
         && Object.getPrototypeOf(MouseEvent.prototype) === UIEvent.prototype) \
         ? 'ok' : 'fail';",
    );
    assert_eq!(out, "ok");
}

#[test]
fn wheel_event_defaults() {
    let out = run("var e = new WheelEvent('wheel'); \
         (e.deltaX === 0 && e.deltaY === 0 && e.deltaZ === 0 && e.deltaMode === 0) \
             ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn wheel_event_delta_round_trip() {
    let out = run("var e = new WheelEvent('wheel', \
             { deltaX: 1.5, deltaY: -2.5, deltaZ: 3, deltaMode: 1 }); \
         (e.deltaX === 1.5 && e.deltaY === -2.5 && e.deltaZ === 3 && e.deltaMode === 1) \
             ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn wheel_event_dom_delta_constants() {
    // Per WebIDL §3.7.6 const members are exposed as own properties of
    // BOTH the interface object (constructor) AND the interface
    // prototype object.  Verify both sides match the spec values
    // 0 / 1 / 2.
    let out = run("(WheelEvent.prototype.DOM_DELTA_PIXEL === 0 \
         && WheelEvent.prototype.DOM_DELTA_LINE === 1 \
         && WheelEvent.prototype.DOM_DELTA_PAGE === 2 \
         && WheelEvent.DOM_DELTA_PIXEL === 0 \
         && WheelEvent.DOM_DELTA_LINE === 1 \
         && WheelEvent.DOM_DELTA_PAGE === 2) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn wheel_event_inherits_mouse_event_init() {
    let out = run("var e = new WheelEvent('wheel', \
             { clientX: 10, clientY: 20, button: 2, ctrlKey: true }); \
         (e.clientX === 10 && e.clientY === 20 && e.button === 2 && e.ctrlKey === true) \
             ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// =====================================================================
// PageTransitionEvent
// =====================================================================

#[test]
fn page_transition_event_global_present() {
    let out = run("(typeof PageTransitionEvent === 'function') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn page_transition_event_brand_check() {
    let out = run("var e = new PageTransitionEvent('pagehide'); \
         (e instanceof PageTransitionEvent && e instanceof Event) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn page_transition_event_default_persisted_false() {
    let out = run("var e = new PageTransitionEvent('pagehide'); \
         (e.persisted === false) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn page_transition_event_persisted_round_trip() {
    let out = run(
        "var e = new PageTransitionEvent('pageshow', { persisted: true }); \
         (e.persisted === true) ? 'ok' : 'fail';",
    );
    assert_eq!(out, "ok");
}

// =====================================================================
// InputEvent extension (D-10 §C-8)
// =====================================================================

#[test]
fn input_event_get_target_ranges_throws_on_non_event_receiver() {
    // R6 IMP regression: WebIDL §3.7.2.4 brand-check — calling
    // `InputEvent.prototype.getTargetRanges.call({})` must throw
    // TypeError "Illegal invocation".  Pre-fix: succeeded with []
    // because the method had no receiver check.
    let out = run("var fn = InputEvent.prototype.getTargetRanges; \
         var ok = false; var msg = ''; \
         try { fn.call({}); } \
         catch (err) { ok = true; msg = String(err); } \
         (ok && msg.indexOf('Illegal invocation') !== -1) ? 'ok' : 'fail:' + msg;");
    assert_eq!(out, "ok");
}

#[test]
fn input_event_get_target_ranges_throws_on_other_event_subclass() {
    // Cross-Event-subclass receiver must also throw — the brand check
    // requires the prototype chain to include InputEvent.prototype.
    let out = run("var fn = InputEvent.prototype.getTargetRanges; \
         var me = new MouseEvent('click'); \
         var ok = false; var msg = ''; \
         try { fn.call(me); } \
         catch (err) { ok = true; msg = String(err); } \
         (ok && msg.indexOf('Illegal invocation') !== -1) ? 'ok' : 'fail:' + msg;");
    assert_eq!(out, "ok");
}

#[test]
fn input_event_get_target_ranges_works_on_real_input_event() {
    // Sanity: brand check accepts the canonical receiver — calling on
    // a real InputEvent instance still returns an empty Array.
    let out = run("var e = new InputEvent('input'); \
         var r = e.getTargetRanges(); \
         (Array.isArray(r) && r.length === 0) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn progress_event_loaded_negative_wraps_per_unsigned_long_long() {
    // R6 IMP regression: WebIDL `unsigned long long` (§3.10.10 without
    // [EnforceRange]) coerces `-1` via mod-2^64 to 2^64 - 1 (a very
    // large positive).  Pre-fix the raw `-1` was reflected verbatim.
    // Test asserts `loaded > 0` and `loaded !== -1` to avoid f64
    // precision-comparison flakiness against the literal 2^64-1.
    let out = run("var e = new ProgressEvent('p', { loaded: -1 }); \
         (e.loaded > 0 && e.loaded !== -1) ? 'ok' \
             : 'fail:loaded=' + e.loaded;");
    assert_eq!(out, "ok");
}

#[test]
fn progress_event_loaded_fractional_truncates_toward_zero() {
    let out = run(
        "var e = new ProgressEvent('p', { loaded: 1.9, total: 2.5 }); \
         (e.loaded === 1 && e.total === 2) ? 'ok' \
             : 'fail:loaded=' + e.loaded + ',total=' + e.total;",
    );
    assert_eq!(out, "ok");
}

#[test]
fn progress_event_loaded_negative_fraction_normalises_to_positive_zero() {
    // R10 MIN regression: `-0.5` truncates to `-0.0` which
    // per WebIDL §3.10.10 step 2 must be normalised to `+0`.
    // Pre-fix the fast-path returned `-0.0` directly, observable
    // via `Object.is(e.loaded, 0) === false` and
    // `1 / e.loaded === -Infinity`.
    let out = run("var e = new ProgressEvent('p', { loaded: -0.5 }); \
         (e.loaded === 0 && Object.is(e.loaded, 0) \
             && 1 / e.loaded === Infinity) ? 'ok' \
             : 'fail:loaded=' + e.loaded + ',isPosZero=' + Object.is(e.loaded, 0) \
                 + ',recip=' + (1 / e.loaded);");
    assert_eq!(out, "ok");
}

#[test]
fn progress_event_loaded_nan_and_infinity_become_zero() {
    let out = run("var e = new ProgressEvent('p', \
             { loaded: NaN, total: Infinity }); \
         (e.loaded === 0 && e.total === 0) ? 'ok' \
             : 'fail:loaded=' + e.loaded + ',total=' + e.total;");
    assert_eq!(out, "ok");
}

#[test]
fn input_event_data_transfer_default_null() {
    let out = run("var e = new InputEvent('input'); \
         (e.dataTransfer === null) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn input_event_get_target_ranges_returns_empty_array() {
    let out = run("var e = new InputEvent('input'); \
         var r = e.getTargetRanges(); \
         (Array.isArray(r) && r.length === 0) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn input_event_get_target_ranges_returns_fresh_array_each_call() {
    let out = run("var e = new InputEvent('input'); \
         (e.getTargetRanges() !== e.getTargetRanges()) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn input_event_data_transfer_brand_check() {
    // D-9 upgrade: `dataTransfer` must be a DataTransfer brand OR
    // null / undefined.  Non-DataTransfer Objects throw TypeError.
    let out = run("var threw = false; \
         try { new InputEvent('input', { dataTransfer: { kind: 'dt-stub' } }); } \
         catch (e) { threw = (e instanceof TypeError); } \
         threw ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn input_event_data_transfer_accepts_data_transfer() {
    let out = run("var dt = new DataTransfer(); \
         var e = new InputEvent('input', { dataTransfer: dt }); \
         (e.dataTransfer === dt) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn input_event_existing_fields_still_work() {
    // InputEvent shape extension from 3 → 4 slots must NOT break the
    // existing data / isComposing / inputType slots.
    let out = run("var e = new InputEvent('input', \
             { data: 'x', isComposing: true, inputType: 'insertText' }); \
         (e.data === 'x' && e.isComposing === true && e.inputType === 'insertText') \
             ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// =====================================================================
// BeforeUnloadEvent.returnValue mutable round-trip via UA-dispatched
// instance — tested via a synthesised Object.create(prototype) trick
// since `new BeforeUnloadEvent(...)` throws.
// =====================================================================

#[test]
fn before_unload_return_value_getter_throws_on_non_event_receiver() {
    // Brand check (WebIDL §3.7.2.4): `Object.create(BeforeUnloadEvent.
    // prototype)` is not an `ObjectKind::Event` instance, so reading
    // `returnValue` must throw TypeError "Illegal invocation" — matches
    // Chrome / Firefox.  Without the brand check the side table would
    // accumulate entries for arbitrary objects.
    let out = run("var e = Object.create(BeforeUnloadEvent.prototype); \
         var ok = false; var msg = ''; \
         try { var v = e.returnValue; } \
         catch (err) { ok = true; msg = String(err); } \
         (ok && msg.indexOf('Illegal invocation') !== -1) ? 'ok' : 'fail:' + msg;");
    assert_eq!(out, "ok");
}

#[test]
fn before_unload_return_value_setter_throws_on_non_event_receiver() {
    let out = run("var e = Object.create(BeforeUnloadEvent.prototype); \
         var ok = false; var msg = ''; \
         try { e.returnValue = 'wait!'; } \
         catch (err) { ok = true; msg = String(err); } \
         (ok && msg.indexOf('Illegal invocation') !== -1) ? 'ok' : 'fail:' + msg;");
    assert_eq!(out, "ok");
}

#[test]
fn before_unload_return_value_getter_throws_on_other_event_subclass() {
    // Brand check must reject Event subclasses that aren't
    // BeforeUnloadEvent — calling the getter `.call(new MouseEvent())`
    // matches Chrome's "Illegal invocation" semantics.  The `ObjectKind::
    // Event` check alone is not enough; the prototype chain must include
    // `BeforeUnloadEvent.prototype`.
    let out = run("var getter = Object.getOwnPropertyDescriptor( \
             BeforeUnloadEvent.prototype, 'returnValue').get; \
         var me = new MouseEvent('click'); \
         var ok = false; var msg = ''; \
         try { getter.call(me); } \
         catch (err) { ok = true; msg = String(err); } \
         (ok && msg.indexOf('Illegal invocation') !== -1) ? 'ok' : 'fail:' + msg;");
    assert_eq!(out, "ok");
}

#[test]
fn before_unload_return_value_setter_throws_on_other_event_subclass() {
    let out = run("var setter = Object.getOwnPropertyDescriptor( \
             BeforeUnloadEvent.prototype, 'returnValue').set; \
         var me = new MouseEvent('click'); \
         var ok = false; var msg = ''; \
         try { setter.call(me, 'wait!'); } \
         catch (err) { ok = true; msg = String(err); } \
         (ok && msg.indexOf('Illegal invocation') !== -1) ? 'ok' : 'fail:' + msg;");
    assert_eq!(out, "ok");
}

#[test]
fn before_unload_return_value_setter_error_message_says_set_not_read() {
    // R3 MIN: setter and getter share `require_before_unload_receiver`,
    // so the error message must be parameterised by op (`read` / `set`)
    // — not hardcoded as one or the other.  Verify setter says "set".
    let out = run("var e = Object.create(BeforeUnloadEvent.prototype); \
         var msg = ''; \
         try { e.returnValue = 'x'; } \
         catch (err) { msg = String(err); } \
         (msg.indexOf(\"set 'returnValue'\") !== -1) ? 'ok' : 'fail:' + msg;");
    assert_eq!(out, "ok");
}

// =====================================================================
// 10 prototypes are all distinct
// =====================================================================

#[test]
fn d10_prototypes_all_distinct() {
    // 10 distinct prototypes — quadratic uniqueness check (Set isn't
    // available in this VM yet; pairwise comparison is fine for n=10).
    let out = run(
        "var protos = [SubmitEvent.prototype, FormDataEvent.prototype, \
             ToggleEvent.prototype, CompositionEvent.prototype, \
             ClipboardEvent.prototype, ProgressEvent.prototype, \
             BeforeUnloadEvent.prototype, MessageEvent.prototype, \
             WheelEvent.prototype, PageTransitionEvent.prototype]; \
         var dup = false; \
         for (var i = 0; i < protos.length; i++) { \
             for (var j = i + 1; j < protos.length; j++) { \
                 if (protos[i] === protos[j]) dup = true; \
             } \
         } \
         (!dup && protos.length === 10) ? 'ok' \
             : 'fail:dup=' + dup + ',count=' + protos.length;",
    );
    assert_eq!(out, "ok");
}

// =====================================================================
// UA-brand fix systemic (D-10 §C-7) — UA-dispatched events of every
// payload variant chain through the matching subclass prototype.
// `instanceof X` returns true for UA-dispatched events; pre-D-10 it
// returned false because `create_event_object` hardcoded
// Event.prototype.
// =====================================================================

#[test]
fn ua_brand_fix_message_event_dispatched_via_post_message() {
    // postMessage + onmessage path dispatches a UA MessageEvent.
    // Pre-fix `e instanceof MessageEvent` returned false; post-fix
    // returns true.  Two-eval pattern matches the existing post_message
    // tests (the listener fires across the eval boundary as the task
    // queue drains).
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_empty_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval(
        "globalThis.classOk = false; \
         window.addEventListener('message', function(e) { \
             globalThis.classOk = (e instanceof MessageEvent && e instanceof Event); \
         }); \
         window.postMessage('hi', '*');",
    )
    .unwrap();
    let result = vm.eval("globalThis.classOk;").unwrap();
    assert!(
        matches!(result, JsValue::Boolean(true)),
        "UA-dispatched MessageEvent must satisfy `instanceof MessageEvent` post-D-10 \
         §C-7 fix; got {result:?}"
    );
    vm.unbind();
}

#[test]
fn ua_brand_fix_existing_form_invalid_event_brand_check() {
    // form.checkValidity() fires `invalid` events on each invalid
    // control via dispatch_simple_event (Event.prototype, no payload
    // variant).  Brand check confirms the dispatch_simple_event path
    // still uses Event.prototype (it goes through alloc_object
    // directly — not affected by §C-7's create_event_object fix).
    let out = run("var f = document.createElement('form'); \
         var inp = document.createElement('input'); \
         inp.required = true; \
         f.appendChild(inp); \
         var classOk = false; \
         inp.addEventListener('invalid', function(e) { \
             classOk = (e instanceof Event); \
         }); \
         f.checkValidity(); \
         classOk ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// ---------------------------------------------------------------------------
// `[Constructor]` gate regression — each of the 9 events-misc ctors fires
// the canonical `CallShape::ConstructorOnly` TypeError at the dispatch
// site when invoked without `new` (WebIDL §3.7.1 step 1.2).  Plan-memo
// `m4-12-pr-vm-native-constructor-only-flag-plan.md` §5 sites #18-26.
// ---------------------------------------------------------------------------

#[test]
fn submit_event_ctor_requires_new() {
    super::assert_ctor_requires_new("SubmitEvent('submit')", "SubmitEvent");
}

#[test]
fn form_data_event_ctor_requires_new() {
    // Gate fires at dispatch before arg coercion, so the missing
    // required `{formData}` member doesn't reach its validation.
    super::assert_ctor_requires_new("FormDataEvent('formdata')", "FormDataEvent");
}

#[test]
fn toggle_event_ctor_requires_new() {
    super::assert_ctor_requires_new("ToggleEvent('toggle')", "ToggleEvent");
}

#[test]
fn composition_event_ctor_requires_new() {
    super::assert_ctor_requires_new("CompositionEvent('compositionend')", "CompositionEvent");
}

#[test]
fn clipboard_event_ctor_requires_new() {
    super::assert_ctor_requires_new("ClipboardEvent('copy')", "ClipboardEvent");
}

#[test]
fn progress_event_ctor_requires_new() {
    super::assert_ctor_requires_new("ProgressEvent('progress')", "ProgressEvent");
}

#[test]
fn message_event_ctor_requires_new() {
    super::assert_ctor_requires_new("MessageEvent('message')", "MessageEvent");
}

#[test]
fn wheel_event_ctor_requires_new() {
    super::assert_ctor_requires_new("WheelEvent('wheel')", "WheelEvent");
}

#[test]
fn page_transition_event_ctor_requires_new() {
    super::assert_ctor_requires_new("PageTransitionEvent('pageshow')", "PageTransitionEvent");
}
