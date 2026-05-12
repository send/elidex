//! D-9 `#11-events-modern-input` — Touch + TouchList + TouchEvent
//! plus Touch-flavoured cross-cluster + post-unbind regressions.
//!
//! Split from `tests_events_modern.rs` (Copilot R3/R4 MIN — 1000-line
//! convention).  The `DataTransfer` half (PointerEvent / DragEvent /
//! DataTransfer / DataTransferItem) stays in the parent file; this
//! sibling holds the Touch-family tests plus the Touch cross-cluster
//! check and the Touch post-unbind tolerance regression.

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
// §E. Touch + TouchList + TouchEvent
// =====================================================================

#[test]
fn touch_global_present() {
    let out = run("(typeof Touch === 'function') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn touch_list_global_present() {
    let out = run("(typeof TouchList === 'function') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn touch_event_global_present() {
    let out = run("(typeof TouchEvent === 'function') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn touch_ctor_requires_init() {
    let out = run("var threw = false; \
         try { new Touch(); } \
         catch (e) { threw = (e instanceof TypeError); } \
         threw ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn touch_ctor_missing_arg_message_says_one_required() {
    // No argument → message reflects the missing-arg case.
    let out = run("var msg = ''; \
         try { new Touch(); } catch (e) { msg = String(e.message); } \
         msg.indexOf('1 argument required') >= 0 ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn touch_ctor_wrong_type_message_says_not_of_type_touch_init() {
    // Argument present but not an object → message reflects the
    // type-conversion failure, not the missing-arg case.
    let out = run("var msg = ''; \
         try { new Touch(1); } catch (e) { msg = String(e.message); } \
         msg.indexOf(\"not of type 'TouchInit'\") >= 0 ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn touch_ctor_null_message_says_one_required() {
    // Explicit `null` is treated as the missing-arg case (matches
    // WebIDL §3.10.18 dictionary conversion for required dicts).
    let out = run("var msg = ''; \
         try { new Touch(null); } catch (e) { msg = String(e.message); } \
         msg.indexOf('1 argument required') >= 0 ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn touch_ctor_requires_identifier() {
    let out = run("var threw = false; \
         try { new Touch({ target: document.body }); } \
         catch (e) { threw = (e instanceof TypeError); } \
         threw ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn touch_ctor_requires_target() {
    let out = run("var threw = false; \
         try { new Touch({ identifier: 1 }); } \
         catch (e) { threw = (e instanceof TypeError); } \
         threw ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn touch_ctor_required_members_round_trip() {
    let out = run(
        "var t = new Touch({ identifier: 42, target: document.body }); \
         (t.identifier === 42 && t.target === document.body) ? 'ok' : 'fail';",
    );
    assert_eq!(out, "ok");
}

#[test]
fn touch_ctor_default_coordinates_zero() {
    let out = run(
        "var t = new Touch({ identifier: 1, target: document.body }); \
         (t.clientX === 0 && t.clientY === 0 && \
          t.screenX === 0 && t.screenY === 0 && \
          t.pageX === 0 && t.pageY === 0) ? 'ok' : 'fail';",
    );
    assert_eq!(out, "ok");
}

#[test]
fn touch_ctor_default_radii_zero() {
    let out = run(
        "var t = new Touch({ identifier: 1, target: document.body }); \
         (t.radiusX === 0 && t.radiusY === 0 && \
          t.rotationAngle === 0 && t.force === 0) ? 'ok' : 'fail';",
    );
    assert_eq!(out, "ok");
}

#[test]
fn touch_ctor_full_init_round_trip() {
    let out = run("var t = new Touch({ \
            identifier: 7, target: document.body, \
            clientX: 1.5, clientY: 2.5, screenX: 3, screenY: 4, \
            pageX: 5, pageY: 6, \
            radiusX: 7, radiusY: 8, rotationAngle: 90, force: 0.5 }); \
         (t.identifier === 7 && t.clientX === 1.5 && t.clientY === 2.5 && \
          t.screenX === 3 && t.screenY === 4 && t.pageX === 5 && \
          t.pageY === 6 && t.radiusX === 7 && t.radiusY === 8 && \
          t.rotationAngle === 90 && t.force === 0.5) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn touch_target_brand_check() {
    // Plain object rejected.
    let out = run("var threw = false; \
         try { new Touch({ identifier: 1, target: {} }); } \
         catch (e) { threw = (e instanceof TypeError); } \
         threw ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn touch_list_illegal_ctor() {
    let out = run("var threw = false; \
         try { new TouchList(); } \
         catch (e) { threw = (e instanceof TypeError); } \
         threw ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn touch_event_default_touches_empty_list() {
    let out = run("var e = new TouchEvent('touchstart'); \
         (e.touches instanceof TouchList && \
          e.touches.length === 0 && \
          e.targetTouches.length === 0 && \
          e.changedTouches.length === 0) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn touch_event_brand_check() {
    let out = run("var e = new TouchEvent('touchstart'); \
         (e instanceof TouchEvent && \
          e instanceof UIEvent && e instanceof Event) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn touch_event_prototype_chain() {
    let out = run(
        "(Object.getPrototypeOf(new TouchEvent('t')) === TouchEvent.prototype \
         && Object.getPrototypeOf(TouchEvent.prototype) === UIEvent.prototype) \
         ? 'ok' : 'fail';",
    );
    assert_eq!(out, "ok");
}

#[test]
fn touch_event_sequence_brand_check() {
    // sequence<Touch> entries must be Touch-brand.
    let out = run("var threw = false; \
         try { new TouchEvent('t', { touches: [{}] }); } \
         catch (e) { threw = (e instanceof TypeError); } \
         threw ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn touch_event_sequence_round_trip() {
    let out = run(
        "var t1 = new Touch({ identifier: 1, target: document.body }); \
         var t2 = new Touch({ identifier: 2, target: document.body }); \
         var e = new TouchEvent('t', { touches: [t1, t2] }); \
         (e.touches.length === 2 && \
          e.touches.item(0) === t1 && e.touches.item(1) === t2) ? 'ok' : 'fail';",
    );
    assert_eq!(out, "ok");
}

#[test]
fn touch_event_modifier_keys_round_trip() {
    let out = run("var e = new TouchEvent('t', { \
            ctrlKey: true, shiftKey: true, altKey: false, metaKey: true }); \
         (e.ctrlKey === true && e.shiftKey === true && \
          e.altKey === false && e.metaKey === true) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn touch_event_modifier_keys_default_false() {
    let out = run("var e = new TouchEvent('t'); \
         (e.ctrlKey === false && e.shiftKey === false && \
          e.altKey === false && e.metaKey === false) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn touch_list_item_out_of_range_null() {
    let out = run("var e = new TouchEvent('t'); \
         (e.touches.item(99) === null) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// =====================================================================
// §E2. sequence<Touch> iterator protocol (Copilot R2-R4 regressions)
// =====================================================================

#[test]
fn touch_event_sequence_accepts_iterable_non_array() {
    // Regression: `sequence<Touch>` falls back to the iterator
    // protocol for non-Array iterables per WebIDL §3.2.27.  Build a
    // custom iterable that yields one Touch via `Symbol.iterator`.
    let out = run(
        "var t = new Touch({ identifier: 1, target: document.body }); \
         var iterable = {}; \
         iterable[Symbol.iterator] = function() { \
             var done = false; \
             return { next: function() { \
                 if (done) return { value: undefined, done: true }; \
                 done = true; \
                 return { value: t, done: false }; \
             } }; \
         }; \
         var ev = new TouchEvent('touchstart', { touches: iterable }); \
         (ev.touches.length === 1 && ev.touches.item(0) === t) \
            ? 'ok' : 'fail';",
    );
    assert_eq!(out, "ok");
}

#[test]
fn touch_event_sequence_honours_array_iterator_override() {
    // Regression: WebIDL §3.2.27 sequence<T> conversion always goes
    // through `@@iterator`.  An Array with overridden
    // `Symbol.iterator` must use the override, not the dense-elements
    // fast-path (which previously bypassed the iterator protocol).
    let out = run(
        "var t = new Touch({ identifier: 7, target: document.body }); \
         var arr = [t, t, t]; \
         arr[Symbol.iterator] = function() { \
             var i = 0; \
             return { next: function() { \
                 if (i >= 1) return { value: undefined, done: true }; \
                 i++; \
                 return { value: t, done: false }; \
             } }; \
         }; \
         var ev = new TouchEvent('touchstart', { touches: arr }); \
         /* override yields only 1 item, not 3 */ \
         (ev.touches.length === 1) ? 'ok' : 'fail';",
    );
    assert_eq!(out, "ok");
}

#[test]
fn touch_event_sequence_accepts_empty_string_as_iterable() {
    // WebIDL §3.2.27: `sequence<T>` accepts any iterable; JS strings
    // are iterable via `String.prototype[@@iterator]`.  Empty string
    // yields no entries, so the resulting list is empty.
    let out = run("var ev = new TouchEvent('touchstart', { touches: '' }); \
         (ev.touches.length === 0) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn touch_event_sequence_rejects_non_iterable() {
    let out = run("var threw = false; \
         try { new TouchEvent('t', { touches: {} }); } \
         catch (e) { threw = (e instanceof TypeError); } \
         threw ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// =====================================================================
// §G. Cross-cluster verifications (Touch-flavoured)
// =====================================================================

#[test]
fn touch_not_event() {
    let out = run(
        "var t = new Touch({ identifier: 1, target: document.body }); \
         (t instanceof Event) ? 'fail' : 'ok';",
    );
    assert_eq!(out, "ok");
}

#[test]
fn touch_event_propagates_event_init() {
    let out = run(
        "var e = new TouchEvent('t', { bubbles: true, cancelable: true }); \
         (e.bubbles === true && e.cancelable === true) ? 'ok' : 'fail';",
    );
    assert_eq!(out, "ok");
}

// =====================================================================
// §H. Post-unbind tolerance — Touch wrapper (Copilot R2 regression)
// =====================================================================

#[test]
fn touch_post_unbind_reads_inert_defaults() {
    // Touch is immutable — only read-path tolerance is needed.
    // All numeric getters return 0; target returns null.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_empty_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval(
        "globalThis.t = new Touch({ identifier: 99, target: document.body, \
             clientX: 10, clientY: 20, force: 0.5 });",
    )
    .unwrap();
    vm.unbind();
    let result = vm
        .eval(
            "globalThis.t.identifier + '|' + \
             (globalThis.t.target === null) + '|' + \
             globalThis.t.clientX + '|' + globalThis.t.clientY + '|' + \
             globalThis.t.force;",
        )
        .unwrap();
    let JsValue::String(sid) = result else {
        panic!("expected string, got {result:?}");
    };
    let out = vm.inner.strings.get_utf8(sid);
    assert_eq!(out, "0|true|0|0|0");
}
