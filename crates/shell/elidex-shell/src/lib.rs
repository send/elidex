//! Window management and event loop shell for elidex.
//!
//! Provides the top-level integration that ties together parsing, styling,
//! layout, and rendering into a windowed application.
//!
//! # Usage
//!
//! ```ignore
//! elidex_shell::run("<h1>Hello</h1>", "h1 { color: red; }").unwrap();
//! ```

/// Maximum rounds of CE callback stabilization after `re_render` flush.
const MAX_CE_STABILIZATION_ROUNDS: usize = 8;

pub(crate) mod animation;
mod app;
pub(crate) mod chrome;
mod content;
mod gpu;
pub mod ipc;
pub(crate) mod key_map;
mod pipeline;
pub mod quota;

#[cfg(test)]
mod tests;

use std::rc::Rc;
use std::sync::Arc;

use elidex_css::Stylesheet;
use elidex_css_anim::engine::AnimationEngine;
use elidex_dom_compat::{
    get_presentational_hints, legacy_ua_stylesheet, parse_compat_stylesheet_with_registry,
};
use elidex_ecs::EcsDom;
use elidex_ecs::Entity;
use elidex_html_parser::parse_html;
use elidex_js_boa::{extract_scripts, JsRuntime};
use elidex_layout::layout_tree;
use elidex_plugin::{Size, Vector, ViewportOverflow};
use elidex_render::{build_display_list, build_display_list_with_scroll, DisplayList};
use elidex_script_session::SessionCore;
use elidex_style::resolve_styles_with_compat;
use elidex_text::FontDatabase;
use winit::event_loop::EventLoop;

use animation::{
    apply_active_animations, collect_computed_without_anim, collect_old_anim_styles,
    create_animation_engine, detect_and_start_transitions, sync_css_animations,
};

use app::App;

/// Convert parsed `Stylesheet`s into lightweight `CssomSheet` representations
/// suitable for the JS bridge.
///
/// Each CSS rule's selectors and declarations are serialized to strings so the
/// CSSOM JS layer can expose them without depending on the CSS parser.
fn stylesheets_to_cssom(sheets: &[Stylesheet]) -> Vec<elidex_js_boa::bridge::CssomSheet> {
    sheets
        .iter()
        .map(|sheet| {
            let rules = sheet
                .rules
                .iter()
                .map(|rule| {
                    let selector_text = rule
                        .selectors
                        .iter()
                        .map(selector_to_css_string)
                        .collect::<Vec<_>>()
                        .join(", ");
                    let declarations = rule
                        .declarations
                        .iter()
                        .map(|d| {
                            (
                                d.property.clone(),
                                elidex_dom_api::css_value_to_string(&d.value),
                            )
                        })
                        .collect();
                    elidex_js_boa::bridge::CssomRule {
                        selector_text,
                        declarations,
                    }
                })
                .collect();
            elidex_js_boa::bridge::CssomSheet { rules }
        })
        .collect()
}

/// Serialize a `Selector` to its CSS text representation.
fn selector_to_css_string(selector: &elidex_css::Selector) -> String {
    use elidex_css::SelectorComponent;

    // Selectors are stored right-to-left for matching; serialize left-to-right.
    let components: Vec<&SelectorComponent> = selector.components.iter().rev().collect();
    let mut parts = Vec::new();

    for comp in &components {
        match comp {
            SelectorComponent::Universal => parts.push("*".to_string()),
            SelectorComponent::Tag(tag) => parts.push(tag.clone()),
            SelectorComponent::Class(cls) => parts.push(format!(".{cls}")),
            SelectorComponent::Id(id) => parts.push(format!("#{id}")),
            SelectorComponent::Descendant => parts.push(" ".to_string()),
            SelectorComponent::Child => parts.push(" > ".to_string()),
            SelectorComponent::AdjacentSibling => parts.push(" + ".to_string()),
            SelectorComponent::GeneralSibling => parts.push(" ~ ".to_string()),
            SelectorComponent::PseudoClass(name) => parts.push(format!(":{name}")),
            SelectorComponent::Attribute { name, matcher } => {
                parts.push(format_attribute_selector(name, matcher.as_ref()));
            }
            SelectorComponent::Not(inner) => {
                let inner_str = inner
                    .iter()
                    .rev()
                    .map(selector_component_to_string)
                    .collect::<String>();
                parts.push(format!(":not({inner_str})"));
            }
            _ => {
                // Future selector components — serialize as empty to avoid panics.
            }
        }
    }

    if let Some(pseudo) = &selector.pseudo_element {
        match pseudo {
            elidex_css::PseudoElement::Before => parts.push("::before".to_string()),
            elidex_css::PseudoElement::After => parts.push("::after".to_string()),
        }
    }

    parts.join("")
}

