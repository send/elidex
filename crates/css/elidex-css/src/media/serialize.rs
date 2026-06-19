//! Canonical serialization of a parsed [`MediaQueryList`] back to a CSS string
//! — CSSOM-1 §4.2 "serialize a media query (list)", extended to the full Media
//! Queries L4 AST. This backs `MediaQueryList.media` (CSSOM-View §4.2, Slice 2
//! VM `matchMedia`) and the canonical cache key keyed on the serialized text.
//!
//! **CSSOM-1 §4.2 is L3-era**: its algorithm only models
//! `[ not ]? <media-type> [ and ( <feature> ) ]*` with the colon feature
//! notation, and (step 4) drops a redundant leading `all`. The common case (a
//! recognized type with colon-notation features) follows it exactly (lowercase
//! names, `: ` value separator, single spaces around `and`). The L4 shapes the
//! algorithm has no
//! grammar for — `or`, the `only` qualifier, nested conditions, the `<mf-range>`
//! comparison notation (`width >= 5px`, `5px <= width <= 10px`), and
//! `<general-enclosed>` — have no spec serialization, so they follow the
//! **browser de-facto** form: operators spelled literally, `or`/`only` literal,
//! `<general-enclosed>` re-emitted verbatim from its captured source text.
//!
//! Two equivalent notations collapse to the same AST at parse and are
//! disambiguated by stored hints so `.media` round-trips the authored form:
//! `(min-width: 5px)` vs `(width >= 5px)` ([`RangeSyntax`]), and the lossy
//! absolute-unit conversion (`2.54cm`) which keeps its specified value+unit
//! ([`RangeValue::Converted`]). Numbers serialize through `f32` (the cssparser
//! token type they were losslessly lifted from) so the author's spelling —
//! `2.54em`, not the `f64`'s `2.5399999618530273em` — comes back.
//!
//! Acknowledged normalizations (equivalent CSS, not fidelity loss, like the
//! case/whitespace ones CSSOM mandates): the resolution unit `x` → `dppx` (its
//! canonical unit, css-values-4 §7.4); a value-first single comparison
//! (`5px < width`) → name-first (`width > 5px`); a unitless `0` length → `0px`
//! (the parser resolves the unit); a bare-`<number>` aspect-ratio → `<n> / 1`
//! (css-values-4 §5.7 `<ratio>`, matching the computed-value form); and
//! redundant grouping `((…))` → `(…)` (the parser drops the inner layer).

use std::fmt;

use cssparser::serialize_identifier;

#[allow(clippy::wildcard_imports)]
use super::types::*;

impl fmt::Display for MediaQueryList {
    /// CSSOM-1 §4.2 "serialize a media query list": the empty list is the empty
    /// string; otherwise each query is serialized and the results joined with
    /// `", "`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, query) in self.0.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            fmt::Display::fmt(query, f)?;
        }
        Ok(())
    }
}

impl fmt::Display for MediaQuery {
    /// CSSOM-1 §4.2 "serialize a media query" (extended for `only` + condition
    /// trees + condition-only queries).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Step 1 (+ de-facto `only`, which CSSOM does not model): the qualifier
        // prefix. A condition-only query never carries one (the parser folds a
        // leading `not` into the condition tree), so this only fires with a
        // media type.
        let qualified = match self.qualifier {
            Some(Qualifier::Not) => {
                f.write_str("not ")?;
                true
            }
            Some(Qualifier::Only) => {
                f.write_str("only ")?;
                true
            }
            None => false,
        };
        match (&self.media_type, &self.condition) {
            // Condition-only query — `(min-width: 5px)`, `(a) or (b)`.
            (None, Some(cond)) => write_condition(f, cond),
            // Type-only query — `screen`, `not print`, `only screen`, `tv`.
            (Some(media_type), None) => write_media_type(f, media_type),
            // Type + condition — CSSOM-1 §4.2 steps 4-5.
            (Some(media_type), Some(cond)) => {
                // Step 4: emit `<type> and ` unless the type is a redundant
                // unqualified `all` (`all and (color)` → `(color)`).
                if !matches!(media_type, MediaType::All) || qualified {
                    write_media_type(f, media_type)?;
                    f.write_str(" and ")?;
                    // The condition follows `<type> and`, a restricted
                    // `<media-condition-without-or>` slot.
                    write_condition_without_or(f, cond)
                } else {
                    // `all` was elided, so the condition is now the entire
                    // query — a full top-level `<media-condition>` (a bare `or`
                    // is valid, not re-wrapped). This keeps `all and (X)`
                    // serializing identically to the equivalent bare `(X)`, so
                    // the two share a canonical cache key.
                    write_condition(f, cond)
                }
            }
            // The parser never builds a query with neither a type nor a
            // condition; serialize it as the empty string for totality.
            (None, None) => Ok(()),
        }
    }
}

