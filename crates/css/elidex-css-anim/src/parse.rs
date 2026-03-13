//! CSS animation/transition property parsing and `@keyframes` rule parsing.

use crate::style::TransitionProperty;
use crate::timing::{StepPosition, TimingFunction};
use elidex_plugin::{CssValue, ParseError, PropertyDeclaration};

/// Maximum number of items in a comma-separated CSS list value to prevent
/// unbounded memory growth from malicious or malformed stylesheets.
const MAX_LIST_ITEMS: usize = 1024;

/// Parse a `<time>` value from a cssparser token, returning seconds.
pub fn parse_time(input: &mut cssparser::Parser<'_, '_>) -> Result<f32, ParseError> {
    let token = input
        .next()
        .map_err(|_| parse_err("time", "unexpected EOF"))?;
    match *token {
        cssparser::Token::Dimension {
            value, ref unit, ..
        } => {
            let lower = unit.to_ascii_lowercase();
            match lower.as_str() {
                "s" => Ok(value),
                "ms" => Ok(value / 1000.0),
                _ => Err(parse_err("time", &format!("unknown unit: {lower}"))),
            }
        }
        // CSS Values Level 3 §5.1.1: unitless 0 is not valid for <time>,
        // but all major browsers accept it for backwards compatibility.
        cssparser::Token::Number { value: 0.0, .. } => Ok(0.0),
        _ => Err(parse_err("time", "expected time value")),
    }
}