/// Serialize a single `SelectorComponent` to a CSS string fragment.
fn selector_component_to_string(comp: &elidex_css::SelectorComponent) -> String {
    use elidex_css::SelectorComponent;
    match comp {
        SelectorComponent::Universal => "*".to_string(),
        SelectorComponent::Tag(tag) => tag.clone(),
        SelectorComponent::Class(cls) => format!(".{cls}"),
        SelectorComponent::Id(id) => format!("#{id}"),
        SelectorComponent::PseudoClass(name) => format!(":{name}"),
        SelectorComponent::Attribute { name, matcher } => {
            format_attribute_selector(name, matcher.as_ref())
        }
        _ => String::new(),
    }
}

/// Escape a string for use inside a CSS quoted string (`"…"`).
fn escape_css_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

/// Serialize an attribute selector to CSS text.
fn format_attribute_selector(name: &str, matcher: Option<&elidex_css::AttributeMatcher>) -> String {
    use elidex_css::AttributeMatcher;
    match matcher {
        None => format!("[{name}]"),
        Some(m) => {
            let (op, val) = match m {
                AttributeMatcher::Exact(v) => ("=", v.as_str()),
                AttributeMatcher::Includes(v) => ("~=", v.as_str()),
                AttributeMatcher::DashMatch(v) => ("|=", v.as_str()),
                AttributeMatcher::Prefix(v) => ("^=", v.as_str()),
                AttributeMatcher::Suffix(v) => ("$=", v.as_str()),
                AttributeMatcher::Substring(v) => ("*=", v.as_str()),
            };
            let escaped = escape_css_string(val);
            format!("[{name}{op}\"{escaped}\"]")
        }
    }
}

/// Sync stylesheet data to the JS bridge for CSSOM access.
///
/// Call this after pipeline initialization and after any stylesheet mutation
/// so that `document.styleSheets` reflects the current state.
fn sync_stylesheets_to_bridge(runtime: &JsRuntime, stylesheets: &[Stylesheet]) {
    let cssom_sheets = stylesheets_to_cssom(stylesheets);
    runtime.bridge().set_stylesheets(cssom_sheets);
}

/// Apply CSSOM mutations (insertRule/deleteRule) to real `Stylesheet` objects.
///
/// Parses rule text using the CSS parser and inserts/deletes rules at the
/// specified positions. Invalid indices or unparseable rules are silently
/// skipped (matching browser behavior for error recovery).
fn apply_cssom_mutations(
    stylesheets: &mut [Stylesheet],
    mutations: &[elidex_js_boa::bridge::CssomMutation],
    registry: &elidex_plugin::CssPropertyRegistry,
) {
    // Track which sheets were modified for batched source_order recomputation.
    let mut dirty_sheets = std::collections::HashSet::new();

    for mutation in mutations {
        match mutation {
            elidex_js_boa::bridge::CssomMutation::InsertRule {
                sheet_index,
                rule_index,
                rule_text,
            } => {
                let Some(sheet) = stylesheets.get_mut(*sheet_index) else {
                    continue;
                };
                if *rule_index > sheet.rules.len() {
                    continue;
                }
                let parsed = elidex_css::parse_stylesheet_with_registry(
                    rule_text,
                    sheet.origin,
                    Some(registry),
                );
                if let Some(mut rule) = parsed.rules.into_iter().next() {
                    rule.source_order = 0;
                    sheet.rules.insert(*rule_index, rule);
                    dirty_sheets.insert(*sheet_index);
                }
            }
            elidex_js_boa::bridge::CssomMutation::DeleteRule {
                sheet_index,
                rule_index,
            } => {
                let Some(sheet) = stylesheets.get_mut(*sheet_index) else {
                    continue;
                };
                if *rule_index < sheet.rules.len() {
                    sheet.rules.remove(*rule_index);
                    dirty_sheets.insert(*sheet_index);
                }
            }
        }
    }

    // Batch recompute source_order for modified sheets (O(n) per sheet, not per mutation).
    for &idx in &dirty_sheets {
        if let Some(sheet) = stylesheets.get_mut(idx) {
            for (i, r) in sheet.rules.iter_mut().enumerate() {
                r.source_order = u32::try_from(i).unwrap_or(u32::MAX);
            }
        }
    }
}

