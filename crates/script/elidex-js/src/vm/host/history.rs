//! `history` global — a subset of the `History` interface
//! (WHATWG HTML §7.2.5 "The History interface").
//!
//! # Navigation model (S1c — enqueue + synchronous pushState)
//!
//! The shell's `NavigationController` is the single session-history source of
//! truth (the VM holds only a current-document view, see [`super::navigation`]):
//!
//! - `back()` / `forward()` / `go(delta)` are session-history *traversals*
//!   (WHATWG HTML §7.4.6 "Applying the history step" — async document loads the
//!   shell performs): they **enqueue** a [`HistoryAction`] and leave the
//!   current-document view (`current_url` / `history.state` / `history.length`)
//!   **untouched** — the traversal has not committed, so a same-turn read still
//!   sees the current entry (matching `location.href` reading the old URL after an
//!   enqueue-only navigation), and a no-op traversal (`back` at the first entry,
//!   `go(0)`) changes nothing.  The target entry's state restoration + `popstate`
//!   on commit are the shell's job (slot
//!   `#11-history-state-traversal-popstate-fidelity`).
//! - `pushState()` / `replaceState()` (§7.2.5 "shared history push/replace state
//!   steps" → the "URL and history update steps" (§7.4.4)) run **synchronously**: the
//!   URL-rewrite gate (§7.2.5 step 6.3 — "can have its URL rewritten", a
//!   document-URL check), then an in-place `current_url` + `history.state` update
//!   (and, for `pushState`, the current index + length: `length = index + 1`),
//!   then an **enqueue** for the shell to persist.
//! - `history.length` reads `history_length`; `history.state` reads
//!   `current_state` — both synchronously maintained.  The VM tracks the current
//!   session-history *index* so `pushState` can set the length correctly even
//!   when the current entry is not the last (computing `index + 1`, not
//!   `length + 1`); the shell re-pushes the authoritative `(index, length)` via
//!   `set_session_history` after a navigation/traversal commits.
//!
//! `history.state` holds the value as a bare [`JsValue`] (identity round-trip);
//! `StructuredSerializeForStorage` (§7.2.5 step 4) is part of the deferred
//! `#11-history-state-traversal-popstate-fidelity` slot.

#![cfg(feature = "engine")]

use elidex_plugin::can_have_url_rewritten;
use elidex_script_session::HistoryAction;

use super::super::coerce;
use super::super::value::{JsValue, NativeContext, VmError};
use super::super::VmInner;

// ---------------------------------------------------------------------------
// Accessors
// ---------------------------------------------------------------------------

pub(super) fn native_history_get_length(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // `history.length` (§7.2.5) is the session-history entry count: shell-pushed
    // (the `NavigationController` owns the stack; `set_session_history` syncs the
    // index + length atomically) and advanced synchronously by `pushState`
    // (`length = current_index + 1`).  Clamp via `u32` to satisfy
    // clippy::cast_lossless.
    let len = u32::try_from(ctx.vm.navigation.history_length).unwrap_or(u32::MAX);
    Ok(JsValue::Number(f64::from(len)))
}

pub(super) fn native_history_get_state(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // `history.state` (§7.2.5) — the current entry's state, set synchronously by
    // pushState/replaceState (§7.4.4).  A traversal leaves it untouched (async;
    // the shell restores the target entry's state on commit), so a same-turn read
    // still sees the current entry.
    Ok(ctx.vm.navigation.current_state)
}

pub(super) fn native_history_get_scroll_restoration(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // Phase 2: always `"auto"`.  A writable setter arrives with the
    // scroll-anchoring work in PR5+.
    Ok(JsValue::String(ctx.vm.well_known.auto))
}

// ---------------------------------------------------------------------------
// Navigation methods (back / forward / go) — enqueue-only traversals
// ---------------------------------------------------------------------------

/// Enqueue a session-history traversal for the shell.  The traversal is async
/// (an `apply the history step` document load the shell performs — WHATWG HTML
/// §7.4.6), so this mutates **none** of the current-document view: `current_url`,
/// `history.state`, and `history.length` all commit only when the shell loads the
/// target entry.  Leaving `history.state` untouched (rather than null-clearing it)
/// keeps a same-turn read seeing the current entry's state and makes a no-op
/// traversal (`back` at the first entry, `go(0)`) a true no-op; the target entry's
/// state restoration + `popstate` on commit are the shell's job (slot
/// `#11-history-state-traversal-popstate-fidelity`).
fn enqueue_traversal(ctx: &mut NativeContext<'_>, action: HistoryAction) {
    ctx.vm.navigation.enqueue_history(action);
}

pub(super) fn native_history_back(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    enqueue_traversal(ctx, HistoryAction::Back);
    Ok(JsValue::Undefined)
}

pub(super) fn native_history_forward(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    enqueue_traversal(ctx, HistoryAction::Forward);
    Ok(JsValue::Undefined)
}

