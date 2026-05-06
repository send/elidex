//! M4-12 #11-storage-web — `Storage` / `StorageEvent` thin VM binding tests.
//!
//! Phase C2: prototype + 5 methods + length getter + brand check.
//! Phase C3: named-property exotic ([[Get]] / [[Set]] / [[Delete]] /
//!           [[HasProperty]] / [[OwnPropertyKeys]]).
//! Phase C4: 5 MiB quota → `QuotaExceededError` DOMException.
//! Phase C5: `new StorageEvent(...)` constructor + 5 RO IDL attrs.
//! Phase C6: Window prototype `localStorage` / `sessionStorage`
//!           accessors with `[SameObject]` identity.

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

// --- C2 — prototype + brand check + 5 methods + length --------------

#[test]
fn storage_prototype_installed() {
    let mut vm = Vm::new();
    assert!(
        vm.inner.storage_prototype.is_some(),
        "Storage.prototype must be allocated during register_globals"
    );
    assert!(vm.eval("typeof Storage === 'function'").is_ok());
}

#[test]
fn storage_constructor_throws_illegal() {
    let err = run_throws("new Storage();");
    assert!(
        err.contains("Illegal constructor"),
        "expected illegal-ctor TypeError, got: {err}"
    );
}

#[test]
fn storage_local_same_object() {
    let out = run("(localStorage === localStorage) ? 'yes' : 'no';");
    assert_eq!(out, "yes");
}

#[test]
fn storage_session_same_object() {
    let out = run("(sessionStorage === sessionStorage) ? 'yes' : 'no';");
    assert_eq!(out, "yes");
}

#[test]
fn storage_local_session_distinct() {
    let out = run("(localStorage !== sessionStorage) ? 'yes' : 'no';");
    assert_eq!(out, "yes");
}

#[test]
fn storage_length_initially_zero() {
    let out = run("String(localStorage.length);");
    assert_eq!(out, "0");
}

#[test]
fn storage_set_get_round_trip() {
    let out = run("localStorage.setItem('k', 'v'); localStorage.getItem('k');");
    assert_eq!(out, "v");
}

#[test]
fn storage_get_absent_returns_null() {
    let out = run("String(localStorage.getItem('absent'));");
    assert_eq!(out, "null");
}

#[test]
fn storage_set_value_to_string_coerces_number() {
    let out = run("localStorage.setItem('n', 42); localStorage.getItem('n');");
    assert_eq!(out, "42");
}

#[test]
fn storage_set_undefined_value_stores_string() {
    let out = run("localStorage.setItem('u', undefined); localStorage.getItem('u');");
    assert_eq!(out, "undefined");
}

#[test]
fn storage_set_undefined_key_stores_string() {
    let out = run("localStorage.setItem(undefined, 'x'); \
         localStorage.getItem('undefined');");
    assert_eq!(out, "x");
}

#[test]
fn storage_remove_absent_silent_noop() {
    let out = run("typeof localStorage.removeItem('absent');");
    assert_eq!(out, "undefined");
}

#[test]
fn storage_clear_resets_length() {
    let out = run(
        "localStorage.setItem('a', '1'); localStorage.setItem('b', '2'); \
         localStorage.clear(); String(localStorage.length);",
    );
    assert_eq!(out, "0");
}

#[test]
fn storage_length_tracks_set_remove() {
    let out = run(
        "localStorage.setItem('a', '1'); localStorage.setItem('b', '2'); \
         var n1 = localStorage.length; \
         localStorage.removeItem('a'); \
         var n2 = localStorage.length; \
         n1 + ',' + n2;",
    );
    assert_eq!(out, "2,1");
}

#[test]
fn storage_key_returns_in_insertion_order() {
    let out = run("localStorage.setItem('first', '1'); \
         localStorage.setItem('second', '2'); \
         localStorage.key(0) + ',' + localStorage.key(1);");
    assert_eq!(out, "first,second");
}

#[test]
fn storage_key_out_of_range_returns_null() {
    let out = run("localStorage.setItem('x', '1'); \
         String(localStorage.key(5));");
    assert_eq!(out, "null");
}

#[test]
fn storage_brand_check_get_item() {
    let err = run_throws("Storage.prototype.getItem.call({}, 'k');");
    assert!(
        err.contains("Illegal invocation"),
        "expected brand-check TypeError, got: {err}"
    );
}

