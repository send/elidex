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
    // Computed BEFORE the pipeline build (via `frame_security`): the build runs
    // the initial scripts, which must already observe this origin — and the
    // referrer, both installed at the pre-eval chokepoint (see `FrameSecurity`).
    let content = iframe_data.srcdoc.as_deref().unwrap_or("");
    let pipeline = build_iframe_pipeline(
        content,
        ctx.parent_url.cloned(),
        ctx,
        frame_security(
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

            let origin =
                apply_sandbox_origin(doc_origin, sandbox_flags, iframe_data.credentialless);

            if ctx.parent_origin != &origin {
                // OOP ⟺ cross-origin: this branch is taken exactly when the frame
                // origin differs from the parent's (or credentialless/opaque), so
                // the loaded document is cross-origin to its embedder. Per the HTML
                // referrer-policy default (`strict-origin-when-cross-origin`), a
                // cross-origin document receives only the parent's ORIGIN as
                // `document.referrer`, not the full parent URL (which the
                // same-origin in-process path below keeps). The TLS-downgrade "no
                // referrer" case and full per-request ReferrerPolicy (meta referrer,
                // rel=noreferrer, Referrer-Policy header) are deferred → slot
                // #11-referrer-policy.
                let mut security =
                    frame_security(origin, sandbox_flags, iframe_data.credentialless, ctx);
                security.referrer = cross_origin_referrer(ctx.parent_origin);
                return make_out_of_process_entry(loaded, security, ctx.device_facts);
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
                // build (see `FrameSecurity`).
                Some(frame_security(
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
/// it rides the pipeline build as [`crate::FrameSecurity`] so it is on the
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
/// `security` (already sandbox/credentialless-adjusted by the caller) rides the
/// pipeline build as [`crate::FrameSecurity`] — the same pre-eval chokepoint as
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
    security: crate::FrameSecurity,
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
            // build (see `FrameSecurity`) — the same chokepoint as the
            // in-process paths. `FrameSecurity` is `Send`, captured into this
            // thread.
            Some(security),
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
        frame_security(origin, sandbox_flags, credentialless, ctx),
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
/// installs **before** its initial scripts run (see [`crate::FrameSecurity`]).
///
/// The referrer is the parent document URL (WHATWG HTML §4.8.5); it rides the
/// same pre-eval chokepoint as origin/flags/depth so the initial scripts read a
/// populated `document.referrer`.
fn frame_security(
    origin: SecurityOrigin,
    sandbox_flags: Option<IframeSandboxFlags>,
    credentialless: bool,
    ctx: &IframeLoadContext<'_>,
) -> crate::FrameSecurity {
    crate::FrameSecurity {
        origin,
        sandbox_flags,
        iframe_depth: ctx.depth,
        credentialless,
        referrer: ctx.parent_url.map(url::Url::to_string),
    }
}

/// The `document.referrer` a **cross-origin** sub-frame receives: only the
/// parent's ORIGIN serialization, per the HTML referrer-policy default
/// (`strict-origin-when-cross-origin`) — not the full parent URL the
/// same-origin path exposes (see [`frame_security`]). An opaque parent origin
/// has no usable referrer to share (serialized `"null"` is not a referrer), so
/// this yields `None` (empty `document.referrer`).
///
/// Full per-request ReferrerPolicy (meta referrer, rel=noreferrer,
/// Referrer-Policy header) and the TLS-downgrade "no referrer" case are deferred
/// → slot `#11-referrer-policy`.
fn cross_origin_referrer(parent_origin: &SecurityOrigin) -> Option<String> {
    match parent_origin {
        SecurityOrigin::Tuple { .. } => Some(parent_origin.serialize()),
        SecurityOrigin::Opaque(_) => None,
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
    security: crate::FrameSecurity,
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
        Some(security),
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
/// `pub(crate)` so the post-redirect origin derivation
/// ([`crate::FrameSecurityInputs::into_frame_security`]) reuses this single
/// policy for the URL-loading rebuild instead of duplicating it.
pub(crate) fn apply_sandbox_origin(
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

#[cfg(test)]
mod tests {
    use super::{cross_origin_referrer, SecurityOrigin};

    /// A cross-origin sub-frame's referrer is trimmed to the parent's ORIGIN
    /// (HTML referrer-policy default `strict-origin-when-cross-origin`), never
    /// the full parent URL. Falsify by reverting the OOP trim to the full URL.
    #[test]
    fn cross_origin_referrer_is_parent_origin_not_full_url() {
        let parent =
            SecurityOrigin::from_url(&url::Url::parse("https://parent.example/a/b?q").unwrap());
        assert_eq!(
            cross_origin_referrer(&parent).as_deref(),
            Some("https://parent.example"),
            "cross-origin referrer must be the parent ORIGIN, not the full URL"
        );
    }

    /// An opaque parent origin has no usable referrer to share (`"null"` is not
    /// a referrer), so a cross-origin child gets an empty `document.referrer`.
    #[test]
    fn cross_origin_referrer_opaque_parent_is_none() {
        assert_eq!(cross_origin_referrer(&SecurityOrigin::opaque()), None);
    }
}
