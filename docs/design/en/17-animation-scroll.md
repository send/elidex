
# 17. Animation & Scroll Architecture

## 17.1 Overview

Animation is one of the most performance-sensitive areas of a browser engine. Users perceive animation jank (missed frames) at thresholds as low as a single dropped frame. This chapter defines:

- The unified animation model (Web Animations as the internal representation)
- The FrameProducer coordinator that bridges the event loop (Ch. 5) and the rendering pipeline (Ch. 15)
- The AnimationEngine that ticks animations using ECS queries
- Compositor animation promotion (when and how animations move off the main thread)
- Smooth scrolling and scroll anchoring

The compositor-driven side of animation and scroll (layer transforms, scroll offset updates, scroll physics) is defined in Ch. 15 §15.9. This chapter covers the main-thread side: animation timing, interpolation, lifecycle, and the coordination layer that feeds the compositor.

## 17.2 FrameProducer

### 17.2.1 Role

The FrameProducer sits between the event loop and the rendering pipeline. The event loop (Ch. 5) decides **when** to produce a frame; the FrameProducer decides **what** happens within a frame.

```
EventLoop (Ch. 5)                    FrameProducer (Ch. 17)
┌─────────────────────┐              ┌─────────────────────────┐
│ vsync / dirty signal │              │ AnimationEngine.tick()  │
│ drain microtasks     │──produce()──▶│ ObserverRunner.run()    │
│ run rAF callbacks    │              │ RenderingPipeline.run() │
│ (script-side work)   │              │ CompositorChannel.submit│
└─────────────────────┘              └─────────────────────────┘
```

This separation provides three benefits:

1. **Responsibility clarity**: The event loop owns script execution and timing; the FrameProducer owns rendering orchestration. Ch. 5 and Ch. 17 have a clean boundary.
2. **Testability**: FrameProducer can be unit-tested with a mock ECS World and injected timestamps, without an event loop.
3. **elidex-app flexibility**: App-mode applications can drive the FrameProducer directly without a full browser event loop.

### 17.2.2 Frame Sequence

Each frame follows the HTML specification's "update the rendering" steps. The event loop handles the script-side steps, then hands off to the FrameProducer for rendering:

```
vsync signal arrives
  │
  │  [Event Loop — script side]
  ├─ 1. Drain microtask queue
  ├─ 2. Execute requestAnimationFrame callbacks
  │     └─ JS may mutate DOM/CSSOM (buffered in ScriptSession)
  ├─ 3. ScriptSession flush (mutations → ECS)
  │
  │  [FrameProducer — rendering side]
  ├─ 4. AnimationEngine.tick(frame_time)
  │     └─ Sample all animations, write ComputedStyle or queue compositor updates
  ├─ 5. ResizeObserver notifications
  ├─ 6. IntersectionObserver notifications
  ├─ 7. StyleSystem.resolve()       ─ parallel (rayon)
  ├─ 8. LayoutSystem.compute()      ─ parallel (rayon)
  ├─ 9. PaintSystem.paint()         ─ parallel per layer
  ├─10. Build/update LayerTree
  ├─11. Submit DisplayList to Compositor (Ch. 15 §15.8, Ch. 6)
  │
  └─ Frame complete. Event loop resumes for next event batch.
```

Steps 5–6 (Observers) may trigger further DOM mutations, which require re-flushing and potentially re-ticking animations. The FrameProducer caps this re-entrant loop (maximum 4 iterations, matching Chromium) to prevent infinite observer cycles.

### 17.2.3 Interface