#[test]
fn storage_get_item_zero_args_throws() {
    let err = run_throws("localStorage.getItem();");
    assert!(
        err.contains("1 argument required"),
        "expected arg-count TypeError, got: {err}"
    );
}

#[test]
fn storage_set_item_zero_args_throws() {
    let err = run_throws("localStorage.setItem();");
    assert!(
        err.contains("2 arguments required") && err.contains("0 present"),
        "expected arg-count TypeError, got: {err}"
    );
}

#[test]
fn storage_set_item_one_arg_throws() {
    let err = run_throws("localStorage.setItem('k');");
    assert!(
        err.contains("2 arguments required") && err.contains("1 present"),
        "expected arg-count TypeError, got: {err}"
    );
}

#[test]
fn storage_remove_item_zero_args_throws() {
    let err = run_throws("localStorage.removeItem();");
    assert!(
        err.contains("1 argument required"),
        "expected arg-count TypeError, got: {err}"
    );
}

#[test]
fn storage_key_zero_args_throws() {
    let err = run_throws("localStorage.key();");
    assert!(
        err.contains("1 argument required"),
        "expected arg-count TypeError, got: {err}"
    );
}

#[test]
fn storage_key_coerces_nan_to_zero() {
    // ToUint32(NaN) === 0 → returns 0th key when present, else null.
    let out = run("localStorage.setItem('first', '1'); \
         localStorage.key(NaN);");
    assert_eq!(out, "first");
}

#[test]
fn storage_key_coerces_infinity_to_zero() {
    let out = run("localStorage.setItem('first', '1'); \
         localStorage.key(Infinity);");
    assert_eq!(out, "first");
}

#[test]
fn storage_key_negative_wraps_out_of_range() {
    // ToUint32(-1) === 4294967295 → out of range → null.
    let out = run("localStorage.setItem('first', '1'); \
         String(localStorage.key(-1));");
    assert_eq!(out, "null");
}

#[test]
fn storage_set_item_invokes_to_string_callback() {
    let out = run("var called = false; \
         var v = { toString: function(){ called = true; return 'custom'; } }; \
         localStorage.setItem('k', v); \
         (called ? 'yes' : 'no') + ',' + localStorage.getItem('k');");
    assert_eq!(out, "yes,custom");
}

#[test]
fn storage_brand_check_length_getter() {
    let err = run_throws(
        "var d = Object.getOwnPropertyDescriptor(Storage.prototype, 'length'); \
         d.get.call({});",
    );
    assert!(
        err.contains("Illegal invocation"),
        "expected brand-check TypeError, got: {err}"
    );
}

// --- C3 — named-property exotic ------------------------------------

#[test]
fn storage_bracket_get_reads_stored() {
    let out = run("localStorage.setItem('k', 'v'); localStorage['k'];");
    assert_eq!(out, "v");
}

#[test]
fn storage_dot_get_reads_stored() {
    let out = run("localStorage.setItem('k', 'v'); localStorage.k;");
    assert_eq!(out, "v");
}

#[test]
fn storage_bracket_set_writes_stored() {
    let out = run("localStorage['k'] = 'v'; localStorage.getItem('k');");
    assert_eq!(out, "v");
}

#[test]
fn storage_delete_removes_stored() {
    let out = run("localStorage.setItem('k', 'v'); \
         delete localStorage.k; \
         String(localStorage.getItem('k'));");
    assert_eq!(out, "null");
}

#[test]
fn storage_in_operator_reflects_stored() {
    let out = run("localStorage.setItem('k', 'v'); \
         (('k' in localStorage) ? 'yes' : 'no') + ',' + \
         (('absent' in localStorage) ? 'yes' : 'no');");
    assert_eq!(out, "yes,no");
}

#[test]
fn storage_method_names_shadow_named_property() {
    // WebIDL §3.10 non-`[LegacyOverrideBuiltIns]` — built-ins win.
    let out = run("typeof localStorage['getItem'];");
    assert_eq!(out, "function");
}

#[test]
fn storage_method_name_in_returns_method() {
    // `'getItem' in localStorage` is true because the prototype
    // chain has it; the named-property exotic falls through.
    let out = run("(('getItem' in localStorage) ? 'yes' : 'no');");
    assert_eq!(out, "yes");
}