/// Build the CSS property registry with all standard property handlers.
///
/// Delegates to [`elidex_style::create_css_property_registry`].
#[must_use]
pub fn create_css_property_registry() -> elidex_plugin::CssPropertyRegistry {
    elidex_style::create_css_property_registry()
}

/// Default viewport width for the initial layout pass.
const DEFAULT_VIEWPORT_WIDTH: f32 = 1024.0;
/// Default viewport height for the initial layout pass.
const DEFAULT_VIEWPORT_HEIGHT: f32 = 768.0;

/// HTML content for a blank new-tab page.
const BLANK_TAB_HTML: &str = "<html><body><h1>New Tab</h1></body></html>";
/// CSS for the blank new-tab page.
const BLANK_TAB_CSS: &str = "body { background-color: #ffffff; color: #333333; font-family: sans-serif; } h1 { text-align: center; margin-top: 200px; }";

/// Resolve styles with the compat layer (legacy UA + presentational hints).
///
/// Passes the CSS property registry to enable handler-based dispatch for
/// `is_inherited()`, `initial_value()`, and `get_computed()` queries.
fn resolve_with_compat(
    dom: &mut EcsDom,
    author_stylesheets: &[&Stylesheet],
    registry: &elidex_plugin::CssPropertyRegistry,
    viewport: Size,
) -> ViewportOverflow {
    let legacy_ua = legacy_ua_stylesheet();
    resolve_styles_with_compat(
        dom,
        author_stylesheets,
        &[legacy_ua],
        &get_presentational_hints,
        viewport,
        Some(registry),
    )
}

/// Run the full browser pipeline and display the result in a window.
///
/// Parses HTML, applies CSS, computes layout, builds a display list,
/// and opens a window rendering the result via Vello + wgpu.
///
/// Content processing (DOM, JS, style, layout) runs on a dedicated thread,
/// communicating with the browser thread via message passing.
///
/// This function blocks until the window is closed.
pub fn run(html: &str, css: &str) -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    let mut app = App::new_threaded(html.to_string(), css.to_string());
    event_loop.run_app(&mut app)?;

    Ok(())
}

/// Execute the rendering pipeline without opening a window.
///
/// Useful for testing the parse → style → layout → display list chain.
/// Includes script execution phase: `<script>` tags are evaluated after
/// initial style resolution, followed by re-resolution and layout.
///
/// Delegates to [`build_pipeline_interactive`] and returns only the display list.
#[must_use]
pub fn build_pipeline(html: &str, css: &str) -> elidex_render::DisplayList {
    build_pipeline_interactive(html, css).display_list
}

