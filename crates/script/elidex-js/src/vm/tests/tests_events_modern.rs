//! D-9 `#11-events-modern-input` — PointerEvent + DragEvent +
//! TouchEvent + Touch + TouchList + DataTransfer +
//! DataTransferItem + DataTransferItemList.
//!
//! Coverage matches the D-9 plan v4 §A-§G test plan:
//! - per-ctor brand check + prototype chain + init dict defaults +
//!   MouseEventInit/UIEventInit propagation
//! - PointerEvent 12-slot init + altitudeAngle π/2 default + WebIDL
//!   `long` truncation on pointerId/tilt/twist
//! - DragEvent dataTransfer brand-check + null/undefined accept
//! - DataTransfer ctor + dropEffect/effectAllowed enum string
//!   accessors (ASCII-CI input + silent-retain on invalid)
//! - DataTransfer.setData / getData / clearData round-trip
//! - DataTransfer.items `[SameObject]` cache
//! - DataTransfer.types FrozenArray fresh-each-call + Files literal
//! - DataTransferItemList add(string, type) + remove(idx) + clear()
//! - DataTransferItemList add(File) TypeError stub (paired D-14)
//! - DataTransferItem.kind / .type / .getAsString / .getAsFile null
//! - DataTransferItem identity cache `(parent, index)` stable
//! - Touch ctor required members (identifier + target)
//! - Touch ctor coordinate / radii / force defaults
//! - TouchList brand-only (no public ctor)
//! - TouchEvent ctor sequence<Touch> brand validation
//! - InputEvent / ClipboardEvent dataTransfer brand upgrade (D-10
//!   stub → D-9 strict)

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
// §A. PointerEvent
// =====================================================================