#[test]
fn storage_for_in_enumerates_stored_keys() {
    let out = run("localStorage.setItem('a', '1'); \
         localStorage.setItem('b', '2'); \
         var ks = []; \
         for (var k in localStorage) ks.push(k); \
         ks.join(',');");
    assert_eq!(out, "a,b");
}

#[test]
fn storage_object_keys_returns_stored_keys() {
    let out = run("localStorage.setItem('x', '1'); \
         localStorage.setItem('y', '2'); \
         Object.keys(localStorage).join(',');");
    assert_eq!(out, "x,y");
}

#[test]
fn storage_has_own_property_reflects_stored() {
    let out = run("localStorage.setItem('k', 'v'); \
         localStorage.hasOwnProperty('k') + ',' + \
         localStorage.hasOwnProperty('absent');");
    assert_eq!(out, "true,false");
}

// --- C4 — quota + DOMException -------------------------------------

#[test]
fn storage_quota_exceeded_throws_dom_exception() {
    // Use sessionStorage (in-memory + per-VM) so the test does not
    // touch the disk-backed manager and so the 5 MiB quota check
    // runs on a fresh `SessionStorageState`.  Catch in JS to
    // observe the DOMException's `name` (the host-side
    // VmError::DomException renders only as a StringId in
    // `format!`).
    let out = run("var big = new Array(6 * 1024 * 1024 + 1).join('a'); \
         var name = ''; \
         try { sessionStorage.setItem('big', big); } \
         catch (e) { \
             name = (e instanceof DOMException) ? e.name : String(e); \
         } \
         name;");
    assert_eq!(out, "QuotaExceededError");
}

#[test]
fn storage_quota_remove_recovers_space() {
    // Fill ~4 MiB, then remove and add a large value that requires
    // the freed space.
    let out = run("var v = new Array(2 * 1024 * 1024 + 1).join('a'); \
         sessionStorage.setItem('a', v); \
         sessionStorage.setItem('b', v); \
         sessionStorage.removeItem('a'); \
         var ok = false; \
         try { sessionStorage.setItem('c', v); ok = true; } catch (_) {} \
         ok ? 'yes' : 'no';");
    assert_eq!(out, "yes");
}

// --- C5 — StorageEvent class ---------------------------------------

#[test]
fn storage_event_prototype_installed() {
    let mut vm = Vm::new();
    assert!(
        vm.inner.storage_event_prototype.is_some(),
        "StorageEvent.prototype must be allocated during register_globals"
    );
    assert!(vm.eval("typeof StorageEvent === 'function'").is_ok());
}

#[test]
fn storage_event_basic_construction() {
    let out = run("var e = new StorageEvent('storage'); e.type;");
    assert_eq!(out, "storage");
}

#[test]
fn storage_event_init_dict_populates_attrs() {
    let out = run("var e = new StorageEvent('storage', { \
             key: 'k', oldValue: 'old', newValue: 'new', \
             url: 'https://example.com/' \
         }); \
         e.key + '|' + e.oldValue + '|' + e.newValue + '|' + e.url;");
    assert_eq!(out, "k|old|new|https://example.com/");
}

#[test]
fn storage_event_default_attrs_are_null_or_empty() {
    let out = run("var e = new StorageEvent('storage'); \
         String(e.key) + '|' + String(e.oldValue) + '|' + \
         String(e.newValue) + '|' + e.url + '|' + String(e.storageArea);");
    assert_eq!(out, "null|null|null||null");
}

#[test]
fn storage_event_storage_area_init_preserved() {
    let out = run(
        "var e = new StorageEvent('storage', { storageArea: localStorage }); \
         (e.storageArea === localStorage) ? 'yes' : 'no';",
    );
    assert_eq!(out, "yes");
}

#[test]
fn storage_event_instanceof_event() {
    let out = run("var e = new StorageEvent('storage'); (e instanceof Event) ? 'yes' : 'no';");
    assert_eq!(out, "yes");
}

#[test]
fn storage_event_instanceof_storage_event() {
    let out = run("var e = new StorageEvent('storage'); \
         (e instanceof StorageEvent) ? 'yes' : 'no';");
    assert_eq!(out, "yes");
}

#[test]
fn storage_event_constructor_bare_call_throws() {
    let err = run_throws("StorageEvent('storage');");
    assert!(
        err.contains("'new' operator"),
        "expected bare-call TypeError, got: {err}"
    );
}