/// CSSOM-1 §4.2 step 2: the media type "serialized as an identifier … converted
/// to ASCII lowercase". The keyword types are fixed lowercase spellings;
/// [`MediaType::Other`] holds the already-lowercased ident and is escaped per
/// the CSS identifier serialization rules.
fn write_media_type(f: &mut fmt::Formatter<'_>, media_type: &MediaType) -> fmt::Result {
    match media_type {
        MediaType::All => f.write_str("all"),
        MediaType::Screen => f.write_str("screen"),
        MediaType::Print => f.write_str("print"),
        MediaType::Other(ident) => serialize_identifier(ident, f),
    }
}

/// Serialize a full `<media-condition>` — the top-level `and`/`or`/`not` are
/// bare (no wrapping parens); each operand is a `<media-in-parens>`.
fn write_condition(f: &mut fmt::Formatter<'_>, cond: &MediaCondition) -> fmt::Result {
    match cond {
        MediaCondition::Feature(feature) => write_feature(f, feature),
        // The captured source text includes its own delimiters (`( … )` or
        // `name( … )`) — re-emit verbatim (CSSOM has no canonical form for it).
        MediaCondition::GeneralEnclosed(raw) => f.write_str(raw),
        MediaCondition::Not(inner) => {
            f.write_str("not ")?;
            write_in_parens(f, inner)
        }
        MediaCondition::And(terms) => write_joined(f, terms, " and "),
        MediaCondition::Or(terms) => write_joined(f, terms, " or "),
    }
}

/// Serialize a `<media-condition-without-or>` — the form after `<media-type>
/// and`. Identical to [`write_condition`] except a top-level `or` is not valid
/// here, so it was a parenthesized `<media-in-parens>` and is re-wrapped.
fn write_condition_without_or(f: &mut fmt::Formatter<'_>, cond: &MediaCondition) -> fmt::Result {
    match cond {
        MediaCondition::Or(_) => write_in_parens(f, cond),
        _ => write_condition(f, cond),
    }
}

/// Serialize a `<media-in-parens>` operand: a nested condition (`and`/`or`/`not`)
/// is wrapped as `( <media-condition> )`; a `<media-feature>` and
/// `<general-enclosed>` already carry their own parens, so they serialize
/// exactly as at the top level.
fn write_in_parens(f: &mut fmt::Formatter<'_>, cond: &MediaCondition) -> fmt::Result {
    match cond {
        MediaCondition::And(_) | MediaCondition::Or(_) | MediaCondition::Not(_) => {
            f.write_str("(")?;
            write_condition(f, cond)?;
            f.write_str(")")
        }
        MediaCondition::Feature(_) | MediaCondition::GeneralEnclosed(_) => write_condition(f, cond),
    }
}

/// Join `<media-in-parens>` operands with `sep` (` and ` / ` or `).
fn write_joined(f: &mut fmt::Formatter<'_>, terms: &[MediaCondition], sep: &str) -> fmt::Result {
    for (i, term) in terms.iter().enumerate() {
        if i > 0 {
            f.write_str(sep)?;
        }
        write_in_parens(f, term)?;
    }
    Ok(())
}

/// Serialize a `<media-feature>` (the parenthesized `( … )`).
fn write_feature(f: &mut fmt::Formatter<'_>, feature: &MediaFeature) -> fmt::Result {
    match feature {
        MediaFeature::Boolean(bf) => write!(f, "({})", boolean_name(*bf)),
        MediaFeature::Discrete { name, value } => {
            write!(f, "({}: {})", discrete_name(*name), discrete_value(*value))
        }
        MediaFeature::Range {
            name,
            constraints,
            syntax,
        } => write_range(f, *name, constraints, *syntax),
    }
}

