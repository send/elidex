//! CSS animation/transition property parsing and `@keyframes` rule parsing.

use crate::style::TransitionProperty;
use crate::timing::{StepPosition, TimingFunction};
use elidex_plugin::{CssValue, ParseError, PropertyDeclaration};

use crate::MAX_LIST_ITEMS;

/// Maximum length of a single animation-name identifier.
const MAX_IDENTIFIER_LENGTH: usize = 256;

/// Parse a comma-separated list of items, up to `MAX_LIST_ITEMS`.
///
/// Calls `parse_one` for each item. If `parse_one` returns `Err`, propagates
/// the error. Stops when a comma is not found after an item.
fn parse_comma_list<T>(
    input: &mut cssparser::Parser<'_, '_>,
    mut parse_one: impl FnMut(&mut cssparser::Parser<'_, '_>) -> Result<T, ParseError>,
) -> Result<Vec<T>, ParseError> {
    let mut items = Vec::new();
    loop {
        if items.len() >= MAX_LIST_ITEMS {
            break;
        }
        items.push(parse_one(input)?);
        if input.try_parse(cssparser::Parser::expect_comma).is_err() {
            break;
        }
    }
    Ok(items)
}

/// Parse a `<time>` value, returning seconds.
///
/// Returns negative values (valid for delays). Use `parse_duration_time()`
/// for duration properties that reject negative values.
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
        // Browsers accept unitless 0 for backwards compatibility with
        // CSS Transitions / Animations, even though CSS Values Level 3 §5.1
        // strictly requires a unit for <time> values.
        cssparser::Token::Number { value: 0.0, .. } => Ok(0.0),
        _ => Err(parse_err("time", "expected time value")),
    }
}

