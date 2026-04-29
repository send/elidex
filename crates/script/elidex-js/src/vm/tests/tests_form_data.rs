//! `FormData` tests (WHATWG XHR §4.3) + multipart-encoder
//! integration via the body-extraction path.

#![cfg(feature = "engine")]

use super::super::value::JsValue;
use super::super::Vm;

fn eval_string(vm: &mut Vm, source: &str) -> String {
    match vm.eval(source).unwrap() {
        JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string, got {other:?}"),
    }
}

fn eval_number(vm: &mut Vm, source: &str) -> f64 {
    match vm.eval(source).unwrap() {
        JsValue::Number(n) => n,
        other => panic!("expected number, got {other:?}"),
    }
}

fn eval_bool(vm: &mut Vm, source: &str) -> bool {
    match vm.eval(source).unwrap() {
        JsValue::Boolean(b) => b,
        other => panic!("expected bool, got {other:?}"),
    }
}

/// Evaluate `source` then read `globalThis[name]` as a String.
/// Mirrors `tests_blob::eval_global_string` — used for tests that
/// drain microtasks via `Promise.then(t => globalThis.x = t)`.
fn eval_global_string(source: &str, name: &str) -> String {
    let mut vm = Vm::new();
    vm.eval(source).unwrap();
    match vm.get_global(name) {
        Some(JsValue::String(id)) => vm.get_string(id),
        other => panic!("expected global {name} to be a string, got {other:?}"),
    }
}

#[test]
fn ctor_empty() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "let f = new FormData(); let acc = ''; \
             for (let [k, v] of f) acc += k + '=' + v + ';'; acc;"
        ),
        ""
    );
}

#[test]
fn ctor_requires_new() {
    let mut vm = Vm::new();
    assert!(vm.eval("FormData();").is_err());
}

#[test]
fn append_string_value() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "let f = new FormData(); f.append('key', 'value'); f.get('key');"
        ),
        "value"
    );
}

#[test]
fn append_coerces_value_to_string() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "let f = new FormData(); f.append('n', 42); f.get('n');"
        ),
        "42"
    );
}

#[test]
fn append_blob_returns_blob() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "let f = new FormData(); let b = new Blob(['hello']); \
         f.append('file', b); f.get('file') === b;"
    ));
}

#[test]
fn append_multiple_same_name() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "let f = new FormData(); f.append('a', '1'); f.append('a', '2'); \
             JSON.stringify(f.getAll('a'));"
        ),
        "[\"1\",\"2\"]"
    );
}

#[test]
fn delete_removes_all_with_name() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "let f = new FormData(); f.append('a', '1'); f.append('a', '2'); \
             f.append('b', '3'); f.delete('a'); f.getAll('a').length + f.getAll('b').length;"
        ),
        1.0
    );
}

#[test]
fn has_returns_bool() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "let f = new FormData(); f.append('a', '1'); f.has('a');"
    ));
    assert!(!eval_bool(
        &mut vm,
        "let f = new FormData(); f.append('a', '1'); f.has('b');"
    ));
}

#[test]
fn get_returns_null_when_absent() {
    let mut vm = Vm::new();
    let result = vm
        .eval("let f = new FormData(); f.get('missing');")
        .unwrap();
    assert!(matches!(result, JsValue::Null));
}

#[test]
fn set_replaces_first_drops_rest() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "let f = new FormData(); f.append('a', '1'); f.append('a', '2'); \
             f.append('b', '3'); f.set('a', '9'); JSON.stringify(f.getAll('a'));"
        ),
        "[\"9\"]"
    );
}

#[test]
fn set_appends_when_absent() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "let f = new FormData(); f.set('a', '1'); f.get('a');"
        ),
        "1"
    );
}

#[test]
fn for_each_invokes_callback_in_order() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "let f = new FormData(); f.append('a', '1'); f.append('b', '2'); \
             let acc = ''; f.forEach((value, name) => { acc += name + '=' + value + ';'; }); acc;"
        ),
        "a=1;b=2;"
    );
}

#[test]
fn entries_iteration() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "let f = new FormData(); f.append('a', '1'); f.append('b', '2'); \
             let acc = ''; for (let [k, v] of f) acc += k + '=' + v + ';'; acc;"
        ),
        "a=1;b=2;"
    );
}

#[test]
fn iterator_alias_to_entries() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "FormData.prototype[Symbol.iterator] === FormData.prototype.entries;"
    ));
}

#[test]
fn brand_check_throws_on_alien_receiver() {
    let mut vm = Vm::new();
    assert!(vm
        .eval("FormData.prototype.append.call({}, 'a', '1');")
        .is_err());
}

#[test]
fn prototype_survives_gc_after_global_removal() {
    // Regression for R2 GC-roots finding: with `FormData` removed
    // from globals, the cached `VmInner::form_data_prototype`
    // ObjectId remains an intrinsic root and a freshly-constructed
    // instance still finds its prototype methods after a forced GC.
    let mut vm = Vm::new();
    vm.eval(
        "globalThis.SavedFD = FormData; \
         delete globalThis.FormData;",
    )
    .unwrap();
    vm.inner.collect_garbage();
    let value = match vm
        .eval("let f = new SavedFD(); f.append('k', 'v'); f.get('k');")
        .unwrap()
    {
        JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string, got {other:?}"),
    };
    assert_eq!(value, "v");
}

// ---------------------------------------------------------------------------
// Body extraction (multipart encoder + Content-Type wiring)
// ---------------------------------------------------------------------------

