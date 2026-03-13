//! CSS Animations and Transitions property handler plugin.
//!
//! Implements CSS Transitions Level 1 and CSS Animations Level 1
//! as a `CssPropertyHandler` plugin for the elidex browser engine.
//!
//! # Modules
//!
//! - [`timing`] — Timing functions (`cubic-bezier`, `steps`, named easings)
//! - [`style`] — `AnimStyle` ECS component
//! - [`parse`] — Property parsing and `@keyframes` rules
//! - [`resolve`] — Property value resolution
//! - [`interpolate`] — CSS value interpolation
//! - [`instance`] — Animation/transition instance tracking
//! - [`engine`] — Animation engine (tick, event dispatch)
//! - [`timeline`] — Document timeline
//! - [`detection`] — Transition change detection

/// Entity identifier (mirrors `hecs::Entity` as `u64` bits).
pub type EntityId = u64;

pub mod detection;
pub mod engine;
pub mod instance;
pub mod interpolate;
pub mod parse;
pub mod resolve;
pub mod style;
pub mod timeline;
pub mod timing;

use elidex_plugin::{
    ComputedStyle, CssPropertyHandler, CssValue, ParseError, PropertyDeclaration, ResolveContext,
};

/// Maximum number of items in a comma-separated CSS list value to prevent
/// unbounded memory growth from malicious or malformed stylesheets.
pub(crate) const MAX_LIST_ITEMS: usize = 1024;

/// All transition/animation property names handled by this plugin.
const PROPERTY_NAMES: &[&str] = &[
    "transition",
    "transition-property",
    "transition-duration",
    "transition-timing-function",
    "transition-delay",
    "animation",
    "animation-name",
    "animation-duration",
    "animation-timing-function",
    "animation-delay",
    "animation-iteration-count",
    "animation-direction",
    "animation-fill-mode",
    "animation-play-state",
];

/// CSS animation/transition property handler.
#[derive(Clone)]
pub struct AnimHandler;

impl AnimHandler {
    /// Register this handler in a CSS property registry.
    pub fn register(registry: &mut elidex_plugin::CssPropertyRegistry) {
        elidex_plugin::register_css_handler(registry, Self);
    }
}

impl CssPropertyHandler for AnimHandler {
    fn property_names(&self) -> &[&str] {
        PROPERTY_NAMES
    }

    fn parse(
        &self,
        name: &str,
        input: &mut cssparser::Parser<'_, '_>,
    ) -> Result<Vec<PropertyDeclaration>, ParseError> {
        match name {
            "transition" => parse::parse_transition_shorthand(input),
            "transition-property" => parse::parse_transition_property(input),
            "transition-duration"
            | "transition-delay"
            | "animation-duration"
            | "animation-delay" => parse::parse_time_list(name, input),
            "transition-timing-function" | "animation-timing-function" => {
                parse::parse_timing_function_list(name, input)
            }
            "animation-name" => parse::parse_animation_name(input),
            "animation-iteration-count" => parse::parse_iteration_count(input),
            "animation-direction" => parse::parse_animation_direction(input),
            "animation-fill-mode" => parse::parse_animation_fill_mode(input),
            "animation-play-state" => parse::parse_animation_play_state(input),
            "animation" => Ok(parse_animation_shorthand(input)),
            _ => Ok(vec![]),
        }
    }

    fn resolve(
        &self,
        _name: &str,
        _value: &CssValue,
        _ctx: &ResolveContext,
        _style: &mut ComputedStyle,
    ) {
        // Animation/transition properties are resolved into AnimStyle ECS
        // component, not into ComputedStyle. The actual resolution happens
        // in the style walk phase via resolve::resolve_anim_property().
        // This is intentionally a no-op on ComputedStyle.
    }

    fn initial_value(&self, name: &str) -> CssValue {
        match name {
            "transition-property" => CssValue::String("all".into()),
            "transition-duration"
            | "transition-delay"
            | "animation-duration"
            | "animation-delay" => CssValue::Time(0.0),
            "transition-timing-function" | "animation-timing-function" => {
                CssValue::String("ease".into())
            }
            "animation-name" | "animation-fill-mode" => CssValue::Keyword("none".into()),
            "animation-iteration-count" => CssValue::Number(1.0),
            "animation-direction" => CssValue::Keyword("normal".into()),
            "animation-play-state" => CssValue::Keyword("running".into()),
            _ => CssValue::Initial,
        }
    }

    fn is_inherited(&self, _name: &str) -> bool {
        // No animation/transition properties are inherited.
        false
    }

    fn affects_layout(&self, _name: &str) -> bool {
        // Animation properties don't directly affect layout; the animated
        // property values do, but those are handled by their own handlers.
        false
    }

    fn get_computed(&self, name: &str, _style: &ComputedStyle) -> CssValue {
        // AnimStyle is stored as a separate ECS component, not on ComputedStyle.
        // Return the initial value as a fallback.
        self.initial_value(name)
    }
}

