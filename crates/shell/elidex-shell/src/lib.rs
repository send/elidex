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

mod app;
pub(crate) mod chrome;
mod content;
mod gpu;
pub mod ipc;
pub(crate) mod key_map;
mod pipeline;

#[cfg(test)]
mod tests;

use std::rc::Rc;

use elidex_css::Stylesheet;
use elidex_css_anim::engine::{AnimationEngine, AnimationEvent};
use elidex_dom_compat::{
    get_presentational_hints, legacy_ua_stylesheet, parse_compat_stylesheet_with_registry,
};
use elidex_ecs::EcsDom;
use elidex_ecs::Entity;
use elidex_html_parser::parse_html;
use elidex_js_boa::{extract_scripts, JsRuntime};
use elidex_layout::layout_tree;
use elidex_net::FetchHandle;
use elidex_plugin::{
    AnimationEventInit, ComputedStyle, EventPayload, Size, TransitionEventInit, Vector,
    ViewportOverflow,
};
use elidex_render::{build_display_list, build_display_list_with_scroll, DisplayList};
use elidex_script_session::{DispatchEvent, SessionCore};
use elidex_style::resolve_styles_with_compat;
use elidex_text::FontDatabase;
use winit::event_loop::EventLoop;

use app::App;

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
    pub font_db: Rc<FontDatabase>,
    /// The URL of the current page, if loaded from a URL.
    pub url: Option<url::Url>,
    /// Shared fetch handle (for cookie sharing across navigation).
    pub fetch_handle: Rc<FetchHandle>,
    /// CSS property registry (cached to avoid re-creation on each re-render).
    pub registry: elidex_plugin::CssPropertyRegistry,
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
    }
}

/// Register `@keyframes` rules from parsed stylesheets into the animation engine.
fn register_keyframes_from_stylesheets(stylesheets: &[Stylesheet], engine: &mut AnimationEngine) {
    for ss in stylesheets {
        for (name, body) in &ss.keyframes_raw {
            let rule = elidex_css_anim::parse::parse_keyframes(name, body);
            engine.register_keyframes(rule);
        }
    }
}

