//! Media Queries Level 4 parser — mediaqueries-4 §3 Syntax.
//!
//! **Total + recovering** per §3.2 Error Handling: never errors or panics
//! (CSSOM `matchMedia` does not throw). The error model is two-way:
//!
//!   * **recognized feature vs general-enclosed** — inside a `( … )`, any
//!     content that is not a recognized `<media-feature>` (known `<mf-name>` +
//!     valid `<mf-value>` that consumes the whole block) still matches the
//!     `( <any-value> )` `<general-enclosed>` production and becomes
//!     `MediaCondition::GeneralEnclosed` → Kleene *unknown* at eval (§3.1). This
//!     covers unknown names, invalid/missing values, trailing junk, and
//!     malformed ranges. §3.2's "unknown `<mf-name>`/`<mf-value>` results in the
//!     value unknown; a `<media-query>` whose value is unknown is replaced with
//!     `not all`" is NOT a parse-time poison of the sibling terms — it is the
//!     §3.1 boundary coercion (`unknown → false`) applied once per environment
//!     at eval, because the outcome is environment-dependent (`(color) or
//!     (unknown)` is true on a color device, false on a monochrome one).
//!   * **`not all`** is reserved for *top-level* grammar failures that no
//!     production accepts: a reserved keyword used as a `<media-type>` (`or`,
//!     `and`, `only`, `not`, `layer`), `and`/`or` mixed at one level, or other
//!     bare-token garbage. These replace the whole `<media-query>`, recovering
//!     at the next top-level comma.
//!
//! An unknown/deprecated `<media-type>` ident is `MediaType::Other`
//! (definite-false but negatable — `not unknowntype` is true).

use cssparser::{ParseError, Parser, ParserInput, Token};
use elidex_plugin::{CalcExpr, CssValue, LengthUnit};

#[allow(clippy::wildcard_imports)]
use super::types::*;

/// Parse a `<media-query-list>` (mediaqueries-4 §3) from an untrusted string.
///
/// Total: a *top-level* grammar-malformed `<media-query>` is replaced by the
/// `not all` sentinel per §3.2, recovering at the next top-level comma; the
/// rest of the list is unaffected. (An unknown/invalid feature *inside* a
/// `( … )` is not malformed at this level — it becomes `<general-enclosed>` →
/// Kleene unknown.) An empty/whitespace string yields the empty list (§3
/// accepts an empty list; it evaluates to `true` per §2.1).
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
                // §3.2: a top-level grammar failure (or trailing junk before the
                // comma) → this query becomes `not all`. (Feature-level unknowns
                // are absorbed as general-enclosed inside `parse_media_query`.)
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

/// Maximum `<media-in-parens>` nesting depth. Each `(` opens one
/// `parse_media_in_parens` → `parse_parens_content` → `parse_media_condition`
/// recursion cycle, so untrusted input like `((((…))))` would recurse without
/// bound and abort the process via stack overflow — violating the parser's
/// total/never-panic contract (a DoS path for page-controlled CSS / `matchMedia`
/// strings). Mirrors the `calc()` parser's `MAX_CALC_DEPTH`; real media queries
/// never nest anywhere near this. Exceeding it fails the over-deep
/// `<media-in-parens>` parse, which then drains iteratively to
/// `<general-enclosed>` → Kleene unknown → false (the bounded recursion is the
/// DoS guarantee; cssparser `next()` returns a nested block as one token, so the
/// drain does not re-descend).
const MAX_MEDIA_NESTING_DEPTH: u32 = 32;

