//! S5-6a (flip-inert capability prereq): the six cross-context VM/trait ADDs —
//! B3 storage-change emit drain, B4 `install_web_storage`, B6 IDB
//! versionchange emit drain, B13 `window.focus()` pending-focus, B16 iframe
//! parent-message out-queue, B21 IDB versionchange deliver — driven through
//! the engine's PUBLIC `HostDriver` surface (the S5-6b shell call sites'
//! exact shape), mirroring `tests_engine_s1c`.
//!
//! These surfaces are deliberately CALLER-LESS from the shell until S5-6b
//! (the flip): per the S5-6 plan memo §7.1a, **the test IS the connection**
//! — each ADD's land-time oracle lives here.

#![cfg(feature = "engine")]

use elidex_ecs::{EcsDom, Entity};
use elidex_script_session::{HostDriver, ScriptContext, ScriptEngine, SessionCore};
use url::Url;

use crate::engine::ElidexJsEngine;
use crate::vm::value::JsValue;

/// Construct an unbound engine + session + dom with a fresh `document_root`
/// (mirrors `tests_engine_s1c::fresh_unbound`).
fn fresh_unbound() -> (ElidexJsEngine, SessionCore, EcsDom, Entity) {
    let mut engine = ElidexJsEngine::new();
    let session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    (engine, session, dom, doc)
}

/// Open the engine's batch bracket (see `tests_engine_s1a::bind_engine`).
#[allow(unsafe_code)]
fn bind_engine(engine: &mut ElidexJsEngine, ctx: &mut ScriptContext<'_>) {
    // SAFETY: the bracket stays open until the paired `unbind`, and no test body
    // aliases `ctx.session`/`ctx.dom` while bound.
    unsafe { engine.bind(ctx) }
}

fn url(s: &str) -> Url {
    Url::parse(s).expect("valid test URL")
}

/// Evaluate `expr` on the (bound) engine's VM and return it as a Rust string
/// (the read-back oracle for listener-observed state).
fn eval_string(engine: &mut ElidexJsEngine, expr: &str) -> String {
    let result = engine.vm().eval(expr).expect("read-back eval succeeds");
    let JsValue::String(sid) = result else {
        panic!("expected string result, got {result:?}");
    };
    engine.vm().inner.strings.get_utf8(sid)
}

// ---------------------------------------------------------------------------
// B3 — storage-change emit drain (WHATWG HTML §12.2.1) + B4 — web-storage
// install.  A2: the Web Storage family is `compat-webapi`-gated.
// ---------------------------------------------------------------------------

#[cfg(feature = "compat-webapi")]
mod storage {
    use std::sync::Arc;

    use elidex_storage_core::WebStorageManager;

    use super::*;

    const DOC_URL: &str = "https://example.com/page";

    /// A disk-rooted manager that never touches disk: `WebStorageManager`
    /// does no I/O until `flush_dirty`, which these tests never call.
    fn fresh_manager() -> Arc<WebStorageManager> {
        Arc::new(WebStorageManager::new(std::env::temp_dir().join(format!(
            "elidex-js-s6a-never-created-{}",
            std::process::id()
        ))))
    }

    #[test]
    fn local_storage_mutations_enqueue_broadcasts_in_order() {
        let (mut engine, mut session, mut dom, doc) = fresh_unbound();
        engine.set_current_url(Some(url(DOC_URL)));
        let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
        bind_engine(&mut engine, &mut ctx);
        let r = ScriptEngine::eval(
            &mut engine,
            "localStorage.setItem('k', 'v1');\
             localStorage.setItem('k', 'v2');\
             localStorage.removeItem('k');",
            &mut ctx,
        );
        assert!(r.success, "{:?}", r.error);
        engine.unbind();

        // The queue survives unbind (the shell drains after the bracket
        // closes, like the navigation back-channel).
        let changes = engine.take_pending_storage_changes();
        assert_eq!(
            changes.len(),
            3,
            "one broadcast per real change: {changes:?}"
        );
        // setItem step 7: key, oldValue (None — newly created), value.
        assert_eq!(changes[0].key.as_deref(), Some("k"));
        assert_eq!(changes[0].old_value, None);
        assert_eq!(changes[0].new_value.as_deref(), Some("v1"));
        assert_eq!(changes[0].url, DOC_URL);
        // The broadcast-targeting key is the storage BUCKET origin (a tuple
        // origin serializes; every change from this document carries it).
        for change in &changes {
            assert_eq!(change.origin, "https://example.com");
        }
        // Overwrite: oldValue = previous value.
        assert_eq!(changes[1].old_value.as_deref(), Some("v1"));
        assert_eq!(changes[1].new_value.as_deref(), Some("v2"));
        // removeItem step 5: key, oldValue, null.
        assert_eq!(changes[2].key.as_deref(), Some("k"));
        assert_eq!(changes[2].old_value.as_deref(), Some("v2"));
        assert_eq!(changes[2].new_value, None);
        // Drain-once: a second take is empty.
        assert!(engine.take_pending_storage_changes().is_empty());
    }