/// Serialize a range feature in its authored notation.
fn write_range(
    f: &mut fmt::Formatter<'_>,
    name: RangeFeature,
    constraints: &[RangeConstraint],
    syntax: RangeSyntax,
) -> fmt::Result {
    let name = range_name(name);
    // The parser builds exactly one constraint for `Plain` and one or two for
    // `Comparison`; the fail-open `Ok(())` arms below are unreachable fallbacks.
    debug_assert!(
        !constraints.is_empty() && constraints.len() <= 2,
        "range feature has 1-2 constraints, got {}",
        constraints.len()
    );
    match syntax {
        // Colon notation — always a single `=` / `>=` / `<=` constraint, mapped
        // back to the plain / `min-` / `max-` feature name (CSSOM-1 §4.2).
        RangeSyntax::Plain => {
            let Some(c) = constraints.first() else {
                return Ok(());
            };
            // `<`/`>` are kept as an explicit no-prefix arm (not folded into
            // `Eq`) to document that the parser never pairs them with `Plain`.
            #[allow(clippy::match_same_arms)]
            let prefix = match c.op {
                RangeOp::Eq => "",
                RangeOp::Ge => "min-",
                RangeOp::Le => "max-",
                RangeOp::Lt | RangeOp::Gt => "",
            };
            write!(f, "({prefix}{name}: ")?;
            write_value(f, &c.value)?;
            f.write_str(")")
        }
        // Comparison notation — `( name <op> v )` or the two-sided
        // `( v0 <op> name <op> v1 )` (the left operator is the stored
        // constraint flipped back to value-first orientation).
        RangeSyntax::Comparison => match constraints {
            [c] => {
                write!(f, "({name} {} ", op_str(c.op))?;
                write_value(f, &c.value)?;
                f.write_str(")")
            }
            [lo, hi] => {
                f.write_str("(")?;
                write_value(f, &lo.value)?;
                write!(f, " {} {name} {} ", op_str(lo.op.flipped()), op_str(hi.op))?;
                write_value(f, &hi.value)?;
                f.write_str(")")
            }
            // The parser only ever builds one or two constraints.
            _ => Ok(()),
        },
    }
}

/// Serialize an `<mf-value>`.
fn write_value(f: &mut fmt::Formatter<'_>, value: &RangeValue) -> fmt::Result {
    match value {
        RangeValue::Length { value, unit } => write!(f, "{}{}", css_num(*value), unit.as_str()),
        RangeValue::Calc(expr) => write!(f, "calc({})", expr.to_css_string()),
        RangeValue::Ratio { num, den } => write!(f, "{} / {}", css_num(*num), css_num(*den)),
        RangeValue::Dppx(dppx) => {
            // §5.1: an `infinite` resolution is a keyword, not a dimension.
            if dppx.is_infinite() {
                f.write_str("infinite")
            } else {
                write!(f, "{}dppx", css_num(*dppx))
            }
        }
        RangeValue::Number(n) => write!(f, "{}", css_num(*n)),
        // The lossy absolute-unit conversion kept the specified value+unit.
        RangeValue::Converted { value, unit, .. } => write!(f, "{}{}", css_num(*value), unit),
    }
}

/// Serialize a media value's number. The stored `f64` was losslessly widened
/// from cssparser's `f32` token, so narrowing back recovers the author's
/// spelling — `2.54em` instead of the `f64`'s `2.5399999618530273em` — and
/// `f32`'s `Display` emits the shortest round-tripping form (no trailing `.0`).
///
/// A token that overflows `f32` to ±∞ (e.g. `1e40px`) is clamped to the nearest
/// finite extremum (`f32::MAX` / `f32::MIN`), NOT censored to `0`: zeroing would
/// invert the query's meaning — an always-false `(min-width: 1e40px)` would
/// reserialize to an always-true `(min-width: 0px)` and, since this output is
/// the canonical cache key, alias a real `0` breakpoint. The clamp evaluates
/// identically to ±∞ for every realistic viewport (no device is `f32::MAX` px
/// wide) and stays valid, round-trip-stable CSS. A NaN (unreachable from a
/// token) censors to `0`. The legitimate `infinite` resolution keyword is
/// handled by the caller, not here.
#[allow(clippy::cast_possible_truncation)] // intentional narrowing — see above.
fn css_num(value: f64) -> f32 {
    let narrowed = value as f32;
    if narrowed.is_finite() {
        narrowed
    } else if narrowed.is_nan() {
        0.0
    } else if narrowed > 0.0 {
        f32::MAX
    } else {
        f32::MIN
    }
}

fn op_str(op: RangeOp) -> &'static str {
    match op {
        RangeOp::Lt => "<",
        RangeOp::Le => "<=",
        RangeOp::Gt => ">",
        RangeOp::Ge => ">=",
        RangeOp::Eq => "=",
    }
}

fn range_name(name: RangeFeature) -> &'static str {
    match name {
        RangeFeature::Width => "width",
        RangeFeature::Height => "height",
        RangeFeature::AspectRatio => "aspect-ratio",
        RangeFeature::Resolution => "resolution",
        RangeFeature::Color => "color",
    }
}