```rust
pub struct FrameProducer {
    animation_engine: AnimationEngine,
    observer_runner: ObserverRunner,
    rendering_pipeline: RenderingPipeline,
    compositor_channel: CompositorChannel,
    frame_policy: FramePolicy,           // Ch. 15 §15.8
    frame_timing: FrameTimingRecorder,
}

impl FrameProducer {
    /// Called by the event loop after script-side steps are complete.
    pub fn produce(&mut self, world: &mut World, frame_time: FrameTime) {
        // Step 4: Animation tick
        let compositor_updates = self.animation_engine.tick(world, frame_time);

        // Steps 5–6: Observers (may re-enter up to 4 times)
        let mut observer_rounds = 0;
        while self.observer_runner.has_pending(world) && observer_rounds < 4 {
            self.observer_runner.run(world);
            observer_rounds += 1;
        }

        // Steps 7–10: Rendering pipeline
        self.rendering_pipeline.run(world);

        // Step 11: Submit to compositor
        self.compositor_channel.submit(world, compositor_updates);

        // Record timing
        self.frame_timing.record(world);
    }
}
```

### 17.2.4 elidex-app Usage

In elidex-app, the FrameProducer can be driven without a browser event loop:

```rust
// elidex-app: minimal frame loop
let mut world = World::new();
let mut frame_producer = FrameProducer::new(config);

// OnDemand mode: only produce frames when dirty
loop {
    let event = platform.wait_event();  // blocks until input or dirty
    handle_event(&mut world, event);

    if world.resource::<DirtyFlag>().is_set() {
        let frame_time = FrameTime::now();
        frame_producer.produce(&mut world, frame_time);
        world.resource_mut::<DirtyFlag>().clear();
    }
}
```

## 17.3 Unified Animation Model

### 17.3.1 Web Animations as Internal Representation

Elidex uses the Web Animations model (WAAPI) as its sole internal animation representation. CSS Transitions and CSS Animations are not separate engines — they are translated into Web Animations instances at creation time.

```
CSS Transitions                    Web Animations API
  property change ─────────┐           element.animate()
                           ▼                 │
                   ┌───────────────┐         │
                   │  Animation    │◄────────┘
                   │  Instance     │
                   │  (unified)    │
                   ├───────────────┤
                   │  Timeline     │
                   │  Keyframes    │
                   │  Timing       │
                   │  Play state   │
                   └───────┬───────┘
                           │
CSS Animations             │
  @keyframes ──────────────┘
  animation-* properties
```

This unified model means:

- One animation sampling path, one timing model, one interpolation engine.
- `element.getAnimations()` returns all active animations regardless of origin (CSS or JS).
- Animation composition (multiple animations targeting the same property) is handled by a single priority/composite system.
- The compositor promotion logic has one input format.

### 17.3.2 ECS Components

Animation data lives in ECS components, enabling the AnimationEngine to use ECS parallel queries:

```rust
/// Attached to entities that have one or more active animations.
pub struct ActiveAnimations {
    pub animations: SmallVec<[AnimationInstance; 4]>,
}

/// A single animation instance (unified: CSS Transition, CSS Animation, or WAAPI).
pub struct AnimationInstance {
    pub id: AnimationId,
    pub origin: AnimationOrigin,
    pub timeline: TimelineRef,
    pub keyframes: KeyframeEffect,
    pub timing: AnimationTiming,
    pub play_state: PlayState,
    pub composite: CompositeOperation,
    /// Which property this animation targets
    pub target_property: AnimatableProperty,
    /// If true, this animation has been promoted to the compositor.
    /// Main thread no longer samples it; compositor handles interpolation.
    pub compositor_promoted: bool,
}

pub enum AnimationOrigin {
    CssTransition { property: CssPropertyId, transition_event_pending: bool },
    CssAnimation { name: AnimationName, iteration_event_pending: bool },
    WebAnimationsApi,
}

pub enum PlayState {
    Idle,
    Running,
    Paused,
    Finished,
}
```

The `DocumentTimeline` is an ECS resource (one per document/World):

