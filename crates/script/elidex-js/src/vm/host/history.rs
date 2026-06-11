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
//!   steps" → §7.4.4 "URL and history update steps") run **synchronously**: a
//!   same-origin gate (§7.2.5 step 6.3), then an in-place `current_url` +
//!   `history.state` update (and, for `pushState`, a `history.length` bump for the
//!   appended entry), then an **enqueue** for the shell to persist.
//! - `history.length` reads `history_length` (shell-pushed, plus the synchronous
//!   `pushState` bump the shell reconciles); `history.state` reads the
//!   synchronously-maintained `current_state`.
//!
//! `history.state` holds the value as a bare [`JsValue`] (identity round-trip);
//! `StructuredSerializeForStorage` (§7.2.5 step 4) is part of the deferred
//! `#11-history-state-traversal-popstate-fidelity` slot.

#![cfg(feature = "engine")]

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
    // (the `NavigationController` owns the stack; `set_history_length` syncs it),
    // plus the synchronous `pushState` bump the shell reconciles after the drain.
    // Clamp via `u32` to satisfy clippy::cast_lossless.
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
    // §7.2.5 `go(optional long delta = 0)` — the WebIDL `long` is ToInt32.
    // `go(0)` reloads the current entry (the shell's `NavigationController.go(0)`
    // re-fetches), so it enqueues `Go(0)` rather than no-op'ing.
    let delta_f = match args.first().copied().unwrap_or(JsValue::Undefined) {
        JsValue::Undefined => 0.0,
        other => coerce::to_number(ctx.vm, other)?,
    };
    #[allow(clippy::cast_possible_truncation)]
    let delta = if delta_f.is_finite() {
        // For history.go an out-of-range delta no-ops in the shell controller, so
        // a saturating cast is observable-equivalent to the modular ToInt32.
        delta_f
            .trunc()
            .clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32
    } else {
        0
    };
    enqueue_traversal(ctx, HistoryAction::Go(delta));
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// State-mutation methods (pushState / replaceState) — synchronous + enqueue
// ---------------------------------------------------------------------------

/// Shared body for `pushState` / `replaceState` (WHATWG HTML §7.2.5 "shared
/// history push/replace state steps").  Runs the same-origin gate (step 6.3),
/// then the synchronous URL-and-history-update half (§7.4.4) — updating
/// `current_url` + `history.state` in place so a same-script read observes them
/// — then enqueues a [`HistoryAction`] for the shell to persist into its
/// `NavigationController`.
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

    // §7.2.5 step 5/6: newURL is the document URL unless `url` is given.
    let url_arg = args.get(2).copied().unwrap_or(JsValue::Undefined);
    let (new_url, pushed_url) = if matches!(url_arg, JsValue::Undefined | JsValue::Null) {
        (ctx.vm.navigation.current_url.clone(), None)
    } else {
        let sid = coerce::to_string(ctx.vm, url_arg)?;
        let input = ctx.vm.strings.get_utf8(sid);
        // §7.2.5 step 6.1: parse `url` relative to the current document URL.
        let Ok(parsed) = ctx.vm.navigation.current_url.join(&input) else {
            let syntax = ctx.vm.well_known.dom_exc_syntax_error;
            return Err(VmError::dom_exception(
                syntax,
                format!(
                    "Failed to execute 'pushState'/'replaceState' on 'History': invalid URL '{input}'."
                ),
            ));
        };
        // §7.2.5 step 6.3: "If document cannot have its URL rewritten to newURL,
        // throw a SecurityError" — newURL's origin must equal the document origin
        // (S1b `document_origin()`).  A sandboxed/opaque document can only rewrite
        // to its own opaque origin, which a fresh parse never reproduces, so its
        // pushState always throws — matching browsers.  For an unsandboxed
        // document this is the cross-origin check (`document_origin` ==
        // `current_url`'s tuple origin).
        if ctx.vm.document_origin() != elidex_plugin::SecurityOrigin::from_url(&parsed) {
            let security = ctx.vm.well_known.dom_exc_security_error;
            return Err(VmError::dom_exception(
                security,
                "Failed to execute 'pushState'/'replaceState' on 'History': the new URL must be same-origin with the document.".to_string(),
            ));
        }
        let serialized = parsed.to_string();
        (parsed, Some(serialized))
    };

    // §7.2.5 step 10 → §7.4.4 "URL and history update steps": synchronously set
    // the document URL + the current entry's state, so a same-script
    // `location.href` / `history.state` read observes them (unlike boa, which is
    // enqueue-only and reads stale).
    ctx.vm.navigation.current_url = new_url;
    ctx.vm.navigation.current_state = state;

    // `pushState` appends a new session-history entry (§7.4.4 with history
    // handling "push"), so `history.length` grows by one synchronously; a
    // same-turn read must observe it.  `replaceState` ("replace") overwrites the
    // current entry, leaving the count unchanged.  The shell reconciles the
    // authoritative count via `set_history_length` after draining the enqueued
    // action (which overwrites, so there is no double-count).
    if !replace_index {
        ctx.vm.navigation.history_length = ctx.vm.navigation.history_length.saturating_add(1);
    }

    // Enqueue for the shell to persist into its NavigationController.
    let action = if replace_index {
        HistoryAction::ReplaceState {
            url: pushed_url,
            title,
        }
    } else {
        HistoryAction::PushState {
            url: pushed_url,
            title,
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
