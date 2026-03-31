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
#[allow(clippy::cast_precision_loss)] // u32 width/height to f32 is acceptable for CSS pixels.
#[allow(clippy::too_many_lines)] // Multi-source iframe loading with security checks.
pub fn load_iframe(
    iframe_data: &elidex_ecs::IframeData,
    ctx: &IframeLoadContext<'_>,
) -> IframeEntry {
    // Guard against excessive iframe nesting (DoS prevention).
    if ctx.depth >= elidex_plugin::MAX_IFRAME_DEPTH {
        eprintln!("iframe nesting exceeds MAX_IFRAME_DEPTH ({})", ctx.depth);
        return blank_entry(SecurityOrigin::opaque(), iframe_data, ctx);
    }

    // Parse sandbox flags once — used by both origin override and meta construction.
    let sandbox_flags = parse_sandbox(iframe_data);

    // Determine content source and origin.
    let (pipeline, iframe_origin) = if let Some(srcdoc) = &iframe_data.srcdoc {
        // srcdoc: parse inline HTML, inherit parent origin (WHATWG HTML §4.8.5).
        let pipeline = build_iframe_pipeline(srcdoc, ctx.parent_url.cloned(), ctx);
        let origin = apply_sandbox_origin(
            ctx.parent_origin.clone(),
            sandbox_flags,
            iframe_data.credentialless,
        );
        (pipeline, origin)
    } else if let Some(src) = &iframe_data.src {
        if src.is_empty() || src == "about:blank" {
            let pipeline = build_iframe_pipeline("", ctx.parent_url.cloned(), ctx);
            let origin = apply_sandbox_origin(
                ctx.parent_origin.clone(),
                sandbox_flags,
                iframe_data.credentialless,
            );
            (pipeline, origin)
        } else {
            return load_iframe_from_url(src, iframe_data, sandbox_flags, ctx);
        }
    } else {
        let pipeline = build_iframe_pipeline("", ctx.parent_url.cloned(), ctx);
        let origin = apply_sandbox_origin(
            ctx.parent_origin.clone(),
            sandbox_flags,
            iframe_data.credentialless,
        );
        (pipeline, origin)
    };

    let entry = make_in_process_entry(pipeline, iframe_origin, ctx.depth, sandbox_flags);
    set_referrer(&entry, ctx);
    entry
}

/// Load an iframe from a URL, handling security checks and origin-based dispatch.
#[allow(clippy::cast_precision_loss)]
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
            apply_sandbox_origin(
                ctx.parent_origin.clone(),
                sandbox_flags,
                iframe_data.credentialless,
            ),
            iframe_data,
            ctx,
        );
    };

    // TODO: credentialless iframes should use a separate NetworkHandle
    // with no cookie jar. For now, use the parent's handle.
    match elidex_navigation::load_document(&resolved, ctx.network_handle, None) {
        Ok(loaded) => {
            let doc_origin = SecurityOrigin::from_url(&loaded.url);
            if !check_framing_allowed(&loaded.response_headers, ctx.parent_origin, &doc_origin) {
                eprintln!(
                    "iframe blocked by frame-ancestors/X-Frame-Options: {}",
                    loaded.url
                );
                return blank_entry(SecurityOrigin::opaque(), iframe_data, ctx);
            }

            let origin = apply_sandbox_origin(
                SecurityOrigin::from_url(&loaded.url),
                sandbox_flags,
                iframe_data.credentialless,
            );

            if ctx.parent_origin != &origin {
                return make_out_of_process_entry(loaded, sandbox_flags);
            }

            let pipeline = crate::build_pipeline_from_loaded(
                loaded,
                ctx.network_handle.clone(),
                ctx.font_db.clone(),
            );
            let entry = make_in_process_entry(pipeline, origin, ctx.depth, sandbox_flags);
            set_referrer(&entry, ctx);
            entry
        }
        Err(e) => {
            eprintln!("iframe load error: {e}");
            blank_entry(
                apply_sandbox_origin(
                    ctx.parent_origin.clone(),
                    sandbox_flags,
                    iframe_data.credentialless,
                ),
                iframe_data,
                ctx,
            )
        }
    }
}

