//! Iframe loading: URL resolution, security checks, pipeline construction.

use elidex_navigation::NavigationController;
use elidex_plugin::{IframeSandboxFlags, SecurityOrigin};

use super::thread::iframe_thread_main;
use super::types::{
    BrowserToIframe, IframeEntry, IframeHandle, IframeLoadContext, IframeToBrowser,
    InProcessIframe, OutOfProcessIframe,
};

// ---------------------------------------------------------------------------
// Public loading entry point
// ---------------------------------------------------------------------------

/// Load an iframe document from a `src` URL or `srcdoc` content.
///
/// 1. Resolves the iframe's origin from its URL (or parent origin for srcdoc/about:blank)
/// 2. Checks CSP frame-ancestors and X-Frame-Options headers
/// 3. Creates a `PipelineResult` (DOM, JS runtime, styles, layout)
/// 4. Wraps it in an `InProcessIframe` (same-origin) or `OutOfProcessIframe` (cross-origin)
///
/// Always returns an `IframeEntry`. If framing is blocked by security headers,
/// returns a blank document with an opaque origin.
///
/// `ctx.depth` is the nesting depth of this iframe (`parent_depth + 1`).
/// It is stored on the iframe's bridge so nested iframes can compute their own depth.
pub fn load_iframe(
    iframe_data: &elidex_ecs::IframeData,
    ctx: &IframeLoadContext<'_>,
) -> IframeEntry {
    // Parse sandbox flags once, before the depth guard — every path (including
    // the guard's blank fallback) reuses them without re-parsing.
    let sandbox_flags = parse_sandbox(iframe_data);

    // Guard against excessive iframe nesting (DoS prevention).
    if ctx.depth >= elidex_plugin::MAX_IFRAME_DEPTH {
        eprintln!("iframe nesting exceeds MAX_IFRAME_DEPTH ({})", ctx.depth);
        return blank_entry(
            SecurityOrigin::opaque(),
            sandbox_flags,
            iframe_data.credentialless,
            ctx,
        );
    }

    // A real `src` URL (srcdoc absent — srcdoc takes precedence): network load,
    // with its origin derived from the loaded URL inside.
    if iframe_data.srcdoc.is_none() {
        if let Some(src) = iframe_data.src.as_deref() {
            if !src.is_empty() && src != "about:blank" {
                return load_iframe_from_url(src, iframe_data, sandbox_flags, ctx);
            }
        }
    }

    // srcdoc / about:blank / no-src documents inherit the parent origin
    // (WHATWG HTML §4.8.5), with sandbox / credentialless opaqueness applied.
    // Computed BEFORE the pipeline build (via `pre_eval_state`): the build runs
    // the initial scripts, which must already observe this origin — and the
    // referrer, both installed at the pre-eval chokepoint (see `PreEvalFrameState`).
    let content = iframe_data.srcdoc.as_deref().unwrap_or("");
    let pipeline = build_iframe_pipeline(
        content,
        ctx.parent_url.cloned(),
        ctx,
        pre_eval_state(
            parent_inherited_origin(iframe_data, sandbox_flags, ctx),
            sandbox_flags,
            iframe_data.credentialless,
            ctx,
        ),
    );

    make_in_process_entry(pipeline)
}