```rust
/// ECS resource: the default timeline for the document.
pub struct DocumentTimeline {
    pub current_time: f64,         // in milliseconds
    pub origin_time: Instant,      // when the document was created
    pub playback_rate: f64,        // normally 1.0
}

impl DocumentTimeline {
    pub fn advance(&mut self, frame_time: Instant) {
        let elapsed = frame_time.duration_since(self.origin_time);
        self.current_time = elapsed.as_secs_f64() * 1000.0 * self.playback_rate;
    }
}
```

### 17.3.3 CSS Transition → Web Animation Translation

When the StyleSystem detects a property change on an element that has a `transition` declaration, it creates an `AnimationInstance` with `origin: CssTransition`:

```rust
impl TransitionHandler {
    pub fn on_style_change(
        &self,
        entity: EntityId,
        property: CssPropertyId,
        old_value: &CssValue,
        new_value: &CssValue,
        transition_def: &TransitionDefinition,
        world: &mut World,
    ) {
        let keyframes = KeyframeEffect {
            frames: vec![
                Keyframe { offset: 0.0, value: old_value.clone() },
                Keyframe { offset: 1.0, value: new_value.clone() },
            ],
        };

        let timing = AnimationTiming {
            duration: transition_def.duration,
            delay: transition_def.delay,
            easing: transition_def.timing_function.clone(),
            iterations: 1.0,
            direction: PlaybackDirection::Normal,
            fill: FillMode::Backwards,
        };

        let instance = AnimationInstance {
            id: AnimationId::new(),
            origin: AnimationOrigin::CssTransition {
                property,
                transition_event_pending: true,
            },
            timeline: TimelineRef::Document,
            keyframes,
            timing,
            play_state: PlayState::Running,
            composite: CompositeOperation::Replace,
            target_property: AnimatableProperty::from(property),
            compositor_promoted: false,
        };

        // Add to ECS — AnimationEngine picks it up on next tick
        world.get_or_insert_default::<ActiveAnimations>(entity)
            .animations.push(instance);
    }
}
```

### 17.3.4 CSS Animation → Web Animation Translation

When the StyleSystem resolves `animation-name` on an element, it looks up the `@keyframes` rule and creates an `AnimationInstance` with `origin: CssAnimation`:

```rust
impl CssAnimationHandler {
    pub fn on_animation_property(
        &self,
        entity: EntityId,
        animation_props: &CssAnimationProperties,
        keyframes_registry: &KeyframesRegistry,
        world: &mut World,
    ) {
        let Some(keyframes_rule) = keyframes_registry.get(&animation_props.name) else {
            return; // Unknown @keyframes name — no animation
        };

        let keyframes = KeyframeEffect {
            frames: keyframes_rule.frames.iter().map(|kf| {
                Keyframe {
                    offset: kf.offset,
                    value: kf.value.clone(),
                }
            }).collect(),
        };

        let timing = AnimationTiming {
            duration: animation_props.duration,
            delay: animation_props.delay,
            easing: animation_props.timing_function.clone(),
            iterations: animation_props.iteration_count,
            direction: animation_props.direction,
            fill: animation_props.fill_mode,
        };

        let instance = AnimationInstance {
            id: AnimationId::new(),
            origin: AnimationOrigin::CssAnimation {
                name: animation_props.name.clone(),
                iteration_event_pending: false,
            },
            timeline: TimelineRef::Document,
            keyframes,
            timing,
            play_state: if animation_props.play_state == CssPlayState::Paused {
                PlayState::Paused
            } else {
                PlayState::Running
            },
            composite: CompositeOperation::Replace,
            target_property: keyframes_rule.target_property(),
            compositor_promoted: false,
        };

        world.get_or_insert_default::<ActiveAnimations>(entity)
            .animations.push(instance);
    }
}
```

### 17.3.5 Web Animations API (Script)

`element.animate()` calls are handled by the ScriptSession, which writes directly to ECS:

```rust
// In ScriptSession — invoked by JS: element.animate(keyframes, options)
pub fn create_animation(
    &mut self,
    entity: EntityId,
    keyframes: Vec<Keyframe>,
    options: AnimationOptions,
) -> AnimationId {
    let instance = AnimationInstance {
        id: AnimationId::new(),
        origin: AnimationOrigin::WebAnimationsApi,
        // ... fill from keyframes and options
        compositor_promoted: false,
    };

    let id = instance.id;
    self.mutation_buffer.add_animation(entity, instance);
    id
}
```

## 17.4 AnimationEngine

### 17.4.1 Tick

The AnimationEngine uses ECS parallel queries to sample all active animations:

```rust
pub struct AnimationEngine {
    interpolator: PropertyInterpolator,
    compositor_updates: Vec<CompositorAnimationUpdate>,
}

impl AnimationEngine {
    pub fn tick(
        &mut self,
        world: &mut World,
        frame_time: FrameTime,
    ) -> Vec<CompositorAnimationUpdate> {
        // Advance the document timeline
        world.resource_mut::<DocumentTimeline>().advance(frame_time.instant);

        let timeline_time = world.resource::<DocumentTimeline>().current_time;
        self.compositor_updates.clear();

        // Parallel query: each entity processed independently.
        // Uses thread-local accumulator to avoid &mut self borrow conflict in parallel closure.
        let interpolator = &self.interpolator;
        let finish_events: Mutex<Vec<(Entity, AnimationId)>> = Mutex::new(Vec::new());

        world.par_query::<(Entity, &mut ActiveAnimations, &mut ComputedStyle)>()
            .for_each(|(entity, anims, style)| {
                anims.animations.retain_mut(|anim| {
                    if anim.play_state != PlayState::Running {
                        return anim.play_state != PlayState::Idle;
                    }

                    // Skip compositor-promoted animations (compositor handles sampling)
                    if anim.compositor_promoted {
                        return true;
                    }

                    // Sample the animation
                    let local_time = anim.timing.local_time(timeline_time);
                    match anim.timing.phase(local_time) {
                        AnimationPhase::Before => {
                            if anim.timing.fill == FillMode::Backwards
                                || anim.timing.fill == FillMode::Both
                            {
                                let value = anim.keyframes.sample(0.0, interpolator);
                                style.set(anim.target_property, value);
                            }
                        }
                        AnimationPhase::Active(progress) => {
                            let eased = anim.timing.easing.ease(progress);
                            let value = anim.keyframes.sample(eased, interpolator);
                            style.set(anim.target_property, value);
                        }
                        AnimationPhase::After => {
                            if anim.timing.fill == FillMode::Forwards
                                || anim.timing.fill == FillMode::Both
                            {
                                let value = anim.keyframes.sample(1.0, interpolator);
                                style.set(anim.target_property, value);
                            }
                            anim.play_state = PlayState::Finished;
                            finish_events.lock().unwrap().push((entity, anim.id));
                        }
                        AnimationPhase::Idle => {
                            // Remove from active set
                            return false;
                        }
                    }
                    true
                });
            });

        // Dispatch finish events after parallel query completes
        for (entity, anim_id) in finish_events.into_inner().unwrap() {
            self.queue_finish_event(entity, anim_id);
        }

        std::mem::take(&mut self.compositor_updates)
    }
}
```

### 17.4.2 Property Interpolation

The PropertyInterpolator handles type-specific interpolation for each animatable CSS property:

```rust
pub struct PropertyInterpolator;

impl PropertyInterpolator {
    pub fn interpolate(
        &self,
        property: AnimatableProperty,
        from: &CssValue,
        to: &CssValue,
        progress: f64,
    ) -> CssValue {
        match property {
            // Numeric properties: linear interpolation
            AnimatableProperty::Opacity => {
                let a = from.as_f32();
                let b = to.as_f32();
                CssValue::Float(a + (b - a) * progress as f32)
            }
            // Length properties
            AnimatableProperty::Width
            | AnimatableProperty::Height
            | AnimatableProperty::MarginTop /* ... */ => {
                let a = from.as_length();
                let b = to.as_length();
                CssValue::Length(a.lerp(&b, progress))
            }
            // Color: interpolation in Oklab
            AnimatableProperty::Color
            | AnimatableProperty::BackgroundColor => {
                let a = from.as_color().to_oklab();
                let b = to.as_color().to_oklab();
                CssValue::Color(Color::from_oklab(a.lerp(&b, progress)))
            }
            // Transform: decompose → interpolate → recompose
            AnimatableProperty::Transform => {
                let a = from.as_transform().decompose();
                let b = to.as_transform().decompose();
                CssValue::Transform(a.interpolate(&b, progress).recompose())
            }
            // Discrete properties: flip at 50%
            AnimatableProperty::Display
            | AnimatableProperty::Visibility => {
                if progress < 0.5 { from.clone() } else { to.clone() }
            }
            // ...
        }
    }
}
```

Color interpolation uses Oklab color space by default (CSS Color Level 4), providing perceptually uniform transitions. Transform interpolation uses the decompose-interpolate-recompose approach specified by the CSS Transforms Level 2 specification.

### 17.4.3 Animation Composition

When multiple animations target the same property on the same element, they are composed according to Web Animations composite ordering:

```rust
pub enum CompositeOperation {
    /// Output replaces underlying value
    Replace,
    /// Output is added to underlying value
    Add,
    /// Output accumulates on underlying value
    Accumulate,
}
```

The AnimationEngine processes animations in composite order (CSS Transitions < CSS Animations < WAAPI, then by creation time) and applies the composition stack:

```rust
fn compose_animations(
    anims: &[AnimationInstance],
    base_value: &CssValue,
    property: AnimatableProperty,
    interpolator: &PropertyInterpolator,
) -> CssValue {
    let mut result = base_value.clone();

    // Sorted by composite priority: transitions, then CSS animations, then WAAPI
    for anim in anims.iter().filter(|a| a.target_property == property) {
        let sampled = anim.sample(interpolator);
        match anim.composite {
            CompositeOperation::Replace => result = sampled,
            CompositeOperation::Add => result = property.add(&result, &sampled),
            CompositeOperation::Accumulate => result = property.accumulate(&result, &sampled),
        }
    }

    result
}
```

### 17.4.4 Timing Functions

```rust
pub enum TimingFunction {
    Linear,
    CubicBezier(f64, f64, f64, f64),  // control points
    Steps(u32, StepPosition),
    // CSS Easing Level 2
    LinearFunction(Vec<LinearStop>),  // linear() with stops
}
```

Built-in named easings (`ease`, `ease-in`, `ease-out`, `ease-in-out`) are pre-defined `CubicBezier` values. The cubic bezier solver uses Newton-Raphson iteration with a tolerance of 1e-7.

## 17.5 Compositor Promotion

### 17.5.1 Promotion Decision

Not all animations can run on the compositor. The AnimationEngine evaluates each animation at creation time and continuously during playback:

```rust
pub fn can_promote_to_compositor(anim: &AnimationInstance) -> PromotionEligibility {
    // Only certain properties can be composited (Ch. 15 §15.9.1)
    if !anim.target_property.is_compositor_animatable() {
        return PromotionEligibility::Ineligible(Reason::NonCompositableProperty);
    }

    // Animations with JS-observable side effects stay on main thread
    if anim.has_event_listeners_requiring_main_thread() {
        return PromotionEligibility::Eligible { priority: Priority::Low };
    }

    // Complex keyframes (more than 2, or with composite: add/accumulate)
    // may not be worth promoting due to compositor complexity limits
    if anim.keyframes.frames.len() > 8 {
        return PromotionEligibility::Eligible { priority: Priority::Low };
    }

    PromotionEligibility::Eligible { priority: Priority::High }
}
```

Compositor-animatable properties (from Ch. 15 §15.9.1): `transform`, `opacity`, `filter` (subset).