    #[test]
    fn no_change_mutations_enqueue_nothing() {
        let (mut engine, mut session, mut dom, doc) = fresh_unbound();
        engine.set_current_url(Some(url(DOC_URL)));
        let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
        bind_engine(&mut engine, &mut ctx);
        // §12.2.1 change gates: same-value setItem (step 3.2), absent-key
        // removeItem (step 1) — only the first setItem broadcasts.
        let r = ScriptEngine::eval(
            &mut engine,
            "localStorage.setItem('k', 'v');\
             localStorage.setItem('k', 'v');\
             localStorage.removeItem('missing');",
            &mut ctx,
        );
        assert!(r.success, "{:?}", r.error);
        let changes = engine.take_pending_storage_changes();
        assert_eq!(
            changes.len(),
            1,
            "gated mutations broadcast nothing: {changes:?}"
        );

        // clear of a non-empty map broadcasts null/null/null (step 3); a
        // second clear on the now-empty map is gated (step 1).
        let r = ScriptEngine::eval(
            &mut engine,
            "localStorage.clear(); localStorage.clear();",
            &mut ctx,
        );
        assert!(r.success, "{:?}", r.error);
        engine.unbind();
        let changes = engine.take_pending_storage_changes();
        assert_eq!(
            changes.len(),
            1,
            "empty-map clear broadcasts nothing: {changes:?}"
        );
        assert_eq!(changes[0].key, None);
        assert_eq!(changes[0].old_value, None);
        assert_eq!(changes[0].new_value, None);
        assert_eq!(changes[0].url, DOC_URL);
    }

    #[test]
    fn named_property_set_and_delete_broadcast_like_the_methods() {
        // The WebIDL named setter/deleter run the same §12.2.1 setItem /
        // removeItem method steps, so `localStorage.x = …` broadcasts too.
        let (mut engine, mut session, mut dom, doc) = fresh_unbound();
        engine.set_current_url(Some(url(DOC_URL)));
        let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
        bind_engine(&mut engine, &mut ctx);
        let r = ScriptEngine::eval(
            &mut engine,
            "localStorage.x = '1'; delete localStorage.x;",
            &mut ctx,
        );
        assert!(r.success, "{:?}", r.error);
        engine.unbind();
        let changes = engine.take_pending_storage_changes();
        assert_eq!(changes.len(), 2, "{changes:?}");
        assert_eq!(changes[0].key.as_deref(), Some("x"));
        assert_eq!(changes[0].new_value.as_deref(), Some("1"));
        assert_eq!(changes[1].old_value.as_deref(), Some("1"));
        assert_eq!(changes[1].new_value, None);
    }

    #[test]
    fn session_storage_mutations_do_not_enqueue() {
        // The cross-context drain is localStorage-only (boa parity; elidex
        // has no second same-session context pre-S5-8 — memo §4.3.2).
        let (mut engine, mut session, mut dom, doc) = fresh_unbound();
        engine.set_current_url(Some(url(DOC_URL)));
        let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
        bind_engine(&mut engine, &mut ctx);
        let r = ScriptEngine::eval(
            &mut engine,
            "sessionStorage.setItem('k', 'v');\
             sessionStorage.removeItem('k');\
             sessionStorage.setItem('k2', 'v2');\
             sessionStorage.clear();",
            &mut ctx,
        );
        assert!(r.success, "{:?}", r.error);
        engine.unbind();
        assert!(engine.take_pending_storage_changes().is_empty());
    }

