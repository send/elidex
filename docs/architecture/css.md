# Architecture: CSS (plugins, animation, style resolution)

## CSS Plugin Crates (elidex-css-{box,text,flex,grid,table,float,anim})

- **Plugin architecture**: Each crate implements `CssPropertyHandler` for a group of related CSS properties. Handlers provide `parse()`, `resolve()`, `initial_value()`, `is_inherited()`, `affects_layout()`, and `get_computed()`.
- **elidex-css-box**: `BoxHandler` — display, position, width/height/min/max, margin-*, padding-*, border-*-{width,style,color}, box-sizing, border-radius, opacity, overflow, background-color, content, row-gap, column-gap (33 tests).
- **elidex-css-text**: `TextHandler` — color, font-size/weight/style/family, line-height, text-align/transform, white-space, letter/word-spacing, text-decoration-*, writing-mode, text-orientation, direction, unicode-bidi, list-style-type (36 tests).
- **elidex-css-flex**: `FlexHandler` — flex-direction/wrap, justify-content, align-items/content/self, flex-grow/shrink, flex-basis, order (20 tests).
- **elidex-css-grid**: `GridHandler` — grid-template-columns/rows, grid-auto-flow/columns/rows, grid-column/row-start/end (20 tests).
- **elidex-css-table**: `TableHandler` — border-collapse (inherited), border-spacing-h/v (inherited), table-layout, caption-side (inherited) (12 tests).
- **elidex-css-float**: `FloatHandler` — float, clear, visibility (inherited), vertical-align (10 tests).
- **elidex-css-anim**: `AnimHandler` — transition/animation shorthands + 12 longhands (113 tests). See below.
- **Registration**: Each handler has `register(&mut CssPropertyRegistry)`. All 7 registered in `elidex_shell::create_css_property_registry()`.
- **Dependencies**: All depend on elidex-plugin (traits, CssValue, ComputedStyle). Some depend on elidex-css (color parsing) and cssparser (tokenization).

## elidex-css-anim

- **AnimHandler**: `CssPropertyHandler` impl for 14 properties: `transition` (shorthand), `transition-property/duration/timing-function/delay`, `animation` (shorthand), `animation-name/duration/timing-function/delay/iteration-count/direction/fill-mode/play-state`. None inherited, none affects layout directly.
- **timing.rs**: `TimingFunction` enum — `CubicBezier(x1,y1,x2,y2)`, `Steps(count, StepPosition)`, `Linear`. Cubic-bezier solver: Newton-Raphson with bisection fallback. Named easings: `EASE`, `EASE_IN`, `EASE_OUT`, `EASE_IN_OUT`. `StepPosition`: JumpStart/JumpEnd/JumpNone/JumpBoth.
- **style.rs**: `AnimStyle` ECS component — 12 `Vec` fields for transition/animation property lists. Separate from `ComputedStyle` (only inserted when animation/transition props are set). Types: `TransitionProperty`, `IterationCount`, `AnimationDirection`, `AnimationFillMode`, `PlayState`.
- **parse.rs**: `parse_time()` (s/ms), `parse_timing_function()` (keywords + cubic-bezier() + steps()), `parse_transition_property/shorthand()`, `parse_animation_name/iteration_count/direction/fill_mode/play_state()`. `KeyframesRule`/`Keyframe` types, `parse_keyframes()` for `@keyframes` block parsing.
- **resolve.rs**: `resolve_anim_property()` dispatches by property name into `AnimStyle` fields. Helpers: `resolve_transition_property()`, `resolve_time_list()`, `resolve_timing_function_list()`, `resolve_animation_names()`, etc.
- **interpolate.rs**: `interpolate(from, to, t)` — numeric lerp for Number/Length/Percentage/Time/Color, discrete flip at 50% for keywords/auto. `interpolate_color()` component-wise RGBA. `is_animatable()` checks ~35 animatable property names.
- **instance.rs**: `AnimationInstance` (name, timing, iteration, direction, fill, play state, elapsed, finished). `progress()` computes iteration-aware progress with direction/fill. `TransitionInstance` (property, from/to values, elapsed). `current_value()` returns interpolated value.
- **engine.rs**: `AnimationEngine` — manages running transitions/animations per entity (`HashMap<u64, Vec>`). `tick(dt)` advances all, returns `Vec<(EntityId, AnimationEvent)>` for `transitionend`/`animationend`. `register_keyframes()`, `add_transition()`/`add_animation()`, auto-cleanup on finish.
- **timeline.rs**: `DocumentTimeline` — monotonic time tracker (`current_time`, `advance(dt)`).
- **detection.rs**: `detect_transitions()` — compares old/new computed values against `AnimStyle.transition_property` list, produces `Vec<DetectedTransition>` with CSS list cycling behavior.

## elidex-style (parallel)

- **Feature flag**: `parallel` enables rayon-based sibling parallel style resolution.
- **Strategy**: Cascade (`collect_and_cascade`) runs sequentially (requires `&EcsDom`), then `build_computed_style` runs in parallel across siblings via rayon, then results applied and children recursed sequentially.
- **parallel.rs**: `OwnedPropertyMap`, `to_owned_map()`, `par_resolve_siblings()` (threshold 8), `build_computed_style_owned()`.
- **walk.rs**: `walk_children_parallel()` — 3-phase approach (cascade → parallel resolve → apply+recurse). Falls back to sequential for shadow hosts.
- **Dependencies**: rayon 1 (optional).