/// Parsed components of a single animation from the `animation` shorthand.
///
/// Also used as the parameter type for [`instance::AnimationInstance::new`]
/// to avoid a 9-parameter constructor.
pub(crate) struct SingleAnimationSpec {
    pub(crate) name: String,
    pub(crate) duration: f32,
    pub(crate) timing_function: timing::TimingFunction,
    pub(crate) delay: f32,
    pub(crate) iteration_count: style::IterationCount,
    pub(crate) direction: style::AnimationDirection,
    pub(crate) fill_mode: style::AnimationFillMode,
    pub(crate) play_state: style::PlayState,
}

/// Parse the `animation` shorthand.
///
/// Syntax: `<duration> || <timing-function> || <delay> || <iteration-count> ||
///          <direction> || <fill-mode> || <play-state> || <name>`
fn parse_animation_shorthand(input: &mut cssparser::Parser<'_, '_>) -> Vec<PropertyDeclaration> {
    let mut specs = Vec::new();

    loop {
        if specs.len() >= MAX_LIST_ITEMS {
            break;
        }
        specs.push(parse_single_animation(input));
        if input.try_parse(cssparser::Parser::expect_comma).is_err() {
            break;
        }
    }

    let names: Vec<String> = specs.iter().map(|s| s.name.clone()).collect();
    let durations: Vec<f32> = specs.iter().map(|s| s.duration).collect();
    let timing_fns: Vec<timing::TimingFunction> =
        specs.iter().map(|s| s.timing_function.clone()).collect();
    let delays: Vec<f32> = specs.iter().map(|s| s.delay).collect();
    let iteration_counts: Vec<style::IterationCount> =
        specs.iter().map(|s| s.iteration_count).collect();
    let directions: Vec<style::AnimationDirection> = specs.iter().map(|s| s.direction).collect();
    let fill_modes: Vec<style::AnimationFillMode> = specs.iter().map(|s| s.fill_mode).collect();
    let play_states: Vec<style::PlayState> = specs.iter().map(|s| s.play_state).collect();

    vec![
        PropertyDeclaration::new("animation-name", CssValue::String(names.join(", "))),
        PropertyDeclaration::new("animation-duration", time_list_value(&durations)),
        PropertyDeclaration::new("animation-timing-function", display_list_value(&timing_fns)),
        PropertyDeclaration::new("animation-delay", time_list_value(&delays)),
        PropertyDeclaration::new(
            "animation-iteration-count",
            display_list_value(&iteration_counts),
        ),
        PropertyDeclaration::new("animation-direction", display_list_value(&directions)),
        PropertyDeclaration::new("animation-fill-mode", display_list_value(&fill_modes)),
        PropertyDeclaration::new("animation-play-state", display_list_value(&play_states)),
    ]
}

/// Build a `CssValue` from a time list (single → `Time`, multiple → `List`).
pub(crate) fn time_list_value(times: &[f32]) -> CssValue {
    let mut list: Vec<CssValue> = times.iter().map(|t| CssValue::Time(*t)).collect();
    if list.len() == 1 {
        // len checked above; pop cannot fail.
        list.pop().expect("len == 1")
    } else {
        CssValue::List(list)
    }
}

/// Serialize a list of `Display` values as a comma-separated string `CssValue`.
pub(crate) fn display_list_value<T: std::fmt::Display>(items: &[T]) -> CssValue {
    CssValue::String(
        items
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", "),
    )
}

/// Maximum number of component values in a single `animation` shorthand entry.
///
/// The `animation` shorthand has 8 longhands (duration, timing-function, delay,
/// iteration-count, direction, fill-mode, play-state, name), so we try to parse
/// at most 8 components per comma-separated entry.
const MAX_ANIMATION_SHORTHAND_COMPONENTS: usize = 8;