    #[test]
    fn opaque_origin_change_carries_the_sentinel_bucket_not_null() {
        // A sandboxed / opaque document's mutation keys the per-VM
        // opaque-origin SENTINEL bucket (S5-4b) — and its broadcast must
        // carry that bucket string, never the `"null"` serialization every
        // opaque origin collapses to (which would alias unrelated sandboxed
        // iframes' broadcasts across the isolation boundary).
        let (mut engine, mut session, mut dom, doc) = fresh_unbound();
        engine.set_current_url(Some(url(DOC_URL)));
        // The shell's sandboxed-iframe load path installs the opaque origin
        // override alongside the sandbox flags.
        engine.set_origin(elidex_plugin::SecurityOrigin::opaque());
        let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
        bind_engine(&mut engine, &mut ctx);
        let r = ScriptEngine::eval(&mut engine, "localStorage.setItem('k', 'v');", &mut ctx);
        assert!(r.success, "{:?}", r.error);
        engine.unbind();

        let changes = engine.take_pending_storage_changes();
        assert_eq!(changes.len(), 1, "{changes:?}");
        assert!(
            changes[0].origin.starts_with("opaque-origin:"),
            "opaque document broadcasts under its sentinel bucket, got {:?}",
            changes[0].origin
        );
        assert_ne!(changes[0].origin, "null");
    }

    #[test]
    fn install_web_storage_routes_local_storage_through_the_manager() {
        let (mut engine, mut session, mut dom, doc) = fresh_unbound();
        engine.set_current_url(Some(url(DOC_URL)));
        let manager = fresh_manager();
        engine.install_web_storage(Arc::clone(&manager));
        let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
        bind_engine(&mut engine, &mut ctx);
        let r = ScriptEngine::eval(&mut engine, "localStorage.setItem('k', 'v');", &mut ctx);
        assert!(r.success, "{:?}", r.error);
        engine.unbind();

        // Post-install the natives route through the manager (the value is
        // visible via the backend under the document's serialized origin),
        // NOT the per-VM in-memory fallback.
        let origin = elidex_plugin::SecurityOrigin::from_url(&url(DOC_URL)).serialize();
        assert_eq!(
            manager.local_get(&origin, "k").as_deref(),
            Some("v"),
            "setItem must land in the installed manager under origin {origin}"
        );
        // The manager-backed write broadcasts like any other (B3 × B4).
        assert_eq!(engine.take_pending_storage_changes().len(), 1);
    }

    #[test]
    fn uninstalled_engine_falls_back_to_in_memory() {
        // H11 hermetic-test pin: without `install_web_storage` the natives
        // fall back to the per-VM in-memory store — reads round-trip, and no
        // manager observes anything.
        let (mut engine, mut session, mut dom, doc) = fresh_unbound();
        engine.set_current_url(Some(url(DOC_URL)));
        let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
        bind_engine(&mut engine, &mut ctx);
        let r = ScriptEngine::eval(&mut engine, "localStorage.setItem('k', 'v');", &mut ctx);
        assert!(r.success, "{:?}", r.error);
        assert_eq!(eval_string(&mut engine, "localStorage.getItem('k')"), "v");
        engine.unbind();

        let bystander = fresh_manager();
        let origin = elidex_plugin::SecurityOrigin::from_url(&url(DOC_URL)).serialize();
        assert_eq!(bystander.local_get(&origin, "k"), None);
    }
}

// ---------------------------------------------------------------------------
// B6 — IDB versionchange emit drain (IndexedDB-3 §4.2)
// ---------------------------------------------------------------------------

#[test]
fn open_with_higher_version_enqueues_versionchange_request() {
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    engine.set_current_url(Some(url("https://example.com/page")));
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    bind_engine(&mut engine, &mut ctx);
    // Fresh (lazily-created in-memory) backend: version 0 → 3 upgrade.
    let r = ScriptEngine::eval(&mut engine, "indexedDB.open('mydb', 3);", &mut ctx);
    assert!(r.success, "{:?}", r.error);
    engine.unbind();

    let reqs = engine.take_pending_idb_versionchange_requests();
    assert_eq!(reqs.len(), 1, "{reqs:?}");
    assert_eq!(reqs[0].db_name, "mydb");
    assert_eq!(reqs[0].old_version, 0);
    assert_eq!(reqs[0].new_version, Some(3));
    // Owning origin captured at enqueue (the shell's same-origin broadcast key).
    assert_eq!(reqs[0].origin, "https://example.com");
    // Correlation id minted from the request identity (the shell echoes it to
    // unblock the opener) — a real ObjectId, not a placeholder.
    assert_ne!(reqs[0].request_id, 0);
    // Drain-once: a second take is empty.
    assert!(engine.take_pending_idb_versionchange_requests().is_empty());
}