### 17.5.2 Promotion Flow

```
AnimationEngine detects promotable animation
  │
  ├─ Ensure target element has its own layer (Ch. 15 §15.4.2)
  │   └─ If not promoted: request layer promotion from PaintSystem
  │
  ├─ Build CompositorAnimationUpdate
  │   ├─ Target layer ID
  │   ├─ Keyframes (pre-interpolated to compositor-friendly format)
  │   ├─ Timing (duration, delay, easing, iterations)
  │   └─ Current time offset
  │
  ├─ Send to compositor via CompositorChannel
  │
  ├─ Mark animation as compositor_promoted = true
  │   └─ Main thread stops sampling this animation
  │
  └─ Compositor runs animation independently (Ch. 15 §15.9.1)
```

### 17.5.3 Demotion (Fall Back to Main Thread)

Animations may return to the main thread if:

| Trigger | Action |
| --- | --- |
| JS modifies animated property (`element.style.transform = ...`) | Cancel compositor animation, main thread takes over |
| Animation is paused via JS (`animation.pause()`) | Transfer current state back, main thread pauses |
| Element gains a non-compositable animation on the same property | Both animations fall back to main thread for correct composition |
| Layer budget exceeded (Ch. 15 §15.4.3) | Layer de-promoted, animation falls back |

```rust
pub struct CompositorDemotion {
    pub layer_id: LayerId,
    pub animation_id: AnimationId,
    /// Compositor reports its current interpolated value so main thread
    /// can resume without a visible jump.
    pub current_value: CssValue,
    pub current_time: f64,
}
```

## 17.6 Animation Events

The Web Animations specification requires firing events at specific animation lifecycle points:

| Origin | Event | When |
| --- | --- | --- |
| CSS Transition | `transitionrun` | Transition created (delay starts) |
| CSS Transition | `transitionstart` | Active phase begins (delay ends) |
| CSS Transition | `transitionend` | Active phase ends |
| CSS Transition | `transitioncancel` | Transition cancelled (element removed, property overridden) |
| CSS Animation | `animationstart` | First iteration begins |
| CSS Animation | `animationiteration` | New iteration begins (not first) |
| CSS Animation | `animationend` | All iterations complete |
| CSS Animation | `animationcancel` | Animation cancelled |
| WAAPI | `finish` | Promise resolves / `onfinish` callback |
| WAAPI | `cancel` | Promise rejects / `oncancel` callback |

Events are queued during the AnimationEngine tick and dispatched after the tick completes (not during sampling). This ensures consistent state: all animations are sampled before any event handler runs and potentially mutates the DOM.

```rust
pub struct AnimationEventQueue {
    events: Vec<PendingAnimationEvent>,
}

pub struct PendingAnimationEvent {
    pub target: EntityId,
    pub event_type: AnimationEventType,
    pub elapsed_time: f64,
    pub animation_name: Option<AnimationName>,  // for CSS Animation events
    pub property_name: Option<CssPropertyId>,   // for CSS Transition events
}
```

## 17.7 Scroll Animations

### 17.7.1 Smooth Scrolling

Smooth scrolling is implemented as an animation on the compositor's scroll offset:

```rust
pub enum ScrollBehavior {
    /// Instant jump to target position
    Instant,
    /// Animated scroll with easing
    Smooth {
        duration: Duration,
        easing: TimingFunction,
    },
}
```

Sources of smooth scroll requests:

| Source | Trigger |
| --- | --- |
| CSS `scroll-behavior: smooth` | User click on anchor link, `scrollTo()` without explicit `behavior` |
| JS `element.scrollTo({ behavior: 'smooth' })` | Explicit programmatic smooth scroll |
| JS `element.scrollIntoView({ behavior: 'smooth' })` | Scroll element into viewport |
| User input | Keyboard scroll (Page Up/Down, Space) when `scroll-behavior: smooth` is set |

