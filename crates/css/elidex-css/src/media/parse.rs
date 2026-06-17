//! Media Queries Level 4 parser — mediaqueries-4 §3 Syntax.
//!
//! **Total + recovering** per §3.2 Error Handling: never errors or panics
//! (CSSOM `matchMedia` does not throw). Four-way failure routing:
//!   (a) grammar mismatch → that `<media-query>` becomes `not all`, recovering
//!       at the next top-level comma;
//!   (b) unknown `<mf-name>`/`<mf-value>` (valid `( <media-feature> )` shape) →
//!       the whole `<media-query>` becomes `not all` at parse;
//!   (c) unknown/deprecated `<media-type>` ident → `MediaType::Other`
//!       (definite-false but negatable);
//!   (d) `<general-enclosed>` (matches neither `( <media-feature> )` nor
//!       `( <media-condition> )`) → `MediaCondition::GeneralEnclosed`
//!       (Kleene unknown at eval).

use cssparser::{ParseError, Parser, ParserInput, Token};
use elidex_plugin::LengthUnit;

#[allow(clippy::wildcard_imports)]
use super::types::*;

/// Parse a `<media-query-list>` (mediaqueries-4 §3) from an untrusted string.
///
/// Total: a grammar-malformed or unknown-feature `<media-query>` is replaced
/// by the `not all` sentinel per §3.2, recovering at the next top-level comma;
/// the rest of the list is unaffected. An empty/whitespace string yields the
/// empty list (§3 accepts an empty list; it evaluates to `true` per §2.1).
#[must_use]
pub fn parse_media_query_list(text: &str) -> MediaQueryList {
    let mut input = ParserInput::new(text);
    let mut parser = Parser::new(&mut input);
    parser.skip_whitespace();
    if parser.is_exhausted() {
        return MediaQueryList(Vec::new());
    }
    let queries = parser.parse_comma_separated_ignoring_errors(
        |segment| -> Result<MediaQuery, ParseError<'_, ()>> {
            let query = match parse_media_query(segment) {
                Ok(q) if segment.is_exhausted() => q,
                // §3.2: any grammar/unknown failure (or trailing junk) → this
                // query becomes `not all`.
                _ => MediaQuery::not_all(),
            };
            // Drain the rest of the segment so the comma-splitter is satisfied
            // (it treats leftover input before the comma as an error/drop).
            while segment.next().is_ok() {}
            Ok(query)
        },
    );
    MediaQueryList(queries)
}

/// `<media-query>` — `<media-condition> | [not|only]? <media-type> [and <media-condition-without-or>]?`.
fn parse_media_query<'i>(input: &mut Parser<'i, '_>) -> Result<MediaQuery, ParseError<'i, ()>> {
    input.skip_whitespace();
    // Try the type-prefixed branch first. It fails (and rewinds) for a leading
    // `(` or `not (`, which is a condition-only query.
    if let Ok(query) = input.try_parse(parse_type_query) {
        return Ok(query);
    }
    let condition = parse_media_condition(input)?;
    Ok(MediaQuery {
        qualifier: None,
        media_type: None,
        condition: Some(condition),
    })
}

/// `[not|only]? <media-type> [and <media-condition-without-or>]?`.
fn parse_type_query<'i>(input: &mut Parser<'i, '_>) -> Result<MediaQuery, ParseError<'i, ()>> {
    input.skip_whitespace();
    let qualifier = input
        .try_parse(|i| -> Result<Qualifier, ParseError<'_, ()>> {
            let q = {
                let ident = i.expect_ident().map_err(ParseError::from)?;
                match_qualifier(ident)
            };
            q.ok_or_else(|| i.new_custom_error(()))
        })
        .ok();
    let media_type = {
        let ident = input.expect_ident().map_err(ParseError::from)?;
        classify_media_type(ident)
    }
    .map_err(|()| input.new_custom_error(()))?;
    let condition = if input.try_parse(|i| expect_keyword(i, "and")).is_ok() {
        Some(parse_media_condition_without_or(input)?)
    } else {
        None
    };
    Ok(MediaQuery {
        qualifier,
        media_type: Some(media_type),
        condition,
    })
}