/// `<media-query>` — `<media-condition> | [not|only]? <media-type> [and <media-condition-without-or>]?`.
fn parse_media_query<'i>(input: &mut Parser<'i, '_>) -> Result<MediaQuery, ParseError<'i, ()>> {
    input.skip_whitespace();
    // Try the type-prefixed branch first. It fails (and rewinds) for a leading
    // `(` or `not (`, which is a condition-only query.
    if let Ok(query) = input.try_parse(parse_type_query) {
        return Ok(query);
    }
    let condition = parse_media_condition(input, 0)?;
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
        Some(parse_media_condition_without_or(input, 0)?)
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
/// `depth` is the current `<media-in-parens>` nesting level (see
/// [`MAX_MEDIA_NESTING_DEPTH`]); the `and`/`or` lists are iterative (heap),
/// only nested `( … )` deepens the recursion.
fn parse_media_condition<'i>(
    input: &mut Parser<'i, '_>,
    depth: u32,
) -> Result<MediaCondition, ParseError<'i, ()>> {
    input.skip_whitespace();
    if input.try_parse(|i| expect_keyword(i, "not")).is_ok() {
        let inner = parse_media_in_parens(input, depth)?;
        return Ok(MediaCondition::Not(Box::new(inner)));
    }
    let first = parse_media_in_parens(input, depth)?;
    if input.try_parse(|i| expect_keyword(i, "and")).is_ok() {
        let mut terms = vec![first, parse_media_in_parens(input, depth)?];
        while input.try_parse(|i| expect_keyword(i, "and")).is_ok() {
            terms.push(parse_media_in_parens(input, depth)?);
        }
        Ok(MediaCondition::And(terms))
    } else if input.try_parse(|i| expect_keyword(i, "or")).is_ok() {
        let mut terms = vec![first, parse_media_in_parens(input, depth)?];
        while input.try_parse(|i| expect_keyword(i, "or")).is_ok() {
            terms.push(parse_media_in_parens(input, depth)?);
        }
        Ok(MediaCondition::Or(terms))
    } else {
        Ok(first)
    }
}

/// `<media-condition-without-or>` — `<media-not> | <media-in-parens> <media-and>*`.
fn parse_media_condition_without_or<'i>(
    input: &mut Parser<'i, '_>,
    depth: u32,
) -> Result<MediaCondition, ParseError<'i, ()>> {
    input.skip_whitespace();
    if input.try_parse(|i| expect_keyword(i, "not")).is_ok() {
        let inner = parse_media_in_parens(input, depth)?;
        return Ok(MediaCondition::Not(Box::new(inner)));
    }
    let first = parse_media_in_parens(input, depth)?;
    let mut terms = vec![first];
    while input.try_parse(|i| expect_keyword(i, "and")).is_ok() {
        terms.push(parse_media_in_parens(input, depth)?);
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
    depth: u32,
) -> Result<MediaCondition, ParseError<'i, ()>> {
    // §3.2 total contract: never a stack-overflow panic. Over the depth cap,
    // this `<media-in-parens>` parse fails; the enclosing block then resolves to
    // `<general-enclosed>` (Kleene unknown) — the bounded recursion is the DoS
    // guard. Check before descending another level.
    if depth >= MAX_MEDIA_NESTING_DEPTH {
        return Err(input.new_custom_error(()));
    }
    input.skip_whitespace();
    // A function token (`name(...)`) can only be <general-enclosed>.
    if input.try_parse(expect_function_token).is_ok() {
        input.parse_nested_block(drain_block)?;
        return Ok(MediaCondition::GeneralEnclosed);
    }
    input.expect_parenthesis_block().map_err(ParseError::from)?;
    input.parse_nested_block(|inner| parse_parens_content(inner, depth + 1))
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