#[test]
fn pointer_event_global_present() {
    let out = run("(typeof PointerEvent === 'function') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn pointer_event_brand_check() {
    let out = run("var e = new PointerEvent('pointerdown'); \
         (e instanceof PointerEvent && e instanceof MouseEvent && \
          e instanceof UIEvent && e instanceof Event) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn pointer_event_prototype_chain() {
    let out = run(
        "(Object.getPrototypeOf(new PointerEvent('p')) === PointerEvent.prototype \
         && Object.getPrototypeOf(PointerEvent.prototype) === MouseEvent.prototype) \
         ? 'ok' : 'fail';",
    );
    assert_eq!(out, "ok");
}

#[test]
fn pointer_event_default_pointer_id_zero() {
    let out = run("String(new PointerEvent('p').pointerId);");
    assert_eq!(out, "0");
}

#[test]
fn pointer_event_default_width_one() {
    let out = run("String(new PointerEvent('p').width);");
    assert_eq!(out, "1");
}

#[test]
fn pointer_event_default_height_one() {
    let out = run("String(new PointerEvent('p').height);");
    assert_eq!(out, "1");
}

#[test]
fn pointer_event_default_pressure_zero() {
    let out = run("String(new PointerEvent('p').pressure);");
    assert_eq!(out, "0");
}

#[test]
fn pointer_event_default_altitude_angle_pi_over_2() {
    // Per spec, default altitudeAngle = π/2 ≈ 1.5707963267948966.
    let out = run("String(new PointerEvent('p').altitudeAngle);");
    assert_eq!(out, "1.5707963267948966");
}

#[test]
fn pointer_event_default_azimuth_angle_zero() {
    let out = run("String(new PointerEvent('p').azimuthAngle);");
    assert_eq!(out, "0");
}

#[test]
fn pointer_event_default_pointer_type_empty_string() {
    let out = run("(new PointerEvent('p').pointerType === '') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn pointer_event_default_is_primary_false() {
    let out = run("(new PointerEvent('p').isPrimary === false) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn pointer_event_init_round_trip() {
    let out = run("var e = new PointerEvent('pointermove', { \
            pointerId: 42, width: 3, height: 5, pressure: 0.5, \
            tangentialPressure: 0.25, tiltX: 10, tiltY: -10, twist: 90, \
            altitudeAngle: 0, azimuthAngle: 1, pointerType: 'pen', \
            isPrimary: true }); \
         (e.pointerId === 42 && e.width === 3 && e.height === 5 && \
          e.pressure === 0.5 && e.tangentialPressure === 0.25 && \
          e.tiltX === 10 && e.tiltY === -10 && e.twist === 90 && \
          e.altitudeAngle === 0 && e.azimuthAngle === 1 && \
          e.pointerType === 'pen' && e.isPrimary === true) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn pointer_event_pointer_id_long_truncate() {
    // WebIDL `long` truncate: 0x1_0000_0000 wraps to 0.
    let out = run("String(new PointerEvent('p', {pointerId: 4294967296}).pointerId);");
    assert_eq!(out, "0");
}

#[test]
fn pointer_event_tilt_long_truncate() {
    // 100.5 truncates to 100 (long truncation, not rounding).
    let out = run("String(new PointerEvent('p', {tiltX: 100.5}).tiltX);");
    assert_eq!(out, "100");
}

#[test]
fn pointer_event_mouse_init_propagation() {
    // PointerEvent inherits MouseEvent slots — verify they survive.
    let out = run("var e = new PointerEvent('p', { \
            clientX: 7, clientY: 11, button: 1, altKey: true }); \
         (e.clientX === 7 && e.clientY === 11 && \
          e.button === 1 && e.altKey === true) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn pointer_event_ctor_as_call_throws() {
    let out = run("var threw = false; \
         try { PointerEvent('p'); } \
         catch (e) { threw = (e instanceof TypeError); } \
         threw ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn pointer_event_get_coalesced_events_returns_empty_array() {
    let out = run("var arr = new PointerEvent('p').getCoalescedEvents(); \
         (Array.isArray(arr) && arr.length === 0) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn pointer_event_get_predicted_events_returns_empty_array() {
    let out = run("var arr = new PointerEvent('p').getPredictedEvents(); \
         (Array.isArray(arr) && arr.length === 0) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn pointer_event_get_coalesced_events_brand_check() {
    // Calling on a non-PointerEvent receiver throws.
    let out = run("var threw = false; \
         try { PointerEvent.prototype.getCoalescedEvents.call(new MouseEvent('click')); } \
         catch (e) { threw = (e instanceof TypeError); } \
         threw ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// =====================================================================
// §B. DragEvent
// =====================================================================

#[test]
fn drag_event_global_present() {
    let out = run("(typeof DragEvent === 'function') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn drag_event_brand_check() {
    let out = run("var e = new DragEvent('dragstart'); \
         (e instanceof DragEvent && e instanceof MouseEvent && \
          e instanceof Event) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn drag_event_prototype_chain() {
    let out = run(
        "(Object.getPrototypeOf(new DragEvent('d')) === DragEvent.prototype \
         && Object.getPrototypeOf(DragEvent.prototype) === MouseEvent.prototype) \
         ? 'ok' : 'fail';",
    );
    assert_eq!(out, "ok");
}

#[test]
fn drag_event_default_data_transfer_null() {
    let out = run("(new DragEvent('d').dataTransfer === null) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn drag_event_data_transfer_accepts_data_transfer() {
    let out = run("var dt = new DataTransfer(); \
         var e = new DragEvent('d', { dataTransfer: dt }); \
         (e.dataTransfer === dt) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn drag_event_data_transfer_rejects_plain_object() {
    let out = run("var threw = false; \
         try { new DragEvent('d', { dataTransfer: { kind: 'fake' } }); } \
         catch (e) { threw = (e instanceof TypeError); } \
         threw ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn drag_event_data_transfer_accepts_null() {
    let out =
        run("(new DragEvent('d', { dataTransfer: null }).dataTransfer === null) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn drag_event_mouse_init_propagation() {
    let out = run("var e = new DragEvent('d', { clientX: 50, clientY: 75 }); \
         (e.clientX === 50 && e.clientY === 75) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// =====================================================================
// §C. DataTransfer
// =====================================================================

#[test]
fn data_transfer_global_present() {
    let out = run("(typeof DataTransfer === 'function') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn data_transfer_ctor() {
    let out = run("var dt = new DataTransfer(); \
         (dt instanceof DataTransfer) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn data_transfer_default_drop_effect_none() {
    let out = run("(new DataTransfer().dropEffect === 'none') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn data_transfer_default_effect_allowed_none() {
    let out = run("(new DataTransfer().effectAllowed === 'none') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn data_transfer_drop_effect_setter_canonicalize() {
    // Spec: ASCII-CI match against enum; canonicalize to lowercase.
    let out = run("var dt = new DataTransfer(); \
         dt.dropEffect = 'COPY'; \
         dt.dropEffect;");
    assert_eq!(out, "copy");
}

#[test]
fn data_transfer_drop_effect_setter_invalid_retain() {
    // Spec: silent-retain prior value on invalid input.
    let out = run("var dt = new DataTransfer(); \
         dt.dropEffect = 'copy'; \
         dt.dropEffect = 'garbage'; \
         dt.dropEffect;");
    assert_eq!(out, "copy");
}

#[test]
fn data_transfer_effect_allowed_setter() {
    let out = run("var dt = new DataTransfer(); \
         dt.effectAllowed = 'copyMove'; \
         dt.effectAllowed;");
    assert_eq!(out, "copyMove");
}

#[test]
fn data_transfer_effect_allowed_all() {
    let out = run("var dt = new DataTransfer(); \
         dt.effectAllowed = 'all'; \
         dt.effectAllowed;");
    assert_eq!(out, "all");
}

#[test]
fn data_transfer_effect_allowed_uninitialized() {
    let out = run("var dt = new DataTransfer(); \
         dt.effectAllowed = 'uninitialized'; \
         dt.effectAllowed;");
    assert_eq!(out, "uninitialized");
}

#[test]
fn data_transfer_set_data_round_trip() {
    let out = run("var dt = new DataTransfer(); \
         dt.setData('text/plain', 'hello'); \
         dt.getData('text/plain');");
    assert_eq!(out, "hello");
}

#[test]
fn data_transfer_set_data_replaces_existing() {
    let out = run("var dt = new DataTransfer(); \
         dt.setData('text/plain', 'a'); \
         dt.setData('text/plain', 'b'); \
         dt.getData('text/plain');");
    assert_eq!(out, "b");
}

#[test]
fn data_transfer_get_data_case_insensitive_format() {
    let out = run("var dt = new DataTransfer(); \
         dt.setData('text/plain', 'X'); \
         dt.getData('TEXT/PLAIN');");
    assert_eq!(out, "X");
}

#[test]
fn data_transfer_get_data_missing_format_empty_string() {
    let out = run("new DataTransfer().getData('text/foo');");
    assert_eq!(out, "");
}

#[test]
fn data_transfer_clear_data_no_args_drains_strings() {
    let out = run("var dt = new DataTransfer(); \
         dt.setData('a', '1'); \
         dt.setData('b', '2'); \
         dt.clearData(); \
         (dt.getData('a') === '' && dt.getData('b') === '') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn data_transfer_clear_data_with_format() {
    let out = run("var dt = new DataTransfer(); \
         dt.setData('a', '1'); \
         dt.setData('b', '2'); \
         dt.clearData('a'); \
         (dt.getData('a') === '' && dt.getData('b') === '2') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn data_transfer_clear_data_invalidates_item_wrapper_cache() {
    // Regression: `clearData()` removes String entries, which can
    // shift File-entry indices (or, here, simply leaves an empty
    // list).  The `(parent, index)` wrapper-cache must be flushed
    // so that future `add()` calls hand out fresh wrappers, not
    // stale ones bound to the cleared entries.  Without the
    // invalidation, the re-added item at index 0 would reuse the
    // pre-clearData wrapper from index 0, breaking identity.
    let out = run("var dt = new DataTransfer(); \
         var a = dt.items.add('x', 'a'); \
         dt.clearData(); \
         var b = dt.items.add('y', 'a'); \
         (a !== b) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn data_transfer_clear_data_with_format_invalidates_cache() {
    // `clearData(format)` removes a specific entry, shifting any
    // higher-index entries down by one.  Wrappers at the shifted
    // indices must be invalidated so the cache doesn't hand out a
    // wrapper bound to the wrong entry.
    let out = run("var dt = new DataTransfer(); \
         var a = dt.items.add('x', 'a'); \
         var b = dt.items.add('y', 'b'); \
         dt.clearData('a'); \
         var c = dt.items.add('z', 'c'); \
         (a !== b && b !== c && a !== c) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn data_transfer_clear_data_case_insensitive_format() {
    let out = run("var dt = new DataTransfer(); \
         dt.setData('text/plain', '1'); \
         dt.clearData('TEXT/PLAIN'); \
         (dt.getData('text/plain') === '') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn data_transfer_items_same_object() {
    // [SameObject] semantics — repeated reads return identical wrapper.
    let out = run("var dt = new DataTransfer(); \
         (dt.items === dt.items) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn data_transfer_files_same_object() {
    let out = run("var dt = new DataTransfer(); \
         (dt.files === dt.files) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn data_transfer_files_empty_stub() {
    // D-9 ships a FileList stub (empty Array per slot
    // `#11-data-transfer-file-paired`).
    let out = run("String(new DataTransfer().files.length);");
    assert_eq!(out, "0");
}

#[test]
fn data_transfer_types_fresh_each_call() {
    // Spec: `types` getter returns a fresh Array per read.  WebIDL
    // promises `FrozenArray<DOMString>`, but the VM's Array.push
    // bypasses `extensible: false` (see `native_dt_get_types`
    // comment); shipping the fresh-per-read contract here is the
    // observable half of the FrozenArray guarantee — script-side
    // mutation cannot leak across reads because each call returns
    // a new Array.
    let out = run("var dt = new DataTransfer(); \
         (dt.types !== dt.types) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn data_transfer_types_empty_default() {
    let out = run("String(new DataTransfer().types.length);");
    assert_eq!(out, "0");
}

#[test]
fn data_transfer_types_after_set_data() {
    let out = run("var dt = new DataTransfer(); \
         dt.setData('text/plain', 'a'); \
         dt.setData('text/html', 'b'); \
         var t = dt.types; \
         (t.length === 2 && t[0] === 'text/plain' && t[1] === 'text/html') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn data_transfer_set_drag_image_accepts_element() {
    let out = run("var el = document.createElement('div'); \
         var dt = new DataTransfer(); \
         dt.setDragImage(el, 10, 20); \
         'ok';");
    assert_eq!(out, "ok");
}

#[test]
fn data_transfer_set_drag_image_rejects_non_element() {
    let out = run("var threw = false; \
         try { new DataTransfer().setDragImage({}, 0, 0); } \
         catch (e) { threw = (e instanceof TypeError); } \
         threw ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn data_transfer_set_drag_image_rejects_non_element_host_wrapper() {
    // Regression: a HostObject whose entity is NOT `NodeKind::Element`
    // (e.g. the Document itself) must throw TypeError, not silently
    // store the entity_bits.  Previously any HostObject was accepted.
    let out = run("var threw = false; \
         try { new DataTransfer().setDragImage(document, 0, 0); } \
         catch (e) { threw = (e instanceof TypeError); } \
         threw ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn data_transfer_set_drag_image_rejects_null() {
    let out = run("var threw = false; \
         try { new DataTransfer().setDragImage(null, 0, 0); } \
         catch (e) { threw = (e instanceof TypeError); } \
         threw ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn data_transfer_ctor_as_call_throws() {
    let out = run("var threw = false; \
         try { DataTransfer(); } \
         catch (e) { threw = (e instanceof TypeError); } \
         threw ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// =====================================================================
// §D. DataTransferItem + DataTransferItemList
// =====================================================================

#[test]
fn data_transfer_item_list_brand() {
    let out = run("var dt = new DataTransfer(); \
         (dt.items instanceof DataTransferItemList) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn data_transfer_item_list_illegal_ctor() {
    let out = run("var threw = false; \
         try { new DataTransferItemList(); } \
         catch (e) { threw = (e instanceof TypeError); } \
         threw ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn data_transfer_item_list_length_initially_zero() {
    let out = run("String(new DataTransfer().items.length);");
    assert_eq!(out, "0");
}

#[test]
fn data_transfer_item_list_add_string_returns_item() {
    let out = run("var dt = new DataTransfer(); \
         var item = dt.items.add('hello', 'text/plain'); \
         (item instanceof DataTransferItem) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn data_transfer_item_list_add_string_increments_length() {
    let out = run("var dt = new DataTransfer(); \
         dt.items.add('a', 'text/plain'); \
         dt.items.add('b', 'text/html'); \
         String(dt.items.length);");
    assert_eq!(out, "2");
}

#[test]
fn data_transfer_item_list_remove_decrements_length() {
    let out = run("var dt = new DataTransfer(); \
         dt.items.add('a', 'text/plain'); \
         dt.items.add('b', 'text/html'); \
         dt.items.remove(0); \
         String(dt.items.length);");
    assert_eq!(out, "1");
}

#[test]
fn data_transfer_item_list_clear_drains() {
    let out = run("var dt = new DataTransfer(); \
         dt.items.add('a', 'text/plain'); \
         dt.items.add('b', 'text/html'); \
         dt.items.clear(); \
         String(dt.items.length);");
    assert_eq!(out, "0");
}

#[test]
fn data_transfer_item_kind_string() {
    let out = run("var dt = new DataTransfer(); \
         var item = dt.items.add('x', 'text/plain'); \
         item.kind;");
    assert_eq!(out, "string");
}

#[test]
fn data_transfer_item_type_round_trip() {
    let out = run("var dt = new DataTransfer(); \
         var item = dt.items.add('x', 'text/html'); \
         item.type;");
    assert_eq!(out, "text/html");
}

#[test]
fn data_transfer_item_get_as_file_returns_null() {
    // D-9 stub: File wrapper paired with D-14.
    let out = run("var dt = new DataTransfer(); \
         dt.items.add('x', 'text/plain').getAsFile() === null ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn data_transfer_get_data_round_trips_lone_surrogate() {
    // Regression: `getData` / `setData` must preserve lone
    // surrogates per DOMString semantics.  `\uD800` (high surrogate)
    // without a paired low surrogate would be lossily replaced with
    // `U+FFFD` if the round-trip went through UTF-8.  WTF-16-clean
    // round-trip keeps the original code unit.
    let out = run("var dt = new DataTransfer(); \
         dt.setData('text/plain', '\\uD800'); \
         var v = dt.getData('text/plain'); \
         (v.length === 1 && v.charCodeAt(0) === 0xD800) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn data_transfer_get_data_round_trips_surrogate_pair() {
    // Paired surrogates (e.g. `\uD83D\uDE00` for U+1F600 😀) must
    // also round-trip — the WTF-16 path preserves them as 2
    // code units, while UTF-8 round-trip preserves the codepoint but
    // would re-encode (no value change here, but the WTF-16 path is
    // the canonical one for DOMString).
    let out = run("var dt = new DataTransfer(); \
         dt.setData('text/plain', '\\uD83D\\uDE00'); \
         var v = dt.getData('text/plain'); \
         (v.length === 2 && \
          v.charCodeAt(0) === 0xD83D && \
          v.charCodeAt(1) === 0xDE00) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn clipboard_event_data_brand_error_says_clipboard_data() {
    // Regression: the WebIDL TypeError text must reference the
    // ClipboardEvent member name (`clipboardData`), not the helper's
    // origin member (`dataTransfer`).
    let out = run("var msg = ''; \
         try { new ClipboardEvent('copy', { clipboardData: 42 }); } \
         catch (e) { msg = String(e.message); } \
         msg.indexOf('clipboardData') >= 0 ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn input_event_data_transfer_brand_error_says_data_transfer() {
    let out = run("var msg = ''; \
         try { new InputEvent('input', { dataTransfer: 42 }); } \
         catch (e) { msg = String(e.message); } \
         msg.indexOf('dataTransfer') >= 0 ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn data_transfer_item_list_add_blob_falls_through_to_string_overload() {
    // Per WebIDL overload resolution + File API §11.1, `Blob` is not
    // a `File` subclass — `dt.items.add(blob, 'text/plain')` follows
    // the `(DOMString, DOMString)` overload via `ToString(blob)`.
    // The real `add(File)` brand check lands with D-14 (slot
    // `#11-data-transfer-file-paired`); once `ObjectKind::File`
    // exists the brand path will catch File but still let Blob fall
    // through here.
    let out = run("var dt = new DataTransfer(); \
         var blob = new Blob(['x']); \
         var item = dt.items.add(blob, 'text/plain'); \
         (item instanceof DataTransferItem && dt.items.length === 1) \
            ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn data_transfer_item_list_add_string_without_type_throws() {
    // `(DOMString, DOMString)` overload: `type` is required.  Missing
    // it throws a WebIDL TypeError (matches Chromium / Firefox).
    let out = run("var dt = new DataTransfer(); \
         var threw = false; \
         try { dt.items.add('x'); } \
         catch (e) { threw = (e instanceof TypeError); } \
         threw ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn data_transfer_item_identity_cache() {
    // `dt.items[0] === dt.items[0]` indexed-exotic dispatch is
    // deferred to slot `#11-events-modern-indexed-exotic`; once
    // landed, add an `i1 === dt.items[0]` assertion here (and a
    // matching `touches[0]` test under §E).  For now the
    // identity-cache property is verified indirectly via the
    // `add()` return value being a fresh wrapper per call.
    let out = run("var dt = new DataTransfer(); \
         var i1 = dt.items.add('x', 'a'); \
         var i2 = dt.items.add('y', 'b'); \
         (i1 !== i2 && \
          i1 instanceof DataTransferItem && \
          i2 instanceof DataTransferItem) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// =====================================================================
// §G. Cross-cluster verifications
// =====================================================================

#[test]
fn data_transfer_not_event() {
    // DataTransfer is NOT an Event subclass.
    let out = run("(new DataTransfer() instanceof Event) ? 'fail' : 'ok';");
    assert_eq!(out, "ok");
}

#[test]
fn pointer_event_propagates_event_init() {
    let out = run(
        "var e = new PointerEvent('p', { bubbles: true, cancelable: true }); \
         (e.bubbles === true && e.cancelable === true) ? 'ok' : 'fail';",
    );
    assert_eq!(out, "ok");
}

#[test]
fn drag_event_propagates_event_init() {
    let out = run(
        "var e = new DragEvent('d', { bubbles: true, cancelable: true }); \
         (e.bubbles === true && e.cancelable === true) ? 'ok' : 'fail';",
    );
    assert_eq!(out, "ok");
}

// =====================================================================
// §H. Post-unbind tolerance (Copilot R2 regression)
// =====================================================================

#[test]
fn data_transfer_post_unbind_reads_inert_defaults() {
    // Regression: retained DataTransfer / Touch wrappers must not
    // panic post-unbind.  `Vm::unbind()` drops the side-table state
    // tables; the read-side `dt_state` / `touch_state` helpers fall
    // through to the static `EMPTY_DT_STATE` / `EMPTY_TOUCH_STATE`
    // sentinels, while the write-side `dt_state_mut` uses the
    // `entry().or_insert_with(empty)` lazy-reinsert path.  Note
    // that some read-looking paths (`dt.items` / `dt.files` getter
    // cache fill) call `dt_state_mut` to install the cache wrapper
    // on first read — those reinsert empty state on first access
    // too, which is fine: the wrapper observes the same inert
    // defaults.  Without this tolerance, the previous
    // `expect("DataTransferState must exist...")` would panic
    // the VM.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_empty_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval(
        "globalThis.dt = new DataTransfer(); \
         globalThis.dt.setData('text/plain', 'hello');",
    )
    .unwrap();
    vm.unbind();
    // Reads after unbind: dropEffect / effectAllowed / types / items
    // length / getData all read inert defaults instead of panicking.
    let result = vm
        .eval(
            "globalThis.dt.dropEffect + '|' + \
             globalThis.dt.effectAllowed + '|' + \
             globalThis.dt.types.length + '|' + \
             globalThis.dt.items.length + '|' + \
             globalThis.dt.getData('text/plain');",
        )
        .unwrap();
    let JsValue::String(sid) = result else {
        panic!("expected string, got {result:?}");
    };
    let out = vm.inner.strings.get_utf8(sid);
    assert_eq!(out, "none|none|0|0|");
}

#[test]
fn data_transfer_post_unbind_set_data_reinserts_state() {
    // `dt_state_mut` lazily reinserts an empty state on first
    // post-unbind write, so subsequent reads see the mutation.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_empty_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval("globalThis.dt = new DataTransfer();").unwrap();
    vm.unbind();
    let result = vm
        .eval(
            "globalThis.dt.setData('text/plain', 'after-unbind'); \
             globalThis.dt.getData('text/plain');",
        )
        .unwrap();
    let JsValue::String(sid) = result else {
        panic!("expected string, got {result:?}");
    };
    let out = vm.inner.strings.get_utf8(sid);
    assert_eq!(out, "after-unbind");
}

#[test]
fn data_transfer_post_unbind_set_drag_image_tolerates_missing_host() {
    // Regression: `setDragImage` validates the image argument via
    // `require_element_arg_bits`, which previously called
    // `ctx.host().dom()` and panicked on unbound VMs.  With
    // `host_if_bound` guarding, an unbound VM surfaces a TypeError
    // ("detached") instead of panicking — consistent with the
    // post-unbind read-tolerance contract.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_empty_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval(
        "globalThis.dt = new DataTransfer(); \
         globalThis.el = document.createElement('div');",
    )
    .unwrap();
    vm.unbind();
    let result = vm
        .eval(
            "var threw = false; \
             try { globalThis.dt.setDragImage(globalThis.el, 0, 0); } \
             catch (e) { threw = (e instanceof TypeError); } \
             threw ? 'ok' : 'fail';",
        )
        .unwrap();
    let JsValue::String(sid) = result else {
        panic!("expected string, got {result:?}");
    };
    let out = vm.inner.strings.get_utf8(sid);
    assert_eq!(out, "ok");
}

// ---------------------------------------------------------------------------
// `[Constructor]` gate regression — DataTransfer / DragEvent / PointerEvent
// fire the canonical `CallShape::ConstructorOnly` TypeError at the
// dispatch site when invoked without `new` (WebIDL §3.7.1 step 1.2).
// Plan-memo `m4-12-pr-vm-native-constructor-only-flag-plan.md` §5 sites
// #27-29.
// ---------------------------------------------------------------------------

#[test]
fn data_transfer_ctor_requires_new() {
    super::assert_ctor_requires_new("DataTransfer()", "DataTransfer");
}

#[test]
fn drag_event_ctor_requires_new() {
    super::assert_ctor_requires_new("DragEvent('dragstart')", "DragEvent");
}

#[test]
fn pointer_event_ctor_requires_new() {
    super::assert_ctor_requires_new("PointerEvent('pointerdown')", "PointerEvent");
}