#[test]
fn reopen_at_current_version_enqueues_nothing() {
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    bind_engine(&mut engine, &mut ctx);
    let r = ScriptEngine::eval(&mut engine, "indexedDB.open('db2', 1);", &mut ctx);
    assert!(r.success, "{:?}", r.error);
    // The creating open (0 → 1) enqueued one request.
    assert_eq!(engine.take_pending_idb_versionchange_requests().len(), 1);

    // A second open at the CURRENT version needs no upgrade → no request.
    let r = ScriptEngine::eval(&mut engine, "indexedDB.open('db2', 1);", &mut ctx);
    assert!(r.success, "{:?}", r.error);
    engine.unbind();
    assert!(engine.take_pending_idb_versionchange_requests().is_empty());
}

#[test]
fn delete_database_enqueues_versionchange_request_with_null_new_version() {
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    engine.set_current_url(Some(url("https://example.com/page")));
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    bind_engine(&mut engine, &mut ctx);
    // Create the database at version 2 (this open enqueues its own request —
    // drained below so the deletion's request is observed in isolation).
    let r = ScriptEngine::eval(&mut engine, "indexedDB.open('deldb', 2);", &mut ctx);
    assert!(r.success, "{:?}", r.error);
    assert_eq!(engine.take_pending_idb_versionchange_requests().len(), 1);

    // IndexedDB-3 §5.3 *delete a database* step 6: fire versionchange with
    // db's version and NULL at other contexts' connections.
    let r = ScriptEngine::eval(&mut engine, "indexedDB.deleteDatabase('deldb');", &mut ctx);
    assert!(r.success, "{:?}", r.error);
    engine.unbind();
    let reqs = engine.take_pending_idb_versionchange_requests();
    assert_eq!(reqs.len(), 1, "{reqs:?}");
    assert_eq!(reqs[0].db_name, "deldb");
    assert_eq!(reqs[0].old_version, 2);
    assert_eq!(reqs[0].new_version, None);
    // Owning origin captured at enqueue (same as the upgrade path).
    assert_eq!(reqs[0].origin, "https://example.com");
    assert_ne!(reqs[0].request_id, 0);
}

/// Regression (Codex PR#453 R5): an opaque-origin document's IDB versionchange
/// request must carry the IDENTITY-PRESERVING storage key (the per-VM opaque
/// sentinel), NOT `SecurityOrigin::serialize()`'s lossy `"null"`. A `"null"`
/// key collapses every distinct opaque origin to one string, so the
/// origin-keyed shell broadcast would fan a versionchange out to unrelated
/// sandboxed / `data:` contexts. `storage_origin_key` (shared with localStorage)
/// keeps the sentinel; the earlier `document_origin().serialize()` did not.
#[test]
fn opaque_origin_idb_versionchange_request_carries_identity_key_not_null() {
    // No `set_current_url` → the document origin is opaque.
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    bind_engine(&mut engine, &mut ctx);
    let r = ScriptEngine::eval(&mut engine, "indexedDB.open('odb', 1);", &mut ctx);
    assert!(r.success, "{:?}", r.error);
    engine.unbind();

    let reqs = engine.take_pending_idb_versionchange_requests();
    assert_eq!(reqs.len(), 1, "{reqs:?}");
    assert!(
        reqs[0].origin.starts_with("opaque-origin:"),
        "opaque IDB origin must be the identity-preserving sentinel, got {:?}",
        reqs[0].origin
    );
    assert_ne!(
        reqs[0].origin, "null",
        "a lossy \"null\" key would cross-broadcast between unrelated opaque origins"
    );
    assert_ne!(reqs[0].request_id, 0);
}