// Time/timing-function parsing delegated to `parse::TimeAndTiming::try_consume()`,
// shared with `parse_single_transition` in parse.rs.
fn parse_single_animation(input: &mut cssparser::Parser<'_, '_>) -> SingleAnimationSpec {
    let mut name = "none".to_string();
    let mut tnt = parse::TimeAndTiming::new();
    let mut iteration_count = style::IterationCount::Number(1.0);
    let mut direction = style::AnimationDirection::Normal;
    let mut fill_mode = style::AnimationFillMode::None;
    let mut play_state = style::PlayState::Running;

    for _ in 0..MAX_ANIMATION_SHORTHAND_COMPONENTS {
        // Try timing function and time values via shared helper.
        // Reject negative duration per CSS Animations §3.2.
        if tnt.try_consume(input, true).unwrap_or(false) {
            continue;
        }
        // Try keyword identifiers
        if let Ok(raw_ident) = input.try_parse(|i| -> Result<String, ParseError> {
            let id = i.expect_ident().map_err(|_| ParseError {
                property: "animation".into(),
                input: String::new(),
                message: "expected ident".into(),
            })?;
            // Preserve original casing — animation-name is case-sensitive
            // (CSS Animations Level 1 §3.1). Only lowercase for keyword
            // comparison; the original string is kept for the name.
            Ok(id.as_ref().to_string())
        }) {
            match raw_ident.to_ascii_lowercase().as_str() {
                "infinite" => iteration_count = style::IterationCount::Infinite,
                "normal" => direction = style::AnimationDirection::Normal,
                "reverse" => direction = style::AnimationDirection::Reverse,
                "alternate" => direction = style::AnimationDirection::Alternate,
                "alternate-reverse" => direction = style::AnimationDirection::AlternateReverse,
                "forwards" => fill_mode = style::AnimationFillMode::Forwards,
                "backwards" => fill_mode = style::AnimationFillMode::Backwards,
                "both" => fill_mode = style::AnimationFillMode::Both,
                "running" => play_state = style::PlayState::Running,
                "paused" => play_state = style::PlayState::Paused,
                "none" => name = "none".to_string(),
                // Not a reserved keyword — treat as animation name, preserving case.
                _ => name = raw_ident,
            }
            continue;
        }
        break;
    }

    SingleAnimationSpec {
        name,
        duration: tnt.duration,
        timing_function: tnt.timing_function,
        delay: tnt.delay,
        iteration_count,
        direction,
        fill_mode,
        play_state,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_plugin::CssPropertyHandler;

    #[test]
    fn handler_property_names() {
        let handler = AnimHandler;
        let names = handler.property_names();
        assert!(names.contains(&"transition"));
        assert!(names.contains(&"animation"));
        assert!(names.contains(&"transition-property"));
        assert!(names.contains(&"animation-name"));
        assert_eq!(names.len(), 14);
    }

    #[test]
    fn handler_parse_transition_duration() {
        let handler = AnimHandler;
        let mut pi = cssparser::ParserInput::new("0.3s");
        let mut parser = cssparser::Parser::new(&mut pi);
        let decls = handler.parse("transition-duration", &mut parser).unwrap();
        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].property, "transition-duration");
        assert_eq!(decls[0].value, CssValue::Time(0.3));
    }

    #[test]
    fn handler_parse_animation_name() {
        let handler = AnimHandler;
        let mut pi = cssparser::ParserInput::new("fadeIn");
        let mut parser = cssparser::Parser::new(&mut pi);
        let decls = handler.parse("animation-name", &mut parser).unwrap();
        assert_eq!(decls[0].value, CssValue::String("fadeIn".into()));
    }

    #[test]
    fn handler_initial_values() {
        let handler = AnimHandler;
        assert_eq!(
            handler.initial_value("transition-duration"),
            CssValue::Time(0.0)
        );
        assert_eq!(
            handler.initial_value("animation-name"),
            CssValue::Keyword("none".into())
        );
        assert_eq!(
            handler.initial_value("animation-iteration-count"),
            CssValue::Number(1.0)
        );
    }

    #[test]
    fn handler_not_inherited() {
        let handler = AnimHandler;
        for name in handler.property_names() {
            assert!(
                !handler.is_inherited(name),
                "{name} should not be inherited"
            );
        }
    }

    #[test]
    fn handler_not_affects_layout() {
        let handler = AnimHandler;
        for name in handler.property_names() {
            assert!(!handler.affects_layout(name));
        }
    }

    #[test]
    fn handler_register() {
        let mut registry = elidex_plugin::CssPropertyRegistry::new();
        AnimHandler::register(&mut registry);
        assert!(registry.resolve("transition-duration").is_some());
        assert!(registry.resolve("animation-name").is_some());
    }

    #[test]
    fn parse_transition_shorthand_basic() {
        let handler = AnimHandler;
        let mut pi = cssparser::ParserInput::new("opacity 0.3s ease");
        let mut parser = cssparser::Parser::new(&mut pi);
        let decls = handler.parse("transition", &mut parser).unwrap();
        assert_eq!(decls.len(), 4);
        assert_eq!(decls[0].property, "transition-property");
        assert_eq!(decls[1].property, "transition-duration");
        assert_eq!(decls[2].property, "transition-timing-function");
        assert_eq!(decls[3].property, "transition-delay");
    }

    #[test]
    fn parse_animation_shorthand_basic() {
        let handler = AnimHandler;
        let mut pi = cssparser::ParserInput::new("fadeIn 1s ease-in 0.5s infinite alternate");
        let mut parser = cssparser::Parser::new(&mut pi);
        let decls = handler.parse("animation", &mut parser).unwrap();
        assert_eq!(decls.len(), 8); // 8 longhands
                                    // animation-name is case-sensitive (CSS Animations Level 1 §3.1)
        assert_eq!(decls[0].property, "animation-name");
        assert_eq!(decls[0].value, CssValue::String("fadeIn".into()));
        // animation-duration
        assert_eq!(decls[1].property, "animation-duration");
        assert_eq!(decls[1].value, CssValue::Time(1.0));
    }

    #[test]
    fn parse_animation_shorthand_multiple() {
        let handler = AnimHandler;
        let mut pi = cssparser::ParserInput::new("fadeIn 1s, slideUp 0.5s");
        let mut parser = cssparser::Parser::new(&mut pi);
        let decls = handler.parse("animation", &mut parser).unwrap();
        // animation-name is case-sensitive; original casing must be preserved
        assert_eq!(decls[0].value, CssValue::String("fadeIn, slideUp".into()));
    }
}