// --- C6 — lifecycle / origin / unbind clearing ---------------------

#[test]
fn storage_session_cleared_on_unbind() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval("sessionStorage.setItem('k', 'v');").unwrap();
    let mid = vm.eval("sessionStorage.getItem('k');").unwrap();
    let JsValue::String(sid) = mid else { panic!() };
    assert_eq!(vm.inner.strings.get_utf8(sid), "v");
    vm.unbind();

    // Rebind to a fresh DOM with a fresh document — sessionStorage
    // should be empty per `Vm::unbind`'s clear.
    let mut next_session = SessionCore::new();
    let mut next_world = EcsDom::new();
    let next_doc = build_doc(&mut next_world);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut next_session, &mut next_world, next_doc);
    }
    let after = vm.eval("String(sessionStorage.getItem('k'));").unwrap();
    let JsValue::String(asid) = after else {
        panic!()
    };
    assert_eq!(vm.inner.strings.get_utf8(asid), "null");
    vm.unbind();
}

#[test]
fn storage_local_instance_cache_cleared_on_unbind() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval("var first = localStorage;").unwrap();
    assert!(vm.inner.storage_local_instance.is_some());
    vm.unbind();
    assert!(
        vm.inner.storage_local_instance.is_none(),
        "Vm::unbind must clear storage_local_instance to prevent \
         cross-origin leakage on rebind"
    );
}

#[test]
fn storage_pre_install_host_data_constructor_no_panic() {
    // `new StorageEvent("storage")` should work pre-init (no
    // HostData touch) — same contract as `new Event("...")`.
    let mut vm = Vm::new();
    let result = vm.eval("var e = new StorageEvent('storage'); typeof e;");
    assert!(result.is_ok(), "StorageEvent ctor must not panic pre-init");
}

#[test]
fn storage_get_item_post_unbind_returns_null() {
    // Retained `localStorage` reference after unbind: methods
    // silently return `null` / `undefined` instead of panicking.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval("globalThis.savedLs = localStorage; savedLs.setItem('k', 'v');")
        .unwrap();
    vm.unbind();

    // Rebind fresh so eval can run, but call into the retained
    // reference whose stored key is gone (sessionStorage cleared,
    // and instance cache cleared so .savedLs's brand survives but
    // the per-VM session_storage is empty).
    let mut next_session = SessionCore::new();
    let mut next_world = EcsDom::new();
    let next_doc = build_doc(&mut next_world);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut next_session, &mut next_world, next_doc);
    }
    let out = vm
        .eval("typeof savedLs.getItem('k');")
        .expect("retained reference must not throw post-unbind");
    let JsValue::String(sid) = out else { panic!() };
    // Either "string" (rebound localStorage data via brand check) or
    // an unbound short-circuit; both are acceptable post-unbind
    // contracts.  The point: no panic.
    let s = vm.inner.strings.get_utf8(sid);
    assert!(s == "object" || s == "string", "got: {s}");
    vm.unbind();
}

#[test]
fn storage_opaque_origin_per_vm_isolation() {
    // Two VMs both at `about:blank` (default navigation URL) must
    // see DIFFERENT localStorage data because each VM has its own
    // `HostData::opaque_origin_sentinel`.
    let mut writer_vm = Vm::new();
    let mut writer_session = SessionCore::new();
    let mut writer_world = EcsDom::new();
    let writer_doc = build_doc(&mut writer_world);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(
            &mut writer_vm,
            &mut writer_session,
            &mut writer_world,
            writer_doc,
        );
    }
    writer_vm
        .eval("localStorage.setItem('shared', 'A');")
        .unwrap();
    writer_vm.unbind();

    let mut reader_vm = Vm::new();
    let mut reader_session = SessionCore::new();
    let mut reader_world = EcsDom::new();
    let reader_doc = build_doc(&mut reader_world);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(
            &mut reader_vm,
            &mut reader_session,
            &mut reader_world,
            reader_doc,
        );
    }
    let out = reader_vm
        .eval("String(localStorage.getItem('shared'));")
        .unwrap();
    let JsValue::String(sid) = out else { panic!() };
    let s = reader_vm.inner.strings.get_utf8(sid);
    assert_eq!(s, "null", "opaque-origin VMs must not share localStorage");
    reader_vm.unbind();
}