pub(super) fn native_history_go(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    // §7.2.5 `go(optional long delta = 0)`.  The WebIDL `long` conversion is
    // ConvertToInt(V, 32, "signed") (Web IDL §3.2.4.9) = ECMAScript ToInt32
    // (ECMA-262 §7.1.7): it wraps **modulo 2^32**, not a saturating clamp.  So
    // `go(4294967295)` becomes `-1` (a one-step back) and must NOT clamp to
    // `i32::MAX` (which the shell controller would treat as an out-of-range
    // no-op).  A missing/`undefined` argument is ToInt32(undefined) = 0, so
    // `go()` / `go(0)` reload the current entry (the shell's `go(0)` re-fetches).
    let delta = coerce::to_int32(ctx.vm, args.first().copied().unwrap_or(JsValue::Undefined))?;
    enqueue_traversal(ctx, HistoryAction::Go(delta));
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// State-mutation methods (pushState / replaceState) — synchronous + enqueue
// ---------------------------------------------------------------------------

/// Shared body for `pushState` / `replaceState` (WHATWG HTML §7.2.5 "shared
/// history push/replace state steps").  Runs the URL-rewrite gate (step 6.3 —
/// [`can_have_url_rewritten`]), then the synchronous URL-and-history-update half
/// (§7.4.4) — updating `current_url` + `history.state` in place so a same-script
/// read observes them — then enqueues a [`HistoryAction`] for the shell to
/// persist into its `NavigationController`.
fn state_mutate(
    ctx: &mut NativeContext<'_>,
    args: &[JsValue],
    replace_index: bool,
) -> Result<(), VmError> {
    let state = args.first().copied().unwrap_or(JsValue::Undefined);

    // `unused` (the title, §7.2.5) — coerced for the WebIDL ToString side-effect,
    // then ignored.  Empty when omitted (matches boa); carried on the action only
    // for API compat.
    let title = if let Some(&title_arg) = args.get(1) {
        let sid = coerce::to_string(ctx.vm, title_arg)?;
        ctx.vm.strings.get_utf8(sid)
    } else {
        String::new()
    };

    // WebIDL argument conversion (`pushState(any data, DOMString unused,
    // optional USVString? url = null)`) runs left-to-right **before** the method
    // algorithm — so the `url` (arg 2) is coerced to a string HERE, before the
    // step-3 serialize below (CR-4 / WebIDL arg-conversion order). `null` when the
    // arg is omitted / null / undefined (`USVString?` default null).
    let url_arg = args.get(2).copied().unwrap_or(JsValue::Null);
    let url_string: Option<String> = if matches!(url_arg, JsValue::Undefined | JsValue::Null) {
        None
    } else {
        let sid = coerce::to_string(ctx.vm, url_arg)?;
        Some(ctx.vm.strings.get_utf8(sid))
    };

    // §7.2.5 step 3: `serializedData = StructuredSerializeForStorage(data)` — after
    // the WebIDL arg coercions (above), before the step-5 URL parse + gate. The
    // interim JSON-shortcut serializer is TOTAL (never throws): a representable value
    // → `Some(bytes)`, anything else → `None`. A throwing `toJSON` does NOT abort
    // (`StructuredSerializeInternal` serializes objects via `? Get`, §2.7.3 step
    // 26.4, and never invokes `toJSON`) — it degrades
    // like BigInt/cyclic (CR-3). `None` = no restorable state (a cross-document
    // traversal restores `null`). The spec's "Rethrow any exceptions" is vacuous in
    // the interim; the full walker slot re-enables real throwing (`DataCloneError` +
    // getter-exception propagation), at which point this regains a fallible result.
    let serialized_state =
        super::structured_serialize::structured_serialize_for_storage(ctx, state);

    // §7.2.5 step 5/6: newURL is the document URL when `url` is null OR the **empty
    // string** — the historical empty-string special case, which (per step 6's note)
    // is NOT parsed, unlike `location.href = ""`.  Otherwise parse the (already
    // coerced) string relative to the document URL and run the gate.
    let new_url = if let Some(input) = url_string.filter(|s| !s.is_empty()) {
        // §7.2.5 step 6.1: parse `url` relative to the current document URL.
        let Ok(parsed) = ctx.vm.navigation.current_url.join(&input) else {
            // §7.2.5 step 6.2: "If newURL is failure, throw a SecurityError" —
            // pushState/replaceState report **SecurityError** (not SyntaxError)
            // on a URL that fails to parse, unlike the `location.href =` setter
            // (§7.2.4 href-setter step 3, which throws SyntaxError).
            let security = ctx.vm.well_known.dom_exc_security_error;
            return Err(VmError::dom_exception(
                security,
                format!(
                    "Failed to execute 'pushState'/'replaceState' on 'History': invalid URL '{input}'."
                ),
            ));
        };
        // §7.2.5 step 6.3: "If document cannot have its URL rewritten to
        // newURL, throw a SecurityError".  This is the document-URL-rewrite
        // check ([`can_have_url_rewritten`]), NOT an origin comparison: they
        // diverge for an inherited-origin `about:blank`/`srcdoc` document,
        // whose URL is `about:blank`, so a rewrite to the inherited tuple
        // origin's URL fails the scheme/host check even though
        // `document_origin()` would match.  For an ordinary http(s) document
        // this still rejects cross-origin rewrites (scheme/host/port differ).
        if !can_have_url_rewritten(&ctx.vm.navigation.current_url, &parsed) {
            let security = ctx.vm.well_known.dom_exc_security_error;
            return Err(VmError::dom_exception(
                security,
                "Failed to execute 'pushState'/'replaceState' on 'History': the new URL must be same-origin with the document and rewritable from its URL.".to_string(),
            ));
        }
        parsed
    } else {
        // `url` null / omitted / empty string → keep the current document URL (the
        // empty-string special case preserves a trailing `#fragment`).
        ctx.vm.navigation.current_url.clone()
    };

    // The enqueued action carries the **effective** URL (newURL), never `None`, so
    // it is self-contained: the per-turn queue's drop-oldest cap
    // ([`NavigationState::pending_history`]) can evict an earlier action without
    // stranding a later no-URL action that would otherwise be applied against the
    // shell's (now stale) current URL.  (boa enqueues `None` for a no-URL call;
    // the VM resolves it to the current document URL up front.)
    let pushed_url = new_url.to_string();

    // §7.2.5 step 10 → the "URL and history update steps" (§7.4.4): synchronously set
    // the document URL + restore `history.state` from the just-serialized entry, so a
    // same-script `location.href` / `history.state` read observes them (unlike boa,
    // which is enqueue-only and reads stale). §7.4.4 restores the history object state
    // from the NEW entry AFTER serialization, so `history.state` is the serialized
    // SNAPSHOT (`seed_history_state` = `StructuredDeserialize(bytes)`, or `null` for a
    // JSON-unrepresentable value), NOT the live `state` object — else `history.state`
    // would observe post-pushState mutations of the passed object (`o.n = 2`) or keep
    // a value a later traversal/reload restores differently (R2-F2 / Codex R2).
    ctx.vm.navigation.current_url = new_url;
    ctx.vm.seed_history_state(serialized_state.clone());

    // `pushState` appends a new entry after the current one, discarding any
    // forward entries, and moves to it — so synchronously (§7.4.4) the current
    // index advances and the length becomes `index + 1` (the new entry is now the
    // last), saturating at the session-history cap (the shell evicts the oldest
    // over the cap).  Computing the length from the index — rather than
    // `length += 1` — is what keeps it correct when the current entry is **not**
    // the last (e.g. after a `back` left forward entries the push discards):
    // `length += 1` would over-count by the forward count.  `replaceState`
    // overwrites the current entry in place, changing neither.  The shell
    // re-pushes the authoritative `(index, length)` via `set_session_history`
    // after it drains/commits.
    if !replace_index {
        ctx.vm.navigation.record_push_state();
    }

    // Enqueue for the shell to persist into its NavigationController (self-
    // contained — `url` is always the effective newURL, see above).
    let action = if replace_index {
        HistoryAction::ReplaceState {
            url: Some(pushed_url),
            title,
            serialized_state,
        }
    } else {
        HistoryAction::PushState {
            url: Some(pushed_url),
            title,
            serialized_state,
        }
    };
    ctx.vm.navigation.enqueue_history(action);
    Ok(())
}

pub(super) fn native_history_push_state(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    state_mutate(ctx, args, false)?;
    Ok(JsValue::Undefined)
}

pub(super) fn native_history_replace_state(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    state_mutate(ctx, args, true)?;
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// Installation
// ---------------------------------------------------------------------------

impl VmInner {
    /// Install `globalThis.history` (WHATWG HTML §7.2.5).
    pub(in crate::vm) fn register_history_global(&mut self) {
        let obj_id = self.create_object_with_methods(HISTORY_METHODS);
        self.install_ro_accessors(obj_id, HISTORY_RO_ACCESSORS);
        let name = self.well_known.history;
        self.globals.insert(name, JsValue::Object(obj_id));
    }
}

const HISTORY_METHODS: &[(&str, super::super::NativeFn)] = &[
    ("back", native_history_back),
    ("forward", native_history_forward),
    ("go", native_history_go),
    ("pushState", native_history_push_state),
    ("replaceState", native_history_replace_state),
];

const HISTORY_RO_ACCESSORS: &[(&str, super::super::NativeFn)] = &[
    ("length", native_history_get_length),
    ("state", native_history_get_state),
    ("scrollRestoration", native_history_get_scroll_restoration),
];
