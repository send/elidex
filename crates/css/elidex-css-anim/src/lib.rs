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
pub struct AnimHandler;

impl AnimHandler {
    /// Register this handler in a CSS property registry.
    pub fn register(registry: &mut elidex_plugin::CssPropertyRegistry) {
        for name in Self.property_names() {
            registry.register_static(name, Box::new(Self));
        }
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
            "transition-duration" | "transition-delay" | "animation-duration"
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
            "transition-property" => CssValue::Keyword("all".into()),
            "transition-duration" | "transition-delay" | "animation-duration"
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

/// Parse the `animation` shorthand.
///
/// Syntax: `<duration> || <timing-function> || <delay> || <iteration-count> ||
///          <direction> || <fill-mode> || <play-state> || <name>`
fn parse_animation_shorthand(
    input: &mut cssparser::Parser<'_, '_>,
) -> Vec<PropertyDeclaration> {
    let mut names = Vec::new();
    let mut durations = Vec::new();
    let mut timing_fns = Vec::new();
    let mut delays = Vec::new();
    let mut iteration_counts = Vec::new();
    let mut directions = Vec::new();
    let mut fill_modes = Vec::new();
    let mut play_states = Vec::new();

    loop {
        let (name, dur, tf, del, ic, dir, fm, ps) = parse_single_animation(input);
        names.push(name);
        durations.push(dur);
        timing_fns.push(tf);
        delays.push(del);
        iteration_counts.push(ic);
        directions.push(dir);
        fill_modes.push(fm);
        play_states.push(ps);
        if input.try_parse(cssparser::Parser::expect_comma).is_err() {
            break;
        }
    }

    build_animation_decls(
        &names,
        &durations,
        &timing_fns,
        &delays,
        &iteration_counts,
        &directions,
        &fill_modes,
        &play_states,
    )
}

/// Build longhand declarations from collected animation shorthand components.
#[allow(clippy::too_many_arguments)]
fn build_animation_decls(
    names: &[String],
    durations: &[f32],
    timing_fns: &[timing::TimingFunction],
    delays: &[f32],
    iteration_counts: &[style::IterationCount],
    directions: &[style::AnimationDirection],
    fill_modes: &[style::AnimationFillMode],
    play_states: &[style::PlayState],
) -> Vec<PropertyDeclaration> {
    let mut decls = Vec::new();
    decls.push(PropertyDeclaration::new(
        "animation-name",
        CssValue::String(names.join(", ")),
    ));
    let dur_list: Vec<CssValue> = durations.iter().map(|d| CssValue::Time(*d)).collect();
    decls.push(PropertyDeclaration::new(
        "animation-duration",
        if dur_list.len() == 1 {
            dur_list.into_iter().next().unwrap()
        } else {
            CssValue::List(dur_list)
        },
    ));
    decls.push(PropertyDeclaration::new(
        "animation-timing-function",
        CssValue::String(
            timing_fns
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", "),
        ),
    ));
    let del_list: Vec<CssValue> = delays.iter().map(|d| CssValue::Time(*d)).collect();
    decls.push(PropertyDeclaration::new(
        "animation-delay",
        if del_list.len() == 1 {
            del_list.into_iter().next().unwrap()
        } else {
            CssValue::List(del_list)
        },
    ));
    decls.push(PropertyDeclaration::new(
        "animation-iteration-count",
        CssValue::String(
            iteration_counts
                .iter()
                .map(|ic| match ic {
                    style::IterationCount::Infinite => "infinite".to_string(),
                    style::IterationCount::Number(n) => n.to_string(),
                })
                .collect::<Vec<_>>()
                .join(", "),
        ),
    ));
    decls.push(PropertyDeclaration::new(
        "animation-direction",
        CssValue::String(
            directions
                .iter()
                .map(|d| match d {
                    style::AnimationDirection::Normal => "normal",
                    style::AnimationDirection::Reverse => "reverse",
                    style::AnimationDirection::Alternate => "alternate",
                    style::AnimationDirection::AlternateReverse => "alternate-reverse",
                })
                .collect::<Vec<_>>()
                .join(", "),
        ),
    ));
    decls.push(PropertyDeclaration::new(
        "animation-fill-mode",
        CssValue::String(
            fill_modes
                .iter()
                .map(|fm| match fm {
                    style::AnimationFillMode::None => "none",
                    style::AnimationFillMode::Forwards => "forwards",
                    style::AnimationFillMode::Backwards => "backwards",
                    style::AnimationFillMode::Both => "both",
                })
                .collect::<Vec<_>>()
                .join(", "),
        ),
    ));
    decls.push(PropertyDeclaration::new(
        "animation-play-state",
        CssValue::String(
            play_states
                .iter()
                .map(|ps| match ps {
                    style::PlayState::Running => "running",
                    style::PlayState::Paused => "paused",
                })
                .collect::<Vec<_>>()
                .join(", "),
        ),
    ));
    decls
}

#[allow(clippy::type_complexity)]
fn parse_single_animation(
    input: &mut cssparser::Parser<'_, '_>,
) -> (
    String,
    f32,
    timing::TimingFunction,
    f32,
    style::IterationCount,
    style::AnimationDirection,
    style::AnimationFillMode,
    style::PlayState,
) {
    let mut name = "none".to_string();
    let mut duration = 0.0_f32;
    let mut timing = timing::TimingFunction::EASE;
    let mut delay = 0.0_f32;
    let mut iteration_count = style::IterationCount::Number(1.0);
    let mut direction = style::AnimationDirection::Normal;
    let mut fill_mode = style::AnimationFillMode::None;
    let mut play_state = style::PlayState::Running;
    let mut found_duration = false;

    for _ in 0..8 {
        // Try timing function
        if let Ok(tf) = input.try_parse(parse::parse_timing_function) {
            timing = tf;
            continue;
        }
        // Try time value
        if let Ok(t) = input.try_parse(parse::parse_time) {
            if found_duration {
                delay = t;
            } else {
                duration = t;
                found_duration = true;
            }
            continue;
        }
        // Try keyword identifiers
        if let Ok(ident) = input.try_parse(|i| -> Result<String, ParseError> {
            let id = i.expect_ident().map_err(|_| {
                ParseError {
                    property: "animation".into(),
                    input: String::new(),
                    message: "expected ident".into(),
                }
            })?;
            Ok(id.to_ascii_lowercase())
        }) {
            match ident.as_str() {
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
                other => name = other.to_string(),
            }
            continue;
        }
        break;
    }

    (
        name,
        duration,
        timing,
        delay,
        iteration_count,
        direction,
        fill_mode,
        play_state,
    )
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
            assert!(!handler.is_inherited(name), "{name} should not be inherited");
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
        // animation-name (lowercased by cssparser)
        assert_eq!(decls[0].property, "animation-name");
        assert_eq!(decls[0].value, CssValue::String("fadein".into()));
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
        // cssparser lowercases identifiers
        assert_eq!(decls[0].value, CssValue::String("fadein, slideup".into()));
    }
}