/// Load an iframe from a URL, handling security checks and origin-based dispatch.
fn load_iframe_from_url(
    src: &str,
    iframe_data: &elidex_ecs::IframeData,
    sandbox_flags: Option<IframeSandboxFlags>,
    ctx: &IframeLoadContext<'_>,
) -> IframeEntry {
    let base = ctx
        .parent_url
        .cloned()
        .unwrap_or_else(|| url::Url::parse("about:blank").expect("about:blank is a valid URL"));
    let Ok(resolved) = base.join(src) else {
        eprintln!("iframe: invalid src URL: {src}");
        return blank_entry(
            parent_inherited_origin(iframe_data, sandbox_flags, ctx),
            sandbox_flags,
            iframe_data.credentialless,
            ctx,
        );
    };

    // Credentialless iframes use an isolated broker (no shared cookies).
    // Non-credentialless iframes share the parent's NetworkHandle.
    let credentialless_broker = if iframe_data.credentialless {
        Some(elidex_net::broker::spawn_network_process(
            elidex_net::NetClient::new_credentialless(),
        ))
    } else {
        None
    };
    let credentialless_handle = credentialless_broker
        .as_ref()
        .map(elidex_net::broker::NetworkProcessHandle::create_renderer_handle);
    let effective_handle: &elidex_net::broker::NetworkHandle =
        credentialless_handle.as_ref().unwrap_or(ctx.network_handle);
    match elidex_navigation::load_document(&resolved, effective_handle, None) {
        Ok(loaded) => {
            let doc_origin = SecurityOrigin::from_url(&loaded.url);
            if !check_framing_allowed(&loaded.response_headers, ctx.parent_origin, &doc_origin) {
                eprintln!(
                    "iframe blocked by frame-ancestors/X-Frame-Options: {}",
                    loaded.url
                );
                return blank_entry(
                    SecurityOrigin::opaque(),
                    sandbox_flags,
                    iframe_data.credentialless,
                    ctx,
                );
            }

            let origin = apply_sandbox_origin(
                doc_origin.clone(),
                sandbox_flags,
                iframe_data.credentialless,
            );

            if ctx.parent_origin != &origin {
                // OOP routing ⟺ the *document* origin differs from the parent's
                // (cross-origin OR sandboxed/credentialless-opaque), so the frame
                // needs its own process. But the referrer's cross-origin-ness is
                // the REQUEST relationship, not this routing decision: it keys on
                // `parent_origin` vs `doc_origin` (the ACTUAL loaded-URL origin,
                // BEFORE sandbox opaque-ification), so a same-origin request that
                // is merely sandboxed-to-opaque still shares the full parent URL
                // (W3C Referrer Policy §3 default `strict-origin-when-cross-origin`,
                // §8.3). The whole default policy — no-referrer-source gates
                // (opaque origin / local scheme), TLS downgrade, cross-vs-same trim
                // — is the ONE `default_referrer` pipeline applied to every path;
                // only full per-request ReferrerPolicy (meta referrer,
                // rel=noreferrer, Referrer-Policy header, referrerpolicy attr) is
                // deferred → slot #11-referrer-policy (a new carve; ledger
                // registration is a landing deliverable).
                let mut state =
                    pre_eval_state(origin, sandbox_flags, iframe_data.credentialless, ctx);
                state.referrer = default_referrer(ctx.parent_url, ctx.parent_origin, &doc_origin);
                return make_out_of_process_entry(loaded, state, ctx.device_facts);
            }

            // Use credentialless handle if applicable, otherwise parent's.
            let pipeline_handle: std::rc::Rc<elidex_net::broker::NetworkHandle> =
                if iframe_data.credentialless {
                    std::rc::Rc::new(
                        credentialless_broker
                            .as_ref()
                            .unwrap()
                            .create_renderer_handle(),
                    )
                } else {
                    ctx.network_handle.clone()
                };
            // Same-origin iframes inherit the parent's cookie jar.
            // Credentialless iframes get None (isolated cookies).
            let iframe_cookies = if iframe_data.credentialless {
                None
            } else {
                ctx.cookie_jar.clone()
            };
            // iframe initial build: the sub-browsing-context box is not yet known
            // (the parent lays out the <iframe> element + delivers it via
            // SetViewport later), so build at DEFAULT *size*. NOTE C1's
            // `run_scripts_and_finalize` now also seeds the JS bridge from this
            // viewport before initial scripts run (the same cascade/bridge
            // unification F1 applies to top-level), so iframe initial-script
            // `innerWidth` observes DEFAULT — where pre-C1 the bridge
            // stayed at its 800x600 default while the cascade used DEFAULT. The
            // size is a placeholder for the real iframe box; the correct box-at-build
            // is deferred → slot #11-iframe-build-viewport.
            let mut pipeline = crate::build_pipeline_from_loaded(
                loaded,
                pipeline_handle,
                ctx.font_db.clone(),
                iframe_cookies,
                elidex_plugin::Size::new(
                    crate::DEFAULT_VIEWPORT_WIDTH,
                    crate::DEFAULT_VIEWPORT_HEIGHT,
                ),
                // Device facts, unlike the size, ARE known at build: dppx/color-scheme
                // are window/display facts the sub-frame shares with its parent (C3), so
                // seed the parent's real facts (`IframeLoadContext::device_facts`) — the
                // iframe's `devicePixelRatio`/`matchMedia` are correct from birth instead
                // of stuck at 1×/Light on a HiDPI/dark display.
                ctx.device_facts,
                // Security installs precede the initial scripts run inside this
                // build (see `PreEvalFrameState`).
                Some(pre_eval_state(
                    origin,
                    sandbox_flags,
                    iframe_data.credentialless,
                    ctx,
                )),
            );
            // Keep credentialless broker alive for the iframe pipeline's lifetime.
            if let Some(cb) = credentialless_broker {
                pipeline.broker_keepalive = Some(cb);
            }
            make_in_process_entry(pipeline)
        }
        Err(e) => {
            eprintln!("iframe load error: {e}");
            blank_entry(
                parent_inherited_origin(iframe_data, sandbox_flags, ctx),
                sandbox_flags,
                iframe_data.credentialless,
                ctx,
            )
        }
    }
}

