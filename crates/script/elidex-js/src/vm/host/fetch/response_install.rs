//! Materialise a broker [`elidex_net::Response`] into a VM
//! `Response` object.  Called from
//! [`super::super::fetch_tick::settle_fetch`] once the broker
//! reply lands during `vm.tick_network()`.

use super::super::super::value::{JsValue, Object, ObjectId, ObjectKind, PropertyStorage};
use super::super::super::{shape, VmInner};
use super::super::headers::HeadersGuard;
use super::super::request_response::{ResponseState, ResponseType};

/// Wrap a broker [`Response`](elidex_net::Response) in a VM
/// `Response` object.  Headers are lowercased name-side (matches
/// `new Response`'s behaviour) and guarded Immutable.  Body bytes
/// land in the shared `body_data` map so `.text()` / `.json()`
/// / `.arrayBuffer()` / `.blob()` work without further copies.
///
/// The [`super::super::cors::CorsClassification`] argument selects
/// the Response shape:
/// - `Basic`: full headers, full body, status / url verbatim.
/// - `Cors`: headers filtered to CORS-safelisted +
///   `Access-Control-Expose-Headers` names; body / status / url
///   passed through.
/// - `Opaque` / `OpaqueRedirect` (`opaque_shape: true`): empty
///   headers, body dropped, status forced to 0, url emptied.
///   Spec-mandated to prevent leakage of cross-origin data.
pub(in crate::vm::host) fn create_response_from_net(
    vm: &mut VmInner,
    response: elidex_net::Response,
    classification: super::super::cors::CorsClassification,
) -> ObjectId {
    let proto = vm.response_prototype;
    let inst_id = vm.alloc_object(Object {
        kind: ObjectKind::Response,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });

    // Root the freshly-allocated Response across the next two
    // allocations (the companion `create_headers` + the per-name
    // / per-value `intern` calls).  Before `response_states`
    // stores `inst_id` near the end of this function, the new
    // Response is reachable only through this Rust local — per
    // `alloc_object`'s contract, any subsequent alloc that
    // triggers GC would reclaim it.  Same defensive invariant
    // as `wrap_in_array_iterator` (R10) and `native_fetch`
    // (R13).  The current runtime runs this site with
    // `gc_enabled = false` (called from inside `native_fetch`),
    // so the hazard is unreachable today; the guard future-
    // proofs it.
    let mut g = vm.push_temp_root(JsValue::Object(inst_id));

    // Companion Headers — allocate mutable, splice, then flip
    // to Immutable (matches `new Response(...)` contract).
    //
    // `headers_id` is also rooted across the header-splice work.
    // `headers_states` is **not** itself a GC root (see
    // `gc::mark_roots` — the entry is reached only via
    // `response_states[inst_id].headers_id`), so until
    // `response_states.insert(...)` links the Headers into the
    // Response, `headers_id` is reachable only through this
    // Rust local.  Route every subsequent allocation through `g2`
    // to keep both `inst_id` and `headers_id` rooted across the
    // `strings.intern` / `body_data.insert` / `response_states
    // .insert` sequence below (R18.2).
    // Apply the CORS classification to the response shape.  An
    // opaque-shape response (Opaque / OpaqueRedirect) discards
    // all headers, body, status, and URL so cross-origin data
    // never leaks into JS.  A Cors-typed response filters
    // headers down to CORS-safelisted +
    // `Access-Control-Expose-Headers` names.  Basic / Default
    // pass through verbatim.
    let opaque_shape = classification.opaque_shape;
    let response_type = classification.response_type;
    let header_pairs: Vec<(String, String)> = if opaque_shape {
        Vec::new()
    } else if matches!(response_type, ResponseType::Cors) {
        super::super::cors::filter_headers_for_cors_response(response.headers)
    } else {
        response.headers
    };

    let headers_id = g.create_headers(HeadersGuard::None);
    let mut g2 = g.push_temp_root(JsValue::Object(headers_id));
    {
        // Route each broker-delivered header through the shared
        // `validate_and_normalise` helper so the resulting
        // `HeadersState` carries the **same** invariants as a
        // script-constructed `Headers` instance: lowercased
        // name, RFC 7230 token-valid name, CR/LF/NUL-free value,
        // HTTP-whitespace-trimmed value.  Malformed entries
        // (broker-side bug, not user input) are silently
        // skipped — defensive, preserves the invariant even if
        // the network layer later relaxes its own filters.
        for (name, value) in header_pairs {
            let name_sid = g2.strings.intern(&name);
            let value_sid = g2.strings.intern(&value);
            if let Ok((nn, nv)) = super::super::headers::validate_and_normalise(
                &mut g2, name_sid, value_sid, "response",
            ) {
                if let Some(state) = g2.headers_states.get_mut(&headers_id) {
                    state.list.push((nn, nv));
                }
            }
        }
        if let Some(state) = g2.headers_states.get_mut(&headers_id) {
            state.guard = HeadersGuard::Immutable;
        }
    }

    // Body bytes.  Insert (even an empty `Vec`) for non-opaque
    // responses so `Response.body` materialises a ReadableStream
    // — spec §4.1: a non-opaque response has a body that is a
    // stream (possibly empty), never `null`.  Two exceptions
    // skip the insert so `.body` stays `null`:
    //   - opaque-shape responses (per §3.1.4: `.body` must be
    //     `null` for opaque / opaque-redirect)
    //   - null-body statuses 204 / 205 / 304 (per §4.1: those
    //     statuses MUST have a null body — Copilot R8 finding).
    //
    // The HTTP response body is owned by `bytes::Bytes` (its own
    // ref-counted handle); we copy it into a fresh `Vec<u8>` for
    // installation in `body_data`, since that map's storage type
    // is owned `Vec<u8>` so subsequent TypedArray / DataView
    // writes can mutate it in place via `byte_io`.
    let null_body_status = matches!(response.status, 204 | 205 | 304);
    if !opaque_shape && !null_body_status {
        g2.body_data.insert(inst_id, response.body.to_vec());
    }

    // Status / url rewrite for opaque-shape responses (WHATWG
    // Fetch §3.1.4 / §3.1.6): status 0, url empty.  Basic /
    // Cors pass through.
    let final_status = if opaque_shape { 0 } else { response.status };
    let url_sid = if opaque_shape {
        g2.well_known.empty
    } else {
        g2.strings.intern(response.url.as_str())
    };
    let status_text_sid = g2.well_known.empty;
    let redirected = response.url_list.len() > 1;

    g2.response_states.insert(
        inst_id,
        ResponseState {
            status: final_status,
            status_text_sid,
            url_sid,
            headers_id,
            response_type,
            redirected,
        },
    );
    drop(g2);
    // `inst_id` is now referenced from `response_states` (and
    // `headers_id` is referenced from its ResponseState field),
    // so dropping the root is safe.
    drop(g);
    inst_id
}