/// `<media-condition>` — `<media-not> | <media-in-parens> [ <media-and>* | <media-or>* ]`.
fn parse_media_condition<'i>(
    input: &mut Parser<'i, '_>,
) -> Result<MediaCondition, ParseError<'i, ()>> {
    input.skip_whitespace();
    if input.try_parse(|i| expect_keyword(i, "not")).is_ok() {
        let inner = parse_media_in_parens(input)?;
        return Ok(MediaCondition::Not(Box::new(inner)));
    }
    let first = parse_media_in_parens(input)?;
    if input.try_parse(|i| expect_keyword(i, "and")).is_ok() {
        let mut terms = vec![first, parse_media_in_parens(input)?];
        while input.try_parse(|i| expect_keyword(i, "and")).is_ok() {
            terms.push(parse_media_in_parens(input)?);
        }
        Ok(MediaCondition::And(terms))
    } else if input.try_parse(|i| expect_keyword(i, "or")).is_ok() {
        let mut terms = vec![first, parse_media_in_parens(input)?];
        while input.try_parse(|i| expect_keyword(i, "or")).is_ok() {
            terms.push(parse_media_in_parens(input)?);
        }
        Ok(MediaCondition::Or(terms))
    } else {
        Ok(first)
    }
}

/// `<media-condition-without-or>` — `<media-not> | <media-in-parens> <media-and>*`.
fn parse_media_condition_without_or<'i>(
    input: &mut Parser<'i, '_>,
) -> Result<MediaCondition, ParseError<'i, ()>> {
    input.skip_whitespace();
    if input.try_parse(|i| expect_keyword(i, "not")).is_ok() {
        let inner = parse_media_in_parens(input)?;
        return Ok(MediaCondition::Not(Box::new(inner)));
    }
    let first = parse_media_in_parens(input)?;
    let mut terms = vec![first];
    while input.try_parse(|i| expect_keyword(i, "and")).is_ok() {
        terms.push(parse_media_in_parens(input)?);
    }
    if terms.len() == 1 {
        Ok(terms.into_iter().next().expect("one term"))
    } else {
        Ok(MediaCondition::And(terms))
    }
}

/// `<media-in-parens>` — `( <media-condition> ) | <media-feature> | <general-enclosed>`.
fn parse_media_in_parens<'i>(
    input: &mut Parser<'i, '_>,
) -> Result<MediaCondition, ParseError<'i, ()>> {
    input.skip_whitespace();
    // A function token (`name(...)`) can only be <general-enclosed>.
    if input.try_parse(expect_function_token).is_ok() {
        input.parse_nested_block(drain_block)?;
        return Ok(MediaCondition::GeneralEnclosed);
    }
    input.expect_parenthesis_block().map_err(ParseError::from)?;
    input.parse_nested_block(parse_parens_content)
}

/// Consume a `<function-token>` (opens its block for `parse_nested_block`).
fn expect_function_token<'i>(input: &mut Parser<'i, '_>) -> Result<(), ParseError<'i, ()>> {
    input
        .expect_function()
        .map(|_| ())
        .map_err(ParseError::from)
}

/// Consume all remaining tokens in a nested block (used to swallow a
/// `<general-enclosed>` block's content). The `Result` wrapper is required by
/// `parse_nested_block`'s closure bound.
#[allow(clippy::unnecessary_wraps)]
fn drain_block<'i>(inner: &mut Parser<'i, '_>) -> Result<(), ParseError<'i, ()>> {
    while inner.next().is_ok() {}
    Ok(())
}

/// The content inside a `( ... )`: try `<media-condition>`, then
/// `<media-feature>`, else `<general-enclosed>`.
fn parse_parens_content<'i>(
    inner: &mut Parser<'i, '_>,
) -> Result<MediaCondition, ParseError<'i, ()>> {
    inner.skip_whitespace();
    // 1. nested `( <media-condition> )`.
    if let Ok(cond) = inner.try_parse(|i| -> Result<MediaCondition, ParseError<'_, ()>> {
        let c = parse_media_condition(i)?;
        i.skip_whitespace();
        i.expect_exhausted().map_err(ParseError::from)?;
        Ok(c)
    }) {
        return Ok(cond);
    }
    // 2. `<media-feature>`.
    match parse_media_feature(inner) {
        Ok(feature) => Ok(MediaCondition::Feature(feature)),
        // feature-shaped but unknown name / invalid value → §3.2 → not all.
        Err(FeatErr::Invalid) => Err(inner.new_custom_error(())),
        // 3. `<general-enclosed>`: not feature-shaped → Kleene unknown.
        Err(FeatErr::NotShaped) => {
            while inner.next().is_ok() {}
            Ok(MediaCondition::GeneralEnclosed)
        }
    }
}

