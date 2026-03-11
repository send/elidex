use super::*;

#[test]
fn status_text_known_codes() {
    assert_eq!(status_text_for(200), "OK");
    assert_eq!(status_text_for(404), "Not Found");
    assert_eq!(status_text_for(500), "Internal Server Error");
}

#[test]
fn status_text_unknown_code() {
    assert_eq!(status_text_for(999), "");
}

#[test]
fn headers_object_get() {
    let mut ctx = Context::default();
    let headers = vec![
        ("Content-Type".to_string(), "text/html".to_string()),
        ("X-Custom".to_string(), "value".to_string()),
    ];
    let obj = create_headers_object(&headers, &mut ctx);
    let obj = obj.as_object().unwrap();

    // get existing header (case-insensitive)
    let get_fn = obj.get(js_string!("get"), &mut ctx).unwrap();
    let result = get_fn
        .as_callable()
        .unwrap()
        .call(
            &obj.clone().into(),
            &[js_string!("content-type").into()],
            &mut ctx,
        )
        .unwrap();
    assert_eq!(
        result.to_string(&mut ctx).unwrap().to_std_string_escaped(),
        "text/html"
    );

    // get missing header
    let result = get_fn
        .as_callable()
        .unwrap()
        .call(
            &obj.clone().into(),
            &[js_string!("missing").into()],
            &mut ctx,
        )
        .unwrap();
    assert!(result.is_null());
}

#[test]
fn headers_get_combines_duplicates() {
    let mut ctx = Context::default();
    let headers = vec![
        ("Set-Cookie".to_string(), "a=1".to_string()),
        ("Set-Cookie".to_string(), "b=2".to_string()),
        ("Content-Type".to_string(), "text/html".to_string()),
    ];
    let obj = create_headers_object(&headers, &mut ctx);
    let obj = obj.as_object().unwrap();

    let get_fn = obj.get(js_string!("get"), &mut ctx).unwrap();
    let result = get_fn
        .as_callable()
        .unwrap()
        .call(
            &obj.clone().into(),
            &[js_string!("set-cookie").into()],
            &mut ctx,
        )
        .unwrap();
    assert_eq!(
        result.to_string(&mut ctx).unwrap().to_std_string_escaped(),
        "a=1, b=2"
    );

    // Single-value header is unchanged.
    let result = get_fn
        .as_callable()
        .unwrap()
        .call(
            &obj.clone().into(),
            &[js_string!("content-type").into()],
            &mut ctx,
        )
        .unwrap();
    assert_eq!(
        result.to_string(&mut ctx).unwrap().to_std_string_escaped(),
        "text/html"
    );
}

#[test]
fn headers_object_has() {
    let mut ctx = Context::default();
    let headers = vec![("Content-Type".to_string(), "text/html".to_string())];
    let obj = create_headers_object(&headers, &mut ctx);
    let obj = obj.as_object().unwrap();

    let has_fn = obj.get(js_string!("has"), &mut ctx).unwrap();

    // has existing
    let result = has_fn
        .as_callable()
        .unwrap()
        .call(
            &obj.clone().into(),
            &[js_string!("content-type").into()],
            &mut ctx,
        )
        .unwrap();
    assert!(result.to_boolean());

    // has missing
    let result = has_fn
        .as_callable()
        .unwrap()
        .call(
            &obj.clone().into(),
            &[js_string!("missing").into()],
            &mut ctx,
        )
        .unwrap();
    assert!(!result.to_boolean());
}

#[test]
fn headers_object_foreach() {
    let mut ctx = Context::default();
    let headers = vec![
        ("a".to_string(), "1".to_string()),
        ("b".to_string(), "2".to_string()),
    ];
    let obj = create_headers_object(&headers, &mut ctx);

    // Use forEach through eval to simplify.
    let global = ctx.global_object();
    global
        .set(js_string!("testHeaders"), obj, false, &mut ctx)
        .unwrap();

    let result = ctx
        .eval(boa_engine::Source::from_bytes(
            "var parts = []; testHeaders.forEach(function(v, k) { parts.push(k + '=' + v); }); parts.join(',')",
        ))
        .unwrap();
    let s = result.to_string(&mut ctx).unwrap().to_std_string_escaped();
    assert_eq!(s, "a=1,b=2");
}