/// Result of the interactive rendering pipeline.
///
/// Contains all state needed to handle user events and re-render.
pub struct PipelineResult {
    /// The initial display list.
    pub display_list: DisplayList,
    /// The ECS DOM.
    pub dom: EcsDom,
    /// The document root entity.
    pub document: Entity,
    /// The script session state.
    pub session: SessionCore,
    /// The JavaScript runtime.
    pub runtime: JsRuntime,
    /// All parsed CSS stylesheets.
    pub stylesheets: Vec<Stylesheet>,
    /// The font database (shared across navigations to avoid re-scanning).
    pub font_db: Arc<FontDatabase>,
    /// The URL of the current page, if loaded from a URL.
    pub url: Option<url::Url>,
    /// Network handle for communicating with the Network Process broker.
    /// `disconnected()` when no broker is available (standalone tests).
    pub network_handle: Rc<elidex_net::broker::NetworkHandle>,
    /// Keeps the broker thread alive for standalone pipelines.
    /// `None` when the App owns the broker (normal tab mode).
    #[allow(dead_code)]
    pub(crate) broker_keepalive: Option<elidex_net::broker::NetworkProcessHandle>,
    /// CSS property registry (cached to avoid re-creation on each re-render).
    /// `Arc`-wrapped so it can be shared with child iframe pipelines.
    pub registry: Arc<elidex_plugin::CssPropertyRegistry>,
    /// CSS animation/transition engine.
    pub animation_engine: AnimationEngine,
    /// Current viewport dimensions for layout.
    pub viewport: Size,
    /// Whether the text input caret should be visible in the display list.
    ///
    /// Set by the content thread's caret blink timer. Defaults to `true`.
    pub caret_visible: bool,
    /// Cached form ancestor lookups (invalidated on DOM mutation).
    pub ancestor_cache: elidex_form::AncestorCache,
    /// Viewport-level overflow propagated from root/body element.
    pub viewport_overflow: ViewportOverflow,
    /// Viewport scroll offset synced from content thread before re-render.
    pub scroll_offset: Vector,
}

impl PipelineResult {
    /// Remove animation/transition state for entities that no longer exist in the DOM.
    pub(crate) fn prune_dead_animation_entities(&mut self) {
        self.animation_engine.prune_dead_entities(&|entity_id| {
            Entity::from_bits(entity_id).is_some_and(|entity| self.dom.world().contains(entity))
        });
        self.animation_engine.prune_unused_keyframes();
    }
}

/// Execute the rendering pipeline and return all state for interactive use.
///
/// Like `build_pipeline`, but returns the full `PipelineResult` instead
/// of just the display list. This allows the shell to handle user events,
/// dispatch DOM events, and re-render.
#[must_use]
pub fn build_pipeline_interactive(html: &str, css: &str) -> PipelineResult {
    let parse_result = parse_html(html);
    for err in &parse_result.errors {
        eprintln!("HTML parse warning: {err}");
    }
    let mut dom = parse_result.dom;
    let document = parse_result.document;

    elidex_form::init_form_controls(&mut dom);

    let registry = Arc::new(create_css_property_registry());

    let stylesheets = vec![parse_compat_stylesheet_with_registry(
        css,
        elidex_css::Origin::Author,
        Some(&registry),
    )];
    let font_db = Arc::new(FontDatabase::new());

    let scripts = extract_scripts(&dom, document);
    let script_sources: Vec<&str> = scripts.iter().map(|s| s.source.as_str()).collect();

    let (session, runtime, viewport_overflow) = pipeline::run_scripts_and_finalize(
        &mut dom,
        document,
        &stylesheets,
        &script_sources,
        None, // No NetworkHandle in standalone mode.
        None, // No CookieJar.
        &font_db,
        None,
        &registry,
    );

    let display_list = build_display_list(&dom, &font_db);

    let animation_engine = create_animation_engine(&stylesheets);

    let mut result = PipelineResult {
        display_list,
        dom,
        document,
        session,
        runtime,
        stylesheets,
        font_db,
        url: None,
        network_handle: Rc::new(elidex_net::broker::NetworkHandle::disconnected()),
        registry,
        animation_engine,
        viewport: Size::new(DEFAULT_VIEWPORT_WIDTH, DEFAULT_VIEWPORT_HEIGHT),
        caret_visible: true,
        ancestor_cache: elidex_form::AncestorCache::new(),
        viewport_overflow,
        scroll_offset: Vector::<f32>::ZERO,
        broker_keepalive: None,
    };

    // Start CSS animations declared in initial styles.
    sync_css_animations(&mut result, &[]);

    // Sync stylesheet data to the JS bridge for CSSOM access.
    sync_stylesheets_to_bridge(&result.runtime, &result.stylesheets);

    result
}