/// The outcome of attempting to parse a `<media-feature>` from parens content.
enum FeatErr {
    /// Matches `<media-feature>` grammar but the name/value is unknown/invalid
    /// → §3.2 → the whole `<media-query>` becomes `not all`.
    Invalid,
    /// Does not match `<media-feature>` grammar → `<general-enclosed>`.
    NotShaped,
}

/// Parse the content of a `( ... )` as a `<media-feature>` (parser scoped to
/// the content). See [`FeatErr`] for the failure routing.
fn parse_media_feature(input: &mut Parser<'_, '_>) -> Result<MediaFeature, FeatErr> {
    input.skip_whitespace();
    // Name-first: an ident leads (boolean | plain | name-first range).
    if let Ok(name) = input.try_parse(|i| {
        i.expect_ident()
            .map(|s| s.as_ref().to_owned())
            .map_err(|_| ())
    }) {
        return parse_name_first(input, &name);
    }
    // Value-first range: `<value> <op> <name> [ <op> <value> ]?`.
    parse_value_first(input)
}

fn parse_name_first(input: &mut Parser<'_, '_>, name: &str) -> Result<MediaFeature, FeatErr> {
    input.skip_whitespace();
    // `(name)` — boolean context.
    if input.is_exhausted() {
        return parse_boolean(name);
    }
    // `name : value` — plain.
    if input
        .try_parse(|i| i.expect_colon().map_err(|_| ()))
        .is_ok()
    {
        return parse_plain(input, name);
    }
    // `name <op> value` — name-first range.
    if let Some(op) = try_comparison(input) {
        return parse_name_first_range(input, name, op);
    }
    // ident followed by some other token → not feature-shaped.
    Err(FeatErr::NotShaped)
}

fn parse_boolean(name: &str) -> Result<MediaFeature, FeatErr> {
    let lower = name.to_ascii_lowercase();
    // §2.4.4: `min-`/`max-` in a boolean context is a syntax error.
    if lower.starts_with("min-") || lower.starts_with("max-") {
        return Err(FeatErr::Invalid);
    }
    match classify_boolean_feature(&lower) {
        Some(bf) => Ok(MediaFeature::Boolean(bf)),
        // a lone ident matches `<mf-boolean>` grammar → unknown name → not all.
        None => Err(FeatErr::Invalid),
    }
}

fn parse_plain(input: &mut Parser<'_, '_>, name: &str) -> Result<MediaFeature, FeatErr> {
    input.skip_whitespace();
    let raw = parse_mf_value(input).map_err(|()| FeatErr::NotShaped)?;
    input.skip_whitespace();
    // mf-plain is exactly one value; extra tokens → future syntax → general-enclosed.
    if !input.is_exhausted() {
        return Err(FeatErr::NotShaped);
    }
    let lower = name.to_ascii_lowercase();
    if let Some(df) = classify_discrete_feature(&lower) {
        return match raw {
            RawMfValue::Ident(kw) => match discrete_value(df, &kw.to_ascii_lowercase()) {
                Some(value) => Ok(MediaFeature::Discrete { name: df, value }),
                None => Err(FeatErr::Invalid),
            },
            _ => Err(FeatErr::Invalid),
        };
    }
    let (base, op) = strip_min_max(&lower);
    match classify_range_feature(base) {
        Some(rf) => match coerce_raw(raw, rf) {
            Some(value) => Ok(MediaFeature::Range {
                name: rf,
                constraints: vec![RangeConstraint { op, value }],
            }),
            None => Err(FeatErr::Invalid),
        },
        None => Err(FeatErr::Invalid),
    }
}