#[test]
fn response_object_properties() {
    let mut ctx = Context::default();
    let response = elidex_net::Response {
        status: 200,
        headers: vec![("content-type".to_string(), "text/plain".to_string())],
        body: bytes::Bytes::from("hello world"),
        url: url::Url::parse("https://example.com/page").unwrap(),
        version: elidex_net::HttpVersion::H1,
    };
    let request_url = url::Url::parse("https://example.com/page").unwrap();
    let obj = create_response_object(&response, &request_url, &mut ctx);
    let obj = obj.as_object().unwrap();

    // ok
    let ok = obj.get(js_string!("ok"), &mut ctx).unwrap();
    assert!(ok.to_boolean());

    // status
    let status = obj.get(js_string!("status"), &mut ctx).unwrap();
    assert_eq!(status.to_number(&mut ctx).unwrap(), 200.0);

    // statusText
    let st = obj.get(js_string!("statusText"), &mut ctx).unwrap();
    assert_eq!(
        st.to_string(&mut ctx).unwrap().to_std_string_escaped(),
        "OK"
    );

    // url
    let url = obj.get(js_string!("url"), &mut ctx).unwrap();
    assert_eq!(
        url.to_string(&mut ctx).unwrap().to_std_string_escaped(),
        "https://example.com/page"
    );

    // type
    let type_val = obj.get(js_string!("type"), &mut ctx).unwrap();
    assert_eq!(
        type_val
            .to_string(&mut ctx)
            .unwrap()
            .to_std_string_escaped(),
        "basic"
    );

    // redirected (same URL -> false)
    let redirected = obj.get(js_string!("redirected"), &mut ctx).unwrap();
    assert!(!redirected.to_boolean());
}

#[test]
fn response_redirected_flag() {
    let mut ctx = Context::default();
    let response = elidex_net::Response {
        status: 200,
        headers: vec![],
        body: bytes::Bytes::new(),
        url: url::Url::parse("https://example.com/final").unwrap(),
        version: elidex_net::HttpVersion::H1,
    };
    let request_url = url::Url::parse("https://example.com/original").unwrap();
    let obj = create_response_object(&response, &request_url, &mut ctx);
    let obj = obj.as_object().unwrap();

    let redirected = obj.get(js_string!("redirected"), &mut ctx).unwrap();
    assert!(redirected.to_boolean());
}

#[test]
fn response_404_not_ok() {
    let mut ctx = Context::default();
    let response = elidex_net::Response {
        status: 404,
        headers: vec![],
        body: bytes::Bytes::from("not found"),
        url: url::Url::parse("https://example.com/missing").unwrap(),
        version: elidex_net::HttpVersion::H1,
    };
    let request_url = url::Url::parse("https://example.com/missing").unwrap();
    let obj = create_response_object(&response, &request_url, &mut ctx);
    let obj = obj.as_object().unwrap();

    let ok = obj.get(js_string!("ok"), &mut ctx).unwrap();
    assert!(!ok.to_boolean());

    let st = obj.get(js_string!("statusText"), &mut ctx).unwrap();
    assert_eq!(
        st.to_string(&mut ctx).unwrap().to_std_string_escaped(),
        "Not Found"
    );
}

#[test]
fn response_text_method() {
    let mut ctx = Context::default();
    let response = elidex_net::Response {
        status: 200,
        headers: vec![],
        body: bytes::Bytes::from("body content"),
        url: url::Url::parse("https://example.com/").unwrap(),
        version: elidex_net::HttpVersion::H1,
    };
    let request_url = url::Url::parse("https://example.com/").unwrap();
    let obj = create_response_object(&response, &request_url, &mut ctx);

    // Set as global and test text() via eval.
    let global = ctx.global_object();
    global
        .set(js_string!("testResp"), obj, false, &mut ctx)
        .unwrap();

    // text() returns a Promise. Since it's already resolved,
    // we chain with .then() and run_jobs().
    ctx.eval(boa_engine::Source::from_bytes(
        "var textResult = ''; testResp.text().then(function(t) { textResult = t; });",
    ))
    .unwrap();
    ctx.run_jobs().unwrap();

    let result = ctx
        .eval(boa_engine::Source::from_bytes("textResult"))
        .unwrap();
    assert_eq!(
        result.to_string(&mut ctx).unwrap().to_std_string_escaped(),
        "body content"
    );
}