Smooth scroll animations are compositor-driven (Ch. 15 §15.9.2). The main thread sends a `SmoothScrollRequest` to the compositor:

```rust
pub struct SmoothScrollRequest {
    pub target_layer: LayerId,
    pub from_offset: ScrollOffset,
    pub to_offset: ScrollOffset,
    pub duration: Duration,
    pub easing: TimingFunction,
}
```

The compositor interpolates the scroll offset per-frame. If the user provides new scroll input during a smooth scroll, the animation is replaced (new target from current position).

### 17.7.2 Scroll Snap

Scroll snap points (CSS `scroll-snap-type`, `scroll-snap-align`) are applied during scroll deceleration:

```rust
pub struct ScrollSnapConfig {
    pub snap_type: ScrollSnapType,  // none, x, y, both, block, inline
    pub strictness: ScrollSnapStrictness,  // mandatory, proximity
    pub snap_points: Vec<SnapPoint>,
}

pub struct SnapPoint {
    pub position: f64,
    pub alignment: SnapAlignment,  // start, center, end
}
```

The compositor evaluates snap points when:
- User scroll input ends (finger lift, mouse wheel stops)
- Smooth scroll animation completes
- Programmatic scroll targets a non-snap position with `mandatory` snap

For `proximity`, the compositor snaps only if the natural resting position is within a threshold of a snap point. For `mandatory`, the compositor always snaps to the nearest snap point.

### 17.7.3 Scroll Anchoring

Scroll anchoring (`overflow-anchor`) prevents visible content jumps when elements above the viewport change size (e.g., lazy-loaded images, dynamic content insertion):

```
Before layout:
  ┌─────────────┐ ← viewport top
  │  Article A   │
  │  (100px)     │ ← anchor node
  │  Article B   │
  └─────────────┘

Image loads above viewport, pushing content down 200px:

Without anchoring:          With anchoring:
  ┌─────────────┐            ┌─────────────┐
  │  (new image) │            │  (new image) │
  │  Article A   │            │              │ ← scroll offset adjusted +200px
  │  (100px)     │            ├─────────────┤ ← viewport top
  ├─────────────┤            │  Article A   │
  │  Article B   │ ← shifted │  (100px)     │ ← anchor node stays in place
  └─────────────┘            │  Article B   │
  User sees jump!             └─────────────┘
                              No visible change.
```

Implementation:

```rust
pub struct ScrollAnchor {
    /// The anchor node: the first visible element in the scroll container
    pub anchor_entity: EntityId,
    /// Offset of the anchor node from the scroll container's top, before layout
    pub anchor_offset_before: f64,
}

impl ScrollAnchoringSystem {
    /// Called after layout, before paint. Adjusts scroll offset to compensate.
    pub fn adjust(&self, world: &mut World) {
        for (container, anchor, scroll) in
            world.query::<(Entity, &ScrollAnchor, &mut ScrollOffset)>()
        {
            let layout = world.get::<LayoutBox>(anchor.anchor_entity);
            let offset_after = layout.position.y;
            let delta = offset_after - anchor.anchor_offset_before;

            if delta.abs() > 0.5 {  // sub-pixel threshold
                scroll.y += delta;
            }
        }
    }
}
```

Scroll anchoring is suppressed when `overflow-anchor: none` is set on the scroll container or the anchor candidate.

## 17.8 Scroll-Linked Animations (ScrollTimeline)

CSS Scroll-Linked Animations allow animation progress to be driven by scroll position instead of time:

```rust
pub enum TimelineRef {
    /// Default: time-based DocumentTimeline
    Document,
    /// Progress driven by scroll position
    Scroll(ScrollTimelineConfig),
    /// Progress driven by element's view within scroller
    View(ViewTimelineConfig),
}

pub struct ScrollTimelineConfig {
    pub source: ScrollSource,       // nearest, root, or specific element
    pub axis: ScrollAxis,           // block, inline, x, y
    pub range_start: ScrollOffset,
    pub range_end: ScrollOffset,
}
```