/// The content inside a `( ... )`: try `( <media-condition> )`, then a
/// recognized `<media-feature>`, else `<general-enclosed>`.
///
/// This never produces `not all`: per §3.2 any `( … )` whose content is not a
/// recognized condition or feature still matches `( <any-value> )` =
/// `<general-enclosed>` → Kleene unknown. The `not all` sentinel is a
/// top-level concern (reserved-keyword media type, mixed `and`/`or`), handled
/// by [`parse_media_query_list`] / [`parse_type_query`], not here.
#[allow(clippy::unnecessary_wraps)] // signature dictated by `parse_nested_block`.
fn parse_parens_content<'i>(
    inner: &mut Parser<'i, '_>,
    depth: u32,
) -> Result<MediaCondition, ParseError<'i, ()>> {
    inner.skip_whitespace();
    // 1. nested `( <media-condition> )`.
    if let Ok(cond) = inner.try_parse(|i| -> Result<MediaCondition, ParseError<'_, ()>> {
        let c = parse_media_condition(i, depth)?;
        i.skip_whitespace();
        i.expect_exhausted().map_err(ParseError::from)?;
        Ok(c)
    }) {
        return Ok(cond);
    }
    // 2. a recognized `<media-feature>`, else 3. `<general-enclosed>`.
    if let Some(feature) = parse_media_feature(inner) {
        return Ok(MediaCondition::Feature(feature));
    }
    // Unknown name / invalid value / trailing junk / malformed range — all
    // still match `( <any-value> )` → Kleene unknown (§3.1/§3.2).
    while inner.next().is_ok() {}
    Ok(MediaCondition::GeneralEnclosed)
}

/// Parse the content of a `( ... )` as a complete `<media-feature>`.
///
/// Returns `Some` only when the content is a recognized feature — a known
/// `<mf-name>` with a valid `<mf-value>` — that consumes the *entire* parens
/// content. Otherwise `None`: the caller treats the block as
/// `<general-enclosed>` (Kleene unknown), never `not all`. Unknown names,
/// invalid/missing values, trailing junk, and malformed (mixed-direction / `=`)
/// ranges all yield `None`.
fn parse_media_feature(input: &mut Parser<'_, '_>) -> Option<MediaFeature> {
    input
        .try_parse(|i| -> Result<MediaFeature, ()> {
            let feature = parse_feature_body(i)?;
            i.skip_whitespace();
            // A `<media-feature>` is the *whole* parenthesized content; a
            // trailing token means it is not a feature → general-enclosed.
            if i.is_exhausted() {
                Ok(feature)
            } else {
                Err(())
            }
        })
        .ok()
}

/// Dispatch parens content to the name-first (`<ident> …`) or value-first
/// (`<value> <op> <name> …`) `<media-feature>` shape. A leading ident that does
/// not resolve to a recognized name-first feature is retried as value-first, so
/// `(infinite > resolution)` (value `infinite`, name `resolution`) is read as
/// `resolution < infinite` rather than an unknown `infinite` feature.
fn parse_feature_body(input: &mut Parser<'_, '_>) -> Result<MediaFeature, ()> {
    let name_first = input.try_parse(|i| -> Result<MediaFeature, ()> {
        i.skip_whitespace();
        let name = i
            .expect_ident()
            .map(|s| s.as_ref().to_owned())
            .map_err(|_| ())?;
        parse_name_first(i, &name)
    });
    if let Ok(feature) = name_first {
        return Ok(feature);
    }
    input.try_parse(|i| parse_value_first(i))
}

/// Parse a name-first feature after its leading `<mf-name>`. `Err(())` means the
/// content is not a recognized name-first feature (caller retries value-first /
/// falls back to general-enclosed).
fn parse_name_first(input: &mut Parser<'_, '_>, name: &str) -> Result<MediaFeature, ()> {
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
    // ident followed by some other token → not a recognized feature.
    Err(())
}

fn parse_boolean(name: &str) -> Result<MediaFeature, ()> {
    let lower = name.to_ascii_lowercase();
    // §2.4.4: `min-`/`max-` in a boolean context is a syntax error → not a
    // recognized feature → general-enclosed.
    if lower.starts_with("min-") || lower.starts_with("max-") {
        return Err(());
    }
    classify_boolean_feature(&lower)
        .map(MediaFeature::Boolean)
        .ok_or(())
}

