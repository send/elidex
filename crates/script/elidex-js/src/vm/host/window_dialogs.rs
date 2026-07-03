//! Simple dialogs (WHATWG HTML §8.9.1) + `window.open` (§7.2.2.1) — the
//! sandbox-gated `Window` method group (S5-4c).
//!
//! All four natives (`alert` / `confirm` / `prompt` / `open`) are
//! **marshal-only**: they WebIDL-convert their JsValue arguments, run the
//! engine-independent gate (the `elidex_plugin::sandbox` predicates via
//! `HostData`, and the `elidex_script_session::window_open_disposition`
//! target dispatch), and either enqueue on the `NavigationState`
//! back-channels or return the spec's step-1 constant.  No algorithm bodies
//! live here (Layering mandate) — the decision logic is in the two
//! engine-independent crates; this module only coerces at the JsValue
//! boundary and routes.  Split out of `window.rs` at S5-4c landing
//! (touch-time cohesion seam — the dialog/open group is self-contained).
//!
//! The natives are installed into `WINDOW_METHODS` by
//! [`super::window::register_window_prototype`].

#![cfg(feature = "engine")]

use elidex_script_session::{
    window_open_disposition, NamedFrameNavigation, NavigationRequest, OpenTabRequest,
    WindowOpenDisposition, WindowOpenIntent,
};

use super::super::coerce;
use super::super::value::{JsValue, NativeContext, VmError};

/// HTML §8.9.1 *cannot show simple dialogs* (`#cannot-show-simple-dialogs`),
/// composed at marshal scale for the three simple-dialog natives:
///
/// - **Step 1** — the *sandboxed modals flag*: the canonical predicate
///   [`elidex_plugin::sandbox::modals_allowed`] over the document's flags
///   via `HostData` (no installed `HostData` = unsandboxed, permissive —
///   the absence of a security context never silently denies, the
///   `scripts_allowed` convention).
/// - **Step 2** (relevant-settings-object origin vs top-level origin not
///   same origin-domain) is *subsumed*: the permanent step-4 opt-in below
///   fires first-class before step 2 could ever be observed, so the
///   top-level origin is NOT threaded to the VM for it (demand-gated on a
///   real presentation surface).
/// - **Step 4** — *"Optionally, return true"*: elidex's UA policy opts in
///   **permanently** (headless — no dialog surface exists), so presentation
///   never happens and each method's step-1 return value (alert →
///   undefined / confirm → false / prompt → null) is simultaneously
///   spec-conformant and boa-parity.
///
/// Security by structure: each native returns through this chokepoint
/// before any presentation branch exists, so a future shell modal surface
/// can only attach behind the gate.
fn cannot_show_simple_dialogs(ctx: &mut NativeContext<'_>) -> bool {
    // Step 1: active sandboxing flag set has the sandboxed modals flag.
    if !ctx.host_opt().is_none_or(|hd| hd.modals_allowed()) {
        return true;
    }
    // Step 4: the permanent UA opt-in (see above; step 2 is unobservable
    // behind it, step 3's termination nesting level is never nonzero here —
    // the VM has no `close()`-driven termination nesting).
    true
}

/// Run the WebIDL `DOMString` conversion for one simple-dialog argument,
/// discarding the result.  The conversion runs BEFORE the method steps per
/// WebIDL argument-conversion order, so a passed object's `toString` side
/// effects are observable even when the dialog cannot show — which is why
/// this cannot be skipped despite no presentation surface consuming the
/// string.  Absent / `undefined` arguments take the `optional DOMString =
/// ""` default with no conversion (for `alert`'s overload pair the
/// distinction is unobservable: `ToString(undefined)` has no side effects).
fn coerce_dialog_arg(
    ctx: &mut NativeContext<'_>,
    args: &[JsValue],
    idx: usize,
) -> Result<(), VmError> {
    if let Some(&arg) = args.get(idx) {
        if !matches!(arg, JsValue::Undefined) {
            coerce::to_string(ctx.vm, arg)?;
        }
    }
    Ok(())
}