/// Create and initialize an `AnimationEngine` with `@keyframes` from stylesheets.
fn create_animation_engine(stylesheets: &[Stylesheet]) -> AnimationEngine {
    let mut engine = AnimationEngine::new();
    register_keyframes_from_stylesheets(stylesheets, &mut engine);
    engine
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

    let registry = create_css_property_registry();

    let stylesheets = vec![parse_compat_stylesheet_with_registry(
        css,
        elidex_css::Origin::Author,
        Some(&registry),
    )];
    let fetch_handle = Rc::new(FetchHandle::new(elidex_net::NetClient::new()));
    let font_db = Rc::new(FontDatabase::new());

    let scripts = extract_scripts(&dom, document);
    let script_sources: Vec<&str> = scripts.iter().map(|s| s.source.as_str()).collect();

    let (session, runtime, viewport_overflow) = pipeline::run_scripts_and_finalize(
        &mut dom,
        document,
        &stylesheets,
        &script_sources,
        Rc::clone(&fetch_handle),
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
        fetch_handle,
        registry,
        animation_engine,
        viewport: Size::new(DEFAULT_VIEWPORT_WIDTH, DEFAULT_VIEWPORT_HEIGHT),
        caret_visible: true,
        ancestor_cache: elidex_form::AncestorCache::new(),
        viewport_overflow,
        scroll_offset: Vector::<f32>::ZERO,
    };

    // Start CSS animations declared in initial styles.
    sync_css_animations(&mut result, &[]);

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
    // Flush applies buffered mutations to the DOM.
    let raw_records = result.session.flush(&mut result.dom);
    let mutation_records: Vec<elidex_script_session::MutationRecord> =
        raw_records.into_iter().flatten().collect();

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

/// Collect old computed styles for entities that have an `AnimStyle` component.
///
/// Returns a list of `(entity_bits, AnimStyle, ComputedStyle)` tuples for
/// entities with transition properties, used by transition detection.
fn collect_old_anim_styles(
    dom: &EcsDom,
) -> Vec<(u64, elidex_css_anim::style::AnimStyle, ComputedStyle)> {
    use elidex_css_anim::style::AnimStyle;

    let mut result = Vec::new();
    for (entity, (anim_style, computed)) in &mut dom
        .world()
        .query::<(Entity, (&AnimStyle, &ComputedStyle))>()
    {
        if !anim_style.transition_property.is_empty() || !anim_style.animation_name.is_empty() {
            result.push((entity.to_bits().get(), anim_style.clone(), computed.clone()));
        }
    }
    result
}

/// Collect computed styles for entities that have `ComputedStyle` but no `AnimStyle`.
///
/// Used as baseline for entities that gain `AnimStyle` (transition-* properties)
/// during style re-resolution, so their first transition has a correct "from" value.
fn collect_computed_without_anim(dom: &EcsDom) -> std::collections::HashMap<u64, ComputedStyle> {
    use elidex_css_anim::style::AnimStyle;

    let has_anim: std::collections::HashSet<u64> = dom
        .world()
        .query::<(Entity, &AnimStyle)>()
        .iter()
        .map(|(e, _)| e.to_bits().get())
        .collect();

    let mut result = std::collections::HashMap::new();
    for (entity, computed) in &mut dom.world().query::<(Entity, &ComputedStyle)>() {
        let bits = entity.to_bits().get();
        if !has_anim.contains(&bits) {
            result.insert(bits, computed.clone());
        }
    }
    result
}

/// Compare old vs new computed values to detect transitions, then add them to the engine.
///
/// Only compares properties listed in the entity's `transition-property` (or all
/// animatable properties when `transition-property: all`).
fn detect_and_start_transitions(
    result: &mut PipelineResult,
    old_styles: &[(
        u64,
        elidex_css_anim::style::AnimStyle,
        elidex_plugin::ComputedStyle,
    )],
    old_computed_no_anim: &std::collections::HashMap<u64, ComputedStyle>,
) {
    use elidex_css_anim::style::{AnimStyle, TransitionProperty};

    let registry = &result.registry;
    let mut cancel_dispatches = Vec::new();

    for &(entity_bits, ref old_anim_style, ref old_computed) in old_styles {
        let entity = Entity::from_bits(entity_bits);
        let Some(entity) = entity else { continue };
        let new_computed = {
            let Ok(r) = result.dom.world().get::<&ComputedStyle>(entity) else {
                continue;
            };
            ComputedStyle::clone(&r)
        };

        let new_anim_style = result
            .dom
            .world()
            .get::<&AnimStyle>(entity)
            .ok()
            .map(|a| AnimStyle::clone(&a));

        // Union of old and new transition-property lists for change detection.
        let comparison = new_anim_style.as_ref().unwrap_or(old_anim_style);
        let prop_sources: &[&[TransitionProperty]] = &[
            &old_anim_style.transition_property,
            &comparison.transition_property,
        ];
        let params_style = new_anim_style.as_ref().unwrap_or(old_anim_style);

        detect_changed_and_add(
            entity_bits,
            old_computed,
            &new_computed,
            prop_sources,
            params_style,
            registry,
            &mut result.animation_engine,
            &mut cancel_dispatches,
        );
    }

    // Handle entities that newly gained AnimStyle (were not in old_styles).
    let old_bits: std::collections::HashSet<u64> =
        old_styles.iter().map(|&(bits, _, _)| bits).collect();
    for (entity, new_anim_style) in &mut result.dom.world().query::<(Entity, &AnimStyle)>() {
        let bits = entity.to_bits().get();
        if old_bits.contains(&bits) || new_anim_style.transition_property.is_empty() {
            continue;
        }
        let Some(old_computed) = old_computed_no_anim.get(&bits) else {
            continue;
        };
        let new_computed = {
            let Ok(c) = result.dom.world().get::<&ComputedStyle>(entity) else {
                continue;
            };
            ComputedStyle::clone(&c)
        };
        let prop_sources: &[&[TransitionProperty]] = &[&new_anim_style.transition_property];

        detect_changed_and_add(
            bits,
            old_computed,
            &new_computed,
            prop_sources,
            new_anim_style,
            registry,
            &mut result.animation_engine,
            &mut cancel_dispatches,
        );
    }

    dispatch_anim_events(&cancel_dispatches, result);
}

/// Synchronize CSS animations from `AnimStyle.animation_name` to the engine.
///
/// CSS Animations Level 1 §4.2: when `animation-name` on an element changes,
/// animations for removed names are cancelled (emitting `animationcancel`) and
/// animations for new names are started.
///
/// `old_styles` contains the pre-re-resolve `AnimStyle` snapshot. For the initial
/// render pass an empty slice (all names are considered new).
fn sync_css_animations(
    result: &mut PipelineResult,
    old_styles: &[(u64, elidex_css_anim::style::AnimStyle, ComputedStyle)],
) {
    use elidex_css_anim::instance::AnimationInstance;
    use elidex_css_anim::style::AnimStyle;
    use std::collections::HashMap;

    // Build lookup from entity_bits → old animation names (ordered Vec).
    let old_names_map: HashMap<u64, Vec<String>> = old_styles
        .iter()
        .map(|(bits, anim_style, _)| (*bits, anim_style.animation_name.clone()))
        .collect();

    // Collect current AnimStyle data for all entities.
    let current: Vec<(u64, AnimStyle)> = result
        .dom
        .world()
        .query::<(Entity, &AnimStyle)>()
        .iter()
        .filter(|(_, anim_style)| !anim_style.animation_name.is_empty())
        .map(|(entity, anim_style)| (entity.to_bits().get(), anim_style.clone()))
        .collect();

    let mut cancel_events = Vec::new();
    let timeline_time = result.animation_engine.timeline().current_time();

    for (entity_bits, anim_style) in &current {
        let old_names_vec: Vec<&str> = old_names_map
            .get(entity_bits)
            .map_or_else(Vec::new, |names| names.iter().map(String::as_str).collect());
        let new_names_vec: Vec<&str> = anim_style
            .animation_name
            .iter()
            .map(String::as_str)
            .collect();

        // CSS Animations L1 §4.2: "The same @keyframes rule name may be repeated...
        // each one is independently started." If the name list changed at all,
        // cancel all old animations and restart from scratch.
        if old_names_vec != new_names_vec {
            let events = result.animation_engine.cancel_animations(*entity_bits);
            cancel_events.extend(events);

            // Start all new animations.
            for (i, name) in anim_style.animation_name.iter().enumerate() {
                if name == "none" {
                    continue;
                }
                if result.animation_engine.get_keyframes(name).is_none() {
                    continue;
                }
                let spec = build_animation_spec(name, i, anim_style);
                let instance = AnimationInstance::new(&spec, timeline_time);
                result
                    .animation_engine
                    .add_animation(*entity_bits, instance);
            }
        }
    }

    // Cancel animations for entities whose animation-name was cleared entirely
    // (no longer in `current` because filter requires non-empty animation_name).
    let current_set: std::collections::HashSet<u64> =
        current.iter().map(|(bits, _)| *bits).collect();
    for (entity_bits, old_names) in &old_names_map {
        if !old_names.is_empty() && !current_set.contains(entity_bits) {
            let events = result.animation_engine.cancel_animations(*entity_bits);
            cancel_events.extend(events);
        }
    }

    if !cancel_events.is_empty() {
        dispatch_anim_events(&cancel_events, result);
    }
}

/// Build a `SingleAnimationSpec` from `AnimStyle` lists with CSS cycling.
///
/// CSS Animations L1 §5.2: when animation property lists have different lengths,
/// values cycle (index mod `list.len()`).
fn build_animation_spec(
    name: &str,
    index: usize,
    anim_style: &elidex_css_anim::style::AnimStyle,
) -> elidex_css_anim::SingleAnimationSpec {
    use elidex_css_anim::style::{
        AnimationDirection, AnimationFillMode, IterationCount, PlayState,
    };
    use elidex_css_anim::timing::TimingFunction;

    let cycle = |list_len: usize| -> usize {
        if list_len == 0 {
            0
        } else {
            index % list_len
        }
    };

    elidex_css_anim::SingleAnimationSpec {
        name: name.to_string(),
        duration: anim_style
            .animation_duration
            .get(cycle(anim_style.animation_duration.len()))
            .copied()
            .unwrap_or(0.0),
        timing_function: anim_style
            .animation_timing_function
            .get(cycle(anim_style.animation_timing_function.len()))
            .cloned()
            .unwrap_or(TimingFunction::EASE),
        delay: anim_style
            .animation_delay
            .get(cycle(anim_style.animation_delay.len()))
            .copied()
            .unwrap_or(0.0),
        iteration_count: anim_style
            .animation_iteration_count
            .get(cycle(anim_style.animation_iteration_count.len()))
            .copied()
            .unwrap_or(IterationCount::default()),
        direction: anim_style
            .animation_direction
            .get(cycle(anim_style.animation_direction.len()))
            .copied()
            .unwrap_or(AnimationDirection::default()),
        fill_mode: anim_style
            .animation_fill_mode
            .get(cycle(anim_style.animation_fill_mode.len()))
            .copied()
            .unwrap_or(AnimationFillMode::default()),
        play_state: anim_style
            .animation_play_state
            .get(cycle(anim_style.animation_play_state.len()))
            .copied()
            .unwrap_or(PlayState::default()),
    }
}

/// Compare old vs new computed values for the given transition-property lists,
/// detect transitions, and add them to the engine.
#[allow(clippy::too_many_arguments)]
fn detect_changed_and_add(
    entity_bits: u64,
    old_computed: &ComputedStyle,
    new_computed: &ComputedStyle,
    prop_sources: &[&[elidex_css_anim::style::TransitionProperty]],
    params_style: &elidex_css_anim::style::AnimStyle,
    registry: &elidex_plugin::CssPropertyRegistry,
    engine: &mut elidex_css_anim::engine::AnimationEngine,
    cancel_dispatches: &mut Vec<(u64, elidex_css_anim::engine::AnimationEvent)>,
) {
    use elidex_css_anim::detection::detect_transitions;
    use elidex_css_anim::instance::TransitionInstance;
    use elidex_css_anim::style::TransitionProperty;

    let has_all = prop_sources
        .iter()
        .flat_map(|s| s.iter())
        .any(|p| matches!(p, TransitionProperty::All));

    let named_props: std::collections::HashSet<&str> = if has_all {
        std::collections::HashSet::new()
    } else {
        prop_sources
            .iter()
            .flat_map(|s| s.iter())
            .filter_map(|p| match p {
                TransitionProperty::Property(name) => Some(name.as_str()),
                _ => None,
            })
            .collect()
    };

    let mut changed = Vec::new();
    for prop_name in elidex_css_anim::interpolate::ANIMATABLE_PROPERTIES {
        if !has_all && !named_props.contains(prop_name) {
            continue;
        }
        let old_val = elidex_style::get_computed_with_registry(prop_name, old_computed, registry);
        let new_val = elidex_style::get_computed_with_registry(prop_name, new_computed, registry);
        if old_val != new_val {
            changed.push((prop_name.to_string(), old_val, new_val));
        }
    }

    if changed.is_empty() {
        return;
    }

    let detected = detect_transitions(params_style, &changed);
    for dt in detected {
        let trans = TransitionInstance::new(
            dt.property,
            dt.from,
            dt.to,
            dt.duration,
            dt.delay,
            dt.timing_function,
        );
        let events = engine.add_transition(entity_bits, trans);
        cancel_dispatches.extend(events);
    }
}

/// Dispatch animation/transition events to the DOM via `PipelineResult`.
///
/// Shared by `re_render` (cancel events during transition detection) and
/// `content/animation.rs` (all event kinds from engine tick).
pub(crate) fn dispatch_anim_events(events: &[(u64, AnimationEvent)], result: &mut PipelineResult) {
    for &(eid, ref event) in events {
        let entity = Entity::from_bits(eid);
        let Some(entity) = entity else { continue };
        if !result.dom.world().contains(entity) {
            continue;
        }
        let (event_type, payload) = anim_event_to_payload(event);
        let mut dispatch = DispatchEvent::new_composed(event_type, entity);
        dispatch.cancelable = false;
        dispatch.payload = payload;
        result.runtime.dispatch_event(
            &mut dispatch,
            &mut result.session,
            &mut result.dom,
            result.document,
        );
    }
}

/// Convert an `AnimationEvent` to its DOM event type string and payload.
pub(crate) fn anim_event_to_payload(event: &AnimationEvent) -> (&'static str, EventPayload) {
    use elidex_css_anim::engine::{
        AnimationEventData, AnimationEventKind, TransitionEventData, TransitionEventKind,
    };

    match event {
        AnimationEvent::Transition(TransitionEventData {
            kind,
            ref property,
            elapsed_time,
        }) => {
            let name = match kind {
                TransitionEventKind::Run => "transitionrun",
                TransitionEventKind::Start => "transitionstart",
                TransitionEventKind::End => "transitionend",
                TransitionEventKind::Cancel => "transitioncancel",
            };
            (
                name,
                EventPayload::Transition(TransitionEventInit {
                    property_name: property.clone(),
                    elapsed_time: *elapsed_time,
                    pseudo_element: String::new(),
                }),
            )
        }
        AnimationEvent::Animation(AnimationEventData {
            kind,
            ref name,
            elapsed_time,
        }) => {
            let event_name = match kind {
                AnimationEventKind::Start => "animationstart",
                AnimationEventKind::End => "animationend",
                AnimationEventKind::Iteration => "animationiteration",
                AnimationEventKind::Cancel => "animationcancel",
            };
            (
                event_name,
                EventPayload::Animation(AnimationEventInit {
                    animation_name: name.clone(),
                    elapsed_time: *elapsed_time,
                    pseudo_element: String::new(),
                }),
            )
        }
    }
}

/// Apply animated values from active transitions and animations to `ComputedStyle`.
///
/// Iterates over all entities with active transitions and animations in the engine,
/// reads the current interpolated values, and overwrites the corresponding
/// `ComputedStyle` fields.
fn apply_active_animations(result: &mut PipelineResult) {
    use elidex_css_anim::apply::apply_animated_value;

    // Collect entity IDs that have active transitions/animations (to avoid borrow conflict).
    // Animations are applied after transitions so they take priority when both affect
    // the same property (CSS Animations L1 §4.1). Duplicates are safe (last write wins).
    let entity_ids: Vec<u64> = result.animation_engine.active_entity_ids().collect();

    for entity_bits in entity_ids {
        let entity = Entity::from_bits(entity_bits);
        let Some(entity) = entity else { continue };

        // Collect animated values from transitions.
        let transitions = result.animation_engine.active_transitions(entity_bits);
        let mut animated_values: Vec<(String, elidex_plugin::CssValue)> = transitions
            .iter()
            .filter_map(|t| t.current_value().map(|v| (t.property.clone(), v)))
            .collect();

        // Collect animated values from keyframe animations.
        // CSS Animations L1 §5: read underlying computed values before applying
        // animation, so before-only keyframe properties interpolate correctly.
        let underlying_values: Option<std::collections::HashMap<String, elidex_plugin::CssValue>> =
            result
                .dom
                .world()
                .get::<&ComputedStyle>(entity)
                .ok()
                .map(|style| {
                    let mut map = std::collections::HashMap::new();
                    for prop in elidex_css_anim::interpolate::ANIMATABLE_PROPERTIES {
                        let v = elidex_style::get_computed(prop, &style);
                        if !matches!(v, elidex_plugin::CssValue::Keyword(ref k) if k == "initial") {
                            map.insert((*prop).to_string(), v);
                        }
                    }
                    map
                });

        let animations = result.animation_engine.active_animations(entity_bits);
        for anim in animations {
            if let Some(progress) = anim.progress() {
                let kf_values = result.animation_engine.keyframe_values(
                    anim.name(),
                    f64::from(progress),
                    Some(anim.timing_function()),
                    underlying_values.as_ref(),
                );
                animated_values.extend(kf_values);
            }
        }

        // Apply to ComputedStyle.
        if !animated_values.is_empty() {
            if let Ok(mut style) = result.dom.world_mut().get::<&mut ComputedStyle>(entity) {
                for (property, value) in &animated_values {
                    apply_animated_value(&mut style, property, value);
                }
            }
        }
    }
}

/// Build a pipeline from a pre-loaded document (from [`elidex_navigation::load_document`]).
///
/// Merges all stylesheets, executes all scripts in document order,
/// resolves styles, computes layout, and builds the display list.
pub fn build_pipeline_from_loaded(
    loaded: elidex_navigation::LoadedDocument,
    fetch_handle: Rc<FetchHandle>,
    font_db: Rc<FontDatabase>,
) -> PipelineResult {
    let elidex_navigation::LoadedDocument {
        mut dom,
        document,
        stylesheets,
        scripts,
        url,
    } = loaded;

    elidex_form::init_form_controls(&mut dom);

    let script_sources: Vec<&str> = scripts.iter().map(|s| s.source.as_str()).collect();

    let registry = create_css_property_registry();

    let (session, runtime, viewport_overflow) = pipeline::run_scripts_and_finalize(
        &mut dom,
        document,
        &stylesheets,
        &script_sources,
        Rc::clone(&fetch_handle),
        &font_db,
        Some(url.clone()),
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
        fetch_handle,
        registry,
        animation_engine,
        viewport: Size::new(DEFAULT_VIEWPORT_WIDTH, DEFAULT_VIEWPORT_HEIGHT),
        caret_visible: true,
        ancestor_cache: elidex_form::AncestorCache::new(),
        viewport_overflow,
        scroll_offset: Vector::<f32>::ZERO,
    };

    // Start CSS animations declared in initial styles.
    sync_css_animations(&mut result, &[]);

    result
}

/// Build a pipeline from a URL.
///
/// Creates a `FetchHandle`, loads the document, and runs the full pipeline.
pub fn build_pipeline_from_url(
    url: &url::Url,
) -> Result<PipelineResult, elidex_navigation::LoadError> {
    let fetch_handle = Rc::new(FetchHandle::new(elidex_net::NetClient::new()));
    let loaded = elidex_navigation::load_document(url, &fetch_handle, None)?;
    let font_db = Rc::new(FontDatabase::new());
    Ok(build_pipeline_from_loaded(loaded, fetch_handle, font_db))
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