fn parse_plain(input: &mut Parser<'_, '_>, name: &str) -> Result<MediaFeature, ()> {
    input.skip_whitespace();
    let raw = parse_mf_value(input)?;
    let lower = name.to_ascii_lowercase();
    if let Some(df) = classify_discrete_feature(&lower) {
        return match raw {
            RawMfValue::Ident(kw) => discrete_value(df, &kw.to_ascii_lowercase())
                .map(|value| MediaFeature::Discrete { name: df, value })
                .ok_or(()),
            _ => Err(()),
        };
    }
    let (base, op) = strip_min_max(&lower);
    let rf = classify_range_feature(base).ok_or(())?;
    let value = coerce_raw(raw, rf).ok_or(())?;
    Ok(MediaFeature::Range {
        name: rf,
        constraints: vec![RangeConstraint { op, value }],
    })
}

fn parse_name_first_range(
    input: &mut Parser<'_, '_>,
    name: &str,
    op: RangeOp,
) -> Result<MediaFeature, ()> {
    input.skip_whitespace();
    let raw = parse_mf_value(input)?;
    let lower = name.to_ascii_lowercase();
    // a discrete feature in range context is invalid (§2.4.1).
    if classify_discrete_feature(&lower).is_some() {
        return Err(());
    }
    let rf = classify_range_feature(&lower).ok_or(())?;
    let value = coerce_raw(raw, rf).ok_or(())?;
    Ok(MediaFeature::Range {
        name: rf,
        constraints: vec![RangeConstraint { op, value }],
    })
}

fn parse_value_first(input: &mut Parser<'_, '_>) -> Result<MediaFeature, ()> {
    input.skip_whitespace();
    let raw1 = parse_mf_value(input)?;
    let op1 = try_comparison(input).ok_or(())?;
    input.skip_whitespace();
    let name = input.try_parse(|i| {
        i.expect_ident()
            .map(|s| s.as_ref().to_owned())
            .map_err(|_| ())
    })?;
    let lower = name.to_ascii_lowercase();
    let rf = classify_range_feature(&lower).ok_or(())?;
    let v1 = coerce_raw(raw1, rf).ok_or(())?;
    let mut constraints = vec![RangeConstraint {
        op: flip_op(op1),
        value: v1,
    }];
    // Optional second `<op> <value>` for `a <= width <= b`. §3 `<mf-range>`
    // requires both comparisons the same direction and forbids `=`. A
    // mixed-direction / `=` second comparison is NOT consumed (the `try_parse`
    // rewinds): it is left for [`parse_media_feature`]'s exhaustion check, which
    // routes the leftover tokens to `<general-enclosed>` (Kleene unknown), NOT
    // `not all`.
    if let Ok((op2, v2)) = input.try_parse(|i| -> Result<(RangeOp, RangeValue), ()> {
        let op2 = try_comparison(i).ok_or(())?;
        if !same_direction_range(op1, op2) {
            return Err(());
        }
        i.skip_whitespace();
        let raw2 = parse_mf_value(i)?;
        let v2 = coerce_raw(raw2, rf).ok_or(())?;
        Ok((op2, v2))
    }) {
        constraints.push(RangeConstraint { op: op2, value: v2 });
    }
    Ok(MediaFeature::Range {
        name: rf,
        constraints,
    })
}

/// A raw `<mf-value>` token before coercion to a feature's value type.
enum RawMfValue {
    /// A `<number>` token. `is_int` records whether the source was an integer
    /// token (cssparser `Token::Number { int_value: Some(_) }`) — `color` is an
    /// `<integer>` (MQ4 §6.1), so `8.0` (a `<number>` token) must be rejected
    /// even though it has no fractional part.
    Number {
        value: f64,
        is_int: bool,
    },
    Dimension(f64, String),
    Ratio(f64, f64),
    Ident(String),
    /// A length-typed `calc()` (CSS math), parsed by the canonical
    /// `crate::values::parse_length` — MQ4 §1.2/§1.3 delegates `<mf-value>` to
    /// CSS Values. Only valid for `width`/`height`; resolved at eval.
    Calc(Box<CalcExpr>),
}