/// The `HostDriver::storage_origin_key` accessor (the receive-side parent-message
/// gate reads this for the PARENT key) must return the SAME identity-preserving
/// serialization the send side resolves `targetOrigin` to: a tuple origin's
/// serialization, an opaque origin's per-VM sentinel (never the lossy `"null"`).
#[test]
fn storage_origin_key_trait_accessor_tuple_and_opaque() {
    const DOC_URL: &str = "https://example.com/page";

    // Opaque (no current_url / no origin override) → per-VM sentinel, not "null".
    let (engine, _session, _dom, _doc) = fresh_unbound();
    let opaque_key = HostDriver::storage_origin_key(&engine);
    assert!(
        opaque_key.starts_with("opaque-origin:"),
        "opaque origin must serialize to the identity-preserving sentinel, got {opaque_key:?}"
    );
    assert_ne!(opaque_key, "null");

    // Tuple origin override → its serialization.
    let (mut engine2, _s2, _d2, _doc2) = fresh_unbound();
    let tuple = elidex_plugin::SecurityOrigin::from_url(&url(DOC_URL));
    HostDriver::set_origin(&mut engine2, tuple.clone());
    assert_eq!(
        HostDriver::storage_origin_key(&engine2),
        tuple.serialize(),
        "tuple origin key must equal SecurityOrigin::serialize()"
    );
}

#[test]
fn delete_nonexistent_database_enqueues_nothing() {
    // §5.3 step 4 returns 0 for a nonexistent database BEFORE the step-6
    // versionchange fire — deleting nothing broadcasts nothing (spec-correct;
    // boa enqueued unconditionally).
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    bind_engine(&mut engine, &mut ctx);
    let r = ScriptEngine::eval(&mut engine, "indexedDB.deleteDatabase('ghost');", &mut ctx);
    assert!(r.success, "{:?}", r.error);
    engine.unbind();
    assert!(engine.take_pending_idb_versionchange_requests().is_empty());
}

// ---------------------------------------------------------------------------
// B13 — window.focus() pending-focus (WHATWG HTML §6.6.6 #dom-window-focus)
// ---------------------------------------------------------------------------

#[test]
fn window_focus_sets_pending_flag_and_take_drains_it() {
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    // No request staged → false.
    assert!(!engine.take_pending_focus());

    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    bind_engine(&mut engine, &mut ctx);
    let r = ScriptEngine::eval(&mut engine, "window.focus();", &mut ctx);
    assert!(r.success, "{:?}", r.error);
    engine.unbind();

    // Drain semantics: true ONCE, then false until the next request.
    assert!(engine.take_pending_focus());
    assert!(!engine.take_pending_focus());
}

// ---------------------------------------------------------------------------
// B16 — iframe parent-message out-queue (WHATWG HTML §9.3.3)
// ---------------------------------------------------------------------------

#[test]
fn iframe_depth_routes_post_message_to_parent_fifo() {
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    engine.set_current_url(Some(url("https://iframe.example/child")));
    engine.set_iframe_depth(1);
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    bind_engine(&mut engine, &mut ctx);
    // In an iframe document, postMessage is parent-directed: it must NOT
    // self-deliver (boa's context-routed queue, mirrored by depth).
    let r = ScriptEngine::eval(
        &mut engine,
        "globalThis.got = false;\
         window.addEventListener('message', function() { globalThis.got = true; });\
         window.postMessage('hello', 'https://parent.example');\
         window.postMessage('again', '*');",
        &mut ctx,
    );
    assert!(r.success, "{:?}", r.error);
    assert_eq!(
        eval_string(&mut engine, "globalThis.got ? 'delivered' : 'suppressed'"),
        "suppressed",
        "iframe postMessage must not self-deliver"
    );
    engine.unbind();

    let msgs = engine.take_pending_parent_messages();
    assert_eq!(msgs.len(), 2, "{msgs:?}");
    // FIFO call order; data is the boa-parity ToString wire form.  A URL
    // targetOrigin is resolved to its ORIGIN serialization before queuing
    // (§9.3.3 step 5.3), so a bare-origin URL round-trips unchanged and "*"
    // rides verbatim — the receive side then origin-gates.
    assert_eq!(msgs[0].data, "hello");
    assert_eq!(msgs[0].target_origin, "https://parent.example");
    assert_eq!(msgs[1].data, "again");
    assert_eq!(msgs[1].target_origin, "*");
    // Sender origin (→ MessageEvent.origin) is captured at enqueue on every
    // message, independent of the per-message targetOrigin gate.
    assert_eq!(msgs[0].origin, "https://iframe.example");
    assert_eq!(msgs[1].origin, "https://iframe.example");
    // Drain-once: a second take is empty.
    assert!(engine.take_pending_parent_messages().is_empty());
}