/// Parse a `<timing-function>` value.
pub fn parse_timing_function(
    input: &mut cssparser::Parser<'_, '_>,
) -> Result<TimingFunction, ParseError> {
    // Try keyword
    if let Ok(ident) = input.try_parse(|i| -> Result<String, ()> {
        let id = i.expect_ident().map_err(|_| ())?;
        Ok(id.to_ascii_lowercase())
    }) {
        return match ident.as_str() {
            "linear" => Ok(TimingFunction::Linear),
            "ease" => Ok(TimingFunction::EASE),
            "ease-in" => Ok(TimingFunction::EASE_IN),
            "ease-out" => Ok(TimingFunction::EASE_OUT),
            "ease-in-out" => Ok(TimingFunction::EASE_IN_OUT),
            "step-start" => Ok(TimingFunction::Steps(1, StepPosition::JumpStart)),
            "step-end" => Ok(TimingFunction::Steps(1, StepPosition::JumpEnd)),
            _ => Err(parse_err(
                "timing-function",
                &format!("unknown keyword: {ident}"),
            )),
        };
    }

    // Try cubic-bezier()
    if let Ok(tf) = input.try_parse(|i| -> Result<TimingFunction, ()> {
        i.expect_function_matching("cubic-bezier").map_err(|_| ())?;
        i.parse_nested_block(|args| {
            parse_cubic_bezier_args(args).map_err(|()| args.new_custom_error(()))
        })
        .map_err(|_: cssparser::ParseError<'_, ()>| ())
    }) {
        return Ok(tf);
    }

    // Try steps()
    if let Ok(tf) = input.try_parse(|i| -> Result<TimingFunction, ()> {
        i.expect_function_matching("steps").map_err(|_| ())?;
        i.parse_nested_block(|args| parse_steps_args(args).map_err(|()| args.new_custom_error(())))
            .map_err(|_: cssparser::ParseError<'_, ()>| ())
    }) {
        return Ok(tf);
    }

    Err(parse_err("timing-function", "expected timing function"))
}

/// Parse `transition-property` value.
pub fn parse_transition_property(
    input: &mut cssparser::Parser<'_, '_>,
) -> Result<Vec<PropertyDeclaration>, ParseError> {
    let mut props = Vec::new();
    loop {
        if props.len() >= MAX_LIST_ITEMS {
            break;
        }
        let ident = input
            .expect_ident()
            .map_err(|_| parse_err("transition-property", "expected identifier"))?;
        let lower = ident.to_ascii_lowercase();
        let tp = match lower.as_str() {
            "none" => TransitionProperty::None,
            "all" => TransitionProperty::All,
            _ => TransitionProperty::Property(lower),
        };
        props.push(tp);
        if input.try_parse(cssparser::Parser::expect_comma).is_err() {
            break;
        }
    }
    // Serialize as a keyword list using CssValue::String for transport
    let serialized = props
        .iter()
        .map(|p| match p {
            TransitionProperty::None => "none".to_string(),
            TransitionProperty::All => "all".to_string(),
            TransitionProperty::Property(name) => name.clone(),
        })
        .collect::<Vec<_>>()
        .join(", ");
    Ok(vec![PropertyDeclaration::new(
        "transition-property",
        CssValue::String(serialized),
    )])
}

/// Parse `transition-duration` or `transition-delay` (comma-separated `<time>`).
pub fn parse_time_list(
    name: &str,
    input: &mut cssparser::Parser<'_, '_>,
) -> Result<Vec<PropertyDeclaration>, ParseError> {
    let mut values = Vec::new();
    loop {
        if values.len() >= MAX_LIST_ITEMS {
            break;
        }
        values.push(parse_time(input)?);
        if input.try_parse(cssparser::Parser::expect_comma).is_err() {
            break;
        }
    }
    // Serialize as comma-separated CssValue::List of Time values
    let list: Vec<CssValue> = values.iter().map(|t| CssValue::Time(*t)).collect();
    Ok(vec![PropertyDeclaration::new(
        name,
        if list.len() == 1 {
            list.into_iter().next().unwrap()
        } else {
            CssValue::List(list)
        },
    )])
}

/// Parse `transition-timing-function` or `animation-timing-function` list.
pub fn parse_timing_function_list(
    name: &str,
    input: &mut cssparser::Parser<'_, '_>,
) -> Result<Vec<PropertyDeclaration>, ParseError> {
    let mut fns = Vec::new();
    loop {
        if fns.len() >= MAX_LIST_ITEMS {
            break;
        }
        fns.push(parse_timing_function(input)?);
        if input.try_parse(cssparser::Parser::expect_comma).is_err() {
            break;
        }
    }
    // Serialize as string
    let serialized = fns
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    Ok(vec![PropertyDeclaration::new(
        name,
        CssValue::String(serialized),
    )])
}

/// Parse `animation-name` (comma-separated identifiers or `none`).
pub fn parse_animation_name(
    input: &mut cssparser::Parser<'_, '_>,
) -> Result<Vec<PropertyDeclaration>, ParseError> {
    let mut names = Vec::new();
    loop {
        if names.len() >= MAX_LIST_ITEMS {
            break;
        }
        let ident = input
            .expect_ident()
            .map_err(|_| parse_err("animation-name", "expected identifier"))?;
        names.push(ident.to_string());
        if input.try_parse(cssparser::Parser::expect_comma).is_err() {
            break;
        }
    }
    let serialized = names.join(", ");
    Ok(vec![PropertyDeclaration::new(
        "animation-name",
        CssValue::String(serialized),
    )])
}

/// Parse `animation-iteration-count`.
pub fn parse_iteration_count(
    input: &mut cssparser::Parser<'_, '_>,
) -> Result<Vec<PropertyDeclaration>, ParseError> {
    let mut counts = Vec::new();
    loop {
        if counts.len() >= MAX_LIST_ITEMS {
            break;
        }
        if input
            .try_parse(|i| i.expect_ident_matching("infinite"))
            .is_ok()
        {
            counts.push("infinite".to_string());
        } else {
            let n = input.expect_number().map_err(|_| {
                parse_err("animation-iteration-count", "expected number or infinite")
            })?;
            if n < 0.0 {
                return Err(parse_err("animation-iteration-count", "negative value"));
            }
            counts.push(n.to_string());
        }
        if input.try_parse(cssparser::Parser::expect_comma).is_err() {
            break;
        }
    }
    let serialized = counts.join(", ");
    Ok(vec![PropertyDeclaration::new(
        "animation-iteration-count",
        CssValue::String(serialized),
    )])
}

/// Parse `animation-direction` (comma-separated keywords).
pub fn parse_animation_direction(
    input: &mut cssparser::Parser<'_, '_>,
) -> Result<Vec<PropertyDeclaration>, ParseError> {
    parse_keyword_list(
        "animation-direction",
        input,
        &["normal", "reverse", "alternate", "alternate-reverse"],
    )
}

/// Parse `animation-fill-mode` (comma-separated keywords).
pub fn parse_animation_fill_mode(
    input: &mut cssparser::Parser<'_, '_>,
) -> Result<Vec<PropertyDeclaration>, ParseError> {
    parse_keyword_list(
        "animation-fill-mode",
        input,
        &["none", "forwards", "backwards", "both"],
    )
}

/// Parse `animation-play-state` (comma-separated keywords).
pub fn parse_animation_play_state(
    input: &mut cssparser::Parser<'_, '_>,
) -> Result<Vec<PropertyDeclaration>, ParseError> {
    parse_keyword_list("animation-play-state", input, &["running", "paused"])
}

fn parse_keyword_list(
    name: &str,
    input: &mut cssparser::Parser<'_, '_>,
    allowed: &[&str],
) -> Result<Vec<PropertyDeclaration>, ParseError> {
    let mut values = Vec::new();
    loop {
        if values.len() >= MAX_LIST_ITEMS {
            break;
        }
        let ident = input
            .expect_ident()
            .map_err(|_| parse_err(name, "expected keyword"))?;
        let lower = ident.to_ascii_lowercase();
        if !allowed.contains(&lower.as_str()) {
            return Err(parse_err(name, &format!("unexpected keyword: {lower}")));
        }
        values.push(lower);
        if input.try_parse(cssparser::Parser::expect_comma).is_err() {
            break;
        }
    }
    let serialized = values.join(", ");
    Ok(vec![PropertyDeclaration::new(
        name,
        CssValue::String(serialized),
    )])
}

// --- @keyframes parsing ---

/// A single `@keyframes` rule.
#[derive(Clone, Debug, PartialEq)]
pub struct KeyframesRule {
    /// The animation name.
    pub name: String,
    /// Keyframe blocks sorted by offset (0.0..=1.0).
    pub keyframes: Vec<Keyframe>,
}

/// A single keyframe within a `@keyframes` rule.
#[derive(Clone, Debug, PartialEq)]
pub struct Keyframe {
    /// Offset as a fraction (0.0 = `from`/`0%`, 1.0 = `to`/`100%`).
    pub offset: f32,
    /// Property declarations at this keyframe.
    pub declarations: Vec<PropertyDeclaration>,
}

/// Parse `@keyframes <name> { ... }` rule body.
///
/// The `name` should already be extracted by the at-rule parser.
/// `block_text` is the content between `{` and `}`.
/// Maximum number of keyframe blocks parsed from a single `@keyframes` rule.
const MAX_KEYFRAMES: usize = 1000;

pub fn parse_keyframes(name: &str, block_text: &str) -> KeyframesRule {
    let mut keyframes = Vec::new();
    let mut remaining = block_text.trim();
    let mut count = 0;

    while !remaining.is_empty() {
        count += 1;
        if count > MAX_KEYFRAMES {
            break;
        }
        // Parse selector (offset list)
        let Some(brace_start) = remaining.find('{') else {
            break;
        };
        let selector_text = remaining[..brace_start].trim();
        let offsets = parse_keyframe_selectors(selector_text);

        // Find matching closing brace (handles nested braces)
        let after_brace = &remaining[brace_start + 1..];
        let Some(brace_end) = find_matching_brace(after_brace) else {
            break;
        };
        let decl_text = &after_brace[..brace_end];

        // Parse declarations (simplified — just property: value pairs)
        let declarations = parse_keyframe_declarations(decl_text);

        for offset in offsets {
            keyframes.push(Keyframe {
                offset,
                declarations: declarations.clone(),
            });
        }

        remaining = after_brace[brace_end + 1..].trim();
    }

    keyframes.sort_by(|a, b| {
        a.offset
            .partial_cmp(&b.offset)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // CSS Animations Level 1 §4.2: if 0% (from) or 100% (to) keyframes
    // are missing, synthesize them with empty declarations so the animation
    // engine interpolates from/to the element's computed values.
    if !keyframes.iter().any(|k| k.offset == 0.0) {
        keyframes.insert(
            0,
            Keyframe {
                offset: 0.0,
                declarations: Vec::new(),
            },
        );
    }
    if !keyframes
        .iter()
        .any(|k| (k.offset - 1.0).abs() < f32::EPSILON)
    {
        keyframes.push(Keyframe {
            offset: 1.0,
            declarations: Vec::new(),
        });
    }

    KeyframesRule {
        name: name.to_string(),
        keyframes,
    }
}

/// Find the position of the matching `}` for an opening `{`, handling nesting.
///
/// Returns `None` if no matching brace is found or nesting exceeds 128 levels.
fn find_matching_brace(text: &str) -> Option<usize> {
    /// Maximum nesting depth for braces in `@keyframes` blocks.
    const MAX_BRACE_DEPTH: u32 = 128;
    let mut depth = 0u32;
    for (i, ch) in text.char_indices() {
        match ch {
            '{' => {
                depth += 1;
                if depth > MAX_BRACE_DEPTH {
                    return None;
                }
            }
            '}' => {
                if depth == 0 {
                    return Some(i);
                }
                depth -= 1;
            }
            _ => {}
        }
    }
    None
}

fn parse_keyframe_selectors(text: &str) -> Vec<f32> {
    text.split(',')
        .filter_map(|s| {
            let s = s.trim();
            if s.eq_ignore_ascii_case("from") {
                Some(0.0)
            } else if s.eq_ignore_ascii_case("to") {
                Some(1.0)
            } else if let Some(pct) = s.strip_suffix('%') {
                pct.trim().parse::<f32>().ok().map(|v| v / 100.0)
            } else {
                None
            }
        })
        .collect()
}

fn parse_keyframe_declarations(text: &str) -> Vec<PropertyDeclaration> {
    let mut decls = Vec::new();
    for part in text.split(';') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some((prop, val)) = part.split_once(':') {
            let property = prop.trim().to_ascii_lowercase();
            // CSS Animations Level 1 §4.1: !important in @keyframes is ignored.
            let value_str = val.trim().trim_end_matches("!important").trim();
            // Parse value using cssparser
            let mut pi = cssparser::ParserInput::new(value_str);
            let mut parser = cssparser::Parser::new(&mut pi);
            if let Ok(css_value) = parse_simple_value(&mut parser) {
                decls.push(PropertyDeclaration::new(property, css_value));
            }
        }
    }
    decls
}

/// Try to parse the current parser position as a CSS color using `elidex_css::parse_color`.
///
/// Returns `Some(CssValue::Color(...))` on success, `None` if not a color token.
fn try_parse_color(input: &mut cssparser::Parser<'_, '_>) -> Option<CssValue> {
    input
        .try_parse(elidex_css::parse_color)
        .ok()
        .map(CssValue::Color)
}

/// Parse a simple CSS value (number, length, percentage, color, keyword).
///
/// TODO: Consider delegating to elidex-css value parsing for full CSS value
/// support in keyframes.
fn parse_simple_value(input: &mut cssparser::Parser<'_, '_>) -> Result<CssValue, ()> {
    // Try CSS color parsing first (covers named colors, #hex, rgb(), hsl(), etc.)
    if let Some(color_val) = try_parse_color(input) {
        return Ok(color_val);
    }

    let token = input.next().map_err(|_| ())?;
    match token.clone() {
        cssparser::Token::Number { value, .. } => Ok(CssValue::Number(value)),
        cssparser::Token::Dimension {
            value, ref unit, ..
        } => {
            let u = elidex_plugin::css_resolve::parse_length_unit(unit);
            Ok(CssValue::Length(value, u))
        }
        cssparser::Token::Percentage { unit_value, .. } => {
            Ok(CssValue::Percentage(unit_value * 100.0))
        }
        cssparser::Token::Ident(ref ident) => {
            let lower = ident.to_ascii_lowercase();
            match lower.as_str() {
                "none" => Ok(CssValue::Keyword("none".into())),
                "auto" => Ok(CssValue::Auto),
                "inherit" => Ok(CssValue::Inherit),
                "initial" => Ok(CssValue::Initial),
                _ => Ok(CssValue::Keyword(lower)),
            }
        }
        _ => Err(()),
    }
}

fn parse_cubic_bezier_args(args: &mut cssparser::Parser<'_, '_>) -> Result<TimingFunction, ()> {
    let x1 = args.expect_number().map_err(|_| ())?;
    args.expect_comma().map_err(|_| ())?;
    let y1 = args.expect_number().map_err(|_| ())?;
    args.expect_comma().map_err(|_| ())?;
    let x2 = args.expect_number().map_err(|_| ())?;
    args.expect_comma().map_err(|_| ())?;
    let y2 = args.expect_number().map_err(|_| ())?;
    if !(0.0..=1.0).contains(&x1) || !(0.0..=1.0).contains(&x2) {
        return Err(());
    }
    Ok(TimingFunction::CubicBezier(x1, y1, x2, y2))
}

fn parse_steps_args(args: &mut cssparser::Parser<'_, '_>) -> Result<TimingFunction, ()> {
    let count = args.expect_integer().map_err(|_| ())?;
    if count < 1 {
        return Err(());
    }
    // Per CSS Easing Functions Level 2, steps(n, jump-none) requires n >= 2.
    let position = if args.try_parse(cssparser::Parser::expect_comma).is_ok() {
        let ident = args.expect_ident().map_err(|_| ())?;
        match ident.to_ascii_lowercase().as_str() {
            "start" | "jump-start" => StepPosition::JumpStart,
            "end" | "jump-end" => StepPosition::JumpEnd,
            "jump-none" => StepPosition::JumpNone,
            "jump-both" => StepPosition::JumpBoth,
            _ => return Err(()),
        }
    } else {
        StepPosition::JumpEnd
    };
    // jump-none requires at least 2 steps (n-1 intervals must be >= 1).
    if position == StepPosition::JumpNone && count < 2 {
        return Err(());
    }
    #[allow(clippy::cast_sign_loss)]
    Ok(TimingFunction::Steps(count as u32, position))
}

fn parse_err(property: &str, message: &str) -> ParseError {
    ParseError {
        property: property.into(),
        input: String::new(),
        message: message.into(),
    }
}

/// Parse the `transition` shorthand into longhand declarations.
///
/// Syntax: `<property> || <duration> || <timing-function> || <delay>`
/// Multiple transitions separated by commas.
pub fn parse_transition_shorthand(
    input: &mut cssparser::Parser<'_, '_>,
) -> Result<Vec<PropertyDeclaration>, ParseError> {
    let mut properties = Vec::new();
    let mut durations = Vec::new();
    let mut timing_fns = Vec::new();
    let mut delays = Vec::new();

    loop {
        let (prop, dur, tf, del) = parse_single_transition(input);
        properties.push(prop);
        durations.push(dur);
        timing_fns.push(tf);
        delays.push(del);
        if input.try_parse(cssparser::Parser::expect_comma).is_err() {
            break;
        }
    }

    Ok(vec![
        PropertyDeclaration::new(
            "transition-property",
            CssValue::String(properties.join(", ")),
        ),
        PropertyDeclaration::new("transition-duration", crate::time_list_value(&durations)),
        PropertyDeclaration::new(
            "transition-timing-function",
            crate::display_list_value(&timing_fns),
        ),
        PropertyDeclaration::new("transition-delay", crate::time_list_value(&delays)),
    ])
}

fn parse_single_transition(
    input: &mut cssparser::Parser<'_, '_>,
) -> (String, f32, TimingFunction, f32) {
    let mut property = "all".to_string();
    let mut duration = 0.0_f32;
    let mut timing = TimingFunction::EASE;
    let mut delay = 0.0_f32;
    let mut found_duration = false;

    // Up to 4 components in any order
    for _ in 0..4 {
        // Try timing function first (before ident, since it can start with an ident like "ease")
        if let Ok(tf) = input.try_parse(parse_timing_function) {
            timing = tf;
            continue;
        }
        // Try time value
        if let Ok(t) = input.try_parse(parse_time) {
            if found_duration {
                delay = t;
            } else {
                duration = t;
                found_duration = true;
            }
            continue;
        }
        // Try property ident
        if let Ok(ident) = input.try_parse(|i| -> Result<String, ParseError> {
            let id = i
                .expect_ident()
                .map_err(|_| parse_err("transition", "expected ident"))?;
            Ok(id.to_ascii_lowercase())
        }) {
            property = ident;
            continue;
        }
        break;
    }

    (property, duration, timing, delay)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_time_seconds() {
        let mut pi = cssparser::ParserInput::new("0.3s");
        let mut parser = cssparser::Parser::new(&mut pi);
        assert_eq!(parse_time(&mut parser).unwrap(), 0.3);
    }

    #[test]
    fn parse_time_milliseconds() {
        let mut pi = cssparser::ParserInput::new("200ms");
        let mut parser = cssparser::Parser::new(&mut pi);
        assert_eq!(parse_time(&mut parser).unwrap(), 0.2);
    }

    #[test]
    fn parse_time_zero() {
        let mut pi = cssparser::ParserInput::new("0");
        let mut parser = cssparser::Parser::new(&mut pi);
        assert_eq!(parse_time(&mut parser).unwrap(), 0.0);
    }

    #[test]
    fn parse_timing_ease() {
        let mut pi = cssparser::ParserInput::new("ease");
        let mut parser = cssparser::Parser::new(&mut pi);
        assert_eq!(
            parse_timing_function(&mut parser).unwrap(),
            TimingFunction::EASE
        );
    }

    #[test]
    fn parse_timing_linear() {
        let mut pi = cssparser::ParserInput::new("linear");
        let mut parser = cssparser::Parser::new(&mut pi);
        assert_eq!(
            parse_timing_function(&mut parser).unwrap(),
            TimingFunction::Linear
        );
    }

    #[test]
    fn parse_timing_cubic_bezier() {
        let mut pi = cssparser::ParserInput::new("cubic-bezier(0.1, 0.2, 0.3, 0.4)");
        let mut parser = cssparser::Parser::new(&mut pi);
        assert_eq!(
            parse_timing_function(&mut parser).unwrap(),
            TimingFunction::CubicBezier(0.1, 0.2, 0.3, 0.4)
        );
    }

    #[test]
    fn parse_timing_cubic_bezier_out_of_range() {
        // x1 must be in [0, 1]
        let mut pi = cssparser::ParserInput::new("cubic-bezier(1.5, 0.0, 0.5, 1.0)");
        let mut parser = cssparser::Parser::new(&mut pi);
        assert!(parse_timing_function(&mut parser).is_err());
    }

    #[test]
    fn parse_timing_steps() {
        let mut pi = cssparser::ParserInput::new("steps(4, start)");
        let mut parser = cssparser::Parser::new(&mut pi);
        assert_eq!(
            parse_timing_function(&mut parser).unwrap(),
            TimingFunction::Steps(4, StepPosition::JumpStart)
        );
    }

    #[test]
    fn parse_timing_steps_default_end() {
        let mut pi = cssparser::ParserInput::new("steps(3)");
        let mut parser = cssparser::Parser::new(&mut pi);
        assert_eq!(
            parse_timing_function(&mut parser).unwrap(),
            TimingFunction::Steps(3, StepPosition::JumpEnd)
        );
    }

    #[test]
    fn parse_timing_step_start() {
        let mut pi = cssparser::ParserInput::new("step-start");
        let mut parser = cssparser::Parser::new(&mut pi);
        assert_eq!(
            parse_timing_function(&mut parser).unwrap(),
            TimingFunction::Steps(1, StepPosition::JumpStart)
        );
    }

    #[test]
    fn parse_transition_property_single() {
        let mut pi = cssparser::ParserInput::new("opacity");
        let mut parser = cssparser::Parser::new(&mut pi);
        let decls = parse_transition_property(&mut parser).unwrap();
        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].value, CssValue::String("opacity".into()));
    }

    #[test]
    fn parse_transition_property_all() {
        let mut pi = cssparser::ParserInput::new("all");
        let mut parser = cssparser::Parser::new(&mut pi);
        let decls = parse_transition_property(&mut parser).unwrap();
        assert_eq!(decls[0].value, CssValue::String("all".into()));
    }

    #[test]
    fn parse_transition_property_multiple() {
        let mut pi = cssparser::ParserInput::new("opacity, width, color");
        let mut parser = cssparser::Parser::new(&mut pi);
        let decls = parse_transition_property(&mut parser).unwrap();
        assert_eq!(
            decls[0].value,
            CssValue::String("opacity, width, color".into())
        );
    }

    #[test]
    fn parse_time_list_single() {
        let mut pi = cssparser::ParserInput::new("0.3s");
        let mut parser = cssparser::Parser::new(&mut pi);
        let decls = parse_time_list("transition-duration", &mut parser).unwrap();
        assert_eq!(decls[0].value, CssValue::Time(0.3));
    }

    #[test]
    fn parse_time_list_multiple() {
        let mut pi = cssparser::ParserInput::new("0.3s, 0.5s");
        let mut parser = cssparser::Parser::new(&mut pi);
        let decls = parse_time_list("transition-duration", &mut parser).unwrap();
        match &decls[0].value {
            CssValue::List(items) => {
                assert_eq!(items.len(), 2);
                assert_eq!(items[0], CssValue::Time(0.3));
                assert_eq!(items[1], CssValue::Time(0.5));
            }
            _ => panic!("expected List"),
        }
    }

    #[test]
    fn parse_animation_name_single() {
        let mut pi = cssparser::ParserInput::new("fadeIn");
        let mut parser = cssparser::Parser::new(&mut pi);
        let decls = parse_animation_name(&mut parser).unwrap();
        assert_eq!(decls[0].value, CssValue::String("fadeIn".into()));
    }

    #[test]
    fn parse_animation_name_none() {
        let mut pi = cssparser::ParserInput::new("none");
        let mut parser = cssparser::Parser::new(&mut pi);
        let decls = parse_animation_name(&mut parser).unwrap();
        assert_eq!(decls[0].value, CssValue::String("none".into()));
    }

    #[test]
    fn parse_iteration_count_number() {
        let mut pi = cssparser::ParserInput::new("3");
        let mut parser = cssparser::Parser::new(&mut pi);
        let decls = parse_iteration_count(&mut parser).unwrap();
        assert_eq!(decls[0].value, CssValue::String("3".into()));
    }

    #[test]
    fn parse_iteration_count_infinite() {
        let mut pi = cssparser::ParserInput::new("infinite");
        let mut parser = cssparser::Parser::new(&mut pi);
        let decls = parse_iteration_count(&mut parser).unwrap();
        assert_eq!(decls[0].value, CssValue::String("infinite".into()));
    }

    #[test]
    fn parse_direction_keywords() {
        let mut pi = cssparser::ParserInput::new("alternate");
        let mut parser = cssparser::Parser::new(&mut pi);
        let decls = parse_animation_direction(&mut parser).unwrap();
        assert_eq!(decls[0].value, CssValue::String("alternate".into()));
    }

    #[test]
    fn parse_fill_mode_both() {
        let mut pi = cssparser::ParserInput::new("both");
        let mut parser = cssparser::Parser::new(&mut pi);
        let decls = parse_animation_fill_mode(&mut parser).unwrap();
        assert_eq!(decls[0].value, CssValue::String("both".into()));
    }

    #[test]
    fn parse_play_state_paused() {
        let mut pi = cssparser::ParserInput::new("paused");
        let mut parser = cssparser::Parser::new(&mut pi);
        let decls = parse_animation_play_state(&mut parser).unwrap();
        assert_eq!(decls[0].value, CssValue::String("paused".into()));
    }

    #[test]
    fn keyframes_parse_basic() {
        let rule = parse_keyframes("fadeIn", "from { opacity: 0; } to { opacity: 1; }");
        assert_eq!(rule.name, "fadeIn");
        assert_eq!(rule.keyframes.len(), 2);
        assert_eq!(rule.keyframes[0].offset, 0.0);
        assert_eq!(rule.keyframes[1].offset, 1.0);
    }

    #[test]
    fn keyframes_parse_percentage() {
        let rule = parse_keyframes(
            "slide",
            "0% { width: 0px; } 50% { width: 50px; } 100% { width: 100px; }",
        );
        assert_eq!(rule.keyframes.len(), 3);
        assert_eq!(rule.keyframes[0].offset, 0.0);
        assert_eq!(rule.keyframes[1].offset, 0.5);
        assert_eq!(rule.keyframes[2].offset, 1.0);
    }

    #[test]
    fn keyframes_parse_multiple_selectors() {
        let rule = parse_keyframes("test", "0%, 100% { opacity: 1; } 50% { opacity: 0; }");
        // 0% and 100% share same block → 2 keyframes, plus 50% → 3 total
        assert_eq!(rule.keyframes.len(), 3);
    }

    #[test]
    fn keyframes_parse_color() {
        let rule = parse_keyframes("color-anim", "from { color: red; } to { color: blue; }");
        assert_eq!(
            rule.keyframes[0].declarations[0].value,
            CssValue::Color(elidex_plugin::CssColor::RED)
        );
        assert_eq!(
            rule.keyframes[1].declarations[0].value,
            CssValue::Color(elidex_plugin::CssColor::BLUE)
        );
    }

    #[test]
    fn transition_shorthand_basic() {
        let mut pi = cssparser::ParserInput::new("opacity 0.3s ease");
        let mut parser = cssparser::Parser::new(&mut pi);
        let decls = parse_transition_shorthand(&mut parser).unwrap();
        assert_eq!(decls.len(), 4);
        // property
        assert_eq!(decls[0].property, "transition-property");
        assert_eq!(decls[0].value, CssValue::String("opacity".into()));
        // duration
        assert_eq!(decls[1].property, "transition-duration");
        assert_eq!(decls[1].value, CssValue::Time(0.3));
    }

    #[test]
    fn hex_color_parse() {
        let mut pi = cssparser::ParserInput::new("#ff0000");
        let mut parser = cssparser::Parser::new(&mut pi);
        let color = try_parse_color(&mut parser).unwrap();
        assert_eq!(color, CssValue::Color(elidex_plugin::CssColor::RED));
    }

    #[test]
    fn hex_color_short() {
        let mut pi = cssparser::ParserInput::new("#f00");
        let mut parser = cssparser::Parser::new(&mut pi);
        let color = try_parse_color(&mut parser).unwrap();
        assert_eq!(
            color,
            CssValue::Color(elidex_plugin::CssColor::rgb(255, 0, 0))
        );
    }

    // F22: Multi-transition shorthand parse test.
    //
    // "opacity 0.3s, transform 0.5s ease-in" should produce two sets of
    // transition longhand values, each property carrying two comma-separated
    // entries.
    #[test]
    fn transition_shorthand_multi_value() {
        let mut pi = cssparser::ParserInput::new("opacity 0.3s, transform 0.5s ease-in");
        let mut parser = cssparser::Parser::new(&mut pi);
        let decls = parse_transition_shorthand(&mut parser).unwrap();

        // Should produce 4 longhands: property, duration, timing-function, delay
        assert_eq!(decls.len(), 4);

        // transition-property: "opacity, transform"
        assert_eq!(decls[0].property, "transition-property");
        assert_eq!(
            decls[0].value,
            CssValue::String("opacity, transform".into())
        );

        // transition-duration: list with 0.3s and 0.5s
        assert_eq!(decls[1].property, "transition-duration");
        match &decls[1].value {
            CssValue::List(items) => {
                assert_eq!(items.len(), 2);
                assert_eq!(items[0], CssValue::Time(0.3));
                assert_eq!(items[1], CssValue::Time(0.5));
            }
            other => panic!("expected List for transition-duration, got {other:?}"),
        }

        // transition-timing-function: the second entry should be ease-in.
        // ease-in is stored internally as CubicBezier(0.42, 0.0, 1.0, 1.0) and
        // serialized as "cubic-bezier(0.42, 0, 1, 1)".
        assert_eq!(decls[2].property, "transition-timing-function");
        let tf_str = match &decls[2].value {
            CssValue::String(s) => s.clone(),
            other => panic!("expected String for timing-function, got {other:?}"),
        };
        // The string contains two entries: the default "ease" for the first
        // transition and "cubic-bezier(0.42, 0, 1, 1)" (ease-in) for the second.
        assert!(
            tf_str.contains("cubic-bezier(0.42"),
            "second timing function should be the ease-in cubic-bezier: {tf_str}"
        );
    }

    // F23: steps(1, jump-none) should be rejected.
    //
    // Per CSS Easing Functions Level 2, jump-none requires at least 2 steps
    // because jump-none produces (n-1) intervals and 0 intervals is invalid.
    #[test]
    fn steps_jump_none_count_one_rejected() {
        let mut pi = cssparser::ParserInput::new("steps(1, jump-none)");
        let mut parser = cssparser::Parser::new(&mut pi);
        assert!(
            parse_timing_function(&mut parser).is_err(),
            "steps(1, jump-none) should be rejected: n=1 with jump-none produces 0 intervals"
        );
    }

    // S4-1: !important in @keyframes should be stripped (CSS Animations §4.1).
    #[test]
    fn keyframes_important_stripped() {
        let rule = parse_keyframes("test", "from { opacity: 0 !important; } to { opacity: 1; }");
        assert_eq!(rule.keyframes.len(), 2);
        assert_eq!(
            rule.keyframes[0].declarations[0].value,
            CssValue::Number(0.0),
            "!important should be stripped, leaving just the value"
        );
    }

    // S4-2: Missing from/to keyframes should be auto-generated.
    #[test]
    fn keyframes_auto_generate_from_to() {
        let rule = parse_keyframes("test", "50% { opacity: 0.5; }");
        assert_eq!(rule.keyframes.len(), 3, "should have from, 50%, and to");
        assert_eq!(rule.keyframes[0].offset, 0.0);
        assert!(
            rule.keyframes[0].declarations.is_empty(),
            "synthesized from should have empty declarations"
        );
        assert_eq!(rule.keyframes[1].offset, 0.5);
        assert_eq!(rule.keyframes[2].offset, 1.0);
        assert!(
            rule.keyframes[2].declarations.is_empty(),
            "synthesized to should have empty declarations"
        );
    }

    // S4-2: Existing from/to should not be duplicated.
    #[test]
    fn keyframes_no_duplicate_from_to() {
        let rule = parse_keyframes(
            "test",
            "from { opacity: 0; } 50% { opacity: 0.5; } to { opacity: 1; }",
        );
        assert_eq!(rule.keyframes.len(), 3);
        assert!(!rule.keyframes[0].declarations.is_empty());
        assert!(!rule.keyframes[2].declarations.is_empty());
    }
}