fn boolean_name(bf: BooleanFeature) -> &'static str {
    match bf {
        BooleanFeature::Width => "width",
        BooleanFeature::Height => "height",
        BooleanFeature::AspectRatio => "aspect-ratio",
        BooleanFeature::Resolution => "resolution",
        BooleanFeature::Orientation => "orientation",
        BooleanFeature::Color => "color",
        BooleanFeature::PrefersColorScheme => "prefers-color-scheme",
        BooleanFeature::PrefersReducedMotion => "prefers-reduced-motion",
    }
}

fn discrete_name(df: DiscreteFeature) -> &'static str {
    match df {
        DiscreteFeature::Orientation => "orientation",
        DiscreteFeature::PrefersColorScheme => "prefers-color-scheme",
        DiscreteFeature::PrefersReducedMotion => "prefers-reduced-motion",
    }
}

fn discrete_value(value: DiscreteValue) -> &'static str {
    match value {
        DiscreteValue::Portrait => "portrait",
        DiscreteValue::Landscape => "landscape",
        DiscreteValue::Light => "light",
        DiscreteValue::Dark => "dark",
        DiscreteValue::NoPreferenceMotion => "no-preference",
        DiscreteValue::Reduce => "reduce",
    }
}

#[cfg(test)]
mod tests {
    use super::super::parse_media_query_list;

    /// Assert a *canonical* query string round-trips: `parse → serialize` is the
    /// identity. The inputs here are already in the serializer's canonical form.
    #[track_caller]
    fn rt(canonical: &str) {
        let got = parse_media_query_list(canonical).to_string();
        assert_eq!(got, canonical, "round-trip changed canonical input");
    }

    /// Assert a non-canonical `input` serializes to `expected` (case/whitespace/
    /// notation normalization), and that `expected` is itself a fixed point.
    #[track_caller]
    fn norm(input: &str, expected: &str) {
        let got = parse_media_query_list(input).to_string();
        assert_eq!(got, expected, "normalization mismatch");
        rt(expected);
    }

    // --- §4.2 common case: type + colon features ---------------------------

    #[test]
    fn type_and_colon_features() {
        rt("screen");
        rt("print");
        rt("not screen");
        rt("only screen");
        rt("screen and (min-width: 5px)");
        rt("not screen and (min-width: 5px) and (max-width: 40px)");
        rt("screen and (width: 600px)");
    }

    #[test]
    fn cssom_examples() {
        // The two worked examples from CSSOM-1 §4.2.
        norm(
            "not screen and (min-WIDTH:5px) AND (max-width:40px)",
            "not screen and (min-width: 5px) and (max-width: 40px)",
        );
        // A redundant unqualified `all` is dropped; features are NOT deduped.
        norm("all and (color) and (color)", "(color) and (color)");
    }

    // --- §4.2 step 4: the `all` skip -------------------------------------

    #[test]
    fn all_skip_unless_qualified() {
        norm("all and (color)", "(color)");
        rt("all"); // type-only `all` is kept (step 3, not step 4).
        rt("not all"); // negated `all` is kept (step 4 "or … negated").
        rt("not all and (color)");
        rt("only all and (color)");
    }

    // --- condition-only queries + boolean tree ---------------------------

    #[test]
    fn condition_only_and_boolean_tree() {
        rt("(min-width: 500px)");
        rt("(color)");
        rt("(orientation: portrait)");
        rt("(width: 600px) and (height: 400px)");
        rt("(width: 600px) or (height: 400px)");
        rt("not (color)");
        // A nested condition as an `and`/`or` operand is re-parenthesized.
        rt("((width: 1px) or (height: 2px)) and (color)");
        rt("screen and ((width: 1px) or (height: 2px))");
    }

    // --- §2.4 range comparison notation ----------------------------------

    #[test]
    fn comparison_notation() {
        rt("(width < 600px)");
        rt("(width <= 600px)");
        rt("(width > 600px)");
        rt("(width >= 600px)");
        rt("(width = 600px)");
        rt("(100px <= width <= 600px)");
        rt("(100px < width < 600px)");
        // Value-first single comparison normalizes to name-first.
        norm("(600px > width)", "(width < 600px)");
    }

    #[test]
    fn min_max_vs_comparison_are_distinct() {
        // The collapse the `RangeSyntax` hint disambiguates: same constraints,
        // different authored notation, each round-trips to itself.
        rt("(min-width: 5px)");
        rt("(width >= 5px)");
        rt("(max-width: 5px)");
        rt("(width <= 5px)");
    }

    // --- value types -----------------------------------------------------