#[test]
fn response_json_method() {
    let mut ctx = Context::default();
    let response = elidex_net::Response {
        status: 200,
        headers: vec![],
        body: bytes::Bytes::from(r#"{"key":"value","num":42}"#),
        url: url::Url::parse("https://example.com/").unwrap(),
        version: elidex_net::HttpVersion::H1,
    };
    let request_url = url::Url::parse("https://example.com/").unwrap();
    let obj = create_response_object(&response, &request_url, &mut ctx);

    let global = ctx.global_object();
    global
        .set(js_string!("testResp"), obj, false, &mut ctx)
        .unwrap();

    ctx.eval(boa_engine::Source::from_bytes(
        "var jsonResult = null; testResp.json().then(function(d) { jsonResult = d; });",
    ))
    .unwrap();
    ctx.run_jobs().unwrap();

    let result = ctx
        .eval(boa_engine::Source::from_bytes("jsonResult.key"))
        .unwrap();
    assert_eq!(
        result.to_string(&mut ctx).unwrap().to_std_string_escaped(),
        "value"
    );

    let num = ctx
        .eval(boa_engine::Source::from_bytes("jsonResult.num"))
        .unwrap();
    assert_eq!(num.to_number(&mut ctx).unwrap(), 42.0);
}

#[test]
fn response_json_invalid_rejects() {
    let mut ctx = Context::default();
    let response = elidex_net::Response {
        status: 200,
        headers: vec![],
        body: bytes::Bytes::from("not json at all"),
        url: url::Url::parse("https://example.com/").unwrap(),
        version: elidex_net::HttpVersion::H1,
    };
    let request_url = url::Url::parse("https://example.com/").unwrap();
    let obj = create_response_object(&response, &request_url, &mut ctx);

    let global = ctx.global_object();
    global
        .set(js_string!("testResp"), obj, false, &mut ctx)
        .unwrap();

    ctx.eval(boa_engine::Source::from_bytes(
        "var jsonErr = ''; testResp.json().catch(function(e) { jsonErr = String(e); });",
    ))
    .unwrap();
    ctx.run_jobs().unwrap();

    let result = ctx.eval(boa_engine::Source::from_bytes("jsonErr")).unwrap();
    let err_str = result.to_string(&mut ctx).unwrap().to_std_string_escaped();
    assert!(!err_str.is_empty(), "Expected error message from json()");
}

#[test]
fn response_clone_method() {
    let mut ctx = Context::default();
    let response = elidex_net::Response {
        status: 201,
        headers: vec![("x-test".to_string(), "yes".to_string())],
        body: bytes::Bytes::from("cloned body"),
        url: url::Url::parse("https://example.com/").unwrap(),
        version: elidex_net::HttpVersion::H1,
    };
    let request_url = url::Url::parse("https://example.com/").unwrap();
    let obj = create_response_object(&response, &request_url, &mut ctx);

    let global = ctx.global_object();
    global
        .set(js_string!("testResp"), obj, false, &mut ctx)
        .unwrap();

    // Clone and verify properties preserved.
    ctx.eval(boa_engine::Source::from_bytes(
        "var cloned = testResp.clone();",
    ))
    .unwrap();

    let status = ctx
        .eval(boa_engine::Source::from_bytes("cloned.status"))
        .unwrap();
    assert_eq!(status.to_number(&mut ctx).unwrap(), 201.0);

    // Clone's text() should work.
    ctx.eval(boa_engine::Source::from_bytes(
        "var cloneText = ''; cloned.text().then(function(t) { cloneText = t; });",
    ))
    .unwrap();
    ctx.run_jobs().unwrap();

    let result = ctx
        .eval(boa_engine::Source::from_bytes("cloneText"))
        .unwrap();
    assert_eq!(
        result.to_string(&mut ctx).unwrap().to_std_string_escaped(),
        "cloned body"
    );
}

#[test]
fn response_clone_has_clone() {
    let mut ctx = Context::default();
    let response = elidex_net::Response {
        status: 200,
        headers: vec![],
        body: bytes::Bytes::from("deep clone"),
        url: url::Url::parse("https://example.com/").unwrap(),
        version: elidex_net::HttpVersion::H1,
    };
    let request_url = url::Url::parse("https://example.com/").unwrap();
    let obj = create_response_object(&response, &request_url, &mut ctx);

    let global = ctx.global_object();
    global
        .set(js_string!("testResp"), obj, false, &mut ctx)
        .unwrap();

    // clone().clone() should work (clone includes clone method).
    ctx.eval(boa_engine::Source::from_bytes(
        "var c1 = testResp.clone(); var c2 = c1.clone();",
    ))
    .unwrap();

    let status = ctx
        .eval(boa_engine::Source::from_bytes("c2.status"))
        .unwrap();
    assert_eq!(status.to_number(&mut ctx).unwrap(), 200.0);

    // Verify text() works on the double-clone.
    ctx.eval(boa_engine::Source::from_bytes(
        "var c2Text = ''; c2.text().then(function(t) { c2Text = t; });",
    ))
    .unwrap();
    ctx.run_jobs().unwrap();

    let result = ctx.eval(boa_engine::Source::from_bytes("c2Text")).unwrap();
    assert_eq!(
        result.to_string(&mut ctx).unwrap().to_std_string_escaped(),
        "deep clone"
    );
}