/// Parse a `<time>` value that must be non-negative (for durations).
///
/// `transition-duration` and `animation-duration` do not accept negative
/// values (CSS Transitions §2.1, CSS Animations §3.2). Delays may be
/// negative and should use [`parse_time`] directly.
pub fn parse_duration_time(input: &mut cssparser::Parser<'_, '_>) -> Result<f32, ParseError> {
    let t = parse_time(input)?;
    if t < 0.0 {
        return Err(parse_err("duration", "negative duration"));
    }
    Ok(t)
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
    let mut props = parse_comma_list(input, |i| {
        let ident = i
            .expect_ident()
            .map_err(|_| parse_err("transition-property", "expected identifier"))?;
        let lower = ident.to_ascii_lowercase();
        Ok(match lower.as_str() {
            "none" => TransitionProperty::None,
            "all" => TransitionProperty::All,
            _ => TransitionProperty::Property(lower),
        })
    })?;
    // CSS Transitions Level 1: `none` is a standalone value.  If it appears
    // anywhere in a multi-value list, treat the whole list as `none`.
    if props.len() > 1 && props.iter().any(|p| matches!(p, TransitionProperty::None)) {
        props = vec![TransitionProperty::None];
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

/// Parse a comma-separated `<time>` list for duration or delay properties.
///
/// Duration properties (`transition-duration`, `animation-duration`) reject
/// negative values via [`parse_duration_time`]. Delay properties accept
/// negative values via [`parse_time`].
pub fn parse_time_list(
    name: &str,
    input: &mut cssparser::Parser<'_, '_>,
) -> Result<Vec<PropertyDeclaration>, ParseError> {
    let is_duration = name.ends_with("-duration");
    let values = parse_comma_list(input, |i| {
        if is_duration {
            parse_duration_time(i)
        } else {
            parse_time(i)
        }
    })?;
    Ok(vec![PropertyDeclaration::new(
        name,
        crate::time_list_value(&values),
    )])
}

/// Parse `transition-timing-function` or `animation-timing-function` list.
pub fn parse_timing_function_list(
    name: &str,
    input: &mut cssparser::Parser<'_, '_>,
) -> Result<Vec<PropertyDeclaration>, ParseError> {
    let fns = parse_comma_list(input, parse_timing_function)?;
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
    let names = parse_comma_list(input, |i| {
        let ident = i
            .expect_ident()
            .map_err(|_| parse_err("animation-name", "expected identifier"))?;
        if ident.len() > MAX_IDENTIFIER_LENGTH {
            return Err(parse_err("animation-name", "identifier too long"));
        }
        Ok(ident.to_string())
    })?;
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
    let counts = parse_comma_list(input, |i| {
        if i.try_parse(|i| i.expect_ident_matching("infinite")).is_ok() {
            Ok("infinite".to_string())
        } else {
            let n = i.expect_number().map_err(|_| {
                parse_err("animation-iteration-count", "expected number or infinite")
            })?;
            if n < 0.0 || !n.is_finite() {
                return Err(parse_err("animation-iteration-count", "negative value"));
            }
            Ok(n.to_string())
        }
    })?;
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
    let values = parse_comma_list(input, |i| {
        let ident = i
            .expect_ident()
            .map_err(|_| parse_err(name, "expected keyword"))?;
        let lower = ident.to_ascii_lowercase();
        if !allowed.contains(&lower.as_str()) {
            return Err(parse_err(name, &format!("unexpected keyword: {lower}")));
        }
        Ok(lower)
    })?;
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

/// Maximum byte length of `@keyframes` block input accepted for parsing.
const MAX_KEYFRAMES_INPUT_BYTES: usize = 65_536;

#[must_use]
pub fn parse_keyframes(name: &str, block_text: &str) -> KeyframesRule {
    if block_text.len() > MAX_KEYFRAMES_INPUT_BYTES {
        return KeyframesRule {
            name: name.to_string(),
            keyframes: Vec::new(),
        };
    }
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
            if keyframes.len() >= MAX_KEYFRAMES {
                break;
            }
            keyframes.push(Keyframe {
                offset,
                declarations: declarations.clone(),
            });
        }

        if keyframes.len() >= MAX_KEYFRAMES {
            break;
        }

        remaining = after_brace[brace_end + 1..].trim();
    }

    keyframes.sort_by(|a, b| {
        a.offset
            .partial_cmp(&b.offset)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Normalize near-equal offsets to canonical values before dedup
    for i in 1..keyframes.len() {
        if (keyframes[i].offset - keyframes[i - 1].offset).abs() < 1e-6 {
            keyframes[i].offset = keyframes[i - 1].offset;
        }
    }

    // CSS Animations Level 1 §3.2: merge duplicate offsets (last wins per property).
    keyframes.dedup_by(|later, earlier| {
        if (later.offset - earlier.offset).abs() < 1e-6 {
            // Merge later's declarations into earlier (earlier is kept by dedup_by).
            // Later declarations win for same property names.
            for decl in later.declarations.drain(..) {
                if let Some(existing) = earlier
                    .declarations
                    .iter_mut()
                    .find(|d| d.property == decl.property)
                {
                    *existing = decl;
                } else {
                    earlier.declarations.push(decl);
                }
            }
            true // Remove `later`
        } else {
            false
        }
    });

    // CSS Animations Level 1 §4.2: if 0% (from) or 100% (to) keyframes
    // are missing, synthesize them with empty declarations so the animation
    // engine interpolates from/to the element's computed values.
    if !keyframes.iter().any(|k| k.offset.abs() < 1e-6) {
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
        .take(MAX_KEYFRAMES)
        .filter_map(|s| {
            let s = s.trim();
            if s.eq_ignore_ascii_case("from") {
                Some(0.0)
            } else if s.eq_ignore_ascii_case("to") {
                Some(1.0)
            } else if let Some(pct) = s.strip_suffix('%') {
                // CSS Animations Level 1 §4.2: reject percentages outside [0%, 100%].
                pct.trim().parse::<f32>().ok().and_then(|v| {
                    if v.is_finite() && (0.0..=100.0).contains(&v) {
                        Some(v / 100.0)
                    } else {
                        None
                    }
                })
            } else {
                None
            }
        })
        .collect()
}

fn parse_keyframe_declarations(text: &str) -> Vec<PropertyDeclaration> {
    const MAX_DECLARATIONS_PER_BLOCK: usize = 1024;
    let mut decls = Vec::new();
    for part in text.split(';').take(MAX_DECLARATIONS_PER_BLOCK) {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some((prop, val)) = part.split_once(':') {
            let property = prop.trim().to_ascii_lowercase();
            // CSS Animations Level 1 §4.1: !important in @keyframes is ignored.
            let value_str = val.trim().trim_end_matches("!important").trim();
            let css_value = elidex_css::parse_raw_token_value(value_str);
            // Skip empty/unparseable values
            if !matches!(css_value, CssValue::RawTokens(ref s) if s.is_empty()) {
                decls.push(PropertyDeclaration::new(property, css_value));
            }
        }
    }
    decls
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
    if !(1..=10_000).contains(&count) {
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

/// Accumulated time and timing-function state shared by transition and
/// animation shorthand parsers.
pub(crate) struct TimeAndTiming {
    pub duration: f32,
    pub delay: f32,
    pub timing_function: TimingFunction,
    /// Whether `duration` has been set (first `<time>` = duration, second = delay).
    found_duration: bool,
}

impl TimeAndTiming {
    /// Create with default values (0s duration, ease, 0s delay).
    pub(crate) fn new() -> Self {
        Self {
            duration: 0.0,
            delay: 0.0,
            timing_function: TimingFunction::EASE,
            found_duration: false,
        }
    }

    /// Try to consume one `<timing-function>` or `<time>` token from the input.
    ///
    /// Returns `true` if a token was consumed. This is the shared core of
    /// `parse_single_transition` and `parse_single_animation`: both use
    /// "first `<time>` = duration, second `<time>` = delay" semantics with
    /// an interleaved `<timing-function>`.
    ///
    /// If `reject_negative_duration` is true, a negative first `<time>` value
    /// returns an error (CSS Transitions §2.1, CSS Animations §3.2).
    pub(crate) fn try_consume(
        &mut self,
        input: &mut cssparser::Parser<'_, '_>,
        reject_negative_duration: bool,
    ) -> Result<bool, ParseError> {
        if let Ok(tf) = input.try_parse(parse_timing_function) {
            self.timing_function = tf;
            return Ok(true);
        }
        if let Ok(t) = input.try_parse(parse_time) {
            if self.found_duration {
                self.delay = t;
            } else {
                if reject_negative_duration && t < 0.0 {
                    return Err(parse_err("duration", "negative duration is invalid"));
                }
                self.duration = t;
                self.found_duration = true;
            }
            return Ok(true);
        }
        Ok(false)
    }
}

pub(crate) fn parse_err(property: &str, message: &str) -> ParseError {
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
        if properties.len() >= MAX_LIST_ITEMS {
            break;
        }
        let (prop, dur, tf, del) = parse_single_transition(input)?;
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

// Time/timing-function parsing delegated to `TimeAndTiming::try_consume()`,
// shared with `parse_single_animation` in lib.rs.
fn parse_single_transition(
    input: &mut cssparser::Parser<'_, '_>,
) -> Result<(String, f32, TimingFunction, f32), ParseError> {
    let mut property = "all".to_string();
    let mut tnt = TimeAndTiming::new();

    // Up to 4 components in any order:
    // <property> || <duration> || <timing-function> || <delay>
    for _ in 0..4 {
        // Try timing function and time values via shared helper.
        // Reject negative duration per CSS Transitions §2.1.
        if tnt.try_consume(input, true)? {
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

    Ok((property, tnt.duration, tnt.timing_function, tnt.delay))
}

#[cfg(test)]
#[path = "parse_tests.rs"]
mod parse_tests;