    #[test]
    fn value_types() {
        // f32-faithful decimal (not the f64's 2.5399999618530273).
        rt("(min-width: 2.54em)");
        rt("(min-width: 10.5px)");
        // aspect-ratio keeps numerator/denominator.
        rt("(min-aspect-ratio: 16 / 9)");
        rt("(aspect-ratio: 4 / 3)");
        // color is a bare integer.
        rt("(min-color: 2)");
        rt("(color: 8)");
        // resolution: dppx canonical; `infinite` keyword.
        rt("(min-resolution: 2dppx)");
        rt("(max-resolution: infinite)");
        // length-typed calc().
        rt("(min-width: calc(50px + 1em))");
    }

    #[test]
    fn lossy_units_keep_specified_form() {
        // Absolute-unit lengths + dpi/dpcm keep their specified value+unit
        // rather than collapsing to the lossy converted px/dppx.
        rt("(min-width: 2.54cm)");
        rt("(width >= 1in)");
        rt("(min-resolution: 96dpi)");
        // `x` normalizes to the canonical `dppx`.
        norm("(min-resolution: 2x)", "(min-resolution: 2dppx)");
    }

    #[test]
    fn documented_normalizations() {
        // Equivalent-CSS normalizations the serializer is allowed to make.
        norm("(width:600px)", "(width: 600px)"); // inner colon spacing
        norm("(min-width: 0)", "(min-width: 0px)"); // unitless 0 → 0px
        norm("(aspect-ratio: 2)", "(aspect-ratio: 2 / 1)"); // bare number → n / 1
        norm("((color))", "(color)"); // redundant grouping dropped
    }

    #[test]
    fn all_elision_matches_bare_condition() {
        // After eliding the redundant `all` (CSSOM-1 §4.2 step 4), the condition
        // is the whole query and must serialize identically to the equivalent
        // bare condition-only query — including a top-level `or` (NOT re-wrapped)
        // — so the two share a canonical cache key.
        let with_all = parse_media_query_list("all and ((color) or (width: 1px))").to_string();
        let bare = parse_media_query_list("(color) or (width: 1px)").to_string();
        assert_eq!(with_all, bare);
        assert_eq!(bare, "(color) or (width: 1px)");
        // The `and`-of-features case was already correct and stays so.
        assert_eq!(
            parse_media_query_list("all and (width: 1px) and (height: 2px)").to_string(),
            "(width: 1px) and (height: 2px)"
        );
    }

    #[test]
    fn overflow_value_preserves_eval_not_zeroed() {
        use super::super::{evaluate, MediaEnvironment};
        let env = MediaEnvironment::default(); // 1024×768 screen
        let overflow = parse_media_query_list("(min-width: 1e40px)");
        let s = overflow.to_string();
        // Must NOT collapse to the always-true `0px` — that would alias an
        // impossible breakpoint onto a real zero breakpoint in the cache key.
        assert_ne!(s, "(min-width: 0px)");
        // `(min-width: 1e40px)` is always false (no viewport is 1e40px); the
        // serialized form must reparse to the same false result, not flip true.
        assert!(!evaluate(&overflow, &env));
        assert!(!evaluate(&parse_media_query_list(&s), &env));
        // The clamped finite value reserializes identically (round-trip stable).
        assert_eq!(parse_media_query_list(&s).to_string(), s);
    }

    // --- §2.3 / §3.2 media types -----------------------------------------

    #[test]
    fn media_types() {
        rt("tv"); // deprecated/unknown type ident is preserved (lowercased).
        norm("TV", "tv");
        rt("not tv");
        rt("projection and (color)");
    }

    // --- §3.1 general-enclosed passthrough -------------------------------

    #[test]
    fn general_enclosed_passthrough() {
        // Unknown feature / future syntax round-trips verbatim.
        rt("(unknownfeature)");
        rt("(weird: x)");
        rt("(color) or (unknownfeature)");
        // A function-token block is captured via the other parse path (the
        // whole `name( … )`, delimiters included).
        rt("myfunc(a b c)");
        rt("(color) and selector(:focus)");
    }

    #[test]
    fn nested_not_reparenthesizes() {
        rt("not (color)");
        rt("not (not (color))");
        rt("not ((width: 1px) and (height: 2px))");
    }

    // --- §3.2 malformed → not all ----------------------------------------

    #[test]
    fn malformed_serializes_not_all() {
        // A top-level grammar failure becomes the `not all` sentinel.
        norm("and", "not all");
        norm("@media", "not all");
    }

    // --- the list join ---------------------------------------------------

    #[test]
    fn query_list_join() {
        assert_eq!(parse_media_query_list("").to_string(), "");
        rt("screen, print");
        rt("(min-width: 5px), (max-width: 40px)");
    }
}