/// The leading token of an `<mf-value>`, captured to release the token borrow
/// before any further parsing (e.g. the ratio `/ <number>` lookahead).
enum FirstToken {
    /// `value` + whether it was an integer token (`int_value.is_some()`).
    Num(f64, bool),
    Dim(f64, String),
    Id(String),
}

/// Parse a single `<mf-value>`
/// (`<number> | <dimension> | <ident> | <ratio> | calc()`).
fn parse_mf_value(input: &mut Parser<'_, '_>) -> Result<RawMfValue, ()> {
    input.skip_whitespace();
    // `calc()` (and CSS math generally) delegates to the canonical CSS Values
    // length parser — MQ4 §1.2/§1.3 defers `<mf-value>` types/units to CSS
    // Values, so there is one calc grammar, not a hand-rolled second one. A
    // non-`calc()` leading token makes the closure return `Err`, so `try_parse`
    // rewinds to the token path below (which also covers the CSS absolute units
    // `parse_length` omits). Only a length-typed `calc()` survives here; a
    // number-typed one (e.g. `calc(40)`) is filtered at coercion per feature.
    // The unsupported tail (abs-unit/resolution-unit `calc()`, `min`/`max`/
    // `clamp`) is the carved slot `#11-media-css-values-fidelity`.
    if let Ok(expr) = input.try_parse(|i| match crate::values::parse_length(i) {
        Ok(CssValue::Calc(expr)) => Ok(expr),
        _ => Err(()),
    }) {
        return Ok(RawMfValue::Calc(expr));
    }
    let first = match input.next().map_err(|_| ())? {
        Token::Number {
            value, int_value, ..
        } => FirstToken::Num(f64::from(*value), int_value.is_some()),
        Token::Dimension { value, unit, .. } => {
            FirstToken::Dim(f64::from(*value), unit.as_ref().to_owned())
        }
        Token::Ident(s) => FirstToken::Id(s.as_ref().to_owned()),
        _ => return Err(()),
    };
    match first {
        FirstToken::Num(n, is_int) => {
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
                Err(()) => Ok(RawMfValue::Number { value: n, is_int }),
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
            RawMfValue::Dimension(v, u) => media_length_to_value(v, &u),
            // A unitless `0` is `0px` (CSS); any other bare number is invalid
            // as a `<length>`.
            RawMfValue::Number { value, .. } => (value == 0.0).then_some(RangeValue::Length {
                value: 0.0,
                unit: LengthUnit::Px,
            }),
            // A length-typed `calc()` is carried symbolically (resolved at
            // eval). A number-typed `calc()` (e.g. `calc(40)`) is not a
            // `<length>` → invalid (§4.1/§4.2).
            RawMfValue::Calc(expr) if calc_has_length(&expr) => Some(RangeValue::Calc(expr)),
            _ => None,
        },
        RangeFeature::AspectRatio => match raw {
            // `<ratio>` components are non-negative (css-values-4 §5.7); a
            // negative component is invalid, but `0` (incl. a zero denominator)
            // is a valid degenerate ratio → ±inf/NaN, handled by `compare`.
            RawMfValue::Ratio(n, d) if n >= 0.0 && d >= 0.0 => Some(RangeValue::Ratio(n / d)),
            RawMfValue::Number { value, .. } if value >= 0.0 => Some(RangeValue::Ratio(value)),
            _ => None,
        },
        RangeFeature::Resolution => match raw {
            RawMfValue::Dimension(v, u) => resolution_to_value(v, &u),
            // §5.1: `resolution` also accepts the `infinite` keyword.
            RawMfValue::Ident(s) if s.eq_ignore_ascii_case("infinite") => {
                Some(RangeValue::Dppx(f64::INFINITY))
            }
            _ => None,
        },
        RangeFeature::Color => match raw {
            // §6.1 + §2.4.3: `color` is an `<integer>` — an integer *token*, so
            // `8.0` (a `<number>` token, `is_int == false`) is invalid even with
            // a zero fraction. Negative integers parse (`false in the negative
            // range`) and must reach `compare`.
            RawMfValue::Number {
                value,
                is_int: true,
            } => Some(RangeValue::Number(value)),
            _ => None,
        },
    }
}