/// Like [`build_pipeline_interactive`] but with a `NetworkHandle` for network access.
pub(crate) fn build_pipeline_interactive_with_network(
    html: &str,
    css: &str,
    network_handle: Rc<elidex_net::broker::NetworkHandle>,
    cookie_jar: Arc<elidex_net::CookieJar>,
) -> PipelineResult {
    let parse_result = parse_html(html);
    for err in &parse_result.errors {
        eprintln!("HTML parse warning: {err}");
    }
    let mut dom = parse_result.dom;
    let document = parse_result.document;

    elidex_form::init_form_controls(&mut dom);

    let registry = Arc::new(create_css_property_registry());
    let stylesheets = vec![parse_compat_stylesheet_with_registry(
        css,
        elidex_css::Origin::Author,
        Some(&registry),
    )];
    let font_db = Arc::new(FontDatabase::new());
    let scripts = extract_scripts(&dom, document);
    let script_sources: Vec<&str> = scripts.iter().map(|s| s.source.as_str()).collect();

    let (session, runtime, viewport_overflow) = pipeline::run_scripts_and_finalize(
        &mut dom,
        document,
        &stylesheets,
        &script_sources,
        Some(Rc::clone(&network_handle)),
        Some(cookie_jar),
        &font_db,
        None,
        &registry,
    );

    let display_list = build_display_list(&dom, &font_db);
    let animation_engine = create_animation_engine(&stylesheets);

    let mut result = PipelineResult {
        display_list,
        dom,
        document,
        session,
        runtime,
        stylesheets,
        font_db,
        url: None,
        network_handle,
        registry,
        animation_engine,
        viewport: Size::new(DEFAULT_VIEWPORT_WIDTH, DEFAULT_VIEWPORT_HEIGHT),
        caret_visible: true,
        ancestor_cache: elidex_form::AncestorCache::new(),
        viewport_overflow,
        scroll_offset: Vector::<f32>::ZERO,
        broker_keepalive: None,
    };

    sync_css_animations(&mut result, &[]);
    sync_stylesheets_to_bridge(&result.runtime, &result.stylesheets);

    result
}

/// Build a pipeline from HTML, sharing the parent's resources.
///
/// Like [`build_pipeline_interactive`], but uses the provided `font_db`,
/// `network_handle`, and `registry` instead of creating fresh instances.
pub(crate) fn build_pipeline_interactive_shared(
    html: &str,
    url: Option<url::Url>,
    font_db: Arc<FontDatabase>,
    network_handle: Rc<elidex_net::broker::NetworkHandle>,
    registry: Arc<elidex_plugin::CssPropertyRegistry>,
    cookie_jar: Option<Arc<elidex_net::CookieJar>>,
) -> PipelineResult {
    let parse_result = parse_html(html);
    for err in &parse_result.errors {
        eprintln!("HTML parse warning: {err}");
    }
    let mut dom = parse_result.dom;
    let document = parse_result.document;

    elidex_form::init_form_controls(&mut dom);

    let stylesheets = vec![parse_compat_stylesheet_with_registry(
        "",
        elidex_css::Origin::Author,
        Some(&registry),
    )];

    let scripts = extract_scripts(&dom, document);
    let script_sources: Vec<&str> = scripts.iter().map(|s| s.source.as_str()).collect();

    let (session, runtime, viewport_overflow) = pipeline::run_scripts_and_finalize(
        &mut dom,
        document,
        &stylesheets,
        &script_sources,
        Some(Rc::clone(&network_handle)),
        cookie_jar,
        &font_db,
        url.as_ref(),
        &registry,
    );

    let display_list = build_display_list(&dom, &font_db);
    let animation_engine = create_animation_engine(&stylesheets);

    let mut result = PipelineResult {
        display_list,
        dom,
        document,
        session,
        runtime,
        stylesheets,
        font_db,
        url,
        network_handle,
        registry,
        animation_engine,
        viewport: Size::new(DEFAULT_VIEWPORT_WIDTH, DEFAULT_VIEWPORT_HEIGHT),
        caret_visible: true,
        ancestor_cache: elidex_form::AncestorCache::new(),
        viewport_overflow,
        scroll_offset: Vector::<f32>::ZERO,
        broker_keepalive: None,
    };

    sync_css_animations(&mut result, &[]);
    sync_stylesheets_to_bridge(&result.runtime, &result.stylesheets);

    result
}