/// Regression (Codex PR#453 review): a URL `targetOrigin` carrying a path /
/// query must be normalized to its ORIGIN serialization before being queued
/// for the parent (WHATWG HTML §9.3.3 window post message steps, step 5.3
/// "Set targetOrigin to parsedURL's origin").  Enqueuing the raw URL would
/// make the receiving-side origin gate reject a legitimately same-origin
/// message.
#[test]
fn iframe_post_message_target_origin_normalized_to_origin() {
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    engine.set_current_url(Some(url("https://iframe.example/child")));
    engine.set_iframe_depth(1);
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    bind_engine(&mut engine, &mut ctx);
    let r = ScriptEngine::eval(
        &mut engine,
        "window.postMessage('m', 'https://parent.example:8443/path?q=1#frag');",
        &mut ctx,
    );
    assert!(r.success, "{:?}", r.error);
    engine.unbind();

    let msgs = engine.take_pending_parent_messages();
    assert_eq!(msgs.len(), 1, "{msgs:?}");
    // Origin only: scheme + host + non-default port, no path/query/fragment.
    assert_eq!(msgs[0].target_origin, "https://parent.example:8443");
    // Sender origin captured at enqueue (→ MessageEvent.origin).
    assert_eq!(msgs[0].origin, "https://iframe.example");
}

/// An opaque (sandboxed) sender's `MessageEvent.origin` is `"null"` (§9.3.3
/// serializes the sender origin; an opaque origin serializes to `"null"`, and
/// opaque senders are deliberately indistinguishable to the receiver). This is
/// the DISPLAYED-origin case — deliberately NOT the identity-preserving sentinel
/// used for the storage-partition KEY (`IdbVersionChangeRequest.origin`), so the
/// two must not be unified onto one derivation.
#[test]
fn iframe_post_message_opaque_sender_origin_is_null() {
    // No `set_current_url` → opaque sender origin.
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    engine.set_iframe_depth(1);
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    bind_engine(&mut engine, &mut ctx);
    let r = ScriptEngine::eval(&mut engine, "window.postMessage('m', '*');", &mut ctx);
    assert!(r.success, "{:?}", r.error);
    engine.unbind();

    let msgs = engine.take_pending_parent_messages();
    assert_eq!(msgs.len(), 1, "{msgs:?}");
    assert_eq!(msgs[0].origin, "null");
}

/// Regression (Codex PR#453 R11, P1): an opaque sender's `"/"` targetOrigin must
/// carry the IDENTITY-PRESERVING key (the per-VM sentinel), NOT the display
/// `"null"` — otherwise the future receiver gate would let any opaque parent
/// match. (The `MessageEvent.origin` sender field stays the display `"null"`;
/// only the gate KEY is identity-preserving.)
#[test]
fn iframe_post_message_opaque_sender_slash_target_is_identity_key() {
    // No `set_current_url` → opaque sender origin.
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    engine.set_iframe_depth(1);
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    bind_engine(&mut engine, &mut ctx);
    let r = ScriptEngine::eval(&mut engine, "window.postMessage('m', '/');", &mut ctx);
    assert!(r.success, "{:?}", r.error);
    engine.unbind();

    let msgs = engine.take_pending_parent_messages();
    assert_eq!(msgs.len(), 1, "{msgs:?}");
    // The gate KEY is the identity-preserving sentinel...
    assert!(
        msgs[0].target_origin.starts_with("opaque-origin:"),
        "opaque '/' gate key must be the identity sentinel, got {:?}",
        msgs[0].target_origin
    );
    assert_ne!(msgs[0].target_origin, "null");
    // ...while the displayed sender origin stays the spec `"null"`.
    assert_eq!(msgs[0].origin, "null");
}

/// Regression (Codex PR#453 R11): an opaque URL targetOrigin (e.g. `data:`) is a
/// fresh opaque that can never be same-origin with the parent, so the message is
/// undeliverable and FAIL-CLOSED at the send site — never enqueued with a lossy
/// `"null"` gate the receiver could alias.
#[test]
fn iframe_post_message_opaque_url_target_is_fail_closed() {
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    engine.set_current_url(Some(url("https://iframe.example/child")));
    engine.set_iframe_depth(1);
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    bind_engine(&mut engine, &mut ctx);
    // A valid, syntactically-parseable URL whose origin is opaque.
    let r = ScriptEngine::eval(
        &mut engine,
        "window.postMessage('m', 'data:text/html,x');",
        &mut ctx,
    );
    assert!(r.success, "{:?}", r.error);
    engine.unbind();

    // Fail-closed: undeliverable opaque target → nothing enqueued.
    assert!(
        engine.take_pending_parent_messages().is_empty(),
        "opaque URL target must be dropped at the send site"
    );
}