#[test]
fn response_with_form_data_body_sets_multipart_content_type() {
    let mut vm = Vm::new();
    let ct = eval_string(
        &mut vm,
        "let f = new FormData(); f.append('a', '1'); \
         new Response(f).headers.get('content-type');",
    );
    assert!(
        ct.starts_with("multipart/form-data; boundary="),
        "unexpected Content-Type: {ct:?}"
    );
}

#[test]
fn response_with_form_data_body_round_trips_via_text() {
    // The serialised body must be reachable through the Body
    // mixin's `.text()` Promise.  We don't pin the boundary
    // (encoder-derived); we check structural invariants:
    // - Two `Content-Disposition: form-data; name="..."` lines.
    // - The two values land in their respective parts.
    // - The closing boundary marker is `--<boundary>--`.
    let acc = eval_global_string(
        "globalThis.s = ''; \
         let f = new FormData(); f.append('a', '1'); f.append('b', '2'); \
         new Response(f).text().then(t => { globalThis.s = t; });",
        "s",
    );
    assert!(
        acc.contains("Content-Disposition: form-data; name=\"a\""),
        "{acc:?}"
    );
    assert!(
        acc.contains("Content-Disposition: form-data; name=\"b\""),
        "{acc:?}"
    );
    // First-part value separator: blank line followed by `1\r\n`.
    assert!(acc.contains("\r\n\r\n1\r\n"), "{acc:?}");
    assert!(acc.contains("\r\n\r\n2\r\n"), "{acc:?}");
    assert!(acc.ends_with("--\r\n"), "{acc:?}");
}

#[test]
fn response_with_form_data_blob_emits_filename_and_content_type() {
    let body = eval_global_string(
        "globalThis.s = ''; \
         let f = new FormData(); \
         let b = new Blob(['hi'], {type: 'text/plain'}); \
         f.append('file', b, 'note.txt'); \
         new Response(f).text().then(t => { globalThis.s = t; });",
        "s",
    );
    assert!(
        body.contains("Content-Disposition: form-data; name=\"file\"; filename=\"note.txt\""),
        "{body:?}"
    );
    assert!(body.contains("Content-Type: text/plain"), "{body:?}");
    assert!(body.contains("\r\n\r\nhi\r\n"), "{body:?}");
}

#[test]
fn response_with_form_data_blob_default_filename_is_blob() {
    let body = eval_global_string(
        "globalThis.s = ''; \
         let f = new FormData(); f.append('payload', new Blob(['x'])); \
         new Response(f).text().then(t => { globalThis.s = t; });",
        "s",
    );
    assert!(
        body.contains("filename=\"blob\""),
        "expected default filename 'blob' in body: {body:?}"
    );
    // Untyped Blob → application/octet-stream per WHATWG XHR §4.3.
    assert!(
        body.contains("Content-Type: application/octet-stream"),
        "{body:?}"
    );
}

#[test]
fn response_with_url_search_params_sets_form_urlencoded_content_type() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "new Response(new URLSearchParams('a=1&b=2')).headers.get('content-type');"
        ),
        "application/x-www-form-urlencoded;charset=UTF-8"
    );
}

#[test]
fn response_with_url_search_params_serialises_body_text() {
    assert_eq!(
        eval_global_string(
            "globalThis.s = ''; \
             new Response(new URLSearchParams('q=hello world&x=1')) \
                 .text().then(t => { globalThis.s = t; });",
            "s",
        ),
        "q=hello+world&x=1"
    );
}

#[test]
fn request_with_form_data_body_sets_multipart_content_type() {
    let mut vm = Vm::new();
    let ct = eval_string(
        &mut vm,
        "let f = new FormData(); f.append('a', '1'); \
         new Request('https://example.test/', {method: 'POST', body: f}) \
             .headers.get('content-type');",
    );
    assert!(
        ct.starts_with("multipart/form-data; boundary="),
        "unexpected Content-Type: {ct:?}"
    );
}

#[test]
fn request_with_url_search_params_sets_form_urlencoded() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "new Request('https://example.test/', \
                 {method: 'POST', body: new URLSearchParams('a=1')}) \
                 .headers.get('content-type');"
        ),
        "application/x-www-form-urlencoded;charset=UTF-8"
    );
}

#[test]
fn multipart_encoder_handles_large_blob_bytes() {
    // Regression for R1 perf finding: Blob bytes flow through the
    // encoder via `Arc<[u8]>` rather than a per-call clone.  The
    // 64 KiB payload exercises every code path (materialise →
    // collision-check → final-body extend) at a size that would
    // make a copy-then-copy implementation observably slower; the
    // structural assertions verify functional correctness.
    let body = eval_global_string(
        "globalThis.s = ''; \
         let chunk = 'x'.repeat(65536); \
         let f = new FormData(); f.append('big', new Blob([chunk], {type: 'application/octet-stream'})); \
         new Response(f).text().then(t => { globalThis.s = t; });",
        "s",
    );
    // Body should contain exactly 65536 'x' bytes between the
    // post-headers separator and the trailing CRLF.
    let needle = "\r\n\r\n";
    let header_end = body.find(needle).expect("missing header/value separator");
    let value_start = header_end + needle.len();
    let value_section = &body[value_start..];
    let value_end = value_section
        .find("\r\n--")
        .expect("missing trailing boundary after value");
    assert_eq!(
        value_end,
        65536,
        "blob bytes truncated or duplicated; body len={}",
        body.len()
    );
}

#[test]
fn explicit_content_type_in_init_headers_is_not_overridden() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "new Response(new URLSearchParams('a=1'), \
                 {headers: {'Content-Type': 'application/x-custom'}}) \
                 .headers.get('content-type');"
        ),
        "application/x-custom"
    );
}
