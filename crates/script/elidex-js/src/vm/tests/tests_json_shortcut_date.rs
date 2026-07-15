//! JSON-shortcut structured-serialize × `Date` (Codex R4 / R5).
//!
//! Worker / ServiceWorker `postMessage` and `history.state` stand in for
//! StructuredSerialize with a JSON shortcut
//! (`natives_json::stringify_for_structured_shortcut`). A `Date` is [Serializable],
//! so real structured clone round-trips it — but `JSON.stringify` flattens it
//! through `toJSON` (ECMA-262 §21.4.4.37) into an ISO String, which would hand the
//! peer (or a history traversal) a **String** where structured clone gives a Date.
//! Every *other* shortcut gap fails loudly (a `BigInt`, a cycle, the depth cap all
//! throw), so only a Date needed an explicit arm.
//!
//! What these tests pin is *where* that arm lives: **inside** the JSON walk, on the
//! value `SerializeJSONProperty` observes and before the `toJSON` hook. Codex R5
//! showed a pre-scan outside the walk gets three things wrong, one test each below —
//! it cannot see an accessor-returned Date, it reorders user exceptions ahead of
//! nothing (reporting `DataCloneError` before user code runs at all), and it adds an
//! unbounded recursion the encoder's depth cap does not protect.
//!
//! The worker cases live here rather than in `tests_worker.rs` so that file stays
//! under the 1000-line backstop. Faithful encoding =
//! `#11-worker-structured-serialize` / `#11-history-state-structured-serialize-fidelity`.

#![cfg(feature = "engine")]
// `with_worker_vm` binds a worker VM through the `unsafe` `Vm::bind_worker` contract
// (see `tests_worker`); the binding is scoped to each test's locals.
#![allow(unsafe_code)]

use super::tests_worker::{eval_str_on, with_main_vm, with_worker_vm, WORKER_URL};

/// A worker that never receives anything — every `postMessage` below fails during
/// serialization, before dispatch.
const NEW_NOOP_WORKER: &str =
    r#"const w = new Worker("data:text/javascript,self.onmessage=function(){}");"#;

#[test]
fn worker_post_message_rejects_a_date() {
    with_main_vm(|vm| {
        // A Date as a plain data property.
        assert_eq!(
            eval_str_on(
                vm,
                &format!(
                    "{NEW_NOOP_WORKER}
                     try {{ w.postMessage({{ d: new Date(0) }}); 'no-throw' }} catch (e) {{ e.name }}"
                ),
            ),
            "DataCloneError"
        );
        // Nested behind an array and a plain object: wherever the JSON walk reaches,
        // the check reaches.
        assert_eq!(
            eval_str_on(
                vm,
                &format!(
                    "{NEW_NOOP_WORKER}
                     try {{ w.postMessage([{{ inner: [new Date(0)] }}]); 'no-throw' }} catch (e) {{ e.name }}"
                ),
            ),
            "DataCloneError"
        );
        // A Date-free payload still serializes — the check must not over-reject.
        assert_eq!(
            eval_str_on(
                vm,
                &format!(
                    "{NEW_NOOP_WORKER}
                     try {{ w.postMessage({{ a: [1, \"x\", {{ b: true }}] }}); 'ok' }} catch (e) {{ e.name }}"
                ),
            ),
            "ok"
        );
    });
}

#[test]
fn worker_scope_post_message_rejects_a_date() {
    // Worker-side `self.postMessage` runs the same encoder (one shared core).
    with_worker_vm("", WORKER_URL, true, |vm| {
        assert_eq!(
            eval_str_on(
                vm,
                "try { postMessage({ d: new Date(0) }); 'no-throw' } catch (e) { e.name }",
            ),
            "DataCloneError"
        );
    });
}

/// Codex R5: a Date reached through an **accessor**. A pre-scan outside the walk
/// had to skip accessors — invoking a getter would be an observable side effect —
/// so the getter's Date sailed on into `toJSON` and the peer received an ISO
/// String anyway. The in-walk check inspects the value `SerializeJSONProperty`
/// actually observes, i.e. the getter's return value, so it is caught; and because
/// the walk is the only traversal, the getter still runs exactly **once**.
#[test]
fn worker_post_message_rejects_a_date_returned_by_an_accessor() {
    with_main_vm(|vm| {
        assert_eq!(
            eval_str_on(
                vm,
                &format!(
                    "{NEW_NOOP_WORKER}
                     globalThis.__gets = 0;
                     const payload = {{ get d() {{ globalThis.__gets++; return new Date(0); }} }};
                     try {{ w.postMessage(payload); 'no-throw' }} catch (e) {{ e.name }}"
                ),
            ),
            "DataCloneError"
        );
        assert_eq!(eval_str_on(vm, "String(globalThis.__gets)"), "1");
    });
}

/// Codex R5: a user exception raised *during* serialization must still propagate
/// unchanged (the documented contract on `serialize_message`) even when the payload
/// also holds a Date. A pre-scan ran before any user code, so it answered
/// `DataCloneError` and the user's `TypeError` never surfaced. In-walk, JSON's own
/// order applies: the top-level `toJSON` — or an earlier enumerable getter — fires
/// first, so the user's exception wins.
#[test]
fn worker_post_message_propagates_a_user_throw_ahead_of_the_date_check() {
    with_main_vm(|vm| {
        assert_eq!(
            eval_str_on(
                vm,
                &format!(
                    "{NEW_NOOP_WORKER}
                     const payload = {{ d: new Date(0), toJSON() {{ throw new TypeError('x'); }} }};
                     try {{ w.postMessage(payload); 'no-throw' }} catch (e) {{ e.name }}"
                ),
            ),
            "TypeError"
        );
        assert_eq!(
            eval_str_on(
                vm,
                &format!(
                    "{NEW_NOOP_WORKER}
                     const payload = {{ get a() {{ throw new TypeError('x'); }}, d: new Date(0) }};
                     try {{ w.postMessage(payload); 'no-throw' }} catch (e) {{ e.name }}"
                ),
            ),
            "TypeError"
        );
    });
}

/// Codex R5: the Date check must not introduce a recursion of its own. A pre-scan
/// walked the whole graph before the encoder ran, with no depth bound — a
/// deep-but-otherwise-representable payload could overflow the Rust stack before
/// JSON's own `MAX_JSON_DEPTH` had a chance to fire. In-walk there is only one
/// traversal, so the existing depth cap still triggers and maps to `DataCloneError`
/// exactly as it did before this PR.
#[test]
fn worker_post_message_hits_the_depth_cap_not_the_stack() {
    with_main_vm(|vm| {
        assert_eq!(
            eval_str_on(
                vm,
                &format!(
                    "{NEW_NOOP_WORKER}
                     let deep = [];
                     for (let i = 0; i < 10000; i++) {{ deep = [deep]; }}
                     try {{ w.postMessage(deep); 'no-throw' }} catch (e) {{ e.name }}"
                ),
            ),
            "DataCloneError"
        );
    });
}