/// Whether a `calc()` tree is length-typed (contains at least one `<length>`
/// leaf). `width`/`height` accept `<length>`, not a bare `<number>`, so a
/// number-only `calc()` (e.g. `calc(40)` or `calc(2 * 3)`) must be rejected
/// (§4.1/§4.2). The canonical parser already excludes percentage-typed
/// `calc()`, so "contains a `<length>`" ⟺ "the expression resolves to a
/// `<length>`".
fn calc_has_length(expr: &CalcExpr) -> bool {
    match expr {
        CalcExpr::Length(..) => true,
        CalcExpr::Number(_) | CalcExpr::Percentage(_) => false,
        CalcExpr::Add(a, b) | CalcExpr::Sub(a, b) | CalcExpr::Mul(a, b) | CalcExpr::Div(a, b) => {
            calc_has_length(a) || calc_has_length(b)
        }
    }
}

/// Resolve a `<length>` dimension for a width/height media feature. Relative +
/// viewport units keep their unit (resolved at eval against the environment,
/// compared exactly); CSS absolute units (`in`/`cm`/`mm`/`q`/`pt`/`pc`) resolve
/// to px here at 96dpi (§1.3 / css-values-4 absolute lengths) and become
/// [`RangeValue::Converted`] — the cssparser `f32` source + factor make that px
/// inexact, so it (unlike a direct px) compares with tolerance.
fn media_length_to_value(value: f64, unit: &str) -> Option<RangeValue> {
    if let Ok(u) = crate::values::parse_length_unit(unit) {
        return Some(RangeValue::Length { value, unit: u });
    }
    let px = match unit.to_ascii_lowercase().as_str() {
        "in" => value * 96.0,
        "cm" => value * (96.0 / 2.54),
        "mm" => value * (96.0 / 25.4),
        "q" => value * (96.0 / 25.4 / 4.0),
        "pt" => value * (96.0 / 72.0),
        "pc" => value * 16.0,
        _ => return None,
    };
    Some(RangeValue::Converted(px))
}

/// `<resolution>` units → a dppx [`RangeValue`] — css-values-4 §7.4. `dppx`/`x`
/// are the canonical unit (exact [`RangeValue::Dppx`]); `dpi`/`dpcm` divide by a
/// (possibly inexact) factor, so they become [`RangeValue::Converted`] and
/// compare with tolerance.
fn resolution_to_value(value: f64, unit: &str) -> Option<RangeValue> {
    match unit.to_ascii_lowercase().as_str() {
        "dppx" | "x" => Some(RangeValue::Dppx(value)),
        "dpi" => Some(RangeValue::Converted(value / 96.0)),
        "dpcm" => Some(RangeValue::Converted(value / (96.0 / 2.54))),
        _ => None,
    }
}

/// Parse an `<mf-comparison>` operator (`<` `<=` `>` `>=` `=`). §3 requires no
/// whitespace between `<`/`>` and `=`, so `< =` parses as a bare `<` (the
/// trailing `=` then leaves the feature unrecognized → general-enclosed →
/// Kleene unknown).
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
            // `next_including_whitespace` so `< =` (with a gap) is NOT read as
            // `<=` — §3 forbids whitespace inside the comparison operator.
            match i.next_including_whitespace().map_err(|_| ())? {
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
        // §6.1: `color` is a range feature (`(color)` boolean + `(min-color: N)`).
        "color" => Some(RangeFeature::Color),
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