/// Re-render after DOM changes: re-resolve styles, re-layout, and rebuild display list.
///
/// Includes transition detection: saves old computed values for entities with
/// `AnimStyle`, re-resolves styles, compares old vs new values to detect
/// transitions, feeds them to the `AnimationEngine`, and applies animated
/// values to `ComputedStyle` before layout.
///
/// Returns the mutation records from the flush, for observer dispatch.
pub(crate) fn re_render(result: &mut PipelineResult) -> Vec<elidex_script_session::MutationRecord> {
    // Apply pending CSSOM mutations (insertRule/deleteRule) to real stylesheets.
    let cssom_mutations = result.runtime.bridge().take_cssom_mutations();
    if !cssom_mutations.is_empty() {
        apply_cssom_mutations(&mut result.stylesheets, &cssom_mutations, &result.registry);
        // Re-sync the bridge representation after applying mutations.
        sync_stylesheets_to_bridge(&result.runtime, &result.stylesheets);
    }

    // Flush applies buffered mutations to the DOM and enqueue CE reactions.
    let raw_records = result.session.flush(&mut result.dom);
    let mut mutation_records: Vec<elidex_script_session::MutationRecord> =
        raw_records.into_iter().flatten().collect();

    // Enqueue and drain CE reactions for any custom elements affected by mutations.
    // CE callbacks may trigger additional mutations, so loop until stable (bounded).
    if !mutation_records.is_empty() {
        result
            .runtime
            .enqueue_ce_reactions_from_mutations(&mutation_records, &result.dom);
        result.runtime.drain_custom_element_reactions_public(
            &mut result.session,
            &mut result.dom,
            result.document,
        );

        // Re-flush: CE callbacks may have recorded new mutations.
        // Bounded to prevent infinite loops from mutually-triggering callbacks.
        for round in 0..MAX_CE_STABILIZATION_ROUNDS {
            let follow_up: Vec<_> = result
                .session
                .flush(&mut result.dom)
                .into_iter()
                .flatten()
                .collect();
            if follow_up.is_empty() {
                break;
            }
            // Only process NEW records (not previously-handled ones).
            result
                .runtime
                .enqueue_ce_reactions_from_mutations(&follow_up, &result.dom);
            mutation_records.extend(follow_up);
            result.runtime.drain_custom_element_reactions_public(
                &mut result.session,
                &mut result.dom,
                result.document,
            );
            if round == MAX_CE_STABILIZATION_ROUNDS - 1 {
                eprintln!(
                    "[CE] stabilization loop hit max rounds ({MAX_CE_STABILIZATION_ROUNDS}); \
                     some mutations may be deferred to next frame"
                );
            }
        }
    }

    // Invalidate ancestor cache when DOM mutations occurred.
    if !mutation_records.is_empty() {
        result.ancestor_cache.invalidate_all();
    }

    // Prune animations/transitions for destroyed entities unconditionally.
    // JS event handlers may destroy entities without generating style mutations,
    // so conditional pruning could leak animation state.
    result.prune_dead_animation_entities();

    // Phase 1: Save old computed values for entities with AnimStyle (transition detection).
    // Also snapshot entities without AnimStyle but with ComputedStyle, so that
    // entities gaining AnimStyle in this render cycle have a baseline for transitions.
    let old_styles = collect_old_anim_styles(&result.dom);
    let old_computed_no_anim = collect_computed_without_anim(&result.dom);

    // Phase 2: Re-resolve styles.
    let stylesheet_refs: Vec<&Stylesheet> = result.stylesheets.iter().collect();
    result.viewport_overflow = resolve_with_compat(
        &mut result.dom,
        &stylesheet_refs,
        &result.registry,
        result.viewport,
    );

    // Phase 3: Detect transitions by comparing old vs new computed values.
    // Includes entities that newly gained AnimStyle (transition-* properties).
    detect_and_start_transitions(result, &old_styles, &old_computed_no_anim);

    // Phase 3b: Start/cancel CSS animations based on animation-name changes.
    // CSS Animations L1 §4.2: when animation-name changes, old names are cancelled
    // and new names are started.
    sync_css_animations(result, &old_styles);

    // Phase 4: Apply animated values from active transitions/animations to ComputedStyle.
    apply_active_animations(result);

    layout_tree(&mut result.dom, result.viewport, &result.font_db);

    result.display_list = build_display_list_with_scroll(
        &result.dom,
        &result.font_db,
        result.caret_visible,
        result.scroll_offset,
    );

    mutation_records
}