// ---------------------------------------------------------------------------
// Entry constructors
// ---------------------------------------------------------------------------

/// Create a same-origin `IframeEntry` from a pipeline.
///
/// The security state (sandbox flags / origin / depth) is NOT installed here:
/// it rides the pipeline build as [`crate::PreEvalFrameState`] so it is on the
/// bridge **before** the initial scripts run (S5-4b ordering invariant) — by
/// the time this constructor wraps the pipeline, the installs already happened.
pub(super) fn make_in_process_entry(pipeline: crate::PipelineResult) -> IframeEntry {
    IframeEntry {
        handle: IframeHandle::InProcess(Box::new(InProcessIframe {
            pipeline,
            nav_controller: NavigationController::new(),
            scroll_state: elidex_ecs::ScrollState::default(),
            needs_render: false,
            cached_display_list: None,
        })),
    }
}

/// Create a cross-origin `IframeEntry` that runs in a separate thread.
///
/// Receives the already-fetched `LoadedDocument` from the parent thread,
/// avoiding a redundant HTTP request. The `PipelineResult` is constructed
/// on the iframe thread because it contains `!Send` types (`Rc`, boa `Context`).
///
/// `state` (already sandbox/credentialless-adjusted by the caller) rides the
/// pipeline build as [`crate::PreEvalFrameState`] — the same pre-eval chokepoint as
/// the in-process paths, so the initial scripts run inside the build already
/// observe the origin / sandbox flags / depth (S5-4b ordering invariant). No
/// post-build install sequence exists on this path anymore.
///
/// `pub(in crate::content)` for the OOP-path ordering tests
/// (`content_iframe_security_tests`), which drive this entry directly with a
/// synthesized `LoadedDocument` — the production route requires a real
/// cross-origin network load.
pub(in crate::content) fn make_out_of_process_entry(
    loaded: elidex_navigation::LoadedDocument,
    state: crate::PreEvalFrameState,
    device_facts: crate::ipc::DeviceFacts,
) -> IframeEntry {
    let (parent_chan, iframe_chan) = crate::ipc::channel_pair::<BrowserToIframe, IframeToBrowser>();

    let thread = std::thread::spawn(move || {
        // Build pipeline on this thread (PipelineResult is !Send).
        // Use the already-fetched LoadedDocument — no redundant HTTP request.
        let network_handle = std::rc::Rc::new(elidex_net::broker::NetworkHandle::disconnected());
        let font_db = std::sync::Arc::new(elidex_text::FontDatabase::new());
        // OOP iframe initial build at DEFAULT *size* — box delivered later via
        // SetViewport (slot #11-iframe-build-viewport, same as the in-process path). As
        // there, C1 now seeds the bridge from this DEFAULT size before initial scripts
        // (was the boa 800x600 bridge default pre-C1); a placeholder pending the real box.
        let oop_pipeline = crate::build_pipeline_from_loaded(
            loaded,
            network_handle,
            font_db,
            None,
            elidex_plugin::Size::new(
                crate::DEFAULT_VIEWPORT_WIDTH,
                crate::DEFAULT_VIEWPORT_HEIGHT,
            ),
            // Device facts inherited from the parent (C3) — `DeviceFacts` is `Copy + Send`,
            // captured into this thread. dppx/color-scheme are window/display facts shared
            // across origins on the same display, so a cross-origin OOP frame inherits them
            // too (they carry no origin-private information — already exposed via matchMedia).
            device_facts,
            // Security installs precede the initial scripts run inside this
            // build (see `PreEvalFrameState`) — the same chokepoint as the
            // in-process paths. `PreEvalFrameState` is `Send`, captured into this
            // thread.
            Some(state),
        );

        iframe_thread_main(oop_pipeline, &iframe_chan);
    });

    IframeEntry {
        handle: IframeHandle::OutOfProcess(OutOfProcessIframe {
            channel: parent_chan,
            display_list: elidex_render::DisplayList::default(),
            thread: Some(thread),
        }),
    }
}

