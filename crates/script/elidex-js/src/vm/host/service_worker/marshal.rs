//! `SwRequest` → `Request` and `Response` → `SwResponse` marshalling for
//! the Service Worker realm (`#11-service-workers-vm` / D-19 PR-2).
//!
//! The inbound `FetchEvent.request` is built from the engine-independent
//! [`SwRequest`] (mirrors `cache/marshal.rs::build_request_from_entry`); the
//! `respondWith` result is the inverse — a `Response` object marshalled into
//! a [`SwResponse`] for the channel.

#![cfg(feature = "engine")]

use elidex_api_sw::{SwRequest, SwResponse};

use super::super::super::shape;
use super::super::super::value::{JsValue, Object, ObjectId, ObjectKind, PropertyStorage, VmError};
use super::super::super::VmInner;
use super::super::headers::HeadersGuard;
use super::super::request_response::{
    RedirectMode, RequestCache, RequestCredentials, RequestMode, RequestState, ResponseType,
};

/// Intern `(name, value)` header pairs and install them as the `list` of
/// the freshly-created Headers `headers_id` (names lowercased — the
/// `headers_states` `list` invariant).
fn install_header_list(vm: &mut VmInner, headers_id: ObjectId, pairs: &[(String, String)]) {
    let mut list = Vec::with_capacity(pairs.len());
    for (name, value) in pairs {
        let name_sid = vm.strings.intern(&name.to_ascii_lowercase());
        let value_sid = vm.strings.intern(value);
        list.push((name_sid, value_sid));
    }
    if let Some(state) = vm.headers_states.get_mut(&headers_id) {
        state.list = list;
    }
}

/// Snapshot a Headers `list` into owned `(name, value)` String pairs.
fn headers_to_owned(vm: &VmInner, headers_id: ObjectId) -> Vec<(String, String)> {
    let Some(state) = vm.headers_states.get(&headers_id) else {
        return Vec::new();
    };
    state
        .list
        .iter()
        .map(|(k, v)| (vm.strings.get_utf8(*k), vm.strings.get_utf8(*v)))
        .collect()
}

/// Map a `RequestMode` IDL string (Fetch §5.4 Request class) to the broker enum.
fn parse_request_mode(mode: &str) -> RequestMode {
    match mode {
        "cors" => RequestMode::Cors,
        "same-origin" => RequestMode::SameOrigin,
        "navigate" => RequestMode::Navigate,
        // "no-cors" + anything unrecognised → the broker's transparent default.
        _ => RequestMode::NoCors,
    }
}

/// Map a `RequestRedirect` IDL string (Fetch §5.4 Request class) to the broker enum.
fn parse_redirect_mode(redirect: &str) -> RedirectMode {
    match redirect {
        "error" => RedirectMode::Error,
        "manual" => RedirectMode::Manual,
        _ => RedirectMode::Follow,
    }
}

/// Build a `FetchEvent.request` `Request` wrapper from the inbound
/// [`SwRequest`] (SW §4.6.1).  Carries url / method / headers / body + the
/// request's mode / redirect (credentials / cache use the Request defaults —
/// the broker re-derives credentials from the SW origin on a re-fetch).
pub(crate) fn build_request_from_sw_request(vm: &mut VmInner, request: &SwRequest) -> ObjectId {
    let proto = vm.request_prototype;
    let id = vm.alloc_object(Object {
        kind: ObjectKind::Request,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });
    // Root the just-allocated Request across `create_headers` (allocates a
    // companion Headers and can GC before the Request is root-reachable).
    let mut g = vm.push_temp_root(JsValue::Object(id));

    let headers_id = g.create_headers(HeadersGuard::Request);
    install_header_list(&mut g, headers_id, &request.headers);

    if !request.body.is_empty() {
        g.body_data.insert(id, request.body.clone());
    }

    let method_sid = g.strings.intern(&request.method);
    let url_sid = g.strings.intern(request.url.as_str());
    g.request_states.insert(
        id,
        RequestState {
            method_sid,
            url_sid,
            headers_id,
            redirect: parse_redirect_mode(&request.redirect),
            mode: parse_request_mode(&request.mode),
            credentials: RequestCredentials::SameOrigin,
            cache: RequestCache::Default,
        },
    );
    id
}

/// Marshal a `respondWith` result `Response` object into a [`SwResponse`]
/// for the channel (the inverse of `cache/marshal.rs::build_response_from_entry`).
///
/// `request_url` is the originating fetch URL — used as the `SwResponse.url`
/// (the response's own URL list is not Cache-style persisted here, and the
/// boa parity path also stamps the request URL).  A non-`Response` fulfilled
/// value is an `Err` (the SW loop maps it to a network passthrough, SW
/// §4.6.7).
pub(crate) fn response_to_sw_response(
    vm: &VmInner,
    value: JsValue,
    request_url: url::Url,
) -> Result<SwResponse, VmError> {
    let JsValue::Object(response_id) = value else {
        return Err(VmError::type_error(
            "FetchEvent.respondWith: fulfilled value is not a Response",
        ));
    };
    if !matches!(vm.get_object(response_id).kind, ObjectKind::Response) {
        return Err(VmError::type_error(
            "FetchEvent.respondWith: fulfilled value is not a Response",
        ));
    }
    let Some(state) = vm.response_states.get(&response_id) else {
        return Err(VmError::type_error(
            "FetchEvent.respondWith: Response has no internal state",
        ));
    };
    // SW §4.6.7: responding with a network-error Response (`Response.error()`,
    // type "error") fails the fetch — surface it as an `Err` (→ network
    // passthrough) rather than a bogus status-0 response delivered to the page.
    // (An opaque response IS a valid respondWith value; `SwResponse` carries no
    // type field, so it round-trips as its status-0 / empty-header shape, which
    // is the closest the wire representation allows.)
    if matches!(state.response_type, ResponseType::Error) {
        return Err(VmError::type_error(
            "FetchEvent.respondWith: a network-error Response cannot be delivered",
        ));
    }
    let status = state.status;
    let status_text = vm.strings.get_utf8(state.status_text_sid);
    let headers = headers_to_owned(vm, state.headers_id);
    let body = vm.body_data.get(&response_id).cloned().unwrap_or_default();
    Ok(SwResponse {
        status,
        status_text,
        headers,
        body,
        url: request_url,
    })
}