/// Build a pipeline from a pre-loaded document (from [`elidex_navigation::load_document`]).
///
/// Merges all stylesheets, executes all scripts in document order,
/// resolves styles, computes layout, and builds the display list.
pub fn build_pipeline_from_loaded(
    loaded: elidex_navigation::LoadedDocument,
    network_handle: Rc<elidex_net::broker::NetworkHandle>,
    font_db: Arc<FontDatabase>,
    cookie_jar: Option<Arc<elidex_net::CookieJar>>,
) -> PipelineResult {
    let elidex_navigation::LoadedDocument {
        mut dom,
        document,
        stylesheets,
        scripts,
        url,
        response_headers: _, // Used by iframe loading for CSP/X-Frame-Options checks.
    } = loaded;

    elidex_form::init_form_controls(&mut dom);

    let script_sources: Vec<&str> = scripts.iter().map(|s| s.source.as_str()).collect();

    let registry = Arc::new(create_css_property_registry());

    let (session, runtime, viewport_overflow) = pipeline::run_scripts_and_finalize(
        &mut dom,
        document,
        &stylesheets,
        &script_sources,
        Some(Rc::clone(&network_handle)),
        cookie_jar,
        &font_db,
        Some(&url),
        &registry,
    );

    let display_list = build_display_list(&dom, &font_db);

    let animation_engine = create_animation_engine(&stylesheets);

    let mut result = PipelineResult {
        display_list,
        dom,
        document,
        session,
        runtime,
        stylesheets,
        font_db,
        url: Some(url),
        network_handle,
        registry,
        animation_engine,
        viewport: Size::new(DEFAULT_VIEWPORT_WIDTH, DEFAULT_VIEWPORT_HEIGHT),
        caret_visible: true,
        ancestor_cache: elidex_form::AncestorCache::new(),
        viewport_overflow,
        scroll_offset: Vector::<f32>::ZERO,
        broker_keepalive: None,
    };

    // Start CSS animations declared in initial styles.
    sync_css_animations(&mut result, &[]);

    // Sync stylesheet data to the JS bridge for CSSOM access.
    sync_stylesheets_to_bridge(&result.runtime, &result.stylesheets);

    result
}

/// Build a pipeline from a URL.
///
/// Spawns a temporary Network Process broker to load the document (standalone mode).
/// Content threads should use `build_pipeline_from_loaded` with a proper `NetworkHandle`.
pub fn build_pipeline_from_url(
    url: &url::Url,
) -> Result<PipelineResult, elidex_navigation::LoadError> {
    // Standalone mode: use a disconnected handle for pipeline (no broker).
    // load_document still routes through NetworkHandle::fetch_blocking which
    // returns "network process disconnected" for disconnected handles, so
    // we create a temporary broker for standalone URL loading.
    let np = elidex_net::broker::spawn_network_process(elidex_net::NetClient::new());
    let network_handle = Rc::new(np.create_renderer_handle());
    let loaded = elidex_navigation::load_document(url, &network_handle, None)?;
    let font_db = Arc::new(FontDatabase::new());
    let cookie_jar = Arc::clone(np.cookie_jar());
    let mut result = build_pipeline_from_loaded(loaded, network_handle, font_db, Some(cookie_jar));
    result.broker_keepalive = Some(np); // Keep broker alive for pipeline lifetime.
    Ok(result)
}

/// Run the browser from a URL string, opening a window.
///
/// Parses the URL, fetches the page and its resources, executes scripts,
/// renders the result, and runs the event loop.
///
/// Content processing runs on a dedicated thread.
///
/// This function blocks until the window is closed.
pub fn run_url(url_str: &str) -> Result<(), Box<dyn std::error::Error>> {
    let url = url::Url::parse(url_str)
        .map_err(|e| elidex_navigation::LoadError::InvalidUrl(format!("{url_str}: {e}")))?;

    let event_loop = EventLoop::new()?;
    let mut app = App::new_threaded_url(url);
    event_loop.run_app(&mut app)?;

    Ok(())
}