fn parse_name_first_range(
    input: &mut Parser<'_, '_>,
    name: &str,
    op: RangeOp,
) -> Result<MediaFeature, FeatErr> {
    input.skip_whitespace();
    let raw = parse_mf_value(input).map_err(|()| FeatErr::Invalid)?;
    input.skip_whitespace();
    if !input.is_exhausted() {
        return Err(FeatErr::NotShaped);
    }
    let lower = name.to_ascii_lowercase();
    // a discrete feature in range context is invalid (§2.4.1).
    if classify_discrete_feature(&lower).is_some() {
        return Err(FeatErr::Invalid);
    }
    match classify_range_feature(&lower) {
        Some(rf) => match coerce_raw(raw, rf) {
            Some(value) => Ok(MediaFeature::Range {
                name: rf,
                constraints: vec![RangeConstraint { op, value }],
            }),
            None => Err(FeatErr::Invalid),
        },
        None => Err(FeatErr::Invalid),
    }
}

fn parse_value_first(input: &mut Parser<'_, '_>) -> Result<MediaFeature, FeatErr> {
    input.skip_whitespace();
    let raw1 = parse_mf_value(input).map_err(|()| FeatErr::NotShaped)?;
    let op1 = try_comparison(input).ok_or(FeatErr::NotShaped)?;
    input.skip_whitespace();
    let name = input
        .try_parse(|i| {
            i.expect_ident()
                .map(|s| s.as_ref().to_owned())
                .map_err(|_| ())
        })
        .map_err(|()| FeatErr::NotShaped)?;
    let lower = name.to_ascii_lowercase();
    let rf = classify_range_feature(&lower).ok_or(FeatErr::Invalid)?;
    let v1 = coerce_raw(raw1, rf).ok_or(FeatErr::Invalid)?;
    // optional second `<op> <value>` for `a <= width <= b`.
    if let Some(op2) = try_comparison(input) {
        // §3 `<mf-range>`: a two-sided form is only
        // `<value> <mf-lt> <name> <mf-lt> <value>` or the `<mf-gt>` dual — both
        // comparisons same-direction, and `=` is not allowed. Anything else
        // (mixed `<`…`>`, or any `=`) matches `( <any-value> )` but not
        // `<mf-range>` → `<general-enclosed>` (Kleene unknown), NOT a Range.
        if !same_direction_range(op1, op2) {
            return Err(FeatErr::NotShaped);
        }
        input.skip_whitespace();
        let raw2 = parse_mf_value(input).map_err(|()| FeatErr::Invalid)?;
        input.skip_whitespace();
        if !input.is_exhausted() {
            return Err(FeatErr::NotShaped);
        }
        let v2 = coerce_raw(raw2, rf).ok_or(FeatErr::Invalid)?;
        return Ok(MediaFeature::Range {
            name: rf,
            constraints: vec![
                RangeConstraint {
                    op: flip_op(op1),
                    value: v1,
                },
                RangeConstraint { op: op2, value: v2 },
            ],
        });
    }
    input.skip_whitespace();
    if !input.is_exhausted() {
        return Err(FeatErr::NotShaped);
    }
    Ok(MediaFeature::Range {
        name: rf,
        constraints: vec![RangeConstraint {
            op: flip_op(op1),
            value: v1,
        }],
    })
}

/// A raw `<mf-value>` token before coercion to a feature's value type.
enum RawMfValue {
    Number(f64),
    Dimension(f64, String),
    Ratio(f64, f64),
    Ident(String),
}

/// The leading token of an `<mf-value>`, captured to release the token borrow
/// before any further parsing (e.g. the ratio `/ <number>` lookahead).
enum FirstToken {
    Num(f64),
    Dim(f64, String),
    Id(String),
}

/// Parse a single `<mf-value>` (`<number> | <dimension> | <ident> | <ratio>`).
fn parse_mf_value(input: &mut Parser<'_, '_>) -> Result<RawMfValue, ()> {
    input.skip_whitespace();
    let first = match input.next().map_err(|_| ())? {
        Token::Number { value, .. } => FirstToken::Num(f64::from(*value)),
        Token::Dimension { value, unit, .. } => {
            FirstToken::Dim(f64::from(*value), unit.as_ref().to_owned())
        }
        Token::Ident(s) => FirstToken::Id(s.as_ref().to_owned()),
        _ => return Err(()),
    };
    match first {
        FirstToken::Num(n) => {
            // optional `/ <number>` → ratio.
            let denom = input.try_parse(|i| -> Result<f64, ()> {
                let is_slash = matches!(i.next().map_err(|_| ())?, Token::Delim('/'));
                if !is_slash {
                    return Err(());
                }
                match i.next().map_err(|_| ())? {
                    Token::Number { value, .. } => Ok(f64::from(*value)),
                    _ => Err(()),
                }
            });
            match denom {
                Ok(d) => Ok(RawMfValue::Ratio(n, d)),
                Err(()) => Ok(RawMfValue::Number(n)),
            }
        }
        FirstToken::Dim(v, u) => Ok(RawMfValue::Dimension(v, u)),
        FirstToken::Id(s) => Ok(RawMfValue::Ident(s)),
    }
}