/// Coerce an optional `DOMString`/`USVString` argument to an owned `String`,
/// substituting `default` when the argument is absent or `undefined` (the
/// WebIDL `optional … = "…"` default — `ToString(undefined)` is not run).
/// The `ToString` of a present non-`undefined` argument executes for its
/// side effects per WebIDL argument-conversion order.
fn coerce_string_arg(
    ctx: &mut NativeContext<'_>,
    args: &[JsValue],
    idx: usize,
    default: &str,
) -> Result<String, VmError> {
    match args.get(idx).copied().unwrap_or(JsValue::Undefined) {
        JsValue::Undefined => Ok(default.to_string()),
        v => {
            let sid = coerce::to_string(ctx.vm, v)?;
            Ok(ctx.vm.strings.get_utf8(sid))
        }
    }
}

/// `window.alert(message)` (WHATWG HTML §8.9.1, `#dom-alert`).
pub(super) fn native_window_alert(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    // WebIDL conversion first (`alert()` / `alert(DOMString message)`).
    coerce_dialog_arg(ctx, args, 0)?;
    // §8.9.1 alert steps step 1: cannot show simple dialogs → return.
    if cannot_show_simple_dialogs(ctx) {
        return Ok(JsValue::Undefined);
    }
    // Steps 3-6 (present + pause) — unreachable while the UA opts into
    // §8.9.1 step 4 permanently; a shell modal surface attaches here,
    // behind the gate.
    Ok(JsValue::Undefined)
}

/// `window.confirm(message)` (WHATWG HTML §8.9.1, `#dom-confirm`).
pub(super) fn native_window_confirm(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    // WebIDL conversion first (`optional DOMString message = ""`).
    coerce_dialog_arg(ctx, args, 0)?;
    // §8.9.1 confirm steps step 1: cannot show simple dialogs → return false.
    if cannot_show_simple_dialogs(ctx) {
        return Ok(JsValue::Boolean(false));
    }
    // Steps 2-6 — unreachable (permanent step-4 opt-in, see the gate).
    Ok(JsValue::Boolean(false))
}

/// `window.prompt(message, default)` (WHATWG HTML §8.9.1, `#dom-prompt`).
pub(super) fn native_window_prompt(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    // WebIDL conversion first (`optional DOMString message = "", optional
    // DOMString default = ""` — BOTH convert before the method steps).
    coerce_dialog_arg(ctx, args, 0)?;
    coerce_dialog_arg(ctx, args, 1)?;
    // §8.9.1 prompt steps step 1: cannot show simple dialogs → return null.
    if cannot_show_simple_dialogs(ctx) {
        return Ok(JsValue::Null);
    }
    // Steps 2-7 — unreachable (permanent step-4 opt-in, see the gate).
    Ok(JsValue::Null)
}