When an animation's timeline is `Scroll`, the AnimationEngine computes progress from the scroll position rather than the DocumentTimeline:

```rust
fn scroll_progress(config: &ScrollTimelineConfig, world: &World) -> f64 {
    let scroll_offset = get_scroll_offset(config.source, config.axis, world);
    let range = config.range_end - config.range_start;
    if range == 0.0 { return 0.0; }
    ((scroll_offset - config.range_start) / range).clamp(0.0, 1.0)
}
```

Scroll-linked animations run on the compositor when the target property is compositor-animatable, providing jank-free parallax, reveal effects, and progress indicators.

## 17.9 AnimationEngine Lifecycle Management

### 17.9.1 Entity Removal

When a DOM element is removed (`despawn`), its `ActiveAnimations` component is destroyed along with the entity. However, the Web Animations spec requires firing cancel events before removal:

```rust
impl AnimationEngine {
    /// Called before entity despawn. Queues cancel events for active animations.
    pub fn on_entity_removing(&mut self, entity: EntityId, world: &World) {
        if let Some(anims) = world.get::<ActiveAnimations>(entity) {
            for anim in &anims.animations {
                if anim.play_state == PlayState::Running
                    || anim.play_state == PlayState::Paused
                {
                    self.event_queue.push(PendingAnimationEvent {
                        target: entity,
                        event_type: match &anim.origin {
                            AnimationOrigin::CssTransition { .. } =>
                                AnimationEventType::TransitionCancel,
                            AnimationOrigin::CssAnimation { .. } =>
                                AnimationEventType::AnimationCancel,
                            AnimationOrigin::WebAnimationsApi =>
                                AnimationEventType::Cancel,
                        },
                        elapsed_time: anim.elapsed_time(),
                        animation_name: anim.animation_name(),
                        property_name: anim.property_name(),
                    });
                }

                // If compositor-promoted, send cancel to compositor
                if anim.compositor_promoted {
                    self.compositor_updates.push(
                        CompositorAnimationUpdate::Cancel {
                            animation_id: anim.id,
                        }
                    );
                }
            }
        }
    }
}
```

### 17.9.2 Style Recalculation Interaction

After the AnimationEngine ticks and writes to `ComputedStyle`, the StyleSystem's subsequent resolve pass must account for animated values. The interaction follows this order:

```
AnimationEngine.tick()
  → writes animated values to ComputedStyle
  → sets AnimationDirtyFlag on affected entities

StyleSystem.resolve()
  → for entities WITH AnimationDirtyFlag:
      base style + animation output = final ComputedStyle
  → for entities WITHOUT flag:
      normal cascade (no animation involvement)
```

This avoids the StyleSystem overwriting animated values during its cascade, while ensuring non-animated properties still resolve normally.

## 17.10 elidex-app Animation

| Aspect | elidex-browser | elidex-app |
| --- | --- | --- |
| FrameProducer driver | Event loop (Ch. 5) | App's own main loop or event loop |
| Default FramePolicy | Vsync | OnDemand (Ch. 15 §15.8) |
| DocumentTimeline | Auto-created per document | Created on World initialization |
| Compositor promotion | Automatic | Automatic (same logic) |
| rAF availability | Yes (HTML spec) | Yes (exposed via elidex-app API) |
| Scroll-linked animations | Full support | Available if scroll containers exist |

```rust
// elidex-app: request animation frame
let app = elidex_app::App::new()
    .frame_policy(FramePolicy::Continuous)
    .build();

// rAF equivalent
app.request_animation_frame(|frame_time| {
    // Update game state
    update_game(frame_time);
});
```

For Continuous-mode apps (games), the FrameProducer runs every vsync. The AnimationEngine ticks every frame, providing smooth interpolation for any ECS-based animations the app defines.