/// Create a blank `IframeEntry` (empty document) for error/fallback cases.
///
/// Used when iframe loading fails, is blocked by security headers,
/// or exceeds the nesting depth limit. Always same-origin (`InProcess`).
///
/// The caller passes the already-parsed `sandbox_flags` and the `credentialless`
/// flag — every call site holds them, so this fallback never re-parses the
/// `sandbox` attribute.
fn blank_entry(
    origin: SecurityOrigin,
    sandbox_flags: Option<IframeSandboxFlags>,
    credentialless: bool,
    ctx: &IframeLoadContext<'_>,
) -> IframeEntry {
    make_in_process_entry(build_iframe_pipeline(
        "",
        ctx.parent_url.cloned(),
        ctx,
        pre_eval_state(origin, sandbox_flags, credentialless, ctx),
    ))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Parse sandbox flags from `IframeData`. Returns `None` if no `sandbox` attribute.
fn parse_sandbox(iframe_data: &elidex_ecs::IframeData) -> Option<IframeSandboxFlags> {
    iframe_data
        .sandbox
        .as_deref()
        .map(elidex_plugin::parse_sandbox_attribute)
}

/// Bundle the security state an iframe build (in-process or OOP-thread)
/// installs **before** its initial scripts run (see [`crate::PreEvalFrameState`]).
///
/// The referrer is computed by the single [`default_referrer`] pipeline; it
/// rides the same pre-eval chokepoint as origin/flags/depth so the initial
/// scripts read a populated `document.referrer`. This is the same-origin default
/// (srcdoc / about:blank / same-origin URL in-process build), so the request
/// origin passed IS the parent origin; the cross-origin call site (OOP branch)
/// passes the real request origin instead.
fn pre_eval_state(
    origin: SecurityOrigin,
    sandbox_flags: Option<IframeSandboxFlags>,
    credentialless: bool,
    ctx: &IframeLoadContext<'_>,
) -> crate::PreEvalFrameState {
    crate::PreEvalFrameState {
        origin,
        sandbox_flags,
        iframe_depth: ctx.depth,
        credentialless,
        // Same-origin default: srcdoc / about:blank / same-origin URL loads
        // inherit the parent origin, so the request origin IS the parent origin.
        referrer: default_referrer(ctx.parent_url, ctx.parent_origin, ctx.parent_origin),
    }
}

/// The Fetch "local scheme" set — `about` / `blob` / `data`
/// (<https://fetch.spec.whatwg.org/#local-scheme>), reused by W3C Referrer
/// Policy §8.4 step 2. A URL with a local scheme has no referrer to disclose.
fn is_local_scheme(scheme: &str) -> bool {
    matches!(scheme, "about" | "blob" | "data")
}

/// Strip a URL "for use as a referrer" (W3C Referrer Policy §8.4 steps 3–5):
/// remove the username, password, and fragment so parent credentials
/// (`user:pass@`) and fragment secrets (`#…`) never leak into a sub-frame's
/// `document.referrer`. The step-2 "no referrer" gates (local scheme / opaque
/// origin) are applied by the sole caller [`default_referrer`], so this is the
/// pure serialization step. Mirrors the VM's `Vm::set_navigation_referrer`
/// sanitisation (`elidex-js` `vm/vm_api.rs`) — the `Referer` header and
/// `document.referrer` share the same exposure surface.
fn strip_referrer_url(url: &url::Url) -> String {
    let mut url = url.clone();
    url.set_fragment(None);
    let _ = url.set_username("");
    let _ = url.set_password(None);
    url.to_string()
}

/// The `document.referrer` a child document receives, per the W3C Referrer
/// Policy §3 DEFAULT policy ("strict-origin-when-cross-origin", §3.7). This is
/// the ONE canonical pipeline every iframe path routes through (in-process
/// same-origin, OOP cross-origin, and — via the persisted bridge referrer — the
/// navigate rebuild), applied uniformly IN ORDER (§8.3 "Determine request's
/// Referrer" → §8.4 "Strip url for use as a referrer"):
///
/// - `source_url` / `source_origin`: the parent (referrer source) document URL
///   and origin.
/// - `request_origin`: the child request's origin (`from_url(loaded.url)`, the
///   ACTUAL loaded-URL origin BEFORE any sandbox opaque-ification) — NOT the
///   post-sandbox document origin and NOT the OOP-routing decision. A
///   same-origin request that is merely sandboxed-to-opaque still shares the
///   full parent URL.
///
/// Steps (each yielding `None` = no `Referer` / empty `document.referrer`):
///
/// 1. **No valid referrer source** → `None`: `source_origin` is opaque (§8.3
///    step 2.2), OR `source_url` has a Fetch **local scheme** (`about` / `blob`
///    / `data`, §8.4 step 2). The local-scheme test is on the **source URL
///    scheme, not the origin** — an `about:blank` parent has a local scheme but
///    an inherited (non-opaque, tuple) origin, so it is caught here on BOTH the
///    same-origin AND cross-origin paths (where an origin-opaque-only gate would
///    leak it cross-origin).
/// 2. `referrerURL` = strip `source_url` (username / password / fragment
///    removed, [`strip_referrer_url`]).
/// 3. **Downgrade** → `None`: `source_origin` is potentially-trustworthy AND
///    `request_origin` is NOT (§3.7). "Potentially trustworthy" — not merely
///    `https` — via [`SecurityOrigin::is_potentially_trustworthy`], so an https
///    parent embedding a loopback-`http` child (`http://localhost`,
///    `http://127.0.0.1`, `http://[::1]`) is NOT a downgrade. Same-origin is
///    necessarily same-scheme, so a downgrade only ever arises cross-origin.
/// 4. **Cross-origin** (`source_origin != request_origin`) → the source
///    ORIGIN-as-URL: its serialization followed by U+002F SOLIDUS (`/`) — §8.4
///    with the origin-only flag leaves an empty path that serializes as `/`.
/// 5. **Same-origin** → the stripped `referrerURL` (full URL, userinfo /
///    fragment removed).
///
/// Only full per-request ReferrerPolicy (meta referrer, `rel=noreferrer`,
/// `Referrer-Policy` header, `referrerpolicy` attr) remains deferred → slot
/// `#11-referrer-policy` (a new carve; ledger registration is a landing
/// deliverable).
fn default_referrer(
    source_url: Option<&url::Url>,
    source_origin: &SecurityOrigin,
    request_origin: &SecurityOrigin,
) -> Option<String> {
    // Step 1: no valid referrer source (opaque origin OR local-scheme URL).
    if matches!(source_origin, SecurityOrigin::Opaque(_)) {
        return None;
    }
    let source_url = source_url?;
    if is_local_scheme(source_url.scheme()) {
        return None;
    }
    // Step 3: TLS downgrade (potentially-trustworthy source → non-p.t. request).
    if source_origin.is_potentially_trustworthy() && !request_origin.is_potentially_trustworthy() {
        return None;
    }
    if source_origin == request_origin {
        // Step 5 (via step 2 strip): same-origin → stripped full source URL.
        Some(strip_referrer_url(source_url))
    } else {
        // Step 4: cross-origin → source ORIGIN-as-URL (trailing `/`).
        Some(format!("{}/", source_origin.serialize()))
    }
}

/// Parent-inherited origin with sandbox / credentialless opaqueness applied.
///
/// The srcdoc / about:blank / no-src documents and the invalid-src / load-error
/// fallbacks all inherit the parent origin the same way (WHATWG HTML §4.8.5),
/// so this collapses the repeated `apply_sandbox_origin(parent_origin, …)`.
fn parent_inherited_origin(
    iframe_data: &elidex_ecs::IframeData,
    sandbox_flags: Option<IframeSandboxFlags>,
    ctx: &IframeLoadContext<'_>,
) -> SecurityOrigin {
    apply_sandbox_origin(
        ctx.parent_origin.clone(),
        sandbox_flags,
        iframe_data.credentialless,
    )
}

/// Build an iframe pipeline from HTML content, sharing the parent's resources.
fn build_iframe_pipeline(
    html: &str,
    url: Option<url::Url>,
    ctx: &IframeLoadContext<'_>,
    state: crate::PreEvalFrameState,
) -> crate::PipelineResult {
    crate::build_pipeline_interactive_shared(
        html,
        url,
        ctx.font_db.clone(),
        ctx.network_handle.clone(),
        ctx.registry.clone(),
        ctx.cookie_jar.clone(),
        // iframe build at DEFAULT *size* — box not yet known (slot #11-iframe-build-viewport).
        elidex_plugin::Size::new(
            crate::DEFAULT_VIEWPORT_WIDTH,
            crate::DEFAULT_VIEWPORT_HEIGHT,
        ),
        // Device facts ARE known: window/display facts inherited from the parent (C3),
        // not box facts — so seed the real dppx/color-scheme, not a 1×/Light placeholder.
        ctx.device_facts,
        Some(state),
    )
}

// ---------------------------------------------------------------------------
// Security helpers
// ---------------------------------------------------------------------------

/// Check framing permission from response headers.
///
/// CSP `frame-ancestors` takes priority over `X-Frame-Options` (W3C CSP L3).
/// For CSP, any header that blocks framing wins (most restrictive).
/// For XFO, the most restrictive value across all header values is used.
pub(super) fn check_framing_allowed(
    headers: &std::collections::HashMap<String, Vec<String>>,
    parent_origin: &SecurityOrigin,
    doc_origin: &SecurityOrigin,
) -> bool {
    // CSP frame-ancestors check (takes priority).
    if let Some(csp_values) = headers.get("content-security-policy") {
        let mut has_frame_ancestors = false;
        for csp in csp_values {
            if let Some(policy) = elidex_plugin::parse_frame_ancestors(csp) {
                has_frame_ancestors = true;
                if !elidex_plugin::is_framing_allowed(&policy, parent_origin, doc_origin) {
                    return false;
                }
            }
        }
        if has_frame_ancestors {
            return true;
        }
    }
    // X-Frame-Options fallback (only if no CSP frame-ancestors).
    if let Some(xfo_values) = headers.get("x-frame-options") {
        for xfo in xfo_values {
            if !elidex_plugin::check_x_frame_options(xfo, parent_origin, doc_origin) {
                return false;
            }
        }
    }
    true
}

/// Apply sandbox and credentialless origin overrides.
///
/// If sandbox is present without `allow-same-origin`, or if the iframe is
/// `credentialless`, returns an opaque origin. Otherwise returns the input origin.
///
/// Private to `content/iframe`: the post-redirect origin derivation
/// ([`crate::PreEvalFrameInputs::into_pre_eval_state`], co-located below)
/// reuses this single policy for the URL-loading rebuild, so the policy never
/// leaves this module.
fn apply_sandbox_origin(
    origin: SecurityOrigin,
    sandbox_flags: Option<IframeSandboxFlags>,
    credentialless: bool,
) -> SecurityOrigin {
    if let Some(flags) = sandbox_flags {
        if !flags.contains(IframeSandboxFlags::ALLOW_SAME_ORIGIN) {
            return SecurityOrigin::opaque();
        }
    }
    if credentialless {
        return SecurityOrigin::opaque();
    }
    origin
}

// The post-redirect origin derivation is co-located with the
// [`apply_sandbox_origin`] policy it applies (rather than in `pipeline.rs` with
// the [`crate::PreEvalFrameState`] bundle) so the policy stays private to this
// module — the URL-loading builder (`build_pipeline_from_url`) invokes the
// resolver by method dispatch instead of reaching up into the origin policy.
impl crate::PreEvalFrameInputs {
    /// Derive the [`crate::PreEvalFrameState`] from the **post-redirect** loaded URL.
    ///
    /// The origin is `apply_sandbox_origin(from_url(loaded_url), flags,
    /// credentialless)` — the exact derivation the initial OOP load performs on
    /// `loaded.url` (above), so `Navigate` attributes the navigated document to
    /// the URL it actually resolved to (post-redirect), and a credentialless
    /// frame keeps its opaque origin.
    #[must_use]
    pub fn into_pre_eval_state(self, loaded_url: &url::Url) -> crate::PreEvalFrameState {
        crate::PreEvalFrameState {
            origin: apply_sandbox_origin(
                SecurityOrigin::from_url(loaded_url),
                self.sandbox_flags,
                self.credentialless,
            ),
            sandbox_flags: self.sandbox_flags,
            iframe_depth: self.iframe_depth,
            credentialless: self.credentialless,
            referrer: self.referrer,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{default_referrer, strip_referrer_url, SecurityOrigin};

    fn origin(s: &str) -> SecurityOrigin {
        SecurityOrigin::from_url(&url::Url::parse(s).unwrap())
    }

    /// Step 5: a **same-origin** request that is merely sandboxed-to-opaque (OOP
    /// routed) still shares the FULL (stripped) source URL — the referrer keys
    /// on the request relationship (`source_origin` == `request_origin`), not the
    /// post-sandbox opaque document origin. Falsify by making the same-origin arm
    /// emit the ORIGIN-as-URL instead of the stripped full URL.
    #[test]
    fn default_referrer_same_origin_request_keeps_full_url() {
        let parent = url::Url::parse("https://parent.example/a/b?q").unwrap();
        let request = origin("https://parent.example/child");
        assert_eq!(
            default_referrer(Some(&parent), &origin("https://parent.example/"), &request)
                .as_deref(),
            Some("https://parent.example/a/b?q"),
            "same-origin request must expose the full parent URL, even when sandboxed"
        );
    }

    /// Step 4: a genuinely cross-origin request → source ORIGIN-as-URL (trailing
    /// slash, R3-F1).
    #[test]
    fn default_referrer_cross_origin_request_is_origin_only() {
        let parent = url::Url::parse("https://parent.example/a/b?q").unwrap();
        let request = origin("https://other.example/");
        assert_eq!(
            default_referrer(Some(&parent), &origin("https://parent.example/"), &request)
                .as_deref(),
            Some("https://parent.example/"),
            "cross-origin request must be trimmed to the source ORIGIN-as-URL"
        );
    }

    /// Step 2: userinfo + fragment are stripped before the source URL becomes a
    /// same-origin referrer (WHATWG Fetch "strip url for use as a referrer"), so
    /// credentials/secrets never leak via `document.referrer`. Falsify by
    /// reverting `strip_referrer_url` to a bare `to_string()`.
    #[test]
    fn strip_referrer_url_removes_userinfo_and_fragment() {
        let url = url::Url::parse("https://user:pass@parent.example/path#frag").unwrap();
        assert_eq!(strip_referrer_url(&url), "https://parent.example/path");
    }

    /// Step 1 (local scheme, same-origin path): a local-scheme URL (`about` /
    /// `blob` / `data`, Fetch "local scheme") has no referrer (W3C Referrer
    /// Policy §8.4 step 2), so its `data:`/`about:`/`blob:` URL never leaks into
    /// `document.referrer`. Each source origin here is the URL's own (opaque for
    /// data:, tuple for the blob:/about: inherited cases we use elsewhere), and
    /// the request is same-origin — the local-scheme gate must fire regardless.
    /// Falsify by removing the `is_local_scheme` gate in `default_referrer`.
    #[test]
    fn default_referrer_local_scheme_source_is_none() {
        for u in [
            "data:text/html,<p>hi",
            "about:blank",
            "blob:https://x.example/uuid",
        ] {
            let url = url::Url::parse(u).unwrap();
            // Use a tuple source origin so the opaque gate can't mask the
            // local-scheme gate for about:/blob:.
            let src = origin("https://parent.example/");
            assert_eq!(
                default_referrer(Some(&url), &src, &src),
                None,
                "local-scheme URL {u} must have no referrer"
            );
        }
    }

    /// Step 1 (opaque origin): `default_referrer` yields `None` for an
    /// opaque-origin source (data:/file:/sandboxed — W3C Referrer Policy §8.3
    /// step 2.2), so a child of such a parent never receives a leaked URL.
    /// Falsify by dropping the opaque-origin gate.
    #[test]
    fn default_referrer_opaque_source_is_none() {
        let data_parent = url::Url::parse("data:text/html,<p>hi").unwrap();
        let opaque = SecurityOrigin::opaque();
        assert_eq!(
            default_referrer(Some(&data_parent), &opaque, &opaque),
            None,
            "an opaque-origin source has no referrer to disclose"
        );
    }

    /// R3-F3 (same-origin path): an `about:blank` parent whose *inherited* origin
    /// is a non-opaque tuple still yields no referrer (its URL is a local scheme)
    /// — the local-scheme gate catches it where the opaque-origin gate does not.
    /// Falsify by reverting step 1 to an origin-opaque-only check.
    #[test]
    fn default_referrer_about_blank_tuple_origin_same_origin_is_none() {
        let about = url::Url::parse("about:blank").unwrap();
        let tuple = origin("https://parent.example/");
        assert_eq!(
            default_referrer(Some(&about), &tuple, &tuple),
            None,
            "an about:blank parent (local-scheme URL) discloses no referrer even \
             with an inherited tuple origin"
        );
    }

    /// **R4** (the leak this PR closes): an `about:blank` parent with an
    /// inherited TUPLE origin embedding a **CROSS-ORIGIN** child. The origin is a
    /// non-opaque tuple, so an origin-opaque-only gate slips past and the
    /// cross-origin arm would emit `https://parent.example/`. The step-1
    /// local-scheme test — on the SOURCE URL SCHEME, not the origin — must catch
    /// it on the cross-origin path too. Falsify by reverting step 1 to an
    /// origin-opaque-only check (the pre-R4 bug): this would return
    /// `Some("https://parent.example/")`.
    #[test]
    fn default_referrer_about_blank_tuple_origin_cross_origin_is_none() {
        let about = url::Url::parse("about:blank").unwrap();
        let tuple = origin("https://parent.example/");
        let cross = origin("https://other.example/");
        assert_eq!(
            default_referrer(Some(&about), &tuple, &cross),
            None,
            "an about:blank (local-scheme) parent leaks NO referrer cross-origin, \
             even though its inherited origin is a non-opaque tuple"
        );
    }

    /// R3-F2: an https parent embedding a cross-origin **loopback**-`http` child
    /// (`http://localhost`) is NOT a TLS downgrade — loopback http is
    /// potentially trustworthy (Secure Contexts) — so the child still receives
    /// the source ORIGIN-as-URL referrer, not `None`. Falsify by reverting the
    /// downgrade check to a bare `https` scheme test.
    #[test]
    fn default_referrer_https_to_loopback_http_is_not_downgrade() {
        let parent = url::Url::parse("https://parent.example/a/b?q").unwrap();
        let request = origin("http://localhost/");
        assert_eq!(
            default_referrer(Some(&parent), &origin("https://parent.example/"), &request)
                .as_deref(),
            Some("https://parent.example/"),
            "loopback-http is potentially trustworthy, so this is not a downgrade"
        );
    }

    /// Step 2 through the same-origin path (end-to-end for the OOP
    /// same-origin-sandboxed case): the stripped full URL, not the raw one.
    #[test]
    fn default_referrer_same_origin_strips_userinfo_and_fragment() {
        let parent = url::Url::parse("https://user:pass@parent.example/path#frag").unwrap();
        let request = origin("https://parent.example/child");
        assert_eq!(
            default_referrer(Some(&parent), &origin("https://parent.example/"), &request)
                .as_deref(),
            Some("https://parent.example/path"),
        );
    }

    /// Step 4: an opaque source origin has no usable referrer to share (`"null"`
    /// is not a referrer), so a cross-origin child gets an empty
    /// `document.referrer` (caught by step 1's opaque gate before the trim).
    #[test]
    fn default_referrer_opaque_source_cross_origin_is_none() {
        let parent = url::Url::parse("data:text/html,<p>hi").unwrap();
        assert_eq!(
            default_referrer(
                Some(&parent),
                &SecurityOrigin::opaque(),
                &origin("https://other.example/")
            ),
            None
        );
    }

    /// R2: a TLS downgrade — an https parent embedding a cross-origin http
    /// child — sends NO referrer (a potentially-trustworthy referrerURL with a
    /// non-potentially-trustworthy current URL, W3C Referrer Policy §3.7), not
    /// the source origin. Falsify by removing the step-3 downgrade branch (it
    /// would return `Some("https://parent.example/")`).
    #[test]
    fn default_referrer_https_to_http_downgrade_is_none() {
        let parent = url::Url::parse("https://parent.example/a/b?q").unwrap();
        let request = origin("http://other.example/");
        assert_eq!(
            default_referrer(Some(&parent), &origin("https://parent.example/"), &request),
            None,
            "secure→non-secure cross-origin downgrade must omit the referrer entirely"
        );
    }

    /// R2 control: an http parent → http cross-origin child is NOT a downgrade
    /// (source was never potentially-trustworthy), so the source origin is
    /// still sent (§3.7 "referrerURL is a non-potentially trustworthy URL").
    #[test]
    fn default_referrer_http_to_http_cross_origin_is_origin() {
        let parent = url::Url::parse("http://parent.example/a/b?q").unwrap();
        let request = origin("http://other.example/");
        assert_eq!(
            default_referrer(Some(&parent), &origin("http://parent.example/"), &request).as_deref(),
            Some("http://parent.example/"),
            "non-secure source cross-origin is not a downgrade; source origin is sent"
        );
    }
}