/// Coerce a raw value to the value type a range feature expects.
fn coerce_raw(raw: RawMfValue, rf: RangeFeature) -> Option<RangeValue> {
    match rf {
        RangeFeature::Width | RangeFeature::Height => match raw {
            RawMfValue::Dimension(v, u) => crate::values::parse_length_unit(&u)
                .ok()
                .map(|unit| RangeValue::Length { value: v, unit }),
            // A unitless `0` is `0px` (CSS); any other bare number is invalid
            // as a `<length>`.
            RawMfValue::Number(n) => (n == 0.0).then_some(RangeValue::Length {
                value: 0.0,
                unit: LengthUnit::Px,
            }),
            _ => None,
        },
        RangeFeature::AspectRatio => match raw {
            // `<ratio>` components are non-negative (css-values-4 §5.7); a
            // negative component or a non-positive denominator is outside the
            // value syntax → None (→ §3.2 `not all`).
            RawMfValue::Ratio(n, d) if n >= 0.0 && d > 0.0 => Some(RangeValue::Ratio(n / d)),
            RawMfValue::Number(n) if n >= 0.0 => Some(RangeValue::Ratio(n)),
            _ => None,
        },
        RangeFeature::Resolution => match raw {
            RawMfValue::Dimension(v, u) => resolution_to_dppx(v, &u).map(RangeValue::Dppx),
            _ => None,
        },
    }
}

/// `<resolution>` units → dppx — css-values-4 §7.4.
fn resolution_to_dppx(value: f64, unit: &str) -> Option<f64> {
    match unit.to_ascii_lowercase().as_str() {
        "dppx" | "x" => Some(value),
        "dpi" => Some(value / 96.0),
        "dpcm" => Some(value / (96.0 / 2.54)),
        _ => None,
    }
}

/// Parse an `<mf-comparison>` operator (`<` `<=` `>` `>=` `=`). Whitespace
/// between `<`/`>` and `=` is tolerated (a minor laxity vs the spec's
/// no-whitespace rule).
fn try_comparison(input: &mut Parser<'_, '_>) -> Option<RangeOp> {
    input
        .try_parse(|i| -> Result<RangeOp, ()> {
            i.skip_whitespace();
            let c = match i.next().map_err(|_| ())? {
                Token::Delim(c) => *c,
                _ => return Err(()),
            };
            match c {
                '<' => Ok(if eat_equals(i) {
                    RangeOp::Le
                } else {
                    RangeOp::Lt
                }),
                '>' => Ok(if eat_equals(i) {
                    RangeOp::Ge
                } else {
                    RangeOp::Gt
                }),
                '=' => Ok(RangeOp::Eq),
                _ => Err(()),
            }
        })
        .ok()
}

fn eat_equals(input: &mut Parser<'_, '_>) -> bool {
    input
        .try_parse(|i| -> Result<(), ()> {
            match i.next().map_err(|_| ())? {
                Token::Delim('=') => Ok(()),
                _ => Err(()),
            }
        })
        .is_ok()
}

/// `a <op> name` ≡ `name <flip(op)> a`.
fn flip_op(op: RangeOp) -> RangeOp {
    match op {
        RangeOp::Lt => RangeOp::Gt,
        RangeOp::Le => RangeOp::Ge,
        RangeOp::Gt => RangeOp::Lt,
        RangeOp::Ge => RangeOp::Le,
        RangeOp::Eq => RangeOp::Eq,
    }
}