/// Regression (Codex PR#453 R12): the message is serialized BEFORE the
/// fail-closed drop (§9.3.3 step 7 precedes the origin-gate return at step 8.1),
/// so a throwing `toString` surfaces even when the opaque target makes the
/// message undeliverable — it must not be silently suppressed by the early drop.
#[test]
fn iframe_post_message_throwing_tostring_surfaces_before_fail_closed() {
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    engine.set_current_url(Some(url("https://iframe.example/child")));
    engine.set_iframe_depth(1);
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    bind_engine(&mut engine, &mut ctx);
    let r = ScriptEngine::eval(
        &mut engine,
        "window.postMessage({ toString() { throw new Error('boom'); } }, 'data:text/html,y');",
        &mut ctx,
    );
    assert!(
        !r.success,
        "a throwing toString must surface before the fail-closed drop"
    );
    engine.unbind();
    assert!(engine.take_pending_parent_messages().is_empty());
}

#[test]
fn top_level_post_message_self_delivers_and_fifo_stays_empty() {
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    // Default depth 0 = top-level: the existing VM-internal self-delivery is
    // UNCHANGED and nothing reaches the parent FIFO.
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    bind_engine(&mut engine, &mut ctx);
    let r = ScriptEngine::eval(
        &mut engine,
        "globalThis.got = false;\
         window.addEventListener('message', function() { globalThis.got = true; });\
         window.postMessage('hello', '*');",
        &mut ctx,
    );
    assert!(r.success, "{:?}", r.error);
    assert_eq!(
        eval_string(&mut engine, "globalThis.got ? 'delivered' : 'suppressed'"),
        "delivered",
        "top-level postMessage keeps self-delivering"
    );
    engine.unbind();
    assert!(engine.take_pending_parent_messages().is_empty());
}

// ---------------------------------------------------------------------------
// B21 — IDB versionchange deliver (IndexedDB-3 §4.2, receive half)
// ---------------------------------------------------------------------------

#[test]
fn deliver_idb_versionchange_fires_on_open_connections() {
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    bind_engine(&mut engine, &mut ctx);
    // Open a connection and hang a versionchange handler on it (the open
    // success is delivered by the task drain inside `eval`).
    let r = ScriptEngine::eval(
        &mut engine,
        "globalThis.fired = 'no';\
         var req = indexedDB.open('vdb', 1);\
         req.onsuccess = function(e) {\
             var db = e.target.result;\
             db.onversionchange = function(ev) {\
                 globalThis.fired = ev.oldVersion + ':' + (ev.newVersion === null ? 'null' : ev.newVersion);\
             };\
         };",
        &mut ctx,
    );
    assert!(r.success, "{:?}", r.error);

    // Another db's broadcast is a no-op for this connection.
    engine.deliver_idb_versionchange("otherdb", 1, Some(2));
    assert_eq!(eval_string(&mut engine, "globalThis.fired"), "no");

    // The matching broadcast fires versionchange with old/new versions.
    engine.deliver_idb_versionchange("vdb", 1, Some(2));
    assert_eq!(eval_string(&mut engine, "globalThis.fired"), "1:2");

    // A deletion-initiated change carries newVersion = null.
    engine.deliver_idb_versionchange("vdb", 1, None);
    assert_eq!(eval_string(&mut engine, "globalThis.fired"), "1:null");
    engine.unbind();
}

#[test]
fn deliver_idb_versionchange_without_connection_is_noop() {
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    bind_engine(&mut engine, &mut ctx);
    // No open connection to any database — must be a silent no-op.
    engine.deliver_idb_versionchange("nowhere", 1, Some(2));
    engine.unbind();
    // Unbound is also a defensive no-op (assume-bound contract, gated like
    // `deliver_history_step_events`).
    engine.deliver_idb_versionchange("nowhere", 1, Some(2));
}