/// `window.open(url, target, features)` (WHATWG HTML §7.2.2.1 window open
/// steps, `#window-open-steps`).  Marshal-only: WebIDL-convert the three
/// optional arguments, resolve `url` at the JsValue boundary (parse failure
/// → `"SyntaxError"` DOMException, step 4.2 — boundary marshalling, thrown
/// BEFORE dispatch), route the target through the engine-independent
/// [`elidex_script_session::window_open_disposition`], and enqueue on the
/// matching `NavigationState` back-channel.
///
/// Returns `null` on every non-throw path: the WindowProxy return (steps
/// 17-18) needs the auxiliary browsing-context model (S5-8), and a
/// sandbox-blocked request is a silent `null` (the spec's "may report to a
/// developer console").
pub(super) fn native_window_open(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    // WebIDL: `open(optional USVString url = "", optional DOMString target
    // = "_blank", optional [LegacyNullToEmptyString] DOMString features =
    // "")` — all three convert before the method steps (`ToString` side
    // effects observable); absent / `undefined` takes the default.
    let url_input = coerce_string_arg(ctx, args, 0, "")?;
    let target = coerce_string_arg(ctx, args, 1, "_blank")?;
    // `features` is converted (side effects) then ignored — boa parity;
    // tokenization (noopener / noreferrer / popup sizing) rides the S5-8
    // WindowProxy/auxiliary-context program (memo §5.3.2, fold
    // `#11-browsing-context-model-window-open-postmessage`).
    // `[LegacyNullToEmptyString]`: `null` → `""` without `ToString`.
    match args.get(2).copied().unwrap_or(JsValue::Undefined) {
        JsValue::Undefined | JsValue::Null => {}
        v => {
            coerce::to_string(ctx.vm, v)?;
        }
    }

    // Steps 3-4: the urlRecord starts null and is set ONLY when `url` is not
    // the empty string (the *JS* empty string, before URL parsing — a
    // whitespace-only url is NOT empty here and is encoding-parsed, per which
    // the WHATWG URL parser strips leading/trailing spaces and resolves the
    // resulting empty relative reference to the document URL; this diverges
    // from boa's non-spec `trim().is_empty()` guard, deliberately). A parse
    // failure throws ("If urlRecord is failure, then throw a \"SyntaxError\"
    // DOMException", step 4.2 — boundary marshalling at the native).
    let url_record = if url_input.is_empty() {
        None
    } else {
        Some(super::location::resolve_url_or_syntax_error(
            ctx,
            &url_input,
            "execute 'open' on 'Window'",
        )?)
    };

    // The target dispatch + sandbox gates are the engine-independent
    // disposition function (HTML §7.3.1.7 / §7.4.2.4).  Script-initiated
    // `window.open` has NO transient activation — the conservative constant
    // `false` (elidex has no user-activation tracking yet; carve
    // `#11-transient-activation-tracking`, memo §4.3.3).  The flags read
    // happens inside the batch-bind bracket by construction (the native
    // runs under `eval`).
    let flags = ctx.host_opt().and_then(|hd| hd.sandbox_flags());
    match window_open_disposition(&target, flags, false) {
        // §7.3.1.7 step 8 sandboxed-auxiliary-navigation case / §7.4.2.4
        // top-navigation denial: enqueue nothing — a blocked request never
        // enters a queue (enqueue-time gating), and the return is a silent
        // `null`.
        WindowOpenDisposition::Blocked => {}
        // `_self` — and `_parent`/`_top` in the single-navigable model —
        // navigate the own (existing) browsing context (boa-parity routing;
        // the real parent/top navigable tree is S5-8).  §7.2.2.1 step 16.1:
        // an EXISTING navigable is navigated only when urlRecord is non-null;
        // an empty url is a NO-OP (the current document is preserved), so a
        // `None` url_record enqueues nothing.  Same channel + shape as
        // `location.assign` (push, not replace).
        WindowOpenDisposition::SelfNavigate | WindowOpenDisposition::TopNavigate => {
            if let Some(record) = url_record {
                ctx.vm.navigation.enqueue_navigation(NavigationRequest {
                    url: record.to_string(),
                    replace: false,
                });
            }
        }
        // A named target rides the shared ordered queue with the call-time
        // aux-nav snapshot; the name is passed as-given (only keyword
        // DETECTION is case-insensitive — shell-side name matching owns its
        // own rules).  The urlRecord null-vs-value distinction rides the
        // payload as `Option`, because the existing-vs-new navigable choice
        // (step 16.1 no-op vs step 15.3 about:blank) is the shell's to make.
        WindowOpenDisposition::Named { aux_nav_allowed } => {
            ctx.vm
                .navigation
                .enqueue_window_open(WindowOpenIntent::NamedFrame(NamedFrameNavigation {
                    name: target,
                    url: url_record.map(|record| record.to_string()),
                    aux_nav_allowed,
                }));
        }
        // A `_blank`/popup target is ALWAYS a new auxiliary navigable, so
        // §7.2.2.1 step 15.3 applies: a null urlRecord defaults to
        // about:blank (materialized here since the OpenTab channel carries a
        // concrete url).  Enqueued on the SAME ordered queue as named opens
        // so call order between a popup and a named MISS is preserved.
        WindowOpenDisposition::OpenTab => {
            let url = url_record.map_or_else(
                || super::navigation::parse_about_blank().to_string(),
                |record| record.to_string(),
            );
            ctx.vm
                .navigation
                .enqueue_window_open(WindowOpenIntent::Popup(OpenTabRequest { url }));
        }
    }
    Ok(JsValue::Null)
}