/// §3 `<mf-range>` two-sided forms require both comparisons to be the same
/// direction (`<`/`<=` … `<`/`<=` or `>`/`>=` … `>`/`>=`); `=` is not allowed
/// in either slot.
fn same_direction_range(op1: RangeOp, op2: RangeOp) -> bool {
    use RangeOp::{Ge, Gt, Le, Lt};
    matches!((op1, op2), (Lt | Le, Lt | Le) | (Gt | Ge, Gt | Ge))
}

/// Split a legacy `min-`/`max-` prefix off a range feature name.
fn strip_min_max(name: &str) -> (&str, RangeOp) {
    if let Some(rest) = name.strip_prefix("min-") {
        (rest, RangeOp::Ge)
    } else if let Some(rest) = name.strip_prefix("max-") {
        (rest, RangeOp::Le)
    } else {
        (name, RangeOp::Eq)
    }
}

fn match_qualifier(ident: &str) -> Option<Qualifier> {
    if ident.eq_ignore_ascii_case("not") {
        Some(Qualifier::Not)
    } else if ident.eq_ignore_ascii_case("only") {
        Some(Qualifier::Only)
    } else {
        None
    }
}

/// `<media-type>` — §2.3 / §3.2. Reserved keywords cannot be a media type
/// (grammar fail → `not all`); any other ident is recognized-but-non-matching.
fn classify_media_type(ident: &str) -> Result<MediaType, ()> {
    match ident.to_ascii_lowercase().as_str() {
        "all" => Ok(MediaType::All),
        "screen" => Ok(MediaType::Screen),
        "print" => Ok(MediaType::Print),
        "not" | "only" | "and" | "or" | "layer" => Err(()),
        _ => Ok(MediaType::Other),
    }
}

fn classify_range_feature(name: &str) -> Option<RangeFeature> {
    match name {
        "width" => Some(RangeFeature::Width),
        "height" => Some(RangeFeature::Height),
        "aspect-ratio" => Some(RangeFeature::AspectRatio),
        "resolution" => Some(RangeFeature::Resolution),
        _ => None,
    }
}

fn classify_discrete_feature(name: &str) -> Option<DiscreteFeature> {
    match name {
        "orientation" => Some(DiscreteFeature::Orientation),
        "prefers-color-scheme" => Some(DiscreteFeature::PrefersColorScheme),
        "prefers-reduced-motion" => Some(DiscreteFeature::PrefersReducedMotion),
        _ => None,
    }
}

fn discrete_value(df: DiscreteFeature, kw: &str) -> Option<DiscreteValue> {
    match (df, kw) {
        (DiscreteFeature::Orientation, "portrait") => Some(DiscreteValue::Portrait),
        (DiscreteFeature::Orientation, "landscape") => Some(DiscreteValue::Landscape),
        (DiscreteFeature::PrefersColorScheme, "light") => Some(DiscreteValue::Light),
        (DiscreteFeature::PrefersColorScheme, "dark") => Some(DiscreteValue::Dark),
        (DiscreteFeature::PrefersReducedMotion, "no-preference") => {
            Some(DiscreteValue::NoPreferenceMotion)
        }
        (DiscreteFeature::PrefersReducedMotion, "reduce") => Some(DiscreteValue::Reduce),
        _ => None,
    }
}

fn classify_boolean_feature(name: &str) -> Option<BooleanFeature> {
    match name {
        "width" => Some(BooleanFeature::Width),
        "height" => Some(BooleanFeature::Height),
        "aspect-ratio" => Some(BooleanFeature::AspectRatio),
        "resolution" => Some(BooleanFeature::Resolution),
        "orientation" => Some(BooleanFeature::Orientation),
        "color" => Some(BooleanFeature::Color),
        "prefers-color-scheme" => Some(BooleanFeature::PrefersColorScheme),
        "prefers-reduced-motion" => Some(BooleanFeature::PrefersReducedMotion),
        _ => None,
    }
}

/// `expect_ident` that matches a specific keyword (case-insensitive).
fn expect_keyword<'i>(input: &mut Parser<'i, '_>, kw: &str) -> Result<(), ParseError<'i, ()>> {
    let matched = {
        let ident = input.expect_ident().map_err(ParseError::from)?;
        ident.eq_ignore_ascii_case(kw)
    };
    if matched {
        Ok(())
    } else {
        Err(input.new_custom_error(()))
    }
}