/// Set the referrer to the parent document's URL (WHATWG HTML §4.8.5).
fn set_referrer(entry: &IframeEntry, ctx: &IframeLoadContext<'_>) {
    if let IframeHandle::InProcess(ref ip) = entry.handle {
        ip.pipeline
            .runtime
            .bridge()
            .set_referrer(ctx.parent_url.map(url::Url::to_string));
    }
}

// ---------------------------------------------------------------------------
// Entry constructors
// ---------------------------------------------------------------------------

/// Create a same-origin `IframeEntry` from a pipeline and origin.
///
/// `depth` is the nesting depth of this iframe, stored on the bridge for
/// correct `MAX_IFRAME_DEPTH` enforcement across nested `EcsDom` instances.
#[allow(clippy::cast_precision_loss)]
pub(super) fn make_in_process_entry(
    pipeline: crate::PipelineResult,
    origin: SecurityOrigin,
    depth: usize,
    sandbox_flags: Option<IframeSandboxFlags>,
) -> IframeEntry {
    pipeline.runtime.bridge().set_sandbox_flags(sandbox_flags);
    pipeline.runtime.bridge().set_origin(origin);
    pipeline.runtime.bridge().set_iframe_depth(depth);

    IframeEntry {
        handle: IframeHandle::InProcess(Box::new(InProcessIframe {
            pipeline,
            nav_controller: NavigationController::new(),
            focus_target: None,
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
#[allow(clippy::cast_precision_loss)]
fn make_out_of_process_entry(
    loaded: elidex_navigation::LoadedDocument,
    sandbox_flags: Option<IframeSandboxFlags>,
) -> IframeEntry {
    let (parent_chan, iframe_chan) = crate::ipc::channel_pair::<BrowserToIframe, IframeToBrowser>();

    let loaded_url = loaded.url.clone();

    let thread = std::thread::spawn(move || {
        // Build pipeline on this thread (PipelineResult is !Send).
        // Use the already-fetched LoadedDocument — no redundant HTTP request.
        let network_handle = std::rc::Rc::new(elidex_net::broker::NetworkHandle::disconnected());
        let font_db = std::sync::Arc::new(elidex_text::FontDatabase::new());
        let oop_pipeline = crate::build_pipeline_from_loaded(loaded, network_handle, font_db);

        oop_pipeline
            .runtime
            .bridge()
            .set_sandbox_flags(sandbox_flags);
        oop_pipeline
            .runtime
            .bridge()
            .set_origin(apply_sandbox_origin_from_flags(
                SecurityOrigin::from_url(&loaded_url),
                sandbox_flags,
            ));

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
fn blank_entry(
    origin: SecurityOrigin,
    iframe_data: &elidex_ecs::IframeData,
    ctx: &IframeLoadContext<'_>,
) -> IframeEntry {
    let sandbox_flags = parse_sandbox(iframe_data);
    make_in_process_entry(
        build_iframe_pipeline("", ctx.parent_url.cloned(), ctx),
        origin,
        ctx.depth,
        sandbox_flags,
    )
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

/// Build an iframe pipeline from HTML content, sharing the parent's resources.
fn build_iframe_pipeline(
    html: &str,
    url: Option<url::Url>,
    ctx: &IframeLoadContext<'_>,
) -> crate::PipelineResult {
    crate::build_pipeline_interactive_shared(
        html,
        url,
        ctx.font_db.clone(),
        ctx.network_handle.clone(),
        ctx.registry.clone(),
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

/// Apply sandbox origin override from pre-parsed flags (B1 fix).
///
/// Used by the OOP iframe thread on `Navigate`, where credentialless status
/// is not available — only the pre-parsed `IframeSandboxFlags` from the bridge.
pub(super) fn apply_sandbox_origin_from_flags(
    origin: SecurityOrigin,
    sandbox: Option<IframeSandboxFlags>,
) -> SecurityOrigin {
    apply_sandbox_origin(origin, sandbox, false)
}
